//! The command says shroom's version — the bundle's own (app/src-tauri/tauri.conf.json, the
//! one tools/bump-version.sh moves) — never its crate's, which nothing moves: `--version`,
//! `version --json` and `status --json` all name it.

use std::process::Command;

fn shroom_version() -> String {
    let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    conf["version"].as_str().unwrap().to_string()
}

fn run(args: &[&str]) -> String {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).env("SHROOM_HOME", home.path()).output().unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn it_says_shroom_s_version_not_its_crate_s() {
    let want = shroom_version();
    assert_eq!(run(&["--version"]).trim(), format!("shroom {want}"));
    let said: serde_json::Value = serde_json::from_str(&run(&["version", "--json"])).unwrap();
    assert_eq!(said["version"], want.as_str());
    let status: serde_json::Value = serde_json::from_str(&run(&["status", "--json"])).unwrap();
    assert_eq!(status["cli"], want.as_str());
}
