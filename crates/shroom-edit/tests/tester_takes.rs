//! Bagrat's takes on the tester (2026-10-18), rebuilt from the words it showed
//! (tests/takes/hand-bagrat-tester-1 to -4): what the planner got wrong in them, and the take
//! he said came out right, held.
//!
//! - A cue that breaks off a sentence takes that sentence, never the finished one before it,
//!   even when the speaker ran straight on from one into the other.
//! - A sentence ending on "is", "are", "was" or "were" after a noun ("and your link is.") was
//!   broken off; after a pronoun ("it is.") or under a question word ("where your files are.")
//!   it is whole.
//! - Where the judgment may begin a redone stretch: a sentence's start, or a pause inside the
//!   cue's own sentence, and never one that leaves a word or two of that sentence behind.
//! - A cut after a cue runs through what still sounds after its last word (the word's
//!   release, a breath) to the first pause, never ending in the short dip right after it.
//! - Two restarts said again and a cue run straight on from “and”, each cut whole and each
//!   ending in a pause (his fourth take, “The second take was better”).

use std::fs;
use std::path::Path;

use shroom_edit::{
    Answer, CutKind, Levels, Options, Silence, Transcript, plan, plan_with, questions,
};

fn rebuilt(name: &str) -> Transcript {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/takes")
        .join(name);
    serde_json::from_str(&fs::read_to_string(dir.join("transcript.json")).unwrap()).unwrap()
}

/// What the judgment answered for a rebuilt take, as the tester showed it.
fn judged(name: &str) -> Vec<Answer> {
    #[derive(serde::Deserialize)]
    struct Judged {
        answers: Vec<Answer>,
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/takes")
        .join(name);
    serde_json::from_str::<Judged>(&fs::read_to_string(dir.join("judged.json")).unwrap())
        .unwrap()
        .answers
}

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

/// What still plays, as words (a word plays when its middle does).
fn kept(t: &Transcript) -> String {
    let keep = plan(t, Options::default()).keep();
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
fn a_broken_off_sentence_never_takes_the_finished_one_it_ran_on_from() {
    // “…in one click. click, you press stop and the link is pasted, strum cut, when you
    // want…”: no pause after “click.”, yet that sentence was his good third try.
    let t = rebuilt("hand-bagrat-tester-2");
    let list = plan(&t, Options::default());
    let last = list.cuts.last().unwrap();
    assert_eq!(
        last.removed,
        "click, you press stop and the link is pasted, strum cut,"
    );
    assert!(kept(&t).contains(
        "Hi, this is a quick test of shroom. Today I'll show you how to share a recording in one click. when you want"
    ));
}

#[test]
fn a_sentence_ending_on_is_after_a_noun_was_broken_off() {
    // “When you want to get your link, just press stop and your link is. Shroom cut. Okay.”
    let t = rebuilt("hand-bagrat-tester-1");
    let list = plan(&t, Options::default());
    let last = list.cuts.last().unwrap();
    assert_eq!(
        (last.kind, last.applied, last.removed.as_str()),
        (
            CutKind::Retake,
            true,
            "When you want to get your link, just press stop and your link is. Shroom cut."
        )
    );
    // The line before his second cue, said again in other words after it, is only offered
    // by the words alone: cutting it is the judgment's to say (he cut it by hand).
    assert!(
        kept(&t).ends_with("in one click. You press stop and the link is pasted. Okay. That's it."),
        "{}",
        kept(&t)
    );
    assert!(
        list.cuts.iter().any(|c| c.kind == CutKind::LineBeforeCue
            && !c.applied
            && c.removed == "You press stop and the link is pasted. Shroom cut."),
        "{:?}",
        list.cuts
    );
}

#[test]
fn a_sentence_ending_on_is_after_a_pronoun_or_under_a_question_word_is_whole() {
    for sentence in [
        &[
            ("That's", 0.5, 0.8),
            ("what", 0.85, 1.0),
            ("it", 1.05, 1.15),
            ("is.", 1.2, 1.5),
        ][..],
        &[
            ("Here's", 0.5, 0.8),
            ("where", 0.85, 1.0),
            ("your", 1.05, 1.15),
            ("files", 1.2, 1.5),
            ("are.", 1.55, 1.8),
        ][..],
    ] {
        let words: Vec<(&str, f64, f64)> = sentence
            .iter()
            .copied()
            .chain([
                ("Shroom", 2.6, 3.0),
                ("cut.", 3.05, 3.5),
                ("Okay,", 4.3, 4.7),
                ("next.", 4.75, 5.2),
            ])
            .collect();
        let t = take(
            &words,
            &[(0.0, 0.5), (1.8, 2.6), (3.5, 4.3), (5.2, 6.0)],
            6.0,
        );
        let kinds: Vec<(CutKind, bool)> = plan(&t, Options::default())
            .cuts
            .iter()
            .map(|c| (c.kind, c.applied))
            .collect();
        assert_eq!(
            kinds,
            [(CutKind::CueOnly, true), (CutKind::LineBeforeCue, false)],
            "{sentence:?}"
        );
    }
}

#[test]
fn the_judgment_never_begins_inside_a_finished_sentence_before_the_cues_own() {
    // His second take's last cue: “Today [pause] I'll show you … in one click.” is whole, so
    // a start at “I'll” would cut half of it.
    let qs = questions(&rebuilt("hand-bagrat-tester-2"), Options::default());
    let last = qs.last().unwrap();
    assert_eq!(
        last.candidates,
        [
            "click, you press stop and the link is pasted,",
            "Today I'll show you how to share a recording in one click. click, you press stop and the link is pasted,",
        ]
    );
}

#[test]
fn the_judgment_never_begins_a_word_or_two_into_a_sentence() {
    // “So [pause] you open the menu and choose the, the...”: a start at “you” leaves “So”.
    // “To share a recording, you click the mushroom [pause] and then the, the...”: a start
    // at “and” leaves seven words behind it, and is one.
    let so = take(
        &[
            ("Okay.", 0.5, 0.9),
            ("So", 1.5, 1.7),
            ("you", 2.3, 2.4),
            ("open", 2.45, 2.7),
            ("the", 2.75, 2.8),
            ("menu", 2.85, 3.1),
            ("and", 3.15, 3.25),
            ("choose", 3.3, 3.6),
            ("the,", 3.65, 3.8),
            ("the...", 4.0, 4.4),
            ("Shroom", 5.2, 5.6),
            ("cut.", 5.65, 6.1),
            ("Open", 7.0, 7.3),
            ("Share.", 7.35, 7.8),
        ],
        &[
            (0.0, 0.5),
            (0.9, 1.5),
            (1.7, 2.3),
            (4.4, 5.2),
            (6.1, 7.0),
            (7.8, 8.5),
        ],
        8.5,
    );
    let qs = questions(&so, Options::default());
    assert!(
        qs[0].candidates.iter().all(|c| !c.starts_with("you ")),
        "{:?}",
        qs[0].candidates
    );
    assert!(
        qs[0]
            .candidates
            .iter()
            .any(|c| c.starts_with("So you open")),
        "{:?}",
        qs[0].candidates
    );
    let mushroom = take(
        &[
            ("To", 0.5, 0.6),
            ("share", 0.65, 0.9),
            ("a", 0.95, 1.0),
            ("recording,", 1.05, 1.6),
            ("you", 1.65, 1.75),
            ("click", 1.8, 2.05),
            ("the", 2.1, 2.15),
            ("mushroom", 2.2, 2.7),
            ("and", 3.3, 3.4),
            ("then", 3.45, 3.6),
            ("the,", 3.65, 3.8),
            ("the...", 4.0, 4.4),
            ("Shroom", 5.2, 5.6),
            ("cut.", 5.65, 6.1),
            ("Then", 7.0, 7.2),
            ("you", 7.25, 7.35),
            ("pick", 7.4, 7.6),
            ("a", 7.65, 7.7),
            ("window.", 7.75, 8.2),
        ],
        &[(0.0, 0.5), (2.7, 3.3), (4.4, 5.2), (6.1, 7.0), (8.2, 9.0)],
        9.0,
    );
    let qs = questions(&mushroom, Options::default());
    assert!(
        qs[0].candidates.iter().any(|c| c == "and then the, the..."),
        "{:?}",
        qs[0].candidates
    );
}

/// His third take's loudness every 50 ms where it was read out (relayed, 2026-10-18) around
/// the ends of its two cuts; elsewhere drawn from the words (-20 dB), the silences (-118 dB,
/// the true silence his browser gave) and between them a breath's -60 dB.
fn with_his_loudness(t: Transcript) -> Transcript {
    const READ: &[(f64, f32)] = &[
        (9.60, -22.0),
        (9.65, -40.0),
        (9.70, -56.0),
        (9.75, -38.0),
        (9.80, -45.0),
        (9.85, -59.0),
        (9.90, -76.0),
        (9.95, -106.0),
        (10.00, -100.0),
        (10.05, -60.0),
        (10.10, -60.0),
        (10.15, -60.0),
        (10.20, -60.0),
        (10.25, -60.0),
        (10.30, -60.0),
        (10.35, -60.0),
        (19.50, -20.0),
        (19.55, -32.0),
        (19.60, -59.0),
        (19.65, -35.0),
        (19.70, -34.0),
        (19.75, -37.0),
        (19.80, -49.0),
        (19.85, -59.0),
        (19.90, -68.0),
        (19.95, -93.0),
        (20.00, -50.0),
        (20.05, -58.0),
        (20.10, -85.0),
        (20.15, -114.0),
        (20.20, -118.0),
        (20.25, -118.0),
        (20.30, -118.0),
        (20.35, -118.0),
        (20.40, -118.0),
        (20.45, -118.0),
        (20.50, -82.0),
        (20.55, -71.0),
        (20.60, -72.0),
        (20.65, -92.0),
        (20.70, -103.0),
        (20.75, -82.0),
        (20.80, -82.0),
        (20.85, -82.0),
    ];
    let step = 0.05;
    let db = (0..(t.duration / step).ceil() as usize)
        .map(|i| {
            let at = i as f64 * step;
            if let Some(&(_, d)) = READ.iter().find(|(r, _)| (r - at).abs() < 1e-6) {
                d
            } else if t.words.iter().any(|w| w.start <= at && at < w.end) {
                -20.0
            } else if t.silences.iter().any(|s| s.start <= at && at < s.end) {
                -118.0
            } else {
                -60.0
            }
        })
        .collect();
    t.with_levels(Levels { step, db })
}

#[test]
fn a_cut_after_the_cue_runs_through_its_tail_and_the_breath_to_the_pause() {
    // His third take: “…the link is pasted. Shroom cut. [the “t”'s release and a breath to
    // about 20.10 s, the pause at 20.15–20.48 s, an inhale] When you want…”. The cut ended in
    // the dip right after “cut.” (19.60 s), and the breath played (Bagrat, 2026-10-18).
    let third = rebuilt("hand-bagrat-tester-3");
    for t in [third.clone(), with_his_loudness(third)] {
        let by_ear = plan_with(&t, Options::default(), &judged("hand-bagrat-tester-3"));
        assert_eq!(
            by_ear
                .cuts
                .iter()
                .map(|c| c.removed.as_str())
                .collect::<Vec<_>>(),
            [
                "Today, I will show you how to share, um, Shroom cut.",
                "You press stop and the link is pasted. Shroom cut.",
            ]
        );
        // The code alone cuts only the cue there, and ends it in the same place.
        let alone = plan(&t, Options::default());
        let cue_only = alone
            .cuts
            .iter()
            .find(|c| c.kind == CutKind::CueOnly)
            .unwrap();
        assert_eq!(cue_only.removed, "Shroom cut.");
        for cut in [&by_ear.cuts[1], cue_only] {
            assert!(
                (20.15..=20.48).contains(&cut.end),
                "the cut ends in the pause, past the breath: {cut:?}"
            );
        }
    }
    // His first cut ended right: in the quiet after its cue, before the inhale that leads
    // into the line said again.
    let t = with_his_loudness(rebuilt("hand-bagrat-tester-3"));
    let first = &plan_with(&t, Options::default(), &judged("hand-bagrat-tester-3")).cuts[0];
    assert!((9.95..10.05).contains(&first.end), "{first:?}");
}

#[test]
fn his_fourth_take_cuts_two_restarts_and_a_run_on_cue_each_ending_in_a_pause() {
    // “Hi, this is a quick test of Shroom. Today I'll show you how to, no, shroom cut. Hi, this
    // is… how to share, no, shroom cut. Hi, this is… in one click. you press stop and the link
    // is pasted and shroom cut you press stop and the link is copied and that's it” (Bagrat,
    // 2026-10-18): each restart said again from its start, the “no,” before each cue going
    // with it, and a cue run straight on from “and” with no sentence's end.
    let t = rebuilt("hand-bagrat-tester-4");
    let list = plan_with(&t, Options::default(), &judged("hand-bagrat-tester-4"));
    assert_eq!(
        list.cuts
            .iter()
            .map(|c| (c.applied, c.removed.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                true,
                "Hi, this is a quick test of Shroom. Today I'll show you how to, no, shroom cut."
            ),
            (
                true,
                "Hi, this is a quick test of shroom. Today I'll show you how to share, no, shroom cut."
            ),
            (true, "you press stop and the link is pasted and shroom cut"),
        ]
    );
    for c in &list.cuts {
        assert!(
            t.silences
                .iter()
                .any(|s| s.start <= c.end && c.end <= s.end),
            "it ends in a pause: {c:?}"
        );
    }
    // A word plays when its sound does: Whisper starts “you” inside the pause before it, so
    // its sound is counted from that pause's end.
    let keep = list.keep();
    let plays: Vec<&str> = t
        .words
        .iter()
        .filter(|w| {
            let from = t
                .silences
                .iter()
                .find(|s| s.start <= w.start && w.start < s.end)
                .map_or(w.start, |s| s.end.min(w.end));
            let mid = (from + w.end) / 2.0;
            keep.iter().any(|(s, e)| mid >= *s && mid < *e)
        })
        .map(|w| w.text.as_str())
        .collect();
    assert_eq!(
        plays.join(" "),
        "Hi, this is a quick test of shroom. Today I'll show you how to share a recording in one click. you press stop and the link is copied and that's it"
    );
}
