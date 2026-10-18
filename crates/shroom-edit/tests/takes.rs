//! The POC's takes as fixtures: real Whisper transcripts of invented scripts spoken by
//! stock voices (dev/proposals/voice-cut/poc), each beside its script with every line's
//! time. The planner must cut every cue, never cut a kept or merely mentioned line, and cut
//! every slip (or, after a cue with no retake, offer it).

use std::fs;
use std::path::Path;

use serde::Deserialize;
use shroom_edit::{Options, Transcript, plan};

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
        let list = plan(&t, Options::default());
        let applied: Vec<(f64, f64)> = list
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
            why.retain(|w| w.starts_with("cut into"));
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
