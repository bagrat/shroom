//! The encode leg against real ffmpeg: synthetic frames carrying the counter strip go
//! in as two takes (a pause between them) and come back frame by frame. Every frame
//! written must come back, in order, exactly once.
//!
//! Each take's timestamps restart at zero and the joined playlist marks the pause with
//! `EXT-X-DISCONTINUITY` (POC §2) — players re-base at the tag, ffmpeg's HLS demuxer
//! doesn't, so the frames are read per take and from the joined MP4. (Shifting later
//! takes with `-output_ts_offset` was tried and rejected: ffmpeg writes the offset into
//! `init.mp4`, which then stops being shareable across takes.)
//!
//! Needs ffmpeg on PATH (the decoding oracle, and the encoder unless
//! `SHROOM_FFMPEG` names another — the release's sidecar, B82): without it these tests fail,
//! never pass silently.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use shroom_encode::args::TakeSpec;
use shroom_encode::playlist::{assemble, take_segments};
use shroom_encode::take::{Take, assemble_preview, next_start_number};
use shroom_encode::{files, quality::Quality};
use shroom_testpattern::{read_gray, summarize, synthetic_bgra};

const W: u32 = 1280;
const H: u32 = 720;

/// The encoder under test: `SHROOM_FFMPEG` (the release's sidecar, B82) or the ffmpeg on PATH.
fn ffmpeg() -> PathBuf {
    shroom_encode::locate::under_test()
}

/// The decoder that checks it: the ffmpeg on PATH, never the encoder under test.
fn oracle() -> PathBuf {
    let out = Command::new("sh").args(["-c", "command -v ffmpeg"]).output().expect("sh runs");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!p.is_empty(), "ffmpeg isn't on PATH — this test needs it (brew install ffmpeg)");
    PathBuf::from(p)
}

fn decode_counters(ffmpeg: &PathBuf, input: &std::path::Path) -> Vec<Option<u32>> {
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("ffmpeg decodes");
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; (W * H) as usize];
    let mut counters = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        counters.push(read_gray(&frame, W, H));
    }
    assert!(child.wait().unwrap().success());
    counters
}

fn ffprobe_duration(path: &std::path::Path) -> f64 {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0.0)
}

#[test]
fn two_takes_come_back_frame_for_frame() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let takes = [(0u32, 0u32..90), (1u32, 90u32..150)]; // 3 s, a pause, then 2 s
    let mut first_init: Option<Vec<u8>> = None;
    for (take, counters) in takes.clone() {
        let spec = TakeSpec {
            width: W,
            height: H,
            video_kbps: Quality::Normal.video_kbps(),
            take,
            start_number: next_start_number(dir.path()).unwrap(),
            audio: None,
        };
        let mut t = Take::start(&ffmpeg, dir.path(), spec).expect("ffmpeg starts");
        for c in counters.clone() {
            t.write_frame(&synthetic_bgra(W, H, c)).unwrap();
        }
        let out = t.finish(Duration::from_secs(20)).expect("take finalizes");
        assert_eq!(out.frames_written, counters.len() as u64);

        // Every take reads with its own playlist, every frame exactly once.
        let s = summarize(&decode_counters(&oracle(), &out.playlist));
        assert_eq!(s.frames, counters.len(), "take {take}: {s:?}");
        assert_eq!((s.unreadable, s.repeated, s.skipped, s.backwards), (0, 0, 0, 0), "take {take}: {s:?}");
        assert_eq!((s.first, s.last), (Some(counters.start), Some(counters.end - 1)), "take {take}");

        // The one init segment serves every take — load-bearing for the joined playlist. A
        // later take writes its own (never the shared one, which stays whole whatever
        // happens to that take), byte-identical, and settling drops it.
        let own = std::fs::read(dir.path().join(files::take_init(take))).unwrap();
        match &first_init {
            None => first_init = Some(own),
            Some(first) => {
                assert!(*first == own, "take {take}'s init differs from the shared one");
                assert!(*first == std::fs::read(dir.path().join(files::INIT_SEGMENT)).unwrap(), "the shared init changed");
                assert_eq!(shroom_encode::take::settle_init(dir.path(), take).unwrap(), files::INIT_SEGMENT);
                assert!(!dir.path().join(files::take_init(take)).exists(), "the matching copy is dropped");
                let text = std::fs::read_to_string(dir.path().join(files::take_playlist(take))).unwrap();
                assert!(text.contains("#EXT-X-MAP:URI=\"init.mp4\"") && !text.contains("init_1.mp4"), "its playlist names the shared init: {text}");
            }
        }
    }

    let per_take: Vec<_> = [0, 1]
        .iter()
        .map(|k| take_segments(&std::fs::read_to_string(dir.path().join(files::take_playlist(*k))).unwrap()))
        .collect();
    let master = assemble(&per_take);
    std::fs::write(dir.path().join(files::PLAYLIST), &master.text).unwrap();
    assert!((master.seconds - 5.0).abs() < 0.05, "playlist says {} s", master.seconds);
    assert_eq!(master.text.matches("#EXT-X-DISCONTINUITY").count(), 1);

    // Segment names continue across the pause.
    let indices: Vec<u32> = master.segments.iter().filter_map(|s| files::segment_index(s)).collect();
    assert_eq!(indices, (0..indices.len() as u32).collect::<Vec<_>>());

    let preview = assemble_preview(&ffmpeg, dir.path(), &[0, 1]).unwrap();
    let d = ffprobe_duration(&preview);
    assert!((d - 5.0).abs() < 0.1, "preview is {d} s");
    let preview_counters = summarize(&decode_counters(&oracle(), &preview));
    assert_eq!((preview_counters.frames, preview_counters.unreadable, preview_counters.skipped), (150, 0, 0), "{preview_counters:?}");
}

/// Save to Movies on a take the uploader never assembled (b4 #1, #10, #15): the single-file
/// copy is built from the take's previews; with those gone, from its HLS (the recovery's
/// remux); a cut-short `preview.mp4` is never handed out as it is.
#[test]
fn a_takes_single_file_copy_is_made_whatever_is_left() {
    use shroom_encode::take::{ensure_preview, playable};
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let spec = TakeSpec { width: W, height: H, video_kbps: Quality::Normal.video_kbps(), take: 0, start_number: 0, audio: None };
    let mut t = Take::start(&ffmpeg, dir.path(), spec).expect("ffmpeg starts");
    for c in 0..60 {
        t.write_frame(&synthetic_bgra(W, H, c)).unwrap();
    }
    let out = t.finish(Duration::from_secs(20)).expect("take finalizes");
    let per_take = vec![take_segments(&std::fs::read_to_string(&out.playlist).unwrap())];
    std::fs::write(dir.path().join(files::PLAYLIST), assemble(&per_take).text).unwrap();
    let preview = dir.path().join(files::PREVIEW);
    assert!(!preview.exists());

    // From the take's own preview.
    assert_eq!(ensure_preview(&ffmpeg, dir.path()).unwrap(), preview);
    assert_eq!(summarize(&decode_counters(&oracle(), &preview)).frames, 60);

    // A copy that isn't a movie (a crash mid-assembly) isn't playable: made again.
    std::fs::write(&preview, b"not a movie").unwrap();
    assert!(!playable(&ffmpeg, &preview));
    ensure_preview(&ffmpeg, dir.path()).unwrap();
    assert!(playable(&ffmpeg, &preview));

    // The per-take previews gone too: the recording's HLS, remuxed.
    std::fs::remove_file(&preview).unwrap();
    std::fs::remove_file(dir.path().join(files::take_preview(0))).unwrap();
    ensure_preview(&ffmpeg, dir.path()).expect("remuxed from the HLS");
    let s = summarize(&decode_counters(&oracle(), &preview));
    assert_eq!((s.frames, s.unreadable), (60, 0), "{s:?}");

    // Nothing left to build from: an error, and no half-file left behind.
    for e in std::fs::read_dir(dir.path()).unwrap().flatten() {
        let _ = std::fs::remove_file(e.path());
    }
    assert!(ensure_preview(&ffmpeg, dir.path()).is_err());
    assert!(!preview.exists());
}

/// The picture's end never waits on the sound. ffmpeg holds back an input that runs ahead
/// of the other, and keeps the encoder's last frames until the picture's input ends: a
/// take whose sound is written further than its picture can only finish once the picture
/// is closed first (`Take::close_input`, then the sound's end, then `finish` — the order
/// `shroom-record` stops a take in; the reverse hung a take under load, 2026-10-02).
#[test]
fn the_pictures_end_never_waits_on_the_sound() {
    use std::io::Write;
    use shroom_encode::args::AudioInput;
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let fifo = dir.path().join("mic_0.f32");
    let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let spec = TakeSpec {
        width: 640,
        height: 360,
        video_kbps: 1500,
        take: 0,
        start_number: 0,
        audio: Some(AudioInput { sample_rate: 48_000, fifo: "mic_0.f32".into(), out_rate: None }),
    };
    let mut t = Take::start(&ffmpeg, dir.path(), spec).expect("ffmpeg starts");
    // The sound: 6 s, written as fast as ffmpeg takes it — far past the picture.
    let (tx, rx) = std::sync::mpsc::channel();
    let path = fifo.clone();
    std::thread::spawn(move || {
        let mut pipe = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let ok = pipe.write_all(&vec![0u8; 48_000 * 4 * 6]).is_ok();
        drop(pipe);
        let _ = tx.send(ok);
    });
    // The picture: half a second.
    for n in 0..15 {
        t.write_frame(&synthetic_bgra(640, 360, n)).unwrap();
    }
    t.close_input();
    let sound = rx.recv_timeout(Duration::from_secs(30)).expect("the sound's end went through once the picture had ended");
    assert!(sound);
    let out = t.finish(Duration::from_secs(20)).expect("the take finalizes");
    assert_eq!(out.frames_written, 15);
}
