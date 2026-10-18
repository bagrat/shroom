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
