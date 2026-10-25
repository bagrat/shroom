//! The words around each voice cut, for a review list (spec N65 (5)): the owner page shows
//! a few words before and after each cut, so the owner hears where it joins. The planner
//! gives up to six either side: `leads` before the cut's first word, `resumes` after its
//! last, and nothing where the take starts or ends.

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

const HI: &[(&str, f64, f64)] = &[
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
const TO: &[(&str, f64, f64)] = &[
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

fn between(middle: &[(&str, f64, f64)]) -> Transcript {
    let words: Vec<(&str, f64, f64)> = HI
        .iter()
        .chain(middle.iter())
        .chain(TO.iter())
        .copied()
        .collect();
    take(&words, QUIET, 18.9)
}

#[test]
fn a_cut_carries_six_words_either_side() {
    // Bagrat's reworded redo: the broken-off sentence goes with the cue, as the line before
    // it while no judgment says it was said again (N65 (2), 2026-10-25).
    let t = between(&[
        ("Click", 6.5, 6.8),
        ("on", 6.85, 6.95),
        ("this", 7.0, 7.2),
        ("link", 7.25, 7.55),
        ("and", 7.6, 7.7),
        ("you", 7.75, 7.85),
        ("will", 7.9, 8.2),
        ("shroom", 8.9, 9.3),
        ("cut.", 9.35, 9.8),
    ]);
    let cuts = plan(&t, Options::default()).cuts;
    assert_eq!(cuts.len(), 2, "{cuts:?}");
    assert_eq!(
        cuts[1].leads.as_deref(),
        Some("quick test of the voice cut.")
    );
    assert_eq!(
        cuts[1].resumes.as_deref(),
        Some("To get your video, use this")
    );
}

#[test]
fn a_cue_with_the_line_before_it_cut_gives_both_the_words_around_them() {
    let t = between(&[
        ("Click", 6.5, 6.8),
        ("on", 6.85, 6.95),
        ("this", 7.0, 7.2),
        ("link", 7.25, 7.55),
        ("to", 7.6, 7.7),
        ("see", 7.75, 7.95),
        ("it.", 8.0, 8.2),
        ("shroom", 8.9, 9.3),
        ("cut.", 9.35, 9.8),
    ]);
    let cuts = plan(&t, Options::default()).cuts;
    let around: Vec<(CutKind, Option<&str>, Option<&str>)> = cuts
        .iter()
        .map(|c| (c.kind, c.leads.as_deref(), c.resumes.as_deref()))
        .collect();
    assert_eq!(
        around,
        [
            (
                CutKind::CueOnly,
                Some("on this link to see it."),
                Some("To get your video, use this")
            ),
            (
                CutKind::LineBeforeCue,
                Some("quick test of the voice cut."),
                Some("To get your video, use this")
            ),
        ]
    );
}

#[test]
fn nothing_leads_into_a_cut_at_the_start_and_nothing_resumes_after_one_at_the_end() {
    // “Hello, that is shroom. Shroom cut. Hello, this is shroom.”: the slip is the take's
    // first words.
    let t = take(
        &[
            ("Hello,", 0.5, 0.9),
            ("that", 1.0, 1.2),
            ("is", 1.25, 1.4),
            ("shroom.", 1.45, 1.9),
            ("Shroom", 2.6, 3.0),
            ("cut.", 3.05, 3.5),
            ("Hello,", 4.3, 4.7),
            ("this", 4.8, 5.0),
            ("is", 5.05, 5.2),
            ("shroom.", 5.25, 5.8),
        ],
        &[(0.0, 0.5), (1.9, 2.6), (3.5, 4.3), (5.8, 7.0)],
        7.0,
    );
    let cuts = plan(&t, Options::default()).cuts;
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(cuts[0].leads, None);
    assert_eq!(cuts[0].resumes.as_deref(), Some("Hello, this is shroom."));
    // A cue that ends the take: the line before it cut with it, nothing after either.
    let t = take(
        &[
            ("Thanks", 0.5, 0.9),
            ("for", 0.95, 1.1),
            ("watching,", 1.15, 1.7),
            ("and", 1.8, 1.95),
            ("see", 2.0, 2.2),
            ("you", 2.25, 2.4),
            ("in", 2.45, 2.55),
            ("the", 2.6, 2.7),
            ("next.", 2.75, 3.1),
            ("Shroom", 3.9, 4.3),
            ("cut.", 4.35, 4.8),
        ],
        &[(0.0, 0.5), (3.1, 3.9), (4.8, 6.0)],
        6.0,
    );
    let cuts = plan(&t, Options::default()).cuts;
    assert!(!cuts.is_empty(), "{cuts:?}");
    assert!(cuts.iter().all(|c| c.resumes.is_none()), "{cuts:?}");
}
