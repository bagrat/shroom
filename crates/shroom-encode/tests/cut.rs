//! The permanent copy against real ffmpeg (N63, B195 (f)): a stand-in recording whose every
//! frame carries its counter and whose sound beeps at known moments is cut to two kept
//! ranges; the copy shows only the kept frames, in order, never a cut one, its beeps land
//! where the kept ranges put them (the sound keeps to the picture across the join), and it
//! is faststart. Its own poster and teaser are made from it.
//!
//! The encoder under test is `SHROOM_FFMPEG` or the ffmpeg on PATH, as the crate's other
//! media tests; the copy is cut with VideoToolbox where it has it, else x264. With neither,
//! the test says it skipped. The stand-in is made, and the copy read back, by the PATH
//! ffmpeg (the oracle).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use shroom_encode::cut::{self, Encoder};
use shroom_encode::peaks;
use shroom_encode::take::{poster_of, teaser_of};
use shroom_testpattern::{read_gray, synthetic_bgra};

const W: u32 = 320;
const H: u32 = 180;
const FPS: u32 = 30;
const SECS: u32 = 6;

fn oracle() -> PathBuf {
    let out = Command::new("sh").args(["-c", "command -v ffmpeg"]).output().expect("sh runs");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!p.is_empty(), "ffmpeg isn't on PATH — this test needs it (brew install ffmpeg)");
    PathBuf::from(p)
}

/// The encoder the copy is cut with on this machine, if any.
fn encoder(ffmpeg: &Path) -> Option<Encoder> {
    if cut::has_encoder(ffmpeg, "h264_videotoolbox") {
        Some(Encoder::VideoToolbox)
    } else if cut::has_encoder(ffmpeg, "libx264") {
        Some(Encoder::Libx264)
    } else {
        None
    }
}

/// 6 s at 30 fps, frame n carrying counter n, a keyframe each second; beeps at 1.0–1.2 s and
/// 3.5–3.7 s, silence otherwise. Faststart, as a take's `preview.mp4` is.
fn stand_in(dir: &Path) -> PathBuf {
    let ff = oracle();
    let enc = encoder(&ff).expect("the PATH ffmpeg encodes H.264");
    let src = dir.join("source.mp4");
    let beeps = format!("aevalsrc=if(between(t\\,1\\,1.2)+between(t\\,3.5\\,3.7)\\,0.8*sin(2*PI*880*t)\\,0):s=48000:d={SECS}");
    let mut child = Command::new(&ff)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "bgra", "-s", &format!("{W}x{H}"), "-framerate", &FPS.to_string(), "-i", "pipe:0"])
        .args(["-f", "lavfi", "-i", &beeps, "-g", &FPS.to_string()])
        .args(enc.video_args(1500))
        .args(["-c:a", "aac", "-b:a", "128k", "-shortest", "-movflags", "+faststart"])
        .arg(&src)
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg starts");
    let mut stdin = child.stdin.take().unwrap();
    for n in 0..FPS * SECS {
        stdin.write_all(&synthetic_bgra(W, H, n)).unwrap();
    }
    drop(stdin);
    assert!(child.wait().unwrap().success(), "the stand-in was made");
    src
}

/// Every frame as it's stored, read back by its counter. Passthrough, never a constant rate:
/// the joined copy's picture starts 21 ms in (behind the first part's AAC priming, which
/// the concat demuxer keeps: the sound stays with the picture), and a constant-rate read
/// would show its first frame twice.
fn counters(file: &Path) -> Vec<Option<u32>> {
    let mut child = Command::new(oracle())
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(file)
        .args(["-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("ffmpeg decodes");
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; (W * H) as usize];
    let mut got = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        got.push(read_gray(&frame, W, H));
    }
    assert!(child.wait().unwrap().success());
    got
}

fn faststart(file: &Path) -> bool {
    let bytes = std::fs::read(file).unwrap();
    let at = |tag: &[u8]| bytes.windows(4).position(|w| w == tag);
    matches!((at(b"moov"), at(b"mdat")), (Some(m), Some(d)) if m < d)
}

#[test]
fn the_copy_holds_only_the_kept_frames_and_its_sound_keeps_to_them() {
    let ffmpeg = shroom_encode::locate::under_test();
    let Some(enc) = encoder(&ffmpeg) else {
        eprintln!("SKIPPED: {} has neither h264_videotoolbox nor libx264 — the copy can't be cut here", ffmpeg.display());
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let src = stand_in(dir.path());
    let out = dir.path().join("edit-3.mp4");

    // Three kept ranges, two joins: 0.4 s, then 0.6 s holding the first beep, then 0.7 s
    // holding the second.
    let keep = [(0.2, 0.6), (0.9, 1.5), (3.3, 4.0)];
    let mut told = Vec::new();
    let secs = cut::render_with(&ffmpeg, &src, &keep, &out, &enc, &mut |f| told.push(f)).expect("the copy is cut");
    // How far it came, as it went: forward only, a part at a time, whole at the end.
    assert!(told.len() >= 3, "a step at least per part: {told:?}");
    assert!(told.windows(2).all(|w| w[0] < w[1]) && told.iter().all(|f| (0.0..=1.0).contains(f)), "{told:?}");
    assert_eq!(told.last(), Some(&1.0));
    assert!((secs - 1.7).abs() < 0.05, "the copy is {secs} s");
    assert!(faststart(&out), "faststart");
    let names: Vec<String> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert!(!names.iter().any(|n| n.ends_with(".parts")), "the parts are cleaned away: {names:?}");

    // The picture: each kept range's frames (6–17, 27–44, 99–119; a frame's slack at each
    // edge), in order, and nothing between them.
    let got = counters(&out);
    assert!(got.iter().all(Option::is_some), "every frame reads: {got:?}");
    let got: Vec<u32> = got.into_iter().flatten().collect();
    let ranges: Vec<(u32, u32)> = keep.iter().map(|(s, e)| ((s * 30.0_f64).round() as u32, (e * 30.0_f64).round() as u32 - 1)).collect();
    let frames: u32 = ranges.iter().map(|(a, b)| b - a + 1).sum();
    assert!((got.len() as i64 - i64::from(frames)).abs() <= 2, "{} frames, not {frames}: {got:?}", got.len());
    assert!(got.windows(2).all(|w| w[0] < w[1]), "in order, none twice: {got:?}");
    assert!(got.iter().all(|c| ranges.iter().any(|(a, b)| (a.saturating_sub(1)..=b + 1).contains(c))), "a cut moment showed: {got:?}");
    for (a, b) in &ranges {
        assert!(got.iter().any(|c| c.abs_diff(*a) <= 1) && got.iter().any(|c| c.abs_diff(*b) <= 1), "{a}–{b} is whole: {got:?}");
    }

    // The sound keeps to the picture across both joins: the first beep (1.0 s of the
    // source) at 0.5–0.7 s of the copy, the second (3.5 s) at 1.2–1.4 s.
    let p = peaks::read(&out).unwrap();
    let loud = |from: f64, to: f64| {
        let (a, b) = ((from * 50.0) as usize, (to * 50.0) as usize);
        p.values[a..b].iter().all(|v| *v >= 150)
    };
    let quiet = |from: f64, to: f64| {
        let (a, b) = ((from * 50.0) as usize, (to * 50.0) as usize);
        p.values[a..b].iter().all(|v| *v <= 8)
    };
    assert!(loud(0.56, 0.66) && loud(1.26, 1.36), "the beeps are where the kept ranges put them: {:?}", p.values);
    assert!(quiet(0.08, 0.44) && quiet(0.82, 1.14) && quiet(1.52, 1.64), "nothing else sounds: {:?}", p.values);

    // Its own poster and teaser.
    let poster = dir.path().join("edit-3-poster.jpg");
    poster_of(&ffmpeg, &out, &poster).expect("poster");
    assert_eq!(&std::fs::read(&poster).unwrap()[..2], &[0xFF, 0xD8], "a JPEG");
    let teaser = dir.path().join("edit-3-teaser.mp4");
    teaser_of(&ffmpeg, &out, &teaser, &enc).expect("teaser");
    assert!(faststart(&teaser));
    assert!(!counters(&teaser).is_empty());
}

#[test]
fn one_kept_range_is_one_part_and_nothing_kept_is_refused() {
    let ffmpeg = shroom_encode::locate::under_test();
    let Some(enc) = encoder(&ffmpeg) else {
        eprintln!("SKIPPED: {} has neither h264_videotoolbox nor libx264 — the copy can't be cut here", ffmpeg.display());
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let src = stand_in(dir.path());
    let out = dir.path().join("it's a copy.mp4");
    let secs = cut::render(&ffmpeg, &src, &[(2.0, 5.0)], &out, &enc).expect("the copy is cut");
    assert!((secs - 3.0).abs() < 0.05, "{secs} s");
    let got: Vec<u32> = counters(&out).into_iter().flatten().collect();
    assert!(got.first().is_some_and(|c| (59..=61).contains(c)) && got.last().is_some_and(|c| (148..=150).contains(c)), "{got:?}");

    let none = dir.path().join("none.mp4");
    let err = cut::render(&ffmpeg, &src, &[(3.0, 3.0)], &none, &enc).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!none.exists());
    // A failed cut leaves no file and no parts behind.
    assert!(cut::render(&ffmpeg, &dir.path().join("missing.mp4"), &[(0.0, 1.0)], &none, &enc).is_err());
    assert!(!none.exists() && !dir.path().join(".none.mp4.parts").exists());
}
