//! The command's own log (Bagrat, 2026-10-18: enough on this Mac to report a problem): one
//! line a run in `logs/cli.log` in its app's folder — when, the command and the options it
//! was given (never their words), how it ended (`ok` or the error code, and the exit), what
//! it counted, what the space or the app said where it said no, and how long it took.
//! Never a title, a name, a link or a key: an id as the run log names it (`AbCd…`).
//! Report a problem and Copy diagnostics carry its last lines among the app's, by their time
//! (`shroom_core::report::clean_logs`).
//!
//! The one file the command writes in shroom's folder, and only while that folder exists
//! (shroom has run here): the folder's other files stay the app's alone. Rolled to
//! `cli.log.1` past `ROLL_BYTES`, so it never grows without end.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// The log rolls once it reaches this: some 500 runs.
pub const ROLL_BYTES: u64 = 64 * 1024;

/// What the run met on the way, in a few words each: the space's or the app's refusals.
static NOTES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Notes something the run met, for its line.
pub fn note(what: impl Into<String>) {
    if let Ok(mut n) = NOTES.lock() {
        n.push(what.into());
    }
}

fn notes() -> Vec<String> {
    NOTES.lock().map(|mut n| std::mem::take(&mut *n)).unwrap_or_default()
}

/// An id as the log names it: enough to tell videos apart, never the whole link.
pub fn short(id: &str) -> String {
    if id.chars().count() <= 4 { id.to_string() } else { format!("{}…", id.chars().take(4).collect::<String>()) }
}

/// The command as the log names it: its verb (and the verb's own word: `members remove`,
/// `invite cancel`, `record start`), never another of its words.
pub fn command(words: &[String]) -> String {
    let w = |i: usize| words.get(i).map(String::as_str).unwrap_or("");
    let verb = match w(0) {
        "" | "help" => "help",
        "ls" | "list" => "list",
        "show" | "info" => "info",
        "skill" | "guide" => "guide",
        "rm" | "delete" => "delete",
        "cleanup" | "cleanup-candidates" => "cleanup",
        v @ ("version" | "status" | "link" | "links" | "open" | "rename") => v,
        "members" if w(1) == "remove" => "members remove",
        "members" => "members",
        "invite" if w(1) == "cancel" => "invite cancel",
        "invite" => "invite",
        // `edit <video> <change>`: the change by its name, never the video or the times.
        "edit" => match w(2) {
            "" => "edit",
            "trim" => "edit trim",
            "cut" => "edit cut",
            "put-back" => "edit put-back",
            "apply" => "edit apply",
            "resize" => "edit resize",
            "revert" => "edit revert",
            "finish" => "edit finish",
            _ => "edit ?",
        },
        "record" => match w(1) {
            "start" => "record start",
            "pause" => "record pause",
            "resume" => "record resume",
            "stop" => "record stop",
            _ => "record ?",
        },
        _ => "?",
    };
    verb.to_string()
}

/// What a run's answer counted: its lists' lengths, its `counts`, the take's state, and an
/// id — never a string an answer carries otherwise (a title, a name, a link).
pub fn counted(answer: &Value) -> Vec<String> {
    let Some(o) = answer.as_object() else { return vec![] };
    let mut out = Vec::new();
    for (k, v) in o {
        match (k.as_str(), v) {
            (_, Value::Array(a)) => out.push(format!("{k} {}", a.len())),
            ("counts", Value::Object(c)) => out.extend(c.iter().filter(|(_, n)| n.is_number()).map(|(name, n)| format!("{name} {n}"))),
            ("take" | "final", Value::String(t)) if t.chars().all(|c| c.is_ascii_lowercase() || c == '_') => out.push(format!("{k} {t}")),
            ("id", Value::String(id)) => out.push(format!("id {}", short(id))),
            ("counting" | "dry_run" | "running" | "saved", Value::Bool(b)) => out.push(format!("{k} {b}")),
            ("rev", Value::Number(n)) => out.push(format!("rev {n}")),
            ("reason", Value::String(r)) if r.chars().all(|c| c.is_ascii_lowercase() || c == '_') => out.push(format!("reason {r}")),
            _ => {}
        }
    }
    out
}

/// One run's line: when (UTC), what was asked, how it ended, what it counted and met, and
/// how long it took.
pub fn line(at: SystemTime, command: &str, options: &[&str], ended: &str, counted: &[String], notes: &[String], took: Duration) -> String {
    let mut asked = command.to_string();
    for o in options {
        asked.push_str(" --");
        asked.push_str(o);
    }
    let mut said = vec![ended.to_string()];
    said.extend(counted.iter().cloned());
    said.extend(notes.iter().cloned());
    format!("{} cli: {}: {} ({}ms)\n", utc(at), asked, said.join(", "), took.as_millis())
}

/// Writes the run's line, with whatever it noted on the way. A folder that isn't there (shroom
/// never ran here), or a log that's a link, takes nothing.
pub fn write(home: &Path, command: &str, options: &[&str], ended: &str, counted: &[String], took: Duration) {
    let line = line(SystemTime::now(), command, options, ended, counted, &notes(), took);
    append(home, &line);
}

fn append(home: &Path, line: &str) {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    if !std::fs::symlink_metadata(home).is_ok_and(|m| m.is_dir()) {
        return;
    }
    let dir = home.join("logs");
    match std::fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() => {}
        Ok(_) => return,
        Err(_) => {
            if std::fs::DirBuilder::new().mode(0o700).create(&dir).is_err() {
                return;
            }
        }
    }
    let log = dir.join("cli.log");
    match std::fs::symlink_metadata(&log) {
        Ok(m) if m.file_type().is_symlink() => return,
        Ok(m) if m.len() >= ROLL_BYTES => {
            let _ = std::fs::rename(&log, dir.join("cli.log.1"));
        }
        _ => {}
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).mode(0o600).open(&log) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// A time as UTC to the second, as the app's run log stamps its lines: “2026-10-18T15:31:05Z”.
fn utc(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days since 1970-01-01 to the civil date (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_reads_the_calendar() {
        assert_eq!(utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(1_792_337_465)), "2026-10-18T15:31:05Z");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(951_782_400)), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn the_command_is_named_by_its_verb_never_its_words() {
        let w = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(command(&w("rename AbCdEfGhIjKlMnOp Secret plans")), "rename");
        assert_eq!(command(&w("members remove m_AbCdEfGhIjKlMnOp")), "members remove");
        assert_eq!(command(&w("invite Ann Lee")), "invite");
        assert_eq!(command(&w("record start")), "record start");
        assert_eq!(command(&w("edit AbCdEfGhIjKlMnOp cut 1:02 1:10")), "edit cut");
        assert_eq!(command(&w("edit AbCdEfGhIjKlMnOp")), "edit");
        assert_eq!(command(&w("Quarterly numbers")), "?");
        assert_eq!(command(&[]), "help");
    }

    #[test]
    fn what_it_counts_carries_no_words() {
        let answer = serde_json::json!({
            "ok": true, "videos": [{"title": "Secret plans"}], "counts": {"done": 2, "failed": 1},
            "take": "recording", "id": "AbCdEfGhIjKlMnOp", "link": "https://v.x.workers.dev/AbCdEfGhIjKlMnOp",
            "title": "Secret plans", "counting": false,
        });
        let c = counted(&answer).join(", ");
        assert_eq!(c, "counting false, done 2, failed 1, id AbCd…, take recording, videos 1");
        assert!(!c.contains("Secret") && !c.contains("https"));
    }

    #[test]
    fn a_run_is_one_line_the_report_keeps_as_it_is() {
        let at = UNIX_EPOCH + Duration::from_secs(1_792_337_465);
        let l = line(at, "record start", &["no-countdown", "json"], "recording_off, exit 10", &[], &["app recording_off".into()], Duration::from_millis(12));
        assert_eq!(l, "2026-10-18T15:31:05Z cli: record start --no-countdown --json: recording_off, exit 10, app recording_off (12ms)\n");
        assert_eq!(shroom_core::report::clean_line(l.trim_end()), l.trim_end(), "nothing in it for the report to take out");
    }

    #[test]
    fn it_writes_only_where_shroom_has_run_and_rolls() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("never-ran");
        append(&gone, "x\n");
        assert!(!gone.exists(), "no folder made for a shroom that never ran here");
        append(tmp.path(), "first\n");
        let log = tmp.path().join("logs/cli.log");
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "first\n");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&log).unwrap().permissions().mode() & 0o777, 0o600);
        std::fs::write(&log, vec![b'x'; ROLL_BYTES as usize]).unwrap();
        append(tmp.path(), "after the roll\n");
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "after the roll\n");
        assert_eq!(std::fs::metadata(tmp.path().join("logs/cli.log.1")).unwrap().len(), ROLL_BYTES);
    }
}
