//! The recording path end to end with synthetic sources and real ffmpeg.
//! Needs ffmpeg on PATH: without it these tests fail, never pass silently.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use shroom_record::{FrameSink, FrameSource, RecordError, Recording};
use shroom_testpattern::{read_gray, summarize, synthetic_bgra_into, Summary};

const W: u32 = 640;
const H: u32 = 360;

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

/// Offers counter frames every `interval`, at most `limit` of them, counting from `start`.
struct Synthetic {
    interval: Duration,
    limit: Option<u32>,
    start: u32,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Synthetic {
    fn new(fps: f64, start: u32) -> Box<Self> {
        Box::new(Synthetic { interval: Duration::from_secs_f64(1.0 / fps), limit: None, start, stop: Arc::default(), thread: None })
    }
    fn frozen_after(n: u32) -> Box<Self> {
        Box::new(Synthetic { interval: Duration::from_millis(30), limit: Some(n), start: 0, stop: Arc::default(), thread: None })
    }
}

impl FrameSource for Synthetic {
    fn size(&self) -> (u32, u32) {
        (W, H)
    }
    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let (interval, limit, start, stop) = (self.interval, self.limit, self.start, self.stop.clone());
        self.thread = Some(std::thread::spawn(move || {
            // Paced by deadline, not by sleeping after the work: under load a
            // sleep-after loop halves the rate this source promises. The frame is drawn
            // straight into the recorder's buffer, a row copy at a time (well under a
            // millisecond in a debug build), so a starved runner still gets the rate.
            let mut c = start;
            let mut due = std::time::Instant::now();
            while !stop.load(Ordering::SeqCst) && limit.is_none_or(|l| c - start < l) {
                sink.offer(|buf| synthetic_bgra_into(buf, W, H, c));
                c += 1;
                due += interval;
                if let Some(wait) = due.checked_duration_since(std::time::Instant::now()) {
                    std::thread::sleep(wait);
                }
            }
        }));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            t.join().map_err(|_| "source thread panicked".to_string())?;
        }
        Ok(())
    }
}

/// A source that starts fine and never delivers (a wedged capture).
struct Silent;
impl FrameSource for Silent {
    fn size(&self) -> (u32, u32) {
        (W, H)
    }
    fn start(&mut self, _sink: FrameSink) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn decode(ffmpeg: &Path, input: &Path) -> Summary {
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; (W * H) as usize];
    let mut counters = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        counters.push(read_gray(&frame, W, H));
    }
    child.wait().unwrap();
    summarize(&counters)
}

/// A take's length against the wall clock (DECISIONS B116, ruled 2026-10-13): never more
/// than 0.15 s short of the time the test slept, never more than 0.15 s past the time it
/// held the take open — from before `start_take` to after `end_take`. On a quiet Mac the two
/// are the same, and so is the bar; on a busy one, starting the encoder and ending the take
/// take time the sleep never counted, and the take is held to what was measured.
fn lasted_as_held(seconds: f64, slept: f64, held: f64) -> bool {
    seconds > slept - 0.15 && seconds < held + 0.15
}

#[test]
fn steady_source_two_takes_match_the_wall_clock() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();

    let began = Instant::now();
    rec.start_take(Synthetic::new(30.0, 0), None).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let t0 = rec.end_take().unwrap();
    let held0 = began.elapsed().as_secs_f64();
    let began = Instant::now();
    rec.start_take(Synthetic::new(30.0, 10_000), None).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    let t1 = rec.end_take().unwrap();
    let held1 = began.elapsed().as_secs_f64();

    for (t, want, held) in [(&t0, 1.5, held0), (&t1, 1.0, held1)] {
        assert!(lasted_as_held(t.seconds, want, held), "take {} lasted {} s, wanted ~{want} (held open {held:.3} s)", t.take, t.seconds);
    }
    let fin = rec.finalize().unwrap();
    assert!((fin.seconds - (t0.seconds + t1.seconds)).abs() < 0.05);

    for t in [&t0, &t1] {
        let s = decode(&oracle(), &dir.path().join(format!("stream_{}.m3u8", t.take)));
        assert_eq!(s.frames as u64, t.frames_written, "take {}: {s:?}", t.take);
        assert_eq!((s.unreadable, s.backwards), (0, 0), "take {}: {s:?}", t.take);
    }
    let p = decode(&oracle(), &rec.preview().unwrap());
    assert_eq!(p.frames as u64, t0.frames_written + t1.frames_written, "{p:?}");
    // The second take's counters start where its source started — the pause is a cut.
    assert_eq!(p.last.map(|c| c >= 10_000), Some(true));
}

#[test]
fn a_frozen_screen_still_makes_a_full_length_video() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    let began = Instant::now();
    rec.start_take(Synthetic::frozen_after(1), None).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    let t = rec.end_take().unwrap();
    let held = began.elapsed().as_secs_f64();
    assert!(lasted_as_held(t.seconds, 1.2, held), "held open {held:.3} s: {t:?}");
    assert_eq!(t.frames_received, 1);
    assert_eq!(t.repeats, t.frames_written - 1);
    let s = decode(&oracle(), &dir.path().join("stream_0.m3u8"));
    assert_eq!((s.first, s.last, s.unreadable), (Some(0), Some(0), 0), "{s:?}");
}

#[test]
fn a_fast_source_is_thinned_to_the_constant_rate() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    let began = Instant::now();
    rec.start_take(Synthetic::new(60.0, 0), None).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    let t = rec.end_take().unwrap();
    let held = began.elapsed().as_secs_f64();
    assert!(lasted_as_held(t.seconds, 1.0, held), "held open {held:.3} s: {t:?}");
    assert!(t.frames_received > t.frames_written, "{t:?}");
    let s = decode(&oracle(), &dir.path().join("stream_0.m3u8"));
    assert_eq!((s.unreadable, s.backwards), (0, 0), "{s:?}");
    assert!(s.skipped > 0, "a 60 fps source must skip counters at 30 fps: {s:?}");
}

#[test]
fn a_source_that_never_delivers_is_no_frames_not_a_hang() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap().with_first_frame_timeout(Duration::from_millis(300));
    rec.start_take(Box::new(Silent), None).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert!(matches!(rec.end_take(), Err(RecordError::NoFrames)));
}
