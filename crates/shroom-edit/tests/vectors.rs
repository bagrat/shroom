//! `keep-vectors.json`: edit lists and what they play, the one truth the Worker's page
//! (`worker/test/edits.test.js`) and the editor's webview (`app/src/editor/keep.test.ts`)
//! are held to as well. `SHROOM_WRITE_VECTORS=1 cargo test -p shroom-edit --test vectors`
//! rewrites the answers from the inputs.

use serde_json::{Value, json};
use shroom_edit::EditList;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/keep-vectors.json");

fn answer(list: &EditList, at_source: &[f64], at_edited: &[f64]) -> Value {
    let r = |x: f64| (x * 1e6).round() / 1e6;
    json!({
        "keep": list.keep().iter().map(|(s, e)| [r(*s), r(*e)]).collect::<Vec<_>>(),
        "edited_duration": r(list.edited_duration()),
        "identity": list.is_identity(),
        "to_edited": at_source.iter().map(|t| list.to_edited(*t).map(r)).collect::<Vec<_>>(),
        "to_source": at_edited.iter().map(|e| r(list.to_source(*e))).collect::<Vec<_>>(),
    })
}

#[test]
fn every_vector_holds() {
    let text = std::fs::read_to_string(PATH).unwrap();
    let mut cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    let write = std::env::var_os("SHROOM_WRITE_VECTORS").is_some();
    for case in &mut cases {
        let list: EditList = serde_json::from_value(case["list"].clone()).unwrap();
        let list = list.checked();
        let src: Vec<f64> = serde_json::from_value(case["at_source"].clone()).unwrap();
        let ed: Vec<f64> = serde_json::from_value(case["at_edited"].clone()).unwrap();
        let got = answer(&list, &src, &ed);
        if write {
            case["expect"] = got;
        } else {
            assert_eq!(case["expect"], got, "{}", case["name"]);
        }
    }
    if write {
        std::fs::write(PATH, serde_json::to_string_pretty(&cases).unwrap() + "\n").unwrap();
    }
}
