//! L3: kill tests. A recording dies mid-take — as an app crash and as a power loss — and
//! repair must save what's on disk (law 2), frame for frame. Needs ffmpeg on PATH: without
//! it these tests fail, never pass silently (DECISIONS B59 c).

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// The decoder that checks a repaired take: the ffmpeg on PATH. (The encoder under test —
/// the recorder's and repair's — is `SHROOM_FFMPEG` when set, the release's sidecar (B82).)
fn oracle() -> std::path::PathBuf {
    let out = Command::new("sh").args(["-c", "command -v ffmpeg"]).output().expect("sh runs");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!p.is_empty(), "ffmpeg isn't on PATH — this test needs it (brew install ffmpeg)");
    std::path::PathBuf::from(p)
}

fn counters(ffmpeg: &Path, input: &Path) -> Vec<Option<u32>> {
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; 640 * 360];
    let mut v = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        v.push(shroom_testpattern::read_gray(&frame, 640, 360));
    }
    let _ = child.wait();
    v
}

fn crash_and_repair(mode: &str, seconds: f64) -> (f64, shroom_testpattern::Summary) {
    let ffmpeg = std::env::var_os("SHROOM_FFMPEG").filter(|p| !p.is_empty()).map(std::path::PathBuf::from).unwrap_or_else(oracle);
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_shroom-crashtest"))
        .arg(dir.path())
        .arg(seconds.to_string())
        .arg(mode)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success(), "the recorder died, as planned");
    // An orphaned encoder finishes on its own after losing its input.
    std::thread::sleep(Duration::from_secs(2));
    let repaired = shroom_record::repair::repair(&ffmpeg, dir.path()).expect("something was saved");
    let s = shroom_testpattern::summarize(&counters(&oracle(), &dir.path().join("stream.m3u8")));
    (repaired.seconds, s)
}

#[test]
fn an_app_crash_keeps_the_whole_take() {
    let (seconds, s) = crash_and_repair("app", 9.0);
    eprintln!("app crash at 9 s: {seconds:.2} s saved, {s:?}");
    assert!(seconds >= 8.5, "an app crash keeps everything up to the crash: {seconds}");
    assert_eq!((s.unreadable, s.backwards), (0, 0));
}

#[test]
fn a_power_loss_loses_at_most_about_a_second() {
    let (seconds, s) = crash_and_repair("power", 9.5);
    eprintln!("power loss at 9.5 s: {seconds:.2} s saved, {s:?}");
    assert!(seconds >= 8.5, "fragments keep all but the last second: {seconds}");
    assert_eq!((s.unreadable, s.backwards), (0, 0));
}

#[test]
fn a_power_loss_in_the_first_seconds_still_saves_something() {
    let (seconds, _) = crash_and_repair("power", 3.5);
    eprintln!("power loss at 3.5 s: {seconds:.2} s saved");
    assert!(seconds >= 2.5, "before the first segment existed: {seconds}");
}
