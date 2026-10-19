//! `shroom edit` (spec N70) against the real Worker (`worker/dev/serve.mjs`: worker.js over
//! an in-memory space, its S3 surface checking every credential): what a video's edit is,
//! one change a call saved as the editor's Done saves, refused for the editor's own reasons,
//! a dry run that saves nothing, and back to the whole recording. Needs node; without it the
//! test says so and passes as skipped.

#[path = "../examples/demo_space.rs"]
#[allow(dead_code)]
mod demo;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use serde_json::Value;

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn serve() -> Option<(Serve, String)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut child = Command::new("node")
        .args([&format!("{root}/worker/dev/serve.mjs"), "--space", "v.myshroom.workers.dev=Ann"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut line = String::new();
    BufReader::new(child.stdout.take()?).read_line(&mut line).ok()?;
    let url = serde_json::from_str::<Value>(&line).ok()?["url"].as_str()?.to_string();
    Some((Serve(child), url))
}

fn shroom(home: &Path, base: &str, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).arg("--json").env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    (out.status.code().unwrap_or(-1), serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

/// A stand-in app on the control socket that answers one ask with `answer`; the ask it heard.
fn app(home: &Path, answer: &'static str) -> std::thread::JoinHandle<Value> {
    let listener = UnixListener::bind(home.join("control.sock")).unwrap();
    std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut ask = String::new();
        BufReader::new(s.try_clone().unwrap()).read_line(&mut ask).unwrap();
        s.write_all(format!("{answer}\n").as_bytes()).unwrap();
        serde_json::from_str(&ask).unwrap()
    })
}

const ID: &str = "St4ndupN0t3sJn05"; // 133 s

#[test]
fn an_agent_reads_cuts_trims_and_reverts_a_videos_edit() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    // Not edited yet: the whole recording plays.
    let (code, v) = shroom(&home, &base, &["edit", ID]);
    assert_eq!(code, 0, "{v}");
    assert_eq!((v["edit"]["plays_sec"].as_f64(), v["edit"]["recording_sec"].as_f64()), (Some(133.0), Some(133.0)), "{v}");
    assert_eq!((v["edit"]["edited"].as_bool(), v["edit"]["final_video"].as_str()), (Some(false), Some("none")), "{v}");

    // A dry run says what the edit would be and saves nothing.
    let (code, v) = shroom(&home, &base, &["edit", ID, "cut", "10", "0:20", "--dry-run"]);
    assert_eq!((code, v["saved"].as_bool(), v["edit"]["plays_sec"].as_f64()), (0, Some(false), Some(123.0)), "{v}");
    let (_, v) = shroom(&home, &base, &["edit", ID]);
    assert_eq!(v["edit"]["edited"], false, "the dry run saved nothing: {v}");

    // A cut, saved: the link plays it at once; the app wasn't asked (--no-launch).
    let (code, v) = shroom(&home, &base, &["edit", ID, "cut", "10", "0:20", "--no-launch"]);
    assert_eq!((code, v["saved"].as_bool()), (0, Some(true)), "{v}");
    assert_eq!(v["rev"], 1, "{v}");
    assert_eq!(v["edit"]["plays_sec"].as_f64(), Some(123.0));
    assert_eq!(v["edit"]["cuts"][0]["id"], "m1");
    assert_eq!((v["edit"]["cuts"][0]["start"].as_f64(), v["edit"]["cuts"][0]["end"].as_f64()), (Some(10.0), Some(20.0)));
    assert_eq!((v["edit"]["final_video"].as_str(), v["final"].as_str()), (Some("not_yet"), Some("later")), "{v}");
    // Read back from the record.
    let (_, v) = shroom(&home, &base, &["edit", ID]);
    assert_eq!((v["edit"]["edited"].as_bool(), v["edit"]["cuts"].as_array().map(Vec::len)), (Some(true), Some(1)), "{v}");

    // A trim on top of it.
    let (code, v) = shroom(&home, &base, &["edit", ID, "trim", "2.5", "2:03", "--no-launch"]);
    assert_eq!((code, v["rev"].as_u64()), (0, Some(2)), "{v}");
    assert_eq!((v["edit"]["trim"]["in"].as_f64(), v["edit"]["trim"]["out"].as_f64()), (Some(2.5), Some(123.0)));
    assert_eq!(v["edit"]["plays_sec"].as_f64(), Some(110.5));

    // The cut moved by its id.
    let (code, v) = shroom(&home, &base, &["edit", ID, "resize", "m1", "10", "15", "--no-launch"]);
    assert_eq!((code, v["edit"]["plays_sec"].as_f64()), (0, Some(115.5)), "{v}");

    // Refused, each for the editor's own reason, nothing saved.
    for (args, reason) in [
        (vec!["cut", "0", "2:13"], "leaves_too_little"),
        (vec!["cut", "40", "40.05"], "too_short"),
        (vec!["cut", "12", "14"], "nothing_changes"),
        (vec!["cut", "125", "130"], "out_of_range"),
        (vec!["put-back", "m7"], "no_such_cut"),
        (vec!["apply", "m1"], "no_such_cut"),
    ] {
        let mut a = vec!["edit", ID];
        a.extend(args.iter());
        a.push("--no-launch");
        let (code, v) = shroom(&home, &base, &a);
        assert_eq!((code, v["error"].as_str(), v["reason"].as_str()), (12, Some("cant_edit"), Some(reason)), "{args:?}: {v}");
        assert!(v["message"].as_str().is_some_and(|m| !m.is_empty()), "{v}");
    }
    let (_, v) = shroom(&home, &base, &["edit", ID]);
    assert_eq!(v["rev"], 3, "nothing refused was saved: {v}");

    // Put back, then back to the whole recording: no final video to make.
    let (code, v) = shroom(&home, &base, &["edit", ID, "put-back", "m1", "--no-launch"]);
    assert_eq!((code, v["edit"]["cuts"].as_array().map(Vec::len)), (0, Some(0)), "{v}");
    let (code, v) = shroom(&home, &base, &["edit", ID, "revert"]);
    assert_eq!((code, v["edit"]["edited"].as_bool(), v["final"].as_str()), (0, Some(false), Some("not_needed")), "{v}");
    let (code, v) = shroom(&home, &base, &["edit", ID, "revert", "--no-launch"]);
    assert_eq!((code, v["reason"].as_str()), (12, Some("nothing_changes")), "{v}");

    // A word that isn't a change, or a time that isn't one, is the command line's fault.
    for args in [vec!["edit", ID, "shorten", "5"], vec!["edit", ID, "cut", "ten", "20"], vec!["edit", ID, "trim", "5"]] {
        let (code, v) = shroom(&home, &base, &args);
        assert_eq!((code, v["error"].as_str()), (2, Some("usage")), "{args:?}: {v}");
    }
}

#[test]
fn a_saved_edit_asks_the_app_for_its_final_video() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);
    let heard = app(&home, r#"{"ok":true,"take":"none"}"#);
    let (code, v) = shroom(&home, &base, &["edit", ID, "cut", "30", "40", "--no-launch"]);
    assert_eq!((code, v["final"].as_str()), (0, Some("asked")), "the app took the ask: {v}");
    let ask = heard.join().unwrap();
    assert_eq!((ask["op"].as_str(), ask["id"].as_str(), ask["rev"].as_u64()), (Some("edit.copy"), Some(ID), Some(1)), "{ask}");
    let _ = std::fs::remove_file(home.join("control.sock"));

    // `finish` asks again for the edit as saved.
    let heard = app(&home, r#"{"ok":true,"take":"none"}"#);
    let (code, v) = shroom(&home, &base, &["edit", ID, "finish", "--no-launch"]);
    assert_eq!((code, v["final"].as_str()), (0, Some("asked")), "{v}");
    assert_eq!(heard.join().unwrap()["rev"], 1);
}

#[test]
fn it_changes_nothing_while_this_macs_editor_holds_the_video_or_a_take_is_open() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);
    let me = std::process::id();
    std::fs::write(home.join("status.json"), format!(r#"{{"v":2,"pid":{me},"take_open":false,"editing":"{ID}"}}"#)).unwrap();
    let (code, v) = shroom(&home, &base, &["edit", ID, "cut", "30", "40", "--no-launch"]);
    assert_eq!((code, v["error"].as_str(), v["reason"].as_str()), (12, Some("cant_edit"), Some("editor_open")), "{v}");
    let (code, v) = shroom(&home, &base, &["edit", ID]);
    assert_eq!((code, v["edit"]["edited"].as_bool()), (0, Some(false)), "reading it is fine; nothing was saved: {v}");

    std::fs::write(home.join("status.json"), format!(r#"{{"v":2,"pid":{me},"take_open":true}}"#)).unwrap();
    let (code, v) = shroom(&home, &base, &["edit", ID, "cut", "30", "40", "--no-launch"]);
    assert_eq!((code, v["error"].as_str()), (9, Some("take_open")), "law 4: {v}");
}
