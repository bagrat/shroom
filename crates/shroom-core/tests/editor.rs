//! The editor (spec N63, DECISIONS B195) and markers (N64, B196): Edit… on a live Library
//! row, the editor window's five faces, its steps and their undo, Done's save and the
//! permanent copy it asks for, the record's answers and failures, and the markers a take
//! drops — every step through `Run::send`, the laws, the entry oracle and the machine's
//! edges checked at each.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::effect::{EditCmd, Effect, RecCmd, ShellCmd};
use shroom_core::event::{EditStep, EditorNews, Event, Key, LibraryNews, RecNews, Surface};
use shroom_core::scene::{Action, Button, EditListView, EditorBody, EditorMediaView, LibraryRowView, LineTone, PausedBody, PopoverBody, Style};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_edit::{Cut, CutKind, EditList, Op as EditOp, Saved};
use support::*;

// ── builders, with the spec's example values ──

const TITLE: &str = "Setting up shroom in two minutes";
const REC: &str = "Sx4kq9dLmNpQrStU";
const OTHER: &str = "Ab3dEfGhJkLmNpQr";
const AUG21: i64 = 1_787_313_600_000;

const FOOTER: &str = "Your link stays the same. Viewers see the edit once it's saved.";
const HINT: &str = "Drag the yellow ends to trim. Drag across the sound to cut a part.";
const GETTING: &str = "Getting the video from your space…";
const CANT_GET: &str = "Couldn't get the video. Check your connection, then try again.";
const SAVING: &str = "Saving your edit…";
const SAVE_FAILED: &str = "Couldn't save your edit. Check your connection, then press Done again.";

fn rec() -> RecId {
    RecId(REC.into())
}

fn row(id: &str, title: &str, live: bool, owner: &str) -> LibraryRow {
    LibraryRow { rec: RecId(id.into()), title: title.into(), created: Wall(AUG21), length_ms: Some(272_400), live, owner: Some(owner.into()), poster: false, edited_ms: None }
}

/// The Library open on a ready Mac: the video to edit (live), another live one, and one
/// still uploading from another Mac.
fn library() -> World {
    let mut w = ready();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    let rows = vec![row(REC, TITLE, true, "m1"), row(OTHER, "Release notes walkthrough", true, "m1"), row("UpLoAdInGrOwXyZ1", "Still going up", false, "m1")];
    w.library = Fact::known(Arc::new(rows), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w
}

/// A saved edit: the ends at 8 s and 241 s, one cut applied, and one suggestion not.
fn saved_edit(rev: u64) -> Saved {
    let mut list = EditList::new(272.4);
    list.in_ = 8.0;
    list.out = 241.0;
    list.cuts = vec![Cut::manual(150.0, 162.0), Cut { kind: CutKind::LineBeforeCue, applied: false, ..Cut::manual(200.0, 204.0) }];
    Saved::new(list, rev)
}

/// The saved edit with one more part cut by hand, 20–30 s: the list's first hand-cut id.
fn edited() -> EditList {
    let mut l = saved_edit(3).list;
    l.cuts.push(Cut { id: "m1".into(), ..Cut::manual(20.0, 30.0) });
    l
}

trait Drive {
    fn lib(&mut self, action: Action) -> &mut Self;
    fn ed(&mut self, action: Action) -> &mut Self;
    fn news(&mut self, news: EditorNews) -> &mut Self;
    fn step(&mut self, step: EditStep) -> &mut Self;
    fn op(&mut self, op: EditOp) -> &mut Self;
}

impl Drive for Run {
    fn lib(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Library, action, rev: 0 })
    }
    fn ed(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Editor, action, rev: 0 })
    }
    fn news(&mut self, news: EditorNews) -> &mut Self {
        self.send(Event::Editor(news))
    }
    fn step(&mut self, step: EditStep) -> &mut Self {
        self.send(Event::Edit(step))
    }
    fn op(&mut self, op: EditOp) -> &mut Self {
        self.send(Event::Edit(EditStep::Op(op)))
    }
}

/// Edit… on the live row: the editor opens on it, its record asked for.
fn opened() -> Run {
    let mut r = Run::new(library());
    r.lib(Action::EditRow(rec()));
    r
}

/// …its record read (`saved`), its markers, and the video here with its sound read.
fn ready_with(saved: Option<Saved>) -> Run {
    let mut r = opened();
    r.news(EditorNews::Opened { rec: rec(), saved, markers: vec![61.5, 130.0], duration_sec: Some(272.4) });
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    r
}

fn ready_editor() -> Run {
    ready_with(None)
}

/// The saved edit (revision 3) open, and one more part cut: Done has something to save.
fn dirty() -> Run {
    let mut r = ready_with(Some(saved_edit(3)));
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    r
}

fn body(r: &Run) -> EditorBody {
    r.scene.editor.as_ref().expect("the editor window").body.clone()
}

fn face(r: &Run) -> &'static str {
    r.scene.editor.as_ref().map_or("closed", |e| e.id)
}

fn list(r: &Run) -> EditListView {
    body(r).list.expect("the record is read")
}

fn button(b: &Button) -> (&str, Style, bool) {
    (b.label.as_str(), b.style, b.enabled)
}

fn lib_row<'a>(r: &'a Run, title: &str) -> &'a LibraryRowView {
    r.scene.library.as_ref().expect("the Library").body.rows.iter().find(|x| x.title.as_deref() == Some(title)).unwrap_or_else(|| panic!("no row {title:?}"))
}

fn every_button(row: &LibraryRowView) -> impl Iterator<Item = &Button> {
    row.buttons.iter().chain(&row.hover).chain(&row.menu)
}

fn edit_button(row: &LibraryRowView) -> Option<&Button> {
    every_button(row).find(|b| matches!(b.action, Action::EditRow(_)))
}

fn opens(r: &Run) -> bool {
    r.has(|e| matches!(e, Effect::Editor(EditCmd::Open { .. })))
}

// ── the door: Edit… in a live row's menu ──

/// N63: Edit… sits in a live row's menu, after Open page; a row that isn't live has none.
#[test]
fn edit_sits_after_open_page_in_a_live_rows_menu_only() {
    let r = Run::new(library());
    let live = lib_row(&r, TITLE);
    let menu: Vec<&str> = live.menu.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(menu, ["Copy link", "Copy thumbnail", "Open page", "Edit…", "Rename", "Delete…"]);
    let edit = edit_button(live).expect("Edit…");
    assert_eq!((button(edit), &edit.action), (("Edit…", Style::Plain, true), &Action::EditRow(rec())));
    assert!(!live.buttons.iter().chain(&live.hover).any(|b| matches!(b.action, Action::EditRow(_))), "the menu only");
    assert!(edit_button(lib_row(&r, "Still going up")).is_none(), "a video still uploading has nothing to edit yet");

    // This Mac's own takes on their way, recovered, or removed from the space: no Edit….
    let mut w = library();
    w.queue.push(Queued { dir: TakeDir("q40".into()), rec: RecId("UpFrOmThIsMaC001".into()), title: "Untitled — Sep 30".into(), stopped: Wall(AUG21), done: 2, total: 5, blocked: None, length_ms: Some(48_000) });
    w.saved.push(saved_take());
    let r = Run::new(w);
    for row in &r.scene.library.as_ref().unwrap().body.rows {
        let live = row.title.as_deref().is_some_and(|t| t == TITLE || t == "Release notes walkthrough");
        assert_eq!(edit_button(row).is_some(), live, "{:?}", row.title);
    }
}

fn saved_take() -> SavedTake {
    support::saved(SavedCause::Interrupted, 272_000)
}

/// Edit… greys offline and while a take is open (law 4), as Rename does — mid-take with
/// the law's line — and a greyed one opens nothing.
#[test]
fn edit_greys_offline_and_mid_take() {
    let mut w = library();
    w.net.online = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    let edit = edit_button(lib_row(&r, TITLE)).unwrap().clone();
    assert_eq!((edit.enabled, edit.note), (false, None), "offline: greyed, as Rename");
    r.lib(Action::EditRow(rec()));
    assert!(r.scene.editor.is_none() && r.effects.is_empty(), "nothing opens offline");

    // The Library was open before the take: it stays, and Edit… waits for the take.
    let mut w = library();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 30_000));
    let mut r = Run::new(w);
    let edit = edit_button(lib_row(&r, TITLE)).unwrap().clone();
    assert!(!edit.enabled);
    assert_eq!(edit.note.as_deref(), Some("You can do this once your recording is finished."));
    r.lib(Action::EditRow(rec()));
    assert!(r.scene.editor.is_none() && !opens(&r) && r.w.session.fields.editor.is_none(), "law 4: nothing new shows mid-take");
}

/// In this Mac's scope (N5, N33): a member edits its own videos — it sees no one else's —
/// and an owner-class Mac edits any live video in the space, someone else's in Everyone too.
#[test]
fn a_member_sees_edit_only_on_their_own_rows() {
    let mut w = library();
    w.space.identity.as_mut().unwrap().role = Role::Member;
    w.space.identity.as_mut().unwrap().member_id = "m2".into();
    w.space.token = false;
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m2"), row(OTHER, "Bagrat's own", true, "m1")]), Mono(950));
    let r = Run::new(w);
    let rows = &r.scene.library.as_ref().unwrap().body.rows;
    assert_eq!(rows.iter().map(|x| x.title.clone().unwrap_or_default()).collect::<Vec<_>>(), [TITLE], "a member's Library lists its own");
    assert!(edit_button(&rows[0]).is_some_and(|b| b.enabled));

    // The owner, on Everyone, edits a member's video too.
    let mut w = library();
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m1"), row(OTHER, "Mher's onboarding notes", true, "m_mher")]), Mono(950));
    w.session.fields.library.scope = LibraryScope::Everyone;
    let r = Run::new(w);
    assert!(edit_button(lib_row(&r, "Mher's onboarding notes")).is_some_and(|b| b.enabled));
}

// ── opening ──

/// Edit… opens the editor — one window of its own, asked for — and asks the editor lane for
/// the record and the video. Until the record answers, the timeline is empty and Done waits.
#[test]
fn edit_opens_the_editor_on_the_row_and_asks_for_its_record() {
    let r = opened();
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Open { rec: rec() }), Effect::Shell(ShellCmd::OpenEditor)]);
    assert_eq!(r.arm.edge, Some(("library", "editor")));
    let win = r.scene.editor.as_ref().expect("the editor");
    assert_eq!(win.id, "editor");
    assert!(win.faces.is_empty());
    assert!(win.activates, "the person asked for it");
    assert!(r.scene.dock, "law 6: a window of shroom's is open");
    assert!(r.scene.library.is_some(), "the Library stays");
    let b = body(&r);
    assert_eq!(b.title, "Edit — Setting up shroom in two minutes");
    assert_eq!(b.rec, REC);
    assert_eq!(b.media, EditorMediaView::Opening);
    assert_eq!((b.hint.as_str(), b.footer.as_str(), b.footer_tone), (HINT, FOOTER, LineTone::Plain));
    let l = &b.labels;
    assert_eq!(
        [l.play.as_str(), l.pause.as_str(), l.undo.as_str(), l.redo.as_str(), l.cut.as_str(), l.put_back.as_str(), l.was.as_str(), l.time.as_str()],
        ["Play", "Pause", "Undo", "Redo", "Cut {} s", "Put back {} s", "was {}", "{} / {}"]
    );
    assert_eq!(b.list, None, "the timeline waits for the record");
    assert!(b.keep.is_empty());
    assert_eq!((b.edited_duration, b.source_duration), (272.4, 272.4), "the row's length until the record answers");
    assert!(b.markers.is_empty());
    assert!(!b.editable && !b.can_undo && !b.can_redo);
    assert_eq!(button(&b.revert), ("Revert to original", Style::Plain, false));
    assert_eq!(button(&b.done), ("Done", Style::Filled, false));
    assert_eq!(b.done.action, Action::EditorDone);
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (None, None), "Done waits; Esc is the webview's");
    assert_eq!(r.w.inflight.ops.get(&Op::EditorOpen), Some(&Mono(r.t + 1_800_000)), "a download can be long: half an hour");
}

/// The record answers: the saved list (its revision), the markers, the length — then the
/// video comes from the space (editor-getting) and, its sound read, the editor is live.
#[test]
fn the_record_then_the_video_make_the_editor_live() {
    let mut r = opened();
    r.news(EditorNews::Opened { rec: rec(), saved: Some(saved_edit(3)), markers: vec![61.5, 130.0], duration_sec: Some(272.4) });
    assert_eq!(face(&r), "editor");
    let l = list(&r);
    assert_eq!((l.in_, l.out), (8.0, 241.0));
    assert_eq!(
        l.cuts.iter().map(|c| (c.start, c.end, c.applied, c.kind.as_str())).collect::<Vec<_>>(),
        [(150.0, 162.0, true, "manual"), (200.0, 204.0, false, "line-before-cue")]
    );
    let b = body(&r);
    assert_eq!(b.keep, vec![[8.0, 150.0], [162.0, 241.0]]);
    assert_eq!((b.edited_duration, b.source_duration), (221.0, 272.4));
    assert_eq!(b.markers, vec![61.5, 130.0]);
    assert!(!b.editable && !b.done.enabled, "the video isn't here yet");
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().rev, 3);

    r.news(EditorNews::Getting { rec: rec(), pct: 40 });
    assert_eq!(face(&r), "editor-getting");
    let b = body(&r);
    assert_eq!(b.media, EditorMediaView::Getting { pct: 40, line: GETTING.into() });
    assert_eq!(b.footer, FOOTER);
    assert!(!b.done.enabled && !b.editable);
    assert_eq!(r.scene.ret(Surface::Editor), None, "Done waits for the video");

    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    assert_eq!(r.arm.edge, Some(("editor-getting", "editor")));
    assert_eq!(face(&r), "editor");
    let b = body(&r);
    assert_eq!(b.media, EditorMediaView::Ready { duration: 272.4 });
    assert!(b.editable && !b.can_undo && !b.can_redo);
    assert_eq!(button(&b.done), ("Done", Style::Filled, true));
    assert_eq!(button(&b.revert), ("Revert to original", Style::Plain, true), "the saved trim and cut can be put back");
    assert_eq!(r.scene.ret(Surface::Editor), Some(Action::EditorDone));
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorOpen), "the open answered");
}

/// Nothing saved yet: the whole recording, as recorded — nothing to put back.
#[test]
fn a_video_never_edited_opens_whole() {
    let r = ready_editor();
    let l = list(&r);
    assert_eq!((l.in_, l.out, l.cuts.len()), (0.0, 272.4, 0));
    let b = body(&r);
    assert_eq!(b.keep, vec![[0.0, 272.4]]);
    assert_eq!(b.edited_duration, 272.4);
    assert!(!b.revert.enabled, "nothing to put back");
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().rev, 0);
}

/// Bytes moving keep the open alive: a long download isn't cut off at the deadline of the
/// open that started it.
#[test]
fn the_videos_progress_keeps_its_fetch_alive() {
    let mut r = opened();
    r.news(EditorNews::Getting { rec: rec(), pct: 1 });
    let first = r.w.inflight.ops[&Op::EditorOpen];
    r.at(first.0 - 60_000);
    r.news(EditorNews::Getting { rec: rec(), pct: 90 });
    assert!(r.w.inflight.ops[&Op::EditorOpen] > first);
    assert_eq!(face(&r), "editor-getting");
}

// ── editing ──

/// Each step changes the list the editor shows and plays (the preview's `keep`); Undo and
/// Redo walk them, Put back and Revert included. Nothing leaves the window until Done.
#[test]
fn steps_change_the_list_and_undo_and_redo_walk_them() {
    let mut r = ready_editor();
    r.op(EditOp::Trim { in_: 8.0, out: 241.0 });
    assert!(r.effects.is_empty(), "nothing leaves the window");
    assert_eq!(body(&r).keep, vec![[8.0, 241.0]]);
    assert_eq!(body(&r).edited_duration, 233.0);
    assert!(body(&r).can_undo && !body(&r).can_redo);
    r.op(EditOp::Cut { start: 150.0, end: 162.0 });
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [162.0, 241.0]]);
    assert_eq!(body(&r).edited_duration, 221.0);
    assert_eq!(list(&r).cuts.iter().map(|c| (c.start, c.end, c.applied, c.kind.clone())).collect::<Vec<_>>(), [(150.0, 162.0, true, "manual".to_string())]);
    r.op(EditOp::PutBack { index: 0 });
    assert!(list(&r).cuts.is_empty(), "a hand cut put back is gone");
    r.step(EditStep::Undo);
    assert_eq!(list(&r).cuts.len(), 1, "Put back undone");
    assert!(body(&r).can_redo);
    r.step(EditStep::Redo);
    assert!(list(&r).cuts.is_empty());
    r.step(EditStep::Undo).step(EditStep::Undo);
    assert_eq!(body(&r).keep, vec![[8.0, 241.0]], "back to the trim alone");
    // Revert to original, the button: undoable like any step.
    r.ed(Action::EditorRevert);
    assert_eq!(body(&r).keep, vec![[0.0, 272.4]]);
    assert!(!body(&r).revert.enabled, "nothing left to put back");
    r.step(EditStep::Undo);
    assert_eq!(body(&r).keep, vec![[8.0, 241.0]]);
    // A suggestion is applied and put back, never removed.
    let mut r = ready_with(Some(saved_edit(3)));
    r.op(EditOp::Apply { index: 1 });
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [162.0, 200.0], [204.0, 241.0]]);
    r.op(EditOp::PutBack { index: 1 });
    assert_eq!(list(&r).cuts.len(), 2);
    assert!(!list(&r).cuts[1].applied);
}

/// A step is refused — dropped, nothing changes — until the record is read and the video is
/// here, while the edit saves, and when the edit model refuses it (the arm says so).
#[test]
fn steps_are_refused_until_ready_while_saving_and_when_the_model_refuses_them() {
    let mut r = opened();
    r.op(EditOp::Trim { in_: 8.0, out: 241.0 });
    assert_eq!(r.arm.name, "dropped", "the record isn't read");
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4) });
    r.op(EditOp::Trim { in_: 8.0, out: 241.0 });
    assert_eq!(r.arm.name, "dropped", "the video isn't here");
    assert_eq!(body(&r).keep, vec![[0.0, 272.4]]);
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });

    for op in [EditOp::Trim { in_: 30.0, out: 30.5 }, EditOp::Trim { in_: 0.0, out: 272.4 }, EditOp::Cut { start: 5.0, end: 5.05 }, EditOp::PutBack { index: 9 }, EditOp::Apply { index: 0 }, EditOp::Revert] {
        r.op(op.clone());
        assert_eq!(r.arm.name, "edit-refused", "{op:?}");
        assert!(r.effects.is_empty());
    }
    r.step(EditStep::Undo);
    assert_eq!(r.arm.name, "edit-refused", "nothing to undo");
    r.step(EditStep::Redo);
    assert_eq!(r.arm.name, "edit-refused", "nothing to redo");
    assert!(!body(&r).can_undo, "a refused step keeps no step");

    // Saving: the list is what's being saved.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    assert_eq!(face(&r), "editor-saving");
    let before = list(&r);
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    assert_eq!(r.arm.name, "dropped");
    r.step(EditStep::Undo);
    assert_eq!(r.arm.name, "dropped");
    assert_eq!(list(&r), before);
    assert!(!body(&r).editable && !body(&r).can_undo && !body(&r).revert.enabled);

    // No editor at all.
    let mut r = Run::new(library());
    r.op(EditOp::Revert);
    assert_eq!(r.arm.name, "dropped");
}

// ── Done ──

/// Done with nothing changed closes the window at once — Return too — and asks nothing.
#[test]
fn done_with_nothing_changed_closes_at_once() {
    let mut r = ready_with(Some(saved_edit(3)));
    r.ed(Action::EditorDone);
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.session.windows.contains_key(&WindowKind::Editor));
    assert!(r.effects.is_empty(), "no save, no copy: {:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor", "library")));
    assert!(r.scene.library.is_some() && r.scene.dock, "the Library stays, and its Dock icon");

    // Changes undone back to what's saved: nothing to save either — by Return.
    let mut r = ready_editor();
    r.op(EditOp::Cut { start: 10.0, end: 20.0 });
    r.step(EditStep::Undo);
    r.key(Surface::Editor, Key::Return);
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
}

/// Done with changes saves: the checked list as the next revision, no copy yet. The window
/// says it's saving, Done reads Saving… greyed, and nothing more is asked meanwhile.
#[test]
fn done_with_changes_saves_the_checked_list_as_the_next_revision() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Save { rec: rec(), saved: Saved::new(edited().checked(), 4) })]);
    assert_eq!(r.arm.edge, Some(("editor", "editor-saving")));
    assert_eq!(face(&r), "editor-saving");
    let b = body(&r);
    assert_eq!((b.footer.as_str(), b.footer_tone), (SAVING, LineTone::Plain));
    assert_eq!(button(&b.done), ("Saving…", Style::Filled, false));
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (None, None));
    assert_eq!(r.w.inflight.ops.get(&Op::EditorSave), Some(&Mono(r.t + 60_000)));
    // Done, Return and ✕ while it saves: nothing.
    r.key(Surface::Editor, Key::Return);
    assert!(r.effects.is_empty());
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert!(r.effects.is_empty());
    assert_eq!(face(&r), "editor-saving", "the window stays until the save lands");
}

/// The window's ✕ is Done (N63): with changes it saves, and the window stays until saved.
#[test]
fn the_windows_close_is_done() {
    let mut r = dirty();
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert!(r.has(|e| matches!(e, Effect::Editor(EditCmd::Save { saved: Saved { rev: 4, .. }, .. }))), "{:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor", "editor-saving")));
    assert_eq!(face(&r), "editor-saving");
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(r.scene.editor.is_none());
    // Nothing changed: ✕ just closes, and stops nothing.
    let mut r = ready_editor();
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    assert_eq!(r.arm.edge, Some(("editor", "library")));
}

/// Saved: the editor closes, the Library row reads the edited length at once, and the Mac
/// is asked to cut the permanent copy of this revision (B195 (f)).
#[test]
fn saved_closes_the_editor_and_asks_for_the_permanent_copy() {
    let mut r = dirty();
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 4:32");
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(r.arm.edge, Some(("editor-saving", "library")));
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorSave));
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })]);
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:31", "the edited length, at once");
    assert_eq!(r.w.inflight.ops.get(&Op::EditorCopy), Some(&Mono(r.t + 3_600_000)));
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4 });
    assert!(r.effects.is_empty() && !r.w.inflight.ops.contains_key(&Op::EditorCopy));
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:31");
    // Edit… again reads the record afresh.
    r.lib(Action::EditRow(rec()));
    assert!(opens(&r) && face(&r) == "editor");
    assert_eq!(body(&r).list, None);
}

/// A save that leaves the whole recording playing is no edit: no copy is asked (the lane
/// drops the old one with the save), and the row reads the recording's own length.
#[test]
fn a_save_back_to_the_original_asks_for_no_copy() {
    let mut r = ready_with(Some(saved_edit(2)));
    r.ed(Action::EditorRevert);
    r.ed(Action::EditorDone);
    let Some(Effect::Editor(EditCmd::Save { saved: s, .. })) = r.effects.first() else { panic!("{:?}", r.effects) };
    assert!(s.list.is_identity() && s.rev == 3);
    r.news(EditorNews::Saved { rec: rec(), rev: 3 });
    assert!(r.scene.editor.is_none());
    assert!(r.effects.is_empty(), "no copy of the original: {:?}", r.effects);
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 4:32");
}

/// The save didn't go through: the window stays with every change in it, the footer says so
/// in amber, and Done (Return) tries again — the same revision.
#[test]
fn a_failed_save_says_so_and_done_tries_again() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Transport });
    assert_eq!(r.arm.edge, Some(("editor-saving", "editor-save-failed")));
    assert_eq!(face(&r), "editor-save-failed");
    let b = body(&r);
    assert_eq!((b.footer.as_str(), b.footer_tone), (SAVE_FAILED, LineTone::Amber));
    assert_eq!(button(&b.done), ("Done", Style::Filled, true));
    assert!(b.editable, "every change is still here");
    assert_eq!(b.keep, vec![[8.0, 20.0], [30.0, 150.0], [162.0, 241.0]]);
    assert_eq!(r.scene.ret(Surface::Editor), Some(Action::EditorDone));
    r.key(Surface::Editor, Key::Return);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Save { rec: rec(), saved: Saved::new(edited().checked(), 4) })]);
    assert_eq!(r.arm.edge, Some(("editor-save-failed", "editor-saving")));
    // A save that never answers fails at its deadline the same way.
    let deadline = r.w.inflight.ops[&Op::EditorSave];
    r.tick_to(deadline.0 + 10);
    assert_eq!(face(&r), "editor-save-failed");
}

// ── the video that can't come ──

/// The fetch failed: one line and Try again where the timeline goes, Close (Return, Esc, ✕)
/// closes the window and stops what was asked; Try again asks again.
#[test]
fn a_video_that_cant_come_says_so_and_offers_try_again() {
    let mut r = opened();
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4) });
    r.news(EditorNews::Getting { rec: rec(), pct: 10 });
    r.send(Event::Failed { op: Op::EditorOpen, class: FailClass::Transport });
    assert_eq!(r.arm.edge, Some(("editor-getting", "editor-cant-get")));
    assert_eq!(face(&r), "editor-cant-get");
    let b = body(&r);
    let EditorMediaView::Failed { line, retry } = &b.media else { panic!("{:?}", b.media) };
    assert_eq!(line, CANT_GET);
    assert_eq!((button(retry), &retry.action), (("Try again", Style::Plain, true), &Action::EditorRetry));
    assert_eq!((button(&b.done), &b.done.action), (("Close", Style::Filled, true), &Action::EditorClose));
    assert!(!b.editable);
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (Some(Action::EditorClose), Some(Action::EditorClose)));
    assert!(!r.w.inflight.failures.contains_key(&Op::EditorOpen), "Try again is the person's: no retry runs behind it");

    r.ed(Action::EditorRetry);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Open { rec: rec() })]);
    assert_eq!(r.arm.edge, Some(("editor-cant-get", "editor-getting")));
    assert_eq!(body(&r).media, EditorMediaView::Opening);
    assert!(r.w.inflight.ops.contains_key(&Op::EditorOpen));

    // It times out this time: the same face.
    let deadline = r.w.inflight.ops[&Op::EditorOpen];
    r.tick_to(deadline.0 + 10);
    assert_eq!(face(&r), "editor-cant-get");
    r.key(Surface::Editor, Key::Esc);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Cancel { rec: rec() })]);
    assert_eq!(r.arm.edge, Some(("editor-cant-get", "library")));
    assert!(r.scene.editor.is_none());

    // Close, and the window's ✕, the same.
    for close in [Event::Act { surface: Surface::Editor, action: Action::EditorClose, rev: 0 }, Event::WindowClosed(WindowKind::Editor)] {
        let mut r = opened();
        r.send(Event::Failed { op: Op::EditorOpen, class: FailClass::Refused });
        assert_eq!(face(&r), "editor-cant-get", "a record that can't be read says the same");
        r.send(close);
        assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Cancel { rec: rec() })]);
        assert!(r.scene.editor.is_none());
    }
}

/// ✕ while the video is coming (or the record is being read) closes at once and stops it.
#[test]
fn closing_while_the_video_comes_stops_it() {
    let mut r = opened();
    r.news(EditorNews::Getting { rec: rec(), pct: 30 });
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Cancel { rec: rec() })]);
    assert!(r.scene.editor.is_none() && !r.w.inflight.ops.contains_key(&Op::EditorOpen));
    // A late answer for it finds no one waiting.
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    r.send(Event::Failed { op: Op::EditorOpen, class: FailClass::Transport });
    assert!(r.scene.editor.is_none() && r.effects.is_empty());

    let mut r = opened();
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Cancel { rec: rec() })]);
    assert_eq!(r.arm.edge, Some(("editor", "library")));
}

// ── one at a time ──

/// Edit… while the editor is open brings it forward, on the video it holds, every change in it.
#[test]
fn edit_while_open_brings_the_editor_forward_on_its_video() {
    let mut r = ready_editor();
    r.op(EditOp::Cut { start: 10.0, end: 20.0 });
    r.lib(Action::EditRow(RecId(OTHER.into())));
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)]);
    let e = r.w.session.fields.editor.as_ref().unwrap();
    assert_eq!(e.rec, rec());
    assert_eq!(list(&r).cuts.len(), 1, "the change is still there");
    assert!(r.scene.editor.as_ref().unwrap().activates);
}

/// News about another video, or for an editor that's gone, is stale.
#[test]
fn news_for_another_video_is_stale() {
    let mut r = opened();
    let other = RecId(OTHER.into());
    r.news(EditorNews::Opened { rec: other.clone(), saved: Some(saved_edit(9)), markers: vec![], duration_sec: Some(10.0) });
    r.news(EditorNews::Ready { rec: other.clone(), duration: 10.0 });
    r.news(EditorNews::Getting { rec: other.clone(), pct: 50 });
    assert_eq!((face(&r), body(&r).media.clone(), body(&r).list.clone()), ("editor", EditorMediaView::Opening, None));
    r.news(EditorNews::Saved { rec: other, rev: 10 });
    assert!(r.scene.editor.is_some() && r.effects.is_empty());
}

// ── the Library's length ──

/// A row whose record holds an edit reads the edited length in place of the recording's;
/// a refresh begun before a save landed never brings the old length back.
#[test]
fn the_library_row_reads_the_edited_length() {
    let mut w = library();
    w.library = Fact::known(Arc::new(vec![LibraryRow { edited_ms: Some(221_000), ..row(REC, TITLE, true, "m1") }]), Mono(950));
    let r = Run::new(w);
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:41");

    let mut r = Run::new(library());
    r.send(Event::WindowFocused(WindowKind::Library, quick()));
    assert!(r.w.inflight.ops.contains_key(&Op::LibRefresh), "a refresh begins");
    r.lib(Action::EditRow(rec()));
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4) });
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    r.op(EditOp::Trim { in_: 8.0, out: 241.0 });
    r.op(EditOp::Cut { start: 150.0, end: 162.0 });
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 1 });
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:41");
    // The refresh begun before the save lists the record as it was.
    r.send(Event::Library(LibraryNews::Rows { rows: vec![row(REC, TITLE, true, "m1")], fresh: true }));
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:41", "still edited");
    // One begun after it is the space's word.
    r.send(Event::WindowFocused(WindowKind::Library, quick()));
    r.send(Event::Library(LibraryNews::Rows { rows: vec![LibraryRow { edited_ms: Some(221_000), ..row(REC, TITLE, true, "m1") }], fresh: true }));
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:41");
    assert!(r.w.library_ops.edited.is_empty());
}

// ── the permanent copy ──

/// Copies go one at a time, and never over a take — the Mac's encoder is the take's; a
/// failed copy is only noted: the page keeps playing the list over the original.
#[test]
fn copies_wait_for_the_take_and_go_one_at_a_time() {
    // The editor was open before the take; its save lands mid-take.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    let mut w = r.w.clone();
    w.take = Some(OpenTake { opened: w.now, since: w.now, ..take(Phase::Recording, 10_000) });
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert_eq!(face(&r), "editor-saving", "open before the take: it stays");
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(r.scene.editor.is_none());
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "not over the take: {:?}", r.effects);
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 10_000 }));
    r.act(Action::Discard);
    r.act(Action::ConfirmDiscard);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })), "the take is over: {:?}", r.effects);

    // Two saves: the second copy waits for the first.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.lib(Action::EditRow(RecId(OTHER.into())));
    r.news(EditorNews::Opened { rec: RecId(OTHER.into()), saved: None, markers: vec![], duration_sec: Some(95.0) });
    r.news(EditorNews::Ready { rec: RecId(OTHER.into()), duration: 95.0 });
    r.op(EditOp::Cut { start: 10.0, end: 20.0 });
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: RecId(OTHER.into()), rev: 1 });
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "one at a time");
    r.send(Event::Failed { op: Op::EditorCopy, class: FailClass::Unclassified });
    assert!(r.scene.editor.is_none(), "a failed copy says nothing");
    let retry = r.w.inflight.failures[&Op::EditorCopy].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Copy { rec: RecId(OTHER.into()), rev: 1 })), "the next copy: {:?}", r.effects);
    r.news(EditorNews::CopyDone { rec: RecId(OTHER.into()), rev: 1 });
    assert!(r.w.library_ops.copies.is_empty() && !r.w.inflight.failures.contains_key(&Op::EditorCopy));
}

// ── leaving ──

/// Disconnect closes the editor with the rest, and stops a video still coming.
#[test]
fn disconnect_closes_the_editor() {
    let mut r = opened();
    r.news(EditorNews::Getting { rec: rec(), pct: 20 });
    let mut w = r.w.clone();
    w.prefs.settings_tab = Some(SettingsTab::Space);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(r.t)));
    let mut r = Run::new(w);
    r.send(Event::Act { surface: Surface::Settings, action: Action::Disconnect, rev: 0 });
    r.send(Event::Act { surface: Surface::Settings, action: Action::ConfirmDisconnect, rev: 0 });
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Cancel { rec: rec() })), "{:?}", r.effects);
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorOpen));
}

/// The editor keeps its window and its changes when the setup window closes beside it.
#[test]
fn the_setup_window_closing_leaves_the_editor_alone() {
    let mut r = ready_editor();
    r.op(EditOp::Cut { start: 10.0, end: 20.0 });
    let mut w = r.w.clone();
    w.session.windows.insert(WindowKind::Onboarding, Door::asked(Mono(r.t)));
    let mut r = Run::new(w);
    r.send(Event::WindowClosed(WindowKind::Onboarding));
    assert_eq!(list(&r).cuts.len(), 1);
}

#[test]
fn every_editor_op_has_a_deadline() {
    for (op, ms) in [(Op::EditorOpen, 1_800_000), (Op::EditorSave, 60_000), (Op::EditorCopy, 3_600_000)] {
        assert_eq!(op.deadline_ms(), ms, "{op:?}");
    }
}

// ── markers (N64) ──

fn paused_popover(cause: Cause) -> World {
    let mut w = ready();
    w.take = Some(take(Phase::Paused(cause), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

fn paused_body(r: &Run) -> PausedBody {
    match &r.scene.popover.as_ref().expect("the paused popover").body {
        PopoverBody::Paused(p) => p.clone(),
        other => panic!("not paused: {other:?}"),
    }
}

fn labels(r: &Run) -> Vec<String> {
    r.scene.popover.as_ref().unwrap().buttons().iter().map(|b| b.label.clone()).collect()
}

/// Drop a marker sits between Finish & Share and Restart; once used at this pause it reads
/// Marker dropped, greyed, until the take resumes.
#[test]
fn drop_a_marker_sits_between_finish_and_restart_and_says_once_its_dropped() {
    let mut r = Run::new(paused_popover(Cause::User));
    r.send(Event::Tick);
    assert_eq!(labels(&r), ["Resume", "Finish & Share", "Drop a marker", "Restart", "Discard…"]);
    let m = paused_body(&r).marker.expect("the marker row");
    assert_eq!((button(&m), &m.action), (("Drop a marker", Style::Plain, true), &Action::DropMarker));
    r.act(Action::DropMarker);
    assert_eq!(r.effects, vec![Effect::Recorder(RecCmd::Marker)]);
    let t = r.w.take.as_ref().unwrap();
    assert_eq!((t.markers, t.marked_at_pause), (1, true));
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("paused"), "no state change");
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Marker dropped", Style::Plain, false));
    r.act(Action::DropMarker);
    assert!(r.effects.is_empty(), "a second marker at the same point says nothing");
    assert_eq!(r.w.take.as_ref().unwrap().markers, 1);
    // Resume: the next pause offers it again.
    r.act(Action::Resume);
    assert!(!r.w.take.as_ref().unwrap().marked_at_pause);
    r.send(Event::Rec(RecNews::Resumed));
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 300_000 }));
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Drop a marker", Style::Plain, true));
}

/// The auto-pause faces stay as they are: no marker row.
#[test]
fn a_pause_you_didnt_make_offers_no_marker() {
    for cause in [Cause::DisplayLost, Cause::MicLost, Cause::CameraLost, Cause::Asleep, Cause::MicRevoked] {
        let r = Run::new(paused_popover(cause));
        if let Some(p) = r.scene.popover.as_ref() {
            let PopoverBody::Paused(b) = &p.body else { continue };
            assert!(b.marker.is_none(), "{cause:?}");
            assert!(!p.offers(&Action::DropMarker));
        }
    }
}

/// ⌥⌘M (N64): recording, a marker at that moment — nothing on screen answers, the take goes
/// on; paused, Drop a marker while it's offered; anything else, nothing.
#[test]
fn the_marker_shortcut_marks_the_take_and_drops_one_while_paused() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.scene.tray.shortcut, "held with ⌥⌘P while the take is open");
    r.send(Event::MarkerShortcut(quick()));
    assert_eq!(r.effects, vec![Effect::Recorder(RecCmd::Marker)]);
    let t = r.w.take.as_ref().unwrap();
    assert_eq!((t.markers, t.marked_at_pause, t.phase), (1, false, Phase::Recording));
    assert!(r.scene.popover.is_none(), "nothing on screen answers it");
    r.send(Event::MarkerShortcut(quick()));
    assert_eq!(r.w.take.as_ref().unwrap().markers, 2, "every press while recording is a marker");

    // Paused: as Drop a marker — once at this pause.
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 61_000 }));
    r.send(Event::MarkerShortcut(quick()));
    assert_eq!(r.effects, vec![Effect::Recorder(RecCmd::Marker)]);
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Marker dropped", Style::Plain, false));
    r.send(Event::MarkerShortcut(quick()));
    assert!(r.effects.is_empty());
    assert_eq!(r.w.take.as_ref().unwrap().markers, 3);

    // Paused with the popover put away: the same.
    let mut w = paused_popover(Cause::User);
    w.session.popover = None;
    let mut r = Run::new(w);
    r.send(Event::MarkerShortcut(quick()));
    assert!(r.has(|e| *e == Effect::Recorder(RecCmd::Marker)) && r.w.take.as_ref().unwrap().marked_at_pause);
    assert!(r.scene.popover.is_none(), "it opens nothing");

    // A question open waits for its answer; a pause you didn't make has no marker.
    let mut r = Run::new(paused_popover(Cause::User));
    r.act(Action::Discard);
    r.send(Event::MarkerShortcut(quick()));
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Marker)) && r.w.take.as_ref().unwrap().markers == 0);
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("discard-confirm"));
    let mut w = paused_popover(Cause::DisplayLost);
    w.mac.displays = Fact::known(vec![], Mono(0));
    let mut r = Run::new(w);
    r.send(Event::MarkerShortcut(QuickProbe { displays: vec![], ..quick() }));
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Marker)) && r.w.take.as_ref().unwrap().markers == 0);

    // No take: it's other apps' — a stray press does nothing.
    let mut r = Run::new(ready());
    r.send(Event::MarkerShortcut(quick()));
    assert!(r.effects.is_empty() && r.scene.popover.is_none());
    // The count isn't the take yet.
    let mut r = Run::new(ready());
    r.click();
    r.key(Surface::Popover, Key::Return);
    assert!(r.scene.countdown.is_some());
    r.send(Event::MarkerShortcut(quick()));
    assert!(r.effects.is_empty() && r.w.take.is_none());
}

/// Markers the record holds show on the editor's timeline, at their times on the recording.
#[test]
fn markers_show_as_flags_in_the_editor() {
    let r = ready_editor();
    assert_eq!(body(&r).markers, vec![61.5, 130.0]);
}

// ── the webview's JSON ──

fn keys(v: &serde_json::Value) -> Vec<String> {
    let mut k: Vec<String> = v.as_object().expect("an object").keys().cloned().collect();
    k.sort();
    k
}

/// The slice is the webview's contract (app/src/editor/types.ts): the window's keys, the
/// body's, the media's `state` tag, the list's `in`, and the actions' `do`.
#[test]
fn the_slice_is_the_webviews_json() {
    use serde_json::json;
    let mut r = ready_with(Some(saved_edit(3)));
    r.op(EditOp::Trim { in_: 8.0, out: 235.0 });
    let v = serde_json::to_value(r.scene.editor.as_ref().unwrap()).unwrap();
    assert_eq!(keys(&v), ["activates", "body", "esc", "faces", "id", "ret"]);
    let b = &v["body"];
    assert_eq!(
        keys(b),
        [
            "can_redo", "can_undo", "done", "editable", "edited_duration", "footer", "footer_tone", "hint", "keep", "labels", "list", "markers", "media", "rec", "revert",
            "source_duration", "title"
        ]
    );
    assert_eq!(keys(&b["labels"]), ["cut", "pause", "play", "put_back", "redo", "time", "undo", "was"]);
    assert_eq!(b["media"], json!({ "state": "ready", "duration": 272.4 }));
    assert_eq!(b["footer_tone"], "plain");
    assert_eq!((b["list"]["in"].clone(), b["list"]["out"].clone()), (json!(8.0), json!(235.0)));
    assert_eq!(b["list"]["cuts"][1], json!({ "start": 200.0, "end": 204.0, "applied": false, "kind": "line-before-cue" }));
    assert_eq!(b["keep"], json!([[8.0, 150.0], [162.0, 235.0]]));
    assert_eq!(b["edited_duration"], json!(215.0));
    assert_eq!(b["done"]["action"], json!({ "do": "editor-done" }));
    assert_eq!(b["revert"]["action"], json!({ "do": "editor-revert" }));
    assert_eq!(v["ret"], json!({ "do": "editor-done" }));
    assert_eq!(v["esc"], serde_json::Value::Null);

    let mut r = opened();
    r.news(EditorNews::Getting { rec: rec(), pct: 40 });
    let b = serde_json::to_value(&r.scene.editor.as_ref().unwrap().body).unwrap();
    assert_eq!(b["media"], json!({ "state": "getting", "pct": 40, "line": GETTING }));
    assert_eq!(b["list"], serde_json::Value::Null);
    assert_eq!(b["keep"], json!([]));
    r.send(Event::Failed { op: Op::EditorOpen, class: FailClass::Transport });
    let b = serde_json::to_value(&r.scene.editor.as_ref().unwrap().body).unwrap();
    assert_eq!((b["media"]["state"].clone(), b["media"]["retry"]["action"].clone()), (json!("failed"), json!({ "do": "editor-retry" })));
    assert_eq!(b["done"]["action"], json!({ "do": "editor-close" }));

    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Transport });
    assert_eq!(serde_json::to_value(&r.scene.editor.as_ref().unwrap().body).unwrap()["footer_tone"], "amber");

    assert_eq!(serde_json::to_value(Action::EditRow(rec())).unwrap(), json!({ "do": "edit-row", "arg": REC }));
    assert_eq!(serde_json::to_value(Surface::Editor).unwrap(), json!("editor"));
    assert_eq!(serde_json::to_value(WindowKind::Editor).unwrap(), json!("editor"));
    let paused = serde_json::to_value(Run::new(paused_popover(Cause::User)).scene.popover.unwrap().body).unwrap();
    assert_eq!(paused["marker"]["action"], json!({ "do": "drop-marker" }));
}
