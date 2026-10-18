//! Landing a take's voice cues in its record's edit list (spec N65): the planner's cuts
//! beside whatever the editor saved (N63), the plan's mark set, once per take. The takes are
//! the POC's Workers AI ones (the model the space hears with, N66), handed over as the
//! space keeps its words: `[text, start, end]` on the recording's clock.

use std::fs;
use std::path::Path;

use shroom_edit::{Cut, CutKind, EditCopy, EditList, Options, Saved, Silence, Transcript, Voice, land, plan};

const NOW: u64 = 1_792_329_066_000;

fn heard(take: &str) -> Transcript {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/takes").join(take).join("transcript.json");
    let t: Transcript = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let words: Vec<(String, f64, f64)> = t.words.iter().map(|w| (w.text.clone(), w.start, w.end)).collect();
    Transcript::heard(t.duration, &words, t.silences.clone())
}

fn ids(s: &Saved) -> Vec<&str> {
    s.list.cuts.iter().map(|c| c.id.as_str()).collect()
}

#[test]
fn a_take_with_no_edit_gets_one_holding_its_cut_and_the_mark() {
    let s = land(None, &heard("wai-en_US-lessac-medium-1-the-example"), 8.7, NOW).expect("planned");
    assert_eq!(s.list.voice, Some(Voice { planned_at: NOW, cues: 1 }));
    // The list is the recording's, on its own length.
    assert_eq!((s.list.source_duration, s.list.in_, s.list.out), (8.7, 0.0, 8.7));
    assert_eq!(ids(&s), ["v1"]);
    let c = &s.list.cuts[0];
    assert_eq!((c.kind, c.applied), (CutKind::Retake, true));
    assert_eq!(c.removed, "Hello, that is Shroom. Shroom cut.");
    // What plays changed, so the revision moved on; no copy was cut from it yet.
    assert_eq!(s.rev, 1);
    assert_eq!(s.copy, None);
}

#[test]
fn the_editors_cuts_trim_and_copy_are_kept_and_the_copy_goes_stale() {
    let mut list = EditList::new(10.28);
    list.out = 10.0;
    list.cuts.push(Cut { id: "m1".into(), ..Cut::manual(9.0, 9.5) });
    let copy = EditCopy { rev: 3, file: EditCopy::file_for(3), duration: 9.5, poster: None, teaser: None };
    let saved = Saved { list, rev: 3, copy: Some(copy.clone()) };
    let s = land(Some(saved), &heard("wai-en_US-lessac-medium-4-no-retake"), 10.28, NOW).unwrap();
    assert_eq!((s.list.in_, s.list.out), (0.0, 10.0));
    assert_eq!(ids(&s), ["m1", "v1", "v2"]);
    // A cue with nothing said again: its own words cut, the line before it offered.
    assert_eq!((s.list.cuts[1].kind, s.list.cuts[1].applied), (CutKind::CueOnly, true));
    assert_eq!((s.list.cuts[2].kind, s.list.cuts[2].applied), (CutKind::LineBeforeCue, false));
    assert_eq!(s.list.voice, Some(Voice { planned_at: NOW, cues: 1 }));
    assert_eq!(s.rev, 4);
    assert_eq!(s.copy, Some(copy));
    assert!(s.current_copy().is_none(), "the copy was cut from the list before the cues");
}

#[test]
fn a_take_is_planned_once_so_a_cut_put_back_stays_put_back() {
    let t = heard("wai-en_US-lessac-medium-1-the-example");
    let mut first = land(None, &t, 8.7, NOW).unwrap();
    first.list.cuts[0].applied = false;
    assert_eq!(land(Some(first), &t, 8.7, NOW + 60_000), None);
}

#[test]
fn a_take_with_no_cue_is_marked_and_nothing_that_plays_moves() {
    // “shroom cut” only mentioned, never said as a cue.
    let t = heard("wai-en_US-lessac-medium-6-said-not-meant");
    let s = land(None, &t, 10.11, NOW).unwrap();
    assert!(s.list.cuts.is_empty());
    assert_eq!(s.list.voice, Some(Voice { planned_at: NOW, cues: 0 }));
    assert_eq!(s.rev, 0);
    assert!(s.list.is_identity());
    // An edit saved before the words came keeps its copy current.
    let list = EditList { out: 9.0, ..EditList::new(10.11) };
    let saved = Saved { list, rev: 2, copy: Some(EditCopy { rev: 2, file: EditCopy::file_for(2), duration: 9.0, poster: None, teaser: None }) };
    let s = land(Some(saved), &t, 10.11, NOW).unwrap();
    assert_eq!((s.rev, s.list.out), (2, 9.0));
    assert!(s.current_copy().is_some());
}

#[test]
fn cuts_are_named_in_the_order_of_their_cues() {
    let t = heard("wai-en_US-lessac-medium-5-two-cues");
    let planned = plan(&t, Options::default());
    assert_eq!(planned.cuts.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["v1", "v2"]);
    let s = land(None, &t, 13.02, NOW).unwrap();
    assert_eq!(ids(&s), ["v1", "v2"]);
    assert!(s.list.cuts[0].start < s.list.cuts[1].start);
    assert_eq!(s.list.voice.map(|v| v.cues), Some(2));
}

#[test]
fn a_cut_past_the_recordings_end_is_held_to_it() {
    // The space heard a little past the recording's own length (its last window's edge).
    let words = [
        ("This", 0.5, 0.7),
        ("is", 0.75, 0.9),
        ("the", 0.95, 1.05),
        ("end", 1.1, 1.4),
        ("of", 1.45, 1.55),
        ("it.", 1.6, 1.9),
        ("Shroom", 3.0, 3.4),
        ("cut.", 3.45, 3.9),
    ]
    .map(|(w, s, e)| (w.to_string(), s, e));
    let t = Transcript::heard(4.2, &words, vec![Silence { start: 1.9, end: 3.0 }, Silence { start: 3.9, end: 4.2 }]);
    let s = land(None, &t, 3.8, NOW).unwrap();
    assert!(!s.list.cuts.is_empty());
    assert!(s.list.cuts.iter().all(|c| c.end <= 3.8 && c.start < c.end), "{:?}", s.list.cuts);
    assert_eq!(s.list.source_duration, 3.8);
}

#[test]
fn the_words_as_the_space_keeps_them() {
    let words = [("world", 2.0, 2.4), ("Hello", 0.5, 0.9), ("", 1.0, 1.2), ("  ", 1.3, 1.4), ("lost", f64::NAN, 1.0), ("late", 3.0, 2.9)]
        .map(|(w, s, e)| (w.to_string(), s, e));
    let t = Transcript::heard(10.0, &words, vec![Silence { start: 0.9, end: 2.0 }]);
    let got: Vec<(&str, f64, f64)> = t.words.iter().map(|w| (w.text.as_str(), w.start, w.end)).collect();
    // Blank and timeless words go, the rest in time order, none ending before it starts.
    assert_eq!(got, [("Hello", 0.5, 0.9), ("world", 2.0, 2.4), ("late", 3.0, 3.0)]);
    assert_eq!((t.duration, t.silences.len()), (10.0, 1));
    // A length short of the last word is the last word's end.
    assert_eq!(Transcript::heard(f64::NAN, &words, vec![]).duration, 3.0);
}
