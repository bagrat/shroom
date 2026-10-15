//! What runs behind the person's back, and how far it may go (the whole-spec audit's final
//! cluster): the token Mac's cleanup sweep (DECISIONS B91; ARCHITECTURE §5.5, §16 #49) and
//! the hold on macOS's will-sleep (B92; ARCHITECTURE §3.3's power row, N39 `mac-away`).
//! Walkthroughs, with the laws — and the runner's own dev-build law check (B93) — held at
//! every step.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::derive::{CLEANUP_FIRST_MS, CLEANUP_MS, SLEEP_HOLD_MS};
use shroom_core::effect::{Effect, LibCmd, RecCmd, ShellCmd};
use shroom_core::event::{CloudNews, Event, LibraryNews, OsNews, RecNews, SweepAnswer};
use shroom_core::time::Mono;
use shroom_core::world::*;
use support::*;

const HOUR: u64 = 3_600_000;

fn booted(mut w: World) -> World {
    w.launch.version = "0.1.3".into();
    w.once.seen_version = Some("0.1.3".into());
    w.space.device_key = true;
    w
}

fn sound_pass() -> SweepAnswer {
    SweepAnswer {
        token: Some(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "Bagrat's Account".into() }, several: false }),
        gates: Some(Gates { email: false, card: false }),
        subdomain: Some(Some("myshroom".into())),
        foreign_v: Some(false),
        label: Some(Some("v".into())),
        bucket: Some(true),
        worker: Some(WorkerFact::Ours),
        found: Some(Some(3)),
    }
}

fn cleanups(r: &Run) -> usize {
    r.effects.iter().filter(|e| matches!(e, Effect::Library(LibCmd::Cleanup))).count()
}

fn cleaned() -> Event {
    Event::Library(LibraryNews::Cleaned { under_tombstones: 1, orphans: 0, keys: 3 })
}

/// A token Mac booted, its launch's whole pass answered sound.
fn proved() -> Run {
    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    r.send(Event::Cloud(CloudNews::Swept(sound_pass())));
    assert!(r.w.space.broken.is(&false));
    r
}

// ── B91: the cleanup sweep ──

#[test]
fn the_token_mac_sweeps_ten_minutes_after_its_launch_check_then_every_six_hours() {
    let mut r = proved();
    let pass = r.w.sweep.pass.unwrap().0;
    r.tick_to(pass + CLEANUP_FIRST_MS - 1_000);
    assert_eq!(cleanups(&r), 0, "never in the launch's own burst");
    r.tick_to(pass + CLEANUP_FIRST_MS);
    assert_eq!(cleanups(&r), 1, "the first sweep, with no window open");
    assert!(r.scene.popover.is_none() && r.scene.library.is_none(), "nothing on screen moves");
    // Settling again asks nothing (it's in flight).
    r.send(Event::Tick);
    assert_eq!(cleanups(&r), 0);
    r.send(cleaned());
    let at = r.w.library_ops.cleaned.unwrap().0;
    r.tick_to(at + HOUR);
    assert_eq!(cleanups(&r), 0);
    r.tick_to(at + CLEANUP_MS);
    assert_eq!(cleanups(&r), 1, "six hours of awake time on");
}

#[test]
fn a_sweep_that_couldnt_run_waits_as_long_as_a_sweep() {
    let mut r = proved();
    let pass = r.w.sweep.pass.unwrap().0;
    r.tick_to(pass + CLEANUP_FIRST_MS);
    assert_eq!(cleanups(&r), 1);
    r.send(Event::Failed { op: Op::Cleanup, class: FailClass::Transport });
    let failed = r.w.inflight.failures[&Op::Cleanup].at.0;
    r.tick_to(failed + 10 * 60_000);
    assert_eq!(cleanups(&r), 0, "fail-soft: never the backoff's two minutes");
    r.tick_to(failed + CLEANUP_MS);
    assert_eq!(cleanups(&r), 1);
    // Its deadline passing is a failure like any other, never a stuck sweep.
    let deadline = r.w.inflight.ops[&Op::Cleanup].0;
    r.tick_to(deadline);
    assert!(!r.w.inflight.ops.contains_key(&Op::Cleanup));
    assert!(r.w.inflight.failures.contains_key(&Op::Cleanup));
}

#[test]
fn only_the_token_mac_sweeps_and_never_over_a_take_offline_or_a_broken_space() {
    // No whole pass yet: nothing.
    let mut r = Run::new(booted(ready()));
    r.tick_to(CLEANUP_FIRST_MS * 3);
    assert_eq!(r.w.library_ops.cleaned, None);
    assert!(shroom_core::derive::next_cleanup(&r.w).is_none(), "no pass has proved the space");

    // A pass that found the space broken: nothing until one finds it sound.
    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    r.send(Event::Cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::Absent), ..sound_pass() })));
    assert!(r.w.space.broken.is(&true));
    assert!(shroom_core::derive::next_cleanup(&r.w).is_none());

    // A member's Mac never sweeps.
    let mut w = booted(ready());
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.sweep.pass = Some(Mono(1_000));
    let mut r = Run::new(w);
    r.tick_to(CLEANUP_FIRST_MS * 3);
    assert!(shroom_core::derive::next_cleanup(&r.w).is_none());

    // Offline: it waits for the network.
    let mut r = proved();
    let pass = r.w.sweep.pass.unwrap().0;
    r.send(Event::Os(OsNews::NetPath(false)));
    r.tick_to(pass + CLEANUP_FIRST_MS);
    assert_eq!(cleanups(&r), 0);
    r.send(Event::Os(OsNews::NetPath(true)));
    assert_eq!(cleanups(&r), 1, "the moment the network is back");

    // A take open: it waits for the take to end (its bandwidth is the take's).
    let mut r = proved();
    let pass = r.w.sweep.pass.unwrap().0;
    r.w.take = Some(take(Phase::Recording, 1_000));
    r.scene = shroom_core::derive(&r.w);
    r.tick_to(pass + CLEANUP_FIRST_MS + 1_000);
    assert_eq!(cleanups(&r), 0, "never over a take");
}

#[test]
fn a_sweep_answering_after_disconnect_is_about_a_space_this_mac_no_longer_knows() {
    let mut r = proved();
    let pass = r.w.sweep.pass.unwrap().0;
    r.tick_to(pass + CLEANUP_FIRST_MS);
    assert_eq!(cleanups(&r), 1);
    r.w.space.identity = None;
    r.w.space.token = false;
    r.scene = shroom_core::derive(&r.w);
    r.send(cleaned());
    assert_eq!(r.arm.name, "library-stale");
    assert!(!r.w.inflight.ops.contains_key(&Op::Cleanup));
    assert_eq!(r.w.library_ops.cleaned, None);
}

// ── B92: the will-sleep hold ──

fn sleeps(r: &Run) -> usize {
    r.effects.iter().filter(|e| matches!(e, Effect::Shell(ShellCmd::AllowSleep))).count()
}

fn recording() -> Run {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 30_000));
    Run::new(w)
}

#[test]
fn will_sleep_pauses_the_take_and_lets_the_mac_sleep_once_the_pause_lands() {
    let mut r = recording();
    r.send(Event::Os(OsNews::WillSleep));
    assert_eq!(r.arm.edge, Some(("recording", "mac-away")), "paused at that instant (N39)");
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    assert_eq!(sleeps(&r), 0, "macOS waits for the pause");
    assert!(r.w.mac.sleep_hold.is_some());
    assert!(r.scene.next_wake.is_some_and(|t| t.0 <= r.t + SLEEP_HOLD_MS), "a wake at the hold's end at the latest");
    assert!(r.scene.popover.is_none(), "nothing shows while the Mac goes away");

    r.send(Event::Rec(RecNews::Paused { length_ms: 30_100 }));
    assert_eq!(sleeps(&r), 1, "the pause landed: sleep goes ahead");
    assert_eq!(r.w.mac.sleep_hold, None);
    r.send(Event::Tick);
    assert_eq!(sleeps(&r), 0, "answered once");
    assert!(matches!(r.w.take.as_ref().map(|t| t.phase), Some(Phase::Paused(Cause::Asleep))));

    // Back: mac-away's face, unasked (N39).
    r.send(Event::Os(OsNews::Session(OsSession::default())));
    r.send(Event::Os(OsNews::Woke));
    assert!(r.ids().contains(&"mac-away"), "{:?}", r.ids());
}

#[test]
fn a_pause_that_never_lands_never_holds_sleep_past_two_seconds() {
    let mut r = recording();
    r.send(Event::Os(OsNews::WillSleep));
    let asked = r.w.mac.sleep_hold.unwrap().0;
    r.tick_to(asked + SLEEP_HOLD_MS - 10);
    assert_eq!(sleeps(&r), 0);
    r.tick_to(asked + SLEEP_HOLD_MS);
    assert_eq!(sleeps(&r), 1, "sleep goes ahead whatever happened");
    // The take is still the take: law 2's crash-safe dir carries it either way.
    assert!(r.w.take.is_some());
}

#[test]
fn with_nothing_recording_sleep_is_answered_at_once() {
    let mut r = Run::new(ready());
    r.send(Event::Os(OsNews::WillSleep));
    assert_eq!(r.arm.name, "will-sleep");
    assert_eq!(sleeps(&r), 1);
    assert!(r.w.mac.os.get().is_some_and(|o| !o.awake));

    // A take already paused by the person: nothing to wait for either.
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 30_000));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::WillSleep));
    assert_eq!(sleeps(&r), 1);
    assert!(matches!(r.w.take.as_ref().map(|t| t.phase), Some(Phase::Paused(Cause::User))), "the person's own pause stays theirs");
}

#[test]
fn the_session_news_first_then_will_sleep_holds_the_same_pause_once() {
    // NSWorkspace's will-sleep may land before IOKit's: one pause, one answer.
    let mut r = recording();
    r.send(Event::Os(OsNews::Session(OsSession { awake: false, ..OsSession::default() })));
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    r.send(Event::Os(OsNews::WillSleep));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))), "never a second pause");
    assert_eq!(sleeps(&r), 0, "still pausing");
    r.send(Event::Rec(RecNews::Paused { length_ms: 30_000 }));
    assert_eq!(sleeps(&r), 1);
}

#[test]
fn a_stop_finalizing_at_will_sleep_is_waited_for_too() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 30_000));
    let mut r = Run::new(w);
    // Stop: the take finalizes, and the Mac starts to sleep meanwhile.
    r.w.take.as_mut().unwrap().phase = Phase::Finalizing(Why::Stop);
    r.scene = shroom_core::derive(&r.w);
    r.send(Event::Os(OsNews::WillSleep));
    assert_eq!(sleeps(&r), 0, "the finalize is still landing");
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))), "a stop is never downgraded to a pause");
    let asked = r.w.mac.sleep_hold.unwrap().0;
    r.tick_to(asked + SLEEP_HOLD_MS);
    assert_eq!(sleeps(&r), 1);
}
