//! The QuickProbe: the local facts every entry event carries, gathered synchronously so a
//! surface never opens on a stale grant and the core never probes inline (ARCHITECTURE
//! §3.1). AppKit parts run on the main thread.

use std::time::Instant;

use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{MainThreadMarker, NSNumber, NSString};
use shroom_core::world::{Access, Device, Display, QuickProbe};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CGRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> CGRect;
}

/// A display's bounds in global points, origin top-left of the main display, y down.
pub fn display_bounds(display: u32) -> CGRect {
    unsafe { CGDisplayBounds(display) }
}

pub fn access(a: shroom_capture::Access) -> Access {
    match a {
        shroom_capture::Access::NotAsked => Access::NotAsked,
        shroom_capture::Access::Restricted => Access::Restricted,
        shroom_capture::Access::Denied => Access::Denied,
        shroom_capture::Access::Granted => Access::Granted,
    }
}

/// Displays by their human names, as macOS shows them (spec §5.2).
pub fn displays(mtm: MainThreadMarker) -> Vec<Display> {
    let main = unsafe { CGMainDisplayID() };
    let key = NSString::from_str("NSScreenNumber");
    NSScreen::screens(mtm)
        .iter()
        .filter_map(|s| {
            let desc = s.deviceDescription();
            let num: Retained<NSNumber> = desc.objectForKey(&key)?.downcast().ok()?;
            let id = num.unsignedIntValue();
            Some(Display { id, name: s.localizedName().to_string(), is_main: id == main, x: s.frame().origin.x as i32, y: display_bounds(id).y as i32 })
        })
        .collect()
}

/// Every microphone, macOS's default input marked (N44: what System default means).
pub fn mics() -> Vec<Device> {
    let default = shroom_capture::mic::default_input_uid();
    shroom_capture::mic::input_devices()
        .into_iter()
        .filter(|m| m.transport != shroom_capture::mic::Transport::Aggregate)
        .map(|m| Device {
            builtin: m.transport == shroom_capture::mic::Transport::BuiltIn,
            continuity: m.transport == shroom_capture::mic::Transport::Continuity,
            system_default: default.as_deref() == Some(m.uid.as_str()),
            uid: m.uid,
            name: m.name,
        })
        .collect()
}

/// Every camera, macOS's preferred one marked (N44).
pub fn cameras() -> Vec<Device> {
    let preferred = preferred_camera();
    shroom_capture::cameras()
        .into_iter()
        .map(|c| Device { builtin: c.builtin, continuity: c.continuity, system_default: preferred.as_deref() == Some(c.uid.as_str()), uid: c.uid, name: c.name })
        .collect()
}

/// The camera macOS prefers now — `AVCaptureDevice.systemPreferredCamera` (macOS 13+),
/// which follows the person's choices and Continuity's hand-offs — by its unique id.
pub fn preferred_camera() -> Option<String> {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    let cls = AnyClass::get(c"AVCaptureDevice")?;
    // SAFETY: a class property returning an autoreleased device or nil; its uniqueID is an NSString.
    let device: Option<Retained<AnyObject>> = unsafe { msg_send![cls, systemPreferredCamera] };
    let uid: Option<Retained<NSString>> = unsafe { msg_send![&*device?, uniqueID] };
    uid.map(|u| u.to_string())
}

pub fn disk_free(path: &std::path::Path) -> u64 {
    let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap_or_default();
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0 { st.f_bavail as u64 * st.f_frsize } else { 0 }
}

/// Gathers everything; must run on the main thread. Returns how long it took (target
/// ≤ 5 ms — ARCHITECTURE §3.10).
pub fn quick(mtm: MainThreadMarker, home: &std::path::Path) -> (QuickProbe, u128) {
    let started = Instant::now();
    let q = QuickProbe {
        screen: shroom_capture::has_permission(),
        mic: access(shroom_capture::mic_access()),
        camera: access(shroom_capture::camera_access()),
        displays: displays(mtm),
        mics: mics(),
        cameras: cameras(),
        disk_free: disk_free(home),
        online: super::net::online(),
        os: super::observe::session(),
        // A crowded menu bar can hide the mushroom behind the notch (N41).
        status_item_visible: super::presenter::status_item_visible(),
        // Read only: what macOS says now, so Settings shows a change made in System
        // Settings on its entry and focus (N36, b4 #6).
        login_item: super::os::login_item(),
    };
    (q, started.elapsed().as_micros())
}
