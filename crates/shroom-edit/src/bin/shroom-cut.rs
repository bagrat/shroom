//! The planner from the command line: a transcript (JSON, stdin) in; its plan (JSON, stdout)
//! out — the edit list, what plays, and the questions for a judge. With `answers` beside the
//! transcript, the plan takes the judge's word.

use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("read stdin");
    let v: serde_json::Value = serde_json::from_str(&input).expect("transcript JSON");
    let answers: Vec<shroom_edit::Answer> = match v.get("answers") {
        Some(a) => serde_json::from_value(a.clone()).expect("answers"),
        None => Vec::new(),
    };
    let transcript: shroom_edit::Transcript = serde_json::from_value(v).expect("transcript");
    let o = shroom_edit::Options::default();
    let list = shroom_edit::plan_with(&transcript, o, &answers);
    let out = serde_json::json!({
        "edit_list": list,
        "keep": list.keep(),
        "edited_duration": list.edited_duration(),
        "questions": shroom_edit::questions(&transcript, o),
    });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialise"));
}
