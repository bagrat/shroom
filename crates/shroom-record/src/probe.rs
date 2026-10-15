//! A source sampled without recording anything: frames arrive, are measured, and are
//! dropped — nothing is encoded or stored. For checking a source on a real Mac (the
//! camera, whose frames are the room) without keeping a single frame of it.

use std::time::{Duration, Instant};

use crate::handoff::FrameTap;
use crate::source::FrameSource;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Sample {
    pub width: u32,
    pub height: u32,
    /// Frames the source delivered.
    pub delivered: u64,
    pub first_frame_ms: Option<u128>,
    /// Delivered frames per second over the window after the first frame.
    pub fps: f64,
    /// The last frame's mean brightness (0–255): above a few means a picture, not black.
    pub mean_luma: f64,
    /// The brightest pixel and the share of pixels that aren't pure black — a dark room
    /// still has noise and a few lit pixels; a broken conversion is flat zero.
    pub max_luma: u8,
    pub nonzero: f64,
    /// Whether the alpha byte is opaque (BGRA order sanity).
    pub alpha_opaque: bool,
    pub error: Option<String>,
}

pub fn sample(source: &mut dyn FrameSource, window: Duration) -> Sample {
    let (width, height) = source.size();
    let mut out = Sample { width, height, ..Sample::default() };
    let (sink, mut tap) = FrameTap::new(width as usize * height as usize * 4);
    let started = Instant::now();
    if let Err(e) = source.start(sink) {
        out.error = Some(e);
        return out;
    }
    let Some(first) = tap.wait_first(Duration::from_secs(5)) else {
        let _ = source.stop();
        out.error = Some("no frames arrived".into());
        return out;
    };
    out.first_frame_ms = Some(first.duration_since(started).as_millis());
    let at_first = tap.received();
    std::thread::sleep(window);
    out.delivered = tap.received();
    out.fps = (out.delivered - at_first) as f64 / window.as_secs_f64();
    let (frame, _) = tap.newest();
    let px = frame.len() / 4;
    if px > 0 {
        let luma = |p: &[u8]| ((p[0] as u32 * 29 + p[1] as u32 * 150 + p[2] as u32 * 77) >> 8) as u8;
        let sum: u64 = frame.chunks_exact(4).map(|p| luma(p) as u64).sum();
        out.mean_luma = sum as f64 / px as f64;
        out.max_luma = frame.chunks_exact(4).map(luma).max().unwrap_or(0);
        out.nonzero = frame.chunks_exact(4).filter(|p| p[0] | p[1] | p[2] != 0).count() as f64 / px as f64;
        out.alpha_opaque = frame.chunks_exact(4).all(|p| p[3] == 255);
    }
    let _ = source.stop();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameSink;

    /// A source that pushes grey frames from a thread.
    struct Grey {
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }
    impl FrameSource for Grey {
        fn size(&self) -> (u32, u32) {
            (8, 4)
        }
        fn start(&mut self, sink: FrameSink) -> Result<(), String> {
            let stop = self.stop.clone();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                    sink.offer(|b| b.fill(128));
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            Ok(())
        }
        fn stop(&mut self) -> Result<(), String> {
            self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn measures_and_keeps_nothing() {
        let mut g = Grey { stop: Default::default() };
        let s = sample(&mut g, Duration::from_millis(300));
        assert!(s.error.is_none());
        assert!(s.fps > 2.0 && s.delivered > 3, "frames flowed: {s:?}");
        assert!((s.mean_luma - 128.0).abs() < 2.0, "{s:?}");
    }
}
