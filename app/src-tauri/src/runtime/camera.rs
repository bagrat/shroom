//! shroom's camera windows (DECISIONS B47, superseding B20's webview): the camera
//! bubble and Camera-as-the-source's preview, each a native non-activating panel showing a
//! layer shroom draws the camera's frames into (`shroom_capture::CameraPreview`), from the
//! exact camera the Scene names — never WebKit's default
//! camera, which can be an iPhone over Continuity (N39).
//!
//! - **The bubble** (N22): round, mirrored, 180 pt; borderless and non-activating, so a click
//!   never makes it key or activates shroom (law 5); dragged anywhere by its picture
//!   (`movableByWindowBackground`), and where the person puts it is posted to the core
//!   (`BubbleMoved`, an offset from its display's top-left) — the core remembers it in
//!   `prefs.bubble_at`; it sits on the display being recorded and moves with the choice. It
//!   is recorded as-is: never excluded from capture.
//! - **The preview**: Camera as the source, at `config` and through the count — the picture
//!   the take records, unmirrored, 400 × 225 pt near the bottom of its display (the count
//!   plays on that display). It is shroom UI, but it's never on screen during a screen take.
//!
//! Each shows at once, dark with a spinner, while its camera starts on the camera thread
//! (`CameraPreview::run`): the popover that opens with the bubble never waits for it.
//!
//! Main thread only: the panels live in a thread-local.

use std::cell::RefCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSBackingStoreType, NSPanel, NSView, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize, NSString};
use shroom_core::event::Event;
use shroom_core::scene::{Bubble, Preview};

use super::Runtime;
use super::probe::display_bounds;

/// The bubble's side, in points.
pub const BUBBLE_SIZE: f64 = 180.0;
/// The bubble's default place: this far from its display's left and bottom edges.
const BUBBLE_MARGIN: f64 = 32.0;
const PREVIEW_W: f64 = 400.0;
const PREVIEW_H: f64 = 225.0;
/// `NSFloatingWindowLevel`.
const FLOATING: isize = 3;
/// A regular spinning `NSProgressIndicator`'s side, in points.
const SPINNER: f64 = 32.0;

struct Native {
    panel: Retained<NSPanel>,
    preview: shroom_capture::CameraPreview,
    /// The display it sits on, and the place it was put at (offset from that display).
    display: u32,
    at: (i32, i32),
    _moved: Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

thread_local! {
    static BUBBLE: RefCell<Option<Native>> = const { RefCell::new(None) };
    static PREVIEW: RefCell<Option<Native>> = const { RefCell::new(None) };
    /// The presenter is moving the bubble itself: that move isn't the person's.
    static PLACING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Where a panel of `size` goes on `display`: `at` (an offset from its top-left), kept on
/// the display — in AppKit's coordinates (origin bottom-left of the main display, y up).
/// Also returns the offset it ended at.
pub fn place(display: (f64, f64, f64, f64), main_h: f64, at: (i32, i32), size: (f64, f64)) -> (NSPoint, (i32, i32)) {
    let (bx, by, bw, bh) = display;
    let dx = (at.0 as f64).clamp(0.0, (bw - size.0).max(0.0));
    let dy = (at.1 as f64).clamp(0.0, (bh - size.1).max(0.0));
    (NSPoint::new(bx + dx, main_h - (by + dy + size.1)), (dx as i32, dy as i32))
}

/// The bubble's default corner on a display of this size: bottom-left, inside the margin.
pub fn default_bubble_at(display_h: f64) -> (i32, i32) {
    (BUBBLE_MARGIN as i32, (display_h - BUBBLE_SIZE - BUBBLE_MARGIN * 2.0) as i32)
}

/// Where the preview sits on a display of this size: centred, near the bottom.
pub fn preview_at(display_w: f64, display_h: f64) -> (i32, i32) {
    (((display_w - PREVIEW_W) / 2.0) as i32, (display_h - PREVIEW_H - 120.0) as i32)
}

fn bounds(display: u32) -> (f64, f64, f64, f64) {
    let b = display_bounds(display);
    (b.x, b.y, b.w, b.h)
}

fn main_h() -> f64 {
    display_bounds(shroom_capture::main_display_id()).h
}

/// The bubble's offset from its display's top-left, from where its window is now.
fn offset_of(panel: &NSPanel, display: u32) -> (i32, i32) {
    let f = panel.frame();
    let (bx, by, _, _) = bounds(display);
    let top = main_h() - (f.origin.y + f.size.height);
    ((f.origin.x - bx).round() as i32, (top - by).round() as i32)
}

/// The spinner a panel shows until its camera delivers. Carried to the camera thread and
/// back, it's only ever touched on the main thread.
struct Spinner(Retained<NSView>);
// SAFETY: made on the main thread, and `stop` hands it back there before anything touches
// or releases it.
unsafe impl Send for Spinner {}

impl Spinner {
    fn stop(self) {
        super::observe::dispatch_main(move || {
            let s = self;
            // SAFETY: a plain AppKit message to the spinner, on the main thread.
            unsafe {
                let _: () = msg_send![&*s.0, stopAnimation: std::ptr::null::<AnyObject>()];
            }
            s.0.setHidden(true);
        });
    }
}

/// Starts `preview`'s camera; once it delivers, `spinner` goes. A picture that stops later
/// restarts its camera (`shroom_capture::watchdog`), logged.
fn run(rt: &Runtime, what: &'static str, preview: &shroom_capture::CameraPreview, spinner: Spinner) {
    let (rt, rt2) = (rt.clone(), rt.clone());
    preview.run(
        move |running| {
            if !running {
                rt.log(&format!("{what}: the camera didn't start"));
            }
            spinner.stop();
        },
        move |n| rt2.log(&format!("{what}: the camera's picture stopped; restarting it ({n})")),
    );
}

fn panel(mtm: MainThreadMarker, frame: NSRect, preview: &shroom_capture::CameraPreview, round: bool) -> (Retained<NSPanel>, Spinner) {
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let p = NSPanel::initWithContentRect_styleMask_backing_defer(NSPanel::alloc(mtm), frame, style, NSBackingStoreType::Buffered, false);
    // SAFETY: the panel is kept in a thread-local until it's closed; it must not free itself.
    unsafe { p.setReleasedWhenClosed(false) };
    p.setLevel(FLOATING);
    p.setOpaque(false);
    p.setHasShadow(true);
    p.setHidesOnDeactivate(false);
    // It stays when shroom's windows are hidden (⌘H) too: the bubble is in the picture a
    // take records, and the Scene never hears of a hide.
    p.setCanHide(false);
    p.setMovableByWindowBackground(true);
    p.setBecomesKeyOnlyIfNeeded(true);
    p.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary);
    let bounds = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
    let content = NSView::initWithFrame(NSView::alloc(mtm), bounds);
    content.setWantsLayer(true);
    // The picture fills the panel; the spinner sits over it, in the middle.
    let picture = NSView::initWithFrame(NSView::alloc(mtm), bounds);
    picture.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
    let at = NSPoint::new((frame.size.width - SPINNER) / 2.0, (frame.size.height - SPINNER) / 2.0);
    // SAFETY: plain AppKit/QuartzCore messages on objects owned here; the layer is the
    // preview's CALayer, kept alive by `preview`.
    let spinner: Retained<NSView> = unsafe {
        let class = |name: &std::ffi::CStr| AnyClass::get(name).expect("an AppKit class");
        let clear: *mut AnyObject = msg_send![class(c"NSColor"), clearColor];
        let _: () = msg_send![&*p, setBackgroundColor: clear];
        let layer = preview.layer() as *mut AnyObject;
        // A layer-hosting view: the layer first, then wantsLayer.
        let _: () = msg_send![&*picture, setLayer: layer];
        picture.setWantsLayer(true);
        let radius = if round { frame.size.width / 2.0 } else { 10.0 };
        let _: () = msg_send![layer, setCornerRadius: radius];
        let _: () = msg_send![layer, setMasksToBounds: true];
        // Dark until the camera's first picture, so the panel shows while it starts.
        let dark: *mut AnyObject = msg_send![class(c"NSColor"), colorWithWhite: 0.12f64, alpha: 1.0f64];
        let dark: *const std::ffi::c_void = msg_send![dark, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: dark];
        // NSProgressIndicatorStyleSpinning, light on the dark.
        let spinner: Retained<NSView> = msg_send![class(c"NSProgressIndicator"), new];
        spinner.setFrame(NSRect::new(at, NSSize::new(SPINNER, SPINNER)));
        let _: () = msg_send![&*spinner, setStyle: 1usize];
        let _: () = msg_send![&*spinner, setIndeterminate: true];
        let _: () = msg_send![&*spinner, setDisplayedWhenStopped: false];
        let dark_aqua = NSString::from_str("NSAppearanceNameDarkAqua");
        let appearance: *mut AnyObject = msg_send![class(c"NSAppearance"), appearanceNamed: &*dark_aqua];
        let _: () = msg_send![&*spinner, setAppearance: appearance];
        let _: () = msg_send![&*spinner, startAnimation: std::ptr::null::<AnyObject>()];
        spinner
    };
    content.addSubview(&picture);
    content.addSubview(&spinner);
    p.setContentView(Some(&content));
    (p, Spinner(spinner))
}

/// Shows, moves or removes the bubble and the preview to match the Scene.
pub fn present(rt: &Runtime, bubble: Option<&Bubble>, preview: Option<&Preview>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    present_bubble(mtm, rt, bubble);
    present_preview(mtm, rt, preview);
}

fn present_bubble(mtm: MainThreadMarker, rt: &Runtime, want: Option<&Bubble>) {
    BUBBLE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(b) = want else {
            // Its camera went, the take ended, the camera picker went to Camera off: it simply goes.
            if let Some(n) = slot.take() {
                n.panel.close();
            }
            return;
        };
        let (_, _, _, dh) = bounds(b.display);
        let at = b.at.unwrap_or_else(|| default_bubble_at(dh));
        if slot.as_ref().is_some_and(|n| n.preview.uid() != b.camera) {
            if let Some(n) = slot.take() {
                n.panel.close();
            }
        }
        match slot.as_mut() {
            Some(n) => {
                // Moved by the core (another display, or a place that isn't where it is).
                if n.display != b.display || (n.at != at && offset_of(&n.panel, n.display) != at) {
                    let (origin, placed) = place(bounds(b.display), main_h(), at, (BUBBLE_SIZE, BUBBLE_SIZE));
                    PLACING.with(|p| p.set(true));
                    n.panel.setFrameOrigin(origin);
                    PLACING.with(|p| p.set(false));
                    n.display = b.display;
                    n.at = placed;
                }
            }
            None => {
                let preview = match shroom_capture::CameraPreview::open(&b.camera, true) {
                    Ok(p) => p,
                    Err(e) => {
                        rt.log(&format!("bubble: the camera didn't open: {e}"));
                        return;
                    }
                };
                let (origin, placed) = place(bounds(b.display), main_h(), at, (BUBBLE_SIZE, BUBBLE_SIZE));
                let (p, spinner) = panel(mtm, NSRect::new(origin, NSSize::new(BUBBLE_SIZE, BUBBLE_SIZE)), &preview, true);
                p.orderFrontRegardless();
                run(rt, "bubble", &preview, spinner);
                let moved = watch_moves(rt, &p);
                rt.log(&format!("bubble: shown on display {} at {placed:?}", b.display));
                *slot = Some(Native { panel: p, preview, display: b.display, at: placed, _moved: Some(moved) });
            }
        }
    });
}

/// The person dragged the bubble: each move AppKit reports goes to the core as the
/// bubble's new place (the core writes only a changed one). The presenter's own moves
/// aren't the person's.
fn watch_moves(rt: &Runtime, p: &NSPanel) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let rt = rt.clone();
    let block = RcBlock::new(move |_n: NonNull<NSNotification>| {
        if PLACING.with(|p| p.get()) {
            return;
        }
        // AppKit posts the move synchronously, so this can run inside the presenter's own
        // turn (the slot busy): that move is the presenter's, never the person's.
        BUBBLE.with(|slot| {
            let Ok(mut slot) = slot.try_borrow_mut() else { return };
            if let Some(n) = slot.as_mut() {
                let at = offset_of(&n.panel, n.display);
                if at != n.at {
                    n.at = at;
                    rt.post(Event::BubbleMoved(at));
                }
            }
        });
    });
    let center = NSNotificationCenter::defaultCenter();
    let name = NSString::from_str("NSWindowDidMoveNotification");
    // SAFETY: the block is retained by the center; delivered on the main queue (the panel's
    // thread); the token lives as long as the panel.
    unsafe { center.addObserverForName_object_queue_usingBlock(Some(&name), Some(p.as_ref()), Some(&objc2_foundation::NSOperationQueue::mainQueue()), &block) }
}

fn present_preview(mtm: MainThreadMarker, rt: &Runtime, want: Option<&Preview>) {
    PREVIEW.with(|slot| {
        let mut slot = slot.borrow_mut();
        let keep = matches!((slot.as_ref(), want), (Some(n), Some(p)) if n.preview.uid() == p.camera && n.display == p.display);
        if keep {
            return;
        }
        if let Some(n) = slot.take() {
            n.panel.close();
        }
        let Some(p) = want else { return };
        let preview = match shroom_capture::CameraPreview::open(&p.camera, false) {
            Ok(x) => x,
            Err(e) => {
                rt.log(&format!("preview: the camera didn't open: {e}"));
                return;
            }
        };
        let (_, _, dw, dh) = bounds(p.display);
        let (origin, placed) = place(bounds(p.display), main_h(), preview_at(dw, dh), (PREVIEW_W, PREVIEW_H));
        let (panel, spinner) = panel(mtm, NSRect::new(origin, NSSize::new(PREVIEW_W, PREVIEW_H)), &preview, false);
        panel.orderFrontRegardless();
        run(rt, "preview", &preview, spinner);
        *slot = Some(Native { panel, preview, display: p.display, at: placed, _moved: None });
    });
}

/// What the harness reads of the bubble (main thread): where it is, whether it could ever
/// take the keyboard, and its camera.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Seen {
    pub window: u32,
    pub display: u32,
    pub at: (i32, i32),
    pub key: bool,
    pub can_become_key: bool,
    pub nonactivating: bool,
    pub camera: String,
    pub running: bool,
    /// The camera frames drawn into it so far.
    pub frames: u64,
    pub size: (f64, f64),
}

#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
fn seen(n: &Native) -> Seen {
    let f = n.panel.frame();
    Seen {
        window: n.panel.windowNumber() as u32,
        display: n.display,
        at: offset_of(&n.panel, n.display),
        key: n.panel.isKeyWindow(),
        can_become_key: n.panel.canBecomeKeyWindow(),
        nonactivating: n.panel.styleMask().contains(NSWindowStyleMask::NonactivatingPanel),
        camera: n.preview.uid().to_string(),
        running: n.preview.running(),
        frames: n.preview.frames(),
        size: (f.size.width, f.size.height),
    }
}

#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn bubble_seen() -> Option<Seen> {
    BUBBLE.with(|s| s.borrow().as_ref().map(seen))
}

#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn preview_seen() -> Option<Seen> {
    PREVIEW.with(|s| s.borrow().as_ref().map(seen))
}

/// The bubble's own window, while it shows (main thread). Camera's preview is the same
/// panel, made by the same function.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn bubble_window() -> Option<Retained<NSPanel>> {
    BUBBLE.with(|s| s.borrow().as_ref().map(|n| n.panel.clone()))
}

/// How the probe's bubble window differs from the real one's, to try other ways it could
/// ride Space switches.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProbeWindow {
    /// `NSWindowCollectionBehaviorStationary` on top of the real one's.
    pub stationary: bool,
    /// Another window level than `NSFloatingWindowLevel`.
    pub level: Option<isize>,
}

/// The harness's bubble probe, with no runtime: shows `b` (main thread) and returns once
/// it's on screen; `close_probe_bubble` reads it and closes it.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn show_probe_bubble(app: &tauri::AppHandle, b: Bubble, window: ProbeWindow) {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let shown = shroom_capture::CameraPreview::open(&b.camera, true).map(|preview| {
            let (_, _, _, dh) = bounds(b.display);
            let (origin, placed) = place(bounds(b.display), main_h(), default_bubble_at(dh), (BUBBLE_SIZE, BUBBLE_SIZE));
            let (p, spinner) = panel(mtm, NSRect::new(origin, NSSize::new(BUBBLE_SIZE, BUBBLE_SIZE)), &preview, true);
            if window.stationary {
                p.setCollectionBehavior(p.collectionBehavior() | NSWindowCollectionBehavior::Stationary);
            }
            if let Some(level) = window.level {
                p.setLevel(level);
            }
            p.orderFrontRegardless();
            preview.run(move |_| spinner.stop(), |_| {});
            Native { panel: p, preview, display: b.display, at: placed, _moved: None }
        });
        if let Ok(n) = shown {
            BUBBLE.with(|s| *s.borrow_mut() = Some(n));
        }
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
}

/// What's on screen of the probe's bubble now (any thread).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn read_probe_bubble(app: &tauri::AppHandle) -> Option<Seen> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(bubble_seen());
    });
    rx.recv_timeout(std::time::Duration::from_secs(5)).ok().flatten()
}

/// Reads the probe's bubble and closes it (any thread).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn close_probe_bubble(app: &tauri::AppHandle) -> Option<Seen> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let seen = BUBBLE.with(|s| s.borrow_mut().take()).map(|n| {
            let out = seen(&n);
            n.panel.close();
            out
        });
        let _ = tx.send(seen);
    });
    rx.recv_timeout(std::time::Duration::from_secs(5)).ok().flatten()
}

/// Moves the bubble the way a drag does — the window moves; the person's place is what
/// AppKit reports (the harness's drag, main thread).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn drag_bubble_by(dx: f64, dy: f64) {
    // The slot is let go before the move: AppKit reports it at once, to `watch_moves`.
    let panel = BUBBLE.with(|s| s.borrow().as_ref().map(|n| n.panel.clone()));
    if let Some(p) = panel {
        let f = p.frame();
        p.setFrameOrigin(NSPoint::new(f.origin.x + dx, f.origin.y - dy));
    }
}

/// Moves Camera's preview the way a drag does (the harness's drag, main thread).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn drag_preview_by(dx: f64, dy: f64) {
    let panel = PREVIEW.with(|s| s.borrow().as_ref().map(|n| n.panel.clone()));
    if let Some(p) = panel {
        let f = p.frame();
        p.setFrameOrigin(NSPoint::new(f.origin.x + dx, f.origin.y - dy));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_is_kept_on_its_display_in_appkits_coordinates() {
        // The main display, 1512 × 982 points; a second one to its right, top-aligned.
        let main = (0.0, 0.0, 1512.0, 982.0);
        let right = (1512.0, 0.0, 2560.0, 1440.0);
        let (o, at) = place(main, 982.0, (32, 700), (180.0, 180.0));
        assert_eq!((o.x, o.y, at), (32.0, 982.0 - 880.0, (32, 700)));
        // Off the edge: kept on the display.
        let (o, at) = place(main, 982.0, (5000, -40), (180.0, 180.0));
        assert_eq!((o.x, o.y, at), (1512.0 - 180.0, 982.0 - 180.0, (1332, 0)));
        // The same place on another display follows it there.
        let (o, _) = place(right, 982.0, (32, 700), (180.0, 180.0));
        assert_eq!((o.x, o.y), (1544.0, 982.0 - 880.0));
        // The default corner is bottom-left, inside the margin.
        assert_eq!(default_bubble_at(982.0), (32, 982 - 180 - 64));
        assert_eq!(preview_at(1512.0, 982.0), (556, 982 - 225 - 120));
    }
}
