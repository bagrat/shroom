//! Recording goes through the app, never around it: the screen, camera and microphone
//! grants belong to shroom.app (macOS charges a capture to the process that asks — a
//! command recording by itself would ask as the terminal; history/POC-LEARNINGS.md), and
//! the countdown, the tray and the stop are the app's. The command asks the running app
//! over a Unix socket in shroom's folder (`control.sock`: the folder is 700, the socket
//! 600, the app checks the peer's uid; `app/src-tauri/src/runtime/control.rs`), one JSON
//! line each way:
//!
//!   → {"v":1,"op":"record.start"}            ← {"ok":true,"take":"countdown"}
//!   → {"v":1,"op":"record.stop"}             ← {"ok":false,"take":"none","error":"no_take"}
//!
//! The app answers with a code; the words are the command's (`say.rs`). Every op is refused
//! with `recording_off` until the person turns on Settings → General's “Let it start and
//! stop recordings” (off by default; Bagrat, 2026-10-18): an agent records only where the
//! person said it may (spec N62, DECISIONS B216).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::out::Fail;
use crate::say;

/// The ops the app takes. Each is what a click in the popover does, nothing more.
pub const OPS: [&str; 4] = ["record.start", "record.pause", "record.resume", "record.stop"];

/// Asks the app once, `how` beside the op (a start's `countdown` and `label`, N62). With
/// `launch`, opens the app this command ships in (`app_id`) when it isn't running, and waits
/// (≤ 15 s) for it to listen.
pub fn ask(socket: &Path, op: &str, how: &serde_json::Map<String, Value>, launch: Option<&str>) -> Result<Value, Fail> {
    let mut stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) => match launch {
            Some(app_id) => {
                let opening = Instant::now();
                open_app(app_id)?;
                let s = wait_for(socket, Duration::from_secs(15))?;
                crate::log::note(format!("app-opened {}ms", opening.elapsed().as_millis()));
                s
            }
            None => {
                crate::log::note("app-not-listening");
                return Err(Fail::AppUnreachable(say::app_not_running()));
            }
        },
    };
    stream.set_read_timeout(Some(Duration::from_secs(20))).ok();
    let mut ask = json!({"v": 1, "op": op});
    if let Some(o) = ask.as_object_mut() {
        o.extend(how.clone());
    }
    let line = format!("{ask}\n");
    let unreachable = |note: &str, words: String| {
        crate::log::note(note);
        Fail::AppUnreachable(words)
    };
    stream.write_all(line.as_bytes()).map_err(|_| unreachable("app-didnt-take-the-ask", say::app_didnt_take_it()))?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).map_err(|_| unreachable("app-no-answer", say::app_didnt_answer()))?;
    let v: Value = serde_json::from_str(answer.trim()).map_err(|_| unreachable("app-answer-unreadable", say::app_answer_unreadable()))?;
    if v["ok"] == true {
        return Ok(v);
    }
    let code = v["error"].as_str().unwrap_or("");
    crate::log::note(format!("app {}", if code.chars().all(|c| c.is_ascii_lowercase() || c == '_') { code } else { "?" }));
    Err(refusal(code, op))
}

/// The app's code, in the command's words.
fn refusal(code: &str, op: &str) -> Fail {
    match code {
        "take_open" => Fail::TakeOpen(say::already_recording()),
        "not_set_up" => Fail::NotSetUp(say::app_not_set_up()),
        "recording_off" => Fail::RecordingOff(say::recording_off()),
        "busy" => Fail::Refused(say::between_two_steps()),
        "no_take" if op == "record.resume" => Fail::Refused(say::nothing_paused()),
        "no_take" => Fail::Refused(say::nothing_recording()),
        "not_ready" => Fail::Refused(say::not_now(op)),
        // `shroom edit` (N70): this Mac's editor holds the video.
        "editor_open" => Fail::CantEdit { reason: "editor_open", message: say::cant_edit("editor_open") },
        _ => Fail::Refused(say::app_said_no()),
    }
}

fn open_app(app_id: &str) -> Result<(), Fail> {
    if !cfg!(target_os = "macos") {
        return Err(Fail::AppUnreachable(say::app_only_on_a_mac()));
    }
    let ok = std::process::Command::new("open").args(["-g", "-b", app_id]).status().is_ok_and(|s| s.success());
    if ok {
        return Ok(());
    }
    crate::log::note("app-wont-open");
    Err(Fail::AppUnreachable(say::app_wont_open()))
}

fn wait_for(socket: &Path, budget: Duration) -> Result<UnixStream, Fail> {
    let start = Instant::now();
    while start.elapsed() < budget {
        if let Ok(s) = UnixStream::connect(socket) {
            return Ok(s);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    crate::log::note(format!("app-opened-not-listening {}s", budget.as_secs()));
    Err(Fail::AppUnreachable(say::app_slow_to_answer()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_code_is_its_own_failure_and_a_resume_says_what_it_missed() {
        assert_eq!(refusal("recording_off", "record.start").code(), "recording_off");
        assert_eq!(refusal("take_open", "record.start").code(), "take_open");
        assert_eq!(refusal("not_set_up", "record.stop").code(), "not_set_up");
        assert_eq!(refusal("busy", "record.stop").code(), "refused");
        assert_eq!(refusal("no_take", "record.resume").message(), say::nothing_paused());
        assert_eq!(refusal("no_take", "record.stop").message(), say::nothing_recording());
        assert_ne!(refusal("not_ready", "record.resume").message(), refusal("not_ready", "record.start").message());
    }

    #[test]
    fn an_app_that_isnt_listening_is_unreachable_when_it_isnt_to_be_opened() {
        let dir = tempfile::tempdir().unwrap();
        let f = ask(&dir.path().join("control.sock"), "record.stop", &serde_json::Map::new(), None).unwrap_err();
        assert_eq!((f.code(), f.exit()), ("app_unreachable", 8));
    }
}
