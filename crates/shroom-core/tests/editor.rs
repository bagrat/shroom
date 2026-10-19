//! The editor (spec N63, DECISIONS B195, B197) and markers (N64, B196): Edit… on a live
//! Library row, the editor window's faces, its steps and their undo, Done's save, the final
//! video it then makes and the question of the original, the time a kept original has left,
//! the record's answers and failures, a save the space turns down (B199), every Mac's
//! upkeep, and the markers a take drops —
//! every step through `Run::send`, the laws, the entry oracle and the machine's edges
//! checked at each.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::effect::{EditCmd, Effect, LibCmd, PersistOp, RecCmd, ShellCmd, UpdCmd};
use shroom_core::event::{EditStep, EditorNews, Event, Key, LibraryNews, QuitVia, RecNews, Surface};
use shroom_core::scene::{Action, Button, EditListView, EditorAfterView, EditorBody, EditorClosingView, EditorMediaView, LibraryRowView, LineTone, PausedBody, PopoverBody, Style};
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
const SAVE_REFUSED: &str = "This video changed since you opened it, so this edit can't be saved.";
const MAKING_LINE: &str = "The link already plays your edit. You can close this window: shroom finishes it.";
const MAKING_FAILED: &str = "Couldn't make the final video. The link plays your edit anyway, and shroom tries again later.";
const ASK_TITLE: &str = "Delete the original?";
const ASK_LINE: &str = "Your edit is the video now. The original is only kept so you can revert to it.";
const CLOSE_TITLE: &str = "Save your edit?";
const CLOSE_LINE: &str = "What you've changed isn't saved yet.";

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
    // The cleanup suggestions' own read (N68) is theirs to test: off here, so the editor's
    // effects and wakes are the editor's alone.
    w.prefs.suggest_cleanup = false;
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

/// The saved edit with its final video up (B197): the copy of this very revision.
fn with_copy(rev: u64) -> Saved {
    let mut s = saved_edit(rev);
    s.copy = Some(shroom_edit::EditCopy { rev, file: shroom_edit::EditCopy::file_for(rev), duration: 221.0, poster: None, teaser: None });
    s.keep_until = Some("2026-09-23T10:00:00Z".into());
    s
}

const HOUR: i64 = 3_600_000;

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
    r.news(EditorNews::Opened { rec: rec(), saved, markers: vec![61.5, 130.0], duration_sec: Some(272.4), keep_until: None });
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

/// Revert to original, where the window offers it (a save the space turned down has none).
fn revert(b: &EditorBody) -> &Button {
    b.revert.as_ref().expect("Revert to original")
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
    assert_eq!(button(revert(&b)), ("Revert to original", Style::Plain, false));
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
    r.news(EditorNews::Opened { rec: rec(), saved: Some(saved_edit(3)), markers: vec![61.5, 130.0], duration_sec: Some(272.4), keep_until: None });
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
    assert_eq!(button(revert(&b)), ("Revert to original", Style::Plain, true), "the saved trim and cut can be put back");
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
    assert!(!revert(&b).enabled, "nothing to put back");
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
    assert!(!revert(&body(&r)).enabled, "nothing left to put back");
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

/// A cut's edge dragged (B195 (g)): one step that moves it, a hand cut or the voice layer's;
/// Undo puts it back. A suggestion's edges move and it stays offered, so its Cut chip takes
/// it as it now is. The arm says what it was.
#[test]
fn a_cuts_edge_dragged_is_one_step_and_undo_puts_it_back() {
    let mut r = ready_with(Some(saved_edit(3)));
    r.op(EditOp::Resize { index: 0, start: 150.0, end: 165.5 });
    assert_eq!(r.arm.name, "edit-resize");
    assert!(r.effects.is_empty(), "nothing leaves the window");
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [165.5, 241.0]]);
    assert!(body(&r).can_undo && !body(&r).can_redo);
    r.step(EditStep::Undo);
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [162.0, 241.0]], "Undo puts the edge back");
    r.step(EditStep::Redo);
    assert_eq!((list(&r).cuts[0].start, list(&r).cuts[0].end), (150.0, 165.5));
    r.op(EditOp::Resize { index: 1, start: 198.0, end: 204.0 });
    assert_eq!(r.arm.name, "edit-resize", "a suggestion's edge");
    assert!(!list(&r).cuts[1].applied, "still offered");
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [165.5, 241.0]], "nothing plays differently yet");
    r.op(EditOp::Apply { index: 1 });
    assert_eq!(body(&r).keep, vec![[8.0, 150.0], [165.5, 198.0], [204.0, 241.0]], "its Cut chip takes it as it now is");
    r.op(EditOp::Resize { index: 0, start: 150.0, end: 150.05 });
    assert_eq!(r.arm.name, "edit-refused", "shorter than a tenth of a second");
}

/// A step is refused — dropped, nothing changes — until the record is read and the video is
/// here, while the edit saves, and when the edit model refuses it (the arm says so).
#[test]
fn steps_are_refused_until_ready_while_saving_and_when_the_model_refuses_them() {
    let mut r = opened();
    r.op(EditOp::Trim { in_: 8.0, out: 241.0 });
    assert_eq!(r.arm.name, "dropped", "the record isn't read");
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4), keep_until: None });
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
    assert!(!body(&r).editable && !body(&r).can_undo && !revert(&body(&r)).enabled);

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

/// The window's ✕ with nothing to save closes at once, and stops nothing (N63, N71).
#[test]
fn the_windows_close_with_nothing_to_save_closes_at_once() {
    let mut r = ready_editor();
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    assert_eq!(r.arm.edge, Some(("editor", "library")));
    // Changes undone back to what's saved: nothing to save either.
    let mut r = ready_with(Some(saved_edit(3)));
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    r.step(EditStep::Undo);
    r.send(Event::WindowClosed(WindowKind::Editor));
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
}

// ── ✕ with changes not saved asks first (N71) ──

fn closing(r: &Run) -> EditorClosingView {
    body(r).closing.expect("the close prompt")
}

fn close(r: &mut Run) {
    r.send(Event::WindowClosed(WindowKind::Editor));
}

/// ✕ with changes not saved asks before the window closes (N71): in the footer's place,
/// “Save your edit?” and its line, then Discard (red, a click only), Keep editing and Save
/// (filled). Nothing is saved yet, and the edit is held behind the question: no step is
/// taken and the preview stops (the webview pauses what isn't editable). Return is Save, Esc
/// is Keep editing, and ✕ again leaves the question as it is.
#[test]
fn the_windows_close_with_changes_asks_first() {
    let mut r = dirty();
    close(&mut r);
    assert!(r.effects.is_empty(), "nothing saved yet: {:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor", "editor-closing")));
    assert_eq!(face(&r), "editor-closing");
    assert!(r.w.session.windows.contains_key(&WindowKind::Editor), "the window stays while it asks");
    let c = closing(&r);
    assert_eq!((c.title.as_str(), c.line.as_str()), (CLOSE_TITLE, CLOSE_LINE));
    assert_eq!(button(&c.discard), ("Discard", Style::RedOutline, true));
    assert_eq!(button(&c.keep), ("Keep editing", Style::Plain, true));
    assert_eq!(button(&c.save), ("Save", Style::Filled, true));
    assert_eq!((&c.discard.action, &c.keep.action, &c.save.action), (&Action::EditorDiscard, &Action::EditorKeepEditing, &Action::EditorDone));
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (Some(Action::EditorDone), Some(Action::EditorKeepEditing)));
    let b = body(&r);
    assert!(!b.editable && !b.can_undo && !b.can_redo, "the edit is held while it asks");
    assert!(!revert(&b).enabled);
    assert_eq!(b.footer, FOOTER, "the footer's own line stays beneath the card");

    // A step meanwhile — a cut, Undo, Revert — is dropped.
    let held = list(&r);
    r.op(EditOp::Cut { start: 40.0, end: 50.0 });
    r.step(EditStep::Undo);
    r.ed(Action::EditorRevert);
    assert_eq!(list(&r), held);
    assert_eq!(face(&r), "editor-closing");

    // ✕ again: the question stays, nothing saved.
    close(&mut r);
    assert!(r.effects.is_empty());
    assert_eq!(face(&r), "editor-closing");
}

/// Keep editing — Esc too — puts the question away: every change still there, steps taken
/// again, and ✕ asks again next time.
#[test]
fn keep_editing_puts_the_question_away() {
    for keep in [Event::Act { surface: Surface::Editor, action: Action::EditorKeepEditing, rev: 0 }, Event::Key { surface: Surface::Editor, key: Key::Esc }] {
        let mut r = dirty();
        let before = list(&r);
        close(&mut r);
        r.send(keep.clone());
        assert!(r.effects.is_empty(), "{keep:?}: {:?}", r.effects);
        assert_eq!(r.arm.edge, Some(("editor-closing", "editor")), "{keep:?}");
        assert_eq!(face(&r), "editor");
        let b = body(&r);
        assert!(b.closing.is_none() && b.editable && b.can_undo);
        assert_eq!(list(&r), before, "{keep:?}: every change still there");
        r.op(EditOp::Cut { start: 40.0, end: 50.0 });
        assert_eq!(list(&r).cuts.len(), before.cuts.len() + 1, "steps are taken again");
        close(&mut r);
        assert_eq!(face(&r), "editor-closing", "✕ asks again");
    }
}

/// Save is Done — Return too: the checked list saved as the next revision, then the final
/// video, as after Done.
#[test]
fn save_on_the_question_is_done() {
    for save in [Event::Act { surface: Surface::Editor, action: Action::EditorDone, rev: 0 }, Event::Key { surface: Surface::Editor, key: Key::Return }] {
        let mut r = dirty();
        close(&mut r);
        r.send(save.clone());
        assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Save { rec: rec(), saved: Saved::new(edited().checked(), 4) })], "{save:?}");
        assert_eq!(r.arm.edge, Some(("editor-closing", "editor-saving")), "{save:?}");
        assert_eq!(face(&r), "editor-saving");
        assert!(body(&r).closing.is_none());
        r.news(EditorNews::Saved { rec: rec(), rev: 4 });
        assert_eq!(face(&r), "editor-making", "{save:?}: the final video, as after Done (B197)");
    }
}

/// Discard closes the window and drops what wasn't saved: nothing written and nothing to
/// stop (the video is here), the video as it was last saved. It is the question's alone,
/// and a click: Return never discards.
#[test]
fn discard_closes_and_drops_what_wasnt_saved() {
    let mut r = dirty();
    close(&mut r);
    r.ed(Action::EditorDiscard);
    assert!(r.effects.is_empty(), "nothing saved: {:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor-closing", "library")));
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.session.windows.contains_key(&WindowKind::Editor));
    assert!(r.scene.library.is_some(), "the Library stays");

    // Not asked: Discard does nothing, and the change stays.
    let mut r = dirty();
    r.ed(Action::EditorDiscard);
    assert!(r.effects.is_empty());
    assert_eq!(face(&r), "editor");
    assert_eq!(list(&r).cuts.len(), 3);
}

/// A save that didn't go through still holds changes not saved: ✕ asks there too, Keep
/// editing goes back to that face and its line, and Save tries the save again.
#[test]
fn the_windows_close_after_a_failed_save_asks_too() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Transport });
    assert_eq!(face(&r), "editor-save-failed");
    close(&mut r);
    assert!(r.effects.is_empty(), "{:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor-save-failed", "editor-closing")));
    assert_eq!(face(&r), "editor-closing");
    r.key(Surface::Editor, Key::Esc);
    assert_eq!(r.arm.edge, Some(("editor-closing", "editor-save-failed")));
    assert_eq!(face(&r), "editor-save-failed");
    assert_eq!((body(&r).footer.as_str(), body(&r).footer_tone), (SAVE_FAILED, LineTone::Amber));
    close(&mut r);
    r.key(Surface::Editor, Key::Return);
    assert!(r.has(|e| matches!(e, Effect::Editor(EditCmd::Save { saved: Saved { rev: 4, .. }, .. }))), "{:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor-closing", "editor-saving")));
}

/// Quit while it asks saves first, without asking (Bagrat, 2026-10-19; B199): the question
/// goes, the edit saves, and shroom quits once it's saved.
#[test]
fn quit_while_it_asks_saves_first() {
    let mut r = dirty();
    close(&mut r);
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert!(saves(&r) && !quits(&r), "{:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor-closing", "editor-saving")));
    assert_eq!(face(&r), "editor-saving");
    assert!(body(&r).closing.is_none());
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().leaving, Some(Leaving::Quit));
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(quits(&r), "{:?}", r.effects);
}

/// Deleting the video while it asks closes the window, as from any face: nothing saved.
#[test]
fn deleting_its_video_while_it_asks_closes_the_editor() {
    let mut r = dirty();
    close(&mut r);
    r.send(Event::Library(LibraryNews::Deleted(vec![rec()])));
    assert!(!saves(&r), "nothing is saved: {:?}", r.effects);
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
}

/// Saved with something left out (B197 (a)): the window stays and makes the final video —
/// the permanent copy of this revision (B195 (f)), asked at once — and the Library row reads
/// the edited length at once. Where the timeline was, “Making the final video… 0%”; the
/// footer says the window can be closed, and Done is Close.
#[test]
fn saved_with_something_left_out_makes_the_final_video() {
    let mut r = dirty();
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 4:32");
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(r.arm.edge, Some(("editor-saving", "editor-making")));
    assert_eq!(face(&r), "editor-making");
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorSave));
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })]);
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:31", "the edited length, at once");
    assert_eq!(r.w.inflight.ops.get(&Op::EditorCopy), Some(&Mono(r.t + 3_600_000)));
    let b = body(&r);
    assert_eq!(b.after, Some(EditorAfterView::Making { pct: 0, line: "Making the final video… 0%".into() }));
    assert_eq!((b.footer.as_str(), b.footer_tone), (MAKING_LINE, LineTone::Plain));
    assert_eq!((button(&b.done), &b.done.action), (("Close", Style::Filled, true), &Action::EditorClose));
    assert!(!b.editable && !b.can_undo && !revert(&b).enabled, "the edit is saved: nothing more to step");
    assert_eq!(b.original_kept, None);
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (Some(Action::EditorClose), Some(Action::EditorClose)));
    assert_eq!(b.keep, vec![[8.0, 20.0], [30.0, 150.0], [162.0, 241.0]], "what plays is what was saved");
    // Steps are refused now.
    r.op(EditOp::Cut { start: 40.0, end: 50.0 });
    assert_eq!(r.arm.name, "dropped");
}

/// The whole after-save flow (B197 (a)–(c)): the bar fills with the lane's news, the final
/// video lands and the window asks “Delete the original?” — Yes, delete, a click and never
/// Return (law 9; DEVIATIONS #55), makes the edit the video and closes.
#[test]
fn the_final_video_lands_and_yes_delete_bakes_it() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.news(EditorNews::Making { rec: rec(), pct: 40 });
    assert_eq!(face(&r), "editor-making");
    assert_eq!(body(&r).after, Some(EditorAfterView::Making { pct: 40, line: "Making the final video… 40%".into() }));
    r.news(EditorNews::Making { rec: rec(), pct: 250 });
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().save, EditorSave::Making { pct: 100 }, "a share is at most all of it");
    assert!(r.effects.is_empty());

    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    assert_eq!(r.arm.edge, Some(("editor-making", "editor-keep-original")));
    assert_eq!(face(&r), "editor-keep-original");
    assert!(r.w.library_ops.copies.is_empty() && !r.w.inflight.ops.contains_key(&Op::EditorCopy));
    let b = body(&r);
    let Some(EditorAfterView::Asking { title, line, keep, delete }) = &b.after else { panic!("{:?}", b.after) };
    assert_eq!((title.as_str(), line.as_str()), (ASK_TITLE, ASK_LINE));
    assert_eq!((button(keep), &keep.action), (("Keep for 2 days", Style::Plain, true), &Action::KeepOriginal));
    assert_eq!((button(delete), &delete.action), (("Yes, delete", Style::Filled, true), &Action::DeleteOriginal));
    assert!(!b.done.enabled && !b.editable, "the question's own buttons answer it");
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (None, Some(Action::KeepOriginal)), "deleting is a click, never Return (law 9)");
    r.key(Surface::Editor, Key::Return);
    assert!(r.effects.is_empty() && face(&r) == "editor-keep-original", "Return asks nothing");

    r.ed(Action::DeleteOriginal);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Bake { rec: rec() })]);
    assert_eq!(r.arm.edge, Some(("editor-keep-original", "library")));
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert_eq!(lib_row(&r, TITLE).meta, "Aug 21 · 3:31");
    // Edit… again reads the record afresh.
    r.lib(Action::EditRow(rec()));
    assert!(opens(&r) && face(&r) == "editor");
    assert_eq!(body(&r).list, None);
}

/// Keep for 2 days, Esc and ✕ on the question close the window and ask nothing: the
/// copy's landing already set when the original goes (B197 (b)).
#[test]
fn keep_esc_and_close_on_the_question_keep_the_original() {
    let asking = || {
        let mut r = dirty();
        r.ed(Action::EditorDone);
        r.news(EditorNews::Saved { rec: rec(), rev: 4 });
        r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
        assert_eq!(face(&r), "editor-keep-original");
        r
    };
    for answer in [
        Event::Act { surface: Surface::Editor, action: Action::KeepOriginal, rev: 0 },
        Event::Key { surface: Surface::Editor, key: Key::Esc },
        Event::WindowClosed(WindowKind::Editor),
    ] {
        let mut r = asking();
        r.send(answer.clone());
        assert!(r.effects.is_empty(), "{answer:?}: {:?}", r.effects);
        assert_eq!(r.arm.edge, Some(("editor-keep-original", "library")), "{answer:?}");
        assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none(), "{answer:?}");
    }
    // Neither answer is offered anywhere else.
    let mut r = dirty();
    r.ed(Action::DeleteOriginal);
    assert_eq!(r.arm.name, "dropped");
    r.ed(Action::KeepOriginal);
    assert_eq!(r.arm.name, "dropped");
    assert!(r.effects.is_empty() && face(&r) == "editor");
}

/// Close, Return, Esc or ✕ while the final video is being made closes the window and stops
/// nothing: the copy is still made (and the question is then answered Keep for 2 days).
#[test]
fn closing_while_the_final_video_is_made_leaves_it_to_finish() {
    for close in [
        Event::Act { surface: Surface::Editor, action: Action::EditorClose, rev: 0 },
        Event::Key { surface: Surface::Editor, key: Key::Return },
        Event::Key { surface: Surface::Editor, key: Key::Esc },
        Event::WindowClosed(WindowKind::Editor),
    ] {
        let mut r = dirty();
        r.ed(Action::EditorDone);
        r.news(EditorNews::Saved { rec: rec(), rev: 4 });
        r.news(EditorNews::Making { rec: rec(), pct: 30 });
        r.send(close.clone());
        assert!(r.effects.is_empty(), "{close:?}: nothing stops — {:?}", r.effects);
        assert_eq!(r.arm.edge, Some(("editor-making", "library")), "{close:?}");
        assert!(r.scene.editor.is_none(), "{close:?}");
        assert!(r.w.inflight.ops.contains_key(&Op::EditorCopy), "{close:?}: the copy goes on");
        assert_eq!(r.w.library_ops.copies.get(&rec()), Some(&CopyOwed { rev: 4, asked: true }));
        // Its progress, and its landing, find no window: nothing is asked.
        r.news(EditorNews::Making { rec: rec(), pct: 80 });
        assert!(r.scene.editor.is_none() && r.effects.is_empty());
        r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
        assert!(r.scene.editor.is_none() && r.effects.is_empty());
        assert!(r.w.library_ops.copies.is_empty() && !r.w.inflight.ops.contains_key(&Op::EditorCopy));
    }

    // Closed before the copy could even start (mid-take): it's still owed, and asked once
    // the take is over.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    let mut w = r.w.clone();
    w.take = Some(OpenTake { opened: w.now, since: w.now, ..take(Phase::Recording, 10_000) });
    let mut r = Run::new(w);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(body(&r).after, Some(EditorAfterView::Making { pct: 0, line: "Making the final video… 0%".into() }), "it waits for the take");
    r.ed(Action::EditorClose);
    assert!(r.scene.editor.is_none());
    assert_eq!(r.w.library_ops.copies.get(&rec()), Some(&CopyOwed { rev: 4, asked: false }), "still owed");
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 10_000 }));
    r.act(Action::Discard);
    r.act(Action::ConfirmDiscard);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })), "the take is over: {:?}", r.effects);
}

/// The final video couldn't be made or put up — the lane said so, or it ran out of time:
/// one amber line and Close (Return, Esc, ✕), and the link plays the edit over the original
/// meanwhile; nothing is asked again behind the window (an upkeep makes it later).
#[test]
fn a_final_video_that_cant_be_made_says_so() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.news(EditorNews::Making { rec: rec(), pct: 55 });
    r.send(Event::Failed { op: Op::EditorCopy, class: FailClass::Transport });
    assert_eq!(r.arm.edge, Some(("editor-making", "editor-making-failed")));
    assert_eq!(face(&r), "editor-making-failed");
    let b = body(&r);
    assert_eq!(b.after, Some(EditorAfterView::Failed));
    assert_eq!((b.footer.as_str(), b.footer_tone), (MAKING_FAILED, LineTone::Amber));
    assert_eq!((button(&b.done), &b.done.action), (("Close", Style::Filled, true), &Action::EditorClose));
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (Some(Action::EditorClose), Some(Action::EditorClose)));
    assert!(r.w.library_ops.copies.is_empty(), "no longer owed: the upkeep makes it");
    for close in [Event::Act { surface: Surface::Editor, action: Action::EditorClose, rev: 0 }, Event::Key { surface: Surface::Editor, key: Key::Esc }, Event::WindowClosed(WindowKind::Editor)] {
        let mut r2 = Run::new(r.w.clone());
        r2.send(close.clone());
        assert!(r2.effects.is_empty() && r2.scene.editor.is_none(), "{close:?}");
        assert_eq!(r2.arm.edge, Some(("editor-making-failed", "library")), "{close:?}");
    }
    r.key(Surface::Editor, Key::Return);
    assert!(r.scene.editor.is_none());

    // A copy that never answers fails at its deadline the same way.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    let deadline = r.w.inflight.ops[&Op::EditorCopy];
    r.tick_to(deadline.0 + 10);
    assert_eq!(face(&r), "editor-making-failed");

    // Progress keeps a long one alive.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    let first = r.w.inflight.ops[&Op::EditorCopy];
    r.at(first.0 - 60_000);
    r.news(EditorNews::Making { rec: rec(), pct: 90 });
    assert!(r.w.inflight.ops[&Op::EditorCopy] > first);
    assert_eq!(face(&r), "editor-making");
}

/// A copy the lane dropped, never named — the record moved on first, a newer save landing —
/// is no final video: the window says it couldn't be made, as for one that failed, and
/// nothing asks about the original (B199).
#[test]
fn a_dropped_copy_asks_nothing_about_the_original() {
    // The path that gets there: a save that ran out of time lands late while its retry is
    // on its way; the window takes it and makes revision 4's copy; the retry lands as 5,
    // and the lane drops 4's copy unnamed.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    let deadline = r.w.inflight.ops[&Op::EditorSave];
    r.tick_to(deadline.0 + 10);
    assert_eq!(face(&r), "editor-save-failed");
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(face(&r), "editor-making");
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })]);
    r.news(EditorNews::Saved { rec: rec(), rev: 5 });
    assert_eq!(r.arm.name, "editor-stale", "the window took the first answer");
    r.news(EditorNews::Making { rec: rec(), pct: 80 });
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: false });
    assert_eq!(r.arm.edge, Some(("editor-making", "editor-making-failed")));
    assert_eq!(face(&r), "editor-making-failed", "no “Delete the original?” over a final video that isn't there");
    let b = body(&r);
    assert_eq!(b.after, Some(EditorAfterView::Failed));
    assert_eq!((b.footer.as_str(), b.footer_tone), (MAKING_FAILED, LineTone::Amber));
    assert!(r.effects.is_empty(), "nothing is made the video: {:?}", r.effects);
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorCopy) && r.w.library_ops.copies.is_empty(), "the copy answered");
    assert!(!r.scene.offers(Surface::Editor, &Action::DeleteOriginal) && !r.scene.offers(Surface::Editor, &Action::KeepOriginal));
    r.ed(Action::EditorClose);
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    assert_eq!(r.arm.edge, Some(("editor-making-failed", "library")));

    // With the window gone, a dropped copy answers and moves nothing.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.ed(Action::EditorClose);
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: false });
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorCopy) && r.w.library_ops.copies.is_empty());
}

/// News of a copy for another video, of an older revision, or for a window already gone,
/// asks nothing and moves no face.
#[test]
fn a_copy_for_another_video_or_revision_moves_nothing() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    let other = RecId(OTHER.into());
    r.news(EditorNews::Making { rec: other.clone(), pct: 70 });
    assert_eq!(body(&r).after, Some(EditorAfterView::Making { pct: 0, line: "Making the final video… 0%".into() }));
    r.news(EditorNews::CopyDone { rec: other, rev: 4, named: true });
    assert_eq!(face(&r), "editor-making");
    assert!(r.w.inflight.ops.contains_key(&Op::EditorCopy), "this video's copy is still being cut");
    assert_eq!(r.w.library_ops.copies.get(&rec()), Some(&CopyOwed { rev: 4, asked: true }));

    // Closed while revision 4's copy is cut, opened again and saved as 5: the copy of 4
    // landing isn't this save's — 5's is asked next, and the window waits for it.
    r.ed(Action::EditorClose);
    r.lib(Action::EditRow(rec()));
    r.news(EditorNews::Opened { rec: rec(), saved: Some(Saved::new(edited(), 4)), markers: vec![], duration_sec: Some(272.4), keep_until: None });
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    r.op(EditOp::Cut { start: 40.0, end: 50.0 });
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 5 });
    assert_eq!(face(&r), "editor-making");
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "one at a time: 4's is still being cut");
    r.news(EditorNews::Making { rec: rec(), pct: 60 });
    assert_eq!(body(&r).after, Some(EditorAfterView::Making { pct: 0, line: "Making the final video… 0%".into() }), "4's share isn't 5's");
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    assert_eq!(face(&r), "editor-making", "an older revision's copy isn't this save's");
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Copy { rec: rec(), rev: 5 })]);
    r.news(EditorNews::CopyDone { rec: rec(), rev: 5, named: true });
    assert_eq!(face(&r), "editor-keep-original");

    // A copy that didn't come out for another video says nothing here.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    let mut w = r.w.clone();
    w.library_ops.copies.insert(RecId(OTHER.into()), CopyOwed { rev: 1, asked: true });
    w.library_ops.copies.insert(rec(), CopyOwed { rev: 4, asked: false });
    let mut r = Run::new(w);
    r.send(Event::Failed { op: Op::EditorCopy, class: FailClass::Unclassified });
    assert_eq!(face(&r), "editor-making");
    assert_eq!(r.w.library_ops.copies.get(&rec()), Some(&CopyOwed { rev: 4, asked: false }));

    // The window gone, a late landing opens nothing.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.ed(Action::EditorClose);
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    // …nor once another video's editor is open.
    r.lib(Action::EditRow(RecId(OTHER.into())));
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    assert_eq!(face(&r), "editor");
}

/// A save that leaves the whole recording playing is no edit: the window closes at once as
/// it did before B197 — nothing to make — no copy is asked (the lane drops the old one with
/// the save), and the row reads the recording's own length.
#[test]
fn a_save_back_to_the_original_asks_for_no_copy() {
    let mut r = ready_with(Some(saved_edit(2)));
    r.ed(Action::EditorRevert);
    r.ed(Action::EditorDone);
    let Some(Effect::Editor(EditCmd::Save { saved: s, .. })) = r.effects.first() else { panic!("{:?}", r.effects) };
    assert!(s.list.is_identity() && s.rev == 3);
    r.news(EditorNews::Saved { rec: rec(), rev: 3 });
    assert_eq!(r.arm.edge, Some(("editor-saving", "library")));
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
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

// ── a save the space turns down (B199) ──

/// The space turned the save down — the video is gone (another Mac deleted it), or another
/// Mac made an edit the video while this window was open: nothing can be saved over it.
/// editor-save-refused says so in amber in the save-failed line's place; Revert to original
/// is gone, nothing steps, Close stands in Done's place, and Return and Esc are Close.
#[test]
fn a_save_the_space_turns_down_says_so_and_only_closes() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    assert_eq!(r.arm.edge, Some(("editor-saving", "editor-save-refused")));
    assert_eq!(face(&r), "editor-save-refused");
    let b = body(&r);
    assert_eq!((b.footer.as_str(), b.footer_tone), (SAVE_REFUSED, LineTone::Amber));
    assert_eq!((button(&b.done), &b.done.action), (("Close", Style::Filled, true), &Action::EditorClose));
    assert_eq!(b.revert, None, "Revert to original is gone");
    assert!(!b.editable && !b.can_undo && !b.can_redo, "nothing steps");
    assert_eq!((b.after.clone(), b.original_kept.clone()), (None, None));
    assert_eq!(b.keep, vec![[8.0, 20.0], [30.0, 150.0], [162.0, 241.0]], "the edit still plays as it stood");
    assert_eq!((r.scene.ret(Surface::Editor), r.scene.esc(Surface::Editor)), (Some(Action::EditorClose), Some(Action::EditorClose)));
    assert!(!r.w.inflight.failures.contains_key(&Op::EditorSave), "nothing tries again behind it");
    r.op(EditOp::Cut { start: 40.0, end: 50.0 });
    assert_eq!(r.arm.name, "dropped");
    r.ed(Action::EditorDone);
    assert_eq!(r.arm.name, "dropped", "Done isn't offered");
    assert!(r.effects.is_empty());

    // An earlier edit's original still counted down when it opened: nothing is said of it.
    let mut r = kept_for(with_copy(3), 20 * HOUR);
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    assert_eq!((face(&r), body(&r).original_kept), ("editor-save-refused", None));
}

/// Close, Return, Esc and ✕ close it, and nothing is saved.
#[test]
fn a_refused_save_closes_without_saving() {
    for close in [
        Event::Act { surface: Surface::Editor, action: Action::EditorClose, rev: 0 },
        Event::Key { surface: Surface::Editor, key: Key::Return },
        Event::Key { surface: Surface::Editor, key: Key::Esc },
        Event::WindowClosed(WindowKind::Editor),
    ] {
        let mut r = dirty();
        r.ed(Action::EditorDone);
        r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
        r.send(close.clone());
        assert!(r.effects.is_empty(), "{close:?}: nothing is saved — {:?}", r.effects);
        assert_eq!(r.arm.edge, Some(("editor-save-refused", "library")), "{close:?}");
        assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none(), "{close:?}");
        assert!(r.scene.library.is_some(), "{close:?}: the Library stays");
    }
}

/// Turned down on Done's retry after a save that couldn't get through: the same face. Any
/// failure but a refusal stays editor-save-failed's, and Done tries again.
#[test]
fn only_a_refusal_is_final() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Transport });
    assert_eq!(face(&r), "editor-save-failed");
    r.ed(Action::EditorDone);
    assert_eq!(face(&r), "editor-saving");
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    assert_eq!(r.arm.edge, Some(("editor-saving", "editor-save-refused")));
    for class in [FailClass::Transport, FailClass::Timeout, FailClass::Panicked, FailClass::Unclassified] {
        let mut r = dirty();
        r.ed(Action::EditorDone);
        r.send(Event::Failed { op: Op::EditorSave, class });
        assert_eq!(face(&r), "editor-save-failed", "{class:?}");
    }
}

// ── its video deleted (B199) ──

/// Delete… confirmed in the Library for the video the editor holds closes the editor: what
/// its open asked for stops, and nothing is saved — not even the change it holds. Another
/// video's delete leaves it alone.
#[test]
fn deleting_its_video_closes_the_editor() {
    let mut r = dirty();
    r.lib(Action::DeleteRow(rec()));
    assert_eq!(face(&r), "editor", "asking changes nothing");
    r.lib(Action::ConfirmDelete);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec()]))), "{:?}", r.effects);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Cancel { rec: rec() })), "its open stops: {:?}", r.effects);
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Save { .. }))), "nothing is saved");
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.session.windows.contains_key(&WindowKind::Editor));
    r.send(Event::Library(LibraryNews::Deleted(vec![rec()])));
    assert!(r.scene.editor.is_none() && r.effects.is_empty(), "{:?}", r.effects);

    let mut r = dirty();
    r.lib(Action::DeleteRow(RecId(OTHER.into())));
    r.lib(Action::ConfirmDelete);
    assert_eq!(face(&r), "editor");
    assert_eq!(list(&r).cuts.len(), 3, "the change is still there");
}

/// The delete landing closes it as well — here with a save on its way, whose late answer
/// then finds no one waiting.
#[test]
fn its_delete_landing_closes_the_editor_and_a_save_on_its_way_answers_no_one() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    assert_eq!(face(&r), "editor-saving");
    r.send(Event::Library(LibraryNews::Deleted(vec![rec()])));
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Cancel { rec: rec() })), "{:?}", r.effects);
    assert!(r.scene.editor.is_none() && r.w.session.fields.editor.is_none());
    assert!(!r.w.inflight.ops.contains_key(&Op::EditorSave), "the save on its way answers no one");
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    assert!(r.scene.editor.is_none() && r.effects.is_empty());
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(r.arm.name, "editor-stale");
}

// ── the video that can't come ──

/// The fetch failed: one line and Try again where the timeline goes, Close (Return, Esc, ✕)
/// closes the window and stops what was asked; Try again asks again.
#[test]
fn a_video_that_cant_come_says_so_and_offers_try_again() {
    let mut r = opened();
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4), keep_until: None });
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
    r.news(EditorNews::Opened { rec: other.clone(), saved: Some(saved_edit(9)), markers: vec![], duration_sec: Some(10.0), keep_until: None });
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
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4), keep_until: None });
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
    assert_eq!(face(&r), "editor-making", "it waits, at 0%, for the take");
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "not over the take: {:?}", r.effects);
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 10_000 }));
    r.act(Action::Discard);
    r.act(Action::ConfirmDiscard);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Copy { rec: rec(), rev: 4 })), "the take is over: {:?}", r.effects);
    assert_eq!(face(&r), "editor-making");

    // Two saves: the second copy waits for the first.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.ed(Action::EditorClose);
    r.lib(Action::EditRow(RecId(OTHER.into())));
    r.news(EditorNews::Opened { rec: RecId(OTHER.into()), saved: None, markers: vec![], duration_sec: Some(95.0), keep_until: None });
    r.news(EditorNews::Ready { rec: RecId(OTHER.into()), duration: 95.0 });
    r.op(EditOp::Cut { start: 10.0, end: 20.0 });
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: RecId(OTHER.into()), rev: 1 });
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "one at a time");
    assert_eq!(face(&r), "editor-making");
    r.send(Event::Failed { op: Op::EditorCopy, class: FailClass::Unclassified });
    assert_eq!(face(&r), "editor-making", "the first video's failed copy says nothing here");
    let retry = r.w.inflight.failures[&Op::EditorCopy].retry_at();
    r.tick_to(retry.0);
    assert!(r.has(|e| *e == Effect::Editor(EditCmd::Copy { rec: RecId(OTHER.into()), rev: 1 })), "the next copy: {:?}", r.effects);
    r.news(EditorNews::CopyDone { rec: RecId(OTHER.into()), rev: 1, named: true });
    assert!(r.w.library_ops.copies.is_empty() && !r.w.inflight.failures.contains_key(&Op::EditorCopy));
    assert_eq!(face(&r), "editor-keep-original");
}

// ── the time the original has left (B197 (e)) ──

/// The editor opened on a record whose original is kept `left` ms more beside its copy.
fn kept_for(saved: Saved, left: i64) -> Run {
    let mut r = opened();
    // Two steps on (each step is 10 ms): `left` is what's left once the video is here.
    let until = Wall(r.w.wall.0 + 20 + left);
    r.news(EditorNews::Opened { rec: rec(), saved: Some(saved), markers: vec![], duration_sec: Some(272.4), keep_until: Some(until) });
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    r
}

/// By Revert, while an earlier edit's original is kept: never more time than it has left
/// (B199) — “two more days” from 47 hours, “one more day” from a day, then whole hours
/// rounded down from 2 hours, “one more hour” from an hour, and “less than an hour” under
/// one; a count to nine in words, from 10 in figures (law 16).
#[test]
fn the_time_the_original_has_left_sits_by_revert() {
    for (left, words) in [
        (48 * HOUR, "The original is kept two more days"),
        (47 * HOUR, "The original is kept two more days"),
        (47 * HOUR - 1, "The original is kept one more day"),
        (30 * HOUR, "The original is kept one more day"),
        (24 * HOUR, "The original is kept one more day"),
        (24 * HOUR - 1, "The original is kept 23 more hours"),
        (14 * HOUR + 30 * 60_000, "The original is kept 14 more hours"),
        (10 * HOUR, "The original is kept 10 more hours"),
        (10 * HOUR - 1, "The original is kept nine more hours"),
        (5 * HOUR + 59 * 60_000, "The original is kept five more hours"),
        (2 * HOUR, "The original is kept two more hours"),
        (2 * HOUR - 1, "The original is kept one more hour"),
        (HOUR, "The original is kept one more hour"),
        (HOUR - 1, "The original is kept less than an hour"),
        (60_000, "The original is kept less than an hour"),
    ] {
        let r = kept_for(with_copy(3), left);
        assert_eq!(body(&r).original_kept.as_deref(), Some(words), "{left} ms left");
        assert_eq!(face(&r), "editor");
        assert!(revert(&body(&r)).enabled, "Revert works while it's kept");
    }
}

/// Its words follow the clock by themselves, never early and never late: each change lands
/// on a wake — two days to one at 47 hours less a moment, a day to hours, hour by hour, to
/// less than an hour — and the line goes with the time.
#[test]
fn the_time_left_counts_down_on_its_wakes() {
    let mut r = kept_for(with_copy(3), 48 * HOUR);
    let mut said = vec![body(&r).original_kept.expect("kept")];
    while let Some(words) = body(&r).original_kept {
        let wake = r.scene.next_wake.expect("a wake while the line shows");
        r.tick_to(wake.0 - 1);
        assert_eq!(body(&r).original_kept.as_deref(), Some(words.as_str()), "never before its wake");
        r.tick_to(wake.0);
        let now = body(&r).original_kept;
        assert_ne!(now.as_deref(), Some(words.as_str()), "the wake changes the words");
        said.extend(now);
    }
    let mut want = vec!["The original is kept two more days".to_string(), "The original is kept one more day".to_string()];
    want.extend((10..=23).rev().map(|n| format!("The original is kept {n} more hours")));
    want.extend(["nine", "eight", "seven", "six", "five", "four", "three", "two"].map(|n| format!("The original is kept {n} more hours")));
    want.extend(["The original is kept one more hour".to_string(), "The original is kept less than an hour".to_string()]);
    assert_eq!(said, want);
    assert_eq!(r.scene.next_wake, None, "nothing left to count");
}

/// No line at all: nothing kept, its time already up, or a copy that isn't the saved edit's
/// (a newer save's copy not made yet) — and once saved again, the old copy is behind it.
#[test]
fn the_time_left_shows_only_while_the_original_is_kept() {
    let r = ready_with(Some(saved_edit(3)));
    assert_eq!(body(&r).original_kept, None, "never kept");
    let r = kept_for(with_copy(3), -HOUR);
    assert_eq!(body(&r).original_kept, None, "its time is up");
    let mut stale = with_copy(3);
    stale.rev = 4;
    let r = kept_for(stale, 20 * HOUR);
    assert_eq!(body(&r).original_kept, None, "the copy is an older revision's");
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().kept_until, None);
    let r = ready_with(Some(saved_edit(3)));
    let mut r = Run::new(r.w);
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4), keep_until: Some(Wall(r.w.wall.0 + HOUR)) });
    assert_eq!(body(&r).original_kept, None, "a read over a read changes nothing");

    // Saved again: the copy it counted for is an older revision's now.
    let mut r = kept_for(with_copy(3), 20 * HOUR);
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    r.ed(Action::EditorDone);
    assert!(body(&r).original_kept.is_some(), "while it saves, still kept");
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().kept_until, None);
    assert_eq!(body(&r).original_kept, None);
}

// ── a suggestion's chip ──

/// A dashed suggestion's chip says what a selection's does, Cut <its length> (Bagrat,
/// 2026-10-19), from the one template; it sends `apply`, which cuts it.
#[test]
fn a_suggestion_is_cut_by_a_chip_like_a_selections() {
    let mut r = ready_with(Some(saved_edit(3)));
    assert_eq!(body(&r).labels.cut, "Cut {} s");
    assert!(!list(&r).cuts[1].applied);
    r.op(EditOp::Apply { index: 1 });
    assert!(list(&r).cuts[1].applied, "it's cut, red");
}

// ── the upkeep (B197 (d)) ──

/// A Mac booted in a space: the launch's version known (the upkeep's cadence starts at boot).
fn booted() -> World {
    let mut w = ready();
    w.launch.version = "1.3.0".into();
    w.once.seen_version = Some("1.3.0".into());
    w
}

fn upkeeps(r: &Run) -> bool {
    r.has(|e| *e == Effect::Editor(EditCmd::Upkeep))
}

/// Once a launch, at once, then every six hours awake after the last one answered: the
/// lane bakes and copies what it finds; nothing on screen moves.
#[test]
fn the_upkeep_runs_once_a_launch_then_every_six_hours() {
    let mut r = Run::new(booted());
    r.send(Event::Tick);
    assert!(upkeeps(&r), "at launch: {:?}", r.effects);
    assert_eq!(r.w.inflight.ops.get(&Op::EditorUpkeep), Some(&Mono(r.t + 3_600_000)), "an hour, a copy's length");
    let before = r.scene.clone();
    r.news(EditorNews::UpkeepDone { baked: 2, copies: 1 });
    assert_eq!(r.arm.name, "editor-upkept");
    assert!(r.effects.is_empty());
    assert_eq!(r.scene.state_ids(), before.state_ids(), "nothing on screen moves");
    let done = r.t;
    assert!(r.scene.next_wake.is_some_and(|t| t.0 <= done + 6 * HOUR as u64));
    r.tick_to(done + 6 * HOUR as u64 - 1_000);
    assert!(!upkeeps(&r), "not before six hours");
    r.tick_to(done + 6 * HOUR as u64);
    assert!(upkeeps(&r), "six hours on: {:?}", r.effects);
    // Not before a space, nor before the launch is known.
    let mut w = booted();
    w.space.identity = None;
    w.space.token = false;
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!upkeeps(&r));
    let mut r = Run::new(ready());
    r.send(Event::Tick);
    assert!(!upkeeps(&r), "not booted yet");
}

/// Never over a take, never offline, never while the editor is open or its lane runs
/// anything else — each waits, then the upkeep runs.
#[test]
fn the_upkeep_waits_for_the_take_the_network_and_the_editor() {
    // A take open: after it.
    let mut w = booted();
    w.take = Some(take(Phase::Recording, 10_000));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!upkeeps(&r), "never mid-take");
    r.click();
    r.send(Event::Rec(RecNews::Paused { length_ms: 10_000 }));
    r.act(Action::Discard);
    r.act(Action::ConfirmDiscard);
    assert!(upkeeps(&r), "the take is over: {:?}", r.effects);

    // Offline: once back.
    let mut w = booted();
    w.net.online = Fact::known(false, Mono(0));
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!upkeeps(&r), "never offline");
    r.send(Event::Os(shroom_core::event::OsNews::NetPath(true)));
    assert!(upkeeps(&r), "back online: {:?}", r.effects);

    // The editor open: once it closes.
    let mut w = booted();
    w.library_ops.upkept = Some(Mono(0));
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m1")]), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    let mut r = Run::new(w);
    r.lib(Action::EditRow(rec()));
    r.news(EditorNews::Opened { rec: rec(), saved: None, markers: vec![], duration_sec: Some(272.4), keep_until: None });
    r.news(EditorNews::Ready { rec: rec(), duration: 272.4 });
    r.tick_to(6 * HOUR as u64 + 1_000);
    assert!(!upkeeps(&r), "never while the editor is open");
    r.ed(Action::EditorDone);
    assert!(r.scene.editor.is_none());
    assert!(upkeeps(&r), "the editor closed: {:?}", r.effects);

    // The lane busy with a copy: after it — and a copy owed goes first.
    let mut w = booted();
    w.library_ops.copies.insert(rec(), CopyOwed { rev: 4, asked: false });
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))) && !upkeeps(&r), "one lane step at a time: {:?}", r.effects);
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    assert!(upkeeps(&r), "the copy is up: {:?}", r.effects);
    // …and while the upkeep runs, an owed copy waits for it.
    let mut w = r.w.clone();
    w.library_ops.copies.insert(RecId(OTHER.into()), CopyOwed { rev: 1, asked: false });
    let mut r = Run::new(w);
    r.send(Event::Tick);
    assert!(!r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "the upkeep holds the lane");
    r.news(EditorNews::UpkeepDone { baked: 0, copies: 0 });
    assert!(r.has(|e| matches!(e, Effect::Editor(EditCmd::Copy { .. }))), "{:?}", r.effects);
}

/// An upkeep that couldn't run is noted and tried at the next turn — six hours on, not at a
/// backoff; one asked before this Mac left its space answers no one.
#[test]
fn a_failed_upkeep_waits_for_the_next_turn() {
    let mut r = Run::new(booted());
    r.send(Event::Tick);
    assert!(upkeeps(&r));
    r.send(Event::Failed { op: Op::EditorUpkeep, class: FailClass::Transport });
    let failed = r.t;
    assert!(r.w.inflight.failures.contains_key(&Op::EditorUpkeep), "noted");
    assert!(r.scene.editor.is_none() && r.scene.popover.is_none(), "nothing on screen");
    let backoff = r.w.inflight.failures[&Op::EditorUpkeep].retry_at();
    r.tick_to(backoff.0 + 1_000);
    assert!(!upkeeps(&r), "not at a backoff");
    r.tick_to(failed + 6 * HOUR as u64);
    assert!(upkeeps(&r), "the next turn: {:?}", r.effects);
    r.news(EditorNews::UpkeepDone { baked: 1, copies: 0 });
    assert!(!r.w.inflight.failures.contains_key(&Op::EditorUpkeep));
    // One that never answers fails at its deadline the same way.
    let mut r = Run::new(booted());
    r.send(Event::Tick);
    let deadline = r.w.inflight.ops[&Op::EditorUpkeep];
    r.tick_to(deadline.0 + 10);
    assert!(r.w.inflight.failures.contains_key(&Op::EditorUpkeep) && !upkeeps(&r));
    // A late answer with nothing asked is stale.
    let mut r = Run::new(booted());
    r.news(EditorNews::UpkeepDone { baked: 0, copies: 0 });
    assert_eq!(r.arm.name, "editor-stale");
}

#[test]
fn the_upkeeps_deadline_is_an_hour() {
    assert_eq!(Op::EditorUpkeep.deadline_ms(), 3_600_000);
    assert_eq!(shroom_core::derive::UPKEEP_MS, 6 * HOUR as u64);
}

// ── quit and update with an edit not saved yet (B199) ──

fn quits(r: &Run) -> bool {
    r.has(|e| *e == Effect::Shell(ShellCmd::Quit))
}

fn saves(r: &Run) -> bool {
    r.has(|e| matches!(e, Effect::Editor(EditCmd::Save { .. })))
}

fn applies(r: &Run) -> bool {
    r.has(|e| matches!(e, Effect::Updater(UpdCmd::Apply(_))))
}

/// An update downloaded and verified (N37), and Settings open on General.
fn with_update(mut w: World) -> Run {
    w.update.offer = Fact::known(Some(UpdateOffer::Ready { version: "1.3.0".into() }), w.now);
    w.update.staged = Some("1.3.0".into());
    w.prefs.settings_tab = Some(SettingsTab::General);
    w.session.windows.insert(WindowKind::Settings, Door::asked(w.now));
    Run::new(w)
}

fn update_in_settings() -> Event {
    Event::Act { surface: Surface::Settings, action: Action::Update, rev: 0 }
}

/// Quit from the Dock, the right-click menu or macOS with a change not saved yet: the edit is
/// saved first, without asking (only ✕ asks, N71), and the editor comes forward to show it saving; shroom
/// quits once the save lands — not waiting for the final video it goes on to make.
#[test]
fn quit_saves_an_unsaved_edit_first_then_quits() {
    for via in [QuitVia::Dock, QuitVia::RightClick, QuitVia::System] {
        let mut r = dirty();
        r.send(Event::QuitRequested(via));
        assert!(!quits(&r), "{via:?}: not before the edit is saved");
        assert!(r.has(|e| *e == Effect::Editor(EditCmd::Save { rec: rec(), saved: Saved::new(edited().checked(), 4) })), "{via:?}: {:?}", r.effects);
        assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenEditor)), "{via:?}: the editor comes forward");
        assert_eq!(r.arm.edge, Some(("editor", "editor-saving")), "{via:?}");
        assert_eq!(face(&r), "editor-saving");
        assert!(r.scene.editor.as_ref().unwrap().activates);
        r.news(EditorNews::Saved { rec: rec(), rev: 4 });
        assert!(quits(&r), "{via:?}: saved — shroom quits: {:?}", r.effects);
        assert_eq!(face(&r), "editor-making", "{via:?}: the final video isn't waited for");
    }
}

/// The popover's Quit the same: the popover makes way for the editor.
#[test]
fn the_popovers_quit_saves_first_too() {
    let mut r = dirty();
    r.click();
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("config"));
    r.act(Action::Quit);
    assert!(saves(&r) && !quits(&r), "{:?}", r.effects);
    assert!(r.scene.popover.is_none(), "the popover makes way for the editor");
    assert!(r.has(|e| *e == Effect::Shell(ShellCmd::OpenEditor)));
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(quits(&r), "{:?}", r.effects);
}

/// Uploads still draining: their question comes first, as ever; Quit anyway then saves the
/// edit, and shroom quits once it lands.
#[test]
fn quit_anyway_saves_the_edit_first() {
    let mut w = dirty().w;
    w.queue.push(Queued { dir: TakeDir("q41".into()), rec: RecId("UpFrOmThIsMaC001".into()), title: "Untitled — Sep 30".into(), stopped: Wall(AUG21), done: 2, total: 5, blocked: None, length_ms: Some(48_000) });
    let mut r = Run::new(w);
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert_eq!(r.scene.popover.as_ref().map(|p| p.id), Some("quit-confirm"));
    assert!(!saves(&r) && !quits(&r), "the question first: {:?}", r.effects);
    r.act(Action::QuitAnyway);
    assert!(saves(&r) && !quits(&r), "{:?}", r.effects);
    assert!(r.scene.popover.is_none() && r.w.session.confirm.is_none(), "answered: the popover makes way");
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(quits(&r), "{:?}", r.effects);
}

/// Update in Settings the same way: the edit is saved first, then the update applies — and
/// the window it brings back after the relaunch is never the editor (N63).
#[test]
fn update_saves_an_unsaved_edit_first_then_applies() {
    let mut r = with_update(dirty().w);
    r.send(update_in_settings());
    assert!(!applies(&r) && !r.w.update.applying, "not before the edit is saved: {:?}", r.effects);
    assert!(saves(&r) && r.has(|e| *e == Effect::Shell(ShellCmd::OpenEditor)), "{:?}", r.effects);
    assert_eq!(r.arm.edge, Some(("editor", "editor-saving")));
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(r.has(|e| *e == Effect::Updater(UpdCmd::Apply("1.3.0".into()))), "{:?}", r.effects);
    assert!(r.w.update.applying);
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Restore(Some(Restore { window: WindowKind::Settings, .. }))))), "{:?}", r.effects);
}

/// A save already on its way is waited for; asking again changes nothing.
#[test]
fn a_quit_while_saving_waits_for_the_save() {
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert!(!saves(&r) && !quits(&r), "no second save, no quit yet: {:?}", r.effects);
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().leaving, Some(Leaving::Quit));
    r.send(Event::QuitRequested(QuitVia::RightClick));
    assert!(!saves(&r) && !quits(&r));
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(quits(&r), "{:?}", r.effects);
}

/// A save that doesn't land holds the quit and the update: shroom stays, and the editor says
/// why; Done later saves, and nothing quits behind it.
#[test]
fn a_save_that_doesnt_land_holds_the_quit() {
    let mut r = dirty();
    r.send(Event::QuitRequested(QuitVia::Dock));
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Transport });
    assert_eq!(face(&r), "editor-save-failed");
    assert!(!quits(&r));
    assert_eq!(r.w.session.fields.editor.as_ref().unwrap().leaving, None, "the quit is dropped");
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert!(!quits(&r), "it went with the failed save: {:?}", r.effects);

    // Turned down: its own face says why, and nothing applies.
    let mut r = with_update(dirty().w);
    r.send(update_in_settings());
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    assert_eq!(face(&r), "editor-save-refused");
    assert!(!applies(&r) && !r.w.update.applying);

    // Never answering: the same, at its deadline.
    let mut r = dirty();
    r.send(Event::QuitRequested(QuitVia::RightClick));
    let deadline = r.w.inflight.ops[&Op::EditorSave];
    r.tick_to(deadline.0 + 10);
    assert_eq!(face(&r), "editor-save-failed");
    assert!(!quits(&r));
}

/// Nothing to save — the editor unchanged, its final video being made, a save the space
/// turned down — and the quit or the update goes on at once.
#[test]
fn with_nothing_to_save_they_go_on_at_once() {
    let mut r = ready_editor();
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::Quit)]);
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    assert_eq!(face(&r), "editor-making");
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert!(quits(&r), "the final video isn't waited for: {:?}", r.effects);
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    r.send(Event::QuitRequested(QuitVia::Dock));
    assert!(quits(&r) && !saves(&r), "{:?}", r.effects);
    let mut r = with_update(ready_editor().w);
    r.send(update_in_settings());
    assert!(applies(&r) && !saves(&r), "{:?}", r.effects);
}

/// A take begun while the save is on its way holds everything that would end the app (law 4):
/// the quit, or the update, is dropped.
#[test]
fn a_take_begun_meanwhile_drops_the_quit_and_the_update() {
    for asked in [Event::QuitRequested(QuitVia::Dock), update_in_settings()] {
        let mut r = with_update(dirty().w);
        r.send(asked.clone());
        assert!(saves(&r), "{asked:?}: {:?}", r.effects);
        let mut w = r.w.clone();
        w.take = Some(OpenTake { opened: w.now, since: w.now, ..take(Phase::Recording, 1_000) });
        let mut r = Run::new(w);
        r.news(EditorNews::Saved { rec: rec(), rev: 4 });
        assert!(!quits(&r) && !applies(&r) && !r.w.update.applying, "{asked:?}: {:?}", r.effects);
        assert!(r.w.take.is_some());
    }
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

/// A question open in a window — the Library's delete sheet — greys Drop a marker in its own
/// words: nothing was dropped, so the row never says Marker dropped (B199). ⌥⌘M drops
/// nothing meanwhile; once the sheet goes the row is live again, and a marker dropped then
/// reads Marker dropped whatever opens next.
#[test]
fn a_windows_question_greys_drop_a_marker_in_its_own_words() {
    // The Library was open before the take; the take is paused, no marker dropped yet.
    let mut w = library();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    w.session.popover = Some(Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.lib(Action::DeleteRow(rec()));
    assert!(matches!(r.w.session.confirm, Some(Confirm::Delete(_))), "the delete sheet is up");
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Drop a marker", Style::Plain, false));
    r.send(Event::MarkerShortcut(quick()));
    assert!(!r.has(|e| *e == Effect::Recorder(RecCmd::Marker)) && r.w.take.as_ref().unwrap().markers == 0, "nothing is dropped meanwhile");
    // Back on the sheet: the row is live again.
    r.key(Surface::Library, Key::Esc);
    assert_eq!(r.w.session.confirm, None);
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Drop a marker", Style::Plain, true));
    // Dropped, then the sheet again: Marker dropped, greyed, as it was.
    r.act(Action::DropMarker);
    r.lib(Action::DeleteRow(rec()));
    assert!(matches!(r.w.session.confirm, Some(Confirm::Delete(_))));
    assert_eq!(button(&paused_body(&r).marker.unwrap()), ("Marker dropped", Style::Plain, false));
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

// ── the machine's edges ──

/// Every edge spec/machine.json draws to or from an editor face is named by an arm (each
/// walked above, where `Run::send` holds it to the machine) — but the one the facts derive.
#[test]
fn every_editor_edge_is_named_by_an_arm() {
    const DERIVED: &[(&str, &str, &str)] = &[
        ("library", "editor-getting", "Edit… names library → editor; the lane's Getting news turns the face"),
        ("owner-page", "editor-getting", "the page's Edit in shroom names owner-page → editor; the lane's Getting news turns the face (N69)"),
    ];
    let src = include_str!("../src/apply.rs");
    let mut missing = Vec::new();
    for e in spec().edges.iter().filter(|e| e.from.starts_with("editor") || e.to.starts_with("editor")) {
        let literal = format!("(\"{}\", \"{}\")", e.from, e.to);
        let derived = DERIVED.iter().any(|(f, t, _)| *f == e.from && *t == e.to);
        if src.contains(&literal) == derived {
            missing.push(format!("{} → {}{}", e.from, e.to, if derived { " (listed as derived, yet named)" } else { "" }));
        }
    }
    assert!(missing.is_empty(), "editor edges no arm names: {missing:?}");
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
            "after", "can_redo", "can_undo", "closing", "done", "editable", "edited_duration", "footer", "footer_tone", "hint", "keep", "labels", "list", "markers",
            "media", "original_kept", "rec", "revert", "source_duration", "title"
        ]
    );
    assert_eq!(keys(&b["labels"]), ["cut", "pause", "play", "put_back", "redo", "time", "undo", "was"]);
    assert_eq!((b["after"].clone(), b["original_kept"].clone(), b["closing"].clone()), (serde_json::Value::Null, serde_json::Value::Null, serde_json::Value::Null));
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

    // ✕ with changes not saved (N71): the question, its three buttons, Return Save and Esc
    // Keep editing.
    let mut r = dirty();
    r.send(Event::WindowClosed(WindowKind::Editor));
    let v = serde_json::to_value(r.scene.editor.as_ref().unwrap()).unwrap();
    let c = &v["body"]["closing"];
    assert_eq!(keys(c), ["discard", "keep", "line", "save", "title"]);
    assert_eq!((c["title"].clone(), c["line"].clone()), (json!(CLOSE_TITLE), json!(CLOSE_LINE)));
    assert_eq!(
        (c["discard"]["action"].clone(), c["keep"]["action"].clone(), c["save"]["action"].clone()),
        (json!({ "do": "editor-discard" }), json!({ "do": "editor-keep-editing" }), json!({ "do": "editor-done" }))
    );
    assert_eq!((v["ret"].clone(), v["esc"].clone()), (json!({ "do": "editor-done" }), json!({ "do": "editor-keep-editing" })));
    assert_eq!(v["body"]["editable"], json!(false));

    // A save the space turned down (B199): no Revert, Close in Done's place, Return and Esc
    // Close.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.send(Event::Failed { op: Op::EditorSave, class: FailClass::Refused });
    let v = serde_json::to_value(r.scene.editor.as_ref().unwrap()).unwrap();
    assert_eq!((v["body"]["revert"].clone(), v["body"]["footer_tone"].clone()), (serde_json::Value::Null, json!("amber")));
    assert_eq!(v["body"]["done"]["action"], json!({ "do": "editor-close" }));
    assert_eq!((v["ret"].clone(), v["esc"].clone()), (json!({ "do": "editor-close" }), json!({ "do": "editor-close" })));

    // After the save (B197): the final video's bar, its failure, then the question.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.news(EditorNews::Saved { rec: rec(), rev: 4 });
    r.news(EditorNews::Making { rec: rec(), pct: 40 });
    let v = serde_json::to_value(r.scene.editor.as_ref().unwrap()).unwrap();
    assert_eq!(v["body"]["after"], json!({ "state": "making", "pct": 40, "line": "Making the final video… 40%" }));
    assert_eq!(v["body"]["done"]["action"], json!({ "do": "editor-close" }));
    assert_eq!((v["ret"].clone(), v["esc"].clone()), (json!({ "do": "editor-close" }), json!({ "do": "editor-close" })));
    let mut failed = Run::new(r.w.clone());
    failed.send(Event::Failed { op: Op::EditorCopy, class: FailClass::Transport });
    let b = serde_json::to_value(&failed.scene.editor.as_ref().unwrap().body).unwrap();
    assert_eq!((b["after"].clone(), b["footer_tone"].clone()), (json!({ "state": "failed" }), json!("amber")));
    r.news(EditorNews::CopyDone { rec: rec(), rev: 4, named: true });
    let v = serde_json::to_value(r.scene.editor.as_ref().unwrap()).unwrap();
    let a = &v["body"]["after"];
    assert_eq!(keys(a), ["delete", "keep", "line", "state", "title"]);
    assert_eq!((a["state"].clone(), a["title"].clone(), a["line"].clone()), (json!("asking"), json!(ASK_TITLE), json!(ASK_LINE)));
    assert_eq!((a["keep"]["label"].clone(), a["keep"]["action"].clone()), (json!("Keep for 2 days"), json!({ "do": "keep-original" })));
    assert_eq!((a["delete"]["label"].clone(), a["delete"]["action"].clone()), (json!("Yes, delete"), json!({ "do": "delete-original" })));
    assert_eq!((v["ret"].clone(), v["esc"].clone()), (json!(null), json!({ "do": "keep-original" })));
    // The time the original has left, by Revert: 20 hours less a moment, in whole hours.
    let mut r = opened();
    let until = Wall(r.w.wall.0 + 20 * HOUR);
    r.news(EditorNews::Opened { rec: rec(), saved: Some(with_copy(3)), markers: vec![], duration_sec: Some(272.4), keep_until: Some(until) });
    let b = serde_json::to_value(&r.scene.editor.as_ref().unwrap().body).unwrap();
    assert_eq!(b["original_kept"], json!("The original is kept 19 more hours"));

    assert_eq!(serde_json::to_value(Action::EditRow(rec())).unwrap(), json!({ "do": "edit-row", "arg": REC }));
    assert_eq!(serde_json::to_value(Surface::Editor).unwrap(), json!("editor"));
    assert_eq!(serde_json::to_value(WindowKind::Editor).unwrap(), json!("editor"));
    let paused = serde_json::to_value(Run::new(paused_popover(Cause::User)).scene.popover.unwrap().body).unwrap();
    assert_eq!(paused["marker"]["action"], json!({ "do": "drop-marker" }));
}

// ── Edit in shroom: the owner page's scissors (N69, B221) ──

const HOST: &str = "v.myshroom.workers.dev";
const ELSEWHERE: &str = "You're editing another video — press Done, then Edit again.";
const OTHER_SPACE: &str = "This Mac isn't set up for that video's space.";
const NOT_YOURS: &str = "Only whoever recorded this video, or the space's owner, can edit it.";
const GONE: &str = "That video isn't in your space anymore.";

/// The page's press reaching this Mac: `shroom://edit/<host>/<id>`, read by the app.
fn pressed(r: &mut Run, host: &str, id: &str) {
    r.send(Event::EditLink { host: host.into(), rec: RecId(id.into()) });
}

/// A ready Mac at rest, its Library known from an earlier look, no window open.
fn at_rest() -> World {
    let mut w = library();
    w.session.windows.clear();
    w
}

fn status(r: &Run) -> Option<String> {
    r.scene.popover.as_ref().and_then(|p| p.status_line.clone())
}

/// The popover the Mac's answer opens: by itself, under the mushroom, without the
/// keyboard (law 5), put away by a click elsewhere (as an update's news is).
fn says_once(r: &Run, line: &str) {
    let p = r.scene.popover.as_ref().expect("the popover opens by itself");
    assert_eq!(p.id, "config");
    assert_eq!(p.status_line.as_deref(), Some(line));
    assert_eq!(p.status_tone, Some(shroom_core::scene::StatusTone::Waiting), "the grey dot, as a take that kept nothing");
    assert!(!p.activates && p.away_closes, "the app the person was in keeps the keyboard; a click away puts it away");
    assert!(r.scene.editor.is_none() && !opens(r), "no editor");
}

/// N69: a video this Mac can edit opens in the editor exactly as Edit… opens it.
#[test]
fn the_pages_edit_opens_the_editor_as_edit_does() {
    let mut r = Run::new(at_rest());
    pressed(&mut r, HOST, REC);
    assert_eq!(r.effects, vec![Effect::Editor(EditCmd::Open { rec: rec() }), Effect::Shell(ShellCmd::OpenEditor)]);
    assert_eq!(r.arm.edge, Some(("owner-page", "editor")));
    let win = r.scene.editor.as_ref().expect("the editor");
    assert!(win.activates, "the person asked for it, on the page");
    assert_eq!((body(&r).title.as_str(), body(&r).rec.as_str()), ("Edit — Setting up shroom in two minutes", REC));
    assert!(r.scene.library.is_none(), "the editor alone: the Library wasn't asked for");
    assert_eq!(status(&r), None);

    // The host as a browser may write it, any case: the same space.
    let mut r = Run::new(at_rest());
    pressed(&mut r, "V.MyShroom.Workers.Dev", REC);
    assert!(opens(&r));

    // The editor goes on as Edit…'s does: the video fetched from the space when it isn't here.
    r.news(EditorNews::Getting { rec: rec(), pct: 0 });
    assert_eq!(face(&r), "editor-getting");
}

/// The owner's Macs edit every video in the space, whichever view the Library was left on;
/// a member's Mac edits its own.
#[test]
fn the_page_reaches_every_video_an_owners_mac_may_edit() {
    let mut w = at_rest();
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m1"), row(OTHER, "Mher's onboarding notes", true, "m_mher")]), Mono(950));
    w.session.fields.library.scope = LibraryScope::Mine;
    let mut r = Run::new(w);
    pressed(&mut r, HOST, OTHER);
    assert!(opens(&r), "a member's video, from an owner's Mac left on Mine");
    assert_eq!(body(&r).title, "Edit — Mher's onboarding notes");

    let mut w = at_rest();
    w.space.identity.as_mut().unwrap().role = Role::Member;
    w.space.identity.as_mut().unwrap().member_id = "m2".into();
    w.space.token = false;
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m2")]), Mono(950));
    let mut r = Run::new(w);
    pressed(&mut r, HOST, REC);
    assert!(opens(&r), "a member's own");
}

/// The editor already open: it comes forward as it is, one at a time — on the same video
/// with nothing to say, on another with the footer saying why, until the next change.
#[test]
fn the_page_brings_the_open_editor_forward_and_says_when_its_on_another_video() {
    let mut r = ready_editor();
    pressed(&mut r, HOST, REC);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)], "forward, nothing asked again");
    assert_eq!(r.arm.edge, Some(("owner-page", "editor")));
    assert_eq!((body(&r).footer.as_str(), body(&r).footer_tone), (FOOTER, LineTone::Plain));

    pressed(&mut r, HOST, OTHER);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)]);
    assert_eq!(body(&r).rec, REC, "it doesn't swap videos");
    assert_eq!((body(&r).footer.as_str(), body(&r).footer_tone), (ELSEWHERE, LineTone::Amber));
    assert!(body(&r).editable, "its changes are as they were, still to step");

    // The next change, and the line goes.
    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    assert_eq!(body(&r).footer, FOOTER);

    // Mid-save, or once it's making the final video, it only comes forward: those faces say
    // what to press.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    pressed(&mut r, HOST, OTHER);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)]);
    assert_eq!(body(&r).footer, SAVING);
}

/// Edit… in the Library for another video, the editor open: the same as the page's Edit. It
/// comes forward as it is and says why, until the next change (2026-10-19: this route said
/// nothing, and the editor looked stuck on the video it held).
#[test]
fn edit_in_the_library_for_another_video_says_why_too() {
    let mut r = ready_editor();
    r.lib(Action::EditRow(RecId(REC.into())));
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)], "its own video: forward, nothing asked again");
    assert_eq!((body(&r).footer.as_str(), body(&r).footer_tone), (FOOTER, LineTone::Plain));

    r.lib(Action::EditRow(RecId(OTHER.into())));
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenEditor)]);
    assert_eq!(body(&r).rec, REC, "it doesn't swap videos");
    assert_eq!((body(&r).footer.as_str(), body(&r).footer_tone), (ELSEWHERE, LineTone::Amber));

    r.op(EditOp::Cut { start: 20.0, end: 30.0 });
    assert_eq!(body(&r).footer, FOOTER, "the next change puts it away");

    // Mid-save it only comes forward: that face says what's happening.
    let mut r = dirty();
    r.ed(Action::EditorDone);
    r.lib(Action::EditRow(RecId(OTHER.into())));
    assert_eq!(body(&r).footer, SAVING);
}

/// Law 4: while a take is open nothing new shows. The press waits, and once the take is
/// over the editor opens on the video it asked for; a later press takes its place.
#[test]
fn a_press_during_a_take_waits_for_it_to_end() {
    let mut w = at_rest();
    w.take = Some(take(Phase::Paused(Cause::User), 272_000));
    let mut r = Run::new(w);
    pressed(&mut r, HOST, OTHER);
    pressed(&mut r, HOST, REC);
    assert!(r.effects.is_empty() && r.scene.editor.is_none() && r.w.session.fields.editor.is_none(), "law 4");
    assert_eq!(status(&r), None);
    r.click();
    r.act(Action::Discard).act(Action::ConfirmDiscard);
    assert!(r.w.take.is_none());
    assert!(opens(&r), "the take is over: the editor opens");
    assert_eq!(body(&r).rec, REC, "the later press");
    let mut again = Run::new(r.w.clone());
    again.send(Event::Tick);
    assert!(!opens(&again), "once");
}

/// A Mac set up for another space says so, once: the popover opens by itself with the line,
/// and it's gone when the popover closes.
#[test]
fn another_spaces_video_is_said_once_in_the_popover() {
    let mut r = Run::new(at_rest());
    pressed(&mut r, "v.someoneelse.workers.dev", REC);
    says_once(&r, OTHER_SPACE);
    assert_eq!(r.arm.edge, Some(("owner-page", "config")));
    r.send(Event::PopoverDismissed);
    r.click();
    assert_eq!(status(&r), None, "said once");
}

/// A video not in this Mac's Library yet: one refresh, then it opens — or, still not there,
/// the owner's Mac (which lists every video) says it's gone, a member's (which lists its
/// own) that it's someone else's.
#[test]
fn a_video_the_library_doesnt_know_is_asked_for_once() {
    let fresh = "FrEsHfRoMmHeR001";
    let mut r = Run::new(at_rest());
    pressed(&mut r, HOST, fresh);
    assert_eq!(r.effects, vec![Effect::Library(LibCmd::Refresh)]);
    assert!(r.scene.editor.is_none() && status(&r).is_none());
    let mut rows = vec![row(REC, TITLE, true, "m1")];
    rows.push(row(fresh, "Mher's walkthrough", true, "m_mher"));
    r.send(Event::Library(LibraryNews::Rows { rows: rows.clone(), fresh: true }));
    assert!(opens(&r), "there now: it opens");
    assert_eq!(body(&r).title, "Edit — Mher's walkthrough");

    // Gone from the space: an owner's Mac says so.
    let mut r = Run::new(at_rest());
    pressed(&mut r, HOST, fresh);
    r.send(Event::Library(LibraryNews::Rows { rows: vec![row(REC, TITLE, true, "m1")], fresh: true }));
    says_once(&r, GONE);

    // A member's Mac: not among its own.
    let mut w = at_rest();
    w.space.identity.as_mut().unwrap().role = Role::Member;
    w.space.identity.as_mut().unwrap().member_id = "m2".into();
    w.space.token = false;
    w.library = Fact::known(Arc::new(vec![row(REC, TITLE, true, "m2")]), Mono(950));
    let mut r = Run::new(w);
    pressed(&mut r, HOST, OTHER);
    assert_eq!(r.effects, vec![Effect::Library(LibCmd::Refresh)]);
    r.send(Event::Library(LibraryNews::Rows { rows: vec![row(REC, TITLE, true, "m2")], fresh: true }));
    says_once(&r, NOT_YOURS);

    // A refresh that can't get through opens nothing and says nothing.
    let mut r = Run::new(at_rest());
    pressed(&mut r, HOST, fresh);
    r.send(Event::Failed { op: Op::LibRefresh, class: FailClass::Transport });
    assert!(r.scene.editor.is_none() && status(&r).is_none());
    r.send(Event::Library(LibraryNews::Rows { rows, fresh: true }));
    assert!(!opens(&r), "dropped: a later answer opens nothing");
}

/// A Mac not set up at all: its welcome window, as a click on the mushroom opens it.
#[test]
fn a_mac_not_set_up_opens_its_welcome_window() {
    let mut r = Run::new(World::blank());
    pressed(&mut r, HOST, REC);
    assert!(r.scene.onboarding.is_some(), "the welcome window");
    assert!(r.scene.editor.is_none() && !opens(&r));
}
