//! `--drive take <out> surfaces` (DEVIATIONS #39, #40): shroom's overlays — the popover,
//! the countdown, the bubble — as AppKit and the window server have them, on a provisioned
//! Mac that never records. No take is ever opened: the one count this drive starts is
//! cancelled a second before it could end, whatever else happens.
//!
//! What it does is done the way a person's hand does it, as far as one process can without
//! asking macOS for anything: the mushroom is clicked by a mouse-down and -up delivered to
//! its own menu-bar window (the status item's real handler answers), a control in the
//! popover by a mouse-down and -up delivered to the popover's window at the control's place
//! on the page, Esc and Return by a key-down delivered to the app — for the window that has
//! the keyboard, whose page answers it. Nothing is posted to the system (no CGEvent, no
//! System Events): those events never leave this process.
//!
//! What it reads, per overlay: the window's class, style, level and Space behaviour, whether
//! it shows, is on the active Space and is on screen for the window server, whether it has
//! the keyboard, whether its page takes a first click — and, for the Mac, who is in front
//! and which process the window server sends keys to. Two of those facts have no public
//! reading (the keys' process, a window's "never activates" tag); they are read through
//! symbols looked up at run time, here in the dev harness only, and reported as unknown
//! where a macOS doesn't have them.
//!
//! With another app's state file beside the report (`helper/state.json`, written by
//! `app/test/fullscreen-helper.js` — the L5 VM run, `tests/l5/surfaces-vm.sh`), the same is
//! read over that app's full-screen Space: the popover follows there, opens there, and the
//! count plays there, and nothing ever moves the person off it.
//!
//! First of all, where the overlays meet a set-up shroom's windows (the Library, a window
//! with its Dock icon — law 6; B105, B106). The first popover since
//! shroom started in the background has the keyboard while the other app stays in front;
//! its door brings the Library in front with the keyboard; a popover opened over the
//! Library has the keyboard, and Esc gives it back to the Library; ⌘H with the popover
//! open hides shroom's windows and leaves the popover where it is (hiding closes nothing,
//! DEVIATIONS #39), the mushroom closes and opens it with shroom still hidden, and its door
//! brings the Library back; and ⌘W, with nothing of shroom's left on screen, gives the
//! keyboard back to the app the person was in. (The count and the bubble are held to the
//! same word — neither goes when shroom's windows are hidden — read from the panel, where
//! no key can be pressed at them.) ⌘H and ⌘W are typed the way AppKit delivers a
//! keyboard's keys — put on this app's own queue for whichever window holds the keyboard
//! (`front.rs`).
//!
//! Then, before any count has played: the Library is opened and another app brought
//! in front of it — shroom then has a window on screen and, where law 6 gives one, a Dock
//! icon — and the mushroom and Start are pressed. The popover and the count must come over
//! the other app with the keyboard, and the Library must stay where it was: nothing shroom
//! does to put an overlay up may touch what it already has on screen.
//!
//! "In front" is read from the Mac — which app macOS has frontmost — never from shroom's
//! own `isActive`: AppKit answers yes to that whenever one of the app's windows has the
//! keyboard, and a panel that takes the keyboard without coming forward is the point (a
//! bare AppKit panel born non-activating, made key beside Finder: `isActive` 1, the
//! frontmost app still Finder, keys to the panel).
//!
//! One thing no process can do to another without macOS's leave is click in it. A click
//! somewhere else is therefore put to the popover's watch directly — the same
//! classification, the same listener the watch calls for a mouse-down in another app —
//! and that macOS reports such mouse-downs to a watching app is taken on its word. Three
//! places are put to it: the Dock (a click there switches apps: the popover stays), a
//! window laid out in Mission Control (the same; the VM run brings Mission Control up by
//! opening it, the way its Dock icon does), and a corner of the screen (a click away).
//!
//! A locked screen gives no window the keyboard, and the window server stops keeping its
//! on-screen list (a window put away is still listed): those checks are then listed under
//! `facts.skipped`, never passed.

use std::ffi::{CStr, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{ClassType, Message, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSEvent, NSEventModifierFlags, NSEventType, NSPanel, NSRunningApplication, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSDictionary, NSPoint, NSString};
use serde::Serialize;
use serde_json::{Value, json};
use shroom_core::event::{Event, Key, OsNews, Surface};
use shroom_core::scene::{Action, Click, PopoverBody, Scene};
use shroom_core::world::{Access, SourceId, WindowKind};
use tauri::Manager;

use super::D;
use crate::harness::snapshot;
use crate::runtime::overlay::{self, ClickAt};
use crate::runtime::presenter::{self, Overlay};
use crate::runtime::{camera, probe};

/// One of shroom's windows, as AppKit and the window server have it now.
#[derive(Serialize, Debug, Clone)]
struct Win {
    number: i64,
    class: String,
    panel: bool,
    visible: bool,
    key: bool,
    can_become_key: bool,
    level: i64,
    style_mask: usize,
    /// The style a panel has to be born with for a click in it never to bring its app forward.
    nonactivating: bool,
    /// The window server's own word on that (unknown where it can't be read).
    prevents_activation: Option<bool>,
    tags: Option<String>,
    all_spaces: bool,
    full_screen_aux: bool,
    on_active_space: bool,
    /// Among the windows the window server has on screen now.
    on_screen: bool,
    hides_on_deactivate: bool,
    /// It goes when its app is hidden (⌘H).
    hides_with_the_app: bool,
    key_only_if_needed: Option<bool>,
    /// Its page takes a click that lands while the window hasn't the keyboard.
    first_mouse: Option<bool>,
    /// Its page is where the window's keys go.
    web_first_responder: Option<bool>,
    frame: (f64, f64, f64, f64),
    excluded: bool,
}

/// The Mac around it.
#[derive(Serialize, Debug, Clone)]
struct Mac {
    pid: i32,
    active: bool,
    policy: String,
    front: String,
    front_pid: i32,
    /// The process the window server sends keys to (unknown where it can't be read).
    key_focus_pid: Option<i32>,
    locked: bool,
    /// shroom is hidden (⌘H).
    hidden: bool,
}

impl Mac {
    /// Keys typed now reach shroom.
    fn keys_are_ours(&self) -> bool {
        self.key_focus_pid.is_none_or(|p| p == self.pid)
    }
    /// shroom is the app macOS has in front.
    fn in_front(&self) -> bool {
        self.front_pid == self.pid
    }
}

fn sym(name: &CStr) -> *mut c_void {
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
}

/// The window server's tags for a window (read-only; dev harness only).
fn window_tags(number: u32) -> Option<u64> {
    let (main, get) = (sym(c"CGSMainConnectionID"), sym(c"CGSGetWindowTags"));
    if main.is_null() || get.is_null() {
        return None;
    }
    let main: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(main) };
    let get: unsafe extern "C" fn(i32, u32, *mut u64, i32) -> i32 = unsafe { std::mem::transmute(get) };
    let mut tags = 0u64;
    (unsafe { get(main(), number, &mut tags, 64) } == 0).then_some(tags)
}

/// The window server's "a click here never activates its app" tag.
const PREVENTS_ACTIVATION: u64 = 1 << 16;

/// The process the window server sends keys to (read-only; dev harness only).
pub(crate) fn key_focus_pid() -> Option<i32> {
    #[repr(C)]
    struct Psn {
        high: u32,
        low: u32,
    }
    let (get, pid_of) = (sym(c"CPSGetKeyFocusProcess"), sym(c"GetProcessPID"));
    if get.is_null() || pid_of.is_null() {
        return None;
    }
    let get: unsafe extern "C" fn(*mut Psn, *mut u8) -> i32 = unsafe { std::mem::transmute(get) };
    let pid_of: unsafe extern "C" fn(*const Psn, *mut i32) -> i32 = unsafe { std::mem::transmute(pid_of) };
    let (mut psn, mut flag, mut pid) = (Psn { high: 0, low: 0 }, 0u8, -1i32);
    if unsafe { get(&mut psn, &mut flag) } != 0 {
        return None;
    }
    (unsafe { pid_of(&psn, &mut pid) } == 0).then_some(pid)
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// Declared as `front.rs` declares it: one symbol, one signature.
    fn CGSessionCopyCurrentDictionary() -> *mut AnyObject;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *mut c_void;
}

/// Whether this Mac's screen is locked.
pub(super) fn locked() -> bool {
    let raw = unsafe { CGSessionCopyCurrentDictionary() } as *mut NSDictionary<NSString, AnyObject>;
    let Some(session) = (unsafe { Retained::from_raw(raw) }) else { return false };
    session.objectForKey(&NSString::from_str("CGSSessionScreenIsLocked")).is_some_and(|v| unsafe { msg_send![&*v, boolValue] })
}

/// The numbers of the windows the window server has on screen now.
fn on_screen() -> Vec<i64> {
    const ON_SCREEN_ONLY: u32 = 1;
    let raw = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN_ONLY, 0) } as *mut NSArray<NSDictionary<NSString, AnyObject>>;
    let Some(list) = (unsafe { Retained::from_raw(raw) }) else { return vec![] };
    let number = NSString::from_str("kCGWindowNumber");
    list.iter().filter_map(|w| w.objectForKey(&number)).map(|n| unsafe { msg_send![&*n, longLongValue] }).collect()
}

/// The windows the window server has on screen now, as (owner, layer, width, height).
fn on_screen_windows() -> Vec<(String, i64, f64, f64)> {
    const ON_SCREEN_ONLY: u32 = 1;
    let raw = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN_ONLY, 0) } as *mut NSArray<NSDictionary<NSString, AnyObject>>;
    let Some(list) = (unsafe { Retained::from_raw(raw) }) else { return vec![] };
    let key = |k: &str| NSString::from_str(k);
    let (owner, layer, bounds, width, height) = (key("kCGWindowOwnerName"), key("kCGWindowLayer"), key("kCGWindowBounds"), key("Width"), key("Height"));
    list.iter()
        .map(|w| {
            let name = w.objectForKey(&owner).map(|o| unsafe { &*(Retained::as_ptr(&o) as *const NSString) }.to_string()).unwrap_or_default();
            let level: i64 = w.objectForKey(&layer).map(|n| unsafe { msg_send![&*n, longLongValue] }).unwrap_or(0);
            let side = |k: &NSString| -> f64 {
                w.objectForKey(&bounds).and_then(|b| unsafe { &*(Retained::as_ptr(&b) as *const NSDictionary<NSString, AnyObject>) }.objectForKey(k)).map(|n| unsafe { msg_send![&*n, doubleValue] }).unwrap_or(0.0)
            };
            (name, level, side(&width), side(&height))
        })
        .collect()
}

/// The Dock's windows on screen above the desktop: its bar always (level 20) and, while
/// Mission Control is up, what Mission Control lays over the screen.
fn dock_windows() -> Vec<(i64, f64, f64)> {
    on_screen_windows().into_iter().filter(|(owner, layer, ..)| owner == "Dock" && *layer > 0).map(|(_, layer, w, h)| (layer, w, h)).collect()
}

/// Mission Control is up: the Dock has more on screen than its bar.
fn mission_control_up() -> bool {
    dock_windows().len() > 1
}

/// A place on the Dock, where this Mac shows one: the middle of the strip it keeps along
/// an edge of its screen (main thread).
fn dock_place(mtm: MainThreadMarker) -> Option<NSPoint> {
    NSScreen::screens(mtm).iter().find_map(|screen| {
        let (f, v) = (screen.frame(), screen.visibleFrame());
        let (bottom, left) = (v.origin.y - f.origin.y, v.origin.x - f.origin.x);
        let right = (f.origin.x + f.size.width) - (v.origin.x + v.size.width);
        if bottom > 8.0 {
            Some(NSPoint::new(f.origin.x + f.size.width / 2.0, f.origin.y + bottom / 2.0))
        } else if left > 8.0 {
            Some(NSPoint::new(f.origin.x + left / 2.0, v.origin.y + v.size.height / 2.0))
        } else if right > 8.0 {
            Some(NSPoint::new(f.origin.x + f.size.width - right / 2.0, v.origin.y + v.size.height / 2.0))
        } else {
            None
        }
    })
}

/// The page inside a window: its web view, wherever it sits under the content view.
fn web_view(ns: &NSWindow) -> Option<Retained<NSView>> {
    fn find(v: &NSView, wk: &AnyClass) -> Option<Retained<NSView>> {
        let is: bool = unsafe { msg_send![v, isKindOfClass: wk] };
        if is {
            return Some(v.retain());
        }
        v.subviews().iter().find_map(|s| find(&s, wk))
    }
    find(&*ns.contentView()?, AnyClass::get(c"WKWebView")?)
}

fn win(ns: &NSWindow, excluded: &[u32]) -> Win {
    let number = ns.windowNumber() as i64;
    let panel: bool = unsafe { msg_send![ns, isKindOfClass: NSPanel::class()] };
    let (mask, behaviour, f) = (ns.styleMask(), ns.collectionBehavior(), ns.frame());
    let tags = window_tags(number as u32);
    let web = web_view(ns);
    Win {
        number,
        class: ns.class().name().to_string_lossy().into_owned(),
        panel,
        visible: ns.isVisible(),
        key: ns.isKeyWindow(),
        can_become_key: ns.canBecomeKeyWindow(),
        level: ns.level() as i64,
        style_mask: mask.0,
        nonactivating: mask.contains(NSWindowStyleMask::NonactivatingPanel),
        prevents_activation: tags.map(|t| t & PREVENTS_ACTIVATION != 0),
        tags: tags.map(|t| format!("{t:#x}")),
        all_spaces: behaviour.contains(NSWindowCollectionBehavior::CanJoinAllSpaces),
        full_screen_aux: behaviour.contains(NSWindowCollectionBehavior::FullScreenAuxiliary),
        on_active_space: ns.isOnActiveSpace(),
        on_screen: on_screen().contains(&number),
        hides_on_deactivate: ns.hidesOnDeactivate(),
        hides_with_the_app: ns.canHide(),
        key_only_if_needed: panel.then(|| unsafe { &*(ns as *const NSWindow as *const NSPanel) }.becomesKeyOnlyIfNeeded()),
        first_mouse: web.as_ref().map(|w| unsafe { msg_send![&**w, acceptsFirstMouse: Option::<&NSEvent>::None] }),
        web_first_responder: web.as_ref().map(|w| ns.firstResponder().is_some_and(|r| Retained::as_ptr(&r) as *const c_void == Retained::as_ptr(w) as *const c_void)),
        frame: (f.origin.x, f.origin.y, f.size.width, f.size.height),
        excluded: excluded.contains(&(number as u32)),
    }
}

fn mac() -> Mac {
    let front = NSWorkspace::sharedWorkspace().frontmostApplication();
    Mac {
        pid: std::process::id() as i32,
        active: crate::harness::app_active(),
        policy: crate::harness::policy(),
        front: front.as_ref().and_then(|a| a.bundleIdentifier()).map(|b| b.to_string()).unwrap_or_default(),
        front_pid: front.as_ref().map(|a| a.processIdentifier()).unwrap_or(-1),
        key_focus_pid: key_focus_pid(),
        locked: locked(),
        hidden: crate::front::hidden(),
    }
}

fn uptime() -> f64 {
    let Some(class) = AnyClass::get(c"NSProcessInfo") else { return 0.0 };
    unsafe {
        let info: Retained<AnyObject> = msg_send![class, processInfo];
        msg_send![&*info, systemUptime]
    }
}

/// Which macOS this is, in its own words.
fn macos() -> String {
    let Some(class) = AnyClass::get(c"NSProcessInfo") else { return String::new() };
    unsafe {
        let info: Retained<AnyObject> = msg_send![class, processInfo];
        let version: Retained<NSString> = msg_send![&*info, operatingSystemVersionString];
        version.to_string()
    }
}

/// A left click delivered to `window` at `at` (its own coordinates): a mouse-down and a
/// mouse-up put on this app's event queue — AppKit routes them as it routes a real click on
/// that window (first-mouse rules included). Nothing leaves the process.
fn post_click(mtm: MainThreadMarker, window: &NSWindow, at: NSPoint) {
    let app = NSApplication::sharedApplication(mtm);
    for (kind, pressure) in [(NSEventType::LeftMouseDown, 1.0f32), (NSEventType::LeftMouseUp, 0.0f32)] {
        let event: Option<Retained<NSEvent>> = unsafe {
            msg_send![
                NSEvent::class(),
                mouseEventWithType: kind,
                location: at,
                modifierFlags: NSEventModifierFlags(0),
                timestamp: uptime(),
                windowNumber: window.windowNumber(),
                context: std::ptr::null::<AnyObject>(),
                eventNumber: 0isize,
                clickCount: 1isize,
                pressure: pressure,
            ]
        };
        if let Some(event) = event {
            app.postEvent_atStart(&event, false);
        }
    }
}

/// A key pressed and let go, put on this app's event queue for `window`.
fn post_key(mtm: MainThreadMarker, window: &NSWindow, code: u16, characters: &str) {
    let app = NSApplication::sharedApplication(mtm);
    let characters = NSString::from_str(characters);
    for kind in [NSEventType::KeyDown, NSEventType::KeyUp] {
        let event: Option<Retained<NSEvent>> = unsafe {
            msg_send![
                NSEvent::class(),
                keyEventWithType: kind,
                location: NSPoint::new(0.0, 0.0),
                modifierFlags: NSEventModifierFlags(0),
                timestamp: uptime(),
                windowNumber: window.windowNumber(),
                context: std::ptr::null::<AnyObject>(),
                characters: &*characters,
                charactersIgnoringModifiers: &*characters,
                isARepeat: false,
                keyCode: code,
            ]
        };
        if let Some(event) = event {
            app.postEvent_atStart(&event, false);
        }
    }
}

const ESC: (u16, &str) = (53, "\u{1b}");
const RETURN: (u16, &str) = (36, "\r");

/// How long (seconds) a click or a key is given to show in the Scene. What is asked is
/// whether it acts, not how fast: a Mac that has just started can take seconds over its
/// first web page, and a wait ends the moment the thing happens.
const PATIENCE: u64 = 10;

/// The other app the runner started beside the report, when it did.
struct Helper {
    dir: PathBuf,
}

impl Helper {
    fn beside(out: &Path) -> Option<Helper> {
        let dir = out.parent()?.join("helper");
        dir.join("state.json").exists().then_some(Helper { dir })
    }
    fn state(&self) -> Option<Value> {
        serde_json::from_str(&std::fs::read_to_string(self.dir.join("state.json")).ok()?).ok()
    }
    fn tell(&self, what: &str) {
        let _ = std::fs::write(self.dir.join(format!("cmd-{what}")), "");
    }
    fn wait(&self, d: &D, what: &str, secs: u64, pred: impl Fn(&Value) -> bool) -> Result<Value, String> {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            if let Some(v) = self.state().filter(|v| pred(v)) {
                d.step(what);
                return Ok(v);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(format!("timed out waiting for {what}; the other app says {:?}", self.state()))
    }
    fn pid(&self) -> i32 {
        self.state().and_then(|v| v["pid"].as_i64()).unwrap_or(-1) as i32
    }
    /// Still full screen, on the Space the person is looking at.
    fn there(v: &Value) -> bool {
        v["fullscreen"] == true && v["on_active_space"] == true
    }
}

struct S<'a> {
    d: &'a D,
    skipped: Mutex<Vec<(String, String)>>,
    /// Whose windows were on screen when the run began.
    at_launch: Mutex<Vec<String>>,
}

/// Whose windows the window server has on screen now, at the level apps' windows sit or
/// above it. (What sits below — the desktop's icons, the wallpaper, a full-screen Space's
/// backdrop — comes and goes with Spaces and is never a question put to the person.)
fn on_screen_owners() -> Vec<String> {
    let mut owners: Vec<String> = on_screen_windows().into_iter().filter(|(_, layer, ..)| *layer >= 0).map(|(owner, ..)| owner).collect();
    owners.sort();
    owners.dedup();
    owners
}

impl S<'_> {
    fn skip(&self, what: &str, why: &str) {
        self.skipped.lock().unwrap().push((what.into(), why.into()));
    }
    /// A window put away is off the screen — for AppKit always; for the window server
    /// too, where it is keeping its list.
    fn put_away(&self, what: &str, m: &Mac, w: &Win) {
        self.d.check(what, !w.visible && (m.locked || !w.on_screen), format!("visible={} on_screen={}", w.visible, w.on_screen));
        if m.locked {
            self.skip(&format!("{what} (the window server's list)"), "the screen is locked: the window server isn't keeping its on-screen list");
        }
    }
    /// A check that only means something while a window can have the keyboard.
    fn keyboard(&self, what: &str, m: &Mac, ok: bool, seen: impl Into<String>) {
        if m.locked { self.skip(what, "the screen is locked: no window is given the keyboard") } else { self.d.check(what, ok, seen) }
    }
    fn look(&self, which: Overlay) -> Result<(Option<Win>, Mac), String> {
        let app = self.d.app.clone();
        let excluded = self.d.rt.excluded.lock().unwrap().clone();
        self.d.on_main(move || (presenter::overlay_window(&app, which).map(|w| win(&w, &excluded)), mac())).ok_or_else(|| "the main thread didn't answer".to_string())
    }
    fn shown(&self, which: Overlay) -> Result<(Win, Mac), String> {
        let (w, m) = self.look(which)?;
        Ok((w.ok_or_else(|| format!("no window for {which:?}"))?, m))
    }
    /// One of shroom's ordinary windows, by its label, as macOS has it now.
    fn window(&self, label: &'static str) -> Result<(Win, Mac), String> {
        let app = self.d.app.clone();
        let excluded = self.d.rt.excluded.lock().unwrap().clone();
        let (w, m) = self
            .d
            .on_main(move || {
                // SAFETY: Tauri's window handle on macOS is its NSWindow, alive while the window is.
                let w = app.get_webview_window(label).and_then(|w| w.ns_window().ok()).map(|p| win(unsafe { &*(p as *const NSWindow) }, &excluded));
                (w, mac())
            })
            .ok_or_else(|| "the main thread didn't answer".to_string())?;
        Ok((w.ok_or_else(|| format!("no {label} window"))?, m))
    }
    /// One of shroom's ordinary windows, read until it stands as asked (or `secs` pass):
    /// what was read last.
    fn window_when(&self, label: &'static str, secs: u64, ok: impl Fn(&Win, &Mac) -> bool) -> Result<(Win, Mac), String> {
        let until = Instant::now() + Duration::from_secs(secs);
        loop {
            let (w, m) = self.window(label)?;
            if ok(&w, &m) || Instant::now() > until {
                return Ok((w, m));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    /// The Mac, read until it stands as asked (or `secs` pass): what was read last.
    fn mac_when(&self, secs: u64, ok: impl Fn(&Mac) -> bool) -> Result<Mac, String> {
        let until = Instant::now() + Duration::from_secs(secs);
        loop {
            let m = self.d.on_main(mac).ok_or("the main thread didn't answer")?;
            if ok(&m) || Instant::now() > until {
                return Ok(m);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    /// ⌘ and a key, typed the way a keyboard's arrives: at whichever of shroom's windows
    /// holds the keyboard (`front.rs`).
    fn command(&self, c: char) -> Result<(), String> {
        self.d.on_main(move || crate::front::command(c)).ok_or("the main thread didn't answer")?
    }
    /// Another app, brought in front of shroom: shroom hands over to it.
    fn hand_over(&self, pid: i32) -> Result<bool, String> {
        self.d.on_main(move || NSRunningApplication::runningApplicationWithProcessIdentifier(pid).is_some_and(|a| a.activateWithOptions(NSApplicationActivationOptions(0))));
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if self.d.on_main(mac).ok_or("the main thread didn't answer")?.front_pid == pid {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(false)
    }
    /// The mushroom, clicked: a mouse-down and -up on its own menu-bar window.
    fn click_mushroom(&self) -> Result<(), String> {
        let app = self.d.app.clone();
        let clicked = self.d.on_main(move || {
            let (Some(mtm), Some(w)) = (MainThreadMarker::new(), presenter::mushroom_window(&app)) else { return false };
            let f = w.frame();
            post_click(mtm, &w, NSPoint::new(f.size.width / 2.0, f.size.height / 2.0));
            true
        });
        if clicked == Some(true) { Ok(()) } else { Err("no mushroom to click".into()) }
    }
    /// `config`, opened by a click on the mushroom (closed first if something is open).
    fn open(&self) -> Result<Scene, String> {
        let d = self.d;
        if snapshot(&d.rt).is_some_and(|s| s.popover.is_some()) {
            d.rt.post(Event::PopoverDismissed);
            d.wait("the popover closed", 5, |s| s.popover.is_none())?;
            std::thread::sleep(Duration::from_millis(200));
        }
        self.click_mushroom()?;
        let s = d.wait("config, from a click on the mushroom", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
        // Its page drawn (the first one loads from cold), then a moment for macOS to settle.
        let until = Instant::now() + Duration::from_secs(8);
        while Instant::now() < until && self.page(Overlay::Popover, PAGE_DRAWN).is_none() {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(600));
        Ok(s)
    }
    /// A key delivered to the app for `which`'s window.
    fn press(&self, which: Overlay, key: (u16, &'static str)) -> Result<(), String> {
        let app = self.d.app.clone();
        let sent = self.d.on_main(move || {
            let (Some(mtm), Some(w)) = (MainThreadMarker::new(), presenter::overlay_window(&app, which)) else { return false };
            post_key(mtm, &w, key.0, key.1);
            true
        });
        if sent == Some(true) { Ok(()) } else { Err(format!("no window for {which:?} to press a key in")) }
    }
    /// What a script in `which`'s page answers (a string, or nothing).
    fn page(&self, which: Overlay, js: &'static str) -> Option<String> {
        let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
        let app = self.d.app.clone();
        let _ = self.d.app.run_on_main_thread(move || {
            let Some(web) = presenter::overlay_window(&app, which).and_then(|w| web_view(&w)) else {
                let _ = tx.send(None);
                return;
            };
            let answer = block2::RcBlock::new(move |result: *mut AnyObject, _error: *mut AnyObject| {
                let s = unsafe { result.as_ref() }.and_then(|o| {
                    let is: bool = unsafe { msg_send![o, isKindOfClass: NSString::class()] };
                    is.then(|| unsafe { &*(o as *const AnyObject as *const NSString) }.to_string())
                });
                let _ = tx.send(s);
            });
            let _: () = unsafe { msg_send![&*web, evaluateJavaScript: &*NSString::from_str(js), completionHandler: &*answer] };
        });
        rx.recv_timeout(Duration::from_secs(5)).ok().flatten().filter(|s| !s.is_empty())
    }
    /// A click at a place on `which`'s page (CSS pixels from its top-left).
    fn click_page(&self, which: Overlay, x: f64, y: f64) -> Result<(), String> {
        let app = self.d.app.clone();
        let clicked = self.d.on_main(move || {
            let (Some(mtm), Some(w)) = (MainThreadMarker::new(), presenter::overlay_window(&app, which)) else { return false };
            let Some(web) = web_view(&w) else { return false };
            let y = if web.isFlipped() { y } else { web.bounds().size.height - y };
            post_click(mtm, &w, web.convertPoint_toView(NSPoint::new(x, y), None));
            true
        });
        if clicked == Some(true) { Ok(()) } else { Err(format!("no page in {which:?} to click")) }
    }
}

fn quality(s: &Scene) -> Option<String> {
    match s.popover.as_ref().map(|p| &p.body) {
        Some(PopoverBody::Config(c)) => c.qualities.iter().find(|q| q.selected).map(|q| q.label.clone()),
        _ => None,
    }
}

/// `config` is on the popover's page.
const PAGE_DRAWN: &str = "document.querySelector('.seg button') ? 'drawn' : ''";

/// The quality that isn't chosen, where it is on the popover's page.
const OTHER_QUALITY: &str = "(() => { const b = [...document.querySelectorAll('.seg button')].find((b) => !b.classList.contains('on') && !b.disabled); if (!b) return ''; const r = b.getBoundingClientRect(); return JSON.stringify({ x: r.left + r.width / 2, y: r.top + r.height / 2, label: b.textContent.trim() }); })()";

pub(super) fn surfaces(d: &D) -> Result<(), String> {
    let s = S { d, skipped: Mutex::default(), at_launch: Mutex::default() };
    let result = run(&s);
    // Whatever happened, nothing is left counting or open.
    if snapshot(&d.rt).is_some_and(|sc| sc.countdown.is_some()) {
        d.rt.post(Event::Key { surface: Surface::Countdown, key: Key::Esc });
    }
    d.rt.post(Event::PopoverDismissed);
    // Whose windows are on screen at the end: a question macOS asked — a permission, a
    // warning — would be a window of somebody new. On a desk nobody else is using (the VM
    // run, beside its other app) nothing new may be there; on a person's Mac it is only
    // written down.
    std::thread::sleep(Duration::from_millis(500));
    let (before, now) = (s.at_launch.lock().unwrap().clone(), on_screen_owners());
    let new: Vec<&String> = now.iter().filter(|o| !before.contains(o)).collect();
    d.fact("on_screen", json!({ "at_launch": before, "at_the_end": now, "new": new }));
    if Helper::beside(&d.out).is_some() && result.is_ok() {
        d.check("nothing was asked of macOS: nobody new has a window on screen at the end", new.is_empty(), format!("{new:?}"));
    }
    d.fact("skipped", &*s.skipped.lock().unwrap());
    result
}

fn run(s: &S) -> Result<(), String> {
    let d = s.d;
    d.wait("ready", 20, |sc| sc.tray.click == Click::OpenPopover)?;
    // The mushroom is laid out within a second of going up.
    std::thread::sleep(Duration::from_millis(1500));
    let m0 = d.on_main(mac).ok_or("the main thread didn't answer")?;
    d.fact("at_launch", &m0);
    d.fact("macos", macos());
    *s.at_launch.lock().unwrap() = on_screen_owners();
    d.fact("readings", json!({ "keys_process": m0.key_focus_pid.is_some(), "window_tags": !sym(c"CGSGetWindowTags").is_null() }));
    d.check("shroom starts in the background: not the app in front", !m0.active, format!("{m0:?}"));
    // `--late-panels`: neither panel was made at start-up, so every overlay below is made
    // the late way — the path a failed start-up takes (`overlay::ensure`).
    let late = overlay::late_panels();
    let app = d.app.clone();
    let at_start = d.on_main(move || [Overlay::Popover, Overlay::Countdown].map(|which| presenter::overlay_window(&app, which).is_some())).ok_or("the main thread didn't answer")?;
    d.fact("panels_at_launch", json!({ "late": late, "popover": at_start[0], "count": at_start[1] }));
    if late {
        d.check("--late-panels: neither panel was made as shroom started", at_start == [false, false], format!("{at_start:?}"));
    }
    let helper = Helper::beside(&d.out);
    d.fact("another_app", helper.as_ref().and_then(|h| h.state()));

    through_the_door(s, &m0, helper.as_ref())?;
    behind_a_window(s, &m0, helper.as_ref())?;
    asked_popover(s, &m0)?;
    unasked_popover(s)?;
    click_away(s, helper.as_ref())?;
    bubble(s)?;
    match &helper {
        Some(h) => full_screen(s, h)?,
        None => {
            s.skip("the popover and the count over another app's full-screen Space", "no other app was started beside this run (the L5 VM run starts one)");
            count(s, None, None)?;
        }
    }
    a_take_would_start(s)
}

/// What a take asks of the overlays before it records a frame. ScreenCaptureKit must find
/// every window shroom leaves out of a recording, or the take refuses to start
/// (`shroom-capture`, `ScreenSource::start`) — and the popover's and the count's panels are
/// on that list from the moment they are made. Asked here the way a take asks it, with
/// nothing recorded, after both panels have shown and gone again: the state a take starts
/// in. Only where recording is already allowed (the listing would raise macOS's question
/// anywhere else): the clean VM hasn't allowed it, so this one is this Mac's to read.
fn a_take_would_start(s: &S) -> Result<(), String> {
    const WHAT: &str = "a take would start: ScreenCaptureKit finds every window shroom leaves out of a recording, the popover's and the count's panels among them";
    let d = s.d;
    let excluded = d.rt.excluded.lock().unwrap().clone();
    let app = d.app.clone();
    let panels = d.on_main(move || [Overlay::Popover, Overlay::Countdown].map(|which| presenter::overlay_window(&app, which).map(|w| w.windowNumber() as u32))).ok_or("the main thread didn't answer")?;
    let listed = panels.iter().all(|p| p.is_some_and(|n| excluded.contains(&n)));
    if overlay::late_panels() {
        d.check("--late-panels: both panels, made late, are on the list of what a take leaves out", listed, format!("the panels {panels:?} in {excluded:?}"));
    }
    if !shroom_capture::has_permission() {
        s.skip(WHAT, "recording isn't allowed for shroom on this Mac");
        return Ok(());
    }
    match shroom_capture::not_shareable(&excluded) {
        Ok(missing) => {
            d.fact("left_out_of_a_recording", json!({ "windows": excluded, "the_panels": panels, "not_found": missing }));
            d.check(WHAT, listed && missing.is_empty(), format!("left out {excluded:?}, the panels {panels:?}, not found {missing:?}"));
        }
        Err(e) => {
            d.fact("left_out_of_a_recording", json!({ "windows": excluded, "the_panels": panels, "error": e }));
            d.check(WHAT, false, format!("ScreenCaptureKit didn't list its windows: {e}"));
        }
    }
    Ok(())
}

/// What a panel looks like as macOS draws it on screen, looked at by a machine: its own
/// window captured (`screencapture -l`, nothing left out), decoded, and kept beside the
/// report as `<name>.png` for a person to see. Painted is a tenth of it drawn (alpha ≥ 32)
/// with light and dark in it — a page with something on it, not a clear or flat panel.
/// Captured again for up to `secs` until it is. Only where shroom may record: the capture
/// would ask macOS anywhere else.
fn looked_at(s: &S, number: i64, name: &str, what: &str, secs: f64) {
    let d = s.d;
    if !shroom_capture::has_permission() {
        s.skip(what, "recording isn't allowed for shroom on this Mac");
        return;
    }
    let path = d.out.with_file_name(format!("{name}.png"));
    let until = Instant::now() + Duration::from_secs_f64(secs);
    let mut said;
    loop {
        let _ = std::fs::remove_file(&path);
        let ran = std::process::Command::new("/usr/sbin/screencapture").arg("-x").arg("-o").arg(format!("-l{number}")).arg(&path).status();
        said = match std::fs::read(&path).map(|b| tauri::image::Image::from_bytes(&b)) {
            Ok(Ok(img)) => {
                let (ok, seen) = painted(img.rgba(), img.width(), img.height());
                if ok {
                    d.check(what, true, format!("{seen}; {}", path.display()));
                    return;
                }
                seen
            }
            Ok(Err(e)) => format!("not an image: {e}"),
            Err(_) => format!("screencapture wrote nothing: {ran:?}"),
        };
        if Instant::now() > until {
            d.check(what, false, said);
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Whether an RGBA picture is painted (see `looked_at`), and what it was.
fn painted(rgba: &[u8], w: u32, h: u32) -> (bool, String) {
    let (mut drawn, mut lo, mut hi) = (0usize, 255u32, 0u32);
    for px in rgba.chunks_exact(4) {
        if px[3] >= 32 {
            drawn += 1;
            let l = (u32::from(px[0]) * 3 + u32::from(px[1]) * 6 + u32::from(px[2])) / 10;
            lo = lo.min(l);
            hi = hi.max(l);
        }
    }
    let share = if w * h == 0 { 0.0 } else { drawn as f64 / f64::from(w * h) };
    let spread = hi.saturating_sub(lo);
    (share >= 0.1 && spread >= 64, format!("{w}×{h} px, {:.0} % drawn, brightness {lo}–{hi}", share * 100.0))
}

/// A person's pace back to the mushroom once the keyboard has left shroom (`drives.rs`
/// measured it: what macOS says about who has the keyboard is ahead of what it has done).
const HAND: Duration = Duration::from_secs(3);

/// Where the overlays meet a set-up shroom's windows: the first popover since shroom
/// started in the background, its door to the Library, the popover over the Library, shroom
/// hidden with ⌘H, and the Library closed with ⌘W — who has the keyboard at each turn.
fn through_the_door(s: &S, m0: &Mac, h: Option<&Helper>) -> Result<(), String> {
    const WHAT: &str = "the popover's door to the Library, the popover over it, shroom hidden, the Library closed";
    let d = s.d;
    if m0.locked {
        s.skip(WHAT, "the screen is locked: no window is given the keyboard");
        return Ok(());
    }
    let other = h.map(|h| h.pid()).unwrap_or(m0.front_pid);
    if other <= 0 || other == m0.pid {
        s.skip(WHAT, "no other app was in front when this run started");
        return Ok(());
    }
    let keys = |m: &Mac| format!("front={} ({}) keys go to {:?}, shroom is {}", m.front, m.front_pid, m.key_focus_pid, m.pid);
    let stands = |w: &Win| format!("visible={} on_screen={} key={}", w.visible, w.on_screen, w.key);
    // The Library in front, with the keyboard.
    let in_front = |w: &Win, m: &Mac| w.visible && w.on_screen && w.key && m.in_front() && m.keys_are_ours();

    // The first popover since shroom started in the background.
    s.open()?;
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("first_popover", json!({ "window": p, "mac": m }));
    d.check("started in the background — the first popover you ask for is on screen", p.visible && p.on_screen && p.on_active_space, format!("visible={} on_screen={} on_active_space={}", p.visible, p.on_screen, p.on_active_space));
    s.keyboard("started in the background — the first popover has the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} {}", p.key, keys(&m)));
    d.check("started in the background — the app you were in stays in front", m.front_pid == other, keys(&m));

    // Its door: the Library.
    d.act(Action::OpenLibrary);
    d.wait("the Library, through the popover's door", 10, |sc| sc.library.is_some() && sc.popover.is_none())?;
    let result = (|| {
        let (lib, m) = s.window_when(presenter::LIBRARY, 10, in_front)?;
        let (p, _) = s.shown(Overlay::Popover)?;
        d.fact("library_through_the_door", json!({ "window": lib, "mac": m, "popover": p }));
        s.keyboard("through the popover's door the Library comes in front with the keyboard", &m, in_front(&lib, &m), format!("{} {}", stands(&lib), keys(&m)));
        // (A panel put away fades for a moment before the window server lets go of it.)
        std::thread::sleep(Duration::from_millis(600));
        let (p, m) = s.shown(Overlay::Popover)?;
        s.put_away("the door taken, the popover is off the screen", &m, &p);

        // The popover over the Library: shroom is the app in front this time.
        s.open()?;
        let (p, m) = s.shown(Overlay::Popover)?;
        let (lib, _) = s.window(presenter::LIBRARY)?;
        d.fact("popover_over_the_library_in_front", json!({ "window": p, "mac": m, "library": lib }));
        d.check("the Library in front — a click on the mushroom opens the popover over it", p.visible && p.on_screen && lib.visible && lib.on_screen, format!("popover: {}; the Library: {}", stands(&p), stands(&lib)));
        s.keyboard("the Library in front — the popover has the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} {}", p.key, keys(&m)));
        s.press(Overlay::Popover, ESC)?;
        let closed = d.wait("Esc closed the popover over the Library", PATIENCE, |sc| sc.popover.is_none()).is_ok();
        if !closed {
            d.rt.post(Event::PopoverDismissed);
            d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
        }
        let (lib, m) = s.window_when(presenter::LIBRARY, 3, in_front)?;
        s.keyboard("the Library in front — Esc closes the popover, and the keyboard is the Library's again", &m, closed && in_front(&lib, &m), format!("closed={closed} {} {}", stands(&lib), keys(&m)));

        // ⌘H with the popover open over the Library. The popover has the keyboard, so the
        // key is shroom's: its windows hide. The popover is no window (SM:1720) and follows
        // the person — hiding closes nothing (DEVIATIONS #39): it stays where it is, and
        // what is on screen stays what the Scene has.
        s.open()?;
        s.command('h')?;
        let m = s.mac_when(10, |m| m.hidden && !m.in_front())?;
        let hid = m.hidden && !m.in_front();
        let away = m.front_pid;
        std::thread::sleep(HAND);
        let (lib, _) = s.window(presenter::LIBRARY)?;
        let (p, m) = s.shown(Overlay::Popover)?;
        let open = snapshot(&d.rt).is_some_and(|sc| sc.popover.is_some());
        d.fact("hidden_with_the_popover_open", json!({ "window": p, "mac": m, "library": lib, "the_scene_has_the_popover": open }));
        d.check("⌘H hides shroom's windows, and another app has the keyboard", hid && !lib.on_screen, format!("hidden={} the Library on_screen={} {}", m.hidden, lib.on_screen, keys(&m)));
        d.check(
            "⌘H with the popover open — the popover stays: hiding shroom's windows closes nothing",
            open && p.visible && p.on_screen && p.on_active_space,
            format!("the Scene has it open={open}; visible={} on_screen={} on_active_space={}", p.visible, p.on_screen, p.on_active_space),
        );
        // The mushroom closes it, as it always does — and opens it again, shroom still
        // hidden: the mushroom is the one way back to a hidden window.
        s.click_mushroom()?;
        let closed = d.wait("a click on the mushroom closed the popover, shroom hidden", PATIENCE, |sc| sc.popover.is_none()).is_ok();
        std::thread::sleep(Duration::from_millis(600));
        let (p, m) = s.shown(Overlay::Popover)?;
        d.check("hidden with ⌘H — a click on the mushroom closes the popover", closed && !p.visible && !p.on_screen, format!("closed={closed} visible={} on_screen={}", p.visible, p.on_screen));
        d.check("hidden with ⌘H — the app you were in is still in front", m.front_pid == away, keys(&m));
        s.open()?;
        let (p, m) = s.shown(Overlay::Popover)?;
        let (lib, _) = s.window(presenter::LIBRARY)?;
        d.fact("popover_while_hidden", json!({ "window": p, "mac": m, "library": lib }));
        d.check("hidden with ⌘H — a click on the mushroom opens the popover on screen", p.visible && p.on_screen && p.on_active_space, format!("visible={} on_screen={} on_active_space={} hidden={}", p.visible, p.on_screen, p.on_active_space, m.hidden));
        s.keyboard("hidden with ⌘H — the popover has the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} {}", p.key, keys(&m)));
        d.check("hidden with ⌘H — the app you were in stays in front", m.front_pid == away, keys(&m));
        d.act(Action::OpenLibrary);
        d.wait("the popover's door, taken again", 10, |sc| sc.library.is_some() && sc.popover.is_none())?;
        let (lib, m) = s.window_when(presenter::LIBRARY, 10, in_front)?;
        d.fact("library_back_from_hidden", json!({ "window": lib, "mac": m }));
        s.keyboard("hidden with ⌘H — the popover's door brings the Library back in front with the keyboard", &m, in_front(&lib, &m) && !m.hidden, format!("{} hidden={} {}", stands(&lib), m.hidden, keys(&m)));

        // ⌘W: nothing of shroom's is left on screen, and the keyboard goes back.
        s.command('w')?;
        let gone = d.wait("⌘W closed the Library", 5, |sc| sc.library.is_none()).is_ok();
        let m = s.mac_when(3, |m| m.front_pid == away && !m.active && m.key_focus_pid.is_none_or(|k| k == away))?;
        d.fact("after_the_library_closed", &m);
        d.check("⌘W closes the Library", gone, "");
        s.keyboard("the Library closed — the keyboard goes back to the app you were in", &m, gone && m.front_pid == away && !m.active && m.key_focus_pid.is_none_or(|k| k == away), format!("active={} {}; the app you were in is {away}", m.active, keys(&m)));
        Ok(())
    })();
    // Whatever happened: shroom isn't left hidden, the Library is put away, and the other
    // app is in front again, as it was.
    d.on_main(|| {
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).unhideWithoutActivation();
        }
    });
    d.rt.post(Event::PopoverDismissed);
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    let closed = d.wait("the Library closed", 5, |sc| sc.library.is_none() && sc.popover.is_none());
    if let Some(h) = h {
        h.tell("front");
    }
    let _ = s.hand_over(other);
    std::thread::sleep(Duration::from_millis(600));
    result.and(closed.map(|_| ()))
}

/// The Library open behind another app, then the mushroom and Start — the first count of
/// the run. The popover and the count come over the other app, with the keyboard, and the
/// Library stays on screen where it was.
fn behind_a_window(s: &S, m0: &Mac, h: Option<&Helper>) -> Result<(), String> {
    const WHAT: &str = "the popover and the count with the Library open behind another app";
    let d = s.d;
    if m0.locked {
        s.skip(WHAT, "the screen is locked: no app can be brought in front of another");
        return Ok(());
    }
    let other = h.map(|h| h.pid()).unwrap_or(m0.front_pid);
    if other <= 0 || other == m0.pid {
        s.skip(WHAT, "no other app was in front when this run started");
        return Ok(());
    }
    // Opened the way a person opens it: the mushroom, then the popover's door.
    s.open()?;
    d.act(Action::OpenLibrary);
    d.wait("the Library", 10, |sc| sc.library.is_some())?;
    let result = (|| {
        // The window on screen (its page loads from cold).
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until && !s.window(presenter::LIBRARY).is_ok_and(|(w, _)| w.visible) {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(600));
        // The other app comes forward: shroom hands over, and the app raises its window.
        if let Some(h) = h {
            h.tell("front");
        }
        if !s.hand_over(other)? {
            s.skip(WHAT, "the app that was in front couldn't be brought in front of the Library");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(800));
        let (lib, m) = s.window(presenter::LIBRARY)?;
        d.fact("library_behind_another_app", json!({ "window": lib, "mac": m }));
        d.check("the Library is open behind another app", lib.visible && lib.on_screen && !lib.key && m.front_pid == other, format!("visible={} on_screen={} key={} front={} ({})", lib.visible, lib.on_screen, lib.key, m.front, m.front_pid));

        s.open()?;
        let (p, m) = s.shown(Overlay::Popover)?;
        let (lib, _) = s.window(presenter::LIBRARY)?;
        d.fact("popover_over_a_window_of_ours", json!({ "window": p, "mac": m, "library": lib }));
        d.check("the Library behind another app — a click on the mushroom opens the popover on screen", p.visible && p.on_screen && p.on_active_space, format!("visible={} on_screen={} on_active_space={}", p.visible, p.on_screen, p.on_active_space));
        s.keyboard("the Library behind another app — the popover has the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} keys go to {:?}, shroom is {}", p.key, m.key_focus_pid, m.pid));
        d.check("the Library behind another app — the other app stays in front", m.front_pid == other, format!("front={} ({})", m.front, m.front_pid));
        d.check("the Library behind another app — the Library stays on screen behind the popover", lib.visible && lib.on_screen, format!("visible={} on_screen={}", lib.visible, lib.on_screen));

        count(s, None, Some(presenter::LIBRARY))?;
        let (lib, m) = s.window(presenter::LIBRARY)?;
        d.check("the Library behind another app — after the count the Library is still there", lib.visible && lib.on_screen && m.front_pid == other, format!("visible={} on_screen={} front={} ({})", lib.visible, lib.on_screen, m.front, m.front_pid));
        Ok(())
    })();
    // Whatever happened, the Library is put away again.
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    let closed = d.wait("the Library closed", 5, |sc| sc.library.is_none());
    std::thread::sleep(Duration::from_millis(400));
    result.and(closed.map(|_| ()))
}

/// A click on the mushroom: the popover the person asked for.
fn asked_popover(s: &S, m0: &Mac) -> Result<(), String> {
    let d = s.d;
    s.open()?;
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("popover_asked", json!({ "window": p, "mac": m }));
    d.check("a click on the mushroom opens the popover on screen", p.visible && p.on_screen && p.on_active_space, format!("{p:?}"));
    s.keyboard("the popover you asked for has the keyboard at once (law 5)", &m, p.key && m.keys_are_ours(), format!("key={} keys go to {:?}, shroom is {}", p.key, m.key_focus_pid, m.pid));
    s.keyboard("its page is where the keys go", &m, p.web_first_responder == Some(true), format!("{:?}", p.web_first_responder));
    d.check("opening it never brings shroom forward", !m.in_front() && m.front_pid == m0.front_pid, format!("front={} ({}), was {} ({})", m.front, m.front_pid, m0.front, m0.front_pid));
    d.check(
        "a click in it never brings shroom forward: it's a panel born that way",
        p.panel && p.nonactivating && p.prevents_activation != Some(false),
        format!("class={} panel={} nonactivating={} tags={:?}", p.class, p.panel, p.nonactivating, p.tags),
    );
    d.check("its page takes the first click", p.first_mouse == Some(true), format!("{:?}", p.first_mouse));
    d.check("it can sit on any Space, a full-screen app's too", p.all_spaces && p.full_screen_aux, format!("all_spaces={} full_screen_aux={}", p.all_spaces, p.full_screen_aux));
    d.check("it stays when another app comes forward", !p.hides_on_deactivate, "");
    d.check("it floats above other apps' windows", p.level >= 3, p.level.to_string());
    d.check("it's left out of every recording", p.excluded, "");
    looked_at(s, p.number, "popover", "its pixels, as macOS draws them: the popover's page is painted in its panel", 5.0);

    // Esc, as a key into the app.
    if m.locked {
        s.skip("Esc closes the popover", "the screen is locked: no window is given the keyboard");
        d.rt.post(Event::PopoverDismissed);
        d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    } else {
        // The first key into a page since the Mac started can take a moment to arrive
        // (macOS brings its text input up for the app): how long is a fact of the run.
        s.press(Overlay::Popover, ESC)?;
        let pressed = Instant::now();
        let closed = d.wait("Esc closed the popover", PATIENCE, |sc| sc.popover.is_none()).is_ok();
        d.fact("first_esc_ms", pressed.elapsed().as_millis() as u64);
        d.check("Esc closes the popover", closed, "");
        if !closed {
            // Not a pass either way — whether a second press gets through says what failed.
            s.press(Overlay::Popover, ESC)?;
            let second = d.wait("a second Esc closed the popover", 5, |sc| sc.popover.is_none()).is_ok();
            d.fact("second_esc_closed", second);
            if !second {
                d.rt.post(Event::PopoverDismissed);
                d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
            }
        }
    }
    std::thread::sleep(Duration::from_millis(300));
    let (p, m) = s.shown(Overlay::Popover)?;
    s.put_away("closed, it's off the screen", &m, &p);
    d.check("and the keyboard is back where it was", !m.active && (m.locked || m.key_focus_pid != Some(m.pid)), format!("{m:?}"));

    // The mushroom again: open, and closed by a second click.
    s.open()?;
    s.click_mushroom()?;
    let closed = d.wait("a click on the mushroom closed the popover", PATIENCE, |sc| sc.popover.is_none()).is_ok();
    d.check("a click on the mushroom closes the popover", closed, "");
    Ok(())
}

/// A popover nobody asked for: shown, never key — and a first click on a control acts, and
/// makes it the person's.
fn unasked_popover(s: &S) -> Result<(), String> {
    let d = s.d;
    let before = s.open()?;
    // Put away and shown again the way a popover that opens by itself is shown.
    let app = d.app.clone();
    d.on_main(move || {
        if let Some(w) = presenter::overlay_window(&app, Overlay::Popover) {
            w.orderOut(None);
        }
        presenter::show_popover_unasked(&app);
    });
    std::thread::sleep(Duration::from_millis(500));
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("popover_unasked", json!({ "window": p, "mac": m }));
    d.check("a popover that opens by itself shows on screen", p.visible && p.on_screen, format!("visible={} on_screen={}", p.visible, p.on_screen));
    d.check(
        "and never takes the keyboard (law 5)",
        !p.key && !m.active && (m.locked || m.key_focus_pid != Some(m.pid)),
        format!("key={} active={} keys go to {:?}, shroom is {}", p.key, m.active, m.key_focus_pid, m.pid),
    );

    let was = quality(&before);
    let target: Option<Value> = s.page(Overlay::Popover, OTHER_QUALITY).and_then(|j| serde_json::from_str(&j).ok());
    let Some(target) = target else {
        d.check("the popover's page has a control to click", false, "no quality to switch to");
        return Ok(());
    };
    d.fact("clicked", &target);
    s.click_page(Overlay::Popover, target["x"].as_f64().unwrap_or(0.0), target["y"].as_f64().unwrap_or(0.0))?;
    let acted = d.wait("the first click acted", PATIENCE, |sc| quality(sc).is_some() && quality(sc) != was);
    d.check(
        "the first click on a control acts — the popover didn't have the keyboard",
        acted.as_ref().is_ok_and(|sc| quality(sc).as_deref() == target["label"].as_str()),
        format!("{:?} → {:?}, clicked {}", was, snapshot(&d.rt).as_ref().and_then(quality), target["label"]),
    );
    std::thread::sleep(Duration::from_millis(300));
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("popover_after_first_click", json!({ "window": p, "mac": m }));
    s.keyboard("and that click gives the popover the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} keys go to {:?}, shroom is {}", p.key, m.key_focus_pid, m.pid));
    d.check("without bringing shroom forward", !m.in_front(), format!("front={} ({})", m.front, m.front_pid));
    d.rt.post(Event::PopoverDismissed);
    d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    Ok(())
}

/// What closes the popover from outside: a click somewhere else — put to its watch the way
/// a mouse-down in another app reaches it. A click that switches apps or Spaces — on the
/// Dock, on a window in Mission Control — is no click away: the popover follows.
fn click_away(s: &S, h: Option<&Helper>) -> Result<(), String> {
    let d = s.d;
    s.open()?;
    let watched = d.on_main(overlay::watching).flatten();
    d.check("while the popover is open, clicks elsewhere are watched", watched == Some((true, true)), format!("{watched:?}"));
    // The mushroom's own place in the menu bar: the mushroom answers that click itself.
    let app = d.app.clone();
    let at = d
        .on_main(move || {
            let f = presenter::mushroom_window(&app)?.frame();
            overlay::heard_elsewhere(&app, NSPoint::new(f.origin.x + f.size.width / 2.0, f.origin.y + f.size.height / 2.0))
        })
        .flatten();
    std::thread::sleep(Duration::from_millis(300));
    let still = snapshot(&d.rt).is_some_and(|sc| sc.popover.is_some());
    d.check("a click on the mushroom's place isn't a click away", at == Some(ClickAt::Mushroom) && still, format!("{at:?}, still open: {still}"));
    // The Dock: a click there is how you switch apps.
    const ON_THE_DOCK: &str = "a click on the Dock is a switch, not a click away: the popover stays";
    let m = d.on_main(mac).ok_or("the main thread didn't answer")?;
    let place = d.on_main(|| MainThreadMarker::new().and_then(dock_place)).flatten();
    match place {
        _ if m.locked => s.skip(ON_THE_DOCK, "the screen is locked: the lock screen is over the Dock"),
        None => s.skip(ON_THE_DOCK, "this Mac keeps its Dock hidden"),
        Some(p) => {
            let app = d.app.clone();
            let (at, there) = d.on_main(move || (overlay::heard_elsewhere(&app, p), overlay::landed_at(p))).unwrap_or_default();
            std::thread::sleep(Duration::from_millis(400));
            let still = snapshot(&d.rt).is_some_and(|sc| sc.popover.is_some());
            d.fact("a_click_on_the_dock", json!({ "at": [p.x, p.y], "macos_has_there": there, "heard_as": format!("{at:?}") }));
            d.check(ON_THE_DOCK, at.is_some() && still, format!("{at:?} at ({}, {}), still open: {still}", p.x, p.y));
            if !still {
                s.open()?;
            }
        }
    }
    // Mission Control: a click on a window or a Space there is a switch too.
    const IN_MISSION_CONTROL: &str = "a click on a window in Mission Control is a switch, not a click away: the popover stays";
    match h {
        Some(h) => mission_control(s, h, IN_MISSION_CONTROL)?,
        None => s.skip(IN_MISSION_CONTROL, "Mission Control takes over the whole screen: only the L5 VM run brings it up"),
    }
    // Anywhere else on the screen.
    let app = d.app.clone();
    let (at, there) = d.on_main(move || (overlay::heard_elsewhere(&app, NSPoint::new(6.0, 6.0)), overlay::landed_at(NSPoint::new(6.0, 6.0)))).unwrap_or_default();
    d.fact("a_click_in_a_corner", json!({ "macos_has_there": there, "heard_as": format!("{at:?}") }));
    let closed = d.wait("a click somewhere else closed the popover", PATIENCE, |sc| sc.popover.is_none()).is_ok();
    d.check("a click somewhere else closes the popover you asked for", at == Some(ClickAt::Elsewhere) && closed, format!("{at:?}, closed: {closed}"));
    if !closed {
        d.rt.post(Event::PopoverDismissed);
        d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    }
    std::thread::sleep(Duration::from_millis(300));
    let watched = d.on_main(overlay::watching).flatten();
    d.check("closed, nothing is watched any more", watched.is_none(), format!("{watched:?}"));
    Ok(())
}

/// Mission Control, brought up the way its Dock icon brings it up (the app of that name is
/// opened — and opened again to leave), with the popover open: coming up closes nothing, and
/// a click on a window it lays out — the middle of the screen — closes nothing either.
fn mission_control(s: &S, h: &Helper, what: &str) -> Result<(), String> {
    let d = s.d;
    let toggle = || std::process::Command::new("/usr/bin/open").args(["-a", "Mission Control"]).status().is_ok_and(|st| st.success());
    let becomes = |up: bool| {
        let until = Instant::now() + Duration::from_secs(8);
        while Instant::now() < until && mission_control_up() != up {
            std::thread::sleep(Duration::from_millis(100));
        }
        mission_control_up() == up
    };
    let desk = dock_windows();
    let came = toggle() && becomes(true);
    // Its windows have flown to their places.
    std::thread::sleep(Duration::from_millis(1200));
    d.fact("mission_control", json!({ "the_docks_windows_on_a_desk": desk, "in_mission_control": dock_windows() }));
    d.check("Mission Control came up", came, format!("the Dock's windows on screen: {:?}", dock_windows()));
    if came {
        let open = snapshot(&d.rt).is_some_and(|sc| sc.popover.is_some());
        d.check("Mission Control coming up closes nothing", open, "");
        if open {
            let app = d.app.clone();
            let (at, there) = d
                .on_main(move || {
                    let f = NSScreen::mainScreen(MainThreadMarker::new()?)?.frame();
                    let p = NSPoint::new(f.origin.x + f.size.width / 2.0, f.origin.y + f.size.height / 2.0);
                    Some((overlay::heard_elsewhere(&app, p), overlay::landed_at(p)))
                })
                .flatten()
                .unwrap_or_default();
            std::thread::sleep(Duration::from_millis(400));
            let still = snapshot(&d.rt).is_some_and(|sc| sc.popover.is_some());
            d.fact("a_click_in_mission_control", json!({ "macos_has_there": there, "heard_as": format!("{at:?}") }));
            d.check(what, at.is_some() && still, format!("{at:?}, still open: {still}"));
        }
        let left = toggle() && becomes(false);
        std::thread::sleep(Duration::from_millis(1200));
        d.check("and Mission Control is left again", left, format!("the Dock's windows on screen: {:?}", dock_windows()));
    }
    // The other app in front again, as it was; `config` open for what follows.
    h.tell("front");
    let _ = h.wait(d, "the other app in front again", 10, |v| v["active"] == true);
    if snapshot(&d.rt).is_none_or(|sc| sc.popover.is_none()) {
        s.open()?;
    }
    Ok(())
}

/// The bubble, where this Mac has a camera shroom may open: it never takes the keyboard and
/// can sit on any Space. (Camera's preview is the same panel, made by the same function.)
fn bubble(s: &S) -> Result<(), String> {
    let d = s.d;
    let cameras = d.devices().2;
    if probe::access(shroom_capture::camera_access()) != Access::Granted || !cameras.iter().any(|c| !c.continuity) {
        s.skip("the bubble can sit on any Space and never takes the keyboard", "no camera shroom may open on this Mac");
        return Ok(());
    }
    let sc = s.open()?;
    let on = matches!(sc.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Config(c)) if c.camera_picked().is_some());
    if !on {
        d.act(super::show_yourself_on(&cameras));
    }
    d.wait("the bubble", 5, |sc| sc.bubble.is_some())?;
    std::thread::sleep(Duration::from_millis(900));
    let excluded = d.rt.excluded.lock().unwrap().clone();
    let b = d.on_main(move || camera::bubble_window().map(|w| win(&w, &excluded))).flatten();
    d.fact("bubble", &b);
    if !on {
        d.act(Action::SetCamera(None));
        d.wait("the bubble gone", 5, |sc| sc.bubble.is_none())?;
    }
    d.rt.post(Event::PopoverDismissed);
    d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    let Some(b) = b else {
        d.check("the bubble shows", false, "no bubble window");
        return Ok(());
    };
    d.check("the bubble can sit on any Space, a full-screen app's too", b.all_spaces && b.full_screen_aux && !b.hides_on_deactivate, format!("{b:?}"));
    d.check("the bubble never takes the keyboard (law 5)", !b.key && !b.can_become_key && b.nonactivating, format!("key={} can_become_key={}", b.key, b.can_become_key));
    d.check("the bubble stays when shroom's windows are hidden (⌘H): it is in the picture", !b.hides_with_the_app, "");
    d.check("the bubble is recorded (never left out)", !b.excluded, "");
    Ok(())
}

/// Another app goes full screen — a Space of its own — while the popover is open; then the
/// popover is opened, and the count played, over it.
fn full_screen(s: &S, h: &Helper) -> Result<(), String> {
    let d = s.d;
    h.wait(d, "another app in front, on a desktop", 15, |v| v["active"] == true && v["fullscreen"] == false)?;
    let other = h.pid();
    // Where the mushroom sits while there is a menu bar to sit in.
    let app = d.app.clone();
    let on_a_desk = d.on_main(move || presenter::placement(&app).2).flatten();
    s.open()?;
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("popover_beside_another_app", json!({ "window": p, "mac": m }));
    s.keyboard("with another app in front, the popover you asked for has the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} keys go to {:?}, shroom is {}", p.key, m.key_focus_pid, m.pid));
    let v = h.state().unwrap_or_default();
    d.check("and the other app stays in front", m.front_pid == other && v["active"] == true, format!("front={} ({}), the other app says {v}", m.front, m.front_pid));

    h.tell("full");
    h.wait(d, "the other app full screen", 20, |v| Helper::there(v) && v["active"] == true)?;
    std::thread::sleep(Duration::from_millis(1500));
    let sc = snapshot(&d.rt).ok_or("no Scene")?;
    let (p, m) = s.shown(Overlay::Popover)?;
    d.fact("popover_after_the_switch", json!({ "window": p, "mac": m, "popover": sc.popover.as_ref().map(|p| p.id) }));
    d.check("switching to a full-screen app closes nothing: the popover is still open", sc.popover.is_some(), format!("{:?}", sc.popover.as_ref().map(|p| p.id)));
    d.check("and it followed you there", p.visible && p.on_active_space && p.on_screen, format!("visible={} on_active_space={} on_screen={}", p.visible, p.on_active_space, p.on_screen));

    // Over the full-screen app: closed and opened by the mushroom.
    if sc.popover.is_some() {
        s.click_mushroom()?;
        d.wait("the mushroom closed it there", 5, |sc| sc.popover.is_none())?;
        std::thread::sleep(Duration::from_millis(300));
    }
    s.open()?;
    let (p, m) = s.shown(Overlay::Popover)?;
    let v = h.state().unwrap_or_default();
    d.fact("popover_over_full_screen", json!({ "window": p, "mac": m, "another_app": v }));
    d.check("over a full-screen app, a click on the mushroom opens the popover there", p.visible && p.on_active_space && p.on_screen, format!("visible={} on_active_space={} on_screen={}", p.visible, p.on_active_space, p.on_screen));
    d.check("and you stay in the full-screen app", Helper::there(&v) && v["active"] == true && m.front_pid == other, format!("{v} front={} ({})", m.front, m.front_pid));
    s.keyboard("with the keyboard", &m, p.key && m.keys_are_ours(), format!("key={} keys go to {:?}, shroom is {}", p.key, m.key_focus_pid, m.pid));
    s.press(Overlay::Popover, ESC)?;
    let closed = d.wait("Esc closed it there", PATIENCE, |sc| sc.popover.is_none()).is_ok();
    d.check("Esc closes it there", closed, "");
    if !closed {
        d.rt.post(Event::PopoverDismissed);
        d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    }
    std::thread::sleep(Duration::from_millis(400));
    let m = d.on_main(mac).ok_or("the main thread didn't answer")?;
    d.check("and the keyboard goes back to the app you were in", m.key_focus_pid.is_none_or(|k| k == other) && m.front_pid == other, format!("keys go to {:?}, the other app is {other}", m.key_focus_pid));

    // Over a full-screen app the menu bar is put away, and the mushroom with it: macOS
    // parks it above the top of the screen. So no click in that app can land on the
    // mushroom's place — a click where it sat on the desk is a click away like any other
    // (the watch gives the mushroom's place to the mushroom: `overlay::at_point`).
    const WHERE_IT_SAT: &str = "over a full-screen app, a click where the mushroom sits on a desk is a click away like any other";
    match on_a_desk.filter(|m| m.visible) {
        None => s.skip(WHERE_IT_SAT, "the mushroom wasn't seen on the desk"),
        Some(desk) => {
            s.open()?;
            let app = d.app.clone();
            let read = d
                .on_main(move || {
                    let now = presenter::placement(&app).2?;
                    let screens: Vec<[f64; 4]> = NSScreen::screens(MainThreadMarker::new()?).iter().map(|s| s.frame()).map(|f| [f.origin.x, f.origin.y, f.size.width, f.size.height]).collect();
                    let at = overlay::heard_elsewhere(&app, NSPoint::new(desk.frame.x + desk.frame.w / 2.0, desk.frame.y + desk.frame.h / 2.0));
                    Some((now, screens, at))
                })
                .flatten();
            let (now, screens, at) = read.ok_or("no mushroom to read over the full-screen app")?;
            let f = now.frame;
            let on_a_screen = screens.iter().any(|s| f.x < s[0] + s[2] && f.x + f.w > s[0] && f.y < s[1] + s[3] && f.y + f.h > s[1]);
            let closed = d.wait("a click where the mushroom sat closed the popover", PATIENCE, |sc| sc.popover.is_none()).is_ok();
            d.fact("the_mushroom_over_full_screen", json!({ "on_a_desk": [desk.frame.x, desk.frame.y, desk.frame.w, desk.frame.h], "there": [f.x, f.y, f.w, f.h], "seen_there": now.visible, "screens": screens, "a_click_where_it_sat": format!("{at:?}") }));
            d.check("over a full-screen app the mushroom is put away with the menu bar: its place is off the screen", !now.visible && !on_a_screen, format!("at ({}, {}) {}×{}, seen={}, screens {screens:?}", f.x, f.y, f.w, f.h, now.visible));
            d.check(WHERE_IT_SAT, at == Some(ClickAt::Elsewhere) && closed, format!("{at:?}, closed: {closed}"));
            if !closed {
                d.rt.post(Event::PopoverDismissed);
                d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
    }

    windows_from_full_screen(s, h)?;
    let counted = count(s, Some(h), None);
    h.tell("desk");
    let _ = h.wait(d, "the other app back on a desktop", 20, |v| v["fullscreen"] == false);
    counted
}

/// The Library and Settings asked for from a full-screen app (Bagrat on 0.1.4, 2026-10-13:
/// they opened over it, stayed on its Space when he ⌘-Tabbed away, and came back there).
/// They are ordinary windows with the Dock icon (law 6): each opens on a desktop — never
/// over the full-screen app — and stays there when the person goes back to it.
fn windows_from_full_screen(s: &S, h: &Helper) -> Result<(), String> {
    let d = s.d;
    h.tell("front");
    h.wait(d, "the full-screen app in front", 10, |v| Helper::there(v) && v["active"] == true)?;
    for (label, door, name) in [(presenter::LIBRARY, Action::OpenLibrary, "the Library"), (presenter::SETTINGS, Action::OpenSettings, "Settings")] {
        s.open()?;
        d.act(door);
        d.wait(&format!("{name}, from the full-screen app"), 10, |sc| (if label == presenter::LIBRARY { sc.library.is_some() } else { sc.settings.is_some() }) && sc.popover.is_none())?;
        // macOS slides to the window's desktop: let it.
        std::thread::sleep(Duration::from_millis(2000));
        let (w, m) = s.window(label)?;
        let v = h.state().unwrap_or_default();
        d.fact(&format!("{name} from full screen"), json!({ "window": w, "mac": m, "another_app": v }));
        d.check(
            &format!("from a full-screen app, {name} opens on a desktop, not over that app — with the Dock icon"),
            w.visible && w.on_active_space && !w.all_spaces && !w.full_screen_aux && v["on_active_space"] != true && m.policy.starts_with("regular"),
            format!("on_active_space={} all_spaces={} full_screen_aux={} policy={}; the full-screen app says {v}", w.on_active_space, w.all_spaces, w.full_screen_aux, m.policy),
        );
        // Back to the full-screen app, as ⌘-Tab takes you: the window stays on its desktop.
        h.tell("front");
        let back = h.wait(d, "back in the full-screen app", 10, |v| Helper::there(v) && v["active"] == true);
        std::thread::sleep(Duration::from_millis(1500));
        let (w, _) = s.window(label)?;
        d.check(&format!("back in the full-screen app, {name} stays on its desktop"), back.is_ok() && !w.on_active_space, format!("{back:?}; on_active_space={}", w.on_active_space));
        d.rt.post(Event::WindowClosed(if label == presenter::LIBRARY { WindowKind::Library } else { WindowKind::Settings }));
        d.wait(&format!("{name} closed"), 5, |sc| sc.library.is_none() && sc.settings.is_none())?;
        std::thread::sleep(Duration::from_millis(500));
        let m = d.on_main(mac).ok_or("the main thread didn't answer")?;
        d.check(&format!("{name} closed: the Dock icon goes"), m.policy.starts_with("accessory"), m.policy.clone());
    }
    Ok(())
}

/// The count on the chosen display, cancelled by Esc — never a take: a second before it
/// could end it is cancelled whatever happened. `h` is another app gone full screen under
/// it; `behind` one of shroom's own windows that is open behind whatever is in front.
fn count(s: &S, h: Option<&Helper>, behind: Option<&'static str>) -> Result<(), String> {
    let d = s.d;
    // Which count a check speaks of, where a run plays more than one.
    let n = |what: &str| if behind.is_some() { format!("the Library behind another app — {what}") } else { what.to_string() };
    s.open()?;
    // The arm-time guard must find nothing to ask macOS for (no prompt is ever raised from
    // a drive): where this Mac hasn't answered for shroom-dev, an answer stands in.
    let real = (shroom_capture::has_permission(), probe::access(shroom_capture::mic_access()), probe::access(shroom_capture::camera_access()));
    let settled = |a: Access| if a == Access::NotAsked { Access::Denied } else { a };
    d.fact("grants", json!({ "screen": real.0, "mic": format!("{:?}", real.1), "camera": format!("{:?}", real.2), "stood_in": !real.0 || real.1 == Access::NotAsked || real.2 == Access::NotAsked }));
    let stand_in = || d.rt.post(Event::Os(OsNews::Grants { screen: true, mic: settled(real.1), camera: settled(real.2) }));
    stand_in();
    let display = shroom_capture::main_display_id();
    d.act(Action::SetSource(SourceId::Display(display)));
    let start_is_live = |sc: &Scene| crate::runtime::records(sc) == Some(SourceId::Display(display)) && matches!(sc.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Config(c)) if c.start.enabled);
    d.wait("Start is live for the display", 5, start_is_live)?;
    let before = d.on_main(mac).ok_or("the main thread didn't answer")?;
    let other = h.map(|h| h.pid());

    // The guard: whenever a count shows, it is cancelled two seconds in — a second before it
    // could end — unless it has gone by then. Whether the guard had to do it.
    let stop = Arc::new(AtomicBool::new(false));
    let (rt, stopped) = (d.rt.clone(), stop.clone());
    let guard = std::thread::spawn(move || {
        let mut since: Option<Instant> = None;
        while !stopped.load(Ordering::SeqCst) {
            let counting = snapshot(&rt).is_some_and(|sc| sc.countdown.is_some());
            match (counting, since) {
                (true, None) => since = Some(Instant::now()),
                (true, Some(t)) if t.elapsed() >= Duration::from_millis(2000) => {
                    rt.post(Event::Key { surface: Surface::Countdown, key: Key::Esc });
                    return true;
                }
                (false, Some(_)) => return false,
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    });
    // Return, pressed the way Esc is: a key-down for the popover's panel, which has the
    // keyboard. AppKit hands it to the page, and the page hands it to the core as every
    // arm goes: with what macOS allows read afresh just before it (`commands::key`, the
    // arm-time guard). So what the key does is this Mac's to say. Where shroom may record,
    // the count starts. Where it may not — a clean VM — the answer that stood in above is
    // replaced by the truth, and `config` says recording is off: news only the page's own
    // Return brings. Either way the key went the whole road. Where the guard would ask
    // macOS first — this Mac has a microphone it hasn't answered for, or a camera is picked
    // and it hasn't answered for the camera — a drive never lets it; and behind a locked
    // screen no window has the keyboard.
    const BY_RETURN: &str = "Return, pressed in the popover, goes through its page to the core";
    let recording_off = |sc: &Scene| sc.state_ids().contains(&"recording-off");
    let showing = snapshot(&d.rt).is_some_and(|sc| matches!(sc.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Config(c)) if c.camera_picked().is_some()));
    let would_ask = real.0 && ((real.1 == Access::NotAsked && !d.devices().1.is_empty()) || (real.2 == Access::NotAsked && showing));
    if before.locked {
        s.skip(&n(BY_RETURN), "the screen is locked: no window is given the keyboard");
    } else if would_ask {
        s.skip(&n(BY_RETURN), "this Mac hasn't answered for the microphone or the camera: Return would ask macOS, and a drive never does");
    } else {
        s.press(Overlay::Popover, RETURN)?;
        let answered = d.wait("Return in the popover, answered", PATIENCE, |sc| sc.countdown.is_some() || recording_off(sc));
        let counts = answered.as_ref().is_ok_and(|sc| sc.countdown.is_some());
        let seen = match (&answered, counts) {
            (Ok(_), true) => "the count started",
            (Ok(_), false) => "config says recording is off: the page's Return re-read what macOS allows",
            (Err(_), _) => "nothing came of it",
        };
        // The count where shroom may record, the truth about recording where it may not.
        d.check(&n(BY_RETURN), answered.is_ok() && counts == real.0, format!("{seen}; shroom may record here: {}", real.0));
    }
    // What follows wants a count to look at. Where Return didn't start one, the answer
    // stands in again and the key is handed to the core.
    if snapshot(&d.rt).is_none_or(|sc| sc.countdown.is_none()) {
        stand_in();
        d.wait("Start is live for the display", 5, start_is_live)?;
        d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    }
    let counting = d.wait("the count", 3, |sc| sc.countdown.is_some());
    if counting.is_err() {
        stop.store(true, Ordering::SeqCst);
        let _ = guard.join();
        return counting.map(|_| ());
    }
    std::thread::sleep(Duration::from_millis(700));
    let (c, m) = s.shown(Overlay::Countdown)?;
    let v = h.and_then(|h| h.state());
    let ours = behind.map(|label| s.window(label)).transpose()?.map(|(w, _)| w);
    d.fact(if behind.is_some() { "countdown_over_a_window_of_ours" } else { "countdown" }, json!({ "window": c, "mac": m, "before": before, "another_app": v, "behind": ours }));
    // The guard ends the count two seconds in: a second is what there is to look at it.
    if behind.is_none() && !m.locked {
        looked_at(s, c.number, "count", "its pixels, as macOS draws them: the count's page is painted in its panel", 1.0);
    }
    if !m.locked {
        s.press(Overlay::Countdown, ESC)?;
    }
    let by_esc = {
        let until = Instant::now() + Duration::from_millis(800);
        let mut gone = false;
        while Instant::now() < until && !gone {
            gone = snapshot(&d.rt).is_some_and(|sc| sc.countdown.is_none());
            std::thread::sleep(Duration::from_millis(25));
        }
        gone
    };
    let guarded = guard.join().unwrap_or(true);
    let after = d.wait("the count is over", 5, |sc| sc.countdown.is_none())?;
    if after.tray.state == Some("recording") {
        return Err("the count reached a take — refused: this drive never records".into());
    }

    d.check(&n("the count shows on the display, over whatever is there"), c.visible && c.on_active_space && c.on_screen, format!("visible={} on_active_space={} on_screen={}", c.visible, c.on_active_space, c.on_screen));
    s.keyboard(&n("the count has the keyboard, so Esc is yours"), &m, c.key && m.keys_are_ours() && c.web_first_responder == Some(true), format!("key={} keys go to {:?}, shroom is {}", c.key, m.key_focus_pid, m.pid));
    d.check(&n("the count never brings shroom forward"), !m.in_front() && m.front_pid == before.front_pid, format!("front={} ({}), was {} ({})", m.front, m.front_pid, before.front, before.front_pid));
    d.check(
        &n("a click on the count never brings shroom forward either: it's a panel born that way"),
        c.panel && c.nonactivating && c.prevents_activation != Some(false),
        format!("class={} panel={} nonactivating={} tags={:?}", c.class, c.panel, c.nonactivating, c.tags),
    );
    d.check(&n("the count takes the first click (any click cancels)"), c.first_mouse == Some(true), format!("{:?}", c.first_mouse));
    d.check(&n("the count can sit on any Space, a full-screen app's too"), c.all_spaces && c.full_screen_aux && !c.hides_on_deactivate, format!("all_spaces={} full_screen_aux={}", c.all_spaces, c.full_screen_aux));
    d.check(&n("the count stays when shroom's windows are hidden (⌘H)"), !c.hides_with_the_app, "");
    d.check(&n("the count is left out of every recording"), c.excluded, "");
    if m.locked {
        s.skip(&n("Esc cancels the count"), "the screen is locked: no window is given the keyboard");
    } else {
        d.check(&n("Esc cancels the count"), by_esc && !guarded, format!("gone in time={by_esc} the drive's own cancel was needed={guarded}"));
    }
    if let Some(v) = &v {
        d.check(&n("the count plays over the full-screen app: you never leave it"), Helper::there(v) && m.front_pid == other.unwrap_or(-1), v.to_string());
    }
    if let Some(w) = &ours {
        d.check(&n("shroom's own window stays on screen behind the count"), w.visible && w.on_screen, format!("visible={} on_screen={}", w.visible, w.on_screen));
    }

    // Cancelled: `config` is back, asked for. Put away, the person is where they were.
    std::thread::sleep(Duration::from_millis(400));
    let (c, m) = s.shown(Overlay::Countdown)?;
    s.put_away(&n("cancelled, the count is off the screen"), &m, &c);
    d.check(&n("and shroom still isn't in front"), !m.in_front() && m.front_pid == before.front_pid, format!("front={} ({})", m.front, m.front_pid));
    d.rt.post(Event::PopoverDismissed);
    d.wait("the popover closed", 5, |sc| sc.popover.is_none())?;
    std::thread::sleep(Duration::from_millis(400));
    let m = d.on_main(mac).ok_or("the main thread didn't answer")?;
    let v = h.and_then(|h| h.state());
    d.fact("after_the_count", json!({ "mac": m, "another_app": v }));
    match (&v, other) {
        (Some(v), Some(other)) => d.check(
            &n("when it ends you're where you were: the full-screen app, with the keyboard"),
            Helper::there(v) && !m.active && m.front_pid == other && m.key_focus_pid.is_none_or(|k| k == other),
            format!("{v} front={} keys go to {:?}", m.front_pid, m.key_focus_pid),
        ),
        _ => d.check(&n("when it ends you're where you were"), !m.active && m.front_pid == before.front_pid && (m.locked || m.key_focus_pid != Some(m.pid)), format!("{m:?}")),
    }
    Ok(())
}
