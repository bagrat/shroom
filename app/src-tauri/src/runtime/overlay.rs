//! shroom's page overlays as macOS panels (DEVIATIONS #40, DECISIONS B106).
//!
//! An overlay is a native panel **born** with the non-activating style, holding the page
//! Tauri made. Born with it, because the window server learns "a click here never brings
//! this window's app forward" only when a window is created: a window turned into a panel
//! afterwards (its class swapped, the style added later) keeps the tag of the ordinary
//! window it was born as. Such a panel can't take the keyboard while another app is in
//! front, and a click on it either does nothing but bring shroom forward (a first click
//! spent on focus) or — worse — moves the person off the Space they were on. A panel born
//! right can be given the keyboard while shroom stays in the background: the person's app
//! stays in front, on its own Space, full screen or not.
//!
//! The page is built the way every shroom page is — a `WebviewWindowBuilder`, so the
//! label, the `scene` events and the commands work unchanged — in a host window that never
//! shows; its content, the web view, is moved into the panel. The host is built with a
//! first click switched on: the page acts on a click that lands while its window hasn't
//! the keyboard.
//!
//! The overlays are made once, when shroom starts (`prepare`) — never when one is first
//! wanted. Making a web view asks macOS to bring its app forward (wry does, for every
//! one). At start-up that is nothing new: a launch asks the same. Later it would be the
//! very thing an overlay must never do — the person is in another app, perhaps full
//! screen — and the one way to stop it, switching shroom off as an app for that moment,
//! costs an app with a Dock icon every window it has on screen (`may_go_quiet`). So by the
//! time anything of shroom's shows, every panel exists, its page is loaded, and each is
//! already left out of every capture.
//!
//! An overlay follows the person, so losing the keyboard closes nothing (DEVIATIONS #39).
//! What closes the popover from outside is a click somewhere else: while one is open, the
//! mouse-downs macOS reports — in other apps, and in shroom's own windows — are watched
//! here and answered by `click_away`. (Mouse-downs are reported to any app; nothing is
//! asked of macOS for it.) A click that is itself the switch — on the Dock, or anywhere
//! while Mission Control is up — is not a click away (`switches`): where it landed is read
//! from the window server's own list of windows, which any app may read. That reading
//! can't tell Mission Control from Show Desktop or App Exposé, so while either of those is
//! up no click closes the popover either. The app switcher was never measured.

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{ClassType, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEvent, NSEventMask, NSPanel, NSRunningApplication, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

/// One of shroom's page overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Popover,
    Countdown,
    /// The agent label (N62, B216): top right of the display a take the command started
    /// records. It takes no clicks: they go to whatever is under it.
    Label,
    /// The marker toast (N78): where the label sits, under it when it shows. It takes no
    /// clicks either.
    Toast,
}

impl Overlay {
    pub const fn label(self) -> &'static str {
        match self {
            Overlay::Popover => "popover",
            Overlay::Countdown => "countdown",
            Overlay::Label => "label",
            Overlay::Toast => "toast",
        }
    }
    fn url(self) -> &'static str {
        match self {
            Overlay::Popover => "popover.html",
            Overlay::Countdown => "countdown.html",
            Overlay::Label => "label.html",
            Overlay::Toast => "toast.html",
        }
    }
    /// Above other apps' windows: the countdown, the label and the toast at
    /// `NSFloatingWindowLevel`, with the bubble; the popover one above them.
    fn level(self) -> isize {
        match self {
            Overlay::Popover => 4,
            Overlay::Countdown | Overlay::Label | Overlay::Toast => 3,
        }
    }
    /// macOS's shadow, cast by what the page draws: the popover's card, the label's pill. The
    /// toast's page draws its own, inside the panel's margin, so it fades with the pill —
    /// macOS casts its shadow once, from what the panel held then (N78).
    fn shadow(self) -> bool {
        matches!(self, Overlay::Popover | Overlay::Label)
    }
    fn size(self) -> NSSize {
        match self {
            Overlay::Popover => NSSize::new(320.0, 360.0),
            Overlay::Countdown => NSSize::new(640.0, 400.0),
            Overlay::Label => NSSize::new(LABEL_SIZE.0, LABEL_SIZE.1),
            Overlay::Toast => NSSize::new(TOAST_SIZE.0, TOAST_SIZE.1),
        }
    }
}

define_class!(
    // SAFETY: a plain NSPanel subclass — no state of its own, no `Drop`; made and used on
    // the main thread only.
    #[unsafe(super = NSPanel)]
    #[thread_kind = MainThreadOnly]
    #[name = "ShroomOverlayPanel"]
    struct OverlayPanel;

    impl OverlayPanel {
        /// A borderless panel refuses the keyboard unless it says otherwise.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// The label's panel, in points: room for its line, the pill drawn at its right edge.
pub const LABEL_SIZE: (f64, f64) = (380.0, 30.0);
/// The marker toast's panel, in points: its pill (`TOAST_PILL_H` tall) at the top right,
/// inside a margin its own shadow fits in (`TOAST_MARGIN`: right, top, bottom), and room to
/// its left for the longest words with their hint.
pub const TOAST_SIZE: (f64, f64) = (470.0, 62.0);
pub const TOAST_PILL_H: f64 = 30.0;
pub const TOAST_MARGIN: (f64, f64, f64) = (16.0, 12.0, 20.0);

thread_local! {
    static POPOVER: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
    static COUNTDOWN: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
    static LABEL: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
    static TOAST: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
}

fn slot<T>(which: Overlay, f: impl FnOnce(&RefCell<Option<Retained<NSPanel>>>) -> T) -> T {
    match which {
        Overlay::Popover => POPOVER.with(f),
        Overlay::Countdown => COUNTDOWN.with(f),
        Overlay::Label => LABEL.with(f),
        Overlay::Toast => TOAST.with(f),
    }
}

/// The overlay's panel, once it has been made (main thread).
pub fn get(which: Overlay) -> Option<Retained<NSPanel>> {
    slot(which, |s| s.borrow().clone())
}

/// When an overlay's panel is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Born {
    /// With shroom itself: nothing of its is on screen yet.
    AtStartUp,
    /// When first wanted — only if making it at start-up failed.
    Late,
}

/// Makes the overlays (main thread, once, as shroom starts — before it has put anything on
/// screen), each left out of every screen capture from the moment it exists
/// (ARCHITECTURE §4.4).
pub fn prepare(app: &AppHandle, excluded: &Arc<Mutex<Vec<u32>>>) {
    // The dev harness's `--late-panels` leaves them all to be made late — the path a failed
    // start-up takes — so a drive can walk it (`take_drive/surfaces.rs`).
    #[cfg(feature = "dev-harness")]
    if late_panels() {
        return;
    }
    for which in [Overlay::Popover, Overlay::Countdown, Overlay::Label, Overlay::Toast] {
        let _ = made(app, which, excluded, Born::AtStartUp);
    }
}

/// Whether this run was started with the dev harness's `--late-panels`.
#[cfg(feature = "dev-harness")]
pub fn late_panels() -> bool {
    std::env::args().any(|a| a == "--late-panels")
}

/// The overlay's panel (main thread): the one `prepare` made — or, had that failed, made
/// now, and left out of every screen capture from the moment it exists.
pub fn ensure(app: &AppHandle, which: Overlay, excluded: &Arc<Mutex<Vec<u32>>>) -> Option<Retained<NSPanel>> {
    made(app, which, excluded, Born::Late)
}

fn made(app: &AppHandle, which: Overlay, excluded: &Arc<Mutex<Vec<u32>>>, born: Born) -> Option<Retained<NSPanel>> {
    if let Some(p) = get(which) {
        return Some(p);
    }
    let panel = build(app, which, MainThreadMarker::new()?, born)?;
    let number = panel.windowNumber() as u32;
    let mut ex = excluded.lock().unwrap();
    if !ex.contains(&number) {
        ex.push(number);
    }
    slot(which, |s| *s.borrow_mut() = Some(panel.clone()));
    Some(panel)
}

/// Whether shroom may be switched off as an app for the moment an overlay's page is made
/// late. Unable to come forward for that moment, it stays where it is — behind the
/// person's app. But an app with a Dock icon that is switched off and on again while it
/// isn't in front loses every window it has on screen a moment later (macOS 26.6.2: the
/// Library gone, and the count with it — measured, `take_drive/surfaces.rs`). So: only
/// while shroom isn't in front already, and never with a Dock icon up.
pub fn may_go_quiet(active: bool, dock_icon: bool) -> bool {
    !active && !dock_icon
}

/// The web view under `view`, wherever it sits.
fn web_view(view: &NSView) -> Option<Retained<NSView>> {
    let wk = AnyClass::get(c"WKWebView")?;
    fn find(v: &NSView, wk: &AnyClass) -> Option<Retained<NSView>> {
        // SAFETY: `isKindOfClass:` on a live view.
        let is: bool = unsafe { msg_send![v, isKindOfClass: wk] };
        if is {
            return Some(v.retain());
        }
        v.subviews().iter().find_map(|s| find(&s, wk))
    }
    find(view, wk)
}

fn build(app: &AppHandle, which: Overlay, mtm: MainThreadMarker, born: Born) -> Option<Retained<NSPanel>> {
    // The page, in a host window that never shows. Making a web view asks macOS to bring
    // its app forward (wry does, for every one). At start-up that is what a launch asks
    // anyway, and nothing is touched. Late, shroom is switched off as an app while the host
    // is built — where that is safe (`may_go_quiet`).
    let ns_app = NSApplication::sharedApplication(mtm);
    let policy = ns_app.activationPolicy();
    let quiet = born == Born::Late && may_go_quiet(ns_app.isActive(), policy == NSApplicationActivationPolicy::Regular);
    if quiet {
        ns_app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    }
    let host = WebviewWindowBuilder::new(app, which.label(), WebviewUrl::App(which.url().into()))
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .accept_first_mouse(true)
        .build();
    if quiet {
        ns_app.setActivationPolicy(policy);
    }
    let host = host.ok()?;
    // SAFETY: Tauri's window handle on macOS is its NSWindow, alive as long as `host`.
    let host_ns: &NSWindow = unsafe { &*(host.ns_window().ok()? as *const NSWindow) };
    let page = host_ns.contentView()?;

    let frame = NSRect::new(NSPoint::new(0.0, 0.0), which.size());
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    // SAFETY: NSPanel's designated initializer, on the main thread.
    let panel: Retained<OverlayPanel> = unsafe { msg_send![OverlayPanel::alloc(mtm), initWithContentRect: frame, styleMask: style, backing: NSBackingStoreType::Buffered, defer: false] };
    let panel: Retained<NSPanel> = Retained::into_super(panel);
    // SAFETY: the panel is kept in a thread-local for the life of the app; it must not
    // free itself when closed.
    unsafe { panel.setReleasedWhenClosed(false) };
    panel.setLevel(which.level());
    panel.setOpaque(false);
    // The popover's shadow is macOS's, cast by what the page draws — its rounded card —
    // never the page's own: a CSS shadow is cut off at the panel's square edge and shows
    // as a grey square at each corner (0.1.15, CHECKPOINT §10). The count covers the
    // display and casts none.
    panel.setHasShadow(which.shadow());
    // The label and the toast take no clicks: they go to whatever is under them (N62, B216;
    // N78).
    if matches!(which, Overlay::Label | Overlay::Toast) {
        panel.setIgnoresMouseEvents(true);
    }
    // It stays when another app comes forward, and takes the keyboard when it's given.
    panel.setHidesOnDeactivate(false);
    panel.setBecomesKeyOnlyIfNeeded(false);
    // It stays when shroom's windows are hidden, too (⌘H — which is shroom's whenever a
    // popover or the count has the keyboard, whatever app is in front): an overlay is no
    // window, and hiding closes nothing (DEVIATIONS #39). Left to hide with them, a popover
    // the Scene still had open was off the screen, and the next click on the mushroom
    // closed what nobody could see; a count would go on counting unseen.
    panel.setCanHide(false);
    // On every Space — another app's full-screen one too — so it is wherever the person
    // is, follows them when they switch, and showing it never moves them (DEVIATIONS #40).
    panel.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary);
    // SAFETY: plain AppKit messages on objects owned here.
    unsafe {
        let clear: *mut AnyObject = msg_send![AnyClass::get(c"NSColor").expect("NSColor"), clearColor];
        let _: () = msg_send![&*panel, setBackgroundColor: clear];
    }
    // The page moves house: the host keeps an empty view.
    host_ns.setContentView(Some(&NSView::initWithFrame(NSView::alloc(mtm), frame)));
    panel.setContentView(Some(&page));
    if let Some(web) = web_view(&page) {
        panel.makeFirstResponder(Some(&web));
    }
    Some(panel)
}

/// Shows the panel in front of other apps' windows without bringing shroom forward — and,
/// when the person asked for it, with the keyboard (law 5).
pub fn show(panel: &NSPanel, keyboard: bool) {
    panel.orderFrontRegardless();
    panel.invalidateShadow();
    if keyboard {
        panel.makeKeyWindow();
    }
}

pub fn hide(panel: &NSPanel) {
    panel.orderOut(None);
}

/// Where a mouse-down landed while the popover is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAt {
    /// In the popover itself.
    Popover,
    /// On the mushroom: its own click opens and closes the popover.
    Mushroom,
    /// On another of shroom's overlays — the bubble, Camera's preview, the countdown.
    Overlay,
    /// The click that switches apps or Spaces: on the Dock, or anywhere while Mission
    /// Control is up (`switches` has what else reads the same). The popover follows the
    /// person there (DEVIATIONS #39).
    Switch,
    /// Anywhere else: another app's window, the desktop, the menu bar, one of shroom's
    /// own windows.
    Elsewhere,
}

/// What a mouse-down does to the popover that is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Away {
    Nothing,
    /// A popover that opened by itself becomes the person's.
    Yours,
    Close,
}

/// A click somewhere else closes the popover the person asked for — the click away. One
/// that opened by itself is news, not theirs yet: it stays until they touch it (a click
/// in it makes it theirs; Esc, its buttons and the mushroom do the rest). Clicks on
/// shroom's own overlays never close anything, and neither does the click that switches
/// apps or Spaces (DEVIATIONS #39). The update relaunch's popover (`away_closes`) is news a
/// click away puts away all the same (N41, as amended 2026-10-15).
pub fn click_away(asked: bool, away_closes: bool, at: ClickAt) -> Away {
    match (at, asked) {
        (ClickAt::Popover, false) => Away::Yours,
        (ClickAt::Elsewhere, true) => Away::Close,
        (ClickAt::Elsewhere, false) if away_closes => Away::Close,
        _ => Away::Nothing,
    }
}

struct Watch {
    global: Option<Retained<AnyObject>>,
    local: Option<Retained<AnyObject>>,
    #[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
    on_click: Rc<dyn Fn(ClickAt)>,
}

thread_local! {
    static WATCH: RefCell<Option<Watch>> = const { RefCell::new(None) };
}

/// The mushroom's own window in the menu bar: its number and where it is on screen.
fn mushroom(app: &AppHandle) -> Option<(isize, NSRect)> {
    app.tray_by_id("shroom")?
        .with_inner_tray_icon(|inner| {
            let window = inner.ns_status_item()?.button(MainThreadMarker::new()?)?.window()?;
            Some((window.windowNumber(), window.frame()))
        })
        .ok()
        .flatten()
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *mut c_void;
    fn CGWindowLevelForKey(key: i32) -> i32;
}

/// What macOS has where a mouse-down landed in another app — as much of it as tells a
/// switch from a click away.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Landed {
    /// The level of the window the click would hit, when that window is the Dock's.
    pub on_the_docks: Option<i64>,
    /// The levels of the Dock's windows on screen that cover a whole display.
    pub docks: Vec<i64>,
    /// The level the Dock's own bar sits at.
    pub bar: i64,
}

/// Whether a mouse-down in another app is the click that switches apps or Spaces — not a
/// click away. Two readings (macOS 26.6.2 and 15.7.7, one display,
/// `take_drive/surfaces.rs`):
///
/// - it landed on something of the Dock's above the desktop. Measured: its bar. (The
///   window server hit-tests the bar's window by its shape, though the window is as wide
///   as the screen. The wallpaper and a full-screen Space's backdrop are the Dock's too,
///   but below the desktop: a click on bare desktop stays a click away.) Never read on
///   screen, and a switch only if each is a window of the Dock's above the desktop when
///   it is clicked: a menu or a stack of the Dock's, and the app switcher.
/// - Mission Control is up. The windows it lays out are still their apps' own — a click on
///   one reads as a click in that app — but it is the Dock that answers it, by switching.
///   Mission Control shows as display-wide windows of the Dock's above the desktop and off
///   its bar's level (at 20 and 18, the bar's own at 20); on a desk the Dock has none but
///   the bar's, and in a full-screen Space none at all. Show Desktop and App Exposé put
///   one such window up too (at 18 — measured), so this reads them as Mission Control:
///   while either is up no click is a click away, one on bare desktop included.
pub fn switches(l: &Landed) -> bool {
    l.on_the_docks.is_some_and(|level| level > 0) || l.docks.iter().any(|&level| level > 0 && level != l.bar)
}

/// The window server's list (any app may read it; no window's name or picture is asked
/// for), as (the owner's pid, level, width, height) per window.
fn listed(option: u32, relative_to: u32) -> Vec<(i32, i64, f64, f64)> {
    // SAFETY: a +1 array of dictionaries, or null.
    let raw = unsafe { CGWindowListCopyWindowInfo(option, relative_to) } as *mut NSArray<NSDictionary<NSString, AnyObject>>;
    let Some(list) = (unsafe { Retained::from_raw(raw) }) else { return vec![] };
    let key = |k: &str| NSString::from_str(k);
    let (owner, level, bounds, width, height) = (key("kCGWindowOwnerPID"), key("kCGWindowLayer"), key("kCGWindowBounds"), key("Width"), key("Height"));
    list.iter()
        .map(|w| {
            // SAFETY: the list's numbers are NSNumbers, its bounds a dictionary of them.
            let pid: i32 = w.objectForKey(&owner).map(|n| unsafe { msg_send![&*n, intValue] }).unwrap_or(-1);
            let lvl: i64 = w.objectForKey(&level).map(|n| unsafe { msg_send![&*n, longLongValue] }).unwrap_or(0);
            let side = |k: &NSString| -> f64 {
                w.objectForKey(&bounds).and_then(|b| unsafe { &*(Retained::as_ptr(&b) as *const NSDictionary<NSString, AnyObject>) }.objectForKey(k)).map(|n| unsafe { msg_send![&*n, doubleValue] }).unwrap_or(0.0)
            };
            (pid, lvl, side(&width), side(&height))
        })
        .collect()
}

/// What macOS has at `p` on screen, read live (main thread). Nothing when the Dock isn't
/// running.
fn landed(mtm: MainThreadMarker, p: NSPoint) -> Option<Landed> {
    const ON_SCREEN_ONLY: u32 = 1;
    const INCLUDING_WINDOW: u32 = 1 << 3;
    const EXCLUDE_DESKTOP: u32 = 1 << 4;
    const DOCK_LEVEL_KEY: i32 = 7;
    let dock = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str("com.apple.dock")).iter().next()?.processIdentifier();
    // The window a mouse-down here would hit, whoever's it is.
    let number = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(p, 0, mtm);
    let on_the_docks = (number > 0).then(|| listed(INCLUDING_WINDOW, number as u32)).and_then(|l| l.first().copied()).filter(|w| w.0 == dock).map(|w| w.1);
    let screens: Vec<NSSize> = NSScreen::screens(mtm).iter().map(|s| s.frame().size).collect();
    let covers = |w: f64, h: f64| screens.iter().any(|s| w >= s.width - 1.0 && h >= s.height - 1.0);
    let docks = listed(ON_SCREEN_ONLY | EXCLUDE_DESKTOP, 0).into_iter().filter(|w| w.0 == dock && covers(w.2, w.3)).map(|w| w.1).collect();
    // SAFETY: a plain lookup of a system constant.
    Some(Landed { on_the_docks, docks, bar: unsafe { CGWindowLevelForKey(DOCK_LEVEL_KEY) } as i64 })
}

/// A mouse-down in another app, at `p` on screen: the mushroom's own place in the menu
/// bar is the mushroom's; the Dock's and Mission Control's are the switch (`switches`);
/// everything else is somewhere else.
///
/// The mushroom's place stays the mushroom's here, though it reads as if a click on
/// shroom's own mushroom could never come this way. On macOS 26 the mushroom on screen is
/// not a window of shroom's: the window server lists it as the menu bar's own (Control
/// Center's — measured, 26.6.2; on 15.7.7 it is the app's). Whether macOS then reports its
/// click to this watch as one in another app, nothing can tell without a hand — so it is
/// taken as possible, and such a click must not also count as a click away: the mushroom
/// closes the popover itself, and closing here too would open it again.
///
/// Nothing is lost for it. Where the menu bar is put away — over a full-screen app — macOS
/// parks the mushroom above the top of the screen (15.7.7 and 26.6.2; it is still at its
/// place for the half second the Space slides in), so no click in that app can land on it:
/// a click where it sat on the desk is a click away like any other
/// (`take_drive/surfaces.rs`).
fn at_point(app: &AppHandle, p: NSPoint) -> ClickAt {
    let on_mushroom = mushroom(app).is_some_and(|(_, f)| p.x >= f.origin.x && p.x < f.origin.x + f.size.width && p.y >= f.origin.y && p.y < f.origin.y + f.size.height);
    if on_mushroom {
        return ClickAt::Mushroom;
    }
    match MainThreadMarker::new().and_then(|mtm| landed(mtm, p)) {
        Some(l) if switches(&l) => ClickAt::Switch,
        _ => ClickAt::Elsewhere,
    }
}

/// A mouse-down in one of shroom's own windows.
fn at_window(app: &AppHandle, window: &NSWindow) -> ClickAt {
    let number = window.windowNumber();
    if get(Overlay::Popover).is_some_and(|p| p.windowNumber() == number) {
        return ClickAt::Popover;
    }
    if mushroom(app).is_some_and(|(n, _)| n == number) {
        return ClickAt::Mushroom;
    }
    // The bubble, the preview and the countdown are panels; shroom's windows are not.
    // SAFETY: `isKindOfClass:` on a live window.
    let panel: bool = unsafe { msg_send![window, isKindOfClass: NSPanel::class()] };
    if panel { ClickAt::Overlay } else { ClickAt::Elsewhere }
}

/// Watches mouse-downs while the popover is open (main thread): `on_click` hears where
/// each one landed. Idempotent; `unwatch` ends it.
pub fn watch(app: &AppHandle, on_click: impl Fn(ClickAt) + 'static) {
    if WATCH.with(|w| w.borrow().is_some()) {
        return;
    }
    let Some(mtm) = MainThreadMarker::new() else { return };
    let on_click: Rc<dyn Fn(ClickAt)> = Rc::new(on_click);
    let mask = NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;
    let (app_there, heard_there) = (app.clone(), on_click.clone());
    let there = RcBlock::new(move |_event: NonNull<NSEvent>| heard_there(at_point(&app_there, NSEvent::mouseLocation())));
    let (app_here, heard_here) = (app.clone(), on_click.clone());
    let here = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        // SAFETY: AppKit hands the monitor a live event; it goes on unchanged.
        if let Some(window) = unsafe { event.as_ref() }.window(mtm) {
            heard_here(at_window(&app_here, &window));
        }
        event.as_ptr()
    });
    let global = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &there);
    // SAFETY: the block returns the event it was given.
    let local = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &here) };
    WATCH.with(|w| *w.borrow_mut() = Some(Watch { global, local, on_click }));
}

pub fn unwatch() {
    let Some(watch) = WATCH.with(|w| w.borrow_mut().take()) else { return };
    for monitor in [watch.global, watch.local].into_iter().flatten() {
        // SAFETY: a monitor this module added, removed once.
        unsafe { NSEvent::removeMonitor(&monitor) };
    }
}

/// Whether mouse-downs are being watched, and whether macOS gave both watches (the
/// surfaces drive reads it).
#[cfg(feature = "dev-harness")]
pub fn watching() -> Option<(bool, bool)> {
    WATCH.with(|w| w.borrow().as_ref().map(|w| (w.global.is_some(), w.local.is_some())))
}

/// A mouse-down in another app at `p`, as the watch hears it — the same classification,
/// the same listener (the surfaces drive: nothing in one process can click in another).
#[cfg(feature = "dev-harness")]
pub fn heard_elsewhere(app: &AppHandle, p: NSPoint) -> Option<ClickAt> {
    let on_click = WATCH.with(|w| w.borrow().as_ref().map(|w| w.on_click.clone()))?;
    let at = at_point(app, p);
    on_click(at);
    Some(at)
}

/// What macOS has at `p`, as the watch reads it (the surfaces drive keeps it beside what
/// the watch made of it).
#[cfg(feature = "dev-harness")]
pub fn landed_at(p: NSPoint) -> Option<Landed> {
    landed(MainThreadMarker::new()?, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_somewhere_else_closes_the_popover_you_asked_for() {
        assert_eq!(click_away(true, false, ClickAt::Elsewhere), Away::Close);
        // Its own clicks are its own: the page acts on them.
        assert_eq!(click_away(true, false, ClickAt::Popover), Away::Nothing);
        // The mushroom's click is the mushroom's (it closes the popover itself — closing
        // here too would open it again).
        assert_eq!(click_away(true, false, ClickAt::Mushroom), Away::Nothing);
        // The bubble can be dragged, the preview touched, with `config` open.
        assert_eq!(click_away(true, false, ClickAt::Overlay), Away::Nothing);
    }

    #[test]
    fn the_click_that_switches_apps_or_spaces_closes_nothing() {
        // The popover follows the person: a click on the Dock or in Mission Control is
        // how they go, not a click away (DEVIATIONS #39).
        assert_eq!(click_away(true, false, ClickAt::Switch), Away::Nothing);
        // Nor does it make a popover that opened by itself theirs.
        assert_eq!(click_away(false, false, ClickAt::Switch), Away::Nothing);
    }

    /// The window server's readings behind `switches`, as measured in clean VMs (macOS
    /// 26.6.2 and 15.7.7, one display): the Dock's bar at level 20; its wallpaper and a full-screen
    /// Space's backdrop far below the desktop (never listed here); Mission Control two
    /// more display-wide windows, at 18 and 20.
    #[test]
    fn a_click_on_the_dock_or_in_mission_control_is_a_switch() {
        const BAR: i64 = 20;
        let desk = |on_the_docks| Landed { on_the_docks, docks: vec![BAR], bar: BAR };
        // On the Dock's bar.
        assert!(switches(&desk(Some(BAR))));
        // Any window of the Dock's above the desktop: a menu of its would be one (macOS
        // draws menus at 101) — as would the app switcher, if it is the Dock's; neither
        // was ever read on screen.
        assert!(switches(&desk(Some(101))));
        // Another app's window, the desktop, the menu bar: somewhere else.
        assert!(!switches(&desk(None)));
        // Bare desktop where the Finder draws none: the wallpaper is the Dock's, but below
        // the desktop — still a click away.
        assert!(!switches(&desk(Some(-2_147_483_624))));
        // Mission Control is up: a window it lays out is its app's own, a gap between them
        // nobody's — either way the Dock answers.
        let mission_control = |on_the_docks| Landed { on_the_docks, docks: vec![BAR, BAR, 18], bar: BAR };
        assert!(switches(&mission_control(None)));
        assert!(switches(&mission_control(Some(18))), "its row of Spaces");
        // Show Desktop and App Exposé read the same (one display-wide window more, at 18):
        // while either is up, a click anywhere — bare desktop too — closes nothing.
        assert!(switches(&Landed { on_the_docks: None, docks: vec![BAR, 18], bar: BAR }));
        // A full-screen Space: the Dock has nothing on screen above the desktop.
        assert!(!switches(&Landed { on_the_docks: None, docks: vec![], bar: BAR }));
        // Two displays, a bar's window on each: still a desk.
        assert!(!switches(&Landed { on_the_docks: None, docks: vec![BAR, BAR], bar: BAR }));
    }

    #[test]
    fn making_an_overlay_late_never_costs_shroom_its_windows() {
        // In the background with no Dock icon: switched off for the moment, it can't come
        // forward over the person's app.
        assert!(may_go_quiet(false, false));
        // A Dock icon up (a window is open, law 6) and another app in front: switching
        // shroom off and on again would take that window off the screen. Never.
        assert!(!may_go_quiet(false, true));
        // Already in front: there is nothing to stop.
        assert!(!may_go_quiet(true, false));
        assert!(!may_go_quiet(true, true));
    }

    #[test]
    fn a_popover_that_opened_by_itself_stays_until_you_touch_it() {
        // News the person hasn't seen: working on in another app doesn't lose it.
        assert_eq!(click_away(false, false, ClickAt::Elsewhere), Away::Nothing);
        assert_eq!(click_away(false, false, ClickAt::Mushroom), Away::Nothing);
        assert_eq!(click_away(false, false, ClickAt::Overlay), Away::Nothing);
        // A click in it makes it theirs (and from then on a click away closes it).
        assert_eq!(click_away(false, false, ClickAt::Popover), Away::Yours);
    }

    #[test]
    fn the_update_relaunchs_popover_goes_on_a_click_away() {
        assert_eq!(click_away(false, true, ClickAt::Elsewhere), Away::Close);
        // Otherwise it's like the others that open by themselves.
        assert_eq!(click_away(false, true, ClickAt::Popover), Away::Yours);
        assert_eq!(click_away(false, true, ClickAt::Mushroom), Away::Nothing);
        assert_eq!(click_away(false, true, ClickAt::Overlay), Away::Nothing);
        assert_eq!(click_away(false, true, ClickAt::Switch), Away::Nothing);
    }
}
