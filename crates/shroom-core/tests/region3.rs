//! Region 3 — the recording machine — against the fixtures: every state derives from a
//! world built with the spec's example values, with the fixture's exact Return default and
//! its ratified lines verbatim; and the design's walkthroughs, with the laws checked at
//! every step.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{CloudCmd, Effect, RecCmd, ShellCmd, TakeCmd};
use shroom_core::event::{Event, Key, OsNews, QuitVia, RecNews, Surface, UploadNews};
use shroom_core::scene::Action;
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::derive;
use shroom_spec::Spec;
use support::*;

fn action_label(a: &Action) -> &'static str {
    match a {
        Action::Start => "Start recording",
        Action::Resume => "Resume",
        Action::GetLink => "Get link",
        Action::CopyLink => "Copy link",
        _ => "?",
    }
}

/// One row per Region 3 state: the world that should show it.
fn rows() -> Vec<(&'static str, World)> {
    let mut rows: Vec<(&'static str, World)> = Vec::new();
    let with_popover = |mut w: World| {
        w.session.popover = Some(Door::asked(Mono(900)));
        w
    };
    rows.push(("idle-ready", ready()));
    rows.push(("plain", ready()));
    rows.push(("config", with_popover(ready())));
    // A take that died with nothing to keep, said in the camera's words or the screen's (#53).
    for camera in [true, false] {
        rows.push(("config", {
            let mut w = with_popover(ready());
            w.session.take_lost = Some(TakeLost::Said { camera });
            w
        }));
    }
    rows.push(("config", {
        let mut w = with_popover(ready());
        w.mac.mic = Fact::known(Access::Denied, Mono(0)); // a denied mic: the quiet hint
        w
    }));
    rows.push(("disk-low", {
        let mut w = with_popover(ready());
        w.mac.disk_free = Fact::known(1_000_000_000 + 20 * 54_960_000, Mono(0));
        w
    }));
    rows.push(("recording-off", {
        let mut w = with_popover(ready());
        w.mac.screen = Fact::known(false, Mono(0));
        w
    }));
    rows.push(("recovered-take", {
        let mut w = with_popover(ready());
        w.saved.push(saved(SavedCause::Interrupted, 272_000));
        w
    }));
    rows.push(("tray-amber-dot", {
        let mut w = ready();
        w.saved.push(saved(SavedCause::Interrupted, 272_000));
        w
    }));
    rows.push(("countdown", {
        let mut w = ready();
        w.session.arming = Some(Arming { started: Mono(900), deadline: Mono(3_900), source: SourceId::Display(1), restart_of: None, fired: false });
        w
    }));
    rows.push(("bubble", {
        let mut w = with_popover(ready());
        w.prefs.show_yourself = true;
        w
    }));
    rows.push(("recording", {
        let mut w = ready();
        w.take = Some(take(Phase::Recording, 0));
        w
    }));
    rows.push(("tray-red-dot", {
        let mut w = ready();
        w.take = Some(take(Phase::Recording, 0));
        w
    }));
    for (id, cause) in [
        ("paused", Cause::User),
        ("display-lost", Cause::DisplayLost),
        ("mic-lost", Cause::MicLost),
        ("camera-lost", Cause::CameraLost),
        ("mac-away", Cause::Asleep),
        ("mic-permission-lost", Cause::MicRevoked),
    ] {
        rows.push((id, {
            let mut w = with_popover(ready());
            w.take = Some(take(Phase::Paused(cause), 272_000));
            if cause == Cause::DisplayLost {
                w.mac.displays = Fact::known(vec![], Mono(0)); // the display is gone
            }
            w
        }));
    }
    rows.push(("tray-pause-bars", {
        let mut w = ready();
        w.take = Some(take(Phase::Paused(Cause::User), 272_000));
        w
    }));
    rows.push(("discard-confirm", {
        let mut w = with_popover(ready());
        w.take = Some(take(Phase::Paused(Cause::User), 272_000));
        w.session.confirm = Some(Confirm::Discard);
        w
    }));
    rows.push(("discard-confirm-saved-take", {
        let mut w = with_popover(ready());
        let s = saved(SavedCause::Interrupted, 272_000);
        w.session.confirm = Some(Confirm::DiscardSaved(s.dir.clone()));
        w.saved.push(s);
        w
    }));
    for (id, cause, len) in [("disk-full-stop", SavedCause::DiskFull, 724_000), ("permission-lost", SavedCause::ScreenOff, 435_000)] {
        rows.push((id, {
            let mut w = with_popover(ready());
            let s = saved(cause, len);
            w.session.last_stop = Some(StopOutcome::AutoStopped(s.dir.clone()));
            w.saved.push(s);
            w
        }));
    }
    rows.push(("stopped-offline", {
        let mut w = with_popover(ready());
        w.session.last_stop = Some(StopOutcome::Offline(RecId("b4kq9d".into())));
        w
    }));
    rows.push(("quit-confirm", {
        let mut w = with_popover(ready());
        w.session.confirm = Some(Confirm::Quit);
        for i in 0..2 {
            w.queue.push(Queued { dir: TakeDir(format!("q{i}")), rec: RecId(format!("r{i}")), title: "t".into(), stopped: Wall(0), done: 1, total: 2, blocked: None, length_ms: Some(1_000) });
        }
        w
    }));
    rows
}

#[test]
fn every_region3_state_derives_with_its_return_default_and_ratified_lines() {
    let spec = Spec::load();
    let by_id = spec.by_id();
    let mut failures = Vec::new();
    // Every row of a state is checked on its own; its ratified lines across all its rows
    // (some lines show only under a condition, like the denied mic's hint).
    let mut seen: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    for (id, w) in rows() {
        let scene = derive(&w);
        let ids = scene.state_ids();
        // Law 15's lint over every string the row's Scene shows (TESTING-PLAN §2.4).
        failures.extend(laws::voice_failures(&scene, id));
        if !ids.contains(&id) {
            failures.push(format!("{id}: not derived (got {ids:?})"));
            continue;
        }
        let fixture = by_id[id].iter().find(|s| s.said_lines.len() + s.actions.len() > 0).unwrap_or(&by_id[id][0]);
        // The Return default, exactly as the fixture says — whatever the enabled flags.
        if fixture.surface == "popover" || fixture.parent.as_deref().is_some_and(|p| ["paused", "config"].contains(&p)) {
            if let Some(p) = &scene.popover {
                let want = fixture.return_default().map(|a| a.label.as_str());
                let got = p.ret.as_ref().map(action_label);
                let is_banner = fixture.kind == "banner";
                if !is_banner && want != got {
                    failures.push(format!("{id}: Return is {got:?}, the fixture says {want:?}"));
                }
            }
        }
        seen.entry(id).or_default().push_str(&serde_json::to_string(&scene).unwrap());
    }
    // Ratified lines Bagrat took off a face, each with its entry (the entry quotes the line).
    const DROPPED: &[(&str, &str, &str)] = &[("paused", "no Quit while a take is open", "DEVIATIONS #45")];
    let deviations = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/DECISIONS.md")).expect("docs/DECISIONS.md");
    for (id, line, entry) in DROPPED {
        let n = entry.trim_start_matches("DEVIATIONS #");
        assert!(deviations.lines().any(|l| l.starts_with(&format!("| {n} |")) && l.contains(line)), "{id}: {entry} doesn't quote “{line}”");
    }
    for (id, json) in &seen {
        let fixture = by_id[id].iter().find(|s| s.said_lines.len() + s.actions.len() > 0).unwrap_or(&by_id[id][0]);
        for l in fixture.said_lines.iter().filter(|l| l.copy_status == "ratified") {
            if DROPPED.iter().any(|(d, t, _)| d == id && *t == l.text) {
                continue;
            }
            let example_only = l.note.as_deref().is_some_and(|n| n.contains("example") || n.contains("varies"));
            let encoded = serde_json::to_string(&l.text).unwrap();
            if !example_only && !json.contains(&encoded[1..encoded.len() - 1]) {
                failures.push(format!("{id}: ratified line missing: {:?}", l.text));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ── walkthroughs (ARCHITECTURE §3.6) ──

#[test]
fn click_return_count_record_pause_stop_and_the_page_opens() {
    let mut r = Run::new(ready());
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"));
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::Start), "click the mushroom, press Return (law 8)");

    r.key(Surface::Popover, Key::Return);
    assert!(r.ids().contains(&"countdown"));
    assert!(r.scene.popover.is_none(), "the popover closes for the count");
    assert_eq!(r.scene.countdown.as_ref().unwrap().digit, 3);
    let wake = r.scene.next_wake.unwrap();
    r.tick_to(wake.0);
    assert_eq!(r.scene.countdown.as_ref().unwrap().digit, 2);

    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { .. }))), "the count fires capture");
    assert!(r.scene.countdown.is_none(), "the scrim lifts the instant capture is asked for");

    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert!(r.ids().contains(&"tray-red-dot"));
    assert!(r.scene.popover.is_none(), "recording has no popover");

    r.click();
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    assert!(r.scene.popover.is_none(), "the popover waits for the pause to be confirmed (POC §8)");
    r.send(Event::Rec(RecNews::Paused { length_ms: 272_000 }));
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.id, "paused");
    assert!(serde_json::to_string(p).unwrap().contains("Paused — 4:32"));
    assert_eq!(p.ret, Some(Action::Resume));

    r.act(Action::Stop);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Finalize { .. }))));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Reach))), "the stop probes reachability");
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert!(r.has(|e| matches!(e, Effect::Publish { .. })), "Stop makes the link (law 3)");
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::Copy(_)))), "nothing auto-copies (law 10)");
    r.send(Event::Cloud(shroom_core::event::CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))), "the page opens");
}

#[test]
fn an_unplugged_display_pauses_without_taking_the_keyboard() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::Devices { displays: vec![], mics: quick().mics, cameras: quick().cameras }));
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    let p = r.scene.popover.as_ref().expect("the popover opens by itself");
    assert_eq!((p.id, p.faces.clone()), ("paused", vec!["display-lost"]));
    assert!(!p.activates, "an unasked popover never takes the keyboard (law 5)");
    assert_eq!(p.ret, None, "the fixture gives display-lost no Return default");
    let shroom_core::scene::PopoverBody::Paused(b) = &p.body else { panic!() };
    assert!(!b.resume.enabled, "Resume waits for the display");
    // Plugging it back in enables Resume by derivation alone.
    r.send(Event::Os(OsNews::Devices { displays: quick().displays, mics: quick().mics, cameras: quick().cameras }));
    let shroom_core::scene::PopoverBody::Paused(b) = &r.scene.popover.as_ref().unwrap().body else { panic!() };
    assert!(b.resume.enabled);
}

#[test]
fn a_crashed_take_comes_back_as_a_banner_and_get_link_publishes_it() {
    let mut r = Run::new(ready());
    r.send(Event::Scanned { saved: vec![saved(SavedCause::Interrupted, 272_000)], queue: vec![], removed: vec![] });
    assert!(r.ids().contains(&"tray-amber-dot"), "something waits for you");
    r.click();
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.banner.as_ref().map(|b| b.id), Some("recovered-take"));
    assert_eq!(p.ret, Some(Action::Start), "the banner's own button is never the Return default (law 8)");
    r.act(Action::GetLink);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Confirm(_)))));
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert!(r.has(|e| matches!(e, Effect::Publish { .. })), "Get link makes the link (law 3)");
}

#[test]
fn discarding_asks_and_never_on_return() {
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.act(Action::Discard);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "discard-confirm");
    r.key(Surface::Popover, Key::Return);
    assert!(r.w.take.is_some(), "Return does nothing on the question");
    r.key(Surface::Popover, Key::Esc);
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "paused", "Esc means Back");
    r.act(Action::Discard).act(Action::ConfirmDiscard);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Discard(_)))));
    assert!(r.w.take.is_none());
}

#[test]
fn quit_is_declined_mid_take() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick); // what's already due (the upload riding along) starts first
    r.send(Event::QuitRequested(QuitVia::CmdQ));
    assert!(r.effects.is_empty(), "declined silently while recording (law 4)");
    r.w.take = Some(take(Phase::Paused(Cause::User), 1_000));
    r.send(Event::QuitRequested(QuitVia::CmdQ));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "paused", "while paused, the paused popover with its note");
}

/// Bagrat, 2026-10-14: “cmd-q-ing with just the lib open when not recording, quits the whole
/// app, I'd only expect the lib to close, but tray to stay” (B145).
#[test]
fn cmd_q_closes_the_window_in_front_and_shroom_stays_in_the_menu_bar() {
    let quits = |r: &Run| r.has(|e| matches!(e, Effect::Shell(ShellCmd::Quit)));
    for kind in [WindowKind::Library, WindowKind::Settings] {
        let mut w = ready();
        w.session.windows.insert(kind, Door::asked(Mono(0)));
        let mut r = Run::new(w);
        r.send(Event::QuitRequested(QuitVia::CmdQ));
        assert!(!quits(&r), "{kind:?} in front: ⌘Q doesn't quit");
        assert!(!r.w.session.windows.contains_key(&kind), "{kind:?} in front: ⌘Q closes it, as its ✕ does");
        // With nothing of shroom's in front, ⌘Q quits as before.
        r.send(Event::QuitRequested(QuitVia::CmdQ));
        assert!(quits(&r), "no window left: ⌘Q quits");
    }
    // Two windows: the one in front closes, the other stays.
    let mut w = ready();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(0)));
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(5)));
    let mut r = Run::new(w);
    r.send(Event::QuitRequested(QuitVia::CmdQ));
    assert!(!r.w.session.windows.contains_key(&WindowKind::Settings) && r.w.session.windows.contains_key(&WindowKind::Library));
    // The Dock's Quit, the right-click menu's and the popover's still quit.
    for via in [QuitVia::Dock, QuitVia::RightClick, QuitVia::Footer] {
        let mut w = ready();
        w.session.windows.insert(WindowKind::Library, Door::asked(Mono(0)));
        let mut r = Run::new(w);
        r.send(Event::QuitRequested(via));
        assert!(quits(&r), "{via:?} quits, a window open or not");
    }
}

#[test]
fn an_offline_stop_keeps_the_link_in_the_popover_and_opens_the_page_on_reconnect() {
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::NetPath(false)));
    r.act(Action::Stop);
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Reach))), "offline is already known");
    r.send(Event::Rec(RecNews::Finalized(Ok(272_000))));
    assert_eq!(r.scene.popover.as_ref().unwrap().id, "stopped-offline");
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::CopyLink));
    r.act(Action::CopyLink);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::Copy(_)))));
    r.send(Event::PopoverDismissed);
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Reach))), "reconnecting probes the link");
    r.send(Event::Cloud(shroom_core::event::CloudNews::Reached(Reach::Shroom)));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))), "the page opens by itself");
}

#[test]
fn a_full_disk_finalizes_and_asks_never_publishes() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Rec(RecNews::DiskFloor));
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Finalize { .. }))));
    r.send(Event::Rec(RecNews::Finalized(Ok(724_000))));
    assert!(!r.has(|e| matches!(e, Effect::Publish { .. })), "never auto-published");
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.id, "disk-full-stop");
    assert!(!p.activates);
    let _ = TakeCmd::Rescan;
}

#[test]
fn settling_twice_starts_nothing_new() {
    let mut r = Run::new(ready());
    r.click().key(Surface::Popover, Key::Return);
    let fx = shroom_core::settle::settle(&r.w);
    assert!(fx.is_empty(), "{fx:?}");
}

#[test]
fn a_right_click_mid_take_is_a_left_click() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::TrayClicked { right: true, quick: quick() });
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))), "recording: the right click pauses");
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    r.send(Event::PopoverDismissed);
    assert!(r.scene.popover.is_none());
    r.send(Event::TrayClicked { right: true, quick: quick() });
    let p = r.scene.popover.as_ref().expect("paused: the right click shows the paused popover");
    assert_eq!(p.id, "paused");
    assert!(p.activates, "asked by the click");
    assert_eq!(r.scene.tray.right_menu, None, "the one-item Quit menu exists only when idle");

    // Idle, the right click is the Quit menu's: the core does nothing.
    let mut r = Run::new(ready());
    r.send(Event::TrayClicked { right: true, quick: quick() });
    assert!(r.effects.is_empty() && r.scene.popover.is_none());
    assert!(r.scene.tray.right_menu.is_some());
}

#[test]
fn the_monthly_reconfirm_runs_at_config_never_inside_a_take() {
    let day = 86_400_000;
    let stale = |w: &mut World| w.mac.primed = Some(Wall(w.wall.0 - 26 * day));
    let mut w = ready();
    stale(&mut w);
    let mut r = Run::new(w.clone());
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Prime)), "config opens: the warm-up runs, so macOS asks now");
    r.send(Event::Rec(RecNews::Primed(true)));
    assert!(r.has(|e| matches!(e, Effect::Persist(shroom_core::effect::PersistOp::Primed(_)))), "the new warm-up is remembered");
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"), "config stays");
    r.send(Event::PopoverDismissed);
    r.click();
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Prime)), "once a month, not every click");

    // Not due yet: nothing.
    let mut w2 = ready();
    w2.mac.primed = Some(Wall(w2.wall.0 - 24 * day));
    let mut r = Run::new(w2);
    r.click();
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));

    // A take is open: the paused popover never warms up (law 11).
    let mut w3 = w.clone();
    w3.take = Some(take(Phase::Paused(Cause::User), 1_000));
    let mut r = Run::new(w3);
    r.send(Event::Tick);
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("paused"));
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));

    // Screen recording switched off: `recording-off`'s banner asks instead.
    let mut w4 = w;
    w4.mac.screen = Fact::known(false, Mono(0));
    let mut r = Run::new(w4);
    r.send(Event::TrayClicked { right: false, quick: QuickProbe { screen: false, ..quick() } });
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));
}

#[test]
fn an_unasked_popover_becomes_asked_by_the_persons_first_key_or_click() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::Devices { displays: vec![], mics: quick().mics, cameras: quick().cameras }));
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    assert!(!r.scene.popover.as_ref().unwrap().activates, "opened by itself: no keyboard (law 5)");
    // The person clicked into the panel and typed: the webview's first key says so.
    r.send(Event::PopoverKeyed);
    assert!(r.scene.popover.as_ref().unwrap().activates, "the person's now");

    // Or they clicked the mushroom for it (`ShowPaused`): asked, so the shell makes it key.
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::Devices { displays: vec![], mics: quick().mics, cameras: quick().cameras }));
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    r.click();
    assert!(r.scene.popover.as_ref().is_some_and(|p| p.id == "paused" && p.activates));
}

// ── the take's interruptions, from the reports the runner really sends (audit b2-take) ──

fn config_body(scene: &shroom_core::Scene) -> &shroom_core::scene::ConfigBody {
    match &scene.popover.as_ref().expect("a popover").body {
        shroom_core::scene::PopoverBody::Config(c) => c,
        other => panic!("not config: {other:?}"),
    }
}

fn paused_body(scene: &shroom_core::Scene) -> &shroom_core::scene::PausedBody {
    match &scene.popover.as_ref().expect("a popover").body {
        shroom_core::scene::PopoverBody::Paused(b) => b,
        other => panic!("not paused: {other:?}"),
    }
}

fn usb_mic() -> Device {
    Device { uid: "usb-mic".into(), name: "Shure MV7".into(), builtin: false, continuity: false, system_default: false }
}

fn iphone_camera() -> Device {
    Device { uid: "iphone-cam".into(), name: "Bagrat's iPhone Camera".into(), builtin: false, continuity: true, system_default: false }
}

fn devices(mics: Vec<Device>, cameras: Vec<Device>) -> Event {
    Event::Os(OsNews::Devices { displays: quick().displays, mics, cameras })
}

#[test]
fn the_disk_floor_finalizes_the_take_from_the_free_space_report() {
    use shroom_core::derive::{DISK_MARGIN, ROOM_FLOOR_MINUTES, bytes_per_minute, take_floor_bytes};
    let per_minute = bytes_per_minute(Quality::Normal);
    // The floor counts both tee legs and the single-file copy still to be made.
    assert_eq!(take_floor_bytes(Quality::Normal, 0), DISK_MARGIN + ROOM_FLOOR_MINUTES * per_minute);
    assert_eq!(take_floor_bytes(Quality::Normal, 600_000), DISK_MARGIN + ROOM_FLOOR_MINUTES * per_minute + per_minute / 2 * 10);

    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    // Room above the floor: nothing.
    r.send(Event::Os(OsNews::Disk(take_floor_bytes(Quality::Normal, 0) + 1)));
    assert!(r.effects.is_empty(), "{:?}", r.effects);
    // At the floor, the take finalizes itself — from the free-space report alone.
    r.send(Event::Os(OsNews::Disk(take_floor_bytes(Quality::Normal, 0) - 1)));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Finalize { why: Why::DiskFull })), "{:?}", r.effects);
    r.send(Event::Os(OsNews::Disk(take_floor_bytes(Quality::Normal, 0) - 2)));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(_))), "finalizing once");
    r.send(Event::Rec(RecNews::Finalized(Ok(724_000))));
    assert!(!r.has(|e| matches!(e, Effect::Publish { .. })), "never auto-published (N25)");
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.id, "disk-full-stop");
    assert!(!p.activates, "it opens by itself, without the keyboard (law 5)");
    assert!(serde_json::to_string(p).unwrap().contains("This Mac ran out of room, so the recording stopped at 12:04 — everything so far is saved."));

    // The recorder's own watchdog says it too; a paused take writes nothing and isn't stopped.
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 1_000));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Os(OsNews::Disk(0)));
    r.send(Event::Rec(RecNews::DiskFloor));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Finalize { .. }))));
}

#[test]
fn disk_low_says_its_minutes_in_words_and_greys_start_under_a_minute() {
    use shroom_core::derive::{DISK_MARGIN, bytes_per_minute};
    let per_minute = bytes_per_minute(Quality::Normal);
    let at = |minutes: u64| {
        let mut w = ready();
        w.session.popover = Some(Door::asked(Mono(900)));
        w.mac.disk_free = Fact::known(DISK_MARGIN + minutes * per_minute + per_minute / 2, Mono(0));
        Run::new(w)
    };
    // Law 16: one to nine are words — the law's own check runs at every step.
    let r = at(7);
    assert_eq!(config_body(&r.scene).room_line.as_deref(), Some("About seven minutes of room left on this Mac."));
    assert!(config_body(&r.scene).start.enabled);
    let r = at(1);
    assert_eq!(config_body(&r.scene).room_line.as_deref(), Some("About one minute of room left on this Mac."));
    assert!(config_body(&r.scene).start.enabled, "a minute of room still records");
    let mut r = at(0);
    assert_eq!(config_body(&r.scene).room_line.as_deref(), Some("This Mac is out of room for recording — free up some space first."));
    assert!(!config_body(&r.scene).start.enabled, "under about a minute: Start greys (N39)");
    r.send(Event::Tick);
    let _ = r.send(Event::Key { surface: Surface::Popover, key: Key::Return });
    assert!(r.w.session.arming.is_none());
}

#[test]
fn the_takes_microphone_leaving_pauses_and_resume_uses_the_fallback() {
    let mut w = ready();
    w.mac.mics = Fact::known(vec![quick().mics[0].clone(), usb_mic()], Mono(0));
    w.prefs.mic = Some("usb-mic".into());
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { mic: Some(m), .. }) if m == "usb-mic")));
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!(r.w.take.as_ref().unwrap().mic.as_deref(), Some("usb-mic"), "the take knows its microphone");

    // Another device coming or going changes nothing.
    r.send(devices(vec![quick().mics[0].clone(), usb_mic(), Device { uid: "other".into(), name: "Other".into(), builtin: false, continuity: false, system_default: false }], quick().cameras));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    // The USB mic is pulled: the list the observer reports says so.
    r.send(devices(quick().mics, quick().cameras));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)), "{:?}", r.effects);
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    let p = r.scene.popover.as_ref().expect("it opens by itself");
    assert_eq!((p.id, p.faces.clone(), p.activates), ("paused", vec!["mic-lost"], false));
    let b = paused_body(&r.scene);
    assert_eq!(b.context.as_deref(), Some("Your microphone disconnected — recording paused."));
    assert_eq!(b.sub.as_deref(), Some("Resume uses MacBook Pro Microphone."));
    assert!(b.resume.enabled, "Resume stays live");
    r.act(Action::Resume);
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Resume { mic: Some("BuiltInMicrophoneDevice".into()) })), "Resume does what it said: {:?}", r.effects);
    r.send(Event::Rec(RecNews::Resumed));
    assert_eq!(r.w.take.as_ref().unwrap().mic.as_deref(), Some("BuiltInMicrophoneDevice"));

    // The recorder's own report (the device's stream failed) pauses the same way; with no
    // other microphone, Resume continues without sound — and says so.
    r.send(devices(vec![], quick().cameras));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)));
    r.send(Event::Rec(RecNews::Paused { length_ms: 90_000 }));
    assert_eq!(paused_body(&r.scene).sub.as_deref(), Some("Resume continues without sound."));
    r.act(Action::Resume);
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Resume { mic: None })));
    r.send(Event::Rec(RecNews::Resumed));
    r.send(Event::Rec(RecNews::MicLost));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))), "a silent take has no microphone to lose");

    // The cpal error flag, while a microphone is recording.
    let mut w = ready();
    let mut t = take(Phase::Recording, 0);
    t.mic = Some("BuiltInMicrophoneDevice".into());
    w.take = Some(t);
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Rec(RecNews::MicLost));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)));
    r.send(Event::Rec(RecNews::Paused { length_ms: 5_000 }));
    assert_eq!(r.scene.popover.as_ref().unwrap().faces, vec!["mic-lost"]);
}

#[test]
fn permission_lost_mid_take_comes_from_the_poll_or_any_entrys_probe() {
    // Screen recording switched off: a stop you didn't make — finalize, then ask (N39).
    let mut w = ready();
    let mut t = take(Phase::Recording, 0);
    t.mic = Some("BuiltInMicrophoneDevice".into());
    w.take = Some(t.clone());
    let mut r = Run::new(w.clone());
    r.send(Event::Tick);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Finalize { why: Why::ScreenRevoked })));
    r.send(Event::Rec(RecNews::Finalized(Ok(435_000))));
    assert!(!r.has(|e| matches!(e, Effect::Publish { .. })));
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!((p.id, p.activates), ("permission-lost", false));
    assert!(serde_json::to_string(p).unwrap().contains("macOS switched off screen recording, so the recording stopped at 7:15 — everything so far is saved."));
    r.send(Event::PopoverDismissed);
    r.send(Event::TrayClicked { right: false, quick: QuickProbe { screen: false, ..quick() } });
    assert_eq!(r.scene.popover.as_ref().unwrap().banner.as_ref().map(|b| b.id), Some("recovered-take"), "the saved take outranks recording-off (N38)");

    // A tray click mid-take carries the same news in its probe: it stops, never a plain pause.
    let mut r = Run::new(w.clone());
    r.send(Event::Tick);
    r.send(Event::TrayClicked { right: false, quick: QuickProbe { screen: false, ..quick() } });
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Finalize { why: Why::ScreenRevoked })), "{:?}", r.effects);
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Pause)));

    // The microphone switched off: an auto-pause, Resume continues without sound.
    let mut r = Run::new(w.clone());
    r.send(Event::Tick);
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Denied, camera: Access::Granted }));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)));
    r.send(Event::Rec(RecNews::Paused { length_ms: 10_000 }));
    assert_eq!(r.scene.popover.as_ref().unwrap().faces, vec!["mic-permission-lost"]);
    r.act(Action::Resume);
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Resume { mic: None })), "{:?}", r.effects);

    // …or from a window coming forward mid-take (law 1: every entry re-probes).
    let mut w2 = w.clone();
    w2.session.windows.insert(WindowKind::Library, Door::asked(Mono(100)));
    let mut r = Run::new(w2);
    r.send(Event::Tick);
    r.send(Event::WindowFocused(WindowKind::Library, QuickProbe { mic: Access::Denied, ..quick() }));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)));

    // Camera as the source doesn't need screen recording: nothing stops.
    let mut w3 = w;
    w3.take = Some(OpenTake { source: SourceId::Camera, camera: Some("cam".into()), ..t });
    let mut r = Run::new(w3);
    r.send(Event::Tick);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(_))), "{:?}", r.effects);
}

#[test]
fn capture_that_stops_making_progress_ends_as_a_saved_take() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Rec(RecNews::FrameStall));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Abandon)), "kept, never deleted (law 2)");
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Rescan)));
    assert!(r.w.take.is_none());
    assert!(r.scene.popover.as_ref().is_some_and(|p| !p.activates), "it opens by itself");
    // The lane lets capture go and releases the lock: the scan reads the folder then.
    r.send(Event::Rec(RecNews::Abandoned));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Rescan)), "a scan after the lock is released");
    // The scan brings it back as the recovered take (DEVIATIONS #6).
    r.send(Event::Scanned { saved: vec![saved(SavedCause::Interrupted, 61_000)], queue: vec![], removed: vec![] });
    assert_eq!(r.scene.popover.as_ref().unwrap().banner.as_ref().map(|b| b.id), Some("recovered-take"));
}

#[test]
fn capture_that_died_with_nothing_to_keep_says_so_at_config() {
    // Bagrat's 0.1.10 checkpoint: Camera only, the camera never sent the take a frame, the
    // watchdog called it dead after five seconds — and the folder held nothing a repair
    // could save. No banner, no take, no word: law 2 broken in silence (DEVIATIONS #53).
    let mut w = ready();
    w.take = Some(OpenTake { source: SourceId::Camera, camera: Some("cam".into()), ..take(Phase::Recording, 0) });
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Rec(RecNews::FrameStall));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Abandon)), "kept, never deleted (law 2)");
    // The lane let capture go and found nothing it could keep.
    r.send(Event::Rec(RecNews::NothingKept { camera: true }));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Rescan)), "the scan still reads the folder");
    r.send(Event::Scanned { saved: vec![], queue: vec![], removed: vec![] });
    let p = r.scene.popover.as_ref().expect("the popover is open");
    assert_eq!(p.id, "config", "back at config, Start the default");
    assert!(p.faces.contains(&"take-lost"), "{:?}", p.faces);
    assert_eq!(p.status_line.as_deref(), Some(shroom_core::copy::nothing_recorded(true).as_str()), "said in the status line");
    assert_eq!(p.ret, Some(Action::Start), "Return tries again");
    // Said once: when the popover that said it closes, it's behind us.
    r.send(Event::PopoverDismissed);
    r.click();
    let p = r.scene.popover.as_ref().unwrap();
    assert!(!p.faces.contains(&"take-lost") && p.status_line.is_none(), "said once: {:?}", p.status_line);
}

#[test]
fn a_display_take_that_kept_nothing_says_so_in_its_own_words() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Rec(RecNews::FrameStall));
    r.send(Event::Rec(RecNews::NothingKept { camera: false }));
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.status_line.as_deref(), Some(shroom_core::copy::nothing_recorded(false).as_str()));
    // Start clears it: the next take begins clean.
    r.key(Surface::Popover, Key::Return);
    assert!(r.w.session.arming.is_some());
    assert!(r.w.session.take_lost.is_none());
}

#[test]
fn a_capture_nobody_asked_for_keeps_quiet_when_it_held_nothing() {
    // The count was cancelled while the recorder was already starting: Started arrives
    // unasked, and the lane lets it go. Nothing was lost that anyone wanted — no line.
    let mut w = ready();
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Abandon)));
    r.send(Event::Rec(RecNews::NothingKept { camera: false }));
    let p = r.scene.popover.as_ref().unwrap();
    assert!(p.status_line.is_none() && !p.faces.contains(&"take-lost"), "{:?}", p.status_line);
}

#[test]
fn camera_lost_is_about_the_camera_being_recorded() {
    let ext = Device { uid: "ext".into(), name: "Logitech BRIO".into(), builtin: false, continuity: false, system_default: false };
    let mut w = ready();
    w.mac.cameras = Fact::known(vec![quick().cameras[0].clone(), ext.clone()], Mono(0));
    w.prefs.source = Some(SourceId::Camera);
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { source: SourceId::Camera, camera: Some(c), .. }) if c == "cam")), "the core picks the camera: {:?}", r.effects);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!(r.w.take.as_ref().unwrap().camera.as_deref(), Some("cam"));
    // On a MacBook the built-in camera stays; the external one going is nothing to this take.
    r.send(devices(quick().mics, quick().cameras));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))));
    // The recorded camera goes, another stays: camera-lost all the same.
    r.send(devices(quick().mics, vec![ext.clone()]));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Pause)), "{:?}", r.effects);
    r.send(Event::Rec(RecNews::Paused { length_ms: 30_000 }));
    assert_eq!(r.scene.popover.as_ref().unwrap().faces, vec!["camera-lost"]);
    assert!(!paused_body(&r.scene).resume.enabled, "Resume greyed until it's back");
    r.send(devices(quick().mics, vec![quick().cameras[0].clone(), ext]));
    assert!(paused_body(&r.scene).resume.enabled, "back: Resume is live");
}

#[test]
fn an_iphone_is_never_picked_by_itself_for_the_camera() {
    // Only an iPhone around: no Camera source, no bubble.
    let mut w = ready();
    w.mac.cameras = Fact::known(vec![iphone_camera()], Mono(0));
    w.prefs.show_yourself = true;
    w.prefs.source = Some(SourceId::Camera);
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w.clone());
    assert!(!config_body(&r.scene).sources.iter().any(|s| s.action == Action::SetSource(SourceId::Camera)));
    assert!(r.scene.bubble.is_none(), "the bubble never opens an iPhone by itself (N39)");
    assert_eq!(shroom_core::derive::selected_camera(&w), None);
    // An iPhone listed first, the built-in second: the built-in.
    w.mac.cameras = Fact::known(vec![iphone_camera(), quick().cameras[0].clone()], Mono(0));
    let r = Run::new(w);
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), None, "Camera is the source: no bubble");
    assert_eq!(r.scene.preview.as_ref().map(|p| p.camera.as_str()), Some("cam"));
}

/// The label a picker shows: its selected choice's.
fn picked(choices: &[shroom_core::scene::Choice]) -> Option<&str> {
    choices.iter().find(|c| c.selected).map(|c| c.label.as_str())
}

fn labels(choices: &[shroom_core::scene::Choice]) -> Vec<&str> {
    choices.iter().map(|c| c.label.as_str()).collect()
}

#[test]
fn with_only_an_iphone_around_show_yourself_picks_nothing_and_keeps_the_wish() {
    // Show yourself on, the Mac's own camera here: the bubble on it.
    let mut w = ready();
    w.prefs.show_yourself = true;
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(labels(&c.cameras), ["System default", "FaceTime HD Camera", "Camera off"]);
    assert_eq!(picked(&c.cameras), Some("System default"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("cam"));
    // The lid closes (the built-in camera goes); an iPhone is nearby: it's listed, but nothing
    // opens it by itself — Show yourself reads Camera off, no bubble, no Camera only (N39).
    r.send(devices(quick().mics, vec![iphone_camera()]));
    let c = config_body(&r.scene);
    assert_eq!(labels(&c.cameras), ["Bagrat's iPhone Camera", "Camera off"]);
    assert_eq!(picked(&c.cameras), Some("Camera off"), "{:?}", c.cameras);
    assert!(!c.sources.iter().any(|s| s.action == Action::SetSource(SourceId::Camera)));
    assert!(c.camera_line.is_none(), "no camera isn't a denial: no line");
    assert!(r.scene.bubble.is_none());
    assert!(c.start.enabled, "a display records without the camera");
    // No camera at all: Camera off, and nothing else to pick.
    r.send(devices(quick().mics, vec![]));
    assert_eq!(labels(&config_body(&r.scene).cameras), ["Camera off"]);
    assert!(r.scene.bubble.is_none());
    // A webcam plugged in: the wish was kept — the bubble comes back on it.
    let ext = Device { uid: "ext".into(), name: "Logitech BRIO".into(), builtin: false, continuity: false, system_default: false };
    r.send(devices(quick().mics, vec![iphone_camera(), ext]));
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.cameras), Some("System default"), "on System default, which now means the webcam (N44)");
    assert_eq!(c.cameras[0].detail.as_deref(), Some("Logitech BRIO"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("ext"));
}

#[test]
fn show_yourself_picks_a_camera_or_camera_off() {
    // Bagrat (2026-10-13): Show yourself is a pick, like the microphone — every camera here,
    // then Camera off. On from the first start, with the camera shroom picks (N43, Bagrat:
    // “yes on by default” — amends N22's off).
    let ext = Device { uid: "ext".into(), name: "Logitech BRIO".into(), builtin: false, continuity: false, system_default: false };
    let mut w = ready();
    w.mac.cameras = Fact::known(vec![quick().cameras[0].clone(), ext, iphone_camera()], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(c.camera_label, "Show yourself");
    assert_eq!(labels(&c.cameras), ["System default", "FaceTime HD Camera", "Logitech BRIO", "Bagrat's iPhone Camera", "Camera off"]);
    assert_eq!(picked(&c.cameras), Some("System default"), "the first-ever Show yourself is on (N43), on System default (N44)");
    assert!(c.cameras.iter().all(|x| x.enabled));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("cam"), "the bubble is live at the first popover");
    // Picking a camera shows it in the bubble, and is remembered.
    r.act(Action::SetCamera(Some("ext".into())));
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("Logitech BRIO"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("ext"));
    assert!(r.w.prefs.show_yourself);
    assert_eq!(r.w.prefs.camera.as_deref(), Some("ext"));
    // The person may pick the iPhone themselves — it's never picked *for* them.
    r.act(Action::SetCamera(Some("iphone-cam".into())));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("iphone-cam"));
    // Camera off: the bubble goes; the pick is kept for next time.
    r.act(Action::SetCamera(None));
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("Camera off"));
    assert!(r.scene.bubble.is_none());
    assert!(!r.w.prefs.show_yourself);
    assert_eq!(r.w.prefs.camera.as_deref(), Some("iphone-cam"));
    // The picked camera leaves: back to System default (N44), the pick kept for its return.
    r.act(Action::SetCamera(Some("ext".into())));
    r.send(devices(quick().mics, quick().cameras));
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("System default"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("cam"));
    assert_eq!(r.w.prefs.camera.as_deref(), Some("ext"));
}

#[test]
fn camera_only_records_the_picked_camera_and_has_no_camera_off() {
    let ext = Device { uid: "ext".into(), name: "Logitech BRIO".into(), builtin: false, continuity: false, system_default: false };
    let mut w = ready();
    // Someone who switched the bubble off: Camera only's pick doesn't switch it back on.
    w.prefs.show_yourself = false;
    w.mac.cameras = Fact::known(vec![quick().cameras[0].clone(), ext], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let camera_only = config_body(&r.scene).sources.iter().find(|s| s.action == Action::SetSource(SourceId::Camera)).map(|s| s.label.clone());
    assert_eq!(camera_only.as_deref(), Some("Camera only"));
    r.act(Action::SetSource(SourceId::Camera));
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.sources), Some("Camera only"));
    // The camera pick stays live — it's what the take records — and Camera off isn't offered.
    assert_eq!(labels(&c.cameras), ["System default", "FaceTime HD Camera", "Logitech BRIO"]);
    assert_eq!(picked(&c.cameras), Some("System default"));
    assert!(c.cameras.iter().all(|x| x.enabled));
    r.act(Action::SetCamera(Some("ext".into())));
    assert_eq!(r.scene.preview.as_ref().map(|p| p.camera.as_str()), Some("ext"), "the preview shows the pick");
    assert!(r.scene.bubble.is_none());
    assert!(!r.w.prefs.show_yourself, "picking Camera only's camera isn't asking for the bubble");
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { source: SourceId::Camera, camera: Some(c), .. }) if c == "ext")), "{:?}", r.effects);
}

#[test]
fn no_mic_is_a_pick_and_records_silently() {
    let mut w = ready();
    w.mac.mics = Fact::known(vec![quick().mics[0].clone(), usb_mic()], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(labels(&c.mics), ["System default", "MacBook Pro Microphone", "Shure MV7", "No mic"]);
    assert_eq!(picked(&c.mics), Some("System default"));
    r.act(Action::SetMic(None));
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.mics), Some("No mic"));
    assert!(c.mic_line.is_none(), "the person's own pick needs no line");
    assert!(r.w.prefs.mic_off);
    // A microphone never asked about isn't asked about now: there's none to record.
    r.w.mac.mic = Fact::known(Access::NotAsked, Mono(0));
    r.key(Surface::Popover, Key::Return);
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::RequestPermission(_)))), "{:?}", r.effects);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.has(|e| matches!(e, Effect::Recorder(RecCmd::Start { mic: None, .. }))), "silent: {:?}", r.effects);
    // Picking one again turns the sound back on.
    let mut w = ready();
    w.prefs.mic_off = true;
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.act(Action::SetMic(Some("BuiltInMicrophoneDevice".into())));
    assert_eq!(picked(&config_body(&r.scene).mics), Some("MacBook Pro Microphone"));
    assert!(!r.w.prefs.mic_off);
}

#[test]
fn a_denied_mic_shows_no_mic_greyed_with_its_hint() {
    let mut w = ready();
    w.mac.mic = Fact::known(Access::Denied, Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.mics), Some("No mic"));
    assert!(c.mics.iter().all(|m| !m.enabled), "none to pick: {:?}", c.mics);
    assert_eq!(c.mic_hint.as_ref().map(|b| b.action.clone()), Some(Action::OpenMicSettings));
    // No microphone at all: No mic, and the line that says the video will be silent.
    let mut w = ready();
    w.mac.mics = Fact::known(vec![], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(labels(&c.mics), ["No mic"]);
    assert_eq!(c.mic_line.as_deref(), Some("No microphone — your video will be silent."));
}

#[test]
fn restart_asks_before_it_starts_over() {
    // The canvas's Restart question (Bagrat, 2026-10-12): Restart asks first; Back keeps the
    // take; Restart in the question starts the count — still Restart's undo (N39).
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.act(Action::Restart);
    let p = r.scene.popover.as_ref().expect("the question");
    assert_eq!(p.id, "restart-confirm");
    let shroom_core::scene::PopoverBody::Question(q) = &p.body else { panic!("not a question: {:?}", p.body) };
    assert_eq!(q.title, "Start over?");
    assert_eq!(q.line, "This replaces the 4:32 you've recorded so far.");
    assert_eq!(q.buttons.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(), ["Back", "Restart"]);
    assert_eq!(q.buttons[1].style, shroom_core::scene::Style::RedOutline);
    assert_eq!(p.ret, None, "law 9: no Return default on a question that replaces a take");
    assert!(r.w.session.arming.is_none());
    r.key(Surface::Popover, Key::Esc);
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("paused"), "Esc is Back");
    r.act(Action::Restart);
    r.act(Action::ConfirmRestart);
    assert!(r.ids().contains(&"countdown"));
    assert!(r.w.session.arming.as_ref().is_some_and(|a| a.restart_of.is_some()));
    // The count is still cancellable: the old take comes back, paused.
    r.act(Action::CancelCountdown);
    assert!(r.w.take.is_some());
}

#[test]
fn choosing_camera_asks_for_the_camera_on_that_click_and_a_denial_greys_start() {
    let mut w = ready();
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    assert!(r.scene.preview.is_none());
    r.act(Action::SetSource(SourceId::Camera));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(shroom_core::effect::Perm::Camera))), "the first camera pick is the prompt (N22, law 11)");
    // Denied: Start greys, one quiet line with the door to System Settings.
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Denied }));
    let c = config_body(&r.scene);
    assert!(!c.start.enabled, "Camera as the source greys Start");
    assert_eq!(c.camera_line.as_deref(), Some("Camera is off for shroom —"));
    assert_eq!(c.camera_settings.as_ref().map(|b| b.action.clone()), Some(Action::OpenCameraSettings));
    assert!(r.scene.preview.is_none());
    // Back to a display: Show yourself, on, flips back off — with the same line.
    r.act(Action::SetSource(SourceId::Display(1)));
    r.act(Action::SetCamera(Some("cam".into())));
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::RequestPermission(_)))), "denied is an answer: no second prompt");
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.cameras), Some("Camera off"), "flips back off");
    assert!(c.start.enabled, "a display records without the camera");
    assert_eq!(c.camera_line.as_deref(), Some("Camera is off for shroom —"));
    assert!(r.scene.bubble.is_none());
    // Allowed again in System Settings: the wish was kept.
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }));
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("FaceTime HD Camera"));
    assert!(r.scene.bubble.is_some());
}

#[test]
fn camera_as_the_source_previews_at_config_and_counts_on_that_display() {
    let mut w = ready();
    w.mac.displays = Fact::known(
        vec![
            Display { id: 7, name: "LG UltraFine".into(), is_main: false, x: -2560, y: 0 },
            Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 },
        ],
        Mono(0),
    );
    w.prefs.source = Some(SourceId::Camera);
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let preview = r.scene.preview.clone().expect("Camera shows its own preview at config (N39)");
    assert_eq!((preview.display, preview.camera.as_str()), (1, "cam"));
    r.key(Surface::Popover, Key::Return);
    assert_eq!(r.scene.countdown.as_ref().map(|c| c.display), Some(preview.display), "the count plays where the preview is");
    assert!(r.scene.preview.is_some(), "and the preview stays through the count");
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    // DEVIATIONS #42: the preview stays while the take records — you keep seeing yourself.
    assert_eq!(r.scene.preview.as_ref(), Some(&preview), "the preview stays while the take records the camera");
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 4_000 }));
    assert_eq!(r.scene.preview.as_ref(), Some(&preview), "and through a pause");
    r.act(Action::Stop);
    assert!(r.scene.preview.is_none(), "Stop ends the take, and the preview goes with it");
}

#[test]
fn camera_onlys_preview_never_drops_between_the_count_and_the_take() {
    // Bagrat's 0.1.10 checkpoint: the preview he dragged snapped back at Start. The count
    // fired, the recorder hadn't said Started yet, and for that moment the Scene had no
    // preview — its window closed and came back at the default place, its camera reopened
    // beside the one the take had just opened. Like the bubble, it stays through the whole
    // count, its last moment included (N39, DEVIATIONS #42).
    let mut w = ready();
    w.prefs.source = Some(SourceId::Camera);
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let preview = r.scene.preview.clone().expect("Camera shows its own preview at config (N39)");
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.w.session.arming.as_ref().is_some_and(|a| a.fired), "the count has fired: the recorder is opening the camera");
    assert_eq!(r.scene.preview.as_ref(), Some(&preview), "the preview stays while the take starts");
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!(r.scene.preview.as_ref(), Some(&preview), "and into the take, the same preview");
}

#[test]
fn camera_onlys_preview_comes_back_to_config_when_the_take_cant_start() {
    let mut w = ready();
    w.prefs.source = Some(SourceId::Camera);
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let preview = r.scene.preview.clone().unwrap();
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    r.send(Event::Rec(RecNews::Failed));
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"), "back at config");
    assert_eq!(r.scene.preview.as_ref(), Some(&preview), "with Camera's preview still there");
}

#[test]
fn recording_off_greys_the_displays_and_camera_still_records() {
    let mut w = ready();
    w.mac.screen = Fact::known(false, Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    assert_eq!(r.scene.popover.as_ref().unwrap().banner.as_ref().map(|b| b.id), Some("recording-off"));
    let c = config_body(&r.scene);
    assert!(c.sources.iter().filter(|s| matches!(s.action, Action::SetSource(SourceId::Display(_)))).all(|s| !s.enabled), "display sources grey out (N38)");
    assert!(c.sources.iter().any(|s| s.action == Action::SetSource(SourceId::Camera) && s.enabled), "Camera still records");
    assert!(!c.start.enabled);
    r.act(Action::SetSource(SourceId::Display(1)));
    assert!(r.effects.is_empty(), "a greyed display can't be picked");
    r.act(Action::SetSource(SourceId::Camera));
    assert!(config_body(&r.scene).start.enabled, "Camera starts with screen recording off");
    r.key(Surface::Popover, Key::Return);
    assert!(r.w.session.arming.is_some());
}

#[test]
fn the_arm_time_guard_replays_exactly_the_missing_beat() {
    // A cleanup tool reset the microphone: Start asks for it — only it — and doesn't arm.
    let mut w = ready();
    w.mac.mic = Fact::known(Access::NotAsked, Mono(0));
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.prefs.show_yourself = true;
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.key(Surface::Popover, Key::Return);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(shroom_core::effect::Perm::Mic))], "one prompt, never stacked");
    assert!(r.w.session.arming.is_none(), "never a prompt under the count (law 11)");
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::NotAsked }));
    r.key(Surface::Popover, Key::Return);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(shroom_core::effect::Perm::Camera))], "then the camera, which Show yourself needs");
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Denied }));
    r.key(Surface::Popover, Key::Return);
    assert!(r.w.session.arming.is_some(), "answered either way: it arms");
}

#[test]
fn a_mac_with_no_camera_is_never_asked_for_one_at_start() {
    // Setup skips the camera beat on a Mac without one (N43), and Show yourself starts on:
    // Start never asks macOS for a camera that isn't there — it arms (CHECKPOINT §10, 6.1).
    let mut w = ready();
    w.mac.cameras = Fact::known(vec![], Mono(0));
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.prefs.show_yourself = true;
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.key(Surface::Popover, Key::Return);
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::RequestPermission(_)))), "no camera, no camera ask");
    assert!(r.w.session.arming.is_some(), "it arms");
}

#[test]
fn displays_of_one_name_get_their_place_or_a_number() {
    let lg = |id: u32, x: i32, y: i32| Display { id, name: "LG UltraFine".into(), is_main: false, x, y };
    let labels = |displays: Vec<Display>| {
        let mut w = ready();
        w.mac.displays = Fact::known(displays, Mono(0));
        w.session.popover = Some(Door::asked(Mono(900)));
        let r = Run::new(w);
        config_body(&r.scene).sources.iter().map(|s| s.label.clone()).collect::<Vec<_>>()
    };
    let main = Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 };
    assert_eq!(labels(vec![main.clone(), lg(2, 1512, 0), lg(3, 4072, 0)]), ["Built-in Display", "LG UltraFine (left)", "LG UltraFine (right)", "Camera only"]);
    assert_eq!(labels(vec![main.clone(), lg(3, 4072, 0), lg(2, 1512, 0)]), ["Built-in Display", "LG UltraFine (right)", "LG UltraFine (left)", "Camera only"]);
    // Three: place can't say it — numbered left to right.
    assert_eq!(
        labels(vec![main.clone(), lg(4, 6632, 0), lg(2, 1512, 0), lg(3, 4072, 0)]),
        ["Built-in Display", "LG UltraFine 3", "LG UltraFine 1", "LG UltraFine 2", "Camera only"]
    );
    // Two stacked (one left edge): numbered top to bottom.
    assert_eq!(labels(vec![main, lg(2, 1512, -1440), lg(3, 1512, 0)]), ["Built-in Display", "LG UltraFine 1", "LG UltraFine 2", "Camera only"]);
}

#[test]
fn the_bubble_sits_on_the_chosen_display_remembers_its_place_and_goes_with_its_camera() {
    let mut w = ready();
    w.mac.displays = Fact::known(
        vec![
            Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 },
            Display { id: 7, name: "LG UltraFine".into(), is_main: false, x: 1512, y: 0 },
        ],
        Mono(0),
    );
    w.prefs.show_yourself = true;
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let b = r.scene.bubble.clone().expect("live from the moment config opens (N22)");
    assert_eq!((b.display, b.camera.as_str(), b.at), (1, "cam", None));
    // With Show yourself on, picking another display moves the bubble there (N39).
    r.act(Action::SetSource(SourceId::Display(7)));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.display), Some(7));
    // Dragged: the place is remembered, relative to its display.
    r.send(Event::BubbleMoved((40, 600)));
    assert!(r.has(|e| matches!(e, Effect::Persist(shroom_core::effect::PersistOp::Prefs(p)) if p.bubble_at == Some((40, 600)))));
    assert_eq!(r.scene.bubble.as_ref().and_then(|b| b.at), Some((40, 600)));
    r.send(Event::BubbleMoved((40, 600)));
    assert!(r.effects.is_empty(), "the same place twice writes nothing");
    // Through the count and the take, on the recorded display.
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.display), Some(7));
    // Its camera disappears: the bubble simply goes, and the take never notices (N39).
    let displays = r.w.mac.displays.get().cloned().unwrap();
    r.send(Event::Os(OsNews::Devices { displays, mics: quick().mics, cameras: vec![] }));
    assert!(r.scene.bubble.is_none());
    assert!(!r.has(|e| matches!(e, Effect::Recorder(RecCmd::Pause))), "{:?}", r.effects);
    // No bubble, nothing to place.
    r.send(Event::BubbleMoved((1, 1)));
    assert!(r.effects.is_empty());
}

#[test]
fn a_denied_mic_shows_the_microphone_off() {
    let mut w = ready();
    w.mac.mic = Fact::known(Access::Denied, Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w);
    let c = config_body(&r.scene);
    // No mic shows, none to pick (Bagrat's picks, DEVIATIONS #43 — was: none picked).
    assert!(c.mics.iter().all(|m| !m.enabled && m.selected == (m.action == Action::SetMic(None))), "No mic, none to pick: {:?}", c.mics);
    assert_eq!(c.mic_hint.as_ref().map(|b| b.label.as_str()), Some("allowed in System Settings"));
    assert!(c.start.enabled, "a denied mic never blocks");
}

/// Law 12's converse over this region's inventory: from every row, every op in flight —
/// as the row stands, after each button it offers, after its next wake — failed every way
/// lands (a face, a line, its note read, a noted retry), with the laws at every step.
#[test]
fn every_failure_from_every_row_lands() {
    let bad = every_failure_lands(rows());
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

/// the machine's law 12, invariant 1: every failure-state this region's rows reach is its own
/// kebab-case state with its own render and its own way on.
#[test]
fn every_failure_here_is_its_own_state() {
    let bad = laws::failures_are_states(&rows(), laws::failures_owned_by("region3"));
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

#[test]
fn the_status_line_says_its_tone() {
    use shroom_core::scene::StatusTone;
    // The canvas's dot beside the status line (DEVIATIONS #45): moving, waiting — and when the
    // last upload finishes, the line just goes, no “Fully live ✓” (DEVIATIONS #52).
    let mut w = ready();
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w.clone());
    assert_eq!((r.scene.popover.as_ref().unwrap().status_line.as_deref(), r.scene.popover.as_ref().unwrap().status_tone), (None, None));
    w.queue = vec![Queued { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()), title: "Untitled — Oct 2".into(), stopped: Wall(1_790_000_000_000), done: 1, total: 4, blocked: None, length_ms: Some(60_000) }];
    let p = derive(&w).popover.unwrap();
    assert!(p.status_line.as_deref().is_some_and(|l| l.starts_with("Uploading")));
    assert_eq!(p.status_tone, Some(StatusTone::Busy));
    w.net.online = Fact::known(false, Mono(0));
    let p = derive(&w).popover.unwrap();
    assert_eq!(p.status_line.as_deref(), Some("Waiting for connection — your links already work."));
    assert_eq!(p.status_tone, Some(StatusTone::Waiting));
    w.net.online = Fact::known(true, Mono(0));
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Live(TakeDir("d1".into()))));
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!((p.status_line.as_deref(), p.status_tone), (None, None));
}

// ── N44: System default, for the microphone and the camera ──

fn mic(uid: &str, name: &str, builtin: bool, continuity: bool, system_default: bool) -> Device {
    Device { uid: uid.into(), name: name.into(), builtin, continuity, system_default }
}

fn built_in_mic(system_default: bool) -> Device {
    mic("BuiltInMicrophoneDevice", "MacBook Pro Microphone", true, false, system_default)
}

fn airpods(system_default: bool) -> Device {
    mic("airpods", "Bagrat's AirPods Pro", false, false, system_default)
}

fn arm_and_start(r: &mut Run) {
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
}

fn started_mic(r: &Run) -> Option<String> {
    r.effects.iter().find_map(|e| match e {
        Effect::Recorder(RecCmd::Start { mic, .. }) => Some(mic.clone()),
        _ => None,
    })?
}

#[test]
fn system_default_is_the_microphones_first_pick_and_follows_macos() {
    let mut w = ready();
    w.mac.mics = Fact::known(vec![built_in_mic(true), airpods(false)], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(labels(&c.mics), ["System default", "MacBook Pro Microphone", "Bagrat's AirPods Pro", "No mic"]);
    assert_eq!(picked(&c.mics), Some("System default"), "a new Mac starts on System default");
    assert_eq!(c.mics[0].detail.as_deref(), Some("MacBook Pro Microphone"), "the device it means, beneath");
    // AirPods go on: macOS makes them the default input, and System default follows.
    r.send(devices(vec![built_in_mic(false), airpods(true)], quick().cameras));
    let c = config_body(&r.scene);
    assert_eq!(picked(&c.mics), Some("System default"));
    assert_eq!(c.mics[0].detail.as_deref(), Some("Bagrat's AirPods Pro"));
    arm_and_start(&mut r);
    assert_eq!(started_mic(&r).as_deref(), Some("airpods"), "the take records the default now");
}

#[test]
fn a_microphone_picked_by_name_stays_picked_and_system_default_brings_the_following_back() {
    let mut w = ready();
    w.mac.mics = Fact::known(vec![built_in_mic(true), airpods(false)], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.act(Action::SetMic(Some("BuiltInMicrophoneDevice".into())));
    r.send(devices(vec![built_in_mic(false), airpods(true)], quick().cameras));
    assert_eq!(picked(&config_body(&r.scene).mics), Some("MacBook Pro Microphone"), "a pick by name isn't moved by the default");
    r.act(Action::SetMicDefault);
    assert_eq!(picked(&config_body(&r.scene).mics), Some("System default"));
    assert_eq!(r.w.prefs.mic, None);
    assert!(!r.w.prefs.mic_off);
    // System default after No mic turns the sound back on.
    r.act(Action::SetMic(None));
    r.act(Action::SetMicDefault);
    assert!(!r.w.prefs.mic_off);
    arm_and_start(&mut r);
    assert_eq!(started_mic(&r).as_deref(), Some("airpods"));
}

#[test]
fn a_default_that_is_an_iphone_is_never_taken_by_itself() {
    let mut w = ready();
    let iphone = mic("iphone-mic", "Bagrat's iPhone Microphone", false, true, true);
    w.mac.mics = Fact::known(vec![built_in_mic(false), iphone], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let r = Run::new(w);
    let c = config_body(&r.scene);
    assert_eq!(c.mics[0].detail.as_deref(), Some("MacBook Pro Microphone"), "N39: the built-in one instead");
}

#[test]
fn a_take_keeps_its_microphone_when_the_default_moves_mid_take() {
    let mut w = ready();
    w.mac.mics = Fact::known(vec![built_in_mic(true), airpods(false)], Mono(0));
    w.take = Some(take(Phase::Recording, 61_000));
    w.take.as_mut().unwrap().mic = Some("BuiltInMicrophoneDevice".into());
    let mut r = Run::new(w);
    r.send(devices(vec![built_in_mic(false), airpods(true)], quick().cameras));
    assert!(!r.has(|e| matches!(e, Effect::Recorder(_))), "nothing switches mid-take (B149): {:?}", r.effects);
    assert_eq!(r.w.take.as_ref().unwrap().mic.as_deref(), Some("BuiltInMicrophoneDevice"));
    assert_eq!(r.w.take.as_ref().unwrap().phase, Phase::Recording);
}

#[test]
fn the_cameras_system_default_turns_the_bubble_on_and_follows_macos() {
    let ext = Device { uid: "ext".into(), name: "Logitech BRIO".into(), builtin: false, continuity: false, system_default: false };
    let mut w = ready();
    w.prefs.show_yourself = false;
    w.prefs.camera = Some("cam".into());
    w.mac.cameras = Fact::known(vec![quick().cameras[0].clone(), ext.clone()], Mono(0));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("Camera off"));
    r.act(Action::SetCameraDefault);
    assert!(r.w.prefs.show_yourself);
    assert_eq!(r.w.prefs.camera, None);
    assert_eq!(picked(&config_body(&r.scene).cameras), Some("System default"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("cam"));
    // macOS prefers the webcam now: the bubble follows.
    let ext_default = Device { system_default: true, ..ext };
    r.send(devices(quick().mics, vec![quick().cameras[0].clone(), ext_default]));
    assert_eq!(config_body(&r.scene).cameras[0].detail.as_deref(), Some("Logitech BRIO"));
    assert_eq!(r.scene.bubble.as_ref().map(|b| b.camera.as_str()), Some("ext"));
}
