//! The editor's operations and their undo history (N63).
//!
//! An operation either changes the list or is refused (`None`): out of range, nothing to
//! change, or it would leave less than [`MIN_KEEP`] playing. The history holds whole lists:
//! they are small, and Undo then can't drift from what was on screen.

use crate::{Cut, CutKind, EditList, MIN_HAND_CUT, MIN_KEEP};
use serde::Deserialize;

/// One change asked for in the editor. Times are seconds on the source timeline.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Op {
    /// The yellow ends.
    Trim {
        #[serde(rename = "in")]
        in_: f64,
        out: f64,
    },
    /// A part selected on the timeline, cut.
    Cut { start: f64, end: f64 },
    /// Put back the cut at `index`: a hand cut goes; a suggested one is unapplied, so it
    /// can be applied again.
    PutBack { index: usize },
    /// Apply the suggestion at `index` (the voice-cut layer's “Cut it too”).
    Apply { index: usize },
    /// Every cut put back and both ends: the recording as it was. Suggestions stay offered.
    Revert,
}

/// What the list becomes under `op`, or `None` when it's refused or changes nothing.
pub fn apply(list: &EditList, op: &Op) -> Option<EditList> {
    let d = list.source_duration;
    let mut next = list.clone();
    match *op {
        Op::Trim { in_, out } => {
            if !in_.is_finite() || !out.is_finite() {
                return None;
            }
            next.in_ = in_.clamp(0.0, d);
            next.out = out.clamp(0.0, d);
            if next.out - next.in_ < MIN_KEEP.min(d) {
                return None;
            }
        }
        Op::Cut { start, end } => {
            if !start.is_finite() || !end.is_finite() {
                return None;
            }
            let (s, e) = (start.max(next.in_).max(0.0), end.min(next.out).min(d));
            if e - s < MIN_HAND_CUT {
                return None;
            }
            next.cuts.push(Cut::manual(s, e));
        }
        Op::PutBack { index } => {
            let c = next.cuts.get(index).filter(|c| c.applied)?;
            if c.kind == CutKind::Manual {
                next.cuts.remove(index);
            } else {
                next.cuts[index].applied = false;
            }
        }
        Op::Apply { index } => {
            let c = next.cuts.get_mut(index).filter(|c| !c.applied)?;
            c.applied = true;
        }
        Op::Revert => {
            next.in_ = 0.0;
            next.out = d;
            next.cuts.retain(|c| c.kind != CutKind::Manual);
            for c in &mut next.cuts {
                c.applied = false;
            }
        }
    }
    if next == *list || next.edited_duration() < MIN_KEEP.min(d) {
        return None;
    }
    Some(next)
}

/// How many steps Undo reaches back.
pub const UNDO_DEPTH: usize = 200;

/// The list being edited, with every step since the editor opened.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    /// What the record holds: the list as it was opened or last saved.
    pub saved: EditList,
    pub now: EditList,
    past: Vec<EditList>,
    future: Vec<EditList>,
}

impl History {
    pub fn new(saved: EditList) -> History {
        let saved = saved.checked();
        History { now: saved.clone(), saved, past: Vec::new(), future: Vec::new() }
    }

    /// Applies `op`; `false` when it was refused or changed nothing (no step is kept).
    pub fn act(&mut self, op: &Op) -> bool {
        let Some(next) = apply(&self.now, op) else { return false };
        self.past.push(std::mem::replace(&mut self.now, next));
        if self.past.len() > UNDO_DEPTH {
            self.past.remove(0);
        }
        self.future.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.past.pop() else { return false };
        self.future.push(std::mem::replace(&mut self.now, prev));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.future.pop() else { return false };
        self.past.push(std::mem::replace(&mut self.now, next));
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Something differs from what the record holds: Done has something to save.
    pub fn dirty(&self) -> bool {
        self.now != self.saved
    }

    /// Revert to original has something to put back.
    pub fn can_revert(&self) -> bool {
        apply(&self.now, &Op::Revert).is_some()
    }

    /// The save landed: what's on screen is what the record holds. The steps stay, so a
    /// later Undo still reaches them.
    pub fn saved_as_now(&mut self) {
        self.saved = self.now.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> EditList {
        EditList::new(60.0)
    }

    #[test]
    fn trim_cut_put_back_undo_redo() {
        let mut h = History::new(list());
        assert!(!h.dirty() && !h.can_undo() && !h.can_revert());
        assert!(h.act(&Op::Trim { in_: 2.0, out: 50.0 }));
        assert!(h.act(&Op::Cut { start: 10.0, end: 22.0 }));
        assert_eq!(h.now.keep(), vec![(2.0, 10.0), (22.0, 50.0)]);
        assert!(h.dirty() && h.can_revert());
        assert!(h.act(&Op::PutBack { index: 0 }));
        assert!(h.now.cuts.is_empty(), "a hand cut put back is gone");
        assert!(h.undo());
        assert_eq!(h.now.cuts.len(), 1);
        assert!(h.redo());
        assert!(h.now.cuts.is_empty());
        assert!(!h.redo());
        assert!(h.undo() && h.undo() && h.undo());
        assert!(!h.dirty() && !h.undo());
        assert!(h.can_redo());
        assert!(h.act(&Op::Cut { start: 1.0, end: 2.0 }));
        assert!(!h.can_redo(), "a new step drops what Redo held");
    }

    #[test]
    fn refused_ops_keep_no_step() {
        let mut h = History::new(list());
        assert!(!h.act(&Op::Trim { in_: 30.0, out: 30.5 }), "less than a second left");
        assert!(!h.act(&Op::Trim { in_: 0.0, out: 60.0 }), "nothing changes");
        assert!(!h.act(&Op::Trim { in_: f64::NAN, out: 10.0 }));
        assert!(!h.act(&Op::Cut { start: 5.0, end: 5.05 }), "too short to be a hand cut");
        assert!(!h.act(&Op::Cut { start: 0.0, end: 59.5 }), "would leave less than a second");
        assert!(!h.act(&Op::PutBack { index: 3 }));
        assert!(!h.act(&Op::Revert));
        assert!(!h.can_undo());
    }

    #[test]
    fn a_cut_is_held_inside_the_trim_and_the_recording() {
        let mut h = History::new(list());
        h.act(&Op::Trim { in_: 5.0, out: 55.0 });
        assert!(h.act(&Op::Cut { start: -4.0, end: 8.0 }));
        assert_eq!((h.now.cuts[0].start, h.now.cuts[0].end), (5.0, 8.0));
    }

    #[test]
    fn suggestions_are_unapplied_not_removed_and_revert_keeps_them_offered() {
        let mut l = list();
        l.cuts.push(Cut { kind: CutKind::Retake, ..Cut::manual(3.0, 6.0) });
        l.cuts.push(Cut { kind: CutKind::LineBeforeCue, applied: false, ..Cut::manual(20.0, 24.0) });
        let mut h = History::new(l);
        assert!(h.act(&Op::PutBack { index: 0 }));
        assert_eq!(h.now.cuts.len(), 2);
        assert!(!h.now.cuts[0].applied);
        assert!(h.act(&Op::Apply { index: 1 }));
        assert!(h.act(&Op::Cut { start: 40.0, end: 41.0 }));
        assert!(h.act(&Op::Revert));
        assert_eq!(h.now.cuts.len(), 2);
        assert!(h.now.cuts.iter().all(|c| !c.applied));
        assert!(h.now.is_identity());
    }

    #[test]
    fn saving_moves_the_mark_and_keeps_the_steps() {
        let mut h = History::new(list());
        h.act(&Op::Cut { start: 10.0, end: 12.0 });
        h.saved_as_now();
        assert!(!h.dirty() && h.can_undo());
        h.undo();
        assert!(h.dirty());
    }

    #[test]
    fn ops_read_as_the_editor_sends_them() {
        let ops: Vec<Op> = serde_json::from_str(r#"[{"op":"trim","in":1,"out":9},{"op":"cut","start":2,"end":3},{"op":"put-back","index":0},{"op":"apply","index":1},{"op":"revert"}]"#).unwrap();
        assert_eq!(ops[0], Op::Trim { in_: 1.0, out: 9.0 });
        assert_eq!(ops[2], Op::PutBack { index: 0 });
        assert_eq!(ops[4], Op::Revert);
    }

    #[test]
    fn undo_reaches_back_a_bounded_way() {
        let mut h = History::new(list());
        for i in 0..(UNDO_DEPTH + 10) {
            let t = 1.0 + (i % 50) as f64;
            h.act(&Op::Trim { in_: t * 0.01, out: 59.0 - (i % 2) as f64 });
        }
        let mut n = 0;
        while h.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_DEPTH);
    }
}
