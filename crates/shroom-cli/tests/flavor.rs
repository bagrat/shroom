//! The command follows the app it ships in (spec N62, DECISIONS B215): inside a bundle, the
//! bundle's identifier picks shroom's folder by the app's own rule — a dev build's command
//! reads `~/.shroom-dev`, never the release app's `~/.shroom` — and a link to it (the
//! install line's `~/.local/bin/shroom`) is the same command. Off a bundle, `~/.shroom`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// `<root>/Applications/shroom.app` with this identifier, the command inside it.
fn bundle(root: &Path, id: &str) -> PathBuf {
    let contents = root.join("Applications/shroom.app/Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    std::fs::write(
        contents.join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key>
	<string>shroom</string>
	<key>CFBundleIdentifier</key>
	<string>{id}</string>
	<key>CFBundleName</key>
	<string>shroom</string>
</dict>
</plist>
"#
        ),
    )
    .unwrap();
    let cli = contents.join("MacOS/shroom-cli");
    std::fs::copy(env!("CARGO_BIN_EXE_shroom"), &cli).unwrap();
    cli
}

/// A space in `<home>/<folder>`, by its host.
fn space(home: &Path, folder: &str, host: &str) {
    let dir = home.join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("space.json"),
        format!(r#"{{"account_id":"acc","label":"v","subdomain":"x","host":"{host}","bucket":"shroom-space","role":"Member","member_id":"m_x","owner_name":"Ann"}}"#),
    )
    .unwrap();
}

fn status(exe: &Path, home: &Path) -> Value {
    let out = Command::new(exe).args(["status", "--json"]).env("HOME", home).env_remove("SHROOM_HOME").output().unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}

fn two_spaces() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    space(&home, ".shroom", "v.release.workers.dev");
    space(&home, ".shroom-dev", "v.dev.workers.dev");
    (tmp, home)
}

#[test]
fn a_dev_builds_command_reads_the_dev_apps_folder_through_its_link_too() {
    let (tmp, home) = two_spaces();
    let cli = bundle(tmp.path(), "am.shroom.dev");
    let v = status(&cli, &home);
    assert_eq!(v["space"]["host"], "v.dev.workers.dev", "{v}");
    assert_eq!(v["app"]["id"], "am.shroom.dev", "{v}");
    // On PATH the way the install line puts it: a link to the command in the bundle.
    let bin = tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(&cli, bin.join("shroom")).unwrap();
    let v = status(&bin.join("shroom"), &home);
    assert_eq!((v["space"]["host"].as_str(), v["app"]["id"].as_str()), (Some("v.dev.workers.dev"), Some("am.shroom.dev")), "{v}");
}

#[test]
fn the_release_apps_command_reads_the_release_folder() {
    let (tmp, home) = two_spaces();
    let v = status(&bundle(tmp.path(), "am.shroom"), &home);
    assert_eq!((v["space"]["host"].as_str(), v["app"]["id"].as_str()), (Some("v.release.workers.dev"), Some("am.shroom")), "{v}");
}

#[test]
fn off_a_bundle_it_is_the_release_apps() {
    let (_tmp, home) = two_spaces();
    let v = status(Path::new(env!("CARGO_BIN_EXE_shroom")), &home);
    assert_eq!((v["space"]["host"].as_str(), v["app"]["id"].as_str()), (Some("v.release.workers.dev"), Some("am.shroom")), "{v}");
}
