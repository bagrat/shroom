//! From cues to suggested cuts.

use crate::align::find_restart;
use crate::cue::{Cue, find_cues};
use crate::text::{ends_sentence, is_hesitation, norm};
use crate::{Confidence, Cut, CutKind, EditList, Silence, Transcript, Word};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// How far back before a cue a restart may be (seconds).
    pub lookback: f64,
    /// How many words after a cue the retake is looked for in.
    pub lookahead_words: usize,
    /// Silence kept on each side of a cut, at most (seconds).
    pub pad: f64,
    /// A pause this long around the cue marks it as said apart from the speech.
    pub cue_pause: f64,
    /// A pause this long ends a line (for a cue with no retake).
    pub line_pause: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            lookback: 60.0,
            lookahead_words: 30,
            pad: 0.12,
            cue_pause: 0.25,
            line_pause: 0.6,
        }
    }
}

struct Ctx<'a> {
    t: &'a Transcript,
    o: Options,
}

impl Ctx<'_> {
    fn word_end_before(&self, i: usize) -> f64 {
        if i == 0 { 0.0 } else { self.t.words[i - 1].end }
    }

    fn word_start_at(&self, i: usize) -> f64 {
        self.t.words.get(i).map_or(self.t.duration, |w| w.start)
    }

    /// The quiet stretch between a word ending at `a` and the next starting at `b`: the
    /// detected silence that best covers the gap, else the gap itself. A long gap with no
    /// silence in it isn't quiet (the transcriber missed speech there), so then the edge
    /// hugs the word on the cut's side: `hug_b` for a cut that starts at `b`.
    fn quiet(&self, a: f64, b: f64, hug_b: bool) -> (f64, f64) {
        // Word times drift (by a few tenths on long audio), so a silence near the gap
        // counts, and keeps its own extent up to 0.6 s past the words.
        let (lo, hi) = (a - 0.25, b + 0.25);
        let found = self
            .t
            .silences
            .iter()
            .filter(|s: &&Silence| s.end > lo && s.start < hi)
            .map(|s| (s.start.max(a - 0.6), s.end.min(b + 0.6)))
            .filter(|(s, e)| e > s)
            .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)))
            // Speech the transcriber missed leaves most of a long gap loud.
            .filter(|(s, e)| e - s >= (b - a) - 0.5);
        match found {
            Some(q) => q,
            None if b - a <= 0.6 || self.t.silences.is_empty() => (a, b.max(a)),
            None if hug_b => (b - 0.2, b),
            None => (a, a + 0.2),
        }
    }

    /// A cut from the quiet before word `from` to the quiet after word `to` (inclusive).
    fn span(&self, from: usize, to: usize) -> (f64, f64) {
        let (q0a, q0b) = self.quiet(self.word_end_before(from), self.t.words[from].start, true);
        let (q1a, q1b) = self.quiet(self.t.words[to].end, self.word_start_at(to + 1), false);
        let start = q0a + self.o.pad.min((q0b - q0a) / 2.0);
        let end = q1b - self.o.pad.min((q1b - q1a) / 2.0);
        (start.max(0.0), end.min(self.t.duration))
    }

    fn said(&self, from: usize, to: usize) -> String {
        self.t.words[from..=to]
            .iter()
            .map(|w| w.text.trim())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn resumes(&self, at: usize) -> Option<String> {
        let ws = &self.t.words[at.min(self.t.words.len())..];
        (!ws.is_empty()).then(|| {
            ws.iter()
                .take(6)
                .map(|w| w.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
    }

    /// How long the quiet before word `i` is. Transcribers stretch a word's end into the
    /// pause after it, so the detected silence counts when there is one.
    fn pause_before(&self, i: usize) -> f64 {
        if i == 0 {
            return f64::INFINITY;
        }
        let (a, b) = self.quiet(self.t.words[i - 1].end, self.t.words[i].start, true);
        (b - a).max(self.t.words[i].start - self.t.words[i - 1].end)
    }

    fn pause_after(&self, i: usize) -> f64 {
        if i + 1 >= self.t.words.len() {
            f64::INFINITY
        } else {
            self.pause_before(i + 1)
        }
    }

    /// Where the line before word `i` began: back to a sentence end with a pause after it,
    /// or a long pause. The pause matters: a transcriber punctuates a hesitation too
    /// ("The second one is him. I forget…" for "is, hmm, I forget…").
    fn line_start(&self, i: usize, floor: usize) -> usize {
        let mut k = i;
        while k > floor {
            let prev: &Word = &self.t.words[k - 1];
            let pause = self.pause_before(k);
            if (ends_sentence(&prev.text) && pause >= self.o.cue_pause)
                || pause >= self.o.line_pause
            {
                break;
            }
            k -= 1;
        }
        k
    }
}

/// Suggested cuts for every cue in the transcript. Retakes found with confidence come
/// applied; the rest come as suggestions to look at.
pub fn plan(t: &Transcript, o: Options) -> EditList {
    let cx = Ctx { t, o };
    let cues = find_cues(&t.words);
    let mut list = EditList::new(t.duration);
    for (k, cue) in cues.iter().enumerate() {
        let floor = if k == 0 { 0 } else { cues[k - 1].last + 1 };
        let ceil = cues.get(k + 1).map_or(t.words.len(), |c| c.first);
        list.cuts.extend(plan_one(&cx, cue, floor, ceil));
    }
    list
}

fn plan_one(cx: &Ctx, cue: &Cue, floor: usize, ceil: usize) -> Vec<Cut> {
    let words = &cx.t.words;
    let lo = (floor..cue.first)
        .find(|&i| words[i].start >= cue.start - cx.o.lookback)
        .unwrap_or(cue.first);
    let hi = (cue.last + 1 + cx.o.lookahead_words).min(ceil);
    let (pre_ix, pre): (Vec<usize>, Vec<String>) = (lo..cue.first)
        .map(|i| (i, norm(&words[i].text)))
        .filter(|(_, w)| !w.is_empty())
        .unzip();
    let (post_ix, post): (Vec<usize>, Vec<String>) = (cue.last + 1..hi)
        .map(|i| (i, norm(&words[i].text)))
        .filter(|(_, w)| !w.is_empty())
        .unzip();
    let apart =
        cx.pause_before(cue.first) >= cx.o.cue_pause || cx.pause_after(cue.last) >= cx.o.cue_pause;

    if let Some(rs) = find_restart(&pre, &post) {
        let flub = pre.len() - rs.r;
        let coverage = rs.matched as f64 / flub as f64;
        let from = pre_ix[rs.r];
        let long = cue.start - words[from].start > 40.0;
        let found = !long && coverage >= 0.4 && (rs.matched >= 2 || flub <= 2);
        if found {
            let high = apart
                && ((rs.matched >= 3 && coverage >= 0.5)
                    || (rs.matched >= 2 && coverage >= 0.66)
                    || flub <= 2);
            // Hesitations between the cue and the retake go with the cue; "so", "okay"
            // and the like are the retake's own opening and stay.
            let mut resume = cue.last + 1;
            while resume < post_ix[rs.s] && is_hesitation(&norm(&words[resume].text)) {
                resume += 1;
            }
            let (start, end) = cx.span(from, resume - 1);
            return vec![Cut {
                id: String::new(),
                start,
                end,
                kind: CutKind::Retake,
                confidence: if high {
                    Confidence::High
                } else {
                    Confidence::Low
                },
                applied: high,
                removed: cx.said(from, resume - 1),
                resumes: cx.resumes(resume),
                why: format!("retake: {} of {} words said again", rs.matched, flub),
            }];
        }
    }

    // No retake. Said apart from the speech, it's still a cue: cut it (and any filler
    // right after), and offer the line before it.
    if !apart {
        return Vec::new();
    }
    let mut last = cue.last;
    while last + 1 < ceil && is_hesitation(&norm(&words[last + 1].text)) {
        last += 1;
    }
    let (start, end) = cx.span(cue.first, last);
    let mut cuts = vec![Cut {
        id: String::new(),
        start,
        end,
        kind: CutKind::CueOnly,
        confidence: Confidence::High,
        applied: true,
        removed: cx.said(cue.first, last),
        resumes: cx.resumes(last + 1),
        why: "cue with no retake after it".into(),
    }];
    if cue.first > floor {
        let line = cx.line_start(cue.first - 1, floor);
        let (start, _) = cx.span(line, cue.first - 1);
        cuts.push(Cut {
            id: String::new(),
            start,
            end,
            kind: CutKind::LineBeforeCue,
            confidence: Confidence::Low,
            applied: false,
            removed: cx.said(line, last),
            resumes: cx.resumes(last + 1),
            why: "the line before a cue with no retake".into(),
        });
    }
    cuts
}
