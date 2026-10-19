//! The `shroom` command's recording asks (spec N62; dev/proposals/cli-agent-skill.md):
//! refused while General's switch is off (the default), and otherwise exactly what each
//! click does — answered once, with a code. And the refusal's link, which opens Settings at
//! that switch and lights it, never flipping it (Bagrat, 2026-10-18; DECISIONS B216).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{CliAnswer, Effect, PersistOp, PublicUrl, ShellCmd};
use shroom_core::derive::LIT_MS;
use shroom_core::effect::RecCmd;
use shroom_core::event::{CliOp, Event, RecNews, SettingsSpot, StartAsk, Surface};
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
    for (id, op) in [(1, CliOp::START), (2, CliOp::Pause), (3, CliOp::Resume), (4, CliOp::Stop)] {
        let a = ask(&mut r, id, op);
        assert_eq!((a.ok, a.error), (false, Some("recording_off")));
    }
    assert!(r.w.session.arming.is_none() && r.w.take.is_none(), "nothing started");
}

#[test]
fn start_is_the_popovers_start_its_countdown_and_all() {
    let mut r = Run::new(allowed());
    let a = ask(&mut r, 7, CliOp::START);
    assert_eq!((a.ok, a.take, a.error), (true, "countdown", None));
    assert!(r.w.session.arming.is_some(), "the count runs, as after a click on Start");
    assert!(r.w.session.popover.is_none(), "no popover left open by the ask");
    // A second start while the count runs is a take already open.
    let b = ask(&mut r, 8, CliOp::START);
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
    let mut no_space_off = ready();
    no_space_off.space.identity = None;
    let mut counting = Run::new(allowed());
    counting.send(Event::Cli { id: 99, op: CliOp::START });
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
        // Settings needs a space (B160): set up first, the switch after (B216).
        ("no space, the switch off", Run::new(no_space_off), ["not_set_up"; 4]),
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
        for (i, op) in [CliOp::START, CliOp::Pause, CliOp::Resume, CliOp::Stop].into_iter().enumerate() {
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
    ask(&mut r, 1, CliOp::START);
    assert_eq!(r.arm.edge, Some(("idle-ready", "countdown")));
    let mut r = Run::new(allowed());
    r.click();
    assert!(r.ids().contains(&"config"), "{:?}", r.ids());
    ask(&mut r, 2, CliOp::START);
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
    assert_eq!(ask(&mut r, 1, CliOp::START).error, Some("recording_off"));
}

/// A stop while recording, not paused, is the take's own stop: the machine's edge
/// recording → stopping is the one the core names, never the paused popover's.
#[test]
fn a_stop_while_recording_names_its_own_edge() {
    let mut w = allowed();
    w.take = Some(take(Phase::Recording, 61_000));
    let mut r = Run::new(w);
    assert!(ask(&mut r, 1, CliOp::Stop).ok);
    assert_eq!(r.arm.edge, Some(("recording", "stopping")));
    let mut w = allowed();
    w.take = Some(take(Phase::Paused(Cause::User), 61_000));
    let mut r = Run::new(w);
    assert!(ask(&mut r, 2, CliOp::Stop).ok);
    assert_eq!(r.arm.edge, Some(("paused", "stopping")));
}

// ── the refusal's link (Bagrat, 2026-10-18; N62, B216) ──

fn general(r: &Run) -> shroom_core::scene::GeneralBody {
    match &r.scene.settings.as_ref().expect("Settings is open").body.pane {
        SettingsPane::General(g) => g.clone(),
        other => panic!("not General: {other:?}"),
    }
}

fn link(r: &mut Run) -> &mut Run {
    r.send(Event::SettingsLink(SettingsSpot::AgentRecording))
}

/// The link the refusal carries, read back exactly: nothing else opens Settings.
#[test]
fn the_link_is_shroom_settings_agent_and_nothing_else_reads_as_it() {
    assert_eq!(SettingsSpot::AgentRecording.link(), "shroom://settings/agent");
    for yes in ["shroom://settings/agent", "shroom://settings/agent/", "SHROOM://settings/agent", " shroom://settings/agent\n"] {
        assert_eq!(SettingsSpot::of_link(yes), Some(SettingsSpot::AgentRecording), "{yes:?}");
    }
    for no in ["shroom://settings", "shroom://settings/agent?on=1", "shroom://settings/agent#x", "shroom://settings/general", "shroom://join/v.myshroom.workers.dev/AbCdEfGhIjKlMnOp", "https://shroom.am/settings/agent", "shroom:settings/agent", ""] {
        assert_eq!(SettingsSpot::of_link(no), None, "{no:?}");
    }
}

/// From the tray at rest: Settings opens on General, the switch's row lit for a moment —
/// the machine's edge idle-ready → settings-general — and the switch stays off.
#[test]
fn the_link_opens_general_at_the_switch_lit_and_flips_nothing() {
    let mut r = Run::new(ready());
    assert!(r.ids().contains(&"idle-ready") && r.scene.settings.is_none(), "{:?}", r.ids());
    link(&mut r);
    assert_eq!(r.arm.edge, Some(("idle-ready", "settings-general")));
    assert!(r.effects.contains(&Effect::Shell(ShellCmd::OpenSettings)), "{:?}", r.effects);
    let g = general(&r);
    assert!(g.agent.lit, "the switch's row is lit");
    assert!(!g.agent.recording.on && !r.w.prefs.agent_recording, "only the person flips it");
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.agent_recording)));
    // The light goes by itself, Settings stays.
    let lit_at = r.t;
    assert!(r.scene.next_wake.is_some_and(|w| w.0 <= lit_at + LIT_MS), "a wake to put the light out: {:?}", r.scene.next_wake);
    r.tick_to(lit_at + LIT_MS + 10);
    assert!(!general(&r).agent.lit, "out after {LIT_MS} ms");
    assert!(r.scene.settings.is_some());
    // The command's next ask is still refused: the link changed nothing.
    assert_eq!(ask(&mut r, 1, CliOp::START).error, Some("recording_off"));
}

/// Settings open on another tab: the link takes it to General, as a click on the tab would,
/// and remembers General as the tab last used.
#[test]
fn from_another_tab_the_link_goes_to_general() {
    let mut w = ready();
    w.prefs.settings_tab = Some(SettingsTab::Space);
    settings(&mut w);
    let mut r = Run::new(w);
    assert!(!matches!(r.scene.settings.as_ref().unwrap().body.pane, SettingsPane::General(_)));
    link(&mut r);
    assert!(general(&r).agent.lit);
    assert_eq!(r.w.prefs.settings_tab, Some(SettingsTab::General));
    assert!(r.effects.iter().any(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.settings_tab == Some(SettingsTab::General))));
}

/// Over an idle popover, the popover closes, as a click away closes it, and Settings opens.
#[test]
fn over_an_idle_popover_the_popover_closes_and_settings_opens() {
    let mut r = Run::new(ready());
    r.click();
    assert!(r.ids().contains(&"config"), "{:?}", r.ids());
    link(&mut r);
    assert!(r.w.session.popover.is_none() && r.scene.popover.is_none());
    assert!(general(&r).agent.lit);
}

/// Mid-take — the count included — the link opens nothing (law 4: no window opens while a
/// take is open).
#[test]
fn mid_take_the_link_opens_nothing() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 61_000));
    let mut r = Run::new(w);
    link(&mut r);
    assert!(r.scene.settings.is_none() && !r.has(|e| matches!(e, Effect::Shell(ShellCmd::OpenSettings))), "{:?}", r.effects);
    assert_eq!(r.arm.name, "dropped");
    let mut r = Run::new(allowed());
    ask(&mut r, 1, CliOp::START);
    assert!(r.w.session.arming.is_some(), "the count runs");
    link(&mut r);
    assert!(r.scene.settings.is_none() && !r.has(|e| matches!(e, Effect::Shell(ShellCmd::OpenSettings))), "{:?}", r.effects);
}

/// A Mac with no space has no Settings (B160): the link opens nothing there — and the
/// command never gives it one, since such a Mac hears `not_set_up` first.
#[test]
fn a_mac_with_no_space_has_no_settings_for_the_link_to_open() {
    let mut w = ready();
    w.space.identity = None;
    let mut r = Run::new(w);
    link(&mut r);
    assert!(r.scene.settings.is_none() && !r.has(|e| matches!(e, Effect::Shell(ShellCmd::OpenSettings))), "{:?}", r.effects);
    assert_eq!(ask(&mut r, 1, CliOp::START).error, Some("not_set_up"));
}

/// A sheet open in Settings is a question the person is in the middle of: the link leaves
/// it as it is, and brings Settings forward as it stands.
#[test]
fn a_sheet_open_in_settings_stays_and_settings_comes_forward() {
    for sheet in [Confirm::Report { sending: false }, Confirm::Uninstall { going: false }] {
        let mut w = ready();
        settings(&mut w);
        w.session.confirm = Some(sheet.clone());
        let mut r = Run::new(w);
        let before = r.scene.settings.clone();
        link(&mut r);
        assert_eq!(r.w.session.confirm, Some(sheet.clone()), "the sheet stays");
        assert!(r.effects.contains(&Effect::Shell(ShellCmd::OpenSettings)), "Settings comes forward: {:?}", r.effects);
        assert!(!general(&r).agent.lit, "nothing lit behind a sheet");
        assert_eq!(r.scene.settings.as_ref().map(|s| &s.body), before.as_ref().map(|s| &s.body), "as it stands");
    }
}

/// The light is a pointer, not a state: the person flipping the switch, or going to another
/// tab, puts it out at once — it never lights the row again when they come back.
#[test]
fn a_flip_or_another_tab_puts_the_light_out() {
    let mut r = Run::new(ready());
    link(&mut r);
    assert!(general(&r).agent.lit);
    r.send(Event::Act { surface: Surface::Settings, action: Action::SetAgentRecording(true), rev: 0 });
    let g = general(&r);
    assert!(g.agent.recording.on && !g.agent.lit, "flipped by the person, and the light out");
    let mut r = Run::new(ready());
    link(&mut r);
    r.send(Event::Act { surface: Surface::Settings, action: Action::SettingsTab(SettingsTab::Space), rev: 0 });
    r.send(Event::Act { surface: Surface::Settings, action: Action::SettingsTab(SettingsTab::General), rev: 0 });
    assert!(!general(&r).agent.lit, "back on General, nothing lit");
}

/// Opened cold by the link, Settings takes a moment to draw: the core keeps the light long
/// enough for that (the window shows it for its own first moments, then fades it).
#[test]
fn the_light_outlasts_a_window_opening_cold() {
    assert!(LIT_MS >= 5_000, "LIT_MS {LIT_MS}: a window opened cold can take seconds to draw");
}

// ── without the count, and the label (Bagrat, 2026-10-18; N62, B216) ──

fn start(countdown: bool, label: bool) -> CliOp {
    CliOp::Start(StartAsk { countdown, label })
}

/// The recorder says capture began.
fn started(r: &mut Run) {
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("20261018-145200-ab12".into()), rec: RecId("AgEnTlAbElTaKe01".into()) }));
}

fn label(r: &Run) -> Option<String> {
    r.scene.label.as_ref().map(|l| l.line.clone())
}

const LABEL: &str = "Your coding agent is recording your screen";

#[test]
fn the_default_start_keeps_the_count_and_the_label() {
    assert_eq!(CliOp::START, start(true, true));
    assert_eq!(StartAsk::default(), StartAsk { countdown: true, label: true });
}

/// `--no-countdown`: the take starts at once — the count's end fired now, no scrim — and the
/// answer says it records.
#[test]
fn without_the_count_the_take_starts_at_once() {
    let mut r = Run::new(allowed());
    let a = ask(&mut r, 1, start(false, true));
    assert_eq!((a.ok, a.take, a.error), (true, "recording", None));
    assert!(r.scene.countdown.is_none(), "no count on screen");
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { .. }))), "the recorder starts now: {:?}", r.effects);
    started(&mut r);
    assert!(r.ids().contains(&"recording"), "{:?}", r.ids());
    assert_eq!(r.w.take.as_ref().map(|t| t.phase), Some(Phase::Recording));
}

/// A take the command starts wears the label while it records, on the display it records:
/// paused, it goes; resumed, it's back; stopped, it's gone.
#[test]
fn a_take_the_command_starts_wears_the_label_while_it_records() {
    let mut r = Run::new(allowed());
    ask(&mut r, 1, start(false, true));
    started(&mut r);
    assert_eq!(label(&r).as_deref(), Some(LABEL));
    let on = r.scene.label.as_ref().unwrap().display;
    assert_eq!(Some(SourceId::Display(on)), r.w.take.as_ref().map(|t| t.source), "on the display being recorded");
    assert!(ask(&mut r, 2, CliOp::Pause).ok);
    r.send(Event::Rec(RecNews::Paused { length_ms: 4_000 }));
    assert_eq!(label(&r), None, "paused: nothing is recording, so no label says so");
    assert!(ask(&mut r, 3, CliOp::Resume).ok);
    r.send(Event::Rec(RecNews::Resumed));
    assert_eq!(label(&r).as_deref(), Some(LABEL), "back when it records again");
    assert!(ask(&mut r, 4, CliOp::Stop).ok);
    assert_eq!(label(&r), None, "stopped");
}

/// With the count (the default), the label comes with capture, once the count has run.
#[test]
fn with_the_count_the_label_comes_with_capture() {
    let mut r = Run::new(allowed());
    let a = ask(&mut r, 1, CliOp::START);
    assert_eq!(a.take, "countdown");
    assert!(r.scene.countdown.is_some());
    assert_eq!(label(&r), None, "not over the count");
    let end = r.w.session.arming.as_ref().unwrap().deadline.0;
    r.tick_to(end + 10);
    started(&mut r);
    assert_eq!(label(&r).as_deref(), Some(LABEL));
}

/// `--no-label`: no label, the count skipped or not; the tray still shows the take.
#[test]
fn no_label_with_no_label() {
    for countdown in [false, true] {
        let mut r = Run::new(allowed());
        ask(&mut r, 1, start(countdown, false));
        if countdown {
            let end = r.w.session.arming.as_ref().unwrap().deadline.0;
            r.tick_to(end + 10);
        }
        started(&mut r);
        assert_eq!(label(&r), None, "countdown {countdown}");
        assert!(r.ids().contains(&"tray-red-dot"), "the tray shows it: {:?}", r.ids());
    }
}

/// The person's own Start keeps its count and never wears the label — not even after a take
/// the command started.
#[test]
fn the_persons_own_start_never_wears_the_label() {
    // A take the command started, with its label, has come and gone.
    let mut w = allowed();
    w.agent = Some(Mono(500));
    let mut r = Run::new(w);
    r.click();
    assert!(r.ids().contains(&"config"), "{:?}", r.ids());
    r.act(Action::Start);
    assert!(r.scene.countdown.is_some(), "the popover's Start keeps its count");
    let end = r.w.session.arming.as_ref().unwrap().deadline.0;
    r.tick_to(end + 10);
    started(&mut r);
    assert!(r.w.take.is_some());
    assert_eq!(label(&r), None);
}

/// A take of the camera only wears no label: it records no screen.
#[test]
fn a_take_of_the_camera_only_wears_no_label() {
    let mut w = allowed();
    let mut t = take(Phase::Recording, 4_000);
    t.source = SourceId::Camera;
    t.camera = Some("cam".into());
    w.agent = Some(t.opened);
    w.take = Some(t);
    let r = Run::new(w);
    assert_eq!(label(&r), None);
    let mut w = allowed();
    let t = take(Phase::Recording, 4_000);
    w.agent = Some(t.opened);
    w.take = Some(t);
    assert_eq!(label(&Run::new(w)).as_deref(), Some(LABEL), "a display's take does");
}


// ── `shroom edit` (spec N70): the command saves an edit itself, through the space's page;
// the running app makes its final video when the command asks, as after the editor's Done.

fn copy(rec: &str, rev: u64) -> CliOp {
    CliOp::Copy { rec: RecId(rec.into()), rev }
}

fn editor_on(w: &mut World, rec: &str) {
    w.session.fields.editor = Some(EditorFields {
        rec: RecId(rec.into()),
        title: "Standup notes".into(),
        length_ms: Some(133_000),
        source_duration: 133.0,
        markers: vec![],
        history: Some(shroom_edit::History::new(shroom_edit::EditList::new(133.0))),
        rev: 1,
        kept_until: None,
        media: EditorMedia::Ready { duration: 133.0 },
        save: EditorSave::Idle,
        leaving: None,
        elsewhere: false,
    });
}

#[test]
fn an_edits_final_video_is_owed_when_the_command_asks_whatever_the_recording_switch_says() {
    // The switch lets an agent record; an edit is no recording.
    let mut r = Run::new(ready());
    assert!(!r.w.prefs.agent_recording);
    let a = ask(&mut r, 1, copy("St4ndupN0t3sJn05", 2));
    assert_eq!((a.ok, a.error, a.take), (true, None, "none"));
    assert_eq!(r.w.library_ops.copies.get(&RecId("St4ndupN0t3sJn05".into())).map(|c| c.rev), Some(2), "owed: revision 2");
    assert!(
        r.effects.iter().any(|e| matches!(e, Effect::Editor(shroom_core::effect::EditCmd::Copy { rec, rev: 2 }) if rec.0 == "St4ndupN0t3sJn05")),
        "nothing else in the way: the lane is asked at once, as after Done"
    );
}

#[test]
fn an_older_revision_never_replaces_a_newer_one_owed() {
    let mut r = Run::new(ready());
    r.w.library_ops.copies.insert(RecId("St4ndupN0t3sJn05".into()), CopyOwed { rev: 5, asked: true });
    let a = ask(&mut r, 1, copy("St4ndupN0t3sJn05", 4));
    assert!(a.ok);
    assert_eq!(r.w.library_ops.copies.get(&RecId("St4ndupN0t3sJn05".into())), Some(&CopyOwed { rev: 5, asked: true }));
}

#[test]
fn over_a_take_it_is_owed_and_waits_for_the_take_to_end() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 61_000));
    let mut r = Run::new(w);
    let a = ask(&mut r, 1, copy("St4ndupN0t3sJn05", 1));
    assert_eq!((a.ok, a.take), (true, "recording"));
    assert_eq!(r.w.library_ops.copies.get(&RecId("St4ndupN0t3sJn05".into())), Some(&CopyOwed { rev: 1, asked: false }), "owed, not cut over the take");
    assert!(!r.effects.iter().any(|e| matches!(e, Effect::Editor(shroom_core::effect::EditCmd::Copy { .. }))));
}

#[test]
fn this_macs_editor_on_that_video_refuses_and_the_seam_says_which_video_it_holds() {
    let mut w = ready();
    editor_on(&mut w, "St4ndupN0t3sJn05");
    assert_eq!(shroom_core::derive::status::posture(&w).editing.as_deref(), Some("St4ndupN0t3sJn05"), "status.json names it");
    let mut r = Run::new(w);
    let a = ask(&mut r, 1, copy("St4ndupN0t3sJn05", 2));
    assert_eq!((a.ok, a.error), (false, Some("editor_open")));
    assert!(r.w.library_ops.copies.is_empty(), "nothing owed: the window's Done is the edit");
    // Another video's final video is fine.
    let b = ask(&mut r, 2, copy("Q3rNdPl4nW1kThr0", 1));
    assert!(b.ok);
    assert_eq!(shroom_core::derive::status::posture(&ready()).editing, None);
}

#[test]
fn a_mac_with_no_space_owes_nothing() {
    let mut r = Run::new(World::default());
    let a = ask(&mut r, 1, copy("St4ndupN0t3sJn05", 1));
    assert_eq!((a.ok, a.error), (false, Some("not_set_up")));
    assert!(r.w.library_ops.copies.is_empty());
}
