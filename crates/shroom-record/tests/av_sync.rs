//! A/V sync through the real encoder: video frames carry milliseconds-since-epoch in the
//! counter strip; the audio is silence with a 50 ms beep at every whole second of the same
//! epoch. In the output, each beep must land where the video shows that second — within
//! TESTING-PLAN §4's 40 ms, with no drift across the take (DECISIONS B58 for how it's
//! measured). Needs ffmpeg on PATH: without it these tests fail, never pass silently.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use shroom_record::{AudioSink, AudioSource, FrameSink, FrameSource, Recording};
use shroom_testpattern::{read_gray, synthetic_bgra};

const W: u32 = 640;
const H: u32 = 360;
const RATE: u32 = 48_000;

/// TESTING-PLAN §4's A/V bar.
const BAR: f64 = 0.040;

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

struct Clocked<T> {
    epoch: Instant,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    _t: std::marker::PhantomData<T>,
}

struct V;
struct A;

impl FrameSource for Clocked<V> {
    fn size(&self) -> (u32, u32) {
        (W, H)
    }
    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let (epoch, stop) = (self.epoch, self.stop.clone());
        self.thread = Some(std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let frame = synthetic_bgra(W, H, epoch.elapsed().as_millis() as u32);
                sink.offer(|b| b.copy_from_slice(&frame));
                std::thread::sleep(Duration::from_millis(16));
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

impl AudioSource for Clocked<A> {
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        let (epoch, stop) = (self.epoch, self.stop.clone());
        self.thread = Some(std::thread::spawn(move || {
            // Real-time pace, as a microphone delivers: each push carries the 10 ms that
            // just ended (a callback hands over what was heard, never what's still to come).
            let mut next = Instant::now() + Duration::from_millis(10);
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(next.saturating_duration_since(Instant::now()));
                let t0 = next.duration_since(epoch).as_secs_f64() - 0.010;
                let chunk: Vec<f32> = (0..480)
                    .map(|i| {
                        let t = t0 + i as f64 / RATE as f64;
                        if t.fract() < 0.05 { (t * 2.0 * std::f64::consts::PI * 1000.0).sin() as f32 * 0.8 } else { 0.0 }
                    })
                    .collect();
                sink.push(&chunk);
                next += Duration::from_millis(10);
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

fn clocked<T>(epoch: Instant) -> Box<Clocked<T>> {
    Box::new(Clocked { epoch, stop: Arc::default(), thread: None, _t: std::marker::PhantomData })
}

fn video_counters(ffmpeg: &Path, input: &Path) -> Vec<Option<u32>> {
    let mut child = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-map", "0:v", "-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; (W * H) as usize];
    let mut v = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        v.push(read_gray(&frame, W, H));
    }
    child.wait().unwrap();
    v
}

fn audio_samples(ffmpeg: &Path, input: &Path) -> Vec<f32> {
    let out = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-map", "0:a", "-f", "f32le", "-ac", "1", "-ar", &RATE.to_string(), "-"])
        .output()
        .unwrap();
    out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect()
}

/// A take of `secs`, measured: for each beep, how far the sound lands from where the
/// picture shows that second (positive: the sound is late).
struct Measured {
    offsets: Vec<(f64, f64)>,
    video_s: f64,
    audio_s: f64,
    padded: u64,
}

fn record(secs: f64) -> Measured {
    let ffmpeg = ffmpeg();
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 1500).unwrap();
    let epoch = Instant::now();
    rec.start_take(clocked::<V>(epoch), Some(clocked::<A>(epoch))).unwrap();
    std::thread::sleep(Duration::from_secs_f64(secs));
    let t = rec.end_take().unwrap();
    let audio_stats = t.audio.clone().expect("the take had audio");
    eprintln!("{t:?}");

    let take = dir.path().join("preview_0.mp4");
    let counters = video_counters(&oracle(), &take);
    let samples = audio_samples(&oracle(), &take);

    // Beep onsets in output time.
    let mut onsets = Vec::new();
    let mut quiet = 0usize;
    for (i, s) in samples.iter().enumerate() {
        if s.abs() > 0.2 {
            if quiet > RATE as usize / 4 {
                onsets.push(i as f64 / RATE as f64);
            }
            quiet = 0;
        } else {
            quiet += 1;
        }
    }

    // Where the picture shows second S: the frames around it say how far the picture's
    // epoch runs ahead of its own output time (each frame shows the newest frame the source
    // made by its tick — so this includes how stale that frame was). Averaged over the
    // second around S, it is free of the ±1 frame a single sample-and-hold frame reads as.
    let mut offsets = Vec::new();
    for onset in &onsets {
        let near: Vec<f64> = counters
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|ms| (i as f64 / 30.0, ms as f64 / 1000.0)))
            .filter(|(out, _)| (out - onset).abs() <= 0.5)
            .map(|(out, epoch)| epoch - out)
            .collect();
        if near.len() < 20 {
            continue;
        }
        let ahead = near.iter().sum::<f64>() / near.len() as f64;
        // The beep's own second (the picture's epoch at the onset, rounded), and where the
        // picture shows that second: output time `second - ahead`.
        let second = (onset + ahead).round();
        offsets.push((second, onset - (second - ahead)));
    }
    eprintln!("beeps at {onsets:?}, sound-vs-picture offsets {offsets:?}");
    Measured {
        offsets,
        video_s: counters.len() as f64 / 30.0,
        audio_s: samples.len() as f64 / RATE as f64,
        padded: audio_stats.samples_padded,
    }
}

fn assert_in_sync(m: &Measured, beeps: usize) {
    // Audio and video are the same length (both on the wall clock).
    assert!((m.video_s - m.audio_s).abs() <= BAR, "video {:.3} s vs audio {:.3} s", m.video_s, m.audio_s);
    // A steady source needs no padding — except the tail at Stop: the last chunk the
    // source hadn't pushed yet, at most one callback (10 ms here) plus scheduling slack.
    assert!(m.padded < RATE as u64 / 40, "padded {}", m.padded);
    assert!(m.offsets.len() >= beeps, "measured {} beeps of {beeps}", m.offsets.len());
    for (second, o) in &m.offsets {
        assert!(o.abs() <= BAR, "the beep for second {second} landed {o:+.3} s off the picture (bar {BAR} s)");
    }
    // No drift: the offset over the take's last third is the offset over its first (a
    // third's mean, so one beep's jitter isn't read as drift).
    let third = (m.offsets.len() / 3).max(1);
    let mean = |o: &[(f64, f64)]| o.iter().map(|(_, x)| x).sum::<f64>() / o.len() as f64;
    let (first, last) = (mean(&m.offsets[..third]), mean(&m.offsets[m.offsets.len() - third..]));
    assert!((last - first).abs() <= 0.015, "the sound drifted {:+.3} s against the picture across the take", last - first);
}

#[test]
fn beeps_land_on_their_seconds() {
    let m = record(8.3);
    assert_in_sync(&m, 6);
}

/// TESTING-PLAN §4's "no drift across the take", over minutes rather than seconds.
#[test]
#[ignore = "real time, 3 minutes: cargo test -p shroom-record --test av_sync -- --ignored"]
fn no_drift_across_a_long_take() {
    let m = record(180.3);
    assert_in_sync(&m, 175);
}
