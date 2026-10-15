//! Updates against a local stand-in for GitHub releases: the check, a verified download,
//! a mismatch that leaves nothing, and the swap helper — waiting for the app to quit,
//! swapping, reopening, and putting the old copy back when the swap can't happen.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use shroom_update::{Offer, Release, SWAP_SH, UpdateError, Version, check, download, offer};

/// A test's own folder, removed when the test ends (pass or fail).
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let d = std::env::temp_dir().join(format!("shroom-update-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Scratch(d)
}

/// A release zip of a fake shroom.app carrying `version`.
fn release_zip(dir: &Path, version: &str) -> (String, Vec<u8>) {
    let app = dir.join("stage/shroom.app/Contents");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("version"), version).unwrap();
    let file = format!("shroom-{version}-arm64.zip");
    let zip = dir.join(&file);
    assert!(Command::new("ditto").args(["-c", "-k", "--keepParent"]).arg(dir.join("stage/shroom.app")).arg(&zip).status().unwrap().success());
    (file, std::fs::read(zip).unwrap())
}

fn serve(files: Vec<(String, Vec<u8>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let files = Arc::new(Mutex::new(files));
    std::thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut line = String::new();
            let mut r = BufReader::new(s.try_clone().unwrap());
            r.read_line(&mut line).unwrap();
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                if h.trim().is_empty() { break; }
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let found = files.lock().unwrap().iter().find(|(p, _)| *p == path).map(|(_, b)| b.clone());
            let (status, body) = match found { Some(b) => ("200 OK", b), None => ("404 Not Found", Vec::new()) };
            let _ = s.write_all(format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len()).as_bytes());
            let _ = s.write_all(&body);
        }
    });
    format!("http://{addr}")
}

fn sha(b: &[u8]) -> String {
    shroom_storage::sigv4::sha256_hex(b)
}

#[test]
fn checks_downloads_and_verifies_like_the_installer() {
    let dir = scratch("dl");
    let (file, bytes) = release_zip(&dir, "1.3.0");
    let manifest = format!("version=1.3.0\nmacos=14\narm64={file} {}\n", sha(&bytes));
    let base = serve(vec![
        ("/latest/download/manifest.txt".into(), manifest.into_bytes()),
        (format!("/download/v1.3.0/{file}"), bytes.clone()),
    ]);
    let m = check(&base).unwrap();
    let Offer::Ready(release) = offer(&m, Version(1, 2, 0), "arm64", 15) else { panic!("an update is offered") };
    let staging = dir.join("updates");
    let app = download(&base, &release, &staging).unwrap();
    assert_eq!(std::fs::read_to_string(app.join("Contents/version")).unwrap(), "1.3.0");
    assert!(!staging.join("1.3.0").join(&file).exists(), "the zip doesn't linger");

    let tampered = Release { sha256: "0".repeat(64), ..release };
    assert!(matches!(download(&base, &tampered, &staging), Err(UpdateError::Mismatch)));
    assert!(!staging.join("1.3.0").exists(), "a mismatch leaves nothing behind");
}

#[test]
fn the_app_reads_update_txt_and_a_release_without_one_its_manifest() {
    let manifest = |v: &str| format!("version={v}\nmacos=14\narm64=shroom-{v}-arm64.zip {}\n", "a".repeat(64)).into_bytes();
    // update.txt is the same file under the app's own name; a different version here
    // shows which of the two was read.
    let both = serve(vec![("/latest/download/update.txt".into(), manifest("1.4.0")), ("/latest/download/manifest.txt".into(), manifest("1.3.0"))]);
    assert_eq!(check(&both).unwrap().version, Version(1, 4, 0));
    let older = serve(vec![("/latest/download/manifest.txt".into(), manifest("1.3.0"))]);
    assert_eq!(check(&older).unwrap().version, Version(1, 3, 0));
    assert!(matches!(check(&serve(vec![])), Err(UpdateError::Transport(_))), "neither: fails soft");
}

#[test]
fn a_check_with_no_network_fails_soft() {
    assert!(matches!(check("http://127.0.0.1:9"), Err(UpdateError::Transport(_))));
}

fn stub_open(dir: &Path) -> (PathBuf, PathBuf) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let log = dir.join("open.log");
    std::fs::write(bin.join("open"), format!("#!/bin/sh\necho \"$*\" >> {}\n", log.display())).unwrap();
    Command::new("chmod").arg("+x").arg(bin.join("open")).status().unwrap();
    (bin, log)
}

fn run_swap(dir: &Path, bin: &Path, pid: u32, staged: &Path, app: &Path) -> bool {
    Command::new("/bin/sh")
        .arg("-c").arg(SWAP_SH).arg("shroom-swap")
        .arg(pid.to_string()).arg(staged).arg(app).arg("--updated").arg("--update-failed")
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .current_dir(dir)
        .status().unwrap().success()
}

#[test]
fn the_swap_waits_for_the_app_then_swaps_and_reopens() {
    let dir = scratch("swap");
    let (bin, log) = stub_open(&dir);
    let app = dir.join("Applications/shroom.app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("v"), "1.2.0").unwrap();
    let staged = dir.join("updates/1.3.0/shroom.app");
    std::fs::create_dir_all(&staged).unwrap();
    std::fs::write(staged.join("v"), "1.3.0").unwrap();

    // The "running app": the swap must not start until it's gone. (Reaped by a thread,
    // as launchd reaps the real app — an unreaped child still answers kill -0.)
    let mut running = Command::new("sleep").arg("1").spawn().unwrap();
    let pid = running.id();
    std::thread::spawn(move || running.wait());
    let t = std::time::Instant::now();
    assert!(run_swap(&dir, &bin, pid, &staged, &app));
    assert!(t.elapsed() >= std::time::Duration::from_millis(900), "it waited for the app to quit");
    assert_eq!(std::fs::read_to_string(app.join("v")).unwrap(), "1.3.0");
    assert!(std::fs::read_to_string(&log).unwrap().contains("--args --updated"));
    let leftovers: Vec<_> = std::fs::read_dir(dir.join("Applications")).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(leftovers.len(), 1, "no old copy left: {leftovers:?}");
}

#[test]
fn a_swap_that_cant_happen_keeps_the_running_version() {
    let dir = scratch("rollback");
    let (bin, log) = stub_open(&dir);
    let app = dir.join("Applications/shroom.app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("v"), "1.2.0").unwrap();
    let missing = dir.join("updates/1.3.0/shroom.app"); // never staged
    assert!(run_swap(&dir, &bin, 999_999, &missing, &app));
    assert_eq!(std::fs::read_to_string(app.join("v")).unwrap(), "1.2.0");
    assert!(std::fs::read_to_string(&log).unwrap().contains("--args --update-failed"));
}
