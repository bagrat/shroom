//! A cut's edges where no silence was found (spec N65 (2): the edges set in the quiet). When
//! the speaker runs a cut word straight into a kept one, the gap between their transcribed
//! times is all there is, and those times drift by a tenth of a second or more: an edge put
//! there can leave the tail of a cut word playing (Bagrat on the tester, 2026-10-18). Given
//! the take's loudness, step by step, the edge goes to the quietest point between them.

use shroom_edit::{Levels, Options, Silence, Transcript, plan};

/// “…link is pasted, strum cut, when you want…”: Whisper ends “cut,” at 27.95 s, but the
/// word sounds until 28.20 s; the dip before “when” is at 28.25 s, too short to be a silence.
fn ran_on(levels: bool) -> Transcript {
    let words: Vec<(String, f64, f64)> = [
        ("Click", 0.5, 0.8),
        ("on", 0.85, 0.95),
        ("this", 1.0, 1.2),
        ("link", 1.25, 1.55),
        ("and", 1.6, 1.7),
        ("you", 1.75, 1.85),
        ("will", 1.9, 2.2),
        ("shroom", 2.9, 3.3),
        ("cut,", 3.35, 3.55),
        ("when", 3.9, 4.1),
        ("you", 4.15, 4.25),
        ("want", 4.3, 4.6),
        ("it.", 4.65, 5.0),
    ]
    .iter()
    .map(|(w, s, e)| (w.to_string(), *s, *e))
    .collect();
    let t = Transcript::heard(
        6.0,
        &words,
        [(0.0, 0.5), (2.2, 2.9), (5.0, 6.0)]
            .iter()
            .map(|&(start, end)| Silence { start, end })
            .collect(),
    );
    if !levels {
        return t;
    }
    // 10 ms steps: speech at -20 dB, quiet at -55 dB, the cut word's tail to 3.80 s and the
    // dip between it and “when” at 3.82–3.86 s.
    let db = (0..600)
        .map(|i| {
            let at = i as f64 * 0.01;
            let quiet =
                at < 0.5 || (2.2..2.9).contains(&at) || at >= 5.0 || (3.82..3.87).contains(&at);
            if quiet { -55.0 } else { -20.0 }
        })
        .collect();
    t.with_levels(Levels { step: 0.01, db })
}

#[test]
fn an_edge_with_no_silence_goes_to_the_quietest_point_between_the_words() {
    let cut = &plan(&ran_on(true), Options::default()).cuts[0];
    assert!(
        (3.82..3.87).contains(&cut.end),
        "the cut ends in the dip, after the cut word's tail: {cut:?}"
    );
    // Without the loudness it can only go by the words' times.
    let cut = &plan(&ran_on(false), Options::default()).cuts[0];
    assert!(cut.end < 3.82, "{cut:?}");
}

#[test]
fn the_loudness_rides_with_the_transcript_and_an_old_one_reads_without_it() {
    let t = ran_on(true);
    let json = serde_json::to_string(&t).unwrap();
    let back: Transcript = serde_json::from_str(&json).unwrap();
    assert_eq!(back.levels.as_ref().map(|l| l.db.len()), Some(600));
    let old = r#"{"duration": 1.0, "words": [], "silences": []}"#;
    assert!(
        serde_json::from_str::<Transcript>(old)
            .unwrap()
            .levels
            .is_none()
    );
}
