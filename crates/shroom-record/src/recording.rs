//! A recording: a directory, a sequence of takes, and the finalize that joins them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Serialize;
use shroom_encode::args::{AudioInput, TakeSpec};
use shroom_encode::clock::FrameClock;
use shroom_encode::playlist::{assemble_mapped, take_segments};
use shroom_encode::take::{Take, TakeError, assemble_preview, next_start_number, settle_init, take_map};
use shroom_encode::{FPS, files};

use crate::audio::{AudioSource, AudioStats, channel, make_fifo, release_fifo, write_audio};
use crate::handoff::{FrameSink, FrameTap};
use crate::source::FrameSource;

#[derive(Debug)]
pub enum RecordError {
    /// The source started but no frame arrived in time (the honest liveness signal).
    NoFrames,
    Source(String),
    Take(TakeError),
    Io(std::io::Error),
    AlreadyRunning,
    NotRunning,
}

impl From<std::io::Error> for RecordError {
    fn from(e: std::io::Error) -> Self {
        RecordError::Io(e)
    }
}

impl From<TakeError> for RecordError {
    fn from(e: TakeError) -> Self {
        RecordError::Take(e)
    }
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::NoFrames => write!(f, "no frames arrived"),
            RecordError::Source(e) => write!(f, "source: {e}"),
            RecordError::Take(e) => write!(f, "encode: {e}"),
            RecordError::Io(e) => write!(f, "io: {e}"),
            RecordError::AlreadyRunning => write!(f, "a take is already running"),
            RecordError::NotRunning => write!(f, "no take is running"),
        }
    }
}

impl std::error::Error for RecordError {}

/// What one take did — the numbers the media-truth checks read.
#[derive(Debug, Clone, Serialize)]
pub struct TakeStats {
    pub take: u32,
    pub width: u32,
    pub height: u32,
    /// Frames the source delivered.
    pub frames_received: u64,
    /// Frames written to the encoder (the frame clock's count).
    pub frames_written: u64,
    /// Written frames that repeated the previous one: nothing new had arrived (a still
    /// screen), or the encoder backlog was at its cap.
    pub repeats: u64,
    /// The most frames that waited for the encoder at once.
    pub backlog_peak: u64,
    /// Ticks that repeated because the backlog was full.
    pub backlog_overflow: u64,
    /// Ticks that repeated because nothing new had arrived since the last one (a still
    /// screen, or a capture late with its frame).
    pub stale_ticks: u64,
    /// Ticks that repeated because the frame clock itself woke late (more than a tick).
    pub late_ticks: u64,
    /// From asking the source to start until its first frame.
    pub first_frame_ms: u64,
    /// The take's length: frames written at the constant rate.
    pub seconds: f64,
    /// The take's audio, when it had any.
    pub audio: Option<AudioStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finalized {
    pub playlist: PathBuf,
    pub seconds: f64,
    pub segments: Vec<String>,
    pub takes: Vec<TakeStats>,
}

struct Running {
    source: Box<dyn FrameSource>,
    audio: Option<Box<dyn AudioSource>>,
    stop: Arc<AtomicBool>,
    writer: JoinHandle<Result<TakeStats, RecordError>>,
    /// The source's side of the hand-off: whether it failed, when it last delivered.
    sink: FrameSink,
    continuous: bool,
    progress: Arc<Progress>,
}

/// The encoder side's liveness: when it last took a frame, and whether it failed.
struct Progress {
    last: std::sync::Mutex<Instant>,
    failed: AtomicBool,
}

impl Progress {
    fn new() -> Arc<Progress> {
        Arc::new(Progress { last: std::sync::Mutex::new(Instant::now()), failed: AtomicBool::new(false) })
    }
    fn tick(&self) {
        *self.last.lock().unwrap() = Instant::now();
    }
}

/// Why a running take counts as dead (the recorder's watchdog, DEVIATIONS #6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stall {
    /// The encoder failed or exited (a write to it failed).
    Encoder,
    /// The encoder took no frame for the whole deadline.
    NoProgress,
    /// The source said it stopped for good.
    Source,
    /// A continuous source (a camera) delivered nothing for the whole deadline.
    NoFrames,
    /// The take's writer ended on its own (no first frame, an error).
    Ended,
}

pub struct Recording {
    dir: PathBuf,
    ffmpeg: PathBuf,
    video_kbps: u32,
    first_frame_timeout: Duration,
    takes: Vec<TakeStats>,
    running: Option<Running>,
    /// The first take's audio: its rate, or `Some(None)` for a video-only recording. Every
    /// later take keeps it — one `init.mp4` for the whole recording (B11): a missing
    /// microphone records silence at that rate, another microphone is resampled to it, and
    /// a video-only recording stays video-only.
    audio_shape: Option<Option<u32>>,
}

impl Recording {
    pub fn create(dir: &Path, ffmpeg: &Path, video_kbps: u32) -> std::io::Result<Recording> {
        std::fs::create_dir_all(dir)?;
        Ok(Recording {
            dir: dir.to_path_buf(),
            ffmpeg: ffmpeg.to_path_buf(),
            video_kbps,
            first_frame_timeout: Duration::from_secs(5),
            takes: Vec::new(),
            running: None,
            audio_shape: None,
        })
    }

    /// How long a take waits for its first frame before it counts as `NoFrames`.
    pub fn with_first_frame_timeout(mut self, timeout: Duration) -> Self {
        self.first_frame_timeout = timeout;
        self
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn takes(&self) -> &[TakeStats] {
        &self.takes
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// The audio every take of this recording carries: `None` before the first take,
    /// `Some(None)` for a video-only recording, else its sample rate.
    pub fn audio_shape(&self) -> Option<Option<u32>> {
        self.audio_shape
    }

    /// The running take, as the watchdog sees it: dead when the encoder failed or took no
    /// frame for `deadline` (the frame clock writes one every tick, a still screen
    /// included), when the source said it stopped, when a continuous source (a camera)
    /// went quiet for `deadline`, or when the writer ended by itself. `None` while it's
    /// alive, or when no take runs.
    pub fn stall(&self, deadline: Duration) -> Option<Stall> {
        let r = self.running.as_ref()?;
        if r.progress.failed.load(Ordering::SeqCst) {
            return Some(Stall::Encoder);
        }
        if r.writer.is_finished() {
            return Some(Stall::Ended);
        }
        if r.sink.failed() {
            return Some(Stall::Source);
        }
        if r.progress.last.lock().unwrap().elapsed() > deadline {
            return Some(Stall::NoProgress);
        }
        if r.continuous && r.sink.last_at().is_some_and(|t| t.elapsed() > deadline) {
            return Some(Stall::NoFrames);
        }
        None
    }

    /// Starts the next take from `source`, with `audio` if any (recording, or resuming
    /// after a pause).
    pub fn start_take(&mut self, mut source: Box<dyn FrameSource>, audio: Option<Box<dyn AudioSource>>) -> Result<(), RecordError> {
        if self.running.is_some() {
            return Err(RecordError::AlreadyRunning);
        }
        let (width, height) = source.size();
        let take_no = self.takes.len() as u32;
        let fifo = self.dir.join(format!("mic_{take_no}.f32"));
        // Every take keeps the first take's audio (one shared init.mp4, B11).
        let shape = self.audio_shape.unwrap_or_else(|| audio.as_ref().map(|a| a.sample_rate()));
        let mut audio = shape.map(|rate| audio.unwrap_or_else(|| Box::new(crate::audio::Silence(rate)) as Box<dyn AudioSource>));
        let audio_input = match &audio {
            Some(a) => {
                make_fifo(&fifo)?;
                Some(AudioInput { sample_rate: a.sample_rate(), fifo: fifo.file_name().unwrap().to_string_lossy().into_owned(), out_rate: shape })
            }
            None => None,
        };
        let spec = TakeSpec {
            width,
            height,
            video_kbps: self.video_kbps,
            take: take_no,
            start_number: next_start_number(&self.dir)?,
            audio: audio_input,
        };
        let take = Take::start(&self.ffmpeg, &self.dir, spec)?;
        let (sink, tap) = FrameTap::new(take.frame_bytes());
        let watch = sink.clone();
        let continuous = source.continuous();
        let progress = Progress::new();
        let stop = Arc::new(AtomicBool::new(false));
        let start_at: Arc<std::sync::Mutex<Option<Instant>>> = Arc::default();

        // Audio first, so the microphone is already listening when the first frame lands.
        let audio_writer = match audio.as_mut() {
            Some(a) => {
                let (asink, atap) = channel(a.sample_rate());
                a.start(asink).map_err(RecordError::Source)?;
                let (fifo, start_at, stop) = (fifo.clone(), start_at.clone(), stop.clone());
                Some(std::thread::Builder::new().name("shroom-audio-writer".into()).spawn(move || write_audio(atap, fifo, start_at, stop))?)
            }
            None => None,
        };

        let asked = Instant::now();
        if let Err(e) = source.start(sink) {
            stop.store(true, Ordering::SeqCst);
            if let Some(mut a) = audio {
                let _ = a.stop();
            }
            return Err(RecordError::Source(e));
        }
        let writer = {
            let stop = stop.clone();
            let timeout = self.first_frame_timeout;
            let progress = progress.clone();
            std::thread::Builder::new()
                .name("shroom-frame-writer".into())
                .spawn(move || write_take(tap, take, stop, asked, timeout, start_at, audio_writer.map(|w| (w, fifo)), progress))?
        };
        self.running = Some(Running { source, audio, stop, writer, sink: watch, continuous, progress });
        self.audio_shape = Some(shape);
        Ok(())
    }

    /// The single-file copy (faststart), from the takes' fragmented previews.
    pub fn preview(&self) -> Result<PathBuf, RecordError> {
        let ids: Vec<u32> = self.takes.iter().map(|t| t.take).collect();
        Ok(assemble_preview(&self.ffmpeg, &self.dir, &ids)?)
    }

    /// Ends the running take — a pause, or the last take before finalize.
    pub fn end_take(&mut self) -> Result<TakeStats, RecordError> {
        let running = self.running.take().ok_or(RecordError::NotRunning)?;
        running.stop.store(true, Ordering::SeqCst);
        let result = running.writer.join().unwrap_or_else(|_| Err(RecordError::Source("frame writer panicked".into())));
        let mut source = running.source;
        let stopped = source.stop();
        if let Some(mut a) = running.audio {
            let _ = a.stop();
        }
        let stats = result?;
        // A source that can't stop again is already stopped: macOS ends the screen's stream
        // by itself when the lid closes or the display sleeps, and ScreenCaptureKit's stop
        // then answers with an error. The writer has every frame it delivered; the take is
        // whole (B92: the lid's pause must land, never the rescue path).
        let _ = stopped;
        // The take's own init segment (a later take never writes the shared one): dropped
        // when it matches, kept for the joined playlist when it doesn't.
        settle_init(&self.dir, stats.take)?;
        self.takes.push(stats.clone());
        Ok(stats)
    }

    /// Joins the takes into the playlist that goes live. Fast at any length: the
    /// single-file copy is assembled later, before its own upload (`preview`).
    pub fn finalize(&mut self) -> Result<Finalized, RecordError> {
        if self.running.is_some() {
            self.end_take()?;
        }
        let mut per_take = Vec::new();
        for t in &self.takes {
            let text = std::fs::read_to_string(self.dir.join(files::take_playlist(t.take)))?;
            per_take.push((take_map(&self.dir, t.take), take_segments(&text)));
        }
        let master = assemble_mapped(&per_take);
        let playlist = self.dir.join(files::PLAYLIST);
        std::fs::write(&playlist, &master.text)?;
        Ok(Finalized { playlist, seconds: master.seconds, segments: master.segments, takes: self.takes.clone() })
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if let Some(r) = self.running.take() {
            r.stop.store(true, Ordering::SeqCst);
            let _ = r.writer.join();
            let mut source = r.source;
            let _ = source.stop();
            if let Some(mut a) = r.audio {
                let _ = a.stop();
            }
        }
    }
}

/// How long a tick waits for a frame when nothing new has come since the last one: a
/// third of a tick at 30 fps. A screen captured at 60 fps delivers two frames a tick, so
/// none for a whole tick is capture's delivery jitter under load as often as a still screen;
/// a frame that arrives within this is that tick's (DECISIONS B58 e).
pub const LATE_FRAME_GRACE: Duration = Duration::from_millis(10);

/// A tick's frame on its way to the encoder: its own copy, or "the previous one again"
/// (a frozen screen, or the queue at its cap).
enum Slot {
    Frame(Vec<u8>),
    Repeat,
}

/// Frames sampled on time, waiting for the encoder. ffmpeg's scheduler throttles the video
/// input whenever video runs ahead of audio (AAC's framing keeps audio slightly behind),
/// so writes block in bursts; sampling never waits on them — it only queues.
struct FrameQueue {
    state: std::sync::Mutex<QueueState>,
    ready: std::sync::Condvar,
}

struct QueueState {
    slots: std::collections::VecDeque<Slot>,
    free: Vec<Vec<u8>>,
    closed: bool,
}

impl FrameQueue {
    fn new() -> Arc<FrameQueue> {
        Arc::new(FrameQueue {
            state: std::sync::Mutex::new(QueueState { slots: Default::default(), free: Vec::new(), closed: false }),
            ready: std::sync::Condvar::new(),
        })
    }
}

/// The encoder side: writes queued frames in order, possibly blocking; returns the take.
/// Each frame the encoder takes is progress; a failed write says the encoder is gone.
fn feed(queue: Arc<FrameQueue>, mut take: Take, progress: Arc<Progress>) -> Result<Take, RecordError> {
    let r = feed_frames(&queue, &mut take, &progress);
    if r.is_err() {
        progress.failed.store(true, Ordering::SeqCst);
    }
    r.map(|()| take)
}

fn feed_frames(queue: &FrameQueue, take: &mut Take, progress: &Progress) -> Result<(), RecordError> {
    let mut last: Vec<u8> = Vec::new();
    loop {
        let slot = {
            let mut st = queue.state.lock().unwrap();
            loop {
                if let Some(s) = st.slots.pop_front() {
                    break Some(s);
                }
                if st.closed {
                    break None;
                }
                st = queue.ready.wait(st).unwrap();
            }
        };
        match slot {
            None => return Ok(()),
            Some(Slot::Frame(buf)) => {
                take.write_frame(&buf)?;
                let old = std::mem::replace(&mut last, buf);
                if !old.is_empty() {
                    queue.state.lock().unwrap().free.push(old);
                }
            }
            Some(Slot::Repeat) => take.write_frame(&last)?,
        }
        progress.tick();
    }
}

#[allow(clippy::too_many_arguments)]
fn write_take(
    mut tap: FrameTap,
    take: Take,
    stop: Arc<AtomicBool>,
    asked: Instant,
    first_frame_timeout: Duration,
    start_at: Arc<std::sync::Mutex<Option<Instant>>>,
    audio_writer: Option<(JoinHandle<std::io::Result<AudioStats>>, PathBuf)>,
    progress: Arc<Progress>,
) -> Result<TakeStats, RecordError> {
    let mut take = take;
    let Some(first) = tap.wait_first(first_frame_timeout) else {
        stop.store(true, Ordering::SeqCst);
        take.kill();
        if let Some((a, fifo)) = audio_writer {
            // ffmpeg is gone; release a writer still waiting to open the pipe.
            release_fifo(&fifo);
            let _ = a.join();
        }
        return Err(RecordError::NoFrames);
    };
    // The take's clock starts at the first frame, for video and audio alike.
    *start_at.lock().unwrap() = Some(first);

    let spec = take.spec().clone();
    let frame_bytes = take.frame_bytes();
    // At most ~256 MiB of frames wait (about a second at 1080p); past that a tick repeats
    // the previous frame rather than grow without bound.
    let cap = (256 * 1024 * 1024 / frame_bytes).max(4);
    let queue = FrameQueue::new();
    progress.tick();
    let feeder = {
        let queue = queue.clone();
        std::thread::Builder::new().name("shroom-frame-feeder".into()).spawn(move || feed(queue, take, progress))?
    };

    let mut clock = FrameClock::new();
    let (mut repeats, mut overflow, mut peak) = (0u64, 0u64, 0usize);
    let (mut stale, mut late) = (0u64, 0u64);
    let mut written = 0u64;
    loop {
        let stopping = stop.load(Ordering::SeqCst);
        let owed = clock.take_due(first.elapsed());
        for i in 0..owed {
            let slot = if i == 0 {
                // A tick on time waits a moment for a frame that's late with it; one already
                // late (owed several) takes what's there.
                let grace = if owed == 1 { LATE_FRAME_GRACE } else { Duration::ZERO };
                let (frame, fresh) = tap.newest_within(grace);
                if !fresh {
                    repeats += 1;
                    stale += 1;
                    Slot::Repeat
                } else {
                    let mut st = queue.state.lock().unwrap();
                    if st.slots.len() >= cap {
                        overflow += 1;
                        repeats += 1;
                        Slot::Repeat
                    } else {
                        let mut buf = st.free.pop().unwrap_or_default();
                        drop(st);
                        buf.clear();
                        buf.extend_from_slice(frame);
                        Slot::Frame(buf)
                    }
                }
            } else {
                // The sampler itself ran late: the extra ticks repeat.
                repeats += 1;
                late += 1;
                Slot::Repeat
            };
            let mut st = queue.state.lock().unwrap();
            st.slots.push_back(slot);
            peak = peak.max(st.slots.len());
            drop(st);
            queue.ready.notify_one();
            written += 1;
        }
        if stopping {
            break;
        }
        let wait = clock.next_deadline().saturating_sub(first.elapsed());
        std::thread::sleep(wait.min(Duration::from_millis(50)));
    }
    queue.state.lock().unwrap().closed = true;
    queue.ready.notify_all();
    let frames_received = tap.received();

    // The video's end first: every queued frame written, then the encoder's input closed —
    // and only then the sound's last samples. ffmpeg holds back an input that runs ahead of
    // the other, and the encoder keeps its last frames until its input ends: waiting for the
    // sound with the picture still open could wait forever (found 2026-10-02 under build
    // load: the sound paused ahead of a picture that would only end after it — DECISIONS B58).
    let fed = feeder.join().map_err(|_| RecordError::Source("frame feeder panicked".into()))?.map(|mut take| {
        take.close_input();
        take
    });
    let audio = match audio_writer {
        Some((a, fifo)) => {
            release_fifo(&fifo);
            Some(a.join().map_err(|_| RecordError::Source("audio writer panicked".into())))
        }
        None => None,
    };
    let take = fed?;
    let audio = match audio {
        Some(joined) => Some(joined??),
        None => None,
    };
    let out = take.finish(Duration::from_secs(10))?;
    debug_assert_eq!(out.frames_written, written);
    Ok(TakeStats {
        take: spec.take,
        width: spec.width,
        height: spec.height,
        frames_received,
        frames_written: out.frames_written,
        repeats,
        backlog_peak: peak as u64,
        backlog_overflow: overflow,
        stale_ticks: stale,
        late_ticks: late,
        first_frame_ms: first.duration_since(asked).as_millis() as u64,
        seconds: out.frames_written as f64 / FPS as f64,
        audio,
    })
}
