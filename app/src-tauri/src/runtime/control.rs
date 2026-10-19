//! The `shroom` command's door into the running app (DECISIONS B216,
//! spec N62): `control.sock` in this flavor's home, one JSON line each way.
//!
//!   → {"v":1,"op":"record.start"}       ← {"ok":true,"take":"countdown"}
//!   → {"v":1,"op":"record.stop"}        ← {"ok":true,"take":"stopping","link":"https://…"}
//!                                       ← {"ok":false,"take":"none","error":"recording_off"}
//!
//! Who may ask: this user alone — the socket lives in the home folder (700) and is 600
//! itself, and every connection's peer uid is checked against ours. What it may ask: only
//! start, pause, resume and stop, and the core answers each exactly as the click it stands
//! for, refusing all of them while General's “Let it start and stop recordings” is off.
//! The answer is a code, never words: the command says them. Nothing secret crosses here.
//!
//! The run log has every ask and its answer, by the ask's number: the op and how a start
//! goes, then the answer's code and how long the core took (`asked`, `answered`) — and any
//! connection dropped or ask not read, with why. Never the link a stop answers with.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_core::effect::CliAnswer;
use shroom_core::event::{CliOp, Event, StartAsk};

use super::Runtime;

/// The connections waiting on the core's answer, by the id their `Event::Cli` carries.
#[derive(Clone, Default)]
pub struct Pending {
    next: Arc<AtomicU64>,
    open: Arc<Mutex<HashMap<u64, (UnixStream, Instant)>>>,
}

/// The longest ask read: the op and its version, nothing more.
const MAX_ASK: u64 = 1024;

pub fn op_of(line: &str) -> Option<CliOp> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if v["v"] != 1 {
        return None;
    }
    match v["op"].as_str()? {
        // How a start goes: the count and the label unless the command turned one off (N62).
        "record.start" => Some(CliOp::Start(StartAsk { countdown: v["countdown"] != false, label: v["label"] != false })),
        "record.pause" => Some(CliOp::Pause),
        "record.resume" => Some(CliOp::Resume),
        "record.stop" => Some(CliOp::Stop),
        _ => None,
    }
}

/// An ask, as the run log has it: its number, the op, and how a start goes when it isn't
/// the app's default.
pub fn asked(id: u64, op: &CliOp) -> String {
    let (name, how) = match op {
        CliOp::Start(a) => ("record.start", [(!a.countdown).then_some(" no-count"), (!a.label).then_some(" no-label")].into_iter().flatten().collect::<String>()),
        CliOp::Pause => ("record.pause", String::new()),
        CliOp::Resume => ("record.resume", String::new()),
        CliOp::Stop => ("record.stop", String::new()),
    };
    format!("control: ask {id} {name}{how}")
}

/// An answer, as the run log has it: its ask's number, ok or no, the take or the refusal's
/// code, and how long it took — never the link.
pub fn answered(a: &CliAnswer, took: Duration) -> String {
    format!("control: answer {} {} {} ({} ms)", a.id, if a.ok { "ok" } else { "no" }, a.error.unwrap_or(a.take), took.as_millis())
}

pub fn line_of(a: &CliAnswer) -> String {
    let mut v = serde_json::json!({"ok": a.ok, "take": a.take});
    if let Some(e) = a.error {
        v["error"] = e.into();
    }
    if let Some(l) = &a.link {
        v["link"] = l.clone().into();
    }
    format!("{v}\n")
}

/// Whether the other end of `s` runs as this user.
fn same_user(s: &UnixStream) -> bool {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: a connected socket's descriptor and two out-parameters the call fills.
    let ok = unsafe { libc::getpeereid(s.as_raw_fd(), &mut uid, &mut gid) } == 0;
    // SAFETY: getuid has no failure mode.
    ok && uid == unsafe { libc::getuid() }
}

/// Listens on `control.sock` for as long as the app runs. A socket another run left
/// behind is replaced; one that can't be made is logged and the app runs without it.
pub fn spawn(rt: Runtime, pending: Pending) {
    let path = rt.home.path("control.sock");
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            rt.log(&format!("control: no socket ({e}) — the shroom command can't reach this app"));
            return;
        }
    };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    rt.log("control: listening for the shroom command");
    let _ = std::thread::Builder::new().name("shroom-control".into()).spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            if !same_user(&conn) {
                rt.log("control: a connection from another user, dropped");
                continue;
            }
            let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = conn.set_write_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let read = conn.try_clone().map(|c| BufReader::new(c.take(MAX_ASK)).read_line(&mut line));
            let Some(op) = read.ok().and_then(|r| r.ok()).and_then(|_| op_of(&line)) else {
                rt.log(&format!("control: an ask it couldn't read ({} bytes), answered usage", line.len()));
                let _ = conn.write_all(b"{\"ok\":false,\"take\":\"none\",\"error\":\"usage\"}\n");
                continue;
            };
            // Start is the popover's Start: the grants as macOS has them now go first, so
            // the core arms on the truth (commands::arms).
            if matches!(op, CliOp::Start(_)) {
                rt.post(Event::Os(super::os::grants()));
            }
            let id = pending.next.fetch_add(1, Ordering::SeqCst) + 1;
            pending.open.lock().unwrap().insert(id, (conn, Instant::now()));
            rt.log(&asked(id, &op));
            rt.post(Event::Cli { id, op });
        }
    });
}

/// The core's answer, onto the connection that asked; then it closes.
pub fn answer(rt: &Runtime, pending: &Pending, a: CliAnswer) {
    let Some((mut conn, at)) = pending.open.lock().unwrap().remove(&a.id) else { return };
    rt.log(&answered(&a, at.elapsed()));
    if conn.write_all(line_of(&a).as_bytes()).is_err() {
        rt.log(&format!("control: answer {} not delivered: the command had gone", a.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_four_ops_of_this_version_are_read() {
        assert_eq!(op_of(r#"{"v":1,"op":"record.start"}"#), Some(CliOp::START));
        // The command's flags (N62, B216): each only when the person or agent turned it off.
        assert_eq!(op_of(r#"{"v":1,"op":"record.start","countdown":false,"label":false}"#), Some(CliOp::Start(StartAsk { countdown: false, label: false })));
        assert_eq!(op_of(r#"{"v":1,"op":"record.start","label":false}"#), Some(CliOp::Start(StartAsk { countdown: true, label: false })));
        assert_eq!(op_of(r#"{"v":1,"op":"record.stop"}"#), Some(CliOp::Stop));
        assert_eq!(op_of(r#"{"v":2,"op":"record.start"}"#), None);
        assert_eq!(op_of(r#"{"v":1,"op":"delete"}"#), None);
        assert_eq!(op_of("not json"), None);
    }

    /// What the run log keeps of an ask and its answer (Bagrat, 2026-10-18: enough to report
    /// a problem): the op, how a start goes, the code, the time — never the link.
    #[test]
    fn the_log_has_each_ask_and_its_answer_by_number() {
        assert_eq!(asked(3, &CliOp::START), "control: ask 3 record.start");
        assert_eq!(asked(4, &CliOp::Start(StartAsk { countdown: false, label: false })), "control: ask 4 record.start no-count no-label");
        assert_eq!(asked(5, &CliOp::Stop), "control: ask 5 record.stop");
        let ok = CliAnswer { id: 5, ok: true, take: "stopping", error: None, link: Some("https://v.x.workers.dev/AbCdEfGhIjKlMnOp".into()) };
        assert_eq!(answered(&ok, Duration::from_millis(41)), "control: answer 5 ok stopping (41 ms)");
        let no = CliAnswer { id: 6, ok: false, take: "none", error: Some("recording_off"), link: None };
        assert_eq!(answered(&no, Duration::from_millis(3)), "control: answer 6 no recording_off (3 ms)");
        for l in [asked(4, &CliOp::Start(StartAsk { countdown: false, label: true })), answered(&ok, Duration::ZERO)] {
            assert_eq!(shroom_core::report::clean_line(&l), l, "nothing in it for the report to take out");
        }
    }

    #[test]
    fn an_answer_is_a_code_and_a_link_never_words() {
        let a = CliAnswer { id: 3, ok: true, take: "stopping", error: None, link: Some("https://v.x.workers.dev/AbCdEfGhIjKlMnOp".into()) };
        assert_eq!(line_of(&a), "{\"link\":\"https://v.x.workers.dev/AbCdEfGhIjKlMnOp\",\"ok\":true,\"take\":\"stopping\"}\n");
        let no = CliAnswer { id: 4, ok: false, take: "none", error: Some("recording_off"), link: None };
        assert_eq!(line_of(&no), "{\"error\":\"recording_off\",\"ok\":false,\"take\":\"none\"}\n");
    }
}
