//! The quiet in a take's sound (spec N65): where a voice cut's edges are set. It is read in
//! Rust, as the editor's waveform is (the bundled encoder carries no filter for it); the
//! stand-in take is the PATH ffmpeg's. Without ffmpeg this test fails, never passes silently.

use std::path::PathBuf;
use std::process::Command;

use shroom_encode::quiet::{STEP, read, silences};

fn tool(name: &str) -> PathBuf {
    let out = Command::new("sh").args(["-c", &format!("command -v {name}")]).output().expect("sh runs");
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!path.is_empty(), "{name} isn't on PATH — this test needs it (brew install ffmpeg)");
    PathBuf::from(path)
}

#[test]
fn the_pauses_in_a_takes_sound_and_the_quiet_it_ends_on() {
    let dir = std::env::temp_dir().join(format!("shroom-quiet-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("preview.mp4");
    // Sound 1 s, quiet 0.5 s, sound 1 s, quiet 0.8 s, sound 1 s, then quiet to the end (0.7 s).
    let parts = [("sine=frequency=330:duration=1", 1.0), ("anullsrc=r=48000:cl=mono", 0.5), ("sine=frequency=440:duration=1", 1.0), ("anullsrc=r=48000:cl=mono", 0.8), ("sine=frequency=550:duration=1", 1.0), ("anullsrc=r=48000:cl=mono", 0.7)];
    let mut cmd = Command::new(tool("ffmpeg"));
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
    for (src, secs) in parts {
        cmd.args(["-f", "lavfi", "-t", &secs.to_string(), "-i", src]);
    }
    let inputs: String = (0..parts.len()).map(|i| format!("[{i}:a]aresample=48000,aformat=channel_layouts=mono[a{i}];")).collect();
    let joined: String = (0..parts.len()).map(|i| format!("[a{i}]")).collect();
    cmd.args(["-filter_complex", &format!("{inputs}{joined}concat=n={}:v=0:a=1[out]", parts.len()), "-map", "[out]", "-c:a", "aac", "-b:a", "96k"]);
    assert!(cmd.arg(&file).status().unwrap().success());

    let got = silences(&file).expect("read");
    let want = [(1.0, 1.5), (2.5, 3.3), (4.3, 5.0)];
    assert_eq!(got.len(), want.len(), "{got:?}");
    for ((s, e), (ws, we)) in got.iter().zip(want) {
        assert!((s - ws).abs() < 0.06 && (e - we).abs() < 0.06, "{got:?}");
    }
    // Its loudness, read in the same pass: every 10 ms, loud in the sound, quiet in the pauses.
    let q = read(&file).expect("read");
    assert_eq!(q.silences, got);
    assert!((q.db.len() as f64 * STEP - 5.0).abs() < 0.1, "{} steps", q.db.len());
    let at = |t: f64| q.db[(t / STEP) as usize];
    // ffmpeg's sine plays at an eighth of full scale: about -21 dBFS RMS.
    assert!(at(0.5) > -25.0 && at(2.0) > -25.0, "the sound: {} {}", at(0.5), at(2.0));
    assert!(at(1.25) < -60.0 && at(2.9) < -60.0, "the pauses: {} {}", at(1.25), at(2.9));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_take_without_sound_has_no_quiet_to_give() {
    let dir = std::env::temp_dir().join(format!("shroom-quiet-mute-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("preview.mp4");
    let made = Command::new(tool("ffmpeg"))
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=64x64:rate=10:duration=1", "-c:v", "mpeg4"])
        .arg(&file)
        .status()
        .unwrap();
    assert!(made.success());
    assert_eq!(silences(&file).expect("read"), []);
    let _ = std::fs::remove_dir_all(&dir);
}
