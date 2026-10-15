//! What macOS tells shroom about the session and the hardware (ARCHITECTURE §3.2 `Mac.os`,
//! `displays`/`mics`/`cameras`, `disk_free`): sleep and wake, a locked screen, a switch to
//! another user, displays/cameras/mics coming and going, and the disk's free space. Each
//! change posts the core's own `Os` news; the core decides what it means for a take (the
//! auto-pause family, N39).
//!
//! Sources, chosen for being pushed rather than polled where macOS offers it:
//! - sleep/wake and user switching: NSWorkspace's notification center (and IOKit's
//!   will-sleep, which macOS waits on — `power.rs`, DECISIONS B92);
//! - lock/unlock: the system's distributed `com.apple.screenIsLocked`/`…Unlocked`;
//! - displays: CGDisplayRegisterReconfigurationCallback; cameras: AVCaptureDevice's
//!   connect/disconnect notifications; microphones: a 2 s re-probe (CoreAudio's listener
//!   adds little for a list that's re-read whole anyway);
//! - disk: a 30 s re-probe, posted when it moved by 100 MB or more (mid-take the recorder's
//!   watchdog looks every second, against the floor);
//! - the grants, while a take is open: macOS can switch screen recording or the microphone
//!   off under a take (System Settings, a cleanup tool) and says nothing — re-read (cheap
//!   preflight calls) on every observer event and every 2 s, posted when they change
//!   (N39 `permission-lost`);
//! - the time zone: `NSSystemTimeZoneDidChangeNotification` (the Library's dates).

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{MainThreadMarker, NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSOperationQueue, NSString};
use shroom_core::event::{Event, OsNews};
use shroom_core::world::OsSession;

use super::{Runtime, probe};

static SESSION: Mutex<OsSession> = Mutex::new(OsSession { awake: true, unlocked: true, on_console: true });
static RT: Mutex<Option<Runtime>> = Mutex::new(None);
/// The grants last posted while a take was open (`None` outside a take).
static GRANTS: Mutex<Option<OsNews>> = Mutex::new(None);

/// Whether a take is open, as the last Scene shows it (the mushroom's red dot or pause bars).
fn take_open(rt: &Runtime) -> bool {
    rt.snapshot.lock().unwrap().as_ref().is_some_and(|s| matches!(s.tray.face, shroom_core::scene::Face::Red | shroom_core::scene::Face::PauseBars))
}

/// While a take is open, posts the grants when they differ from the last ones posted
/// (`read` is `os::grants` — a seam for the tests); outside a take, forgets them.
pub fn grants_if_changed(rt: &Runtime, read: impl Fn() -> OsNews) {
    let mut last = GRANTS.lock().unwrap();
    if !take_open(rt) {
        *last = None;
        return;
    }
    let now = read();
    if last.as_ref() != Some(&now) {
        rt.log(&format!("os: grants during a take {now:?}"));
        *last = Some(now.clone());
        rt.post(Event::Os(now));
    }
}

fn change(rt: &Runtime, f: impl FnOnce(&mut OsSession), woke: bool) {
    let now = {
        let mut s = SESSION.lock().unwrap();
        let before = *s;
        f(&mut s);
        (before != *s).then_some(*s)
    };
    if let Some(s) = now {
        rt.log(&format!("os: awake={} unlocked={} on_console={}", s.awake, s.unlocked, s.on_console));
        rt.post(Event::Os(OsNews::Session(s)));
    }
    if woke {
        rt.post(Event::Os(OsNews::Woke));
    }
    grants_if_changed(rt, super::os::grants);
}

/// IOKit's will-sleep (`power.rs`, DECISIONS B92): the session is asleep from this instant
/// — NSWorkspace's own will-sleep, if it comes after, finds nothing new to say.
pub(crate) fn asleep(rt: &Runtime) {
    change(rt, |s| s.awake = false, false);
}

/// The tests' way back from `asleep` (the session is process-wide).
#[cfg(test)]
pub(crate) fn awake(rt: &Runtime) {
    change(rt, |s| s.awake = true, false);
}

/// The session as last reported, for the QuickProbe.
pub fn session() -> OsSession {
    *SESSION.lock().unwrap()
}

fn observe(center: &NSNotificationCenter, name: &NSString, rt: &Runtime, f: fn(&Runtime)) {
    let rt = rt.clone();
    let block = RcBlock::new(move |_n: NonNull<NSNotification>| f(&rt));
    // SAFETY: the block is retained by the center; observers are delivered on the main
    // queue, and nothing in them blocks.
    let token: Retained<ProtocolObject<dyn NSObjectProtocol>> =
        unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, Some(&NSOperationQueue::mainQueue()), &block) };
    // Observers live as long as the process: the token is never removed.
    std::mem::forget(token);
}

/// Re-reads every device list and posts it (main thread: display names come from NSScreen).
fn devices_now(rt: &Runtime) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    rt.post(Event::Os(OsNews::Devices { displays: probe::displays(mtm), mics: probe::mics(), cameras: probe::cameras() }));
    grants_if_changed(rt, super::os::grants);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayRegisterReconfigurationCallback(cb: extern "C" fn(u32, u32, *mut c_void), user: *mut c_void) -> i32;
}

/// `kCGDisplayBeginConfigurationFlag`: the change is about to happen — wait for the end.
const BEGIN_CONFIGURATION: u32 = 1;

extern "C" fn display_changed(_display: u32, flags: u32, _user: *mut c_void) {
    if flags & BEGIN_CONFIGURATION != 0 {
        return;
    }
    if let Some(rt) = RT.lock().unwrap().clone() {
        devices_now(&rt);
    }
}

/// Starts every observer. Call on the main thread, once.
pub fn watch(rt: Runtime, home: std::path::PathBuf) {
    *RT.lock().unwrap() = Some(rt.clone());
    let ws = NSWorkspace::sharedWorkspace().notificationCenter();
    let names = |s: &str| NSString::from_str(s);
    observe(&ws, &names("NSWorkspaceWillSleepNotification"), &rt, |rt| change(rt, |s| s.awake = false, false));
    observe(&ws, &names("NSWorkspaceDidWakeNotification"), &rt, |rt| change(rt, |s| s.awake = true, true));
    observe(&ws, &names("NSWorkspaceSessionDidResignActiveNotification"), &rt, |rt| change(rt, |s| s.on_console = false, false));
    observe(&ws, &names("NSWorkspaceSessionDidBecomeActiveNotification"), &rt, |rt| change(rt, |s| s.on_console = true, false));
    let dist = NSDistributedNotificationCenter::defaultCenter();
    observe(&dist, &names("com.apple.screenIsLocked"), &rt, |rt| change(rt, |s| s.unlocked = false, false));
    observe(&dist, &names("com.apple.screenIsUnlocked"), &rt, |rt| change(rt, |s| s.unlocked = true, false));
    let default = NSNotificationCenter::defaultCenter();
    observe(&default, &names("AVCaptureDeviceWasConnectedNotification"), &rt, devices_now);
    observe(&default, &names("AVCaptureDeviceWasDisconnectedNotification"), &rt, devices_now);
    // The time zone moved (travel, a manual change) or the clock was set: the Library's
    // dates follow it. A daylight-saving change moves neither — the quiet thread below
    // re-reads the offset every 30 s and posts a change.
    observe(&default, &names("NSSystemTimeZoneDidChangeNotification"), &rt, |rt| {
        rt.post(Event::Os(OsNews::UtcOffset(super::os::utc_offset_min())));
    });
    observe(&default, &names("NSSystemClockDidChangeNotification"), &rt, |rt| {
        rt.post(Event::Os(OsNews::UtcOffset(super::os::utc_offset_min())));
    });
    // SAFETY: the callback only reads a static and posts; it lives for the process.
    unsafe { CGDisplayRegisterReconfigurationCallback(display_changed, std::ptr::null_mut()) };

    // Microphones and the disk, re-read on a quiet thread.
    let rt2 = rt.clone();
    std::thread::Builder::new()
        .name("shroom-observe".into())
        .spawn(move || {
            let mut mics = probe::mics();
            let mut camera = probe::preferred_camera();
            let mut disk = probe::disk_free(&home);
            let mut offset = super::os::utc_offset_min();
            let mut ticks = 0u32;
            loop {
                std::thread::sleep(Duration::from_secs(2));
                ticks += 1;
                grants_if_changed(&rt2, super::os::grants);
                // The microphones and their default, and macOS's preferred camera (N44: what
                // System default follows — AirPods joining move the default input).
                let now = probe::mics();
                let preferred = probe::preferred_camera();
                if now != mics || preferred != camera {
                    mics = now;
                    camera = preferred;
                    // Display names need the main thread; the core takes all three lists at once.
                    let rt3 = rt2.clone();
                    dispatch_main(move || devices_now(&rt3));
                }
                if ticks % 15 == 0 {
                    let free = probe::disk_free(&home);
                    if free.abs_diff(disk) >= 100 * 1024 * 1024 {
                        disk = free;
                        rt2.post(Event::Os(OsNews::Disk(free)));
                    }
                    // Daylight saving began or ended: the offset moved with no notification.
                    let now = super::os::utc_offset_min();
                    if now != offset {
                        offset = now;
                        rt2.post(Event::Os(OsNews::UtcOffset(now)));
                    }
                }
            }
        })
        .expect("the observer thread starts");
    rt.log("os: sleep/wake, lock, user switching, displays, cameras, time zone, clock observed; mics (and, mid-take, the grants) every 2 s, disk and the UTC offset every 30 s");
}

/// Runs `f` on the main queue.
pub fn dispatch_main(f: impl FnOnce() + Send + 'static) {
    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
    }
    extern "C" fn trampoline(ctx: *mut c_void) {
        // SAFETY: ctx is the Box made below, run exactly once.
        let f: Box<Box<dyn FnOnce()>> = unsafe { Box::from_raw(ctx as *mut Box<dyn FnOnce()>) };
        f();
    }
    let boxed: Box<Box<dyn FnOnce()>> = Box::new(Box::new(f));
    // SAFETY: the main queue outlives the process; the context is consumed by the trampoline.
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, Box::into_raw(boxed) as *mut c_void, trampoline) };
}
