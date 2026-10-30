//! The setup a coding agent drives with `shroom setup`, no window (spec N88): the wizard's
//! own states, run with its window shut. Every step the commands can stand on has its agent
//! line in the spec; an `--agent` launch opens no window; each ask is the click it names,
//! answered with where the setup stands; the token rides the stash like a paste; a click on
//! the mushroom opens the normal window at the current step; and the screen relaunch comes
//! back without one.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{CliAnswer, CloudCmd, Effect, PersistOp, Perm, PublicUrl, SetupReport, ShellCmd, SweepPart};
use shroom_core::event::{ClaimFail, CliOp, CloudNews, Disk, Event, OsNews, SetupAsk, Shape, SweepAnswer};
use shroom_core::time::Fact;
use shroom_core::world::*;
use support::*;

fn bagrat() -> Account {
    Account { id: "bagrat".into(), name: "Bagrat's Account".into() }
}

fn fresh_quick() -> QuickProbe {
    QuickProbe { screen: false, mic: Access::NotAsked, camera: Access::NotAsked, ..quick() }
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

trait Cloud {
    fn cloud(&mut self, news: CloudNews) -> &mut Self;
}

impl Cloud for Run {
    fn cloud(&mut self, news: CloudNews) -> &mut Self {
        self.send(Event::Cloud(news))
    }
}

/// The answer to the last ask.
fn answer(r: &Run) -> CliAnswer {
    r.effects
        .iter()
        .find_map(|e| match e {
            Effect::Cli(a) => Some(a.clone()),
            _ => None,
        })
        .expect("every ask is answered")
}

fn ask(r: &mut Run, a: SetupAsk) -> SetupReport {
    r.send(Event::Cli { id: 1, op: CliOp::Setup(a) });
    let got = answer(r);
    assert!(got.ok, "a setup ask answers where it stands: {got:?}");
    got.setup.expect("a setup ask's answer says where the setup stands")
}

/// No window, no Dock icon: the plan runs out of sight.
fn unseen(r: &Run) {
    assert!(r.scene.onboarding.is_none(), "an agent's setup shows no window");
    assert!(!r.scene.dock, "no window, no Dock icon");
}

fn swept(gates: Gates, subdomain: Option<&str>) -> CloudNews {
    CloudNews::Swept(SweepAnswer {
        token: Some(TokenVerdict::Valid { account: bagrat(), several: false }),
        gates: Some(gates),
        subdomain: Some(subdomain.map(String::from)),
        foreign_v: Some(false),
        label: Some(Some("v".into())),
        bucket: Some(false),
        worker: Some(WorkerFact::Absent),
        found: Some(None),
    })
}

const CLEAR: Gates = Gates { email: false, card: false };

#[test]
fn every_step_the_commands_stand_on_has_its_agent_line() {
    let spec = spec();
    for id in [
        "welcome", "account", "account-signing-up", "token", "token-garbled", "token-wrong-powers", "token-pick-account", "card", "gate-email", "gate-card",
        "name", "name-taken", "wiring", "space-found", "done", "perm-mic", "perm-camera", "perm-screen", "relaunching", "perm-priming", "all-set",
        "wizard-offline", "wizard-unexpected",
    ] {
        let s = spec.state(id).unwrap_or_else(|| panic!("{id}: not a state"));
        let line = s.agent.as_ref().unwrap_or_else(|| panic!("{id}: no agent line (N88)"));
        assert_eq!(line.id, format!("{id}.agent"));
        assert!(!line.text.is_empty());
    }
}

#[test]
fn an_agent_launch_opens_no_window() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Agent, &World::blank(), fresh_quick(), None))));
    assert_eq!(r.arm.edge, Some(("first-launch", "idle-unprovisioned")), "N88: the mushroom only");
    unseen(&r);
    assert!(r.ids().contains(&"idle-unprovisioned"));
}

#[test]
fn an_agent_drives_the_whole_setup_with_no_window() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Agent, &World::blank(), fresh_quick(), None))));

    // The first ask passes the welcome by (Set up my space), never registering the login
    // item: its box was never shown (N88).
    let s = ask(&mut r, SetupAsk::Status);
    assert_eq!(s.at, "account");
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(_)))), "never set silently");
    assert!(r.w.prefs.setup_chosen);
    unseen(&r);

    let s = ask(&mut r, SetupAsk::Account { new: true });
    assert_eq!((s.at, s.took), ("account-signing-up", true));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Signup))));
    let s = ask(&mut r, SetupAsk::Account { new: false });
    assert_eq!(s.at, "token", "I signed up");
    assert!(s.signed_up);

    let s = ask(&mut r, SetupAsk::TokenPage);
    assert!(s.took);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::TokenPage))));

    let s = ask(&mut r, SetupAsk::Token { id: SecretId(7), shape: Shape::Garbled });
    assert_eq!(s.at, "token-garbled");
    let s = ask(&mut r, SetupAsk::Token { id: SecretId(8), shape: Shape::Token });
    assert_eq!(s.at, "token");
    assert!(s.busy, "the paste is being checked");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Validate(SecretId(8)))), "the check runs with the window shut");
    r.cloud(CloudNews::Validated { id: SecretId(8), result: Ok(vec![bagrat()]) });
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SaveToken(SecretId(8), "bagrat".into()))));
    r.cloud(CloudNews::TokenSaved(SecretId(8)));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "the probe pass runs");
    let s = ask(&mut r, SetupAsk::Peek);
    assert_eq!((s.at, s.busy), ("card", true));

    r.cloud(swept(Gates { email: false, card: true }, None));
    let s = ask(&mut r, SetupAsk::Peek);
    assert_eq!(s.at, "gate-card");
    let s = ask(&mut r, SetupAsk::CardPage);
    assert!(s.took);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Billing { account_id: "bagrat".into() }))));
    // The watch polls with the window shut, as it does with it open.
    let mut polled = false;
    for _ in 0..20 {
        let wake = r.scene.next_wake.expect("the gate polls with the window shut");
        r.tick_to(wake.0);
        if r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates))) {
            polled = true;
            break;
        }
    }
    assert!(polled, "the card's watch polls");
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(CLEAR), ..Default::default() }));
    r.cloud(swept(CLEAR, None));
    let s = ask(&mut r, SetupAsk::Peek);
    assert_eq!(s.at, "name");

    let s = ask(&mut r, SetupAsk::Name("myshroom".into()));
    assert!(s.took);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Claim("myshroom".into()))));
    r.cloud(CloudNews::Claimed { name: "myshroom".into(), result: Err(ClaimFail::Taken) });
    assert_eq!(ask(&mut r, SetupAsk::Peek).at, "name-taken");
    ask(&mut r, SetupAsk::Name("myshroomhq".into()));
    r.cloud(CloudNews::Claimed { name: "myshroomhq".into(), result: Ok(()) });
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into(), fresh: true })), "wiring starts out of sight");
    let s = ask(&mut r, SetupAsk::Peek);
    assert_eq!(s.at, "done");
    assert_eq!(s.host.as_deref(), Some("v.myshroomhq.workers.dev"));

    // One permission per run: each asks the next, or says which prompt is up.
    let s = ask(&mut r, SetupAsk::Permissions);
    assert_eq!(s.at, "perm-mic");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Mic))));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Camera))), "the mic's answer chains the camera");
    let s = ask(&mut r, SetupAsk::Permissions);
    assert_eq!((s.at, s.took), ("perm-camera", false), "its prompt is up: nothing more is asked");
    // A denied camera never blocks.
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Denied }));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))));
    let s = ask(&mut r, SetupAsk::Permissions);
    assert_eq!(s.at, "perm-screen");
    unseen(&r);

    // macOS's own Quit & Reopen: the restore says the setup had no window.
    r.send(Event::Os(OsNews::Terminating { ours: false }));
    let restore = r
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::Persist(PersistOp::Restore(Some(x))) => Some(x.clone()),
            _ => None,
        })
        .expect("the chapter is remembered on the way out");
    assert!(restore.headless, "it comes back without a window");
    let d = disk(LaunchKind::User, &r.w, QuickProbe { screen: true, mic: Access::Granted, camera: Access::Denied, ..quick() }, Some(restore));
    let mut n = Run::new(World::blank());
    n.t = r.t + 1_000;
    n.send(Event::Booted(Box::new(d)));
    unseen(&n);
    assert!(n.w.session.chapter, "the chapter resumes");
}

#[test]
fn a_click_on_the_mushroom_opens_the_window_at_the_current_step() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Agent, &World::blank(), fresh_quick(), None))));
    ask(&mut r, SetupAsk::Status);
    ask(&mut r, SetupAsk::Account { new: false });
    unseen(&r);
    r.send(Event::TrayClicked { right: false, quick: fresh_quick() });
    assert_eq!(r.scene.onboarding.as_ref().map(|o| o.id), Some("token"), "the normal window, where the agent left off");
    assert!(r.scene.dock);
    // The commands go on driving it: the window follows.
    let s = ask(&mut r, SetupAsk::Token { id: SecretId(3), shape: Shape::Garbled });
    assert_eq!(s.at, "token-garbled");
    assert_eq!(r.scene.onboarding.as_ref().map(|o| o.id), Some("token-garbled"));
}

#[test]
fn a_step_asked_at_the_wrong_moment_takes_nothing_and_says_where_it_stands() {
    let mut r = Run::new(World::blank());
    r.send(Event::Booted(Box::new(disk(LaunchKind::Agent, &World::blank(), fresh_quick(), None))));
    let s = ask(&mut r, SetupAsk::Name("myshroom".into()));
    assert_eq!((s.at, s.took), ("account", false));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(_))));
    let s = ask(&mut r, SetupAsk::Permissions);
    assert_eq!((s.at, s.took), ("account", false));
}

#[test]
fn a_set_up_mac_answers_all_set_and_a_take_refuses() {
    let mut r = Run::new(ready());
    let s = ask(&mut r, SetupAsk::Status);
    assert_eq!(s.at, "all-set");
    assert_eq!(s.host.as_deref(), Some(r.w.space.identity.as_ref().unwrap().host.as_str()));
    unseen(&r);
    r.w.take = Some(take(Phase::Recording, 1_000));
    r.send(Event::Cli { id: 2, op: CliOp::Setup(SetupAsk::Status) });
    assert_eq!(answer(&r).error, Some("take_open"));
}

#[test]
fn an_unprovisioned_mac_without_the_launch_still_takes_the_commands() {
    // The app opened by hand, its welcome dismissed: the commands start the setup unseen.
    let mut w = ready();
    w.space.identity = None;
    w.space.token = false;
    w.mac.screen = Fact::known(false, shroom_core::time::Mono(0));
    let mut r = Run::new(w);
    let s = ask(&mut r, SetupAsk::Status);
    assert_eq!(s.at, "account");
    unseen(&r);
}
