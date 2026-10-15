//! Triple-buffered hand-off of the latest frame from the capture callback to the
//! encoder's writer thread. The capture side never waits on the encoder: it fills a
//! spare buffer outside the lock and swaps it in; the writer swaps the newest out.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

struct Slots {
    latest: Vec<u8>,
    spare: Vec<u8>,
    seq: u64,
    first_at: Option<Instant>,
    /// When the newest frame arrived (the watchdog's view of a continuous source).
    last_at: Option<Instant>,
    /// The source said it stopped for good (ScreenCaptureKit's stop-with-error).
    failed: bool,
}

pub(crate) struct Shared {
    slots: Mutex<Slots>,
    arrived: Condvar,
    frame_bytes: usize,
}

/// The capture side's handle.
#[derive(Clone)]
pub struct FrameSink(Arc<Shared>);

impl FrameSink {
    /// Offers a frame: `fill` writes exactly the frame's bytes (tightly packed BGRA)
    /// into the buffer it's given. Called on the capture thread.
    pub fn offer(&self, fill: impl FnOnce(&mut [u8])) {
        let mut buf = {
            let mut s = self.0.slots.lock().unwrap();
            std::mem::take(&mut s.spare)
        };
        buf.resize(self.0.frame_bytes, 0);
        fill(&mut buf);
        let mut s = self.0.slots.lock().unwrap();
        s.spare = std::mem::replace(&mut s.latest, buf);
        s.seq += 1;
        let now = Instant::now();
        s.first_at.get_or_insert(now);
        s.last_at = Some(now);
        self.0.arrived.notify_all();
    }

    pub fn frame_bytes(&self) -> usize {
        self.0.frame_bytes
    }

    /// The source stopped for good mid-take (the stream died, its device went away): the
    /// recorder's watchdog reads it as capture dying (`Recording::stall`).
    pub fn fail(&self) {
        self.0.slots.lock().unwrap().failed = true;
    }

    pub(crate) fn failed(&self) -> bool {
        self.0.slots.lock().unwrap().failed
    }

    /// When the newest frame arrived, if one has.
    pub(crate) fn last_at(&self) -> Option<Instant> {
        self.0.slots.lock().unwrap().last_at
    }
}

/// The writer side.
pub(crate) struct FrameTap {
    shared: Arc<Shared>,
    current: Vec<u8>,
    seen: u64,
}

impl FrameTap {
    pub(crate) fn new(frame_bytes: usize) -> (FrameSink, FrameTap) {
        let shared = Arc::new(Shared {
            slots: Mutex::new(Slots { latest: Vec::new(), spare: Vec::new(), seq: 0, first_at: None, last_at: None, failed: false }),
            arrived: Condvar::new(),
            frame_bytes,
        });
        (FrameSink(shared.clone()), FrameTap { shared, current: Vec::new(), seen: 0 })
    }

    /// Waits for the first frame; returns when it arrived, or `None` on timeout.
    pub(crate) fn wait_first(&self, timeout: Duration) -> Option<Instant> {
        let s = self.shared.slots.lock().unwrap();
        let (s, _) = self.shared.arrived.wait_timeout_while(s, timeout, |s| s.seq == 0).unwrap();
        s.first_at
    }

    /// The newest frame; `fresh` is false when it repeats the one returned last time.
    pub(crate) fn newest(&mut self) -> (&[u8], bool) {
        let mut s = self.shared.slots.lock().unwrap();
        let fresh = s.seq != self.seen;
        if fresh {
            std::mem::swap(&mut s.latest, &mut self.current);
            self.seen = s.seq;
        }
        drop(s);
        (&self.current, fresh)
    }

    /// The newest frame, waiting up to `grace` for one when nothing new has come since the
    /// last look: a frame a moment late for its tick (capture's delivery jitter under load)
    /// is still that tick's, not a repeat (DECISIONS B58 e).
    pub(crate) fn newest_within(&mut self, grace: Duration) -> (&[u8], bool) {
        {
            let s = self.shared.slots.lock().unwrap();
            if s.seq == self.seen && !grace.is_zero() {
                let _ = self.shared.arrived.wait_timeout_while(s, grace, |s| s.seq == self.seen && !s.failed).unwrap();
            }
        }
        self.newest()
    }

    pub(crate) fn received(&self) -> u64 {
        self.shared.slots.lock().unwrap().seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_writer_sees_the_newest_frame_and_knows_a_repeat() {
        let (sink, mut tap) = FrameTap::new(4);
        sink.offer(|b| b.copy_from_slice(&[1, 1, 1, 1]));
        sink.offer(|b| b.copy_from_slice(&[2, 2, 2, 2]));
        assert_eq!(tap.newest(), (&[2u8, 2, 2, 2][..], true));
        assert_eq!(tap.newest(), (&[2u8, 2, 2, 2][..], false));
        sink.offer(|b| b.copy_from_slice(&[3, 3, 3, 3]));
        assert_eq!(tap.newest(), (&[3u8, 3, 3, 3][..], true));
        assert_eq!(tap.received(), 3);
    }

    #[test]
    fn a_frame_a_moment_late_for_its_tick_is_still_that_ticks() {
        let (sink, mut tap) = FrameTap::new(4);
        sink.offer(|b| b.copy_from_slice(&[1, 1, 1, 1]));
        assert_eq!(tap.newest_within(Duration::ZERO), (&[1u8, 1, 1, 1][..], true));
        let late = {
            let sink = sink.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(3));
                sink.offer(|b| b.copy_from_slice(&[2, 2, 2, 2]));
            })
        };
        assert_eq!(tap.newest_within(Duration::from_millis(500)), (&[2u8, 2, 2, 2][..], true));
        late.join().unwrap();
        // Nothing comes: after the grace, the tick repeats.
        let t = Instant::now();
        assert_eq!(tap.newest_within(Duration::from_millis(20)), (&[2u8, 2, 2, 2][..], false));
        assert!(t.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn waiting_for_the_first_frame_times_out_when_none_comes() {
        let (_sink, tap) = FrameTap::new(4);
        assert!(tap.wait_first(Duration::from_millis(20)).is_none());
    }
}
