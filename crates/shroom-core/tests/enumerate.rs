//! TESTING-PLAN §2.1's exhaustive enumeration (ARCHITECTURE §3.7 item 3), for the
//! onboarding window and Settings · Space: hand-written odometers walk narrow projections of
//! the World — the facts each surface reads, a few values each — and every world is derived.
//!
//! Per world, the three failure classes the inventory tables can't catch:
//! - **no state**: the open window derives to nothing, or to an id the fixtures don't have;
//! - **two faces**: two states of the fixtures claim one world (or a plan with two current
//!   rows, or a Settings · Space in two moods);
//! - after the walk, **no world reaches** an inventory state in scope.
//!
//! Then one world per distinct face (id, faces, Return default, current row, nest) is fed an
//! event alphabet through `update` with the laws checked at every step, and every move of
//! the window must be one the spec lists: an edge of spec/machine.json, or one of its rules
//! with its spec line (the converse check, ARCHITECTURE §3.7 item 4).
//!
//! The popover and the tray get the same walk (B89, the deferral B56 named): a take's
//! popover over every phase and pause cause, and `config` over its banners, status lines
//! and the stops' popovers, by every kind of Mac (`popover_tray_*` below).
//!
//! Every world is also held to the independent entry oracle (`common/entry.rs`, B89): the
//! fixtures' entry conditions, read on their own, must name what `derive` shows.
//!
//! The sub-derives still read `&World` (ARCHITECTURE §3.7 planned narrow view structs);
//! the projections are built here instead, over a base World, and `projection_fields`
//! destructures `World` exhaustively so a new field fails to compile until a projection
//! takes it or it's named as fixed (DECISIONS B22, B56).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet};

use shroom_core::derive::{onboarding, settings};
use shroom_core::event::{ClaimFail, CloudNews, Event, Field, Key, OsNews, RecNews, SecretField, Shape, Surface, SweepAnswer, TokenFail};
use shroom_core::scene::{Action, Glyph, Nest, OnboardingBody, Scene, SettingsPane};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

/// Two fixture states one world may show at once, by design.
const COEXIST: &[(&str, &str, &str)] = &[
    ("welcome", "first-launch", "Region 1: first-launch is the welcome the installer's launch opened — the same window"),
    ("name", "wizard-unexpected", "DEVIATIONS #25: under the name row the line sits beneath the field, and Claim stays live"),
    ("name-taken", "wizard-unexpected", "DEVIATIONS #25: under the name row the line sits beneath the field, and Claim stays live"),
];

fn odometer(radices: &[usize], mut f: impl FnMut(&[usize])) {
    let mut d = vec![0; radices.len()];
    loop {
        f(&d);
        let mut i = 0;
        loop {
            if i == d.len() {
                return;
            }
            d[i] += 1;
            if d[i] < radices[i] {
                break;
            }
            d[i] = 0;
            i += 1;
        }
    }
}

/// Every field of the World, by name: the ones a projection walks, and the ones held fixed
/// at the base world's value. A field added to `World` fails to compile here.
#[allow(dead_code)]
fn projection_fields(w: &World) {
    let World {
        now: _,            // fixed: 10 s after boot
        wall: _,           // fixed
        launch: _,         // walked: kind (User / Install), ending; version fixed
        mac: _,            // walked: mic, screen, primed; the rest fixed at a ready Mac's
        net: _,            // walked: online
        space: _,          // walked: identity (role), token, broken
        sweep: _,          // walked: paste, token, gates, subdomain/label, found, taken, bucket, worker, pass
        take: _,           // walked (popover/tray): every phase and pause cause; else none; markers
                           // and marked_at_pause fixed at none (editor.rs walks N64's)
        saved: _,          // walked (popover/tray): none, or one saved take of each cause
        queue: _,          // walked (popover/tray): none, uploading, stranded
        removed: _,        // fixed: none
        library: _,        // fixed: unknown (so no row's edited_ms)
        library_ops: _,    // fixed: idle — no write landed, no edited length, no copy owed
        update: _,         // walked (popover/tray): ready, applying, failed, an update landed
        members: _,        // walked (popover/tray): standing — fine, revoked, a confirmed strike, behind; no invite (region6.rs's)
        inflight: _,       // walked: one failure class on the wizard's calls
        popover_closed: _, // walked (popover/tray): a one-shot line aged out or not
        prefs: _,          // walked: setup_chosen; settings_tab fixed at Space for Settings
        once: _,           // fixed
        session: _,        // walked: fork, chapter, the name field, settings.add_token/fresh, door asked;
                           // (popover/tray) the popover's door, its questions, the last stop, the countdown;
                           // the editor's fields fixed at none (region5.rs's rows and editor.rs walk N63)
        display_pick: _,   // fixed: no click's display, no hand pick (region3.rs walks N56)
    } = w;
}

/// What one walk collects.
#[derive(Default)]
struct Walk {
    worlds: usize,
    reached: BTreeSet<&'static str>,
    reps: BTreeMap<String, World>,
    failures: Vec<String>,
}

impl Walk {
    fn fail(&mut self, what: String) {
        if self.failures.len() < 40 && !self.failures.contains(&what) {
            self.failures.push(what);
        }
    }
}

fn spec_kind(id: &str) -> Option<&'static str> {
    spec().state(id).map(|s| s.kind.as_str())
}

fn known_id(id: &str) -> bool {
    spec().state(id).is_some()
}

fn nest_kind(n: &Nest) -> String {
    serde_json::to_value(n).ok().and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_string)).unwrap_or_default()
}

// ── the onboarding window ──

fn bagrat() -> Account {
    Account { id: "bagrat".into(), name: "Bagrat's Account".into() }
}

fn acme() -> Account {
    Account { id: "acme".into(), name: "Acme Engineering".into() }
}

/// A Mac with every local fact known and the onboarding window open (asked), 10 s in.
fn onboarding_base() -> World {
    let mut w = ready();
    w.space.identity = None;
    w.space.token = false;
    w.mac.primed = None;
    w.mac.screen = Fact::known(false, Mono(0));
    w.mac.mic = Fact::known(Access::NotAsked, Mono(0));
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.now = Mono(10_000);
    w.launch.version = "1.3.0".into();
    w.launch.booted = Mono(900);
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    w
}

fn fail_on(w: &mut World, ops: &[Op], class: Option<FailClass>) {
    if let Some(class) = class {
        for op in ops {
            w.inflight.failures.insert(*op, Failure { at: Mono(9_000), class, tries: 0 });
        }
    }
}

/// Projection 1 — the wizard, rows one to five, on a Mac with no space yet (~310 k worlds).
fn wizard_worlds(mut f: impl FnMut(World)) {
    let radices = [2, 2, 3, 6, 3, 4, 5, 2, 3, 2, 3, 2, 2];
    odometer(&radices, |d| {
        let mut w = onboarding_base();
        w.space.token = d[0] == 1;
        w.prefs.setup_chosen = d[1] == 1;
        w.session.fork = [Fork::Virgin, Fork::SigningUp, Fork::Affirmed][d[2]];
        w.sweep.paste = match d[3] {
            0 => None,
            1 => Some(Paste::Garbled),
            2 => Some(Paste::WrongPowers),
            3 => Some(Paste::OtherAccount),
            4 => Some(Paste::Checking(SecretId(1))),
            _ => Some(Paste::PickAccount { id: SecretId(1), accounts: vec![bagrat(), acme()] }),
        };
        w.sweep.token = match d[4] {
            0 => Fact::Unknown,
            1 => Fact::known(TokenVerdict::Valid { account: bagrat(), several: false }, Mono(9_500)),
            _ => Fact::known(TokenVerdict::WrongPowers, Mono(9_500)),
        };
        w.sweep.gates = match d[5] {
            0 => Fact::Unknown,
            1 => Fact::known(Gates { email: false, card: false }, Mono(9_500)),
            2 => Fact::known(Gates { email: true, card: false }, Mono(9_500)),
            _ => Fact::known(Gates { email: false, card: true }, Mono(9_500)),
        };
        let (sub, label) = match d[6] {
            0 => (Fact::Unknown, Fact::Unknown),
            1 => (Fact::known(None, Mono(9_500)), Fact::known(Some("v".to_string()), Mono(9_500))),
            2 => (Fact::known(Some("myshroom".to_string()), Mono(9_500)), Fact::known(Some("v".to_string()), Mono(9_500))),
            // 3: someone else's `v` was found; 4: `v` was passed by without a look (the
            // checkpoint) — the label is `shroom` both times, the clash only the first.
            _ => (Fact::known(Some("myshroom".to_string()), Mono(9_500)), Fact::known(Some("shroom".to_string()), Mono(9_500))),
        };
        w.sweep.subdomain = sub;
        w.sweep.label = label;
        w.sweep.foreign_v = Fact::known(d[6] == 3, Mono(9_500));
        if d[5] > 0 {
            w.sweep.pass = Some(Mono(9_500));
        }
        w.sweep.found = if d[7] == 1 { Fact::known(Some(14), Mono(9_500)) } else { Fact::Unknown };
        match d[8] {
            0 => {}
            1 => w.session.fields.name = "myshroomhq".into(),
            _ => {
                w.session.fields.name = "myshroom".into();
                w.sweep.taken = Some("myshroom".into());
            }
        }
        w.net.online = Fact::known(d[9] == 0, Mono(9_500));
        fail_on(&mut w, &Op::WIZARD, [None, Some(FailClass::Transport), Some(FailClass::Refused)][d[10]]);
        if d[11] == 1 {
            w.launch.kind = LaunchKind::Install { join: false };
            w.session.windows.insert(WindowKind::Onboarding, Door::asked(w.launch.booted));
        }
        // This Mac's permissions done before wiring was: wiring is the last step (N45).
        if d[12] == 1 {
            w.mac.primed = Some(Wall(1_790_000_000_000));
            w.mac.screen = Fact::known(true, Mono(9_500));
            w.mac.mic = Fact::known(Access::Granted, Mono(9_500));
            w.mac.camera = Fact::known(Access::Granted, Mono(9_500));
        }
        f(w);
    });
}

/// Projection 2 — row six, the permissions chapter, on a Mac in a space (every role).
fn chapter_worlds(mut f: impl FnMut(World)) {
    let radices = [3, 2, 4, 3, 2, 2, 2, 2, 2, 2, 2];
    odometer(&radices, |d| {
        let mut w = onboarding_base();
        let role = [Role::TokenHolder, Role::OwnerClass, Role::Member][d[0]];
        w.space.identity = Some(SpaceIdentity { role, ..identity() });
        // A space set up beside someone else's `v`: under `shroom`, and it remembers why.
        if d[8] == 1 {
            let id = w.space.identity.as_mut().unwrap();
            (id.label, id.host, id.foreign_v) = ("shroom".into(), "shroom.myshroom.workers.dev".into(), true);
        }
        w.space.token = role == Role::TokenHolder;
        w.session.chapter = d[1] == 1;
        w.mac.mic = match d[2] {
            0 => Fact::Unknown,
            1 => Fact::known(Access::NotAsked, Mono(0)),
            2 => Fact::known(Access::Granted, Mono(0)),
            _ => Fact::known(Access::Denied, Mono(0)),
        };
        w.mac.mic_asked = w.session.chapter && d[2] == 1;
        // The camera the mic's answer chained (N43): its prompt up, or answered.
        w.mac.camera_asked = w.session.chapter && d[2] >= 2 && d[9] == 1;
        w.mac.camera = Fact::known(if w.mac.camera_asked { Access::NotAsked } else { Access::Granted }, Mono(0));
        // How macOS took the screen ask: its own alert leads, or the pane shroom opened.
        w.mac.screen_alert = (d[10] == 1).then_some(true);
        w.mac.screen = match d[3] {
            0 => Fact::Unknown,
            1 => Fact::known(false, Mono(0)),
            _ => Fact::known(true, Mono(0)),
        };
        w.launch.ending = d[4] == 1;
        w.mac.primed = (d[5] == 1).then_some(shroom_core::time::Wall(1_789_000_000_000));
        w.net.online = Fact::known(d[6] == 0, Mono(9_500));
        if d[7] == 1 {
            w.session.windows.insert(WindowKind::Onboarding, Door::unasked(Mono(900)));
        }
        f(w);
    });
}

/// The entry oracle (B89) against the whole Scene of one world.
fn oracle(walk: &mut Walk, w: &World, scene: &Scene) {
    let d = laws::entry::disagreements(w, scene);
    if !d.is_empty() {
        walk.fail(format!("the entry oracle disagrees: {}", d.join("; ")));
    }
}

fn check_onboarding(walk: &mut Walk, w: World) {
    walk.worlds += 1;
    oracle(walk, &w, &shroom_core::derive(&w));
    let Some(o) = onboarding::derive(&w) else {
        walk.fail(format!("no state: the open window derives nothing — {:?}", onboarding::at(&w)));
        return;
    };
    let all: Vec<&'static str> = std::iter::once(o.id).chain(o.faces.iter().copied()).collect();
    for id in &all {
        if !known_id(id) {
            walk.fail(format!("no state: “{id}” isn't in the machine (at {:?})", onboarding::at(&w)));
        }
        walk.reached.insert(id);
    }
    // One state claims once: wizard-unexpected is the window's id and, under the name row,
    // also its face (DEVIATIONS #25) — the same state, not two.
    let mut claim: Vec<&str> = all.iter().copied().filter(|id| matches!(spec_kind(id), Some("state" | "failure-state"))).collect();
    claim.dedup();
    let two = claim.len() > 1 && !(claim.len() == 2 && COEXIST.iter().any(|(a, b, _)| claim.contains(a) && claim.contains(b)));
    if two {
        walk.fail(format!("two faces: {claim:?} claim one world (at {:?})", onboarding::at(&w)));
    }
    if let OnboardingBody::Plan(p) = &o.body {
        let current: Vec<usize> = p.rows.iter().enumerate().filter(|(_, r)| r.glyph == Glyph::Current).map(|(i, _)| i + 1).collect();
        if current.len() > 1 {
            walk.fail(format!("two faces: rows {current:?} both current ({})", o.id));
        }
        // The nest sits under `current`; a row marked current is that row (space-found's
        // nest sits under its ticked row five, B52 d).
        if current.first().is_some_and(|r| Some(*r as u8) != p.current) {
            walk.fail(format!("the plan's nest sits under row {:?}, but row {current:?} is marked current ({})", p.current, o.id));
        }
        let nest = nest_kind(&p.nest);
        let sig = format!("{}|{:?}|{:?}|{:?}|{nest}", o.id, o.faces, o.ret, p.current);
        walk.reps.entry(sig).or_insert(w);
    } else {
        let sig = format!("{}|{:?}|{:?}", o.id, o.faces, o.ret);
        walk.reps.entry(sig).or_insert(w);
    }
}

// ── Settings · Space ──

fn settings_base() -> World {
    let mut w = ready();
    w.now = Mono(10_000);
    w.launch.version = "1.3.0".into();
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door { opened: Mono(900), entered: Mono(5_000), asked: true });
    w
}

/// Projection 3 — the token Mac's rows two to five: the repair mood and the inspection.
fn repair_worlds(mut f: impl FnMut(World)) {
    let radices = [5, 3, 4, 3, 3, 3, 2, 4, 2, 3];
    odometer(&radices, |d| {
        let mut w = settings_base();
        w.sweep.paste = match d[0] {
            0 => None,
            1 => Some(Paste::Garbled),
            2 => Some(Paste::WrongPowers),
            3 => Some(Paste::OtherAccount),
            _ => Some(Paste::Checking(SecretId(1))),
        };
        w.sweep.token = match d[1] {
            0 => Fact::Unknown,
            1 => Fact::known(TokenVerdict::Valid { account: bagrat(), several: false }, Mono(6_000)),
            _ => Fact::known(TokenVerdict::Revoked, Mono(6_000)),
        };
        w.sweep.gates = match d[2] {
            0 => Fact::Unknown,
            1 => Fact::known(Gates { email: false, card: false }, Mono(6_000)),
            2 => Fact::known(Gates { email: true, card: false }, Mono(6_000)),
            _ => Fact::known(Gates { email: false, card: true }, Mono(6_000)),
        };
        w.sweep.subdomain = match d[3] {
            0 => Fact::Unknown,
            1 => Fact::known(Some("myshroom".into()), Mono(6_000)),
            _ => Fact::known(Some("myshroomhq".into()), Mono(6_000)),
        };
        w.sweep.bucket = [Fact::Unknown, Fact::known(true, Mono(6_000)), Fact::known(false, Mono(6_000))][d[4]].clone();
        w.sweep.worker = [Fact::Unknown, Fact::known(WorkerFact::Ours, Mono(6_000)), Fact::known(WorkerFact::Absent, Mono(6_000))][d[5]].clone();
        w.session.fields.settings.fresh = d[6] == 1;
        // The space reported broken: unknown, sound, broken with this entry's re-probe
        // running, broken with it answered.
        match d[7] {
            0 => {}
            1 => w.space.broken = Fact::known(false, Mono(4_000)),
            2 => w.space.broken = Fact::known(true, Mono(4_000)),
            _ => {
                w.space.broken = Fact::known(true, Mono(4_000));
                w.sweep.pass = Some(Mono(6_000));
            }
        }
        w.net.online = Fact::known(d[8] == 0, Mono(6_000));
        fail_on(&mut w, &Op::WIZARD, [None, Some(FailClass::Transport), Some(FailClass::Refused)][d[9]]);
        f(w);
    });
}

/// Projection 4 — every kind of Mac's Space tab: row six, a token asked for here.
fn six_worlds(mut f: impl FnMut(World)) {
    let radices = [4, 2, 3, 2, 2, 2, 2, 2, 2, 3];
    odometer(&radices, |d| {
        let mut w = settings_base();
        let (role, token) = [(Role::TokenHolder, true), (Role::TokenHolder, false), (Role::OwnerClass, false), (Role::Member, false)][d[0]];
        w.space.identity = Some(SpaceIdentity { role, ..identity() });
        w.space.token = token;
        w.session.fields.settings.add_token = d[1] == 1;
        w.sweep.paste = [None, Some(Paste::Garbled), Some(Paste::Checking(SecretId(1)))][d[2]].clone();
        w.mac.screen = Fact::known(d[3] == 0, Mono(0));
        w.mac.primed = (d[4] == 0).then_some(shroom_core::time::Wall(1_789_000_000_000));
        w.session.chapter = d[5] == 1;
        w.launch.ending = d[6] == 1;
        w.net.online = Fact::known(d[7] == 0, Mono(6_000));
        w.mac.mic = Fact::known([Access::Granted, Access::Denied][d[8]], Mono(0));
        fail_on(&mut w, &Op::WIZARD, [None, Some(FailClass::Transport), Some(FailClass::Refused)][d[9]]);
        f(w);
    });
}

const MOODS: &[&str] = &["settings-space-repair", "settings-space-inspection", "settings-space-no-token", "settings-space-member"];
/// The faces that say what the tab is, not where its plan stands.
const FRAME: &[&str] = &["settings-space", "repair-mood", "re-entry-as-settings"];

fn check_settings(walk: &mut Walk, w: World) {
    walk.worlds += 1;
    oracle(walk, &w, &shroom_core::derive(&w));
    let Some(st) = settings::derive(&w) else {
        walk.fail("no state: Settings derives nothing for a provisioned Mac with its door open".into());
        return;
    };
    for id in std::iter::once(st.id).chain(st.faces.iter().copied()) {
        if !known_id(id) {
            walk.fail(format!("no state: “{id}” isn't in the machine ({:?})", settings::space_at(&w)));
        }
        walk.reached.insert(id);
    }
    let moods: Vec<&str> = st.faces.iter().copied().filter(|f| MOODS.contains(f)).collect();
    if moods.len() != 1 {
        walk.fail(format!("two faces: the Space tab in moods {moods:?} ({:?})", settings::space_at(&w)));
    }
    let nests: Vec<&str> = st.faces.iter().copied().filter(|f| !FRAME.contains(f) && !MOODS.contains(f) && matches!(spec_kind(f), Some("state" | "failure-state"))).collect();
    if nests.len() > 1 {
        walk.fail(format!("two faces: {nests:?} claim the Space tab ({:?})", settings::space_at(&w)));
    }
    let SettingsPane::Space(body) = &st.body.pane else {
        walk.fail("Settings opened on the Space tab shows another pane".into());
        return;
    };
    let current: Vec<usize> = body.plan.rows.iter().enumerate().filter(|(_, r)| r.glyph == Glyph::Current).map(|(i, _)| i + 1).collect();
    if current.len() > 1 {
        walk.fail(format!("two faces: rows {current:?} both current ({:?})", settings::space_at(&w)));
    }
    let sig = format!("{:?}|{:?}|{:?}|{}", st.faces, st.ret, body.plan.current, nest_kind(&body.plan.nest));
    walk.reps.entry(sig).or_insert(w);
}

// ── the converse: every move the window makes is one the spec lists ──

/// Where a Scene stands for the window under test: its most specific id, else what the
/// person sees instead (the popover, else the tray's state).
fn standing(scene: &Scene, kind: WindowKind) -> &'static str {
    match kind {
        WindowKind::Onboarding => match &scene.onboarding {
            Some(o) => o.id,
            None => scene.popover.as_ref().map(|p| p.id).or(scene.tray.state).unwrap_or("closed"),
        },
        _ => match &scene.settings {
            Some(st) => st.faces.last().copied().unwrap_or(st.id),
            None => scene.tray.state.unwrap_or("closed"),
        },
    }
}

/// Whether the machine lists a move of the window: an edge of spec/machine.json, or a rule
/// of it that holds for the move (B133: a move that holds across a group is one rule).
fn listed(from: &str, to: &str, ev: &Event, _beat: bool) -> Option<String> {
    spec().listed(from, to, &event_kind(ev))
}

/// The events a person, the OS and the lanes can send a face: every control it offers,
/// Return and Esc, ✕, the network flipping, grants, the next wake, a paste, typing — and the
/// answers to the calls it has out (a lane answers only what it was asked, ARCHITECTURE
/// §3.2: each op ends in exactly one answer or `Failed`).
fn alphabet(w: &World, kind: WindowKind) -> Vec<Event> {
    let scene = shroom_core::derive(w);
    let surface = if kind == WindowKind::Onboarding { Surface::Onboarding } else { Surface::Settings };
    let mut actions: Vec<Action> = Vec::new();
    if let Some(o) = &scene.onboarding {
        actions.extend(o.buttons().iter().map(|b| b.action.clone()));
        if let OnboardingBody::Plan(p) = &o.body {
            if let Nest::PickAccount { accounts, .. } = &p.nest {
                actions.extend(accounts.iter().map(|c| c.action.clone()));
            }
        }
    }
    if let Some(st) = &scene.settings {
        actions.extend(st.buttons().iter().map(|b| b.action.clone()));
    }
    let mut ev: Vec<Event> = actions.into_iter().map(|action| Event::Act { surface, action, rev: 0 }).collect();
    ev.extend([
        Event::Key { surface, key: Key::Return },
        Event::Key { surface, key: Key::Esc },
        Event::WindowClosed(kind),
        Event::Os(OsNews::NetPath(false)),
        Event::Os(OsNews::NetPath(true)),
        Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }),
        Event::Os(OsNews::Grants { screen: false, mic: Access::Denied, camera: Access::NotAsked }),
        Event::SecretPasted { field: SecretField::Token, id: SecretId(7), shape: Shape::Token },
        Event::SecretPasted { field: SecretField::Token, id: SecretId(8), shape: Shape::Garbled },
        Event::Field { field: Field::Name, value: "My Shroom".into() },
        Event::Field { field: Field::Name, value: "myshroomhq".into() },
    ]);
    let out = |op: Op| w.inflight.ops.contains_key(&op);
    let paste = match &w.sweep.paste {
        Some(Paste::Checking(id) | Paste::Saving { id, .. }) => Some(*id),
        _ => None,
    };
    if let (true, Some(id)) = (out(Op::Validate), paste) {
        ev.push(Event::Cloud(CloudNews::Validated { id, result: Ok(vec![bagrat()]) }));
        ev.push(Event::Cloud(CloudNews::Validated { id, result: Ok(vec![bagrat(), acme()]) }));
        ev.push(Event::Cloud(CloudNews::Validated { id, result: Err(TokenFail::WrongPowers) }));
        ev.push(Event::Cloud(CloudNews::Validated { id, result: Err(TokenFail::Rejected) }));
    }
    if let (true, Some(id)) = (out(Op::SaveToken), paste) {
        ev.push(Event::Cloud(CloudNews::TokenSaved(id)));
    }
    if out(Op::Sweep) {
        let sound = SweepAnswer {
            token: Some(TokenVerdict::Valid { account: bagrat(), several: false }),
            gates: Some(Gates { email: false, card: false }),
            subdomain: Some(Some("myshroom".into())),
            foreign_v: Some(false),
            label: Some(Some("v".into())),
            bucket: Some(true),
            worker: Some(WorkerFact::Ours),
            found: Some(None),
        };
        ev.push(Event::Cloud(CloudNews::Swept(sound.clone())));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { subdomain: Some(None), ..sound.clone() })));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { found: Some(Some(14)), ..sound.clone() })));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { gates: Some(Gates { email: true, card: false }), ..Default::default() })));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() })));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { bucket: Some(false), ..sound.clone() })));
        ev.push(Event::Cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::Absent), ..sound })));
    }
    if out(Op::Claim) {
        let name = w.session.fields.name.clone();
        ev.push(Event::Cloud(CloudNews::Claimed { name: name.clone(), result: Ok(()) }));
        ev.push(Event::Cloud(CloudNews::Claimed { name, result: Err(ClaimFail::Taken) }));
    }
    if out(Op::Wire) {
        ev.push(Event::Cloud(CloudNews::Wired(Ok(identity()))));
    }
    if out(Op::Prime) {
        ev.push(Event::Rec(RecNews::Primed(true)));
        ev.push(Event::Rec(RecNews::Primed(false)));
    }
    for op in w.inflight.ops.keys() {
        ev.push(Event::Failed { op: *op, class: FailClass::Transport });
        ev.push(Event::Failed { op: *op, class: FailClass::Refused });
    }
    ev.push(Event::Tick);
    ev
}

/// Each face, settled (a first Tick starts what its entry has due — its probes, its calls),
/// then each event of its alphabet from there, laws checked at every step.
fn converse(walk: &Walk, kind: WindowKind) -> (usize, Vec<String>) {
    let mut steps = 0;
    let mut answers = 0;
    let mut unlisted: BTreeSet<String> = BTreeSet::new();
    for rep in walk.reps.values() {
        let mut settled = Run::new(rep.clone());
        settled.send(Event::Tick);
        let (w, t) = (settled.w.clone(), settled.t);
        let before = standing(&settled.scene, kind);
        for ev in alphabet(&w, kind) {
            let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut r = Run::new(w.clone());
                r.t = t;
                if matches!(ev, Event::Tick) {
                    if let Some(next) = r.scene.next_wake {
                        r.tick_to(next.0.max(r.t + 20));
                    } else {
                        r.send(Event::Tick);
                    }
                } else {
                    r.send(ev.clone());
                }
                let beat = matches!(r.w.sweep.paste, Some(Paste::Checking(_) | Paste::Saving { .. }));
                (standing(&r.scene, kind), beat)
            }));
            steps += 1;
            answers += usize::from(matches!(ev, Event::Cloud(_) | Event::Rec(_) | Event::Failed { .. }));
            match ran {
                Ok((after, beat)) => {
                    if listed(before, after, &ev, beat).is_none() {
                        unlisted.insert(format!("{before} → {after} on {}", short(&ev)));
                    }
                }
                Err(e) => {
                    let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                    unlisted.insert(format!("{before} on {}: {}", short(&ev), msg.lines().next().unwrap_or("panicked")));
                }
            }
        }
    }
    // The lanes' answers are in the walk, not only the person's clicks.
    assert!(answers * 10 > steps, "only {answers} of {steps} steps were a lane's answer");
    (steps, unlisted.into_iter().collect())
}

fn short(ev: &Event) -> String {
    let s = format!("{ev:?}");
    s.chars().take(90).collect()
}

fn in_scope(region_states: &[&'static str], walk: &Walk) -> Vec<&'static str> {
    region_states.iter().copied().filter(|id| !walk.reached.contains(id)).collect()
}

#[test]
fn onboarding_enumeration_every_world_one_state_every_state_reached_every_move_listed() {
    let mut walk = Walk::default();
    wizard_worlds(|w| check_onboarding(&mut walk, w));
    chapter_worlds(|w| check_onboarding(&mut walk, w));
    assert!(walk.worlds > 200_000, "the walk covered only {} worlds", walk.worlds);
    assert!(walk.failures.is_empty(), "{} worlds:\n{}", walk.worlds, walk.failures.join("\n"));
    // Every onboarding inventory state of Regions 1–3 a Mac reaches without an invite.
    let unreached = in_scope(
        &[
            "welcome", "first-launch", "account", "account-signing-up", "token", "token-garbled", "token-wrong-powers", "token-pick-account", "card", "gate-email", "gate-card",
            "name", "name-taken", "name-v-clash", "wiring", "wizard-offline", "space-found", "done", "perm-mic", "perm-camera", "perm-screen", "relaunching", "perm-priming", "all-set",
            "member-done", "member-all-set", "member-all-set-own-mac", "wizard-unexpected",
        ],
        &walk,
    );
    assert!(unreached.is_empty(), "no world reaches {unreached:?}");
    let (steps, unlisted) = converse(&walk, WindowKind::Onboarding);
    assert!(steps > 500, "the converse ran only {steps} steps");
    assert!(unlisted.is_empty(), "{} faces × the alphabet = {steps} steps; moves the spec doesn't list:\n{}", walk.reps.len(), unlisted.join("\n"));
}

#[test]
fn settings_space_enumeration_every_world_one_state_every_state_reached_every_move_listed() {
    let mut walk = Walk::default();
    repair_worlds(|w| check_settings(&mut walk, w));
    six_worlds(|w| check_settings(&mut walk, w));
    assert!(walk.worlds > 80_000, "the walk covered only {} worlds", walk.worlds);
    assert!(walk.failures.is_empty(), "{} worlds:\n{}", walk.worlds, walk.failures.join("\n"));
    let unreached = in_scope(
        &[
            "settings", "settings-space", "settings-space-inspection", "settings-space-repair", "settings-space-no-token", "settings-space-member",
            "re-entry-as-settings", "repair-mood", "token-repair-mood", "token", "token-garbled", "token-wrong-powers", "token-other-account", "gate-email", "gate-card",
            "name-changed", "wiring", "space-storage-gone", "wizard-offline", "wizard-unexpected", "perm-screen", "relaunching",
        ],
        &walk,
    );
    assert!(unreached.is_empty(), "no world reaches {unreached:?}");
    let (steps, unlisted) = converse(&walk, WindowKind::Settings);
    assert!(steps > 500, "the converse ran only {steps} steps");
    assert!(unlisted.is_empty(), "{} faces × the alphabet = {steps} steps; moves the spec doesn't list:\n{}", walk.reps.len(), unlisted.join("\n"));
}

// ── the popover and the tray (B89; the walk B56 deferred) ──

const TAKE_DIR: &str = "20261002-120000-ab12";
const SAVED_DIR: &str = "20261001-090000-cd34";
const QUEUED_DIR: &str = "20261001-100000-ef56";

fn popover_base() -> World {
    let mut w = ready();
    w.now = Mono(100_000);
    w.launch.version = "1.3.0".into();
    w.launch.booted = Mono(900);
    w.once.seen_version = Some("1.3.0".into());
    w.mac.displays = Fact::known(
        vec![
            shroom_core::world::Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 },
            shroom_core::world::Display { id: 7, name: "LG UltraFine".into(), is_main: false, x: 1512, y: 0 },
        ],
        Mono(0),
    );
    w
}

/// Projection 5 — a take's popover and the tray during it: every phase and pause cause,
/// the Mac away or back, a Restart's count, the popover's door, its questions, the bubble's
/// facts, and the onboarding window opened before or after the take (law 4).
fn take_worlds(mut f: impl FnMut(World)) {
    use shroom_core::world::{Arming, Cause, Confirm, Phase, SourceId};
    let phases: [Option<Phase>; 13] = [
        None,
        Some(Phase::Starting),
        Some(Phase::Recording),
        Some(Phase::Pausing(Cause::User)),
        Some(Phase::Paused(Cause::User)),
        Some(Phase::Paused(Cause::DisplayLost)),
        Some(Phase::Paused(Cause::MicLost)),
        Some(Phase::Paused(Cause::CameraLost)),
        Some(Phase::Paused(Cause::Asleep)),
        Some(Phase::Paused(Cause::Locked)),
        Some(Phase::Paused(Cause::MicRevoked)),
        Some(Phase::Resuming),
        Some(Phase::Finalizing(shroom_core::world::Why::Stop)),
    ];
    let radices = [13, 2, 3, 3, 4, 2, 2, 2, 3];
    odometer(&radices, |d| {
        let mut w = popover_base();
        let camera_source = d[7] == 1;
        if let Some(phase) = phases[d[0]] {
            let mut t = take(phase, 272_000);
            if camera_source {
                t.source = SourceId::Camera;
                t.camera = Some("cam".into());
            }
            w.take = Some(t);
        }
        if d[1] == 1 {
            w.mac.os = Fact::known(shroom_core::world::OsSession { awake: false, unlocked: true, on_console: true }, Mono(0));
        }
        // A count: Start's (no take), or Restart's over a paused take; or one that just fired.
        if d[2] > 0 {
            w.session.arming = Some(Arming {
                started: Mono(99_000),
                deadline: Mono(102_000),
                source: if camera_source { SourceId::Camera } else { SourceId::Display(7) },
                restart_of: w.take.as_ref().map(|t| t.dir.clone()),
                fired: d[2] == 2,
            });
        }
        w.session.popover = [None, Some(Door::asked(Mono(99_500))), Some(Door::unasked(Mono(99_500)))][d[3]];
        w.session.confirm = match d[4] {
            0 => None,
            1 if matches!(w.take.as_ref().map(|t| t.phase), Some(Phase::Paused(_))) => Some(Confirm::Discard),
            2 if w.take.is_none() && w.session.arming.is_none() => {
                w.queue = vec![queued(None)];
                Some(Confirm::Quit)
            }
            // Restart's question (DEVIATIONS #44).
            3 if matches!(w.take.as_ref().map(|t| t.phase), Some(Phase::Paused(_))) => Some(Confirm::Restart),
            _ => return,
        };
        w.prefs.show_yourself = d[5] == 1;
        w.mac.camera = Fact::known([Access::Granted, Access::Denied][d[6]], Mono(0));
        w.prefs.source = Some(if camera_source { SourceId::Camera } else { SourceId::Display(7) });
        match d[8] {
            0 => {}
            // Opened before the take: it stays (law 4).
            1 => {
                w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
            }
            // Opened after the take began: it waits (law 4).
            _ => {
                w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(99_800)));
            }
        }
        f(w);
    });
}

fn queued(blocked: Option<shroom_core::world::Blocked>) -> shroom_core::world::Queued {
    shroom_core::world::Queued {
        dir: TakeDir(QUEUED_DIR.into()),
        rec: RecId("QrStUvWxYz012399".into()),
        title: "Untitled — Oct 1".into(),
        stopped: shroom_core::time::Wall(1_789_000_000_000),
        done: 2,
        total: 10,
        blocked,
        length_ms: Some(90_000),
    }
}

/// Projection 6 — no take: `config` (its one banner by N38's order, its status line, disk-low),
/// the stops' popovers and the saved-take questions, and the tray's state and face, on every
/// kind of Mac and standing (~0.5 M worlds).
fn idle_worlds(mut f: impl FnMut(World)) {
    use shroom_core::world::{Arming, Blocked, Confirm, SavedCause, SourceId, StopOutcome, Strike, UpdateOffer};
    let radices = [2, 7, 4, 3, 13, 2, 5, 3, 2, 2, 2];
    odometer(&radices, |d| {
        let mut w = popover_base();
        w.session.popover = (d[0] == 0).then(|| Door::asked(Mono(99_500)));
        w.saved = match d[2] {
            0 => vec![],
            c => {
                let mut s = saved([SavedCause::Interrupted, SavedCause::DiskFull, SavedCause::ScreenOff][c - 1], 272_000);
                s.dir = TakeDir(SAVED_DIR.into());
                vec![s]
            }
        };
        w.queue = match d[3] {
            0 => vec![],
            1 => vec![queued(None)],
            _ => vec![queued(Some(Blocked::Space))],
        };
        // The question or the stop the popover holds; only with its subject here.
        match d[1] {
            0 => {}
            1 if !w.saved.is_empty() => w.session.confirm = Some(Confirm::DiscardSaved(TakeDir(SAVED_DIR.into()))),
            2 if !w.queue.is_empty() => w.session.confirm = Some(Confirm::DiscardRow(TakeDir(QUEUED_DIR.into()))),
            3 if !w.queue.is_empty() => w.session.confirm = Some(Confirm::Quit),
            4 => w.session.last_stop = Some(StopOutcome::Offline(RecId("AbCdEfGhIjKlMnOp".into()))),
            5 if !w.queue.is_empty() => w.session.last_stop = Some(StopOutcome::Stranded(TakeDir(QUEUED_DIR.into()))),
            6 if w.saved.first().is_some_and(|s| s.cause != SavedCause::Interrupted) => w.session.last_stop = Some(StopOutcome::AutoStopped(TakeDir(SAVED_DIR.into()))),
            _ => return,
        }
        let strike = |gone| Some(Strike { gone, first: Mono(10_000), last: Mono(50_000) });
        let member = |w: &mut World, role| w.space.identity = Some(SpaceIdentity { role, member_id: "m_mher".into(), ..identity() });
        match d[4] {
            0 => {}
            1 => w.space.broken = Fact::known(true, Mono(60_000)),
            2 => w.mac.primed = None,
            3 => member(&mut w, Role::Member),
            4 => {
                member(&mut w, Role::Member);
                w.members.standing.revoked = true;
            }
            5 => {
                member(&mut w, Role::Member);
                w.members.standing.strike = strike(true);
            }
            6 => {
                member(&mut w, Role::Member);
                w.members.standing.strike = strike(false);
            }
            7 => {
                member(&mut w, Role::Member);
                w.members.standing.behind = true;
            }
            8 => {
                member(&mut w, Role::OwnerClass);
                w.members.standing.revoked = true;
            }
            9 => {
                member(&mut w, Role::OwnerClass);
                w.members.standing.strike = strike(false);
            }
            10 => {
                member(&mut w, Role::OwnerClass);
                w.members.standing.behind = true;
            }
            c => {
                w.space.identity = None;
                w.space.token = false;
                w.mac.primed = None;
                if c == 12 {
                    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
                }
            }
        }
        if matches!(w.space.role(), Some(Role::Member | Role::OwnerClass)) {
            w.space.token = false;
            w.space.device_key = true;
        }
        w.mac.screen = Fact::known(d[5] == 0, Mono(0));
        match d[6] {
            0 => {}
            1 | 2 => {
                w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: "1.4.0".into() }), Mono(1_000));
                w.update.staged = Some("1.4.0".into());
                w.update.applying = d[6] == 2;
            }
            3 => w.update.failed = true,
            _ => w.once.seen_version = Some("1.2.0".into()),
        }
        let per_minute = shroom_core::derive::bytes_per_minute(w.prefs.quality);
        w.mac.disk_free = Fact::known([200_000_000_000, shroom_core::derive::DISK_MARGIN + 10 * per_minute, shroom_core::derive::DISK_MARGIN][d[7]], Mono(0));
        if d[8] == 1 {
            w.popover_closed = Some(Mono(80_000));
        }
        w.net.online = Fact::known(d[9] == 0, Mono(0));
        if d[10] == 1 {
            w.session.arming = Some(Arming { started: Mono(99_000), deadline: Mono(102_000), source: SourceId::Display(1), restart_of: None, fired: false });
        }
        // The popover exists once `config` does: a space, and this Mac's chapter done (law 5,
        // bend 5 — before that the tray click is the onboarding window's door).
        if (w.session.popover.is_some() || w.session.arming.is_some()) && !(w.space.identity.is_some() && w.mac.primed.is_some()) {
            return;
        }
        f(w);
    });
}

/// Fixture states a popover or the tray may show together, by design.
const POPOVER_COEXIST: &[(&str, &str, &str)] = &[];

fn check_popover(walk: &mut Walk, w: World) {
    walk.worlds += 1;
    let scene = shroom_core::derive(&w);
    oracle(walk, &w, &scene);
    let ids = scene.state_ids();
    for id in &ids {
        if !known_id(id) {
            walk.fail(format!("no state: “{id}” isn't in the machine"));
        }
        walk.reached.insert(id);
    }
    if let Some(p) = &scene.popover {
        if let Some(b) = &p.banner {
            walk.reached.insert(b.id);
        }
        // Two faces: one popover state at a time (the status line's one-shots aside — they
        // are the status line's, not the popover's).
        let claim: Vec<&str> =
            std::iter::once(p.id).chain(p.faces.iter().copied()).filter(|id| spec().state(id).is_some_and(|s| s.surface == "popover" && matches!(s.kind.as_str(), "state" | "failure-state"))).collect();
        if claim.len() > 1 && !POPOVER_COEXIST.iter().any(|(a, b, _)| claim.contains(a) && claim.contains(b)) {
            walk.fail(format!("two faces: {claim:?} claim the popover"));
        }
    } else if w.session.popover.is_some() {
        // No state: the popover's door is open and nothing shows — only under the count's
        // scrim, mid-recording, while a take ends, or while the Mac is away (law 4, N39).
        let allowed = w.session.arming.as_ref().is_some_and(|a| !a.fired)
            || matches!(w.take.as_ref().map(|t| t.phase), Some(shroom_core::world::Phase::Starting | shroom_core::world::Phase::Recording | shroom_core::world::Phase::Pausing(_) | shroom_core::world::Phase::Resuming | shroom_core::world::Phase::Finalizing(_)))
            || w.mac.os.get().is_some_and(|o| !o.awake || !o.unlocked || !o.on_console);
        if !allowed {
            walk.fail(format!("no state: the popover's door is open and nothing shows ({:?})", w.take.as_ref().map(|t| t.phase)));
        }
    }
    // The tray and the popover tell one story: a red dot has no popover; pause bars only a
    // paused take's; the count closes it; the idle states never over a take.
    let face = scene.tray.face;
    let pid = scene.popover.as_ref().map(|p| p.id);
    if face == shroom_core::scene::Face::Red && pid.is_some() {
        walk.fail(format!("two faces: the red dot under the {pid:?} popover"));
    }
    if face == shroom_core::scene::Face::PauseBars && !matches!(pid, None | Some("paused" | "discard-confirm" | "restart-confirm")) {
        walk.fail(format!("two faces: pause bars under the {pid:?} popover"));
    }
    if scene.countdown.is_some() && pid.is_some() {
        walk.fail(format!("two faces: the countdown under the {pid:?} popover"));
    }
    if matches!(scene.tray.state, Some("idle-ready" | "idle-unprovisioned")) && w.take.is_some() {
        walk.fail(format!("two faces: {:?} over a take", scene.tray.state));
    }
    if scene.tray.state == Some("recording") && face != shroom_core::scene::Face::Red {
        walk.fail(format!("two faces: recording under the {face:?} face"));
    }
    let sig = format!("{:?}|{:?}|{:?}|{:?}|{}|{}|{:?}", scene.tray.state, face, pid, scene.popover.as_ref().map(|p| (&p.faces, p.banner.as_ref().map(|b| b.id), p.ret.clone())), scene.countdown.is_some(), scene.bubble.is_some(), scene.settings.is_some());
    walk.reps.entry(sig).or_insert(w);
}

/// Where the popover and the tray stand: the popover's state (a paused take's cause face
/// first, as the map draws them), else the count, else the take's beat, else the tray's state.
fn pop_standing(scene: &Scene, w: &World) -> &'static str {
    use shroom_core::world::Phase;
    if let Some(p) = &scene.popover {
        if p.id == "paused" {
            if let Some(f) = p.faces.first() {
                return f;
            }
        }
        return p.id;
    }
    if scene.countdown.is_some() {
        return "countdown";
    }
    match w.take.as_ref().map(|t| t.phase) {
        Some(Phase::Paused(_)) => "paused",
        Some(Phase::Finalizing(_)) => "stopping",
        _ => scene.tray.state.unwrap_or("closed"),
    }
}

fn pop_listed(from: &str, to: &str, ev: &Event) -> Option<String> {
    spec().listed(from, to, &event_kind(ev))
}

/// The events a person, the OS and the lanes can send the popover and the tray.
fn pop_alphabet(w: &World) -> Vec<Event> {
    use shroom_core::event::{CloudNews, OsNews, QuitVia, RecNews};
    use shroom_core::world::{Op, OsSession, Reach};
    let scene = shroom_core::derive(w);
    let mut ev: Vec<Event> = Vec::new();
    if let Some(p) = &scene.popover {
        ev.extend(p.buttons().iter().filter(|b| b.enabled).map(|b| Event::Act { surface: Surface::Popover, action: b.action.clone(), rev: 0 }));
    }
    let q = quick();
    ev.extend([
        Event::TrayClicked { right: false, quick: q.clone() },
        Event::TrayClicked { right: true, quick: q.clone() },
        Event::Key { surface: Surface::Popover, key: Key::Return },
        Event::Key { surface: Surface::Popover, key: Key::Esc },
        Event::PopoverDismissed,
        Event::QuitRequested(QuitVia::RightClick),
        Event::Os(OsNews::NetPath(false)),
        Event::Os(OsNews::NetPath(true)),
        Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }),
        Event::Os(OsNews::Grants { screen: true, mic: Access::Denied, camera: Access::Granted }),
        Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }),
        Event::Os(OsNews::Session(OsSession { awake: false, unlocked: true, on_console: true })),
        Event::Os(OsNews::Session(OsSession::default())),
        Event::Os(OsNews::Devices { displays: q.displays.clone(), mics: vec![], cameras: vec![] }),
    ]);
    let out = |op: Op| w.inflight.ops.contains_key(&op);
    if out(Op::RecStart) {
        ev.push(Event::Rec(RecNews::Started { dir: TakeDir(TAKE_DIR.into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    }
    if out(Op::RecPause) {
        ev.push(Event::Rec(RecNews::Paused { length_ms: 272_000 }));
    }
    if out(Op::RecResume) {
        ev.push(Event::Rec(RecNews::Resumed));
    }
    if out(Op::RecFinalize) {
        ev.push(Event::Rec(RecNews::Finalized(Ok(272_000))));
    }
    if w.take.as_ref().is_some_and(|t| t.phase == shroom_core::world::Phase::Recording) {
        ev.push(Event::Rec(RecNews::DiskFloor));
        ev.push(Event::Rec(RecNews::MicLost));
    }
    if out(Op::Reach) {
        ev.push(Event::Cloud(CloudNews::Reached(Reach::Shroom)));
        ev.push(Event::Cloud(CloudNews::Reached(Reach::Unreachable)));
    }
    for op in w.inflight.ops.keys() {
        ev.push(Event::Failed { op: *op, class: FailClass::Transport });
    }
    ev.push(Event::Tick);
    ev
}

fn pop_converse(walk: &Walk) -> (usize, Vec<String>) {
    let mut steps = 0;
    let mut unlisted: BTreeSet<String> = BTreeSet::new();
    for rep in walk.reps.values() {
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut r = Run::new(rep.clone());
            r.send(Event::Tick);
            (r.w.clone(), r.t, pop_standing(&r.scene, &r.w))
        }));
        let Ok((w, t, before)) = ran else {
            unlisted.insert("a face's settling Tick panicked".into());
            continue;
        };
        for ev in pop_alphabet(&w) {
            let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut r = Run::new(w.clone());
                r.t = t;
                if matches!(ev, Event::Tick) {
                    match r.scene.next_wake {
                        Some(next) => {
                            r.tick_to(next.0.max(r.t + 20));
                        }
                        None => {
                            r.send(Event::Tick);
                        }
                    }
                } else {
                    r.send(ev.clone());
                }
                pop_standing(&r.scene, &r.w)
            }));
            steps += 1;
            match ran {
                Ok(after) => {
                    if pop_listed(before, after, &ev).is_none() {
                        unlisted.insert(format!("{before} → {after} on {}", short(&ev)));
                    }
                }
                Err(e) => {
                    let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                    unlisted.insert(format!("{before} on {}: {}", short(&ev), msg.lines().take(3).collect::<Vec<_>>().join(" / ")));
                }
            }
        }
    }
    (steps, unlisted.into_iter().collect())
}

/// Every popover and tray state, face, banner and overlay of Regions 3, 5 and 6 a Mac reaches.
const POPOVER_SCOPE: &[&str] = &[
    "idle-unprovisioned", "idle-ready", "recording", "plain", "tray-red-dot", "tray-pause-bars", "tray-amber-dot", "tray-frost-dot", "tray-frost-dot",
    "config", "paused", "discard-confirm", "discard-confirm-saved-take", "restart-confirm", "quit-confirm", "stopped-offline", "stopped-stranded", "permission-lost", "disk-full-stop",
    "display-lost", "mic-lost", "camera-lost", "mac-away", "mic-permission-lost", "disk-low", "updated", "update-failed",
    "recovered-take", "space-degraded", "space-degraded-no-token-mac", "member-revoked", "space-needs-owner", "space-gone", "space-needs-update-member",
    "space-needs-update-no-token", "owner-mac-removed", "recording-off", "update-ready", "updating-app", "countdown", "bubble",
];

#[test]
fn popover_tray_enumeration_every_world_one_state_every_state_reached_every_move_listed() {
    let started = std::time::Instant::now();
    let mut walk = Walk::default();
    take_worlds(|w| check_popover(&mut walk, w));
    let takes = walk.worlds;
    idle_worlds(|w| check_popover(&mut walk, w));
    assert!(takes > 8_000 && walk.worlds > 250_000, "the walk covered only {takes} + {} worlds", walk.worlds - takes);
    assert!(walk.failures.is_empty(), "{} worlds:\n{}", walk.worlds, walk.failures.join("\n"));
    let unreached = in_scope(POPOVER_SCOPE, &walk);
    assert!(unreached.is_empty(), "no world reaches {unreached:?}");
    let (steps, unlisted) = pop_converse(&walk);
    assert!(steps > 1_000, "the converse ran only {steps} steps");
    assert!(unlisted.is_empty(), "{} faces × the alphabet = {steps} steps; moves the spec doesn't list:\n{}", walk.reps.len(), unlisted.join("\n"));
    eprintln!("popover/tray: {takes} take worlds + {} idle worlds, {} faces, {steps} converse steps, {:?}", walk.worlds - takes, walk.reps.len(), started.elapsed());
}
