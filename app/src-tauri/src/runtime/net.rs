//! The network path (ARCHITECTURE §3.1: `Net.online` from NWPathMonitor). One monitor for
//! the process, on its own dispatch queue; every change posts `Os(NetPath)`, and the
//! latest answer fills `QuickProbe.online` so an entry never opens on a stale guess.
//!
//! Choice (logged at start): NWPathMonitor over SCNetworkReachability — it is Apple's
//! current API, pushes changes instead of being polled, and its "satisfied" is the same
//! notion of online the system uses.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, Ordering};

use block2::RcBlock;
use shroom_core::event::{Event, OsNews};

use super::Runtime;

/// 0 = not answered yet, 1 = offline, 2 = online.
static ONLINE: AtomicU8 = AtomicU8::new(0);

/// The path's last answer, for the QuickProbe.
pub fn online() -> Option<bool> {
    match ONLINE.load(Ordering::SeqCst) {
        1 => Some(false),
        2 => Some(true),
        _ => None,
    }
}

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {
    fn nw_path_monitor_create() -> *mut c_void;
    fn nw_path_monitor_set_update_handler(monitor: *mut c_void, handler: &block2::Block<dyn Fn(*mut c_void)>);
    fn nw_path_monitor_set_queue(monitor: *mut c_void, queue: *mut c_void);
    fn nw_path_monitor_start(monitor: *mut c_void);
    fn nw_path_get_status(path: *mut c_void) -> i32;
}

unsafe extern "C" {
    fn dispatch_queue_create(label: *const std::ffi::c_char, attr: *mut c_void) -> *mut c_void;
}

/// `nw_path_status_satisfied`.
const SATISFIED: i32 = 1;

/// Starts the monitor; it lives as long as the process.
pub fn watch(rt: Runtime) {
    rt.log("net: NWPathMonitor (the network path, pushed on change)");
    let rt2 = rt.clone();
    let handler = RcBlock::new(move |path: *mut c_void| {
        let on = unsafe { nw_path_get_status(path) } == SATISFIED;
        let now = if on { 2 } else { 1 };
        if ONLINE.swap(now, Ordering::SeqCst) != now {
            rt2.log(&format!("net: {}", if on { "online" } else { "offline" }));
            rt2.post(Event::Os(OsNews::NetPath(on)));
        }
    });
    unsafe {
        let monitor = nw_path_monitor_create();
        if monitor.is_null() {
            rt.log("net: no path monitor");
            return;
        }
        let queue = dispatch_queue_create(c"am.shroom.net".as_ptr(), std::ptr::null_mut());
        nw_path_monitor_set_update_handler(monitor, &handler);
        nw_path_monitor_set_queue(monitor, queue);
        nw_path_monitor_start(monitor);
        // The monitor and its queue are never released: one per process (the handler
        // block is copied by Network itself).
    }
}
