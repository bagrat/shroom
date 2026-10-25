//! The onboarding gallery: one World per onboarding inventory row (the worlds of
//! `tests/onboarding.rs`'s inventory, plus a few in-between faces worth a look), derived,
//! printed as a JSON array `[{state, variant, faces, scene}]` where `scene` is the
//! onboarding window's slice exactly as the webview receives it.
//!
//! `cargo run -q -p shroom-core --example gallery > gallery.json` — the app's gallery test
//! (app/test/onboarding-gallery.mjs) renders every entry in WebKit.
//!
//! `… --example gallery -- library` and `… -- settings` print the Library and Settings
//! windows the same way (`scene` = that window's slice), from the worlds of
//! `tests/region5.rs`'s inventory. With no argument the output is the onboarding gallery,
//! unchanged.
//!
//! `… --example gallery -- members` prints Region 6's faces from the worlds of
//! `tests/region6.rs` — the Members tab and its sheets, the invite window (member-welcome,
//! joining, the dead and the switching invites, the member plan), and the member variants of
//! Settings · Space, the Library and the popover's banners — each entry with its `window`
//! (`settings`, `onboarding`, `library` or `popover`), so `-- library` and `-- settings`
//! stay exactly what their webviews render today. Those worlds are fed through `update`,
//! as the tests build them.
//!
//! `… --example gallery -- region3` prints Region 3's faces: every popover state of the
//! recording machine (config and its variants, disk-low, recording-off, the paused faces,
//! the questions, the stops) and the countdown, each with its `window` (`popover` or
//! `countdown`). The bubble and Camera's preview are native panels (no webview to render).
//! The paused face shows Drop a marker, and again once one is dropped (N64). `config` says,
//! once, why the owner page's Edit in shroom opened nothing (N69).
//!
//! `… --example gallery -- editor` prints the editor's faces (N63), from the worlds of
//! `tests/region5.rs`'s editor rows: ready on a trimmed video with a cut applied, a
//! suggestion waiting, markers and a step to undo; just opened; opened again while the
//! earlier edit's original is kept, with each of the time-left lines by Revert; the video
//! coming at 40%; couldn't come; saving; the save that didn't go through, and the one the
//! space turned down (B199); and after a save (B197), the final video being made at 40%, the
//! one that couldn't be made, and “Delete the original?”; and the window brought forward by the
//! owner page's Edit in shroom for another video (N69).
//!
//! Examples can't import tests, so the builders are copied from
//! `tests/common/support.rs` and `tests/onboarding.rs`; keep them in step.

use std::sync::Arc;

use std::collections::BTreeMap;

use serde_json::{Value, json};
use shroom_core::event::{Event, JoinAnswer, MemberNews, PeekAnswer, SettingsSpot, StandingAnswer, Surface};
use shroom_core::scene::{Action, RowRef};
use shroom_core::time::{Envelope, Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{derive, update};

// ── from tests/common/support.rs ──

fn quick() -> QuickProbe {
    QuickProbe {
        screen: true,
        mic: Access::Granted,
        camera: Access::Granted,
        displays: vec![Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 }],
        mics: vec![Device { uid: "BuiltInMicrophoneDevice".into(), name: "MacBook Pro Microphone".into(), builtin: true, continuity: false, system_default: false }],
        cameras: vec![Device { uid: "cam".into(), name: "FaceTime HD Camera".into(), builtin: true, continuity: false, system_default: false }],
        disk_free: 200_000_000_000,
        online: Some(true),
        os: OsSession::default(),
        status_item_visible: true,
        login_item: None,
        clicked_display: None,
    }
}

fn identity() -> SpaceIdentity {
    SpaceIdentity {
        account_id: "acct".into(),
        label: "v".into(),
        subdomain: "myshroom".into(),
        host: "v.myshroom.workers.dev".into(),
        bucket: "shroom".into(),
        role: Role::TokenHolder,
        member_id: "m1".into(),
        owner_name: "Bagrat".into(),
        account_name: None,
        foreign_v: false,
    }
}

fn ready() -> World {
    let mut w = World::blank();
    w.space.identity = Some(identity());
    w.space.token = true;
    w.mac.primed = Some(Wall(1));
    let q = quick();
    let at = Mono(0);
    w.mac.screen = Fact::known(q.screen, at);
    w.mac.mic = Fact::known(q.mic, at);
    w.mac.camera = Fact::known(q.camera, at);
    w.mac.displays = Fact::known(q.displays, at);
    w.mac.mics = Fact::known(q.mics, at);
    w.mac.cameras = Fact::known(q.cameras, at);
    w.mac.disk_free = Fact::known(q.disk_free, at);
    w.mac.os = Fact::known(q.os, at);
    w.net.online = Fact::known(true, at);
    w.now = Mono(1_000);
    w.wall = Wall(1_790_000_000_000);
    w
}

// ── from tests/onboarding.rs ──

fn bagrat() -> Account {
    Account { id: "bagrat".into(), name: "Bagrat's Account".into() }
}

fn acme() -> Account {
    Account { id: "acme".into(), name: "Acme Engineering".into() }
}

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

/// The name row ticked: the permissions are the step, wiring underneath (N45).
fn named() -> World {
    let mut w = gated(false, false);
    w.sweep.subdomain = Fact::known(Some("myshroom".into()), Mono(950));
    w.sweep.found = Fact::known(None, Mono(950));
    w
}

/// The last step since N45: the permissions done, wiring not yet.
fn at_wiring() -> World {
    let mut w = named();
    w.mac.primed = Some(Wall(1_790_000_000_000));
    w.mac.screen = Fact::known(true, Mono(0));
    w.mac.mic = Fact::known(Access::Granted, Mono(0));
    w.mac.camera = Fact::known(Access::Granted, Mono(0));
    w
}

fn at_done() -> World {
    let mut w = window(unprovisioned());
    w.space.identity = Some(identity());
    w.space.token = true;
    w
}

fn chapter(mic: Access, screen: bool) -> World {
    let mut w = at_done();
    w.session.chapter = true;
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

/// The inventory's window rows (the tray-only rows `idle-unprovisioned` and `idle-ready`
/// have no window to draw), each with a variant name for its screenshot.
fn rows() -> Vec<(&'static str, &'static str, World)> {
    let mut rows: Vec<(&'static str, &'static str, World)> = Vec::new();
    rows.push(("welcome", "welcome", window(unprovisioned())));
    // Uninstall shroom… over the welcome (B160).
    rows.push(("welcome", "welcome-uninstall", {
        let mut w = window(unprovisioned());
        w.session.confirm = Some(Confirm::Uninstall { going: false });
        w
    }));
    rows.push(("first-launch", "first-launch", {
        let mut w = window(unprovisioned());
        w.launch.kind = LaunchKind::Install { join: false };
        w.launch.booted = Mono(900);
        w
    }));
    rows.push(("account", "account", plan()));
    rows.push(("account-signing-up", "account-signing-up", {
        let mut w = plan();
        w.session.fork = Fork::SigningUp;
        w
    }));
    rows.push(("token", "token", affirmed()));
    rows.push(("token", "token-signed-up", {
        let mut w = affirmed();
        w.session.fields.signed_up = true;
        w
    }));
    rows.push(("token", "token-checking", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::Checking(SecretId(3)));
        w
    }));
    // Accepted, being put away: still held, the field's dots still showing (D:B104).
    rows.push(("token", "token-saving", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::Saving { id: SecretId(3), account: bagrat(), several: false });
        w
    }));
    rows.push(("token-garbled", "token-garbled", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::Garbled);
        w
    }));
    rows.push(("token-wrong-powers", "token-wrong-powers", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::WrongPowers);
        w
    }));
    rows.push(("token-pick-account", "token-pick-account", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::PickAccount { id: SecretId(7), accounts: vec![bagrat(), acme()] });
        w
    }));
    rows.push(("token-pick-account", "token-pick-account-picked", {
        let mut w = affirmed();
        w.sweep.paste = Some(Paste::PickAccount { id: SecretId(7), accounts: vec![bagrat(), acme()] });
        w.session.fields.account = Some("bagrat".into());
        w
    }));
    rows.push(("card", "card", token_saved()));
    // N74: the check's parts as they answer — storage underway, two seconds in.
    rows.push(("card", "card-check", {
        let mut w = token_saved();
        w.sweep.check = Some(Progress { part: 3, since: Mono(1_000) });
        w.now = Mono(3_000);
        w
    }));
    rows.push(("gate-email", "gate-email", gated(true, true)));
    rows.push(("gate-card", "gate-card", gated(false, true)));
    rows.push(("gate-card", "gate-card-opened", {
        let mut w = gated(false, true);
        w.session.fields.billing_opened = true;
        w
    }));
    // N74: the watch with its clock, and from 45 s its quiet line.
    for (name, ms) in [("gate-card-watching", 24_000), ("gate-card-long", 70_000)] {
        rows.push(("gate-card", name, {
            let mut w = gated(false, true);
            w.session.fields.billing_opened = true;
            w.session.fields.billing_at = Some(Mono(1_000));
            w.now = Mono(1_000 + ms);
            w
        }));
    }
    rows.push(("name", "name", at_name()));
    rows.push(("name", "name-typed", {
        let mut w = at_name();
        w.session.fields.name = "myshroom".into();
        w
    }));
    // The longest name the claim accepts (63), beside the v-clash label: the widest field.
    rows.push(("name-v-clash", "name-longest", {
        let mut w = at_name();
        w.sweep.foreign_v = Fact::known(true, Mono(950));
        w.sweep.label = Fact::known(Some("shroom".into()), Mono(950));
        w.session.fields.name = "a".repeat(63);
        w
    }));
    rows.push(("name", "name-hint", {
        let mut w = at_name();
        w.session.fields.name_hint = Some(Mono(990));
        w
    }));
    rows.push(("name-taken", "name-taken", {
        let mut w = at_name();
        w.session.fields.name = "myshroom".into();
        w.sweep.taken = Some("myshroom".into());
        w
    }));
    rows.push(("name-v-clash", "name-v-clash", {
        let mut w = at_name();
        w.sweep.foreign_v = Fact::known(true, Mono(950));
        w.sweep.label = Fact::known(Some("shroom".into()), Mono(950));
        w
    }));
    rows.push(("wiring", "wiring", at_wiring()));
    // N74: wiring's parts, switching the name on with its clock, and past 3 min its line.
    for (name, ms) in [("wiring-parts", 48_000), ("wiring-long", 252_000)] {
        rows.push(("wiring", name, {
            let mut w = at_wiring();
            w.sweep.wiring = Some(Progress { part: 3, since: Mono(1_000) });
            w.now = Mono(1_000 + ms);
            w
        }));
    }
    rows.push(("wizard-offline", "wizard-offline", {
        let mut w = at_wiring();
        w.net.online = Fact::known(false, Mono(0));
        w
    }));
    // A failure nothing else classifies (DEVIATIONS #25): its face under the wiring row.
    rows.push(("wizard-unexpected", "wizard-unexpected", {
        let mut w = at_wiring();
        w.inflight.failures.insert(Op::Wire, Failure { at: Mono(990), class: FailClass::Refused, tries: 0 });
        w
    }));
    rows.push(("space-found", "space-found", {
        let mut w = at_wiring();
        w.sweep.found = Fact::known(Some(14), Mono(950));
        w
    }));
    rows.push(("done", "done", at_done()));
    rows.push(("done", "done-wiring-underneath", named()));
    // N74: the working row names the part underway, with its clock.
    rows.push(("done", "done-wiring-part", {
        let mut w = named();
        w.sweep.wiring = Some(Progress { part: 3, since: Mono(1_000) });
        w.now = Mono(49_000);
        w
    }));
    rows.push(("perm-mic", "perm-mic", chapter(Access::NotAsked, false)));
    rows.push(("perm-camera", "perm-camera", {
        let mut w = chapter(Access::Granted, false);
        w.mac.camera_asked = true;
        w.mac.camera = Fact::known(Access::NotAsked, Mono(0));
        w
    }));
    rows.push(("perm-screen", "perm-screen", chapter(Access::Granted, false)));
    rows.push(("perm-screen", "perm-screen-asking", {
        let mut w = chapter(Access::Granted, false);
        w.mac.screen_alert = Some(true);
        w
    }));
    rows.push(("relaunching", "relaunching", {
        let mut w = chapter(Access::Granted, false);
        w.launch.ending = true;
        w
    }));
    rows.push(("perm-priming", "perm-priming", chapter(Access::Granted, true)));
    rows.push(("all-set", "all-set", all_set(Access::Granted)));
    rows.push(("all-set", "all-set-mic-off", all_set(Access::Denied)));
    rows
}

// ── from tests/region5.rs ──

const AUG21: i64 = 1_787_313_600_000;
const AUG19: i64 = 1_787_140_800_000;
const AUG03: i64 = 1_785_758_400_000;
const SEP30: i64 = 1_790_769_600_000;

fn remote(id: &str, title: &str, at: i64, length_ms: u64) -> LibraryRow {
    LibraryRow { rec: RecId(id.into()), title: title.into(), created: Wall(at), length_ms: Some(length_ms), live: true, owner: Some("m1".into()), poster: false, edited_ms: None, pinned: false }
}

fn space_rows() -> Vec<LibraryRow> {
    vec![
        remote("r502", "Where the 502s come from", AUG21, 272_000),
        remote("r19", "Untitled — Aug 19", AUG19, 48_000),
        remote("onb", "Onboarding flow, take 2", AUG03, 730_000),
    ]
}

/// A take open (copied from `tests/common/support.rs`).
fn recording() -> OpenTake {
    OpenTake {
        dir: TakeDir("20261002-120000-ab12".into()),
        rec: RecId("AbCdEfGhIjKlMnOp".into()),
        opened: Mono(500),
        source: SourceId::Display(1),
        phase: Phase::Recording,
        since: Mono(500),
        length_ms: 30_000,
        markers: 0,
        marked_at_pause: false,
        toast: None,
        mic: None,
        camera: None,
    }
}

/// Every kind of row at once: uploading, waiting, stranded, recovered, live — one copied,
/// one selected.
fn every_row() -> World {
    let mut w = library(ready(), space_rows());
    w.queue.push(queued("q40", "up40", "Untitled — Sep 30", 2, None));
    w.queue.push(queued("qoff", "upoff", "Demo for Mher", 1, Some(Blocked::Offline)));
    w.queue.push(queued("qspace", "upspace", "Design review", 1, Some(Blocked::Space)));
    w.saved.push(SavedTake {
        dir: TakeDir("20261001-090000-cd34".into()),
        rec: RecId("QrStUvWxYz012345".into()),
        length_ms: 272_000,
        cause: SavedCause::Interrupted,
        created: Wall(SEP30),
        movies: false,
    });
    w.session.copied = Some((CopyTarget::Link(RecId("r502".into())), Mono(990)));
    w.session.fields.library.selected = vec![RecId("r19".into())];
    w
}

fn library(mut w: World, rows: Vec<LibraryRow>) -> World {
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(Arc::new(rows), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w
}

fn queued(dir: &str, id: &str, title: &str, done: u32, blocked: Option<Blocked>) -> Queued {
    Queued { dir: TakeDir(dir.into()), rec: RecId(id.into()), title: title.into(), stopped: Wall(SEP30), done, total: 5, blocked, length_ms: Some(48_000) }
}

fn library_rows() -> Vec<(&'static str, &'static str, World)> {
    let mut rows = Vec::new();
    rows.push(("library", "library", library(ready(), space_rows())));
    rows.push(("library", "library-every-row", every_row()));
    // A window opened before a take stays through it (N35): what would put something new
    // on screen greys with law 4's line.
    rows.push(("library", "library-mid-take", {
        let mut w = every_row();
        w.session.windows.insert(WindowKind::Library, Door::asked(Mono(100)));
        w.take = Some(recording());
        w
    }));
    // An update applying: Get link waits with its line.
    rows.push(("library", "library-updating", {
        let mut w = every_row();
        w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: "1.3.0".into() }), Mono(950));
        w.update.staged = Some("1.3.0".into());
        w.update.applying = true;
        w
    }));
    rows.push(("library", "library-renaming", {
        let mut w = library(ready(), space_rows());
        w.session.fields.library.renaming = Some((RecId("r502".into()), "Where the 502s come from".into()));
        w
    }));
    rows.push(("library-empty", "library-empty", library(ready(), vec![])));
    rows.push(("library-no-match", "library-no-match", {
        let mut w = library(ready(), space_rows());
        w.session.fields.library.search = "zzz".into();
        w
    }));
    rows.push(("library-offline", "library-offline", {
        let mut w = library(ready(), space_rows());
        w.net.online = Fact::known(false, Mono(0));
        w
    }));
    // Suggested to clean up (N68): the line above the rows, the view, its Undo line, and
    // the view once nothing is left.
    rows.push(("library", "library-cleanup-line", suggesting(library(ready(), space_rows()))));
    rows.push(("library-cleanup", "library-cleanup", {
        let mut w = suggesting(library(ready(), space_rows()));
        w.session.fields.library.cleanup = true;
        w
    }));
    rows.push(("library-cleanup", "library-cleanup-undo", {
        let mut w = suggesting(library(ready(), space_rows()));
        w.session.fields.library.cleanup = true;
        w.session.fields.library.undo = Some((vec![RecId("r19".into())], Mono(990)));
        w
    }));
    rows.push(("library-cleanup", "library-cleanup-empty", {
        let mut w = library(ready(), space_rows());
        w.suggestions = Fact::known(Suggestions { counting: true, space_bytes: 34_000_000_000, rows: vec![], ..Default::default() }, Mono(960));
        w.session.fields.library.cleanup = true;
        w
    }));
    rows.push(("delete-confirm", "library-delete-sheet", {
        let mut w = library(ready(), space_rows());
        w.session.confirm = Some(Confirm::Delete(vec![RecId("r502".into())]));
        w
    }));
    // N75: a pinned row and its menu's Unpin; auto-cleanup's line and the going rows; the
    // view with what's going first; Delete all's sheet and its one Undo line.
    rows.push(("library", "library-pinned", pinned(library(ready(), space_rows()), "onb")));
    rows.push(("library", "library-going", going(suggesting(library(ready(), space_rows())))));
    rows.push(("library", "library-cleaned", {
        let mut w = going(suggesting(library(ready(), space_rows())));
        if let Fact::Known { v, .. } = &mut w.suggestions {
            v.going.clear();
        }
        w
    }));
    rows.push(("library-cleanup", "library-cleanup-going", {
        let mut w = going(suggesting(library(ready(), space_rows())));
        w.session.fields.library.cleanup = true;
        w
    }));
    rows.push(("library-cleanup", "library-cleanup-undo-all", {
        let mut w = suggesting(library(ready(), space_rows()));
        w.session.fields.library.cleanup = true;
        w.session.fields.library.undo = Some((["onb", "r19", "r502"].map(|r| RecId(r.into())).to_vec(), Mono(990)));
        w
    }));
    rows.push(("delete-confirm-cleanup", "library-delete-all-sheet", {
        let mut w = suggesting(library(ready(), space_rows()));
        w.session.fields.library.cleanup = true;
        w.session.confirm = Some(Confirm::DeleteSuggested(["onb", "r19", "r502"].map(|r| RecId(r.into())).to_vec()));
        w
    }));
    rows.push(("delete-confirm-several", "library-delete-sheet-several", {
        let mut w = library(ready(), space_rows());
        w.session.confirm = Some(Confirm::Delete(["r502", "r19", "onb"].map(|r| RecId(r.into())).to_vec()));
        w
    }));
    rows
}

/// The space's videos known on a Mac whose Library isn't open (the popover's banner, N75).
fn library_closed(mut w: World) -> World {
    w.library = Fact::known(std::sync::Arc::new(space_rows()), Mono(950));
    w
}

/// A row pinned (N75).
fn pinned(mut w: World, id: &str) -> World {
    if let Fact::Known { v, .. } = &mut w.library {
        if let Some(r) = std::sync::Arc::make_mut(v).iter_mut().find(|r| r.rec.0 == id) {
            r.pinned = true;
        }
    }
    w
}

/// Auto-cleanup at 30 days (N75): two of the space's videos going in three days, and the
/// round before it.
fn going(mut w: World) -> World {
    let due = Wall(w.wall.0 + 3 * 86_400_000);
    if let Fact::Known { v, .. } = &mut w.suggestions {
        v.autoclean = AutoClean { days: 30, last: Some(Cleaned { at: Wall(w.wall.0 - 86_400_000), recs: vec![RecId("old1".into()), RecId("old2".into())], bytes: 1_700_000_000 }) };
        v.going = vec![
            Going { rec: RecId("r19".into()), bytes: 640_000_000, plays: 2, quiet_days: 31, stopped_early: false, deletes_at: due },
            Going { rec: RecId("r502".into()), bytes: 1_100_000_000, plays: 0, quiet_days: 33, stopped_early: false, deletes_at: due },
        ];
        v.idle = vec![
            Idle { rec: RecId("r19".into()), quiet_days: 31 },
            Idle { rec: RecId("r502".into()), quiet_days: 33 },
            Idle { rec: RecId("onb".into()), quiet_days: 300 },
        ];
    }
    w
}

/// What the space said is worth deleting (N68): three of the space's videos, of 34 GB.
fn suggesting(mut w: World) -> World {
    let s = |id: &str, bytes: u64, plays: u32, quiet_days: u32| Suggestion { rec: RecId(id.into()), bytes, plays, quiet_days, stopped_early: false };
    w.suggestions = Fact::known(
        Suggestions { counting: true, space_bytes: 34_000_000_000, rows: vec![s("onb", 3_200_000_000, 4, 300), s("r19", 1_800_000_000, 0, 45), s("r502", 1_300_000_000, 2, 120)], ..Default::default() },
        Mono(960),
    );
    w
}

fn settings(mut w: World, tab: SettingsTab) -> World {
    w.prefs.settings_tab = Some(tab);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn inspected(mut w: World) -> World {
    let at = Mono(950);
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "Bagrat's Account".into() }, several: false }, at);
    w.sweep.gates = Fact::known(Gates { email: false, card: false }, at);
    w.sweep.subdomain = Fact::known(Some("myshroom".into()), at);
    w.sweep.label = Fact::known(Some("v".into()), at);
    w.sweep.bucket = Fact::known(true, at);
    w.sweep.worker = Fact::known(WorkerFact::Ours, at);
    w.sweep.pass = Some(at);
    w
}

fn space() -> World {
    inspected(settings(ready(), SettingsTab::Space))
}

fn settings_rows() -> Vec<(&'static str, &'static str, World)> {
    let mut rows = Vec::new();
    rows.push(("settings-general", "general-up-to-date", {
        let mut w = settings(ready(), SettingsTab::General);
        w.launch.version = "1.2.0".into();
        w
    }));
    // Your videos (N68): the space past the free 10 GB; and inside it, counting off, the
    // switch's failed line.
    rows.push(("settings-general", "general-your-videos", suggesting(settings(ready(), SettingsTab::General))));
    rows.push(("settings-general", "general-your-videos-free", {
        let mut w = settings(ready(), SettingsTab::General);
        w.suggestions = Fact::known(Suggestions { counting: false, space_bytes: 8_200_000_000, rows: vec![], ..Default::default() }, Mono(960));
        w.inflight.failures.insert(Op::Plays, Failure { at: Mono(970), class: FailClass::Transport, tries: 1 });
        w
    }));
    // N75: auto-cleanup on the setup Mac, on another Mac, and turning it on.
    rows.push(("settings-general", "general-autoclean", going(suggesting(settings(ready(), SettingsTab::General)))));
    rows.push(("settings-general", "general-autoclean-other-mac", {
        let mut w = going(suggesting(settings(ready(), SettingsTab::General)));
        w.space.token = false;
        w
    }));
    rows.push(("autoclean-confirm", "general-autoclean-confirm", {
        let mut w = suggesting(settings(ready(), SettingsTab::General));
        w = going(w);
        if let Fact::Known { v, .. } = &mut w.suggestions {
            v.autoclean.days = 0;
        }
        w.session.confirm = Some(Confirm::Autoclean(30));
        w
    }));
    rows.push(("settings-general", "general-update-ready", {
        let mut w = settings(ready(), SettingsTab::General);
        w.launch.version = "1.2.0".into();
        w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: "1.3.0".into() }), Mono(950));
        w.update.staged = Some("1.3.0".into());
        w
    }));
    rows.push(("settings-general", "general-needs-macos", {
        let mut w = settings(ready(), SettingsTab::General);
        w.launch.version = "1.3.2".into();
        w.update.offer = Fact::known(Some(UpdateOffer::NeedsMacos { version: "1.4.0".into(), macos: 15 }), Mono(950));
        w
    }));
    // The shroom command's settings link (N62, DECISIONS B216): General, the switch's row lit.
    rows.push(("settings-general", "general-agent-lit", {
        let mut w = settings(ready(), SettingsTab::General);
        w.launch.version = "1.2.0".into();
        w.session.fields.settings.lit = Some((SettingsSpot::AgentRecording, w.now));
        w
    }));
    rows.push(("settings-space-inspection", "space-inspection", space()));
    // N66: naming's switch on the setup Mac, off and on, and a re-wiring that didn't go through.
    rows.push(("settings-space", "space-ai-off", {
        let mut w = space();
        w.space.ai = Some(false);
        w
    }));
    rows.push(("settings-space", "space-ai-on", {
        let mut w = space();
        w.space.ai = Some(true);
        w
    }));
    rows.push(("settings-space", "space-ai-failed", {
        let mut w = space();
        w.space.ai = Some(true);
        w.inflight.failures.insert(Op::Ai, Failure { at: Mono(990), class: FailClass::Transport, tries: 1 });
        w
    }));
    rows.push(("settings-space", "space-mic-off", {
        let mut w = space();
        w.mac.mic = Fact::known(Access::Denied, Mono(0));
        w
    }));
    rows.push(("settings-space", "space-screen-off", {
        let mut w = space();
        w.mac.screen = Fact::known(false, Mono(0));
        w
    }));
    rows.push(("token-repair-mood", "repair-token", {
        let mut w = space();
        w.sweep.token = Fact::known(TokenVerdict::Revoked, Mono(950));
        w
    }));
    // A fresh token pasted in the repair mood, being checked: the field holds it (D:B104).
    rows.push(("token", "repair-token-checking", {
        let mut w = space();
        w.sweep.token = Fact::known(TokenVerdict::Revoked, Mono(950));
        w.sweep.paste = Some(Paste::Checking(SecretId(3)));
        w
    }));
    rows.push(("token-other-account", "repair-other-account", {
        let mut w = space();
        w.sweep.token = Fact::known(TokenVerdict::Revoked, Mono(950));
        w.sweep.paste = Some(Paste::OtherAccount);
        w
    }));
    rows.push(("gate-card", "repair-gate-card", {
        let mut w = space();
        w.sweep.gates = Fact::known(Gates { email: false, card: true }, Mono(950));
        w
    }));
    rows.push(("wiring", "repair-wiring", {
        let mut w = space();
        w.sweep.worker = Fact::known(WorkerFact::KeyGone, Mono(950));
        w
    }));
    rows.push(("space-storage-gone", "repair-storage-gone", {
        let mut w = space();
        w.sweep.bucket = Fact::known(false, Mono(950));
        w
    }));
    rows.push(("name-changed", "repair-name-changed", {
        let mut w = space();
        w.sweep.subdomain = Fact::known(Some("myshroomhq".into()), Mono(950));
        w
    }));
    rows.push(("disconnect-confirm", "disconnect-sheet", {
        let mut w = space();
        w.session.confirm = Some(Confirm::Exit);
        w
    }));
    // A take the space refused is stranded: it never blocks, and the sheet offers Movies
    // first (DEVIATIONS #17) — three buttons.
    rows.push(("disconnect-confirm", "disconnect-sheet-stranded", {
        let mut w = space();
        w.session.confirm = Some(Confirm::Exit);
        w.queue.push(queued("qs", "us", "Design review", 1, Some(Blocked::Space)));
        w
    }));
    rows.push(("disconnect-confirm", "disconnect-sheet-stranded-mid-take", {
        let mut w = space();
        w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
        w.session.confirm = Some(Confirm::Exit);
        w.queue.push(queued("qs", "us", "Design review", 1, Some(Blocked::Space)));
        w.take = Some(recording());
        w
    }));
    // Uninstall shroom… (B135): the sheet, and its cases as the boards draw them.
    let uninstall = |going: bool| {
        let mut w = settings(ready(), SettingsTab::General);
        w.launch.version = "1.2.0".into();
        w.session.confirm = Some(Confirm::Uninstall { going });
        w
    };
    rows.push(("uninstall-confirm", "uninstall-sheet", uninstall(false)));
    rows.push(("uninstall-confirm", "uninstall-sheet-member", {
        let mut w = uninstall(false);
        w.space.identity.as_mut().unwrap().role = Role::Member;
        w
    }));
    rows.push(("uninstall-confirm", "uninstall-sheet-stranded", {
        let mut w = uninstall(false);
        w.queue.push(queued("q1", "u1", "One", 1, Some(Blocked::Space)));
        w.queue.push(queued("q2", "u2", "Two", 1, Some(Blocked::Space)));
        w
    }));
    rows.push(("uninstall-confirm", "uninstall-sheet-blocked", {
        let mut w = uninstall(false);
        w.queue.push(queued("q1", "u1", "One", 1, None));
        w.queue.push(queued("q2", "u2", "Two", 1, None));
        w
    }));
    rows.push(("uninstall-confirm", "uninstall-sheet-going", uninstall(true)));
    // ── Report a problem (B180) ──
    let reporting = |sending: bool, gathered: bool, words: &str, failed: bool| {
        let mut w = uninstall(false);
        w.session.confirm = Some(Confirm::Report { sending });
        let f = &mut w.session.fields.settings.report;
        f.words = words.into();
        f.reply_to = if words.is_empty() { String::new() } else { "you@example.com".into() };
        f.failed = failed;
        f.gathered = gathered.then(|| shroom_core::report::Diagnostics {
            shroom: "1.3.0".into(),
            macos: "15.3.1".into(),
            chip: "Apple M2".into(),
            memory: "16 GB".into(),
            setup: "2026-10-12 10:06 UTC key · 10:14 recording allowed · 10:15 wired".into(),
            role: "owns its space".into(),
            log: shroom_core::report::clean_log(
                "    1840 core config [Probe, Draw]\n  412900 recorder finalize failed: the screen stream ended early\n  413020 uploader 7f3a…: request to https://v.example-owner.workers.dev/x timed out",
            ),
        });
        w
    };
    let words = "I stopped a recording and the link never finished uploading.";
    rows.push(("report-problem", "report-sheet", reporting(false, false, "", false)));
    rows.push(("report-problem", "report-sheet-filled", reporting(false, true, words, false)));
    rows.push(("report-problem", "report-sheet-sending", reporting(true, true, words, false)));
    rows.push(("report-problem", "report-sheet-failed", reporting(false, true, words, true)));
    rows.push(("settings-general", "settings-general-sent", {
        let mut w = uninstall(false);
        w.session.confirm = None;
        w.session.fields.settings.sent = true;
        w
    }));
    rows.push(("disconnect-confirm", "disconnect-sheet-blocked", {
        let mut w = space();
        w.session.confirm = Some(Confirm::Exit);
        w.queue.push(queued("q1", "u1", "One", 1, None));
        w.queue.push(queued("q2", "u2", "Two", 1, None));
        w
    }));
    rows
}

// ── Region 6 (from tests/region6.rs) ──

const HOST: &str = "v.myshroom.workers.dev";
const DAY: i64 = 86_400_000;
const JUL20: i64 = 1_784_548_800_000;
const AUG10: i64 = 1_786_363_200_000;
const AUG23: i64 = 1_787_486_400_000;
const SEP02: i64 = 1_788_350_400_000;
const WALL0: i64 = 1_790_000_000_000;

fn id_as(role: Role, member: &str) -> SpaceIdentity {
    SpaceIdentity { role, member_id: member.into(), bucket: "shroom-space".into(), ..identity() }
}

fn owner() -> World {
    let mut w = ready();
    w.space.device_key = true;
    w
}

fn owner_class() -> World {
    let mut w = ready();
    w.space.identity = Some(id_as(Role::OwnerClass, "m1"));
    w.space.token = false;
    w.space.device_key = true;
    w
}

fn member() -> World {
    let mut w = ready();
    w.space.identity = Some(id_as(Role::Member, "m_mher"));
    w.space.token = false;
    w.space.device_key = true;
    w
}

fn mine_owner() -> World {
    let mut w = owner();
    w.space.identity = Some(SpaceIdentity { account_id: "acct-mher".into(), subdomain: "mine".into(), host: "v.mine.workers.dev".into(), owner_name: "Mher".into(), ..identity() });
    w
}

fn popover(mut w: World) -> World {
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

/// The events, through `update`, stamped as the tests stamp them.
fn feed(mut w: World, events: impl IntoIterator<Item = Event>) -> World {
    let mut t = w.now.0;
    for event in events {
        t += 10;
        update(&mut w, Envelope { mono: Mono(t), wall: Wall(WALL0 + t as i64), event });
    }
    w
}

fn news(n: MemberNews) -> Event {
    Event::Member(n)
}

fn act(surface: Surface, action: Action) -> Event {
    Event::Act { surface, action, rev: 0 }
}

fn arrived(link: u32) -> Event {
    news(MemberNews::Arrived { link: SecretId(link), host: HOST.into(), remembered: false })
}

fn pending(link: u32, kind: InviteKind) -> Event {
    news(MemberNews::Peeked { link: SecretId(link), peek: PeekAnswer::Pending { host: HOST.into(), kind, inviter: "Bagrat".into(), current: true } })
}

fn standing(a: StandingAnswer) -> Event {
    news(MemberNews::Standing(a))
}

fn fine() -> StandingAnswer {
    StandingAnswer::Fine { current: true, only_holder: Some(false), mark: Some(true), ai: None }
}

fn struck(w: World, a: StandingAnswer) -> World {
    let mut w = feed(w, [standing(a.clone())]);
    w.now = w.now.plus(31_000);
    feed(w, [standing(a)])
}

fn invite_row(id: &str, label: &str, created: i64) -> PendingInvite {
    PendingInvite { id: id.into(), label: Some(label.into()), created: Wall(created), expires: Wall(created + 7 * DAY), own_mac: false }
}

fn roster(this: &str) -> Roster {
    Roster {
        macs: vec![
            RosterMac { id: "d_pro".into(), name: "MacBook Pro".into(), joined: Wall(JUL20), token: true, this_mac: this == "d_pro" },
            RosterMac { id: "d_air".into(), name: "MacBook Air".into(), joined: Wall(SEP02), token: false, this_mac: this == "d_air" },
        ],
        members: vec![RosterMember { id: "m_mher".into(), label: "Mher".into(), joined: Wall(1_785_758_400_000) }],
        invites: vec![invite_row("i_sona", "Sona", AUG10), invite_row("i_ani", "Ani", WALL0 - 2 * DAY)],
        labels: BTreeMap::from([("m_mher".to_string(), "Mher".to_string())]),
    }
}

fn members(w: World, this: &str) -> World {
    feed(settings(w, SettingsTab::Members), [news(MemberNews::Roster(roster(this)))])
}

fn joining(kind: InviteKind) -> World {
    feed(unprovisioned(), [arrived(1), pending(1, kind), act(Surface::Onboarding, Action::Join)])
}

fn stranded_queue(n: usize) -> Vec<Queued> {
    (0..n).map(|i| queued(&format!("q{i}"), &format!("u{i}"), "Untitled — Sep 30", 1, None)).collect()
}

/// Region 6's faces: `(window, state, variant, world)`.
fn member_rows() -> Vec<(&'static str, &'static str, &'static str, World)> {
    use InviteKind::{Member as M, OwnMac as O};
    let set = |a: Action| act(Surface::Settings, a);
    let mut rows = Vec::new();
    // ── the Members tab ──
    rows.push(("settings", "invite-born", "members", members(owner(), "d_pro")));
    rows.push(("settings", "invite-born", "members-born", {
        let w = members(owner(), "d_pro");
        let ani = PendingInvite { id: "i_lilit".into(), label: Some("Lilit".into()), created: Wall(WALL0), expires: Wall(WALL0 + 7 * DAY), own_mac: false };
        feed(w, [Event::Field { field: shroom_core::event::Field::InviteName, value: "Lilit".into() }, set(Action::Invite), news(MemberNews::Invited { invite: ani, link: SecretId(7) })])
    }));
    rows.push(("settings", "invite-born", "members-born-copied", {
        let w = members(owner(), "d_pro");
        let lilit = PendingInvite { id: "i_lilit".into(), label: Some("Lilit".into()), created: Wall(WALL0), expires: Wall(WALL0 + 7 * DAY), own_mac: false };
        feed(w, [Event::Field { field: shroom_core::event::Field::InviteName, value: "Lilit".into() }, set(Action::Invite), news(MemberNews::Invited { invite: lilit, link: SecretId(7) }), set(Action::CopyInvite(SecretId(7)))])
    }));
    rows.push(("settings", "invite-born", "members-aging", {
        // The spec's own example: Ani invited Aug 21, read on Aug 23 — “5 days left”.
        let mut r = roster("d_pro");
        r.invites = vec![invite_row("i_sona", "Sona", AUG10), invite_row("i_ani", "Ani", AUG21)];
        let mut w = feed(settings(owner(), SettingsTab::Members), [news(MemberNews::Roster(r))]);
        w.wall = Wall(AUG23);
        w
    }));
    rows.push(("settings", "invite-born", "members-from-the-air", members(owner_class(), "d_air")));
    // Who invites say they're from, being changed (B158).
    rows.push(("settings", "invite-born", "members-from-changing", feed(members(owner(), "d_pro"), [set(Action::ChangeFrom)])));
    // One Mac: no sub-row under You (B137).
    rows.push(("settings", "invite-born", "members-one-mac", {
        let mut r = roster("d_pro");
        r.macs.retain(|m| m.this_mac);
        feed(settings(owner(), SettingsTab::Members), [news(MemberNews::Roster(r))])
    }));
    rows.push(("settings", "add-another-mac", "members-add-another-mac", {
        let mac = PendingInvite { id: "i_mac".into(), label: None, created: Wall(WALL0), expires: Wall(WALL0 + 3_600_000), own_mac: true };
        feed(members(owner(), "d_pro"), [set(Action::AddMac), news(MemberNews::Invited { invite: mac, link: SecretId(8) })])
    }));
    rows.push(("settings", "member-remove-confirm-member", "remove-member", feed(members(owner(), "d_pro"), [set(Action::Remove(RowRef::Member("m_mher".into())))])));
    rows.push(("settings", "member-remove-confirm-invite", "remove-invite", feed(members(owner(), "d_pro"), [set(Action::Remove(RowRef::Invite("i_ani".into())))])));
    rows.push(("settings", "member-remove-confirm-own-mac", "remove-mac", feed(members(owner(), "d_pro"), [set(Action::Remove(RowRef::Mac("d_air".into())))])));
    // ── Settings · Space on every kind of Mac ──
    rows.push(("settings", "settings-space", "space-owner-mark", feed(inspected(settings(owner(), SettingsTab::Space)), [standing(StandingAnswer::Fine { current: true, only_holder: Some(true), mark: Some(true), ai: None })])));
    rows.push(("settings", "settings-space-no-token", "space-owner-class", feed(settings(owner_class(), SettingsTab::Space), [standing(fine())])));
    rows.push(("settings", "token", "space-owner-class-add-token", feed(settings(owner_class(), SettingsTab::Space), [standing(fine()), set(Action::AddToken)])));
    rows.push(("settings", "settings-space-member", "space-member", feed(settings(member(), SettingsTab::Space), [standing(fine())])));
    // Removed by the owner: the space's row un-ticked, the banner's line (B157).
    rows.push(("settings", "settings-space-member", "space-member-removed", feed(settings(member(), SettingsTab::Space), [standing(StandingAnswer::Revoked)])));
    rows.push(("settings", "settings-space-no-token", "space-owner-class-removed", feed(settings(owner_class(), SettingsTab::Space), [standing(StandingAnswer::Revoked)])));
    rows.push(("settings", "leave-confirm", "leave-sheet", {
        let mut w = settings(member(), SettingsTab::Space);
        w.queue = stranded_queue(2).into_iter().map(|q| Queued { blocked: Some(Blocked::Space), ..q }).collect();
        feed(w, [standing(fine()), set(Action::Disconnect)])
    }));
    rows.push(("settings", "disconnect-confirm-no-token", "disconnect-owner-class", {
        let mut w = settings(owner_class(), SettingsTab::Space);
        w.queue = stranded_queue(2);
        feed(w, [standing(fine()), set(Action::Disconnect)])
    }));
    // ── the invite window ──
    rows.push(("onboarding", "member-welcome", "member-welcome", feed(unprovisioned(), [arrived(1), pending(1, M)])));
    rows.push(("onboarding", "member-welcome-own-mac", "member-welcome-own-mac", feed(unprovisioned(), [arrived(1), pending(1, O)])));
    rows.push(("onboarding", "joining", "joining", joining(M)));
    rows.push(("onboarding", "joining-own-mac", "joining-own-mac", joining(O)));
    rows.push(("onboarding", "join-waiting", "join-waiting-offline", feed(joining(M), [Event::Os(shroom_core::event::OsNews::NetPath(false)), Event::Failed { op: Op::Join, class: FailClass::Transport }])));
    rows.push(("onboarding", "join-waiting", "join-waiting-not-answering", feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Waiting })])));
    rows.push(("onboarding", "join-expired", "join-expired", feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Expired })])));
    rows.push(("onboarding", "join-used", "join-used", feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Used })])));
    rows.push(("onboarding", "join-space-missing", "join-space-missing", feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::SpaceMissing })])));
    rows.push(("onboarding", "join-already-here", "join-already-here", feed(member(), [arrived(1), pending(1, M), standing(fine())])));
    // Dead at arrival (DEVIATIONS #20): peek knew it — the whole window's face, no plan.
    let dead = |peek: PeekAnswer| feed(unprovisioned(), [arrived(1), news(MemberNews::Peeked { link: SecretId(1), peek })]);
    rows.push(("onboarding", "join-expired", "join-expired-arrival", dead(PeekAnswer::Expired { host: HOST.into(), kind: M, inviter: Some("Bagrat".into()) })));
    rows.push(("onboarding", "join-used", "join-used-arrival", dead(PeekAnswer::Used { host: HOST.into(), kind: M, inviter: "Bagrat".into() })));
    rows.push(("onboarding", "join-space-missing", "join-space-missing-arrival", dead(PeekAnswer::SpaceMissing { host: HOST.into() })));
    // An arrival whose peek couldn't get through: the waiting posture, the spinner resting.
    rows.push(("onboarding", "join-waiting", "join-waiting-arrival", feed(unprovisioned(), [arrived(1), Event::Failed { op: Op::Peek, class: FailClass::Transport }])));
    // The owner's own Mac opening its own space's link: the own-space wording.
    rows.push(("onboarding", "join-already-here", "join-already-here-own", feed(owner_class(), [arrived(1), pending(1, O), standing(fine())])));
    rows.push(("onboarding", "join-switch-confirm", "join-switch-confirm", feed(mine_owner(), [arrived(1), pending(1, M)])));
    rows.push(("onboarding", "join-switch-confirm-uploads", "join-switch-draining", {
        let mut w = mine_owner();
        w.queue = stranded_queue(2);
        feed(w, [arrived(1), pending(1, M)])
    }));
    rows.push(("onboarding", "join-switch-confirm-saved-take", "join-switch-recovered", {
        let mut w = mine_owner();
        w.saved.push(SavedTake {
            dir: TakeDir("20261001-090000-cd34".into()),
            rec: RecId("QrStUvWxYz012345".into()),
            length_ms: 272_000,
            cause: SavedCause::Interrupted,
            created: Wall(SEP30),
            movies: false,
        });
        feed(w, [arrived(1), pending(1, M)])
    }));
    // Mid-take (law 4): Join greyed with its note; the window was open before the take.
    rows.push(("onboarding", "join-switch-confirm", "join-switch-midtake", {
        let mut w = feed(mine_owner(), [arrived(1), pending(1, M)]);
        let mut t = recording();
        t.opened = Mono(w.now.0 + 5);
        t.since = t.opened;
        w.take = Some(t);
        w
    }));
    // Half set up (a token, no space yet): the switch's own line.
    rows.push(("onboarding", "join-switch-confirm", "join-switch-half-set-up", {
        let mut w = unprovisioned();
        w.space.token = true;
        feed(w, [arrived(1), pending(1, M)])
    }));
    rows.push(("onboarding", "member-done", "done-member", feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Joined(id_as(Role::Member, "m_mher")) })])));
    rows.push(("onboarding", "member-all-set", "all-set-member", {
        let mut w = feed(joining(M), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Joined(id_as(Role::Member, "m_mher")) })]);
        w.mac.primed = Some(Wall(WALL0));
        w.mac.screen = Fact::known(true, w.now);
        w.mac.mic = Fact::known(Access::Granted, w.now);
        w
    }));
    rows.push(("onboarding", "member-all-set-own-mac", "all-set-own-mac", {
        let mut w = feed(joining(O), [news(MemberNews::Joined { link: SecretId(1), answer: JoinAnswer::Joined(id_as(Role::OwnerClass, "m1")) })]);
        w.mac.primed = Some(Wall(WALL0));
        w.mac.screen = Fact::known(true, w.now);
        w.mac.mic = Fact::known(Access::Granted, w.now);
        w
    }));
    // ── the Library across members ──
    let mixed = || vec![remote("r502", "Where the 502s come from", AUG21, 272_000), LibraryRow { owner: Some("m_mher".into()), ..remote("rmher", "Mher's onboarding notes", AUG23, 95_000) }];
    rows.push(("library", "library", "library-everyone", feed(library(owner(), mixed()), [news(MemberNews::Roster(roster("d_pro"))), act(Surface::Library, Action::SetScope(LibraryScope::Everyone))])));
    rows.push(("library", "delete-confirm-other-member", "library-delete-theirs", {
        feed(library(owner(), mixed()), [news(MemberNews::Roster(roster("d_pro"))), act(Surface::Library, Action::SetScope(LibraryScope::Everyone)), act(Surface::Library, Action::DeleteRow(RecId("rmher".into())))])
    }));
    rows.push(("library", "library", "library-member", library(member(), mixed())));
    rows.push(("library", "library-unreachable", "library-unreachable", {
        let mut w = library(member(), mixed());
        w.queue = stranded_queue(1);
        feed(w, [standing(StandingAnswer::Revoked)])
    }));
    // ── the popover's member banners and the stranded stop ──
    rows.push(("popover", "member-revoked", "member-revoked", {
        let mut w = popover(member());
        w.queue = stranded_queue(3);
        feed(w, [standing(StandingAnswer::Revoked)])
    }));
    rows.push(("popover", "owner-mac-removed", "owner-mac-removed", {
        let mut w = popover(owner_class());
        w.queue = stranded_queue(2);
        feed(w, [standing(StandingAnswer::Revoked)])
    }));
    rows.push(("popover", "space-needs-owner", "space-needs-owner", struck(popover(member()), StandingAnswer::SpaceSide)));
    rows.push(("popover", "space-gone", "space-gone", {
        let mut w = popover(member());
        w.queue = stranded_queue(1);
        struck(w, StandingAnswer::Gone)
    }));
    rows.push(("popover", "space-needs-update-member", "needs-update-member", feed(popover(member()), [standing(StandingAnswer::Fine { current: false, only_holder: None, mark: None, ai: None })])));
    rows.push(("popover", "space-needs-update-no-token", "needs-update-owner-class", feed(popover(owner_class()), [standing(StandingAnswer::Fine { current: false, only_holder: Some(false), mark: Some(true), ai: None })])));
    rows.push(("popover", "space-degraded-no-token-mac", "degraded-no-token", struck(popover(owner_class()), StandingAnswer::SpaceSide)));
    rows.push(("popover", "stopped-stranded", "stopped-stranded", {
        let mut w = popover(member());
        let mut t = recording();
        t.phase = Phase::Paused(Cause::User);
        w.take = Some(t);
        feed(w, [standing(StandingAnswer::Revoked), act(Surface::Popover, Action::Stop), Event::Rec(shroom_core::event::RecNews::Finalized(Ok(30_000)))])
    }));
    rows
}

// ── Region 3: the recording machine's popover faces, the countdown ──

fn saved_take(cause: SavedCause, length_ms: u64) -> SavedTake {
    SavedTake { dir: TakeDir("20261001-090000-cd34".into()), rec: RecId("QrStUvWxYz012345".into()), length_ms, cause, created: Wall(1_789_000_000_000), movies: false }
}

fn paused_take(cause: Cause, length_ms: u64) -> World {
    let mut w = popover(ready());
    let mut t = recording();
    t.phase = Phase::Paused(cause);
    t.length_ms = length_ms;
    t.mic = Some("BuiltInMicrophoneDevice".into());
    w.take = Some(t);
    w
}

/// Every Region 3 popover face, and the countdown — `window` is `popover` or `countdown`.
fn region3_rows() -> Vec<(&'static str, &'static str, &'static str, World)> {
    let mut rows = Vec::new();
    let two_twins = || {
        vec![
            Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 },
            Display { id: 2, name: "LG UltraFine".into(), is_main: false, x: 1512, y: 0 },
            Display { id: 3, name: "LG UltraFine".into(), is_main: false, x: 4072, y: 0 },
        ]
    };
    rows.push(("popover", "config", "config", popover(ready())));
    rows.push(("popover", "config", "config-twins-show-yourself", {
        let mut w = popover(ready());
        w.mac.displays = Fact::known(two_twins(), Mono(0));
        w.prefs.show_yourself = true;
        w
    }));
    rows.push(("popover", "config", "config-mic-denied", {
        let mut w = popover(ready());
        w.mac.mic = Fact::known(Access::Denied, Mono(0));
        w
    }));
    rows.push(("popover", "config", "config-no-mic", {
        let mut w = popover(ready());
        w.mac.mics = Fact::known(vec![], Mono(0));
        w
    }));
    rows.push(("popover", "config", "config-only-an-iphone", {
        let mut w = popover(ready());
        w.mac.cameras = Fact::known(vec![Device { uid: "iphone".into(), name: "iPhone Camera".into(), builtin: false, continuity: true, system_default: false }], Mono(0));
        w.prefs.show_yourself = true;
        w
    }));
    // The camera's and the microphone's picks (DEVIATIONS #43): two cameras, one shown;
    // Camera only, its camera picked and no Camera off; No mic, picked.
    let two_cameras = || {
        vec![
            Device { uid: "cam".into(), name: "FaceTime HD Camera".into(), builtin: true, continuity: false, system_default: false },
            Device { uid: "ext".into(), name: "Studio Display Camera".into(), builtin: false, continuity: false, system_default: false },
        ]
    };
    rows.push(("popover", "config", "config-camera-picked", {
        let mut w = popover(ready());
        w.mac.cameras = Fact::known(two_cameras(), Mono(0));
        w.prefs.show_yourself = true;
        w.prefs.camera = Some("ext".into());
        w
    }));
    rows.push(("popover", "config", "config-camera-only", {
        let mut w = popover(ready());
        w.mac.cameras = Fact::known(two_cameras(), Mono(0));
        w.prefs.source = Some(SourceId::Camera);
        w
    }));
    // Camera only on a Mac with one camera: the camera picker has that camera alone, and stays live.
    rows.push(("popover", "config", "config-camera-only-one", {
        let mut w = popover(ready());
        w.prefs.source = Some(SourceId::Camera);
        w
    }));
    rows.push(("popover", "config", "config-no-mic-picked", {
        let mut w = popover(ready());
        w.prefs.mic_off = true;
        w
    }));
    rows.push(("popover", "config", "config-camera-denied", {
        let mut w = popover(ready());
        w.mac.camera = Fact::known(Access::Denied, Mono(0));
        w.prefs.source = Some(SourceId::Camera);
        w
    }));
    rows.push(("popover", "take-lost", "take-lost-camera", {
        let mut w = popover(ready());
        w.prefs.source = Some(SourceId::Camera);
        w.session.take_lost = Some(shroom_core::world::TakeLost::Said { camera: true });
        w
    }));
    rows.push(("popover", "take-lost", "take-lost-screen", {
        let mut w = popover(ready());
        w.session.take_lost = Some(shroom_core::world::TakeLost::Said { camera: false });
        w
    }));
    // The owner page's Edit in shroom, for a video this Mac can't edit (N69): the popover
    // opens by itself, without the keyboard, the line said once.
    for (variant, no) in [("edit-link-other-space", EditLinkNo::OtherSpace), ("edit-link-not-yours", EditLinkNo::NotYours), ("edit-link-gone", EditLinkNo::Gone)] {
        rows.push(("popover", "config", variant, {
            let mut w = ready();
            let door = Door::unasked(Mono(900));
            w.session.popover = Some(door);
            w.session.news = Some(door.opened);
            w.session.fields.edit_link = Some(EditLink::Said(no));
            w
        }));
    }
    rows.push(("popover", "disk-low", "disk-low", {
        let mut w = popover(ready());
        w.mac.disk_free = Fact::known(1_000_000_000 + 20 * 54_960_000, Mono(0));
        w
    }));
    rows.push(("popover", "disk-low", "disk-low-seven", {
        let mut w = popover(ready());
        w.mac.disk_free = Fact::known(1_000_000_000 + 7 * 54_460_000 + 1_000_000, Mono(0));
        w
    }));
    rows.push(("popover", "disk-low", "out-of-room", {
        let mut w = popover(ready());
        w.mac.disk_free = Fact::known(900_000_000, Mono(0));
        w
    }));
    rows.push(("popover", "recording-off", "recording-off", {
        let mut w = popover(ready());
        w.mac.screen = Fact::known(false, Mono(0));
        w
    }));
    rows.push(("popover", "recovered-take", "recovered-take", {
        let mut w = popover(ready());
        w.saved.push(saved_take(SavedCause::Interrupted, 272_000));
        w
    }));
    // N76: AI on by default — the setup Mac's banner as it turns the AI on and once it's on,
    // its Turn off re-wiring, a Turn off that didn't go through, off; and a member's banner.
    let ai_on = || {
        let mut w = popover(ready());
        w.space.ai = Some(true);
        w
    };
    rows.push(("popover", "ai-turned-on", "ai-turned-on", ai_on()));
    // N75: auto-cleanup's banner three days ahead, after Pin both, and once a round went.
    let going_pop = || going(suggesting(library_closed(popover(ready()))));
    rows.push(("popover", "autoclean-warning", "autoclean-warning", going_pop()));
    rows.push(("popover", "autoclean-warning", "autoclean-pinned", {
        let mut w = going_pop();
        w.session.fields.library.pinned_note = Some((2, Wall(w.wall.0 + 3 * 86_400_000)));
        w
    }));
    rows.push(("popover", "autoclean-warning", "autoclean-cleaned", {
        let mut w = going_pop();
        if let Fact::Known { v, .. } = &mut w.suggestions {
            v.going.clear();
        }
        w
    }));
    rows.push(("popover", "ai-turning-off", "ai-turning-off", {
        let mut w = ai_on();
        w.space.ai_turn = Some(AiTurn::TurningOff);
        w.inflight.ops.insert(Op::Ai, Mono(990).plus(Op::Ai.deadline_ms()));
        w
    }));
    rows.push(("popover", "ai-turn-off-failed", "ai-turn-off-failed", {
        let mut w = ai_on();
        w.space.ai_turn = Some(AiTurn::TurningOff);
        w.inflight.failures.insert(Op::Ai, Failure { at: Mono(990), class: FailClass::Transport, tries: 1 });
        w
    }));
    rows.push(("popover", "ai-off", "ai-off", {
        let mut w = popover(ready());
        w.space.ai = Some(false);
        w.space.ai_turn = Some(AiTurn::Off);
        w.once.ai_offer_answered = true;
        w
    }));
    rows.push(("popover", "ai-on-in-space", "ai-on-in-space", {
        let mut w = popover(member());
        w.space.ai = Some(true);
        w
    }));
    rows.push(("popover", "paused", "paused", paused_take(Cause::User, 272_000)));
    // N64: Drop a marker used at this pause — the row reads Marker dropped, greyed.
    rows.push(("popover", "paused", "paused-marker-dropped", {
        let mut w = paused_take(Cause::User, 272_000);
        if let Some(t) = w.take.as_mut() {
            t.markers = 1;
            t.marked_at_pause = true;
        }
        w
    }));
    rows.push(("popover", "display-lost", "display-lost", {
        let mut w = paused_take(Cause::DisplayLost, 61_000);
        w.mac.displays = Fact::known(vec![], Mono(0));
        w
    }));
    rows.push(("popover", "mic-lost", "mic-lost", paused_take(Cause::MicLost, 61_000)));
    rows.push(("popover", "mic-lost", "mic-lost-no-other", {
        let mut w = paused_take(Cause::MicLost, 61_000);
        w.mac.mics = Fact::known(vec![], Mono(0));
        w
    }));
    rows.push(("popover", "camera-lost", "camera-lost", {
        let mut w = paused_take(Cause::CameraLost, 61_000);
        if let Some(t) = w.take.as_mut() {
            t.source = SourceId::Camera;
            t.camera = Some("ext".into());
        }
        w
    }));
    rows.push(("popover", "mac-away", "mac-away", paused_take(Cause::Asleep, 61_000)));
    rows.push(("popover", "mic-permission-lost", "permission-lost-microphone", paused_take(Cause::MicRevoked, 61_000)));
    rows.push(("popover", "discard-confirm", "discard-confirm", {
        let mut w = paused_take(Cause::User, 272_000);
        w.session.confirm = Some(Confirm::Discard);
        w
    }));
    rows.push(("popover", "restart-confirm", "restart-confirm", {
        let mut w = paused_take(Cause::User, 272_000);
        w.session.confirm = Some(Confirm::Restart);
        w
    }));
    rows.push(("popover", "discard-confirm-saved-take", "discard-confirm-saved", {
        let mut w = popover(ready());
        let s = saved_take(SavedCause::Interrupted, 272_000);
        w.session.confirm = Some(Confirm::DiscardSaved(s.dir.clone()));
        w.saved.push(s);
        w
    }));
    for (state, variant, cause, len) in [("disk-full-stop", "disk-full-stop", SavedCause::DiskFull, 724_000), ("permission-lost", "permission-lost-screen", SavedCause::ScreenOff, 435_000)] {
        rows.push(("popover", state, variant, {
            let mut w = popover(ready());
            let s = saved_take(cause, len);
            w.session.last_stop = Some(StopOutcome::AutoStopped(s.dir.clone()));
            w.saved.push(s);
            w
        }));
    }
    rows.push(("popover", "stopped-offline", "stopped-offline", {
        let mut w = popover(ready());
        w.session.last_stop = Some(StopOutcome::Offline(RecId("b4kq9d".into())));
        w
    }));
    rows.push(("popover", "quit-confirm", "quit-confirm", {
        let mut w = popover(ready());
        w.session.confirm = Some(Confirm::Quit);
        for i in 0..2 {
            w.queue.push(Queued { dir: TakeDir(format!("q{i}")), rec: RecId(format!("r{i}")), title: "t".into(), stopped: Wall(0), done: 1, total: 2, blocked: None, length_ms: Some(1_000) });
        }
        w
    }));
    // The agent label (N62, B216): a take the shroom command started, recording.
    rows.push(("label", "recording", "agent-label", {
        let mut w = ready();
        let t = recording();
        w.agent = Some(t.opened);
        w.take = Some(t);
        w
    }));
    // The marker toast (N78): ⌥⌘M's answer, the take's first marker, a later one, and again
    // at a pause that has its marker (the popover put away).
    for (variant, phase, says) in [
        ("marker-dropped", Phase::Recording, MarkerToastSays::Dropped(1)),
        ("marker-3-dropped", Phase::Recording, MarkerToastSays::Dropped(3)),
        ("already-marked", Phase::Paused(Cause::User), MarkerToastSays::Already),
    ] {
        let state = if phase == Phase::Recording { "recording" } else { "tray-pause-bars" };
        rows.push(("toast", state, variant, {
            let mut w = ready();
            let n = if let MarkerToastSays::Dropped(n) = says { n } else { 1 };
            w.take = Some(OpenTake { phase, markers: n, marked_at_pause: says == MarkerToastSays::Already, toast: Some((says, w.now)), ..recording() });
            w
        }));
    }
    for (variant, at) in [("countdown-3", 900), ("countdown-1", 2_950)] {
        rows.push(("countdown", "countdown", variant, {
            let mut w = ready();
            w.now = Mono(at);
            w.session.arming = Some(Arming { started: Mono(900), deadline: Mono(3_900), source: SourceId::Display(1), restart_of: None, fired: false });
            w
        }));
    }
    rows
}

fn region3_gallery() -> Vec<Value> {
    region3_rows()
        .into_iter()
        .map(|(window, state, variant, w)| {
            let scene = derive(&w);
            let ids = scene.state_ids();
            assert!(ids.contains(&state), "{variant}: {state} not derived (got {ids:?})");
            let (slice, faces) = match window {
                "countdown" => (serde_json::to_value(&scene.countdown), vec![]),
                "label" => (serde_json::to_value(&scene.label), vec![]),
                "toast" => (serde_json::to_value(&scene.toast), vec![]),
                _ => (serde_json::to_value(&scene.popover), scene.popover.as_ref().map(|x| x.faces.clone()).unwrap_or_default()),
            };
            let slice = slice.expect("the slice serializes");
            assert!(!slice.is_null(), "{variant}: no {window}");
            json!({ "window": window, "state": state, "variant": variant, "faces": faces, "scene": slice })
        })
        .collect()
}

fn member_gallery() -> Vec<Value> {
    member_rows()
        .into_iter()
        .map(|(window, state, variant, w)| {
            let scene = derive(&w);
            let ids = scene.state_ids();
            assert!(ids.contains(&state), "{variant}: {state} not derived (got {ids:?})");
            let (slice, faces) = match window {
                "settings" => (serde_json::to_value(&scene.settings), scene.settings.as_ref().map(|x| x.faces.clone())),
                "onboarding" => (serde_json::to_value(&scene.onboarding), scene.onboarding.as_ref().map(|x| x.faces.clone())),
                "library" => (serde_json::to_value(&scene.library), scene.library.as_ref().map(|x| x.faces.clone())),
                _ => (serde_json::to_value(&scene.popover), scene.popover.as_ref().map(|x| x.faces.clone())),
            };
            let slice = slice.expect("the slice serializes");
            assert!(!slice.is_null(), "{variant}: no {window}");
            json!({ "window": window, "state": state, "variant": variant, "faces": faces.unwrap_or_default(), "scene": slice })
        })
        .collect()
}

/// The editor (N63), open from the Library on the spec's example video (4:32, saved as
/// revision 3 with the ends at 0:08 and 4:01, 150–162 s cut, a spoken-cue suggestion at
/// 200–204 s waiting) and its two markers.
fn editor_rows() -> Vec<(&'static str, &'static str, World)> {
    let saved = || {
        let mut list = shroom_edit::EditList::new(272.4);
        list.in_ = 8.0;
        list.out = 241.0;
        list.cuts = vec![
            shroom_edit::Cut::manual(150.0, 162.0),
            shroom_edit::Cut { kind: shroom_edit::CutKind::LineBeforeCue, applied: false, ..shroom_edit::Cut::manual(200.0, 204.0) },
        ];
        shroom_edit::History::new(list)
    };
    // The end pulled in to 3:55 since it opened: a step Undo can take back.
    let stepped = || {
        let mut h = saved();
        h.act(&shroom_edit::Op::Trim { in_: 8.0, out: 235.0 });
        h
    };
    let open = |media: EditorMedia, save: EditorSave, history: Option<shroom_edit::History>| {
        let mut w = library(ready(), vec![remote("set2", "Setting up shroom in two minutes", SEP30, 272_400)]);
        w.session.windows.insert(WindowKind::Editor(EditorId(1)), Door::asked(Mono(960)));
        w.session.fields.editors_opened = 1;
        w.session.fields.editors.insert(EditorId(1), EditorFields {
            rec: RecId("set2".into()),
            title: "Setting up shroom in two minutes".into(),
            length_ms: Some(272_400),
            source_duration: 272.4,
            markers: vec![61.5, 130.0],
            history,
            rev: 3,
            kept_until: None,
            media,
            save,
            closing: false,
        });
        w
    };
    let here = || EditorMedia::Ready { duration: 272.4 };
    // Opened again while the earlier edit's original is kept, with `left` ms to go (B197 (e)).
    let kept = |left: i64| {
        let mut w = open(here(), EditorSave::Idle, Some(saved()));
        w.session.fields.editors.get_mut(&EditorId(1)).unwrap().kept_until = Some(Wall(w.wall.0 + left));
        w
    };
    // The saved list, as the window holds it once the save landed.
    let after = |save: EditorSave| {
        let mut h = stepped();
        h.saved_as_now();
        open(here(), save, Some(h))
    };
    vec![
        ("editor", "editor-ready", open(here(), EditorSave::Idle, Some(stepped()))),
        ("editor", "editor-opening", open(EditorMedia::Opening, EditorSave::Idle, None)),
        ("editor", "editor-original-kept-2-days", kept(47 * 3_600_000 + 30 * 60_000)),
        ("editor", "editor-original-kept-1-day", kept(30 * 3_600_000)),
        ("editor", "editor-original-kept-hours", kept(20 * 3_600_000)),
        ("editor", "editor-original-kept-hours-in-words", kept(5 * 3_600_000 + 30 * 60_000)),
        ("editor", "editor-original-kept-1-hour", kept(90 * 60_000)),
        ("editor", "editor-original-kept-under-an-hour", kept(25 * 60_000)),
        ("editor-getting", "editor-getting", open(EditorMedia::Getting { pct: 40 }, EditorSave::Idle, Some(saved()))),
        ("editor-cant-get", "editor-cant-get", open(EditorMedia::Failed, EditorSave::Idle, Some(saved()))),
        ("editor-saving", "editor-saving", open(here(), EditorSave::Saving, Some(stepped()))),
        ("editor-save-failed", "editor-save-failed", open(here(), EditorSave::Failed, Some(stepped()))),
        ("editor-save-refused", "editor-save-refused", open(here(), EditorSave::Refused, Some(stepped()))),
        ("editor-making", "editor-making", after(EditorSave::Making { pct: 40 })),
        ("editor-making-failed", "editor-making-failed", after(EditorSave::MakingFailed)),
        ("editor-keep-original", "editor-keep-original", after(EditorSave::Asking)),
        // ✕ with the end pulled in not saved: “Save your edit?” (N71).
        ("editor-closing", "editor-closing", {
            let mut w = open(here(), EditorSave::Idle, Some(stepped()));
            w.session.fields.editors.get_mut(&EditorId(1)).unwrap().closing = true;
            w
        }),
    ]
}

/// The Send feedback window (N60): empty and gathering, filled with what goes shown, the
/// log box unticked, sending, a send that didn't go through, and sent.
fn feedback_rows() -> Vec<(&'static str, &'static str, World)> {
    let open = |words: &str, gathered: bool| {
        let mut w = ready();
        w.launch.version = "1.3.0".into();
        w.session.windows.insert(WindowKind::Feedback, Door::asked(Mono(100)));
        let f = &mut w.session.fields.feedback;
        f.words = words.into();
        f.reply_to = if words.is_empty() { String::new() } else { "you@example.com".into() };
        f.gathered = gathered.then(|| shroom_core::report::Diagnostics {
            shroom: "1.3.0".into(),
            macos: "15.3.1".into(),
            chip: "Apple M2".into(),
            memory: "16 GB".into(),
            setup: "2026-10-12 10:06 UTC key · 10:14 recording allowed · 10:15 wired".into(),
            role: "owns its space".into(),
            log: shroom_core::report::clean_log("    1840 core config [Probe, Draw]\n    2210 library refreshed (Owner): 0 new, 1 changed, 0 gone"),
        });
        w
    };
    let words = "Love it. One thing: I'd like the link copied the moment I stop, without opening the popover again.";
    let with = |mut w: World, f: fn(&mut shroom_core::world::FeedbackFields)| {
        f(&mut w.session.fields.feedback);
        w
    };
    vec![
        ("send-feedback", "feedback-empty", open("", false)),
        ("send-feedback", "feedback-filled", open(words, true)),
        ("send-feedback", "feedback-without-log", with(open(words, false), |f| f.without_log = true)),
        ("send-feedback", "feedback-sending", with(open(words, true), |f| f.sending = true)),
        ("send-feedback", "feedback-failed", with(open(words, true), |f| f.failed = true)),
        ("send-feedback", "feedback-sent", with(open("", false), |f| f.sent = true)),
    ]
}

fn window_gallery(rows: Vec<(&'static str, &'static str, World)>, slice: fn(&shroom_core::Scene) -> (Value, Vec<&'static str>)) -> Vec<Value> {
    rows.into_iter()
        .map(|(state, variant, w)| {
            let scene = derive(&w);
            let ids = scene.state_ids();
            assert!(ids.contains(&state), "{variant}: {state} not derived (got {ids:?})");
            let (slice, faces) = slice(&scene);
            assert!(!slice.is_null(), "{variant}: no window");
            json!({ "state": state, "variant": variant, "faces": faces, "scene": slice })
        })
        .collect()
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("library") => {
            let out = window_gallery(library_rows(), |s| {
                (serde_json::to_value(&s.library).expect("the slice serializes"), s.library.as_ref().map(|l| l.faces.clone()).unwrap_or_default())
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            return;
        }
        Some("members") => {
            println!("{}", serde_json::to_string_pretty(&member_gallery()).unwrap());
            return;
        }
        Some("region3") => {
            println!("{}", serde_json::to_string_pretty(&region3_gallery()).unwrap());
            return;
        }
        Some("editor") => {
            let out = window_gallery(editor_rows(), |s| {
                // One window, the gallery's: its slice is what its webview gets.
                let e = s.editors.values().next();
                (serde_json::to_value(e).expect("the slice serializes"), e.map(|e| e.faces.clone()).unwrap_or_default())
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            return;
        }
        Some("feedback") => {
            let out = window_gallery(feedback_rows(), |s| {
                (serde_json::to_value(&s.feedback).expect("the slice serializes"), s.feedback.as_ref().map(|f| f.faces.clone()).unwrap_or_default())
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            return;
        }
        Some("settings") => {
            let out = window_gallery(settings_rows(), |s| {
                (serde_json::to_value(&s.settings).expect("the slice serializes"), s.settings.as_ref().map(|l| l.faces.clone()).unwrap_or_default())
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            return;
        }
        _ => {}
    }
    let mut out: Vec<Value> = Vec::new();
    for (state, variant, w) in rows() {
        let scene = derive(&w);
        let ids = scene.state_ids();
        assert!(ids.contains(&state), "{variant}: {state} not derived (got {ids:?})");
        let slice = serde_json::to_value(&scene.onboarding).expect("the slice serializes");
        assert!(!slice.is_null(), "{variant}: no onboarding window");
        let faces = scene.onboarding.as_ref().map(|o| o.faces.clone()).unwrap_or_default();
        out.push(json!({ "state": state, "variant": variant, "faces": faces, "scene": slice }));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
