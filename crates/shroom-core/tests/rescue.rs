//! The whole-spec audit's b4-rescue cluster, walked with the laws checked at every step:
//! Save to Movies that names its file and holds every exit until its copy lands (law 2),
//! the Library's remembered list and one stranded predicate, the login item and the update
//! line telling only what's known, and the join edges' own-Mac and unreadable-link faces
//! (DECISIONS B54, DEVIATIONS #27).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::derive;
use shroom_core::effect::{Effect, LibCmd, MemberCmd, MoviesCopy, TakeCmd};
use shroom_core::event::{Event, JoinAnswer, LibraryNews, MemberNews, PeekAnswer, RecNews, StandingAnswer, Surface, UpdateNews};
use shroom_core::scene::{Action, Button, Scene};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

const HOST: &str = "v.myshroom.workers.dev";
const MINE: &str = "v.mine.workers.dev";
/// Sep 30 (the wall clock `ready()` sits near), in UTC.
const SEP30: i64 = 1_790_769_600_000;

fn says(s: &Scene, line: &str) -> bool {
    let enc = serde_json::to_string(line).unwrap();
    serde_json::to_string(s).unwrap().contains(&enc[1..enc.len() - 1])
}

fn queued(dir: &str, id: &str, title: &str, blocked: Option<Blocked>) -> Queued {
    Queued { dir: TakeDir(dir.into()), rec: RecId(id.into()), title: title.into(), stopped: Wall(SEP30), done: 1, total: 4, blocked, length_ms: Some(48_000) }
}

fn member_identity() -> SpaceIdentity {
    SpaceIdentity { role: Role::Member, member_id: "m_mher".into(), bucket: "shroom-space".into(), ..identity() }
}

fn member() -> World {
    let mut w = ready();
    w.space.identity = Some(member_identity());
    w.space.token = false;
    w.space.device_key = true;
    w
}

fn mine_owner() -> World {
    let mut w = ready();
    w.space.device_key = true;
    w.space.identity = Some(SpaceIdentity { account_id: "acct-mher".into(), subdomain: "mine".into(), host: MINE.into(), owner_name: "Mher".into(), ..identity() });
    w
}

fn settings_on(mut w: World, tab: SettingsTab) -> World {
    w.prefs.settings_tab = Some(tab);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn library_open(mut w: World) -> World {
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w
}

fn row(id: &str, title: &str, owner: Option<&str>, poster: bool) -> LibraryRow {
    LibraryRow { rec: RecId(id.into()), title: title.into(), created: Wall(SEP30), length_ms: Some(95_000), live: true, owner: owner.map(Into::into), poster }
}

fn button(s: &Scene, surface: Surface, action: &Action) -> Button {
    let all: Vec<Button> = match surface {
        Surface::Settings => s.settings.iter().flat_map(|w| w.buttons()).cloned().collect(),
        Surface::Onboarding => s.onboarding.iter().flat_map(|w| w.buttons()).cloned().collect(),
        Surface::Popover => s.popover.iter().flat_map(|p| p.buttons()).cloned().collect(),
        Surface::Library => s.library.iter().flat_map(|w| w.buttons()).cloned().collect(),
        _ => vec![],
    };
    all.into_iter().find(|b| &b.action == action).unwrap_or_else(|| panic!("no {action:?} on {surface:?}"))
}

fn saves(fx: &[Effect]) -> Vec<MoviesCopy> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::Takes(TakeCmd::SaveToMovies(c)) => Some(c.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

// ── (A) Save to Movies: named, and every exit waits for the copy (law 2) ──

/// The file is named for the take — its title, or “shroom recording — Aug 21” in this
/// Mac's calendar — and a copy that couldn't be made says so and asks again (b4 #1, #2,
/// #10, #11, #15).
#[test]
fn save_to_movies_names_the_file_and_a_failed_copy_says_so() {
    let mut w = library_open(member());
    w.queue = vec![queued("q0", "u0", "Where the 502s come from", Some(Blocked::Space))];
    let mut r = Run::new(w);
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: true }));
    r.send(Event::Act { surface: Surface::Library, action: Action::SaveRowToMovies(TakeDir("q0".into())), rev: 0 });
    assert_eq!(saves(&r.effects), vec![MoviesCopy { dir: TakeDir("q0".into()), name: "Where the 502s come from".into() }]);
    // While it's under way the row's button waits, and a second click asks nothing.
    let b = button(&r.scene, Surface::Library, &Action::SaveRowToMovies(TakeDir("q0".into())));
    assert_eq!((b.label.as_str(), b.enabled), ("Saving to Movies…", false));
    r.send(Event::Act { surface: Surface::Library, action: Action::SaveRowToMovies(TakeDir("q0".into())), rev: 0 });
    assert!(r.effects.is_empty(), "never asked twice");
    // It couldn't be made: the button says so, and asks again.
    r.send(Event::Rec(RecNews::SaveFailed(TakeDir("q0".into()))));
    let b = button(&r.scene, Surface::Library, &Action::SaveRowToMovies(TakeDir("q0".into())));
    assert_eq!((b.label.as_str(), b.enabled), ("Couldn't save to Movies — try again", true));
    assert_eq!(r.w.queue.len(), 1, "nothing was lost");
    r.send(Event::Act { surface: Surface::Library, action: Action::SaveRowToMovies(TakeDir("q0".into())), rev: 0 });
    assert_eq!(saves(&r.effects).len(), 1);
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("q0".into()))));
    assert!(r.w.members.movies.contains(&TakeDir("q0".into())) && r.w.queue.len() == 1, "saved, and it keeps waiting (N34)");

}

/// The Leave sheet's “Save them to Movies first” means first: Leave greys while the
/// copies run, and leaving deletes only after they landed (law 2, b4 #1, #10).
#[test]
fn leave_waits_for_the_copies_it_offered() {
    let mut w = settings_on(member(), SettingsTab::Space);
    w.queue = vec![queued("q0", "u0", "One", Some(Blocked::Space)), queued("q1", "u1", "Two", Some(Blocked::Space))];
    let mut r = Run::new(w);
    r.send(Event::Act { surface: Surface::Settings, action: Action::Disconnect, rev: 0 });
    r.send(Event::Act { surface: Surface::Settings, action: Action::SaveStrandedToMovies, rev: 0 });
    assert_eq!(saves(&r.effects).iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["One", "Two"]);
    assert!(!button(&r.scene, Surface::Settings, &Action::ConfirmDisconnect).enabled);
    r.send(Event::Act { surface: Surface::Settings, action: Action::ConfirmDisconnect, rev: 0 });
    assert!(r.effects.is_empty(), "nothing deleted while a copy runs");
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("q0".into()))));
    r.send(Event::Rec(RecNews::SaveFailed(TakeDir("q1".into()))));
    // One landed, one couldn't: the sheet asks again for the one, and Leave is live — the
    // sheet still says leaving deletes what never reached the space.
    let save = button(&r.scene, Surface::Settings, &Action::SaveStrandedToMovies);
    assert_eq!((save.label.as_str(), save.enabled), ("Couldn't save to Movies — try again", true));
    assert!(says(&r.scene, "Two recordings never reached the space — leaving deletes them from this Mac."));
    assert!(button(&r.scene, Surface::Settings, &Action::ConfirmDisconnect).enabled);
}

/// A take that raced into a switch is copied to Movies and deleted only once the copy
/// landed; one whose copy can't be made is set aside — kept, never sent to the new space
/// (law 2, b4 #12, DEVIATIONS #27).
#[test]
fn a_switch_deletes_nothing_before_its_copy_lands() {
    let mut w = mine_owner();
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.send(Event::Member(MemberNews::Arrived { link: SecretId(1), host: HOST.into(), remembered: false }));
    r.send(Event::Member(MemberNews::Peeked { link: SecretId(1), peek: PeekAnswer::Pending { host: HOST.into(), kind: InviteKind::Member, inviter: "Bagrat".into(), current: true } }));
    r.send(Event::Act { surface: Surface::Onboarding, action: Action::Join, rev: 0 });
    assert!(r.w.members.left_behind.is_empty(), "nothing stranded was named");
    // Two takes raced in behind the click (a stop landing as the join answered).
    r.w.queue = vec![queued("qa", "ua", "Raced A", None), queued("qb", "ub", "Raced B", None)];
    r.send(Event::Member(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Joined(member_identity()) }));
    assert!(r.w.queue.is_empty(), "nothing of the old space rides into the new one");
    assert_eq!(saves(&r.effects).iter().map(|c| c.dir.0.as_str()).collect::<Vec<_>>(), ["qa", "qb"]);
    assert!(!r.has(|e| matches!(e, Effect::Takes(TakeCmd::Delete(_)))), "no take deleted before its copy lands");
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("qa".into()))));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir("qa".into())))));
    r.send(Event::Rec(RecNews::SaveFailed(TakeDir("qb".into()))));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::SetAside(TakeDir("qb".into())))));
    assert!(!r.has(|e| matches!(e, Effect::Takes(TakeCmd::Delete(_)))));
    assert!(r.w.removed.iter().any(|q| q.dir.0 == "qb"), "kept, offered Save to Movies and Discard…");
}

// ── (C) the Library's truth ──

/// Opened cold and offline, the Library shows what this Mac remembers (b4 #4): the cache
/// is read on entry whatever the network; only the refresh waits for it.
#[test]
fn the_library_opened_offline_shows_what_this_mac_remembers() {
    let mut w = ready();
    w.net.online = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    r.send(Event::TrayClicked { right: false, quick: QuickProbe { online: Some(false), ..quick() } });
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::LoadCache)), "{:?}", r.effects);
    assert!(!r.has(|e| *e == Effect::Library(LibCmd::Refresh)), "no network offline");
    r.send(Event::Library(LibraryNews::Rows { rows: vec![row("r502", "Where the 502s come from", Some("m1"), true)], fresh: false }));
    let lib = r.scene.library.as_ref().unwrap();
    assert!(lib.faces.contains(&"library-offline"));
    assert_eq!(lib.body.rows.iter().map(|x| x.title.clone().unwrap()).collect::<Vec<_>>(), ["Where the 502s come from"]);
    assert_eq!(lib.body.rows[0].poster.as_deref(), Some("r502"), "its poster, served by the shell");
    // Back online: the refresh runs; the cache isn't read again.
    r.send(Event::Os(shroom_core::event::OsNews::NetPath(true)));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Refresh)));
    assert!(!r.has(|e| *e == Effect::Library(LibCmd::LoadCache)));
}

/// Posters (b4 #3): a take this Mac holds names its own folder (its `poster.jpg` first);
/// the space's records only when the record has one.
#[test]
fn library_rows_carry_their_posters() {
    let mut w = library_open(ready());
    w.queue.push(queued("q0", "u0", "Uploading one", None));
    w.saved.push(saved(SavedCause::Interrupted, 272_000));
    w.library = Fact::known(Arc::new(vec![row("withp", "Has one", Some("m1"), true), row("nop", "Has none", Some("m1"), false)]), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    let s = derive(&w);
    let rows = &s.library.as_ref().unwrap().body.rows;
    let poster = |key: &str| rows.iter().find(|x| x.key == key).unwrap().poster.clone();
    assert_eq!(poster("u0").as_deref(), Some("u0/q0"));
    assert_eq!(poster(&w.saved[0].dir.0).as_deref(), Some(format!("{}/{}", w.saved[0].rec.0, w.saved[0].dir.0).as_str()));
    assert_eq!(poster("withp").as_deref(), Some("withp"));
    assert_eq!(poster("nop"), None);
}

/// A row whose owner the ledger hasn't named yet shows in Everyone without a label, and
/// never in Mine (WORKER-INTERFACE §4.10, b4 #5).
#[test]
fn an_unknown_owner_is_never_mine() {
    let mut w = library_open(ready());
    w.space.device_key = true;
    w.library = Fact::known(Arc::new(vec![row("mine", "Mine for sure", Some("m1"), false), row("mher", "Mher's", Some("m_mher"), false), row("unk", "Owner not known yet", None, false)]), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    let mut r = Run::new(w);
    let titles = |s: &Scene| s.library.as_ref().unwrap().body.rows.iter().map(|x| x.title.clone().unwrap()).collect::<Vec<_>>();
    assert_eq!(titles(&r.scene), ["Mine for sure"], "Mine: only what the ledger says is mine");
    r.send(Event::Act { surface: Surface::Library, action: Action::SetScope(LibraryScope::Everyone), rev: 0 });
    let t = titles(&r.scene);
    assert!(t.contains(&"Owner not known yet".to_string()) && t.len() == 3);
    let unk = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.key == "unk").unwrap();
    assert_eq!(unk.member, None, "no label until it's known");
}

/// One predicate (b4 #8): a degraded token Mac's queued take reads “Waiting for your
/// space” in the Library and is counted on the Disconnect sheet — never “Uploading — 0%”.
#[test]
fn the_library_and_the_disconnect_sheet_agree_on_what_is_stranded() {
    let mut w = settings_on(library_open(ready()), SettingsTab::Space);
    w.space.broken = Fact::known(true, Mono(950));
    w.queue.push(queued("q0", "u0", "New take", None));
    w.library = Fact::known(Arc::new(vec![]), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    let mut r = Run::new(w);
    let state = r.scene.library.as_ref().unwrap().body.rows[0].state.clone();
    assert_eq!(state.as_deref(), Some("Waiting for your space"));
    r.send(Event::Act { surface: Surface::Settings, action: Action::Disconnect, rev: 0 });
    assert!(says(&r.scene, "One recording never reached the space — leaving deletes it from this Mac."));
    assert_eq!(derive::settings::stranded(&r.w).len(), 1);
    let rows = derive::library::rows(&r.w);
    let waiting = rows.iter().filter(|x| matches!(x.kind, derive::library::Kind::WaitingSpace(_))).count();
    assert_eq!(waiting, derive::settings::stranded(&r.w).len(), "both surfaces count the same takes");
}

/// A recovered take on a member Mac its space can't take videos from: Get link would ask
/// that space, so it greys; Save to Movies (N34's rescue) stands beside it (b4 #9).
#[test]
fn a_recovered_take_on_a_cut_off_member_mac_offers_movies() {
    let mut w = library_open(member());
    w.saved.push(saved(SavedCause::Interrupted, 272_000));
    w.mac.utc_offset_min = -120;
    w.library = Fact::known(Arc::new(vec![]), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    let mut r = Run::new(w);
    let dir = r.w.saved[0].dir.clone();
    assert!(button(&r.scene, Surface::Library, &Action::GetLinkRow(dir.clone())).enabled);
    r.send(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
    assert!(r.scene.library.as_ref().unwrap().faces.contains(&"library-unreachable"));
    assert!(!button(&r.scene, Surface::Library, &Action::GetLinkRow(dir.clone())).enabled);
    let save = button(&r.scene, Surface::Library, &Action::SaveRowToMovies(dir.clone()));
    assert_eq!((save.label.as_str(), save.enabled), ("Save to Movies", true));
    r.send(Event::Act { surface: Surface::Library, action: Action::SaveRowToMovies(dir.clone()), rev: 0 });
    // A recovered take has no title: the dated name, in this Mac's calendar (N34).
    let day = shroom_core::copy::day(r.w.saved[0].created, r.w.mac.utc_offset_min);
    assert_eq!(saves(&r.effects), vec![MoviesCopy { dir: dir.clone(), name: format!("shroom recording — {day}") }]);
    r.send(Event::Rec(RecNews::SavedToMovies(dir)));
    assert!(r.w.saved[0].movies);
}

// ── (D) Settings tells only what's known ──

/// The login box reads macOS's answer on every entry — a change made in System Settings
/// shows when Settings is entered or focused (b4 #6; the runner reads `requiresApproval`
/// as off).
#[test]
fn the_login_box_reads_the_system_on_entry() {
    let mut w = settings_on(ready(), SettingsTab::General);
    w.mac.login_item = Fact::known(true, Mono(0));
    let mut r = Run::new(w);
    let on = |s: &Scene| serde_json::to_value(s).unwrap()["settings"]["body"]["pane"]["login"]["on"].as_bool();
    assert_eq!(on(&r.scene), Some(true));
    r.send(Event::WindowFocused(WindowKind::Settings, QuickProbe { login_item: Some(false), ..quick() }));
    assert_eq!(on(&r.scene), Some(false), "switched off in System Settings: the box says so");
    r.send(Event::WindowFocused(WindowKind::Settings, QuickProbe { login_item: None, ..quick() }));
    assert_eq!(on(&r.scene), Some(false), "an unreadable answer changes nothing");
}

/// “Up to date” only when a check said so (b4 #7): unknown reads the bare version, a
/// failed update its own line.
#[test]
fn general_says_up_to_date_only_when_known() {
    let w = settings_on(ready(), SettingsTab::General);
    let v = w.launch.version.clone();
    let mut r = Run::new(w);
    assert!(says(&r.scene, &format!("shroom {v}.")), "nothing known yet");
    assert!(!says(&r.scene, "up to date"));
    r.send(Event::Update(UpdateNews::Checked(None)));
    assert!(says(&r.scene, &format!("shroom {v} — up to date.")));
    let mut w = settings_on(ready(), SettingsTab::General);
    w.update.failed = true;
    w.update.checked = Some(Mono(500));
    let s = derive(&w);
    assert!(says(&s, "The update didn't finish — shroom will try again later."));
    assert!(!says(&s, "up to date"));
}

// ── (E) join edges ──

/// The installer's hand-off that isn't a join link opens the nameless dead-link face —
/// the paste door's answer for the same text — never nothing at all (b4 #13).
#[test]
fn an_unreadable_installer_invite_opens_the_dead_link_face() {
    let mut r = Run::new(World::blank());
    r.send(Event::Member(MemberNews::Unreadable { link: SecretId(3), opened: true }));
    let o = r.scene.onboarding.as_ref().expect("a window opens");
    assert_eq!(o.id, "join-expired", "{:?}", r.ids());
    assert!(says(&r.scene, "This invite link doesn't work — ask whoever sent it for a fresh one."));
    assert!(says(&r.scene, "Have a new link? Paste it here."));
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert!(r.has(|e| *e == Effect::Member(MemberCmd::ForgetInvite)), "closing its face is the “shown once”");
}

/// Own-Mac links never put the owner's name on the owner's screen (b4 #14): a used one
/// reads the page's own-Mac words, and a space not answering is “Your space”.
#[test]
fn own_mac_links_speak_to_the_owner() {
    let mut r = Run::new(World::blank());
    r.send(Event::Member(MemberNews::Arrived { link: SecretId(1), host: HOST.into(), remembered: false }));
    r.send(Event::Member(MemberNews::Peeked { link: SecretId(1), peek: PeekAnswer::Used { host: HOST.into(), kind: InviteKind::OwnMac, inviter: "Bagrat".into() } }));
    assert!(says(&r.scene, "This link was already used — make a fresh one from Members on your other Mac."));
    assert!(!says(&r.scene, "ask Bagrat"));

    let mut r = Run::new(World::blank());
    r.send(Event::Member(MemberNews::Arrived { link: SecretId(1), host: HOST.into(), remembered: false }));
    r.send(Event::Member(MemberNews::Peeked { link: SecretId(1), peek: PeekAnswer::Pending { host: HOST.into(), kind: InviteKind::OwnMac, inviter: "Bagrat".into(), current: true } }));
    r.send(Event::Act { surface: Surface::Onboarding, action: Action::Join, rev: 0 });
    r.send(Event::Failed { op: Op::Join, class: FailClass::Transport });
    assert!(says(&r.scene, "Your space isn't answering right now — I'll keep trying."), "{:?}", r.ids());
    assert!(!says(&r.scene, "Bagrat's space isn't answering"));
}
