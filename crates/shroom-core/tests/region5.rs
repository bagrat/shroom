//! Region 5 — the Library (N35), Settings (N36), updates (N37) — and Region 2's afterlife
//! (`space-degraded`, the repair mood, re-entry as settings, N42), against the fixtures:
//! every in-scope state derives from a world built with the spec's example values, with the
//! fixture's exact Return default and its lines verbatim; the design's walkthroughs with
//! the laws checked at every step; and the machine's drawn edges, each realized by one event or
//! listed out of scope with its reason.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use shroom_core::effect::{CloudCmd, Effect, LibCmd, PersistOp, Perm, PublicUrl, ShellCmd, SweepPart, TakeCmd, UpdCmd};
use shroom_core::event::{CloudNews, Disk, Event, Field, Key, LibraryNews, OsNews, RecNews, SecretField, Shape, Surface, SweepAnswer, UpdateNews, UploadNews};
use shroom_core::scene::{Action, Face, LibraryRowView, Nest, Scene, SettingsPane, Style};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{derive, update};
use shroom_spec::Spec;
use support::*;

// ── builders, with the spec's example values ──

const AUG21: i64 = 1_787_313_600_000;
const AUG19: i64 = 1_787_140_800_000;
const AUG03: i64 = 1_785_758_400_000;
const JUL14: i64 = 1_784_030_400_000;
const SEP30: i64 = 1_790_769_600_000;

fn rec(id: &str) -> RecId {
    RecId(id.into())
}

fn remote(id: &str, title: &str, at: i64, length_ms: u64, live: bool) -> LibraryRow {
    LibraryRow { rec: rec(id), title: title.into(), created: Wall(at), length_ms: Some(length_ms), live, owner: Some("m1".into()), poster: false, edited_ms: None }
}

/// The space's records, as the mock draws them (and enough of them for six at once).
fn space_rows() -> Vec<LibraryRow> {
    vec![
        remote("r502", "Where the 502s come from", AUG21, 272_000, true),
        remote("r19", "Untitled — Aug 19", AUG19, 48_000, true),
        remote("onb", "Onboarding flow, take 2", AUG03, 730_000, true),
        remote("a1", "Release notes walkthrough", JUL14, 95_000, true),
        remote("a2", "Bug triage", JUL14 - 86_400_000, 300_000, true),
        remote("a3", "Pairing on the uploader", JUL14 - 2 * 86_400_000, 1_200_000, true),
        remote("a4", "Standup", JUL14 - 3 * 86_400_000, 61_000, true),
    ]
}

fn space_account() -> Account {
    Account { id: "acct".into(), name: "Bagrat's Account".into() }
}

fn acme() -> Account {
    Account { id: "acme".into(), name: "Acme Engineering".into() }
}

fn with_popover(mut w: World) -> World {
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

/// The setup Mac's popover while the space's AI is off: its banner is due (N66).
fn ai_offered() -> World {
    let mut w = with_popover(ready());
    w.space.ai = Some(false);
    w
}

fn library_with(mut w: World, rows: Vec<LibraryRow>) -> World {
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(Arc::new(rows), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w
}

fn library(w: World) -> World {
    library_with(w, space_rows())
}

/// One video worth deleting (N68).
fn suggestion(id: &str, bytes: u64, plays: u32, quiet_days: u32) -> Suggestion {
    Suggestion { rec: rec(id), bytes, plays, quiet_days, stopped_early: false }
}

/// What the space said is worth deleting, with the space's size (N68).
fn suggested(mut w: World, rows: Vec<Suggestion>, space_bytes: u64) -> World {
    w.suggestions = Fact::known(Suggestions { counting: true, space_bytes, rows }, Mono(960));
    w
}

/// The Library over a space of 34 GB with three videos worth deleting.
fn library_suggesting() -> World {
    suggested(library(ready()), vec![suggestion("a3", 3_200_000_000, 4, 300), suggestion("onb", 1_800_000_000, 0, 45), suggestion("a1", 780_000_000, 2, 120)], 34_000_000_000)
}

fn queued(dir: &str, id: &str, title: &str, done: u32, total: u32, blocked: Option<Blocked>) -> Queued {
    Queued { dir: TakeDir(dir.into()), rec: rec(id), title: title.into(), stopped: Wall(SEP30), done, total, blocked, length_ms: Some(48_000) }
}

/// Every kind of row at once: uploading, waiting, stranded, recovered, removed, live.
fn library_rich() -> World {
    let mut w = library(ready());
    w.queue.push(queued("q40", "up40", "Untitled — Sep 30", 2, 5, None));
    w.queue.push(queued("qoff", "upoff", "Demo for Mher", 1, 4, Some(Blocked::Offline)));
    w.queue.push(queued("qspace", "upspace", "Design review", 1, 4, Some(Blocked::Space)));
    w.saved.push(saved(SavedCause::Interrupted, 272_000));
    let mut gone = queued("qgone", "gone", "Old demo", 3, 4, None);
    gone.stopped = Wall(AUG19);
    w.removed.push(gone);
    w.session.copied = Some((CopyTarget::Link(rec("r502")), Mono(990)));
    w
}

fn settings_on(mut w: World, tab: SettingsTab) -> World {
    w.prefs.settings_tab = Some(tab);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn swept_ok() -> SweepAnswer {
    SweepAnswer {
        token: Some(TokenVerdict::Valid { account: space_account(), several: false }),
        gates: Some(Gates { email: false, card: false }),
        subdomain: Some(Some("myshroom".into())),
        foreign_v: Some(false),
        label: Some(Some("v".into())),
        bucket: Some(true),
        worker: Some(WorkerFact::Ours),
        found: Some(Some(14)),
    }
}

/// A token-holding Mac whose re-probe (after the window's entry) found every row sound.
fn inspected(mut w: World) -> World {
    let a = swept_ok();
    let at = Mono(950);
    w.sweep.token = Fact::known(a.token.unwrap(), at);
    w.sweep.gates = Fact::known(a.gates.unwrap(), at);
    w.sweep.subdomain = Fact::known(a.subdomain.unwrap(), at);
    w.sweep.foreign_v = Fact::known(false, at);
    w.sweep.label = Fact::known(Some("v".into()), at);
    w.sweep.bucket = Fact::known(true, at);
    w.sweep.worker = Fact::known(WorkerFact::Ours, at);
    w.sweep.found = Fact::known(Some(14), at);
    w.sweep.pass = Some(at);
    w
}

/// Settings · General with Uninstall shroom… asked (B135).
fn uninstall_asked() -> World {
    let mut w = settings_on(ready(), SettingsTab::General);
    w.session.confirm = Some(Confirm::Uninstall { going: false });
    w
}

/// Settings · General with Report a problem… asked (B180).
fn report_asked() -> World {
    let mut w = settings_on(ready(), SettingsTab::General);
    w.session.confirm = Some(Confirm::Report { sending: false });
    w
}

/// The Send feedback window, opened from the popover (N60).
fn feedback_open() -> World {
    let mut w = ready();
    w.session.windows.insert(WindowKind::Feedback, Door::asked(Mono(900)));
    w
}

/// The editor (N63), open from the Library on the spec's example video: its record read
/// (a trim and a cut saved as revision 3), its markers, and the video `media`; `changed`
/// adds a cut since — Done has something to save.
const EDITED: &str = "Setting up shroom in two minutes";
fn editor_open(media: EditorMedia, save: EditorSave, changed: bool) -> World {
    let mut rows = space_rows();
    rows.insert(0, remote("set2", EDITED, SEP30, 272_000, true));
    let mut w = library_with(ready(), rows);
    w.session.windows.insert(WindowKind::Editor, Door::asked(Mono(960)));
    let mut list = shroom_edit::EditList::new(272.0);
    list.in_ = 8.0;
    list.out = 241.0;
    list.cuts = vec![shroom_edit::Cut::manual(150.0, 162.0)];
    let mut history = shroom_edit::History::new(list);
    if changed {
        history.act(&shroom_edit::Op::Cut { start: 20.0, end: 30.0 });
    }
    w.session.fields.editor = Some(EditorFields {
        rec: rec("set2"),
        title: EDITED.into(),
        length_ms: Some(272_000),
        source_duration: 272.0,
        markers: vec![61.5],
        history: Some(history),
        rev: 3,
        kept_until: None,
        media,
        save,
        leaving: None,
        elsewhere: false,
        closing: false,
    });
    w
}

fn report_gathered() -> shroom_core::report::Diagnostics {
    shroom_core::report::Diagnostics {
        shroom: "1.2.0".into(),
        macos: "15.3.1".into(),
        chip: "Apple M2".into(),
        memory: "16 GB".into(),
        setup: String::new(),
        role: "owns its space".into(),
        log: "  1 boot: launch login".into(),
    }
}

fn space_tab() -> World {
    inspected(settings_on(ready(), SettingsTab::Space))
}

fn token_dead() -> World {
    let mut w = space_tab();
    w.sweep.token = Fact::known(TokenVerdict::Revoked, Mono(950));
    w
}

fn update_ready(mut w: World, version: &str) -> World {
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: version.into() }), Mono(950));
    w.update.checked = Some(Mono(950));
    w.update.staged = Some(version.into());
    w
}

fn disk(launch: LaunchKind, w: &World, version: &str, restore: Option<Restore>) -> Disk {
    Disk {
        launch,
        version: version.into(),
        prefs: w.prefs.clone(),
        once: w.once.clone(),
        space: w.space.identity.clone(),
        token: w.space.token,
        device_key: w.space.device_key,
        primed: w.mac.primed,
        restore,
        quick: quick(),
    }
}

/// Kill and relaunch: a new process boots from what's on disk.
fn reboot(r: &Run, launch: LaunchKind, version: &str, restore: Option<Restore>) -> Run {
    let d = disk(launch, &r.w, version, restore);
    let mut n = Run::new(World::blank());
    n.t = r.t + 1_000;
    n.send(Event::Booted(Box::new(d)));
    n
}

// ── reading the Scene ──

fn lib_rows(s: &Scene) -> &[LibraryRowView] {
    &s.library.as_ref().expect("the Library window").body.rows
}

fn titles(s: &Scene) -> Vec<String> {
    lib_rows(s).iter().map(|r| r.title.clone().unwrap_or_else(|| r.state.clone().unwrap_or_default())).collect()
}

fn row<'a>(s: &'a Scene, title: &str) -> &'a LibraryRowView {
    lib_rows(s).iter().find(|r| r.title.as_deref() == Some(title)).unwrap_or_else(|| panic!("no row {title:?} in {:?}", titles(s)))
}

fn button<'a>(r: &'a LibraryRowView, action: &Action) -> &'a shroom_core::scene::Button {
    r.buttons.iter().chain(&r.hover).chain(&r.menu).find(|b| &b.action == action).unwrap_or_else(|| panic!("no {action:?}"))
}

fn plan(s: &Scene) -> &shroom_core::scene::PlanView {
    match &s.settings.as_ref().expect("the Settings window").body.pane {
        SettingsPane::Space(sp) => &sp.plan,
        SettingsPane::General(_) => panic!("General, not Space"),
        SettingsPane::Members(_) => panic!("Members, not Space"),
    }
}

fn general(s: &Scene) -> &shroom_core::scene::GeneralBody {
    match &s.settings.as_ref().expect("the Settings window").body.pane {
        SettingsPane::General(g) => g,
        SettingsPane::Space(_) => panic!("Space, not General"),
        SettingsPane::Members(_) => panic!("Members, not General"),
    }
}

fn faces(s: &Scene) -> Vec<&'static str> {
    s.settings.as_ref().map(|w| w.faces.clone()).unwrap_or_default()
}

fn banner(s: &Scene) -> Option<&'static str> {
    s.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| b.id)
}

fn status(s: &Scene) -> Option<String> {
    s.popover.as_ref().and_then(|p| p.status_line.clone())
}

fn json(s: &Scene) -> String {
    serde_json::to_string(s).unwrap()
}

trait Drive {
    fn lib(&mut self, action: Action) -> &mut Self;
    fn set(&mut self, action: Action) -> &mut Self;
    fn cloud(&mut self, news: CloudNews) -> &mut Self;
    fn paste(&mut self, id: u32) -> &mut Self;
    fn field(&mut self, field: Field, value: &str) -> &mut Self;
}

impl Drive for Run {
    fn lib(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Library, action, rev: 0 })
    }
    fn set(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Settings, action, rev: 0 })
    }
    fn cloud(&mut self, news: CloudNews) -> &mut Self {
        self.send(Event::Cloud(news))
    }
    fn paste(&mut self, id: u32) -> &mut Self {
        self.send(Event::SecretPasted { field: SecretField::Token, id: SecretId(id), shape: Shape::Token })
    }
    fn field(&mut self, field: Field, value: &str) -> &mut Self {
        self.send(Event::Field { field, value: value.into() })
    }
}

// ── the inventory: one row (or more) per in-scope state ──

fn rows() -> Vec<(&'static str, World)> {
    let mut rows: Vec<(&'static str, World)> = Vec::new();
    // ── the Library ──
    rows.push(("library", library(ready())));
    rows.push(("library", library_rich()));
    rows.push(("library", {
        let mut w = library(ready());
        w.session.fields.library.renaming = Some((rec("r502"), "Where the 502s come from".into()));
        w
    }));
    // ── Suggested to clean up (N68) ──
    rows.push(("library", library_suggesting()));
    rows.push(("library-cleanup", {
        let mut w = library_suggesting();
        w.session.fields.library.cleanup = true;
        w.session.fields.library.undo = Some((rec("a1"), Mono(990)));
        w
    }));
    rows.push(("library-cleanup", {
        let mut w = library_suggesting();
        w.session.fields.library.cleanup = true;
        for id in ["a3", "onb", "a1"] {
            w.library_ops.landed.insert(rec(id), (Landed::Kept, Mono(980)));
        }
        w
    }));
    rows.push(("library-empty", library_with(ready(), vec![])));
    // N66 (Bagrat, 2026-10-18): the setup Mac's popover while the space's AI is off — its
    // banner, the confirm Turn on… opens, the banner while the page is re-wired and when that
    // didn't go through, and the line once it's on.
    rows.push(("ai-offer", ai_offered()));
    rows.push(("ai-confirm", {
        let mut w = ai_offered();
        w.session.confirm = Some(Confirm::Ai);
        w
    }));
    rows.push(("ai-turning-on", {
        let mut w = ai_offered();
        w.space.ai_turn = Some(AiTurn::Turning);
        w.inflight.ops.insert(Op::Ai, Mono(990).plus(Op::Ai.deadline_ms()));
        w
    }));
    rows.push(("ai-offer-failed", {
        let mut w = ai_offered();
        w.space.ai_turn = Some(AiTurn::Turning);
        w.inflight.failures.insert(Op::Ai, Failure { at: Mono(990), class: FailClass::Transport, tries: 1 });
        w
    }));
    rows.push(("ai-on", {
        let mut w = with_popover(ready());
        w.space.ai = Some(true);
        w.space.ai_turn = Some(AiTurn::On);
        w
    }));
    rows.push(("library-no-match", {
        let mut w = library_with(ready(), space_rows().into_iter().filter(|r| !r.title.contains("502")).collect());
        w.session.fields.library.search = "502".into();
        w
    }));
    rows.push(("library-offline", {
        let mut w = library(ready());
        w.net.online = Fact::known(false, Mono(0));
        w
    }));
    rows.push(("delete-confirm", {
        let mut w = library(ready());
        w.session.confirm = Some(Confirm::Delete(vec![rec("r502")]));
        w
    }));
    rows.push(("delete-confirm-several", {
        let mut w = library(ready());
        w.session.confirm = Some(Confirm::Delete(["r502", "r19", "onb", "a1", "a2", "a3"].map(rec).to_vec()));
        w
    }));
    // ── Settings ──
    rows.push(("settings", settings_on(ready(), SettingsTab::General)));
    rows.push(("settings-general", {
        let mut w = settings_on(ready(), SettingsTab::General);
        w.launch.version = "1.2.0".into();
        w
    }));
    rows.push(("settings-general", update_ready(settings_on(ready(), SettingsTab::General), "1.3.0")));
    rows.push(("settings-general", {
        let mut w = settings_on(ready(), SettingsTab::General);
        w.launch.version = "1.3.2".into();
        w.update.offer = Fact::known(Some(UpdateOffer::NeedsMacos { version: "1.4.0".into(), macos: 15 }), Mono(950));
        w
    }));
    rows.push(("settings-general", {
        // Opened before the take: it stays, and Update greys with its line (law 4).
        let mut w = update_ready(settings_on(ready(), SettingsTab::General), "1.3.0");
        w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
        w.take = Some(take(Phase::Recording, 30_000));
        w
    }));
    rows.push(("settings-space", space_tab()));
    // The setup Mac, once its page said whether it names videos (N66).
    rows.push(("settings-space", {
        let mut w = space_tab();
        w.space.ai = Some(true);
        w
    }));
    // …and a re-wiring that didn't go through (law 12).
    rows.push(("settings-space", {
        let mut w = space_tab();
        w.space.ai = Some(true);
        w.inflight.failures.insert(Op::Ai, Failure { at: Mono(990), class: FailClass::Transport, tries: 1 });
        w
    }));
    rows.push(("settings-space", {
        let mut w = space_tab();
        w.mac.mic = Fact::known(Access::Denied, Mono(0));
        w
    }));
    rows.push(("settings-space", {
        let mut w = space_tab();
        w.mac.screen = Fact::known(false, Mono(0));
        w
    }));
    rows.push(("settings-space", {
        let mut w = space_tab();
        w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
        w.take = Some(take(Phase::Paused(Cause::User), 30_000));
        w
    }));
    rows.push(("settings-space-inspection", space_tab()));
    rows.push(("re-entry-as-settings", space_tab()));
    rows.push(("settings-space-repair", token_dead()));
    rows.push(("repair-mood", token_dead()));
    rows.push(("token-repair-mood", token_dead()));
    rows.push(("token-other-account", {
        let mut w = token_dead();
        w.sweep.paste = Some(Paste::OtherAccount);
        w
    }));
    rows.push(("gate-card", {
        let mut w = space_tab();
        w.sweep.gates = Fact::known(Gates { email: false, card: true }, Mono(950));
        w
    }));
    // Add credit card clicked: the quiet line says the billing page is open (N45).
    rows.push(("gate-card", {
        let mut w = space_tab();
        w.sweep.gates = Fact::known(Gates { email: false, card: true }, Mono(950));
        w.session.fields.billing_opened = true;
        w
    }));
    rows.push(("wiring", {
        let mut w = space_tab();
        w.sweep.worker = Fact::known(WorkerFact::Absent, Mono(950));
        w
    }));
    rows.push(("space-storage-gone", {
        let mut w = space_tab();
        w.sweep.bucket = Fact::known(false, Mono(950));
        w
    }));
    rows.push(("name-changed", {
        let mut w = space_tab();
        w.sweep.subdomain = Fact::known(Some("myshroomhq".into()), Mono(950));
        w
    }));
    rows.push(("disconnect-confirm", {
        let mut w = space_tab();
        w.session.confirm = Some(Confirm::Exit);
        w
    }));
    rows.push(("disconnect-confirm", {
        let mut w = space_tab();
        w.session.confirm = Some(Confirm::Exit);
        w.queue.push(queued("q1", "u1", "One", 1, 4, None));
        w.queue.push(queued("q2", "u2", "Two", 1, 4, None));
        w
    }));
    // ── uninstall (B135) ──
    // ── report a problem (B180) ──
    rows.push(("settings-general", {
        let mut w = settings_on(ready(), SettingsTab::General);
        w.session.copied = Some((CopyTarget::Diagnostics, Mono(950)));
        w
    }));
    rows.push(("settings-general", {
        let mut w = settings_on(ready(), SettingsTab::General);
        w.session.fields.settings.sent = true;
        w
    }));
    rows.push(("settings-general", suggested(settings_on(ready(), SettingsTab::General), vec![], 34_000_000_000)));
    rows.push(("settings-general", {
        let mut w = suggested(settings_on(ready(), SettingsTab::General), vec![], 34_000_000_000);
        w.inflight.failures.insert(Op::Plays, Failure { class: FailClass::Transport, at: Mono(970), tries: 1 });
        w
    }));
    rows.push(("report-problem", report_asked()));
    rows.push(("report-problem", {
        let mut w = report_asked();
        w.session.fields.settings.report.gathered = Some(report_gathered());
        w.session.fields.settings.report.words = "It stopped.".into();
        w
    }));
    rows.push(("report-problem", {
        let mut w = report_asked();
        w.session.fields.settings.report.gathered = Some(report_gathered());
        w.session.fields.settings.report.words = "It stopped.".into();
        w.session.confirm = Some(Confirm::Report { sending: true });
        w
    }));
    rows.push(("report-problem", {
        let mut w = report_asked();
        w.session.fields.settings.report.gathered = Some(report_gathered());
        w.session.fields.settings.report.words = "It stopped.".into();
        w.session.fields.settings.report.failed = true;
        w
    }));
    // ── send feedback (N60) ──
    rows.push(("send-feedback", feedback_open()));
    rows.push(("send-feedback", {
        let mut w = feedback_open();
        w.session.fields.feedback.gathered = Some(report_gathered());
        w.session.fields.feedback.words = "Love it.".into();
        w
    }));
    rows.push(("send-feedback", {
        let mut w = feedback_open();
        w.session.fields.feedback.without_log = true;
        w.session.fields.feedback.words = "Love it.".into();
        w.session.fields.feedback.sending = true;
        w
    }));
    rows.push(("send-feedback", {
        let mut w = feedback_open();
        w.session.fields.feedback.gathered = Some(report_gathered());
        w.session.fields.feedback.words = "Love it.".into();
        w.session.fields.feedback.failed = true;
        w
    }));
    rows.push(("send-feedback", {
        let mut w = feedback_open();
        w.session.fields.feedback.sent = true;
        w
    }));
    // ── the editor (N63) ──
    let live = || EditorMedia::Ready { duration: 272.0 };
    rows.push(("editor", editor_open(live(), EditorSave::Idle, false)));
    rows.push(("editor", editor_open(live(), EditorSave::Idle, true)));
    rows.push(("editor", {
        // Just opened: the record not read yet, the timeline empty, Done waiting.
        let mut w = editor_open(EditorMedia::Opening, EditorSave::Idle, false);
        w.session.fields.editor.as_mut().unwrap().history = None;
        w.inflight.ops.insert(Op::EditorOpen, Mono(1_801_000));
        w
    }));
    rows.push(("editor-getting", {
        let mut w = editor_open(EditorMedia::Getting { pct: 40 }, EditorSave::Idle, false);
        w.inflight.ops.insert(Op::EditorOpen, Mono(1_801_000));
        w
    }));
    rows.push(("editor-cant-get", editor_open(EditorMedia::Failed, EditorSave::Idle, false)));
    rows.push(("editor-saving", {
        let mut w = editor_open(live(), EditorSave::Saving, true);
        w.inflight.ops.insert(Op::EditorSave, Mono(61_000));
        w
    }));
    rows.push(("editor-save-failed", editor_open(live(), EditorSave::Failed, true)));
    // The space turned the save down (B199): nothing to save over, Close only.
    rows.push(("editor-save-refused", editor_open(live(), EditorSave::Refused, true)));
    rows.push(("editor", {
        // Edited again while an earlier edit's original is kept (B197 (e)): 30 hours left,
        // a day as the line says it (B199).
        let mut w = editor_open(live(), EditorSave::Idle, false);
        w.session.fields.editor.as_mut().unwrap().kept_until = Some(Wall(w.wall.0 + 30 * 3_600_000));
        w
    }));
    rows.push(("editor", {
        // The owner page's Edit in shroom asked for another video while this one is open
        // (N69): it came forward as it is, its footer saying why.
        let mut w = editor_open(live(), EditorSave::Idle, true);
        w.session.fields.editor.as_mut().unwrap().elsewhere = true;
        w
    }));
    // ── after the save (B197) ──
    rows.push(("editor-making", {
        let mut w = editor_open(live(), EditorSave::Making { pct: 40 }, false);
        w.library_ops.copies.insert(rec("set2"), CopyOwed { rev: 3, asked: true });
        w.inflight.ops.insert(Op::EditorCopy, Mono(3_601_000));
        w
    }));
    rows.push(("editor-making-failed", editor_open(live(), EditorSave::MakingFailed, false)));
    rows.push(("editor-keep-original", editor_open(live(), EditorSave::Asking, false)));
    rows.push(("uninstall-confirm", uninstall_asked()));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.space.identity.as_mut().unwrap().role = Role::Member;
        w
    }));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.queue.push(queued("q1", "u1", "One", 1, 4, None));
        w.queue.push(queued("q2", "u2", "Two", 1, 4, None));
        w
    }));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.queue.push(queued("q1", "u1", "One", 1, 4, Some(Blocked::Space)));
        w.queue.push(queued("q2", "u2", "Two", 1, 4, Some(Blocked::Space)));
        w
    }));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.session.confirm = Some(Confirm::Uninstall { going: true });
        w
    }));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.queue.push(queued("q1", "u1", "One", 1, 4, None));
        w.queue.push(queued("q2", "u2", "Two", 1, 4, Some(Blocked::Space)));
        w
    }));
    rows.push(("uninstall-confirm", {
        let mut w = uninstall_asked();
        w.saved.push(saved(SavedCause::Interrupted, 272_000));
        w
    }));
    // ── updates ──
    rows.push(("update-ready", with_popover(update_ready(ready(), "1.3.0"))));
    rows.push(("tray-frost-dot", update_ready(ready(), "1.3.0")));
    rows.push(("updating-app", {
        let mut w = with_popover(update_ready(ready(), "1.3.0"));
        w.update.applying = true;
        w
    }));
    rows.push(("updated", {
        let mut w = with_popover(ready());
        w.launch.version = "1.3.0".into();
        w.once.seen_version = Some("1.2.0".into());
        w
    }));
    rows.push(("update-failed", {
        let mut w = with_popover(ready());
        w.launch.version = "1.2.0".into();
        w.update.failed = true;
        w
    }));
    // ── Region 2's afterlife ──
    rows.push(("space-degraded", {
        let mut w = with_popover(ready());
        w.space.broken = Fact::known(true, Mono(950));
        w
    }));
    rows
}

/// Lines of in-scope states this slice deliberately doesn't render, with the reason.
const LINES_OUT_OF_SCOPE: &[(&str, &str, &str)] = &[
    ("library", "Mine", "Mine · Everyone — owner-class Macs once someone else records: Region 6"),
    ("library", "Everyone", "Mine · Everyone — Region 6"),
    ("settings", "Members", "the Members tab — Region 6"),
    ("settings-space", "“made with shroom” on your pages", "the mark switch (U3) — a space setting the page also flips; not in this slice"),
    ("settings-space", "The small mark on every video page. You can flip it right on a page, too.", "the mark switch — not in this slice"),
    ("settings-space", "Leave this space…", "members' exit — Region 6"),
    ("wiring", "Almost there — switching your name on takes a few minutes the first time, and only once.", "the setup's last step — the onboarding window's (N45, onboarding.rs)"),
    ("editor", "was 4:32", "a template: the Scene hands the editor “was {}” and the webview puts the recording's length in it, as it does the time"),
];

fn surface_of(id: &str) -> Surface {
    if id.starts_with("library") || id.starts_with("delete-confirm") {
        Surface::Library
    } else if id == "send-feedback" {
        Surface::Feedback
    } else if id.starts_with("editor") {
        Surface::Editor
    } else if id.starts_with("settings") || ["disconnect-confirm", "disconnect-confirm-no-token", "leave-confirm", "uninstall-confirm", "report-problem", "repair-mood", "re-entry-as-settings", "token-repair-mood", "token-other-account", "gate-card", "wiring", "space-storage-gone", "name-changed"].contains(&id) {
        Surface::Settings
    } else {
        Surface::Popover
    }
}

fn action_for(label: &str, renaming: bool) -> Option<Action> {
    match label {
        // A field's Return is the field's (law 8): only while the title is being typed.
        "Rename (click a title)" => renaming.then_some(Action::CommitRename),
        "Create token in Cloudflare" => Some(Action::CreateToken),
        "Add credit card" => Some(Action::AddCard),
        "Set it up fresh" => Some(Action::SetUpFresh),
        "Start recording" => Some(Action::Start),
        "Done" => Some(Action::CloseFeedback),
        other => panic!("no mapping for the Return default {other:?}"),
    }
}

#[test]
fn every_region5_state_derives_with_its_return_default_and_its_lines() {
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
        let fixture = by_id[id].iter().find(|s| !s.said_lines.is_empty() || !s.actions.is_empty()).unwrap_or(&by_id[id][0]);
        let surface = surface_of(id);
        let renaming = w.session.fields.library.renaming.is_some();
        // The repair mood's own Return is its nest's — checked under the nest's id.
        let structural = ["settings-space-repair", "repair-mood", "tray-frost-dot"].contains(&id);
        // Send feedback (N60): Return types a new line in the words until the sent face,
        // whose Done it is; Esc is Cancel, but not while it goes.
        // The editor (N63): Done is Return once the video is here and its record read (the
        // button greys until then, and reads Saving… while it saves); Close in
        // editor-cant-get, which Esc means too. Elsewhere Esc is the webview's (it drops a
        // selection).
        if id.starts_with("editor") {
            let e = w.session.fields.editor.as_ref().expect("the editor's fields");
            let live = matches!(e.media, EditorMedia::Ready { .. }) && e.history.is_some() && e.save != EditorSave::Saving;
            // Never Yes, delete (law 9; DEVIATIONS #55): a fixture naming it has no mapping.
            let want = fixture.return_default().and_then(|a| match a.label.as_str() {
                "Done" => live.then_some(Action::EditorDone),
                "Close" => Some(Action::EditorClose),
                other => panic!("no mapping for the editor's Return default {other:?}"),
            });
            if scene.ret(surface) != want {
                failures.push(format!("{id}: Return is {:?}, want {want:?}", scene.ret(surface)));
            }
            // After a save, Esc is Close — Keep for 2 days on the question (B197 (b)) — and on
            // a save the space turned down, Close too (B199).
            let want_esc = match id {
                "editor-cant-get" | "editor-making" | "editor-making-failed" | "editor-save-refused" => Some(Action::EditorClose),
                "editor-keep-original" => Some(Action::KeepOriginal),
                _ => None,
            };
            if scene.esc(surface) != want_esc {
                failures.push(format!("{id}: Esc is {:?}, want {want_esc:?}", scene.esc(surface)));
            }
        } else if id == "send-feedback" {
            let f = &w.session.fields.feedback;
            let want = f.sent.then(|| fixture.return_default().and_then(|a| action_for(&a.label, false))).flatten();
            if scene.ret(surface) != want {
                failures.push(format!("{id}: Return is {:?}, want {want:?}", scene.ret(surface)));
            }
            let want_esc = (!f.sending).then_some(Action::CloseFeedback);
            if scene.esc(surface) != want_esc {
                failures.push(format!("{id}: Esc is {:?}, want {want_esc:?}", scene.esc(surface)));
            }
        } else if id == "ai-confirm" {
            // The popover's question (N66): a click, not Return (as every question there); Esc is Back.
            if scene.ret(surface).is_some() || scene.esc(surface) != Some(Action::Back) {
                failures.push(format!("{id}: Return {:?}, Esc {:?}", scene.ret(surface), scene.esc(surface)));
            }
        } else if !structural {
            let got = scene.ret(surface);
            let want = match fixture.kind.as_str() {
                // A banner's own filled button is never the Return default (law 8): config's
                // Start stays it.
                "banner" => Some(Action::Start),
                _ if surface == Surface::Popover => Some(Action::Start),
                _ => fixture.return_default().and_then(|a| action_for(&a.label, renaming)),
            };
            if got != want {
                failures.push(format!("{id}: Return is {got:?}, the fixture says {want:?}"));
            }
            let esc = scene.esc(surface);
            let sheet = fixture.kind == "sheet" || fixture.surface == "sheet";
            let want_esc = if sheet { Some(Action::Back) } else if renaming { Some(Action::CancelRename) } else if surface == Surface::Popover { Some(Action::Dismiss) } else { None };
            if esc != want_esc {
                failures.push(format!("{id}: Esc is {esc:?}, want {want_esc:?}"));
            }
        }
        seen.entry(id).or_default().push_str(&json(&scene));
    }
    for (id, haystack) in &seen {
        for fixture in &by_id[id] {
            for l in &fixture.said_lines {
                let varies = l.note.as_deref().is_some_and(|n| n.contains("example") || n.contains("varies"));
                let out = LINES_OUT_OF_SCOPE.iter().any(|(s, t, _)| s == id && *t == l.text);
                let encoded = serde_json::to_string(&l.text).unwrap();
                if !varies && !out && !haystack.contains(&encoded[1..encoded.len() - 1]) {
                    failures.push(format!("{id}: {} line missing: {:?}", l.copy_status, l.text));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Region 5's states and Region 2's afterlife: each in the inventory above, or listed here
/// with why this slice doesn't build it.
const STATES_OUT_OF_SCOPE: &[(&str, &str)] = &[
    ("settings-members", "the Members tab is Region 6's members surface"),
    ("settings-space-no-token", "owner-class Macs without the token join through Add another Mac — Region 6"),
    ("settings-space-member", "a member's Mac — Region 6"),
    ("leave-confirm", "members' exit — Region 6"),
    ("disconnect-confirm-no-token", "owner-class Macs without the token — Region 6"),
    ("library-unreachable", "member-revoked · space-gone (N34) — Region 6's member life"),
    ("delete-confirm-other-member", "needs member labels (N28) — Region 6"),
    ("space-needs-update-member", "members and the page's version (N34, N37) — Region 6"),
    ("space-needs-update-no-token", "owner-class Macs without the token — Region 6"),
    ("space-degraded-no-token-mac", "owner-class Macs without the token read a different banner — Region 6"),
];

#[test]
fn every_region5_state_is_built_or_listed_out_of_scope() {
    let spec = Spec::load();
    let afterlife = ["space-degraded", "space-degraded-no-token-mac", "repair-mood", "space-storage-gone", "name-changed", "re-entry-as-settings", "token-repair-mood", "token-other-account"];
    let wanted: BTreeSet<&str> = spec.region(5).map(|s| s.id.as_str()).chain(afterlife).collect();
    let built: BTreeSet<&str> = rows().into_iter().map(|(id, _)| id).collect();
    let out: BTreeSet<&str> = STATES_OUT_OF_SCOPE.iter().map(|(id, _)| *id).collect();
    assert!(built.is_disjoint(&out), "both built and out of scope: {:?}", built.intersection(&out).collect::<Vec<_>>());
    let missing: Vec<_> = wanted.iter().filter(|id| !built.contains(*id) && !out.contains(*id)).collect();
    assert!(missing.is_empty(), "neither built nor listed: {missing:?}");
    let stale: Vec<_> = out.iter().filter(|id| !wanted.contains(*id)).collect();
    assert!(stale.is_empty(), "listed states the spec doesn't have: {stale:?}");
}

// ── walkthroughs ──

/// Law 6 as ratified (SM:133; DEVIATIONS #38's narrowing rejected by Bagrat on
/// 2026-10-13): the Library and Settings are ordinary windows under the dock rule — the
/// icon while either is open, alone or together, and gone with the last of them.
#[test]
fn the_library_and_settings_bring_the_dock_icon_while_open() {
    let mut r = Run::new(with_popover(ready()));
    assert!(!r.scene.dock, "config: popovers never bring it");
    r.act(Action::OpenLibrary);
    assert!(r.scene.library.as_ref().is_some_and(|l| l.activates), "asked for: it takes the keyboard");
    assert!(r.scene.dock, "the Library: a window, and the Dock icon with it");
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"), "the mushroom still opens config over it");
    r.act(Action::OpenSettings);
    assert!(r.scene.settings.as_ref().is_some_and(|w| w.activates));
    assert!(r.scene.library.is_some() && r.scene.dock, "both open: the Dock icon");
    // The door to a window already open brings it forward, beside the Dock and ⌘-Tab.
    r.click();
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenLibrary)), "the footer's door brings an open Library forward");
    assert!(r.scene.library.as_ref().is_some_and(|l| l.activates) && r.scene.dock);
    r.click();
    r.act(Action::OpenSettings);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenSettings)), "and an open Settings");
    r.send(Event::WindowClosed(WindowKind::Library));
    assert!(r.scene.settings.is_some() && r.scene.dock, "Settings still open: the icon stays");
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert!(r.scene.library.is_none() && r.scene.settings.is_none() && !r.scene.dock, "the last window closed: the icon goes");
}

#[test]
fn the_library_opens_searches_renames_copies_deletes_and_goes_offline() {
    let mut w = with_popover(ready());
    w.queue.push(queued("q40", "up40", "Untitled — Sep 30", 2, 5, None));
    let mut r = Run::new(w);
    r.act(Action::OpenLibrary);
    assert_eq!(r.scene.library.as_ref().map(|l| l.id), Some("library"));
    assert!(r.scene.popover.is_none() && r.scene.dock, "the popover is the door; the Library brings the Dock icon (law 6)");
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Refresh)), "the bucket refresh runs behind the window");
    assert_eq!(titles(&r.scene), ["Untitled — Sep 30"], "it opens on what this Mac knows at once");
    assert_eq!(row(&r.scene, "Untitled — Sep 30").state.as_deref(), Some("Uploading — 40%"));

    r.send(Event::Library(LibraryNews::Rows { rows: space_rows()[..3].to_vec(), fresh: false }));
    assert_eq!(titles(&r.scene)[..3], ["Untitled — Sep 30", "Where the 502s come from", "Untitled — Aug 19"], "newest first");
    let r502 = row(&r.scene, "Where the 502s come from");
    assert_eq!(r502.meta, "Aug 21 · 4:32");
    assert_eq!(r502.state, None, "silence means live");
    let footer = r.scene.library.as_ref().unwrap().body.footer.clone();
    assert_eq!(footer.as_deref(), Some("4 videos"), "the count only");
    r.send(Event::Library(LibraryNews::Rows { rows: space_rows(), fresh: true }));
    assert_eq!(lib_rows(&r.scene).len(), 8, "new rows slide in — videos from other Macs too");
    assert!(r.effects.is_empty(), "one refresh per entry");

    // Search: local and instant, over titles.
    r.field(Field::LibrarySearch, "502");
    assert_eq!(titles(&r.scene), ["Where the 502s come from"]);
    r.field(Field::LibrarySearch, "zzz");
    assert!(r.ids().contains(&"library-no-match"));
    assert_eq!(r.scene.library.as_ref().unwrap().body.empty.as_deref(), Some("No titles match “zzz”."));
    r.field(Field::LibrarySearch, "");
    assert_eq!(lib_rows(&r.scene).len(), 8);

    // Rename in place: an emptied title brings the previous one back.
    r.lib(Action::StartRename(rec("r502")));
    let lw = r.scene.library.as_ref().unwrap();
    assert_eq!((lw.ret.clone(), lw.esc.clone()), (Some(Action::CommitRename), Some(Action::CancelRename)), "Enter saves, Esc cancels");
    r.field(Field::Title, "   ");
    r.key(Surface::Library, Key::Return);
    assert!(r.effects.is_empty(), "never blank");
    assert!(row(&r.scene, "Where the 502s come from").rename.is_none());
    r.lib(Action::StartRename(rec("r502")));
    r.field(Field::Title, "Why  the 502s happen ");
    r.key(Surface::Library, Key::Return);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Rename(rec("r502"), "Why the 502s happen".into()))));
    assert!(lib_rows(&r.scene).iter().any(|x| x.title.as_deref() == Some("Why the 502s happen")), "the new title shows while it saves");
    r.send(Event::Library(LibraryNews::Renamed { rec: rec("r502"), title: "Why the 502s happen".into() }));
    assert!(!r.w.inflight.ops.contains_key(&Op::Rename));
    r.lib(Action::StartRename(rec("r19")));
    r.key(Surface::Library, Key::Esc);
    assert!(row(&r.scene, "Untitled — Aug 19").rename.is_none(), "Esc cancels");

    // Copy link: “Copied” for a moment (law 10).
    r.lib(Action::CopyRow(rec("onb")));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::Copy(rec("onb")))));
    assert_eq!(row(&r.scene, "Onboarding flow, take 2").buttons[0].label, "Copied");
    let back = r.scene.next_wake.unwrap();
    r.tick_to(back.0);
    assert_eq!(row(&r.scene, "Onboarding flow, take 2").buttons[0].label, "Copy link");

    // Delete several: ⌘-click, then the Delete key — one sheet under N23's law.
    r.lib(Action::SelectRow(rec("r502")));
    r.lib(Action::ToggleRow(rec("onb")));
    r.lib(Action::DeleteSelected);
    assert!(r.ids().contains(&"delete-confirm-several"));
    let sheet = r.scene.library.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.title, "Delete two videos?");
    assert_eq!(sheet.buttons.iter().map(|b| (b.label.as_str(), b.style)).collect::<Vec<_>>(), [("Cancel", Style::Plain), ("Delete", Style::RedOutline)]);
    r.key(Surface::Library, Key::Return);
    assert!(r.effects.is_empty() && r.scene.library.as_ref().unwrap().body.sheet.is_some(), "no Return default (law 9)");
    r.key(Surface::Library, Key::Esc);
    assert!(r.scene.library.as_ref().unwrap().body.sheet.is_none(), "Esc ≡ Cancel");
    r.lib(Action::DeleteRow(rec("onb")));
    r.lib(Action::ConfirmDelete);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("r502"), rec("onb")]))), "the tombstones first — the lane's order");
    assert!(!titles(&r.scene).iter().any(|t| t.contains("502") || t.contains("Onboarding")), "gone while it deletes");
    r.send(Event::Library(LibraryNews::Deleted(vec![rec("r502"), rec("onb")])));
    assert!(r.effects.is_empty(), "nothing of theirs is on this Mac");

    // A video still uploading: the same sheet; this Mac's copy goes once the tombstone landed.
    r.lib(Action::DeleteRow(rec("up40")));
    assert_eq!(r.scene.library.as_ref().unwrap().body.sheet.as_ref().unwrap().title, "Delete “Untitled — Sep 30”?");
    r.lib(Action::ConfirmDelete);
    r.send(Event::Library(LibraryNews::Deleted(vec![rec("up40")])));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir("q40".into())))));
    assert!(r.w.queue.is_empty());

    // Offline: what this Mac remembers; Copy link stays live, rename and delete grey.
    r.send(Event::Os(OsNews::NetPath(false)));
    assert!(r.ids().contains(&"library-offline"));
    let a19 = row(&r.scene, "Untitled — Aug 19");
    assert!(button(a19, &Action::CopyRow(rec("r19"))).enabled);
    assert!(!button(a19, &Action::DeleteRow(rec("r19"))).enabled);
    assert!(!button(a19, &Action::StartRename(rec("r19"))).enabled);
    assert_eq!(a19.gestures.title_click, None);
    r.lib(Action::DeleteRow(rec("r19")));
    assert!(r.scene.library.as_ref().unwrap().body.sheet.is_none(), "Delete… is greyed offline");
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(!r.ids().contains(&"library-offline"), "the face clears by itself (law 1)");

    r.send(Event::WindowClosed(WindowKind::Library));
    assert!(r.scene.library.is_none() && !r.scene.dock, "the window closes, and the Dock icon goes with it");
    assert_eq!(r.w.session.fields.library, LibraryFields::default(), "its search, selection and typing go with it");
}

/// A typed field as its window reads it off the Scene: what it holds, and how many of its
/// edits the core has taken in (DECISIONS B107).
fn typed(field: &impl serde::Serialize) -> (String, u64) {
    let v = serde_json::to_value(field).unwrap();
    (v["value"].as_str().expect("the field's value").to_string(), v["edits"].as_u64().expect("the field's edits, counted"))
}

#[test]
fn the_library_counts_the_edits_of_its_search_and_of_a_title_being_renamed() {
    // What's typed goes to the core and comes back in the Scene a moment later — by then
    // more may have been typed. The Scene says how many edits of a field it holds, so a
    // window can tell the answer to an older edit from the answer to its newest (B107).
    let mut r = Run::new(with_popover(ready()));
    r.act(Action::OpenLibrary);
    r.send(Event::Library(LibraryNews::Rows { rows: space_rows(), fresh: true }));
    let search = |r: &Run| typed(&r.scene.library.as_ref().unwrap().body.search);
    let title = |r: &Run| lib_rows(&r.scene).iter().find_map(|x| x.rename.as_ref()).map(typed);
    assert_eq!(search(&r), (String::new(), 0));
    r.field(Field::LibrarySearch, "5");
    r.field(Field::LibrarySearch, "50");
    assert_eq!(search(&r), ("50".into(), 2), "one for every edit taken in");
    r.field(Field::LibrarySearch, "50");
    assert_eq!(search(&r), ("50".into(), 3), "the same text again is still an edit");
    r.send(Event::Tick);
    assert_eq!(search(&r), ("50".into(), 3), "only an edit moves the count");
    r.field(Field::LibrarySearch, "");
    assert_eq!(search(&r), (String::new(), 4));

    // A title's edits count apart, and only while a title is being renamed.
    r.field(Field::Title, "x");
    assert_eq!(r.arm.name, "dropped", "no title is being renamed");
    r.lib(Action::StartRename(rec("r502")));
    assert_eq!(title(&r), Some(("Where the 502s come from".into(), 0)), "a dropped edit is never counted");
    r.field(Field::Title, "W");
    r.field(Field::Title, "Wh");
    assert_eq!(title(&r), Some(("Wh".into(), 2)));
    assert_eq!(search(&r).1, 4, "each field counts its own");
    r.key(Surface::Library, Key::Esc);
    r.lib(Action::StartRename(rec("r19")));
    assert_eq!(title(&r), Some(("Untitled — Aug 19".into(), 2)), "starting a rename is not an edit: the count stands");
    r.field(Field::Title, "U");
    assert_eq!(title(&r), Some(("U".into(), 3)));

    // The counts go with the window's fields, and start again with them.
    r.send(Event::WindowClosed(WindowKind::Library));
    r.click();
    r.act(Action::OpenLibrary);
    assert_eq!(search(&r), (String::new(), 0));
    r.lib(Action::StartRename(rec("r502")));
    assert_eq!(title(&r).map(|t| t.1), Some(0));
}

#[test]
fn library_rows_carry_their_state_words_and_their_doors() {
    let s = derive(&library_rich());
    let all = lib_rows(&s);
    assert!(all[0].pinned, "a recovered take pins to the top");
    assert_eq!(all[0].state.as_deref(), Some("Interrupted recording — 4:32 saved"));
    // Mirroring N25's banner: “… saved” · [ Get link ] · Discard… — always visible; only a
    // published row's Open page and Delete… wait for hover or selection.
    assert_eq!(
        all[0].buttons.iter().map(|b| (b.label.as_str(), b.style)).collect::<Vec<_>>(),
        [("Get link", Style::Filled), ("Discard…", Style::RedText)],
        "its one filled button leads"
    );
    assert!(all[0].hover.is_empty());
    assert_eq!(all[0].tone, Some(shroom_core::scene::Tone::Amber));
    assert!(!all[0].buttons.iter().any(|b| matches!(b.action, Action::CopyRow(_))), "no link exists yet (law 3)");

    let stranded = row(&s, "Design review");
    assert_eq!(stranded.state.as_deref(), Some("Waiting for your space"));
    assert_eq!(stranded.buttons.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(), ["Copy link", "Save to Movies"]);
    assert_eq!(row(&s, "Demo for Mher").state.as_deref(), Some("Waiting for connection"));
    let up = row(&s, "Untitled — Sep 30");
    assert_eq!((up.state.as_deref(), up.progress), (Some("Uploading — 40%"), Some(40)));
    let gone = row(&s, "Old demo");
    assert_eq!(gone.state.as_deref(), Some("Removed from the space"));
    assert_eq!(gone.buttons.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(), ["Save to Movies", "Discard…"], "always visible, as the spec writes it");
    assert_eq!(gone.meta, "Aug 19 · 0:48", "date · length — the length known here since the stop");
    let live = row(&s, "Where the 502s come from");
    assert_eq!(
        live.menu.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(),
        ["Copied", "Copy thumbnail", "Open page", "Edit…", "Rename", "Delete…"],
        "N46: Copy thumbnail under Copy link; N63: Edit… after Open page"
    );
    assert_eq!(live.gestures.double_click, Some(Action::OpenPage(rec("r502"))));
    assert_eq!(s.library.as_ref().unwrap().body.footer.as_deref(), Some("12 videos"));

    // Discard… on the recovered row is N25's question, not the delete sheet — and its yes
    // is the person's own (law 2).
    let mut r = Run::new(library_rich());
    let dir = TakeDir("20261001-090000-cd34".into());
    r.lib(Action::DiscardRow(dir.clone()));
    assert!(r.ids().contains(&"discard-confirm") && r.ids().contains(&"discard-confirm-saved-take"));
    assert_eq!(r.scene.library.as_ref().unwrap().body.sheet.as_ref().unwrap().lines, ["4:32 was saved. There's no undo."]);
    r.lib(Action::ConfirmDiscardRow);
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(dir.clone()))));
    assert!(r.w.saved.is_empty());
    r.lib(Action::SaveRowToMovies(TakeDir("qgone".into())));
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir("qgone".into())])));
    r.lib(Action::OpenPage(rec("r19")));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::OpenOwnerPage(rec("r19")))), "the owner-keyed URL (B25)");

    // Another Mac's delete stops this Mac's upload: the row stays, its maker's (law 2).
    let mut r = Run::new(library_rich());
    r.send(Event::Upload(UploadNews::Tombstoned(TakeDir("q40".into()))));
    assert_eq!(row(&r.scene, "Untitled — Sep 30").state.as_deref(), Some("Removed from the space"));
    assert!(!r.has(|e| matches!(e, Effect::Takes(TakeCmd::Delete(_)))));
    r.send(Event::Scanned { saved: r.w.saved.clone(), queue: vec![queued("q40", "up40", "Untitled — Sep 30", 2, 5, None)], removed: vec![] });
    assert_eq!(row(&r.scene, "Untitled — Sep 30").state.as_deref(), Some("Removed from the space"), "a rescan doesn't requeue it");
}

#[test]
fn settings_opens_on_general_then_the_last_tab_and_the_space_tab_repairs_by_the_same_machine() {
    let mut r = Run::new(with_popover(ready()));
    r.act(Action::OpenSettings);
    assert_eq!(faces(&r.scene), ["settings-general"], "first open ever: General");
    assert!(!general(&r.scene).login.on, "the box reads the system's truth");
    r.send(Event::Os(OsNews::LoginItem(true)));
    assert!(general(&r.scene).login.on);
    r.set(Action::SetLoginItem(false));
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::SetLoginItem(false))));
    r.set(Action::ReportIssue);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::OpenUrl(PublicUrl::NewIssue { .. })))));

    r.set(Action::SettingsTab(SettingsTab::Space));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.settings_tab == Some(SettingsTab::Space))));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "every entry re-probes (law 1)");
    r.cloud(CloudNews::Swept(swept_ok()));
    let f = faces(&r.scene);
    assert!(f.contains(&"settings-space-inspection") && f.contains(&"re-entry-as-settings"), "{f:?}");
    let p = plan(&r.scene);
    assert_eq!(
        p.rows.iter().map(|x| x.line.as_str()).collect::<Vec<_>>(),
        [
            "✓ Cloudflare account — Bagrat's Account",
            "✓ Access token valid — on this Mac",
            "✓ Cloudflare billing set up",
            "✓ Your link name — v.myshroom.workers.dev",
            "✓ Wired up",
            "✓ macOS permissions granted — on this Mac",
        ]
    );
    assert_eq!(p.nest, Nest::Ready { line: "Your space is ready — nothing to redo.".into() });

    // Close, reopen: the last tab used, re-probed again.
    r.send(Event::WindowClosed(WindowKind::Settings));
    r.click();
    r.act(Action::OpenSettings);
    assert!(faces(&r.scene).contains(&"settings-space"));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));

    // The re-probe finds the token dead: the repair mood, at token~repair-mood.
    r.cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() }));
    let f = faces(&r.scene);
    assert!(f.contains(&"settings-space-repair") && f.contains(&"token-repair-mood"), "{f:?}");
    assert_eq!(plan(&r.scene).current, Some(2));
    assert_eq!(plan(&r.scene).rows[1].line, "→ One access token for shroom (a pre-filled page — review, create, paste)");
    let Nest::Token { line, .. } = &plan(&r.scene).nest else { panic!() };
    assert_eq!(line, "Cloudflare stopped accepting shroom's token — create a fresh one and paste it here. Nothing else needs redoing.");
    assert_eq!(r.scene.tray.face, Face::Amber, "something needs you");
    r.key(Surface::Settings, Key::Return);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenUrl(PublicUrl::TokenPage))));

    // A token for another account: never a second space by accident.
    r.paste(1);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Validate(SecretId(1)))));
    r.cloud(CloudNews::Validated { id: SecretId(1), result: Ok(vec![acme()]) });
    assert!(faces(&r.scene).contains(&"token-other-account"));
    assert!(json(&r.scene).contains("create it in the account that holds v.myshroom.workers.dev."));
    r.paste(2);
    assert_eq!(faces(&r.scene).iter().filter(|f| **f == "token").count(), 1, "the paste is checking under the token row");
    r.cloud(CloudNews::Validated { id: SecretId(2), result: Ok(vec![acme(), space_account()]) });
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SaveToken(SecretId(2), "acct".into()))), "the space's own account, among several");
    r.cloud(CloudNews::TokenSaved(SecretId(2)));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "fixed: the plan re-probes");
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"), "all probes pass (law 1)");
    assert_eq!(r.scene.tray.face, Face::Plain);
    assert_eq!(r.w.space.broken.get(), Some(&false));
}

/// Each break un-ticks its own row; the nest is Region 2's state, and the fix re-probes.
#[test]
fn the_repair_mood_routes_each_break_to_its_row() {
    let refocus = |r: &mut Run| {
        r.send(Event::WindowFocused(WindowKind::Settings, quick()));
        assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "re-probed when the window regains focus");
    };
    let mut r = Run::new(space_tab());

    // The card removed: gate-card, watched like the wizard's.
    refocus(&mut r);
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(Gates { email: false, card: true }), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"gate-card"));
    assert_eq!(plan(&r.scene).current, Some(3));
    assert_eq!(r.scene.settings.as_ref().unwrap().ret, Some(Action::AddCard));
    let poll = r.scene.next_wake.unwrap();
    r.tick_to(poll.0);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates))));
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(Gates { email: false, card: false }), ..Default::default() }));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"));

    // The bucket gone: space-storage-gone, Set it up fresh on choice.
    refocus(&mut r);
    r.cloud(CloudNews::Swept(SweepAnswer { bucket: Some(false), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"space-storage-gone"));
    assert_eq!(plan(&r.scene).current, Some(5));
    let Nest::StorageGone { buttons, .. } = &plan(&r.scene).nest else { panic!() };
    assert_eq!(buttons.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(), ["Set it up fresh", "Disconnect this Mac…"]);
    r.key(Surface::Settings, Key::Return);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into() })));
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "wired: the plan re-probes");
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"));

    // The name changed: one line, re-probed on return.
    refocus(&mut r);
    r.cloud(CloudNews::Swept(SweepAnswer { subdomain: Some(Some("myshroomhq".into())), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"name-changed"));
    assert_eq!(plan(&r.scene).current, Some(4));
    assert_eq!(r.scene.settings.as_ref().unwrap().ret, None);
    refocus(&mut r);
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"));

    // The page or its key gone: wiring runs again by itself (the click was the consent).
    refocus(&mut r);
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::KeyGone), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"wiring"));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Wire { label: "v".into() })));
    r.cloud(CloudNews::Wired(Ok(identity())));
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"));

    // Offline mid-repair: the wizard's own face.
    refocus(&mut r);
    r.send(Event::Os(OsNews::NetPath(false)));
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::Absent), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"wizard-offline"));
}

#[test]
fn space_degraded_asks_in_the_popover_and_reconnect_opens_the_repair_mood() {
    let mut w = with_popover(ready());
    w.queue.push(queued("q1", "u1", "Untitled — Sep 30", 1, 4, None));
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space)));
    assert_eq!(banner(&r.scene), Some("space-degraded"));
    assert_eq!(r.scene.tray.face, Face::Amber);
    let p = r.scene.popover.as_ref().unwrap();
    assert_eq!(p.ret, Some(Action::Start), "the banner's button is never the Return default");
    let shroom_core::scene::PopoverBody::Config(c) = &p.body else { panic!() };
    assert!(c.start.enabled, "Start recording stays live while degraded");

    r.act(Action::ReconnectSpace);
    assert!(r.scene.popover.is_none());
    let f = faces(&r.scene);
    assert!(f.contains(&"settings-space-repair") && f.contains(&"repair-mood"), "{f:?}");
    assert_eq!(plan(&r.scene).nest, Nest::Checking, "re-probed from scratch — no stored step");
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r.cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() }));
    assert!(faces(&r.scene).contains(&"token-repair-mood"));

    // A saved take outranks it; it outranks recording-off and an update (N38).
    let mut w = with_popover(update_ready(ready(), "1.3.0"));
    w.space.broken = Fact::known(true, Mono(950));
    w.mac.screen = Fact::known(false, Mono(0));
    assert_eq!(banner(&derive(&w)), Some("space-degraded"));
    w.saved.push(saved(SavedCause::Interrupted, 1_000));
    assert_eq!(banner(&derive(&w)), Some("recovered-take"));
    w.saved.clear();
    w.space.broken = Fact::known(false, Mono(950));
    assert_eq!(banner(&derive(&w)), Some("recording-off"));
    w.mac.screen = Fact::known(true, Mono(0));
    assert_eq!(banner(&derive(&w)), Some("update-ready"));

    // An upload going through says the space takes videos again.
    let mut w = with_popover(ready());
    w.queue.push(queued("q1", "u1", "t", 1, 4, Some(Blocked::Space)));
    w.space.broken = Fact::known(true, Mono(950));
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Live(TakeDir("q1".into()))));
    assert_eq!(banner(&r.scene), None);
}

#[test]
fn row_six_is_live_in_settings_and_turn_it_back_on_runs_the_chapter_there() {
    let mut w = space_tab();
    w.mac.mic = Fact::known(Access::Denied, Mono(0));
    let s = derive(&w);
    assert_eq!(plan(&s).rows[5].line, "✓ macOS permissions granted — microphone off for now", "a denied mic never blocks");
    let SettingsPane::Space(sp) = &s.settings.as_ref().unwrap().body.pane else { panic!() };
    assert_eq!(sp.mic.as_ref().map(|b| (b.label.as_str(), b.style)), Some(("Turn on the microphone", Style::Quiet)));

    // recording-off's banner: Settings, Space, row six current at perm-screen (N36, N38).
    let mut w = with_popover(ready());
    w.mac.screen = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    assert_eq!(banner(&r.scene), Some("recording-off"));
    r.act(Action::TurnBackOn);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))));
    assert!(faces(&r.scene).contains(&"perm-screen"), "{:?}", faces(&r.scene));
    assert!(faces(&r.scene).contains(&"settings-space-inspection"), "row six is per Mac — the space itself is sound");
    r.set(Action::ReopenShroom);
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Restore(Some(x))) if x.why == RelaunchWhy::Permission && x.window == WindowKind::Settings)));
    assert!(faces(&r.scene).contains(&"relaunching"));
    let restore = Restore { why: RelaunchWhy::Permission, window: WindowKind::Settings, written: Wall(1_790_000_000_000 + r.t as i64) };
    let mut n = reboot(&r, LaunchKind::PermissionRelaunch, "", Some(restore));
    n.send(Event::Os(OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }));
    assert!(faces(&n.scene).contains(&"perm-screen"), "back right where it was, in Settings");
    n.send(Event::Os(OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }));
    assert_eq!(plan(&n.scene).rows[5].line, "✓ macOS permissions granted — on this Mac");

    // The row's own Turn it back on, unasked otherwise.
    let mut w = space_tab();
    w.mac.screen = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    assert_eq!(plan(&r.scene).rows[5].line, "→ Letting macOS allow recording — screen recording is off");
    r.set(Action::TurnBackOn);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::RequestPermission(Perm::Screen))));
}

#[test]
fn disconnect_asks_waits_for_uploads_and_goes_back_to_the_start() {
    let mut w = space_tab();
    w.queue.push(queued("q1", "u1", "One", 1, 4, None));
    w.queue.push(queued("q2", "u2", "Two", 1, 4, None));
    let mut r = Run::new(w);
    r.set(Action::Disconnect);
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.title, "Disconnect this Mac?");
    assert_eq!(sheet.blocked.as_deref(), Some("Two videos are still uploading — you can disconnect once they're done."));
    assert!(!sheet.buttons[1].enabled, "blocked while uploads drain");
    r.set(Action::ConfirmDisconnect);
    assert!(r.effects.is_empty());
    r.key(Surface::Settings, Key::Return);
    assert!(r.effects.is_empty(), "no Return default (law 9)");
    r.send(Event::Upload(UploadNews::Live(TakeDir("q1".into()))));
    r.send(Event::Upload(UploadNews::Live(TakeDir("q2".into()))));
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.blocked, None);
    r.key(Surface::Settings, Key::Esc);
    assert!(r.scene.settings.as_ref().unwrap().body.sheet.is_none(), "Esc ≡ Cancel");
    r.set(Action::Disconnect);
    r.set(Action::ConfirmDisconnect);
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)));
    assert!(!r.has(|e| matches!(e, Effect::Shell(ShellCmd::SetLoginItem(_)))), "the login item stays as it was");
    assert!(r.ids().contains(&"idle-unprovisioned"));
    assert!(!r.scene.dock);
    r.click();
    assert_eq!(r.scene.onboarding.as_ref().map(|o| o.id), Some("welcome"), "the tray click opens the welcome");
    assert!(r.scene.dock, "setting up again: the Dock icon is back with the setup window");

    // A recovered take waiting blocks it too; mid-take both the exit and the sheet grey.
    let mut w = space_tab();
    w.saved.push(saved(SavedCause::Interrupted, 1_000));
    w.session.confirm = Some(Confirm::Exit);
    assert!(!derive(&w).settings.unwrap().body.sheet.unwrap().buttons[1].enabled);
    let mut w = space_tab();
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 1_000));
    let mut r = Run::new(w);
    r.set(Action::Disconnect);
    assert!(r.scene.settings.as_ref().unwrap().body.sheet.is_none(), "greyed mid-take (law 4)");
}

#[test]
fn an_update_is_offered_waits_out_the_take_applies_on_the_click_and_says_so_once() {
    let mut w = with_popover(ready());
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "checked quietly at launch");
    r.send(Event::Update(UpdateNews::Checked(Some(UpdateOffer::Ready { version: "1.3.0".into() }))));
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Download("1.3.0".into()))), "downloaded in the background");
    assert_eq!(r.scene.tray.face, Face::Plain, "nothing shows before it's verified");
    r.send(Event::Update(UpdateNews::Staged("1.3.0".into())));
    assert_eq!(r.scene.tray.face, Face::Frost);
    assert!(r.ids().contains(&"tray-frost-dot"));
    assert_eq!(banner(&r.scene), Some("update-ready"));
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::Start), "Return stays Start recording");
    // The update check's own due time (the scene's next wake is the earliest of every due
    // reaction — the token Mac's periodic self-check runs too, DECISIONS B51).
    let next = shroom_core::derive::next_check(&r.w).unwrap();
    assert_eq!(next.0 - r.w.update.checked.unwrap().0, 4 * 3_600_000, "the next check in a few hours");

    // Settings opened before the take: it stays; Update greys with its line.
    r.act(Action::OpenSettings);
    assert_eq!(general(&r.scene).update_line, "shroom 1.3.0 is ready.");
    r.click();
    r.key(Surface::Popover, Key::Return);
    let fire = r.w.session.arming.as_ref().unwrap().deadline;
    r.tick_to(fire.0);
    r.send(Event::Rec(RecNews::Started { dir: TakeDir("t1".into()), rec: rec("t1") }));
    assert_eq!(r.scene.tray.face, Face::Red, "the recording faces override the Frost dot");
    let up = general(&r.scene).update.clone().unwrap();
    assert!(!up.enabled);
    assert_eq!(up.note.as_deref(), Some("You can do this once your recording is finished."));
    r.set(Action::Update);
    assert!(r.effects.is_empty(), "never mid-take");
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 4_000 }));
    r.act(Action::Discard);
    r.act(Action::ConfirmDiscard);
    assert_eq!(r.scene.tray.face, Face::Frost, "the take ended; the dot was waiting");

    r.set(Action::Update);
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Apply("1.3.0".into()))));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Restore(Some(x))) if x.why == RelaunchWhy::Update && x.window == WindowKind::Settings)));
    assert_eq!(general(&r.scene).update_line, "Updating — back in a few seconds.");
    r.click();
    assert_eq!(banner(&r.scene), Some("updating-app"));
    assert_eq!(r.scene.popover.as_ref().unwrap().banner.as_ref().unwrap().line, "Updating — back in a few seconds.");

    // quit → swap → relaunch with --updated: the popover opens by itself and says so, and
    // the window that was open comes back — neither takes the keyboard (law 5). Bagrat,
    // 2026-10-15, the 0.1.12 checkpoint: “after update it is too silent, probably should
    // open the popover after relaunch” (amends N41).
    let restore = Restore { why: RelaunchWhy::Update, window: WindowKind::Settings, written: Wall(1_790_000_000_000 + r.t as i64) };
    let mut n = reboot(&r, LaunchKind::UpdateRelaunch, "1.3.0", Some(restore));
    let p = n.scene.popover.as_ref().expect("the popover opens by itself");
    assert!(!p.activates, "without taking the keyboard");
    assert!(p.away_closes, "a click anywhere else puts it away, like a popover you opened");
    assert_eq!(status(&n.scene).as_deref(), Some("Updated to 1.3.0 ✓"));
    assert!(n.ids().contains(&"updated"));
    let st = n.scene.settings.as_ref().expect("the window that was open comes back");
    assert!(!st.activates, "without taking the keyboard");
    n.send(Event::PopoverDismissed);
    assert!(n.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.seen_version.as_deref() == Some("1.3.0"))));
    n.click();
    assert_eq!(status(&n.scene), None, "once only");
}

#[test]
fn opening_the_popover_checks_again_once_the_last_check_is_a_while_ago_by_the_wall_clock() {
    // B154: the few-hours cadence counts awake time, so a Mac that sleeps between uses
    // would go days without looking; the popover opening looks too, at most every 15 min.
    let mut w = ready();
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "checked quietly at launch");
    r.send(Event::Update(UpdateNews::Checked(None)));
    r.click();
    assert!(r.scene.popover.is_some());
    assert!(!r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "not again straight away");
    r.send(Event::PopoverDismissed);

    // A night asleep: no awake time passed, so the few-hours check isn't due…
    r.slept += 10 * 3_600_000;
    r.send(Event::Tick);
    assert!(!r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "nothing while the popover is closed");
    assert!(shroom_core::derive::next_check(&r.w).unwrap().0 > r.t);
    // …but opening the popover looks.
    r.click();
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "the popover opening checks");
    r.send(Event::Update(UpdateNews::Checked(Some(UpdateOffer::Ready { version: "1.3.0".into() }))));
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Download("1.3.0".into()))), "downloaded in the background");
    r.send(Event::PopoverDismissed);

    // A failed check stays silent and is throttled the same way.
    r.slept += 3_600_000;
    r.click();
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)));
    let line = status(&r.scene);
    r.send(Event::Failed { op: Op::UpdateCheck, class: FailClass::Transport });
    assert_eq!(status(&r.scene), line, "a failed check says nothing");
    r.send(Event::PopoverDismissed);
    r.slept += 14 * 60_000;
    r.click();
    assert!(!r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "within 15 minutes of the last try");
    r.send(Event::PopoverDismissed);
    r.slept += 60_000;
    r.click();
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "15 minutes on, it looks again");
    r.send(Event::Update(UpdateNews::Checked(None)));
    r.send(Event::PopoverDismissed);

    // Offline, the open popover doesn't try.
    r.click();
    r.send(Event::Os(OsNews::NetPath(false)));
    r.slept += 3_600_000;
    r.send(Event::Tick);
    assert!(r.scene.popover.is_some());
    assert!(!r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "never while offline");
    r.send(Event::Os(OsNews::NetPath(true)));
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Check)), "back online with the popover open, it looks");
}

#[test]
fn a_failed_update_says_so_once_and_tries_again_later() {
    let mut w = ready();
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    let r = Run::new(w);
    let mut n = reboot(&r, LaunchKind::UpdateFailed, "1.2.0", None);
    assert!(!n.has(|e| *e == Effect::Updater(UpdCmd::Check)), "not straight away");
    let p = n.scene.popover.as_ref().expect("the popover opens by itself, as after an update");
    assert!(!p.activates && p.away_closes);
    assert_eq!(status(&n.scene).as_deref(), Some("The update didn't finish — shroom will try again later."));
    assert!(n.ids().contains(&"update-failed"));
    n.send(Event::PopoverDismissed);
    n.click();
    assert_eq!(status(&n.scene), None);
    // The check's own due time (the next wake may be the self-check's, DECISIONS B51).
    let later = shroom_core::derive::next_check(&n.w).unwrap();
    n.tick_to(later.0);
    assert!(n.has(|e| *e == Effect::Updater(UpdCmd::Check)), "later: a few hours on");

    // The app that never quits for the swap: nothing changed, said once.
    let mut r = Run::new(with_popover(update_ready(ready(), "1.3.0")));
    r.act(Action::Update);
    let deadline = r.w.inflight.ops[&Op::UpdateApply];
    r.tick_to(deadline.0);
    assert_eq!(status(&r.scene).as_deref(), Some("The update didn't finish — shroom will try again later."));
    assert_eq!(banner(&r.scene), Some("update-ready"), "still ready to apply");

    // A fingerprint mismatch is discarded silently and retried later, never installed.
    let mut w = ready();
    w.launch.version = "1.2.0".into();
    w.update.checked = Some(Mono(950));
    w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: "1.3.0".into() }), Mono(950));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Download("1.3.0".into()))));
    r.send(Event::Failed { op: Op::UpdateDownload, class: FailClass::Refused });
    assert!(r.effects.is_empty() && r.scene.tray.face == Face::Plain);
}

#[test]
fn past_this_macs_macos_general_says_so_and_nothing_else_does() {
    let mut w = with_popover(settings_on(ready(), SettingsTab::General));
    w.launch.version = "1.3.2".into();
    let mut r = Run::new(w);
    r.send(Event::Tick);
    r.send(Event::Update(UpdateNews::Checked(Some(UpdateOffer::NeedsMacos { version: "1.4.0".into(), macos: 15 }))));
    assert!(r.effects.is_empty(), "nothing to download");
    assert_eq!(r.scene.tray.face, Face::Plain, "no dot");
    assert_eq!(banner(&r.scene), None, "no banner");
    assert_eq!(general(&r.scene).update_line, "shroom 1.4.0 needs macOS 15 — this Mac stays on 1.3.2.");
    assert!(general(&r.scene).update.is_none());

    // Before a space exists: no check, no dot, no banner.
    let mut w = ready();
    w.space.identity = None;
    w.launch.version = "1.2.0".into();
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!r.has(|e| matches!(e, Effect::Updater(_))));
    let mut w = update_ready(ready(), "1.3.0");
    w.space.identity = None;
    assert_eq!(derive(&w).tray.face, Face::Plain);
}

#[test]
fn a_window_open_before_the_take_stays_and_greys_what_it_cant_do() {
    let mut w = library_rich();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 1_000));
    let mut r = Run::new(w);
    assert!(r.scene.library.is_some(), "their screen, their choice (N41)");
    let live = row(&r.scene, "Where the 502s come from").clone();
    let live = &live;
    let open = button(live, &Action::OpenPage(rec("r502")));
    assert!(!open.enabled && open.note.as_deref() == Some("You can do this once your recording is finished."));
    assert!(!lib_rows(&r.scene)[0].buttons[0].enabled, "Get link waits for the take");
    r.lib(Action::OpenPage(rec("r502")));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::OpenOwnerPage(_)))), "no page opens mid-take");
    assert!(button(live, &Action::CopyRow(rec("r502"))).enabled, "copying puts nothing on screen");

    // A window opened during the take never shows until it ends (law 4).
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 1_000));
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    assert!(derive(&w).settings.is_none());
}

#[test]
fn every_region5_op_has_a_deadline() {
    for (op, ms) in [(Op::LibRefresh, 60_000), (Op::Rename, 60_000), (Op::Delete, 60_000), (Op::Suggest, 60_000), (Op::Keep, 60_000), (Op::Plays, 60_000), (Op::UpdateCheck, 10_000), (Op::UpdateDownload, 1_800_000), (Op::UpdateApply, 30_000)] {
        assert_eq!(op.deadline_ms(), ms, "{op:?}");
    }
    // A refresh that never answers ends at its deadline; the window still shows the cache.
    let mut r = Run::new(with_popover(library(ready())));
    r.send(Event::WindowFocused(WindowKind::Library, quick()));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Refresh)));
    let deadline = r.w.inflight.ops[&Op::LibRefresh];
    r.tick_to(deadline.0);
    assert!(r.scene.library.is_some() && lib_rows(&r.scene).len() == 7);
    // A rename that never lands: the row reads as the space has it.
    let mut r = Run::new(library(ready()));
    r.lib(Action::StartRename(rec("r19")));
    r.field(Field::Title, "Aug 19 retro");
    r.lib(Action::CommitRename);
    let deadline = r.w.inflight.ops[&Op::Rename];
    r.tick_to(deadline.0);
    assert!(lib_rows(&r.scene).iter().any(|x| x.title.as_deref() == Some("Untitled — Aug 19")));
}

// ── the machine's drawn edges (TESTING-PLAN §2.2) ──

/// What the app shows at an end that isn't an app state of its own (a page, a status-line
/// fact), or at a map note this slice also walks.
fn shows(name: &str) -> &str {
    match name {
        "library-faces" => "library-offline",
        "removed" => "library",
        "update-failed" => "idle-ready",
        other => other,
    }
}

/// The map's notes walked here: not states, so not edges.
const NOTES: &[&str] = &["library-faces"];

fn realized() -> Vec<(&'static str, &'static str, World, Event)> {
    use Event as E;
    let at = |surface, action| E::Act { surface, action, rev: 0 };
    let repairing = || {
        let mut w = settings_on(ready(), SettingsTab::Space);
        w.space.broken = Fact::known(true, Mono(950));
        w
    };
    let applying = || {
        let mut w = with_popover(update_ready(ready(), "1.3.0"));
        w.update.applying = true;
        w
    };
    vec![
        ("config", "library", with_popover(ready()), at(Surface::Popover, Action::OpenLibrary)),
        ("library", "delete-confirm", library(ready()), at(Surface::Library, Action::DeleteRow(rec("r502")))),
        ("library", "library-faces", library(ready()), E::Os(OsNews::NetPath(false))),
        ("delete-confirm", "removed", { let mut w = library(ready()); w.session.confirm = Some(Confirm::Delete(vec![rec("r502")])); w }, at(Surface::Library, Action::ConfirmDelete)),
        ("config", "settings-general", with_popover(ready()), at(Surface::Popover, Action::OpenSettings)),
        ("settings-space", "disconnect-confirm", space_tab(), at(Surface::Settings, Action::Disconnect)),
        ("update-ready", "updating-app", with_popover(update_ready(ready(), "1.3.0")), at(Surface::Popover, Action::Update)),
        // The swap helper's `--update-failed` relaunch: a new process, so the crossing boots a
        // blank World from `update-ready · updating`'s disk (the in-process path — the app
        // never quit — leaves `update-ready · updating` in a_failed_update_says_so_once_…).
        ("updating-app", "update-failed", applying(), E::Booted(Box::new(disk(LaunchKind::UpdateFailed, &applying(), "1.2.0", None)))),
        ("idle-ready", "space-degraded", { let mut w = with_popover(ready()); w.queue.push(queued("q1", "u1", "t", 1, 4, None)); w }, E::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space))),
        ("space-degraded", "repair-mood", { let mut w = with_popover(ready()); w.space.broken = Fact::known(true, Mono(950)); w }, at(Surface::Popover, Action::ReconnectSpace)),
        ("repair-mood", "space-storage-gone", repairing(), E::Cloud(CloudNews::Swept(SweepAnswer { bucket: Some(false), ..swept_ok() }))),
        ("repair-mood", "name-changed", repairing(), E::Cloud(CloudNews::Swept(SweepAnswer { subdomain: Some(Some("myshroomhq".into())), ..swept_ok() }))),
    ]
}

/// Region 2's states this slice owns: the wizard's afterlife.
const AFTERLIFE: &[&str] = &["space-degraded", "repair-mood", "space-storage-gone", "name-changed"];

#[test]
fn every_region5_and_afterlife_drawn_edge_is_realized() {
    let in_scope = drawn_edges(|from, to| [from, to].iter().any(|s| region_of(s) == 5 || AFTERLIFE.contains(s)));
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let mut failures = Vec::new();
    for (from, to, world, event) in realized() {
        assert!(NOTES.contains(&to) || covered.insert((from.into(), to.into())), "{from} → {to} realized twice");
        let before = derive(&world).state_ids();
        if !before.contains(&shows(from)) {
            failures.push(format!("{from} → {to}: the start world derives {before:?}"));
            continue;
        }
        let booted = matches!(event, Event::Booted(_));
        let mut r = if booted { Run::new(World::blank()) } else { Run::new(world) };
        r.send(event);
        let after = r.ids();
        let crossed = after.contains(&shows(to))
            && match to {
                "removed" => r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("r502")]))),
                "update-failed" => r.w.update.failed,
                "library" => r.scene.library.is_some(),
                _ => true,
            };
        if !crossed {
            failures.push(format!("{from} → {to}: ended at {after:?}"));
        }
    }
    let missing: Vec<_> = in_scope.difference(&covered).collect();
    let stale: Vec<_> = covered.difference(&in_scope).collect();
    assert!(missing.is_empty(), "drawn edges not realized: {missing:?}");
    assert!(stale.is_empty(), "realized edges the machine doesn't draw: {stale:?}");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn library_dates_are_this_macs_calendar() {
    assert_eq!(shroom_core::copy::day(Wall(AUG21), 0), "Aug 21");
    assert_eq!(shroom_core::copy::day(Wall(AUG21 - 12 * 3_600_000), -60), "Aug 20", "an hour west of UTC, just after midnight UTC");
    assert_eq!(shroom_core::copy::day(Wall(SEP30), 0), "Sep 30");
    assert_eq!(shroom_core::copy::day(Wall(0), 0), "Jan 1");
    let mut w = library(ready());
    let mut r = Run::new(w.clone());
    r.send(Event::Os(OsNews::UtcOffset(14 * 60)));
    assert_eq!(row(&r.scene, "Where the 502s come from").meta, "Aug 22 · 4:32");
    w.session.fields.library.search = "UNTITLED".into();
    assert_eq!(titles(&derive(&w)), ["Untitled — Aug 19"], "search ignores case");
    let _ = update;
}

/// Each Session field's clear rule, as its doc comment states it (ARCHITECTURE §3.2).
#[test]
fn region5_session_fields_clear_by_their_rules() {
    // A window's sheet is that window's: the popover closing leaves it.
    let mut w = with_popover(library(ready()));
    w.session.confirm = Some(Confirm::Delete(vec![rec("r502")]));
    let mut r = Run::new(w);
    r.send(Event::PopoverDismissed);
    assert!(r.scene.library.as_ref().unwrap().body.sheet.is_some());
    // The Library's door closing takes its fields and its question.
    r.field(Field::LibrarySearch, "502");
    r.send(Event::WindowClosed(WindowKind::Library));
    assert_eq!(r.w.session.confirm, None);
    assert_eq!(r.w.session.fields.library, LibraryFields::default());
    // The onboarding window closing leaves the Library's fields alone.
    let mut w = library(ready());
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(900)));
    w.session.fields.library.search = "502".into();
    let mut r = Run::new(w);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert_eq!(r.w.session.fields.library.search, "502");
    // Settings closing takes its sheet and the chapter it ran.
    let mut w = space_tab();
    w.session.confirm = Some(Confirm::Exit);
    w.session.chapter = true;
    let mut r = Run::new(w);
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert_eq!((r.w.session.confirm.clone(), r.w.session.chapter), (None, false));
    // A footer door closes the popover as it opens its window.
    let mut r = Run::new(with_popover(ready()));
    r.act(Action::OpenLibrary);
    assert!(r.w.session.popover.is_none());
}

/// B24 in the repair mood: a Worker under the space's label that isn't shroom's is never
/// written over — wiring rests (DEVIATIONS #11's gap).
#[test]
fn repair_never_rewires_over_someone_elses_worker() {
    let mut r = Run::new(space_tab());
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::Foreign), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"wiring"));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
}

/// The repair mood's token field is the wizard's (N42): it holds a taken paste while it's
/// checked and put away, and is empty again when the paste is turned down (DECISIONS B104).
#[test]
fn the_repair_moods_token_field_holds_a_paste_the_way_the_wizards_does() {
    let held = |s: &Scene| {
        let n = serde_json::to_value(&plan(s).nest).unwrap();
        assert_eq!(n["kind"], "token", "the token row's nest");
        n["held"].clone()
    };
    let (yes, no) = (serde_json::json!(true), serde_json::json!(false));
    let mut r = Run::new(token_dead());
    assert_eq!(held(&r.scene), no, "the token that died is no paste: the field is empty");
    r.send(Event::SecretPasted { field: SecretField::Token, id: SecretId(1), shape: Shape::Garbled });
    assert_eq!(held(&r.scene), no);

    // Taken, then turned down — another account's token: empty again, with its line.
    r.paste(2);
    assert_eq!(held(&r.scene), yes, "the paste was taken");
    r.cloud(CloudNews::Validated { id: SecretId(2), result: Ok(vec![acme()]) });
    assert!(faces(&r.scene).contains(&"token-other-account"));
    assert_eq!(held(&r.scene), no);

    // Accepted: still held while it's put away — then the row ticks and the field is gone.
    r.paste(3);
    r.cloud(CloudNews::Validated { id: SecretId(3), result: Ok(vec![space_account()]) });
    assert_eq!(held(&r.scene), yes, "accepted, being put away");
    r.cloud(CloudNews::TokenSaved(SecretId(3)));
    assert!(!matches!(plan(&r.scene).nest, Nest::Token { .. }), "{:?}", plan(&r.scene).nest);
}

// ── uninstall (B135) ──

fn sheet_of(r: &Run) -> shroom_core::scene::Sheet {
    r.scene.settings.as_ref().unwrap().body.sheet.clone().expect("a sheet")
}

fn uninstall_link(s: &Scene) -> shroom_core::scene::Button {
    match &s.settings.as_ref().unwrap().body.pane {
        SettingsPane::General(g) => g.uninstall.clone(),
        other => panic!("not General: {other:?}"),
    }
}

/// General's quiet Uninstall shroom… asks first under N23's law: the space stays, this
/// Mac lets go, how to come back; Cancel and a red-outlined Uninstall, no Return default.
#[test]
fn uninstall_asks_first_and_says_the_space_stays() {
    // Your videos has read the space already (N68): asking and cancelling read nothing more.
    let mut r = Run::new(suggested(settings_on(ready(), SettingsTab::General), vec![], 34_000_000_000));
    let link = uninstall_link(&r.scene);
    assert_eq!((link.label.as_str(), link.style, link.enabled), ("Uninstall shroom…", Style::Quiet, true));
    r.set(Action::Uninstall);
    assert!(r.ids().contains(&"uninstall-confirm"), "{:?}", r.ids());
    assert!(r.effects.is_empty(), "asking changes nothing");
    let sheet = sheet_of(&r);
    assert_eq!(sheet.title, "Uninstall shroom?");
    assert_eq!(sheet.lines, [
        "Your space, every video in it and every link stay exactly as they are.",
        "This Mac lets go of shroom: the app, its settings, and its permission to record your screen, microphone and camera.",
        "To come back, install shroom again and set it up with a fresh token — everything will be there.",
    ]);
    assert_eq!(sheet.buttons.iter().map(|b| (b.label.as_str(), b.style, b.enabled)).collect::<Vec<_>>(), [("Cancel", Style::Plain, true), ("Uninstall", Style::RedOutline, true)]);
    assert_eq!((r.scene.ret(Surface::Settings), r.scene.esc(Surface::Settings)), (None, Some(Action::Back)));
    r.key(Surface::Settings, Key::Esc);
    assert_eq!(faces(&r.scene), ["settings-general"], "Esc ≡ Cancel: back on General, nothing done");
    assert!(r.effects.is_empty());
}

/// A member comes back with a new invite — and the space that stays is the owner's (B156).
#[test]
fn a_member_comes_back_with_a_new_invite() {
    let mut w = uninstall_asked();
    let id = w.space.identity.as_mut().unwrap();
    id.role = Role::Member;
    id.owner_name = "Bagrat".into();
    let sheet = derive(&w).settings.unwrap().body.sheet.unwrap();
    assert_eq!(sheet.lines.first().map(String::as_str), Some("Your videos and every link stay exactly as they are, in Bagrat's space."));
    assert_eq!(sheet.lines.last().map(String::as_str), Some("To come back, install shroom again and ask for a new invite."));
}

/// The answer: the button reads Uninstalling…, both grey, and the shell is asked to remove
/// shroom — once. Nothing reaches the space: no cloud, member or library effect.
#[test]
fn uninstall_hands_the_shell_its_removal_once_and_never_touches_the_space() {
    let mut r = Run::new(uninstall_asked());
    r.set(Action::ConfirmUninstall);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::Uninstall)]);
    let sheet = sheet_of(&r);
    assert_eq!(sheet.buttons.iter().map(|b| (b.label.as_str(), b.enabled)).collect::<Vec<_>>(), [("Cancel", false), ("Uninstalling…", false)]);
    r.set(Action::ConfirmUninstall);
    assert!(r.effects.is_empty(), "a second click asks nothing more");
    r.key(Surface::Settings, Key::Esc);
    assert!(r.ids().contains(&"uninstall-confirm"), "under way: Esc doesn't take it back");
}

/// Disconnect's guards (N31's rule): uploads that can still drain and a waiting recovered
/// take block it with one line; stranded takes don't — one line, and Movies first.
#[test]
fn uninstall_waits_for_uploads_and_offers_movies_for_stranded_takes() {
    let mut w = uninstall_asked();
    w.queue.push(queued("q1", "u1", "One", 1, 4, None));
    let mut r = Run::new(w);
    let sheet = sheet_of(&r);
    assert_eq!(sheet.blocked.as_deref(), Some("One video is still uploading — you can uninstall once it's done."));
    assert!(!sheet.buttons.last().unwrap().enabled);
    r.set(Action::ConfirmUninstall);
    assert!(!r.has(|e| *e == Effect::Shell(ShellCmd::Uninstall)), "blocked while it drains");

    let mut w = uninstall_asked();
    w.queue.push(queued("q1", "u1", "One", 1, 4, Some(Blocked::Space)));
    w.queue.push(queued("q2", "u2", "Two", 1, 4, Some(Blocked::Space)));
    let mut r = Run::new(w);
    let sheet = sheet_of(&r);
    assert_eq!(sheet.blocked, None);
    assert!(sheet.lines.contains(&"Two recordings never reached the space — uninstalling deletes them from this Mac.".to_string()));
    assert_eq!(sheet.buttons.iter().map(|b| b.label.as_str()).collect::<Vec<_>>(), ["Cancel", "Save them to Movies first", "Uninstall"]);
    r.set(Action::SaveStrandedToMovies);
    assert!(r.has(|e| saved_dirs(e).is_some()));
    r.set(Action::ConfirmUninstall);
    assert!(r.effects.is_empty(), "no removal while the copy is under way (law 2, B54)");
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("q1".into()))));
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("q2".into()))));
    r.set(Action::ConfirmUninstall);
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::Uninstall)));
}

/// Law 4: mid-take the link greys with its line, and the ask is dropped.
#[test]
fn uninstall_greys_mid_take() {
    // Opened before the take: it stays, and the link greys with its line.
    let mut w = settings_on(ready(), SettingsTab::General);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 30_000));
    let mut r = Run::new(w);
    let link = uninstall_link(&r.scene);
    assert!(!link.enabled);
    assert_eq!(link.note.as_deref(), Some("You can do this once your recording is finished."));
    r.set(Action::Uninstall);
    assert!(!r.ids().contains(&"uninstall-confirm"));
}

// ── the review's findings, each walked ──

/// N36's two exits: stranded takes don't block (they can't drain) — one line, Movies first,
/// and leaving deletes them from this Mac. space-storage-gone's own Disconnect… opens.
#[test]
fn stranded_takes_never_block_disconnect_and_leaving_deletes_them() {
    let mut w = space_tab();
    w.space.broken = Fact::known(true, Mono(950));
    w.queue.push(queued("qs", "us", "Design review", 1, 4, Some(Blocked::Space)));
    let mut r = Run::new(w);
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    r.cloud(CloudNews::Swept(SweepAnswer { bucket: Some(false), ..swept_ok() }));
    assert!(faces(&r.scene).contains(&"space-storage-gone"));
    r.set(Action::Disconnect);
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.blocked, None, "a take that can't drain doesn't block");
    assert!(sheet.lines.contains(&"One recording never reached the space — leaving deletes it from this Mac.".to_string()));
    assert_eq!(
        sheet.buttons.iter().map(|b| (b.label.as_str(), b.style, b.enabled)).collect::<Vec<_>>(),
        [("Cancel", Style::Plain, true), ("Save it to Movies first", Style::Plain, true), ("Disconnect", Style::RedOutline, true)]
    );
    r.set(Action::SaveStrandedToMovies);
    assert!(r.has(|e| saved_dirs(e) == Some(vec![TakeDir("qs".into())])));
    assert!(r.scene.settings.as_ref().unwrap().body.sheet.is_some(), "Movies first, then the question still stands");
    // “First” means first (law 2, DECISIONS B54): while the copy is under way the exit
    // waits for it, and nothing is deleted.
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(
        sheet.buttons.iter().map(|b| (b.label.as_str(), b.enabled)).collect::<Vec<_>>(),
        [("Cancel", true), ("Saving to Movies…", false), ("Disconnect", false)]
    );
    r.set(Action::ConfirmDisconnect);
    assert!(r.effects.is_empty(), "no exit while the copy is under way");
    r.send(Event::Rec(RecNews::SavedToMovies(TakeDir("qs".into()))));
    r.set(Action::ConfirmDisconnect);
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)));
    assert!(r.has(|e| *e == Effect::Takes(TakeCmd::Delete(TakeDir("qs".into())))), "leaving deletes it, as the sheet said");
    assert!(r.ids().contains(&"idle-unprovisioned"));

    // Two of them, and one still able to drain: that one blocks, counted alone.
    let mut w = space_tab();
    w.session.confirm = Some(Confirm::Exit);
    w.queue.push(queued("q1", "u1", "One", 1, 4, Some(Blocked::Space)));
    w.queue.push(queued("q2", "u2", "Two", 1, 4, Some(Blocked::Space)));
    w.queue.push(queued("q3", "u3", "Three", 1, 4, Some(Blocked::Offline)));
    let sheet = derive(&w).settings.unwrap().body.sheet.unwrap();
    assert_eq!(sheet.blocked.as_deref(), Some("One video is still uploading — you can disconnect once it's done."));
    assert!(sheet.lines.contains(&"Two recordings never reached the space — leaving deletes them from this Mac.".to_string()));
    assert_eq!(sheet.buttons[1].label, "Save them to Movies first");
    assert!(!sheet.buttons[2].enabled);
}

/// B139 (Bagrat, 2026-10-14): the Space tab's account row shows the account's email, as
/// written, when the account still has Cloudflare's own name for it (“<email>'s Account”);
/// any other name shows as it is.
#[test]
fn the_account_row_shows_the_email_alone() {
    let mut w = space_tab();
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "You@example.com's Account".into() }, several: false }, Mono(950));
    assert_eq!(plan(&derive(&w)).rows[0].line, "✓ Cloudflare account — you@example.com");
    w.sweep.token = Fact::known(TokenVerdict::Valid { account: Account { id: "acct".into(), name: "Studio Films".into() }, several: false }, Mono(950));
    assert_eq!(plan(&derive(&w)).rows[0].line, "✓ Cloudflare account — Studio Films");
}

/// N42: “whatever broke un-ticked, the first broken row current” — and row six un-ticks on
/// a revoked grant whatever row is current (N36).
#[test]
fn every_broken_row_unticks_and_only_the_first_is_current() {
    let mut w = space_tab();
    w.sweep.gates = Fact::known(Gates { email: false, card: true }, Mono(950));
    w.sweep.worker = Fact::known(WorkerFact::KeyGone, Mono(950));
    w.mac.screen = Fact::known(false, Mono(0));
    let s = derive(&w);
    let p = plan(&s);
    assert_eq!(p.current, Some(3));
    assert_eq!(
        p.rows.iter().map(|x| (x.glyph, x.line.as_str())).collect::<Vec<_>>(),
        [
            (shroom_core::scene::Glyph::Done, "✓ Cloudflare account — Bagrat's Account"),
            (shroom_core::scene::Glyph::Done, "✓ Access token valid — on this Mac"),
            (shroom_core::scene::Glyph::Current, "→ A credit card for storage (their rule, not a charge — free up to about three hours of video)"),
            (shroom_core::scene::Glyph::Done, "✓ Your link name — v.myshroom.workers.dev"),
            (shroom_core::scene::Glyph::Ahead, "• Wiring it up (shroom does this part)"),
            (shroom_core::scene::Glyph::Ahead, "• Letting macOS allow recording (microphone, camera and screen — a one-time yes each)"),
        ]
    );
}

/// N37: “Updated to … ✓” follows an in-app update's relaunch only; the install script's
/// manual update says nothing.
#[test]
fn only_an_update_relaunch_says_updated() {
    let mut w = ready();
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    let r = Run::new(w);
    let mut n = reboot(&r, LaunchKind::User, "1.3.0", None);
    assert!(n.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.seen_version.as_deref() == Some("1.3.0"))), "seen, silently");
    assert_eq!(status(&n.scene), None);
    n.send(Event::PopoverDismissed);
    n.click();
    assert_eq!(status(&n.scene), None, "no in-app update ran");
    let n = reboot(&r, LaunchKind::UpdateRelaunch, "1.3.0", None);
    assert_eq!(status(&n.scene).as_deref(), Some("Updated to 1.3.0 ✓"), "said in the popover the relaunch opens");
}

/// The popover an update relaunch opens is news a click away puts away; the others that
/// open by themselves (a pause or stop you didn't make) stay until touched (DEVIATIONS #39).
#[test]
fn only_the_update_relaunchs_popover_goes_on_a_click_away() {
    let mut w = ready();
    w.launch.version = "1.2.0".into();
    w.once.seen_version = Some("1.2.0".into());
    let r = Run::new(w);
    let mut n = reboot(&r, LaunchKind::UpdateRelaunch, "1.3.0", None);
    assert!(n.scene.popover.as_ref().unwrap().away_closes);
    n.send(Event::PopoverDismissed);
    assert!(n.scene.popover.is_none());
    n.click();
    let p = n.scene.popover.as_ref().unwrap();
    assert!(p.activates && !p.away_closes, "the next one is an ordinary click's");
    assert_eq!(status(&n.scene), None, "once only");
    // A login launch stays silent.
    let n = reboot(&r, LaunchKind::LoginItem, "1.2.0", None);
    assert!(n.scene.popover.is_none());
}

/// N35 offline: renaming waits until you're back — Return saves nothing, the typing stays.
#[test]
fn an_offline_return_never_saves_a_greyed_title() {
    let mut r = Run::new(library(ready()));
    r.lib(Action::StartRename(rec("r19")));
    r.field(Field::Title, "Retro");
    r.send(Event::Os(OsNews::NetPath(false)));
    assert_eq!(r.scene.library.as_ref().unwrap().ret, None, "no Return default on a greyed field");
    r.key(Surface::Library, Key::Return);
    assert!(r.effects.is_empty());
    r.lib(Action::CommitRename);
    assert!(r.effects.is_empty(), "a click-away saves nothing offline either");
    assert_eq!(r.w.session.fields.library.renaming, Some((rec("r19"), "Retro".into())), "the typing waits");
    r.send(Event::Os(OsNews::NetPath(true)));
    r.key(Surface::Library, Key::Return);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Rename(rec("r19"), "Retro".into()))));
}

/// N36: after Disconnect the app is back at the start — an answer to something asked before
/// it never sets the space up again, nor brings a token back.
#[test]
fn late_answers_after_disconnect_are_stale() {
    let mut r = Run::new(space_tab());
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::KeyGone), ..swept_ok() }));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "rewiring in flight");
    r.set(Action::Disconnect);
    r.set(Action::ConfirmDisconnect);
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)));
    assert!(r.w.inflight.ops.is_empty() || !r.w.inflight.ops.contains_key(&Op::Wire));
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r.effects.is_empty(), "no space written back: {:?}", r.effects);
    assert!(!r.w.provisioned());
    r.cloud(CloudNews::Swept(swept_ok()));
    assert_eq!(r.w.sweep, Sweep::default());
    r.send(Event::Library(LibraryNews::Rows { rows: space_rows(), fresh: true }));
    assert_eq!((r.w.library.clone(), r.w.library_ops.refreshed), (Fact::Unknown, None), "the old space's rows never come back");
    r.cloud(CloudNews::TokenSaved(SecretId(7)));
    assert!(r.has(|e| *e == Effect::Persist(PersistOp::Wipe)), "a token saved after the wipe is wiped again");
    assert!(r.ids().contains(&"idle-unprovisioned"));
}

/// Law 4 / N37: once Update is clicked the app quits any moment — no take opens under it.
#[test]
fn no_take_opens_while_an_update_applies() {
    let mut w = with_popover(update_ready(ready(), "1.3.0"));
    w.saved.push(saved(SavedCause::Interrupted, 1_000));
    let mut r = Run::new(settings_on(w, SettingsTab::General));
    r.set(Action::Update);
    assert!(r.w.update.applying);
    r.send(Event::PopoverDismissed);
    r.click();
    let p = r.scene.popover.as_ref().unwrap();
    let shroom_core::scene::PopoverBody::Config(c) = &p.body else { panic!() };
    assert!(!c.start.enabled, "Start greys");
    let get = &p.banner.as_ref().unwrap().buttons[0];
    assert_eq!((get.enabled, get.note.as_deref()), (false, Some("Updating — back in a few seconds.")));
    r.key(Surface::Popover, Key::Return);
    r.act(Action::Start);
    r.act(Action::GetLink);
    assert!(!r.w.take_open() && r.effects.is_empty());
}

/// Law 1: a paste the repair mood turned down never stops the next entry's re-probe.
#[test]
fn a_turned_down_paste_never_stops_the_next_reprobe() {
    let mut r = Run::new(token_dead());
    r.send(Event::SecretPasted { field: SecretField::Token, id: SecretId(1), shape: Shape::Garbled });
    assert!(faces(&r.scene).contains(&"token-garbled"));
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "a refocus re-probes, the error notwithstanding");
    r.cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() }));
    r.send(Event::WindowClosed(WindowKind::Settings));
    r.click();
    r.act(Action::OpenSettings);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))), "every entry re-probes");
    r.cloud(CloudNews::Swept(SweepAnswer { token: Some(TokenVerdict::Revoked), ..Default::default() }));
    let f = faces(&r.scene);
    assert!(f.contains(&"token-repair-mood") && !f.contains(&"token-garbled"), "read afresh: {f:?}");
}

/// Law 1: the degraded banner and the amber dot clear when uploads go through again, even
/// with the last re-probe's facts still saying a gate was up.
#[test]
fn space_degraded_clears_when_uploads_go_through() {
    let mut w = with_popover(space_tab());
    w.queue.push(queued("q1", "u1", "t", 1, 4, None));
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space)));
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    r.cloud(CloudNews::Swept(SweepAnswer { gates: Some(Gates { email: false, card: true }), ..swept_ok() }));
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert_eq!(r.scene.tray.face, Face::Amber);
    // The card is added in Cloudflare; the next upload goes through.
    r.send(Event::Upload(UploadNews::Progress { dir: TakeDir("q1".into()), done: 2, total: 4 }));
    r.send(Event::Upload(UploadNews::Live(TakeDir("q1".into()))));
    r.click();
    assert_eq!(banner(&r.scene), None);
    assert_eq!(r.scene.tray.face, Face::Plain);
}

/// A space that can't take videos is never hammered: a refused upload waits its backoff.
#[test]
fn a_blocked_upload_waits_its_backoff() {
    let mut w = ready();
    w.queue.push(queued("q1", "u1", "t", 1, 4, None));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| matches!(e, Effect::Uploader(_))));
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space)));
    // No re-kick at once. The one effect is N41's keep-awake letting go — nothing can drain
    // into a space that refuses it (DEVIATIONS #18: this line asserted no effect at all).
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::PowerAssertion(false))], "no re-kick at once: {:?}", r.effects);
    r.send(Event::Upload(UploadNews::Idle));
    assert!(r.effects.is_empty());
    let retry = r.w.inflight.failures[&Op::Upload].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| matches!(e, Effect::Uploader(_))), "one try per backoff notices recovery");
    r.send(Event::Upload(UploadNews::Blocked(TakeDir("q1".into()), Blocked::Space)));
    assert_eq!(r.w.inflight.failures[&Op::Upload].tries, 1, "the backoff grows");
    r.send(Event::Upload(UploadNews::Live(TakeDir("q1".into()))));
    assert!(!r.w.inflight.failures.contains_key(&Op::Upload));
}

/// The repair mood's rewire never ping-pongs with the probe that follows it.
#[test]
fn a_rewire_the_probe_disagrees_with_waits_its_backoff() {
    let mut r = Run::new(space_tab());
    r.send(Event::WindowFocused(WindowKind::Settings, quick()));
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::KeyGone), ..swept_ok() }));
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))));
    r.cloud(CloudNews::Wired(Ok(identity())));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::Sweep(SweepPart::Full))));
    r.cloud(CloudNews::Swept(SweepAnswer { worker: Some(WorkerFact::KeyGone), ..swept_ok() }));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "not straight back: {:?}", r.effects);
    let retry = r.w.inflight.failures[&Op::Wire].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| matches!(e, Effect::Cloud(CloudCmd::Wire { .. }))), "after the backoff");
    r.cloud(CloudNews::Wired(Ok(identity())));
    r.cloud(CloudNews::Swept(swept_ok()));
    assert!(!r.w.inflight.failures.contains_key(&Op::Wire), "a sound pass clears it");
}

/// DEVIATIONS #7 across a relaunch: the tombstoned take stays “Removed from the space”.
#[test]
fn a_removed_take_survives_a_relaunch() {
    let mut w = library(ready());
    w.queue.push(queued("q40", "up40", "Untitled — Sep 30", 2, 5, None));
    let mut r = Run::new(w);
    r.send(Event::Upload(UploadNews::Tombstoned(TakeDir("q40".into()))));
    let mut n = reboot(&r, LaunchKind::User, "", None);
    n.send(Event::Scanned { saved: vec![], queue: vec![], removed: vec![queued("q40", "up40", "Untitled — Sep 30", 2, 5, None)] });
    assert!(n.scene.popover.is_some(), "a launch you did is a click");
    n.act(Action::OpenLibrary);
    let gone = row(&n.scene, "Untitled — Sep 30");
    assert_eq!(gone.state.as_deref(), Some("Removed from the space"));
    n.lib(Action::DiscardRow(TakeDir("q40".into())));
    assert_eq!(n.scene.library.as_ref().unwrap().body.sheet.as_ref().unwrap().lines, ["0:48 was saved. There's no undo."], "N25's whole question");
}

/// A refresh begun before a delete or rename landed never brings the old row back.
#[test]
fn a_refresh_begun_before_a_write_landed_never_undoes_it() {
    let mut w = with_popover(ready());
    w.library = Fact::known(Arc::new(space_rows()), Mono(500));
    let mut r = Run::new(w);
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Refresh)), "a refresh begins");
    r.lib(Action::DeleteRow(rec("r502")));
    r.lib(Action::ConfirmDelete);
    r.send(Event::Library(LibraryNews::Deleted(vec![rec("r502")])));
    r.lib(Action::StartRename(rec("r19")));
    r.field(Field::Title, "Retro");
    r.lib(Action::CommitRename);
    r.send(Event::Library(LibraryNews::Renamed { rec: rec("r19"), title: "Retro".into() }));
    r.send(Event::Library(LibraryNews::Rows { rows: space_rows(), fresh: true }));
    assert!(!titles(&r.scene).iter().any(|t| t.contains("502")), "still deleted");
    assert!(titles(&r.scene).contains(&"Retro".to_string()) && !titles(&r.scene).contains(&"Untitled — Aug 19".to_string()), "still renamed");
    // A refresh begun after they landed is the space's word.
    r.send(Event::WindowFocused(WindowKind::Library, quick()));
    r.send(Event::Library(LibraryNews::Rows { rows: space_rows()[1..].to_vec(), fresh: true }));
    assert!(r.w.library_ops.landed.is_empty());
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
    let bad = laws::failures_are_states(&rows(), laws::failures_owned_by("region5"));
    assert!(bad.is_empty(), "law 12:\n{}", bad.join("\n"));
}

// ── Names, chapters and captions (N66) ──

fn space_pane(scene: &Scene) -> &shroom_core::scene::SpaceBody {
    match &scene.settings.as_ref().expect("Settings is open").body.pane {
        SettingsPane::Space(sp) => sp,
        _ => panic!("not the Space tab"),
    }
}

/// N66: on the Mac that holds the setup key, Settings · Space has the switch once the page has
/// said where it stands; on and off re-wire the page, the switch greying meanwhile and
/// mid-take (law 4); a re-wiring that didn't go through leaves it as the page has it.
#[test]
fn naming_is_a_switch_on_the_setup_mac_that_re_wires_the_page() {
    assert!(space_pane(&derive(&space_tab())).ai.is_none(), "not before the page said");
    let mut w = space_tab();
    w.space.ai = Some(false);
    let mut r = Run::new(w);
    let t = space_pane(&r.scene).ai.clone().expect("the switch");
    assert_eq!((t.label.as_str(), t.on, t.enabled), (shroom_core::copy::ai_label().as_str(), false, true));
    assert_eq!(space_pane(&r.scene).ai_line, Some(shroom_core::copy::ai_cost()));
    r.set(Action::SetAi(true));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAi(true))));
    assert!(!space_pane(&r.scene).ai.as_ref().unwrap().enabled, "greys while the page is re-wired");
    r.send(Event::Cloud(CloudNews::AiSet(true)));
    let t = space_pane(&r.scene).ai.clone().unwrap();
    assert!(t.on && t.enabled && t.action == Action::SetAi(false), "{t:?}");
    r.set(Action::SetAi(false));
    r.send(Event::Failed { op: Op::Ai, class: FailClass::Transport });
    assert!(space_pane(&r.scene).ai.as_ref().is_some_and(|t| t.on && t.enabled), "as the page has it");
    assert_eq!(space_pane(&r.scene).ai_line, Some(shroom_core::copy::ai_failed()), "and it says so (law 12)");
    r.set(Action::SetAi(false));
    r.send(Event::Cloud(CloudNews::AiSet(false)));
    assert_eq!(space_pane(&r.scene).ai_line, Some(shroom_core::copy::ai_cost()), "until it goes through");
    let mut w = space_tab();
    w.space.ai = Some(true);
    w.take = Some(OpenTake { opened: Mono(950), since: Mono(950), ..take(Phase::Recording, 61_000) });
    assert!(!space_pane(&derive(&w)).ai.as_ref().unwrap().enabled, "never mid-take");
}

/// N66: the setup Mac's page check (B50) says whether the page names videos.
#[test]
fn the_page_check_says_whether_naming_is_on() {
    let mut r = Run::new(space_tab());
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: Some(true) }));
    assert_eq!(r.w.space.ai, Some(true));
    assert!(space_pane(&r.scene).ai.as_ref().is_some_and(|t| t.on));
    // A check that couldn't tell leaves what was known.
    r.send(Event::Cloud(CloudNews::Refreshed { deployed: false, ai: None }));
    assert_eq!(r.w.space.ai, Some(true));
}

/// N66 (Bagrat, 2026-10-18: off by default, but visible): on the setup Mac, while the space's
/// AI is off, the popover's lowest banner offers it. Turn on… asks first, with what it costs;
/// Turn on re-wires the page, the banner says so, a Turn on that didn't go through says that
/// (law 12), and once it's on the status line says it, once.
#[test]
fn the_popover_offers_ai_on_the_setup_mac() {
    let banner = |r: &Run| r.scene.popover.as_ref().and_then(|p| p.banner.clone());
    let labels = |b: &shroom_core::scene::Banner| b.buttons.iter().map(|x| (x.label.clone(), x.style)).collect::<Vec<_>>();
    let mut r = Run::new(ai_offered());
    let b = banner(&r).expect("the banner");
    assert_eq!((b.id, b.tone, b.line.as_str()), ("ai-offer", shroom_core::scene::Tone::Frost, shroom_core::copy::ai_banner().as_str()));
    assert_eq!(labels(&b), [(shroom_core::copy::turn_on_ask(), Style::Filled), (shroom_core::copy::not_now(), Style::Link)]);
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::Start), "a banner's own button is never Return (law 8)");

    // Turn on… asks first: what it does and costs; Turn on is a click, Back and Esc go out.
    r.act(Action::AiAsk);
    let p = r.scene.popover.clone().unwrap();
    assert_eq!(p.id, "ai-confirm");
    let shroom_core::scene::PopoverBody::Question(q) = &p.body else { panic!("a question") };
    assert_eq!((q.title.as_str(), q.line.as_str()), (shroom_core::copy::ai_confirm_title().as_str(), shroom_core::copy::ai_cost().as_str()));
    assert_eq!(q.buttons.iter().map(|b| b.label.clone()).collect::<Vec<_>>(), [shroom_core::copy::back(), shroom_core::copy::turn_on()]);
    assert_eq!((p.ret, p.esc), (None, Some(Action::Back)));
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::SetAi(_)))), "asking costs nothing");
    r.act(Action::Back);
    assert_eq!(banner(&r).map(|b| b.id), Some("ai-offer"), "back: the banner stays");

    // Turn on: the page is re-wired, and the banner says so with nothing to press.
    r.act(Action::AiAsk).act(Action::AiOn);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAi(true))));
    let b = banner(&r).unwrap();
    assert_eq!((b.id, b.line.as_str(), b.buttons.len()), ("ai-turning-on", shroom_core::copy::ai_turning_on().as_str(), 0));
    assert!(r.scene.state_ids().contains(&"ai-offer"));

    // It didn't go through: said in place of the line, Turn on… live again (law 12).
    r.send(Event::Failed { op: Op::Ai, class: FailClass::Transport });
    let b = banner(&r).unwrap();
    assert_eq!((b.id, b.line.as_str()), ("ai-offer-failed", shroom_core::copy::ai_failed().as_str()));
    assert_eq!(labels(&b), [(shroom_core::copy::turn_on_ask(), Style::Filled), (shroom_core::copy::not_now(), Style::Link)]);

    // Again, and it's on: the banner goes, the status line says so once, and it's done here.
    r.act(Action::AiAsk).act(Action::AiOn);
    r.send(Event::Cloud(CloudNews::AiSet(true)));
    assert!(banner(&r).is_none());
    assert_eq!(status(&r.scene), Some(shroom_core::copy::ai_on_said()));
    assert!(r.scene.state_ids().contains(&"ai-on"));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.ai_offer_answered)), "answered for good on this Mac");
    r.act(Action::Dismiss);
    r.click();
    assert_ne!(status(&r.scene), Some(shroom_core::copy::ai_on_said()), "said once");
}

/// N66 (Bagrat, 2026-10-18: never again): Not now hides the banner for good on this Mac, however
/// many recordings follow; from then on only the switch in Settings · Space turns AI on. AI
/// turned on anywhere answers it too.
#[test]
fn not_now_hides_the_ai_banner_for_good() {
    let due = |w: &World| derive(w).popover.and_then(|p| p.banner).is_some_and(|b| b.id == "ai-offer");
    let mut r = Run::new(ai_offered());
    r.act(Action::AiNotNow);
    assert!(r.scene.popover.as_ref().unwrap().banner.is_none());
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.ai_offer_answered)), "kept across launches");
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::SetAi(_)))));
    let mut w = space_tab();
    w.space.ai = Some(false);
    w.once.ai_offer_answered = true;
    assert!(space_pane(&derive(&w)).ai.is_some(), "the switch stays");

    // Recordings never bring it back.
    let mut w = ai_offered();
    w.once.ai_offer_answered = true;
    w.session.popover = None;
    w.take = Some(OpenTake { phase: Phase::Finalizing(Why::Stop), ..take(Phase::Recording, 61_000) });
    let mut r = Run::new(w);
    r.send(Event::Rec(RecNews::Finalized(Ok(61_000))));
    assert!(!due(&r.w), "not after a recording");
    let mut w = ai_offered();
    w.once.ai_offer_answered = true;
    assert!(!due(&w), "not ever");

    // On from Settings answers it too, even if it's turned off again later.
    let mut r = Run::new(ai_offered());
    r.send(Event::Cloud(CloudNews::AiSet(true)));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Once(o)) if o.ai_offer_answered)));
}

/// N66, law 12: a Turn on that didn't go through is tried again quietly — ten minutes, then
/// longer — even with a take running meanwhile, until it goes through; Not now ends it.
#[test]
fn a_turn_on_that_didnt_go_through_is_tried_again_until_not_now() {
    let mut r = Run::new(ai_offered());
    r.act(Action::AiAsk).act(Action::AiOn);
    r.send(Event::Failed { op: Op::Ai, class: FailClass::Transport });
    let at = r.w.inflight.failures[&Op::Ai].ai_retry_at();
    r.tick_to(at.0 - 1);
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::SetAi(_)))), "not before its time");
    r.tick_to(at.0);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAi(true))), "tried again");
    assert_eq!(r.scene.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| b.id), Some("ai-turning-on"));
    r.send(Event::Cloud(CloudNews::AiSet(true)));
    assert_eq!(status(&r.scene), Some(shroom_core::copy::ai_on_said()), "and when it goes through, it's said");

    let mut r = Run::new(ai_offered());
    r.act(Action::AiAsk).act(Action::AiOn);
    r.send(Event::Failed { op: Op::Ai, class: FailClass::Refused });
    r.act(Action::AiNotNow);
    r.tick_to(10 * 3_600_000);
    assert!(!r.has(|e| matches!(e, Effect::Cloud(CloudCmd::SetAi(_)))), "Not now: no more tries");
}

/// N66: the banner sits under every other banner, and never mid-take or while the space's
/// AI is unknown.
#[test]
fn the_ai_banner_is_the_lowest_banner() {
    let mut w = ai_offered();
    w.mac.screen = Fact::known(false, Mono(0));
    assert_eq!(derive(&w).popover.unwrap().banner.map(|b| b.id), Some("recording-off"));
    let mut w = ai_offered();
    w.space.ai = None;
    assert!(derive(&w).popover.unwrap().banner.is_none(), "unknown: nothing offered");
}

/// N66: only the Mac that holds the setup key can re-wire the page, so only it shows naming:
/// never another of the owner's Macs, never a member's.
#[test]
fn naming_shows_only_where_the_setup_key_is() {
    for role in [Role::OwnerClass, Role::Member] {
        let mut w = space_tab();
        w.space.token = false;
        w.space.identity.as_mut().unwrap().role = role;
        w.space.ai = Some(false);
        if let Some(SettingsPane::Space(sp)) = derive(&w).settings.as_ref().map(|s| &s.body.pane) {
            assert!(sp.ai.is_none(), "{role:?}");
        }
        let w = with_popover(w);
        assert!(derive(&w).popover.unwrap().banner.is_none(), "{role:?}");
    }
}
