//! The machine's drawn edges (TESTING-PLAN §2.2, ARCHITECTURE §3.7 item 4) for Region 3 —
//! the recording machine — and Region 4's core half, plus the checks that hold the core to
//! spec/machine.json everywhere: every `Arm.edge` the core can name is an edge of the
//! machine (the walkthroughs check each emitted one at every step, `support::Run::send`);
//! and every drawn edge belongs to some suite's edge test.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::collections::BTreeSet;

use shroom_core::effect::{CloudCmd, Effect, RecCmd};
use shroom_core::event::{CloudNews, Event, Key, OsNews, RecNews, Surface, UploadNews};
use shroom_core::scene::Action;
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{Arm, derive};
use support::*;

// ── builders ──

fn with_popover(mut w: World) -> World {
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

fn recording() -> World {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 61_000));
    w
}

fn recording_with_mic() -> World {
    let mut w = recording();
    if let Some(t) = w.take.as_mut() {
        t.mic = Some("BuiltInMicrophoneDevice".into());
    }
    w
}

fn pausing(cause: Cause) -> World {
    let mut w = ready();
    w.take = Some(take(Phase::Pausing(cause), 61_000));
    w.inflight.ops.insert(Op::RecPause, Mono(6_000));
    w
}

fn paused() -> World {
    let mut w = with_popover(ready());
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w
}

fn discard_confirm() -> World {
    let mut w = paused();
    w.session.confirm = Some(Confirm::Discard);
    w
}

fn restart_confirm() -> World {
    let mut w = paused();
    w.session.confirm = Some(Confirm::Restart);
    w
}

fn recovered() -> World {
    let mut w = with_popover(ready());
    w.saved.push(saved(SavedCause::Interrupted, 272_000));
    w
}

fn arming() -> World {
    let mut w = ready();
    w.session.arming = Some(Arming { started: Mono(900), deadline: Mono(3_900), source: SourceId::Display(1), restart_of: None, fired: false });
    w
}

/// The count at its last digit: the next Tick is its deadline.
fn elapsing() -> World {
    let mut w = ready();
    w.session.arming = Some(Arming { started: Mono(0), deadline: Mono(1_005), source: SourceId::Display(1), restart_of: None, fired: false });
    w
}

fn draining() -> World {
    let mut w = with_popover(ready());
    for i in 0..2 {
        w.queue.push(Queued { dir: TakeDir(format!("q{i}")), rec: RecId(format!("r{i}")), title: "t".into(), stopped: Wall(0), done: 1, total: 2, blocked: None, length_ms: Some(1_000) });
    }
    w
}

/// A take whose Stop is finalizing — the stop's beat (`stopping`), online or not.
fn stopping(online: bool) -> World {
    let mut w = ready();
    let mut t = take(Phase::Paused(Cause::User), 272_000);
    t.phase = Phase::Finalizing(Why::Stop);
    w.take = Some(t);
    w.inflight.ops.insert(Op::RecFinalize, Mono(6_000));
    w.net.online = Fact::known(online, Mono(0));
    w
}

/// A World run through events with the laws checked at every step.
fn feed(w: World, events: impl IntoIterator<Item = Event>) -> World {
    let mut r = Run::new(w);
    for e in events {
        r.send(e);
    }
    r.w
}

fn stopped_offline() -> World {
    feed(stopping(false), [Event::Rec(RecNews::Finalized(Ok(272_000)))])
}

fn act(action: Action) -> Event {
    Event::Act { surface: Surface::Popover, action, rev: 0 }
}

fn popover_id(r: &Run) -> Option<&'static str> {
    r.scene.popover.as_ref().map(|p| p.id)
}

fn banner_id(r: &Run) -> Option<&'static str> {
    r.scene.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| b.id)
}

// ── Regions 3 and 4: every edge realized, or listed with its reason ──

/// Each edge: its start world, the events that cross it (one, or one plus the beat the map
/// doesn't draw — ARCHITECTURE §3.7's BEATS: the recorder's confirmation of a pause, a
/// resume or a finalize, and the stop's reach probe), and what “arrived” means.
type Arrived = fn(&Run, &[Effect]) -> bool;

fn realized() -> Vec<(&'static str, &'static str, World, Vec<Event>, Arrived)> {
    use Event as E;
    let devices_without = |mic: bool| {
        let q = quick();
        let mics = if mic { vec![] } else { q.mics.clone() };
        let displays = if mic { q.displays.clone() } else { vec![] };
        E::Os(OsNews::Devices { displays, mics, cameras: q.cameras })
    };
    vec![
        ("idle-ready", "config", ready(), vec![E::TrayClicked { right: false, quick: quick() }], |r, _| popover_id(r) == Some("config")),
        ("config", "idle-ready", with_popover(ready()), vec![E::Key { surface: Surface::Popover, key: Key::Esc }], |r, _| r.scene.popover.is_none() && r.ids().contains(&"idle-ready")),
        // A map note, not an edge: config's status line says what there is to say (here, an upload draining).
        ("config", "statusnote", draining(), vec![E::Upload(UploadNews::Progress { dir: TakeDir("q0".into()), done: 1, total: 2 })], |r, _| {
            popover_id(r) == Some("config") && r.scene.popover.as_ref().is_some_and(|p| p.status_line.is_some())
        }),
        ("config", "countdown", with_popover(ready()), vec![act(Action::Start)], |r, _| r.ids().contains(&"countdown") && r.scene.popover.is_none()),
        ("countdown", "config", arming(), vec![E::TrayClicked { right: false, quick: quick() }], |r, _| popover_id(r) == Some("config") && r.scene.countdown.is_none()),
        // 3-2-1 elapses (the Tick at its deadline asks for capture, the scrim lifts); the
        // recorder's start is the beat.
        ("countdown", "recording", elapsing(), vec![E::Tick, E::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) })], |r, _| {
            r.ids().contains(&"tray-red-dot") && r.w.take.as_ref().is_some_and(|t| t.phase == Phase::Recording)
        }),
        // The tray click asked for the pause (Pausing); the popover waits for its confirmation.
        ("recording", "paused", pausing(Cause::User), vec![E::Rec(RecNews::Paused { length_ms: 272_000 })], |r, _| popover_id(r) == Some("paused")),
        ("recording", "display-lost", recording(), vec![devices_without(false), E::Rec(RecNews::Paused { length_ms: 61_000 })], |r, _| {
            r.scene.popover.as_ref().is_some_and(|p| p.id == "paused" && p.faces.contains(&"display-lost"))
        }),
        ("recording", "mic-lost", recording_with_mic(), vec![devices_without(true), E::Rec(RecNews::Paused { length_ms: 61_000 })], |r, _| {
            r.scene.popover.as_ref().is_some_and(|p| p.id == "paused" && p.faces.contains(&"mic-lost"))
        }),
        ("recording", "disk-full-stop", recording(), vec![E::Rec(RecNews::DiskFloor), E::Rec(RecNews::Finalized(Ok(724_000)))], |r, _| popover_id(r) == Some("disk-full-stop")),
        ("paused", "recording", paused(), vec![act(Action::Resume), E::Rec(RecNews::Resumed)], |r, _| {
            r.ids().contains(&"tray-red-dot") && r.scene.popover.is_none()
        }),
        // Restart asks first (DEVIATIONS #44, B134): the question, then the count once its
        // Restart is clicked.
        ("paused", "restart-confirm", paused(), vec![act(Action::Restart)], |r, _| popover_id(r) == Some("restart-confirm")),
        ("restart-confirm", "countdown", restart_confirm(), vec![act(Action::ConfirmRestart)], |r, _| r.ids().contains(&"countdown")),
        // `stopping` is a beat no surface shows: the popover closes and the take finalizes.
        ("paused", "stopping", paused(), vec![act(Action::Stop)], |r, fx| {
            r.scene.popover.is_none() && fx.iter().any(|e| matches!(e, Effect::Recorder(RecCmd::Finalize { .. })))
        }),
        ("paused", "discard-confirm", paused(), vec![act(Action::Discard)], |r, _| popover_id(r) == Some("discard-confirm")),
        ("discard-confirm", "paused", discard_confirm(), vec![E::Key { surface: Surface::Popover, key: Key::Esc }], |r, _| popover_id(r) == Some("paused")),
        ("discard-confirm", "countdown", discard_confirm(), vec![act(Action::RestartInstead)], |r, _| r.ids().contains(&"countdown")),
        ("discard-confirm", "idle-ready", discard_confirm(), vec![act(Action::ConfirmDiscard)], |r, fx| {
            r.w.take.is_none() && fx.iter().any(|e| matches!(e, Effect::Recorder(RecCmd::Discard(_)))) && r.ids().contains(&"plain")
        }),
        // A crash: the relaunch's takes scan finds the take, truncated but valid.
        ("recording", "recovered-take", with_popover(ready()), vec![E::Scanned { saved: vec![saved(SavedCause::Interrupted, 272_000)], queue: vec![], removed: vec![] }], |r, _| {
            banner_id(r) == Some("recovered-take")
        }),
        ("recovered-take", "stopping", recovered(), vec![act(Action::GetLink), E::Rec(RecNews::Finalized(Ok(272_000)))], |_, fx| fx.iter().any(|e| matches!(e, Effect::Publish { .. }))),
        ("recovered-take", "discard-confirm", recovered(), vec![act(Action::DiscardSaved)], |r, _| popover_id(r) == Some("discard-confirm")),
        ("config", "library", with_popover(ready()), vec![act(Action::OpenLibrary)], |r, _| r.scene.library.is_some() && r.scene.popover.is_none()),
        ("config", "recording-off", with_popover(ready()), vec![E::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted })], |r, _| {
            banner_id(r) == Some("recording-off")
        }),
        ("config", "quit-confirm", draining(), vec![act(Action::Quit)], |r, _| popover_id(r) == Some("quit-confirm")),
        // ── Region 4's core half: the stop arriving ──
        ("stopping", "owner-page", stopping(true), vec![E::Rec(RecNews::Finalized(Ok(272_000))), E::Cloud(CloudNews::Reached(Reach::Shroom))], |_, fx| {
            fx.iter().any(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_))))
        }),
        ("stopping", "stopped-offline", stopping(false), vec![E::Rec(RecNews::Finalized(Ok(272_000)))], |r, _| popover_id(r) == Some("stopped-offline")),
        ("stopped-offline", "owner-page", stopped_offline(), vec![E::Os(OsNews::NetPath(true)), E::Cloud(CloudNews::Reached(Reach::Shroom))], |_, fx| {
            fx.iter().any(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_))))
        }),
    ]
}

/// What a start world shows when the edge's own `from` isn't on screen: a crash is seen at
/// the next launch, and the stop's beat is the take finalizing under the pause bars.
fn starts(from: &str, to: &str) -> Vec<&'static str> {
    match (from, to) {
        ("recording", "recovered-take") => vec!["idle-ready"],
        ("stopping", _) => vec!["tray-pause-bars"],
        _ => vec![],
    }
}

/// The map's notes walked here: not states, so not edges.
const NOTES: &[&str] = &["statusnote"];

/// Crossings the core derives without an arm naming them — law 1: the face follows the
/// facts — each with what moves it.
const UNNAMED: &[(&str, &str, &str)] = &[
    ("idle-ready", "config", "a tray click is the door: derive resolves it (law 5)"),
    ("config", "idle-ready", "Esc or a click away closes the popover; nothing else changes"),
    ("config", "statusnote", "a note: the status line follows the upload's facts"),
    ("discard-confirm", "paused", "Back/Esc clears the question; the paused popover is what's left"),
    ("recording", "recovered-take", "the takes scan reports the take; the banner is derived"),
    ("recovered-take", "stopping", "Get link confirms the saved take; Stop's finalize arm names the stop's own end"),
    ("recovered-take", "discard-confirm", "Discard… on the banner asks N25's question"),
    ("config", "recording-off", "the grant report flips the fact; the banner is derived"),
    ("stopped-offline", "owner-page", "back online, the reach probe answers and the page opens itself (N40)"),
];

/// Region 4's edges no app event crosses — the page's own faces — each with the Worker
/// test that realizes it (`worker/test/<file>`, the `it(...)` title verbatim).
const WORKER_EDGES: &[(&str, &str, &str, &str)] = &[
    ("owner-page", "fully-live", "page.test.js", "drops the upload line once the count is reached — no “Fully live ✓” on the page"),
    ("owner-page", "waiting-conn", "owner.test.js", "shows the upload line while bytes arrive, then waiting-for-connection when they stop"),
    ("waiting-conn", "owner-page", "page.test.js", "an upload whose progress moves is never stalled; one whose updatedAt aged is"),
    ("owner-page", "live", "worker.test.js", "plays a live video from its master playlist, offering Download only when the file exists"),
    ("owner-page", "on-its-way", "worker.test.js", "never 404s an unknown id — it looks like a video on its way, with no title"),
    ("on-its-way", "live", "page.test.js", "puts the line where the player will be, over a thin bar filled to the upload's progress"),
    ("waiting-conn", "on-its-way", "worker.test.js", "shows the title while uploading, and calls it stalled after the window"),
    ("delete-confirm", "removed", "worker.test.js", "shows only the removed line for a tombstone — no title"),
    ("on-its-way", "on-its-way-stalled", "page.test.js", "rests the bar when the upload stalls"),
];

/// Region 3's chapter edges are onboarding.rs's; the drawn edges this slice touches that
/// another suite realizes, each with where.
const CHAPTER: &[&str] = &["done", "perm-mic", "perm-camera", "perm-screen", "relaunching", "perm-priming", "all-set"];
const ELSEWHERE: &[(&str, &str, &str)] = &[
    ("wiring", "all-set", "onboarding.rs — every_in_scope_drawn_edge_is_realized_or_out_of_scope (the setup's last step, N45)"),
    ("stopping", "stopped-stranded", "region6.rs — every_region6_and_deferred_drawn_edge_is_realized_or_listed"),
    ("landing", "live", "onboarding.rs — out of scope there: shroom.am, the web suite"),
    ("idle-ready", "space-degraded", "region5.rs — every_region5_and_afterlife_drawn_edge_is_realized"),
    ("config", "settings-general", "region5.rs — every_region5_and_afterlife_drawn_edge_is_realized"),
    ("member-done", "perm-mic", "region6.rs — every_region6_and_deferred_drawn_edge_is_realized_or_listed"),
    ("member-all-set", "config", "region6.rs — every_region6_and_deferred_drawn_edge_is_realized_or_listed"),
    ("recording", "camera-lost", "region3.rs — camera_lost_is_about_the_camera_being_recorded"),
    ("recording", "mac-away", "upkeep.rs — will_sleep_pauses_the_take_and_lets_the_mac_sleep_once_the_pause_lands"),
    ("recording", "mic-permission-lost", "region3.rs — permission_lost_mid_take_comes_from_the_poll_or_any_entrys_probe"),
];

#[test]
fn every_region3_and_region4_drawn_edge_is_realized_or_listed() {
    let in_scope = drawn_edges(|from, to| [from, to].iter().any(|s| matches!(region_of(s), 3 | 4)) && !CHAPTER.contains(&from));
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let mut failures = Vec::new();
    for (from, to, world, events, arrived) in realized() {
        assert!(NOTES.contains(&to) || covered.insert((from.into(), to.into())), "{from} → {to} realized twice");
        let before = derive(&world).state_ids();
        if !before.contains(&from) && !starts(from, to).iter().any(|s| before.contains(s)) {
            failures.push(format!("{from} → {to}: the start world derives {before:?}"));
            continue;
        }
        let crossing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut r = Run::new(world);
            let mut fx: Vec<Effect> = Vec::new();
            let mut named: Vec<(&'static str, &'static str)> = Vec::new();
            for e in events {
                r.send(e);
                fx.extend(r.effects.clone());
                named.extend(r.arm.edge);
            }
            let ok = arrived(&r, &fx);
            // The arm names the edge it crossed (ARCHITECTURE §3.7), unless it's a crossing
            // the facts derive by themselves.
            let names_it = named.contains(&(from, to)) || NOTES.contains(&to) || UNNAMED.iter().any(|(f, t, _)| *f == from && *t == to);
            (ok, names_it, format!("{:?} named {named:?}", r.ids()))
        }));
        match crossing {
            Ok((true, true, _)) => {}
            Ok((false, _, after)) => failures.push(format!("{from} → {to}: ended at {after}")),
            Ok((true, false, after)) => failures.push(format!("{from} → {to}: no arm names the edge — {after}")),
            Err(e) => failures.push(format!("{from} → {to}: {}", e.downcast_ref::<String>().cloned().unwrap_or_default().lines().next().unwrap_or("panicked"))),
        }
    }
    for (from, to, file, title) in WORKER_EDGES {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test").join(file);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(text.contains(&format!("it(\"{title}\"")), "{from} → {to}: worker/test/{file} has no test “{title}”");
        assert!(covered.insert(((*from).into(), (*to).into())), "{from} → {to} both realized and the Worker's");
    }
    for (from, to, _) in ELSEWHERE {
        assert!(covered.insert(((*from).into(), (*to).into())), "{from} → {to} listed twice");
    }
    for (from, to, _) in UNNAMED {
        assert!(realized().iter().any(|(f, t, ..)| f == from && t == to), "UNNAMED lists {from} → {to}, which isn't realized here");
    }
    let missing: Vec<_> = in_scope.difference(&covered).collect();
    let stale: Vec<_> = covered.iter().filter(|(f, t)| spec().edge(f, t).is_none()).collect();
    assert!(missing.is_empty(), "drawn edges neither realized nor listed: {missing:?}");
    assert!(stale.is_empty(), "listed edges the machine doesn't have: {stale:?}");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ── the converse: the core names nothing the spec doesn't draw ──

/// Every edge an arm can name, read from the core's source: the literal pairs, and the
/// computed ends (`join`'s, `quit-asked`'s and `restart`'s `from`).
fn arm_edges_in_source() -> BTreeSet<(String, String)> {
    let src = include_str!("../src/apply.rs");
    let mut out = BTreeSet::new();
    // ("a", "b") — two string literals in a tuple.
    for (at, _) in src.match_indices("(\"") {
        let a_start = at + 2;
        let Some(a_len) = src[a_start..].find('"') else { continue };
        let Some(rest) = src[a_start + a_len + 1..].strip_prefix(", \"") else { continue };
        let Some(b_len) = rest.find('"') else { continue };
        if rest[b_len + 1..].starts_with(')') {
            out.insert((src[a_start..a_start + a_len].to_string(), rest[..b_len].to_string()));
        }
    }
    for (from, to) in [("member-welcome", "joining"), ("join-switch-confirm", "joining"), ("config", "quit-confirm"), ("idle-ready", "quit-confirm"), ("restart-confirm", "countdown"), ("discard-confirm", "countdown")] {
        out.insert((from.to_string(), to.to_string()));
    }
    out
}

#[test]
fn every_edge_an_arm_names_is_an_edge_of_the_machine() {
    let edges = arm_edges_in_source();
    assert!(edges.len() > 60, "the scan found only {} arm edges — has apply.rs's shape changed?", edges.len());
    let unlisted: Vec<_> = edges.iter().filter(|(a, b)| !edge_listed((a, b))).collect();
    assert!(unlisted.is_empty(), "arms name edges spec/machine.json doesn't have: {unlisted:?}");
    let _ = Arm::DROPPED;
}

/// Every drawn edge belongs to one suite's edge test: onboarding.rs (Regions 1–2 and the
/// permissions chapter), this file (Region 3 past the chapter, Region 4), region5.rs
/// (Region 5 and the wizard's afterlife), region6.rs (Region 6 and the deferred edges).
#[test]
fn every_drawn_edge_belongs_to_an_edge_test() {
    const AFTERLIFE: &[&str] = &["space-degraded", "repair-mood", "space-storage-gone", "name-changed"];
    let owned = |from: &str, to: &str| {
        let ends = [from, to];
        let onboarding = ends.iter().any(|s| matches!(region_of(s), 1 | 2)) || CHAPTER.contains(&from);
        let here = ends.iter().any(|s| matches!(region_of(s), 3 | 4)) && !CHAPTER.contains(&from);
        let region5 = ends.iter().any(|s| region_of(s) == 5 || AFTERLIFE.contains(s));
        let region6 = ends.iter().any(|s| region_of(s) == 6);
        onboarding || here || region5 || region6
    };
    let orphans: Vec<_> = drawn_edges(|f, t| !owned(f, t)).into_iter().map(|(f, t)| format!("{f} → {t}")).collect();
    assert!(orphans.is_empty(), "drawn edges no edge test owns: {orphans:?}");
    assert_eq!(drawn_edges(|_, _| true).len(), 118, "the drawn edges: the map's 125, one id per state, the notes not edges, paused → countdown gone (B134), the camera's beat in the chapter (N43), Report a problem's two (B180)");
}

/// the machine's law 12, invariant 1, across the fixtures: every failure-state the core renders
/// belongs to exactly one region test's inventory (`laws::FAILURE_HOMES`, where each is
/// checked as its own state), and the installer's terminal failures each have their test.
#[test]
fn every_failure_state_is_checked_as_its_own_state() {
    let spec = support::spec();
    let installer = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../installer/test/install.test.mjs")).expect("the installer's tests");
    let mut failures = Vec::new();
    for st in spec.states().filter(|s| s.kind == "failure-state") {
        let homes = laws::FAILURE_HOMES.iter().filter(|(_, ids)| ids.contains(&st.id.as_str())).count();
        if laws::core_surface(&st.surface) {
            if homes != 1 {
                failures.push(format!("{}: in {homes} region tests' lists, not one", st.id));
            }
        } else if st.surface == "terminal" {
            if !installer.contains(&st.id) {
                failures.push(format!("{}: no installer test names it", st.id));
            }
        } else {
            failures.push(format!("{}: a failure-state on {} — who checks it?", st.id, st.surface));
        }
    }
    let listed: usize = laws::FAILURE_HOMES.iter().map(|(_, ids)| ids.len()).sum();
    let core = spec.states().filter(|s| s.kind == "failure-state" && laws::core_surface(&s.surface)).count();
    assert_eq!(listed, core, "FAILURE_HOMES lists an id the fixtures don't call a core failure-state");
    assert!(failures.is_empty(), "law 12:\n{}", failures.join("\n"));
}
