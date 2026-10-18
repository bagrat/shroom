//! Which spoken cues the planner hears (spec N65 (1)), as Bagrat asked to try (2026-10-18,
//! relayed by the orchestrator): plain “cut”, and a relaxed set (“cut here”, “cut that”,
//! “cut cut”, “scratch that”, “redo”, “let me redo that”…). “shroom cut” is distinctive
//! enough to stand alone, and stays the default. Any other phrase is a cue only when it is
//! said apart from the speech (a sentence of its own, or a pause either side) and the
//! judgment says the speaker meant it for whoever edits the video; with no judgment, only
//! “shroom cut” counts.

use shroom_edit::{
    CueWords, Meant, Options, Said, Silence, Transcript, find_cues_in, intents, plan_meant,
};

/// Words a second apart unless a time is given: (text, start, end).
fn take(words: &[(&str, f64, f64)]) -> Transcript {
    let end = words.last().map_or(0.0, |w| w.2);
    let words: Vec<(String, f64, f64)> = words
        .iter()
        .map(|(w, s, e)| (w.to_string(), *s, *e))
        .collect();
    Transcript::heard(end + 1.0, &words, Vec::<Silence>::new())
}

/// A slip, the cue as `cue` (one word or several, said apart), and the line said again.
fn slipped(cue: &[&str]) -> Transcript {
    let mut words = vec![
        ("Open", 0.5, 0.8),
        ("the", 0.85, 0.95),
        ("menu", 1.0, 1.3),
        ("and", 1.35, 1.45),
        ("choose", 1.5, 1.8),
        ("the,", 1.85, 2.0),
        ("the...", 2.2, 2.6),
    ];
    let mut at = 3.4;
    for w in cue {
        words.push((w, at, at + 0.35));
        at += 0.4;
    }
    at += 0.8;
    for w in ["Open", "the", "menu", "and", "choose", "Share."] {
        words.push((w, at, at + 0.3));
        at += 0.35;
    }
    take(&words)
}

fn opts(cues: CueWords) -> Options {
    Options {
        cues,
        ..Options::default()
    }
}

#[test]
fn shroom_cut_alone_is_heard_by_default_and_needs_no_judgment() {
    let t = slipped(&["Shroom", "cut."]);
    let cues = find_cues_in(&t.words, CueWords::Shroom);
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].said, Said::ShroomCut);
    assert!(intents(&t, Options::default()).is_empty());
    for set in [CueWords::Plain, CueWords::Relaxed] {
        assert!(intents(&t, opts(set)).is_empty(), "{set:?}");
        assert_eq!(plan_meant(&t, opts(set), &[], &[]).cuts.len(), 1, "{set:?}");
    }
}

#[test]
fn plain_cut_is_heard_only_said_apart_and_only_when_the_judgment_says_it_was_meant() {
    let t = slipped(&["Cut."]);
    // The default set doesn't hear it.
    assert!(find_cues_in(&t.words, CueWords::Shroom).is_empty());
    let cues = find_cues_in(&t.words, CueWords::Plain);
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].said, Said::Cut);
    // The planner asks whether it was meant for the editor, with what was said around it.
    let asked = intents(&t, opts(CueWords::Plain));
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].said, "Cut.");
    assert!(
        asked[0].before.ends_with("choose the, the..."),
        "{:?}",
        asked[0]
    );
    assert!(
        asked[0].after.starts_with("Open the menu"),
        "{:?}",
        asked[0]
    );
    // No judgment, or one that says no: nothing is cut.
    assert!(
        plan_meant(&t, opts(CueWords::Plain), &[], &[])
            .cuts
            .is_empty()
    );
    let no = [Meant {
        word: asked[0].word,
        score: 0.1,
    }];
    assert!(
        plan_meant(&t, opts(CueWords::Plain), &no, &[])
            .cuts
            .is_empty()
    );
    // Meant: the slip goes with it.
    let yes = [Meant {
        word: asked[0].word,
        score: 0.9,
    }];
    let list = plan_meant(&t, opts(CueWords::Plain), &yes, &[]);
    assert_eq!(list.cuts.len(), 1);
    assert!(list.cuts[0].applied);
    assert!(list.cuts[0].removed.ends_with("Cut."), "{:?}", list.cuts[0]);
}

#[test]
fn cut_in_the_flow_of_speech_is_never_asked_about() {
    for s in [
        "You can cut the start of a video in the editor.",
        "We cut the price in half last year.",
        "Then cut that part and paste it below.",
        "It's a quick test of the voice cut and it works.",
    ] {
        let words: Vec<(String, f64, f64)> = s
            .split_whitespace()
            .enumerate()
            .map(|(i, w)| (w.to_string(), 0.5 + i as f64 * 0.3, 0.75 + i as f64 * 0.3))
            .collect();
        let t = Transcript::heard(10.0, &words, Vec::new());
        assert!(intents(&t, opts(CueWords::Relaxed)).is_empty(), "{s}");
    }
}

#[test]
fn the_relaxed_set_hears_the_other_ways_of_saying_it() {
    for (cue, said) in [
        (&["Cut", "here."][..], Said::CutHere),
        (&["Cut", "that."][..], Said::CutThat),
        (&["Cut,", "cut."][..], Said::CutCut),
        (&["Scratch", "that."][..], Said::ScratchThat),
        (&["Redo."][..], Said::Redo),
        (&["Let", "me", "redo", "that."][..], Said::Redo),
        (
            &["Let", "me", "say", "that", "again."][..],
            Said::SayThatAgain,
        ),
    ] {
        let t = slipped(cue);
        let cues = find_cues_in(&t.words, CueWords::Relaxed);
        assert_eq!(cues.len(), 1, "{cue:?}");
        assert_eq!(cues[0].said, said, "{cue:?}");
        assert_eq!(cues[0].last - cues[0].first + 1, cue.len(), "{cue:?}");
        // Plain hears none of them but a lone "cut".
        assert!(
            find_cues_in(&t.words, CueWords::Plain)
                .iter()
                .all(|c| c.said == Said::Cut),
            "{cue:?}"
        );
    }
}
