//! Audio for a take: a source pushes mono f32 samples into an [`AudioSink`]; a writer
//! thread drains them through the audio clock into a named pipe ffmpeg reads.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use shroom_encode::clock::AudioClock;

/// Anything that delivers mono 32-bit float samples at a fixed rate: the microphone,
/// a test tone.
pub trait AudioSource: Send {
    fn sample_rate(&self) -> u32;
    fn start(&mut self, sink: AudioSink) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}

/// Silence at a rate: what a take records when its microphone is gone (or switched off),
/// so every take of a recording keeps the first take's audio track (one shared
/// `init.mp4`, B11). It pushes nothing; the audio clock pads the take with zeros.
pub struct Silence(pub u32);

impl AudioSource for Silence {
    fn sample_rate(&self) -> u32 {
        self.0
    }
    fn start(&mut self, _sink: AudioSink) -> Result<(), String> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// The capture side's handle. Bounded: if nobody drains it, the oldest samples go.
#[derive(Clone)]
pub struct AudioSink {
    buf: Arc<Mutex<VecDeque<f32>>>,
    cap: usize,
}

impl AudioSink {
    pub fn push(&self, samples: &[f32]) {
        let mut b = self.buf.lock().unwrap();
        b.extend(samples.iter().copied());
        let over = b.len().saturating_sub(self.cap);
        b.drain(..over);
    }
}

/// How a take's audio went — the numbers the media-truth checks read.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AudioStats {
    pub sample_rate: u32,
    pub samples_written: u64,
    /// Silence written because the device fell behind wall time.
    pub samples_padded: u64,
    /// Samples dropped because the device ran ahead.
    pub samples_dropped: u64,
}

pub(crate) struct AudioTap {
    buf: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
}

pub(crate) fn channel(rate: u32) -> (AudioSink, AudioTap) {
    let buf = Arc::new(Mutex::new(VecDeque::with_capacity(rate as usize)));
    (AudioSink { buf: buf.clone(), cap: rate as usize * 10 }, AudioTap { buf, rate })
}

/// Creates the named pipe ffmpeg reads the audio from.
pub(crate) fn make_fifo(path: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(path);
    let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).map_err(std::io::Error::other)?;
    if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

/// Runs on its own thread for one take: waits for the take's start instant (the first
/// video frame), then keeps the pipe exactly `elapsed × rate` samples full until `stop`.
pub(crate) fn write_audio(
    tap: AudioTap,
    fifo: PathBuf,
    start: Arc<Mutex<Option<Instant>>>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<AudioStats> {
    // Opening the write end blocks until ffmpeg opens the read end.
    let mut pipe = std::io::BufWriter::with_capacity(64 * 1024, OpenOptions::new().write(true).open(&fifo)?);
    let t0 = loop {
        if let Some(t) = *start.lock().unwrap() {
            break t;
        }
        if stop.load(Ordering::SeqCst) {
            return Ok(AudioStats { sample_rate: tap.rate, ..Default::default() });
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    // Whatever was captured before the take began isn't part of it — and what was captured
    // since is. ffmpeg opens this pipe only once it has read the first frames (tens of
    // milliseconds after `t0`, more under load); clearing the buffer then put the sound
    // heard at that moment at the take's zero, every beep early by the open's delay (DECISIONS
    // B58). The buffer's newest sample is now, so the newest `elapsed × rate` samples start at
    // `t0`.
    let lead_in = trim_to_take(&mut tap.buf.lock().unwrap(), t0.elapsed(), tap.rate);
    let mut clock = AudioClock::new(tap.rate, Duration::from_millis(80));
    let mut out: Vec<u8> = Vec::with_capacity(tap.rate as usize);
    loop {
        let stopping = stop.load(Ordering::SeqCst);
        let elapsed = t0.elapsed();
        out.clear();
        {
            let mut b = tap.buf.lock().unwrap();
            let plan = if stopping { clock.finish(elapsed, b.len()) } else { clock.plan(elapsed, b.len()) };
            b.drain(..plan.drop);
            out.extend(std::iter::repeat_n(0u8, plan.silence * 4));
            for s in b.drain(..plan.take) {
                out.extend_from_slice(&s.to_le_bytes());
            }
        }
        pipe.write_all(&out)?;
        if stopping {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    pipe.flush()?;
    Ok(AudioStats {
        sample_rate: tap.rate,
        samples_written: clock.written(),
        samples_padded: clock.padded() + lead_in as u64,
        samples_dropped: clock.dropped(),
    })
}

/// Makes the buffer exactly the take's sound so far, `since × rate` samples ending now (its
/// last sample is the latest the device delivered): what was heard before the take began
/// goes; a device that started after the take did (a microphone takes its moment to start)
/// gets the silence before its first sample — else every sound after would land early by
/// that moment, for the whole take. Returns the silence added.
pub(crate) fn trim_to_take(buf: &mut VecDeque<f32>, since: Duration, rate: u32) -> usize {
    let keep = (since.as_nanos() * rate as u128 / 1_000_000_000) as usize;
    let over = buf.len().saturating_sub(keep);
    buf.drain(..over);
    let short = keep - buf.len();
    for _ in 0..short {
        buf.push_front(0.0);
    }
    short
}

/// Opens the read end without blocking, so a writer stuck opening the pipe (because
/// ffmpeg died before reading it) is released and can see that the take stopped.
pub(crate) fn release_fifo(path: &Path) {
    use std::os::unix::fs::OpenOptionsExt;
    let _ = OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_take_keeps_what_was_heard_since_it_began() {
        // 300 ms buffered (the microphone started before the first frame); the writer gets
        // to the pipe 40 ms into the take: the newest 40 ms stay, in order.
        let mut buf: VecDeque<f32> = (0..14_400).map(|i| i as f32).collect();
        assert_eq!(trim_to_take(&mut buf, Duration::from_millis(40), 48_000), 0);
        assert_eq!(buf.len(), 1_920);
        assert_eq!(buf.front().copied(), Some((14_400 - 1_920) as f32));
        // A device that started after the take: silence before its first sample, so that
        // sample stays where it was heard (the newest is now).
        let mut late: VecDeque<f32> = (1..=100).map(|i| i as f32).collect();
        assert_eq!(trim_to_take(&mut late, Duration::from_millis(40), 48_000), 1_820);
        assert_eq!(late.len(), 1_920);
        assert!(late.iter().take(1_820).all(|s| *s == 0.0));
        assert_eq!(late.back().copied(), Some(100.0));
    }
}
