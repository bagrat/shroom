//! Regions 1–2 and Region 3's permissions chapter — the welcome, the wizard's plan, and row
//! six — against the fixtures: every state derives from a world built with the spec's
//! example values, with the fixture's exact Return default and its lines verbatim; the
//! design's walkthroughs with the laws checked at every step; and the machine's drawn edges, each
//! realized by one event or listed out of scope with its reason.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::collections::{BTreeSet, HashMap};

use shroom_core::effect::{CloudCmd, Effect, PersistOp, Perm, PublicUrl, RecCmd, ShellCmd, SweepPart};
use shroom_core::event::{ClaimFail, CloudNews, Disk, Event, Field, Key, OsNews, RecNews, SecretField, Shape, Surface, SweepAnswer, TokenFail};
use shroom_core::scene::{Action, Glyph, Nest, OnboardingBody, Scene, SettingsPane};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{derive, update};
use shroom_spec::Spec;
use support::*;

// ── builders, with the spec's example values ──

/// A Mac that has never been asked anything: no grants, nothing provisioned.
fn fresh_quick() -> QuickProbe {
    QuickProbe { screen: false, mic: Access::NotAsked, camera: Access::NotAsked, ..quick() }
}

fn bagrat() -> Account {
    Account { id: "bagrat".into(), name: "Bagrat's Account".into() }
}

fn acme() -> Account {
    Account { id: "acme".into(), name: "Acme Engineering".into() }
}

fn space(sub: &str, label: &str) -> SpaceIdentity {
    SpaceIdentity { subdomain: sub.into(), label: label.into(), host: format!("{label}.{sub}.workers.dev"), bucket: "shroom-space".into(), ..identity() }
}

/// A fresh Mac, every local fact known, no window open.
fn unprovisioned() -> World {
    let mut w = ready();
    w.space.identity = None;
    w.space.token = false;
    w.mac.primed = None;
    w.mac.screen = Fact::known(false, Mono(0));
    w.mac.mic = Fact::known(Access::NotAsked, Mono(0));
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w
}

fn window(mut w: World) -> World {
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    w
}

fn plan() -> World {
    let mut w = window(unprovisioned());
    w.prefs.setup_chosen = true;
    w
}

fn affirmed() -> World {
    let mut w = plan();
    w.session.fork = Fork::Affirmed;
    w
}

/// A token saved for Bagrat's account, re-proved in this process.
fn token_saved() -> World {
    let mut w = affirmed();
    w.space.token = true;
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: bagrat(), several: false }, Mono(950));
    w
}

fn gated(email: bool, card: bool) -> World {
    let mut w = token_saved();
    w.sweep.gates = Fact::known(Gates { email, card }, Mono(950));
    w.sweep.subdomain = Fact::known(None, Mono(950));
    w.sweep.foreign_v = Fact::known(false, Mono(950));
    w.sweep.label = Fact::known(Some("v".into()), Mono(950));
    w.sweep.pass = Some(Mono(950));
    w
}

fn at_name() -> World {
    gated(false, false)
}

/// The name row ticked: the permissions chapter is the step, wiring underneath (N45).
fn named() -> World {
    let mut w = gated(false, false);
    w.sweep.subdomain = Fact::known(Some("myshroom".into()), Mono(950));
    w.sweep.found = Fact::known(None, Mono(950));
    w
}

/// The last step since N45: this Mac's permissions done, wiring not yet.
fn at_wiring() -> World {
    let mut w = named();
    w.mac.primed = Some(Wall(1_790_000_000_000));
    w.mac.screen = Fact::known(true, Mono(0));
    w.mac.mic = Fact::known(Access::Granted, Mono(0));
    w.mac.camera = Fact::known(Access::Granted, Mono(0));
    w
}

/// Provisioned; this Mac's chapter not done.
fn at_done() -> World {
    let mut w = window(unprovisioned());
    w.space.identity = Some(identity());
    w.space.token = true;
    w
}

fn chapter(mic: Access, screen: bool) -> World {
    let mut w = at_done();
    w.session.chapter = true;
    // Allow recording was clicked: a mic not yet answered is the prompt it put to macOS.
    w.mac.mic_asked = mic == Access::NotAsked;
    w.mac.mic = Fact::known(mic, Mono(0));
    w.mac.screen = Fact::known(screen, Mono(0));
    w
}

fn all_set(mic: Access) -> World {
    let mut w = window(ready());
    w.mac.mic = Fact::known(mic, Mono(0));
    w
}

fn disk(launch: LaunchKind, w: &World, quick: QuickProbe, restore: Option<Restore>) -> Disk {
    Disk {
        launch,
        version: "1.3.0".into(),
        prefs: w.prefs.clone(),
        once: w.once.clone(),
        space: w.space.identity.clone(),
        token: w.space.token,
        device_key: w.space.device_key,
        primed: w.mac.primed,
        restore,
        quick,
    }
}

/// Kill and relaunch: a new process boots from what's on disk, fed fresh probe answers.
fn reboot(r: &Run, launch: LaunchKind, quick: QuickProbe, restore: Option<Restore>) -> Run {
    let d = disk(launch, &r.w, quick, restore);
    let mut n = Run::new(World::blank());
    n.t = r.t + 1_000;
    n.send(Event::Booted(Box::new(d)));
    n
}

// ── reading the Scene ──

fn win(s: &Scene) -> &'static str {
    s.onboarding.as_ref().map(|o| o.id).unwrap_or("-")
}

fn nest(s: &Scene) -> &Nest {
    match &s.onboarding.as_ref().expect("the onboarding window").body {
        OnboardingBody::Plan(p) => &p.nest,
        OnboardingBody::Welcome(_) => panic!("the welcome, not the plan"),
        OnboardingBody::Invite(_) => panic!("the invite window, not the plan"),
    }
}

fn row(s: &Scene, n: usize) -> String {
    match &s.onboarding.as_ref().unwrap().body {
        OnboardingBody::Plan(p) => p.rows[n - 1].line.clone(),
        OnboardingBody::Welcome(_) => panic!("no rows on the welcome"),
        OnboardingBody::Invite(_) => panic!("no rows on the invite window"),
    }
}

fn json(s: &Scene) -> String {
    serde_json::to_string(s).unwrap()
}

trait Drive {
    fn on(&mut self, action: Action) -> &mut Self;
    fn cloud(&mut self, news: CloudNews) -> &mut Self;
    fn paste(&mut self, field: SecretField, id: u32, shape: Shape) -> &mut Self;
    fn typed(&mut self, value: &str) -> &mut Self;
}

impl Drive for Run {
    fn on(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Onboarding, action, rev: 0 })
    }
    fn cloud(&mut self, news: CloudNews) -> &mut Self {
        self.send(Event::Cloud(news))
    }
    fn paste(&mut self, field: SecretField, id: u32, shape: Shape) -> &mut Self {
        self.send(Event::SecretPasted { field, id: SecretId(id), shape })
    }
    fn typed(&mut self, value: &str) -> &mut Self {
        self.send(Event::Field { field: Field::Name, value: value.into() })
    }
}

fn swept(gates: Gates, subdomain: Option<&str>, foreign_v: bool, found: Option<u32>) -> CloudNews {
    CloudNews::Swept(SweepAnswer {
        token: Some(TokenVerdict::Valid { account: bagrat(), several: false }),
        gates: Some(gates),
        subdomain: Some(subdomain.map(String::from)),
        foreign_v: Some(foreign_v),
        label: Some(Some(if foreign_v { "shroom" } else { "v" }.into())),
        bucket: Some(found.is_some()),
        worker: Some(if found.is_some() { WorkerFact::Ours } else { WorkerFact::Absent }),
        found: Some(found),
    })
}

const CLEAR: Gates = Gates { email: false, card: false };

/// The card step's check with its part `part` underway (N74), since Mono(950).
fn checking(mut w: World, part: u8) -> World {
    w.sweep.check = Some(Progress { part, since: Mono(950) });
    w
}

/// Wiring with its part `part` underway since `since`, read at `now` (N74).
fn wiring_at(mut w: World, part: u8, since: u64, now: u64) -> World {
    w.sweep.wiring = Some(Progress { part, since: Mono(since) });
    w.now = Mono(now);
    w
}

/// The parts a nest lists (N74): each one's mark, line and clock.
fn parts(s: &Scene) -> Vec<(Glyph, String, Option<String>)> {
    let list = match nest(s) {
        Nest::Check { parts } => parts.clone(),
        Nest::Calm { parts, .. } => parts.clone(),
        Nest::Gate { watch, .. } => watch.iter().cloned().collect(),
        other => panic!("no parts in {other:?}"),
    };
    list.into_iter().map(|p| (p.glyph, p.line, p.clock)).collect()
}

fn long(s: &Scene) -> Option<String> {
    match nest(s) {
        Nest::Calm { long, .. } | Nest::Gate { long, .. } => long.clone(),
        other => panic!("no long line in {other:?}"),
    }
}

fn clock(s: &Scene, n: usize) -> Option<String> {
    match &s.onboarding.as_ref().unwrap().body {
        OnboardingBody::Plan(p) => p.rows[n - 1].clock.clone(),
        _ => panic!("no rows"),
    }
}

// ── the inventory: one row (or more) per in-scope state ──

fn rows() -> Vec<(&'static str, World)> {
    let mut rows: Vec<(&'static str, World)> = Vec::new();
    rows.push(("idle-unprovisioned", unprovisioned()));
    rows.push(("welcome", window(unprovisioned())));
    // Uninstall shroom… asked over the welcome (B160).
    rows.push(("welcome", {
        let mut w = window(unprovisioned());
        w.session.confirm = Some(Confirm::Uninstall { going: false });
        w
    }));
    rows.push(("first-launch", {
        let mut w = window(unprovisioned());
        w.launch.kind = LaunchKind::Install { join: false };
        w.launch.booted = Mono(900);
        w
    }));
    rows.push(("account", plan()));
    rows.push(("account-signing-up", {
        let mut w = plan();
        w.session.fork = Fork::SigningUp;
        w
    }));
    rows.push(("token", affirmed()));
    // Reached by I signed up: the email comes first (N45).
    rows.push(("token", {
        let mut w = affirmed();
        w.session.fields.signed_up = true;
        w
    }));
    rows.push(("token-garbled", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::Garbled);
        w
    }));
    rows.push(("token-wrong-powers", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::WrongPowers);
        w
    }));
    rows.push(("token-pick-account", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::PickAccount { id: SecretId(7), accounts: vec![bagrat(), acme()] });
        w
    }));
    rows.push(("card", token_saved()));
    // N74: the check's parts as they answer — the first underway, then the last.
    rows.push(("card", checking(token_saved(), 1)));
    rows.push(("card", checking(token_saved(), 4)));
    rows.push(("gate-email", gated(true, true)));
    rows.push(("gate-card", gated(false, true)));
    // Add credit card clicked: the page is open, shroom watches (N45).
    rows.push(("gate-card", {
        let mut w = gated(false, true);
        w.session.fields.billing_opened = true;
        w
    }));
    // N74: the watch with its clock, and from 45 s its quiet line.
    rows.push(("gate-card", {
        let mut w = gated(false, true);
        w.session.fields.billing_opened = true;
        w.session.fields.billing_at = Some(Mono(1_000));
        w.now = Mono(46_000);
        w
    }));
    rows.push(("name", at_name()));
    rows.push(("name", {
        let mut w = at_name();
        w.session.fields.name_hint = Some(Mono(990));
        w
    }));
    rows.push(("name-taken", {
        let mut w = at_name();
        w.session.fields.name = "myshroom".into();
        w.sweep.taken = Some("myshroom".into());
        w
    }));
    rows.push(("name-v-clash", {
        let mut w = at_name();
        w.sweep.foreign_v = Fact::known(true, Mono(950));
        w.sweep.label = Fact::known(Some("shroom".into()), Mono(950));
        w
    }));
    rows.push(("wiring", at_wiring()));
    // N74: wiring's parts — the first underway, the name past 3 min, the last.
    rows.push(("wiring", wiring_at(at_wiring(), 1, 1_000, 1_000)));
    rows.push(("wiring", wiring_at(at_wiring(), 3, 1_000, 200_000)));
    rows.push(("wiring", wiring_at(at_wiring(), 5, 1_000, 1_000)));
    rows.push(("wizard-offline", {
        let mut w = at_wiring();
        w.net.online = Fact::known(false, Mono(0));
        w
    }));
    // DEVIATIONS #25: wiring failed in a way nothing else classifies.
    rows.push(("wizard-unexpected", {
        let mut r = Run::new(at_wiring());
        r.send(Event::Failed { op: Op::Wire, class: FailClass::Unclassified });
        r.w
    }));
    rows.push(("space-found", {
        let mut w = at_wiring();
        w.sweep.found = Fact::known(Some(14), Mono(950));
        w
    }));
    rows.push(("done", at_done()));
    // The name ticked, wiring running underneath the chapter (N45).
    rows.push(("done", named()));
    // N74: the working row names the part underway.
    for part in 1..=5 {
        rows.push(("done", wiring_at(named(), part, 1_000, 2_000)));
    }
    rows.push(("perm-mic", chapter(Access::NotAsked, false)));
    rows.push(("perm-camera", {
        let mut w = chapter(Access::Granted, false);
        w.mac.camera_asked = true;
        w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
        w
    }));
    rows.push(("perm-screen", chapter(Access::Granted, false)));
    // N43: while macOS's own alert leads.
    rows.push(("perm-screen", {
        let mut w = chapter(Access::Granted, false);
        w.mac.screen_alert = Some(true);
        w
    }));
    rows.push(("relaunching", {
        let mut w = chapter(Access::Granted, false);
        w.launch.ending = true;
        w
    }));
    rows.push(("perm-priming", chapter(Access::Granted, true)));
    rows.push(("all-set", all_set(Access::Granted)));
    rows.push(("all-set", all_set(Access::Denied)));
    // A space whose chapter isn't done here, the window closed: tray-only life (bend 5
    // changes only where its click goes).
    rows.push(("idle-ready", {
        let mut w = at_done();
        w.session.windows.clear();
        w
    }));
    rows
}

fn action_label(a: &Action) -> &'static str {
    match a {
        Action::SetUp => "Set up my space",
        Action::HaveOne => "I have one",
        Action::SignedUp => "I signed up",
        Action::CreateToken => "Create token in Cloudflare",
        Action::UseAccount => "Use this account",
        Action::AddCard => "Add credit card",
        Action::Claim => "Claim this name",
        Action::AllowRecording => "Allow recording",
        Action::RecordNow => "Record now",
        _ => "?",
    }
}

#[test]
fn every_onboarding_state_derives_with_its_return_default_and_its_lines() {
    let spec = Spec::load();
    let by_id = spec.by_id();
    let mut failures = Vec::new();
    let mut seen: HashMap<&str, String> = HashMap::new();
    for (id, w) in rows() {
        let scene = derive(&w);
        let ids = scene.state_ids();
        // Law 15's lint over every string the row's Scene shows (TESTING-PLAN §2.4).
        failures.extend(laws::voice_failures(&scene, id));
        if !ids.contains(&id) {
            failures.push(format!("{id}: not derived (got {ids:?})"));
            continue;
        }
        let fixture = by_id[id][0];
        // The Return default, exactly as the fixture gives it, whatever the enabled flags.
        // A face is its parent's render (N18); first-launch is the welcome it opens.
        if fixture.surface == "window" && fixture.kind != "face" && id != "first-launch" {
            // Under the Uninstall sheet (B160): no Return default, Esc ≡ Cancel (N23).
            let sheet = id == "welcome" && w.session.confirm.is_some();
            let want = if sheet { None } else { fixture.return_default().map(|a| a.label.as_str()) };
            let got = scene.onboarding.as_ref().and_then(|o| o.ret.as_ref()).map(action_label);
            if want != got {
                failures.push(format!("{id}: Return is {got:?}, the fixture says {want:?}"));
            }
            let esc = if sheet { Some(Action::Back) } else { None };
            assert_eq!(scene.onboarding.as_ref().unwrap().esc, esc, "{id}: the window has no Esc meaning but a sheet's");
        }
        seen.entry(id).or_default().push_str(&json(&scene));
    }
    for (id, haystack) in &seen {
        let fixture = by_id[id][0];
        // perm-mic's and perm-camera's lines are macOS's own prompts (Info.plist, checked
        // by tests/copy.rs).
        if *id == "perm-mic" || *id == "perm-camera" {
            continue;
        }
        for l in &fixture.said_lines {
            // The repair mood's re-wire line is Settings · Space's (region5.rs), since N45.
            if *id == "wiring" && l.text.starts_with("Setting up your space…") {
                continue;
            }
            let varies = l.note.as_deref().is_some_and(|n| n.contains("example") || n.contains("varies"));
            let encoded = serde_json::to_string(&l.text).unwrap();
            if !varies && !haystack.contains(&encoded[1..encoded.len() - 1]) {
                failures.push(format!("{id}: {} line missing: {:?}", l.copy_status, l.text));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn the_plan_marks_its_rows_from_the_facts() {
    // Six rows: → current · ✓ done · • ahead; parentheticals drop off rows behind you.
    let s = derive(&plan());
    let OnboardingBody::Plan(p) = &s.onboarding.as_ref().unwrap().body else { panic!() };
    assert_eq!(p.rows.iter().map(|r| r.glyph).collect::<Vec<_>>(), [Glyph::Current, Glyph::Ahead, Glyph::Ahead, Glyph::Ahead, Glyph::Ahead, Glyph::Ahead]);
    assert_eq!(p.current, Some(1));

    let s = derive(&at_name());
    assert_eq!(row(&s, 1), "✓ A Cloudflare account");
    assert_eq!(row(&s, 3), "✓ A credit card for storage");
    assert_eq!(row(&s, 4), "→ Your link name (part of every link you share)", "the name row sheds only its example");

    let s = derive(&at_wiring());
    assert_eq!(row(&s, 4), "✓ Your link name — v.myshroom.workers.dev", "the plan's only outcome-bearing tick");

    let s = derive(&all_set(Access::Granted));
    assert_eq!(row(&s, 5), "✓ Letting macOS allow recording");
    let OnboardingBody::Plan(p) = &s.onboarding.as_ref().unwrap().body else { panic!() };
    assert!(p.rows.iter().all(|r| r.glyph == Glyph::Done));
    assert_eq!(p.current, None);

    // Gates are probed, never assumed: unknown gates leave the card row current.
    assert_eq!(win(&derive(&token_saved())), "card");
    // A stored token not yet re-proved in this process: its row stays ticked and the
    // re-probe's spinner sits on the card row — never a blank plan, and never `token`
    // without its controls (`token` is for a re-probe that finds no valid token).
    let mut w = affirmed();
    w.space.token = true;
    let s = derive(&w);
    assert_eq!(win(&s), "card");
    assert!(matches!(nest(&s), Nest::Check { .. }), "N74: the re-probe shows the check's parts");
    assert_eq!(row(&s, 1), "✓ A Cloudflare account", "a stored token re-proves box 1");
    assert_eq!(row(&s, 2), "✓ One access token for shroom");
    w.sweep.token = Fact::known(TokenVerdict::Revoked, Mono(950));
    let s = derive(&w);
    assert_eq!(win(&s), "token", "the re-probe found it dead: the row un-ticks and becomes current");
    assert_eq!(s.onboarding.as_ref().unwrap().ret, Some(Action::CreateToken));
}

// ── walkthroughs ──

#[test]
fn fresh_install_through_the_wizard_and_the_chapter_to_idle_ready() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Install { join: false }, &World::blank(), fresh_quick(), None))));
    assert_eq!(win(&r.scene), "welcome");
    assert!(r.ids().contains(&"first-launch"));
    assert!(r.scene.dock, "a normal macOS app on day one: dock icon + the window");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::SetUp));

    r.on(Action::SetUp);
    assert_eq!(win(&r.scene), "account", "the same window morphs");
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(true)))), "the login box was checked");
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.setup_chosen)));

    r.on(Action::NotYet);
    assert_eq!(win(&r.scene), "account-signing-up");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Signup))));
    r.on(Action::SignupAgain);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Signup))), "the tab reopens, no transition");
    assert_eq!(win(&r.scene), "account-signing-up");
    r.key(Surface::Onboarding, Key::Return);
    assert_eq!(win(&r.scene), "token", "I signed up — box 1 ticks at proceed");
    assert!(r.effects.is_empty(), "the token step never auto-opens the browser");

    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::TokenPage))), "Create token in Cloudflare");

    r.paste(SecretField::Token, 7, Shape::Garbled);
    assert_eq!(win(&r.scene), "token-garbled");
    assert!(r.effects.is_empty(), "the shape check runs before any network call");

    r.paste(SecretField::Token, 8, Shape::Token);
    assert_eq!(win(&r.scene), "token");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Validate(SecretId(8)))));
    r.cloud(CloudNews::Validated { id: SecretId(8), result: Ok(vec![bagrat()]) });
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SaveToken(SecretId(8), "bagrat".into()))), "the token is saved before any wiring");
    r.cloud(CloudNews::TokenSaved(SecretId(8)));
    assert_eq!(win(&r.scene), "card");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "the probe pass runs at paste-validate");

    r.cloud(swept(Gates { email: true, card: true }, None, false, None));
    assert_eq!(win(&r.scene), "gate-email", "email first, then card");
    let poll = r.scene.next_wake.expect("the gate polls");
    r.tick_to(poll.0);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates))), "the auto-poll");
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(Gates { email: false, card: true }), ..Default::default() }));
    assert_eq!(win(&r.scene), "gate-card");
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Billing { account_id: "bagrat".into() }))));
    let poll = r.scene.next_wake.unwrap();
    r.tick_to(poll.0);
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }));
    assert_eq!(win(&r.scene), "card", "both clear — back on the card step while the rest is probed");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r.cloud(swept(CLEAR, None, false, None));
    assert_eq!(win(&r.scene), "name", "no workers.dev name yet (10007)");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::Claim));

    r.key(Surface::Onboarding, Key::Return);
    assert!(r.effects.is_empty(), "Claim is disabled until the charset check passes");
    r.typed("MyShroom!");
    let Nest::Name { value, hint, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(value, "myshroom", "uppercase lowercases silently; the ! never lands");
    assert_eq!(hint.as_deref(), Some("Lowercase letters, numbers, and dashes."));
    let fade = r.scene.next_wake.unwrap();
    r.tick_to(fade.0);
    let Nest::Name { hint, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(*hint, None, "the quiet line fades");

    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Claim("myshroom".into()))), "Return in the field is Claim");
    r.cloud(CloudNews::Claimed { name: "myshroom".into(), result: Err(ClaimFail::Taken) });
    assert_eq!(win(&r.scene), "name-taken");
    let Nest::Name { value, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(value, "myshroom", "the value stays put for editing");
    r.typed("myshroomhq");
    assert_eq!(win(&r.scene), "name", "inline retry");
    r.on(Action::Claim);
    r.cloud(CloudNews::Claimed { name: "myshroomhq".into(), result: Ok(()) });
    // N45: the permissions follow the name at once; wiring starts underneath.
    assert_eq!(win(&r.scene), "done");
    assert_eq!(row(&r.scene, 4), "✓ Your link name — v.myshroomhq.workers.dev");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into() })), "wiring starts out of sight");
    let deadline = r.w.inflight.ops[&Op::Wire];
    assert_eq!(deadline.0 - r.t, 180_000, "wiring gets three minutes (DEVIATIONS #8)");
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::RequestPermission(_)))), "nothing auto-fires at done");
    // The permissions are drawn fifth, the wiring row last — working.
    assert_eq!(row(&r.scene, 5), "→ Letting macOS allow recording (microphone, camera and screen — a one-time yes each)");
    assert_eq!(row(&r.scene, 6), "• Wiring it up (shroom is doing this part now)");
    let OnboardingBody::Plan(p) = &r.scene.onboarding.as_ref().unwrap().body else { panic!() };
    assert_eq!((p.current, p.rows[5].glyph), (Some(5), Glyph::Working), "the nest hangs under the fifth row drawn");

    // Unanswered under the chapter: never the offline face, it just tries again.
    r.send(Event::Failed { op: Op::Wire, class: FailClass::Transport });
    assert_eq!(win(&r.scene), "done", "the chapter isn't the network's");

    r.key(Surface::Onboarding, Key::Return);
    assert_eq!(win(&r.scene), "perm-mic");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Mic))), "the mic first, one prompt at a time");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert_eq!(win(&r.scene), "perm-camera", "then the camera (N43)");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Camera))));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert_eq!(win(&r.scene), "perm-screen");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))), "the chained screen step");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.effects.is_empty(), "the screen step is asked once, never on every grants report");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None, "perm-screen has no Return default");

    r.on(Action::ReopenShroom);
    assert_eq!(win(&r.scene), "relaunching");
    let Some(restore) = r.effects.iter().find_map(|e| match e {
        Effect::Persist(PersistOp::Restore(Some(x))) => Some(x.clone()),
        _ => None,
    }) else {
        panic!("restore.json is written on the way out")
    };
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::Relaunch(RelaunchWhy::Permission))));

    // The new process: the screen grant took effect; the chapter resumes from derived state.
    let mut r = reboot(&r, LaunchKind::PermissionRelaunch, quick(), Some(restore));
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Restore(None))), "restore.json is read once, then deleted");
    // Wiring wasn't done: no space on disk yet, so the plan re-probes from the token on —
    // under the chapter, never back on the card step (CHECKPOINT §10, 0.1.15).
    assert_eq!(win(&r.scene), "perm-priming");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Prime)), "the warm-up, the moment the grant is live");
    r.cloud(swept(CLEAR, Some("myshroomhq"), false, None));
    assert_eq!(win(&r.scene), "perm-priming");
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "and wiring picks up underneath");
    r.send(Event::Rec(RecNews::Primed(true)));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Primed(_)))));
    // The permissions done before wiring: wiring is the last step (N45).
    assert_eq!(win(&r.scene), "wiring");
    let Nest::Calm { line } = nest(&r.scene) else { panic!() };
    assert_eq!(line, "Almost there — switching your name on takes a few minutes the first time, and only once.");
    assert_eq!(row(&r.scene, 5), "✓ Letting macOS allow recording");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None, "wiring has zero affordances");

    // Offline at the last step: the spinner rests, one amber line, no retry button.
    r.send(Event::Failed { op: Op::Wire, class: FailClass::Transport });
    r.send(Event::Os(OsNews::NetPath(false)));
    assert_eq!(win(&r.scene), "wizard-offline");
    assert!(r.scene.onboarding.as_ref().unwrap().buttons().is_empty());
    r.send(Event::Os(OsNews::NetPath(true)));
    assert_eq!(win(&r.scene), "wiring", "it resumes by itself on reconnect");
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));

    r.cloud(CloudNews::Wired(Ok(space("myshroomhq", "v"))));
    assert_eq!(win(&r.scene), "all-set");
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Space(_)))));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if !p.setup_chosen)), "provisioning clears setup_chosen");
    assert_eq!(row(&r.scene, 6), "✓ Wiring it up");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::RecordNow));

    r.on(Action::CloseWindow);
    assert!(r.scene.onboarding.is_none());
    assert!(!r.scene.dock, "the window closes, the dock icon goes away");
    assert!(r.ids().contains(&"idle-ready"));
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"), "tray-only life: the click opens config");
}

/// The link name counts its edits, like every typed field (DECISIONS B107; region5.rs has
/// the Library's): an edit the guard changed — a character refused, uppercase lowercased —
/// is still one edit taken in, so the window can tell that the answer covers what it shows
/// and take the refused character back out; off the name step an edit is dropped, uncounted.
#[test]
fn the_link_name_counts_its_edits() {
    fn name(r: &Run) -> (String, u64) {
        let v = serde_json::to_value(nest(&r.scene)).unwrap();
        assert_eq!(v["kind"], "name", "the name step");
        (v["value"].as_str().expect("the field's value").to_string(), v["edits"].as_u64().expect("the field's edits, counted"))
    }
    let mut r = Run::new(at_name());
    assert_eq!(name(&r), (String::new(), 0));
    r.typed("m");
    r.typed("my");
    assert_eq!(name(&r), ("my".into(), 2), "one for every edit taken in");
    r.typed("my!");
    assert_eq!(name(&r), ("my".into(), 3), "a refused character: the value as it was, the edit counted");
    r.typed("myS");
    assert_eq!(name(&r), ("mys".into(), 4), "lowercased, one edit");
    r.typed("myshroom");
    r.on(Action::Claim);
    r.cloud(CloudNews::Claimed { name: "myshroom".into(), result: Err(ClaimFail::Taken) });
    assert_eq!(win(&r.scene), "name-taken");
    assert_eq!(name(&r), ("myshroom".into(), 5), "the taken name stays for editing, its count with it");
    r.typed("myshroomhq");
    assert_eq!(name(&r), ("myshroomhq".into(), 6));
    r.on(Action::Claim);
    r.cloud(CloudNews::Claimed { name: "myshroomhq".into(), result: Ok(()) });
    assert_eq!(win(&r.scene), "done", "N45: the permissions come before the wiring");
    r.typed("x");
    assert_eq!(r.arm.name, "dropped", "the name field isn't showing");
}

#[test]
fn a_returning_paste_finds_the_space_already_there() {
    // A Cloudflare token pasted anywhere in the welcome (no door shows, B159): the wizard
    // opens at `token`, the paste landed. Anything else pasted there is nothing.
    let mut r = Run::new(window(unprovisioned()));
    assert!(!json(&r.scene).contains("Have an invite link?"), "no invite line (B159)");
    r.paste(SecretField::Invite, 3, Shape::Other);
    assert_eq!(win(&r.scene), "welcome");
    assert!(r.effects.is_empty());

    r.paste(SecretField::Invite, 4, Shape::Token);
    assert_eq!(win(&r.scene), "token");
    assert_eq!(row(&r.scene, 1), "✓ A Cloudflare account", "a token proves the account");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Validate(SecretId(4)))));
    r.cloud(CloudNews::Validated { id: SecretId(4), result: Ok(vec![bagrat()]) });
    r.cloud(CloudNews::TokenSaved(SecretId(4)));
    r.cloud(swept(CLEAR, Some("myshroom"), false, Some(14)));
    assert_eq!(win(&r.scene), "space-found");
    let Nest::Found { line } = nest(&r.scene) else { panic!() };
    assert_eq!(line, "Found your space — v.myshroom.workers.dev, 14 videos. This Mac is joined.");
    for n in 3..=4 {
        assert!(row(&r.scene, n).starts_with('✓'), "rows three to five tick in order");
    }
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "this Mac is joined (idempotent calls)");
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert_eq!(win(&r.scene), "done", "row six becomes current — permissions are per Mac");
}

#[test]
fn a_foreign_v_gives_the_space_the_label_shroom() {
    let mut r = Run::new(token_saved());
    r.send(Event::Tick);
    r.cloud(swept(CLEAR, None, true, None));
    assert_eq!(win(&r.scene), "name");
    assert!(r.ids().contains(&"name-v-clash"));
    let Nest::Name { prefix, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(prefix, "https://shroom.", "the URL field shows the right label from the start");
    r.typed("myname").on(Action::Claim);
    r.cloud(CloudNews::Claimed { name: "myname".into(), result: Ok(()) });
    assert_eq!(row(&r.scene, 4), "✓ Your link name — shroom.myname.workers.dev (“v” was already taken in your Cloudflare)");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "shroom".into() })), "wiring never touches their v");
}

/// What the name row says in the wizard's plan and in Settings · Space's.
fn name_rows(w: &World) -> (String, String) {
    let mut w = w.clone();
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    let s = derive(&w);
    let SettingsPane::Space(space) = &s.settings.as_ref().expect("Settings").body.pane else { panic!("not Settings · Space") };
    (row(&s, 4), space.plan.rows[3].line.clone())
}

/// The name row says “v” was taken only where the survey **saw** someone else's `v`
/// (N42's own words: the account “already holds a Worker named `v` that isn't shroom's”).
/// A space under `shroom` beside a `v` nobody looked at — the checkpoint passes `v` by,
/// whatever is or isn't there — says its address and nothing more: at the name step, while
/// it wires, once wired, after a relaunch, and in Settings · Space.
#[test]
fn only_a_v_seen_to_be_someone_elses_is_called_taken() {
    const PLAIN: &str = "✓ Your link name — shroom.myshroom.workers.dev";
    const TAKEN: &str = "✓ Your link name — shroom.myshroom.workers.dev (“v” was already taken in your Cloudflare)";
    for (seen, line) in [(false, PLAIN), (true, TAKEN)] {
        let mut r = Run::new(token_saved());
        r.send(Event::Tick);
        r.cloud(CloudNews::Swept(SweepAnswer { foreign_v: Some(seen), label: Some(Some("shroom".into())), ..sweep_answer(None) }));
        assert_eq!(win(&r.scene), "name");
        assert_eq!(r.ids().contains(&"name-v-clash"), seen, "seen {seen}: at the name step");
        let Nest::Name { prefix, .. } = nest(&r.scene) else { panic!() };
        assert_eq!(prefix, "https://shroom.", "the label is the survey's either way");
        r.typed("myshroom").on(Action::Claim);
        r.cloud(CloudNews::Claimed { name: "myshroom".into(), result: Ok(()) });
        assert_eq!(win(&r.scene), "done", "N45: the permissions come before the wiring");
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "shroom".into() })), "a v passed by is never wired either");
        assert_eq!(row(&r.scene, 4), line, "seen {seen}: while it wires");

        r.cloud(CloudNews::Wired(Ok(space("myshroom", "shroom"))));
        assert_eq!(win(&r.scene), "done");
        assert_eq!(row(&r.scene, 4), line, "seen {seen}: once wired");
        assert_eq!(r.ids().contains(&"name-v-clash"), seen, "seen {seen}: once wired");

        // What was seen is the space's, for good: a new process reads it from the file the
        // space is kept in, with no survey to ask.
        let kept = r.effects.iter().find_map(|e| if let Effect::Persist(PersistOp::Space(id)) = e { Some(id.clone()) } else { None }).expect("the space is kept");
        let on_disk: SpaceIdentity = serde_json::from_str(&serde_json::to_string(&kept).unwrap()).unwrap();
        r.on(Action::AllowRecording);
        r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
        r.on(Action::ReopenShroom);
        let mut d = disk(LaunchKind::PermissionRelaunch, &r.w, quick(), None);
        d.space = Some(on_disk);
        let mut n = Run::new(World::blank());
        n.t = r.t + 1_000;
        n.send(Event::Booted(Box::new(d)));
        n.send(Event::Rec(RecNews::Primed(true)));
        assert_eq!(win(&n.scene), "all-set");
        assert_eq!(n.ids().contains(&"name-v-clash"), seen, "seen {seen}: after a relaunch");
        assert_eq!(name_rows(&n.w), (line.to_string(), line.to_string()), "seen {seen}: after a relaunch, in the wizard and in Settings · Space");
    }
}

/// A rewire in the repair mood (the page or its storage set up again, under the space's own
/// label) keeps what the setup saw at `v`: that pass never looks there, whatever it carries.
#[test]
fn a_rewire_keeps_what_the_setup_saw_at_v() {
    for seen in [false, true] {
        let mut w = all_set(Access::Granted);
        w.space.identity = Some(SpaceIdentity { foreign_v: seen, ..space("myshroom", "shroom") });
        w.sweep.foreign_v = Fact::known(!seen, Mono(950));
        let mut r = Run::new(w);
        r.cloud(CloudNews::Wired(Ok(space("myshroom", "shroom"))));
        assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Space(id)) if id.foreign_v == seen)), "seen {seen}");
        assert_eq!(r.w.space.identity.as_ref().map(|i| i.foreign_v), Some(seen));
    }
}

/// A space set up before the app kept what the survey saw has nothing to say about `v`: the
/// checkpoint's own space (under `shroom`, its file written by an earlier build) reads plain.
#[test]
fn a_space_that_never_recorded_a_v_says_nothing_about_one() {
    let mut file = serde_json::to_value(space("myshroom", "shroom")).unwrap();
    file.as_object_mut().unwrap().retain(|k, _| ["account_id", "label", "subdomain", "host", "bucket", "role", "member_id", "owner_name"].contains(&k.as_str()));
    let mut w = all_set(Access::Granted);
    w.space.identity = Some(serde_json::from_value(file).unwrap());
    let plain = "✓ Your link name — shroom.myshroom.workers.dev".to_string();
    assert_eq!(name_rows(&w), (plain.clone(), plain));
    assert!(!derive(&w).state_ids().contains(&"name-v-clash"));
}

#[test]
fn a_token_for_several_accounts_asks_which_one() {
    let mut r = Run::new(affirmed());
    r.paste(SecretField::Token, 9, Shape::Token);
    r.cloud(CloudNews::Validated { id: SecretId(9), result: Ok(vec![bagrat(), acme()]) });
    assert_eq!(win(&r.scene), "token-pick-account");
    assert!(!r.has(|e| matches!(e, Effect::Cloud(_))), "nothing is created before the choice");
    let Nest::PickAccount { accounts, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(accounts.iter().map(|c| (c.label.as_str(), c.selected)).collect::<Vec<_>>(), [("Bagrat's Account", true), ("Acme Engineering", false)]);
    r.on(Action::PickAccount("acme".into()));
    let Nest::PickAccount { accounts, .. } = nest(&r.scene) else { panic!() };
    assert!(accounts[1].selected);
    r.on(Action::PickAccount("bagrat".into()));
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SaveToken(SecretId(9), "bagrat".into()))));
    r.cloud(CloudNews::TokenSaved(SecretId(9)));
    assert_eq!(win(&r.scene), "card");
    assert_eq!(row(&r.scene, 1), "✓ A Cloudflare account — Bagrat's Account", "row one gains the account's name");

    // The choice is the space's, for good: a re-proof that reports one account keeps it,
    // the space keeps it, and it survives the screen-recording relaunch.
    r.cloud(swept(CLEAR, Some("myshroom"), false, None));
    assert_eq!(win(&r.scene), "done", "N45: the permissions come before the wiring");
    assert_eq!(row(&r.scene, 1), "✓ A Cloudflare account — Bagrat's Account");
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Space(id)) if id.account_name.as_deref() == Some("Bagrat's Account"))));
    r.on(Action::AllowRecording);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    r.on(Action::ReopenShroom);
    let mut r = reboot(&r, LaunchKind::PermissionRelaunch, quick(), None);
    r.send(Event::Rec(RecNews::Primed(true)));
    assert_eq!(win(&r.scene), "all-set");
    assert_eq!(row(&r.scene, 1), "✓ A Cloudflare account — Bagrat's Account");
}

#[test]
fn a_real_token_without_the_powers_points_back_at_the_button() {
    let mut r = Run::new(affirmed());
    r.paste(SecretField::Token, 5, Shape::Token);
    r.cloud(CloudNews::Validated { id: SecretId(5), result: Err(TokenFail::WrongPowers) });
    assert_eq!(win(&r.scene), "token-wrong-powers");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::CreateToken), "the create button is already on screen");
    // A stale answer for an earlier paste changes nothing.
    r.paste(SecretField::Token, 6, Shape::Token);
    r.cloud(CloudNews::Validated { id: SecretId(5), result: Ok(vec![bagrat()]) });
    assert_eq!(win(&r.scene), "token");
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::SaveToken(..)))));
}

/// The token row's nest, as the webview reads it.
fn token_nest(s: &Scene) -> serde_json::Value {
    let n = serde_json::to_value(nest(s)).unwrap();
    assert_eq!(n["kind"], "token", "the token row's nest");
    n
}

/// The token field's own fact: a paste was taken and shroom holds it (DECISIONS B104).
fn held(s: &Scene) -> serde_json::Value {
    token_nest(s)["held"].clone()
}

#[test]
fn the_token_field_holds_a_paste_while_its_checked_and_put_away_and_empties_when_refused() {
    let (yes, no) = (serde_json::json!(true), serde_json::json!(false));
    let mut r = Run::new(affirmed());
    assert_eq!(held(&r.scene), no, "nothing pasted yet");

    // Not a token's shape: nothing is kept — the field is empty, with its line.
    r.paste(SecretField::Token, 7, Shape::Garbled);
    assert_eq!((win(&r.scene), held(&r.scene)), ("token-garbled", no.clone()));

    // Taken: held while Cloudflare is asked about it…
    r.paste(SecretField::Token, 8, Shape::Token);
    assert_eq!((win(&r.scene), held(&r.scene)), ("token", yes.clone()), "the paste was taken");
    // …and refused: empty again, with its line.
    r.cloud(CloudNews::Validated { id: SecretId(8), result: Err(TokenFail::WrongPowers) });
    assert_eq!((win(&r.scene), held(&r.scene)), ("token-wrong-powers", no.clone()));
    r.paste(SecretField::Token, 9, Shape::Token);
    assert_eq!(held(&r.scene), yes);
    r.cloud(CloudNews::Validated { id: SecretId(9), result: Err(TokenFail::Rejected) });
    assert_eq!((win(&r.scene), held(&r.scene)), ("token-garbled", no.clone()));

    // Accepted: still held while it's put away — then the row ticks and the field is gone.
    r.paste(SecretField::Token, 10, Shape::Token);
    r.cloud(CloudNews::Validated { id: SecretId(10), result: Ok(vec![bagrat()]) });
    assert_eq!((win(&r.scene), held(&r.scene)), ("token", yes.clone()), "accepted, being put away");
    r.cloud(CloudNews::TokenSaved(SecretId(10)));
    assert_eq!(win(&r.scene), "card");
    assert!(row(&r.scene, 2).starts_with("✓ "), "{}", row(&r.scene, 2));

    // A token pasted in the welcome lands in the token row, held (N30, B159).
    let mut r = Run::new(window(unprovisioned()));
    r.paste(SecretField::Invite, 4, Shape::Token);
    assert_eq!((win(&r.scene), held(&r.scene)), ("token", yes));
}

/// Law 13 at the field: what the Scene says of a held paste is a yes or a no. Nothing of
/// the paste rides with it — not its text, not its length, not even which paste it was.
#[test]
fn a_held_token_is_a_bare_yes_never_anything_of_the_paste() {
    let holding = |id: u32| {
        let mut r = Run::new(affirmed());
        r.paste(SecretField::Token, id, Shape::Token);
        token_nest(&r.scene)
    };
    let (one, other) = (holding(1), holding(40_053));
    assert!(one["held"].is_boolean(), "a yes or a no, never a mask or a count: {}", one["held"]);
    assert_eq!(one, other, "the same field whichever paste is held");

    // Holding a paste turns two yes/no facts and adds nothing else: every other word in
    // the nest is the wizard's own, there before anything was pasted.
    let strip = |mut n: serde_json::Value| {
        let o = n.as_object_mut().unwrap();
        o.remove("held");
        o.remove("checking");
        n
    };
    assert_eq!(strip(one), strip(token_nest(&Run::new(affirmed()).scene)));
}

#[test]
fn the_welcome_registers_the_login_item_only_while_its_box_is_checked() {
    let mut r = Run::new(window(unprovisioned()));
    r.on(Action::SetLoginBox(false));
    let OnboardingBody::Welcome(b) = &r.scene.onboarding.as_ref().unwrap().body else { panic!() };
    assert!(!b.login.on);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(_)))), "unchecked: never registered");
    assert!(r.ids().contains(&"idle-unprovisioned"), "✕ ≡ Later");
    assert!(!r.scene.dock);

    let mut r = Run::new(window(unprovisioned()));
    r.on(Action::Later);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(true))));
    assert!(r.ids().contains(&"idle-unprovisioned"));
    r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
    assert_eq!(win(&r.scene), "welcome", "the same window, any day");
}

#[test]
fn the_door_follows_the_facts() {
    // No space, a saved token: the plan, not the welcome.
    let mut w = unprovisioned();
    w.space.token = true;
    let mut r = Run::new(w);
    r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
    assert_eq!(win(&r.scene), "card", "the stored token is re-proved while the plan stays marked");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));

    // A space whose chapter isn't done on this Mac: idle-ready, whose click opens the plan
    // at row six (bend 5).
    let mut w = at_done();
    w.session.windows.clear();
    let mut r = Run::new(w);
    assert_eq!(r.scene.tray.state, Some("idle-ready"));
    r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
    assert_eq!(win(&r.scene), "done");

    // A launch shroom did itself stays silent.
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::LoginItem, &World::blank(), fresh_quick(), None))));
    assert!(r.scene.onboarding.is_none());
    assert!(r.ids().contains(&"idle-unprovisioned"));
}

#[test]
fn a_relaunch_rederives_where_the_person_was() {
    // Mid-signup: the virgin fork (nothing is probeable before a token exists).
    let mut r = Run::new(plan());
    r.on(Action::NotYet);
    let r2 = reboot(&r, LaunchKind::User, fresh_quick(), None);
    assert_eq!(win(&r2.scene), "account");

    // After Later: the welcome.
    let mut r = Run::new(plan());
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    let r2 = reboot(&r, LaunchKind::User, fresh_quick(), None);
    assert_eq!(win(&r2.scene), "welcome");

    // Mid-wire: no stored step — the plan re-probes and wiring resumes.
    let mut r = Run::new(at_wiring());
    r.send(Event::Tick);
    let mut r2 = reboot(&r, LaunchKind::User, fresh_quick(), None);
    assert_eq!(win(&r2.scene), "card");
    assert_eq!(row(&r2.scene, 2), "✓ One access token for shroom");
    assert!(r2.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r2.cloud(swept(CLEAR, Some("myshroom"), false, None));
    assert_eq!(win(&r2.scene), "wiring");
    assert!(r2.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));

    // macOS's own quit-and-reopen looks like a user launch: the fresh restore.json says
    // it was the chapter (bend 9) — and a stale one is ignored.
    let mut r = Run::new(chapter(Access::Granted, false));
    r.send(Event::Os(OsNews::Terminating { ours: false }));
    assert_eq!(win(&r.scene), "relaunching");
    let restore = Restore { why: RelaunchWhy::Permission, window: WindowKind::Onboarding, written: Wall(1_790_000_000_000 + r.t as i64) };
    let r2 = reboot(&r, LaunchKind::User, QuickProbe { screen: false, ..quick() }, Some(restore.clone()));
    assert_eq!(win(&r2.scene), "perm-screen", "resumes right here — derived state");
    let stale = Restore { written: Wall(restore.written.0 - 200_000), ..restore };
    let r3 = reboot(&r, LaunchKind::User, QuickProbe { screen: false, ..quick() }, Some(stale));
    assert_eq!(win(&r3.scene), "done", "a stale restore is a plain click");
}

/// Since N45 the chapter comes before the wiring: a permission relaunch while wiring is
/// still under way resumes at the permissions step — no earlier row is current while the
/// re-probe runs (the 0.1.15 two-VM run showed the card step for ~6 s, CHECKPOINT §10).
#[test]
fn a_permission_relaunch_before_wiring_resumes_at_the_permissions_step() {
    let mut r = Run::new(window(named()));
    r.on(Action::AllowRecording);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert_eq!(win(&r.scene), "perm-screen");
    r.send(Event::Os(OsNews::Terminating { ours: false }));
    assert_eq!(win(&r.scene), "relaunching");
    let restore = Restore { why: RelaunchWhy::Permission, window: WindowKind::Onboarding, written: Wall(1_790_000_000_000 + r.t as i64) };
    let granted = QuickProbe { screen: true, mic: Access::Granted, camera: Access::Granted, ..quick() };
    for (launch, restore) in [(LaunchKind::User, Some(restore)), (LaunchKind::PermissionRelaunch, None)] {
        let mut r2 = reboot(&r, launch.clone(), granted.clone(), restore);
        assert_eq!(win(&r2.scene), "perm-priming", "{launch:?}: resumes in the chapter, never the card step");
        for n in 1..=4 {
            assert!(row(&r2.scene, n).starts_with('✓'), "{launch:?}: row {n} stays ticked while the re-probe runs");
        }
        assert!(r2.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "{launch:?}: the plan still re-probes");
        r2.cloud(swept(CLEAR, Some("myshroom"), false, None));
        assert_eq!(win(&r2.scene), "perm-priming", "{launch:?}");
        assert!(r2.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "{launch:?}: wiring resumes under the chapter");
    }
}

#[test]
fn a_denied_mic_never_blocks() {
    let mut r = Run::new(at_done());
    r.on(Action::AllowRecording);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Denied, camera: Access::Granted }));
    assert_eq!(win(&r.scene), "perm-screen");
    let mut r = reboot(&r, LaunchKind::PermissionRelaunch, QuickProbe { mic: Access::Denied, ..quick() }, None);
    r.send(Event::Rec(RecNews::Primed(true)));
    assert_eq!(win(&r.scene), "all-set");
    assert_eq!(row(&r.scene, 5), "✓ Letting macOS allow recording — microphone off for now");
}

#[test]
fn a_warm_up_that_isnt_clean_retries_after_a_backoff() {
    let mut r = Run::new(chapter(Access::Granted, true));
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));
    r.send(Event::Rec(RecNews::Primed(false)));
    assert!(r.effects.is_empty(), "never a tight loop");
    assert_eq!(win(&r.scene), "perm-priming");
    let retry = r.scene.next_wake.unwrap();
    r.tick_to(retry.0);
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));
}

#[test]
fn a_permission_prompt_never_comes_without_a_click() {
    // Grants reports outside the chapter ask nothing.
    let mut r = Run::new(at_done());
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.effects.is_empty());
    assert_eq!(win(&r.scene), "done");
}

#[test]
fn record_now_hands_off_to_the_tray() {
    let mut r = Run::new(all_set(Access::Granted));
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.scene.onboarding.is_none());
    assert_eq!(r.scene.popover.as_ref().map(|p| (p.id, p.activates)), Some(("config", true)));
    assert!(!r.scene.dock, "popovers never bring the dock icon");
}

/// Law 6 as ratified (SM:133 — DEVIATIONS #38's narrowing rejected by Bagrat on
/// 2026-10-13): the Dock icon shows while any window of shroom's is open, and goes with the
/// last of them.
#[test]
fn the_dock_icon_shows_while_any_window_is_open() {
    let mut w = all_set(Access::Granted);
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(950)));
    let mut r = Run::new(w);
    assert!(r.scene.onboarding.is_some() && r.scene.library.is_some());
    assert!(r.scene.dock, "the setup window is open: the Dock icon shows");
    r.on(Action::CloseWindow);
    assert!(r.scene.onboarding.is_none() && r.scene.library.is_some(), "the Library stays where it was");
    assert!(r.scene.dock, "setup is closed, the Library still open: the Dock icon stays");
    r.send(Event::WindowClosed(WindowKind::Library));
    assert!(r.scene.library.is_none() && !r.scene.dock, "the last window closed: the Dock icon goes");

    // Coming back to finish setup (this Mac's chapter isn't done): the icon is back with
    // the window, and goes with it again.
    let mut r = Run::new(at_done());
    assert_eq!(win(&r.scene), "done");
    assert!(r.scene.dock);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(r.scene.onboarding.is_none() && !r.scene.dock);
    r.click();
    assert!(r.scene.onboarding.is_some() && r.scene.popover.is_none(), "the mushroom opens the plan again (DEVIATIONS #5)");
    assert!(r.scene.dock);
}

/// Law 6's own teeth, without touching the core. The walkthroughs only ever hand the law a
/// Scene `derive` made, so the law must be able to fail on its own: it holds the Dock icon
/// to the World — the setup window's door — and a Scene that agrees with itself and parts
/// from the World is caught.
#[test]
fn law_6_holds_the_dock_icon_to_the_world() {
    // As derived: setup is open, the Dock icon shows, and the law holds.
    let w = at_done();
    let s = derive(&w);
    assert!(s.onboarding.is_some() && s.dock);
    assert_eq!(laws::law_6(&w, &s), None);

    // The setup window and its Dock icon dropped from the Scene together — it agrees with
    // itself — while the World's door to setup stands open.
    let mut dropped = s.clone();
    dropped.onboarding = None;
    dropped.dock = false;
    let why = laws::law_6(&w, &dropped).expect("law 6 let through a Scene that dropped the setup window and its Dock icon");
    assert!(why.starts_with("law 6: no Dock icon, and in the World a window of shroom's is open"), "{why}");

    // The door closed in the World, and a Scene that still shows the window and its icon.
    let mut closed = w.clone();
    closed.session.windows.remove(&WindowKind::Onboarding);
    let why = laws::law_6(&closed, &s).expect("law 6 let through a Dock icon with no setup window in the World");
    assert!(why.starts_with("law 6: a Dock icon, and in the World no window of shroom's is open"), "{why}");

    // A set-up Mac with the Library open: the icon as derived — and a Scene without it is
    // caught on the World's word.
    let mut w = all_set(Access::Granted);
    w.session.windows.remove(&WindowKind::Onboarding);
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(950)));
    let mut s = derive(&w);
    assert!(s.library.is_some() && s.onboarding.is_none() && s.dock);
    assert_eq!(laws::law_6(&w, &s), None);
    s.dock = false;
    let why = laws::law_6(&w, &s).expect("law 6 let the Library go without its Dock icon");
    assert!(why.starts_with("law 6: no Dock icon, and in the World a window of shroom's is open"), "{why}");
}

// ── the machine's drawn edges (TESTING-PLAN §2.2) ──

/// The map's notes this slice also walks: not states, so not edges — each the state its
/// world shows, and what makes the note's line true there.
fn note_state(name: &str) -> &str {
    match name {
        "launch-door" => "welcome",
        "charset" => "name",
        "mic-denied" => "perm-screen",
        other => other,
    }
}

const NOTES: &[&str] = &["launch-door", "charset", "mic-denied"];

/// One realized edge: the world it starts from and the one event that crosses it. A
/// `Booted` event runs in a new process built from the start world's disk.
fn realized() -> Vec<(&'static str, &'static str, World, Event)> {
    use Event as E;
    let act = |action: Action| E::Act { surface: Surface::Onboarding, action, rev: 0 };
    let cloud = E::Cloud;
    let booted = |launch, w: &World, q: QuickProbe| E::Booted(Box::new(disk(launch, w, q, None)));
    let fresh = World::blank();
    vec![
        ("idle-unprovisioned", "launch-door", unprovisioned(), E::Reopened(fresh_quick())),
        ("first-launch", "welcome", fresh.clone(), booted(LaunchKind::Install { join: false }, &fresh, fresh_quick())),
        ("welcome", "idle-unprovisioned", window(unprovisioned()), act(Action::Later)),
        ("welcome", "account", window(unprovisioned()), act(Action::SetUp)),
        ("idle-unprovisioned", "welcome", unprovisioned(), E::TrayClicked { right: false, quick: fresh_quick() }),
        ("account", "account-signing-up", plan(), act(Action::NotYet)),
        ("account-signing-up", "token", { let mut w = plan(); w.session.fork = Fork::SigningUp; w }, act(Action::SignedUp)),
        ("account", "token", plan(), act(Action::HaveOne)),
        ("token", "token-garbled", affirmed(), E::SecretPasted { field: SecretField::Token, id: SecretId(1), shape: Shape::Garbled }),
        ("token-garbled", "token", { let mut w = affirmed(); w.sweep.paste = Some(Paste::Garbled); w }, E::SecretPasted { field: SecretField::Token, id: SecretId(2), shape: Shape::Token }),
        ("token", "token-wrong-powers", { let mut w = affirmed(); w.sweep.paste = Some(Paste::Checking(SecretId(2))); w }, cloud(CloudNews::Validated { id: SecretId(2), result: Err(TokenFail::WrongPowers) })),
        ("token-wrong-powers", "token", { let mut w = affirmed(); w.sweep.paste = Some(Paste::WrongPowers); w }, E::SecretPasted { field: SecretField::Token, id: SecretId(3), shape: Shape::Token }),
        ("token", "card", { let mut w = affirmed(); w.sweep.paste = Some(Paste::Saving { id: SecretId(3), account: bagrat(), several: false }); w }, cloud(CloudNews::TokenSaved(SecretId(3)))),
        ("card", "gate-email", token_saved(), cloud(swept(Gates { email: true, card: false }, None, false, None))),
        ("gate-email", "card", gated(true, false), cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }))),
        ("card", "gate-card", token_saved(), cloud(swept(Gates { email: false, card: true }, None, false, None))),
        ("gate-card", "card", gated(false, true), cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }))),
        ("card", "name", token_saved(), cloud(swept(CLEAR, None, false, None))),
        ("card", "done", token_saved(), cloud(swept(CLEAR, Some("myshroom"), false, None))),
        ("name", "name-taken", { let mut w = at_name(); w.session.fields.name = "myshroom".into(); w }, cloud(CloudNews::Claimed { name: "myshroom".into(), result: Err(ClaimFail::Taken) })),
        ("name-taken", "name", { let mut w = at_name(); w.session.fields.name = "myshroom".into(); w.sweep.taken = Some("myshroom".into()); w }, E::Field { field: Field::Name, value: "myshroomhq".into() }),
        ("name", "charset", at_name(), E::Field { field: Field::Name, value: "My Shroom".into() }),
        ("name", "done", at_name(), cloud(CloudNews::Claimed { name: "myshroom".into(), result: Ok(()) })),
        ("wiring", "wizard-offline", at_wiring(), E::Failed { op: Op::Wire, class: FailClass::Transport }),
        ("wizard-offline", "wiring", { let mut w = at_wiring(); w.net.online = Fact::known(false, Mono(0)); w }, E::Os(OsNews::NetPath(true))),
        ("wiring", "all-set", at_wiring(), cloud(CloudNews::Wired(Ok(identity())))),
        ("token", "token-pick-account", { let mut w = affirmed(); w.sweep.paste = Some(Paste::Checking(SecretId(4))); w }, cloud(CloudNews::Validated { id: SecretId(4), result: Ok(vec![bagrat(), acme()]) })),
        ("wiring", "space-found", { let mut w = at_wiring(); w.sweep.found = Fact::Unknown; w }, cloud(CloudNews::Swept(SweepAnswer { found: Some(Some(14)), ..Default::default() }))),
        ("done", "perm-mic", at_done(), act(Action::AllowRecording)),
        ("perm-mic", "mic-denied", chapter(Access::NotAsked, false), E::Os(OsNews::Grants { screen: false, mic: Access::Denied, camera: Access::Granted })),
        ("perm-mic", "perm-screen", chapter(Access::NotAsked, false), E::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted })),
        ("perm-mic", "perm-camera", chapter(Access::NotAsked, false), E::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked })),
        ("perm-camera", "perm-screen", {
            let mut w = chapter(Access::Granted, false);
            w.mac.camera_asked = true;
            w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
            w
        }, E::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Denied })),
        ("perm-screen", "relaunching", chapter(Access::Granted, false), act(Action::ReopenShroom)),
        ("relaunching", "perm-screen", { let mut w = chapter(Access::Granted, false); w.launch.ending = true; w }, E::Booted(Box::new(disk(LaunchKind::PermissionRelaunch, &at_done(), QuickProbe { screen: false, ..quick() }, None)))),
        ("perm-screen", "perm-priming", chapter(Access::Granted, false), E::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::NotAsked })),
        ("perm-priming", "all-set", chapter(Access::Granted, true), E::Rec(RecNews::Primed(true))),
        ("all-set", "config", all_set(Access::Granted), act(Action::RecordNow)),
        ("all-set", "idle-ready", all_set(Access::Granted), act(Action::CloseWindow)),
    ]
}

/// In-scope drawn edges this slice doesn't build, each with its reason.
const OUT_OF_SCOPE: &[(&str, &str, &str)] = &[
    ("landing", "source-view", "shroom.am — the web suite"),
    ("source-view", "landing", "shroom.am — the web suite"),
    ("landing", "installing", "the installer — its own suite"),
    ("installing", "install-net-drop", "the installer — its own suite"),
    ("install-net-drop", "installing", "the installer — its own suite"),
    ("installing", "install-checksum-fail", "the installer — its own suite"),
    ("install-checksum-fail", "installing", "the installer — its own suite"),
    ("installing", "install-unsupported-os", "the installer — its own suite"),
    ("installing", "install-refused-midtake", "the installer — its own suite"),
    ("install-refused-midtake", "installing", "the installer — its own suite"),
    ("installing", "updating", "the installer — its own suite"),
    ("installing", "first-launch", "the installer's half; the app's half is first-launch → welcome"),
    ("idle-ready", "space-degraded", "space-degraded — realized in region5.rs (Region 2's afterlife)"),
    ("space-degraded", "repair-mood", "the repair mood — realized in region5.rs"),
    ("repair-mood", "space-storage-gone", "the repair mood — realized in region5.rs"),
    ("repair-mood", "name-changed", "the repair mood — realized in region5.rs"),
    ("all-set", "invite-born", "Invite someone — realized in region6.rs"),
    ("perm-priming", "member-all-set", "a member's chapter — realized in region6.rs"),
];

const CHAPTER: &[&str] = &["done", "perm-mic", "perm-camera", "perm-screen", "relaunching", "perm-priming", "all-set"];

#[test]
fn every_in_scope_drawn_edge_is_realized_or_out_of_scope() {
    let in_scope = drawn_edges(|from, to| [from, to].iter().any(|s| matches!(region_of(s), 1 | 2)) || CHAPTER.contains(&from));
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let mut failures = Vec::new();
    for (from, to, world, event) in realized() {
        assert!(NOTES.contains(&to) || covered.insert((from.into(), to.into())), "{from} → {to} realized twice");
        let before = derive(&world).state_ids();
        if from != "first-launch" && !before.contains(&note_state(from)) {
            failures.push(format!("{from} → {to}: the start world derives {before:?}"));
            continue;
        }
        let mut r = if matches!(event, Event::Booted(_)) { Run::new(World::blank()) } else { Run::new(world) };
        r.send(event);
        let after = r.ids();
        let crossed = after.contains(&note_state(to))
            && match to {
                "charset" => json(&r.scene).contains("Lowercase letters, numbers, and dashes."),
                "mic-denied" => r.w.mac.mic.is(&Access::Denied),
                "config" => r.scene.popover.as_ref().is_some_and(|p| p.id == "config"),
                _ => true,
            };
        if !crossed {
            failures.push(format!("{from} → {to}: ended at {after:?}"));
        }
    }
    for (from, to, _) in OUT_OF_SCOPE {
        assert!(covered.insert(((*from).into(), (*to).into())), "{from} → {to} both realized and out of scope");
    }
    let missing: Vec<_> = in_scope.difference(&covered).collect();
    let stale: Vec<_> = covered.difference(&in_scope).collect();
    assert!(missing.is_empty(), "drawn edges neither realized nor out of scope: {missing:?}");
    assert!(stale.is_empty(), "listed edges the machine doesn't draw: {stale:?}");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn every_wizard_op_has_a_deadline() {
    for (op, ms) in [(Op::Validate, 60_000), (Op::SaveToken, 60_000), (Op::Sweep, 60_000), (Op::Claim, 60_000), (Op::Wire, 180_000)] {
        assert_eq!(op.deadline_ms(), ms, "{op:?}");
    }
    // A lost answer ends as a stated face, never a stall: the deadline passes, the step
    // retries after its backoff.
    let mut w = token_saved();
    let (fx, _) = update(&mut w, shroom_core::Envelope { mono: Mono(1_000), wall: Wall(0), event: Event::Tick });
    assert!(fx.contains(&Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    let (_, _) = update(&mut w, shroom_core::Envelope { mono: Mono(61_000), wall: Wall(0), event: Event::Tick });
    assert_eq!(w.inflight.failures[&Op::Sweep].class, FailClass::Timeout);
    assert_eq!(win(&derive(&w)), "card", "a timeout isn't offline: the spinner keeps watching");
    let (fx, _) = update(&mut w, shroom_core::Envelope { mono: Mono(66_000), wall: Wall(0), event: Event::Tick });
    assert!(fx.contains(&Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "retried after the backoff");
}

// ── the review's edges: liveness, the one chained prompt, and the survey's label ──

/// A permission relaunch resumes the chapter, but only the mic answer the Allow recording
/// click asked for chains the screen step — a grants report after the relaunch asks nothing.
#[test]
fn after_a_relaunch_no_grants_report_reopens_system_settings() {
    let mut w = at_done();
    w.session.windows.clear();
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::PermissionRelaunch, &w, QuickProbe { screen: false, ..quick() }, None))));
    assert_eq!(win(&r.scene), "perm-screen");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.effects.is_empty(), "no click asked: {:?}", r.effects);
    r.send(Event::WindowFocused(WindowKind::Onboarding, QuickProbe { screen: false, ..quick() }));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.effects.is_empty());
    assert_eq!(win(&r.scene), "perm-screen");

    // The click's own chain survives the window refocusing before macOS's report lands.
    let mut r = Run::new(at_done());
    r.on(Action::AllowRecording);
    r.send(Event::WindowFocused(WindowKind::Onboarding, QuickProbe { screen: false, ..quick() }));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))));

    // Closing the window mid-prompt drops the chain with it.
    let mut r = Run::new(at_done());
    r.on(Action::AllowRecording);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.effects.is_empty());
}

/// A pass that stops early is a whole answer: no re-fire until the next entry or the poll.
#[test]
fn a_pass_that_stops_early_never_loops() {
    let mut w = plan();
    w.space.token = true;
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r.cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() }));
    assert_eq!(win(&r.scene), "token");
    assert!(r.effects.is_empty(), "a dead token answers the pass: {:?}", r.effects);
    r.tick_to(r.t + 120_000);
    assert!(r.effects.is_empty());
    // The next entry re-probes.
    r.send(Event::WindowFocused(WindowKind::Onboarding, quick()));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));

    let mut w = plan();
    w.space.token = true;
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.cloud(CloudNews::Swept(SweepAnswer {
        token: Some(TokenVerdict::Valid { account: bagrat(), several: false }),
        gates: Some(Gates { email: true, card: true }),
        ..Default::default()
    }));
    assert_eq!(win(&r.scene), "gate-email");
    assert!(r.effects.is_empty(), "a gate stops the pass; only its poll re-asks: {:?}", r.effects);
    let poll = r.scene.next_wake.unwrap();
    assert_eq!(poll.0, r.t + 5_000);
    r.tick_to(poll.0);
    assert_eq!(r.effects, vec![Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates))]);
}

/// Answer the first cloud call in flight with a success.
fn answer_one(r: &mut Run) -> bool {
    let Some(e) = r.effects.iter().find(|e| matches!(e, Effect::Cloud(_))).cloned() else { return false };
    match e {
        Effect::Cloud(CloudCmd::Validate(id)) => r.cloud(CloudNews::Validated { id, result: Ok(vec![bagrat()]) }),
        Effect::Cloud(CloudCmd::SaveToken(id, _)) => r.cloud(CloudNews::TokenSaved(id)),
        Effect::Cloud(CloudCmd::Sweep(SweepPart::Full)) => r.cloud(swept(CLEAR, None, false, None)),
        Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates)) => r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() })),
        Effect::Cloud(CloudCmd::Claim(name)) => r.cloud(CloudNews::Claimed { name, result: Ok(()) }),
        Effect::Cloud(CloudCmd::Wire { .. }) => r.cloud(CloudNews::Wired(Ok(identity()))),
        _ => return false,
    };
    true
}

/// `wizard-offline` never outlives the network: a transport failure of each wizard call,
/// with the OS's path never flipping (the captive portal, the LAN MTU black hole), resumes
/// by the backoff or a re-probe once the calls answer.
#[test]
fn every_wizard_transport_failure_resumes_without_the_path_flipping() {
    let mut cases: Vec<(Op, Run)> = Vec::new();
    let mut r = Run::new(affirmed());
    r.paste(SecretField::Token, 8, Shape::Token);
    cases.push((Op::Validate, r));
    let mut w = affirmed();
    w.sweep.paste = Some(Paste::Checking(SecretId(8)));
    let mut r = Run::new(w);
    r.cloud(CloudNews::Validated { id: SecretId(8), result: Ok(vec![bagrat()]) });
    cases.push((Op::SaveToken, r));
    let mut r = Run::new(token_saved());
    r.send(Event::Tick);
    cases.push((Op::Sweep, r));
    let mut r = Run::new(at_name());
    r.typed("myshroom").on(Action::Claim);
    cases.push((Op::Claim, r));
    let mut r = Run::new(at_wiring());
    r.send(Event::Tick);
    cases.push((Op::Wire, r));

    for (op, mut r) in cases {
        assert!(r.w.inflight.ops.contains_key(&op), "{op:?} wasn't in flight");
        r.send(Event::Failed { op, class: FailClass::Transport });
        assert_eq!(win(&r.scene), "wizard-offline", "{op:?}");
        let mut steps = 0;
        while win(&r.scene) == "wizard-offline" {
            steps += 1;
            assert!(steps < 20, "{op:?}: stuck at wizard-offline");
            if !answer_one(&mut r) {
                let wake = r.scene.next_wake.unwrap_or_else(|| panic!("{op:?}: wizard-offline with nothing to wake for"));
                r.tick_to(wake.0);
            }
        }
        assert_ne!(win(&r.scene), "-", "{op:?}");
    }

    // A lost claim that landed after all: the re-probe finds the name and wiring goes on.
    let mut r = Run::new(at_name());
    r.typed("myshroom").on(Action::Claim);
    r.send(Event::Failed { op: Op::Claim, class: FailClass::Transport });
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "the plan re-probes");
    r.cloud(swept(CLEAR, Some("myshroom"), false, None));
    assert_eq!(win(&r.scene), "done", "N45: the permissions come before the wiring");
}

/// ✕ on the plan forgets the fork with the window: Set up my space always asks afresh.
#[test]
fn closing_the_plan_mid_fork_lands_set_up_on_the_clean_question() {
    for first in [Action::NotYet, Action::HaveOne] {
        let mut r = Run::new(plan());
        r.on(first.clone());
        r.send(Event::WindowClosed(WindowKind::Onboarding));
        r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
        assert_eq!(win(&r.scene), "welcome");
        r.on(Action::SetUp);
        assert_eq!(win(&r.scene), "account", "after {first:?}");
        assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::OpenUrl(_)))));
    }
}

/// `first-launch` is the installer's window only; the welcome a later tray click opens is
/// the any-day one.
#[test]
fn first_launch_is_only_the_installers_window() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Install { join: false }, &World::blank(), fresh_quick(), None))));
    assert!(r.ids().contains(&"first-launch"));
    r.on(Action::Later);
    r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
    assert_eq!(win(&r.scene), "welcome");
    assert!(!r.ids().contains(&"first-launch"), "{:?}", r.ids());
}

/// CHECKPOINT §10 11b.1 (DECISIONS B160; Bagrat: “we should probably have an uninstall
/// option at that state”): a Mac with no space has no Settings, so the welcome carries
/// “Uninstall shroom…”, quiet at its foot; it asks first, in the sheet Settings shows, with
/// only the line that's true of a Mac without a space.
#[test]
fn the_welcome_can_uninstall_shroom() {
    let mut r = Run::new(window(unprovisioned()));
    let b = r.scene.onboarding.as_ref().unwrap().buttons().into_iter().find(|b| b.action == Action::Uninstall).cloned().expect("Uninstall shroom… on the welcome");
    assert_eq!((b.label.as_str(), b.style), ("Uninstall shroom…", shroom_core::scene::Style::Quiet));
    r.on(Action::Uninstall);
    assert_eq!(win(&r.scene), "welcome");
    assert!(r.ids().contains(&"uninstall-confirm"), "{:?}", r.ids());
    assert!(r.scene.settings.is_none(), "Settings needs a space");
    let j = json(&r.scene);
    assert!(j.contains("Uninstall shroom?"));
    assert!(j.contains("This Mac lets go of shroom: the app, its settings, and its permission to record your screen, microphone and camera."));
    assert!(!j.contains("Your space, every video in it"), "no space on this Mac");
    // N23's law: no Return default; Esc ≡ Cancel.
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.effects.is_empty());
    r.key(Surface::Onboarding, Key::Esc);
    assert!(!r.ids().contains(&"uninstall-confirm"));
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::SetUp));
    // Asked again and answered: shroom removes itself.
    r.on(Action::Uninstall);
    r.on(Action::ConfirmUninstall);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::Uninstall)));
    let buttons = r.scene.onboarding.as_ref().unwrap().buttons().into_iter().map(|b| (b.label.clone(), b.enabled)).collect::<Vec<_>>();
    assert_eq!(buttons, [("Cancel".to_string(), false), ("Uninstalling…".to_string(), false)]);
}

/// A relaunch that never comes: `relaunching` ends at its deadline, back at perm-screen with
/// Reopen shroom offered again and the restore note withdrawn.
#[test]
fn a_relaunch_that_never_comes_returns_to_perm_screen() {
    let mut r = Run::new(chapter(Access::Granted, false));
    r.on(Action::ReopenShroom);
    assert_eq!(win(&r.scene), "relaunching");
    let deadline = r.scene.next_wake.expect("relaunching has a deadline");
    r.tick_to(deadline.0);
    assert_eq!(win(&r.scene), "perm-screen");
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Restore(None))));
    assert!(r.scene.onboarding.as_ref().unwrap().buttons().iter().any(|b| b.action == Action::ReopenShroom));

    let mut r = Run::new(chapter(Access::Granted, false));
    r.send(Event::Os(OsNews::Terminating { ours: false }));
    assert_eq!(win(&r.scene), "relaunching");
    let deadline = r.scene.next_wake.expect("macOS's relaunch has a deadline too");
    r.tick_to(deadline.0);
    assert_eq!(win(&r.scene), "perm-screen");
}

/// Wiring's fallback classification: a gate that closed after the card step routes there,
/// and once it clears the plan re-probes and wiring resumes.
#[test]
fn wiring_falls_back_to_the_gates() {
    use shroom_core::event::WireFail;
    for (gates, face) in [(Gates { email: true, card: true }, "gate-email"), (Gates { email: false, card: true }, "gate-card")] {
        let mut r = Run::new(at_wiring());
        r.send(Event::Tick);
        assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
        r.cloud(CloudNews::Wired(Err(WireFail::Gate(gates))));
        assert_eq!(win(&r.scene), face);
        let poll = r.scene.next_wake.unwrap();
        r.tick_to(poll.0);
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates))));
        r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }));
        assert_eq!(win(&r.scene), "card");
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
        r.cloud(swept(CLEAR, Some("myshroom"), false, None));
        assert_eq!(win(&r.scene), "wiring");
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into() })));
    }
}

/// The label is the survey's (B24, B26): past someone else's `v` and `shroom` it is
/// `shroom-v`; with every label taken, wiring never writes one.
#[test]
fn the_label_is_the_surveys() {
    let mut r = Run::new(token_saved());
    r.send(Event::Tick);
    r.cloud(CloudNews::Swept(SweepAnswer { label: Some(Some("shroom-v".into())), ..sweep_answer(Some("myname")) }));
    assert_eq!(win(&r.scene), "done", "N45: the permissions come before the wiring");
    assert!(r.ids().contains(&"name-v-clash"));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "shroom-v".into() })));
    assert_eq!(row(&r.scene, 4), "✓ Your link name — shroom-v.myname.workers.dev (“v” was already taken in your Cloudflare)");

    let mut r = Run::new(token_saved());
    r.send(Event::Tick);
    r.cloud(CloudNews::Swept(SweepAnswer { label: Some(None), ..sweep_answer(Some("myname")) }));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "never a label someone else holds");
}

fn sweep_answer(subdomain: Option<&str>) -> SweepAnswer {
    let CloudNews::Swept(a) = swept(CLEAR, subdomain, true, None) else { unreachable!() };
    a
}

#[test]
fn an_unchecked_box_switches_a_registered_login_item_off_at_dismissal() {
    // After Disconnect or Leave the login item stays on (B34/N36) and the welcome comes
    // back: U4 applies the box at dismissal both ways.
    let mut w = window(unprovisioned());
    w.mac.login_item = Fact::known(true, Mono(0));
    let mut r = Run::new(w.clone());
    r.on(Action::SetLoginBox(false));
    r.on(Action::Later);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(false))), "Later with the box unchecked switches it off");

    let mut r = Run::new(w.clone());
    r.on(Action::SetLoginBox(false));
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(false))), "✕ ≡ Later, both ways");

    let mut r = Run::new(w.clone());
    r.on(Action::SetLoginBox(false));
    r.on(Action::SetUp);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(false))), "Set up my space is a dismissal too");

    // Checked and already on: nothing to change.
    let mut r = Run::new(w);
    r.on(Action::Later);
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(_)))), "already registered: never re-registered");
}

#[test]
fn launches_shroom_does_itself_stay_silent_and_the_installers_launch_is_the_first_launch() {
    // The installer's fresh install (`--relaunched=install`): the welcome, its first-launch face.
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Install { join: false }, &World::blank(), fresh_quick(), None))));
    let o = r.scene.onboarding.as_ref().expect("the welcome opens");
    assert!(o.faces.contains(&"first-launch"));
    assert!(o.activates);

    // The install script's update lane (`--relaunched=install-update`) and a login launch:
    // tray-only and silent — on a fresh Mac and on a ready one.
    for kind in [LaunchKind::InstallUpdate, LaunchKind::LoginItem] {
        let mut r = Run::new(World::blank());
        r.send(Event::Booted(Box::new(disk(kind, &World::blank(), fresh_quick(), None))));
        assert!(r.scene.onboarding.is_none() && r.scene.popover.is_none() && !r.scene.dock, "{kind:?} opens nothing");
        assert!(r.ids().contains(&"idle-unprovisioned"));

        let mut w = ready();
        w.once.seen_version = Some("1.2.0".into());
        let mut r = Run::new(World::blank());
        r.send(Event::Booted(Box::new(disk(kind, &w, quick(), None))));
        assert!(r.scene.popover.is_none() && r.scene.onboarding.is_none() && !r.scene.dock, "{kind:?} opens nothing on a ready Mac");
        assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.seen_version.as_deref() == Some("1.3.0"))), "{kind:?}: the new version is seen, never announced");
        r.click();
        assert!(!serde_json::to_string(&r.scene.popover).unwrap().contains("Updated to"), "{kind:?}: no “Updated to … ✓”");
    }

    // A person's launch is a click (N41): a ready Mac's popover, asked.
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::User, &ready(), quick(), None))));
    assert!(r.scene.popover.as_ref().is_some_and(|p| p.id == "config" && p.activates));
}

/// Law 12's converse over this region's inventory: from every row, every op in flight —
/// as the row stands, after each button it offers, after its next wake — failed every way
/// lands (a face, a line, its note read, a noted retry), with the laws at every step.
#[test]
fn every_failure_from_every_row_lands() {
    let bad = every_failure_lands(rows());
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

/// Law 12's own teeth, without touching the core: a step that swallows a failure — the
/// claim's spinner simply gone, no face, no note, no wake — or hammers it (the paste's check
/// asked again at once with no backoff) is caught by law 12 itself, not by a walkthrough's
/// assertion. The spinner going away is not a face.
#[test]
fn law_12_catches_a_failure_swallowed_or_hammered() {
    fn caught(pw: &World, ps: &Scene, ev: &Event, nw: &World, fx: &[Effect]) -> String {
        let ns = derive(nw);
        let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| laws::check(pw, ps, ev, nw, &ns, fx))).expect_err("law 12 let it through");
        err.downcast_ref::<String>().cloned().unwrap_or_default()
    }
    // Swallowed: the claim fails, and nothing shows but its spinner going away.
    let mut r = Run::new(at_name());
    r.typed("myshroom").on(Action::Claim);
    let mut nw = r.w.clone();
    nw.now = Mono(r.t + 10);
    nw.inflight.ops.remove(&Op::Claim);
    let why = caught(&r.w, &r.scene, &Event::Failed { op: Op::Claim, class: FailClass::Transport }, &nw, &[]);
    assert!(why.starts_with("law 12: Claim failed into nothing"), "{why}");
    // …and its deadline going by the same way.
    let mut nw = r.w.clone();
    nw.now = r.w.inflight.ops[&Op::Claim].plus(10);
    nw.inflight.ops.remove(&Op::Claim);
    let why = caught(&r.w, &r.scene, &Event::Tick, &nw, &[]);
    assert!(why.starts_with("law 12: Claim timed out into nothing"), "{why}");
    // Hammered: the paste's check fails and is asked again at once, no failure noted.
    let mut r = Run::new(affirmed());
    r.paste(SecretField::Token, 8, Shape::Token);
    let check = r.effects.iter().find(|e| e.op() == Some(Op::Validate)).cloned().expect("the check runs");
    let mut nw = r.w.clone();
    nw.now = Mono(r.t + 10);
    nw.inflight.ops.insert(Op::Validate, nw.now.plus(Op::Validate.deadline_ms()));
    let why = caught(&r.w, &r.scene, &Event::Failed { op: Op::Validate, class: FailClass::Transport }, &nw, &[check]);
    assert!(why.starts_with("law 12: Validate failed into nothing"), "{why}");
    // The core itself lands both: wizard-offline says it, and the backoff runs it again.
    r.send(Event::Failed { op: Op::Validate, class: FailClass::Transport });
    assert_eq!(win(&r.scene), "wizard-offline");
    // The name guard, recomputed by the law: an uppercase that lands, or a rejected
    // character, is caught; the core lowercases silently and keeps only the set.
    let r = Run::new(at_name());
    for (typed, landed) in [("MyShroom", "MyShroom"), ("my shroom", "my shroom"), ("my_shroom", "myshroom-")] {
        let mut nw = r.w.clone();
        nw.now = Mono(r.t + 10);
        nw.session.fields.name = landed.into();
        let why = caught(&r.w, &r.scene, &Event::Field { field: Field::Name, value: typed.into() }, &nw, &[]);
        assert!(why.contains("law 12: a rejected character landed") || why.contains("law 12: the name guard kept"), "{typed}: {why}");
    }
    let mut r = Run::new(at_name());
    r.typed("My Shroom!");
    assert_eq!(r.w.session.fields.name, "myshroom");
}

/// the machine's law 12, invariant 1: every failure-state this region's rows reach is its own
/// kebab-case state with its own render and its own way on.
#[test]
fn every_failure_here_is_its_own_state() {
    let bad = laws::failures_are_states(&rows(), laws::failures_owned_by("onboarding"));
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

// ── N43: the chapter asks mic → camera → screen, one surface at a time ──

fn cam() -> Device {
    Device { uid: "cam".into(), name: "FaceTime HD Camera".into(), builtin: true, continuity: false, system_default: false }
}

/// The chapter at its first ask: Allow recording just put the mic prompt to macOS.
fn chapter_asking(cameras: Vec<Device>) -> Run {
    let mut w = chapter(Access::NotAsked, false);
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.mac.cameras = Fact::known(cameras, Mono(0));
    Run::new(w)
}

#[test]
fn the_mics_answer_asks_the_camera_and_the_cameras_answer_asks_the_screen() {
    let mut r = chapter_asking(vec![cam()]);
    assert_eq!(win(&r.scene), "perm-mic");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert_eq!(win(&r.scene), "perm-camera", "the camera is asked at setup (N43)");
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(Perm::Camera))], "one prompt at a time");
    assert!(matches!(nest(&r.scene), Nest::PermCamera), "macOS's own camera prompt is up: nothing of ours");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.effects.is_empty(), "the camera is asked once, never on every grants report");

    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Denied }));
    assert_eq!(win(&r.scene), "perm-screen", "a denied camera never blocks");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))), "the camera's answer chains the screen step");
    assert!(!r.w.prefs.show_yourself, "a camera denied at setup turns the bubble off");
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if !p.show_yourself)));
}

#[test]
fn show_yourself_starts_on_and_a_granted_camera_keeps_it_on() {
    assert!(Prefs::default().show_yourself, "on from the first start (N43, Bagrat: “yes on by default”)");
    let mut r = chapter_asking(vec![cam()]);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.w.prefs.show_yourself);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))], "nothing to write: the default holds");
}

#[test]
fn a_mac_with_no_camera_goes_from_the_mic_straight_to_the_screen() {
    let mut r = chapter_asking(vec![]);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Denied, camera: Access::NotAsked }));
    assert_eq!(win(&r.scene), "perm-screen");
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))], "no prompt for hardware that isn't there");
}

#[test]
fn allow_recording_with_the_mic_answered_asks_the_camera_first() {
    let mut w = chapter(Access::Granted, false);
    w.session.chapter = false;
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w.mac.cameras = Fact::known(vec![cam()], Mono(0));
    let mut r = Run::new(w);
    assert_eq!(win(&r.scene), "done");
    r.on(Action::AllowRecording);
    assert_eq!(win(&r.scene), "perm-camera");
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::RequestPermission(Perm::Camera))]);
}

#[test]
fn while_macos_asks_about_the_screen_shroom_opens_nothing_and_points_at_its_window() {
    let mut r = chapter_asking(vec![]);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert_eq!(win(&r.scene), "perm-screen");
    r.send(Event::Os(OsNews::ScreenAsked { alert: true }));
    assert!(r.effects.is_empty(), "macOS's alert leads; its Open System Settings opens the pane");
    let Nest::PermScreen { line, settings, .. } = nest(&r.scene) else { panic!("perm-screen") };
    assert_eq!(line, "macOS is asking about screen recording now — choose Open System Settings in its window, then flip the switch next to shroom. When macOS offers to quit and reopen it, say yes. I'll be right where you left me.");
    let door = settings.as_ref().expect("a quiet door to the pane, for a closed alert");
    assert_eq!(door.quiet, "Closed macOS's window?");
    assert_eq!(door.button.label, "Open System Settings");
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None, "perm-screen has no Return default");

    r.on(Action::OpenScreenSettings);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::SettingsPane(shroom_core::effect::Pane::ScreenRecording))]);
    let Nest::PermScreen { line, settings, .. } = nest(&r.scene) else { panic!("perm-screen") };
    assert_eq!(line, "macOS handles screen recording in System Settings — it's open. Flip the switch next to shroom, and when macOS offers to quit and reopen it, say yes. I'll be right where you left me.");
    assert!(settings.is_none(), "the pane is open: the door goes");
}

#[test]
fn when_macos_doesnt_ask_shroom_has_opened_the_pane_and_says_so() {
    let mut r = chapter_asking(vec![]);
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    r.send(Event::Os(OsNews::ScreenAsked { alert: false }));
    let Nest::PermScreen { line, settings, .. } = nest(&r.scene) else { panic!("perm-screen") };
    assert_eq!(line, "macOS handles screen recording in System Settings — it's open. Flip the switch next to shroom, and when macOS offers to quit and reopen it, say yes. I'll be right where you left me.");
    assert!(settings.is_none());
}

#[test]
fn row_six_and_dones_preview_name_the_camera() {
    let r = Run::new(at_done());
    assert_eq!(row(&r.scene, 5), "→ Letting macOS allow recording (microphone, camera and screen — a one-time yes each)");
    let Nest::Done { lines, .. } = nest(&r.scene) else { panic!("done") };
    assert_eq!(lines[0], "macOS will ask three times: your microphone and your camera — one click each — then screen recording, which goes through System Settings.");
}

/// Cloudflare makes no token until the email is confirmed, and shroom can't see that
/// before it has one (CHECKPOINT §10, 3.1): the signup trip says the email comes first,
/// and the token step reached by I signed up reminds — until a paste lands. I have one
/// never sees it (N45).
#[test]
fn the_signup_trip_says_the_email_comes_first() {
    const REMINDER: &str = "Cloudflare won't make it yet? Confirm your email first — their message is in your inbox.";
    let mut r = Run::new(plan());
    r.on(Action::NotYet);
    let Nest::SigningUp { line, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(line, "Cloudflare's signup is open in your browser. Finish there, then confirm your email from the message they send — they make no token until you do. I'll wait.");
    r.on(Action::SignedUp);
    assert_eq!(win(&r.scene), "token");
    let Nest::Token { quiet, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(quiet.as_deref(), Some(REMINDER), "the sign-up path reminds");
    r.paste(SecretField::Token, 3, Shape::Garbled);
    let Nest::Token { quiet, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(*quiet, None, "a paste landed: the reminder goes");

    let mut r = Run::new(plan());
    r.on(Action::HaveOne);
    let Nest::Token { quiet, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(*quiet, None, "I have one: no reminder");

    // The window closes: the next Set up my space starts on the clean question.
    let mut r = Run::new(plan());
    r.on(Action::NotYet).on(Action::SignedUp);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(!r.w.session.fields.signed_up);
}

/// The card gate says where Add credit card goes and that shroom is watching — and, once
/// it was clicked, that the page is open (CHECKPOINT §10, 3.2; N45).
#[test]
fn the_card_gate_says_where_it_goes_and_that_it_watches() {
    let mut r = Run::new(gated(false, true));
    assert_eq!(win(&r.scene), "gate-card");
    let Nest::Gate { quiet, button, .. } = nest(&r.scene) else { panic!() };
    assert!(button.is_some(), "Add credit card");
    assert_eq!(quiet.as_deref(), Some("Add credit card opens your Cloudflare billing page. I'll notice the card the moment it's there and carry on."));
    r.on(Action::AddCard);
    let Nest::Gate { quiet, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(quiet.as_deref(), Some("Billing is open in your browser — add the card there. I'll carry on by myself."));
    // The email gate says its own line, nothing beneath.
    let r = Run::new(gated(true, false));
    let Nest::Gate { quiet, .. } = nest(&r.scene) else { panic!() };
    assert_eq!(*quiet, None);
}

/// Wiring runs under the permissions chapter (N45) and, if the chapter ends first, is the
/// last step; if wiring ends first, the chapter's end is all set.
#[test]
fn wiring_finished_under_the_chapter_ends_at_all_set() {
    let mut r = Run::new(named());
    assert_eq!(win(&r.scene), "done");
    r.send(Event::Tick);
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "wiring starts underneath");
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert_eq!(win(&r.scene), "done", "the chapter goes on; the wiring row just ticks");
    assert_eq!(row(&r.scene, 6), "✓ Wiring it up");
    r.on(Action::AllowRecording);
    assert_eq!(win(&r.scene), "perm-mic");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Mic))));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert_eq!(win(&r.scene), "perm-screen", "the chained screen step, wired or not");
    // A gate that closed meanwhile is the fallback, even mid-chapter (N16).
    let mut r = Run::new(named());
    r.send(Event::Tick);
    r.cloud(CloudNews::Wired(Err(shroom_core::event::WireFail::Gate(Gates { email: false, card: true }))));
    assert_eq!(win(&r.scene), "gate-card");
}

// ── N74: the waits show what's happening ──

const DONE: Glyph = Glyph::Done;
const NOW: Glyph = Glyph::Working;
const AHEAD: Glyph = Glyph::Ahead;

fn p(g: Glyph, line: &str, clock: Option<&str>) -> (Glyph, String, Option<String>) {
    (g, line.into(), clock.map(String::from))
}

#[test]
fn the_card_check_shows_its_parts_as_they_answer() {
    let mut r = Run::new(token_saved());
    r.at(1_000).send(Event::Tick);
    assert_eq!(win(&r.scene), "card");
    // Nothing answered yet: the first part is underway.
    assert_eq!(parts(&r.scene), [
        p(NOW, "Checking your token…", None),
        p(AHEAD, "Checking your Cloudflare email is confirmed", None),
        p(AHEAD, "Checking that storage is switched on", None),
        p(AHEAD, "Looking for your link name and any earlier space", None),
    ]);
    r.cloud(CloudNews::Part { op: Op::Sweep, part: 2 });
    let began = r.t;
    assert_eq!(parts(&r.scene)[..2], [p(DONE, "Your token works", None), p(NOW, "Checking your Cloudflare email is confirmed…", None)]);
    r.cloud(CloudNews::Part { op: Op::Sweep, part: 3 });
    let storage = r.t;
    r.tick_to(storage + 600);
    assert_eq!(parts(&r.scene)[2], p(NOW, "Checking that storage is switched on…", None), "under a second: no clock yet");
    assert!(r.scene.next_wake.is_some_and(|t| t.0 <= storage + 1_000), "the clock's first second is a wake");
    r.tick_to(storage + 2_000);
    assert_eq!(parts(&r.scene), [
        p(DONE, "Your token works", None),
        p(DONE, "Your Cloudflare email is confirmed", None),
        p(NOW, "Checking that storage is switched on…", Some("0:02")),
        p(AHEAD, "Looking for your link name and any earlier space", None),
    ], "a clock on the spinning row only");
    assert!(storage > began);
    r.cloud(CloudNews::Part { op: Op::Sweep, part: 4 });
    assert_eq!(parts(&r.scene)[2..], [
        p(DONE, "Storage is switched on", None),
        p(NOW, "Looking for your link name and any earlier space…", None),
    ]);
    r.cloud(swept(CLEAR, None, false, None));
    assert_eq!(win(&r.scene), "name", "the pass answered: the check's parts are behind it");
    assert_eq!(r.w.sweep.check, None, "no step is kept once the pass answers (law 1)");
    // A pass that answers part 1 again (a retry, a relaunch) starts over: nothing is ticked
    // that this pass hasn't answered.
    let mut r = Run::new(checking(token_saved(), 3));
    r.cloud(CloudNews::Part { op: Op::Sweep, part: 1 });
    assert_eq!(parts(&r.scene)[0], p(NOW, "Checking your token…", None));
}

#[test]
fn the_card_watch_counts_from_add_credit_card() {
    let mut r = Run::new(gated(false, true));
    r.at(1_000).send(Event::Tick);
    assert_eq!(win(&r.scene), "gate-card");
    assert_eq!(parts(&r.scene), [], "no watch row until the billing page was opened");
    r.on(Action::AddCard);
    let clicked = r.t;
    assert_eq!(parts(&r.scene), [p(NOW, "Waiting for Cloudflare to confirm your card is in place…", None)]);
    r.tick_to(clicked + 24_000);
    assert_eq!(parts(&r.scene), [p(NOW, "Waiting for Cloudflare to confirm your card is in place…", Some("0:24"))]);
    assert_eq!(long(&r.scene), None);
    r.tick_to(clicked + 45_000);
    assert_eq!(long(&r.scene).as_deref(), Some("A new card can take Cloudflare a minute or so to count. Once it's added, I'll move on — hang tight!"));
    // The card counted: the gate clears, and the plan goes on.
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }));
    assert_eq!(win(&r.scene), "card");
}

#[test]
fn the_last_step_lists_wirings_parts_and_its_clock_survives_a_retry() {
    let mut r = Run::new(window(at_wiring()));
    r.at(1_000).send(Event::Tick);
    assert_eq!(win(&r.scene), "wiring");
    r.cloud(CloudNews::Part { op: Op::Wire, part: 1 });
    r.cloud(CloudNews::Part { op: Op::Wire, part: 2 });
    r.cloud(CloudNews::Part { op: Op::Wire, part: 3 });
    let name = r.t;
    r.tick_to(name + 48_000);
    assert_eq!(parts(&r.scene), [
        p(DONE, "Your private storage", None),
        p(DONE, "Your video page", None),
        p(NOW, "Switching your name on…", Some("0:48")),
        p(AHEAD, "A last check that it all works", None),
        p(AHEAD, "Adding this Mac to your space", None),
    ]);
    assert_eq!(long(&r.scene), None);
    let OnboardingBody::Plan(plan) = &r.scene.onboarding.as_ref().unwrap().body else { panic!() };
    let Nest::Calm { line, .. } = &plan.nest else { panic!() };
    assert_eq!(line, "Almost there — switching your name on takes a few minutes the first time, and only once.", "the ratified line stays");
    // The name's wait ran out: wiring tries again; the parts before it re-run quickly and
    // stay ticked, and the clock counts on from the first try.
    r.send(Event::Failed { op: Op::Wire, class: FailClass::Timeout });
    r.tick_to(name + 150_000);
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "wiring tries again");
    r.cloud(CloudNews::Part { op: Op::Wire, part: 1 });
    r.tick_to(name + 181_000);
    assert_eq!(parts(&r.scene)[..3], [
        p(DONE, "Your private storage", None),
        p(DONE, "Your video page", None),
        p(NOW, "Switching your name on…", Some("3:01")),
    ]);
    assert_eq!(long(&r.scene).as_deref(), Some("New names can take Cloudflare a while longer, sometimes ten minutes. It's fine to close this window — I'll finish by myself."));
    r.cloud(CloudNews::Part { op: Op::Wire, part: 4 });
    r.cloud(CloudNews::Part { op: Op::Wire, part: 5 });
    assert_eq!(parts(&r.scene)[2..], [
        p(DONE, "Your name is on", None),
        p(DONE, "Last check passed", None),
        p(NOW, "Adding this Mac to your space…", None),
    ]);
    assert_eq!(long(&r.scene), None, "the long line is the name's");
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert_eq!(win(&r.scene), "all-set");
    assert_eq!(r.w.sweep.wiring, None, "nothing of wiring's parts is kept once it lands");
}

#[test]
fn the_working_row_names_the_part_underway_with_its_clock() {
    let mut r = Run::new(window(named()));
    r.at(1_000).send(Event::Tick);
    assert_eq!(win(&r.scene), "done");
    assert_eq!(row(&r.scene, 6), "• Wiring it up (shroom is doing this part now)", "no part reported yet");
    assert_eq!(clock(&r.scene, 6), None);
    r.cloud(CloudNews::Part { op: Op::Wire, part: 3 });
    let name = r.t;
    r.tick_to(name + 48_000);
    assert_eq!(row(&r.scene, 6), "• Wiring it up (shroom is doing this part now · switching your name on)");
    assert_eq!(clock(&r.scene, 6).as_deref(), Some("0:48"));
    for n in 1..=5 {
        assert_eq!(clock(&r.scene, n), None, "row {n}: a clock on the spinning row only");
    }
}

#[test]
fn wiring_finishes_on_its_own_with_the_window_closed() {
    let mut r = Run::new(window(at_wiring()));
    r.at(1_000).send(Event::Tick);
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert_eq!(r.scene.onboarding, None);
    // The name's wait ran out with the window closed: wiring tries again all the same.
    r.send(Event::Failed { op: Op::Wire, class: FailClass::Timeout });
    r.tick_to(r.t + 150_000);
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "N74: wiring keeps trying with the window closed");
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r.w.provisioned());
    assert_eq!(r.scene.onboarding, None, "it lands without opening anything");
    assert_eq!(r.scene.popover, None);
    r.click();
    assert!(r.ids().contains(&"config"), "the next click on the mushroom: the popover, ready to record");
}

#[test]
fn after_a_quit_wiring_resumes_from_the_next_launch_without_a_window() {
    let mut r = Run::new(window(at_wiring()));
    r.at(1_000).send(Event::Tick);
    // The login item's silent launch: nothing opens, and the launch's own probe pass says
    // where the setup stands.
    let granted = QuickProbe { screen: true, mic: Access::Granted, camera: Access::Granted, ..quick() };
    let mut r2 = reboot(&r, LaunchKind::LoginItem, granted.clone(), None);
    assert_eq!(r2.scene.onboarding, None);
    assert!(r2.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "the launch re-probes the saved token");
    r2.cloud(swept(CLEAR, Some("myshroom"), false, None));
    assert!(r2.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into() })), "and wiring resumes, window closed");
    assert_eq!(r2.scene.onboarding, None);
    r2.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r2.w.provisioned());
    assert_eq!(r2.scene.onboarding, None, "it lands without opening anything (laws 5 and 11)");

    // A Mac stopped at the card gate: the launch's pass finds the gate, and nothing else
    // runs until the window is open — no poll, no wiring.
    let mut r = Run::new(window(gated(false, true)));
    r.at(1_000).send(Event::Tick);
    let mut r3 = reboot(&r, LaunchKind::LoginItem, granted, None);
    assert!(r3.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r3.cloud(swept(Gates { email: false, card: true }, None, false, None));
    r3.tick_to(r3.t + 30_000);
    assert!(!r3.has(|e| matches!(e, Effect::Cloud(_))), "no poll and no wiring with the window closed");
    assert_eq!(r3.scene.onboarding, None);
}
