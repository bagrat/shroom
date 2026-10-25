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
    /// `shroom edit` can't make that change (exit 12, spec N72): `reason` is the editor's own
    /// (`out_of_range`, `leaves_too_little`, `too_short`, `nothing_changes`, `no_such_cut`), or
    /// `editor_open` while the video is open in an editor window on this Mac. Nothing was saved.
    CantEdit { reason: &'static str, message: String },
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
            Fail::CantEdit { .. } => "cant_edit",
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
            Fail::CantEdit { .. } => 12,
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
            Fail::Ambiguous { message, .. } | Fail::NeedsConfirmation { message, .. } | Fail::Partial { message, .. } | Fail::CantEdit { message, .. } => message,
        }
    }

    pub fn json(&self) -> Value {
        let mut v = json!({"ok": false, "error": self.code(), "message": self.message()});
        match self {
            Fail::Ambiguous { matches, .. } => v["matches"] = Value::Array(matches.clone()),
            Fail::NeedsConfirmation { what, would, .. } => v[*what] = Value::Array(would.clone()),
            Fail::RecordingOff(_) => v["settings_link"] = json!(shroom_core::event::SettingsSpot::AgentRecording.link()),
            Fail::CantEdit { reason, .. } => v["reason"] = json!(reason),
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

/// A character a terminal acts on rather than shows: C0 controls but tab and line feed (the
/// escape that starts a clipboard write or a cursor move, the carriage return that rewrites a
/// line), DEL, C1 controls, and the bidi marks and overrides that reorder what's shown.
fn acts_on_terminal(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// What a person's terminal is given: titles and names are written by the people in a space,
/// so each character a terminal would act on shows as � (the security review, 2026-10-18).
pub fn clean(text: &str) -> String {
    text.chars().map(|c| if acts_on_terminal(c) { '\u{fffd}' } else { c }).collect()
}

/// An answer's JSON with every character a terminal would act on escaped (`\u001b`, `\u202e`):
/// still exactly what the space holds once parsed, and nothing raw on the terminal.
/// serde_json escapes the C0 controls itself; this adds DEL, C1 and the bidi marks.
pub fn json_safe(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        if acts_on_terminal(c) {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// `93` → `1:33`; `3725` → `1:02:05`.
pub fn length(sec: Option<f64>) -> String {
    let Some(s) = sec else { return "–".into() };
    let s = s.round() as u64;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// A time on a recording, for a person, to the tenth: `62.5` → `1:02.5`; `62` → `1:02`.
pub fn at(sec: f64) -> String {
    let tenths = (sec.max(0.0) * 10.0).round() as u64;
    let (s, t) = (tenths / 10, tenths % 10);
    let whole = if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) };
    if t == 0 { whole } else { format!("{whole}.{t}") }
}

/// The date part of an ISO time, for a person (`2026-10-02`).
pub fn day(iso: &str) -> &str {
    iso.get(..10).unwrap_or(iso)
}

/// `2026-10-27T03:00:00Z` → `Oct 27`, as the Library says a day (N75).
pub fn short_day(iso: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month = iso.get(5..7).and_then(|m| m.parse::<usize>().ok()).filter(|m| (1..=12).contains(m));
    let d = iso.get(8..10).and_then(|d| d.parse::<u32>().ok());
    match (month, d) {
        (Some(m), Some(d)) => format!("{} {d}", MONTHS[m - 1]),
        _ => day(iso).to_string(),
    }
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

    /// Titles and names are written by the people in a space: what the command prints can't
    /// drive the terminal — no escape sequence (a clipboard write, a line rewritten), no
    /// carriage return, no C1 control, no bidi override — and its JSON still says exactly
    /// what the space holds, those characters escaped (the security review, 2026-10-18).
    #[test]
    fn text_from_the_space_cant_drive_the_terminal() {
        let title = "Q3\u{1b}]52;c;cm0gLXJmIH4=\u{7}\u{1b}[2K\r\u{9b}31m\u{7f}\u{202e}fdp.exe\u{2066}x\u{2069}\tok\nnext";
        let bad = |c: char| matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
        let shown = clean(title);
        assert!(!shown.chars().any(bad), "{shown:?}");
        assert!(shown.starts_with("Q3") && shown.ends_with("\tok\nnext"), "tabs and line breaks stay: {shown:?}");
        let j = json_safe(&json!({"title": title}).to_string());
        assert!(!j.chars().any(bad), "{j}");
        assert_eq!(serde_json::from_str::<Value>(&j).unwrap()["title"], title, "the JSON still says exactly what the space holds");
        assert_eq!(clean("Weekly sync — Q3 ✓"), "Weekly sync — Q3 ✓", "ordinary words are left alone");
    }

    #[test]
    fn lengths_and_columns_read_for_a_person() {
        assert_eq!(length(Some(93.4)), "1:33");
        assert_eq!(length(Some(3725.0)), "1:02:05");
        assert_eq!(length(None), "–");
        assert_eq!(col("abc", 5), "abc  ");
        assert_eq!(col("abcdef", 4), "abc…");
        assert_eq!(short_day("2026-10-07T03:00:00.000Z"), "Oct 7");
    }
}
