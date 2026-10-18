//! The command's recording refusal (spec N62, DECISIONS B216; Bagrat, 2026-10-18): with
//! General's switch off, the app answers `recording_off`, and the command says three things —
//! it's off, the switch in Settings turns it on, and a link that opens Settings at that
//! switch — the link in its own field with `--json`, exit 10 either way. A stand-in app
//! answers on the control socket, as the app does.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

const LINK: &str = "shroom://settings/agent";

/// Runs `shroom record start --no-launch` (and `extra`) against a stand-in app that refuses
/// with `code`: what the command printed, and how it exited.
fn refused(code: &str, extra: &[&str]) -> Output {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let listener = UnixListener::bind(home.join("control.sock")).unwrap();
    let answer = format!("{{\"ok\":false,\"take\":\"none\",\"error\":\"{code}\"}}\n");
    let app = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut ask = String::new();
        BufReader::new(s.try_clone().unwrap()).read_line(&mut ask).unwrap();
        s.write_all(answer.as_bytes()).unwrap();
        ask
    });
    let out = run(home, &[&["record", "start", "--no-launch"], extra].concat());
    let ask = app.join().unwrap();
    assert!(ask.contains("\"record.start\""), "{ask}");
    out
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).env("SHROOM_HOME", home).output().unwrap()
}

#[test]
fn recording_off_says_its_off_how_to_turn_it_on_and_gives_the_link() {
    let out = refused("recording_off", &["--json"]);
    assert_eq!(out.status.code(), Some(10));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stdout)));
    assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("recording_off")));
    assert_eq!(v["settings_link"], LINK, "the link in its own field, for the agent to hand over");
    let message = v["message"].as_str().unwrap();
    let lines: Vec<&str> = message.lines().collect();
    assert_eq!(lines.len(), 3, "{message}");
    assert!(lines[0].contains("off"), "it's off: {message}");
    assert!(lines[1].contains("“Let it start and stop recordings”") && lines[1].contains("Settings"), "the switch turns it on: {message}");
    assert!(lines[2].ends_with(LINK), "the link, last: {message}");
}

#[test]
fn a_person_reads_the_same_three_lines_and_the_link() {
    let out = refused("recording_off", &[]);
    assert_eq!(out.status.code(), Some(10));
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(said.trim_end().lines().count(), 3, "{said}");
    assert!(said.trim_end().ends_with(LINK), "{said}");
}

#[test]
fn no_other_refusal_carries_the_link() {
    for code in ["not_set_up", "take_open", "no_take", "busy", "not_ready"] {
        let out = refused(code, &["--json"]);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(v.get("settings_link").is_none(), "{code}: {v}");
        assert!(!v["message"].as_str().unwrap().contains(LINK), "{code}: {v}");
    }
}
