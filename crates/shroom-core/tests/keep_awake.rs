//! N41's keep-awake rule: “Uploads hold off idle sleep, never lid sleep.” The core asks the
//! shell for the assertion once when uploads can drain and lets go once when nothing can
//! — never again in between (the World remembers what it asked), with the laws checked at
//! every step.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::Effect;
use shroom_core::effect::ShellCmd;
use shroom_core::event::{Event, OsNews, UploadNews};
use shroom_core::time::Wall;
use shroom_core::world::*;
use support::*;

fn queued(dir: &str, rec: &str) -> Queued {
    Queued {
        dir: TakeDir(dir.into()),
        rec: RecId(rec.into()),
        title: "Untitled — Sep 30".into(),
        stopped: Wall(1_790_000_000_000),
        done: 0,
        total: 0,
        blocked: None,
        length_ms: Some(48_000),
    }
}

/// Every keep-awake ask over a run of events, in order.
fn asks(r: &mut Run, events: Vec<Event>) -> Vec<bool> {
    let mut out = Vec::new();
    for e in events {
        r.send(e);
        out.extend(r.effects.iter().filter_map(|e| match e {
            Effect::Shell(ShellCmd::PowerAssertion(on)) => Some(*on),
            _ => None,
        }));
    }
    out
}

fn scanned(queue: Vec<Queued>) -> Event {
    Event::Scanned { saved: vec![], queue, removed: vec![] }
}

fn progress(dir: &str, done: u32) -> Event {
    Event::Upload(UploadNews::Progress { dir: TakeDir(dir.into()), done, total: 4 })
}

#[test]
fn a_drain_holds_idle_sleep_off_once_and_lets_go_once() {
    let mut r = Run::new(ready());
    let seen = asks(
        &mut r,
        vec![
            Event::Tick,
            // Stop published two takes; they drain one after the other.
            scanned(vec![queued("a", "ra"), queued("b", "rb")]),
            progress("a", 1),
            progress("a", 4),
            Event::Upload(UploadNews::Live(TakeDir("a".into()))),
            progress("b", 2),
            Event::Tick,
            Event::Upload(UploadNews::Live(TakeDir("b".into()))),
            Event::Upload(UploadNews::Idle),
            Event::Tick,
        ],
    );
    assert_eq!(seen, vec![true, false], "exactly one hold and one release");
    assert!(!r.w.mac.awake_held);
}

#[test]
fn nothing_to_drain_never_asks() {
    let mut r = Run::new(ready());
    let seen = asks(&mut r, vec![Event::Tick, scanned(vec![]), Event::Upload(UploadNews::Idle), Event::Tick]);
    assert!(seen.is_empty(), "{seen:?}");
}

#[test]
fn offline_lets_go_and_the_network_coming_back_holds_again() {
    let mut r = Run::new(ready());
    let seen = asks(
        &mut r,
        vec![
            scanned(vec![queued("a", "ra")]),
            Event::Os(OsNews::NetPath(false)),
            Event::Tick,
            Event::Os(OsNews::NetPath(true)),
            progress("a", 4),
            Event::Upload(UploadNews::Live(TakeDir("a".into()))),
        ],
    );
    assert_eq!(seen, vec![true, false, true, false]);
}

#[test]
fn a_space_that_refuses_the_take_never_holds_the_mac_awake() {
    let mut r = Run::new(ready());
    let seen = asks(
        &mut r,
        vec![
            scanned(vec![queued("a", "ra")]),
            Event::Upload(UploadNews::Blocked(TakeDir("a".into()), Blocked::Space)),
            Event::Upload(UploadNews::Idle),
            Event::Tick,
        ],
    );
    assert_eq!(seen, vec![true, false], "stranded takes can't drain");
}

#[test]
fn an_unprovisioned_mac_never_holds() {
    let mut w = ready();
    w.space.identity = None;
    let mut r = Run::new(w);
    let seen = asks(&mut r, vec![scanned(vec![queued("a", "ra")]), Event::Tick]);
    assert!(seen.is_empty(), "{seen:?}");
}

/// An upload that can't get through while macOS still calls the network up (a captive
/// portal, a dead link, an outage) waits its backoff without holding the Mac awake; the
/// next try that moves bytes holds it again.
#[test]
fn an_upload_failing_on_a_network_still_up_lets_go_until_bytes_move() {
    let mut r = Run::new(ready());
    let seen = asks(
        &mut r,
        vec![
            Event::Os(OsNews::NetPath(true)),
            scanned(vec![queued("a", "ra")]),
            Event::Upload(UploadNews::Blocked(TakeDir("a".into()), Blocked::Offline)),
            Event::Upload(UploadNews::Idle),
            Event::Tick,
        ],
    );
    assert_eq!(seen, vec![true, false], "waiting on the backoff never holds idle sleep off");
    assert_eq!(r.w.online(), Some(true), "the network still reads as up");
    let retry = r.w.inflight.failures[&shroom_core::world::Op::Upload].retry_at();
    r.tick_to(retry.0);
    let seen = asks(&mut r, vec![progress("a", 2), progress("a", 4), Event::Upload(UploadNews::Live(TakeDir("a".into())))]);
    assert_eq!(seen, vec![true, false], "bytes moving hold it again, the drain lets go");
}

/// One take waiting on its backoff doesn't let go while another still moves.
#[test]
fn a_blocked_take_beside_a_moving_one_keeps_the_hold() {
    let mut r = Run::new(ready());
    let seen = asks(
        &mut r,
        vec![
            scanned(vec![queued("a", "ra"), queued("b", "rb")]),
            Event::Upload(UploadNews::Blocked(TakeDir("a".into()), Blocked::Offline)),
            progress("b", 2),
        ],
    );
    assert_eq!(seen, vec![true]);
    assert!(r.w.mac.awake_held);
}
