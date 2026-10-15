//! macOS's will-sleep, held just long enough for the take to pause (ARCHITECTURE §3.3's
//! power row, DECISIONS B92; N39 `mac-away`).
//!
//! NSWorkspace's will-sleep notification (`observe.rs`) is a courtesy: macOS doesn't wait
//! for it, so a lid closed mid-take could sleep before the pause lands — safe by law 2 (the
//! take's folder is a crash-recovered take on the next launch), but not the pause the spec
//! draws. IOKit's system-power port is the one macOS waits on: `kIOMessageSystemWillSleep`
//! must be answered with `IOAllowPowerChange` (or macOS waits up to 30 s on its own).
//!
//! So, on a thread of its own with a run loop: at will-sleep this posts the session change
//! and `Os(WillSleep)`, and keeps the notification's id. The core answers with
//! `ShellCmd::AllowSleep` the moment the take is paused — or no take is open — and at the
//! latest two seconds on (`derive::SLEEP_HOLD_MS`); the shell then lets macOS go. Should the
//! core never answer (a wedged process), the hold's own cap lets it go anyway: shroom never
//! holds sleep beyond the pause. Idle sleep's question (`kIOMessageCanSystemSleep`) is never
//! vetoed — uploads hold off idle sleep with their own assertion (N41, `os.rs`).
//!
//! The tests inject at the callback (`handle`): this Mac is never put to sleep.

use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;

use shroom_core::event::{Event, OsNews};

use super::Runtime;

/// `iokit_common_msg(0x270)`: may the system idle-sleep? (Answered at once, never vetoed.)
pub const CAN_SYSTEM_SLEEP: u32 = 0xe000_0270;
/// `iokit_common_msg(0x280)`: the system will sleep — macOS waits for the answer.
pub const SYSTEM_WILL_SLEEP: u32 = 0xe000_0280;
/// `iokit_common_msg(0x300)`: the system woke (NSWorkspace's did-wake posts `Woke`).
pub const SYSTEM_HAS_POWERED_ON: u32 = 0xe000_0300;

/// The hold's own cap, past the core's two seconds: should the core never answer, macOS
/// still goes (it would wait up to 30 s on its own).
pub const HOLD_CAP: Duration = Duration::from_millis(2_500);

/// The will-sleep waiting on an answer: its notification id.
#[derive(Default)]
pub struct Hold {
    pending: Mutex<Option<isize>>,
}

impl Hold {
    pub const fn new() -> Hold {
        Hold { pending: Mutex::new(None) }
    }

    /// Lets macOS go, once: true when a will-sleep was waiting.
    pub fn release(&self, allow: &dyn Fn(isize)) -> bool {
        match self.pending.lock().unwrap().take() {
            Some(id) => {
                allow(id);
                true
            }
            None => false,
        }
    }

    /// The cap's release: only the will-sleep it was set for (a newer one keeps its own).
    fn release_if(&self, id: isize, allow: &dyn Fn(isize)) -> bool {
        let mut p = self.pending.lock().unwrap();
        if *p == Some(id) {
            *p = None;
            allow(id);
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    pub fn waiting(&self) -> bool {
        self.pending.lock().unwrap().is_some()
    }
}

static HOLD: Hold = Hold::new();
static RT: Mutex<Option<Runtime>> = Mutex::new(None);
/// The root power domain's connection (`io_connect_t`), once registered.
static ROOT: Mutex<u32> = Mutex::new(0);

/// One power message, as IOKit delivers it — the seam the tests inject at. `allow` is
/// `IOAllowPowerChange` on the root connection; `cap` how long before the hold's own
/// release (`HOLD_CAP`).
pub fn handle(hold: &'static Hold, rt: &Runtime, msg: u32, arg: isize, allow: fn(isize), cap: Duration) {
    match msg {
        CAN_SYSTEM_SLEEP => allow(arg),
        SYSTEM_WILL_SLEEP => {
            {
                let mut p = hold.pending.lock().unwrap();
                // An older will-sleep never answered (it can't be: macOS sent a new one) goes now.
                if let Some(old) = p.replace(arg) {
                    allow(old);
                }
            }
            rt.log("power: macOS will sleep — held for the take's pause");
            super::observe::asleep(rt);
            rt.post(Event::Os(OsNews::WillSleep));
            let rt = rt.clone();
            std::thread::Builder::new()
                .name("shroom-power-cap".into())
                .spawn(move || {
                    std::thread::sleep(cap);
                    if hold.release_if(arg, &allow) {
                        rt.log("power: sleep allowed (the hold's cap — the core never answered)");
                    }
                })
                .ok();
        }
        // NSWorkspace's did-wake posts the session and `Woke` (observe.rs): nothing to add.
        SYSTEM_HAS_POWERED_ON => {}
        _ => {}
    }
}

/// `ShellCmd::AllowSleep`: the core says the take is safe — macOS may sleep now.
pub fn allow_pending(rt: &Runtime) {
    if HOLD.release(&allow_now) {
        rt.log("power: sleep allowed");
    }
}

fn allow_now(id: isize) {
    let root = *ROOT.lock().unwrap();
    if root != 0 {
        // SAFETY: the root connection lives for the process; the id is macOS's own.
        unsafe { IOAllowPowerChange(root, id) };
    }
}

type NotificationPort = *mut c_void;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IORegisterForSystemPower(refcon: *mut c_void, port: *mut NotificationPort, callback: extern "C" fn(*mut c_void, u32, u32, *mut c_void), notifier: *mut u32) -> u32;
    fn IONotificationPortGetRunLoopSource(port: NotificationPort) -> *mut c_void;
    fn IOAllowPowerChange(kernel_port: u32, notification_id: isize) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    static kCFRunLoopDefaultMode: *const c_void;
}

extern "C" fn power_changed(_refcon: *mut c_void, _service: u32, msg: u32, arg: *mut c_void) {
    if let Some(rt) = RT.lock().unwrap().clone() {
        handle(&HOLD, &rt, msg, arg as isize, allow_now, HOLD_CAP);
    }
}

/// Registers for the system's power messages on a thread of its own. Call once.
pub fn watch(rt: Runtime) {
    *RT.lock().unwrap() = Some(rt.clone());
    std::thread::Builder::new()
        .name("shroom-power".into())
        .spawn(move || {
            let mut port: NotificationPort = std::ptr::null_mut();
            let mut notifier: u32 = 0;
            // SAFETY: IOKit's documented registration; the port and notifier live as long
            // as the process (never deregistered), and the callback only posts.
            let root = unsafe { IORegisterForSystemPower(std::ptr::null_mut(), &mut port, power_changed, &mut notifier) };
            if root == 0 || port.is_null() {
                rt.log("power: the system's power messages couldn't be observed — sleep pauses on NSWorkspace's word alone");
                return;
            }
            *ROOT.lock().unwrap() = root;
            unsafe {
                CFRunLoopAddSource(CFRunLoopGetCurrent(), IONotificationPortGetRunLoopSource(port), kCFRunLoopDefaultMode);
            }
            rt.log("power: will-sleep held for a take's pause (two seconds at most)");
            unsafe { CFRunLoopRun() };
        })
        .expect("the power thread starts");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicIsize, Ordering};

    /// The session is process-wide: the tests that move it take turns.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn rt() -> (tempfile::TempDir, Runtime, std::sync::mpsc::Receiver<shroom_core::time::Envelope>) {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        (tmp, rt, rx)
    }

    fn events(rx: &std::sync::mpsc::Receiver<shroom_core::time::Envelope>) -> Vec<Event> {
        rx.try_iter().map(|e| e.event).collect()
    }

    static ALLOWED_A: AtomicIsize = AtomicIsize::new(0);
    fn allow_a(id: isize) {
        ALLOWED_A.store(id, Ordering::SeqCst);
    }

    /// Will-sleep is held: the session goes asleep and the core hears `WillSleep`; macOS is
    /// answered only when the core says so, and only once.
    #[test]
    fn will_sleep_waits_for_the_cores_answer_and_is_answered_once() {
        static H: Hold = Hold::new();
        let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let (_tmp, rt, rx) = rt();
        handle(&H, &rt, SYSTEM_WILL_SLEEP, 41, allow_a, Duration::from_secs(60));
        let got = events(&rx);
        assert!(got.iter().any(|e| matches!(e, Event::Os(OsNews::Session(s)) if !s.awake)), "{got:?}");
        assert!(got.iter().any(|e| matches!(e, Event::Os(OsNews::WillSleep))), "{got:?}");
        assert!(H.waiting());
        assert_eq!(ALLOWED_A.load(Ordering::SeqCst), 0, "macOS waits");
        assert!(H.release(&allow_a));
        assert_eq!(ALLOWED_A.load(Ordering::SeqCst), 41);
        assert!(!H.release(&allow_a), "answered once");
        super::super::observe::awake(&rt);
    }

    static ALLOWED_B: AtomicIsize = AtomicIsize::new(0);
    fn allow_b(id: isize) {
        ALLOWED_B.store(id, Ordering::SeqCst);
    }

    /// A core that never answers never holds sleep past the cap.
    #[test]
    fn the_hold_lets_go_by_itself_at_its_cap() {
        static H: Hold = Hold::new();
        let _turn = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let (_tmp, rt, _rx) = rt();
        handle(&H, &rt, SYSTEM_WILL_SLEEP, 7, allow_b, Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(ALLOWED_B.load(Ordering::SeqCst), 7);
        assert!(!H.waiting());
        assert!(HOLD_CAP > Duration::from_millis(shroom_core::derive::SLEEP_HOLD_MS), "the core answers first");
        assert!(HOLD_CAP < Duration::from_secs(30), "well inside macOS's own wait");
        super::super::observe::awake(&rt);
    }

    static ALLOWED_C: AtomicIsize = AtomicIsize::new(0);
    fn allow_c(id: isize) {
        ALLOWED_C.store(id, Ordering::SeqCst);
    }

    /// Idle sleep's question is never vetoed, and posts nothing.
    #[test]
    fn idle_sleep_is_never_vetoed() {
        static H: Hold = Hold::new();
        let (_tmp, rt, rx) = rt();
        handle(&H, &rt, CAN_SYSTEM_SLEEP, 9, allow_c, HOLD_CAP);
        assert_eq!(ALLOWED_C.load(Ordering::SeqCst), 9);
        assert!(!H.waiting());
        handle(&H, &rt, SYSTEM_HAS_POWERED_ON, 0, allow_c, HOLD_CAP);
        assert!(events(&rx).is_empty());
    }
}
