//! `--tray-probe <out>` (dev builds only): the mushroom against every menu bar it can sit
//! on. Each face the tray can wear is put on a real status item, the way the presenter
//! puts it on, and drawn under each appearance a menu bar can have — light, dark, and
//! both with Increase Contrast — with the app's own appearance set to the other one, as it
//! is over a dark wallpaper in Light mode (DECISIONS B151). The mushroom's pixels (the cap,
//! where no dot ever sits) must stand out from that bar: light on a dark one, dark on a
//! light one; the plain and paused faces must stay template images, which the bar tints.
//! Each drawing is written as a PNG beside the report.
//!
//! Then it waits for the scenario's asks, for screenshots of the real menu bar: a face's
//! name in `<dir>/want` puts that face on (the item following the system, as in the app)
//! and writes it back to `<dir>/shown`; `done` ends the probe. Where the item is, in
//! screencapture's coordinates (top-left origin, points), is in `<dir>/rect`.

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceName, NSAppearanceNameAccessibilityHighContrastAqua, NSAppearanceNameAccessibilityHighContrastDarkAqua, NSAppearanceNameAqua,
    NSAppearanceNameDarkAqua, NSApplication, NSBitmapImageFileType, NSBitmapImageRep, NSColorSpace, NSStatusBarButton,
};
use objc2_foundation::{MainThreadMarker, NSDictionary};
use serde::Serialize;
use shroom_core::scene::Face;
use tauri::AppHandle;
use tauri::image::Image;
use tauri::tray::TrayIconBuilder;

use crate::harness::write_json;
use crate::runtime::presenter::face_image;

const FACES: [(Face, &str); 5] = [(Face::Plain, "plain"), (Face::PauseBars, "paused"), (Face::Red, "recording"), (Face::Amber, "attention"), (Face::Frost, "update")];

#[derive(Serialize, Default)]
struct Report {
    checks: Vec<Check>,
    error: Option<String>,
}

#[derive(Serialize)]
struct Check {
    face: &'static str,
    /// The menu bar's appearance the item was drawn in.
    bar: &'static str,
    /// The app's own, set to the other family.
    app: &'static str,
    /// The cap's mean lightness (0 black … 1 white) over its opaque, uncolored pixels.
    cap: Option<f64>,
    pixels: usize,
    /// The face is a template image (the plain mushroom and the pause bars).
    template: bool,
    ok: bool,
}

/// The bars, each with whether it is dark and the app appearance set against it.
fn bars() -> [(&'static str, &'static NSAppearanceName, bool, &'static str, &'static NSAppearanceName); 4] {
    unsafe {
        [
            ("light", NSAppearanceNameAqua, false, "dark", NSAppearanceNameDarkAqua),
            ("dark", NSAppearanceNameDarkAqua, true, "light", NSAppearanceNameAqua),
            ("light, increased contrast", NSAppearanceNameAccessibilityHighContrastAqua, false, "dark", NSAppearanceNameDarkAqua),
            ("dark, increased contrast", NSAppearanceNameAccessibilityHighContrastDarkAqua, true, "light", NSAppearanceNameAqua),
        ]
    }
}

/// The cap's lightness in a drawing of the item: the top half's opaque, unsaturated pixels.
fn cap(rep: &NSBitmapImageRep) -> (f64, usize) {
    let srgb = NSColorSpace::sRGBColorSpace();
    let (w, h) = (rep.pixelsWide(), rep.pixelsHigh());
    let (mut sum, mut n) = (0.0, 0);
    for y in 0..h / 2 {
        for x in 0..w {
            let Some(c) = rep.colorAtX_y(x, y).and_then(|c| c.colorUsingColorSpace(&srgb)) else { continue };
            let (r, g, b, a) = (c.redComponent(), c.greenComponent(), c.blueComponent(), c.alphaComponent());
            let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
            if a > 0.5 && hi - lo < 0.15 {
                sum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
                n += 1;
            }
        }
    }
    (if n > 0 { sum / n as f64 } else { f64::NAN }, n)
}

fn save(rep: &NSBitmapImageRep, path: &Path) {
    let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) };
    if let Some(data) = png {
        let _ = std::fs::write(path, data.to_vec());
    }
}

/// Draws every face under every bar and reads its cap (main thread).
fn measure(button: &NSStatusBarButton, dir: &Path, mtm: MainThreadMarker) -> Vec<Check> {
    let ns_app = NSApplication::sharedApplication(mtm);
    let mut checks = Vec::new();
    for (face, face_name) in FACES {
        button.setImage(Some(&face_image(face)));
        for (bar, bar_name, dark, app, app_name) in bars() {
            ns_app.setAppearance(NSAppearance::appearanceNamed(app_name).as_deref());
            button.setAppearance(NSAppearance::appearanceNamed(bar_name).as_deref());
            let bounds = button.bounds();
            let Some(rep) = button.bitmapImageRepForCachingDisplayInRect(bounds) else { continue };
            button.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
            save(&rep, &dir.join(format!("{face_name}-{}.png", bar.replace(", ", "-").replace(' ', "-"))));
            // A template face is tinted by the menu bar itself, outside the item's own
            // drawing (a cached drawing of it is empty): it passes by staying a template.
            if matches!(face, Face::Plain | Face::PauseBars) {
                let template = button.image().is_some_and(|i| i.isTemplate());
                checks.push(Check { face: face_name, bar, app, cap: None, pixels: 0, template, ok: template });
                continue;
            }
            let (lightness, pixels) = cap(&rep);
            let ok = pixels >= 20 && if dark { lightness > 0.6 } else { lightness < 0.4 };
            checks.push(Check { face: face_name, bar, app, cap: Some((lightness * 1000.0).round() / 1000.0), pixels, template: false, ok });
        }
    }
    ns_app.setAppearance(None);
    button.setAppearance(None);
    checks
}

/// Runs on a helper thread, hopping to the main thread for every AppKit call.
pub fn tray_probe(app: AppHandle, out: PathBuf) {
    let dir = out.parent().map(Path::to_path_buf).unwrap_or_default();
    let on_main = |f: Box<dyn FnOnce(&AppHandle) + Send>| {
        let (tx, rx) = channel();
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            f(&h);
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(Duration::from_secs(20));
    };
    let report = std::sync::Arc::new(std::sync::Mutex::new(Report::default()));

    // The item, made as the presenter makes it, then every face measured.
    {
        let (r, dir) = (report.clone(), dir.clone());
        on_main(Box::new(move |app| {
            let built = TrayIconBuilder::with_id("shroom")
                .icon(Image::from_bytes(include_bytes!("../icons/tray/plain-template@2x.png")).expect("a bundled tray icon"))
                .icon_as_template(true)
                .build(app);
            let tray = match built {
                Ok(t) => t,
                Err(e) => return r.lock().unwrap().error = Some(format!("tray: {e}")),
            };
            let measured = tray.with_inner_tray_icon(move |inner| {
                let mtm = MainThreadMarker::new()?;
                let button: Retained<NSStatusBarButton> = inner.ns_status_item()?.button(mtm)?;
                Some(measure(&button, &dir, mtm))
            });
            match measured.ok().flatten() {
                Some(checks) => r.lock().unwrap().checks = checks,
                None => r.lock().unwrap().error = Some("no status item to draw".into()),
            }
        }));
    }
    let failed = {
        let rep = report.lock().unwrap();
        rep.error.is_some() || rep.checks.is_empty() || rep.checks.iter().any(|c| !c.ok)
    };
    let written = write_json(&out, &*report.lock().unwrap());

    // Where the item is, for the scenario's screenshots, once it has settled.
    std::thread::sleep(Duration::from_millis(800));
    {
        let dir = dir.clone();
        on_main(Box::new(move |app| {
            let Some(t) = app.tray_by_id("shroom") else { return };
            let rect = t.with_inner_tray_icon(|inner| {
                let mtm = MainThreadMarker::new()?;
                let window = inner.ns_status_item()?.button(mtm)?.window()?;
                let (f, screen) = (window.frame(), window.screen()?.frame());
                Some(format!("{},{},{},{}", f.origin.x - screen.origin.x, screen.origin.y + screen.size.height - (f.origin.y + f.size.height), f.size.width, f.size.height))
            });
            if let Some(rect) = rect.ok().flatten() {
                let _ = std::fs::write(dir.join("rect"), rect);
            }
        }));
    }

    // The scenario's asks, for at most ten minutes.
    let started = Instant::now();
    let mut shown = String::new();
    while started.elapsed() < Duration::from_secs(600) {
        let want = std::fs::read_to_string(dir.join("want")).unwrap_or_default().trim().to_string();
        if want == "done" {
            break;
        }
        if want != shown {
            if let Some(&(face, _)) = FACES.iter().find(|(_, n)| *n == want) {
                on_main(Box::new(move |app| {
                    let Some(t) = app.tray_by_id("shroom") else { return };
                    let _ = t.with_inner_tray_icon(move |inner| {
                        let mtm = MainThreadMarker::new()?;
                        inner.ns_status_item()?.button(mtm)?.setImage(Some(&face_image(face)));
                        Some(())
                    });
                }));
                let _ = std::fs::write(dir.join("shown"), &want);
            }
            shown = want;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    app.exit(if failed { 1 } else { written });
}
