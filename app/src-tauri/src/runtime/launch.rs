//! How this process started, and how it may end — read from the Apple events macOS hands
//! the app. A login launch is silent (N41): `SMAppService.mainAppService` starts shroom
//! with no arguments, so the only mark it leaves is the launch event's
//! `keyAELaunchedAsLogInItem`. And nothing ends the app mid-take (law 4): ⌘Q (the app
//! menu) and the Dock's Quit reach `applicationShouldTerminate:`, which tao's delegate
//! doesn't answer — shroom adds it, posts `QuitRequested` and lets the core decide.
//! Logout, restart and shutdown, and another app's quit (System Settings' quit-and-reopen,
//! the install script's quit after its status check) end the app as macOS's
//! (`Terminating { ours: false }` — a take then lands in `recovered-take`).

use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{msg_send, sel};
use shroom_core::event::{Event, QuitVia};
use tauri::{AppHandle, Manager};

use super::Runtime;

const fn code(c: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*c)
}

/// `kCoreEventClass`.
pub const AEVT: u32 = code(b"aevt");
/// `kAEOpenApplication`.
pub const OAPP: u32 = code(b"oapp");
/// `kAEQuitApplication`.
pub const QUIT: u32 = code(b"quit");
/// `keyAEPropData`.
pub const PRDT: u32 = code(b"prdt");
/// `keyAELaunchedAsLogInItem`.
pub const LGIT: u32 = code(b"lgit");
/// `kAEQuitReason`: set on the quit events of a logout, a restart or a shutdown.
pub const WHY: u32 = code(b"why?");
/// `keySenderPIDAttr`.
pub const SPID: u32 = code(b"spid");
pub const DOCK: &str = "com.apple.dock";

/// The Apple event being handled, as far as shroom reads it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppleEvent {
    pub class: u32,
    pub id: u32,
    /// `keyAEPropData`'s enum code (the launch event's login mark).
    pub prop: Option<u32>,
    /// `kAEQuitReason` (logout, restart, shutdown).
    pub quit_reason: Option<u32>,
    /// The sending app's bundle id.
    pub sender: Option<String>,
}

/// macOS started shroom at login: the open-application event carries the login mark.
pub fn at_login(ev: Option<&AppleEvent>) -> bool {
    ev.is_some_and(|e| e.class == AEVT && e.id == OAPP && e.prop == Some(LGIT))
}

/// What a terminate request is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Let it end: shroom asked for it, or it's macOS's (logout, restart, shutdown,
    /// another app's quit).
    Let,
    /// The person's quit: the core decides (law 4, `quit-confirm`).
    Ask(QuitVia),
}

/// ⌘Q and the app menu's Quit send no Apple event; the Dock's Quit is a quit event from
/// the Dock. Everything else that asks shroom to quit is macOS's.
pub fn ending(ours: bool, ev: Option<&AppleEvent>) -> Ending {
    if ours {
        return Ending::Let;
    }
    match ev {
        None => Ending::Ask(QuitVia::CmdQ),
        Some(e) if e.class == AEVT && e.id == QUIT => {
            if e.quit_reason.is_some() {
                Ending::Let
            } else if e.sender.as_deref() == Some(DOCK) {
                Ending::Ask(QuitVia::Dock)
            } else {
                Ending::Let
            }
        }
        // Some other event is being handled while the menu's Quit runs: still the person's.
        Some(_) => Ending::Ask(QuitVia::CmdQ),
    }
}

/// The Apple event the app is handling now, if any. Main thread only.
pub fn current_event() -> Option<AppleEvent> {
    // SAFETY: plain Foundation messages with their documented types; every object is
    // checked for nil before it's messaged.
    unsafe {
        let mgr_cls = AnyClass::get(c"NSAppleEventManager")?;
        let mgr: *mut AnyObject = msg_send![mgr_cls, sharedAppleEventManager];
        if mgr.is_null() {
            return None;
        }
        let ev: *mut AnyObject = msg_send![mgr, currentAppleEvent];
        if ev.is_null() {
            return None;
        }
        let class: u32 = msg_send![ev, eventClass];
        let id: u32 = msg_send![ev, eventID];
        let param = |key: u32| -> *mut AnyObject { msg_send![ev, paramDescriptorForKeyword: key] };
        let attr = |key: u32| -> *mut AnyObject { msg_send![ev, attributeDescriptorForKeyword: key] };
        let prop = param(PRDT);
        let prop = (!prop.is_null()).then(|| {
            let v: u32 = msg_send![prop, enumCodeValue];
            v
        });
        let why = Some(attr(WHY)).filter(|d| !d.is_null()).or_else(|| Some(param(WHY)).filter(|d| !d.is_null()));
        let quit_reason = why.map(|d| {
            let v: u32 = msg_send![d, typeCodeValue];
            v
        });
        let pid = attr(SPID);
        let sender = (!pid.is_null())
            .then(|| {
                let pid: i32 = msg_send![pid, int32Value];
                pid
            })
            .and_then(|pid| {
                let cls = AnyClass::get(c"NSRunningApplication")?;
                let running: *mut AnyObject = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
                if running.is_null() {
                    return None;
                }
                let bundle: *mut objc2_foundation::NSString = msg_send![running, bundleIdentifier];
                (!bundle.is_null()).then(|| (*bundle).to_string())
            });
        Some(AppleEvent { class, id, prop, quit_reason, sender })
    }
}

/// Read while the launch event is being handled (the app's setup runs inside
/// `applicationDidFinishLaunching:`).
pub fn launched_at_login() -> bool {
    at_login(current_event().as_ref())
}

static APP: OnceLock<AppHandle> = OnceLock::new();

const TERMINATE_CANCEL: usize = 0;
const TERMINATE_NOW: usize = 1;

extern "C-unwind" fn should_terminate(_this: &AnyObject, _cmd: Sel, _sender: *mut AnyObject) -> usize {
    let Some(rt) = APP.get().and_then(|a| a.try_state::<Runtime>()) else { return TERMINATE_NOW };
    let ev = current_event();
    match ending(rt.ending_ours.load(Ordering::SeqCst), ev.as_ref()) {
        Ending::Let => TERMINATE_NOW,
        Ending::Ask(via) => {
            rt.log(&format!("quit asked: {via:?}"));
            rt.post(Event::QuitRequested(via));
            TERMINATE_CANCEL
        }
    }
}

/// Gives the app delegate an `applicationShouldTerminate:` (tao's has none). Main thread,
/// after the event loop exists. Whether the method was added.
pub fn hook_terminate(app: &AppHandle) -> bool {
    let _ = APP.set(app.clone());
    // SAFETY: the method is added to the delegate's own class (tao's, which has no such
    // method), with the type encoding of `- (NSApplicationTerminateReply)…:(NSApplication *)`.
    unsafe {
        let Some(ns_app_cls) = AnyClass::get(c"NSApplication") else { return false };
        let ns_app: *mut AnyObject = msg_send![ns_app_cls, sharedApplication];
        if ns_app.is_null() {
            return false;
        }
        let delegate: *mut AnyObject = msg_send![ns_app, delegate];
        if delegate.is_null() {
            return false;
        }
        let cls = (*delegate).class() as *const AnyClass as *mut AnyClass;
        let f: extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> usize = should_terminate;
        let imp: Imp = std::mem::transmute(f);
        objc2::ffi::class_addMethod(cls, sel!(applicationShouldTerminate:), imp, c"Q@:@".as_ptr()).as_bool()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quit(reason: Option<&[u8; 4]>, sender: Option<&str>) -> AppleEvent {
        AppleEvent { class: AEVT, id: QUIT, prop: None, quit_reason: reason.map(code), sender: sender.map(String::from) }
    }

    #[test]
    fn a_login_launch_is_read_from_the_launch_event() {
        let login = AppleEvent { class: AEVT, id: OAPP, prop: Some(LGIT), ..Default::default() };
        assert!(at_login(Some(&login)));
        assert!(!at_login(None), "no event: a launch someone did");
        assert!(!at_login(Some(&AppleEvent { prop: None, ..login.clone() })), "a plain open-application event");
        assert!(!at_login(Some(&AppleEvent { id: code(b"odoc"), ..login.clone() })), "a document open");
        assert!(!at_login(Some(&AppleEvent { prop: Some(code(b"lhid")), ..login })), "launched hidden is not at login");
    }

    #[test]
    fn the_persons_quit_asks_the_core_and_macos_quits_go_through() {
        assert_eq!(ending(false, None), Ending::Ask(QuitVia::CmdQ), "⌘Q and the app menu's Quit");
        assert_eq!(ending(false, Some(&quit(None, Some(DOCK)))), Ending::Ask(QuitVia::Dock), "the Dock's Quit");
        for why in [b"logo", b"rlgo", b"rrst", b"rsdn", b"rest", b"shut"] {
            assert_eq!(ending(false, Some(&quit(Some(why), Some("com.apple.loginwindow")))), Ending::Let, "shutdown never asks");
            assert_eq!(ending(false, Some(&quit(Some(why), Some(DOCK)))), Ending::Let);
        }
        assert_eq!(ending(false, Some(&quit(None, Some("com.apple.systempreferences")))), Ending::Let, "System Settings' quit-and-reopen (bend 9)");
        assert_eq!(ending(false, Some(&quit(None, Some("com.apple.osascript")))), Ending::Let, "the install script's quit, after its status check (N9)");
        assert_eq!(ending(false, Some(&quit(None, None))), Ending::Let, "an unknown sender is macOS's");
        assert_eq!(ending(true, None), Ending::Let, "an end shroom asked for");
        assert_eq!(ending(true, Some(&quit(None, Some(DOCK)))), Ending::Let);
    }
}
