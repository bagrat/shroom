//! `shroom setup` (spec N89): the setup a coding agent drives. A stand-in app answers on the
//! control socket, as the app does. The token piped in reaches the app once, in its own ask,
//! and nowhere else: not the command's output, not its log, not its arguments (DEVIATIONS
//! #59).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

const CANARY: &str = "cfat_CanaryCanaryCanaryCanaryCanary0123456789";

/// A stand-in app answering each ask, in order, with `answers`; what it was asked.
fn app(home: &Path, answers: Vec<Value>) -> std::thread::JoinHandle<Vec<String>> {
    let listener = UnixListener::bind(home.join("control.sock")).unwrap();
    std::thread::spawn(move || {
        let mut asked = Vec::new();
        for a in answers {
            let (mut s, _) = listener.accept().unwrap();
            let mut ask = String::new();
            BufReader::new(s.try_clone().unwrap()).read_line(&mut ask).unwrap();
            asked.push(ask);
            s.write_all(format!("{a}\n").as_bytes()).unwrap();
        }
        asked
    })
}

fn run(home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_shroom-cli"))
        .args(args)
        .env("SHROOM_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    c.wait_with_output().unwrap()
}

fn answer(at: &str, took: bool, busy: bool) -> Value {
    json!({"ok": true, "take": "none", "setup": {"at": at, "took": took, "busy": busy, "signed_up": false, "accounts": []}})
}

#[test]
fn the_token_piped_in_reaches_the_app_once_and_is_never_said() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let asked = app(home, vec![answer("token", true, true), answer("gate-card", false, false)]);
    let out = run(home, &["setup", "token", "--json", "--no-launch"], &format!("{CANARY}\n"));
    let asked = asked.join().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let ask: Value = serde_json::from_str(&asked[0]).unwrap();
    assert_eq!((ask["op"].as_str(), ask["token"].as_str()), (Some("setup.token"), Some(CANARY)), "the token, trimmed, in its own ask");
    assert!(!asked[1].contains(CANARY), "only once: the wait's polls never carry it");
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(!said.contains("cfat_"), "never said: {said}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["at"], "gate-card", "the check's answer is waited for");
    assert_eq!(v["next"], "shroom setup card open --json");
    let log = std::fs::read_to_string(home.join("logs/cli.log")).unwrap_or_default();
    assert!(!log.contains("cfat_") && log.contains("setup token"), "the log names the step only: {log}");
}

#[test]
fn nothing_to_read_says_so_and_asks_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    // No stand-in: an ask would fail as unreachable, not as this.
    let out = run(tmp.path(), &["setup", "token", "--json", "--no-launch"], "  \n");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    // Off a Mac there's no clipboard either.
    assert_eq!(v["error"], "refused");
    assert!(v["message"].as_str().unwrap().contains("Press Copy"));
}

#[test]
fn every_answer_has_the_line_to_say_and_the_next_command() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let asked = app(home, vec![answer("account", false, false)]);
    let out = run(home, &["setup", "--json", "--no-launch"], "");
    let asked = asked.join().unwrap();
    assert!(asked[0].contains("\"setup.status\""));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["at"], "account");
    assert_eq!(v["say"], "Your videos will live in your own Cloudflare account. Do you have one already?");
    assert_eq!(v["next"], "shroom setup account have --json");
    assert_eq!(v["or"], "shroom setup account new --json");
    assert!(v["do"].is_string());
}
