//! The status seam's projection (ARCHITECTURE §11.2, DECISIONS B80; the retired spec text's
//! guided-agent overlay, SM:1636-1655): what a reader outside the app — the installer, a
//! guiding agent — learns about setup, permission, join and take state, walked along the
//! machine with the laws checked at every step. The projection is pure and adds no
//! behaviour; these walkthroughs pin its vocabulary to the states the spec names, and that
//! nothing it says carries a host, a name, a link or a code (law 13).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::derive::status::{Posture, posture};
use shroom_core::event::{Event, JoinAnswer, Key, MemberNews, OsNews, PeekAnswer, RecNews, StandingAnswer, Surface};
use shroom_core::scene::Action;
use shroom_core::time::{Fact, Mono};
use shroom_core::world::*;
use support::*;

const HOST: &str = "v.myshroom.workers.dev";
const MINE: &str = "v.mine.workers.dev";

fn member_identity() -> SpaceIdentity {
    SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), bucket: "shroom-space".into(), ..identity() }
}

fn own_mac_identity() -> SpaceIdentity {
    SpaceIdentity { role: Role::OwnerClass, member_id: "m1".into(), bucket: "shroom-space".into(), ..identity() }
}

/// A Mac with nothing on it yet: no space, nothing asked of macOS.
fn fresh() -> World {
    let mut w = ready();
    w.space.identity = None;
    w.space.token = false;
    w.mac.primed = None;
    w.mac.screen = Fact::known(false, Mono(0));
    w.mac.mic = Fact::known(Access::NotAsked, Mono(0));
    w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
    w
}

/// A Mac that set up its own space (`v.mine.workers.dev`, it keeps the token).
fn mine_owner() -> World {
    let mut w = ready();
    w.space.device_key = true;
    w.space.identity = Some(SpaceIdentity { account_id: "acct-mher".into(), subdomain: "mine".into(), host: MINE.into(), owner_name: "Mher".into(), ..identity() });
    w
}

fn member() -> World {
    let mut w = ready();
    w.space.identity = Some(member_identity());
    w.space.token = false;
    w.space.device_key = true;
    w
}

fn arrived(link: u32) -> Event {
    Event::Member(MemberNews::Arrived { link: SecretId(link), host: HOST.into(), remembered: false })
}

fn peeked(link: u32, peek: PeekAnswer) -> Event {
    Event::Member(MemberNews::Peeked { link: SecretId(link), peek })
}

fn pending(link: u32, kind: InviteKind) -> Event {
    peeked(link, PeekAnswer::Pending { host: HOST.into(), kind, inviter: "Bagrat".into(), current: true })
}

fn joined(link: u32, answer: JoinAnswer) -> Event {
    Event::Member(MemberNews::Joined { link: SecretId(link), answer })
}

fn on(action: Action) -> Event {
    Event::Act { surface: Surface::Onboarding, action, rev: 0 }
}

fn p(r: &Run) -> Posture {
    posture(&r.w)
}

/// The seam's words on this step: (setup, role, invite, invite_kind).
fn join(r: &Run) -> (&'static str, &'static str, &'static str, &'static str) {
    let p = p(r);
    (p.setup, p.role, p.invite, p.invite_kind)
}

/// Law 13 at the seam: its words never carry a host, a person's name or a link's code.
fn nothing_secret(r: &Run) {
    let j = serde_json::to_string(&p(r)).unwrap();
    for s in [HOST, MINE, "Bagrat", "Mher", "myshroom", "join/", "m_mher"] {
        assert!(!j.contains(s), "the seam says {s:?}: {j}");
    }
}

#[test]
fn a_member_joins_and_the_seam_names_each_join_state_then_the_chapter_then_the_take() {
    let mut r = Run::new(fresh());
    assert_eq!(join(&r), ("welcome", "none", "none", "none"));
    r.send(arrived(1));
    // Peek hasn't answered: the waiting posture (DEVIATIONS #20).
    assert_eq!(join(&r), ("join-waiting", "none", "join-waiting", "none"));
    r.send(pending(1, InviteKind::Member));
    assert_eq!(join(&r), ("member-welcome", "none", "member-welcome", "member"));
    nothing_secret(&r);
    r.key(Surface::Onboarding, Key::Return);
    assert_eq!(join(&r), ("joining", "none", "joining", "member"));
    nothing_secret(&r);
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    // `done · member`: row six ahead, the invite behind it.
    assert_eq!(join(&r), ("done", "member", "none", "none"));
    assert_eq!(p(&r).chapter, "ahead");
    assert_eq!(p(&r).standing, "fine");
    nothing_secret(&r);

    // Region 3's chapter: each step is the seam's `setup`, and the grants as the core heard them.
    r.key(Surface::Onboarding, Key::Return);
    assert_eq!(p(&r).setup, "perm-mic");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert_eq!(p(&r).setup, "perm-camera");
    let g = p(&r).grants.unwrap();
    assert_eq!((g.screen, g.mic, g.camera), ("not-granted", "granted", "not-asked"));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Denied }));
    assert_eq!(p(&r).setup, "perm-screen");
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Denied }));
    assert_eq!(p(&r).setup, "perm-priming");
    r.send(Event::Rec(RecNews::Primed(true)));
    assert_eq!((p(&r).setup, p(&r).chapter, p(&r).role), ("all-set", "done", "member"));

    // The take: where the agent has already left the stage — the installer's refusal reads it.
    r.key(Surface::Onboarding, Key::Return);
    r.key(Surface::Popover, Key::Return);
    assert_eq!((p(&r).take, p(&r).take_open), ("countdown", true));
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("d1".into()), rec: RecId("AbCdEfGhIjKlMnOp".into()) }));
    assert_eq!((p(&r).take, p(&r).take_open), ("recording", true));
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    assert_eq!(p(&r).take, "paused");
    r.act(Action::Stop);
    assert_eq!((p(&r).take, p(&r).take_open), ("stopping", true));
    r.send(Event::Rec(RecNews::Finalized(Ok(61_000))));
    assert_eq!((p(&r).take, p(&r).take_open), ("none", false));
    assert_eq!(p(&r).setup, "all-set");
}

#[test]
fn an_own_mac_link_is_its_own_kind_and_joins_owner_class() {
    let mut r = Run::new(fresh());
    r.send(arrived(1)).send(pending(1, InviteKind::OwnMac));
    assert_eq!(join(&r), ("member-welcome", "none", "member-welcome", "own-mac"));
    r.send(on(Action::Join));
    r.send(joined(1, JoinAnswer::Joined(own_mac_identity())));
    assert_eq!(join(&r), ("done", "owner-class", "none", "none"));
}

#[test]
fn a_switch_reads_its_confirm_then_the_new_role() {
    let mut r = Run::new(mine_owner());
    assert_eq!(join(&r), ("all-set", "token-holder", "none", "none"));
    r.send(arrived(1)).send(pending(1, InviteKind::Member));
    assert_eq!(join(&r), ("join-switch-confirm", "token-holder", "join-switch-confirm", "member"));
    nothing_secret(&r);
    r.send(on(Action::Join));
    assert_eq!(join(&r).2, "joining");
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    // A primed Mac goes straight to all-set · member (DEVIATIONS #23).
    assert_eq!(join(&r), ("all-set", "member", "none", "none"));
    nothing_secret(&r);
}

#[test]
fn the_refusals_and_the_waiting_posture_are_their_own_ids() {
    for (answer, id) in [
        (JoinAnswer::Expired, "join-expired"),
        (JoinAnswer::Used, "join-used"),
        (JoinAnswer::SpaceMissing, "join-space-missing"),
        (JoinAnswer::AlreadyHere, "join-already-here"),
    ] {
        let mut r = Run::new(fresh());
        r.send(arrived(1)).send(pending(1, InviteKind::Member)).send(on(Action::Join)).send(joined(1, answer));
        assert_eq!((p(&r).setup, p(&r).invite, p(&r).role), (id, id, "none"), "{id}");
        nothing_secret(&r);
    }
    // Dead at the peek: the same ids before any Join.
    let mut r = Run::new(fresh());
    r.send(arrived(1)).send(peeked(1, PeekAnswer::Used { host: HOST.into(), kind: InviteKind::Member, inviter: "Bagrat".into() }));
    assert_eq!(join(&r), ("join-used", "none", "join-used", "member"));
    let mut r = Run::new(fresh());
    r.send(arrived(1)).send(peeked(1, PeekAnswer::SpaceMissing { host: HOST.into() }));
    assert_eq!(join(&r), ("join-space-missing", "none", "join-space-missing", "none"));
}

#[test]
fn a_members_standing_reads_from_its_space() {
    let mut r = Run::new(member());
    assert_eq!(p(&r).standing, "fine");
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert_eq!((p(&r).role, p(&r).standing), ("member", "removed"));

    let mut r = Run::new(member());
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Gone)));
    assert_eq!(p(&r).standing, "fine", "one answer is not a face (C7)");
    r.t += 31_000;
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Gone)));
    assert_eq!(p(&r).standing, "gone");

    let mut r = Run::new(member());
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::SpaceSide)));
    r.t += 31_000;
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::SpaceSide)));
    assert_eq!(p(&r).standing, "needs-owner");

    let mut r = Run::new(member());
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Fine { current: false, only_holder: None, mark: None, ai: None })));
    assert_eq!(p(&r).standing, "needs-update");

    // The token's Mac repairs its space instead (N42): never a member standing.
    let mut r = Run::new(ready());
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert_eq!((p(&r).role, p(&r).standing), ("token-holder", "fine"));
}

/// The gap the seam had: `all-set` read as all-set whatever macOS said since. The chapter
/// stays done (row six is the warm-up's fact), and the grants say what was last heard.
#[test]
fn a_grant_switched_off_after_all_set_is_said_beside_it() {
    let mut r = Run::new(ready());
    assert_eq!((p(&r).setup, p(&r).chapter), ("all-set", "done"));
    assert_eq!(p(&r).grants.unwrap().screen, "granted");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Denied, camera: Access::Granted }));
    let g = p(&r).grants.unwrap();
    assert_eq!((p(&r).setup, g.screen, g.mic, g.camera), ("all-set", "not-granted", "denied", "granted"));
    // Before the first report the core knows nothing: the runner then says `unknown`.
    assert!(posture(&World::blank()).grants.is_none());
}

/// The projection follows onboarding's own derivation everywhere — it never stores a step.
#[test]
fn setup_is_onboardings_own_state_id() {
    for world in [World::blank(), fresh(), ready(), member(), mine_owner()] {
        assert_eq!(posture(&world).setup, shroom_core::derive::onboarding::at(&world).id());
    }
}
