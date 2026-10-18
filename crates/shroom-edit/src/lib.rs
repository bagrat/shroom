//! Editing, POC (dev/proposals/voice-cut.md).
//!
//! Two things live here:
//!
//! - **The edit list** ([`EditList`]): cuts over a recording's source timeline. It never
//!   touches the media; playback and the published copy apply it. Every cut is a
//!   suggestion until it's `applied`, and un-applying one is the undo.
//! - **The voice-cue planner** ([`plan`]): given a transcript with word times, find the
//!   spoken cue ("shroom cut"), find where the speaker restarted the line it retakes, and
//!   suggest a cut from that restart through the end of the cue, snapped into the silence
//!   on either side.
//!
//! Deterministic and pure: words in, edit list out. Transcription happens elsewhere.

mod align;
mod cue;
mod plan;
mod text;

pub use cue::{Cue, find_cues};
pub use plan::{Options, plan};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CutKind {
    /// The cue and the take it retakes.
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
    pub start: f64,
    pub end: f64,
    pub kind: CutKind,
    pub confidence: Confidence,
    pub applied: bool,
    /// What the cut removes, as transcribed (for the review list).
    pub removed: String,
    /// What plays right after the cut (for the review list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumes: Option<String>,
    /// Why the planner suggested it, in a few words (debugging, not product copy).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
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
}

impl EditList {
    pub fn new(source_duration: f64) -> Self {
        Self {
            version: 1,
            source_duration,
            in_: 0.0,
            out: source_duration,
            cuts: Vec::new(),
        }
    }

    /// The source ranges that play, in order: trim minus applied cuts, overlaps merged.
    pub fn keep(&self) -> Vec<(f64, f64)> {
        let mut cuts: Vec<(f64, f64)> = self
            .cuts
            .iter()
            .filter(|c| c.applied)
            .map(|c| (c.start, c.end))
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut keep = Vec::new();
        let mut at = self.in_;
        for (s, e) in cuts {
            let (s, e) = (s.max(self.in_), e.min(self.out));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(start: f64, end: f64, applied: bool) -> Cut {
        Cut {
            start,
            end,
            kind: CutKind::Manual,
            confidence: Confidence::High,
            applied,
            removed: String::new(),
            resumes: None,
            why: String::new(),
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
    }
}
