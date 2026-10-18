//! The editor (N63, B195): one window, on one video — `editor` (the video here, or its
//! record still being read), `editor-getting` (it comes from the space), `editor-cant-get`,
//! `editor-saving` and `editor-save-failed`. It shows the list as it stands, what plays,
//! and what can be done; the webview plays, draws and sends steps, and decides nothing.

use crate::copy;
use crate::scene::{Action, Button, CutView, EditListView, EditorBody, EditorLabels, EditorMediaView, LineTone, Style, Window};
use crate::time::Mono;
use crate::world::{EditorFields, EditorMedia, EditorSave, WindowKind, World};

pub fn derive(w: &World) -> Option<Window<EditorBody>> {
    let door = w.session.windows.get(&WindowKind::Editor)?;
    let e = w.session.fields.editor.as_ref()?;
    if !w.window_visible(WindowKind::Editor) || !w.provisioned() {
        return None;
    }
    let id = state(e);
    let live = ready(e);
    let editable = editable(e);
    let history = e.history.as_ref();
    let media = match e.media {
        EditorMedia::Opening => EditorMediaView::Opening,
        EditorMedia::Getting { pct } => EditorMediaView::Getting { pct, line: copy::getting_video() },
        EditorMedia::Failed => EditorMediaView::Failed { line: copy::cant_get_video(), retry: Button::new(Action::EditorRetry, copy::try_again(), Style::Plain) },
        EditorMedia::Ready { duration } => EditorMediaView::Ready { duration },
    };
    let (footer, footer_tone) = match e.save {
        EditorSave::Idle => (copy::editor_footer(), LineTone::Plain),
        EditorSave::Saving => (copy::saving_edit(), LineTone::Plain),
        EditorSave::Failed => (copy::save_failed(), LineTone::Amber),
    };
    let done = match (e.media, e.save) {
        // The video couldn't come: Close is the way out.
        (EditorMedia::Failed, _) => Button::new(Action::EditorClose, copy::close(), Style::Filled),
        (_, EditorSave::Saving) => Button::new(Action::EditorDone, copy::saving_done(), Style::Filled).enabled(false),
        _ => Button::new(Action::EditorDone, copy::done(), Style::Filled).enabled(live),
    };
    let now = history.map(|h| &h.now);
    let body = EditorBody {
        title: copy::editor_title(&e.title),
        rec: e.rec.0.clone(),
        media,
        hint: copy::editor_hint(),
        footer,
        footer_tone,
        labels: EditorLabels {
            play: copy::editor_play(),
            pause: copy::editor_pause(),
            undo: copy::undo(),
            redo: copy::redo(),
            cut: copy::cut_part("{}"),
            put_back: copy::put_back("{}"),
            was: copy::editor_was("{}"),
            time: copy::editor_time("{}", "{}"),
            cut_it_too: copy::cut_it_too(),
        },
        list: now.map(|l| EditListView {
            in_: l.in_,
            out: l.out,
            cuts: l.cuts.iter().map(|c| CutView { start: c.start, end: c.end, applied: c.applied, kind: kind(c.kind).into() }).collect(),
        }),
        keep: now.map(|l| l.keep().into_iter().map(|(s, e)| [s, e]).collect()).unwrap_or_default(),
        edited_duration: now.map_or(e.source_duration, |l| l.edited_duration()),
        source_duration: now.map_or(e.source_duration, |l| l.source_duration),
        markers: e.markers.clone(),
        editable,
        can_undo: editable && history.is_some_and(|h| h.can_undo()),
        can_redo: editable && history.is_some_and(|h| h.can_redo()),
        revert: Button::new(Action::EditorRevert, copy::revert_to_original(), Style::Plain).enabled(editable && history.is_some_and(|h| h.can_revert())),
        done,
        original_kept: kept_left(w, e).map(copy::original_kept),
    };
    // Return is Done once there's a video to save over (cant-get's Close); Esc is the
    // webview's own — it drops a selection — except where Close is the one way on.
    let (ret, esc) = match id {
        "editor-cant-get" => (Some(Action::EditorClose), Some(Action::EditorClose)),
        "editor" | "editor-save-failed" if live => (Some(Action::EditorDone), None),
        _ => (None, None),
    };
    Some(Window { id, faces: vec![], body, ret, esc, activates: door.asked })
}

/// The editor's state id, by where the save and the video stand.
pub fn state(e: &EditorFields) -> &'static str {
    match (e.save, e.media) {
        (EditorSave::Saving, _) => "editor-saving",
        (EditorSave::Failed, _) => "editor-save-failed",
        (_, EditorMedia::Getting { .. }) => "editor-getting",
        (_, EditorMedia::Failed) => "editor-cant-get",
        (EditorSave::Idle, EditorMedia::Opening | EditorMedia::Ready { .. }) => "editor",
    }
}

/// The video is here and its record read: there is something Done can save over.
pub fn ready(e: &EditorFields) -> bool {
    matches!(e.media, EditorMedia::Ready { .. }) && e.history.is_some()
}

/// Steps are taken: ready, and nothing saving.
pub fn editable(e: &EditorFields) -> bool {
    ready(e) && e.save != EditorSave::Saving
}

/// How long the original has left beside its copy (ms), while it's kept (B197 (e)).
fn kept_left(w: &World, e: &EditorFields) -> Option<i64> {
    let left = e.kept_until?.0 - w.wall.0;
    (left > 0).then_some(left)
}

/// When the time-left line by Revert next changes its words — a day left, an hour left,
/// gone — as wakes on the monotonic clock.
pub fn wakes(w: &World, out: &mut Vec<Mono>) {
    const HOUR: i64 = 3_600_000;
    let Some(left) = w.session.fields.editor.as_ref().and_then(|e| kept_left(w, e)) else { return };
    // A day left reads “1 more day”; under an hour, “a few more hours”; at none, nothing.
    for at in [left - 24 * HOUR, left - HOUR + 1, left] {
        if at > 0 {
            out.push(w.now.plus(at as u64));
        }
    }
}

/// A cut's kind, as the record spells it (`shroom_edit::CutKind`'s serde names).
fn kind(k: shroom_edit::CutKind) -> &'static str {
    match k {
        shroom_edit::CutKind::Retake => "retake",
        shroom_edit::CutKind::CueOnly => "cue-only",
        shroom_edit::CutKind::LineBeforeCue => "line-before-cue",
        shroom_edit::CutKind::Manual => "manual",
    }
}
