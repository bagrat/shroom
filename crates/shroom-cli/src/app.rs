//! Recording goes through the app, never around it: the screen, camera and microphone
//! grants belong to shroom.app (TCC charges a capture to the process that asks — a CLI
//! child would prompt as the terminal; history/POC-LEARNINGS.md), and the countdown, the
//! tray and the stop are the app's. The CLI asks the running app over a Unix socket in
//! shroom's home (`control.sock`: the folder is 700, the app checks the peer's uid), one
//! JSON line each way:
//!
//!   → {"v":1,"op":"record.start"}            ← {"ok":true,"take":"countdown"}
//!   → {"v":1,"op":"record.stop"}             ← {"ok":false,"error":"no_take","message":"…"}
//!
//! The app side is proposed, not built (dev/proposals/cli-agent-skill.md §3).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::out::Fail;

/// The ops the app takes. Each is what a click in the popover does, nothing more.
pub const OPS: [&str; 4] = ["record.start", "record.pause", "record.resume", "record.stop"];

/// Asks the app once. With `launch`, opens shroom first when it isn't running and waits
/// (≤ 15 s) for it to listen.
pub fn ask(socket: &Path, op: &str, launch: bool) -> Result<Value, Fail> {
    let mut stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(_) if launch => {
            open_app()?;
            wait_for(socket, Duration::from_secs(15))?
        }
        Err(_) => return Err(Fail::AppUnreachable("shroom isn't running. Open it, or leave out --no-launch.".into())),
    };
    stream.set_read_timeout(Some(Duration::from_secs(20))).ok();
    let line = format!("{}\n", json!({"v": 1, "op": op}));
    stream.write_all(line.as_bytes()).map_err(|_| Fail::AppUnreachable("shroom didn't take the request.".into()))?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).map_err(|_| Fail::AppUnreachable("shroom didn't answer.".into()))?;
    let v: Value = serde_json::from_str(answer.trim()).map_err(|_| Fail::AppUnreachable("shroom's answer wasn't readable.".into()))?;
    if v["ok"] == true {
        return Ok(v);
    }
    let message = v["message"].as_str().unwrap_or("shroom said no.").to_string();
    Err(match v["error"].as_str() {
        Some("take_open") => Fail::TakeOpen(message),
        Some("not_set_up") => Fail::NotSetUp(message),
        _ => Fail::Refused(message),
    })
}

fn open_app() -> Result<(), Fail> {
    if !cfg!(target_os = "macos") {
        return Err(Fail::AppUnreachable("shroom isn't running, and it runs only on a Mac.".into()));
    }
    let ok = std::process::Command::new("open").args(["-g", "-b", "am.shroom"]).status().is_ok_and(|s| s.success());
    if ok { Ok(()) } else { Err(Fail::AppUnreachable("shroom couldn't be opened. Is it installed?".into())) }
}

fn wait_for(socket: &Path, budget: Duration) -> Result<UnixStream, Fail> {
    let start = Instant::now();
    while start.elapsed() < budget {
        if let Ok(s) = UnixStream::connect(socket) {
            return Ok(s);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(Fail::AppUnreachable("shroom opened but didn't answer in time.".into()))
}
