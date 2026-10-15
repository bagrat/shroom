//! The unfurl files against real ffmpeg (R5.3, B27): a poster 1200 wide from the first
//! frame, and a teaser that is small — 640 wide, at most 15 s, faststart — however long
//! the recording. Needs ffmpeg and ffprobe on PATH: without them this test fails, never
//! passes silently.

use std::path::{Path, PathBuf};
use std::process::Command;

use shroom_encode::files;
use shroom_encode::take::{TEASER_SECS, poster, teaser};

fn tool(name: &str) -> PathBuf {
    let out = Command::new("sh").args(["-c", &format!("command -v {name}")]).output().expect("sh runs");
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!path.is_empty(), "{name} isn't on PATH — this test needs it (brew install ffmpeg)");
    PathBuf::from(path)
}

fn probe(ffprobe: &Path, file: &Path, entries: &str) -> String {
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", entries, "-of", "csv=p=0"])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn a_poster_and_a_small_teaser_from_a_long_recording() {
    // The encoder under test (`SHROOM_FFMPEG`: the release's sidecar, B82) makes the poster and
    // teaser; the stand-in recording is the PATH ffmpeg's (the sidecar has no test sources).
    let (ffmpeg, ffprobe) = (shroom_encode::locate::under_test(), tool("ffprobe"));
    let dir = std::env::temp_dir().join(format!("shroom-unfurl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // A 40 s stand-in for the single-file copy, 1920×1080 with sound.
    let made = Command::new(tool("ffmpeg"))
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=1920x1080:rate=30:duration=40"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=40", "-c:v", "h264_videotoolbox", "-b:v", "3000k", "-c:a", "aac", "-shortest"])
        .args(["-movflags", "+faststart", files::PREVIEW])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(made.success());

    let p = poster(&ffmpeg, &dir).expect("poster");
    assert_eq!(probe(&ffprobe, &p, "stream=width,height"), "1200,675");
    assert_eq!(&std::fs::read(&p).unwrap()[..2], &[0xFF, 0xD8], "a JPEG");

    let t = teaser(&ffmpeg, &dir).expect("teaser");
    assert_eq!(probe(&ffprobe, &t, "stream=width,height"), "640,360");
    let secs: f64 = probe(&ffprobe, &t, "format=duration").parse().unwrap();
    assert!(secs <= TEASER_SECS as f64 + 0.1 && secs >= TEASER_SECS as f64 - 0.5, "{secs}");
    let bytes = std::fs::read(&t).unwrap();
    assert!(bytes.len() < 2_000_000, "small: {} bytes", bytes.len());
    let moov = bytes.windows(4).position(|w| w == b"moov").unwrap();
    let mdat = bytes.windows(4).position(|w| w == b"mdat").unwrap();
    assert!(moov < mdat, "faststart");
    let _ = std::fs::remove_dir_all(&dir);
}
