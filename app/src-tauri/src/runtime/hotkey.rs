//! ⌥⌘P, the pause shortcut (spec N54), and ⌥⌘M beside it, a marker (N64): held through the
//! system's hot-key service (Carbon's `RegisterEventHotKey`) only while the scene says a
//! take is open, and let go the moment it closes, so the keys belong to other apps the rest
//! of the time. A hot key needs no permission (no Accessibility, no Input Monitoring: only
//! key taps and global key monitors do), and the system swallows the press, so the app in
//! front never sees it. ⌥⌘P posts `Event::Shortcut`, ⌥⌘M `Event::MarkerShortcut`; the core
//! decides what each means. Each key is held on its own: one another app has taken leaves
//! the other working.

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
/// `kVK_ANSI_M`.
const KEY_M: u32 = 0x2E;
/// `cmdKey | optionKey`.
const CMD_OPTION: u32 = (1 << 8) | (1 << 11);
const SIGNATURE: u32 = u32::from_be_bytes(*b"shrm");

/// The two keys, by their hot-key ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hot {
    /// ⌥⌘P: pause, or Resume (N54).
    Pause,
    /// ⌥⌘M: a marker (N64).
    Marker,
}

impl Hot {
    const ALL: [Hot; 2] = [Hot::Pause, Hot::Marker];

    fn id(self) -> EventHotKeyID {
        EventHotKeyID { signature: SIGNATURE, id: self as u32 + 1 }
    }

    fn code(self) -> u32 {
        match self {
            Hot::Pause => KEY_P,
            Hot::Marker => KEY_M,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Hot::Pause => "⌥⌘P",
            Hot::Marker => "⌥⌘M",
        }
    }

    /// The key a press's id names — shroom's own ids only.
    fn of(id: EventHotKeyID) -> Option<Hot> {
        Hot::ALL.into_iter().find(|h| h.id().signature == id.signature && h.id().id == id.id)
    }
}

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
    fn GetEventParameter(event: EventRef, name: u32, kind: u32, out_kind: *mut u32, size: usize, out_size: *mut usize, data: *mut c_void) -> OSStatus;
}

/// `kEventParamDirectObject`, and `typeEventHotKeyID`: a press's key.
const PARAM_DIRECT: u32 = u32::from_be_bytes(*b"----");
const TYPE_HOT_KEY_ID: u32 = u32::from_be_bytes(*b"hkid");

struct Held {
    rt: Option<Runtime>,
    /// The handler is installed once and stays; only the keys come and go.
    handler: bool,
    /// Each key's ref while it's held (0: not held), in `Hot::ALL`'s order.
    keys: [usize; 2],
    /// Whether the scene holds the keys now (each may still have been refused).
    on: bool,
}

static HELD: Mutex<Held> = Mutex::new(Held { rt: None, handler: false, keys: [0; 2], on: false });

extern "C" fn pressed(_call: EventHandlerCallRef, event: EventRef, _data: *mut c_void) -> OSStatus {
    let mut id = EventHotKeyID { signature: 0, id: 0 };
    // SAFETY: Carbon's documented read of a hot-key event's id, into a value of its size.
    let read = unsafe { GetEventParameter(event, PARAM_DIRECT, TYPE_HOT_KEY_ID, std::ptr::null_mut(), std::mem::size_of::<EventHotKeyID>(), std::ptr::null_mut(), &mut id as *mut _ as *mut c_void) };
    let Some(hot) = (read == 0).then(|| Hot::of(id)).flatten() else { return 0 };
    let rt = HELD.lock().unwrap().rt.clone();
    if let (Some(rt), Some(mtm)) = (rt, objc2_foundation::MainThreadMarker::new()) {
        // The click's own probe: a grant switched off or a device gone answers first.
        let (quick, _us) = super::probe::quick(mtm, &rt.home.root);
        rt.log(&format!("hotkey: {}", hot.name()));
        rt.post(match hot {
            Hot::Pause => Event::Shortcut(quick),
            Hot::Marker => Event::MarkerShortcut(quick),
        });
    }
    0
}

/// Holds ⌥⌘P and ⌥⌘M or lets them go, as the scene says (main thread; the presenter calls
/// it when `tray.shortcut` changes).
pub fn hold(rt: &Runtime, on: bool) {
    let mut h = HELD.lock().unwrap();
    if h.rt.is_none() {
        h.rt = Some(rt.clone());
    }
    if on == h.on {
        return;
    }
    h.on = on;
    // SAFETY: Carbon's documented hot-key calls, on the main thread; the handler and the
    // key refs are owned here, and the handler only posts.
    unsafe {
        let target = GetApplicationEventTarget();
        if on {
            if !h.handler {
                let spec = EventTypeSpec { event_class: K_EVENT_CLASS_KEYBOARD, event_kind: K_EVENT_HOT_KEY_PRESSED };
                let mut out: EventHandlerRef = std::ptr::null_mut();
                let err = InstallEventHandler(target, pressed, 1, &spec, std::ptr::null_mut(), &mut out);
                if err != 0 {
                    rt.log(&format!("hotkey: the handler wasn't installed ({err}) — ⌥⌘P and ⌥⌘M do nothing this take"));
                    return;
                }
                h.handler = true;
            }
            for (i, hot) in Hot::ALL.into_iter().enumerate() {
                let mut key: EventHotKeyRef = std::ptr::null_mut();
                let err = RegisterEventHotKey(hot.code(), CMD_OPTION, hot.id(), target, 0, &mut key);
                if err != 0 || key.is_null() {
                    rt.log(&format!("hotkey: {} is taken ({err}) — {}", hot.name(), if hot == Hot::Pause { "the mushroom still pauses" } else { "the paused popover still drops one" }));
                    continue;
                }
                h.keys[i] = key as usize;
                rt.log(&format!("hotkey: {} held for the take", hot.name()));
            }
        } else {
            for (i, hot) in Hot::ALL.into_iter().enumerate() {
                if h.keys[i] != 0 {
                    UnregisterEventHotKey(h.keys[i] as EventHotKeyRef);
                    h.keys[i] = 0;
                    rt.log(&format!("hotkey: {} let go", hot.name()));
                }
            }
        }
    }
}

/// Whether ⌥⌘P is held now (the take drive's check).
#[cfg(feature = "dev-harness")]
pub fn held() -> bool {
    held_key(Hot::Pause)
}

/// Whether one of the keys is held now.
#[cfg(feature = "dev-harness")]
pub fn held_key(hot: Hot) -> bool {
    HELD.lock().unwrap().keys[hot as usize] != 0
}

/// A press of ⌥⌘P as the system delivers it (the take drive's).
#[cfg(feature = "dev-harness")]
pub fn press() -> bool {
    press_key(Hot::Pause)
}

/// A press of one of the keys as the system delivers it, through the application's event
/// target to the installed handler (a synthesized keystroke would need Accessibility).
#[cfg(feature = "dev-harness")]
pub fn press_key(hot: Hot) -> bool {
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
        let id = hot.id();
        let set = SetEventParameter(ev, u32::from_be_bytes(*b"----"), u32::from_be_bytes(*b"hkid"), std::mem::size_of::<EventHotKeyID>(), &id as *const _ as *const c_void);
        let sent = set == 0 && SendEventToEventTarget(ev, GetApplicationEventTarget()) == 0;
        ReleaseEvent(ev);
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_key_has_its_own_id_and_only_shrooms_ids_are_read() {
        assert_eq!(Hot::of(Hot::Pause.id()), Some(Hot::Pause));
        assert_eq!(Hot::of(Hot::Marker.id()), Some(Hot::Marker));
        assert_ne!(Hot::Pause.id().id, Hot::Marker.id().id);
        assert_eq!(Hot::Pause.id().id, 1, "⌥⌘P keeps the id it always had");
        assert_eq!(Hot::of(EventHotKeyID { signature: u32::from_be_bytes(*b"othr"), id: 1 }), None, "another app's key");
        assert_eq!(Hot::of(EventHotKeyID { signature: SIGNATURE, id: 9 }), None);
        assert_eq!((Hot::Pause.code(), Hot::Marker.code()), (0x23, 0x2E), "kVK_ANSI_P, kVK_ANSI_M");
    }
}
