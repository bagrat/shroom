//! Many at once (dev/proposals/cli-agent-skill.md §3b; Bagrat, 2026-10-18: batch commands,
//! so an agent can invite a team, or tidy many videos, in one step). Every batch command
//! takes its items the same two ways — each argument one item (`--batch`), or `--from
//! <file>`, a JSON array or one item per line — answers item by item in the order given,
//! and is safe to run again: an item already done is reported as such, never done twice.
//! When some items failed and others didn't, the answer is `partial` (exit 11) with every
//! item's result, so the agent can say what happened and re-run for the rest.

use serde_json::{Value, json};

use crate::out::{Done, Fail};
use crate::say;

/// The most items one call takes: a space holds at most 20 pending invites (WORKER-INTERFACE
/// §4.12), and a hundred of anything is more than a person checks before saying yes.
pub const MAX: usize = 100;
pub const MAX_INVITES: usize = 20;

/// One item as given: a line or a JSON string (`text`), or a JSON object (`obj`).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Text(String),
    Obj(serde_json::Map<String, Value>),
}

impl Item {
    /// The item's text, or `key` of its object.
    pub fn field(&self, key: &str) -> Option<String> {
        match self {
            Item::Text(t) => Some(t.clone()),
            Item::Obj(o) => o.get(key).and_then(Value::as_str).map(str::to_string),
        }
    }
}

/// The items: `--from` (a file, or `-` for stdin) when given, else the arguments.
pub fn items(args: &[String], from: Option<&str>) -> Result<Vec<Item>, Fail> {
    let items = match from {
        Some(path) => {
            let text = if path == "-" {
                let mut s = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut s).map_err(|_| Fail::Usage(say::couldnt_read_stdin()))?;
                s
            } else {
                std::fs::read_to_string(path).map_err(|_| Fail::Usage(say::couldnt_read(path)))?
            };
            parse(&text)?
        }
        None => args.iter().map(|a| Item::Text(a.trim().to_string())).filter(|i| *i != Item::Text(String::new())).collect(),
    };
    if items.is_empty() {
        return Err(Fail::Usage(say::nothing_to_do()));
    }
    if items.len() > MAX {
        return Err(Fail::Usage(say::too_many_items(MAX, items.len())));
    }
    Ok(items)
}

/// A JSON array (of strings or objects), or one item per line (blank lines and `#` lines
/// skipped).
pub fn parse(text: &str) -> Result<Vec<Item>, Fail> {
    let t = text.trim();
    if t.starts_with('[') {
        let v: Vec<Value> = serde_json::from_str(t).map_err(|_| Fail::Usage(say::not_a_json_array()))?;
        return v
            .into_iter()
            .map(|x| match x {
                Value::String(s) => Ok(Item::Text(s.trim().to_string())),
                Value::Object(o) => Ok(Item::Obj(o)),
                _ => Err(Fail::Usage(say::item_shape())),
            })
            .collect();
    }
    Ok(t.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| Item::Text(l.to_string())).collect())
}

/// One item's outcome. `status` is a stable word an agent branches on.
pub struct Outcome {
    pub json: Value,
    pub failed: bool,
    pub line: String,
}

impl Outcome {
    pub fn ok(json: Value, line: impl Into<String>) -> Outcome {
        Outcome { json, failed: false, line: line.into() }
    }
    /// A failed item: its own code and words beside what it was.
    pub fn failed(mut json: Value, f: &Fail, line: impl Into<String>) -> Outcome {
        json["status"] = "failed".into();
        json["error"] = f.code().into();
        json["message"] = f.message().into();
        Outcome { json, failed: true, line: line.into() }
    }
}

/// The batch's answer: `{<key>: [...], "counts": {status: n}}` — `partial` (exit 11) when
/// any item failed, with every item's result beside it.
pub fn finish(key: &str, outcomes: Vec<Outcome>, head: String, tail: &str) -> Result<Done, Fail> {
    let mut counts = serde_json::Map::new();
    for o in &outcomes {
        let s = o.json["status"].as_str().unwrap_or("?").to_string();
        let n = counts.get(&s).and_then(Value::as_u64).unwrap_or(0) + 1;
        counts.insert(s, n.into());
    }
    let failed = outcomes.iter().filter(|o| o.failed).count();
    let lines = outcomes.iter().map(|o| o.line.clone()).collect::<Vec<_>>().join("\n");
    let text = if tail.is_empty() { format!("{head}\n{lines}") } else { format!("{head}\n{lines}\n{tail}") };
    let results = json!({key: outcomes.iter().map(|o| o.json.clone()).collect::<Vec<_>>(), "counts": counts});
    if failed == 0 {
        return Ok(Done::new(results, text));
    }
    Err(Fail::Partial { message: say::partly(failed, outcomes.len(), &lines), results })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_a_json_array_or_one_item_a_line() {
        assert_eq!(parse("Ann Lee\n\n# the designers\nSam Ng\n").unwrap(), vec![Item::Text("Ann Lee".into()), Item::Text("Sam Ng".into())]);
        let j = parse(r#"["Ann Lee", {"id": "AbCdEfGhIjKlMnOp", "title": "Demo"}]"#).unwrap();
        assert_eq!(j[0], Item::Text("Ann Lee".into()));
        assert_eq!(j[1].field("title").as_deref(), Some("Demo"));
        assert_eq!(parse("[1, 2]").unwrap_err().code(), "usage");
    }

    #[test]
    fn too_many_at_once_is_refused_before_anything_happens() {
        let many: Vec<String> = (0..=MAX).map(|i| format!("p{i}")).collect();
        assert_eq!(items(&many, None).unwrap_err().code(), "usage");
        assert_eq!(items(&[], None).unwrap_err().code(), "usage");
    }

    #[test]
    fn some_failed_is_partial_with_every_result() {
        let f = Fail::Offline("x".into());
        let out = finish("invites", vec![Outcome::ok(json!({"status": "invited"}), "a"), Outcome::failed(json!({"name": "b"}), &f, "b")], "head".into(), "");
        let e = out.unwrap_err();
        assert_eq!((e.code(), e.exit()), ("partial", 11));
        let j = e.json();
        assert_eq!(j["invites"][1]["error"], "offline");
        assert_eq!(j["counts"]["failed"], 1);
    }
}
