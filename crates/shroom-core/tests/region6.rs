//! Region 6 — members, the join machine and member life (N28–N34) — and the member parts
//! Regions 2, 4 and 5 left for it (the Members tab, the Space tab's other kinds of Mac and
//! their exits, Mine · Everyone, the member banners, `stopped-stranded`), against the
//! fixtures: every in-scope state derives from a world built with the spec's example values
//! (Bagrat's space at `v.myshroom.workers.dev`, Mher, Ani, the MacBook Air), with the
//! fixture's exact Return default and its lines verbatim; the design's walkthroughs with
//! the laws checked at every step (law 14 — one space per Mac — and law 13's join codes
//! among them); and the machine's drawn edges, each realized by its events or listed with its reason.
//!
//! Every member-side fact comes from the space's Worker as `shroom-cloud::member` types it
//! (WORKER-INTERFACE §4), reaching the core as `Event::Member`. Worlds are built by feeding
//! those events, never by writing the Region 6 core's own facts: the tests pin the
//! interface and the faces, not the World's shape.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use shroom_core::effect::{Effect, MemberCmd, PersistOp, Perm, RecCmd, ShellCmd, TakeCmd, UpCmd};
use shroom_core::event::{
    Disk, Event, Field, JoinAnswer, Key, MemberNews, OsNews, PeekAnswer, RecNews, SecretField, Shape, StandingAnswer, Surface, UploadNews,
};
use shroom_core::scene::{Action, Button, Face, RowRef, Scene, Sheet, Style};
use shroom_core::time::{Envelope, Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{derive, update};
use shroom_spec::Spec;
use support::*;

// ── the spec's example values ──

const HOST: &str = "v.myshroom.workers.dev";
/// Another space, the one a switching Mac records to now (the spec's `v.mine.workers.dev`).
const MINE: &str = "v.mine.workers.dev";
const DAY: i64 = 86_400_000;
const JUL20: i64 = 1_784_548_800_000;
const AUG03: i64 = 1_785_758_400_000;
const AUG10: i64 = 1_786_363_200_000;
const AUG21: i64 = 1_787_313_600_000;
const AUG23: i64 = 1_787_486_400_000;
const SEP02: i64 = 1_788_350_400_000;
/// The wall clock `Run` stamps (support.rs): an event at `t` lands at this plus `t`.
const WALL0: i64 = 1_790_000_000_000;
/// C7's debounce (WORKER-INTERFACE §4.8, §6.7): a space-side face only after two answers
/// at least this far apart.
const STRIKE_GAP_MS: u64 = 30_000;

// The said-lines the walkthroughs read (verbatim from spec/machine.json).
const MW_TITLE: &str = "Bagrat invited you to record with shroom.";
const MW_TRUTH: &str =
    "shroom records your screen and hands you a link to share. Your videos will live in Bagrat's space, where Bagrat can see them too — nothing to sign up for, nothing to pay.";
const MW_STEP: &str = "One step before your first video: let macOS allow recording. About a minute, once.";
const N5_TRUTH: &str = "Everyone here records to your space — their videos live in your storage, and you can see and delete any of them.";
const ROW_SIX_AHEAD: &str = "• Letting macOS allow recording (microphone, camera and screen — a one-time yes each)";
const ROW_SIX_CURRENT: &str = "→ Letting macOS allow recording (microphone, camera and screen — a one-time yes each)";
const OFFLINE: &str = "You're offline — I'll finish joining the moment you're back.";
const NOT_ANSWERING: &str = "Bagrat's space isn't answering right now — I'll keep trying.";
const EXPIRED: &str = "This invite has expired — ask Bagrat for a fresh one.";
const USED: &str = "This invite was already used. If that wasn't you, ask Bagrat for a fresh one.";
const MISSING: &str = "This invite's space isn't there anymore — ask Bagrat about it.";
const NEW_LINK_DOOR: &str = "Have a new link? Paste it here.";
const REVOKED: &str = "Your access to Bagrat's space was turned off. Recordings on this Mac are safe — ask Bagrat if that's a surprise.";
const NEEDS_OWNER: &str = "Bagrat's space needs a fix on Bagrat's side. Your videos wait safely on this Mac and upload by themselves once it's done.";
const GONE: &str = "Bagrat's space isn't there anymore. Recordings on this Mac are safe — save any you want to keep.";
const SWITCH_LINE: &str =
    "This Mac records to v.mine.workers.dev now. Joining moves it to Bagrat's space — your videos in v.mine.workers.dev stay right where they are.";
const ONLY_HOLDER: &str = "You can come back to your own space anytime by setting it up again with a fresh token — everything will be there.";
const HALF_SET_UP: &str = "You've started setting up your own space. Joining Bagrat's instead sets that aside — your Cloudflare stays as it is.";
const MARK_LABEL: &str = "“made with shroom” on your pages";
const MARK_LINE: &str = "The small mark on every video page. You can flip it right on a page, too.";
/// The Space tab's paste door, retired (B159): no door shows.
const SPACE_PASTE_DOOR: &str = "Have an invite link? Paste it here.";
const AFTER_TAKE: &str = "You can do this once your recording is finished.";

// ── builders ──

fn id_as(role: Role, member: &str) -> SpaceIdentity {
    SpaceIdentity { role, member_id: member.into(), bucket: "shroom-space".into(), ..identity() }
}

/// Mher's Mac, joined to Bagrat's space (§4.5's 200).
fn member_identity() -> SpaceIdentity {
    id_as(Role::Member, "m_mher")
}

/// The owner's MacBook Air, joined through Add another Mac: member #1, owner-class (N33).
fn own_mac_identity() -> SpaceIdentity {
    id_as(Role::OwnerClass, "m1")
}

/// Mher's own space, set up on this Mac before Bagrat's invite arrived.
fn mine_identity() -> SpaceIdentity {
    SpaceIdentity { account_id: "acct-mher".into(), subdomain: "mine".into(), host: MINE.into(), owner_name: "Mher".into(), ..identity() }
}

/// The owner's setup Mac (the MacBook Pro): it keeps the token, and holds a device key
/// since wiring's enroll (§6.11).
fn owner() -> World {
    let mut w = ready();
    w.space.device_key = true;
    w
}

/// The owner's MacBook Air: owner-class, no token.
fn owner_class() -> World {
    let mut w = ready();
    w.space.identity = Some(own_mac_identity());
    w.space.token = false;
    w.space.device_key = true;
    w
}

/// Mher's Mac, a member: ready to record.
fn member() -> World {
    let mut w = ready();
    w.space.identity = Some(member_identity());
    w.space.token = false;
    w.space.device_key = true;
    w
}

/// A Mac that sets up its own space (it keeps that space's token) — then Bagrat invites it.
fn mine_owner() -> World {
    let mut w = owner();
    w.space.identity = Some(mine_identity());
    w
}

/// A Mac with nothing on it yet: no space, nothing asked of macOS, no window open.
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

fn with_popover(mut w: World) -> World {
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

fn settings_on(mut w: World, tab: SettingsTab) -> World {
    w.prefs.settings_tab = Some(tab);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn library_with(mut w: World, rows: Vec<LibraryRow>) -> World {
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(Arc::new(rows), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w
}

fn queued(dir: &str, id: &str, title: &str, blocked: Option<Blocked>) -> Queued {
    Queued { dir: TakeDir(dir.into()), rec: RecId(id.into()), title: title.into(), stopped: Wall(WALL0), done: 1, total: 4, blocked, length_ms: Some(48_000) }
}

fn uploading(n: usize) -> Vec<Queued> {
    (0..n).map(|i| queued(&format!("q{i}"), &format!("u{i}"), "Untitled — Sep 30", None)).collect()
}

fn stranded(n: usize) -> Vec<Queued> {
    (0..n).map(|i| queued(&format!("q{i}"), &format!("u{i}"), "Untitled — Sep 30", Some(Blocked::Space))).collect()
}

fn lib_row(id: &str, title: &str, at: i64, owner: &str) -> LibraryRow {
    LibraryRow { rec: RecId(id.into()), title: title.into(), created: Wall(at), length_ms: Some(95_000), live: true, owner: Some(owner.into()), poster: false, edited_ms: None, pinned: false }
}

/// Bagrat's video and Mher's, as the space lists them.
fn mixed_rows() -> Vec<LibraryRow> {
    vec![lib_row("r_mine", "Where the 502s come from", AUG21, "m1"), lib_row("r_mher", "Mher's onboarding notes", AUG23, "m_mher")]
}

fn ani_pending() -> PendingInvite {
    PendingInvite { id: "i_ani".into(), label: Some("Ani".into()), created: Wall(AUG21), expires: Wall(AUG21 + 7 * DAY), own_mac: false }
}

fn sona_expired() -> PendingInvite {
    PendingInvite { id: "i_sona".into(), label: Some("Sona".into()), created: Wall(AUG10), expires: Wall(AUG10 + 7 * DAY), own_mac: false }
}

/// An own-Mac link still in its hour: `GET /members` returns it tagged, and the face skips
/// it (§6.4, open question 16).
fn own_mac_pending() -> PendingInvite {
    PendingInvite { id: "i_mac".into(), label: None, created: Wall(WALL0), expires: Wall(WALL0 + 3_600_000), own_mac: true }
}

/// The roster as the setup Mac (the MacBook Pro, which keeps the token) reads it.
fn roster() -> Roster {
    Roster {
        macs: vec![
            RosterMac { id: "d_pro".into(), name: "MacBook Pro".into(), joined: Wall(JUL20), token: true, this_mac: true },
            RosterMac { id: "d_air".into(), name: "MacBook Air".into(), joined: Wall(SEP02), token: false, this_mac: false },
        ],
        members: vec![RosterMember { id: "m_mher".into(), label: "Mher".into(), joined: Wall(AUG03) }],
        invites: vec![sona_expired(), own_mac_pending()],
        labels: BTreeMap::from([("m_mher".to_string(), "Mher".to_string())]),
    }
}

/// The same roster, read on the MacBook Air.
fn roster_from_air() -> Roster {
    let mut r = roster();
    for m in &mut r.macs {
        m.this_mac = m.id == "d_air";
    }
    r
}

fn with_ani(mut r: Roster) -> Roster {
    r.invites.push(ani_pending());
    r
}

// ── events ──

fn news(n: MemberNews) -> Event {
    Event::Member(n)
}

/// A join link arrives — Open shroom's deep link, or the installer's file under `--invite`.
fn arrived(link: u32) -> Event {
    news(MemberNews::Arrived { link: SecretId(link), host: HOST.into(), remembered: false })
}

fn remembered(link: u32) -> Event {
    news(MemberNews::Arrived { link: SecretId(link), host: HOST.into(), remembered: true })
}

fn peeked(link: u32, peek: PeekAnswer) -> Event {
    news(MemberNews::Peeked { link: SecretId(link), peek })
}

/// Peek: a live invite from Bagrat, the page current.
fn pending(link: u32, kind: InviteKind) -> Event {
    peeked(link, PeekAnswer::Pending { host: HOST.into(), kind, inviter: "Bagrat".into(), current: true })
}

fn joined(link: u32, answer: JoinAnswer) -> Event {
    news(MemberNews::Joined { link: SecretId(link), answer })
}

fn standing(a: StandingAnswer) -> Event {
    news(MemberNews::Standing(a))
}

fn fine() -> StandingAnswer {
    StandingAnswer::Fine { current: true, only_holder: None, mark: None, ai: None }
}

fn owner_facts(only_holder: bool, mark: bool) -> StandingAnswer {
    StandingAnswer::Fine { current: true, only_holder: Some(only_holder), mark: Some(mark), ai: None }
}

/// Invite answered 201 for Ani: her row, and her new link in the stash under 7.
fn invited_ani() -> Event {
    let invite = PendingInvite { id: "i_ani".into(), label: Some("Ani".into()), created: Wall(WALL0), expires: Wall(WALL0 + 7 * DAY), own_mac: false };
    news(MemberNews::Invited { invite, link: SecretId(7) })
}

/// Add another Mac answered 201: the own-Mac link, in the stash under 8.
fn invited_mac() -> Event {
    let invite = PendingInvite { id: "i_mac2".into(), label: None, created: Wall(WALL0), expires: Wall(WALL0 + 3_600_000), own_mac: true };
    news(MemberNews::Invited { invite, link: SecretId(8) })
}

fn at(surface: Surface, action: Action) -> Event {
    Event::Act { surface, action, rev: 0 }
}

fn on(action: Action) -> Event {
    at(Surface::Onboarding, action)
}

fn set(action: Action) -> Event {
    at(Surface::Settings, action)
}

fn pop(action: Action) -> Event {
    at(Surface::Popover, action)
}

fn lib(action: Action) -> Event {
    at(Surface::Library, action)
}

fn pasted_link(id: u32) -> Event {
    Event::SecretPasted { field: SecretField::Invite, id: SecretId(id), shape: Shape::JoinLink }
}

/// A World after these events, stamped as `Run` stamps them. The builders feed through
/// `update` alone so the inventory can list every state it misses; the walkthroughs run
/// the same events through `Run`, the laws checked at every step.
fn feed(mut w: World, events: impl IntoIterator<Item = Event>) -> World {
    let mut t = w.now.0;
    for event in events {
        t += 10;
        update(&mut w, Envelope { mono: Mono(t), wall: Wall(WALL0 + t as i64), event });
    }
    w
}

/// Two answers of one kind, more than C7's 30 seconds apart.
fn struck(w: World, a: StandingAnswer) -> World {
    let w = feed(w, [standing(a.clone())]);
    let mut later = w.clone();
    later.now = w.now.plus(STRIKE_GAP_MS + 1_000);
    feed(later, [standing(a)])
}

// ── worlds along the join machine ──

fn welcomed(kind: InviteKind) -> World {
    feed(fresh(), [arrived(1), pending(1, kind)])
}

fn joining(kind: InviteKind) -> World {
    feed(welcomed(kind), [on(Action::Join)])
}

fn joined_as(kind: InviteKind) -> World {
    let id = match kind {
        InviteKind::Member => member_identity(),
        InviteKind::OwnMac => own_mac_identity(),
    };
    feed(joining(kind), [joined(1, JoinAnswer::Joined(id))])
}

/// Joined, and this Mac's permissions chapter done.
fn all_set_as(kind: InviteKind) -> World {
    let mut w = joined_as(kind);
    w.mac.primed = Some(Wall(WALL0));
    w.mac.screen = Fact::known(true, w.now);
    w.mac.mic = Fact::known(Access::Granted, w.now);
    w
}

fn switching(mut w: World) -> World {
    w = feed(w, [arrived(1), pending(1, InviteKind::Member)]);
    w
}

fn members_face(w: World, r: Roster) -> World {
    feed(settings_on(w, SettingsTab::Members), [news(MemberNews::Roster(r))])
}

/// Ani invited just now: her row, her link under it.
fn born(w: World) -> World {
    feed(w, [Event::Field { field: Field::InviteName, value: "Ani".into() }, set(Action::Invite), invited_ani()])
}

/// A member Mac that heard “removed”, with these takes waiting.
fn revoked(takes: Vec<Queued>) -> World {
    let mut w = with_popover(member());
    w.queue = takes;
    feed(w, [standing(StandingAnswer::Revoked)])
}

fn stranded_stop(cause: StandingAnswer) -> World {
    let mut w = with_popover(member());
    w.take = Some(take(Phase::Paused(Cause::User), 30_000));
    let w = match cause {
        StandingAnswer::Revoked => feed(w, [standing(StandingAnswer::Revoked)]),
        other => struck(w, other),
    };
    feed(w, [pop(Action::Stop), Event::Rec(RecNews::Finalized(Ok(30_000)))])
}

/// The owner's MacBook Air stops a take while its space answers as gone.
fn owner_class_stranded_stop() -> World {
    let mut w = with_popover(owner_class());
    w.take = Some(take(Phase::Paused(Cause::User), 30_000));
    let w = struck(w, StandingAnswer::Gone);
    feed(w, [pop(Action::Stop), Event::Rec(RecNews::Finalized(Ok(30_000)))])
}

/// Mher's Mac as a member of another space (`v.mine.workers.dev`), before Bagrat's invite.
fn mine_member() -> World {
    let mut w = member();
    w.space.identity = Some(SpaceIdentity { role: Role::Member, member_id: "m_x".into(), ..mine_identity() });
    w
}

// ── reading the Scene ──

fn json(s: &Scene) -> String {
    serde_json::to_string(s).unwrap()
}

/// A line as the Scene's JSON carries it.
fn enc(line: &str) -> String {
    let e = serde_json::to_string(line).unwrap();
    e[1..e.len() - 1].to_string()
}

fn says(s: &Scene, line: &str) -> bool {
    json(s).contains(&enc(line))
}

fn win(s: &Scene) -> Option<&'static str> {
    s.onboarding.as_ref().map(|o| o.id)
}

fn settings_faces(s: &Scene) -> Vec<&'static str> {
    s.settings.as_ref().map(|w| w.faces.clone()).unwrap_or_default()
}

fn banner(s: &Scene) -> Option<&'static str> {
    s.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| b.id)
}

fn buttons(s: &Scene, surface: Surface) -> Vec<Button> {
    match surface {
        Surface::Onboarding => s.onboarding.as_ref().map(|o| o.buttons().into_iter().cloned().collect()),
        Surface::Settings => s.settings.as_ref().map(|o| o.buttons().into_iter().cloned().collect()),
        Surface::Library => s.library.as_ref().map(|o| o.buttons().into_iter().cloned().collect()),
        Surface::Popover => s.popover.as_ref().map(|o| o.buttons().into_iter().cloned().collect()),
        _ => None,
    }
    .unwrap_or_default()
}

fn labels(s: &Scene, surface: Surface) -> Vec<(String, Style)> {
    buttons(s, surface).into_iter().map(|b| (b.label, b.style)).collect()
}

fn button(s: &Scene, surface: Surface, action: &Action) -> Button {
    buttons(s, surface).into_iter().find(|b| &b.action == action).unwrap_or_else(|| panic!("no {action:?} on {surface:?}: {:?}", s.state_ids()))
}

fn sheet(s: &Scene) -> Option<&Sheet> {
    s.settings.as_ref().and_then(|w| w.body.sheet.as_ref()).or_else(|| s.library.as_ref().and_then(|w| w.body.sheet.as_ref()))
}

fn start(s: &Scene) -> Button {
    button(s, Surface::Popover, &Action::Start)
}

fn titles(s: &Scene) -> Vec<String> {
    s.library.as_ref().expect("the Library window").body.rows.iter().filter_map(|r| r.title.clone()).collect()
}

fn s(x: &str) -> String {
    x.to_string()
}

trait Drive {
    fn on(&mut self, action: Action) -> &mut Self;
    fn set(&mut self, action: Action) -> &mut Self;
    fn lib(&mut self, action: Action) -> &mut Self;
    fn hear(&mut self, n: MemberNews) -> &mut Self;
    fn field(&mut self, field: Field, value: &str) -> &mut Self;
    /// Ticks to each next wake until `pred` holds of an effect, within `ms` of now.
    fn until(&mut self, ms: u64, pred: impl Fn(&Effect) -> bool) -> bool;
}

impl Drive for Run {
    fn on(&mut self, action: Action) -> &mut Self {
        self.send(on(action))
    }
    fn set(&mut self, action: Action) -> &mut Self {
        self.send(set(action))
    }
    fn lib(&mut self, action: Action) -> &mut Self {
        self.send(lib(action))
    }
    fn hear(&mut self, n: MemberNews) -> &mut Self {
        self.send(Event::Member(n))
    }
    fn field(&mut self, field: Field, value: &str) -> &mut Self {
        self.send(Event::Field { field, value: value.into() })
    }
    fn until(&mut self, ms: u64, pred: impl Fn(&Effect) -> bool) -> bool {
        let end = self.t + ms;
        for _ in 0..20 {
            if self.has(&pred) {
                return true;
            }
            match self.scene.next_wake {
                Some(t) if t.0 <= end => {
                    self.tick_to(t.0.max(self.t + 20));
                }
                _ => return false,
            }
        }
        self.has(&pred)
    }
}

fn disk(launch: LaunchKind, w: &World) -> Disk {
    Disk {
        launch,
        version: String::new(),
        prefs: w.prefs.clone(),
        once: w.once.clone(),
        space: w.space.identity.clone(),
        token: w.space.token,
        device_key: w.space.device_key,
        primed: w.mac.primed,
        restore: None,
        quick: QuickProbe { screen: false, mic: Access::NotAsked, camera: Access::NotAsked, ..quick() },
    }
}

/// A new process: it boots from what's on disk.
fn boot(launch: LaunchKind, w: &World) -> Run {
    let mut r = Run::new(World::blank());
    r.t = 2_000;
    r.send(Event::Booted(Box::new(disk(launch, w))));
    r
}

// ── the inventory: one row (or more) per in-scope state ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    /// The fixture's Return and Esc, and every said-line.
    Full,
    /// Lines only: a world for this state's Region 6 lines (or a face whose keys differ).
    Lines,
}

fn rows() -> Vec<(&'static str, World, Check)> {
    use Check::*;
    use InviteKind::{Member as M, OwnMac as O};
    let mut rows: Vec<(&'static str, World, Check)> = Vec::new();

    // ── the Members face (N28, N33) ──
    let members = || members_face(owner(), roster());
    rows.push(("invite-born", members(), Full));
    rows.push(("invite-born", born(members()), Full));
    rows.push(("invite-born", feed(born(members()), [set(Action::CopyInvite(SecretId(7)))]), Full));
    rows.push(("invite-born", {
        // Later: Ani's row ages; Sona's expired one offers [ New link ].
        let mut w = members_face(owner(), with_ani(roster()));
        w.wall = Wall(AUG23);
        w
    }, Full));
    // Read on the MacBook Air: the Pro keeps the token — its fact, and no Remove….
    rows.push(("invite-born", members_face(owner_class(), roster_from_air()), Full));
    rows.push(("settings-members", members(), Full));
    rows.push(("add-another-mac", feed(members(), [set(Action::AddMac), invited_mac()]), Full));
    rows.push(("member-remove-confirm", feed(members(), [set(Action::Remove(RowRef::Member(s("m_mher"))))]), Full));
    rows.push(("member-remove-confirm-member", feed(members(), [set(Action::Remove(RowRef::Member(s("m_mher"))))]), Full));
    rows.push((
        "member-remove-confirm-invite",
        feed(members_face(owner(), with_ani(roster())), [set(Action::Remove(RowRef::Invite(s("i_ani"))))]),
        Full,
    ));
    rows.push(("member-remove-confirm-own-mac", feed(members(), [set(Action::Remove(RowRef::Mac(s("d_air"))))]), Full));

    // ── the join machine (N29–N31) ──
    rows.push(("member-welcome", welcomed(M), Full));
    rows.push(("member-welcome", {
        // The welcome's paste door lands on the same window (N30).
        let mut w = fresh();
        w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
        feed(w, [pasted_link(2), pending(2, M)])
    }, Full));
    rows.push(("member-welcome-own-mac", welcomed(O), Full));
    rows.push(("joining", joining(M), Full));
    rows.push(("joining-own-mac", joining(O), Full));
    rows.push(("member-done", joined_as(M), Full));
    rows.push(("member-done", joined_as(O), Lines));
    rows.push(("member-all-set", all_set_as(M), Full));
    rows.push(("member-all-set-own-mac", all_set_as(O), Full));
    rows.push(("join-waiting", feed(joining(M), [Event::Os(OsNews::NetPath(false)), Event::Failed { op: Op::Join, class: FailClass::Transport }]), Full));
    rows.push(("join-waiting", feed(joining(M), [joined(1, JoinAnswer::Waiting)]), Full));
    rows.push(("join-expired", feed(joining(M), [joined(1, JoinAnswer::Expired)]), Full));
    rows.push(("join-used", feed(joining(M), [joined(1, JoinAnswer::Used)]), Full));
    rows.push(("join-space-missing", feed(joining(M), [joined(1, JoinAnswer::SpaceMissing)]), Full));
    rows.push(("join-already-here", feed(member(), [arrived(1), pending(1, M), standing(fine())]), Full));
    // Its wording follows this Mac's role (§6.2, U5): the owner's own Mac reads “your space”.
    rows.push(("join-already-here", feed(owner_class(), [arrived(1), pending(1, O), standing(fine())]), Full));
    rows.push(("join-switch-confirm", switching(mine_owner()), Full));
    rows.push(("join-switch-confirm", {
        // A half-set-up space counts: a token saved, nothing wired yet.
        let mut w = fresh();
        w.space.token = true;
        switching(w)
    }, Full));
    rows.push(("join-switch-confirm", {
        // The window was up before the take began: Join greys with law 4's line.
        let mut w = switching(mine_owner());
        let mut t = take(Phase::Paused(Cause::User), 30_000);
        t.opened = w.now.plus(1);
        w.take = Some(t);
        w
    }, Lines));
    rows.push(("join-switch-confirm-uploads", {
        let mut w = mine_owner();
        w.queue = uploading(2);
        switching(w)
    }, Full));
    rows.push(("join-switch-confirm-saved-take", {
        let mut w = mine_owner();
        w.saved = vec![saved(SavedCause::Interrupted, 272_000)];
        switching(w)
    }, Full));

    // ── member life (N34) ──
    rows.push(("member-revoked", revoked(uploading(1)), Full));
    rows.push(("member-revoked", revoked(uploading(3)), Full));
    rows.push(("member-revoked", feed(revoked(uploading(1)), [Event::Rec(RecNews::SavedToMovies(TakeDir(s("q0"))))]), Full));
    rows.push(("space-needs-owner", struck(with_popover(member()), StandingAnswer::SpaceSide), Full));
    rows.push(("space-gone", {
        let mut w = with_popover(member());
        w.queue = uploading(1);
        struck(w, StandingAnswer::Gone)
    }, Full));
    rows.push(("space-gone", {
        let mut w = with_popover(member());
        w.queue = uploading(3);
        struck(w, StandingAnswer::Gone)
    }, Full));
    rows.push(("space-gone", {
        let mut w = with_popover(member());
        w.queue = uploading(1);
        feed(struck(w, StandingAnswer::Gone), [Event::Rec(RecNews::SavedToMovies(TakeDir(s("q0"))))])
    }, Full));
    rows.push(("stopped-stranded", stranded_stop(StandingAnswer::Revoked), Full));
    rows.push(("stopped-stranded", stranded_stop(StandingAnswer::Gone), Full));
    // Owners also get [ Reconnect your space ] (r4): an owner-class Mac whose space is gone.
    rows.push(("stopped-stranded", owner_class_stranded_stop(), Full));

    // ── Region 5's member parts ──
    rows.push(("settings-space-no-token", feed(settings_on(owner_class(), SettingsTab::Space), [standing(owner_facts(false, true))]), Full));
    rows.push(("settings-space-member", feed(settings_on(member(), SettingsTab::Space), [standing(fine())]), Full));
    rows.push(("leave-confirm", {
        let mut w = settings_on(member(), SettingsTab::Space);
        w.queue = stranded(2);
        feed(w, [standing(fine()), set(Action::Disconnect)])
    }, Full));
    rows.push(("disconnect-confirm-no-token", {
        let mut w = settings_on(owner_class(), SettingsTab::Space);
        w.queue = uploading(2);
        feed(w, [standing(owner_facts(false, true)), set(Action::Disconnect)])
    }, Full));
    rows.push(("library-unreachable", {
        let mut w = library_with(member(), mixed_rows());
        w.queue = uploading(1);
        feed(w, [standing(StandingAnswer::Revoked)])
    }, Full));
    rows.push(("delete-confirm-other-member", {
        let w = library_with(owner(), mixed_rows());
        feed(w, [news(MemberNews::Roster(roster())), lib(Action::SetScope(LibraryScope::Everyone)), lib(Action::DeleteRow(RecId(s("r_mher"))))])
    }, Full));
    rows.push(("space-needs-update-member", feed(with_popover(member()), [standing(StandingAnswer::Fine { current: false, only_holder: None, mark: None, ai: None })]), Full));
    rows.push((
        "space-needs-update-no-token",
        feed(with_popover(owner_class()), [standing(StandingAnswer::Fine { current: false, only_holder: Some(false), mark: Some(true), ai: None })]),
        Full,
    ));
    rows.push(("space-degraded-no-token-mac", struck(with_popover(owner_class()), StandingAnswer::SpaceSide), Full));
    // DEVIATIONS #21: an owner-class Mac its space removed, a take waiting.
    rows.push(("owner-mac-removed", {
        let mut w = with_popover(owner_class());
        w.queue = uploading(1);
        struck(w, StandingAnswer::Revoked)
    }, Full));
    // Region 6's lines in Region 5's own states (each checked against LINES_HERE only).
    rows.push(("library", feed(library_with(owner(), mixed_rows()), [news(MemberNews::Roster(roster()))]), Lines));
    rows.push(("settings", settings_on(owner(), SettingsTab::General), Lines));
    rows.push(("settings-space", feed(settings_on(owner(), SettingsTab::Space), [standing(owner_facts(true, true))]), Lines));
    rows.push(("settings-space", feed(settings_on(member(), SettingsTab::Space), [standing(fine())]), Lines));
    rows
}

/// Region 5's states whose Region 6 lines this file owns: only these lines are checked
/// there (Region 5's suite checks the rest).
const LINES_HERE: &[(&str, &str)] = &[
    ("library", "Mine"),
    ("library", "Everyone"),
    ("settings", "Members"),
    ("settings-space", MARK_LABEL),
    ("settings-space", MARK_LINE),
    ("settings-space", "Leave this space…"),
];

/// Lines of in-scope states that aren't in the Scene as written, with the reason.
const LINES_ELSEWHERE: &[(&str, &str, &str)] = &[
    ("invite-born", "v.myshroom.workers.dev/join/7xk2pm", "the link pill: the Scene carries its host part and the link's stash handle, the code is drawn from the stash (law 13) — an_owner_invites_from_members_… and join_codes_never_reach_a_scene"),
    ("add-another-mac", "v.myshroom.workers.dev/join/q8w3zt", "the own-Mac link pill, as above"),
    ("joining-own-mac", "✓ Your space — v.myshroom.workers.dev", "the row's tick once joined (the fixture's own note) — not while joining: it shows on done · member's own-Mac face (add_another_mac_joins_…)"),
    ("invite-born", "You · MacBook Air — joined Sep 2 · Remove…", "composed: the sub-row's line “You · MacBook Air — joined Sep 2” and its Remove… button — an_owner_invites_from_members_…"),
    ("member-revoked", "shroom recording — Aug 21", "the saved file's name in ~/Movies, never on screen: named by the core (`derive::members::movies_name`, `copy::movies_untitled`), made by the takes lane — rescue.rs's save_to_movies_… and the `take movies` drive (B54)"),
    ("space-gone", "shroom recording — Aug 21", "the saved file's name in ~/Movies, never on screen: named by the core (`derive::members::movies_name`, `copy::movies_untitled`), made by the takes lane — rescue.rs's save_to_movies_… and the `take movies` drive (B54)"),
];

/// The fixture's Return default, as an Action.
fn action_for(label: &str) -> Action {
    match label {
        "Invite" => Action::Invite,
        "Join" | "Join Bagrat's space" => Action::Join,
        "Allow recording" => Action::AllowRecording,
        "Record now" => Action::RecordNow,
        "Start recording" => Action::Start,
        "Save to Movies" => Action::SaveToMovies,
        other => panic!("no mapping for the Return default {other:?}"),
    }
}

fn surface_of(id: &str) -> Surface {
    if id.starts_with("library") {
        Surface::Library
    } else if id.starts_with("delete-confirm") {
        Surface::Library
    } else if id.starts_with("settings") || id.starts_with("member-remove-confirm") || ["invite-born", "add-another-mac", "leave-confirm", "disconnect-confirm-no-token"].contains(&id) {
        Surface::Settings
    } else if ["member-revoked", "space-needs-owner", "space-gone", "stopped-stranded", "space-degraded-no-token-mac", "owner-mac-removed"].contains(&id) || id.starts_with("space-needs-update") {
        Surface::Popover
    } else {
        Surface::Onboarding
    }
}

/// States whose own Return isn't theirs to state: a nest whose window's Return is the
/// composer's (Add another Mac) and a tab face with no actions of its own.
const STRUCTURAL: &[&str] = &["add-another-mac", "settings-members"];

#[test]
fn every_region6_state_derives_with_its_return_default_and_its_lines() {
    let spec = Spec::load();
    let by_id = spec.by_id();
    let mut failures = Vec::new();
    let mut seen: HashMap<&str, String> = HashMap::new();
    for (id, w, check) in rows() {
        let scene = derive(&w);
        let ids = scene.state_ids();
        // Law 15's lint over every string the row's Scene shows (TESTING-PLAN §2.4).
        failures.extend(laws::voice_failures(&scene, id));
        if !ids.contains(&id) {
            failures.push(format!("{id}: not derived (got {ids:?})"));
            continue;
        }
        let fixture = by_id[id].iter().find(|s| !s.said_lines.is_empty() || !s.actions.is_empty()).unwrap_or(&by_id[id][0]);
        let surface = surface_of(id);
        if check == Check::Full && !STRUCTURAL.contains(&id) {
            let want = if fixture.kind == "banner" && !fixture.actions.iter().any(|a| a.label == "Start recording") {
                // A banner's own buttons are never the Return default (law 8): config's Start
                // stays it.
                Some(Action::Start)
            } else {
                fixture.return_default().map(|a| action_for(&a.label))
            };
            let got = scene.ret(surface);
            if got != want {
                failures.push(format!("{id}: Return is {got:?}, the fixture says {want:?}"));
            }
            let is_sheet = fixture.kind == "sheet" || fixture.parent.as_deref().is_some_and(|p| spec.state(p).is_some_and(|s| s.kind == "sheet") || p == "member-remove-confirm");
            let want_esc = if is_sheet {
                Some(Action::Back)
            } else if surface == Surface::Popover {
                Some(Action::Dismiss)
            } else {
                None
            };
            let esc = scene.esc(surface);
            if esc != want_esc {
                failures.push(format!("{id}: Esc is {esc:?}, want {want_esc:?}"));
            }
        }
        seen.entry(id).or_default().push_str(&json(&scene));
    }
    for (id, haystack) in &seen {
        let only: Vec<&str> = LINES_HERE.iter().filter(|(s, _)| s == id).map(|(_, l)| *l).collect();
        for fixture in by_id[id].iter().filter(|_| only.is_empty()) {
            for l in &fixture.said_lines {
                // A line that varies by case (the copy table's cases, an example's values) is
                // one world's, not every world's.
                let varies = l.note.as_deref().is_some_and(|n| n.contains("example") || n.contains("varies"));
                if varies || LINES_ELSEWHERE.iter().any(|(s, t, _)| s == id && *t == l.text) {
                    continue;
                }
                if !haystack.contains(&enc(&l.text)) {
                    failures.push(format!("{id}: {} line missing: {:?}", l.copy_status, l.text));
                }
            }
        }
        for line in &only {
            if !haystack.contains(&enc(line)) {
                failures.push(format!("{id}: Region 6 line missing: {line:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// The states the Worker, the installer or the agent render — never the app's.
const SERVED_ELSEWHERE: &[(&str, &str)] = &[
    ("join-landing", "the space's Worker serves the join page (worker/test/region6.test.js)"),
    ("join-landing-owner-view", "the join page's face for the owner's cookie — the Worker's"),
    ("join-landing-elsewhere", "the join page on something that isn't a Mac — the Worker's"),
    ("join-landing-own-mac", "the own-Mac link's page — the Worker's"),
    ("join-expired-page", "the join page's expired face — the Worker's"),
    ("join-used-page", "the join page's used face — the Worker's"),
    ("join-installing", "the install script (installer/test): it hands the link over by file with `--invite` (B29 amended); the app's half is `Arrived`"),
    ("owner-page-member", "the owner page for a member's cookie — the Worker's"),
    ("waiting-space", "the page's “Waiting for Bagrat's space” — the Worker reads the health the Mac reports (§4.18)"),
];

/// Region 5's and Region 2's member parts, left for Region 6 (tests/region5.rs's
/// STATES_OUT_OF_SCOPE), and Region 4's app-side stranded stop.
const DEFERRED: &[&str] = &[
    "settings-members",
    "settings-space-no-token",
    "settings-space-member",
    "leave-confirm",
    "disconnect-confirm-no-token",
    "library-unreachable",
    "delete-confirm-other-member",
    "space-needs-update-member",
    "space-needs-update-no-token",
    "space-degraded-no-token-mac",
    "stopped-stranded",
    "owner-page-member",
    "waiting-space",
];

#[test]
fn every_region6_state_is_built_or_served_elsewhere() {
    let spec = Spec::load();
    let wanted: BTreeSet<&str> = spec.region(6).map(|s| s.id.as_str()).chain(DEFERRED.iter().copied()).collect();
    let built: BTreeSet<&str> = rows().into_iter().filter(|(_, _, c)| *c == Check::Full).map(|(id, _, _)| id).collect();
    let elsewhere: BTreeSet<&str> = SERVED_ELSEWHERE.iter().map(|(id, _)| *id).collect();
    assert!(built.is_disjoint(&elsewhere), "both built and served elsewhere: {:?}", built.intersection(&elsewhere).collect::<Vec<_>>());
    let missing: Vec<_> = wanted.iter().filter(|id| !built.contains(*id) && !elsewhere.contains(*id)).collect();
    assert!(missing.is_empty(), "neither built nor served elsewhere: {missing:?}");
    let stale: Vec<_> = elsewhere.iter().chain(&built).filter(|id| !wanted.contains(*id)).collect();
    assert!(stale.is_empty(), "listed states the scope doesn't have: {stale:?}");
}

// ── the owner's side: Members (N28, N32, N33) ──

#[test]
fn an_owner_invites_from_members_and_the_born_link_copies_only_on_its_click() {
    let mut r = Run::new(with_popover(owner()));
    r.act(Action::OpenSettings);
    let tabs: Vec<String> = r.scene.settings.as_ref().unwrap().body.tabs.iter().map(|t| t.label.clone()).collect();
    assert_eq!(tabs, ["General", "Videos", "Space", "Members"], "Members, owner-class Macs only (N36); Videos before Space (N79)");
    r.set(Action::SettingsTab(SettingsTab::Members));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Roster)), "every entry reads the rows (law 1)");
    r.hear(MemberNews::Roster(roster()));
    let f = settings_faces(&r.scene);
    assert!(f.contains(&"settings-members") && f.contains(&"invite-born"), "{f:?}");
    for line in ["Members", N5_TRUTH, "You", "Add another Mac", "Mher — joined Aug 3", "You · MacBook Air — joined Sep 2", "You'll get a link to send them — over any chat."] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert!(!says(&r.scene, "MacBook Pro"), "this Mac is You: it carries no fact (N28) and no sub-row");
    // Remove… on every row but You — and never on this Mac (that's Disconnect) or a Mac with the token.
    let remove = |r: &Run, row: RowRef| r.scene.offers(Surface::Settings, &Action::Remove(row));
    assert!(remove(&r, RowRef::Member(s("m_mher"))));
    assert!(remove(&r, RowRef::Mac(s("d_air"))));
    assert!(!remove(&r, RowRef::Mac(s("d_pro"))));
    assert!(!remove(&r, RowRef::Invite(s("i_mac"))), "a pending own-Mac link has no row (§6.4)");
    assert!(r.scene.offers(Surface::Settings, &Action::AddMac), "quiet Frost text beside You");
    assert_eq!(button(&r.scene, Surface::Settings, &Action::AddMac).style, Style::Link);

    // The composer is the ask: Return in “Who's this for?” ≡ Invite (N28).
    assert!(says(&r.scene, "Who's this for?"));
    assert_eq!(r.scene.ret(Surface::Settings), Some(Action::Invite));
    r.field(Field::InviteName, "Ani");
    r.key(Surface::Settings, Key::Return);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Invite { label: s("Ani") })), "{:?}", r.effects);
    r.hear(match invited_ani() {
        Event::Member(n) => n,
        _ => unreachable!(),
    });
    // The born state, under the new row.
    for line in ["Ani — invited just now", "Send Ani this link — it's good for one person, for seven days.", "v.myshroom.workers.dev/join/"] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    let copy = button(&r.scene, Surface::Settings, &Action::CopyInvite(SecretId(7)));
    assert_eq!((copy.label.as_str(), copy.style), ("Copy link", Style::Filled), "the moment's one action");
    assert_eq!(button(&r.scene, Surface::Settings, &Action::Invite).style, Style::Plain, "[ Invite ] drops to outline while a born link shows");
    assert!(!r.has(|e| matches!(e, Effect::Shell(_))), "nothing is auto-copied (law 10)");
    r.set(Action::CopyInvite(SecretId(7)));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::CopyInvite(SecretId(7)))));
    assert_eq!(button(&r.scene, Surface::Settings, &Action::CopyInvite(SecretId(7))).label, "Copied");
    let back = r.scene.next_wake.unwrap();
    r.tick_to(back.0);
    assert_eq!(button(&r.scene, Surface::Settings, &Action::CopyInvite(SecretId(7))).label, "Copy link");
}

/// The composer counts its edits, like every typed field (DECISIONS B107; region5.rs has the
/// Library's): the name cut to its 80 is one edit taken in; the composer emptied once its
/// invite is made is the core's own change, not an edit; an edit off the Members face is
/// dropped, uncounted; the count goes with the window's fields.
#[test]
fn the_members_composer_counts_its_edits() {
    fn name(r: &Run) -> (String, u64) {
        let Some(shroom_core::scene::SettingsPane::Members(m)) = r.scene.settings.as_ref().map(|x| &x.body.pane) else { panic!("the Members tab") };
        let v = serde_json::to_value(&m.name).unwrap();
        (v["value"].as_str().expect("the field's value").to_string(), v["edits"].as_u64().expect("the field's edits, counted"))
    }
    let mut r = Run::new(with_popover(owner()));
    r.act(Action::OpenSettings);
    r.set(Action::SettingsTab(SettingsTab::Members));
    r.hear(MemberNews::Roster(roster()));
    assert_eq!(name(&r), (String::new(), 0));
    r.field(Field::InviteName, "A");
    r.field(Field::InviteName, "An");
    r.field(Field::InviteName, "Ani");
    assert_eq!(name(&r), (s("Ani"), 3), "one for every edit taken in");
    r.field(Field::InviteName, &"a".repeat(81));
    assert_eq!(name(&r), ("a".repeat(80), 4), "cut to its 80, still one edit");
    r.field(Field::InviteName, "Ani");
    r.set(Action::Invite);
    r.send(invited_ani());
    assert_eq!(name(&r), (String::new(), 5), "the core emptied it: not an edit, the count stands");

    r.set(Action::SettingsTab(SettingsTab::General));
    r.field(Field::InviteName, "x");
    assert_eq!(r.arm.name, "dropped", "the composer isn't showing");
    r.set(Action::SettingsTab(SettingsTab::Members));
    assert_eq!(name(&r), (String::new(), 5), "a dropped edit is never counted");

    r.send(Event::WindowClosed(WindowKind::Settings));
    r.click();
    r.act(Action::OpenSettings);
    r.set(Action::SettingsTab(SettingsTab::Members));
    assert_eq!(name(&r), (String::new(), 0), "the count goes with the window's fields");
}

/// The born link is shown once: only its hash exists afterwards (§6.1.7) — closing Settings
/// takes the pill, the composer's text and any question with it.
#[test]
fn the_born_link_and_the_composer_go_with_the_window() {
    let mut r = Run::new(born(members_face(owner(), roster())));
    assert!(says(&r.scene, "v.myshroom.workers.dev/join/"));
    r.field(Field::InviteName, "Lilit");
    r.set(Action::Remove(RowRef::Member(s("m_mher"))));
    r.send(Event::WindowClosed(WindowKind::Settings));
    let mut r = Run::new(with_popover(r.w));
    r.act(Action::OpenSettings);
    r.hear(MemberNews::Roster(with_ani(roster())));
    assert!(settings_faces(&r.scene).contains(&"invite-born"), "the last tab used");
    assert!(!says(&r.scene, "v.myshroom.workers.dev/join/"), "not re-shown");
    assert!(!says(&r.scene, "Lilit"), "the composer starts empty");
    assert!(sheet(&r.scene).is_none());
}

#[test]
fn invite_rows_age_and_an_expired_one_offers_a_new_link() {
    let mut w = members_face(owner(), with_ani(roster()));
    w.wall = Wall(AUG23);
    let s0 = derive(&w);
    assert!(says(&s0, "Ani — invited Aug 21 · 5 days left"), "figures in a row fact (law 16)");
    // Sona's ran out Aug 17: [ New link ] — a row face, nothing blocked (N18).
    assert!(s0.offers(Surface::Settings, &Action::NewLink(s("i_sona"))));
    assert!(!s0.offers(Surface::Settings, &Action::NewLink(s("i_ani"))));
    let mut r = Run::new(w);
    r.set(Action::NewLink(s("i_sona")));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Renew(s("i_sona")))), "the same row, a new code (§6.1.6)");
    let renewed = PendingInvite { id: s("i_sona"), label: Some(s("Sona")), created: Wall(WALL0), expires: Wall(WALL0 + 7 * DAY), own_mac: false };
    r.hear(MemberNews::Invited { invite: renewed, link: SecretId(9) });
    assert!(says(&r.scene, "Sona — invited just now"));
    assert!(r.scene.offers(Surface::Settings, &Action::CopyInvite(SecretId(9))));
}

#[test]
fn removing_asks_at_its_own_beat_and_never_on_return() {
    let mut r = Run::new(members_face(owner(), with_ani(roster())));
    // A member.
    r.set(Action::Remove(RowRef::Member(s("m_mher"))));
    assert!(r.ids().contains(&"member-remove-confirm") && r.ids().contains(&"member-remove-confirm-member"));
    let sh = sheet(&r.scene).unwrap().clone();
    assert_eq!(sh.title, "Remove Mher?");
    assert_eq!(
        sh.lines,
        ["Mher won't be able to record to your space anymore — a video already on its way has up to an hour to finish. Mher's videos stay; you'll find them in your Library."]
    );
    assert_eq!(sh.buttons.iter().map(|b| (b.label.as_str(), b.style)).collect::<Vec<_>>(), [("Cancel", Style::Plain), ("Remove", Style::RedOutline)]);
    r.key(Surface::Settings, Key::Return);
    assert!(r.effects.is_empty() && sheet(&r.scene).is_some(), "no Return default (law 9)");
    r.key(Surface::Settings, Key::Esc);
    assert!(sheet(&r.scene).is_none(), "Esc ≡ Cancel");
    r.set(Action::Remove(RowRef::Member(s("m_mher"))));
    r.set(Action::ConfirmRemove);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::RemoveMember(s("m_mher")))));
    r.hear(MemberNews::Removed(s("m_mher")));
    assert!(!says(&r.scene, "Mher — joined"), "the row goes");

    // A pending invite.
    r.set(Action::Remove(RowRef::Invite(s("i_ani"))));
    assert!(r.ids().contains(&"member-remove-confirm-invite"));
    let sh = sheet(&r.scene).unwrap().clone();
    assert_eq!((sh.title.as_str(), sh.lines.clone()), ("Cancel Ani's invite?", vec![s("The link stops working right away.")]));
    assert_eq!(sh.buttons.iter().map(|b| (b.label.as_str(), b.style)).collect::<Vec<_>>(), [("Keep it", Style::Plain), ("Cancel invite", Style::RedOutline)]);
    r.set(Action::ConfirmRemove);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::CancelInvite(s("i_ani")))));
    r.hear(MemberNews::Removed(s("i_ani")));
    assert!(!says(&r.scene, "Ani —"));

    // Your other Mac.
    r.set(Action::Remove(RowRef::Mac(s("d_air"))));
    assert!(r.ids().contains(&"member-remove-confirm-own-mac"));
    let sh = sheet(&r.scene).unwrap().clone();
    assert_eq!((sh.title.as_str(), sh.lines.clone()), ("Remove MacBook Air?", vec![s("That Mac stops recording to your space. Its videos stay.")]));
    r.set(Action::ConfirmRemove);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::RemoveMac(s("d_air")))));
    r.hear(MemberNews::Removed(s("d_air")));
    assert!(!says(&r.scene, "MacBook Air"));
}

/// N33: a Mac that keeps the token mints from it, so a device key can't stop it — its row
/// says so and has no Remove….
#[test]
fn a_mac_that_keeps_the_token_says_so_and_cant_be_removed() {
    let s0 = derive(&members_face(owner_class(), roster_from_air()));
    assert!(says(&s0, "keeps your Cloudflare token"));
    assert!(says(&s0, "You · MacBook Pro"));
    assert!(!s0.offers(Surface::Settings, &Action::Remove(RowRef::Mac(s("d_pro")))));
    assert!(!s0.offers(Surface::Settings, &Action::Remove(RowRef::Mac(s("d_air")))), "never this Mac: that's Disconnect (N36)");
}

/// B137 (Bagrat, 2026-10-14: “Hide it until a 2nd Mac”): with one Mac, the You row says it
/// all — no sub-row for it, and no token line.
#[test]
fn one_mac_shows_no_sub_row_under_you() {
    let mut r = roster();
    r.macs.retain(|m| m.id == "d_pro");
    let s0 = derive(&members_face(owner(), r));
    assert!(says(&s0, "Mher — joined Aug 3"), "the face is up");
    assert!(!says(&s0, "keeps your Cloudflare token"));
    assert!(!says(&s0, "You · "), "no Mac row under You while there's one Mac");
}

/// B137: a second record of this same Mac — an earlier enroll's, under the same name — isn't
/// a second Mac, so it brings no sub-row either.
#[test]
fn a_second_record_of_this_mac_is_not_a_second_mac() {
    let mut r = roster();
    r.macs.retain(|m| m.id == "d_pro");
    r.macs.push(RosterMac { id: "d_pro_old".into(), name: "MacBook Pro".into(), joined: Wall(JUL20), token: true, this_mac: false });
    let s0 = derive(&members_face(owner(), r));
    assert!(!says(&s0, "keeps your Cloudflare token"));
    assert!(!says(&s0, "You · "));
}

/// B137: once a second Mac is in the space, the other Mac's sub-row shows.
#[test]
fn a_second_mac_brings_the_sub_rows() {
    let s0 = derive(&members_face(owner(), roster()));
    assert!(says(&s0, "You · MacBook Air"));
    assert!(s0.offers(Surface::Settings, &Action::Remove(RowRef::Mac(s("d_air")))));
}

/// all-set's Invite someone morphs the window into Settings on Members, the field focused.
#[test]
fn invite_someone_opens_members() {
    let mut w = owner();
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    assert_eq!(win(&r.scene), Some("all-set"));
    assert!(r.scene.dock, "the setup window: the Dock icon shows");
    r.on(Action::InviteSomeone);
    assert!(r.scene.onboarding.is_none(), "the same window, morphed");
    assert!(r.scene.settings.is_some() && r.scene.dock, "setup is over: Settings carries on, the Dock icon with it (law 6)");
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Roster)));
    r.hear(MemberNews::Roster(roster()));
    assert!(settings_faces(&r.scene).contains(&"invite-born"));
    assert!(r.scene.settings.as_ref().unwrap().activates);
    assert_eq!(r.scene.ret(Surface::Settings), Some(Action::Invite), "Return in the focused field invites");
}

// ── the member's side: the join machine (N29–N31) ──

#[test]
fn a_member_joins_by_the_deep_link_and_records_their_first_video() {
    let mut r = Run::new(fresh());
    r.send(arrived(1));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))), "the arrival pings and peeks (§6.2)");
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));
    let o = r.scene.onboarding.as_ref().unwrap();
    assert!(o.activates, "the person opened the link");
    assert_eq!(o.ret, Some(Action::Join));
    for line in [MW_TITLE, MW_TRUTH, MW_STEP, "Start shroom when you log in"] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert_eq!(labels(&r.scene, Surface::Onboarding), [(s("Join"), Style::Filled), (s("Later"), Style::Plain)], "filled action first, as its twin");
    assert!(!says(&r.scene, "Cloudflare") && !says(&r.scene, "$"), "not their bill: no Cloudflare, no card, no prices");

    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(true))), "a dismissal registers the login item iff its box is checked (U4)");
    assert!(r.ids().contains(&"joining"));
    for line in ["Bagrat's space, on this Mac:", "→ Joining Bagrat's space (shroom does this part)", ROW_SIX_AHEAD, "Joining — a few seconds."] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert!(buttons(&r.scene, Surface::Onboarding).is_empty(), "zero affordances (N16)");

    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.ids().contains(&"member-done"), "{:?}", r.ids());
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Space(member_identity()))));
    assert_eq!(r.w.space.identity.as_ref().map(|i| i.role), Some(Role::Member));
    for line in [
        "✓ Bagrat's space — v.myshroom.workers.dev",
        ROW_SIX_CURRENT,
        "macOS will ask three times: your microphone and your camera — one click each — then screen recording, which goes through System Settings.",
    ] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::AllowRecording), "the window rests: no prompt to an empty chair (N17)");

    // Region 3's chapter, verbatim.
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Mic))));
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Camera))), "then the camera (N43)");
    r.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))));
    r.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Prime)));
    r.send(Event::Rec(RecNews::Primed(true)));
    assert!(r.ids().contains(&"member-all-set"), "{:?}", r.ids());
    assert_eq!(labels(&r.scene, Surface::Onboarding), [(s("Close"), Style::Plain), (s("Record now"), Style::Filled)], "two doors: members don't invite (N5)");
    assert!(!r.scene.offers(Surface::Onboarding, &Action::InviteSomeone));

    // Identical life after joining.
    r.key(Surface::Onboarding, Key::Return);
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"));
    assert!(r.scene.settings.is_none());
    r.key(Surface::Popover, Key::Return);
    let deadline = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(deadline.0);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir(s("d1")), rec: RecId(s("AbCdEfGhIjKlMnOp")) }));
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    r.act(Action::Stop);
    r.send(Event::Rec(RecNews::Finalized(Ok(61_000))));
    assert!(r.has(|e| matches!(e, Effect::Publish { .. })), "Stop makes the link (law 3)");
}

/// Three doors, one window (N29, N30): the paste door and the installer's file land where
/// the deep link does.
#[test]
fn the_paste_door_and_the_installers_file_land_on_the_same_welcome() {
    // A link pasted anywhere in the welcome — no door shows (B159).
    let mut w = fresh();
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    assert!(!says(&r.scene, "Have an invite link?"));
    r.send(pasted_link(2));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(2)))), "a pasted link is written and peeked (DEVIATIONS #2)");
    r.send(pending(2, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));

    // The installer's hand-off: the file, and a launch with `--invite`.
    let mut r = boot(LaunchKind::Install { join: true }, &fresh());
    r.send(arrived(3));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(3)))));
    r.send(pending(3, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"), "the app opens at member-welcome instead of welcome");
    assert!(r.scene.onboarding.as_ref().unwrap().activates, "the person ran the command");
}

#[test]
fn later_remembers_the_invite_and_a_dead_one_shows_once() {
    let mut r = Run::new(welcomed(InviteKind::Member));
    r.on(Action::SetLoginBox(false));
    r.on(Action::Later);
    assert!(r.scene.onboarding.is_none() && r.ids().contains(&"idle-unprovisioned"));
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(_)))), "the box was unchecked (U4)");
    assert!(!r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)), "Later remembers the invite");
    // The tray click brings it back while it lives — re-peeked (law 1).
    r.click();
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))), "a tray click reruns peek (§6.2)");
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));
    // ✕ ≡ Later.
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(!r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)));

    // Across a launch: the remembered file opens nothing by itself.
    let mut n = boot(LaunchKind::LoginItem, &fresh());
    n.send(remembered(1));
    assert!(n.scene.onboarding.is_none(), "a login launch stays silent (law 5)");
    assert!(!n.has(|e| matches!(e, Effect::Member(_))), "nothing is asked until the click");
    n.click();
    assert!(n.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))));
    // It died meanwhile: its face once, then the plain welcome.
    n.send(peeked(1, PeekAnswer::Expired { host: s(HOST), kind: InviteKind::Member, inviter: Some(s("Bagrat")) }));
    assert!(n.ids().contains(&"join-expired"));
    assert!(says(&n.scene, EXPIRED));
    n.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(n.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)), "closing its face is the “shown once”");
    n.click();
    assert_eq!(win(&n.scene), Some("welcome"));
    assert!(!n.has(|e| matches!(e, Effect::Member(_))));
}

#[test]
fn join_failures_each_have_their_face_and_the_same_words_as_the_page() {
    // Offline mid-join: the spinner rests, one line, no button — and it finishes by itself.
    let mut r = Run::new(joining(InviteKind::Member));
    r.send(Event::Os(OsNews::NetPath(false)));
    r.send(Event::Failed { op: Op::Join, class: FailClass::Transport });
    assert!(r.ids().contains(&"join-waiting"), "{:?}", r.ids());
    assert!(says(&r.scene, OFFLINE));
    assert!(buttons(&r.scene, Surface::Onboarding).is_empty());
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))), "auto-resume (N18's posture)");
    assert!(r.ids().contains(&"joining"));
    // The space answering with a transient error: retried after the backoff.
    r.send(joined(1, JoinAnswer::Waiting));
    assert!(says(&r.scene, NOT_ANSWERING));
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Join(_)))), "never straight back");
    assert!(r.until(130_000, |e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))), "the identical join replays");
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.ids().contains(&"member-done"));

    // The Worker's terminal answers: each its face; dead links offer the paste door.
    for (answer, id, line, door) in
        [(JoinAnswer::Expired, "join-expired", EXPIRED, true), (JoinAnswer::Used, "join-used", USED, true), (JoinAnswer::SpaceMissing, "join-space-missing", MISSING, false)]
    {
        let mut r = Run::new(joining(InviteKind::Member));
        r.send(joined(1, answer));
        assert!(r.ids().contains(&id), "{id}: {:?}", r.ids());
        assert!(says(&r.scene, line), "{id}");
        assert!(!r.scene.offers(Surface::Onboarding, &Action::Join), "{id}: nothing to retry");
        let o = r.scene.onboarding.as_ref().unwrap();
        assert_eq!(o.takes_paste(SecretField::Invite), door, "{id}");
        assert_eq!(says(&r.scene, NEW_LINK_DOOR), door, "{id}");
        if door {
            r.send(pasted_link(2));
            assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(2)))), "{id}: a new link");
            r.send(pending(2, InviteKind::Member));
            assert_eq!(win(&r.scene), Some("member-welcome"), "{id}");
        }
    }
}

/// DEVIATIONS #20: peek knows at arrival that a link is dead or its space missing — the
/// face says so at once, with nothing to click first; an offline arrival rests on
/// join-waiting's offline line until peek answers (WORKER-INTERFACE open question 6).
#[test]
fn a_dead_link_says_so_on_arrival_and_an_offline_one_waits() {
    let at_arrival = |peek: PeekAnswer| {
        let mut r = Run::new(fresh());
        r.send(arrived(1));
        r.send(peeked(1, peek));
        r
    };
    let r = at_arrival(PeekAnswer::Expired { host: s(HOST), kind: InviteKind::Member, inviter: Some(s("Bagrat")) });
    assert!(r.ids().contains(&"join-expired") && says(&r.scene, EXPIRED));
    // An unknown code: the page's own nameless words (B39).
    let r = at_arrival(PeekAnswer::Expired { host: s(HOST), kind: InviteKind::Member, inviter: None });
    assert!(r.ids().contains(&"join-expired"));
    assert!(says(&r.scene, "This invite link doesn't work — ask whoever sent it for a fresh one."));
    let r = at_arrival(PeekAnswer::Used { host: s(HOST), kind: InviteKind::Member, inviter: s("Bagrat") });
    assert!(r.ids().contains(&"join-used") && says(&r.scene, USED));
    // A missing space can't name its inviter (peek's SpaceMissing carries only the host):
    // the nameless form of the line (DECISIONS B40), as for an unknown code.
    let r = at_arrival(PeekAnswer::SpaceMissing { host: s(HOST) });
    assert!(r.ids().contains(&"join-space-missing"));
    assert!(says(&r.scene, "This invite's space isn't there anymore — ask whoever sent it about it."));

    let mut r = Run::new(fresh());
    r.send(Event::Os(OsNews::NetPath(false)));
    r.send(arrived(1));
    assert!(r.ids().contains(&"join-waiting") && says(&r.scene, OFFLINE), "{:?}", r.ids());
    assert!(!r.has(|e| matches!(e, Effect::Member(_))), "nothing to ask offline");
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))));
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));
}

#[test]
fn a_mac_already_in_that_space_says_so_and_a_removed_one_rejoins() {
    // A member's Mac opens a link for its own space: /me proves the identity alive.
    let mut r = Run::new(member());
    r.send(arrived(1));
    r.send(pending(1, InviteKind::Member));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Probe)) || r.until(70_000, |e| *e == Effect::Member(MemberCmd::Probe)), "the same host asks /me (§6.2 step 4)");
    r.send(standing(fine()));
    assert!(r.ids().contains(&"join-already-here"), "{:?}", r.ids());
    assert!(says(&r.scene, "This Mac is already in Bagrat's space — nothing to do. The invite is still unused, so it can go to someone else."));
    assert_eq!(labels(&r.scene, Surface::Onboarding), [(s("Close"), Style::Plain)]);
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Join(_)))), "the link stays unused");
    r.on(Action::CloseWindow);
    assert!(r.scene.onboarding.is_none());
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)), "nothing left to remember on this Mac (B39)");

    // The owner's own Mac: the own-space words, whichever kind of link (U5) — and, as for
    // the member's, nothing joins: no device record, the invite unused (Bagrat's question,
    // 2026-10-14, on the "MacBook Pro (2)" he saw — not this path; DECISIONS B144).
    let r = Run::new(feed(owner_class(), [arrived(1), pending(1, InviteKind::OwnMac), standing(fine())]));
    assert!(says(&r.scene, "This Mac is already in your space — open the link on the Mac you want to add."));
    assert!(!says(&r.scene, "already in Bagrat's space"));
    let mut r = Run::new(owner_class());
    r.send(arrived(1));
    r.send(pending(1, InviteKind::OwnMac));
    r.send(standing(fine()));
    assert!(r.ids().contains(&"join-already-here"), "{:?}", r.ids());
    for a in [Action::Join, Action::CloseWindow] {
        r.on(a);
        assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Join(_)))), "the owner's own link stays unused");
    }

    // Re-invited after removal: the identity is dead, so it re-joins — and waiting takes drain.
    let mut w = revoked(stranded(1));
    w.session.popover = None;
    let mut r = Run::new(w);
    r.send(arrived(1));
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"), "not join-already-here: this Mac's identity is dead (N34)");
    r.on(Action::Join);
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.until(70_000, |e| matches!(e, Effect::Uploader(UpCmd::Kick))), "the waiting take drains");
    r.click();
    assert_eq!(banner(&r.scene), None, "member-revoked clears by itself (law 1)");
}

#[test]
fn switching_spaces_asks_and_waits_for_what_the_old_one_still_holds() {
    let mut w = mine_owner();
    w.queue = uploading(2);
    let mut r = Run::new(switching(w));
    let ids = r.ids();
    assert!(ids.contains(&"join-switch-confirm") && ids.contains(&"join-switch-confirm-uploads"), "{ids:?}");
    for line in [MW_TITLE, SWITCH_LINE, ONLY_HOLDER, "Two videos are still uploading — you can switch once they're done."] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    let join = button(&r.scene, Surface::Onboarding, &Action::Join);
    assert_eq!((join.label.as_str(), join.style, join.enabled), ("Join Bagrat's space", Style::Filled, false));
    assert_eq!(labels(&r.scene, Surface::Onboarding)[1], (s("Stay"), Style::Plain));
    r.on(Action::Join);
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Join(_)))));
    // The queue empties: Join enables itself.
    r.send(Event::Upload(UploadNews::Live(TakeDir(s("q0")))));
    r.send(Event::Upload(UploadNews::Live(TakeDir(s("q1")))));
    assert!(button(&r.scene, Surface::Onboarding, &Action::Join).enabled);
    assert!(!r.ids().contains(&"join-switch-confirm-uploads"));
    // A recovered take blocks the same way (N25): nothing of the old space is orphaned.
    r.send(Event::Scanned { saved: vec![saved(SavedCause::Interrupted, 272_000)], queue: vec![], removed: vec![] });
    assert!(r.ids().contains(&"join-switch-confirm-saved-take"));
    assert!(says(&r.scene, "Get the link for your saved recording first, or discard it."));
    assert!(!button(&r.scene, Surface::Onboarding, &Action::Join).enabled);
    r.send(Event::Scanned { saved: vec![], queue: vec![], removed: vec![] });
    // The old space says another Mac keeps a token too: the come-back line goes (U9).
    r.send(standing(owner_facts(false, true)));
    assert!(!says(&r.scene, ONLY_HOLDER));
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::Join));
    r.key(Surface::Onboarding, Key::Return);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))));
    assert!(r.ids().contains(&"joining"));
    // The old space stays as it was until the 200 (§5).
    assert_eq!(r.w.space.identity.as_ref().map(|i| i.host.as_str()), Some(MINE));
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert_eq!(r.w.space.identity.as_ref().map(|i| (i.host.as_str(), i.role)), Some((HOST, Role::Member)));
    assert!(r.ids().contains(&"member-all-set"), "row six is per Mac, and this Mac's is done: {:?}", r.ids());

    // Stay ≡ ✕: the invite stays unused, and is behind us.
    for close in [on(Action::Stay), Event::WindowClosed(WindowKind::Onboarding)] {
        let mut r = Run::new(switching(mine_owner()));
        r.send(close);
        assert!(r.scene.onboarding.is_none());
        assert!(r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)));
        assert_eq!(r.w.space.identity, Some(mine_identity()));
    }

    // A half-set-up space counts: its one different line.
    let mut w = fresh();
    w.space.token = true;
    let r = Run::new(switching(w));
    assert!(r.ids().contains(&"join-switch-confirm"));
    assert!(says(&r.scene, HALF_SET_UP));
    assert!(!says(&r.scene, "This Mac records to"));
}

/// Law 4: invites wait while a take is open — nothing opens over it, nothing is asked; the
/// invite window comes when the take ends.
#[test]
fn an_invite_waits_out_the_take() {
    let mut w = mine_owner();
    w.take = Some(take(Phase::Recording, 30_000));
    let mut r = Run::new(w);
    r.send(arrived(1));
    assert!(r.scene.onboarding.is_none(), "nothing opens mid-take");
    assert!(!r.has(|e| matches!(e, Effect::Member(_))), "everything waits (§6.2 step 1)");
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 30_000 }));
    r.act(Action::Stop);
    r.send(Event::Rec(RecNews::Finalized(Ok(30_000))));
    assert!(!r.w.take_open());
    assert!(r.until(70_000, |e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))), "the take ended: the invite's turn");
    r.send(pending(1, InviteKind::Member));
    assert!(r.ids().contains(&"join-switch-confirm"), "{:?}", r.ids());
}

/// Law 4 (N41): joining another space never runs mid-take — a window up before the take
/// stays, and its Join greys with the line.
#[test]
fn joining_another_space_greys_mid_take() {
    let mut w = switching(mine_owner());
    let mut t = take(Phase::Paused(Cause::User), 30_000);
    t.opened = w.now.plus(1);
    w.take = Some(t);
    let mut r = Run::new(w);
    assert!(r.ids().contains(&"join-switch-confirm"), "a window opened before the take stays: {:?}", r.ids());
    let join = button(&r.scene, Surface::Onboarding, &Action::Join);
    assert_eq!((join.enabled, join.note.as_deref()), (false, Some(AFTER_TAKE)));
    r.on(Action::Join);
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Join(_)))));
}

// ── own Macs (N33) ──

#[test]
fn add_another_mac_joins_owner_class_and_its_space_tab_says_where_repairs_happen() {
    let mut r = Run::new(members_face(owner(), roster()));
    r.set(Action::AddMac);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::AddMac)));
    r.send(invited_mac());
    assert!(r.ids().contains(&"add-another-mac"));
    assert!(says(&r.scene, "Open this link on your other Mac — it's good for one hour."));
    assert!(says(&r.scene, "v.myshroom.workers.dev/join/"));
    let copy = button(&r.scene, Surface::Settings, &Action::CopyInvite(SecretId(8)));
    assert_eq!((copy.label.as_str(), copy.style), ("Copy link", Style::Filled));
    assert!(!r.scene.offers(Surface::Settings, &Action::Remove(RowRef::Invite(s("i_mac2")))), "no row for a pending own-Mac link in v1 (§6.4)");

    // The other Mac: the same machine, different words.
    let mut m = Run::new(fresh());
    m.send(arrived(1));
    m.send(pending(1, InviteKind::OwnMac));
    assert!(m.ids().contains(&"member-welcome") && m.ids().contains(&"member-welcome-own-mac"), "{:?}", m.ids());
    for line in ["Add this Mac to your space.", "Same videos, same members — v.myshroom.workers.dev, on this Mac too.", "One step before your first video here: let macOS allow recording. About a minute, once."] {
        assert!(says(&m.scene, line), "missing {line:?}");
    }
    assert!(!says(&m.scene, MW_TITLE));
    m.on(Action::Join);
    assert!(m.ids().contains(&"joining-own-mac"));
    assert!(says(&m.scene, "Your space, on this Mac:") && says(&m.scene, "Joining your space"));
    m.send(joined(1, JoinAnswer::Joined(own_mac_identity())));
    assert!(m.ids().contains(&"member-done"));
    assert!(says(&m.scene, "✓ Your space — v.myshroom.workers.dev"));
    m.on(Action::AllowRecording);
    m.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::NotAsked }));
    m.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::NotAsked }));
    m.send(Event::Rec(RecNews::Primed(true)));
    assert!(m.ids().contains(&"member-all-set-own-mac"), "{:?}", m.ids());
    assert_eq!(
        labels(&m.scene, Surface::Onboarding),
        [(s("Close"), Style::Plain), (s("Invite someone"), Style::Plain), (s("Record now"), Style::Filled)],
        "owner-class: all three doors"
    );
    m.on(Action::InviteSomeone);
    assert!(settings_faces(&m.scene).contains(&"invite-born"), "its Settings shows Members");

    // Its Space tab: everything but repairs, and where those happen.
    m.set(Action::SettingsTab(SettingsTab::Space));
    m.send(standing(owner_facts(false, true)));
    assert!(settings_faces(&m.scene).contains(&"settings-space-no-token"), "{:?}", settings_faces(&m.scene));
    for line in [
        "✓ Your space — v.myshroom.workers.dev",
        "✓ macOS permissions granted — on this Mac",
        "Repairs happen where your Cloudflare token lives — on the Mac you set your space up on, or here if you add a fresh one.",
        MARK_LABEL,
    ] {
        assert!(says(&m.scene, line), "missing {line:?}");
    }
    assert_eq!(button(&m.scene, Surface::Settings, &Action::Disconnect).label, "Disconnect this Mac…");
    // Add a token here: the wizard's token row — this Mac becomes a token-holder.
    m.set(Action::AddToken);
    assert!(settings_faces(&m.scene).contains(&"token"), "{:?}", settings_faces(&m.scene));
    assert!(m.scene.settings.as_ref().unwrap().takes_paste(SecretField::Token));
}

#[test]
fn an_owner_class_mac_without_the_token_disconnects_by_its_own_sheet() {
    let mut w = settings_on(owner_class(), SettingsTab::Space);
    w.queue = uploading(2);
    let mut r = Run::new(w);
    r.send(standing(owner_facts(false, true)));
    r.set(Action::Disconnect);
    assert!(r.ids().contains(&"disconnect-confirm-no-token"), "{:?}", r.ids());
    let sh = sheet(&r.scene).unwrap().clone();
    assert_eq!(sh.title, "Disconnect this Mac?");
    assert!(sh.lines.contains(&s("Your space and every video stay as they are — add this Mac again anytime from Members.")));
    assert_eq!(sh.blocked.as_deref(), Some("Two videos are still uploading — you can disconnect once they're done."));
    assert!(!sh.buttons.last().unwrap().enabled);
    r.send(Event::Upload(UploadNews::Live(TakeDir(s("q0")))));
    r.send(Event::Upload(UploadNews::Live(TakeDir(s("q1")))));
    r.set(Action::ConfirmDisconnect);
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)));
    assert!(r.ids().contains(&"idle-unprovisioned"));
}

// ── member life (N34) ──

#[test]
fn removed_from_the_space_the_takes_stay_safe_and_save_to_movies() {
    let mut w = with_popover(member());
    w.queue = uploading(1);
    let mut r = Run::new(w);
    r.send(standing(StandingAnswer::Revoked));
    assert_eq!(banner(&r.scene), Some("member-revoked"));
    assert!(says(&r.scene, REVOKED));
    assert_eq!(r.scene.tray.face, Face::Amber, "something needs you (N38)");
    assert!(!start(&r.scene).enabled, "a new video would have nowhere to go");
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, None);
    let save = button(&r.scene, Surface::Popover, &Action::SaveStrandedToMovies);
    assert_eq!(save.label, "Save to Movies");
    assert_eq!(button(&r.scene, Surface::Popover, &Action::Disconnect).label, "Leave this space…");
    r.act(Action::SaveStrandedToMovies);
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir(s("q0"))])));
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir(s("q0")))));
    assert!(says(&r.scene, "Saved to Movies ✓"));
    let discard = buttons(&r.scene, Surface::Popover).into_iter().find(|b| b.label == "Discard…").expect("a quiet Discard… beside it");
    assert_eq!(discard.style, Style::RedText);
    assert!(r.w.queue.len() == 1, "the take keeps waiting — it still uploads if the space comes back");
    // Several waiting: one button for them all.
    let r = Run::new(revoked(uploading(3)));
    assert_eq!(button(&r.scene, Surface::Popover, &Action::SaveStrandedToMovies).label, "Save three to Movies");
    // Leave this space… opens its sheet in Settings.
    let mut r = Run::new(revoked(stranded(1)));
    r.act(Action::Disconnect);
    assert!(r.ids().contains(&"leave-confirm"), "{:?}", r.ids());
}

#[test]
fn space_side_faces_need_two_answers_thirty_seconds_apart_and_clear_by_themselves() {
    // One answer is a silent backoff — and a second look is asked for.
    let mut r = Run::new(with_popover(member()));
    r.at(10_000);
    r.send(standing(StandingAnswer::SpaceSide));
    assert_eq!(banner(&r.scene), None, "one answer is a silent backoff (C7)");
    assert!(r.until(70_000, |e| *e == Effect::Member(MemberCmd::Probe)), "a second look is asked for");

    let mut r = Run::new(with_popover(member()));
    r.at(10_000);
    r.send(standing(StandingAnswer::SpaceSide));
    r.at(20_000);
    r.send(standing(StandingAnswer::SpaceSide));
    assert_eq!(banner(&r.scene), None, "ten seconds apart is not two answers 30 s apart");
    r.at(41_000);
    r.send(standing(StandingAnswer::SpaceSide));
    assert_eq!(banner(&r.scene), Some("space-needs-owner"));
    assert!(says(&r.scene, NEEDS_OWNER));
    assert!(start(&r.scene).enabled, "live — the videos drain later");
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::Start));
    assert_eq!(r.scene.tray.face, Face::Amber);
    r.send(standing(fine()));
    assert_eq!(banner(&r.scene), None, "the cause clears, the face clears (law 1)");
    assert_eq!(r.scene.tray.face, Face::Plain);

    // The space is gone: the same debounce; Start greys; Save to Movies while takes wait.
    let mut w = with_popover(member());
    w.queue = uploading(1);
    let mut r = Run::new(w);
    r.at(10_000);
    r.send(standing(StandingAnswer::Gone));
    assert_eq!(banner(&r.scene), None, "a DNS blip greys nothing");
    r.at(41_000);
    r.send(standing(StandingAnswer::Gone));
    assert_eq!(banner(&r.scene), Some("space-gone"));
    assert!(says(&r.scene, GONE));
    assert!(!start(&r.scene).enabled);
    assert_eq!(button(&r.scene, Surface::Popover, &Action::SaveStrandedToMovies).label, "Save to Movies");
    assert_eq!(button(&r.scene, Surface::Popover, &Action::Disconnect).label, "Leave this space…");
    r.send(standing(fine()));
    assert_eq!(banner(&r.scene), None, "the space comes back: the banner clears by itself");
    assert!(start(&r.scene).enabled);

    // On the owner's other Mac the space-side rows read Region 2's own words (§6.7).
    let mut r = Run::new(struck(with_popover(owner_class()), StandingAnswer::SpaceSide));
    assert_eq!(banner(&r.scene), Some("space-degraded-no-token-mac"));
    assert!(says(&r.scene, "Your space needs a fix — open shroom on the Mac you set it up on, or reconnect here with a fresh token."));
    let reconnect = button(&r.scene, Surface::Popover, &Action::AddToken);
    assert_eq!((reconnect.label.as_str(), reconnect.style), ("Reconnect here", Style::Plain));
    r.act(Action::AddToken);
    assert!(settings_faces(&r.scene).contains(&"token"), "the token row, in Settings · Space: {:?}", r.ids());
}

#[test]
fn a_member_mac_asks_its_space_on_entry() {
    let mut r = Run::new(member());
    r.click();
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Probe)), "law 1's re-probe on a Mac without the token (§4.7)");
    // The token's Mac re-proves by the sweep; its /me is for the Space tab's owner facts.
    let mut r = Run::new(with_popover(owner()));
    r.act(Action::OpenSettings);
    r.set(Action::SettingsTab(SettingsTab::Space));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Probe)), "the mark and the only-holder fact");
}

#[test]
fn a_stop_the_space_cant_take_stays_safe_on_this_mac() {
    let mut w = with_popover(member());
    w.take = Some(take(Phase::Paused(Cause::User), 30_000));
    let mut r = Run::new(w);
    r.send(standing(StandingAnswer::Revoked));
    r.act(Action::Stop);
    r.send(Event::Rec(RecNews::Finalized(Ok(30_000))));
    assert!(r.has(|e| matches!(e, Effect::Publish { .. })), "it's a link, waiting to upload (law 3)");
    let p = r.scene.popover.as_ref().expect("the stop's popover");
    assert_eq!(p.id, "stopped-stranded");
    assert!(says(&r.scene, "Your recording is safe on this Mac."));
    assert!(says(&r.scene, REVOKED), "the banner's reason line (N34)");
    let save = button(&r.scene, Surface::Popover, &Action::SaveToMovies);
    assert_eq!((save.label.as_str(), save.style), ("Save to Movies", Style::Filled));
    assert_eq!(p.ret, Some(Action::SaveToMovies));
    assert!(!r.scene.offers(Surface::Popover, &Action::ReconnectSpace), "owners only");
    assert!(!r.has(|e| matches!(e, Effect::Cloud(_))), "no page to open");
    r.key(Surface::Popover, Key::Return);
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir(s("20261002-120000-ab12"))])));

    let r = Run::new(stranded_stop(StandingAnswer::Gone));
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("stopped-stranded"));
    assert!(says(&r.scene, GONE));
}

#[test]
fn space_needs_an_update_reads_by_role() {
    let behind = StandingAnswer::Fine { current: false, only_holder: None, mark: None, ai: None };
    let mut r = Run::new(with_popover(member()));
    r.send(standing(behind.clone()));
    assert_eq!(banner(&r.scene), Some("space-needs-update-member"));
    assert!(says(&r.scene, "Bagrat's space needs an update — it happens when Bagrat updates shroom."));
    assert!(start(&r.scene).enabled);
    r.send(standing(fine()));
    assert_eq!(banner(&r.scene), None);

    let mut r = Run::new(with_popover(owner_class()));
    r.send(standing(behind));
    assert_eq!(banner(&r.scene), Some("space-needs-update-no-token"));
    assert!(says(&r.scene, "Your space needs an update — open shroom on the Mac you set it up on, or add a fresh token here."));
    assert_eq!(button(&r.scene, Surface::Popover, &Action::AddToken).label, "Add a token here");
    r.act(Action::AddToken);
    assert!(settings_faces(&r.scene).contains(&"token"));
}

#[test]
fn a_member_leaves_saving_what_never_reached_the_space() {
    let mut w = settings_on(member(), SettingsTab::Space);
    w.queue = stranded(2);
    let mut r = Run::new(w);
    r.send(standing(fine()));
    let f = settings_faces(&r.scene);
    assert!(f.contains(&"settings-space") && f.contains(&"settings-space-member"), "{f:?}");
    for line in ["✓ Bagrat's space — v.myshroom.workers.dev", "✓ macOS permissions granted — on this Mac", "Your videos live there, where Bagrat can see them too."] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert!(!says(&r.scene, MARK_LABEL), "the mark is the owner's (N40)");
    assert!(!says(&r.scene, "Disconnect this Mac…"));
    let leave = button(&r.scene, Surface::Settings, &Action::Disconnect);
    assert_eq!((leave.label.as_str(), leave.style), ("Leave this space…", Style::Quiet));
    r.set(Action::Disconnect);
    assert!(r.ids().contains(&"leave-confirm"));
    let sh = sheet(&r.scene).unwrap().clone();
    assert_eq!(sh.title, "Leave Bagrat's space?");
    assert!(sh.lines.contains(&s("Your videos stay there, where Bagrat can still see them — delete any you want gone first. This Mac goes back to the start.")));
    assert!(sh.lines.contains(&s("Two recordings never reached the space — leaving deletes them from this Mac.")));
    assert_eq!(
        sh.buttons.iter().map(|b| (b.label.as_str(), b.style, b.enabled)).collect::<Vec<_>>(),
        [("Cancel", Style::Plain, true), ("Save them to Movies first", Style::Plain, true), ("Leave", Style::RedOutline, true)]
    );
    r.key(Surface::Settings, Key::Return);
    assert!(r.effects.is_empty(), "no Return default (law 9)");
    r.set(Action::SaveStrandedToMovies);
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir(s("q0")), TakeDir(s("q1"))])));
    // Movies *first* (law 2, DECISIONS B54): Leave waits while the copies are under way.
    assert!(!button(&r.scene, Surface::Settings, &Action::ConfirmDisconnect).enabled);
    assert_eq!(button(&r.scene, Surface::Settings, &Action::SaveStrandedToMovies).label, "Saving to Movies…");
    for d in ["q0", "q1"] {
        r.send(Event::Rec(RecNews::SavedToMovies(TakeDir(s(d)))));
    }
    r.set(Action::ConfirmDisconnect);
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir(s("q0"))))), "leaving deletes them, as the sheet said");
    assert!(r.ids().contains(&"idle-unprovisioned"));
    assert!(!r.w.provisioned());
}

/// CHECKPOINT §10 11.1 (DECISIONS B157): a removed member's Space tab said “✓ Bagrat's
/// space” and “Your videos live there…” while the popover said access was turned off.
/// Settings reads the same member life the popover does: the space's row loses its tick
/// and the banner's own line stands in for N5's — the same words, nothing new to say.
#[test]
fn settings_space_says_what_the_popover_says_once_a_member_is_removed() {
    const TICKED: &str = "✓ Bagrat's space — v.myshroom.workers.dev";
    const UNTICKED: &str = "• Bagrat's space — v.myshroom.workers.dev";
    const LIVES_THERE: &str = "Your videos live there, where Bagrat can see them too.";
    let mut r = Run::new(settings_on(with_popover(member()), SettingsTab::Space));
    r.send(standing(fine()));
    assert!(says(&r.scene, TICKED) && says(&r.scene, LIVES_THERE));

    r.send(standing(StandingAnswer::Revoked));
    assert_eq!(banner(&r.scene), Some("member-revoked"));
    let f = settings_faces(&r.scene);
    assert!(f.contains(&"settings-space-member"), "{f:?}");
    assert!(!says(&r.scene, TICKED), "the space no longer takes this Mac's videos");
    assert!(says(&r.scene, UNTICKED));
    assert!(!says(&r.scene, LIVES_THERE));
    assert!(says(&r.scene, "✓ macOS permissions granted — on this Mac"), "this Mac's own row stands");
    assert_eq!(json(&r.scene).matches(&enc(REVOKED)).count(), 2, "the popover's line, and Settings' — the same words");
    assert_eq!(button(&r.scene, Surface::Settings, &Action::Disconnect).label, "Leave this space…");

    // Closed and opened again: still so (it was “still the same” on 0.1.12).
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert!(r.scene.settings.is_none());
    r.act(Action::OpenSettings);
    assert!(says(&r.scene, UNTICKED) && !says(&r.scene, TICKED), "{:?}", settings_faces(&r.scene));

    // Access back: the tick and N5's line come back by themselves (law 1).
    r.send(standing(fine()));
    assert!(says(&r.scene, TICKED) && says(&r.scene, LIVES_THERE));
    assert!(!says(&r.scene, REVOKED));

    // The space gone (C7's two answers): the same, in space-gone's words.
    let r = Run::new(struck(settings_on(member(), SettingsTab::Space), StandingAnswer::Gone));
    assert!(says(&r.scene, UNTICKED) && says(&r.scene, GONE) && !says(&r.scene, LIVES_THERE));

    // The owner's other Mac, removed: “This Mac was removed…”, and no token door — a
    // removed owner Mac has nothing to reconnect to (DEVIATIONS #21).
    let mut r = Run::new(settings_on(owner_class(), SettingsTab::Space));
    r.send(standing(StandingAnswer::Revoked));
    assert!(says(&r.scene, "• Your space — v.myshroom.workers.dev"));
    assert!(says(&r.scene, "This Mac was removed from your space. Recordings on this Mac are safe — add it again from Members on your other Mac."));
    assert!(!says(&r.scene, "Add a token here"));
    assert!(!says(&r.scene, "Repairs happen where your Cloudflare token lives — on the Mac you set your space up on, or here if you add a fresh one."));
}

/// CHECKPOINT §10 7.1 (DECISIONS B158; Bagrat: “it takes the owners name from Mac, we
/// should probably ask for a name huh?”, then “7.1 ok”): under the composer, always, the
/// name invites carry and *Change*; the name becomes a field, Save keeps it in the space's
/// identity, Esc puts it back.
#[test]
fn the_members_face_says_who_invites_are_from_and_changes_it() {
    const FROM: &str = "Invites will say they're from Bagrat.";
    let mut r = Run::new(settings_on(owner(), SettingsTab::Members));
    r.hear(MemberNews::Roster(roster()));
    assert!(says(&r.scene, FROM), "the name is read before a link goes out");
    assert!(json(&r.scene).contains(r#"{"text":"Bagrat","em":"strong"}"#), "the name in bold: {}", json(&r.scene));
    let change = button(&r.scene, Surface::Settings, &Action::ChangeFrom);
    assert_eq!((change.label.as_str(), change.style), ("Change", Style::Link));

    // Change: the line's head labels the field, the name in it; Save beside it.
    r.set(Action::ChangeFrom);
    assert!(!says(&r.scene, FROM));
    assert!(says(&r.scene, "Invites will say they're from"));
    assert_eq!(button(&r.scene, Surface::Settings, &Action::SaveFrom).label, "Save");
    // Esc puts it back.
    r.field(Field::FromName, "Bagrat A.");
    r.key(Surface::Settings, Key::Esc);
    assert!(says(&r.scene, FROM));
    assert!(r.effects.is_empty());

    // Empty: Save greys. More than the Worker keeps (40): cut there.
    r.set(Action::ChangeFrom);
    r.field(Field::FromName, "   ");
    assert!(!button(&r.scene, Surface::Settings, &Action::SaveFrom).enabled);
    r.field(Field::FromName, &"x".repeat(60));
    assert!(says(&r.scene, &"x".repeat(40)) && !says(&r.scene, &"x".repeat(41)));

    // Save: kept in space.json, and the next invite carries it (the runtime sends the
    // identity's name as the invite's `inviter`).
    r.field(Field::FromName, "  Bagrat   Aznauryan ");
    r.set(Action::SaveFrom);
    assert!(says(&r.scene, "Invites will say they're from Bagrat Aznauryan."));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Space(id)) if id.owner_name == "Bagrat Aznauryan")));
    assert_eq!(r.w.space.identity.as_ref().unwrap().owner_name, "Bagrat Aznauryan");

    // A name left mid-change on another tab is put back.
    r.set(Action::ChangeFrom);
    r.set(Action::SettingsTab(SettingsTab::General));
    r.set(Action::SettingsTab(SettingsTab::Members));
    assert!(says(&r.scene, "Invites will say they're from Bagrat Aznauryan."));
}

// ── the Library across members (N5, N33, N35) ──

#[test]
fn the_library_scopes_mine_and_everyone_and_names_another_members_video() {
    // A member's Library is theirs alone, and has no scope control.
    let sm = derive(&library_with(member(), mixed_rows()));
    assert_eq!(titles(&sm), ["Mher's onboarding notes"]);
    assert!(!says(&sm, "Everyone"));

    // Owner-class: Mine · Everyone, once someone else has a video here — Mine first.
    let mut r = Run::new(library_with(owner(), mixed_rows()));
    r.send(news(MemberNews::Roster(roster())));
    assert!(says(&r.scene, "Mine") && says(&r.scene, "Everyone"));
    assert_eq!(titles(&r.scene), ["Where the 502s come from"], "Mine is the default");
    r.lib(Action::SetScope(LibraryScope::Everyone));
    assert_eq!(titles(&r.scene), ["Mher's onboarding notes", "Where the 502s come from"]);
    let mher = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.key == "r_mher").unwrap();
    assert!(serde_json::to_string(mher).unwrap().contains("Mher\""), "Everyone's rows add the member's label");
    r.lib(Action::DeleteRow(RecId(s("r_mher"))));
    assert!(r.ids().contains(&"delete-confirm-other-member"), "{:?}", r.ids());
    assert!(sheet(&r.scene).unwrap().lines.contains(&s("This is Mher's video.")));
    r.lib(Action::ConfirmDelete);
    assert!(r.has(|e| matches!(e, Effect::Library(shroom_core::effect::LibCmd::Delete(_)))), "the owner deletes anything (N5)");

    // Only my own videos here: no control.
    let only = derive(&library_with(owner(), mixed_rows()[..1].to_vec()));
    assert!(!says(&only, "Everyone"));
    assert_eq!(titles(&only), ["Where the 502s come from"]);
}

#[test]
fn the_library_says_when_the_space_cant_be_reached() {
    let mut w = library_with(member(), mixed_rows());
    w.queue = uploading(1);
    let mut r = Run::new(w);
    r.send(standing(StandingAnswer::Revoked));
    assert!(r.scene.library.as_ref().unwrap().faces.contains(&"library-unreachable"));
    let row = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.key == "u0").unwrap().clone();
    assert_eq!(row.state.as_deref(), Some("Waiting for your space"));
    assert!(row.buttons.iter().any(|b| b.action == Action::SaveRowToMovies(TakeDir(s("q0")))));
    assert!(row.buttons.iter().any(|b| b.label == "Copy link"));
    // Only Copy link and Save to Movies stay live (r5): nothing that asks the space.
    let published = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.key == "r_mher").unwrap().clone();
    let live = |bs: &[Button], a: &Action| bs.iter().find(|b| &b.action == a).map(|b| b.enabled);
    let rec = RecId(s("r_mher"));
    assert_eq!(live(&published.menu, &Action::CopyRow(rec.clone())), Some(true));
    for a in [Action::OpenPage(rec.clone()), Action::StartRename(rec.clone()), Action::DeleteRow(rec.clone())] {
        assert_eq!(live(&published.menu, &a), Some(false), "{a:?} live in the menu");
    }
    for a in [Action::OpenPage(rec.clone()), Action::DeleteRow(rec.clone())] {
        assert_eq!(live(&published.hover, &a), Some(false), "{a:?} live on hover");
    }
    assert_eq!((published.gestures.title_click.clone(), published.gestures.double_click.clone()), (None, None));
    r.send(standing(fine()));
    let published = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.key == "r_mher").unwrap().clone();
    assert_eq!(live(&published.menu, &Action::StartRename(rec.clone())), Some(true), "the cause clears, the controls come back");
    assert!(!r.scene.library.as_ref().unwrap().faces.contains(&"library-unreachable"), "the cause clears (law 1)");
}

// ── Settings · Space's two late arrivals (DEVIATIONS #19) ──

#[test]
fn settings_space_has_the_mark_switch_for_owners_and_takes_a_pasted_link_on_any_mac() {
    let mut r = Run::new(settings_on(owner(), SettingsTab::Space));
    r.send(standing(owner_facts(true, true)));
    for line in [MARK_LABEL, MARK_LINE] {
        assert!(says(&r.scene, line), "missing {line:?}");
    }
    assert!(!says(&r.scene, SPACE_PASTE_DOOR), "no door shows (B159)");
    assert!(r.scene.offers(Surface::Settings, &Action::SetMark(false)), "the switch reads the space: on");
    r.set(Action::SetMark(false));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Mark(false))));
    r.send(news(MemberNews::Marked(false)));
    assert!(r.scene.offers(Surface::Settings, &Action::SetMark(true)));
    assert!(r.scene.settings.as_ref().unwrap().takes_paste(SecretField::Invite));
    // The setup Mac, its space's only token-holder: Disconnect's come-back line (U9).
    r.set(Action::Disconnect);
    assert!(sheet(&r.scene).unwrap().lines.contains(&s("To manage your space again, set it up with a fresh token — everything will be there.")));

    // A member: a paste, never the switch.
    let r = Run::new(feed(settings_on(member(), SettingsTab::Space), [standing(fine())]));
    assert!(r.scene.settings.as_ref().unwrap().takes_paste(SecretField::Invite));
    assert!(!r.scene.offers(Surface::Settings, &Action::SetMark(true)) && !r.scene.offers(Surface::Settings, &Action::SetMark(false)));

    // A link for another space pasted here: the invite window asks first (N31).
    let mut r = Run::new(settings_on(mine_owner(), SettingsTab::Space));
    r.send(pasted_link(3));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(3)))));
    r.send(pending(3, InviteKind::Member));
    assert!(r.ids().contains(&"join-switch-confirm"), "{:?}", r.ids());
}

/// Settings · Space, as the webview reads it.
fn space_pane(s: &Scene) -> serde_json::Value {
    serde_json::to_value(s.settings.as_ref().expect("the Settings window")).unwrap()["body"]["pane"].clone()
}

/// “Add a token here” is the wizard's token row (N33): its field holds a taken paste while
/// it's checked and put away, and is empty again when it's turned down (DECISIONS B104).
#[test]
fn add_a_token_heres_field_holds_a_paste_the_way_the_wizards_does() {
    use shroom_core::event::{CloudNews, TokenFail};
    let held = |s: &Scene| {
        let n = space_pane(s)["plan"]["nest"].clone();
        assert_eq!(n["kind"], "token", "the token row's nest");
        n["held"].clone()
    };
    let (yes, no) = (serde_json::json!(true), serde_json::json!(false));
    let token = |id: u32| Event::SecretPasted { field: SecretField::Token, id: SecretId(id), shape: Shape::Token };
    let own = own_mac_identity();
    let account = Account { id: own.account_id.clone(), name: "Bagrat's Account".into() };

    let mut r = Run::new(settings_on(owner_class(), SettingsTab::Space));
    r.send(standing(owner_facts(false, true)));
    r.set(Action::AddToken);
    assert_eq!(held(&r.scene), no, "nothing pasted yet");
    r.send(token(1));
    assert_eq!(held(&r.scene), yes, "the paste was taken");
    r.send(Event::Cloud(CloudNews::Validated { id: SecretId(1), result: Err(TokenFail::WrongPowers) }));
    assert!(settings_faces(&r.scene).contains(&"token-wrong-powers"), "{:?}", settings_faces(&r.scene));
    assert_eq!(held(&r.scene), no, "turned down: empty again, with its line");
    r.send(token(2));
    r.send(Event::Cloud(CloudNews::Validated { id: SecretId(2), result: Ok(vec![account]) }));
    assert_eq!(held(&r.scene), yes, "accepted, being put away");
    r.send(Event::Cloud(CloudNews::TokenSaved(SecretId(2))));
    assert_ne!(space_pane(&r.scene)["plan"]["nest"]["kind"], "token", "the row ticks and the field is gone");
}

/// No invite door shows in the Space tab (B159, retiring D:B104's dots with the door): a
/// link pasted anywhere in it is taken and the space asked; its own window says the rest.
/// Mid-take it waits, never asked (law 4), and opens once the take ends.
#[test]
fn a_link_pasted_in_the_space_tab_opens_its_invite_with_no_door() {
    let mut r = Run::new(settings_on(mine_owner(), SettingsTab::Space));
    assert!(!says(&r.scene, SPACE_PASTE_DOOR));
    assert!(space_pane(&r.scene).get("invite_door").is_none() && space_pane(&r.scene).get("invite_held").is_none());

    // Not an invite link: nothing.
    r.send(Event::SecretPasted { field: SecretField::Invite, id: SecretId(2), shape: Shape::Other });
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Peek(_)))));

    r.send(pasted_link(3));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(3)))));
    r.send(pending(3, InviteKind::Member));
    assert!(r.ids().contains(&"join-switch-confirm"), "{:?}", r.ids());

    // While the token row shows, a paste is the token's.
    let mut r = Run::new(settings_on(owner_class(), SettingsTab::Space));
    r.send(standing(owner_facts(false, true)));
    r.set(Action::AddToken);
    assert!(!r.scene.settings.as_ref().unwrap().takes_paste(SecretField::Invite));

    // Mid-take an invite waits (law 4). (The window was open before the take began — a
    // take hides the ones opened after it.)
    let mut w = settings_on(mine_owner(), SettingsTab::Space);
    let mut t = take(Phase::Recording, 30_000);
    t.opened = Mono(w.now.0 + 5);
    t.since = t.opened;
    w.take = Some(t);
    let mut r = Run::new(w);
    r.send(pasted_link(5));
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Peek(_)))), "never asked mid-take");
    assert!(r.scene.onboarding.is_none());
}

// ── liveness and law 13 ──

#[test]
fn every_region6_op_has_a_deadline() {
    for op in [Op::Peek, Op::Join, Op::Standing, Op::Roster, Op::Invite, Op::Remove, Op::Mark] {
        assert_eq!(op.deadline_ms(), 60_000, "{op:?}");
    }
    // A join that never answers ends at its deadline: join-waiting, and it's tried again.
    let mut r = Run::new(joining(InviteKind::Member));
    assert!(r.w.inflight.ops.contains_key(&Op::Join), "Join started an op");
    let deadline = r.w.inflight.ops[&Op::Join];
    r.tick_to(deadline.0);
    assert!(r.ids().contains(&"join-waiting"), "{:?}", r.ids());
    assert!(r.until(130_000, |e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))));
}

/// Law 13: join links ride as stash handles — the inventory's every Scene carries a join
/// path's host part at most, never a code.
#[test]
fn join_codes_never_reach_a_scene() {
    assert!(laws::carries_join_code("v.myshroom.workers.dev/join/7xk2pm"));
    assert!(laws::carries_join_code("Peek(\"v.x.workers.dev/join/AbCdEfGhIjKlMnOp\")"));
    assert!(!laws::carries_join_code("{\"prefix\":\"v.myshroom.workers.dev/join/\",\"link\":7}"));
    for (id, w, _) in rows() {
        assert!(!laws::carries_join_code(&json(&derive(&w))), "{id}");
    }
}

// ── the machine's drawn edges (TESTING-PLAN §2.2) ──

/// The ids an edge's end may show as in the app (any of them): a page or a terminal shows
/// nothing of the app's, and a map note walked here shows its state.
fn shows(name: &str) -> &'static [&'static str] {
    match name {
        "all-set" => &["all-set"],
        "invite-born" | "invite-row-faces" => &["invite-born"],
        "add-another-mac" => &["add-another-mac"],
        "member-remove-confirm" => &["member-remove-confirm"],
        "join-installing" => &[],
        "join-landing" => &["idle-unprovisioned"],
        "member-welcome" => &["member-welcome"],
        "joining" => &["joining"],
        "join-waiting" => &["join-waiting"],
        "join-space-missing" => &["join-space-missing"],
        "join-switch-confirm" => &["join-switch-confirm"],
        "member-done" => &["member-done"],
        "perm-mic" | "perm-priming" => &["perm-mic", "perm-screen", "relaunching", "perm-priming"],
        "member-all-set" => &["member-all-set", "member-all-set-own-mac"],
        "config" => &["config", "idle-ready"],
        "member-revoked" | "save-movies" => &["member-revoked"],
        "settings-space" => &["settings"],
        "space-moods" => &["settings-space-no-token", "settings-space-member"],
        "stopping" => &[],
        "stopped-stranded" => &["stopped-stranded"],
        other => panic!("no shows for {other:?}"),
    }
}

/// The map's notes walked here: not states, so not edges.
const NOTES: &[&str] = &["invite-row-faces", "save-movies", "space-moods"];

/// Each edge: its start world and the events that cross it.
fn realized() -> Vec<(&'static str, &'static str, World, Vec<Event>)> {
    use InviteKind::Member as M;
    let mut owner_all_set = owner();
    owner_all_set.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    // Ani's invite runs out at WALL0 + 5 s; the Tick lands after it.
    let mut fading = roster();
    fading.invites.push(PendingInvite { id: s("i_ani"), label: Some(s("Ani")), created: Wall(WALL0), expires: Wall(WALL0 + 5_000), own_mac: false });
    let mut expiring = members_face(owner(), fading);
    expiring.now = Mono(10_000);
    let mut stopping = member();
    let mut t = take(Phase::Paused(Cause::User), 30_000);
    t.phase = Phase::Finalizing(Why::Stop);
    stopping.take = Some(t);
    let stopping = feed(stopping, [standing(StandingAnswer::Revoked)]);
    let priming = {
        let mut w = joined_as(M);
        w.session.chapter = true;
        w.mac.mic = Fact::known(Access::Granted, w.now);
        w.mac.screen = Fact::known(true, w.now);
        w
    };
    vec![
        ("all-set", "invite-born", owner_all_set, vec![on(Action::InviteSomeone)]),
        ("invite-born", "member-remove-confirm", members_face(owner(), roster()), vec![set(Action::Remove(RowRef::Member(s("m_mher"))))]),
        ("invite-born", "invite-row-faces", expiring, vec![Event::Tick]),
        ("invite-born", "add-another-mac", members_face(owner(), roster()), vec![set(Action::AddMac), invited_mac()]),
        ("join-installing", "member-welcome", World::blank(), vec![Event::Booted(Box::new(disk(LaunchKind::Install { join: true }, &fresh()))), arrived(1), pending(1, M)]),
        ("join-landing", "member-welcome", fresh(), vec![arrived(1), pending(1, M)]),
        ("member-welcome", "joining", welcomed(M), vec![on(Action::Join)]),
        ("join-switch-confirm", "joining", switching(mine_owner()), vec![on(Action::Join)]),
        ("joining", "join-waiting", joining(M), vec![Event::Failed { op: Op::Join, class: FailClass::Transport }]),
        ("join-waiting", "joining", feed(joining(M), [Event::Os(OsNews::NetPath(false)), Event::Failed { op: Op::Join, class: FailClass::Transport }]), vec![Event::Os(OsNews::NetPath(true))]),
        ("joining", "join-space-missing", joining(M), vec![joined(1, JoinAnswer::SpaceMissing)]),
        ("joining", "member-done", joining(M), vec![joined(1, JoinAnswer::Joined(member_identity()))]),
        ("member-done", "perm-mic", joined_as(M), vec![on(Action::AllowRecording)]),
        ("perm-priming", "member-all-set", priming, vec![Event::Rec(RecNews::Primed(true))]),
        ("member-all-set", "config", all_set_as(M), vec![on(Action::RecordNow)]),
        ("member-revoked", "save-movies", revoked(uploading(1)), vec![pop(Action::SaveStrandedToMovies)]),
        // The deferred edges of Regions 3, 4 and 5.
        ("settings-space", "space-moods", settings_on(owner_class(), SettingsTab::General), vec![set(Action::SettingsTab(SettingsTab::Space))]),
        ("stopping", "stopped-stranded", stopping, vec![Event::Rec(RecNews::Finalized(Ok(30_000)))]),
    ]
}

/// Region 6's edges no app event crosses, each with its reason.
const NOT_REALIZED_HERE: &[(&str, &str, &str)] = &[
    ("invite-born", "join-landing", "the link travels over any chat and opens the Worker's page — the Worker's suite"),
    ("join-landing", "join-landing-elsewhere", "the join page on something that isn't a Mac — the Worker's"),
    ("join-landing", "join-expired", "the join page's expired face — the Worker's; the app's own is walked in join_failures_…"),
    ("join-landing", "join-used", "the join page's used face — the Worker's; the app's own is walked in join_failures_…"),
    ("join-landing", "join-installing", "curl with the link built in — the installer's suite"),
    ("member-welcome", "join-already-here", "a Mac already in that space never shows member-welcome: the arrival routes straight there (§6.2 step 4) — a_mac_already_in_that_space_…"),
    ("member-welcome", "join-switch-confirm", "a Mac in another space never shows member-welcome: the arrival routes straight there — switching_spaces_…"),
];

/// The deferred edges of other regions this file owns.
const DEFERRED_EDGES: &[(&str, &str)] = &[("stopping", "stopped-stranded")];

#[test]
fn every_region6_and_deferred_drawn_edge_is_realized_or_listed() {
    let in_scope = drawn_edges(|from, to| [from, to].iter().any(|s| region_of(s) == 6) || DEFERRED_EDGES.contains(&(from, to)));
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let mut failures = Vec::new();
    for (from, to, world, events) in realized() {
        assert!(NOTES.contains(&to) || covered.insert((from.into(), to.into())), "{from} → {to} realized twice");
        let before = derive(&world).state_ids();
        let starts = shows(from);
        if !starts.is_empty() && !starts.iter().any(|a| before.contains(a)) {
            failures.push(format!("{from} → {to}: the start world derives {before:?}"));
            continue;
        }
        // Each edge runs with the laws; a law's panic is that edge's failure, not the test's end.
        let crossing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut r = Run::new(world);
            let mut seen_effects: Vec<Effect> = Vec::new();
            for e in events {
                r.send(e);
                seen_effects.extend(r.effects.clone());
            }
            let after = r.ids();
            let crossed = shows(to).iter().any(|a| after.contains(a))
                && match to {
                    "invite-row-faces" => r.scene.offers(Surface::Settings, &Action::NewLink(s("i_ani"))),
                    "save-movies" => seen_effects.iter().any(|e| matches!(e, Effect::Takes(TakeCmd::SaveToMovies(_)))),
                    "config" => r.scene.popover.as_ref().is_some_and(|p| p.id == "config"),
                    "perm-mic" => seen_effects.contains(&Effect::Shell(ShellCmd::RequestPermission(Perm::Mic))),
                    _ => true,
                };
            (crossed, format!("{after:?}"))
        }));
        match crossing {
            Ok((true, _)) => {}
            Ok((false, after)) => failures.push(format!("{from} → {to}: ended at {after}")),
            Err(e) => failures.push(format!("{from} → {to}: {}", e.downcast_ref::<String>().cloned().unwrap_or_default().lines().next().unwrap_or("panicked"))),
        }
    }
    for (from, to, _) in NOT_REALIZED_HERE {
        assert!(covered.insert(((*from).into(), (*to).into())), "{from} → {to} both realized and listed");
    }
    let missing: Vec<_> = in_scope.difference(&covered).collect();
    let stale: Vec<_> = covered.difference(&in_scope).collect();
    assert!(missing.is_empty(), "drawn edges neither realized nor listed: {missing:?}");
    assert!(stale.is_empty(), "listed edges the machine doesn't draw: {stale:?}");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    let _ = update;
}

// ── the review's edges (DECISIONS B41) ──

/// A switch never carries the old space's takes into the new one (law 14, N31): the takes
/// the old space can't take don't block — Leave's rule, one line and Movies first — and go
/// with the switch; nothing of them is ever sent to the new space.
#[test]
fn a_switch_leaves_the_old_spaces_stranded_takes_behind() {
    // A member whose access to v.mine.workers.dev was turned off, two takes waiting.
    let mut w = with_popover(mine_member());
    w.queue = uploading(2);
    let mut r = Run::new(w);
    r.send(standing(StandingAnswer::Revoked));
    r.send(arrived(1));
    r.send(pending(1, InviteKind::Member));
    assert!(r.ids().contains(&"join-switch-confirm"), "{:?}", r.ids());
    assert!(says(&r.scene, "Two recordings never reached v.mine.workers.dev — joining deletes them from this Mac."));
    assert!(r.scene.offers(Surface::Onboarding, &Action::SaveStrandedToMovies));
    assert_eq!(button(&r.scene, Surface::Onboarding, &Action::SaveStrandedToMovies).label, "Save them to Movies first");
    r.on(Action::SaveStrandedToMovies);
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir(s("q0")), TakeDir(s("q1"))])));
    // Movies first (law 2, DECISIONS B54): Join waits while the copies are under way.
    assert!(!button(&r.scene, Surface::Onboarding, &Action::Join).enabled, "a copy under way holds the switch");
    for d in ["q0", "q1"] {
        r.send(Event::Rec(RecNews::SavedToMovies(TakeDir(s(d)))));
    }
    assert!(button(&r.scene, Surface::Onboarding, &Action::Join).enabled, "they can't drain, so they don't block");
    r.on(Action::Join);
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))));
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.w.queue.is_empty(), "no take of the old space rides into the new one");
    for d in ["q0", "q1"] {
        assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir(s(d))))), "{d} goes with the switch");
    }
    assert!(!r.has(|e| matches!(e, Effect::Uploader(_))), "nothing is sent to the new space");

    // A token Mac whose space refuses its uploads: the same.
    let mut w = mine_owner();
    w.queue = stranded(1);
    let mut r = Run::new(switching(w));
    assert!(says(&r.scene, "One recording never reached v.mine.workers.dev — joining deletes it from this Mac."));
    r.on(Action::Join);
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.w.queue.is_empty() && r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir(s("q0"))))));

    // The same space joined again (a re-invite, §6.3) keeps its takes: they drain.
    let mut r = Run::new(revoked(uploading(1)));
    r.send(arrived(1));
    r.send(pending(1, InviteKind::Member));
    r.on(Action::Join);
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert_eq!(r.w.queue.len(), 1);
}

/// While a switch's join runs, a new video would belong to a space this Mac is leaving:
/// Start greys with its line, and comes back with the new space.
#[test]
fn start_waits_while_a_switch_joins() {
    let mut r = Run::new(with_popover(switching(mine_owner())));
    r.on(Action::Join);
    let start = button(&r.scene, Surface::Popover, &Action::Start);
    assert_eq!((start.enabled, start.note.as_deref()), (false, Some("You can record once this Mac has joined Bagrat's space.")));
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, None);
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    let start = button(&r.scene, Surface::Popover, &Action::Start);
    assert_eq!((start.enabled, start.note), (true, None));
}

/// A switch parked on what the old space still holds says so (N31), never that the space
/// isn't answering: a join that couldn't get through, and meanwhile a recovered take turned
/// up (the boot replay of `join.json` meets the same).
#[test]
fn a_parked_switch_says_what_it_waits_for() {
    let mut r = Run::new(switching(mine_owner()));
    r.on(Action::Join);
    r.send(Event::Failed { op: Op::Join, class: FailClass::Transport });
    assert!(says(&r.scene, NOT_ANSWERING));
    r.send(Event::Scanned { saved: vec![saved(SavedCause::Interrupted, 272_000)], queue: vec![], removed: vec![] });
    assert_eq!(win(&r.scene), Some("join-switch-confirm"), "{:?}", r.ids());
    assert!(says(&r.scene, "Get the link for your saved recording first, or discard it."));
    assert!(!says(&r.scene, NOT_ANSWERING));
    assert!(!r.until(200_000, |e| matches!(e, Effect::Member(MemberCmd::Join(_)))), "the join waits for it");
    r.send(Event::Scanned { saved: vec![], queue: vec![], removed: vec![] });
    assert!(r.until(200_000, |e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))), "and runs once it's resolved");
}

/// The own Mac's link's filled Copy link is the face's one default: Invite drops to
/// outline under it too (law 8).
#[test]
fn invite_drops_to_outline_under_either_born_link() {
    let mut r = Run::new(members_face(owner(), roster()));
    r.set(Action::AddMac);
    r.send(invited_mac());
    r.field(Field::InviteName, "Ani");
    let invite = button(&r.scene, Surface::Settings, &Action::Invite);
    assert_eq!((invite.style, invite.enabled), (Style::Plain, true));
    let filled = buttons(&r.scene, Surface::Settings).into_iter().filter(|b| b.style == Style::Filled).count();
    assert_eq!(filled, 1, "one default per surface");
}

/// An owner-class Mac whose space is gone: its stranded stop offers [ Reconnect your space ]
/// — the token row here (r4: owners also get it).
#[test]
fn an_owner_macs_stranded_stop_offers_to_reconnect() {
    let mut r = Run::new(owner_class_stranded_stop());
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("stopped-stranded"));
    assert_eq!(labels(&r.scene, Surface::Popover), vec![(s("Save to Movies"), Style::Filled), (s("Reconnect your space"), Style::Plain)]);
    r.act(Action::AddToken);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenSettings)));
    // A member's stop has nothing to reconnect.
    let r = Run::new(stranded_stop(StandingAnswer::Gone));
    assert!(!r.scene.offers(Surface::Popover, &Action::AddToken));
}

/// A second arrival while a join runs never throws the join's answer away: a 200 the space
/// already recorded is this Mac's, and its window stays on joining.
#[test]
fn a_second_arrival_mid_join_keeps_the_join() {
    let mut r = Run::new(joining(InviteKind::Member));
    r.send(arrived(2));
    assert!(r.w.inflight.ops.contains_key(&Op::Join), "the join stays in flight");
    assert_eq!(win(&r.scene), Some("joining"));
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.w.provisioned(), "the 200 lands");
    assert!(r.ids().contains(&"member-done"), "{:?}", r.ids());
}

/// `join.json` at launch (§5): the join replays, and its 200 lands — the invite then goes,
/// so a later launch never reads this Mac's own invite as used.
#[test]
fn a_join_interrupted_by_a_crash_replays_at_launch() {
    let mut r = boot(LaunchKind::User, &fresh());
    r.send(news(MemberNews::JoinPending { link: SecretId(1), host: HOST.into() }));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Join(SecretId(1)))), "the identical join replays");
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))), "and peek, once, for the plan's name (the launch's window is up)");
    r.click();
    assert_eq!(win(&r.scene), Some("joining"), "{:?}", r.ids());
    r.send(peeked(1, PeekAnswer::Used { host: HOST.into(), kind: InviteKind::Member, inviter: "Bagrat".into() }));
    assert_eq!(win(&r.scene), Some("joining"), "its own join's `used` changes nothing");
    assert!(says(&r.scene, "→ Joining Bagrat's space (shroom does this part)"));
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.w.provisioned());
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)), "the invite is behind us (§5)");
    assert!(r.w.members.invite.is_none());

    // Any join's 200 forgets the invite.
    let mut r = Run::new(joining(InviteKind::Member));
    r.send(joined(1, JoinAnswer::Joined(member_identity())));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)));
}

/// An opened invite whose peek can't get through still opens its window: join-waiting's
/// spinner and line, no default — and peek keeps trying behind it.
#[test]
fn an_unanswered_peek_opens_the_waiting_window() {
    let mut r = Run::new(fresh());
    r.send(arrived(1));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))));
    r.send(Event::Failed { op: Op::Peek, class: FailClass::Transport });
    assert_eq!(win(&r.scene), Some("join-waiting"), "{:?}", r.ids());
    assert!(says(&r.scene, "This invite's space isn't answering right now — I'll keep trying."));
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None);
    assert!(r.until(130_000, |e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))), "peek tries again");
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));
}

/// A remembered invite's window, opened by a click, waits on peek — never the owner's
/// welcome with Set up my space as its default.
#[test]
fn a_remembered_invite_never_shows_the_owners_welcome() {
    let mut r = boot(LaunchKind::LoginItem, &fresh());
    r.send(remembered(1));
    r.click();
    assert_eq!(win(&r.scene), Some("join-waiting"), "{:?}", r.ids());
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None);
    assert!(!r.scene.offers(Surface::Onboarding, &Action::SetUp));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(1)))));
    r.send(pending(1, InviteKind::Member));
    assert_eq!(win(&r.scene), Some("member-welcome"));
}

/// An invite for this Mac's own space while that space's side faults: C7's second look
/// settles it — this Mac is still here — never a spinner with nothing to wake it.
#[test]
fn a_same_space_invite_during_a_space_fault_settles() {
    let mut r = Run::new(member());
    r.send(arrived(1));
    r.send(pending(1, InviteKind::Member));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Probe)));
    r.send(standing(StandingAnswer::SpaceSide));
    assert_eq!(win(&r.scene), Some("join-waiting"));
    assert!(says(&r.scene, NOT_ANSWERING));
    assert!(r.until(40_000, |e| *e == Effect::Member(MemberCmd::Probe)), "C7's second look");
    r.send(standing(StandingAnswer::SpaceSide));
    assert_eq!(win(&r.scene), Some("join-already-here"), "{:?}", r.ids());
    assert!(r.scene.offers(Surface::Onboarding, &Action::CloseWindow));
}

/// A stale peek answer touches nothing of the current link's peek in flight.
#[test]
fn a_stale_peek_answer_never_refires_the_current_peek() {
    let mut r = Run::new(fresh());
    r.send(arrived(1));
    r.send(arrived(2));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Peek(SecretId(2)))));
    r.send(pending(1, InviteKind::Member));
    assert!(r.effects.is_empty(), "{:?}", r.effects);
    assert!(r.w.inflight.ops.contains_key(&Op::Peek));
}

/// §4.18: once C7 confirms the space refuses this Mac's uploads, the owner page learns it —
/// once — and learns it's over on the next upload that lands.
#[test]
fn the_space_hears_when_uploads_are_refused_and_when_they_move_again() {
    let mut w = with_popover(member());
    w.queue = uploading(1);
    let mut r = Run::new(w);
    r.send(standing(StandingAnswer::SpaceSide));
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Health { .. }))), "one answer is a blip");
    r.at(r.t + STRIKE_GAP_MS + 1_000);
    r.send(standing(StandingAnswer::SpaceSide));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Health { rec: RecId(s("u0")), refused: true })));
    r.at(r.t + 5_000);
    r.send(standing(StandingAnswer::SpaceSide));
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Health { .. }))), "said once");
    r.send(Event::Upload(UploadNews::Progress { dir: TakeDir(s("q0")), done: 2, total: 4 }));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::Health { rec: RecId(s("u0")), refused: false })));
    r.send(Event::Upload(UploadNews::Progress { dir: TakeDir(s("q0")), done: 3, total: 4 }));
    assert!(!r.has(|e| matches!(e, Effect::Member(MemberCmd::Health { .. }))));
}

/// Open question 15, built as recommended: a page older than this app needs rests the
/// invite window on the needs-update line, Join greyed — until a peek finds it caught up.
#[test]
fn an_invite_to_a_space_that_needs_an_update_waits_for_it() {
    let behind = |link: u32| peeked(link, PeekAnswer::Pending { host: HOST.into(), kind: InviteKind::Member, inviter: "Bagrat".into(), current: false });
    let mut r = Run::new(fresh());
    r.send(arrived(1));
    r.send(behind(1));
    assert_eq!(win(&r.scene), Some("member-welcome"));
    assert!(says(&r.scene, "Bagrat's space needs an update — it happens when Bagrat updates shroom."));
    assert!(!button(&r.scene, Surface::Onboarding, &Action::Join).enabled);
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, None);
    r.send(pending(1, InviteKind::Member));
    assert!(button(&r.scene, Surface::Onboarding, &Action::Join).enabled);
    assert_eq!(r.scene.onboarding.as_ref().unwrap().ret, Some(Action::Join));

    // The join's re-ping says so: the join stops there, never replayed.
    let mut r = Run::new(joining(InviteKind::Member));
    r.send(joined(1, JoinAnswer::Behind));
    assert_eq!(win(&r.scene), Some("member-welcome"));
    assert!(!button(&r.scene, Surface::Onboarding, &Action::Join).enabled);
    assert!(!r.until(200_000, |e| matches!(e, Effect::Member(MemberCmd::Join(_)))));
}

/// N31's table: a refusal Join's answer brings — expired, used, its space missing — renders
/// under the joining row (“Rendered in the window under the joining row”), the plan kept:
/// its header, “→ Joining Bagrat's space”, row six ahead, the line (and the paste door
/// where a new link is the way on) nested under row one. Only already-here is the whole
/// window's face; a link peek knew dead at arrival never had a plan (DEVIATIONS #20).
#[test]
fn a_refusal_after_join_renders_under_the_joining_row() {
    use shroom_core::scene::{Glyph, Nest, OnboardingBody};
    let plan_of = |s: &Scene| match &s.onboarding.as_ref().expect("the window").body {
        OnboardingBody::Plan(p) => Some(p.clone()),
        _ => None,
    };
    for (answer, id, line, door) in
        [(JoinAnswer::Expired, "join-expired", EXPIRED, true), (JoinAnswer::Used, "join-used", USED, true), (JoinAnswer::SpaceMissing, "join-space-missing", MISSING, false)]
    {
        for (from, w) in [("a fresh Mac", joining(InviteKind::Member)), ("a switch", feed(switching(mine_member()), [on(Action::Join)]))] {
            let s = derive(&feed(w, [joined(1, answer.clone())]));
            assert!(s.state_ids().contains(&id), "{id} from {from}: {:?}", s.state_ids());
            let p = plan_of(&s).unwrap_or_else(|| panic!("{id} from {from}: the plan is kept"));
            assert_eq!(p.header, "Bagrat's space, on this Mac:", "{id} from {from}: about the invite's space");
            assert_eq!(p.rows.len(), 2, "{id} from {from}");
            assert_eq!((p.rows[0].glyph, p.rows[0].line.as_str()), (Glyph::Current, "→ Joining Bagrat's space (shroom does this part)"), "{id} from {from}");
            assert_eq!(p.rows[1].line, ROW_SIX_AHEAD, "{id} from {from}");
            assert_eq!(p.current, Some(1), "{id} from {from}: under row one");
            match &p.nest {
                Nest::JoinRefused { line: l, door: d } => {
                    assert_eq!(l, line, "{id} from {from}");
                    assert_eq!(d.as_deref(), door.then_some(NEW_LINK_DOOR), "{id} from {from}");
                }
                other => panic!("{id} from {from}: nest {other:?}"),
            }
            assert_eq!(s.onboarding.as_ref().unwrap().takes_paste(SecretField::Invite), door, "{id} from {from}: the door takes a paste");
            assert_eq!(s.ret(Surface::Onboarding), None, "{id} from {from}: nothing to retry");
        }
    }
    // Already here: the whole window's face (N31's “window face”), no plan.
    let s = derive(&feed(joining(InviteKind::Member), [joined(1, JoinAnswer::AlreadyHere)]));
    assert!(plan_of(&s).is_none() && s.state_ids().contains(&"join-already-here"));
    // Dead at arrival (peek): the whole-window face, as DEVIATIONS #20 has it.
    let dead = derive(&feed(fresh(), [arrived(1), peeked(1, PeekAnswer::Expired { host: HOST.into(), kind: InviteKind::Member, inviter: Some("Bagrat".into()) })]));
    assert!(plan_of(&dead).is_none() && dead.state_ids().contains(&"join-expired"));
}

/// The member plan's `current` is a row's place among the rows it shows (two), never the
/// owner's six-row numbering: done · member rests on row two, its nest under it.
#[test]
fn the_member_plans_current_row_is_one_of_its_own_two() {
    use shroom_core::scene::{Nest, OnboardingBody};
    for kind in [InviteKind::Member, InviteKind::OwnMac] {
        let s = derive(&joined_as(kind));
        let OnboardingBody::Plan(p) = &s.onboarding.as_ref().unwrap().body else { panic!("{kind:?}: a plan") };
        assert_eq!(p.rows.len(), 2);
        assert_eq!(p.current, Some(2), "{kind:?}: row six is this plan's second");
        assert_eq!(p.rows[1].line, ROW_SIX_CURRENT);
        assert!(matches!(p.nest, Nest::Done { .. }), "{kind:?}: done's nest");
        let s = derive(&joining(kind));
        let OnboardingBody::Plan(p) = &s.onboarding.as_ref().unwrap().body else { panic!("{kind:?}: a plan") };
        assert_eq!(p.current, Some(1));
        for s in [derive(&all_set_as(kind))] {
            if let Some(OnboardingBody::Plan(p)) = s.onboarding.as_ref().map(|o| &o.body) {
                assert!(p.current.is_none_or(|c| (1..=2).contains(&c)), "{kind:?}: all-set's current {:?}", p.current);
            }
        }
    }
    // Settings · Space on a Mac without the token: row six un-ticked by a revoked screen
    // grant is current — the second of its two rows.
    for w in [member(), owner_class()] {
        let mut w = feed(settings_on(w, SettingsTab::Space), [standing(fine())]);
        w.mac.screen = Fact::known(false, w.now);
        let s = derive(&w);
        let Some(shroom_core::scene::SettingsPane::Space(sp)) = s.settings.as_ref().map(|x| &x.body.pane) else { panic!("the Space tab") };
        assert_eq!(sp.plan.rows.len(), 2);
        assert_eq!(sp.plan.current, Some(2), "{:?}", sp.plan.rows.iter().map(|r| &r.line).collect::<Vec<_>>());
    }
}

/// Law 12's converse over this region's inventory: from every row, every op in flight —
/// as the row stands, after each button it offers, after its next wake — failed every way
/// lands (a face, a line, its note read, a noted retry), with the laws at every step.
#[test]
fn every_failure_from_every_row_lands() {
    let bad = every_failure_lands(rows().into_iter().map(|(id, w, _)| (id, w)).collect());
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

/// Law 12 on the Members face (DEVIATIONS #28): a link that didn't come back, or a Remove…
/// that didn't go through, says so in the composer's line — the row stays as it was, the
/// buttons come back — until the next try answers, or the window is opened afresh.
#[test]
fn a_members_write_that_fails_says_so() {
    const INVITE_FAILED: &str = "Couldn't make a link just now — try again in a moment.";
    const REMOVE_FAILED: &str = "Couldn't remove that just now — try again in a moment.";
    const INVITE_LINE: &str = "You'll get a link to send them — over any chat.";
    let mut r = Run::new(members_face(owner(), roster()));
    r.field(Field::InviteName, "Lilit");
    r.set(Action::Invite);
    assert!(r.w.inflight.ops.contains_key(&Op::Invite));
    for class in [FailClass::Transport, FailClass::Refused, FailClass::Unclassified] {
        let mut f = Run::new(r.w.clone());
        f.send(Event::Failed { op: Op::Invite, class });
        assert!(says(&f.scene, INVITE_FAILED) && !says(&f.scene, INVITE_LINE), "{class:?}");
        assert!(f.scene.offers(Surface::Settings, &Action::Invite), "Invite comes back ({class:?})");
        assert!(says(&f.scene, "Lilit"), "what was typed stays");
    }
    // Its deadline gone by reads the same; the next Invite that answers puts the line back.
    let deadline = r.w.inflight.ops[&Op::Invite];
    r.tick_to(deadline.0 + 10);
    assert!(says(&r.scene, INVITE_FAILED));
    r.set(Action::Invite);
    assert!(says(&r.scene, INVITE_LINE), "a try running says nothing of the last one");
    let invite = PendingInvite { id: s("i_lilit"), label: Some(s("Lilit")), created: Wall(WALL0), expires: Wall(WALL0 + 7 * DAY), own_mac: false };
    r.hear(MemberNews::Invited { invite, link: SecretId(9) });
    assert!(says(&r.scene, INVITE_LINE) && !says(&r.scene, INVITE_FAILED));

    // A Remove… that doesn't go through: the row stays, and the line says so.
    r.set(Action::Remove(RowRef::Member(s("m_mher"))));
    r.set(Action::ConfirmRemove);
    r.send(Event::Failed { op: Op::Remove, class: FailClass::Transport });
    assert!(says(&r.scene, REMOVE_FAILED));
    assert!(says(&r.scene, "Mher — joined"), "the row stays");
    assert!(r.scene.offers(Surface::Settings, &Action::Remove(RowRef::Member(s("m_mher")))));
    // Opened afresh, the window starts clean.
    r.send(Event::WindowClosed(WindowKind::Settings));
    let mut r = Run::new(with_popover(r.w));
    r.act(Action::OpenSettings);
    r.hear(MemberNews::Roster(roster()));
    assert!(says(&r.scene, INVITE_LINE) && !says(&r.scene, REMOVE_FAILED));
}

/// the machine's law 12, invariant 1: every failure-state this region's rows reach is its own
/// kebab-case state with its own render and its own way on.
#[test]
fn every_failure_here_is_its_own_state() {
    let rows: Vec<(&'static str, World)> = rows().into_iter().map(|(id, w, _)| (id, w)).collect();
    let bad = laws::failures_are_states(&rows, laws::failures_owned_by("region6"));
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}
