//! The `shroom` command's recording asks (spec N62, provisional; dev/proposals/cli-agent-skill.md):
//! refused while General's switch is off (the default), and otherwise exactly what each
//! click does — answered once, with a code.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{CliAnswer, Effect, PersistOp, PublicUrl, ShellCmd};
use shroom_core::event::{CliOp, Event, Surface};
use shroom_core::scene::{Action, SettingsPane};
use shroom_core::world::{Cause, Why};
use shroom_core::time::Mono;
use shroom_core::world::*;
use support::*;

fn answers(r: &Run) -> Vec<CliAnswer> {
    r.effects.iter().filter_map(|e| if let Effect::Cli(a) = e { Some(a.clone()) } else { None }).collect()
}

fn ask(r: &mut Run, id: u64, op: CliOp) -> CliAnswer {
    r.send(Event::Cli { id, op });
    let a = answers(r);
    assert_eq!(a.len(), 1, "one answer per ask: {a:?}");
    assert_eq!(a[0].id, id);
    a[0].clone()
}

fn allowed() -> World {
    let mut w = ready();
    w.prefs.agent_recording = true;
    w
}

#[test]
fn the_switch_is_off_by_default_and_every_ask_is_refused_while_it_is() {
    assert!(!Prefs::default().agent_recording);
    let mut r = Run::new(ready());
    for (id, op) in [(1, CliOp::Start), (2, CliOp::Pause), (3, CliOp::Resume), (4, CliOp::Stop)] {
        let a = ask(&mut r, id, op);
        assert_eq!((a.ok, a.error), (false, Some("recording_off")));
    }
    assert!(r.w.session.arming.is_none() && r.w.take.is_none(), "nothing started");
}

#[test]
fn start_is_the_popovers_start_its_countdown_and_all() {
    let mut r = Run::new(allowed());
    let a = ask(&mut r, 7, CliOp::Start);
    assert_eq!((a.ok, a.take, a.error), (true, "countdown", None));
    assert!(r.w.session.arming.is_some(), "the count runs, as after a click on Start");
    assert!(r.w.session.popover.is_none(), "no popover left open by the ask");
    // A second start while the count runs is a take already open.
    let b = ask(&mut r, 8, CliOp::Start);
    assert_eq!(b.error, Some("take_open"));
}

#[test]
fn pause_resume_and_stop_follow_the_take_and_stop_answers_its_link() {
    let mut w = allowed();
    w.take = Some(take(Phase::Recording, 61_000));
    let mut r = Run::new(w);
    assert_eq!(ask(&mut r, 1, CliOp::Resume).error, Some("no_take"), "nothing paused to resume");
    let p = ask(&mut r, 2, CliOp::Pause);
    assert!(p.ok, "{p:?}");
    assert!(matches!(r.w.take.as_ref().unwrap().phase, Phase::Pausing(_)));
    assert_eq!(ask(&mut r, 9, CliOp::Stop).error, Some("busy"), "between two steps: ask again in a moment");
    // The recorder says it paused.
    if let Some(t) = r.w.take.as_mut() {
        if let Phase::Pausing(c) = t.phase {
            t.phase = Phase::Paused(c);
        }
    }
    let rec = r.w.take.as_ref().unwrap().rec.0.clone();
    let s = ask(&mut r, 3, CliOp::Stop);
    assert!(s.ok, "{s:?}");
    assert_eq!(s.link, Some(format!("https://{}/{rec}", identity().host)));
    assert!(matches!(r.w.take.as_ref().unwrap().phase, Phase::Finalizing(_)));
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Shell(ShellCmd::Copy(_)))), "law 10: nothing is copied");
}

#[test]
fn nothing_to_stop_or_pause_says_so() {
    let mut r = Run::new(allowed());
    assert_eq!(ask(&mut r, 1, CliOp::Stop).error, Some("no_take"));
    assert_eq!(ask(&mut r, 2, CliOp::Pause).error, Some("no_take"));
}

fn settings(w: &mut World) {
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
}

#[test]
fn general_carries_the_agent_card_its_copy_its_guide_and_the_switch() {
    let mut w = ready();
    settings(&mut w);
    let mut r = Run::new(w);
    let general = |r: &Run| match &r.scene.settings.as_ref().unwrap().body.pane {
        SettingsPane::General(g) => g.clone(),
        other => panic!("not General: {other:?}"),
    };
    let g = general(&r);
    assert_eq!(g.agent.prompt, "Use shroom on my Mac: read https://shroom.am/cli and follow it.");
    assert!(!g.agent.recording.on, "off by default");
    r.send(Event::Act { surface: Surface::Settings, action: Action::CopyAgentPrompt, rev: 0 });
    assert!(r.effects.contains(&Effect::Shell(ShellCmd::CopyAgentPrompt)));
    assert_eq!(general(&r).agent.copy.label, "Copied");
    r.send(Event::Act { surface: Surface::Settings, action: Action::OpenAgentGuide, rev: 0 });
    assert!(r.effects.contains(&Effect::Shell(ShellCmd::OpenUrl(PublicUrl::AgentGuide))));
    r.send(Event::Act { surface: Surface::Settings, action: Action::SetAgentRecording(true), rev: 0 });
    assert!(r.w.prefs.agent_recording && general(&r).agent.recording.on);
    assert!(r.effects.iter().any(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.agent_recording)), "kept for the next launch");
}

// ── every moment (N62, B216) ──

/// What the command hears: ok and the take's face, or the code.
fn heard(a: &CliAnswer) -> String {
    match a.error {
        Some(code) => code.to_string(),
        None => format!("ok:{}", a.take),
    }
}

/// Every ask, in every moment of a take, is answered exactly once — the app holds each
/// connection until the core answers, so an ask left unanswered would hang the command —
/// and the answer is the click it stands for, or why it isn't one now: `busy` between two
/// steps (the count, the recorder starting, pausing, resuming, stopping), `not_ready` where
/// the popover wouldn't offer it (a display that's gone, a question open).
#[test]
fn every_ask_in_every_moment_is_answered_once_as_its_click() {
    let with = |phase: Phase| {
        let mut w = allowed();
        w.take = Some(take(phase, 61_000));
        Run::new(w)
    };
    let mut gone = allowed();
    let mut t = take(Phase::Paused(Cause::DisplayLost), 61_000);
    t.source = SourceId::Display(999);
    gone.take = Some(t);
    let mut no_space = allowed();
    no_space.space.identity = None;
    let mut counting = Run::new(allowed());
    counting.send(Event::Cli { id: 99, op: CliOp::Start });
    assert!(counting.w.session.arming.is_some(), "the count runs");
    let mut open = Run::new(allowed());
    open.click();
    let mut asking = with(Phase::Paused(Cause::User));
    asking.click();
    asking.act(Action::Discard);
    assert!(asking.w.session.confirm.is_some(), "the paused popover asks: discard this take?");
    // The moment → what start, pause, resume and stop hear.
    let table: Vec<(&str, Run, [&str; 4])> = vec![
        ("the switch off", Run::new(ready()), ["recording_off"; 4]),
        ("no space", Run::new(no_space), ["not_set_up"; 4]),
        ("ready", Run::new(allowed()), ["ok:countdown", "no_take", "no_take", "no_take"]),
        ("the popover open", open, ["ok:countdown", "no_take", "no_take", "no_take"]),
        ("the count", counting, ["take_open", "busy", "busy", "busy"]),
        ("starting", with(Phase::Starting), ["take_open", "busy", "busy", "busy"]),
        ("recording", with(Phase::Recording), ["take_open", "ok:paused", "no_take", "ok:stopping"]),
        ("pausing", with(Phase::Pausing(Cause::User)), ["busy"; 4]),
        ("paused", with(Phase::Paused(Cause::User)), ["take_open", "no_take", "ok:recording", "ok:stopping"]),
        ("paused, its display gone", Run::new(gone), ["take_open", "no_take", "not_ready", "ok:stopping"]),
        ("paused, a question open", asking, ["take_open", "no_take", "not_ready", "not_ready"]),
        ("resuming", with(Phase::Resuming), ["busy"; 4]),
        ("stopping", with(Phase::Finalizing(Why::Stop)), ["busy"; 4]),
    ];
    for (moment, base, want) in table {
        for (i, op) in [CliOp::Start, CliOp::Pause, CliOp::Resume, CliOp::Stop].into_iter().enumerate() {
            let mut r = base.clone_run();
            let a = ask(&mut r, 10 + i as u64, op);
            assert_eq!(heard(&a), want[i], "{moment}, {op:?}");
        }
    }
}

/// The command's start is the popover's Start, the popover open or not: from a closed one
/// it is the machine's own edge idle-ready → countdown (N62), from an open one config →
/// countdown — each named by the arm that takes it.
#[test]
fn start_names_the_edge_it_takes() {
    let mut r = Run::new(allowed());
    assert!(r.ids().contains(&"idle-ready"), "{:?}", r.ids());
    ask(&mut r, 1, CliOp::Start);
    assert_eq!(r.arm.edge, Some(("idle-ready", "countdown")));
    let mut r = Run::new(allowed());
    r.click();
    assert!(r.ids().contains(&"config"), "{:?}", r.ids());
    ask(&mut r, 2, CliOp::Start);
    assert_eq!(r.arm.edge, Some(("config", "countdown")));
}

/// Turned off again, the switch refuses every ask again, and the app keeps that for its next
/// launch.
#[test]
fn turned_off_again_every_ask_is_refused_again() {
    let mut w = allowed();
    settings(&mut w);
    let mut r = Run::new(w);
    r.send(Event::Act { surface: Surface::Settings, action: Action::SetAgentRecording(false), rev: 0 });
    assert!(!r.w.prefs.agent_recording);
    assert!(r.effects.iter().any(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if !p.agent_recording)), "kept for the next launch");
    assert_eq!(ask(&mut r, 1, CliOp::Start).error, Some("recording_off"));
}
