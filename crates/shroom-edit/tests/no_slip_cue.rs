//! A cue with nothing said again, never cut unasked (N65 (2), 2026-10-25):
//! a take of Bagrat's on the early build tried the no-slip case, "shroom cut" said between
//! two halves of one sentence ("…a good feature, shroom cut, because it…"). The planner read
//! the half before it as a sentence the cue broke off, and cut it, sure, with nothing on the
//! owner page asking to check it. Rebuilt here in invented words of the same shape. Unless
//! the judgment hears the sentence said again after the cue, nothing was: it's a cue with
//! nothing said again, and its line comes cut and asking to be checked. A slip the judgment
//! hears said again in other words, in the same shape (his tester take's "…the link is
//! pasted, strum cut, when you want…"), is cut sure as before.

use std::fs;
use std::path::Path;

use shroom_edit::{
    Answer, Confidence, CutKind, Options, Silence, Transcript, plan, plan_with, questions,
};

fn take(lines: &[(&str, f64, f64)], duration: f64) -> Transcript {
    // Each line's words spread over its time, the quiet between the lines.
    let mut words = Vec::new();
    for &(line, a, b) in lines {
        let ws: Vec<&str> = line.split_whitespace().collect();
        let step = (b - a) / ws.len() as f64;
        for (i, w) in ws.iter().enumerate() {
            let s = a + i as f64 * step;
            words.push((w.to_string(), s, s + 0.9 * step));
        }
    }
    let silences = words
        .windows(2)
        .filter(|p| p[1].1 - p[0].2 > 0.3)
        .map(|p| Silence {
            start: p[0].2 + 0.05,
            end: p[1].1 - 0.05,
        })
        .collect();
    Transcript::heard(duration, &words, silences)
}

/// The judgment hearing nothing said again: every start scored low.
fn nothing_again(t: &Transcript) -> Vec<Answer> {
    questions(t, Options::default())
        .iter()
        .map(|q| Answer {
            cue: q.cue,
            scores: vec![0.1; q.candidates.len()],
        })
        .collect()
}

const NO_SLIP: &[(&str, f64, f64)] = &[
    ("Here is the next one.", 0.0, 3.0),
    ("So,", 5.0, 5.5),
    ("this is a neat feature,", 6.5, 7.5),
    ("shroom cut,", 8.5, 9.2),
    ("because it lets you,", 10.0, 11.8),
    (
        "trim the video as you talk and never record it again.",
        13.0,
        19.0,
    ),
];

#[test]
fn a_no_slip_cue_inside_a_sentence_asks_to_check_its_cut() {
    let t = take(NO_SLIP, 21.0);
    let cuts = plan_with(&t, Options::default(), &nothing_again(&t)).cuts;
    let kinds: Vec<_> = cuts.iter().map(|c| (c.kind, c.confidence)).collect();
    assert_eq!(
        kinds,
        [
            (CutKind::CueOnly, Confidence::High),
            (CutKind::LineBeforeCue, Confidence::Low)
        ],
        "{cuts:?}"
    );
    assert_eq!(cuts[0].removed, "shroom cut,");
    assert_eq!(cuts[1].removed, "this is a neat feature, shroom cut,");
}

#[test]
fn with_no_judgment_it_still_asks_to_check() {
    // The judgment couldn't answer (the free day used up, a busy model): nothing says the
    // sentence was said again, so the same words are cut and the strip asks.
    let t = take(NO_SLIP, 21.0);
    let cuts = plan(&t, Options::default()).cuts;
    assert_eq!(cuts.len(), 2, "{cuts:?}");
    assert_eq!(
        (cuts[1].kind, cuts[1].confidence, cuts[1].removed.as_str()),
        (
            CutKind::LineBeforeCue,
            Confidence::Low,
            "this is a neat feature, shroom cut,"
        )
    );
}

#[test]
fn a_cue_that_ends_its_sentence_asks_to_check_the_one_it_broke_off() {
    // "…and your link is. Shroom cut. Okay.": nothing said again, so it asks too. Run on
    // from the finished sentence before it with no pause, so only the words say where the
    // broken-off one began ("is" after a noun ends nothing): the finished one stays.
    let t = take(
        &[
            (
                "Here is the next one. When you want it, just press stop and your link is.",
                0.0,
                8.0,
            ),
            ("Shroom cut.", 9.0, 9.7),
            ("Okay.", 10.5, 11.0),
        ],
        12.0,
    );
    let cuts = plan_with(&t, Options::default(), &nothing_again(&t)).cuts;
    assert_eq!(cuts.len(), 2, "{cuts:?}");
    assert_eq!(
        (cuts[1].kind, cuts[1].removed.as_str()),
        (
            CutKind::LineBeforeCue,
            "When you want it, just press stop and your link is. Shroom cut."
        )
    );
}

#[test]
fn a_broken_off_sentence_the_judgment_heard_said_again_is_cut_sure() {
    let t = take(
        &[
            ("Here is the next one.", 0.0, 3.0),
            ("Click on this link and you will", 4.0, 7.0),
            ("shroom cut.", 8.0, 8.7),
            ("To get your video, use this link.", 9.5, 12.0),
        ],
        13.0,
    );
    let said_again: Vec<Answer> = questions(&t, Options::default())
        .iter()
        .map(|q| Answer {
            cue: q.cue,
            scores: (0..q.candidates.len())
                .map(|i| if i == 0 { 0.8 } else { 0.1 })
                .collect(),
        })
        .collect();
    let cuts = plan_with(&t, Options::default(), &said_again).cuts;
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(
        (cuts[0].kind, cuts[0].confidence, cuts[0].removed.as_str()),
        (
            CutKind::Retake,
            Confidence::High,
            "Click on this link and you will shroom cut."
        )
    );
}

#[test]
fn a_slip_the_judgment_heard_said_again_is_cut_as_before() {
    // His tester take, with what the judgment answered for it (2026-10-18).
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/takes/hand-bagrat-tester-2");
    let t: Transcript =
        serde_json::from_str(&fs::read_to_string(dir.join("transcript.json")).unwrap()).unwrap();
    #[derive(serde::Deserialize)]
    struct Judged {
        answers: Vec<Answer>,
    }
    let judged: Judged =
        serde_json::from_str(&fs::read_to_string(dir.join("judged.json")).unwrap()).unwrap();
    let cuts = plan_with(&t, Options::default(), &judged.answers).cuts;
    let last = cuts.last().unwrap();
    assert_eq!(
        (last.kind, last.confidence, last.removed.as_str()),
        (
            CutKind::Retake,
            Confidence::High,
            "click, you press stop and the link is pasted, strum cut,"
        )
    );
}

#[test]
fn the_judgment_hearing_only_the_sentence_s_tail_said_again_still_asks() {
    // A pause of 0.4–0.6 s inside the sentence begins a start the judgment may score: it
    // heard "which allows trimming" said again, and not the sentence's head.
    let t = take(
        &[
            ("Here is the next one.", 0.0, 3.0),
            ("So this is a really neat feature", 4.0, 6.0),
            ("which allows trimming,", 6.5, 7.5),
            ("shroom cut,", 8.3, 9.0),
            ("that lets you cut the video as you talk.", 9.8, 12.5),
        ],
        13.5,
    );
    let qs = questions(&t, Options::default());
    assert_eq!(qs[0].candidates[0], "which allows trimming,");
    let tail: Vec<Answer> = qs
        .iter()
        .map(|q| Answer {
            cue: q.cue,
            scores: (0..q.candidates.len())
                .map(|i| if i == 0 { 0.8 } else { 0.1 })
                .collect(),
        })
        .collect();
    let cuts = plan_with(&t, Options::default(), &tail).cuts;
    assert_eq!(cuts.len(), 2, "{cuts:?}");
    assert_eq!(
        (cuts[1].kind, cuts[1].confidence, cuts[1].removed.as_str()),
        (
            CutKind::LineBeforeCue,
            Confidence::Low,
            "So this is a really neat feature which allows trimming, shroom cut,"
        )
    );
}
