//! The `shroom` command's door into the running app (DECISIONS B216,
//! spec N62 provisional): `control.sock` in this flavor's home, one JSON line each way.
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

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shroom_core::effect::CliAnswer;
use shroom_core::event::{CliOp, Event};

use super::Runtime;

/// The connections waiting on the core's answer, by the id their `Event::Cli` carries.
#[derive(Clone, Default)]
pub struct Pending {
    next: Arc<AtomicU64>,
    open: Arc<Mutex<HashMap<u64, UnixStream>>>,
}

/// The longest ask read: the op and its version, nothing more.
const MAX_ASK: u64 = 1024;

pub fn op_of(line: &str) -> Option<CliOp> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if v["v"] != 1 {
        return None;
    }
    match v["op"].as_str()? {
        "record.start" => Some(CliOp::Start),
        "record.pause" => Some(CliOp::Pause),
        "record.resume" => Some(CliOp::Resume),
        "record.stop" => Some(CliOp::Stop),
        _ => None,
    }
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
                let _ = conn.write_all(b"{\"ok\":false,\"take\":\"none\",\"error\":\"usage\"}\n");
                continue;
            };
            // Start is the popover's Start: the grants as macOS has them now go first, so
            // the core arms on the truth (commands::arms).
            if op == CliOp::Start {
                rt.post(Event::Os(super::os::grants()));
            }
            let id = pending.next.fetch_add(1, Ordering::SeqCst) + 1;
            pending.open.lock().unwrap().insert(id, conn);
            rt.log(&format!("control: the shroom command asks {op:?}"));
            rt.post(Event::Cli { id, op });
        }
    });
}

/// The core's answer, onto the connection that asked; then it closes.
pub fn answer(rt: &Runtime, pending: &Pending, a: CliAnswer) {
    let Some(mut conn) = pending.open.lock().unwrap().remove(&a.id) else { return };
    rt.log(&format!("control: answered {} {}", if a.ok { "ok" } else { "no" }, a.error.unwrap_or(a.take)));
    let _ = conn.write_all(line_of(&a).as_bytes());
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

    #[test]
    fn an_answer_is_a_code_and_a_link_never_words() {
        let a = CliAnswer { id: 3, ok: true, take: "stopping", error: None, link: Some("https://v.x.workers.dev/AbCdEfGhIjKlMnOp".into()) };
        assert_eq!(line_of(&a), "{\"link\":\"https://v.x.workers.dev/AbCdEfGhIjKlMnOp\",\"ok\":true,\"take\":\"stopping\"}\n");
        let no = CliAnswer { id: 4, ok: false, take: "none", error: Some("recording_off"), link: None };
        assert_eq!(line_of(&no), "{\"error\":\"recording_off\",\"ok\":false,\"take\":\"none\"}\n");
    }
}
