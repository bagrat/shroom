//! The editor's operations and their undo history (N63).
//!
//! An operation either changes what plays or is refused ([`Refusal`]): out of range, too
//! short a cut, nothing that plays changes, or it would leave less than [`MIN_KEEP`]
//! playing. The history holds whole lists: they are small, and Undo then can't drift from
//! what was on screen.

use crate::{Cut, CutKind, EditList, MIN_HAND_CUT, MIN_KEEP};
use serde::{Deserialize, Serialize};

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
    /// Put back the cut at `index` (its place in the list the editor shows, which only the
    /// editor changes while it's open): a hand cut goes; a suggested one is unapplied, so it
    /// can be applied again.
    PutBack { index: usize },
    /// Apply the suggestion at `index` (its chip: Cut <its length>, as a selection's).
    Apply { index: usize },
    /// Move the edges of the cut at `index` (B195 (g): one of them dragged on the timeline):
    /// it becomes `start`–`end`, held inside the ends. A hand cut or the voice layer's, taken
    /// or offered: it keeps what it was, marked `adjusted`.
    Resize { index: usize, start: f64, end: f64 },
    /// Every cut put back and both ends: the recording as it was. Suggestions stay offered.
    Revert,
}

/// Why an operation was refused. The editor snaps back or greys; the CLI says which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    /// A time that isn't a number, a part backwards, or a part wholly outside what plays
    /// between the ends.
    OutOfRange,
    /// It would leave less than [`MIN_KEEP`] playing.
    LeavesTooLittle,
    /// A hand cut shorter than [`MIN_HAND_CUT`].
    TooShort,
    /// Nothing that plays would change: a part already cut, an end moved within a cut.
    NothingChanges,
    /// No cut at that index in the state the op needs: put back one that's cut, apply a
    /// suggestion that isn't.
    NoSuchCut,
}

/// What the list becomes under `op`, or why it's refused. An op that changes the list but
/// not what plays (a cut inside a cut, an end moved within one) is refused too: it would
/// leave Done something to save, and a final video to make, that no viewer could tell.
pub fn check(list: &EditList, op: &Op) -> Result<EditList, Refusal> {
    let d = list.source_duration;
    let mut next = list.clone();
    match *op {
        Op::Trim { in_, out } => {
            if !in_.is_finite() || !out.is_finite() {
                return Err(Refusal::OutOfRange);
            }
            next.in_ = in_.clamp(0.0, d);
            next.out = out.clamp(0.0, d);
            if next.out - next.in_ < MIN_KEEP.min(d) {
                return Err(Refusal::LeavesTooLittle);
            }
        }
        Op::Cut { start, end } => {
            if !start.is_finite() || !end.is_finite() || end < start || end <= next.in_ || start >= next.out {
                return Err(Refusal::OutOfRange);
            }
            let (s, e) = (start.max(next.in_).max(0.0), end.min(next.out).min(d));
            if e - s < MIN_HAND_CUT {
                return Err(Refusal::TooShort);
            }
            let id = next.next_manual_id();
            next.cuts.push(Cut {
                id,
                ..Cut::manual(s, e)
            });
        }
        Op::PutBack { index } => {
            let c = next.cuts.get(index).filter(|c| c.applied).ok_or(Refusal::NoSuchCut)?;
            if c.kind == CutKind::Manual {
                next.cuts.remove(index);
            } else {
                next.cuts[index].applied = false;
            }
        }
        Op::Apply { index } => {
            let c = next.cuts.get_mut(index).filter(|c| !c.applied).ok_or(Refusal::NoSuchCut)?;
            c.applied = true;
        }
        Op::Resize { index, start, end } => {
            if !start.is_finite() || !end.is_finite() || end < start {
                return Err(Refusal::OutOfRange);
            }
            let (s, e) = (start.max(next.in_).max(0.0), end.min(next.out).min(d));
            let c = next.cuts.get_mut(index).ok_or(Refusal::NoSuchCut)?;
            if e - s < MIN_HAND_CUT {
                return Err(Refusal::TooShort);
            }
            if (c.start, c.end) == (s, e) {
                return Err(Refusal::NothingChanges);
            }
            (c.start, c.end, c.adjusted) = (s, e, true);
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
    // A suggestion moved plays nothing new, and is still a change: what its Cut chip would take.
    let offered_moved = matches!(*op, Op::Resize { index, .. } if !list.cuts[index].applied);
    if next == *list || (!offered_moved && next.keep() == list.keep()) {
        return Err(Refusal::NothingChanges);
    }
    if next.edited_duration() < MIN_KEEP.min(d) {
        return Err(Refusal::LeavesTooLittle);
    }
    Ok(next)
}

/// What the list becomes under `op`, or `None` when it's refused ([`check`] says why).
pub fn apply(list: &EditList, op: &Op) -> Option<EditList> {
    check(list, op).ok()
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
        History {
            now: saved.clone(),
            saved,
            past: Vec::new(),
            future: Vec::new(),
        }
    }

    /// Applies `op`; `false` when it was refused or changed nothing (no step is kept).
    pub fn act(&mut self, op: &Op) -> bool {
        let Some(next) = apply(&self.now, op) else {
            return false;
        };
        self.past.push(std::mem::replace(&mut self.now, next));
        if self.past.len() > UNDO_DEPTH {
            self.past.remove(0);
        }
        self.future.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.past.pop() else {
            return false;
        };
        self.future.push(std::mem::replace(&mut self.now, prev));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.future.pop() else {
            return false;
        };
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
        assert!(h.act(&Op::Trim {
            in_: 2.0,
            out: 50.0
        }));
        assert!(h.act(&Op::Cut {
            start: 10.0,
            end: 22.0
        }));
        assert_eq!(h.now.keep(), vec![(2.0, 10.0), (22.0, 50.0)]);
        assert_eq!(h.now.cuts[0].id, "m1");
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
        assert!(h.act(&Op::Cut {
            start: 1.0,
            end: 2.0
        }));
        assert!(!h.can_redo(), "a new step drops what Redo held");
    }

    #[test]
    fn refused_ops_keep_no_step() {
        let mut h = History::new(list());
        assert!(
            !h.act(&Op::Trim {
                in_: 30.0,
                out: 30.5
            }),
            "less than a second left"
        );
        assert!(
            !h.act(&Op::Trim {
                in_: 0.0,
                out: 60.0
            }),
            "nothing changes"
        );
        assert!(!h.act(&Op::Trim {
            in_: f64::NAN,
            out: 10.0
        }));
        assert!(
            !h.act(&Op::Cut {
                start: 5.0,
                end: 5.05
            }),
            "too short to be a hand cut"
        );
        assert!(
            !h.act(&Op::Cut {
                start: 0.0,
                end: 59.5
            }),
            "would leave less than a second"
        );
        assert!(!h.act(&Op::PutBack { index: 3 }));
        assert!(!h.act(&Op::Revert));
        assert!(!h.can_undo());
    }

    #[test]
    fn what_changes_nothing_that_plays_is_refused() {
        let mut h = History::new(list());
        assert!(h.act(&Op::Cut { start: 10.0, end: 22.0 }));
        h.saved_as_now();
        let cut = |start, end| Op::Cut { start, end };
        assert_eq!(check(&h.now, &cut(12.0, 15.0)), Err(Refusal::NothingChanges), "a part already cut");
        assert!(!h.act(&cut(12.0, 15.0)) && !h.dirty());
        assert!(h.act(&Op::Trim { in_: 0.0, out: 30.0 }));
        h.saved_as_now();
        let (cuts, keep) = (h.now.cuts.clone(), h.now.keep());
        assert_eq!(check(&h.now, &cut(25.0, 31.0)).map(|l| l.keep()), Ok(vec![(0.0, 10.0), (22.0, 25.0)]), "held to the end");
        let mut l = EditList::new(60.0);
        l.cuts.push(Cut::manual(0.0, 5.0));
        assert_eq!(check(&l, &Op::Trim { in_: 3.0, out: 60.0 }), Err(Refusal::NothingChanges), "an end moved within a cut");
        assert_eq!(check(&l, &Op::Trim { in_: 6.0, out: 60.0 }).map(|l| l.keep()), Ok(vec![(6.0, 60.0)]));
        let mut inner = l.clone();
        inner.cuts.push(Cut { id: "m2".into(), ..Cut::manual(1.0, 2.0) });
        assert_eq!(check(&inner, &Op::PutBack { index: 1 }), Err(Refusal::NothingChanges), "a cut inside a cut put back");
        assert_eq!((h.now.cuts.clone(), h.now.keep()), (cuts, keep));
    }

    #[test]
    fn a_refusal_says_why() {
        let l = list();
        let cut = |start, end| Op::Cut { start, end };
        assert_eq!(check(&l, &cut(f64::NAN, 3.0)), Err(Refusal::OutOfRange));
        assert_eq!(check(&l, &cut(9.0, 3.0)), Err(Refusal::OutOfRange), "backwards");
        assert_eq!(check(&l, &cut(61.0, 70.0)), Err(Refusal::OutOfRange), "past the end");
        assert_eq!(check(&l, &cut(5.0, 5.05)), Err(Refusal::TooShort));
        assert_eq!(check(&l, &cut(0.0, 59.5)), Err(Refusal::LeavesTooLittle));
        assert_eq!(check(&l, &Op::Trim { in_: 30.0, out: 30.5 }), Err(Refusal::LeavesTooLittle));
        assert_eq!(check(&l, &Op::Trim { in_: 0.0, out: 60.0 }), Err(Refusal::NothingChanges));
        assert_eq!(check(&l, &Op::PutBack { index: 0 }), Err(Refusal::NoSuchCut));
        assert_eq!(check(&l, &Op::Apply { index: 0 }), Err(Refusal::NoSuchCut));
        assert_eq!(check(&l, &Op::Revert), Err(Refusal::NothingChanges));
        assert_eq!(serde_json::to_string(&Refusal::LeavesTooLittle).unwrap(), r#""leaves-too-little""#);
        let trimmed = check(&l, &Op::Trim { in_: 10.0, out: 50.0 }).unwrap();
        assert_eq!(check(&trimmed, &cut(2.0, 8.0)), Err(Refusal::OutOfRange), "wholly before the trim's in");
        assert_eq!(check(&trimmed, &cut(8.0, 12.0)).map(|l| l.keep()), Ok(vec![(12.0, 50.0)]), "held to the in");
    }

    #[test]
    fn a_cuts_edges_move_and_it_keeps_what_it_was() {
        let mut l = list();
        l.cuts.push(Cut { id: "m1".into(), ..Cut::manual(20.0, 28.0) });
        l.cuts.push(Cut { kind: CutKind::Retake, applied: false, removed: "so um".into(), ..Cut::manual(40.0, 44.0) });
        let resize = |index, start, end| Op::Resize { index, start, end };
        let moved = check(&l, &resize(0, 20.0, 30.4)).unwrap();
        assert_eq!(moved.keep(), vec![(0.0, 20.0), (30.4, 60.0)]);
        assert_eq!((moved.cuts[0].id.as_str(), moved.cuts[0].kind, moved.cuts[0].applied, moved.cuts[0].adjusted), ("m1", CutKind::Manual, true, true));
        let offered = check(&l, &resize(1, 42.5, 44.0)).unwrap();
        assert_eq!(offered.keep(), l.keep(), "a suggestion moved plays nothing new");
        assert_eq!((offered.cuts[1].start, offered.cuts[1].applied, offered.cuts[1].removed.as_str(), offered.cuts[1].adjusted), (42.5, false, "so um", true));
        assert_eq!(check(&l, &resize(0, 20.0, 20.05)), Err(Refusal::TooShort));
        assert_eq!(check(&l, &resize(0, 20.0, 28.0)), Err(Refusal::NothingChanges));
        assert_eq!(check(&l, &resize(5, 1.0, 2.0)), Err(Refusal::NoSuchCut));
        assert_eq!(check(&l, &resize(0, 30.0, 25.0)), Err(Refusal::OutOfRange), "backwards");
        assert_eq!(check(&l, &resize(0, 0.5, 59.8)), Err(Refusal::LeavesTooLittle));
        let trimmed = check(&l, &Op::Trim { in_: 10.0, out: 50.0 }).unwrap();
        assert_eq!(check(&trimmed, &resize(0, 5.0, 28.0)).map(|l| l.cuts[0].start), Ok(10.0), "held to the trim");
        let ops: Vec<Op> = serde_json::from_str(r#"[{"op":"resize","index":1,"start":2.5,"end":3}]"#).unwrap();
        assert_eq!(ops[0], resize(1, 2.5, 3.0));
    }

    #[test]
    fn a_cut_is_held_inside_the_trim_and_the_recording() {
        let mut h = History::new(list());
        h.act(&Op::Trim {
            in_: 5.0,
            out: 55.0,
        });
        assert!(h.act(&Op::Cut {
            start: -4.0,
            end: 8.0
        }));
        assert_eq!((h.now.cuts[0].start, h.now.cuts[0].end), (5.0, 8.0));
    }

    #[test]
    fn suggestions_are_unapplied_not_removed_and_revert_keeps_them_offered() {
        let mut l = list();
        l.cuts.push(Cut {
            kind: CutKind::Retake,
            ..Cut::manual(3.0, 6.0)
        });
        l.cuts.push(Cut {
            kind: CutKind::LineBeforeCue,
            applied: false,
            ..Cut::manual(20.0, 24.0)
        });
        let mut h = History::new(l);
        assert!(h.act(&Op::PutBack { index: 0 }));
        assert_eq!(h.now.cuts.len(), 2);
        assert!(!h.now.cuts[0].applied);
        assert!(h.act(&Op::Apply { index: 1 }));
        assert!(h.act(&Op::Cut {
            start: 40.0,
            end: 41.0
        }));
        assert!(h.act(&Op::Cut {
            start: 44.0,
            end: 45.0
        }));
        assert_eq!(
            h.now.cuts.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec!["", "", "m1", "m2"]
        );
        assert!(h.act(&Op::Revert));
        assert_eq!(h.now.cuts.len(), 2);
        assert!(h.now.cuts.iter().all(|c| !c.applied));
        assert!(h.now.is_identity());
    }

    #[test]
    fn saving_moves_the_mark_and_keeps_the_steps() {
        let mut h = History::new(list());
        h.act(&Op::Cut {
            start: 10.0,
            end: 12.0,
        });
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
            h.act(&Op::Trim {
                in_: t * 0.01,
                out: 59.0 - (i % 2) as f64,
            });
        }
        let mut n = 0;
        while h.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_DEPTH);
    }
}
