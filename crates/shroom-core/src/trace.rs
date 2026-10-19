//! The trace ring (ARCHITECTURE §3.3 “Trace”, DECISIONS B93): the last `RING_ROWS` steps of
//! the core — the event's kind, the arm that took it and the edge it realized, the kinds of
//! the effects it emitted, and the state ids each batch left on screen. “Why is this screen
//! up” is answered by reading it.
//!
//! It holds **kinds and names only** (law 13): never a secret, a link's code (a recording
//! id), a title, a label, a host or a path. A kind is read from the value's own `Debug`
//! shape — its variant names, at most two levels deep — and stops at the first character
//! that isn't part of a name, so a quoted string or a number never gets in.
//!
//! The runner dumps it to the private `run.log` when a law breaks (`breach`, dev builds),
//! when a take turns out to have been rescued (a crash-recovery launch, a capture that
//! died), when anything panics, and as a harness run ends.

use std::collections::VecDeque;
use std::fmt::Write as _;

use crate::apply::Arm;
use crate::effect::{CloudCmd, Effect, MemberCmd, PersistOp, ShellCmd, UpdCmd};
use crate::event::Event;
use crate::scene::Scene;
use crate::world::World;

/// How many rows the ring keeps.
pub const RING_ROWS: usize = 512;

/// One row: a step (an event and what it did), or the screen after a batch.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Step { mono: u64, event: String, arm: &'static str, edge: Option<(&'static str, &'static str)>, effects: Vec<String> },
    Screen { mono: u64, states: Vec<&'static str> },
}

impl Row {
    /// The row as one log line.
    pub fn line(&self) -> String {
        match self {
            Row::Step { mono, event, arm, edge, effects } => {
                let mut l = format!("{mono:>8} {event} → {arm}");
                if let Some((from, to)) = edge {
                    let _ = write!(l, " [{from} → {to}]");
                }
                if !effects.is_empty() {
                    let _ = write!(l, " ⇒ {}", effects.join(", "));
                }
                l
            }
            Row::Screen { mono, states } => format!("{mono:>8} screen: {}", states.join(" · ")),
        }
    }
}

/// A bounded ring of rows: the oldest goes when a new one comes past the bound.
#[derive(Debug, Clone)]
pub struct Ring {
    rows: VecDeque<Row>,
    cap: usize,
}

impl Default for Ring {
    fn default() -> Self {
        Ring::new(RING_ROWS)
    }
}

impl Ring {
    pub fn new(cap: usize) -> Ring {
        Ring { rows: VecDeque::with_capacity(cap.min(RING_ROWS)), cap: cap.max(1) }
    }

    fn push(&mut self, row: Row) {
        while self.rows.len() >= self.cap {
            self.rows.pop_front();
        }
        self.rows.push_back(row);
    }

    /// One step: the event as it arrived, the arm that took it, what it emitted.
    pub fn step(&mut self, mono: u64, event: &Event, arm: &Arm, effects: &[Effect]) {
        self.step_named(mono, event_kind(event), arm, effects);
    }

    /// One step whose event was named before `update` took it (`event_kind`).
    pub fn step_named(&mut self, mono: u64, event: String, arm: &Arm, effects: &[Effect]) {
        self.push(Row::Step { mono, event, arm: arm.name, edge: arm.edge, effects: effects.iter().map(effect_kind).collect() });
    }

    /// The screen after a batch — kept only when it changed since the last one.
    pub fn screen(&mut self, mono: u64, scene: &Scene) {
        let states = scene.state_ids();
        let last = self.rows.iter().rev().find_map(|r| match r {
            Row::Screen { states, .. } => Some(states),
            Row::Step { .. } => None,
        });
        if last != Some(&states) {
            self.push(Row::Screen { mono, states });
        }
    }

    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The whole ring as log lines, oldest first.
    pub fn lines(&self) -> Vec<String> {
        self.rows.iter().map(Row::line).collect()
    }
}

/// A `fmt::Write` that keeps the first `cap` bytes and then refuses: a `Debug` of a big
/// value (a Library's rows) stops early instead of formatting all of it.
struct Head {
    s: String,
    cap: usize,
}

impl std::fmt::Write for Head {
    fn write_str(&mut self, part: &str) -> std::fmt::Result {
        let room = self.cap.saturating_sub(self.s.len());
        if part.len() > room {
            let mut end = room;
            while !part.is_char_boundary(end) {
                end -= 1;
            }
            self.s.push_str(&part[..end]);
            return Err(std::fmt::Error);
        }
        self.s.push_str(part);
        Ok(())
    }
}

fn debug_head(v: &impl std::fmt::Debug) -> String {
    let mut h = Head { s: String::new(), cap: 160 };
    let _ = write!(h, "{v:?}");
    h.s
}

/// The variant names at the head of a `Debug` rendering, at most `levels` deep:
/// `Cloud(OpenOwnerPage(RecId("AbCd…")))` → `Cloud.OpenOwnerPage`. Only a name directly
/// after `(` (a tuple variant's payload) continues the path; anything else — a quote, a
/// digit, a brace — ends it, so no content is ever read.
pub fn names(debug: &str, levels: usize) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut rest = debug;
    while out.len() < levels {
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        let name = &rest[..end];
        // A name starts with a capital (a variant or a type) — never a value.
        if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_uppercase()) {
            break;
        }
        out.push(name);
        match rest[end..].strip_prefix('(') {
            Some(inner) => rest = inner,
            None => break,
        }
    }
    out.join(".")
}

/// An event's kind (law 13: names only). An Act names its surface and action, a Key its
/// key, a failure its op and class — all enum names.
pub fn event_kind(e: &Event) -> String {
    match e {
        // An editor window's number, or an editor op's, is a value: the names alone.
        Event::Act { surface, action, .. } => format!("Act.{}.{}", names(&format!("{surface:?}"), 1), names(&debug_head(action), 1)),
        Event::Key { surface, key } => format!("Key.{}.{key:?}", names(&format!("{surface:?}"), 1)),
        Event::Failed { op, class } => format!("Failed.{}.{class:?}", names(&format!("{op:?}"), 1)),
        Event::Edit(_, step) => format!("Edit.{}", names(&debug_head(step), 1)),
        Event::Field { field, .. } => format!("Field.{field:?}"),
        Event::SecretPasted { field, shape, .. } => format!("SecretPasted.{field:?}.{shape:?}"),
        other => names(&debug_head(other), 2),
    }
}

/// An effect's kind (law 13: names only).
pub fn effect_kind(e: &Effect) -> String {
    names(&debug_head(e), 2)
}

/// The laws the runner can check cheaply on every batch of a dev build (B93) — a subset of
/// the walkthroughs' own checks (`tests/common/laws.rs`), which hold the core to the same
/// at every step: what settled is all that was due (law 1), the Dock only while a window
/// of shroom's is open (law 6, SM:133), and nothing
/// new on screen, nothing that ends or moves the app, while a take stays open (law 4).
/// `None` when every one holds.
pub fn breach(take_was_open: bool, prev: &Scene, w: &World, next: &Scene, effects: &[Effect]) -> Option<&'static str> {
    if !crate::settle::settle(w).is_empty() {
        return Some("law 1: settle isn't idempotent");
    }
    if next.dock != (next.onboarding.is_some() || next.library.is_some() || next.settings.is_some() || next.feedback.is_some() || !next.editors.is_empty()) {
        return Some("law 6: the dock and the windows disagree");
    }
    if take_was_open && w.take_open() {
        let appeared = |now: bool, before: bool| now && !before;
        if appeared(next.onboarding.is_some(), prev.onboarding.is_some())
            || appeared(next.library.is_some(), prev.library.is_some())
            || appeared(next.settings.is_some(), prev.settings.is_some())
            || appeared(next.feedback.is_some(), prev.feedback.is_some())
            || next.editors.keys().any(|id| !prev.editors.contains_key(id))
        {
            return Some("law 4: a window appeared mid-take");
        }
        if next.popover.as_ref().is_some_and(|p| !["paused", "discard-confirm", "restart-confirm"].contains(&p.id)) {
            return Some("law 4: a popover appeared mid-take");
        }
        let ends = |e: &Effect| {
            matches!(
                e,
                Effect::Shell(ShellCmd::Quit) | Effect::Cloud(CloudCmd::OpenOwnerPage(_)) | Effect::Updater(UpdCmd::Apply(_)) | Effect::Persist(PersistOp::Wipe) | Effect::Member(MemberCmd::Join(_))
            )
        };
        if effects.iter().any(ends) {
            return Some("law 4: something that ends or moves the app ran mid-take");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{CloudNews, Field, OsNews, SecretField, Shape, Surface};
    use crate::scene::Action;
    use crate::world::{EditorId, RecId, SecretId};

    #[test]
    fn names_read_only_the_shape() {
        assert_eq!(names(r#"Cloud(OpenOwnerPage(RecId("AbCdEfGhIjKlMnOp")))"#, 2), "Cloud.OpenOwnerPage");
        assert_eq!(names("Os(NetPath(true))", 2), "Os.NetPath");
        assert_eq!(names(r#"Publish { dir: TakeDir("x"), rec: RecId("y") }"#, 2), "Publish");
        assert_eq!(names(r#"Updater(Download("1.2.0"))"#, 3), "Updater.Download");
        assert_eq!(names(r#"Thing("Secret")"#, 2), "Thing");
        assert_eq!(names("lower(Case)", 2), "");
        assert_eq!(names("Tick", 2), "Tick");
    }

    #[test]
    fn the_ring_is_bounded_and_keeps_the_newest() {
        let mut r = Ring::new(4);
        for i in 0..10u64 {
            r.step(i, &Event::Tick, &Arm::named("tick"), &[]);
        }
        assert_eq!(r.len(), 4);
        let monos: Vec<u64> = r.rows().map(|row| if let Row::Step { mono, .. } = row { *mono } else { 0 }).collect();
        assert_eq!(monos, [6, 7, 8, 9], "the oldest go first");
        assert_eq!(Ring::default().cap, RING_ROWS);
        assert_eq!(Ring::new(0).cap, 1, "never unbounded, never zero");
    }

    /// Law 13: nothing a person typed, no link's code, no secret's handle, no host or path
    /// reaches the ring — kinds and names only.
    #[test]
    fn the_ring_holds_kinds_and_names_only() {
        let rec = RecId("AbCdEfGhIjKlMnOp".into());
        let mut r = Ring::default();
        let events = [
            Event::Field { field: Field::Title, value: "Quarterly numbers cfat_secret".into() },
            Event::SecretPasted { field: SecretField::Token, id: SecretId(42), shape: Shape::Token },
            Event::Cloud(CloudNews::PageOpened(rec.clone())),
            Event::Act { surface: Surface::Library, action: Action::CopyRow(rec.clone()), rev: 7 },
            Event::Os(OsNews::WillSleep),
        ];
        let effects = [Effect::Cloud(CloudCmd::OpenOwnerPage(rec.clone())), Effect::Updater(UpdCmd::Download("9.9.9".into())), Effect::Shell(ShellCmd::AllowSleep)];
        for e in &events {
            r.step(1, e, &Arm { name: "an-arm", edge: Some(("recording", "mac-away")) }, &effects);
        }
        let all = r.lines().join("\n");
        for leak in ["AbCdEfGhIjKlMnOp", "Quarterly", "cfat_", "42", "9.9.9"] {
            assert!(!all.contains(leak), "{leak:?} leaked into the ring:\n{all}");
        }
        for kind in ["Field.Title", "SecretPasted.Token.Token", "Cloud.PageOpened", "Act.Library.CopyRow", "Os.WillSleep", "Cloud.OpenOwnerPage", "Updater.Download", "Shell.AllowSleep", "[recording → mac-away]"] {
            assert!(all.contains(kind), "{kind} missing:\n{all}");
        }
    }

    /// The editor's events and effects (N63) are kinds too: never the video's id, its
    /// title, or what a cut removed.
    #[test]
    fn the_editors_steps_reach_the_ring_as_kinds() {
        use crate::effect::EditCmd;
        use crate::event::{EditStep, EditorNews};
        let rec = RecId("AbCdEfGhIjKlMnOp".into());
        let mut list = shroom_edit::EditList::new(60.0);
        list.cuts.push(shroom_edit::Cut { removed: "Quarterly numbers, take two".into(), ..shroom_edit::Cut::manual(10.0, 20.0) });
        let saved = shroom_edit::Saved::new(list, 3);
        let mut r = Ring::default();
        let events = [
            Event::Editor(EditorNews::Opened { rec: rec.clone(), saved: Some(saved.clone()), markers: vec![12.5], duration_sec: Some(60.0), keep_until: None }),
            Event::Edit(EditorId(1), EditStep::Op(shroom_edit::Op::Cut { start: 1.0, end: 2.0 })),
            Event::Edit(EditorId(1), EditStep::Undo),
            Event::Act { surface: Surface::Library, action: Action::EditRow(rec.clone()), rev: 1 },
        ];
        let effects = [Effect::Editor(EditCmd::Save { id: EditorId(1), rec: rec.clone(), saved }), Effect::Editor(EditCmd::Copy { rec: rec.clone(), rev: 3 })];
        for e in &events {
            r.step(1, e, &Arm { name: "an-arm", edge: Some(("editor", "editor-saving")) }, &effects);
        }
        let all = r.lines().join("\n");
        for leak in ["AbCdEfGhIjKlMnOp", "Quarterly", "12.5"] {
            assert!(!all.contains(leak), "{leak:?} leaked into the ring:\n{all}");
        }
        for kind in ["Editor.Opened", "Edit.Op", "Edit.Undo", "Act.Library.EditRow", "Editor.Save", "Editor.Copy"] {
            assert!(all.contains(kind), "{kind} missing:\n{all}");
        }
    }

    #[test]
    fn a_big_value_is_never_formatted_whole() {
        let big = Event::Field { field: Field::LibrarySearch, value: "x".repeat(1_000_000) };
        assert!(debug_head(&big).len() <= 160);
        assert_eq!(event_kind(&big), "Field.LibrarySearch");
    }
}
