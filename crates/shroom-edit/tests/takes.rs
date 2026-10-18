//! The POC's takes as fixtures: real Whisper transcripts of invented scripts spoken by
//! stock voices (dev/proposals/voice-cut/poc), each beside its script with every line's
//! time, and, for the Workers AI ones, what the judgment above the planner answered
//! (`judged.json`, Clef). The plan with that judgment must cut every cue, never cut a kept or
//! merely mentioned line, and cut every slip (or, after a cue with no retake, offer it); the
//! code alone, as when no judgment comes, must never cut a kept or mentioned line either.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use shroom_edit::{Answer, Options, Transcript, plan, plan_with};

/// What the judgment answered for a take (Clef on Workers AI, recorded by
/// dev/proposals/voice-cut/poc/judge.py).
#[derive(Deserialize)]
struct Judged {
    answers: Vec<Answer>,
}

#[derive(Deserialize)]
struct Line {
    role: String,
    start: f64,
    end: f64,
}

fn overlap(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.1.min(b.1) - a.0.max(b.0)).max(0.0)
}

/// Takes the planner gets wrong today, and why. Each still must not cut a kept line.
const KNOWN: &[(&str, &str)] = &[
    (
        "small-en_US-amy-medium-4-no-retake",
        "Whisper wrote the hesitation \"hmm\" as \"him.\", with a real pause after it, so the \
     offered line before the cue starts after it and leaves \"The second one is\"",
    ),
    (
        "turbo-en_US-ryan-medium-7-cue-at-the-end",
        "Whisper left the first 8 s out of the transcript (the slip, the first cue and the \
         retake): nothing heard, nothing cut. Chunked transcription with VAD is the fix",
    ),
    (
        "wai-en_US-lessac-medium-2-mid-paragraph",
        "Workers AI's Whisper wrote the line as meant (\"Then you pick the window you want to \
         share.\"), leaving out the slip and the cue: nothing heard, nothing cut",
    ),
];

#[test]
fn every_take() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/takes");
    let mut checked = 0;
    let mut failed = Vec::new();
    for dir in fs::read_dir(&root).expect("tests/takes") {
        let dir = dir.unwrap().path();
        let t: Transcript =
            serde_json::from_str(&fs::read_to_string(dir.join("transcript.json")).unwrap())
                .unwrap();
        let lines: Vec<Line> =
            serde_json::from_str(&fs::read_to_string(dir.join("script.json")).unwrap()).unwrap();
        let answers = fs::read_to_string(dir.join("judged.json"))
            .map(|j| serde_json::from_str::<Judged>(&j).unwrap().answers)
            .unwrap_or_default();
        let list = plan_with(&t, Options::default(), &answers);
        let applied: Vec<(f64, f64)> = list
            .cuts
            .iter()
            .filter(|c| c.applied)
            .map(|c| (c.start, c.end))
            .collect();
        // The code alone, as when no judgment comes: it may leave a slip, never cut what stays.
        let alone: Vec<(f64, f64)> = plan(&t, Options::default())
            .cuts
            .iter()
            .filter(|c| c.applied)
            .map(|c| (c.start, c.end))
            .collect();
        let offered: Vec<(f64, f64)> = list
            .cuts
            .iter()
            .filter(|c| !c.applied)
            .map(|c| (c.start, c.end))
            .collect();
        let cut = |l: &Line, of: &[(f64, f64)]| {
            of.iter()
                .map(|&c| overlap((l.start, l.end), c))
                .sum::<f64>()
        };
        let mut why = Vec::new();
        for l in &lines {
            let len = l.end - l.start;
            if matches!(l.role.as_str(), "keep" | "mention") && cut(l, &alone) > 0.08 {
                why.push(format!("the code alone cut into a {} line", l.role));
            }
            match l.role.as_str() {
                "keep" | "mention" if cut(l, &applied) > 0.08 => {
                    why.push(format!("cut into a {} line", l.role))
                }
                "cue" if cut(l, &applied) < 0.9 * len => why.push("left a cue".into()),
                "flub" if cut(l, &applied) + cut(l, &offered) < 0.9 * len => {
                    why.push("left a slip".into())
                }
                _ => {}
            }
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let known = KNOWN.iter().any(|(k, _)| *k == name);
        if known {
            why.retain(|w| w.contains("cut into"));
        }
        if !why.is_empty() {
            failed.push(format!(
                "{}: {}",
                dir.file_name().unwrap().to_string_lossy(),
                why.join(", ")
            ));
        }
        checked += 1;
    }
    assert!(checked > 0, "no takes in {}", root.display());
    assert!(
        failed.is_empty(),
        "{} of {checked} takes failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// Ten minutes of the takes run together (52 cues), transcribed by Workers AI in one
/// request and in 60 s chunks. Nothing kept may be cut; most cues must go. Word times drift
/// on long audio and Whisper tidies some slips away, so the floor is below every cue.
#[test]
fn ten_minutes() {
    for (name, floor) in [("wai-one-request", 44), ("wai-60s-chunks", 48)] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/long")
            .join(name);
        let t: Transcript =
            serde_json::from_str(&fs::read_to_string(dir.join("transcript.json")).unwrap())
                .unwrap();
        let lines: Vec<Line> =
            serde_json::from_str(&fs::read_to_string(dir.join("script.json")).unwrap()).unwrap();
        let list = plan(&t, Options::default());
        let applied: Vec<(f64, f64)> = list
            .cuts
            .iter()
            .filter(|c| c.applied)
            .map(|c| (c.start, c.end))
            .collect();
        let cut = |l: &Line| {
            applied
                .iter()
                .map(|&c| overlap((l.start, l.end), c))
                .sum::<f64>()
        };
        let lines: Vec<&Line> = lines.iter().filter(|l| l.end < t.duration - 2.0).collect();
        let damaged = lines
            .iter()
            .filter(|l| matches!(l.role.as_str(), "keep" | "mention") && cut(l) > 0.08)
            .count();
        let cues = lines.iter().filter(|l| l.role == "cue").count();
        let gone = lines
            .iter()
            .filter(|l| l.role == "cue" && cut(l) >= 0.9 * (l.end - l.start))
            .count();
        assert_eq!(damaged, 0, "{name}: kept lines cut");
        assert!(
            gone >= floor,
            "{name}: {gone} of {cues} cues cut, floor {floor}"
        );
    }
}
