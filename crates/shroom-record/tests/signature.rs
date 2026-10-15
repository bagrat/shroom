//! TESTING-PLAN §3 (Oracle 2): the capture recipe's shape as a golden. A reference
//! recording — two takes with a pause, picture and sound — through the real encoder; its
//! ffprobe signature (codecs, profile, pixel format, rate, timescale, GOP, sample rate,
//! channels), its playlist's shape and its fMP4 boxes must equal
//! `tests/e2e/golden/hls-signature.json` — the same golden the heartbeat holds the published
//! recording to (`tests/e2e/signature.mjs`). A changed recipe changes the golden on purpose:
//! `SHROOM_BLESS=1` rewrites it, and the diff is reviewed. Needs ffmpeg and ffprobe on PATH:
//! without them this test fails, never passes silently.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};
use shroom_record::{AudioSink, AudioSource, FrameSink, FrameSource, Recording};
use shroom_testpattern::synthetic_bgra;

const W: u32 = 1280;
const H: u32 = 720;

fn tool(name: &str) -> PathBuf {
    let out = Command::new("sh").args(["-c", &format!("command -v {name}")]).output().expect("sh runs");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!p.is_empty(), "{name} isn't on PATH — this test needs it (brew install ffmpeg)");
    PathBuf::from(p)
}

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/e2e/golden/hls-signature.json")
}

#[derive(Default)]
struct Frames {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FrameSource for Frames {
    fn size(&self) -> (u32, u32) {
        (W, H)
    }
    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let stop = self.stop.clone();
        self.thread = Some(std::thread::spawn(move || {
            let mut c = 0u32;
            let mut due = std::time::Instant::now();
            while !stop.load(Ordering::SeqCst) {
                let frame = synthetic_bgra(W, H, c);
                sink.offer(|b| b.copy_from_slice(&frame));
                c += 1;
                due += Duration::from_millis(33);
                std::thread::sleep(due.saturating_duration_since(std::time::Instant::now()));
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

#[derive(Default)]
struct Tone {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioSource for Tone {
    fn sample_rate(&self) -> u32 {
        48_000
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        let stop = self.stop.clone();
        self.thread = Some(std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(Ordering::SeqCst) {
                let s: Vec<f32> = (n..n + 480).map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.5).collect();
                sink.push(&s);
                n += 480;
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

fn boxed<T: Default>() -> Box<T> {
    Box::default()
}

fn probe_json(ffprobe: &Path, args: &[&str], input: &Path) -> Value {
    let out = Command::new(ffprobe).args(["-v", "error", "-allowed_extensions", "ALL"]).args(args).args(["-of", "json"]).arg(input).output().unwrap();
    assert!(out.status.success(), "ffprobe: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn has_box(bytes: &[u8], name: &[u8; 4]) -> bool {
    // Top-level boxes only: size (4) + type (4), walked.
    let mut at = 0usize;
    while at + 8 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        if &bytes[at + 4..at + 8] == name {
            return true;
        }
        if size < 8 {
            return false;
        }
        at += size;
    }
    false
}

/// The signature: what TESTING-PLAN §3 names, nothing that depends on the display or the
/// length (size, level, bitrate, durations beyond the segment rule).
fn signature(ffprobe: &Path, dir: &Path) -> Value {
    let playlist = dir.join("stream.m3u8");
    let streams = probe_json(ffprobe, &["-show_entries", "stream=codec_type,codec_name,profile,pix_fmt,r_frame_rate,time_base,sample_rate,channels"], &playlist);
    let pick = |kind: &str, fields: &[&str]| -> Value {
        let s = streams["streams"].as_array().unwrap().iter().find(|s| s["codec_type"] == kind).cloned().unwrap_or(Value::Null);
        Value::Object(fields.iter().map(|f| (f.to_string(), s[*f].clone())).collect())
    };
    // The GOP: frames from one keyframe to the next, inside the first take.
    let packets = probe_json(ffprobe, &["-select_streams", "v:0", "-show_entries", "packet=flags"], &dir.join("stream_0.m3u8"));
    let keys: Vec<usize> = packets["packets"].as_array().unwrap().iter().enumerate().filter(|(_, p)| p["flags"].as_str().is_some_and(|f| f.starts_with('K'))).map(|(i, _)| i).collect();
    let gop = keys.windows(2).map(|w| w[1] - w[0]).max().unwrap_or(0);

    let text = std::fs::read_to_string(&playlist).unwrap();
    let tag = |t: &str| text.lines().find_map(|l| l.strip_prefix(t)).map(str::to_string);
    // Every segment but the last of each take is whole (6 s).
    let mut whole = true;
    let mut takes: Vec<Vec<f64>> = vec![Vec::new()];
    for l in text.lines() {
        if l == "#EXT-X-DISCONTINUITY" {
            takes.push(Vec::new());
        } else if let Some(d) = l.strip_prefix("#EXTINF:") {
            takes.last_mut().unwrap().push(d.trim_end_matches(',').parse().unwrap());
        }
    }
    for t in &takes {
        for d in t.iter().take(t.len().saturating_sub(1)) {
            whole &= (d - 6.0).abs() < 0.05;
        }
    }
    let init = std::fs::read(dir.join("init.mp4")).unwrap();
    let seg = std::fs::read(dir.join("seg_00000.m4s")).unwrap();
    let boxes = |bytes: &[u8], names: &[&str]| -> Vec<String> {
        names.iter().filter(|b| has_box(bytes, b.as_bytes().try_into().unwrap())).map(|b| b.to_string()).collect()
    };
    let (init_boxes, seg_boxes) = (boxes(&init, &["ftyp", "moov", "moof", "mdat"]), boxes(&seg, &["styp", "moof", "mdat"]));
    let maps = text.matches("#EXT-X-MAP").count();
    let per_pause = text.matches("#EXT-X-DISCONTINUITY").count() / (takes.len() - 1).max(1);
    let ends = text.trim_end().ends_with("#EXT-X-ENDLIST");
    json!({
        "video": pick("video", &["codec_name", "profile", "pix_fmt", "r_frame_rate", "time_base"]),
        "audio": pick("audio", &["codec_name", "sample_rate", "channels", "time_base"]),
        "gop_frames": gop,
        "playlist": {
            "version": tag("#EXT-X-VERSION:"),
            "target_duration": tag("#EXT-X-TARGETDURATION:"),
            "type": tag("#EXT-X-PLAYLIST-TYPE:"),
            "maps": maps,
            "discontinuities_per_pause": per_pause,
            "segments_whole_but_each_takes_last": whole,
            "ends": ends,
        },
        "fmp4": { "init": init_boxes, "segment": seg_boxes },
    })
}

#[test]
fn the_recipe_matches_its_golden_signature() {
    // The encoder under test (`SHROOM_FFMPEG`: the release's sidecar, B82); ffprobe from PATH reads it.
    let (ffmpeg, ffprobe) = (shroom_encode::locate::under_test(), tool("ffprobe"));
    let dir = tempfile::tempdir().unwrap();
    let mut rec = Recording::create(dir.path(), &ffmpeg, 3500).unwrap();
    // Take one runs past a segment boundary (the GOP shows), then a pause, then take two.
    rec.start_take(boxed::<Frames>(), Some(boxed::<Tone>())).unwrap();
    std::thread::sleep(Duration::from_millis(7500));
    rec.end_take().unwrap();
    rec.start_take(boxed::<Frames>(), Some(boxed::<Tone>())).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    rec.end_take().unwrap();
    rec.finalize().unwrap();

    let got = signature(&ffprobe, dir.path());
    let pretty = serde_json::to_string_pretty(&got).unwrap() + "\n";
    if std::env::var("SHROOM_BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(golden().parent().unwrap()).unwrap();
        std::fs::write(golden(), &pretty).unwrap();
    }
    let want: Value = serde_json::from_slice(&std::fs::read(golden()).expect("the golden exists (SHROOM_BLESS=1 writes it)")).unwrap();
    assert_eq!(got, want, "the recipe's signature changed:\n{pretty}");
}
