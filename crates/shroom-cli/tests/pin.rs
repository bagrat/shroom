//! `shroom pin` / `shroom unpin` (spec N74): a pinned video is never suggested and never
//! deleted by auto-cleanup. Many at once, each its own result, safe to run again; `--dry-run`
//! says what would change. Against the real Worker (`worker/dev/serve.mjs`), as
//! `tests/space.rs` is; without node the test says so and passes as skipped.

#[path = "../examples/demo_space.rs"]
#[allow(dead_code)]
mod demo;

use std::io::{BufRead, BufReader};
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

fn shroom(home: &std::path::Path, base: &str, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).arg("--json").env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    (out.status.code().unwrap_or(-1), serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

fn text(home: &std::path::Path, base: &str, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn statuses(v: &Value) -> Vec<&str> {
    v["results"].as_array().unwrap().iter().map(|x| x["status"].as_str().unwrap()).collect()
}

#[test]
fn pin_and_unpin_one_or_many_and_a_re_run_is_safe() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    // A dry run changes nothing.
    let (code, v) = shroom(&home, &base, &["pin", "0ldD3m0F0rAcm3Co", "--dry-run"]);
    assert_eq!((code, statuses(&v)), (0, vec!["would_pin"]), "{v}");
    assert_eq!(v["dry_run"], true);
    assert_eq!(shroom(&home, &base, &["info", "0ldD3m0F0rAcm3Co"]).1["video"]["pinned"], false);

    // One, then the list and info say so.
    let t = text(&home, &base, &["pin", "0ldD3m0F0rAcm3Co"]);
    assert_eq!(t.trim(), "Pinned “Old demo for a prospect”. It won't be suggested for cleanup.");
    assert_eq!(shroom(&home, &base, &["info", "0ldD3m0F0rAcm3Co"]).1["video"]["pinned"], true);
    let list = text(&home, &base, &["list"]);
    assert!(list.lines().any(|l| l.starts_with("0ldD3m0F0rAcm3Co") && l.ends_with("Old demo for a prospect · pinned")), "{list}");
    assert!(text(&home, &base, &["info", "0ldD3m0F0rAcm3Co"]).contains("· pinned"));

    // Several: one pinned already, one not here; each its own result, no --yes needed.
    let (code, v) = shroom(&home, &base, &["pin", "0ldD3m0F0rAcm3Co", "https://v.myshroom.workers.dev/Fr1st3v3rT3st000", "NoSuchVide0Here1"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(statuses(&v), ["unchanged", "pinned", "not_found"]);
    assert_eq!(v["results"][1]["title"], "Untitled — Feb 2");
    let (_, v) = shroom(&home, &base, &["pin", "0ldD3m0F0rAcm3Co", "Fr1st3v3rT3st000"]);
    assert_eq!(statuses(&v), ["unchanged", "unchanged"], "a re-run pins nothing twice");

    // Title words name nothing here, as for a delete.
    let (code, v) = shroom(&home, &base, &["pin", "old demo"]);
    assert_eq!((code, v["results"][0]["error"].as_str()), (11, Some("usage")), "{v}");

    // Unpin.
    let t = text(&home, &base, &["unpin", "0ldD3m0F0rAcm3Co"]);
    assert_eq!(t.trim(), "Unpinned “Old demo for a prospect”. It can be suggested for cleanup again.");
    let (_, v) = shroom(&home, &base, &["unpin", "0ldD3m0F0rAcm3Co", "Fr1st3v3rT3st000", "--dry-run"]);
    assert_eq!(statuses(&v), ["unchanged", "would_unpin"]);
    let (_, v) = shroom(&home, &base, &["list"]);
    let pinned: Vec<&str> = v["videos"].as_array().unwrap().iter().filter(|x| x["pinned"] == true).map(|x| x["id"].as_str().unwrap()).collect();
    assert_eq!(pinned, ["Fr1st3v3rT3st000"]);

    // A pinned video isn't old enough to go, whatever its age (the by-age fallback too).
    let (_, v) = shroom(&home, &base, &["cleanup-candidates", "--older-than", "200"]);
    assert!(!v["candidates"].as_array().unwrap().iter().any(|c| c["id"] == "Fr1st3v3rT3st000"), "{v}");

    let (code, v) = shroom(&home, &base, &["pin"]);
    assert_eq!((code, v["error"].as_str()), (2, Some("usage")));
}

#[test]
fn autoclean_on_a_fresh_space_is_off() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);
    let (code, v) = shroom(&home, &base, &["autoclean"]);
    assert_eq!((code, v["on"].as_bool(), v["days"].as_u64()), (0, Some(false), Some(0)), "{v}");
    assert_eq!(text(&home, &base, &["autoclean"]).trim(), "Auto-cleanup is off: shroom deletes nothing by itself. It's turned on in shroom's Settings, on the Mac that set up your space.");
}
