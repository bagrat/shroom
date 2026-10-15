//! The drives' microphone: no room at all, only the test pattern's beep (TESTING-PLAN §4 —
//! “a full-frame flash once a second, and a beep on the same tick”), generated here.
//! Unattended runs record and publish this instead of a real microphone, so nothing a
//! drive uploads ever carries the room (whole-spec audit b4 #17).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use shroom_record::audio::{AudioSink, AudioSource};

/// The beep: a 1 kHz tone for the first `BEEP_MS` of every wall-clock second.
pub const BEEP_HZ: f32 = 1_000.0;
pub const BEEP_MS: u64 = 100;
/// How loud (of full scale): clearly above any codec's floor, never clipping.
const LEVEL: f32 = 0.5;

/// One sample of the beep at a wall-clock instant, in seconds since the epoch.
pub fn sample_at(t: f64) -> f32 {
    let into = t.rem_euclid(1.0);
    if into * 1000.0 < BEEP_MS as f64 { LEVEL * (2.0 * std::f64::consts::PI * BEEP_HZ as f64 * t).sin() as f32 } else { 0.0 }
}

/// A microphone that hears only the beep, in real time, at `rate`.
pub struct BeepSource {
    rate: u32,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl BeepSource {
    pub fn new(rate: u32) -> BeepSource {
        BeepSource { rate, stop: Arc::default(), thread: None }
    }
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

impl AudioSource for BeepSource {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        self.stop.store(false, Ordering::SeqCst);
        let stop = self.stop.clone();
        let rate = self.rate as f64;
        self.thread = Some(
            std::thread::Builder::new()
                .name("shroom-drive-beep".into())
                .spawn(move || {
                    // Samples are paced by the wall clock, 10 ms at a time, as a device's
                    // callback delivers them.
                    let mut t = now();
                    let mut buf = Vec::new();
                    while !stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(10));
                        let until = now();
                        let n = ((until - t) * rate).floor() as usize;
                        buf.clear();
                        buf.extend((0..n).map(|i| sample_at(t + i as f64 / rate)));
                        sink.push(&buf);
                        t += n as f64 / rate;
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        Ok(())
    }
}

impl Drop for BeepSource {
    fn drop(&mut self) {
        let _ = AudioSource::stop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beep_sounds_for_the_first_tenth_of_each_second_only() {
        let loud = |t: f64| (0..48).map(|i| sample_at(t + i as f64 / 48_000.0).abs()).fold(0.0f32, f32::max);
        assert!(loud(100.0) > 0.4 && loud(100.05) > 0.4, "on the tick");
        assert_eq!(loud(100.2), 0.0);
        assert_eq!(loud(100.9), 0.0);
        assert!(loud(101.01) > 0.4, "the next second");
    }
}
