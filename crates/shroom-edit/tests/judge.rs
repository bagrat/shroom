//! The judgment above the planner (spec N65): mechanism deterministic, judgment above. The
//! code lists where a redone stretch might begin behind each cue; a judge (Clef) scores
//! whether what was said after the cue says all of it again; the code takes the farthest
//! start the judge scored past the bar, refuses anything that isn't one of its own, and
//! never lets a judgment shorten a cut the words already set. No judge: the code's own cut.

use shroom_edit::{
    Answer, CutKind, JUDGED, Options, Silence, Transcript, plan, plan_with, questions,
};

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

/// “Hi, this is a quick test of the voice cut. <middle> shroom cut. To get your video, use
/// this link and you'll see it right away.”
fn bagrats(middle: &[(&str, f64, f64)]) -> Transcript {
    let before = [
        ("Hi,", 0.5, 0.8),
        ("this", 1.1, 1.4),
        ("is", 1.45, 1.55),
        ("a", 1.6, 1.65),
        ("quick", 1.7, 2.0),
        ("test", 2.05, 2.35),
        ("of", 2.4, 2.5),
        ("the", 2.55, 2.65),
        ("voice", 2.7, 3.0),
        ("cut.", 3.05, 3.4),
    ];
    let cue = [("shroom", 8.9, 9.3), ("cut.", 9.35, 9.8)];
    let after = [
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
    let words: Vec<(&str, f64, f64)> = before
        .iter()
        .chain(middle.iter())
        .chain(cue.iter())
        .chain(after.iter())
        .copied()
        .collect();
    // A short breath after “Hi,” (0.3 s) is no place to begin: a long pause (0.4 s) would be.
    take(
        &words,
        &[
            (0.0, 0.5),
            (0.8, 1.1),
            (3.4, 6.5),
            (8.2, 8.9),
            (9.8, 11.3),
            (12.3, 12.5),
            (14.9, 18.9),
        ],
        18.9,
    )
}

const BROKEN: &[(&str, f64, f64)] = &[
    ("Click", 6.5, 6.8),
    ("on", 6.85, 6.95),
    ("this", 7.0, 7.2),
    ("link", 7.25, 7.55),
    ("and", 7.6, 7.7),
    ("you", 7.75, 7.85),
    ("will", 7.9, 8.2),
];
const FINISHED: &[(&str, f64, f64)] = &[
    ("Click", 6.5, 6.8),
    ("on", 6.85, 6.95),
    ("this", 7.0, 7.2),
    ("link", 7.25, 7.55),
    ("to", 7.6, 7.7),
    ("see", 7.75, 7.95),
    ("it.", 8.0, 8.2),
];

fn kept(t: &Transcript, answers: &[Answer]) -> String {
    let keep = plan_with(t, Options::default(), answers).keep();
    t.words
        .iter()
        .filter(|w| {
            keep.iter()
                .any(|(s, e)| (w.start + w.end) / 2.0 >= *s && (w.start + w.end) / 2.0 < *e)
        })
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

const REDONE: &str = "Hi, this is a quick test of the voice cut. To get your video, use this link and you'll see it right away.";

#[test]
fn a_question_holds_what_was_said_after_the_cue_and_where_the_redo_might_begin_nearest_first() {
    let qs = questions(&bagrats(FINISHED), Options::default());
    assert_eq!(qs.len(), 1);
    let q = &qs[0];
    assert_eq!(q.cue, 0);
    assert_eq!(
        q.after,
        "To get your video, use this link and you'll see it right away."
    );
    // Sentence starts and long pauses behind the cue, nearest first, back to the take's start.
    assert_eq!(
        q.candidates,
        [
            "Click on this link to see it.",
            "Hi, this is a quick test of the voice cut. Click on this link to see it."
        ]
    );
}

#[test]
fn the_farthest_start_scored_past_the_bar_is_cut_with_the_cue() {
    let t = bagrats(FINISHED);
    // No judge: the sentence before ended cleanly and nothing was said again word for word, so
    // the line before goes with the cue, unsure how far back the slip went (Bagrat, 2026-10-18).
    let alone = plan(&t, Options::default());
    assert_eq!(
        alone
            .cuts
            .iter()
            .map(|c| (c.kind, c.applied))
            .collect::<Vec<_>>(),
        [(CutKind::CueOnly, true), (CutKind::LineBeforeCue, true)]
    );
    assert_eq!(kept(&t, &[]), REDONE);
    // The judge hears the line said again in other words; the take's opening, not.
    let list = plan_with(
        &t,
        Options::default(),
        &[Answer {
            cue: 0,
            scores: vec![0.93, 0.08],
        }],
    );
    assert_eq!(list.cuts.len(), 1);
    assert_eq!(
        (
            list.cuts[0].kind,
            list.cuts[0].applied,
            list.cuts[0].removed.as_str()
        ),
        (
            CutKind::Retake,
            true,
            "Click on this link to see it. shroom cut."
        )
    );
    assert_eq!(
        kept(
            &t,
            &[Answer {
                cue: 0,
                scores: vec![0.93, 0.08]
            }]
        ),
        REDONE
    );
    // Both past the bar: the farthest back wins.
    assert!(
        kept(
            &t,
            &[Answer {
                cue: 0,
                scores: vec![0.9, JUDGED + 0.01]
            }]
        )
        .starts_with("To get your video")
    );
    // None past it: the code's own plan.
    assert_eq!(
        plan_with(
            &t,
            Options::default(),
            &[Answer {
                cue: 0,
                scores: vec![JUDGED - 0.01, 0.0]
            }]
        ),
        alone
    );
}

#[test]
fn an_answer_that_isnt_one_of_the_codes_own_is_refused() {
    let t = bagrats(FINISHED);
    let alone = plan(&t, Options::default());
    for wrong in [
        Answer {
            cue: 0,
            scores: vec![0.99],
        },
        Answer {
            cue: 0,
            scores: vec![0.99, 0.99, 0.99],
        },
        Answer {
            cue: 7,
            scores: vec![0.99, 0.0],
        },
        Answer {
            cue: 0,
            scores: vec![f64::NAN, 0.0],
        },
        Answer {
            cue: 0,
            scores: vec![1.5, 0.0],
        },
    ] {
        assert_eq!(
            plan_with(&t, Options::default(), &[wrong.clone()]),
            alone,
            "{wrong:?}"
        );
    }
}

#[test]
fn a_judgment_reaches_further_back_but_never_shortens_a_cut_the_words_set() {
    let t = bagrats(BROKEN);
    // The broken-off sentence goes by the code alone.
    assert_eq!(kept(&t, &[]), REDONE);
    // A judge that would keep part of it changes nothing: “Click on this link and you will”
    // is the nearest start there is, and a judgment never moves a cut forward.
    let qs = questions(&t, Options::default());
    assert_eq!(qs[0].candidates[0], "Click on this link and you will");
    assert_eq!(
        kept(
            &t,
            &[Answer {
                cue: 0,
                scores: vec![0.1, 0.0]
            }]
        ),
        REDONE
    );
    // One that says the opening was said again too takes it as well.
    assert_eq!(
        kept(
            &t,
            &[Answer {
                cue: 0,
                scores: vec![0.9, 0.9]
            }]
        ),
        "To get your video, use this link and you'll see it right away."
    );
}
