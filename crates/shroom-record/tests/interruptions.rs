//! What a take survives mid-recording, through the real encoder: a microphone that goes
//! (or changes) between takes keeps the recording's one `init.mp4` intact (B11), and the
//! watchdog's view of a take that stopped making progress — the encoder killed, a source
//! that says it died, a camera gone quiet — and a crash as a later take starts, which must
//! leave every earlier take whole (law 2). Needs ffmpeg on PATH: without it these tests
//! fail, never pass silently.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use shroom_record::{AudioSink, AudioSource, FrameSink, FrameSource, Recording, Stall};
use shroom_testpattern::synthetic_bgra;

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

/// Kills (-9) this test's ffmpeg: a child of this process working in `dir` (other tests run
/// beside it, each in its own folder).
fn kill_encoder_in(dir: &Path) {
    let here = std::fs::canonicalize(dir).unwrap();
    let children = Command::new("pgrep").args(["-P", &std::process::id().to_string(), "ffmpeg"]).output().unwrap();
    let pids: Vec<String> = String::from_utf8_lossy(&children.stdout)
        .split_whitespace()
        .filter(|pid| {
            let cwd = Command::new("lsof").args(["-a", "-d", "cwd", "-Fn", "-p", pid]).output().unwrap();
            String::from_utf8_lossy(&cwd.stdout).lines().any(|l| l.strip_prefix('n').is_some_and(|p| Path::new(p) == here))
        })
        .map(String::from)
        .collect();
    assert_eq!(pids.len(), 1, "the take's ffmpeg");
    for pid in &pids {
        Command::new("kill").args(["-9", pid]).status().unwrap();
    }
}

/// Counter frames at 30 fps until stopped — or, `quiet_after`, only that many, then
/// nothing while it stays "alive" (a camera whose stream froze). `continuous` says whether
/// it's a camera-like source.
struct Frames {
    size: (u32, u32),
    quiet_after: Option<u32>,
    continuous: bool,
    fail_after: Option<u32>,
    /// Its stop answers with an error, as ScreenCaptureKit's does for a stream macOS has
    /// already ended (the lid closed, the display asleep).
    stop_fails: bool,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn frames() -> Box<Frames> {
    Box::new(Frames { size: (W, H), quiet_after: None, continuous: false, fail_after: None, stop_fails: false, stop: Arc::default(), thread: None })
}

impl FrameSource for Frames {
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let (quiet, fail, stop, (w, h)) = (self.quiet_after, self.fail_after, self.stop.clone(), self.size);
        self.thread = Some(std::thread::spawn(move || {
            let mut c = 0u32;
            while !stop.load(Ordering::SeqCst) {
                if fail.is_some_and(|f| c == f) {
                    sink.fail();
                }
                if quiet.is_none_or(|q| c < q) {
                    let frame = synthetic_bgra(w, h, c);
                    sink.offer(|buf| buf.copy_from_slice(&frame));
                }
                c += 1;
                std::thread::sleep(Duration::from_millis(33));
            }
        }));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            t.join().map_err(|_| "source thread panicked".to_string())?;
        }
        if self.stop_fails {
            return Err("the stream is not running".into());
        }
        Ok(())
    }
    fn continuous(&self) -> bool {
        self.continuous
    }
}

/// A 440 Hz tone at `rate`, pushed in real time.
struct Tone {
    rate: u32,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn tone(rate: u32) -> Box<Tone> {
    Box::new(Tone { rate, stop: Arc::default(), thread: None })
}

impl AudioSource for Tone {
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        let (rate, stop) = (self.rate, self.stop.clone());
        self.thread = Some(std::thread::spawn(move || {
            let mut n = 0u64;
            let chunk = rate as u64 / 100;
            while !stop.load(Ordering::SeqCst) {
                let s: Vec<f32> = (n..n + chunk).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.5).collect();
                sink.push(&s);
                n += chunk;
                std::thread::sleep(Duration::from_millis(10));
            }
        }));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().map(|t| t.join());
        Ok(())
    }
}

fn probe(ffmpeg: &Path, input: &Path, entries: &str, stream: &str) -> String {
    let out = Command::new(ffmpeg.with_file_name("ffprobe"))
        .args(["-v", "error", "-select_streams", stream, "-show_entries", entries, "-of", "csv=p=0"])
        .arg(input)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn a_microphone_that_goes_or_changes_never_touches_the_shared_init() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    let init = || std::fs::read(dir.path().join("init.mp4")).unwrap();

    // Take one: a 48 kHz microphone.
    rec.start_take(frames(), Some(tone(48_000))).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let t0 = rec.end_take().unwrap();
    assert_eq!(rec.audio_shape(), Some(Some(48_000)));
    let first = init();
    // Take two: the microphone is gone (mic-lost, Resume continues without your microphone).
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let t1 = rec.end_take().unwrap();
    assert!(t1.audio.as_ref().is_some_and(|a| a.sample_rate == 48_000 && a.samples_padded > 0), "silence at the recording's rate: {:?}", t1.audio);
    assert!(first == init(), "init.mp4 changed when the microphone went");
    // Take three: the fallback microphone runs at 44.1 kHz — resampled to the recording's.
    rec.start_take(frames(), Some(tone(44_100))).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let t2 = rec.end_take().unwrap();
    assert!(first == init(), "init.mp4 changed with a microphone at another rate");

    let fin = rec.finalize().unwrap();
    let joined = fin.playlist.clone();
    assert!(joined.exists(), "{fin:?}");
    let rates = probe(&oracle(), &joined, "stream=sample_rate", "a:0");
    assert!(rates.lines().filter(|l| !l.is_empty()).all(|l| l == "48000") && rates.contains("48000"), "{rates:?}");
    // The whole recording decodes, audio and video, with no errors.
    let out = Command::new(oracle()).args(["-v", "error", "-i"]).arg(&joined).args(["-f", "null", "-"]).output().unwrap();
    assert!(out.status.success() && out.stderr.is_empty(), "decode: {}", String::from_utf8_lossy(&out.stderr));
    let secs: f64 = probe(&oracle(), &joined, "format=duration", "a:0").lines().find(|l| !l.is_empty()).and_then(|l| l.parse().ok()).unwrap_or(0.0);
    let want = t0.seconds + t1.seconds + t2.seconds;
    assert!((secs - want).abs() < 0.6, "the joined recording lasts {secs} s, its takes {want} s");
}

#[test]
fn a_video_only_recording_stays_video_only() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    rec.end_take().unwrap();
    let first = std::fs::read(dir.path().join("init.mp4")).unwrap();
    // A microphone appearing later would change the tracks: the recording keeps its shape.
    rec.start_take(frames(), Some(tone(48_000))).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    let t = rec.end_take().unwrap();
    assert!(t.audio.is_none());
    assert!(first == std::fs::read(dir.path().join("init.mp4")).unwrap());
}

#[test]
fn the_watchdog_sees_a_killed_encoder() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(rec.stall(Duration::from_secs(5)), None, "alive and writing");
    // The encoder dies (kill -9): the next write fails.
    kill_encoder_in(dir.path());
    let until = std::time::Instant::now() + Duration::from_secs(5);
    while rec.stall(Duration::from_secs(5)).is_none() && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(rec.stall(Duration::from_secs(5)), Some(Stall::Encoder));
    let _ = rec.end_take();
}

#[test]
fn the_watchdog_sees_a_source_that_died_or_went_quiet() {
    let ffmpeg = ffmpeg();
    // The source says so (ScreenCaptureKit's stop-with-error).
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Frames { fail_after: Some(10), ..*frames() }), None).unwrap();
    // The source says so at its tenth frame (~0.3 s in; later on a loaded Mac).
    let until = std::time::Instant::now() + Duration::from_secs(5);
    while rec.stall(Duration::from_secs(5)).is_none() && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(rec.stall(Duration::from_secs(5)), Some(Stall::Source));
    rec.end_take().unwrap();

    // A camera goes quiet: its silence is death past the deadline.
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Frames { quiet_after: Some(5), continuous: true, ..*frames() }), None).unwrap();
    // Its five frames land ~0.2 s in on this Mac, later on a slower one (a hosted runner's
    // encoder took most of a second to start), so wait for the silence to outlast the
    // deadline rather than for a fixed time. The watchdog names a stalled encoder first, and
    // an encoder slow to start holds the frame clock past a 500 ms deadline (NoProgress,
    // #66's CI): wait for the camera's verdict, which comes once the encoder takes frames.
    let until = std::time::Instant::now() + Duration::from_secs(5);
    while rec.stall(Duration::from_millis(500)) != Some(Stall::NoFrames) && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(rec.stall(Duration::from_millis(500)), Some(Stall::NoFrames));
    assert_eq!(rec.stall(Duration::from_secs(5)), None, "inside the deadline it's alive");
    rec.end_take().unwrap();

    // A still screen is never death: the frame clock keeps writing.
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Frames { quiet_after: Some(5), ..*frames() }), None).unwrap();
    // Watched until the source's silence (from ~0.2 s) has outlasted the deadline and the
    // take reads alive. On the way, only the encoder's own slow start may show (NoProgress):
    // never the source's death.
    let began = std::time::Instant::now();
    let mut seen = Vec::new();
    loop {
        let s = rec.stall(Duration::from_millis(500));
        if began.elapsed() >= Duration::from_millis(900) && s.is_none() {
            break;
        }
        seen.extend(s);
        assert!(began.elapsed() < Duration::from_secs(5), "never alive: {seen:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen.iter().all(|s| *s == Stall::NoProgress), "a still screen is never the source's death: {seen:?}");
    rec.end_take().unwrap();
}

#[test]
fn a_source_already_gone_at_the_pause_keeps_its_take() {
    // Bagrat's 0.1.10 checkpoint (B92): the lid closed mid-take, macOS ended the screen's
    // stream, and the pause's stop answered with an error — the take was thrown away as if
    // capture had died, and came back only as an interrupted one. Every frame was written:
    // the take ends whole, and the next one (the Resume after wake) starts.
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Frames { stop_fails: true, ..*frames() }), None).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    let t = rec.end_take().expect("a source that can't stop again still ends its take");
    assert!(t.seconds > 0.5, "the take's frames are kept: {t:?}");
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    rec.end_take().unwrap();
    let done = rec.finalize().unwrap();
    assert_eq!(done.takes.len(), 2, "both takes in the recording");
    let (decoded, errors) = decode(&oracle(), &done.playlist);
    assert!(decoded > 30 && errors.is_empty(), "{decoded} frames, errors: {errors}");
}

/// Decodes `input` (a playlist or a movie) whole: frames and errors.
fn decode(ffmpeg: &Path, input: &Path) -> (usize, String) {
    let out = Command::new(ffmpeg)
        .args(["-v", "error", "-allowed_extensions", "ALL", "-i"])
        .arg(input)
        .args(["-map", "0:v", "-f", "framemd5", "-"])
        .output()
        .unwrap();
    let frames = String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.starts_with('#')).count();
    (frames, String::from_utf8_lossy(&out.stderr).into_owned())
}

/// The process dies just as a later take starts — before that take's encoder has written
/// anything (it fills its init segment only with its first segment, seconds in). Whatever
/// the moment, the recording's shared `init.mp4` is never emptied: every earlier take still
/// plays from its own playlist, and the relaunch's repair brings the whole recording back.
#[test]
fn a_crash_as_a_later_take_starts_leaves_the_earlier_takes_whole() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(frames(), Some(tone(48_000))).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let t0 = rec.end_take().unwrap();
    let first = std::fs::read(dir.path().join("init.mp4")).unwrap();
    assert!(!first.is_empty());

    // Take two starts; the Mac loses the app 1.5 s in — inside the window where a later
    // take's encoder has opened its init segment and not yet written it (measured
    // 2026-10-02: from ~0.4 s to its first segment at ~6 s, when every take wrote the shared
    // name). Its encoder is gone mid-start…
    rec.start_take(frames(), Some(tone(48_000))).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    kill_encoder_in(dir.path());
    // …and so is everything else (what a relaunch finds is what's on disk now).
    std::mem::forget(rec);

    let init = std::fs::read(dir.path().join("init.mp4")).unwrap();
    assert!(init == first, "the shared init.mp4 was touched by the take that died ({} bytes, was {})", init.len(), first.len());
    // Take one plays from its own playlist, every frame.
    let (frames0, errors) = decode(&oracle(), &dir.path().join("stream_0.m3u8"));
    assert!(errors.is_empty(), "take one decodes cleanly: {errors}");
    assert!((frames0 as i64 - t0.frames_written as i64).abs() <= 1, "take one: {frames0} frames of {}", t0.frames_written);

    // The relaunch's repair: the whole recording, decodable from its joined playlist.
    let repaired = shroom_record::repair::repair(&ffmpeg, dir.path()).expect("something to save");
    assert!(repaired.seconds >= t0.seconds - 0.1, "{repaired:?}");
    let (frames, errors) = decode(&oracle(), &dir.path().join("stream.m3u8"));
    assert!(errors.is_empty() && frames as u64 >= t0.frames_written, "repaired: {frames} frames, {errors}");
    let leftovers: Vec<String> = std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with("init_")).collect();
    assert!(leftovers.is_empty(), "repair leaves only the shared init: {leftovers:?}");
}

/// A later take whose picture differs (another size: the display changed between takes)
/// can't share the first take's init segment. It keeps its own, the joined playlist names it
/// for that take after the pause, and the whole recording plays — the shared `init.mp4`
/// untouched.
#[test]
fn a_take_with_another_picture_keeps_its_own_init() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    rec.end_take().unwrap();
    let first = std::fs::read(dir.path().join("init.mp4")).unwrap();
    rec.start_take(Box::new(Frames { size: (480, 270), ..*frames() }), None).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    rec.end_take().unwrap();
    // A third take back at the first size shares the first init again.
    rec.start_take(frames(), None).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    rec.end_take().unwrap();
    assert!(first == std::fs::read(dir.path().join("init.mp4")).unwrap(), "the shared init is the first take's");
    assert!(dir.path().join("init_1.mp4").is_file(), "the odd take keeps its own init");
    assert!(!dir.path().join("init_2.mp4").exists(), "a take matching the shared init leaves no copy");

    let fin = rec.finalize().unwrap();
    let text = std::fs::read_to_string(&fin.playlist).unwrap();
    let maps: Vec<&str> = text.lines().filter(|l| l.starts_with("#EXT-X-MAP")).collect();
    assert_eq!(maps, ["#EXT-X-MAP:URI=\"init.mp4\"", "#EXT-X-MAP:URI=\"init_1.mp4\"", "#EXT-X-MAP:URI=\"init.mp4\""], "{text}");
    for take in 0..3 {
        let (n, errors) = decode(&oracle(), &dir.path().join(format!("stream_{take}.m3u8")));
        assert!(errors.is_empty() && n > 30, "take {take}: {n} frames, {errors}");
    }
}


/// Stop while the picture lags the sound: a big picture backs the encoder up, so at Stop
/// its frames are still queued and in flight while the sound is already written to the
/// end. ffmpeg holds back the input that runs ahead, and the encoder keeps its last frames
/// until its input ends — so a take that waited for the sound's last samples before ending
/// the picture never ended (found under build load, 2026-10-02). Ending the picture first,
/// the take ends, whole.
#[test]
fn a_take_whose_picture_lags_its_sound_still_ends() {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Frames { size: (3840, 2160), ..*frames() }), Some(tone(48_000))).unwrap();
    std::thread::sleep(Duration::from_millis(2500));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ended = rec.end_take();
        let _ = tx.send(ended.map(|t| (t.frames_written, t.backlog_peak, t.audio.map(|a| a.samples_written))));
    });
    let ended = rx.recv_timeout(Duration::from_secs(30)).expect("the take ended (it hung waiting for its sound)");
    let (frames, backlog, samples) = ended.expect("the take ended whole");
    eprintln!("frames {frames}, backlog peak {backlog}, samples {samples:?}");
    // Whole: picture and sound both there, as long as each other (a loaded Mac starts the
    // big picture late, so how long isn't the point).
    let (video_s, audio_s) = (frames as f64 / 30.0, samples.unwrap_or(0) as f64 / 48_000.0);
    assert!(frames > 0 && (video_s - audio_s).abs() < 0.1, "picture {video_s:.3} s, sound {audio_s:.3} s");
}
