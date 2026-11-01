//! A cue the hearing wrote into silence (N65 (1), 2026-10-25): a take of Bagrat's on the
//! early build, with no speech in it, came back from the space with "Shroom cut." in its
//! first seconds, the prime the hearing is asked with echoed into the quiet, and the
//! planner cut from the take's start through it. Rebuilt here from what the take itself
//! shows: its quiet at the planner's level (−38 dBFS, measured on the public stream), and
//! the cue where the space put it and where Whisper puts it when the same sound is heard
//! again, primed as the space primes it. A cue said aloud, its sound under its words or a
//! few tenths off them, is cut as before.

use shroom_edit::{Options, Silence, Transcript, intents, plan, questions};

/// The take's quiet: all of it but a few blips, each under a tenth of a second.
const QUIET: &[(f64, f64)] = &[
    (0.0, 6.369),
    (6.369, 10.176),
    (10.187, 13.004),
    (13.006, 13.198),
    (13.211, 29.749),
    (29.756, 30.239),
    (30.290, 38.592),
];
const LENGTH: f64 = 38.592;

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

#[test]
fn a_cue_heard_in_the_silent_start_of_a_take_is_never_cut() {
    // Anywhere in the quiet the take opens with: as built, the planner cut 0.12–6.249 s, both
    // edges in that quiet, the take's cut to the millisecond.
    let t = take(&[("Shroom", 0.0, 4.7), ("cut.", 4.7, 5.6)], QUIET, LENGTH);
    assert!(plan(&t, Options::default()).cuts.is_empty());
    assert!(questions(&t, Options::default()).is_empty());
}

#[test]
fn the_space_s_own_hearing_of_the_take_cuts_nothing() {
    // The take heard again as the space hears it (whisper-large-v3-turbo, primed, filtered):
    // the cue in the silent start, and Whisper's stock words for a quiet room after it.
    let t = take(
        &[
            ("Shroom", 2.29, 2.95),
            ("cut.", 2.95, 2.99),
            ("Thank", 21.72, 21.94),
            ("you", 21.94, 21.94),
            ("so", 29.78, 31.46),
            ("much", 31.46, 31.46),
            ("for", 31.46, 31.66),
            ("joining", 31.66, 31.68),
            ("us.", 31.68, 31.7),
        ],
        QUIET,
        LENGTH,
    );
    assert!(plan(&t, Options::default()).cuts.is_empty());
}

#[test]
fn a_cue_heard_twice_in_silence_is_never_cut() {
    let t = take(
        &[
            ("Shroom", 28.58, 29.98),
            ("cut.", 29.98, 29.98),
            ("Shroom", 30.0, 30.36),
            ("cut.", 30.36, 30.76),
        ],
        QUIET,
        LENGTH,
    );
    assert!(plan(&t, Options::default()).cuts.is_empty());
}

#[test]
fn a_cue_over_a_blip_of_sound_is_still_silence() {
    // Whisper heard it here, primed and filtered: two blips of 0.05 s inside its reach.
    let t = take(
        &[("Shroom", 30.4, 31.8), ("cut.", 31.8, 31.8)],
        QUIET,
        LENGTH,
    );
    assert!(plan(&t, Options::default()).cuts.is_empty());
}

#[test]
fn the_judgment_is_never_asked_about_a_cue_in_silence() {
    let t = take(&[("Shroom", 0.0, 4.7), ("cut.", 4.7, 5.6)], QUIET, LENGTH);
    let wider = Options {
        cues: shroom_edit::CueWords::Relaxed,
        ..Options::default()
    };
    assert!(intents(&t, wider).is_empty());
    assert!(questions(&t, wider).is_empty());
    assert!(plan(&t, wider).cuts.is_empty());
}

/// A line, the cue, and the line again, with the sound under the words as said: quiet only
/// before, between and after them.
const SAID: &[(&str, f64, f64)] = &[
    ("This", 1.0, 1.2),
    ("is", 1.2, 1.35),
    ("the", 1.35, 1.5),
    ("first", 1.5, 1.9),
    ("line.", 1.9, 2.4),
    ("Shroom", 3.0, 3.35),
    ("cut.", 3.35, 3.7),
    ("This", 4.4, 4.6),
    ("is", 4.6, 4.75),
    ("the", 4.75, 4.9),
    ("first", 4.9, 5.3),
    ("line.", 5.3, 5.8),
];
const SAID_QUIET: &[(f64, f64)] = &[(0.0, 0.95), (2.45, 2.95), (3.75, 4.35), (5.85, 7.0)];

#[test]
fn a_cue_said_aloud_is_cut_as_before() {
    let t = take(SAID, SAID_QUIET, 7.0);
    let cuts = plan(&t, Options::default()).cuts;
    assert_eq!(cuts.len(), 1);
    assert!(cuts[0].start < 1.0 && cuts[0].end > 3.7 && cuts[0].end < 4.4);
}

#[test]
fn a_cue_whose_times_drifted_off_its_sound_is_still_heard() {
    // The words a few tenths late against the sound: the cue's sound is 3.0–3.7 s.
    let drifted: Vec<(&str, f64, f64)> = SAID
        .iter()
        .map(|&(w, s, e)| {
            if w.starts_with("Shroom") || w == "cut." {
                (w, s + 0.4, e + 0.4)
            } else {
                (w, s, e)
            }
        })
        .collect();
    let t = take(&drifted, SAID_QUIET, 7.0);
    assert_eq!(plan(&t, Options::default()).cuts.len(), 1);
}

#[test]
fn with_the_takes_sound_unknown_a_cue_is_taken_as_heard() {
    // No quiet found (the take's own copy unread): the words alone decide, as before.
    let t = take(&[("Shroom", 0.0, 4.7), ("cut.", 4.7, 5.6)], &[], LENGTH);
    assert_eq!(plan(&t, Options::default()).cuts.len(), 1);
}
