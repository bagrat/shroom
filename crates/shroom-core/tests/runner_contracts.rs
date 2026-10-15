//! What the runner's lanes leave on disk that the core must know without a relaunch —
//! facts the Region 6 member lane relies on (WORKER-INTERFACE §5, §6.11).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{Effect, MemberCmd};
use shroom_core::event::{CloudNews, Event};
use shroom_core::world::*;
use support::*;

/// Wiring's last step writes `device.key` before enroll (§6.11): the Mac that just set its
/// space up holds a device key from that answer on, so the Members tab reads its rows
/// without waiting for the next launch's `Booted`.
#[test]
fn wiring_leaves_this_mac_a_device_key() {
    let mut w = ready();
    w.space.identity = None;
    w.space.device_key = false;
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Wired(Ok(identity()))));
    assert!(r.w.space.device_key, "the runner wrote device.key before enroll");
    assert_eq!(r.w.space.identity.as_ref().map(|i| i.role), Some(Role::TokenHolder));
    // An answer from before a Disconnect (no token) changes nothing.
    let mut w = ready();
    w.space.identity = None;
    w.space.token = false;
    w.space.device_key = false;
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Wired(Ok(identity()))));
    assert!(!r.w.space.device_key && r.w.space.identity.is_none());
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Member(MemberCmd::Roster))));
}
