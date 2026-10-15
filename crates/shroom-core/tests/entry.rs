//! The independent entry oracle's own tests (TESTING-PLAN §2.1, ARCHITECTURE §3 / §3.7
//! item 3; DECISIONS B89). The oracle itself lives in `common/entry.rs`, so that every
//! walkthrough step (`support::Run::send`) and every world of the enumeration
//! (`tests/enumerate.rs`) is held to it; here it is held to the fixtures:
//!
//! - every reading rests on its fixture's own entry conditions, word for word;
//! - every state of the surfaces it reads is read, or named out of its scope with why;
//! - it has teeth: a Scene that says anything else is caught, surface by surface;
//! - the one core bug it found stays fixed (a click in the count's hand-off).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use laws::entry::{self, ABSTAINS, READS};
use shroom_core::event::{Event, Key, RecNews, Surface};
use shroom_core::scene::{Click, Face};
use shroom_core::time::Mono;
use shroom_core::world::*;
use support::*;

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn every_reading_rests_on_its_fixtures_entry_conditions() {
    let deviations = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/DECISIONS.md")).expect("docs/DECISIONS.md");
    let mut bad = Vec::new();
    for r in READS {
        if let Some(n) = r.rests_on.strip_prefix("DEVIATIONS #") {
            // An id the build added: its entry names it.
            let row = deviations.lines().find(|l| l.starts_with(&format!("| {n} |")));
            if !row.is_some_and(|l| l.contains(&format!("`{}`", r.id))) {
                bad.push(format!("{}: DEVIATIONS #{n} doesn't name it", r.id));
            }
            continue;
        }
        // An id two regions share (`stopping`) is read from any of them.
        let states: Vec<_> = spec().states().filter(|s| s.id == r.id).collect();
        if states.is_empty() {
            bad.push(format!("{}: no such fixture state", r.id));
        } else if !states.iter().any(|s| squash(&s.entry_conditions).contains(&squash(r.rests_on))) {
            bad.push(format!("{}: its entry conditions don't say “{}”", r.id, r.rests_on));
        }
    }
    let mut ids: Vec<&str> = READS.iter().map(|r| r.id).collect();
    ids.sort();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "an id read twice");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(ABSTAINS.iter().all(|(what, why)| !what.is_empty() && why.len() > 40), "every abstention says why");
}

/// The fixture states of the surfaces the oracle reads that it doesn't name, each with why.
const OUT_OF_SCOPE: &[(&str, &str)] = &[
    ("stopping", "an `other`-surface beat between paused and the page — the popover closes and the tray shows the take's beat (ABSTAINS: its face)"),
    ("guided-agent-overlay", "the BYO agent's overlay is the agent's (B81), not the app's"),
    ("tray-frost-dot", "the Frost dot read as `update-ready`'s tray face (`Frost dot`)"),
    ("installing", "the installer's terminal (installer/test)"),
    ("install-net-drop", "the installer's terminal (installer/test)"),
    ("install-checksum-fail", "the installer's terminal (installer/test)"),
    ("install-unsupported-os", "the installer's terminal (installer/test)"),
    ("install-refused-midtake", "the installer's terminal (installer/test)"),
    ("updating", "the installer's terminal (installer/test)"),
    ("token-repair-mood", "Settings · Space's nest (ABSTAINS: the nest beneath the mood)"),
    ("token-other-account", "Settings · Space's nest (ABSTAINS)"),
    ("space-storage-gone", "Settings · Space's nest (ABSTAINS)"),
    ("name-changed", "Settings · Space's nest (ABSTAINS)"),
    ("re-entry-as-settings", "the inspection mood's Region 2 name — read as `settings · Space · inspection`"),
];

#[test]
fn every_state_of_the_surfaces_it_reads_is_read_or_named_out_of_scope() {
    let read: Vec<&str> = READS.iter().map(|r| r.id).collect();
    let mut missing = Vec::new();
    for s in spec().states() {
        // The tray, the popover, its banners and status line, the overlays, the onboarding
        // and Settings windows' states (Regions 1–3, 5) — not the invite window (Region 6:
        // ABSTAINS), the Library (ABSTAINS), the pages or the terminal's states.
        let surface = matches!(s.surface.as_str(), "tray" | "popover" | "banner" | "overlay" | "status-line")
            || (s.surface == "window" && !s.id.starts_with("library") && !s.id.starts_with("join") && !s.id.contains("member-welcome") && !s.id.starts_with("joining") && !s.id.starts_with("invite-born") && s.id != "add-another-mac" && s.id != "member-remove-confirm")
            || (s.surface == "other" && s.kind != "web-page");
        let region6_window = s.surface == "window" && s.id.ends_with("-own-mac") && s.id != "member-all-set-own-mac";
        if !surface || region6_window || s.kind == "sheet" || read.contains(&s.id.as_str()) {
            continue;
        }
        if ["disconnect-confirm", "disconnect-confirm-no-token", "leave-confirm"].contains(&s.id.as_str()) {
            continue;
        }
        if !OUT_OF_SCOPE.iter().any(|(id, _)| *id == s.id) {
            missing.push(s.id.clone());
        }
    }
    assert!(missing.is_empty(), "states the oracle neither reads nor names out of scope: {missing:?}");
}

/// The oracle catches a Scene that says anything else — the tray, its face, the popover, its
/// banner, the count, the bubble, the onboarding window, Settings' mood.
#[test]
fn the_oracle_has_teeth() {
    // config on a ready Mac, a saved take waiting.
    let mut w = ready();
    w.session.popover = Some(Door::asked(Mono(900)));
    w.saved = vec![saved(SavedCause::Interrupted, 272_000)];
    let s = shroom_core::derive(&w);
    assert!(entry::disagreements(&w, &s).is_empty(), "{:?}", entry::disagreements(&w, &s));
    let caught = |m: &dyn Fn(&mut shroom_core::scene::Scene)| {
        let mut bad = s.clone();
        m(&mut bad);
        !entry::disagreements(&w, &bad).is_empty()
    };
    assert!(caught(&|s| s.tray.state = Some("idle-unprovisioned")));
    assert!(caught(&|s| s.tray.face = Face::Plain), "the amber dot, read from the saved take");
    assert!(caught(&|s| s.popover.as_mut().unwrap().id = "quit-confirm"));
    assert!(caught(&|s| s.popover.as_mut().unwrap().banner.as_mut().unwrap().id = "recording-off"), "N38's order");
    assert!(caught(&|s| s.popover.as_mut().unwrap().faces.push("disk-low")));
    assert!(caught(&|s| s.popover = None));
    // The welcome on a fresh Mac, opened by the installer's launch.
    let mut w = World::blank();
    w.launch.kind = LaunchKind::Install { join: false };
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(0)));
    let s = shroom_core::derive(&w);
    assert!(entry::disagreements(&w, &s).is_empty(), "{:?}", entry::disagreements(&w, &s));
    let mut bad = s.clone();
    bad.onboarding.as_mut().unwrap().id = "account";
    assert!(!entry::disagreements(&w, &bad).is_empty());
    let mut bad = s.clone();
    bad.onboarding.as_mut().unwrap().faces.clear();
    assert!(!entry::disagreements(&w, &bad).is_empty(), "first-launch is the installer's window");
    // Settings · Space on a member's Mac.
    let mut w = ready();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, ..identity() });
    w.space.token = false;
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    let s = shroom_core::derive(&w);
    assert!(entry::disagreements(&w, &s).is_empty(), "{:?}", entry::disagreements(&w, &s));
    let mut bad = s.clone();
    let st = bad.settings.as_mut().unwrap();
    st.faces.retain(|f| *f != "settings-space-member");
    st.faces.push("settings-space-inspection");
    assert!(!entry::disagreements(&w, &bad).is_empty());
    // The count, and the bubble over it.
    let mut w = ready();
    w.prefs.show_yourself = true;
    w.session.arming = Some(Arming { started: Mono(900), deadline: Mono(3_900), source: SourceId::Display(1), restart_of: None, fired: false });
    let s = shroom_core::derive(&w);
    assert!(entry::disagreements(&w, &s).is_empty(), "{:?}", entry::disagreements(&w, &s));
    let mut bad = s.clone();
    bad.countdown = None;
    assert!(!entry::disagreements(&w, &bad).is_empty());
    let mut bad = s.clone();
    bad.bubble = None;
    assert!(!entry::disagreements(&w, &bad).is_empty());
}

/// The bug the popover/tray enumeration found (B89): between the count firing and the
/// recorder confirming capture began, a tray click opened `config` over the starting take
/// (law 4). The beat is the take's own, like `Starting`: a click opens nothing.
#[test]
fn a_click_in_the_counts_hand_off_opens_nothing() {
    let mut r = Run::new(ready());
    r.click();
    r.key(Surface::Popover, Key::Return);
    assert!(r.scene.countdown.is_some());
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    assert!(r.w.session.arming.as_ref().is_some_and(|a| a.fired), "the count fired; the recorder is starting");
    assert_eq!(r.scene.tray.click, Click::Nothing);
    r.click();
    assert!(r.scene.popover.is_none(), "nothing pops over the take's start: {:?}", r.ids());
    r.send(Event::TrayClicked { right: true, quick: quick() });
    assert!(r.scene.popover.is_none(), "a right-click mid-take is a left click (N39)");
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!(r.scene.tray.state, Some("recording"));
    // …and the click is the pause again once capture began.
    r.click();
    assert!(r.has(|e| matches!(e, shroom_core::Effect::Recorder(shroom_core::effect::RecCmd::Pause))));
    // A Restart's hand-off over a paused take: the old take's popover stays shut too.
    let mut w = ready();
    w.take = Some(take(Phase::Paused(Cause::User), 60_000));
    w.session.arming = Some(Arming { started: Mono(800), deadline: Mono(900), source: SourceId::Display(1), restart_of: Some(TakeDir("20261002-120000-ab12".into())), fired: true });
    let mut r = Run::new(w);
    r.click();
    assert!(r.scene.popover.is_none(), "{:?}", r.ids());
}
