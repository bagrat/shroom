//! A cue that breaks off a sentence (spec N65): the speaker stops mid-sentence, says
//! “shroom cut” and says it again in other words. Nothing repeats word for word, yet the
//! broken-off sentence is the slip, so it goes with the cue. The take is Bagrat's on the
//! tester (2026-10-18), rebuilt from what he said, on the times his screenshot showed.

use shroom_edit::{CutKind, Options, Silence, Transcript, plan};

/// `(text, start, end)` words, then the quiet between them.
fn take(words: &[(&str, f64, f64)], silences: &[(f64, f64)], duration: f64) -> Transcript {
    let words: Vec<(String, f64, f64)> = words
        .iter()
        .map(|(w, s, e)| (w.to_string(), *s, *e))
        .collect();
    Transcript::heard(
        duration,
        &words,
        silences
            .iter()
            .map(|&(start, end)| Silence { start, end })
            .collect(),
    )
}

const BEFORE: &[(&str, f64, f64)] = &[
    ("Hi,", 0.5, 0.8),
    ("this", 1.2, 1.4),
    ("is", 1.45, 1.55),
    ("a", 1.6, 1.65),
    ("quick", 1.7, 2.0),
    ("test", 2.05, 2.35),
    ("of", 2.4, 2.5),
    ("the", 2.55, 2.65),
    ("voice", 2.7, 3.0),
    ("cut.", 3.05, 3.4),
];
const AFTER: &[(&str, f64, f64)] = &[
    ("To", 11.3, 11.4),
    ("get", 11.45, 11.6),
    ("your", 11.65, 11.8),
    ("video,", 11.85, 12.3),
    ("use", 12.5, 12.7),
    ("this", 12.75, 12.9),
    ("link", 12.95, 13.25),
    ("and", 13.3, 13.4),
    ("you'll", 13.45, 13.7),
    ("see", 13.75, 13.95),
    ("it", 14.0, 14.1),
    ("right", 14.15, 14.4),
    ("away.", 14.45, 14.9),
];
const QUIET: &[(f64, f64)] = &[
    (0.0, 0.5),
    (0.8, 1.2),
    (3.4, 6.5),
    (8.2, 8.9),
    (9.8, 11.3),
    (12.3, 12.5),
    (14.9, 18.9),
];

/// “Click on this link and you will <last> shroom cut.”, between BEFORE and AFTER.
fn reworded(last: &str) -> Transcript {
    let broken = [
        ("Click", 6.5, 6.8),
        ("on", 6.85, 6.95),
        ("this", 7.0, 7.2),
        ("link", 7.25, 7.55),
        ("and", 7.6, 7.7),
        ("you", 7.75, 7.85),
        (last, 7.9, 8.2),
        ("shroom", 8.9, 9.3),
        ("cut.", 9.35, 9.8),
    ];
    let words: Vec<(&str, f64, f64)> = BEFORE
        .iter()
        .chain(broken.iter())
        .chain(AFTER.iter())
        .copied()
        .collect();
    take(&words, QUIET, 18.9)
}

/// What still plays, as words.
fn kept(t: &Transcript) -> String {
    let list = plan(t, Options::default());
    let keep = list.keep();
    t.words
        .iter()
        .filter(|w| {
            let mid = (w.start + w.end) / 2.0;
            keep.iter().any(|(s, e)| mid >= *s && mid < *e)
        })
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_reworded_redo_after_a_broken_off_sentence_takes_the_sentence_with_the_cue() {
    let t = reworded("will");
    let list = plan(&t, Options::default());
    assert_eq!(list.cuts.len(), 1, "{:?}", list.cuts);
    let c = &list.cuts[0];
    assert_eq!(
        (c.kind, c.applied, c.id.as_str()),
        (CutKind::Retake, true, "v1")
    );
    assert_eq!(c.removed, "Click on this link and you will shroom cut.");
    // Its edges sit in the quiet on either side.
    assert!(
        c.start > 3.4 && c.start < 6.5 && c.end > 9.8 && c.end < 11.3,
        "{c:?}"
    );
    assert_eq!(
        kept(&t),
        "Hi, this is a quick test of the voice cut. To get your video, use this link and you'll see it right away."
    );
}

#[test]
fn a_sentence_trailing_off_into_an_ellipsis_is_broken_off_too() {
    assert_eq!(
        kept(&reworded("will...")),
        "Hi, this is a quick test of the voice cut. To get your video, use this link and you'll see it right away."
    );
    assert_eq!(
        kept(&reworded("will…")),
        "Hi, this is a quick test of the voice cut. To get your video, use this link and you'll see it right away."
    );
}

#[test]
fn a_full_stop_on_a_word_no_sentence_ends_on_is_broken_off_too() {
    // Whisper puts a full stop where the speaker stopped (the stock-voice take of these
    // words came back "…and you will. Shroom cut."), but no sentence ends on “will”.
    assert_eq!(
        kept(&reworded("will.")),
        "Hi, this is a quick test of the voice cut. To get your video, use this link and you'll see it right away."
    );
}

#[test]
fn a_sentence_that_ended_cleanly_is_cut_with_the_cue_unsure() {
    // “Click on this link to see it.” is whole; nothing like it follows the cue. How far back
    // the slip went isn't clear, so the line goes with the cue for the owner to check
    // (Bagrat, 2026-10-18): the cue's own cut, and the line's.
    let finished = [
        ("Click", 6.5, 6.8),
        ("on", 6.85, 6.95),
        ("this", 7.0, 7.2),
        ("link", 7.25, 7.55),
        ("to", 7.6, 7.7),
        ("see", 7.75, 7.95),
        ("it.", 8.0, 8.2),
        ("shroom", 8.9, 9.3),
        ("cut.", 9.35, 9.8),
    ];
    let words: Vec<(&str, f64, f64)> = BEFORE
        .iter()
        .chain(finished.iter())
        .chain(AFTER.iter())
        .copied()
        .collect();
    let list = plan(&take(&words, QUIET, 18.9), Options::default());
    let kinds: Vec<(CutKind, bool)> = list.cuts.iter().map(|c| (c.kind, c.applied)).collect();
    assert_eq!(
        kinds,
        [(CutKind::CueOnly, true), (CutKind::LineBeforeCue, true)]
    );
}

#[test]
fn a_cue_run_into_the_speech_is_still_left_alone() {
    // No pause on either side: “shroom cut” said in the flow of a sentence isn't a cue.
    let words: Vec<(&str, f64, f64)> = BEFORE
        .iter()
        .copied()
        .chain([
            ("Click", 6.5, 6.8),
            ("and", 6.85, 6.95),
            ("you", 7.0, 7.1),
            ("will", 7.15, 7.4),
            ("shroom", 7.45, 7.75),
            ("cut", 7.8, 8.0),
            ("the", 8.05, 8.15),
            ("intro", 8.2, 8.6),
        ])
        .collect();
    let t = take(
        &words,
        &[(0.0, 0.5), (0.8, 1.2), (3.4, 6.5), (8.6, 10.0)],
        10.0,
    );
    assert!(plan(&t, Options::default()).cuts.is_empty());
}
