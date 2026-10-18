//! Editing (N63, DECISIONS B195).
//!
//! - **The edit list** ([`EditList`]): a trim and cuts over a recording's own timeline
//!   (seconds on the joined takes, the clock `markers.json` and `meta.markers` use). It
//!   never touches the media: the editor's preview, the share page and the embed apply it as
//!   they play, and the Mac's permanent copy is cut from it. Every cut is a suggestion until
//!   it's `applied`. The shape is the voice-cut proposal's (dev/proposals/voice-cut.md), so
//!   its planner adds suggestions to the same list.
//! - **What's saved** ([`Saved`]): the list as it sits in the record (`meta.edits`), with
//!   the revision a save bumps and the permanent copy cut from it, once there is one.
//! - **Editing** ([`edit`]): the editor's operations and their undo history.
//! - **The voice-cue planner** ([`plan`], N65): given a take's words with their times, find
//!   the spoken cue ("shroom cut"), find where the speaker started again the line it retakes,
//!   and cut from that restart through the end of the cue, the edges set in the quiet on
//!   either side.
//!
//! Deterministic and pure: no media, no clock, no I/O.

mod align;
mod cue;
pub mod edit;
mod judge;
mod land;
mod plan;
mod text;

pub use cue::{Cue, find_cues};
pub use edit::{History, Op};
pub use judge::{Answer, JUDGED, Question};
pub use land::{land, land_with};
pub use plan::{Options, plan, plan_with, questions};

use serde::{Deserialize, Serialize};

/// One transcribed word, in seconds on the source timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// A stretch of silence found in the audio (seconds), used to snap cut edges.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Silence {
    pub start: f64,
    pub end: f64,
}

/// The planner's input: a transcript and, when known, the silences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub duration: f64,
    pub words: Vec<Word>,
    #[serde(default)]
    pub silences: Vec<Silence>,
}

impl Transcript {
    /// A take's words as the space keeps what it heard (N66: `[text, start, end]` in seconds
    /// on the recording's clock), with the quiet found in the take's own sound. Blank or
    /// timeless words go; the rest in time order, none ending before it starts. A length
    /// short of the last word is the last word's end.
    pub fn heard(
        duration: f64,
        words: &[(String, f64, f64)],
        silences: Vec<Silence>,
    ) -> Transcript {
        let mut words: Vec<Word> = words
            .iter()
            .filter(|(text, start, end)| {
                !text.trim().is_empty() && start.is_finite() && end.is_finite()
            })
            .map(|(text, start, end)| Word {
                text: text.trim().to_string(),
                start: *start,
                end: end.max(*start),
            })
            .collect();
        words.sort_by(|a, b| a.start.total_cmp(&b.start));
        let last = words.iter().map(|w| w.end).fold(0.0, f64::max);
        let duration = if duration.is_finite() && duration >= last {
            duration
        } else {
            last
        };
        Transcript {
            duration,
            words,
            silences,
        }
    }
}

/// Where a cut came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CutKind {
    /// The spoken cue and the take it retakes.
    Retake,
    /// The cue alone: no retake was found after it.
    CueOnly,
    /// The line before a cue that has no retake: the likely flub, offered unapplied.
    LineBeforeCue,
    /// Drawn by hand on the timeline.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Low,
    High,
}

/// One cut: `[start, end)` on the source timeline is left out when applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    /// Short and unique within the list: `m1`, `m2`… by hand, `v1`… from the voice cues. A
    /// writer that flips one cut, or merges a list another writer changed, names it by this,
    /// never by its place. Empty in lists written before ids.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub kind: CutKind,
    pub confidence: Confidence,
    pub applied: bool,
    /// What plays right before the cut (for a review list). Never sent to viewers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leads: Option<String>,
    /// What the cut removes, as transcribed (for a review list). Never sent to viewers.
    #[serde(default)]
    pub removed: String,
    /// What plays right after the cut (for a review list). Never sent to viewers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumes: Option<String>,
    /// Why a planner suggested it, in a few words (debugging, not product copy).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
}

impl Cut {
    /// A cut drawn by hand: applied at once.
    pub fn manual(start: f64, end: f64) -> Cut {
        Cut {
            id: String::new(),
            start,
            end,
            kind: CutKind::Manual,
            confidence: Confidence::High,
            applied: true,
            leads: None,
            removed: String::new(),
            resumes: None,
            why: String::new(),
        }
    }

    pub fn length(&self) -> f64 {
        self.end - self.start
    }
}

/// The edit list for one recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditList {
    pub version: u32,
    pub source_duration: f64,
    /// Trim: what plays is `[in, out)` of the source, minus applied cuts.
    #[serde(rename = "in")]
    pub in_: f64,
    pub out: f64,
    pub cuts: Vec<Cut>,
    /// The voice cues were planned once (spec N65): a later plan never brings back a cut
    /// someone took out. The editor carries it through untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
}

/// When the take's spoken cues were planned, and how many there were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    pub planned_at: u64,
    pub cues: u32,
}

/// Nothing shorter plays: a trim or a cut that would leave less is refused (seconds).
pub const MIN_KEEP: f64 = 1.0;
/// A cut drawn by hand is at least this long (seconds); a planner's cuts are never held
/// to it, since a lone spoken cue can be shorter than a hand can draw.
pub const MIN_HAND_CUT: f64 = 0.1;

impl EditList {
    pub fn new(source_duration: f64) -> Self {
        let d = if source_duration.is_finite() && source_duration > 0.0 {
            source_duration
        } else {
            0.0
        };
        Self {
            version: 1,
            source_duration: d,
            in_: 0.0,
            out: d,
            cuts: Vec::new(),
            voice: None,
        }
    }

    /// The next free hand-cut id: `m1`, `m2`…
    pub fn next_manual_id(&self) -> String {
        let n = self
            .cuts
            .iter()
            .filter_map(|c| c.id.strip_prefix('m')?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("m{}", n + 1)
    }

    /// The list made safe to play and to edit, whatever it was read from: times clamped to
    /// the recording (a cut touching either end is normal), a cut that isn't finite or has
    /// no length dropped, the trim kept in order and at least [`MIN_KEEP`] long where the
    /// recording allows. Overlapping cuts are kept: [`EditList::keep`] merges them.
    pub fn checked(mut self) -> Self {
        let d = if self.source_duration.is_finite() && self.source_duration > 0.0 {
            self.source_duration
        } else {
            0.0
        };
        self.source_duration = d;
        self.version = 1;
        let clamp = |t: f64| {
            if t.is_finite() {
                t.clamp(0.0, d)
            } else {
                f64::NAN
            }
        };
        let (mut a, mut b) = (clamp(self.in_), clamp(self.out));
        if a.is_nan() {
            a = 0.0;
        }
        if b.is_nan() {
            b = d;
        }
        if b - a < MIN_KEEP.min(d) {
            (a, b) = (0.0, d);
        }
        self.in_ = a;
        self.out = b;
        self.cuts.retain_mut(|c| {
            let (s, e) = (clamp(c.start), clamp(c.end));
            if s.is_nan() || e.is_nan() || e <= s {
                return false;
            }
            c.start = s;
            c.end = e;
            true
        });
        self
    }

    /// The applied cuts, inside the trim, sorted and merged.
    fn applied(&self) -> Vec<(f64, f64)> {
        let mut cuts: Vec<(f64, f64)> = self
            .cuts
            .iter()
            .filter(|c| c.applied)
            .map(|c| (c.start.max(self.in_), c.end.min(self.out)))
            .filter(|(s, e)| e > s)
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        cuts
    }

    /// The source ranges that play, in order: trim minus applied cuts, overlaps merged.
    pub fn keep(&self) -> Vec<(f64, f64)> {
        let mut keep = Vec::new();
        let mut at = self.in_;
        for (s, e) in self.applied() {
            if e <= at {
                continue;
            }
            if s > at {
                keep.push((at, s));
            }
            at = at.max(e);
        }
        if at < self.out {
            keep.push((at, self.out));
        }
        keep
    }

    /// How long the edited video plays.
    pub fn edited_duration(&self) -> f64 {
        self.keep().iter().map(|(s, e)| e - s).sum()
    }

    /// Maps a source time to edited time; `None` when it falls in a cut or outside the trim.
    pub fn to_edited(&self, t: f64) -> Option<f64> {
        let mut acc = 0.0;
        for (s, e) in self.keep() {
            if t >= s && t < e {
                return Some(acc + (t - s));
            }
            acc += e - s;
        }
        None
    }

    /// Maps an edited time to the source: the edited timeline's `e` seconds in. Past the
    /// end it is the trim's out; before the start, the trim's in.
    pub fn to_source(&self, e: f64) -> f64 {
        let mut acc = 0.0;
        let keep = self.keep();
        for (s, end) in &keep {
            let len = end - s;
            if e < acc + len {
                return s + (e - acc).max(0.0);
            }
            acc += len;
        }
        keep.last().map_or(self.out, |k| k.1)
    }

    /// Nothing is left out: the whole recording plays, as recorded.
    pub fn is_identity(&self) -> bool {
        let keep = self.keep();
        keep.len() == 1 && keep[0].0 <= 0.0 && keep[0].1 >= self.source_duration
    }
}

/// The permanent copy of a saved edit (B195 (f)): `rec/<id>/<file>`, cut by the Mac from
/// the original. Current only while its `rev` is the saved list's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditCopy {
    pub rev: u64,
    /// The file's name beside the recording, `edit-<rev>.mp4`.
    pub file: String,
    /// Its length in seconds.
    pub duration: f64,
    /// Its own first frame and 15-second teaser beside it (`edit-<rev>-poster.jpg`,
    /// `edit-<rev>-teaser.mp4`), so a trimmed-off start never shows in an unfurl.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaser: Option<String>,
}

impl EditCopy {
    pub fn file_for(rev: u64) -> String {
        format!("edit-{rev}.mp4")
    }

    pub fn poster_for(rev: u64) -> String {
        format!("edit-{rev}-poster.jpg")
    }

    pub fn teaser_for(rev: u64) -> String {
        format!("edit-{rev}-teaser.mp4")
    }

    /// The copy's files, each a name beside the recording (`rec/<id>/<name>`).
    pub fn files(&self) -> Vec<&str> {
        std::iter::once(self.file.as_str())
            .chain(self.poster.as_deref())
            .chain(self.teaser.as_deref())
            .collect()
    }
}

/// The edit as the record keeps it (`meta.edits`): the list, the revision each save bumps,
/// and the permanent copy once one is up. Fields a reader doesn't know are kept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    #[serde(flatten)]
    pub list: EditList,
    #[serde(default)]
    pub rev: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<EditCopy>,
    /// While the original is kept beside the copy, when it goes (B197): RFC 3339, UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_until: Option<String>,
    /// Fields this build doesn't know, kept through every rewrite.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Saved {
    /// A list saved as `rev`, with no copy yet.
    pub fn new(list: EditList, rev: u64) -> Saved {
        Saved { list, rev, copy: None, keep_until: None, extra: serde_json::Map::new() }
    }

    /// The copy, when it was cut from this very revision.
    pub fn current_copy(&self) -> Option<&EditCopy> {
        self.copy.as_ref().filter(|c| c.rev == self.rev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(start: f64, end: f64, applied: bool) -> Cut {
        Cut {
            applied,
            kind: CutKind::Retake,
            ..Cut::manual(start, end)
        }
    }

    #[test]
    fn keep_is_the_trim_minus_applied_cuts() {
        let mut l = EditList::new(10.0);
        l.in_ = 1.0;
        l.out = 9.0;
        l.cuts = vec![
            cut(2.0, 3.0, true),
            cut(2.5, 4.0, true),
            cut(5.0, 6.0, false),
            cut(8.5, 12.0, true),
        ];
        assert_eq!(l.keep(), vec![(1.0, 2.0), (4.0, 8.5)]);
        assert_eq!(l.edited_duration(), 5.5);
        assert_eq!(l.to_edited(4.5), Some(1.5));
        assert_eq!(l.to_edited(2.7), None);
    }

    #[test]
    fn undo_is_unapplying() {
        let mut l = EditList::new(10.0);
        l.cuts = vec![cut(2.0, 3.0, true)];
        l.cuts[0].applied = false;
        assert_eq!(l.keep(), vec![(0.0, 10.0)]);
        assert!(l.is_identity());
    }

    #[test]
    fn to_source_walks_the_kept_ranges() {
        let mut l = EditList::new(10.0);
        l.in_ = 1.0;
        l.cuts = vec![cut(3.0, 5.0, true)];
        assert_eq!(l.to_source(0.0), 1.0);
        assert_eq!(l.to_source(1.5), 2.5);
        assert_eq!(l.to_source(2.0), 5.0);
        assert_eq!(l.to_source(3.0), 6.0);
        assert_eq!(l.to_source(99.0), 10.0);
        assert_eq!(l.to_source(-1.0), 1.0);
        for e in [0.0, 0.5, 1.99, 2.0, 4.5, 6.9] {
            assert!(
                (l.to_edited(l.to_source(e)).unwrap() - e).abs() < 1e-9,
                "{e}"
            );
        }
    }

    #[test]
    fn checked_clamps_and_never_rejects_an_end_or_an_overlap() {
        let mut l = EditList::new(10.0);
        l.in_ = -3.0;
        l.out = 42.0;
        l.cuts = vec![
            cut(-1.0, 0.8, true),
            cut(9.5, 11.0, true),
            cut(4.0, 5.0, true),
            cut(4.6, 5.4, true),
            cut(f64::NAN, 2.0, true),
            cut(6.0, 6.0, true),
        ];
        let l = l.checked();
        assert_eq!((l.in_, l.out), (0.0, 10.0));
        assert_eq!(
            l.cuts.iter().map(|c| (c.start, c.end)).collect::<Vec<_>>(),
            vec![(0.0, 0.8), (9.5, 10.0), (4.0, 5.0), (4.6, 5.4)]
        );
        assert_eq!(l.keep(), vec![(0.8, 4.0), (5.4, 9.5)]);
    }

    #[test]
    fn checked_puts_a_trim_back_that_leaves_nothing() {
        let mut l = EditList::new(10.0);
        l.in_ = 6.0;
        l.out = 6.2;
        assert_eq!(l.checked().keep(), vec![(0.0, 10.0)]);
        let mut l = EditList::new(f64::NAN);
        l.out = 3.0;
        let l = l.checked();
        assert_eq!((l.source_duration, l.in_, l.out), (0.0, 0.0, 0.0));
        assert!(l.keep().is_empty());
    }

    #[test]
    fn ids_and_the_voice_plan_round_trip_and_old_lists_still_load() {
        let json = r#"{"version":1,"source_duration":10.0,"in":0.0,"out":10.0,"cuts":[{"id":"v1","start":2.0,"end":3.0,"kind":"retake","confidence":"high","applied":true,"removed":"a slip"}],"voice":{"planned_at":1792329066000,"cues":1}}"#;
        let l: EditList = serde_json::from_str(json).unwrap();
        assert_eq!(l.cuts[0].id, "v1");
        assert_eq!(
            l.voice,
            Some(Voice {
                planned_at: 1792329066000,
                cues: 1
            })
        );
        assert_eq!(l.clone().checked().voice, l.voice);
        let back = serde_json::to_value(&l).unwrap();
        assert_eq!(back["cuts"][0]["id"], "v1");
        assert_eq!(back["voice"]["cues"], 1);
        let old: EditList = serde_json::from_str(r#"{"version":1,"source_duration":5.0,"in":0.0,"out":5.0,"cuts":[{"start":1.0,"end":2.0,"kind":"manual","confidence":"high","applied":true}]}"#).unwrap();
        assert_eq!((old.cuts[0].id.as_str(), old.voice), ("", None));
        assert!(
            serde_json::to_value(&old).unwrap()["cuts"][0]
                .get("id")
                .is_none()
        );
        assert_eq!(old.next_manual_id(), "m1");
    }

    #[test]
    fn saved_round_trips_flat_and_keeps_its_copy_only_while_current() {
        let json = r#"{"version":1,"source_duration":10.0,"in":1.0,"out":9.0,"cuts":[{"start":2.0,"end":3.0,"kind":"manual","confidence":"high","applied":true}],"rev":3,"copy":{"rev":3,"file":"edit-3.mp4","duration":7.0}}"#;
        let s: Saved = serde_json::from_str(json).unwrap();
        assert_eq!(s.list.in_, 1.0);
        assert_eq!(
            s.current_copy().map(|c| c.file.as_str()),
            Some("edit-3.mp4")
        );
        let back = serde_json::to_value(&s).unwrap();
        assert_eq!(back["in"], 1.0);
        assert_eq!(back["rev"], 3);
        let stale = Saved { rev: 4, ..s };
        assert!(stale.current_copy().is_none());
        assert_eq!(EditCopy::file_for(4), "edit-4.mp4");
        assert_eq!(EditCopy::poster_for(4), "edit-4-poster.jpg");
        let c = EditCopy {
            rev: 4,
            file: EditCopy::file_for(4),
            duration: 1.0,
            poster: Some(EditCopy::poster_for(4)),
            teaser: None,
        };
        assert_eq!(c.files(), vec!["edit-4.mp4", "edit-4-poster.jpg"]);
    }
}
