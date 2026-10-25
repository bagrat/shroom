//! What the app says must match what is true (the whole-spec audit's b3-truth cluster,
//! DECISIONS B49–B52, DEVIATIONS #25, #26): the periodic self-check, the page that follows
//! the app, the stop's probe as the detector, the space-class publish failures, the
//! wizard's honest faces, and a removed member's videos already on their way. Walkthroughs,
//! with the laws checked at every step.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{CloudCmd, Effect, MemberCmd, ShellCmd, SweepPart, UpCmd};
use shroom_core::event::{CloudNews, Event, Field, MemberNews, OsNews, RecNews, StandingAnswer, SweepAnswer, TokenFail, UploadNews};
use shroom_core::event::Surface;
use shroom_core::scene::{Action, Nest, OnboardingBody, PopoverBody, SettingsPane};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

const HOUR: u64 = 3_600_000;

/// A token Mac that booted this version: its periodic reactions run.
fn booted(mut w: World) -> World {
    w.launch.version = "0.1.0".into();
    w.once.seen_version = Some("0.1.0".into());
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

fn swept(a: SweepAnswer) -> Event {
    Event::Cloud(CloudNews::Swept(a))
}

fn sweeps(r: &Run) -> usize {
    r.effects.iter().filter(|e| matches!(e, Effect::Cloud(CloudCmd::Sweep(SweepPart::Full)))).count()
}

fn popover_json(r: &Run) -> String {
    serde_json::to_string(r.scene.popover.as_ref().expect("a popover")).unwrap()
}

fn queued(dir: &str) -> Queued {
    Queued { dir: TakeDir(dir.into()), rec: RecId("AbCdEfGhIjKlMnOp".into()), title: "Untitled — Oct 2".into(), stopped: Wall(1_790_000_000_000), done: 0, total: 4, blocked: None, length_ms: Some(60_000) }
}

// ── (B) the periodic self-check: #3, #24 ──

#[test]
fn a_token_mac_re_proves_its_space_at_launch_and_every_six_hours_and_its_answer_is_the_banner() {
    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    assert_eq!(sweeps(&r), 1, "at launch, with no window open");

    // A whole pass that finds the page gone raises space-degraded — no Settings needed.
    r.send(swept(SweepAnswer { worker: Some(WorkerFact::Absent), ..sound_pass() }));
    assert!(r.w.space.broken.is(&true));
    r.click();
    assert!(popover_json(&r).contains("Your space needs you — new videos wait safely on this Mac."));
    r.send(Event::PopoverDismissed);

    // Nothing more until six hours of awake time have passed.
    let pass = r.w.sweep.pass.unwrap();
    r.tick_to(pass.0 + HOUR);
    assert_eq!(sweeps(&r), 0);
    r.tick_to(pass.0 + 6 * HOUR);
    assert_eq!(sweeps(&r), 1, "six hours on");
    // The page is back: the banner clears (law 1: the face clears when its cause does).
    r.send(swept(sound_pass()));
    assert!(r.w.space.broken.is(&false));
    r.click();
    assert!(!popover_json(&r).contains("Your space needs you"));
}

#[test]
fn the_self_check_is_fail_soft_and_only_a_token_macs() {
    // A pass that couldn't run waits as long as a check — never a loop.
    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    assert_eq!(sweeps(&r), 1);
    r.send(Event::Failed { op: Op::Sweep, class: FailClass::Unclassified });
    r.tick_to(r.t + 10 * 60_000);
    assert_eq!(sweeps(&r), 0, "not on the backoff's two minutes");
    assert!(shroom_core::derive::next_self_check(&r.w).unwrap().0 >= r.w.inflight.failures[&Op::Sweep].at.0 + 6 * HOUR);

    // Offline: nothing is asked.
    let mut w = booted(ready());
    w.net.online = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert_eq!(sweeps(&r), 0);

    // A member's Mac never runs the token's pass.
    let mut w = booted(ready());
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert_eq!(sweeps(&r), 0);
}

// ── (A) the page follows the app: #5, #21, #23, #27 ──

#[test]
fn a_token_mac_refreshes_its_page_once_a_launch_and_a_failed_refresh_waits_an_hour() {
    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)), "at launch — so after every update");
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: true, ai: None }));
    r.tick_to(r.t + 10 * HOUR);
    assert!(!r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)), "once a launch");

    let mut r = Run::new(booted(ready()));
    r.send(Event::Tick);
    r.send(Event::Failed { op: Op::Refresh, class: FailClass::Unclassified });
    assert_eq!(r.w.space.page_checked, None, "a refresh that couldn't run isn't this launch's check");
    r.tick_to(r.t + 30 * 60_000);
    assert!(!r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)), "not before the hour");
    r.tick_to(r.w.inflight.failures[&Op::Refresh].at.0 + HOUR);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)), "an hour on");
    // A transport failure is behind the person the moment the network is back.
    r.send(Event::Failed { op: Op::Refresh, class: FailClass::Transport });
    r.send(Event::Os(OsNews::NetPath(false)));
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)), "back online");

    // Only the token's Mac deploys a page.
    let mut w = booted(ready());
    w.space.identity = Some(SpaceIdentity { role: Role::OwnerClass, ..identity() });
    w.space.token = false;
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!r.has(|e| *e == Effect::Cloud(CloudCmd::Refresh)));
}

// ── (F) stop truth: #15, #16, #20 ──

fn stopping() -> Run {
    let mut w = booted(ready());
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: None }));
    r.send(swept(sound_pass()));
    r.act(Action::Stop);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Reach)));
    r
}

#[test]
fn a_stop_whose_probe_cant_reach_the_page_while_online_keeps_the_link_and_opens_the_page_later() {
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert!(r.scene.popover.is_none(), "the probe hasn't answered");
    r.send(Event::Cloud(CloudNews::Reached(Reach::Unreachable)));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline", "macOS says online, but the page can't be reached");
    assert!(popover_json(&r).contains("v.myshroom.workers.dev/AbCdEfGhIjKlMnOp"));

    // Re-probed on a backoff — not only on a network change — and the page opens itself.
    r.send(Event::PopoverDismissed);
    let retry = r.w.inflight.failures[&Op::Reach].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Reach)), "asked again on the backoff");
    r.send(Event::Cloud(CloudNews::Reached(Reach::Unreachable)));
    assert!(r.scene.popover.is_none(), "a later miss re-probes quietly — the popover doesn't pop again");
    let retry = r.w.inflight.failures[&Op::Reach].retry_at();
    r.tick_to(retry.0);
    r.send(Event::Cloud(CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))), "the page opens by itself");
}

#[test]
fn the_stop_probes_deadline_is_the_same_face() {
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    let deadline = r.w.inflight.ops[&Op::Reach];
    r.tick_to(deadline.0);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline");
}

#[test]
fn the_offline_face_clears_when_its_cause_does() {
    // macOS's own offline stop: back online, the popover falls back to config.
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::NetPath(false)));
    r.act(Action::Stop);
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline");
    r.send(Event::Os(OsNews::NetPath(true)));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "config", "“You're offline…” is no longer true");
    r.send(Event::Cloud(CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))));

    // The probe's miss, then the page opened: behind us too.
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    r.send(Event::Cloud(CloudNews::Reached(Reach::Unreachable)));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline");
    r.send(Event::Cloud(CloudNews::PageOpened(RecId("AbCdEfGhIjKlMnOp".into()))));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "config");
}

/// The stop's probe keeps answering “not shroom” until the verdict holds (B60): each
/// unconfirmed answer schedules a wake a few seconds on, the probe is asked again there, and
/// nothing strands before the last answer of the run.
fn hold_not_shroom(r: &mut Run) {
    while !r.w.net.not_shroom_held() {
        assert_ne!(r.scene.popover.as_ref().map(|p| p.id), Some("stopped-stranded"), "one answer never strands a stop");
        let again = r.w.net.not_shroom_reprobe().expect("an unconfirmed answer asks again");
        assert_eq!(r.scene.next_wake, Some(again), "the runner is woken for the re-probe");
        r.tick_to(again.0);
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Reach)), "asked again");
        r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    }
}

#[test]
fn a_token_macs_stop_whose_page_is_gone_is_stranded_with_reconnect_your_space() {
    // The probe answers before the finalize…
    let mut r = stopping();
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert!(r.has(|e| matches!(e, Effect::Uploader(UpCmd::Kick))), "the take still goes up over the token's own keys");
    assert!(r.scene.popover.is_none(), "one answer isn't a verdict (B60)");
    assert!(!r.w.inflight.ops.contains_key(&Op::Sweep), "nor a finding the self-check runs on");
    hold_not_shroom(&mut r);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-stranded");
    let p = popover_json(&r);
    assert!(p.contains("Your recording is safe on this Mac."));
    assert!(p.contains("Your space needs you — new videos wait safely on this Mac."), "the degraded banner's reason line");
    assert!(p.contains("Reconnect your space"));
    assert!(r.w.session.page_open.is_none(), "no page to open");
    // The self-check looks at once.
    assert!(r.w.inflight.ops.contains_key(&Op::Sweep), "the stop's finding is re-proved now, not in six hours");
    // Reconnect your space → Settings · Space, brought forward (#11).
    r.act(Action::ReconnectSpace);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenSettings)));
    assert_eq!(r.w.prefs.settings_tab, Some(SettingsTab::Space));

    // …or after it.
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    hold_not_shroom(&mut r);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-stranded");
}

#[test]
fn a_freshly_wired_address_answering_not_shroom_once_never_strands_the_stop() {
    // The heartbeat's finding (tests-sys #7): a fresh workers.dev address answered “not
    // shroom” once, then as the space. The page opens, as a live stop's would.
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    assert!(r.scene.popover.is_none(), "no stranded face on one answer");
    assert!(r.w.session.page_open.is_some(), "the page still waits to open");
    let again = r.w.net.not_shroom_reprobe().unwrap();
    r.tick_to(again.0);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Reach)));
    r.send(Event::Cloud(CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))), "the page opens");
    assert!(r.w.net.not_shroom.is_none(), "the run is over");

    // Two “not shroom”, an unreachable answer, then the space: still never stranded, and the
    // unreachable answer is the offline face's as ever (it neither counts nor breaks the run).
    let mut r = stopping();
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    r.tick_to(r.w.net.not_shroom_reprobe().unwrap().0);
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    assert_eq!(r.w.net.not_shroom.unwrap().count, 2);
    r.tick_to(r.w.net.not_shroom_reprobe().unwrap().0);
    r.send(Event::Cloud(CloudNews::Reached(Reach::Unreachable)));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline");
    assert_eq!(r.w.net.not_shroom.unwrap().count, 2, "kept");
    let retry = r.w.inflight.failures[&Op::Reach].retry_at();
    r.tick_to(retry.0);
    r.send(Event::Cloud(CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))));

    // A new stop starts its own run: earlier answers never count toward it.
    let mut w = booted(ready());
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    assert_eq!(r.w.net.not_shroom.unwrap().count, 2);
    r.act(Action::Stop);
    assert!(r.w.net.not_shroom.is_none(), "Stop clears the run");
}

#[test]
fn a_members_stop_whose_page_is_gone_is_stranded_in_the_members_words() {
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.space.device_key = true;
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.act(Action::Stop);
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    r.send(Event::Cloud(CloudNews::Reached(Reach::NotShroom)));
    hold_not_shroom(&mut r);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-stranded");
    let p = popover_json(&r);
    assert!(p.contains("Bagrat's space isn't there anymore"), "{p}");
    assert!(!p.contains("Reconnect your space"), "owners only");
}

// ── (G) space-class publish failures: #2, #19 ──

#[test]
fn a_space_class_publish_failure_raises_degraded_reports_blocked_and_never_loops() {
    let mut w = booted(ready());
    w.queue = vec![queued("q1")];
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: None }));
    r.send(swept(sound_pass()));
    assert!(r.w.inflight.ops.contains_key(&Op::Upload), "the take is being sent");
    // The token Cloudflare no longer accepts: the uploader says so about the waiting take.
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space)));
    assert!(r.w.space.broken.is(&true));
    assert!(r.has(|e| matches!(e, Effect::Member(MemberCmd::Health { refused: true, .. }))), "the owner page learns “Waiting for your space”");
    r.send(Event::Upload(UploadNews::Idle));
    assert!(!r.has(|e| *e == Effect::Uploader(UpCmd::Kick)), "no Kick loop");
    // The next upload that lands says ok, once.
    r.send(Event::Upload(UploadNews::Progress { dir: TakeDir("q1".into()), done: 1, total: 4 }));
    assert!(r.has(|e| matches!(e, Effect::Member(MemberCmd::Health { refused: false, .. }))));
}

#[test]
fn an_uploader_that_stops_with_takes_waiting_waits_its_backoff() {
    let mut w = ready();
    w.queue = vec![queued("q1")];
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Uploader(UpCmd::Kick)));
    r.send(Event::Upload(UploadNews::Idle));
    assert!(!r.has(|e| *e == Effect::Uploader(UpCmd::Kick)), "never straight back");
    let retry = r.w.inflight.failures[&Op::Upload].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| *e == Effect::Uploader(UpCmd::Kick)), "after the backoff");
    // A context that couldn't be had at all is a failure too.
    r.send(Event::Failed { op: Op::Upload, class: FailClass::Refused });
    assert!(!r.has(|e| *e == Effect::Uploader(UpCmd::Kick)));
}

// ── (I) the wizard's truth: #6–#12 ──

fn wizard() -> World {
    let mut w = World::blank();
    w.launch.version = "0.1.0".into();
    w.prefs.setup_chosen = true;
    w.session.fork = Fork::Affirmed;
    w.net.online = Fact::known(true, Mono(0));
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(0)));
    w.now = Mono(1_000);
    w
}

fn plan(r: &Run) -> &shroom_core::scene::PlanView {
    match &r.scene.onboarding.as_ref().expect("the window").body {
        OnboardingBody::Plan(p) => p,
        b => panic!("not the plan: {b:?}"),
    }
}

#[test]
fn a_token_verify_rejects_reads_garbled_never_wrong_powers() {
    let mut r = Run::new(wizard());
    r.send(Event::SecretPasted { field: shroom_core::event::SecretField::Token, id: SecretId(1), shape: shroom_core::event::Shape::Token });
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Validate(_)))));
    r.send(Event::Cloud(CloudNews::Validated { id: SecretId(1), result: Err(TokenFail::Rejected) }));
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "token-garbled");
    assert!(serde_json::to_string(plan(&r)).unwrap().contains("Tokens sometimes lose characters in the copy — copy it once more."));
}

#[test]
fn an_unclassified_wizard_failure_has_a_face_and_try_again_asks_again_now() {
    let mut r = Run::new(wizard());
    r.send(Event::SecretPasted { field: shroom_core::event::SecretField::Token, id: SecretId(1), shape: shroom_core::event::Shape::Token });
    r.send(Event::Failed { op: Op::Validate, class: FailClass::Unclassified });
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "wizard-unexpected");
    let Nest::Unexpected { line, button } = &plan(&r).nest else { panic!("{:?}", plan(&r).nest) };
    assert_eq!(line, "Something unexpected got in the way — everything so far is saved. I'll keep trying.");
    assert_eq!(button.action, Action::TryAgain);
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Validate(_)))), "the backoff holds");
    r.send(Event::Act { surface: Surface::Onboarding, action: Action::TryAgain, rev: 0 });
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Validate(_)))), "asked again now");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "token", "its spinner again");

    // Cloudflare busy (429/5xx → a timeout) is a calm retry, never the face.
    r.send(Event::Failed { op: Op::Validate, class: FailClass::Timeout });
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "token");
}

#[test]
fn a_claim_that_fails_unexpectedly_says_so_and_keeps_claim() {
    let mut w = wizard();
    w.space.token = true;
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "A".into() }, several: false }, Mono(10));
    w.sweep.gates = Fact::known(Gates::default(), Mono(10));
    w.sweep.subdomain = Fact::known(None, Mono(10));
    w.sweep.label = Fact::known(Some("v".into()), Mono(10));
    w.sweep.pass = Some(Mono(10));
    let mut r = Run::new(w);
    r.send(Event::Field { field: Field::Name, value: "myshroom".into() });
    r.send(Event::Act { surface: Surface::Onboarding, action: Action::Claim, rev: 0 });
    r.send(Event::Failed { op: Op::Claim, class: FailClass::Unclassified });
    let Nest::Name { unexpected, claim, .. } = &plan(&r).nest else { panic!("{:?}", plan(&r).nest) };
    assert!(unexpected.is_some(), "never a silent nothing");
    assert!(claim.enabled, "Claim stays the way on");
}

#[test]
fn space_found_ticks_rows_three_to_five_then_row_six_becomes_current() {
    let mut w = wizard();
    w.space.token = true;
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "A".into() }, several: false }, Mono(10));
    w.sweep.gates = Fact::known(Gates::default(), Mono(10));
    w.sweep.subdomain = Fact::known(Some("myshroom".into()), Mono(10));
    w.sweep.label = Fact::known(Some("v".into()), Mono(10));
    w.sweep.found = Fact::known(Some(14), Mono(10));
    w.sweep.pass = Some(Mono(10));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "space-found");
    let p = plan(&r);
    // The wiring row is drawn last, the permissions before it (N45).
    assert_eq!(p.current, Some(6), "the found line under the wiring row");
    assert!([2, 3, 5].iter().all(|&i| p.rows[i].glyph == shroom_core::scene::Glyph::Done), "rows three to five ticked");
    assert_eq!(p.rows[4].glyph, shroom_core::scene::Glyph::Ahead, "the permissions still ahead");
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "joining it (the re-place, the enroll)");
    r.send(Event::Cloud(CloudNews::Wired(Ok(identity()))));
    assert_eq!(r.scene.onboarding.as_ref().unwrap().id, "done", "row six becomes current");
    assert_eq!(plan(&r).current, Some(5), "drawn fifth (N45)");
}

#[test]
fn the_name_hint_is_for_a_character_outside_the_set_never_for_length() {
    let mut w = wizard();
    w.space.token = true;
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "A".into() }, several: false }, Mono(10));
    w.sweep.gates = Fact::known(Gates::default(), Mono(10));
    w.sweep.subdomain = Fact::known(None, Mono(10));
    w.sweep.label = Fact::known(Some("v".into()), Mono(10));
    w.sweep.pass = Some(Mono(10));
    let mut r = Run::new(w);
    r.send(Event::Field { field: Field::Name, value: "a".repeat(70) });
    assert_eq!(r.w.session.fields.name.len(), 63, "cut silently");
    assert!(r.w.session.fields.name_hint.is_none(), "no charset line for a valid character");
    r.send(Event::Field { field: Field::Name, value: "My_Shroom".into() });
    assert_eq!(r.w.session.fields.name, "myshroom");
    assert!(r.w.session.fields.name_hint.is_some(), "the underscore never lands — the quiet line");
}

fn settings_space(r: &Run) -> &shroom_core::scene::SpaceBody {
    match &r.scene.settings.as_ref().expect("Settings").body.pane {
        SettingsPane::Space(s) => s,
        p => panic!("not Space: {p:?}"),
    }
}

#[test]
fn set_it_up_fresh_shows_the_wiring_and_a_second_click_adds_nothing() {
    let mut w = booted(ready());
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: None }));
    r.send(swept(SweepAnswer { bucket: Some(false), found: Some(None), ..sound_pass() }));
    assert!(matches!(settings_space(&r).plan.nest, Nest::StorageGone { .. }));
    r.send(Event::Act { surface: Surface::Settings, action: Action::SetUpFresh, rev: 0 });
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
    assert!(matches!(settings_space(&r).plan.nest, Nest::Calm { .. }), "the calm line while it wires");
    r.send(Event::Act { surface: Surface::Settings, action: Action::SetUpFresh, rev: 0 });
    assert!(r.effects.is_empty(), "a second click adds nothing");
    r.send(Event::Cloud(CloudNews::Wired(Ok(identity()))));
    assert!(!matches!(settings_space(&r).plan.nest, Nest::StorageGone { .. } | Nest::Calm { .. }));
}

// ── (K) a removed member's videos already on their way: #22 ──

#[test]
fn a_removed_members_takes_with_a_minted_credential_still_finish() {
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.space.device_key = true;
    w.queue = vec![queued("q-minted"), Queued { dir: TakeDir("q-new".into()), rec: RecId("QrStUvWxYz012345".into()), ..queued("x") }];
    let mut r = Run::new(w);
    // Its credential runs for days yet; the other take never got one.
    r.send(Event::Upload(UploadNews::Credentials(vec![(TakeDir("q-minted".into()), Wall(1_790_000_000_000 + 5 * 86_400_000))])));
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert!(r.has(|e| *e == Effect::Uploader(UpCmd::Kick)) || r.w.inflight.ops.contains_key(&Op::Upload), "the minted take keeps going");
    let stranded: Vec<String> = shroom_core::derive::settings::stranded(&r.w).iter().map(|q| q.dir.0.clone()).collect();
    assert_eq!(stranded, ["q-new"], "only the take that needs a new mint is stranded");
    assert!(r.w.mac.awake_held, "a take actually moving holds off idle sleep");

    // Once it's done, nothing else is asked of a space that removed this Mac.
    r.send(Event::Upload(UploadNews::Live(TakeDir("q-minted".into()))));
    r.send(Event::Upload(UploadNews::Idle));
    r.tick_to(r.t + HOUR);
    assert!(!r.has(|e| *e == Effect::Uploader(UpCmd::Kick)));

    // A credential with under a quarter of an hour left counts as none (the uploader
    // re-mints those; a credential lives an hour).
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.space.device_key = true;
    w.queue = vec![queued("q-old")];
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Credentials(vec![(TakeDir("q-old".into()), Wall(1_790_000_000_000 + 10 * 60_000))])));
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert_eq!(shroom_core::derive::settings::stranded(&r.w).len(), 1);
    let _ = PopoverBody::Config;
}

/// The skeptic's window (#22): a take recorded offline gets its credential after the
/// launch's scan; the space then removes this Mac and the upload meets a transient
/// failure. The uploader's own report keeps it on its way — no relaunch needed — and a
/// scan that raced the mint never takes it back; a credential the space refused does.
#[test]
fn a_take_minted_after_the_scan_still_finishes_once_the_mac_is_removed() {
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.space.device_key = true;
    w.queue = vec![queued("q-offline")];
    let mut r = Run::new(w);
    let days = Wall(1_790_000_000_000 + 5 * 86_400_000);
    // The launch's scan: no credential yet (recorded offline).
    r.send(Event::Upload(UploadNews::Credentials(vec![])));
    // Online now: the uploader mints, sends a little, then the network blips.
    r.send(Event::Upload(UploadNews::Minted { dir: TakeDir("q-offline".into()), until: Some(days) }));
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q-offline".into()), Blocked::Offline)));
    r.send(Event::Upload(UploadNews::Idle));
    // A scan that read the disk before the mint landed lands after it.
    r.send(Event::Upload(UploadNews::Credentials(vec![])));
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert!(shroom_core::derive::settings::stranded(&r.w).is_empty(), "on its way, never stranded");
    // Its backoff (2 min at most), not the next launch.
    r.tick_to(r.t + 3 * 60_000);
    assert!(r.has(|e| *e == Effect::Uploader(UpCmd::Kick)), "it resumes this launch");

    // A credential the space refused is dropped: that take waits, stranded.
    r.send(Event::Upload(UploadNews::Minted { dir: TakeDir("q-offline".into()), until: None }));
    assert_eq!(shroom_core::derive::settings::stranded(&r.w).len(), 1);
}

/// A credential lives an hour (re-minted while the upload runs): once this Mac is removed, a
/// take still sends while its credential has more than a quarter of an hour left; past that
/// it waits, stranded, as one with none does.
#[test]
fn once_removed_a_take_sends_only_while_its_credential_has_time_left() {
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), ..identity() });
    w.space.token = false;
    w.space.device_key = true;
    w.queue = vec![queued("q-left"), queued("q-near")];
    let mut r = Run::new(w);
    let now = r.w.wall.0;
    r.send(Event::Upload(UploadNews::Minted { dir: TakeDir("q-left".into()), until: Some(Wall(now + 40 * 60_000)) }));
    r.send(Event::Upload(UploadNews::Minted { dir: TakeDir("q-near".into()), until: Some(Wall(now + 10 * 60_000)) }));
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    let stranded: Vec<String> = shroom_core::derive::settings::stranded(&r.w).iter().map(|q| q.dir.0.clone()).collect();
    assert_eq!(stranded, ["q-near"], "forty minutes left still sends; ten doesn't");
    assert_eq!(shroom_core::derive::members::CRED_LEFT_MS, 15 * 60_000);
}

// ── #1: a provisioned space's page is someone else's: never wired over ──

#[test]
fn a_space_whose_own_label_reads_foreign_rests_and_is_never_rewired() {
    let mut w = booted(ready());
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: None }));
    // The runner probes the space's own label (`v`): someone else's `v` is there now.
    r.send(swept(SweepAnswer { worker: Some(WorkerFact::Foreign), ..sound_pass() }));
    assert!(r.w.space.broken.is(&true), "the space can't take videos");
    assert!(matches!(settings_space(&r).plan.nest, Nest::Calm { .. }), "wiring's calm line, at rest");
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "never over a Worker that isn't shroom's (B24)");
    r.tick_to(r.t + 10 * 60_000);
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
}
