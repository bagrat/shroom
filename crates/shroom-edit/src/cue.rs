//! Finding the spoken cue ("shroom cut") in a transcript.

use serde::{Deserialize, Serialize};

use crate::Word;
use crate::text::{levenshtein, norm, sound_key};

/// Which spoken cues to listen for (spec N65 (1)). "shroom cut" is distinctive enough to
/// stand alone. Any other phrase is a cue only when it is said apart from the speech (a
/// sentence of its own, or a pause either side) and the judgment says the speaker meant it
/// for whoever edits the video (see [`crate::plan_meant`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CueWords {
    /// "shroom cut", heard by sound.
    #[default]
    Shroom,
    /// That, and "cut" said on its own.
    Plain,
    /// Those, and "cut here", "cut that", "cut cut", "scratch that", "redo", "let me redo
    /// that", "let me say that again".
    Relaxed,
}

/// What a cue was said as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Said {
    ShroomCut,
    Cut,
    CutHere,
    CutThat,
    CutCut,
    ScratchThat,
    Redo,
    SayThatAgain,
}

/// One spoken cue: words `first..=last` of the transcript.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cue {
    pub first: usize,
    pub last: usize,
    pub start: f64,
    pub end: f64,
    pub said: Said,
}

fn is_code_word(w: &str) -> bool {
    let key = sound_key(w);
    levenshtein(&key, "shrum") <= 1 || levenshtein(&key, "shrums") <= 1
}

/// A further mishearing of the code word ("Troom", "drum", "Trung"): only a cue when said
/// as a sentence of its own, since "drum cut" could be speech.
fn is_code_word_heard_loosely(w: &str) -> bool {
    let key = sound_key(w);
    key.chars().count() >= 3
        && levenshtein(&key, "shrum") <= 2
        && (key.ends_with("um") || key.ends_with("ung"))
}

fn is_cut(w: &str) -> bool {
    matches!(w, "cut" | "cuts" | "kut" | "cutt" | "cot")
}

/// Words that make the cue a mention rather than a command: "say shroom cut", "the
/// shroom cut feature".
fn is_mention_lead(w: &str) -> bool {
    matches!(
        w,
        "say"
            | "says"
            | "said"
            | "saying"
            | "the"
            | "a"
            | "called"
            | "word"
            | "words"
            | "phrase"
            | "like"
            | "with"
            | "use"
            | "using"
    )
}

/// Every cue in the transcript, in order. Matches the code word by sound, so the
/// transcriber's spelling doesn't matter, and skips mentions ("you say shroom cut").
pub fn find_cues(words: &[Word]) -> Vec<Cue> {
    let n: Vec<String> = words.iter().map(|w| norm(&w.text)).collect();
    let mut cues = Vec::new();
    let mut i = 0;
    while i < words.len() {
        // A lead word only makes a mention when it runs into the cue ("say shroom cut"),
        // not when a sentence ends between them ("…what it's called. Shroom cut.").
        let mention = i > 0
            && is_mention_lead(&n[i - 1])
            && !words[i - 1]
                .text
                .trim_end()
                .ends_with(['.', '?', '!', ',', ';', ':']);
        // "shroom cut", "Shroom-cut" (one token, hyphen gone), or "shroomcut".
        let head = n[i].strip_suffix("cut");
        let joined = head.is_some_and(is_code_word);
        let joined_loose = !joined && head.is_some_and(is_code_word_heard_loosely);
        let then_cut = i + 1 < words.len() && is_cut(&n[i + 1]);
        let pair = is_code_word(&n[i]) && then_cut;
        let loose = (!pair && is_code_word_heard_loosely(&n[i]) && then_cut) || joined_loose;
        let last = if joined || joined_loose {
            Some(i)
        } else if pair || loose {
            Some(i + 1)
        } else {
            None
        };
        if let Some(last) = last {
            // Said as a sentence of its own ("…one more thing, the shroom cut.") it's a
            // command even after a lead word: the transcriber often hears "uh" as "the".
            let alone = words[last].text.trim_end().ends_with(['.', '!', '?'])
                || words
                    .get(last + 1)
                    .is_none_or(|n| n.start - words[last].end >= 0.5);
            let opens = i == 0 || words[i - 1].text.trim_end().ends_with(['.', '!', '?', ',']);
            let stops = words[last].text.trim_end().ends_with(['.', '!', '?'])
                || words
                    .get(last + 1)
                    .is_some_and(|n| n.start - words[last].end >= 0.5);
            if loose && !(stops && opens) {
                i += 1;
                continue;
            }
            if !mention || alone {
                cues.push(Cue {
                    first: i,
                    last,
                    start: words[i].start,
                    end: words[last].end,
                    said: Said::ShroomCut,
                });
            }
            i = last + 1;
        } else {
            i += 1;
        }
    }
    cues
}

/// The other ways of saying it, as their words: `None` stands for any spelling of "cut".
/// The last field: heard only in the relaxed set.
const PHRASES: &[(Said, &[Option<&str>], bool)] = &[
    (Said::Cut, &[None], false),
    (Said::CutHere, &[None, Some("here")], true),
    (Said::CutThat, &[None, Some("that")], true),
    (Said::CutThat, &[None, Some("this")], true),
    (Said::CutCut, &[None, None], true),
    (Said::ScratchThat, &[Some("scratch"), Some("that")], true),
    (Said::Redo, &[Some("redo")], true),
    (Said::Redo, &[Some("redo"), Some("that")], true),
    (
        Said::Redo,
        &[Some("let"), Some("me"), Some("redo"), Some("that")],
        true,
    ),
    (
        Said::Redo,
        &[Some("let"), Some("me"), Some("redo"), Some("it")],
        true,
    ),
    (Said::Redo, &[Some("let"), Some("me"), Some("redo")], true),
    (
        Said::SayThatAgain,
        &[
            Some("let"),
            Some("me"),
            Some("say"),
            Some("that"),
            Some("again"),
        ],
        true,
    ),
    (
        Said::SayThatAgain,
        &[
            Some("let"),
            Some("me"),
            Some("try"),
            Some("that"),
            Some("again"),
        ],
        true,
    ),
    (
        Said::SayThatAgain,
        &[Some("let"), Some("me"), Some("try"), Some("again")],
        true,
    ),
];

/// A pause at least this long (seconds) sets a phrase apart from the speech around it.
const APART: f64 = 0.25;

/// Words `i..=last` said apart: after a sentence's end, a comma or a pause, and before a
/// sentence's end, a pause or the take's end.
fn apart(words: &[Word], i: usize, last: usize) -> bool {
    let opens = i == 0
        || words[i - 1]
            .text
            .trim_end()
            .ends_with(['.', '!', '?', ',', ';', ':'])
        || words[i].start - words[i - 1].end >= APART;
    let stops = words[last].text.trim_end().ends_with(['.', '!', '?'])
        || words
            .get(last + 1)
            .is_none_or(|n| n.start - words[last].end >= APART);
    opens && stops
}

/// [`find_cues`], and in `set` the other ways of saying it, each said apart (the longest
/// that fits), in the order they were said.
pub fn find_cues_in(words: &[Word], set: CueWords) -> Vec<Cue> {
    let mut cues = find_cues(words);
    if set == CueWords::Shroom {
        return cues;
    }
    let n: Vec<String> = words.iter().map(|w| norm(&w.text)).collect();
    let taken = |k: usize, cues: &[Cue]| cues.iter().any(|c| c.first <= k && k <= c.last);
    let mut more = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let fits = |p: &[Option<&str>]| {
            i + p.len() <= words.len()
                && p.iter().enumerate().all(|(j, w)| match w {
                    None => is_cut(&n[i + j]),
                    Some(w) => n[i + j] == *w,
                })
                && (i..i + p.len()).all(|k| !taken(k, &cues))
        };
        let found = PHRASES
            .iter()
            .filter(|(_, p, relaxed)| (set == CueWords::Relaxed || !relaxed) && fits(p))
            .map(|(said, p, _)| (*said, p.len()))
            .filter(|&(_, len)| apart(words, i, i + len - 1))
            .max_by_key(|&(_, len)| len);
        if let Some((said, len)) = found {
            let last = i + len - 1;
            more.push(Cue {
                first: i,
                last,
                start: words[i].start,
                end: words[last].end,
                said,
            });
            i = last + 1;
        } else {
            i += 1;
        }
    }
    cues.extend(more);
    cues.sort_by_key(|c| c.first);
    cues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<Word> {
        s.split_whitespace()
            .enumerate()
            .map(|(i, t)| Word {
                text: t.into(),
                start: i as f64,
                end: i as f64 + 0.9,
            })
            .collect()
    }

    #[test]
    fn spellings_of_the_code_word() {
        for s in [
            "Shroom cut.",
            "it's called. Shroom cut.",
            "one more thing, the Shroom cut.",
            "schroom, cut",
            "shrewm cut",
            "Shroom-cut",
            "shroomcut",
            "shrooms cut",
            "is pasted. Troom cut.",
            "is pasted. Troomcut.",
            "uh, drum cut.",
        ] {
            assert_eq!(find_cues(&words(s)).len(), 1, "{s}");
        }
    }

    #[test]
    fn not_a_cue() {
        for s in [
            "you can cut it here",
            "shroom uploads it",
            "room cut",
            "the drum cut was loud",
            "a drum cut. Then",
            "just say shroom cut and",
            "the shroom cut feature",
        ] {
            assert!(find_cues(&words(s)).is_empty(), "{s}");
        }
    }
}
