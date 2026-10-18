//! POC driver: a transcript (JSON, stdin) in, the planned edit list (JSON, stdout) out.

use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("read stdin");
    let transcript: shroom_edit::Transcript =
        serde_json::from_str(&input).expect("transcript JSON");
    let list = shroom_edit::plan(&transcript, shroom_edit::Options::default());
    let out = serde_json::json!({ "edit_list": list, "keep": list.keep(), "edited_duration": list.edited_duration() });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialise"));
}
