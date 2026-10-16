//! ⌥⌘P, the pause shortcut (spec N54): held through the system's hot-key service
//! (Carbon's `RegisterEventHotKey`) only while the scene says a take is open, and let go
//! the moment it closes, so the keys belong to other apps the rest of the time. A hot key
//! needs no permission (no Accessibility, no Input Monitoring: only key taps and global key
//! monitors do), and the system swallows the press, so the app in front never sees it.
//! The press posts `Event::Shortcut`; the core decides what it means.

use std::ffi::c_void;
use std::sync::Mutex;

use shroom_core::event::Event;

use super::Runtime;

type OSStatus = i32;
type EventTargetRef = *mut c_void;
type EventHandlerRef = *mut c_void;
type EventHandlerCallRef = *mut c_void;
type EventRef = *mut c_void;
type EventHotKeyRef = *mut c_void;

#[repr(C)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EventHotKeyID {
    signature: u32,
    id: u32,
}

const K_EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
/// `kVK_ANSI_P`.
const KEY_P: u32 = 0x23;
/// `cmdKey | optionKey`.
const CMD_OPTION: u32 = (1 << 8) | (1 << 11);
const ID: EventHotKeyID = EventHotKeyID { signature: u32::from_be_bytes(*b"shrm"), id: 1 };

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> EventTargetRef;
    fn InstallEventHandler(
        target: EventTargetRef,
        handler: extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus,
        num_types: usize,
        list: *const EventTypeSpec,
        user_data: *mut c_void,
        out: *mut EventHandlerRef,
    ) -> OSStatus;
    fn RegisterEventHotKey(code: u32, modifiers: u32, id: EventHotKeyID, target: EventTargetRef, options: u32, out: *mut EventHotKeyRef) -> OSStatus;
    fn UnregisterEventHotKey(key: EventHotKeyRef) -> OSStatus;
}

struct Held {
    rt: Option<Runtime>,
    /// The handler is installed once and stays; only the key comes and goes.
    handler: bool,
    key: usize,
}

static HELD: Mutex<Held> = Mutex::new(Held { rt: None, handler: false, key: 0 });

extern "C" fn pressed(_call: EventHandlerCallRef, _event: EventRef, _data: *mut c_void) -> OSStatus {
    let rt = HELD.lock().unwrap().rt.clone();
    if let (Some(rt), Some(mtm)) = (rt, objc2_foundation::MainThreadMarker::new()) {
        // The click's own probe: a grant switched off or a device gone answers first.
        let (quick, _us) = super::probe::quick(mtm, &rt.home.root);
        rt.log("hotkey: ⌥⌘P");
        rt.post(Event::Shortcut(quick));
    }
    0
}

/// Holds ⌥⌘P or lets it go, as the scene says (main thread; the presenter calls it when
/// `tray.shortcut` changes).
pub fn hold(rt: &Runtime, on: bool) {
    let mut h = HELD.lock().unwrap();
    if h.rt.is_none() {
        h.rt = Some(rt.clone());
    }
    if on == (h.key != 0) {
        return;
    }
    // SAFETY: Carbon's documented hot-key calls, on the main thread; the handler and the
    // key ref are owned here, and the handler only posts.
    unsafe {
        let target = GetApplicationEventTarget();
        if on {
            if !h.handler {
                let spec = EventTypeSpec { event_class: K_EVENT_CLASS_KEYBOARD, event_kind: K_EVENT_HOT_KEY_PRESSED };
                let mut out: EventHandlerRef = std::ptr::null_mut();
                let err = InstallEventHandler(target, pressed, 1, &spec, std::ptr::null_mut(), &mut out);
                if err != 0 {
                    rt.log(&format!("hotkey: the handler wasn't installed ({err}) — ⌥⌘P does nothing this take"));
                    return;
                }
                h.handler = true;
            }
            let mut key: EventHotKeyRef = std::ptr::null_mut();
            let err = RegisterEventHotKey(KEY_P, CMD_OPTION, ID, target, 0, &mut key);
            if err != 0 || key.is_null() {
                rt.log(&format!("hotkey: ⌥⌘P is taken ({err}) — the mushroom still pauses"));
                return;
            }
            h.key = key as usize;
            rt.log("hotkey: ⌥⌘P held for the take");
        } else {
            UnregisterEventHotKey(h.key as EventHotKeyRef);
            h.key = 0;
            rt.log("hotkey: ⌥⌘P let go");
        }
    }
}

/// Whether ⌥⌘P is held now (the take drive's check).
#[cfg(feature = "dev-harness")]
pub fn held() -> bool {
    HELD.lock().unwrap().key != 0
}

/// A press of ⌥⌘P as the system delivers it, through the application's event target to the
/// installed handler (the take drive's; a synthesized keystroke would need Accessibility).
#[cfg(feature = "dev-harness")]
pub fn press() -> bool {
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn CreateEvent(alloc: *const c_void, class: u32, kind: u32, when: f64, flags: u32, out: *mut EventRef) -> OSStatus;
        fn SetEventParameter(event: EventRef, name: u32, kind: u32, size: usize, data: *const c_void) -> OSStatus;
        fn SendEventToEventTarget(event: EventRef, target: EventTargetRef) -> OSStatus;
        fn ReleaseEvent(event: EventRef);
    }
    // SAFETY: a Carbon event made, sent on the main thread and released here.
    unsafe {
        let mut ev: EventRef = std::ptr::null_mut();
        if CreateEvent(std::ptr::null(), K_EVENT_CLASS_KEYBOARD, K_EVENT_HOT_KEY_PRESSED, 0.0, 0, &mut ev) != 0 {
            return false;
        }
        let id = ID;
        let set = SetEventParameter(ev, u32::from_be_bytes(*b"----"), u32::from_be_bytes(*b"hkid"), std::mem::size_of::<EventHotKeyID>(), &id as *const _ as *const c_void);
        let sent = set == 0 && SendEventToEventTarget(ev, GetApplicationEventTarget()) == 0;
        ReleaseEvent(ev);
        sent
    }
}
