//! The command's own log (Bagrat, 2026-10-18: enough on this Mac to report a problem): every
//! run leaves one line in `logs/cli.log` — what was asked, how it ended, what it counted,
//! how long it took — and never a word it was given or said. A stand-in app answers on
//! the control socket, as the app does.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::Command;

/// Runs the command in `home` with `args`, a stand-in app answering `answer` if given.
fn run(home: &Path, args: &[&str], answer: Option<&str>) -> i32 {
    let app = answer.map(|a| {
        let listener = UnixListener::bind(home.join("control.sock")).unwrap();
        let a = format!("{a}\n");
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut ask = String::new();
            BufReader::new(s.try_clone().unwrap()).read_line(&mut ask).unwrap();
            s.write_all(a.as_bytes()).unwrap();
        })
    });
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).env("SHROOM_HOME", home).output().unwrap();
    if let Some(app) = app {
        app.join().unwrap();
    }
    let _ = std::fs::remove_file(home.join("control.sock"));
    out.status.code().unwrap()
}

fn log(home: &Path) -> Vec<String> {
    std::fs::read_to_string(home.join("logs/cli.log")).unwrap_or_default().lines().map(String::from).collect()
}

#[test]
fn a_start_and_a_refusal_each_leave_their_line() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    assert_eq!(run(home, &["record", "start", "--no-countdown", "--no-launch", "--json"], Some(r#"{"ok":true,"take":"recording"}"#)), 0);
    assert_eq!(run(home, &["record", "start", "--no-launch"], Some(r#"{"ok":false,"take":"none","error":"recording_off"}"#)), 10);
    let lines = log(home);
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert!(lines[0].contains(" UTC cli: record start --json --no-launch --no-countdown: ok, take recording ("), "{}", lines[0]);
    assert!(lines[1].contains(" cli: record start --no-launch: recording_off, exit 10, app recording_off ("), "{}", lines[1]);
    for l in &lines {
        assert!(l.ends_with("ms)"), "how long it took: {l}");
        assert_eq!(&shroom_core::report::clean_line(l), l, "nothing in it for a report to take out");
    }
}

#[test]
fn an_app_that_isnt_there_says_so_in_the_line() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(run(tmp.path(), &["record", "stop", "--no-launch"], None), 8);
    let lines = log(tmp.path());
    assert!(lines[0].contains("cli: record stop --no-launch: app_unreachable, exit 8, app-not-listening ("), "{lines:?}");
}

#[test]
fn never_a_word_it_was_given() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    // No space here: each is refused before anything is asked, but the line is still written.
    run(home, &["rename", "AbCdEfGhIjKlMnOp", "Secret", "quarterly", "plans"], None);
    run(home, &["invite", "Ann", "Lee", "--json"], None);
    run(home, &["list", "--search", "Secret", "--since", "7d"], None);
    run(home, &["Secret", "plans"], None);
    let text = log(home).join("\n");
    assert_eq!(log(home).len(), 4, "{text}");
    for word in ["Secret", "quarterly", "plans", "Ann", "Lee", "7d", "AbCdEfGhIjKlMnOp"] {
        assert!(!text.contains(word), "{word} in the log:\n{text}");
    }
    assert!(text.contains("cli: rename: not_set_up, exit 3"), "{text}");
    assert!(text.contains("cli: list --search --since: not_set_up, exit 3"), "{text}");
    assert!(text.contains("cli: ?: usage, exit 2"), "{text}");
}

#[test]
fn a_mac_where_shroom_never_ran_gets_no_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let never = tmp.path().join("never-ran");
    run(tmp.path(), &["status"], None);
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(["status"]).env("SHROOM_HOME", &never).output().unwrap();
    assert!(out.status.success());
    assert!(!never.exists(), "the command made shroom's folder");
}
