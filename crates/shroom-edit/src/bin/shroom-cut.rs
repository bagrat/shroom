//! The planner from the command line: a transcript (JSON, stdin) in; its plan (JSON, stdout)
//! out — the edit list, what plays, and the questions for a judge. With `answers` beside the
//! transcript, the plan takes the judge's word; with `cues` ("plain", "relaxed") it listens
//! for more than "shroom cut", asks first whether each was meant (`intents`), and hears those
//! `meant` lets in.

use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("read stdin");
    let v: serde_json::Value = serde_json::from_str(&input).expect("transcript JSON");
    let field = |k: &str| v.get(k).cloned();
    let answers: Vec<shroom_edit::Answer> = field("answers")
        .map(|a| serde_json::from_value(a).expect("answers"))
        .unwrap_or_default();
    let meant: Vec<shroom_edit::Meant> = field("meant")
        .map(|m| serde_json::from_value(m).expect("meant"))
        .unwrap_or_default();
    let cues: shroom_edit::CueWords = field("cues")
        .map(|c| serde_json::from_value(c).expect("cues"))
        .unwrap_or_default();
    let transcript: shroom_edit::Transcript = serde_json::from_value(v).expect("transcript");
    let o = shroom_edit::Options {
        cues,
        ..shroom_edit::Options::default()
    };
    let list = shroom_edit::plan_meant(&transcript, o, &meant, &answers);
    let heard: Vec<serde_json::Value> = shroom_edit::find_cues_in(&transcript.words, cues)
        .iter()
        .map(|c| serde_json::json!({"first": c.first, "last": c.last, "start": c.start, "end": c.end, "said": c.said}))
        .collect();
    let out = serde_json::json!({
        "edit_list": list,
        "keep": list.keep(),
        "edited_duration": list.edited_duration(),
        "questions": shroom_edit::questions_meant(&transcript, o, &meant),
        "intents": shroom_edit::intents(&transcript, o),
        "cues": heard,
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialise"));
}
