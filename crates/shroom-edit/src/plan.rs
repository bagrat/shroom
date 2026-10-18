//! From cues to suggested cuts.

use crate::align::find_restart;
use crate::cue::{Cue, CueWords, Said, find_cues_in};
use crate::judge::{Answer, Intent, MEANT, Meant, Question, pick};
use crate::text::is_filler;
use crate::text::{ends_cleanly, ends_sentence, ends_whole, is_hesitation, norm};
use crate::{Confidence, Cut, CutKind, EditList, Silence, Transcript, Word};

/// A sentence the cue broke off goes with it only when it began at most this long before the
/// cue (seconds): a run-on the transcriber never stopped is not one sentence.
const SENTENCE_MAX: f64 = 20.0;
/// Where a redone stretch might begin, for the judgment: a pause at least this long
/// (seconds) begins one, as a sentence start does…
const START_PAUSE: f64 = 0.4;
/// …back this many sentences, or this many seconds, whichever comes first, and at most
/// this many starts.
const REACH_SENTENCES: usize = 3;
const REACH_BACK: f64 = 20.0;
const MOST_STARTS: usize = 6;
/// A pause inside a sentence begins a redone stretch only with at least this many words of
/// the sentence (fillers aside) before it: a word or two left behind is a stranded scrap.
const LEAVES: usize = 3;
/// A cut's words either side, for a review list (`leads`, `resumes`): a few, so its row
/// says where it joins. The owner page marks a full six as more said (worker.js `AROUND`).
const AROUND: usize = 6;
/// Where no silence was found, a cut's edge goes to the latest step at most this much
/// louder than the quietest one between the words (dB): just before what follows, and never
/// in the closure of a "t" or a "k" the word still has to release.
const NEAR_QUIETEST: f32 = 3.0;
/// After a cut's last word, what still sounds (the word's release, a breath) goes with the
/// cut, to the first pause, when that pause begins at most this long after the word
/// (seconds): sound that runs on longer may be speech the transcriber missed.
const TAIL: f64 = 1.0;

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
    /// Which spoken cues to listen for.
    pub cues: CueWords,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            lookback: 60.0,
            lookahead_words: 30,
            pad: 0.12,
            cue_pause: 0.25,
            line_pause: 0.6,
            cues: CueWords::Shroom,
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
    /// hugs the word on the cut's side: `hug_b` for a cut that starts at `b`. A cut that
    /// ends at `a` runs through what sounds after its last word to the first pause there
    /// ([`TAIL`]).
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
            None => self.first_pause(a, b).unwrap_or((a, a + 0.2)),
        }
    }

    /// The first silence after a word ending at `a`, beginning within [`TAIL`] of it and
    /// before the next word starts at `b`, with its extent kept as in [`Ctx::quiet`].
    fn first_pause(&self, a: f64, b: f64) -> Option<(f64, f64)> {
        self.t
            .silences
            .iter()
            .filter(|s| s.end > a && s.start < b && s.start <= a + TAIL)
            .map(|s| (s.start.max(a - 0.6), s.end.min(b + 0.6)))
            .filter(|(s, e)| e > s)
            .min_by(|x, y| x.0.total_cmp(&y.0))
    }

    /// A cut from the quiet before word `from` to the quiet after word `to` (inclusive).
    /// An edge with no silence under it goes to the quietest point between the words, when
    /// the take's loudness is known: word times drift, and the tail of a cut word must not
    /// play.
    fn span(&self, from: usize, to: usize) -> (f64, f64) {
        let (a0, b0) = (self.word_end_before(from), self.t.words[from].start);
        let (a1, b1) = (self.t.words[to].end, self.word_start_at(to + 1));
        let (q0a, q0b) = self.quiet(a0, b0, true);
        let (q1a, q1b) = self.quiet(a1, b1, false);
        let mut start = q0a + self.o.pad.min((q0b - q0a) / 2.0);
        let mut end = q1b - self.o.pad.min((q1b - q1a) / 2.0);
        if !self.in_silence(start) {
            start = self.quietest(near(a0, b0, true)).unwrap_or(start);
        }
        if !self.in_silence(end) {
            end = self.quietest(near(a1, b1, false)).unwrap_or(end);
        }
        (start.max(0.0), end.min(self.t.duration))
    }

    fn in_silence(&self, t: f64) -> bool {
        self.t.silences.iter().any(|s| s.start <= t && t <= s.end)
    }

    /// The latest step in `(from, to)` within [`NEAR_QUIETEST`] of the quietest one there,
    /// by the take's loudness (its middle, in seconds); none without the loudness.
    fn quietest(&self, (from, to): (f64, f64)) -> Option<f64> {
        let l = self.t.levels.as_ref()?;
        if !(l.step > 0.0) {
            return None;
        }
        let i0 = (from.max(0.0) / l.step).floor() as usize;
        let i1 = ((to.max(0.0) / l.step).ceil() as usize).min(l.db.len());
        if i0 >= i1 {
            return None;
        }
        let low = l.db[i0..i1].iter().copied().fold(f32::INFINITY, f32::min);
        let i = (i0..i1).rev().find(|&i| l.db[i] <= low + NEAR_QUIETEST)?;
        Some((i as f64 + 0.5) * l.step)
    }

    fn said(&self, from: usize, to: usize) -> String {
        self.t.words[from..=to]
            .iter()
            .map(|w| w.text.trim())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Up to [`AROUND`] words right before word `first`: what leads into a cut there.
    fn leads(&self, first: usize) -> Option<String> {
        let end = first.min(self.t.words.len());
        let ws = &self.t.words[end.saturating_sub(AROUND)..end];
        (!ws.is_empty()).then(|| {
            ws.iter()
                .map(|w| w.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        })
    }

    /// Up to [`AROUND`] words from word `at` on: what plays after a cut that ends before it.
    fn resumes(&self, at: usize) -> Option<String> {
        let ws = &self.t.words[at.min(self.t.words.len())..];
        (!ws.is_empty()).then(|| {
            ws.iter()
                .take(AROUND)
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

    /// Said apart from the speech: a pause before it or after it.
    fn apart(&self, cue: &Cue) -> bool {
        self.pause_before(cue.first) >= self.o.cue_pause
            || self.pause_after(cue.last) >= self.o.cue_pause
    }

    /// Where the stretch a cue redoes might begin, nearest first: each sentence start, and
    /// each long pause inside the cue's own sentence that leaves [`LEAVES`] words of it
    /// before it (and the earliest word it may reach), back [`REACH_SENTENCES`] sentences or
    /// [`REACH_BACK`] seconds, never past `floor`. A finished sentence before the cue's own
    /// is redone whole or not at all, so nothing begins inside one. A start with nothing but
    /// filler between it and the cue is none.
    fn starts(&self, cue: &Cue, floor: usize) -> Vec<usize> {
        let words = &self.t.words;
        let mut out = Vec::new();
        let mut sentences = 0;
        let mut k = cue.first;
        while k > floor && out.len() < MOST_STARTS {
            k -= 1;
            if words[k].start < cue.start - REACH_BACK {
                break;
            }
            let sentence = k > floor && ends_sentence(&words[k - 1].text);
            let pause =
                k > floor && sentences == 0 && self.gap(k) >= START_PAUSE && self.leaves(k, floor);
            let begins = k == floor || sentence || pause;
            if begins
                && words[k..cue.first]
                    .iter()
                    .any(|w| !is_filler(&norm(&w.text)))
            {
                out.push(k);
            }
            if sentence {
                sentences += 1;
                if sentences >= REACH_SENTENCES {
                    break;
                }
            }
        }
        out
    }

    /// Whether a pause before word `k` leaves [`LEAVES`] words of its sentence (fillers
    /// aside) behind it.
    fn leaves(&self, k: usize, floor: usize) -> bool {
        let words = &self.t.words;
        let mut n = 0;
        let mut i = k;
        while i > floor && !ends_sentence(&words[i - 1].text) {
            i -= 1;
            if !is_filler(&norm(&words[i].text)) {
                n += 1;
                if n >= LEAVES {
                    return true;
                }
            }
        }
        false
    }

    /// The pause before word `i` as heard there: the gap between the words, or the detected
    /// silence inside it (a little past either word). Unlike [`Ctx::pause_before`] it never
    /// borrows a silence from nearby, so the quiet before a cue doesn't make the word ahead
    /// of it look like the start of something.
    fn gap(&self, i: usize) -> f64 {
        let (a, b) = (self.t.words[i - 1].end, self.t.words[i].start);
        let inside = self
            .t
            .silences
            .iter()
            .map(|s| s.end.min(b + 0.1) - s.start.max(a - 0.1))
            .fold(0.0, f64::max);
        (b - a).max(inside)
    }

    /// What was said after the cue, its hesitations aside: up to two sentences, 30 words
    /// at most, never past `ceil`.
    fn after(&self, cue: &Cue, ceil: usize) -> String {
        let words = &self.t.words;
        let mut i = cue.last + 1;
        while i < ceil && is_hesitation(&norm(&words[i].text)) {
            i += 1;
        }
        let (mut said, mut sentences) = (Vec::new(), 0);
        while i < ceil && said.len() < 30 {
            said.push(words[i].text.trim());
            if ends_cleanly(&words[i].text) {
                sentences += 1;
                if sentences == 2 {
                    break;
                }
            }
            i += 1;
        }
        said.join(" ")
    }

    fn pause_after(&self, i: usize) -> f64 {
        if i + 1 >= self.t.words.len() {
            f64::INFINITY
        } else {
            self.pause_before(i + 1)
        }
    }

    /// Where the sentence holding word `i` began: just after the last one that ended as said,
    /// or after a long pause, never before `floor`. Unlike [`Ctx::line_start`] a finished
    /// sentence needs no pause after it: a speaker may run a good sentence straight into the
    /// one a cue breaks off, and the good one stays.
    fn sentence_start(&self, i: usize, floor: usize) -> usize {
        let mut k = i;
        while k > floor
            && !ends_cleanly(&self.t.words[k - 1].text)
            && self.pause_before(k) < self.o.line_pause
        {
            k -= 1;
        }
        k
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

/// Suggested cuts for every cue in the transcript, named `v1`… in the order of their cues.
/// Retakes found with confidence come applied; the rest come as suggestions to look at.
/// The code's own plan: [`plan_with`] and no judgment.
pub fn plan(t: &Transcript, o: Options) -> EditList {
    plan_with(t, o, &[])
}

/// Where to look for an edge's quietest point between a word ending at `a` and the next
/// starting at `b`: the gap and a little either side; in a long gap the transcriber may have
/// missed speech in, only near the word on the cut's side (`hug_b` for a cut starting at `b`).
fn near(a: f64, b: f64, hug_b: bool) -> (f64, f64) {
    if b - a <= 0.6 {
        (a - 0.08, b + 0.04)
    } else if hug_b {
        (b - 0.28, b + 0.04)
    } else {
        (a - 0.08, a + 0.24)
    }
}

/// The words before cue `k` it may reach back to (the previous cue's end), and the words
/// after it it may run to (the next cue).
fn bounds(cues: &[Cue], k: usize, len: usize) -> (usize, usize) {
    let floor = if k == 0 { 0 } else { cues[k - 1].last + 1 };
    (floor, cues.get(k + 1).map_or(len, |c| c.first))
}

/// The cues heard: "shroom cut", and in a wider set (`o.cues`) any other phrase the judgment
/// said was meant for the editor ([`MEANT`]); with no word on it, it isn't one.
fn heard(t: &Transcript, o: Options, meant: &[Meant]) -> Vec<Cue> {
    find_cues_in(&t.words, o.cues)
        .into_iter()
        .filter(|c| {
            c.said == Said::ShroomCut
                || meant.iter().any(|m| {
                    m.word == c.first && (0.0..=1.0).contains(&m.score) && m.score >= MEANT
                })
        })
        .collect()
}

/// The judgment's first question in a wider set of cues (`o.cues`): for each phrase said
/// apart but "shroom cut", whether the speaker meant it for the editor, with what was said
/// before and after it (two sentences or 30 words at most each way).
pub fn intents(t: &Transcript, o: Options) -> Vec<Intent> {
    let cx = Ctx { t, o };
    let words = &t.words;
    find_cues_in(words, o.cues)
        .iter()
        .filter(|c| c.said != Said::ShroomCut)
        .map(|c| {
            let before: Vec<&str> = words[c.first.saturating_sub(30)..c.first]
                .iter()
                .map(|w| w.text.trim())
                .collect();
            let after: Vec<&str> = words[c.last + 1..(c.last + 31).min(words.len())]
                .iter()
                .map(|w| w.text.trim())
                .collect();
            Intent {
                word: c.first,
                said: cx.said(c.first, c.last),
                before: before.join(" "),
                after: after.join(" "),
            }
        })
        .collect()
}

/// The planner's questions for the judgment above it: one for each cue said apart from the
/// speech that has somewhere to reach back to.
pub fn questions(t: &Transcript, o: Options) -> Vec<Question> {
    questions_meant(t, o, &[])
}

/// [`questions`], over the cues `meant` let in.
pub fn questions_meant(t: &Transcript, o: Options, meant: &[Meant]) -> Vec<Question> {
    let cx = Ctx { t, o };
    let cues = heard(t, o, meant);
    let mut out = Vec::new();
    for (k, cue) in cues.iter().enumerate() {
        let (floor, ceil) = bounds(&cues, k, t.words.len());
        let starts = if cx.apart(cue) {
            cx.starts(cue, floor)
        } else {
            Vec::new()
        };
        if !starts.is_empty() {
            let candidates = starts.iter().map(|&s| cx.said(s, cue.first - 1)).collect();
            out.push(Question {
                cue: k,
                after: cx.after(cue, ceil),
                candidates,
            });
        }
    }
    out
}

/// The plan with the judgment's answers (see [`crate::judge`]): a cue whose answer scores a
/// start past the bar has its cut reach back to the farthest such start; one with no answer,
/// or an answer that doesn't fit its question, keeps the code's own plan. A judgment only
/// ever reaches further back: it never shortens a cut the words themselves set.
pub fn plan_with(t: &Transcript, o: Options, answers: &[Answer]) -> EditList {
    plan_meant(t, o, &[], answers)
}

/// [`plan_with`], over the cues `meant` let in (in a wider set of cues, `o.cues`).
pub fn plan_meant(t: &Transcript, o: Options, meant: &[Meant], answers: &[Answer]) -> EditList {
    let cx = Ctx { t, o };
    let cues = heard(t, o, meant);
    let mut list = EditList::new(t.duration);
    for (k, cue) in cues.iter().enumerate() {
        let (floor, ceil) = bounds(&cues, k, t.words.len());
        let judged = answers.iter().find(|a| a.cue == k).and_then(|a| {
            let starts = if cx.apart(cue) {
                cx.starts(cue, floor)
            } else {
                Vec::new()
            };
            pick(a, starts.len()).map(|i| starts[i])
        });
        list.cuts.extend(plan_one(&cx, cue, floor, ceil, judged));
    }
    for (n, cut) in list.cuts.iter_mut().enumerate() {
        cut.id = format!("v{}", n + 1);
    }
    list
}

/// One cue's cuts. `judged`: where the judgment put the start of the stretch it redoes.
fn plan_one(cx: &Ctx, cue: &Cue, floor: usize, ceil: usize, judged: Option<usize>) -> Vec<Cut> {
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
    let apart = cx.apart(cue);

    if let Some(rs) = find_restart(&pre, &post) {
        let flub = pre.len() - rs.r;
        let coverage = rs.matched as f64 / flub as f64;
        let restart = pre_ix[rs.r];
        let long = cue.start - words[restart].start > 40.0;
        let found = !long && coverage >= 0.4 && (rs.matched >= 2 || flub <= 2);
        if found {
            // The judgment can say more was said again than the words line up. A hesitation
            // right before the restart ("Um, today I'll…") goes with it.
            let mut from = judged.map_or(restart, |j| j.min(restart));
            while from > floor && is_hesitation(&norm(&words[from - 1].text)) {
                from -= 1;
            }
            let high = judged.is_some()
                || apart
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
                leads: cx.leads(from),
                removed: cx.said(from, resume - 1),
                resumes: cx.resumes(resume),
                why: if from < restart {
                    format!(
                        "retake: {} of {} words said again, and more by the judgment",
                        rs.matched, flub
                    )
                } else {
                    format!("retake: {} of {} words said again", rs.matched, flub)
                },
            }];
        }
    }

    // No retake. Said apart from the speech, it's still a cue.
    if !apart {
        return Vec::new();
    }
    // It broke off a sentence: whatever was said after it, in the same words or others,
    // that sentence is the slip, and it goes with the cue; the one before it, finished,
    // stays.
    let sentence = (cue.first > floor)
        .then(|| cx.sentence_start(cue.first - 1, floor))
        .filter(|&s| {
            let said: Vec<&str> = words[s..cue.first]
                .iter()
                .map(|w| w.text.as_str())
                .collect();
            !ends_whole(&said)
        });
    let sentence = sentence.filter(|&s| cue.start - words[s].start <= SENTENCE_MAX);
    // The judgment heard a stretch said again in other words: it goes with the cue, as far
    // back as the judgment put its start, and never less than the sentence the cue broke off.
    let reach = match (sentence, judged) {
        (Some(s), Some(j)) => Some((
            s.min(j),
            "the sentence the cue broke off, and more by the judgment",
        )),
        (Some(s), None) => Some((s, "the sentence the cue broke off")),
        (None, Some(j)) => Some((j, "said again in other words, by the judgment")),
        (None, None) => None,
    };
    if let Some((from, why)) = reach {
        {
            let mut resume = cue.last + 1;
            while resume < ceil && is_hesitation(&norm(&words[resume].text)) {
                resume += 1;
            }
            let (start, end) = cx.span(from, resume - 1);
            return vec![Cut {
                id: String::new(),
                start,
                end,
                kind: CutKind::Retake,
                confidence: Confidence::High,
                applied: true,
                leads: cx.leads(from),
                removed: cx.said(from, resume - 1),
                resumes: cx.resumes(resume),
                why: why.into(),
            }];
        }
    }
    // The sentence before it ended: cut the cue (and any filler right after), and offer the
    // line before it.
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
        leads: cx.leads(cue.first),
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
            leads: cx.leads(line),
            removed: cx.said(line, last),
            resumes: cx.resumes(last + 1),
            why: "the line before a cue with no retake".into(),
        });
    }
    cuts
}
