//! L5's encode proof (DECISIONS B82): inside the clean VM — no Homebrew, no ffmpeg of its own —
//! the installed release's bundled encoder, found the way a release finds it
//! (`shroom_encode::locate`, release road: beside the app's binary and nowhere else), records
//! a synthetic take through the app's own recording code (`shroom_record::Recording`: the
//! take's exact argv, picture and sound, two takes with a pause between) and then makes every
//! file the uploader makes from it (the single-file copy, the poster, the teaser) with the
//! same functions the app calls. The take's HLS must then decode with that encoder.
//!
//! It never opens a window, a link or a capture: the frames are the counter pattern and the
//! sound is a tone. The host then decodes the copied-out take with its own ffmpeg and holds
//! it to the recipe's golden (tests/l5/encode-signature.mjs).
//!
//!   encode-check <path/to/shroom.app> <out-dir>

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use shroom_encode::{files, locate, take};
use shroom_record::{AudioSink, AudioSource, FrameSink, FrameSource, Recording};

const W: u32 = 1280;
const H: u32 = 720;

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
            let mut due = Instant::now();
            while !stop.load(Ordering::SeqCst) {
                let frame = shroom_testpattern::synthetic_bgra(W, H, c);
                sink.offer(|b| b.copy_from_slice(&frame));
                c += 1;
                due += Duration::from_millis(33);
                std::thread::sleep(due.saturating_duration_since(Instant::now()));
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

fn fail(why: impl std::fmt::Display) -> ! {
    eprintln!("encode-check: {why}");
    std::process::exit(1);
}

/// The take's picture decodes end to end with the encoder itself (`-xerror`: any damage fails).
fn decodes(ffmpeg: &Path, dir: &Path, input: &str) -> bool {
    let hls: &[&str] = if input.ends_with(".m3u8") { &["-allowed_extensions", "ALL"] } else { &[] };
    Command::new(ffmpeg)
        .args(["-hide_banner", "-v", "error", "-xerror"])
        .args(hls)
        .args(["-i", input, "-map", "0:v", "-f", "null", "-"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, app, out] = args.as_slice() else { fail("usage: encode-check <path/to/shroom.app> <out-dir>") };
    let (app, out) = (PathBuf::from(app), PathBuf::from(out));
    // A release's road: its own sidecar, no override, no Homebrew.
    let exe_dir = app.join("Contents/MacOS");
    let Some((source, ffmpeg)) = locate::locate(Some(&exe_dir), None, false) else { fail("the app carries no encoder") };
    assert_eq!(source, locate::Source::Bundled);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap_or_else(|e| fail(e));

    let mut rec = Recording::create(&out, &ffmpeg, 3500).unwrap_or_else(|e| fail(format!("create: {e}")));
    // Take one runs past a segment boundary, then a pause, then take two (the golden's shape).
    rec.start_take(Box::<Frames>::default(), Some(Box::<Tone>::default())).unwrap_or_else(|e| fail(format!("take 1: {e:?}")));
    std::thread::sleep(Duration::from_millis(7500));
    rec.end_take().unwrap_or_else(|e| fail(format!("end take 1: {e:?}")));
    rec.start_take(Box::<Frames>::default(), Some(Box::<Tone>::default())).unwrap_or_else(|e| fail(format!("take 2: {e:?}")));
    std::thread::sleep(Duration::from_millis(1500));
    rec.end_take().unwrap_or_else(|e| fail(format!("end take 2: {e:?}")));
    let fin = rec.finalize().unwrap_or_else(|e| fail(format!("finalize: {e:?}")));
    drop(rec);

    // What the uploader makes before a link goes live, with the app's own calls.
    let preview = take::ensure_preview(&ffmpeg, &out).unwrap_or_else(|e| fail(format!("single-file copy: {e}")));
    let poster = take::poster(&ffmpeg, &out).unwrap_or_else(|e| fail(format!("poster: {e}")));
    let teaser = take::teaser(&ffmpeg, &out).unwrap_or_else(|e| fail(format!("teaser: {e}")));
    let hls = decodes(&ffmpeg, &out, files::PLAYLIST);
    let copy = take::playable(&ffmpeg, &preview) && decodes(&ffmpeg, &out, files::PREVIEW);
    let size = |p: &Path| p.metadata().map(|m| m.len()).unwrap_or(0);
    println!(
        "{{\"encoder\":\"{}\",\"takes\":{},\"seconds\":{:.2},\"segments\":{},\"hls_decodes\":{hls},\"copy_decodes\":{copy},\"preview_bytes\":{},\"poster_bytes\":{},\"teaser_bytes\":{}}}",
        source.word(),
        fin.takes.len(),
        fin.seconds,
        fin.segments.len(),
        size(&preview),
        size(&poster),
        size(&teaser)
    );
    if !(hls && copy) {
        fail("the take didn't decode");
    }
}
