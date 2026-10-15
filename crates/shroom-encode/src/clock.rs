//! The frame clock: how many frames the encoder should have received by now.
//!
//! ScreenCaptureKit delivers a frame only when the screen changes. Writing exactly
//! `elapsed × FPS` frames — repeating the latest one when nothing new arrived, skipping
//! extras when several did — makes the video's duration the wall-clock duration, so a
//! take's video stays in step with its audio (probe: playlist duration exactly 30.000 s).

use std::time::Duration;

use crate::FPS;

#[derive(Debug, Clone, Copy)]
pub struct FrameClock {
    emitted: u64,
}

impl FrameClock {
    pub fn new() -> Self {
        Self { emitted: 0 }
    }

    /// Frames due at `elapsed` since the take started, counting frame 0 as due at once.
    pub fn due(elapsed: Duration) -> u64 {
        (elapsed.as_nanos() * FPS as u128 / 1_000_000_000) as u64 + 1
    }

    /// How many frames to write now to catch up, and records them as written.
    pub fn take_due(&mut self, elapsed: Duration) -> u64 {
        let due = Self::due(elapsed);
        let n = due.saturating_sub(self.emitted);
        self.emitted = self.emitted.max(due);
        n
    }

    pub fn emitted(&self) -> u64 {
        self.emitted
    }

    /// When the next frame becomes due, measured from the take's start.
    pub fn next_deadline(&self) -> Duration {
        Duration::from_nanos(self.emitted * 1_000_000_000 / FPS as u64)
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_frame_is_due_immediately() {
        assert_eq!(FrameClock::new().take_due(Duration::ZERO), 1);
    }

    #[test]
    fn ten_seconds_is_exactly_three_hundred_frames_after_the_first() {
        let mut c = FrameClock::new();
        let mut total = 0;
        for ms in (0..=10_000).step_by(7) {
            total += c.take_due(Duration::from_millis(ms));
        }
        total += c.take_due(Duration::from_secs(10));
        assert_eq!(total, 301);
        assert_eq!(c.emitted(), 301);
    }

    #[test]
    fn a_stall_is_caught_up_with_repeats() {
        let mut c = FrameClock::new();
        c.take_due(Duration::ZERO);
        // Nothing written for half a second: 15 frames are owed at once.
        assert_eq!(c.take_due(Duration::from_millis(500)), 15);
    }

    #[test]
    fn asking_twice_in_the_same_tick_owes_nothing() {
        let mut c = FrameClock::new();
        c.take_due(Duration::from_millis(40));
        assert_eq!(c.take_due(Duration::from_millis(41)), 0);
    }

    #[test]
    fn the_next_deadline_follows_the_emitted_count() {
        let mut c = FrameClock::new();
        c.take_due(Duration::ZERO);
        assert_eq!(c.next_deadline(), Duration::from_nanos(33_333_333));
    }
}

/// The audio clock: keeps a take's audio exactly as long as its wall-clock duration.
///
/// Microphone samples arrive in callbacks on the device's own clock. Writing exactly
/// `elapsed × rate` samples — filling silence when the device has fallen behind by more
/// than `tolerance`, dropping the oldest buffered samples when it has run ahead — means
/// the audio can never drift from the video. (The POC's ffmpeg-avfoundation mic lost 6–10 %
/// of its samples and its audio came out short and clicking — POC-LEARNINGS §3. Here a
/// shortfall becomes measured silence, never a shorter track.)
#[derive(Debug, Clone, Copy)]
pub struct AudioClock {
    rate: u32,
    tolerance: u64,
    written: u64,
    padded: u64,
    dropped: u64,
}

/// What to do with the buffer right now, in order: drop the oldest `drop` samples, write
/// `silence` zeros, then write the next `take` buffered samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioPlan {
    pub drop: usize,
    pub silence: usize,
    pub take: usize,
}

impl AudioClock {
    /// `tolerance` is how far the device may lag or lead before the clock corrects it.
    pub fn new(rate: u32, tolerance: Duration) -> Self {
        let tolerance = (tolerance.as_nanos() * rate as u128 / 1_000_000_000) as u64;
        Self { rate, tolerance, written: 0, padded: 0, dropped: 0 }
    }

    pub fn due(&self, elapsed: Duration) -> u64 {
        (elapsed.as_nanos() * self.rate as u128 / 1_000_000_000) as u64
    }

    /// Plans the next write, given `buffered` samples waiting, and records it as done.
    pub fn plan(&mut self, elapsed: Duration, buffered: usize) -> AudioPlan {
        let want = self.due(elapsed).saturating_sub(self.written);
        let buffered = buffered as u64;
        let mut plan = AudioPlan { drop: 0, silence: 0, take: 0 };
        if buffered >= want {
            let excess = buffered - want;
            if excess > self.tolerance {
                plan.drop = (excess - self.tolerance / 2) as usize;
            }
            plan.take = want as usize;
        } else {
            plan.take = buffered as usize;
            let deficit = want - buffered;
            if deficit > self.tolerance {
                plan.silence = (deficit - self.tolerance / 2) as usize;
            }
        }
        self.written += (plan.take + plan.silence) as u64;
        self.padded += plan.silence as u64;
        self.dropped += plan.drop as u64;
        plan
    }

    /// Everything still owed at the end of a take, from what's buffered then silence.
    pub fn finish(&mut self, elapsed: Duration, buffered: usize) -> AudioPlan {
        let want = self.due(elapsed).saturating_sub(self.written) as usize;
        let take = want.min(buffered);
        let plan = AudioPlan { drop: 0, silence: want - take, take };
        self.written += want as u64;
        self.padded += plan.silence as u64;
        plan
    }

    pub fn written(&self) -> u64 {
        self.written
    }
    pub fn padded(&self) -> u64 {
        self.padded
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn clock() -> AudioClock {
        AudioClock::new(RATE, Duration::from_millis(80))
    }

    #[test]
    fn a_steady_device_is_passed_through_untouched() {
        let mut c = clock();
        let mut buffered = 0usize;
        for ms in (10..=10_000).step_by(10) {
            buffered += 480; // 10 ms of samples per callback
            let p = c.plan(Duration::from_millis(ms), buffered);
            assert_eq!((p.drop, p.silence), (0, 0), "at {ms} ms");
            buffered -= p.take;
        }
        assert_eq!(c.written(), 480_000);
        assert_eq!((c.padded(), c.dropped()), (0, 0));
    }

    #[test]
    fn a_device_that_loses_ten_percent_is_padded_to_wall_time() {
        // The POC bug's signature: 90 % of the samples arrive.
        let mut c = clock();
        let mut buffered = 0usize;
        for ms in (10..=10_000).step_by(10) {
            buffered += 432;
            let p = c.plan(Duration::from_millis(ms), buffered);
            buffered -= p.take;
        }
        let end = c.finish(Duration::from_secs(10), buffered);
        assert_eq!(end.take, buffered);
        assert_eq!(c.written(), 480_000, "the audio is exactly as long as the take");
        assert!(c.padded() > 40_000 && c.padded() < 52_000, "{}", c.padded());
    }

    #[test]
    fn a_device_running_ahead_is_trimmed_not_delayed() {
        let mut c = clock();
        // 300 ms already buffered at the first tick (the mic started before the video).
        let p = c.plan(Duration::from_millis(10), 14_880);
        assert_eq!(p.take, 480);
        assert!(p.drop > 0);
        let latency = 14_880 - p.drop - p.take;
        assert!(latency as u64 <= 3_840, "what stays buffered is within the tolerance: {latency}");
    }

    #[test]
    fn small_jitter_is_left_alone() {
        let mut c = clock();
        // Callbacks arrive late by up to 50 ms: within tolerance, no silence inserted.
        let p = c.plan(Duration::from_millis(50), 0);
        assert_eq!(p, AudioPlan { drop: 0, silence: 0, take: 0 });
        let p = c.plan(Duration::from_millis(60), 2_880);
        assert_eq!(p.silence, 0);
    }
}
