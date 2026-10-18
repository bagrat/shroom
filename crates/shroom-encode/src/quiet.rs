//! The quiet in a take's sound (spec N65): the stretches a voice cut's edges are set in.

use std::io;
use std::path::Path;

use crate::sound;

/// Quieter than this is quiet: −38 dBFS, the level the voice-cue planner was tuned on (the
/// POC's `silencedetect=noise=-38dB:d=0.12`).
pub const NOISE: f32 = 0.012_589_254;
/// A quiet stretch is at least this long (seconds).
pub const LEAST: f64 = 0.12;

/// The quiet stretches of `path`'s sound, `(start, end)` in seconds on its own clock, in
/// order; one the take ends on closes at its end. A file with no sound has none. Sound that
/// is missing between two blocks (a packet that wouldn't decode) is never counted as quiet.
pub fn silences(path: &Path) -> io::Result<Vec<(f64, f64)>> {
    let mut runs = Runs::default();
    let Some(rate) = sound::each_block(path, |at, rate, levels| runs.block(at, rate, levels))? else {
        return Ok(Vec::new());
    };
    runs.end(rate);
    Ok(runs.found)
}

#[derive(Default)]
struct Runs {
    /// The frame after the last one read.
    next: u64,
    /// Where the quiet now running began.
    open: Option<u64>,
    found: Vec<(f64, f64)>,
}

impl Runs {
    fn block(&mut self, at: u64, rate: u32, levels: &[f32]) {
        if at != self.next {
            self.close(self.next, rate);
        }
        for (i, &l) in levels.iter().enumerate() {
            let frame = at + i as u64;
            if l < NOISE {
                self.open.get_or_insert(frame);
            } else {
                self.close(frame, rate);
            }
        }
        self.next = at + levels.len() as u64;
    }

    fn close(&mut self, end: u64, rate: u32) {
        if let Some(start) = self.open.take() {
            let (s, e) = (start as f64 / rate as f64, end as f64 / rate as f64);
            if e - s >= LEAST {
                self.found.push((s, e));
            }
        }
    }

    fn end(&mut self, rate: u32) {
        self.close(self.next, rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(blocks: &[(u64, &[f32])]) -> Vec<(f64, f64)> {
        let mut r = Runs::default();
        for (at, levels) in blocks {
            r.block(*at, 10, levels);
        }
        r.end(10);
        r.found
    }

    #[test]
    fn quiet_long_enough_counts_and_a_blip_doesnt() {
        // At 10 frames a second: quiet 0.3 s, loud, quiet 0.1 s (too short), loud, quiet to the end.
        let loud = 0.5;
        let got = runs(&[(0, &[0.0, 0.0, 0.0, loud, 0.001, loud, loud]), (7, &[0.0, 0.0])]);
        assert_eq!(got, [(0.0, 0.3), (0.7, 0.9)]);
    }

    #[test]
    fn the_loudness_is_each_steps_rms_and_missing_sound_reads_loud() {
        // At 200 frames a second, two frames a step: full scale, then half, then nothing,
        // then two frames that never came, then quiet again.
        let mut l = Loudness::default();
        l.block(0, 200, &[1.0, 1.0, 0.5, 0.5, 0.0, 0.0]);
        l.block(8, 200, &[0.0, 0.0]);
        let db = l.db();
        assert_eq!(db.len(), 5);
        assert!(db[0].abs() < 0.01, "{db:?}");
        assert!((db[1] + 6.02).abs() < 0.05, "{db:?}");
        assert!(db[2] < -100.0, "{db:?}");
        assert_eq!(db[3], 0.0, "a step whose sound never came is loud: {db:?}");
        assert!(db[4] < -100.0, "{db:?}");
    }

    #[test]
    fn missing_sound_is_never_quiet() {
        // Frames 3–5 never came: the quiet before them ends where they would have begun.
        let got = runs(&[(0, &[0.0, 0.0, 0.0]), (6, &[0.0, 0.0])]);
        assert_eq!(got, [(0.0, 0.3), (0.6, 0.8)]);
    }
}
