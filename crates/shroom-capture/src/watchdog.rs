//! When a camera picture has stopped: the bubble's and Camera's preview's watchdog.
//!
//! The rehearsal of 2026-10-17 froze the bubble on a switch into a full-screen Space: its
//! camera's light stayed on and its picture never changed again, on every Space after.
//! shroom draws the preview's frames itself now, so it knows when the last one landed;
//! this decides from those times alone when to restart the camera. Pure, so it's tested
//! anywhere; `preview.rs` feeds it the clock.

use std::time::Duration;

/// A camera that delivered and then sent nothing for this long has stopped (30 fps is a
/// frame every 33 ms; a camera dimming in a dark room still sends).
pub const STALL: Duration = Duration::from_secs(2);
/// A camera just started or restarted gets this long for its first frame (`startRunning`
/// alone can take a second or more).
pub const FIRST: Duration = Duration::from_secs(5);
/// Restarts that bring nothing back wait longer each time, up to this.
pub const MOST: Duration = Duration::from_secs(30);

/// The watch over one camera picture, in times since the preview opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watchdog {
    /// When the camera was last started (or restarted).
    started: Duration,
    /// The frames seen when it was last checked, and when the count last moved.
    frames: u64,
    moved: Option<Duration>,
    /// How long the camera gets before it's restarted: `FIRST`, doubled by each restart
    /// that brought no frame back.
    wait: Duration,
    restarts: u32,
}

impl Watchdog {
    /// A camera started at `now`.
    pub fn new(now: Duration) -> Watchdog {
        Watchdog { started: now, frames: 0, moved: None, wait: FIRST, restarts: 0 }
    }

    /// Reads the frame count at `now`: true when the camera should be restarted now (and the
    /// watch counts it as restarted).
    pub fn check(&mut self, now: Duration, frames: u64) -> bool {
        if frames != self.frames {
            self.frames = frames;
            self.moved = Some(now);
            // It delivers again: the next stall is met at once.
            self.wait = FIRST;
            return false;
        }
        let stalled = match self.moved {
            // It delivered since its (re)start: a gap of `STALL` is a stop.
            Some(at) if at >= self.started => now.saturating_sub(at) >= STALL,
            // Nothing yet since its (re)start: it gets `wait`.
            _ => now.saturating_sub(self.started) >= self.wait,
        };
        if stalled {
            if self.moved.is_none_or(|at| at < self.started) {
                self.wait = (self.wait * 2).min(MOST);
            }
            self.started = now;
            self.restarts += 1;
        }
        stalled
    }

    /// The restarts so far.
    pub fn restarts(&self) -> u32 {
        self.restarts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// Checks every 250 ms from `from` to `to`, the count moving as `frames` says; the times
    /// it said restart.
    fn run(w: &mut Watchdog, from: u64, to: u64, frames: impl Fn(u64) -> u64) -> Vec<u64> {
        (from..=to).step_by(250).filter(|t| w.check(ms(*t), frames(*t))).collect()
    }

    #[test]
    fn a_camera_that_keeps_delivering_is_left_alone() {
        let mut w = Watchdog::new(ms(0));
        // The first frame at 1.2 s (startRunning is slow), then 30 a second for ten minutes.
        let restarts = run(&mut w, 0, 600_000, |t| if t < 1_200 { 0 } else { (t - 1_200) / 33 + 1 });
        assert!(restarts.is_empty(), "restarted at {restarts:?}");
    }

    #[test]
    fn a_picture_that_stops_is_restarted_two_seconds_on() {
        // The rehearsal: live, then nothing from 168 s on, the camera's light still on.
        let mut w = Watchdog::new(ms(0));
        let frames = |t: u64| if t < 168_000 { t / 33 + 1 } else { 168_000 / 33 + 1 };
        let restarts = run(&mut w, 0, 170_500, frames);
        assert_eq!(restarts, vec![170_000], "two seconds after the last frame");
        assert_eq!(w.restarts(), 1);
    }

    #[test]
    fn a_restart_that_brings_it_back_watches_as_before() {
        let mut w = Watchdog::new(ms(0));
        // Stopped at 10 s, restarted at 12 s, delivering again from 13 s, stopped again at 60 s.
        let frames = |t: u64| match t {
            ..10_000 => t / 33 + 1,
            10_000..13_000 => 10_000 / 33 + 1,
            13_000..60_000 => 10_000 / 33 + 1 + (t - 13_000) / 33 + 1,
            _ => 10_000 / 33 + 1 + (60_000 - 13_000) / 33,
        };
        assert_eq!(run(&mut w, 0, 63_000, frames), vec![12_000, 62_000]);
    }

    #[test]
    fn a_camera_that_never_delivers_is_retried_ever_less_often() {
        let mut w = Watchdog::new(ms(0));
        let restarts = run(&mut w, 0, 200_000, |_| 0);
        // 5 s for the first frame, then 10, 20, and 30 s from then on.
        assert_eq!(restarts, vec![5_000, 15_000, 35_000, 65_000, 95_000, 125_000, 155_000, 185_000]);
    }

    #[test]
    fn a_restart_gets_its_time_for_the_first_frame() {
        let mut w = Watchdog::new(ms(0));
        // Live to 10 s, restarted at 12 s; the restart's first frame lands 1.5 s later.
        let frames = |t: u64| match t {
            ..10_000 => t / 33 + 1,
            10_000..13_500 => 10_000 / 33 + 1,
            _ => 10_000 / 33 + 1 + (t - 13_500) / 33 + 1,
        };
        assert_eq!(run(&mut w, 0, 30_000, frames), vec![12_000], "a slow start isn't a second stall");
    }
}
