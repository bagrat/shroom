//! What the CLI prints and how it exits. One shape for an agent (`--json`: one object on
//! stdout, `{"ok": true, …}` or `{"ok": false, "error": "<code>", "message": "…"}`) and
//! plain lines for a person. The error codes and exit codes are a contract: an agent
//! branches on them, so a code is added, never renamed (DECISIONS B215).

use serde_json::{Value, json};

/// Why a command didn't do what was asked. Each has a stable code and exit status.
#[derive(Debug, Clone, PartialEq)]
pub enum Fail {
    /// The command line itself was wrong (exit 2).
    Usage(String),
    /// This Mac isn't in a space yet, or hasn't finished joining (exit 3).
    NotSetUp(String),
    /// No video matches, or more than one does (exit 4).
    NotFound(String),
    Ambiguous { message: String, matches: Vec<Value> },
    /// The space couldn't be reached (exit 5): offline, or busy for now.
    Offline(String),
    /// The space said no (exit 6): not this person's video, not owner-class, removed.
    Refused(String),
    /// A delete without `--yes` (exit 7): nothing was changed.
    /// `what` names the list in the JSON: `would_delete` (videos), `would_remove` (members).
    NeedsConfirmation { message: String, what: &'static str, would: Vec<Value> },
    /// The app is needed and isn't there, or didn't answer (exit 8).
    AppUnreachable(String),
    /// A recording is open: shroom stays quiet until it ends (law 4) (exit 9).
    TakeOpen(String),
    /// A batch did some of its items and not others (exit 11): `results` says which, item
    /// by item, so a re-run can finish the rest.
    Partial { message: String, results: Value },
    /// The person hasn't let a coding agent record: Settings → General's switch is off,
    /// as it is by default (exit 10). Its JSON carries `settings_link`, the link that opens
    /// Settings at the switch, for the agent to hand over (DECISIONS B216).
    RecordingOff(String),
    /// Anything else (exit 1).
    Other(String),
}

impl Fail {
    pub fn code(&self) -> &'static str {
        match self {
            Fail::Usage(_) => "usage",
            Fail::NotSetUp(_) => "not_set_up",
            Fail::NotFound(_) => "not_found",
            Fail::Ambiguous { .. } => "ambiguous",
            Fail::Offline(_) => "offline",
            Fail::Refused(_) => "refused",
            Fail::NeedsConfirmation { .. } => "needs_confirmation",
            Fail::AppUnreachable(_) => "app_unreachable",
            Fail::TakeOpen(_) => "take_open",
            Fail::RecordingOff(_) => "recording_off",
            Fail::Partial { .. } => "partial",
            Fail::Other(_) => "failed",
        }
    }

    pub fn exit(&self) -> i32 {
        match self {
            Fail::Other(_) => 1,
            Fail::Usage(_) => 2,
            Fail::NotSetUp(_) => 3,
            Fail::NotFound(_) | Fail::Ambiguous { .. } => 4,
            Fail::Offline(_) => 5,
            Fail::Refused(_) => 6,
            Fail::NeedsConfirmation { .. } => 7,
            Fail::AppUnreachable(_) => 8,
            Fail::TakeOpen(_) => 9,
            Fail::RecordingOff(_) => 10,
            Fail::Partial { .. } => 11,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Fail::Usage(m)
            | Fail::NotSetUp(m)
            | Fail::NotFound(m)
            | Fail::Offline(m)
            | Fail::Refused(m)
            | Fail::AppUnreachable(m)
            | Fail::TakeOpen(m)
            | Fail::RecordingOff(m)
            | Fail::Other(m) => m,
            Fail::Ambiguous { message, .. } | Fail::NeedsConfirmation { message, .. } | Fail::Partial { message, .. } => message,
        }
    }

    pub fn json(&self) -> Value {
        let mut v = json!({"ok": false, "error": self.code(), "message": self.message()});
        match self {
            Fail::Ambiguous { matches, .. } => v["matches"] = Value::Array(matches.clone()),
            Fail::NeedsConfirmation { what, would, .. } => v[*what] = Value::Array(would.clone()),
            Fail::RecordingOff(_) => v["settings_link"] = json!(shroom_core::event::SettingsSpot::AgentRecording.link()),
            Fail::Partial { results, .. } => {
                if let (Some(o), Some(r)) = (v.as_object_mut(), results.as_object()) {
                    for (k, x) in r {
                        o.insert(k.clone(), x.clone());
                    }
                }
            }
            _ => {}
        }
        v
    }
}

/// A command's success: the object an agent reads, and the lines a person reads.
#[derive(Debug)]
pub struct Done {
    pub json: Value,
    pub text: String,
}

impl Done {
    pub fn new(mut json: Value, text: impl Into<String>) -> Done {
        if let Some(o) = json.as_object_mut() {
            o.insert("ok".into(), Value::Bool(true));
        }
        Done { json, text: text.into() }
    }
}

/// `93` → `1:33`; `3725` → `1:02:05`.
pub fn length(sec: Option<f64>) -> String {
    let Some(s) = sec else { return "–".into() };
    let s = s.round() as u64;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// The date part of an ISO time, for a person (`2026-10-02`).
pub fn day(iso: &str) -> &str {
    iso.get(..10).unwrap_or(iso)
}

/// Pads or cuts `s` to `n` characters, for one table column.
pub fn col(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count > n { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) } else { format!("{s}{}", " ".repeat(n - count)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_failure_has_its_own_exit_and_a_stable_code() {
        let all = [
            Fail::Usage(String::new()),
            Fail::NotSetUp(String::new()),
            Fail::NotFound(String::new()),
            Fail::Offline(String::new()),
            Fail::Refused(String::new()),
            Fail::NeedsConfirmation { message: String::new(), what: "would_delete", would: vec![] },
            Fail::AppUnreachable(String::new()),
            Fail::TakeOpen(String::new()),
            Fail::RecordingOff(String::new()),
            Fail::Partial { message: String::new(), results: Value::Null },
            Fail::Other(String::new()),
        ];
        let exits: std::collections::BTreeSet<i32> = all.iter().map(Fail::exit).collect();
        assert_eq!(exits.len(), all.len());
        assert!(all.iter().all(|f| f.json()["ok"] == false && f.json()["error"] == f.code()));
    }

    #[test]
    fn lengths_and_columns_read_for_a_person() {
        assert_eq!(length(Some(93.4)), "1:33");
        assert_eq!(length(Some(3725.0)), "1:02:05");
        assert_eq!(length(None), "–");
        assert_eq!(col("abc", 5), "abc  ");
        assert_eq!(col("abcdef", 4), "abc…");
    }
}
