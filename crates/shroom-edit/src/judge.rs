//! The judgment above the planner (spec N65): mechanism deterministic, judgment above. The
//! planner asks one [`Question`] per cue — what was said after it, and where behind it the
//! redone stretch might begin — and a judge (Clef, in the space's own account) answers with
//! a score per start. The code picks; the judge never gives a time or a word.

use serde::{Deserialize, Serialize};

/// A start scored at least this is said again after the cue: cutting from it loses nothing
/// the speaker meant to keep.
pub const JUDGED: f64 = 0.5;

/// For one cue: what was said after it, and the places behind it where the stretch it
/// redoes might begin, nearest first, each as the words from there up to the cue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    /// Which cue, in the order they were said.
    pub cue: usize,
    pub after: String,
    pub candidates: Vec<String>,
}

/// The judge's word on one question: a score from 0 to 1 per candidate, in its order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub cue: usize,
    pub scores: Vec<f64>,
}

/// The farthest-back candidate scored past [`JUDGED`], when the answer fits a question of
/// `n` candidates: one score each, every one a number from 0 to 1. Anything else is refused.
pub(crate) fn pick(answer: &Answer, n: usize) -> Option<usize> {
    if answer.scores.len() != n || answer.scores.iter().any(|s| !(0.0..=1.0).contains(s)) {
        return None;
    }
    answer.scores.iter().rposition(|s| *s >= JUDGED)
}
