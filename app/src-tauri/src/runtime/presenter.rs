//! The presenter: the only place the Scene becomes AppKit. The tray's face and menu, the
//! popover's, the countdown's and the agent label's panels (`overlay.rs`; the popover key
//! only when asked — law 5), the three windows (onboarding, Library, Settings — each
//! excluded from capture at creation), the camera panels (`camera.rs`), and the Dock (law
//! 6: the setup window's alone). Webviews get their slice as a `scene` event; they render,
//! nothing more.

use std::sync::{Arc, Mutex};

use shroom_core::event::{Event, QuitVia};
use shroom_core::scene::{Click, Face, Scene};
use shroom_core::world::{EditorId, WindowKind};
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::AllocAnyThread;
use objc2_app_kit::NSImage;
use objc2_foundation::{NSArray, NSData, NSRect, NSSize};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub use super::overlay::Overlay;
use super::{Runtime, overlay, probe};

pub const POPOVER: &str = "popover";
pub const COUNTDOWN: &str = "countdown";
/// The agent label's page (N62, B216).
pub const LABEL: &str = "label";
/// The marker toast's page (N78).
pub const TOAST: &str = "toast";
pub const ONBOARDING: &str = "onboarding";
pub const LIBRARY: &str = "library";
pub const SETTINGS: &str = "settings";
pub const FEEDBACK: &str = "feedback";
/// The editor windows' labels begin so: `editor-<its number>`, one window per video (N73).
pub const EDITOR: &str = "editor";
const POPOVER_WIDTH: f64 = 320.0;

/// A window's label: its kind's — an editor window's with its number (N73).
pub fn label(kind: WindowKind) -> std::borrow::Cow<'static, str> {
    match kind {
        WindowKind::Onboarding => ONBOARDING.into(),
        WindowKind::Library => LIBRARY.into(),
        WindowKind::Settings => SETTINGS.into(),
        WindowKind::Feedback => FEEDBACK.into(),
        WindowKind::Editor(id) => format!("{EDITOR}-{}", id.0).into(),
    }
}

/// The editor window a label names (`editor-<its number>`, as `label` makes it), if it
/// names one.
pub fn editor_of(label: &str) -> Option<EditorId> {
    let n = label.strip_prefix(EDITOR)?.strip_prefix('-')?;
    let digits = !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| n.parse().ok().map(EditorId)).flatten()
}

/// One of shroom's ordinary windows: titled, the standard ✕ (which the core answers — the
/// native close never runs), one instance each — an editor's one per video (N73). A
/// resizable one has a smallest size, and opens no bigger than the screen it opens on.
struct WindowSpec {
    url: &'static str,
    width: f64,
    height: f64,
    resizable: bool,
    min: Option<(f64, f64)>,
}

/// The setup window: one fixed size for every step, the canvas's frame — tall enough for the
/// tallest step (the token step with its reminder or its error) under the title bar (B189).
const ONBOARDING_WINDOW: WindowSpec = WindowSpec { url: "onboarding.html", width: 600.0, height: 680.0, resizable: false, min: None };
/// The Library: a list that grows — resizable.
const LIBRARY_WINDOW: WindowSpec = WindowSpec { url: "library.html", width: 680.0, height: 560.0, resizable: true, min: None };
/// A standard macOS settings window: fixed size, its toolbar tabs drawn by the webview. As tall
/// as the editor (Bagrat, 2026-10-25; B247): every tab fits the 688 under the title bar, Space
/// with the AI switch's parts included, and it fits a 1024×768 screen.
const SETTINGS_WINDOW: WindowSpec = WindowSpec { url: "settings.html", width: 600.0, height: 720.0, resizable: false, min: None };

/// Send feedback (N60): a small fixed window, the board's frame — room for the form with
/// “See exactly what's sent” open under the title bar.
const FEEDBACK_WINDOW: WindowSpec = WindowSpec { url: "feedback.html", width: 520.0, height: 600.0, resizable: false, min: None };

/// The editor (N63): the board's 1100×720 frame — the video, the time, the timeline and the
/// footer — resizable down to where the timeline still reads (900×640).
const EDITOR_WINDOW: WindowSpec = WindowSpec { url: "editor.html", width: 1100.0, height: 720.0, resizable: true, min: Some((900.0, 640.0)) };

fn spec(kind: WindowKind) -> &'static WindowSpec {
    match kind {
        WindowKind::Onboarding => &ONBOARDING_WINDOW,
        WindowKind::Library => &LIBRARY_WINDOW,
        WindowKind::Settings => &SETTINGS_WINDOW,
        WindowKind::Feedback => &FEEDBACK_WINDOW,
        WindowKind::Editor(_) => &EDITOR_WINDOW,
    }
}

pub struct Presenter {
    app: AppHandle,
    rt: Runtime,
    last: Arc<Mutex<Option<Scene>>>,
    excluded: Arc<Mutex<Vec<u32>>>,
}

/// The app, for the probe's read of the mushroom's visibility (main thread).
static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

impl Presenter {
    pub fn new(app: AppHandle, rt: Runtime, excluded: Arc<Mutex<Vec<u32>>>) -> Presenter {
        let _ = APP.set(app.clone());
        // The mushroom goes up before the core's first Scene (the setup runs on the main
        // thread): macOS lays a new status item out over the next run-loop turns, and a
        // popover the launch opens hangs from where it settled.
        if objc2_foundation::MainThreadMarker::new().is_some() {
            let _ = tray(&app, &rt);
            // The popover's and the count's panels are made now, while nothing of shroom's
            // is on screen: making one later could cost shroom its windows (`overlay.rs`).
            overlay::prepare(&app, &excluded);
        }
        Presenter { app, rt, last: Arc::default(), excluded }
    }

    pub fn present(&mut self, scene: &Scene) {
        let (app, rt, last, excluded) = (self.app.clone(), self.rt.clone(), self.last.clone(), self.excluded.clone());
        let s = scene.clone();
        let _ = self.app.run_on_main_thread(move || {
            let prev = last.lock().unwrap().replace(s.clone());
            apply(&app, &rt, prev.as_ref(), &s, &excluded);
        });
    }
}

/// A rectangle in AppKit's global coordinates (origin bottom-left, y up), in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Frame {
    fn of(r: objc2_foundation::NSRect) -> Frame {
        Frame { x: r.origin.x, y: r.origin.y, w: r.size.width, h: r.size.height }
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The mushroom on screen: its frame, whether macOS shows it (a crowded menu bar hides
/// items behind the notch), the visible frame of its screen, and that screen's top edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mushroom {
    pub frame: Frame,
    pub visible: bool,
    pub screen: Frame,
    pub top: f64,
}

/// Where the popover's top-left corner goes: centred under the mushroom, kept on its
/// screen — or, when the mushroom can't be seen, the top-right of the screen with the
/// pointer (N41). Whether it went under the mushroom.
pub fn popover_corner(mushroom: Option<Mushroom>, pointer_screen: Frame, width: f64) -> (f64, f64, bool) {
    const GAP: f64 = 6.0;
    const MARGIN: f64 = 8.0;
    let clamp = |x: f64, s: &Frame| x.max(s.x + MARGIN).min(s.x + s.w - width - MARGIN);
    match mushroom {
        Some(m) if m.visible && m.screen.w > 0.0 => (clamp(m.frame.x + m.frame.w / 2.0 - width / 2.0, &m.screen), m.frame.y - GAP, true),
        _ => (pointer_screen.x + pointer_screen.w - width - MARGIN, pointer_screen.y + pointer_screen.h - GAP, false),
    }
}

/// Whether the mushroom can be seen. macOS's word for it — the status item's window
/// occlusion — is taken when it says seen. When it says not, the mushroom is still seen
/// if, with the screen unlocked, it rests in the menu bar of its screen, clear of the
/// notch: inside the menu bar's part left or right of it (`notch`: those two parts'
/// widths), or anywhere across a screen without one. A reopen from Spotlight read the
/// occlusion as not seen with the mushroom in plain view, and its popover took the corner
/// (Bagrat's checkpoint, 2026-10-14: “definitely on spotlight open”); a mushroom behind
/// the notch, above a hidden menu bar, or behind the lock stays unseen (N41). A screen
/// mirrored to another display has no notch to hide behind: the mirror shows the whole
/// menu bar, and a MacBook mirrored to an external display put the popover in the corner
/// with the mushroom in view (the 0.1.12 checkpoint, 2026-10-15: “Still top right”).
pub fn seen(occlusion_visible: bool, locked: bool, frame: Frame, whole: Frame, notch: Option<(f64, f64)>, mirrored: bool) -> bool {
    if occlusion_visible {
        return true;
    }
    let (left, right) = (frame.x, frame.x + frame.w);
    let within = |lo: f64, hi: f64| left >= lo - 0.5 && right <= hi + 0.5;
    // In the menu bar: its top edge on the screen's, or a few points short of it (a mirror
    // set's menu bar is drawn at one display's height).
    let below = whole.y + whole.h - (frame.y + frame.h);
    // Never at the screen's left edge, the Apple menu's: there it isn't laid out yet (N57).
    let at_rest = whole.w > 0.0 && frame.w > 0.0 && frame.h > 0.0 && (-0.5..=frame.h / 2.0).contains(&below) && frame.x > whole.x + 0.5;
    let clear = match notch.filter(|_| !mirrored) {
        Some((l, r)) => within(whole.x, whole.x + l) || within(whole.x + whole.w - r, whole.x + whole.w),
        None => within(whole.x, whole.x + whole.w),
    };
    !locked && at_rest && clear
}

/// Which of the screens holds the mushroom: the one its top edge's middle is on. The
/// window's own word for its screen can be another one's (a mirror set), or none.
pub fn home(frame: Frame, screens: &[Frame]) -> Option<usize> {
    let (cx, top) = (frame.x + frame.w / 2.0, frame.y + frame.h - 1.0);
    screens.iter().position(|s| s.contains(cx, top))
}

/// How far over a screen's top edge macOS parks the mushroom while that screen's menu bar
/// is hidden: on the edge itself on the hosted Mac, 32 pt over it on Bagrat's mirror set.
const PARKED_REACH: f64 = 100.0;

/// The mushroom as it will rest, when it is parked over a hidden menu bar — the menu bar
/// set to hide, or a full-screen app's Space (DECISIONS B161): over no screen, just above
/// the top edge of the one under it (`screens`: each one's whole and visible frames).
/// It shows there as the menu bar does, so a popover hangs from there. Never behind the
/// lock.
pub fn parked(frame: Frame, screens: &[(Frame, Frame)], locked: bool) -> Option<Mushroom> {
    let wholes: Vec<Frame> = screens.iter().map(|s| s.0).collect();
    if locked || frame.w <= 0.0 || frame.h <= 0.0 || home(frame, &wholes).is_some() {
        return None;
    }
    let cx = frame.x + frame.w / 2.0;
    screens.iter().find_map(|&(whole, usable)| {
        let top = whole.y + whole.h;
        let over = frame.y - top;
        (cx >= whole.x && cx < whole.x + whole.w && (-0.5..=PARKED_REACH).contains(&over))
            .then_some(Mushroom { frame: Frame { y: top - frame.h, ..frame }, visible: true, screen: usable, top })
    })
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGSessionCopyCurrentDictionary() -> *mut objc2::runtime::AnyObject;
    fn CGDisplayIsInMirrorSet(display: u32) -> u32;
}

/// Whether the screen is locked (or there is no login session on screen).
fn screen_locked() -> bool {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_foundation::{NSDictionary, NSString};
    // SAFETY: the dictionary comes back owned (the Copy rule) and is toll-free bridged to
    // NSDictionary; it's read and released here.
    let raw = unsafe { CGSessionCopyCurrentDictionary() } as *mut NSDictionary<NSString, objc2::runtime::AnyObject>;
    let Some(session) = (unsafe { Retained::from_raw(raw) }) else { return true };
    session.objectForKey(&NSString::from_str("CGSSessionScreenIsLocked")).is_some_and(|v| unsafe { msg_send![&*v, boolValue] })
}

/// What the mushroom's reading was made of, for the log line each placement writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub occlusion: bool,
    pub locked: bool,
    pub whole: Frame,
    pub notch: Option<(f64, f64)>,
    pub mirrored: bool,
    /// The mushroom as it will rest, while it's parked over a hidden menu bar.
    pub parked: Option<Mushroom>,
}

/// The mushroom, read live from its status item (main thread).
fn mushroom(t: &TrayIcon) -> Option<Mushroom> {
    reading(t).map(|(m, _)| m)
}

/// The mushroom and what it was read from (main thread). Its screen is the one its frame
/// is on, whichever the window names; under mirroring the mirror set shows as one screen.
fn reading(t: &TrayIcon) -> Option<(Mushroom, Reading)> {
    t.with_inner_tray_icon(|inner| {
        use objc2::msg_send;
        use objc2_app_kit::{NSScreen, NSWindowOcclusionState};
        use objc2_foundation::NSString;
        let mtm = objc2_foundation::MainThreadMarker::new()?;
        let item = inner.ns_status_item()?;
        let window = item.button(mtm)?.window()?;
        let frame = Frame::of(window.frame());
        let screens = NSScreen::screens(mtm);
        let frames: Vec<Frame> = screens.iter().map(|s| Frame::of(s.frame())).collect();
        let both: Vec<(Frame, Frame)> = screens.iter().map(|s| (Frame::of(s.frame()), Frame::of(s.visibleFrame()))).collect();
        let screen = home(frame, &frames).map(|i| screens.objectAtIndex(i as _)).or_else(|| window.screen());
        const NONE: Frame = Frame { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        let (usable, whole) = screen.as_ref().map_or((NONE, NONE), |s| (Frame::of(s.visibleFrame()), Frame::of(s.frame())));
        let notch = screen.as_ref().filter(|s| s.safeAreaInsets().top > 0.0).map(|s| (s.auxiliaryTopLeftArea().size.width, s.auxiliaryTopRightArea().size.width));
        let mirrored = screen.as_ref().is_some_and(|s| {
            let id = s.deviceDescription().objectForKey(&NSString::from_str("NSScreenNumber"));
            // SAFETY: AppKit's NSScreenNumber is an NSNumber holding the CGDirectDisplayID.
            id.is_some_and(|n| unsafe { CGDisplayIsInMirrorSet(msg_send![&*n, unsignedIntValue]) } != 0)
        });
        let occlusion = window.occlusionState().contains(NSWindowOcclusionState::Visible);
        let locked = screen_locked();
        let visible = screen.is_some() && seen(occlusion, locked, frame, whole, notch, mirrored);
        let parked = parked(frame, &both, locked);
        Some((Mushroom { frame, visible, screen: usable, top: whole.y + whole.h }, Reading { occlusion, locked, whole, notch, mirrored, parked }))
    })
    .ok()
    .flatten()
}

/// The log line a popover's placement writes: where it went and everything that decided it.
pub fn placement_line(x: f64, y: f64, under: bool, held_ms: u64, m: Option<&(Mushroom, Reading)>) -> String {
    let at = if under { "under the mushroom" } else { "in the corner (N41)" };
    format!("popover: placed {at} at {x:.0},{y:.0} (held {held_ms} ms) — {}", read_text(m))
}

/// The log line an opening writes when it starts to wait for the mushroom: what it read.
pub fn held_line(m: Option<&(Mushroom, Reading)>) -> String {
    format!("popover: held for the mushroom — {}", read_text(m))
}

fn read_text(m: Option<&(Mushroom, Reading)>) -> String {
    let f = |r: &Frame| format!("{:.0},{:.0} {:.0}x{:.0}", r.x, r.y, r.w, r.h);
    match m {
        None => "mushroom not read".to_string(),
        Some((m, r)) => format!(
            "item {} screen {} notch {} mirrored {} occlusion {} locked {} seen {}{}",
            f(&m.frame),
            f(&r.whole),
            r.notch.map_or("none".to_string(), |(l, rr)| format!("{l:.1}|{rr:.1}")),
            r.mirrored,
            if r.occlusion { "visible" } else { "not visible" },
            r.locked,
            m.visible,
            r.parked.map_or(String::new(), |p| format!(" parked over a hidden menu bar, rests at {}", f(&p.frame)))
        ),
    }
}

/// Whether the mushroom can be seen now (the probe's `status_item_visible`, law 1). Before
/// the tray exists, or while a just-made one is still being laid out, it's taken as seen.
pub fn status_item_visible() -> bool {
    let m = APP.get().and_then(|a| a.tray_by_id("shroom")).and_then(|t| mushroom(&t));
    m.is_none_or(|m| m.visible) || holds(tray_young(), m)
}

/// How long macOS may take to lay a new status item out, once the event loop turns. A
/// fresh `NSStatusItem` reads a zero-height frame and no `Visible` occlusion bit at first;
/// then macOS parks it under the screen's bottom edge — sized, and for a moment already
/// marked as seen — and brings it into the menu bar, from below or from above. It settles
/// within a few hundred milliseconds (a JXA probe on this Mac: 100–200 ms; traced on a
/// macOS 26.6.2 clone: 400–470 ms after it was made, 100–150 ms after the loop's first
/// turn) — and not while the main thread is kept: a start-up that held it for 1.1 s found
/// the mushroom still on its way in at the loop's first turn, at rest 30–45 ms later.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(1000);
/// How often a held popover looks again.
const SETTLE_POLL: std::time::Duration = std::time::Duration::from_millis(25);
/// When the event loop first turned with the mushroom up.
static FIRST_TURN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
/// A popover whose opening waits for a just-made mushroom to settle: whether it takes the
/// keyboard once shown (main thread only).
static HELD: Mutex<Option<bool>> = Mutex::new(None);
/// When the opening that's being held first waited (kept across its polls).
static HELD_SINCE: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// Where the last popover that opened was put, and how long its opening waited for a
/// just-made mushroom to be laid out (the launch drive compares it with the panel and the
/// mushroom on screen).
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Placed {
    pub x: f64,
    pub y: f64,
    pub under: bool,
    pub held_ms: u64,
}
static PLACED: Mutex<Option<Placed>> = Mutex::new(None);

fn tray_young() -> bool {
    young(FIRST_TURN.get().map(|t| t.elapsed()))
}

/// Whether the mushroom is still just-made: until the event loop has turned with it up —
/// however long the start-up took to get there — and for `SETTLE` after that.
pub fn young(since_first_turn: Option<std::time::Duration>) -> bool {
    since_first_turn.is_none_or(|d| d < SETTLE)
}

/// Whether a popover opening now waits for the mushroom: only while a just-made status
/// item hasn't come to rest (no reading, a zero-height frame, not yet `Visible`, not yet
/// in the menu bar). An older one that still isn't seen is hidden for real — behind the
/// notch — and the popover goes to the fallback corner at once (N41).
pub fn holds(young: bool, m: Option<Mushroom>) -> bool {
    young && !m.is_some_and(|m| m.visible && m.frame.h > 0.0 && m.screen.w > 0.0 && rests(&m))
}

/// The mushroom is in the menu bar: its top edge on its screen's top edge, which is where
/// macOS rests a status item whatever the menu bar's height (a notch's, an external
/// display's). On its way in it is lower — parked under the screen, or a few points
/// short — or higher.
/// A just-made one can read at its screen's very left edge before it is laid out, already
/// marked as seen: not at rest (N57).
fn rests(m: &Mushroom) -> bool {
    (m.frame.y + m.frame.h - m.top).abs() <= 0.5 && m.frame.x > m.screen.x + 0.5
}

/// Which of the screens a click on the mushroom came from (N56), each with its whole and
/// visible frames. With “Displays have separate Spaces” every display has its own menu bar
/// and mushroom: a pointer in a screen's menu bar, above its visible frame, clicked that
/// one. Otherwise (a launch you did, from the Dock or Spotlight) the screen the mushroom
/// rests on, else the pointer's.
pub fn clicked_screen(pointer: (f64, f64), item: Option<Frame>, screens: &[(Frame, Frame)]) -> Option<usize> {
    let (x, y) = pointer;
    let wholes: Vec<Frame> = screens.iter().map(|s| s.0).collect();
    let under = wholes.iter().position(|s| s.contains(x, y));
    under
        .filter(|&i| y >= screens[i].1.y + screens[i].1.h)
        .or_else(|| item.and_then(|f| home(f, &wholes)))
        .or(under)
}

/// The display a click on the mushroom came from, by its display id (N56; main thread).
pub fn clicked_display(mtm: objc2_foundation::MainThreadMarker) -> Option<u32> {
    use objc2_app_kit::{NSEvent, NSScreen};
    let screens = NSScreen::screens(mtm);
    let both: Vec<(Frame, Frame)> = screens.iter().map(|s| (Frame::of(s.frame()), Frame::of(s.visibleFrame()))).collect();
    let item = APP.get().and_then(|a| a.tray_by_id("shroom")).and_then(|t| {
        t.with_inner_tray_icon(|inner| {
            let mtm = objc2_foundation::MainThreadMarker::new()?;
            Some(Frame::of(inner.ns_status_item()?.button(mtm)?.window()?.frame()))
        })
        .ok()
        .flatten()
    });
    let p = NSEvent::mouseLocation();
    let i = clicked_screen((p.x, p.y), item, &both)?;
    probe::screen_id(&screens.objectAtIndex(i as _))
}

/// The visible frame of the screen holding the pointer (the main screen if none does).
fn pointer_screen(mtm: objc2_foundation::MainThreadMarker) -> Frame {
    use objc2_app_kit::{NSEvent, NSScreen};
    let p = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let under = screens.iter().find(|s| Frame::of(s.frame()).contains(p.x, p.y));
    under.or_else(|| NSScreen::mainScreen(mtm)).map(|s| Frame::of(s.visibleFrame())).unwrap_or(Frame { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 })
}

/// Opens the popover's panel: placed from the mushroom as it is now, then shown (with the
/// keyboard when asked). While a just-made mushroom is still being laid out (a launch that
/// opens a popover in its first Scene), the opening waits for it — polled on the main
/// thread until it settles or `SETTLE` runs out — so the panel never shows in the
/// fallback corner first and is never left there once the mushroom can be read.
fn open_popover(app: &AppHandle, rt: &Runtime, activates: bool) {
    let Some(mtm) = objc2_foundation::MainThreadMarker::new() else { return };
    let Some(panel) = overlay::get(Overlay::Popover) else { return };
    let read = app.tray_by_id("shroom").and_then(|t| reading(&t));
    // Parked over a hidden menu bar: hung from where it rests, as a click's would be (B161).
    let m = read.and_then(|(_, r)| r.parked).or(read.map(|(m, _)| m));
    if holds(tray_young(), m) {
        let mut since = HELD_SINCE.lock().unwrap();
        if since.is_none() {
            *since = Some(std::time::Instant::now());
            rt.log(&held_line(read.as_ref()));
        }
        drop(since);
        if HELD.lock().unwrap().replace(activates).is_none() {
            let (app, rt) = (app.clone(), rt.clone());
            std::thread::spawn(move || {
                std::thread::sleep(SETTLE_POLL);
                let app2 = app.clone();
                let _ = app.run_on_main_thread(move || {
                    // Still wanted (the Scene can close it meanwhile): look again.
                    let held = HELD.lock().unwrap().take();
                    if let Some(a) = held {
                        open_popover(&app2, &rt, a);
                    }
                });
            });
        }
        return;
    }
    *HELD.lock().unwrap() = None;
    let held_ms = HELD_SINCE.lock().unwrap().take().map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
    let (x, y, under) = popover_corner(m, pointer_screen(mtm), POPOVER_WIDTH);
    panel.setFrameTopLeftPoint(objc2_foundation::NSPoint::new(x, y));
    *PLACED.lock().unwrap() = Some(Placed { x, y, under, held_ms });
    rt.log(&placement_line(x, y, under, held_ms, read.as_ref()));
    overlay::show(&panel, activates);
}

/// The popover's page says how tall it is: the panel takes that height, its top edge
/// staying where it hangs.
pub fn fit_popover(app: &AppHandle, height: f64) {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    let _ = app.run_on_main_thread(move || {
        let Some(panel) = overlay::get(Overlay::Popover) else { return };
        let f = panel.frame();
        let h = height.clamp(120.0, 720.0);
        let top = f.origin.y + f.size.height;
        panel.setFrame_display(NSRect::new(NSPoint::new(f.origin.x, top - h), NSSize::new(POPOVER_WIDTH, h)), true);
        // The shadow follows the card's new outline, not the last one's.
        panel.invalidateShadow();
    });
}

/// The popover's panel on screen: its top-left corner (x, y) and whether it shows.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub type PanelCorner = (f64, f64, bool);

/// What the launch drive reads (main thread): where the last popover was put, the
/// popover's panel as it is on screen (its top-left corner and whether it shows), and the
/// mushroom as it is now.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn placement(app: &AppHandle) -> (Option<Placed>, Option<PanelCorner>, Option<Mushroom>) {
    let panel = overlay::get(Overlay::Popover).map(|ns| {
        let f = Frame::of(ns.frame());
        (f.x, f.y + f.h, ns.isVisible())
    });
    (*PLACED.lock().unwrap(), panel, app.tray_by_id("shroom").and_then(|t| mushroom(&t)))
}

/// The mushroom's size in the menu bar, in points: the bitmaps are drawn at twice it.
const FACE: f64 = 18.0;

/// A face's image. The plain mushroom and the pause bars are template images, which macOS
/// tints for whatever the menu bar is. A colored dot can't be a template (it would be
/// tinted too), so a dotted face has two bitmaps — the mushroom dark for a light bar,
/// light for a dark one — and draws the one that fits the appearance it is drawn in: the
/// status item's own, which is the menu bar's as it looks (dark over a dark wallpaper in
/// Light mode, Increase Contrast, a switch while it shows). Never the app's appearance:
/// that is the system's mode, and over a dark bar in Light mode it would draw the dark
/// mushroom, unseen (DECISIONS B151).
pub fn face_image(face: Face) -> Retained<NSImage> {
    let bitmap = |bytes: &'static [u8]| {
        let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes)).expect("a bundled tray icon");
        image.setSize(NSSize::new(FACE, FACE));
        image
    };
    let template = |bytes: &'static [u8]| {
        let image = bitmap(bytes);
        image.setTemplate(true);
        image
    };
    let (light, dark): (&'static [u8], &'static [u8]) = match face {
        Face::Plain => return template(include_bytes!("../../icons/tray/plain-template@2x.png")),
        Face::PauseBars => return template(include_bytes!("../../icons/tray/paused-template@2x.png")),
        Face::Red => (include_bytes!("../../icons/tray/recording-light@2x.png"), include_bytes!("../../icons/tray/recording-dark@2x.png")),
        Face::Amber => (include_bytes!("../../icons/tray/attention-light@2x.png"), include_bytes!("../../icons/tray/attention-dark@2x.png")),
        Face::Frost => (include_bytes!("../../icons/tray/update-light@2x.png"), include_bytes!("../../icons/tray/update-dark@2x.png")),
    };
    let (light, dark) = (bitmap(light), bitmap(dark));
    let draw = block2::RcBlock::new(move |rect: NSRect| {
        if drawn_dark() { dark.drawInRect(rect) } else { light.drawInRect(rect) }
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(FACE, FACE), false, &draw)
}

/// Whether what is being drawn now sits on a dark appearance (the high-contrast and
/// vibrant ones count by their family).
fn drawn_dark() -> bool {
    use objc2_app_kit::{NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua};
    let names = unsafe { NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]) };
    NSAppearance::currentDrawingAppearance().bestMatchFromAppearancesWithNames(&names).is_some_and(|n| &*n == unsafe { NSAppearanceNameDarkAqua })
}

/// Puts a face on the mushroom (main thread).
fn paint(t: &TrayIcon, face: Face) {
    let _ = t.with_inner_tray_icon(move |inner| {
        let mtm = objc2_foundation::MainThreadMarker::new()?;
        inner.ns_status_item()?.button(mtm)?.setImage(Some(&face_image(face)));
        Some(())
    });
}

fn tray(app: &AppHandle, rt: &Runtime) -> Option<TrayIcon> {
    if let Some(t) = app.tray_by_id("shroom") {
        return Some(t);
    }
    let rt2 = rt.clone();
    let rt3 = rt.clone();
    // Made with the plain face's bitmap (tray-icon lays the item out by it); `paint`
    // puts the face's own image on at once.
    TrayIconBuilder::with_id("shroom")
        .icon(Image::from_bytes(include_bytes!("../../icons/tray/plain-template@2x.png")).expect("a bundled tray icon"))
        .icon_as_template(true)
        .tooltip("shroom")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |tray, event| {
            let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, .. } = event else { return };
            // A right click is the one-item Quit menu's while it exists (idle); mid-take
            // there's no menu, and it's a left click (N39).
            let right = match button {
                MouseButton::Left => false,
                MouseButton::Right if right_is_a_click(rt2.snapshot.lock().unwrap().as_ref()) => true,
                _ => return,
            };
            if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
                let (quick, _us) = probe::quick(mtm, &rt2.home.root);
                // The click is the door (law 5): an onboarding window already open
                // comes forward, asked — the core moves its entry and re-probes.
                let opens_window = !right && rt2.snapshot.lock().unwrap().as_ref().is_some_and(|s| s.tray.click == Click::OpenOnboarding);
                rt2.post(Event::TrayClicked { right, quick });
                if opens_window {
                    bring_forward(tray.app_handle());
                }
            }
        })
        .on_menu_event(move |_app, event| {
            if event.id().as_ref() == "quit" {
                rt3.post(Event::QuitRequested(QuitVia::RightClick));
            }
        })
        .build(app)
        .ok()
        .inspect(|t| {
            paint(t, Face::Plain);
            // The settle wait counts from the loop's first turn, not from here: the rest
            // of the start-up runs on this thread first, and takes what it takes.
            super::observe::dispatch_main(|| {
                let _ = FIRST_TURN.set(std::time::Instant::now());
            });
        })
}

/// Whether the tray's right click goes to the core: only while it has no menu to show.
pub fn right_is_a_click(scene: Option<&Scene>) -> bool {
    scene.is_some_and(|s| s.tray.right_menu.is_none())
}

/// The popover was shown without the keyboard and has just become asked (a click on the
/// mushroom for a popover that opened by itself, the core's `ShowPaused`): now it takes it.
pub fn becomes_key(prev_activates: Option<bool>, activates: bool) -> bool {
    prev_activates == Some(false) && activates
}

/// What a Scene puts on screen that the keyboard can be in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Holds {
    /// One of shroom's windows: setup, the Library, Settings, Send feedback, the editor.
    pub window: bool,
    /// A popover the person asked for (an unasked one never takes the keyboard — law 5).
    pub asked_popover: bool,
    pub countdown: bool,
}

impl Holds {
    pub fn of(s: &Scene) -> Holds {
        Holds {
            window: s.onboarding.is_some() || s.library.is_some() || s.settings.is_some() || s.feedback.is_some() || !s.editors.is_empty(),
            asked_popover: s.popover.as_ref().is_some_and(|p| p.activates),
            countdown: s.countdown.is_some(),
        }
    }
}

/// The last of shroom's windows just closed, and nothing else of shroom's is there for
/// the keyboard: it goes back to the app the person was in. With its last window gone
/// shroom has no Dock icon (law 6), and macOS never takes the keyboard back from such an
/// app by itself — it would stay shroom's, with nothing on screen to type into (DECISIONS
/// B105).
pub fn gives_keyboard_back(before: Option<Holds>, now: Holds) -> bool {
    before.is_some_and(|b| b.window) && now == Holds::default()
}

/// Hands the keyboard to the app whose menus the menu bar shows: an app with no Dock icon
/// never owns the menu bar, so that is the app the person was in. macOS 14's own
/// hand-over — shroom yields to it by name, and it takes. Nothing happens when shroom
/// isn't the app holding the keyboard, or when the menu bar is still shroom's own (the
/// setup window's close: its Dock icon goes, and macOS hands the keyboard on by itself).
fn give_keyboard_back(rt: &Runtime) {
    use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
    let Some(mtm) = objc2_foundation::MainThreadMarker::new() else { return };
    let ns_app = NSApplication::sharedApplication(mtm);
    if !ns_app.isActive() {
        return;
    }
    let me = NSRunningApplication::currentApplication();
    let Some(other) = NSWorkspace::sharedWorkspace().menuBarOwningApplication().filter(|a| a.processIdentifier() != me.processIdentifier()) else { return };
    ns_app.yieldActivationToApplication(&other);
    let took = other.activateFromApplication_options(&me, NSApplicationActivationOptions(0)) || other.activateWithOptions(NSApplicationActivationOptions(0));
    rt.log(if took { "shell: the last window closed — the keyboard goes back to the app behind it" } else { "shell: the last window closed — the app behind it didn't take the keyboard" });
}

fn apply(app: &AppHandle, rt: &Runtime, prev: Option<&Scene>, s: &Scene, excluded: &Arc<Mutex<Vec<u32>>>) {
    // The Dock icon iff a window of shroom's is open (law 6, SM:133): the setup window, the
    // Library or Settings — an ordinary app's windows, which open on a desktop, never on
    // another app's full-screen Space; with none open, a menu-bar app.
    let _ = app.set_activation_policy(if s.dock { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory });

    // The mushroom.
    if let Some(t) = tray(app, rt) {
        if prev.is_none_or(|p| p.tray.face != s.tray.face) {
            paint(&t, s.tray.face);
        }
        if prev.is_none_or(|p| p.tray.right_menu != s.tray.right_menu) {
            let menu = s.tray.right_menu.as_ref().and_then(|label| {
                let item = MenuItem::with_id(app, "quit", label, true, None::<&str>).ok()?;
                Menu::with_items(app, &[&item]).ok()
            });
            let _ = t.set_menu(menu);
        }
    }
    // ⌥⌘P, held only while a take is open (N54).
    if prev.is_none_or(|p| p.tray.shortcut != s.tray.shortcut) {
        super::hotkey::hold(rt, s.tray.shortcut);
    }

    // The popover.
    match &s.popover {
        Some(p) => {
            // Its panel is left out of every capture from the moment it's made.
            if let Some(panel) = overlay::ensure(app, Overlay::Popover, excluded) {
                let _ = app.emit_to(POPOVER, "scene", p);
                let before = prev.and_then(|ps| ps.popover.as_ref()).map(|pp| pp.activates);
                let held = HELD.lock().unwrap().is_some();
                if before.is_none() || held {
                    // Placed from the mushroom as it is now — whatever opened it (a click,
                    // a launch, Reopen, a pause that wasn't yours, the right-click Quit).
                    // An opening still held for the mushroom takes this Scene's keyboard
                    // wish and keeps waiting.
                    open_popover(app, rt, p.activates);
                } else if becomes_key(before, p.activates) {
                    panel.makeKeyWindow();
                }
                // It follows the person across apps and Spaces; a click somewhere else
                // is what closes it (DEVIATIONS #39).
                let rt = rt.clone();
                overlay::watch(app, move |at| {
                    let asked = rt.snapshot.lock().unwrap().as_ref().and_then(|s| s.popover.as_ref()).map(|p| (p.activates, p.away_closes));
                    match asked.map(|(asked, away)| overlay::click_away(asked, away, at)) {
                        Some(overlay::Away::Close) => rt.post(Event::PopoverDismissed),
                        Some(overlay::Away::Yours) => rt.post(Event::PopoverKeyed),
                        _ => {}
                    }
                });
            }
        }
        None => {
            *HELD.lock().unwrap() = None;
            *HELD_SINCE.lock().unwrap() = None;
            overlay::unwatch();
            if let Some(panel) = overlay::get(Overlay::Popover) {
                overlay::hide(&panel);
            }
        }
    }

    // The windows: shown when they open; unasked, in front without the keyboard (law 5).
    let window = |kind: WindowKind, now: Option<(serde_json::Value, bool, String)>, before: Option<bool>| present_window(app, rt, kind, now, before, excluded);
    window(WindowKind::Onboarding, s.onboarding.as_ref().map(|o| (json(o), o.activates, "shroom".to_string())), prev.and_then(|p| p.onboarding.as_ref()).map(|o| o.activates));
    window(
        WindowKind::Library,
        s.library.as_ref().map(|l| (json(l), l.activates, l.body.title.clone())),
        prev.and_then(|p| p.library.as_ref()).map(|l| l.activates),
    );
    window(WindowKind::Settings, s.settings.as_ref().map(|w| (json(w), w.activates, "shroom".to_string())), prev.and_then(|p| p.settings.as_ref()).map(|w| w.activates));
    window(
        WindowKind::Feedback,
        s.feedback.as_ref().map(|f| (json(f), f.activates, f.body.title.clone())),
        prev.and_then(|p| p.feedback.as_ref()).map(|f| f.activates),
    );
    // Each editor window (N73): its title is its video's (“Edit — <title>”), changing with a
    // rename. One gone from the Scene goes for good.
    for (id, e) in &s.editors {
        window(WindowKind::Editor(*id), Some((json(e), e.activates, e.body.title.clone())), prev.and_then(|p| p.editors.get(id)).map(|e| e.activates));
    }
    for (id, e) in prev.map(|p| &p.editors).into_iter().flatten().filter(|(id, _)| !s.editors.contains_key(id)) {
        window(WindowKind::Editor(*id), None, Some(e.activates));
    }
    // The last of them closed: the keyboard goes back to the app the person was in.
    if gives_keyboard_back(prev.map(Holds::of), Holds::of(s)) {
        give_keyboard_back(rt);
    }

    // The countdown: a light scrim on the chosen display (N22), over whatever the person
    // is looking at there — a full-screen app too. The person asked for it (Start), so it
    // has the keyboard — Esc cancels — while their app stays in front, on its own Space:
    // when the count ends they are where they were (DEVIATIONS #40). Its panel is left out
    // of every capture from the moment it's made.
    match &s.countdown {
        Some(c) => {
            if let Some(panel) = overlay::ensure(app, Overlay::Countdown, excluded) {
                cover_display(&panel, c.display);
                let _ = app.emit_to(COUNTDOWN, "scene", c);
                if !panel.isVisible() {
                    overlay::show(&panel, true);
                }
            }
        }
        None => {
            if let Some(panel) = overlay::get(Overlay::Countdown) {
                overlay::hide(&panel);
            }
        }
    }

    // The agent label (N62, B216): top right of the display a take the command started
    // records, while it records — over other apps' windows, taking no clicks, never the
    // keyboard, and left out of every capture. Only the panel made as shroom started, never
    // one made late: the label is wanted only once a take records, and a panel made then
    // could miss the list of what that take leaves out. Without it, the tray still shows
    // the take.
    match &s.label {
        Some(l) => {
            if let Some(panel) = overlay::get(Overlay::Label) {
                place_label(&panel, l.display);
                let _ = app.emit_to(LABEL, "scene", l);
                if !panel.isVisible() {
                    overlay::show(&panel, false);
                    rt.log(&format!("label: shown on display {}", l.display));
                }
            } else if prev.is_none_or(|p| p.label.is_none()) {
                rt.log("label: no panel made at start, none shown (the tray shows the take)");
            }
        }
        None => {
            if let Some(panel) = overlay::get(Overlay::Label) {
                if panel.isVisible() {
                    rt.log("label: gone");
                }
                overlay::hide(&panel);
            }
        }
    }

    // The marker toast (N78): ⌥⌘M's answer, where the label sits — under it when it shows —
    // taking no clicks, never the keyboard, and left out of every capture. As the label,
    // only the panel made as shroom started; its page fades it in and out.
    match &s.toast {
        Some(t) => {
            if let Some(panel) = overlay::get(Overlay::Toast) {
                place_toast(&panel, t.display, t.under_label);
                let _ = app.emit_to(TOAST, "scene", t);
                if !panel.isVisible() {
                    overlay::show(&panel, false);
                    rt.log(&format!("toast: shown on display {}", t.display));
                }
            } else if prev.is_none_or(|p| p.toast.is_none()) {
                rt.log("toast: no panel made at start, none shown");
            }
        }
        None => {
            if let Some(panel) = overlay::get(Overlay::Toast) {
                if panel.isVisible() {
                    rt.log("toast: gone");
                }
                overlay::hide(&panel);
            }
        }
    }

    // The bubble (born at config, through the count and the take — N22) and Camera's
    // preview: native panels on the exact camera the Scene names (camera.rs).
    super::camera::present(rt, s.bubble.as_ref(), s.preview.as_ref());
}

fn json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// One window from its slice: `now` is (the slice, whether it was asked for, its title);
/// `before` is whether it showed in the previous Scene, and was asked for then.
fn present_window(app: &AppHandle, rt: &Runtime, kind: WindowKind, now: Option<(serde_json::Value, bool, String)>, before: Option<bool>, excluded: &Arc<Mutex<Vec<u32>>>) {
    let label = label(kind);
    match now {
        Some((slice, activates, title)) => {
            let Some(w) = app.get_webview_window(&label).or_else(|| build_window(app, rt, kind, excluded)) else { return };
            let _ = app.emit_to(label.as_ref(), "scene", &slice);
            if w.title().ok().as_deref() != Some(title.as_str()) {
                let _ = w.set_title(&title);
            }
            // Shown when it opens — not on every Scene: a window the person minimized (or
            // hid with shroom) stays where they put it until they reach for it.
            // It opens on the mushroom's display, the popover's (N57) — an editor window
            // down and to the right of the one opened before it (N73).
            if before.is_none() {
                to_mushrooms_display(app, &w);
                if let WindowKind::Editor(id) = kind {
                    cascade(app, &w, id);
                }
            }
            match before {
                None if activates => {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                None => order_front_quietly(&w),
                // It became asked (the person reached for it): now it may take the keyboard.
                Some(false) if activates => {
                    let _ = w.set_focus();
                }
                _ => {}
            }
        }
        None => {
            let Some(w) = app.get_webview_window(&label) else { return };
            // A window closed for good (N73, B229): its page is told first, so a video it
            // held stops, and its window leaves the list kept out of captures.
            let for_good = matches!(kind, WindowKind::Editor(_)) || (closes_for_good(kind) && !super::memory_as_before(rt));
            if for_good {
                let _ = app.emit_to(label.as_ref(), "scene", serde_json::Value::Null);
                remember_excluded(&w, excluded, false);
                let _ = w.destroy();
            } else {
                let _ = w.hide();
            }
        }
    }
}

/// Whether a window that closes is destroyed rather than hidden: the editors (N73), and the
/// Library and Settings, whose web pages hold about 44 MB and two processes while hidden
/// (B229). Each comes back only when the person reaches for it, so the page made again then
/// brings shroom forward only when asked (B106; the core's tests hold it). The setup window
/// and Send feedback are still hidden: rarely opened, they were left out of that measure.
pub fn closes_for_good(kind: WindowKind) -> bool {
    matches!(kind, WindowKind::Editor(_) | WindowKind::Library | WindowKind::Settings)
}

/// A new editor window goes down and to the right of the newest other editor window open,
/// as the Mac stacks documents (N73, B227 (c)), kept on that screen; with none open it
/// stays where it opened.
fn cascade(app: &AppHandle, w: &tauri::WebviewWindow, id: EditorId) {
    let windows = app.webview_windows();
    let newest = windows.iter().filter_map(|(l, w)| Some((editor_of(l)?, w))).filter(|(other, w)| *other != id && w.is_visible().unwrap_or(false)).max_by_key(|(other, _)| *other);
    let Some((_, before)) = newest else { return };
    let (Ok(b), Ok(n)) = (before.ns_window(), w.ns_window()) else { return };
    let before: &objc2_app_kit::NSWindow = unsafe { &*(b as *const objc2_app_kit::NSWindow) };
    let ns: &objc2_app_kit::NSWindow = unsafe { &*(n as *const objc2_app_kit::NSWindow) };
    // AppKit's own cascade: the point down and to the right of the window before's top
    // left, then this window's top left put there, kept on the screen.
    let next = before.cascadeTopLeftFromPoint(objc2_foundation::NSPoint::new(0.0, 0.0));
    ns.cascadeTopLeftFromPoint(next);
}

/// The person reached for shroom (a tray click, a launch): an onboarding window already
/// open comes back — un-minimized, shown, with the keyboard. A window that isn't open
/// yet is the Scene's to open.
pub fn bring_forward(app: &AppHandle) {
    bring_forward_kind(app, WindowKind::Onboarding);
}

/// A window the Scene shows comes forward with the keyboard (the person reached for it:
/// the tray, a launch, the popover footer's door to a window already open).
pub fn bring_forward_kind(app: &AppHandle, kind: WindowKind) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(w) = app2.get_webview_window(&label(kind)) else { return };
        let Some(rt) = app2.try_state::<Runtime>() else { return };
        let open = rt.snapshot.lock().unwrap().as_ref().is_some_and(|s| match kind {
            WindowKind::Onboarding => s.onboarding.is_some(),
            WindowKind::Library => s.library.is_some(),
            WindowKind::Settings => s.settings.is_some(),
            WindowKind::Feedback => s.feedback.is_some(),
            WindowKind::Editor(id) => s.editors.contains_key(&id),
        });
        if !open {
            return;
        }
        if w.is_minimized().unwrap_or(false) {
            let _ = w.unminimize();
        }
        let _ = w.show();
        let _ = w.set_focus();
    });
}

/// One of shroom's windows, built hidden — and left out of every screen capture from the
/// moment it exists (ARCHITECTURE §4.4: every shroom window but the bubble is excluded): a
/// Settings window showing Members, or a link just born, never lands in a video.
fn build_window(app: &AppHandle, rt: &Runtime, kind: WindowKind, excluded: &Arc<Mutex<Vec<u32>>>) -> Option<tauri::WebviewWindow> {
    let sp = spec(kind);
    let (width, height) = opening_size(sp, mushrooms_screen(app));
    let mut b = WebviewWindowBuilder::new(app, label(kind), WebviewUrl::App(sp.url.into()))
        .title("shroom")
        .inner_size(width, height)
        .resizable(sp.resizable)
        .maximizable(sp.resizable)
        .center()
        .visible(false)
        .focused(false);
    if let Some((w, h)) = sp.min {
        b = b.min_inner_size(w, h);
    }
    let w = b.build().ok()?;
    remember_excluded(&w, excluded, true);
    let rt = rt.clone();
    w.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            // The ✕ is an event like any click; the Scene hides the window.
            api.prevent_close();
            rt.post(Event::WindowClosed(kind));
        }
        WindowEvent::Focused(true) => {
            // Every entry re-probes (law 1).
            if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
                let (quick, _) = probe::quick(mtm, &rt.home.root);
                rt.post(Event::WindowFocused(kind, quick));
            }
        }
        _ => {}
    });
    Some(w)
}

/// The visible frame of the mushroom's display (main thread), where a window opens (N57).
fn mushrooms_screen(app: &AppHandle) -> Option<Frame> {
    app.tray_by_id("shroom").and_then(|t| mushroom(&t)).map(|m| m.screen).filter(|f| f.w > 0.0 && f.h > 0.0)
}

/// The size a window opens at: its spec's, made to fit `screen` (the visible frame it opens
/// on) when it's resizable and bigger than that — never below its smallest. A fixed window
/// keeps its size.
fn opening_size(sp: &WindowSpec, screen: Option<Frame>) -> (f64, f64) {
    fit(sp.width, sp.height, sp.resizable.then_some(sp.min.unwrap_or((0.0, 0.0))), screen)
}

/// `(width, height)` within `screen`, no smaller than `min`; `min: None` never shrinks.
pub fn fit(width: f64, height: f64, min: Option<(f64, f64)>, screen: Option<Frame>) -> (f64, f64) {
    match (min, screen) {
        (Some((mw, mh)), Some(s)) => (width.min(s.w).max(mw), height.min(s.h).max(mh)),
        _ => (width, height),
    }
}

/// Where a window opening now goes (its frame's origin): centred on `target`, the visible
/// frame of the mushroom's display — the one the popover hangs on (N57). `None` when it is
/// already on that display (it stays where the person left it) or there's none to go to.
pub fn window_origin(window: Frame, target: Frame) -> Option<(f64, f64)> {
    let (cx, cy) = (window.x + window.w / 2.0, window.y + window.h / 2.0);
    if target.w <= 0.0 || target.h <= 0.0 || target.contains(cx, cy) {
        return None;
    }
    let x = target.x + ((target.w - window.w) / 2.0).max(0.0);
    let top = target.y + target.h - ((target.h - window.h) / 2.0).max(0.0);
    Some((x, top - window.h))
}

/// Moves a window that is opening to the mushroom's display (N57; main thread).
fn to_mushrooms_display(app: &AppHandle, w: &tauri::WebviewWindow) {
    let Some(target) = app.tray_by_id("shroom").and_then(|t| mushroom(&t)).map(|m| m.screen) else { return };
    let Ok(ptr) = w.ns_window() else { return };
    let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
    if let Some((x, y)) = window_origin(Frame::of(ns.frame()), target) {
        ns.setFrameOrigin(objc2_foundation::NSPoint::new(x, y));
    }
}

/// Shows `w` in front without making it key or activating shroom (law 5).
fn order_front_quietly(w: &tauri::WebviewWindow) {
    let Ok(ptr) = w.ns_window() else { return };
    let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
    ns.orderFrontRegardless();
}

/// Where a display sits in AppKit's coordinates (origin bottom-left of the main display,
/// y up), from its CoreGraphics bounds (origin top-left of the main display, y down).
pub fn display_frame(bounds: (f64, f64, f64, f64), main_h: f64) -> Frame {
    let (x, y, w, h) = bounds;
    Frame { x, y: main_h - (y + h), w, h }
}

/// Places `panel` over the whole of the display with CoreGraphics id `display`.
fn cover_display(panel: &objc2_app_kit::NSPanel, display: u32) {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    let b = probe::display_bounds(display);
    let f = display_frame((b.x, b.y, b.w, b.h), probe::display_bounds(shroom_capture::main_display_id()).h);
    panel.setFrame_display(NSRect::new(NSPoint::new(f.x, f.y), NSSize::new(f.w, f.h)), true);
}

/// Places the agent label at the top right of the display with CoreGraphics id `display`,
/// under its menu bar: the top of that screen's visible frame, where AppKit has it.
pub fn place_label(panel: &objc2_app_kit::NSWindow, display: u32) {
    set_frame(panel, label_place(display));
}

/// The agent label's frame on display `display`, in AppKit's coordinates.
fn label_place(display: u32) -> Frame {
    let b = probe::display_bounds(display);
    let screen = display_frame((b.x, b.y, b.w, b.h), probe::display_bounds(shroom_capture::main_display_id()).h);
    let top = objc2_foundation::MainThreadMarker::new()
        .and_then(|mtm| objc2_app_kit::NSScreen::screens(mtm).iter().find(|s| probe::screen_id(s) == Some(display)).map(|s| s.visibleFrame()))
        .map_or(screen.y + screen.h, |v| v.origin.y + v.size.height);
    let (w, h) = overlay::LABEL_SIZE;
    label_frame(screen, top, w, h)
}

fn set_frame(panel: &objc2_app_kit::NSWindow, f: Frame) {
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    panel.setFrame_display(NSRect::new(NSPoint::new(f.x, f.y), NSSize::new(f.w, f.h)), true);
}

/// Places the marker toast (N78) where the agent label goes on display `display` — or just
/// under the label, when it shows there. One move, and none when it is already there.
pub fn place_toast(panel: &objc2_app_kit::NSWindow, display: u32, under_label: bool) {
    let f = toast_frame(label_place(display), under_label);
    let at = panel.frame();
    if (at.origin.x, at.origin.y, at.size.width, at.size.height) != (f.x, f.y, f.w, f.h) {
        set_frame(panel, f);
    }
}

/// The toast's panel from the label's (`label`, as `label_frame` gives it): its pill's top
/// and right edge where the label's pill has them, or under the label's pill, 6 points below
/// it — and the panel around the pill by its margin. The label's pill is 26 points of its 30,
/// centred.
pub fn toast_frame(label: Frame, under_label: bool) -> Frame {
    let (w, h) = overlay::TOAST_SIZE;
    let (right, above, _) = overlay::TOAST_MARGIN;
    let (pill_right, pill_top, pill_bottom) = (label.x + label.w, label.y + label.h - 2.0, label.y + 2.0);
    let top = if under_label { pill_bottom - 6.0 } else { pill_top } + above;
    Frame { x: pill_right + right - w, y: top - h, w, h }
}

/// Where the agent label goes (AppKit's coordinates, y up): its right edge 12 points in
/// from the display's, its top 8 under `top` — the menu bar's foot.
pub fn label_frame(screen: Frame, top: f64, w: f64, h: f64) -> Frame {
    const MARGIN: f64 = 12.0;
    const GAP: f64 = 8.0;
    Frame { x: screen.x + screen.w - w - MARGIN, y: top - GAP - h, w, h }
}

/// Keeps shroom's own UI out of every frame (the countdown scrim, the popover, the
/// windows); `on: false` as a window goes for good (an editor's, N73), so its number,
/// should macOS give it again, keeps nothing else out.
fn remember_excluded(w: &tauri::WebviewWindow, excluded: &Arc<Mutex<Vec<u32>>>, on: bool) {
    let Ok(ptr) = w.ns_window() else { return };
    let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
    if on {
        exclude(ns, excluded);
    } else {
        let n = ns.windowNumber() as u32;
        excluded.lock().unwrap().retain(|x| *x != n);
    }
}

fn exclude(ns: &objc2_app_kit::NSWindow, excluded: &Arc<Mutex<Vec<u32>>>) {
    let n = ns.windowNumber() as u32;
    let mut ex = excluded.lock().unwrap();
    if !ex.contains(&n) {
        ex.push(n);
    }
}

/// An overlay's window as AppKit has it, once it has been made (main thread) — what the
/// surfaces drive reads its facts from.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn overlay_window(_app: &AppHandle, which: Overlay) -> Option<objc2::rc::Retained<objc2_app_kit::NSWindow>> {
    use objc2::Message;
    let panel = overlay::get(which)?;
    let ns: &objc2_app_kit::NSWindow = &panel;
    Some(ns.retain())
}

/// The mushroom's own window in the menu bar (main thread): where a click on it lands.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn mushroom_window(app: &AppHandle) -> Option<objc2::rc::Retained<objc2_app_kit::NSWindow>> {
    let mtm = objc2_foundation::MainThreadMarker::new()?;
    let number = app
        .tray_by_id("shroom")?
        .with_inner_tray_icon(|inner| {
            let mtm = objc2_foundation::MainThreadMarker::new()?;
            Some(inner.ns_status_item()?.button(mtm)?.window()?.windowNumber())
        })
        .ok()
        .flatten()?;
    objc2_app_kit::NSApplication::sharedApplication(mtm).windowWithWindowNumber(number)
}

/// Shows the popover the way one that opened by itself is shown — in front, without the
/// keyboard (law 5) — with no Scene behind it: the surfaces drive reads what macOS made of
/// that, and the next Scene puts it away again (main thread).
#[cfg(feature = "dev-harness")]
pub fn show_popover_unasked(app: &AppHandle) {
    let rt = app.state::<Runtime>();
    open_popover(app, &rt, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::world::{OpenTake, Phase, RecId, SourceId, TakeDir, World};

    const SCREEN: Frame = Frame { x: 0.0, y: 0.0, w: 1512.0, h: 944.0 };

    /// Every page shroom shows is in the capability its calls go through: a window left out
    /// can't listen for its `scene` slice (the event plugin refuses it), and its page stays
    /// blank — the agent label's did, until it was listed. The editor windows, one per video
    /// (N73), are listed by their labels' pattern, `editor-*`.
    #[test]
    fn every_page_may_hear_its_scene() {
        let cap: serde_json::Value = serde_json::from_str(include_str!("../../capabilities/default.json")).unwrap();
        let listed: Vec<&str> = cap["windows"].as_array().unwrap().iter().filter_map(|w| w.as_str()).collect();
        let covers = |page: &str| listed.iter().any(|l| *l == page || l.strip_suffix('*').is_some_and(|stem| page.starts_with(stem)));
        let kinds = [WindowKind::Onboarding, WindowKind::Library, WindowKind::Settings, WindowKind::Feedback, WindowKind::Editor(EditorId(1)), WindowKind::Editor(EditorId(27))];
        let pages = kinds.map(label).into_iter().chain([Overlay::Popover.label(), Overlay::Countdown.label(), Overlay::Label.label(), Overlay::Toast.label()].map(Into::into));
        for page in pages {
            assert!(covers(&page), "the {page} window is not in capabilities/default.json");
        }
        assert!(listed.contains(&"editor-*") && !listed.contains(&"editor"), "the editor windows, by their labels' pattern");
        assert_eq!((POPOVER, COUNTDOWN, LABEL, TOAST), (Overlay::Popover.label(), Overlay::Countdown.label(), Overlay::Label.label(), Overlay::Toast.label()));
    }

    /// The pages may listen for their slice and stop listening (`app/src/lib/ipc.ts`), and
    /// nothing more of Tauri's own: no default set, no emitting, no tray or menu — the app's
    /// commands are its own. A debug build's devtools key is the one other (Tauri's hotkey).
    #[test]
    fn the_pages_may_only_listen() {
        let cap: serde_json::Value = serde_json::from_str(include_str!("../../capabilities/default.json")).unwrap();
        let granted: Vec<&str> = cap["permissions"].as_array().unwrap().iter().map(|p| p.as_str().expect("a permission by its name")).collect();
        assert_eq!(granted, ["core:event:allow-listen", "core:event:allow-unlisten", "core:webview:allow-internal-toggle-devtools"]);
        for p in &granted {
            assert!(!p.ends_with(":default") && *p != "core:default", "a default set: {p}");
            assert!(!p.contains("emit") && !p.contains("tray") && !p.contains("menu"), "{p}");
        }
    }

    /// An editor window's label carries its number (N73): one made, one read back, and no
    /// other label reads as an editor window's.
    #[test]
    fn an_editor_windows_label_carries_its_number() {
        assert_eq!(label(WindowKind::Editor(EditorId(1))), "editor-1");
        assert_eq!(editor_of(&label(WindowKind::Editor(EditorId(4_000_000)))), Some(EditorId(4_000_000)));
        for not_one in [EDITOR, "editor-", "editor-0", "editor-01", "editor-1a", "editor--1", "library", "feedback"] {
            assert_eq!(editor_of(not_one), None, "{not_one}");
        }
    }

    #[test]
    fn the_popover_hangs_under_the_mushroom_or_falls_back_to_the_top_right() {
        let item = Frame { x: 1200.0, y: 950.0, w: 24.0, h: 32.0 };
        let seen = Mushroom { frame: item, visible: true, screen: SCREEN, top: 982.0 };
        let (x, y, under) = popover_corner(Some(seen), SCREEN, 320.0);
        assert!(under);
        assert_eq!((x, y), (1212.0 - 160.0, 944.0), "centred under it, a small gap below");
        // Near the screen's edge it stays on the screen.
        let edge = Mushroom { frame: Frame { x: 1490.0, ..item }, ..seen };
        assert_eq!(popover_corner(Some(edge), SCREEN, 320.0).0, 1512.0 - 320.0 - 8.0);
        // Hidden behind the notch (or never read): the top-right of the pointer's screen (N41).
        let other = Frame { x: 1512.0, y: 0.0, w: 2560.0, h: 1415.0 };
        assert_eq!(popover_corner(Some(Mushroom { visible: false, ..seen }), other, 320.0), (1512.0 + 2560.0 - 328.0, 1409.0, false));
        assert_eq!(popover_corner(None, SCREEN, 320.0), (1512.0 - 328.0, 938.0, false));
    }

    /// A reopen from Spotlight read the occlusion as not seen with the mushroom in plain
    /// view (Bagrat's checkpoint, 2026-10-14): where it rests decides then.
    #[test]
    fn the_mushroom_is_seen_where_it_rests_whatever_the_occlusion_says() {
        // A 14-inch MacBook Pro: 1512 wide, its menu bar 38 tall, the notch 185 wide in the
        // middle — 663.5 of menu bar left of it, 663.5 right.
        let whole = Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 };
        let notch = Some((663.5, 663.5));
        let at = |x: f64| Frame { x, y: 944.0, w: 24.0, h: 38.0 };
        assert!(seen(true, false, at(1200.0), whole, notch, false), "macOS says seen: seen");
        assert!(seen(false, false, at(1200.0), whole, notch, false), "right of the notch, at rest: seen");
        assert!(seen(false, false, at(100.0), whole, notch, false), "left of it, past the app's menus: seen");
        assert!(!seen(false, false, at(700.0), whole, notch, false), "behind the notch: hidden (N41)");
        assert!(!seen(false, false, at(840.0), whole, notch, false), "half behind it: hidden");
        assert!(!seen(false, true, at(1200.0), whole, notch, false), "behind the lock: hidden");
        assert!(!seen(false, false, Frame { y: 982.0, ..at(1200.0) }, whole, notch, false), "above a hidden menu bar: hidden");
        assert!(!seen(false, false, Frame { y: -15.0, ..at(1200.0) }, whole, notch, false), "parked under the screen: hidden");
        assert!(!seen(false, false, at(1200.0), Frame { w: 0.0, ..whole }, notch, false), "on no screen: hidden");
        // A display without a notch, to the right of the main one: anywhere across it.
        let other = Frame { x: 1512.0, y: -200.0, w: 2560.0, h: 1440.0 };
        assert!(seen(false, false, Frame { x: 3000.0, y: 1210.0, w: 24.0, h: 30.0 }, other, None, false));
        assert!(!seen(false, false, Frame { x: 1500.0, y: 1210.0, w: 24.0, h: 30.0 }, other, None, false), "off its edge: hidden");
    }

    /// Bagrat's update relaunch, 2026-10-17 (N57): 359 ms after boot, the mushroom read on
    /// the MacBook (left of the primary external display) at its screen's very left edge,
    /// occlusion not visible, and the popover hung from there, at the screen's top-left
    /// corner. A status item never rests at a screen's left edge: it isn't laid out yet.
    #[test]
    fn a_mushroom_at_its_screens_left_edge_isnt_laid_out_yet() {
        let whole = Frame { x: -1512.0, y: 98.0, w: 1512.0, h: 982.0 };
        let usable = Frame { h: 944.0, ..whole };
        let notch = Some((663.0, 664.0));
        let early = Frame { x: -1512.0, y: 1047.0, w: 34.0, h: 33.0 };
        let later = Frame { x: -616.0, ..early };
        assert!(!seen(false, false, early, whole, notch, false), "at the screen's left edge: not laid out");
        assert!(seen(false, false, later, whole, notch, false), "where it settled 14 s later: seen");
        let m = |frame: Frame, visible: bool| Mushroom { frame, visible, screen: usable, top: 1080.0 };
        assert!(holds(true, Some(m(early, true))), "a just-made one waits there, even if macOS calls it seen");
        assert!(!holds(true, Some(m(later, true))));
    }

    /// Bagrat, 2026-10-17 (N57): Settings and the Library, opened from the popover on the
    /// MacBook, came up on the external display. A window opens centred on the mushroom's
    /// display; one already there stays where it was left.
    #[test]
    fn a_window_opens_on_the_mushrooms_display() {
        let external = Frame { x: 0.0, y: 0.0, w: 1920.0, h: 1055.0 };
        let macbook = Frame { x: -1512.0, y: 98.0, w: 1512.0, h: 944.0 };
        let window = Frame { x: 600.0, y: 300.0, w: 720.0, h: 480.0 };
        assert_eq!(window_origin(window, macbook), Some((-1512.0 + 396.0, 98.0 + 232.0)), "centred on the MacBook");
        assert_eq!(window_origin(window, external), None, "already on the mushroom's display: left where it is");
        assert_eq!(window_origin(window, Frame { w: 0.0, ..macbook }), None, "no display to go to: left where it is");
        let bigger = Frame { w: 2000.0, h: 1200.0, ..window };
        assert_eq!(window_origin(bigger, macbook), Some((-1512.0, 98.0 + 944.0 - 1200.0)), "too big for it: its top-left on the display's");
    }

    /// The editor (N63) opens at the board's 1100×720 where that fits, and no bigger than
    /// the screen it opens on — never below 900×640; a fixed window keeps its size.
    #[test]
    fn a_resizable_window_opens_within_its_screen() {
        let roomy = Frame { x: 0.0, y: 0.0, w: 1512.0, h: 944.0 };
        let small = Frame { x: 0.0, y: 0.0, w: 1024.0, h: 743.0 };
        let tiny = Frame { x: 0.0, y: 0.0, w: 800.0, h: 600.0 };
        assert_eq!(opening_size(&EDITOR_WINDOW, Some(roomy)), (1100.0, 720.0));
        assert_eq!(opening_size(&EDITOR_WINDOW, Some(small)), (1024.0, 720.0), "the hosted Mac's screen");
        assert_eq!(opening_size(&EDITOR_WINDOW, Some(tiny)), (900.0, 640.0), "never below its smallest");
        assert_eq!(opening_size(&EDITOR_WINDOW, None), (1100.0, 720.0), "no screen read: the spec's");
        assert_eq!(opening_size(&SETTINGS_WINDOW, Some(tiny)), (600.0, 720.0), "a fixed window keeps its size");
        assert_eq!(opening_size(&LIBRARY_WINDOW, Some(Frame { w: 600.0, h: 500.0, ..tiny })), (600.0, 500.0));
    }

    /// A MacBook mirrored to an external display put the popover in the corner with the
    /// mushroom in plain view (the 0.1.12 checkpoint, 2026-10-15: “Still top right”).
    #[test]
    fn a_mirrored_screen_has_no_notch_to_hide_behind() {
        let whole = Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 };
        let notch = Some((663.5, 663.5));
        let at = |x: f64| Frame { x, y: 944.0, w: 24.0, h: 38.0 };
        assert!(seen(false, false, at(700.0), whole, notch, true), "where the notch would be: the mirror shows it");
        assert!(!seen(false, false, at(700.0), whole, notch, false), "unmirrored, behind the notch: hidden (N41)");
        assert!(!seen(false, true, at(700.0), whole, notch, true), "behind the lock: hidden, mirrored or not");
        assert!(!seen(false, false, Frame { y: 982.0, ..at(1200.0) }, whole, notch, true), "above a hidden menu bar: hidden");
        // Mirrored at the external display's size: a 1920×1080 set, a 24-point menu bar.
        let set = Frame { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0 };
        assert!(seen(false, false, Frame { x: 1500.0, y: 1056.0, w: 24.0, h: 24.0 }, set, None, true));
        assert!(seen(false, false, Frame { x: 1500.0, y: 1050.0, w: 24.0, h: 24.0 }, set, None, true), "a few points short of the top: still in the menu bar");
        assert!(!seen(false, false, Frame { x: 1500.0, y: 1000.0, w: 24.0, h: 24.0 }, set, None, true), "under the menu bar: not resting there");
        assert!(!seen(false, false, Frame { x: 1500.0, y: 1056.0, w: 0.0, h: 24.0 }, set, None, true), "a zero-width frame: not laid out");
    }

    #[test]
    fn a_click_comes_from_the_menu_bar_the_pointer_is_in() {
        // The MacBook, and an external display to its right, each with its own menu bar
        // (“Displays have separate Spaces”): Bagrat's desk, 2026-10-17 (N56).
        let built_in = (Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 }, Frame { x: 0.0, y: 0.0, w: 1512.0, h: 944.0 });
        let external = (Frame { x: 1512.0, y: -200.0, w: 2560.0, h: 1440.0 }, Frame { x: 1512.0, y: -200.0, w: 2560.0, h: 1415.0 });
        let screens = [built_in, external];
        let on_external = Some(Frame { x: 3000.0, y: 1210.0, w: 24.0, h: 30.0 });
        assert_eq!(clicked_screen((1210.0, 970.0), on_external, &screens), Some(0), "the pointer in the MacBook's menu bar: the MacBook");
        assert_eq!(clicked_screen((3010.0, 1230.0), Some(Frame { x: 1200.0, y: 944.0, w: 24.0, h: 38.0 }), &screens), Some(1), "and the external one's");
        assert_eq!(clicked_screen((700.0, 400.0), on_external, &screens), Some(1), "a launch, the pointer on a desktop: where the mushroom rests");
        assert_eq!(clicked_screen((700.0, 400.0), None, &screens), Some(0), "and with no mushroom to read, the pointer's screen");
        assert_eq!(clicked_screen((-50.0, 400.0), None, &screens), None, "off every screen: can't tell");
    }

    #[test]
    fn the_mushrooms_screen_is_the_one_its_frame_is_on() {
        let built_in = Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 };
        let external = Frame { x: 1512.0, y: -200.0, w: 2560.0, h: 1440.0 };
        let screens = [built_in, external];
        assert_eq!(home(Frame { x: 1200.0, y: 944.0, w: 24.0, h: 38.0 }, &screens), Some(0));
        assert_eq!(home(Frame { x: 3000.0, y: 1210.0, w: 24.0, h: 30.0 }, &screens), Some(1));
        assert_eq!(home(Frame { x: 1200.0, y: 982.0, w: 24.0, h: 38.0 }, &screens), None, "above a hidden menu bar: on none");
    }

    #[test]
    fn each_placement_says_what_decided_it() {
        let m = Mushroom { frame: Frame { x: 700.0, y: 944.0, w: 24.0, h: 38.0 }, visible: true, screen: SCREEN, top: 982.0 };
        let r = Reading { occlusion: false, locked: false, whole: Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 }, notch: Some((663.5, 663.5)), mirrored: true, parked: None };
        assert_eq!(
            placement_line(552.0, 938.0, true, 0, Some(&(m, r))),
            "popover: placed under the mushroom at 552,938 (held 0 ms) — item 700,944 24x38 screen 0,0 1512x982 notch 663.5|663.5 mirrored true occlusion not visible locked false seen true"
        );
        assert_eq!(placement_line(1184.0, 938.0, false, 0, None), "popover: placed in the corner (N41) at 1184,938 (held 0 ms) — mushroom not read");
        assert_eq!(held_line(None), "popover: held for the mushroom — mushroom not read");
        let rest = Mushroom { frame: Frame { y: 944.0, ..m.frame }, ..m };
        let parked = (Mushroom { frame: Frame { y: 1014.0, ..m.frame }, visible: false, ..m }, Reading { occlusion: false, parked: Some(rest), ..r });
        assert_eq!(
            placement_line(552.0, 938.0, true, 0, Some(&parked)),
            "popover: placed under the mushroom at 552,938 (held 0 ms) — item 700,1014 24x38 screen 0,0 1512x982 notch 663.5|663.5 mirrored true occlusion not visible locked false seen false parked over a hidden menu bar, rests at 700,944 24x38"
        );
    }

    #[test]
    fn a_launchs_popover_waits_for_a_just_made_mushroom_to_settle() {
        let item = Frame { x: 1200.0, y: 950.0, w: 24.0, h: 32.0 };
        let seen = Mushroom { frame: item, visible: true, screen: SCREEN, top: 982.0 };
        // Just made: no reading yet, or AppKit's first one (a zero-height frame, no
        // Visible bit, no screen) — wait.
        assert!(holds(true, None));
        let unlaid = Mushroom { frame: Frame { x: 0.0, y: 0.0, w: 27.0, h: 0.0 }, visible: false, screen: Frame { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, top: 0.0 };
        assert!(holds(true, Some(unlaid)));
        assert!(holds(true, Some(Mushroom { frame: Frame { h: 0.0, ..item }, ..seen })), "Visible, but not laid out yet");
        // Laid out: under it at once.
        assert!(!holds(true, Some(seen)));
        // Settled and still not seen (behind the notch), or never read: the fallback
        // corner at once (N41) — no waiting on a tray that's been up a while.
        assert!(!holds(false, Some(Mushroom { visible: false, ..seen })));
        assert!(!holds(false, None));
        assert!(!holds(false, Some(unlaid)));
    }

    /// Where a just-made mushroom was while macOS brought it in (a throwaway macOS 26.6.2
    /// clone, a 1024×768 screen whose menu bar is 30 pt): parked under the screen's bottom
    /// edge, already marked as seen and already on its screen; then short of the menu bar.
    /// Only at rest in the menu bar has it settled — a popover hung from any earlier frame
    /// stays where that frame was.
    #[test]
    fn a_just_made_mushroom_has_settled_only_once_it_rests_in_the_menu_bar() {
        let screen = Frame { x: 0.0, y: 58.0, w: 1024.0, h: 680.0 };
        let at = |x: f64, y: f64| Mushroom { frame: Frame { x, y, w: 34.0, h: 30.0 }, visible: true, screen, top: 768.0 };
        // Parked (read as such): seen, sized, on its screen — 15 pt under its bottom edge.
        assert!(holds(true, Some(at(0.0, -15.0))), "the popover went to (8, -21)");
        // Short of the menu bar by 15 pt (where a popover placed at y 717 found it), and by
        // the 4 pt the popover lane's reviewer measured.
        assert!(holds(true, Some(at(765.0, 723.0))), "the popover hung 15 pt low");
        assert!(holds(true, Some(at(765.0, 734.0))), "the popover hung 4 pt low");
        // Some launches bring it down from above instead: 15 pt over its resting place,
        // seen, on its screen, its foot above the menu bar's lower edge.
        assert!(holds(true, Some(at(765.0, 753.0))), "the popover hung 15 pt high");
        // At rest: its top edge on its screen's.
        assert!(!holds(true, Some(at(765.0, 738.0))));
        // A taller menu bar (a notch), the mushroom shorter than it: still top to top.
        let notch = Mushroom { frame: Frame { x: 1200.0, y: 950.0, w: 24.0, h: 32.0 }, visible: true, screen: SCREEN, top: 982.0 };
        assert!(!holds(true, Some(notch)));
        // One that has been up a while is taken as it reads (N41).
        assert!(!holds(false, Some(at(765.0, 723.0))));
    }

    /// Bagrat's 0.1.13 take on a mirrored 1920×1080 set (2026-10-15): the popover he
    /// opened read the mushroom at rest and hung under it; the one shroom opened by itself
    /// when his microphone went read it on no screen, 62 pt over the top — parked over a
    /// hidden menu bar — and went to the corner. It hangs where a click's would now
    /// (DECISIONS B161).
    #[test]
    fn a_mushroom_parked_over_a_hidden_menu_bar_is_hung_from_where_it_rests() {
        let set = Frame { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0 };
        // The set's visible frame, under its 30 pt menu bar (the corner it fell to: 1592,1044).
        let usable = Frame { x: 0.0, y: 0.0, w: 1920.0, h: 1050.0 };
        let screens = [(set, usable)];
        let clicked = Mushroom { frame: Frame { x: 1300.0, y: 1050.0, w: 34.0, h: 30.0 }, visible: true, screen: usable, top: 1080.0 };
        let paused = Frame { x: 1300.0, y: 1112.0, w: 34.0, h: 30.0 };
        assert_eq!(popover_corner(Some(clicked), usable, 320.0), (1157.0, 1044.0, true), "the click's, as the log has it");
        let rest = parked(paused, &screens, false);
        assert_eq!(rest, Some(clicked), "the paused one rests where the click's did");
        assert_eq!(popover_corner(rest, usable, 320.0), (1157.0, 1044.0, true));
        // The probe on a hosted Mac (dev/mac/scenarios/tray-moves.sh, 1024×768, a 30 pt
        // bar): the menu bar set to hide, or TextEdit full screen, parks it on the top edge.
        let small = Frame { x: 0.0, y: 0.0, w: 1024.0, h: 768.0 };
        let hidden = (small, small);
        assert_eq!(parked(Frame { x: 756.0, y: 768.0, w: 35.0, h: 30.0 }, &[hidden], false).map(|m| m.frame), Some(Frame { x: 756.0, y: 738.0, w: 35.0, h: 30.0 }));
        // Not parked: in the menu bar (home has it), on its way out (no height), off every
        // screen's span, far over it, or behind the lock.
        assert_eq!(parked(clicked.frame, &screens, false), None);
        assert_eq!(parked(Frame { h: 0.0, ..paused }, &screens, false), None);
        assert_eq!(parked(Frame { x: 1024.0, y: 768.0, w: 35.0, h: 30.0 }, &[hidden], false), None);
        assert_eq!(parked(Frame { y: 1300.0, ..paused }, &screens, false), None);
        assert_eq!(parked(paused, &screens, true), None);
    }

    /// A start-up can keep the main thread for longer than the wait itself (1.1 s, traced):
    /// the mushroom is just-made until the loop has turned with it up, and for the wait
    /// after that.
    #[test]
    fn the_settle_wait_counts_from_the_event_loops_first_turn() {
        let ms = std::time::Duration::from_millis;
        assert!(young(None), "the loop hasn't turned yet, however long ago the mushroom was made");
        assert!(young(Some(ms(0))));
        assert!(young(Some(ms(999))));
        assert!(!young(Some(ms(1000))));
    }

    #[test]
    fn the_count_covers_its_display_wherever_it_sits() {
        // The main display: the whole of it.
        assert_eq!(display_frame((0.0, 0.0, 1512.0, 982.0), 982.0), Frame { x: 0.0, y: 0.0, w: 1512.0, h: 982.0 });
        // A taller display to the right, its top 200 above the main one's.
        assert_eq!(display_frame((1512.0, -200.0, 2560.0, 1440.0), 982.0), Frame { x: 1512.0, y: 982.0 - 1240.0, w: 2560.0, h: 1440.0 });
        // One below the main display.
        assert_eq!(display_frame((0.0, 982.0, 1920.0, 1080.0), 982.0), Frame { x: 0.0, y: -1080.0, w: 1920.0, h: 1080.0 });
    }

    #[test]
    fn the_label_sits_top_right_under_the_menu_bar() {
        // The main display, its menu bar 24 points tall: 12 in from its right, 8 under the bar.
        let main = display_frame((0.0, 0.0, 1440.0, 900.0), 900.0);
        assert_eq!(label_frame(main, 876.0, 380.0, 30.0), Frame { x: 1048.0, y: 838.0, w: 380.0, h: 30.0 });
        // A display to the right of it: on that one, under its own menu bar.
        let right = display_frame((1440.0, 0.0, 1920.0, 1080.0), 900.0);
        let f = label_frame(right, right.y + right.h - 24.0, 380.0, 30.0);
        assert_eq!((f.x + f.w, f.y + f.h), (1440.0 + 1920.0 - 12.0, right.y + right.h - 24.0 - 8.0));
    }

    /// The marker toast (N78) sits where the label does — its pill's top and right edge the
    /// label pill's; with the label up, under it, the pills 6 points apart. Its panel stands
    /// its margin off the pill.
    #[test]
    fn the_toast_sits_where_the_label_does_or_under_it() {
        let main = display_frame((0.0, 0.0, 1440.0, 900.0), 900.0);
        let label = label_frame(main, 876.0, 380.0, 30.0);
        let (right, above, below) = overlay::TOAST_MARGIN;
        let pill = |f: Frame| (f.x + f.w - right, f.y + f.h - above, f.y + f.h - above - overlay::TOAST_PILL_H);
        let (label_right, label_top, label_bottom) = (label.x + label.w, label.y + label.h - 2.0, label.y + 2.0);
        let at = toast_frame(label, false);
        assert_eq!((pill(at).0, pill(at).1), (label_right, label_top));
        assert_eq!((at.w, at.h), overlay::TOAST_SIZE);
        assert!(pill(at).2 - at.y >= below, "room under the pill for its shadow");
        let under = toast_frame(label, true);
        assert_eq!(pill(under).0, label_right, "right edges lined up");
        assert_eq!(label_bottom - pill(under).1, 6.0);
        // On the main display, 1440 wide under a 24-point menu bar: the pill 12 in from the right.
        assert_eq!(pill(at).0, 1440.0 - 12.0);
    }

    #[test]
    fn the_library_settings_and_editors_close_for_good() {
        for kind in [WindowKind::Library, WindowKind::Settings, WindowKind::Editor(EditorId(1))] {
            assert!(closes_for_good(kind), "{kind:?} is destroyed when it closes (B229, N73)");
        }
        for kind in [WindowKind::Onboarding, WindowKind::Feedback] {
            assert!(!closes_for_good(kind), "{kind:?} is hidden");
        }
    }

    #[test]
    fn an_unasked_popover_takes_the_keyboard_only_once_asked() {
        assert!(becomes_key(Some(false), true), "the click that asked for it");
        assert!(!becomes_key(Some(false), false), "still unasked (law 5)");
        assert!(!becomes_key(Some(true), true), "already key");
        assert!(!becomes_key(None, true), "a popover opening is shown with the keyboard, not made key later");
    }

    #[test]
    fn the_keyboard_goes_back_when_the_last_window_closes() {
        let nothing = Holds::default();
        let window = Holds { window: true, ..nothing };
        assert!(gives_keyboard_back(Some(window), nothing), "the Library's ✕ or ⌘W, Settings closed: nothing of shroom's is left to type into");
        assert!(!gives_keyboard_back(Some(window), window), "one window of two closed, or setup handed over to Settings");
        assert!(!gives_keyboard_back(Some(window), Holds { asked_popover: true, ..nothing }), "the popover the person opened has the keyboard");
        assert!(!gives_keyboard_back(Some(window), Holds { countdown: true, ..nothing }), "the count has it");
        assert!(!gives_keyboard_back(Some(nothing), nothing), "nothing closed");
        assert!(!gives_keyboard_back(Some(Holds { asked_popover: true, ..nothing }), nothing), "a popover going never made shroom the app in front");
        assert!(!gives_keyboard_back(None, nothing), "the first Scene");
        // Read from Scenes: an idle shroom holds nothing; an open setup window is a window.
        assert_eq!(Holds::of(&shroom_core::derive(&World::blank())), nothing);
        let mut w = World::blank();
        w.session.windows.insert(WindowKind::Onboarding, shroom_core::world::Door::asked(shroom_core::time::Mono(0)));
        assert_eq!(Holds::of(&shroom_core::derive(&w)), window);
    }

    #[test]
    fn a_right_click_is_a_click_only_while_the_tray_has_no_menu() {
        let idle = shroom_core::derive(&World::blank());
        assert!(!right_is_a_click(Some(&idle)), "idle: the one-item Quit menu");
        let mut w = World::blank();
        w.take = Some(OpenTake {
            dir: TakeDir("t".into()),
            rec: RecId("AbCdEfGhIjKlMnOp".into()),
            opened: shroom_core::time::Mono(0),
            source: SourceId::Display(1),
            phase: Phase::Recording,
            since: shroom_core::time::Mono(0),
            length_ms: 0,
            markers: 0,
            marked_at_pause: false,
            toast: None,
            mic: None,
            camera: None,
        });
        assert!(right_is_a_click(Some(&shroom_core::derive(&w))), "mid-take: a left click");
        assert!(!right_is_a_click(None));
    }
}
