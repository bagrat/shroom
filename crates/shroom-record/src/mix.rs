//! The Mac's own sound, mixed with the voice into a take's one track (spec N47, DECISIONS
//! B172). The screen's capture feeds a [`SystemAudio`] handle with each buffer and the
//! instant its first sample was heard; the take's audio source is a [`Mixed`].
//!
//! ScreenCaptureKit hands its sound over tens of milliseconds after it was heard (58–71 ms
//! on the hosted Mac, measured), so the mix can't be made as the voice arrives without the
//! Mac's sound trailing it. Instead the mix runs [`LATENCY`] behind: the voice waits in a
//! delay line of exactly that length; each block that leaves it is added to the Mac's sound
//! heard at the same instants (placed by its timestamps), through a soft limiter; the
//! source declares the latency and the take's writer places the track that much earlier.
//! A voice alone comes out sample for sample as it went in. With no microphone, the Mac's
//! sound alone is let out as it falls `LATENCY` behind.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audio::{AudioSink, AudioSource};

/// The rate the screen's capture delivers the Mac's sound at (mono).
pub const SYSTEM_RATE: u32 = 48_000;
/// How far behind the mix runs: past ScreenCaptureKit's delivery delay with room to spare.
pub const LATENCY: Duration = Duration::from_millis(120);
/// The Mac's sound kept beyond what the mix still needs (a stalled voice): dropped past this.
const KEEP: f64 = 1.0;

/// The screen's side of the Mac's sound: cheap to clone, fed from the capture's thread.
#[derive(Clone)]
pub struct SystemAudio {
    target: Arc<Mutex<Option<Arc<Mutex<Mix>>>>>,
    counts: Arc<Counts>,
}

/// What reached the handle: the checks read it to tell "nothing played" from "nothing
/// arrived" from "arrived in a shape we couldn't read".
#[derive(Default)]
struct Counts {
    buffers: AtomicU64,
    samples: AtomicU64,
    unreadable: AtomicU64,
    format: Mutex<String>,
    /// The loudest sample fed, as f32 bits (all samples are finite).
    peak: AtomicU64,
    /// How late the capture delivered each buffer, summed (µs), and how many.
    late_us: AtomicU64,
    late_n: AtomicU64,
    /// Samples of silence put in for the Mac's sound that hadn't arrived in time.
    missed: AtomicU64,
}

/// [`SystemAudio::counts`]: what the capture delivered, and how it went.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct SystemCounts {
    pub buffers: u64,
    pub samples: u64,
    pub unreadable: u64,
    /// The capture's own description of what it delivered, as first seen.
    pub format: String,
    /// The loudest sample fed, of full scale: 0 means it delivered silence.
    pub peak: f32,
    /// How late, on average, a buffer reached shroom after its first sample was heard.
    pub late_ms: f64,
    /// The Mac's sound that came too late for the mix, in ms (silence in its place).
    pub missed_ms: f64,
}

impl Default for SystemAudio {
    fn default() -> Self {
        SystemAudio::new()
    }
}

impl SystemAudio {
    pub fn new() -> SystemAudio {
        SystemAudio { target: Arc::default(), counts: Arc::default() }
    }

    /// A buffer the capture delivered but couldn't read.
    pub fn note_unreadable(&self) {
        self.counts.unreadable.fetch_add(1, Ordering::Relaxed);
    }

    /// Notes the format the capture delivers (kept once).
    pub fn note_format(&self, describe: impl FnOnce() -> String) {
        let mut f = self.counts.format.lock().unwrap();
        if f.is_empty() {
            *f = describe();
        }
    }

    pub fn counts(&self) -> SystemCounts {
        let c = &self.counts;
        SystemCounts {
            buffers: c.buffers.load(Ordering::Relaxed),
            samples: c.samples.load(Ordering::Relaxed),
            unreadable: c.unreadable.load(Ordering::Relaxed),
            format: c.format.lock().unwrap().clone(),
            peak: f32::from_bits(c.peak.load(Ordering::Relaxed) as u32),
            late_ms: c.late_us.load(Ordering::Relaxed) as f64 / 1000.0 / c.late_n.load(Ordering::Relaxed).max(1) as f64,
            missed_ms: c.missed.load(Ordering::Relaxed) as f64 * 1000.0 / SYSTEM_RATE as f64,
        }
    }

    /// Mono samples at [`SYSTEM_RATE`] whose first was heard at `heard`, as the capture
    /// delivers them.
    pub fn feed_heard(&self, samples: &[f32], heard: Instant) {
        let c = &self.counts;
        c.buffers.fetch_add(1, Ordering::Relaxed);
        c.samples.fetch_add(samples.len() as u64, Ordering::Relaxed);
        let loudest = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
        c.peak.fetch_max(loudest.to_bits() as u64, Ordering::Relaxed);
        c.late_us.fetch_add(heard.elapsed().as_micros() as u64, Ordering::Relaxed);
        c.late_n.fetch_add(1, Ordering::Relaxed);
        let mix = self.target.lock().unwrap().clone();
        if let Some(mix) = mix {
            mix.lock().unwrap().feed_system(samples, heard);
        }
    }

    /// Mono samples at [`SYSTEM_RATE`] whose last was heard just now.
    pub fn feed(&self, samples: &[f32]) {
        let span = Duration::from_secs_f64(samples.len() as f64 / SYSTEM_RATE as f64);
        let now = Instant::now();
        self.feed_heard(samples, now.checked_sub(span).unwrap_or(now));
    }

    fn set(&self, mix: Option<Arc<Mutex<Mix>>>) {
        *self.target.lock().unwrap() = mix;
    }
}

/// The mix of one take: the voice's delay line and the Mac's sound by when it was heard.
struct Mix {
    rate: u32,
    base: Instant,
    latency: f64,
    out: AudioSink,
    counts: Arc<Counts>,
    /// With a microphone: its samples, held back `latency`.
    voice: Option<VecDeque<f32>>,
    /// The Mac's sound at `rate`, and when (seconds from `base`) its front sample was heard.
    system: VecDeque<f32>,
    system_at: f64,
    resampler: Resampler,
    /// With no microphone: when the next sample let out was heard.
    next: Option<f64>,
}

impl Mix {
    fn new(rate: u32, out: AudioSink, counts: Arc<Counts>, voice: bool) -> Mix {
        Mix {
            rate,
            base: Instant::now(),
            latency: LATENCY.as_secs_f64(),
            out,
            counts,
            voice: voice.then(VecDeque::new),
            system: VecDeque::new(),
            system_at: 0.0,
            resampler: Resampler::new(SYSTEM_RATE, rate),
            next: None,
        }
    }

    fn secs(&self, at: Instant) -> f64 {
        match at.checked_duration_since(self.base) {
            Some(d) => d.as_secs_f64(),
            None => -self.base.duration_since(at).as_secs_f64(),
        }
    }

    fn now(&self) -> f64 {
        self.base.elapsed().as_secs_f64()
    }

    fn feed_system(&mut self, samples: &[f32], heard: Instant) {
        let (at, now) = (self.secs(heard), self.now());
        self.feed_system_at(samples, at, now);
    }

    /// The Mac's sound whose first sample was heard at `at`, delivered at `now` (seconds
    /// from `base`).
    fn feed_system_at(&mut self, samples: &[f32], at: f64, now: f64) {
        let rate = self.rate as f64;
        if self.system.is_empty() {
            self.system_at = at;
        } else {
            // A gap in what the capture delivered stays a gap; a small overlap is its jitter.
            let end = self.system_at + self.system.len() as f64 / rate;
            let gap = ((at - end) * rate).round();
            if gap > rate * 0.005 {
                self.system.extend(std::iter::repeat_n(0.0, gap as usize));
            }
        }
        self.resampler.run(samples, &mut self.system);
        // Never more than KEEP behind what the mix will want next.
        let wanted = now - self.latency - KEEP;
        let stale = (((wanted - self.system_at) * rate).floor().max(0.0) as usize).min(self.system.len());
        self.system.drain(..stale);
        self.system_at += stale as f64 / rate;
        if self.voice.is_none() {
            self.let_out(now - self.latency);
        }
    }

    /// The Mac's sound heard over `n` samples from `from`: silence where none arrived.
    fn system_over(&mut self, from: f64, n: usize) -> Vec<f32> {
        let rate = self.rate as f64;
        let mut block = vec![0.0f32; n];
        // What was heard before `from` is past.
        let past = (((from - self.system_at) * rate).round().max(0.0) as usize).min(self.system.len());
        self.system.drain(..past);
        self.system_at += past as f64 / rate;
        if self.system.is_empty() {
            self.counts.missed.fetch_add(n as u64, Ordering::Relaxed);
            return block;
        }
        // The Mac's sound from `system_at` on: silence before it.
        let lead = (((self.system_at - from) * rate).round().max(0.0) as usize).min(n);
        let take = (n - lead).min(self.system.len());
        for (b, s) in block[lead..lead + take].iter_mut().zip(self.system.drain(..take)) {
            *b = s;
        }
        self.system_at += take as f64 / rate;
        self.counts.missed.fetch_add((n - take) as u64, Ordering::Relaxed);
        block
    }

    /// The voice's block, as it arrives: held back `latency`, then let out with the Mac's
    /// sound of the same instants added.
    fn push_voice(&mut self, samples: &[f32], flushing: bool) {
        let now = self.now();
        self.push_voice_at(samples, now, flushing);
    }

    /// The voice's block that arrived at `now` (seconds from `base`).
    fn push_voice_at(&mut self, samples: &[f32], now: f64, flushing: bool) {
        let Some(voice) = self.voice.as_mut() else { return };
        voice.extend(samples.iter().copied());
        let rate = self.rate as f64;
        let held = if flushing { 0 } else { (self.latency * rate) as usize };
        let n = voice.len().saturating_sub(held);
        if n == 0 {
            return;
        }
        let mut block: Vec<f32> = voice.drain(..n).collect();
        // The newest sample in the line was heard now; the block ends `held` before it.
        let from = now - (held + n) as f64 / rate;
        let system = self.system_over(from, n);
        for (v, s) in block.iter_mut().zip(system) {
            *v = limit(*v + s);
        }
        self.out.push(&block);
    }

    /// With no microphone: the Mac's sound up to `until`.
    fn let_out(&mut self, until: f64) {
        let rate = self.rate as f64;
        let from = *self.next.get_or_insert(self.system_at);
        let n = ((until - from) * rate).floor();
        if n < 1.0 {
            return;
        }
        let block: Vec<f32> = self.system_over(from, n as usize).into_iter().map(limit).collect();
        self.next = Some(from + n / rate);
        self.out.push(&block);
    }

    /// The take ends: everything held back goes out.
    fn flush(&mut self) {
        if self.voice.is_some() {
            self.push_voice(&[], true);
        } else if self.next.is_some() {
            self.let_out(self.now());
        }
    }
}

/// Soft limiter: untouched up to 0.9 of full scale, then eased towards 1.0.
fn limit(x: f32) -> f32 {
    const KNEE: f32 = 0.9;
    let a = x.abs();
    if a <= KNEE { x } else { x.signum() * (KNEE + (1.0 - KNEE) * ((a - KNEE) / (1.0 - KNEE)).tanh()) }
}

/// Linear interpolation from one rate to another, carried across blocks.
struct Resampler {
    step: f64,
    /// Where the next output sample sits, in input samples, from `last`.
    pos: f64,
    last: Option<f32>,
}

impl Resampler {
    fn new(from: u32, to: u32) -> Resampler {
        Resampler { step: from as f64 / to as f64, pos: 0.0, last: None }
    }

    fn run(&mut self, input: &[f32], out: &mut VecDeque<f32>) {
        if self.step == 1.0 {
            out.extend(input.iter().copied());
            return;
        }
        for &x in input {
            let Some(prev) = self.last else {
                self.last = Some(x);
                out.push_back(x);
                self.pos = self.step - 1.0;
                continue;
            };
            // Output samples between `prev` (0.0) and `x` (1.0).
            while self.pos < 1.0 {
                let t = self.pos.max(0.0) as f32;
                out.push_back(prev + (x - prev) * t);
                self.pos += self.step;
            }
            self.pos -= 1.0;
            self.last = Some(x);
        }
    }
}

/// A take's audio: the microphone (if any) with the Mac's sound mixed in.
pub struct Mixed {
    primary: Option<Box<dyn AudioSource>>,
    system: SystemAudio,
}

impl Mixed {
    pub fn new(primary: Option<Box<dyn AudioSource>>, system: SystemAudio) -> Mixed {
        Mixed { primary, system }
    }
}

impl AudioSource for Mixed {
    fn sample_rate(&self) -> u32 {
        self.primary.as_ref().map(|p| p.sample_rate()).unwrap_or(SYSTEM_RATE)
    }

    fn latency(&self) -> Duration {
        LATENCY
    }

    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        let mix = Arc::new(Mutex::new(Mix::new(self.sample_rate(), sink.clone(), self.system.counts.clone(), self.primary.is_some())));
        let flushing = mix.clone();
        sink.on_flush(move || flushing.lock().unwrap().flush());
        self.system.set(Some(mix.clone()));
        let Some(p) = self.primary.as_mut() else { return Ok(()) };
        let voice = sink.routed(Arc::new(move |samples: &[f32]| mix.lock().unwrap().push_voice(samples, false)));
        let started = p.start(voice);
        if started.is_err() {
            self.system.set(None);
        }
        started
    }

    fn stop(&mut self) -> Result<(), String> {
        self.system.set(None);
        match self.primary.as_mut() {
            Some(p) => p.stop(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A take on a clock the test keeps: the Mac plays `wave(t)` (t in seconds, host clock)
    /// as ScreenCaptureKit delivers it, 10 ms buffers at 48 kHz, each arriving 65 ms after
    /// its first sample was heard, with the Mac's sound card `mac_ppm` fast against the
    /// host; a silent microphone at `rate` hands over `block`-sample blocks, its clock
    /// `mic_ppm` fast, each arriving up to `jitter` seconds after its last sample was heard
    /// (a callback is never early). What the take's track carries, and when each of its
    /// samples was heard (host clock).
    fn take(rate: u32, block: usize, secs: f64, jitter: f64, mic_ppm: f64, mac_ppm: f64, wave: impl Fn(f64) -> f32) -> (Vec<f32>, Vec<f64>) {
        let out = AudioSink::with_capacity(rate as usize * (secs as usize + 2));
        let mut mix = Mix::new(rate, out.clone(), Arc::default(), true);
        let mic_period = block as f64 / rate as f64 / (1.0 + mic_ppm * 1e-6);
        let mac_buf = 480usize;
        let mac_period = mac_buf as f64 / SYSTEM_RATE as f64 / (1.0 + mac_ppm * 1e-6);
        // A fixed pseudo-random jitter, so the test is the same every run.
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut rand = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let (mut j, mut m, mut last_voice) = (0usize, 0usize, 0.0f64);
        loop {
            let voice_due = ((j + 1) as f64 * mic_period + jitter * rand()).max(last_voice);
            let mac_heard = m as f64 * mac_period;
            let mac_due = mac_heard + mac_period + 0.065;
            if voice_due > secs && mac_due > secs {
                break;
            }
            if mac_due <= voice_due {
                let samples: Vec<f32> = (0..mac_buf).map(|i| wave(mac_heard + i as f64 * mac_period / mac_buf as f64)).collect();
                mix.feed_system_at(&samples, mac_heard, mac_due);
                m += 1;
            } else {
                mix.push_voice_at(&vec![0.0; block], voice_due, false);
                last_voice = voice_due;
                j += 1;
            }
        }
        let heard = out.drain();
        let when = (0..heard.len()).map(|k| k as f64 * mic_period / block as f64).collect();
        (heard, when)
    }

    /// Where the track bends by more than `step` from one sample to the next: a splice.
    fn splices(s: &[f32], step: f32) -> Vec<usize> {
        (2..s.len()).filter(|&i| (s[i] - 2.0 * s[i - 1] + s[i - 2]).abs() > step).collect()
    }

    /// How far behind (seconds) the Mac's sound sits in `s[from..]`, against when each
    /// sample was heard: the best fit over ±60 ms in 0.1 ms steps.
    fn lag(s: &[f32], when: &[f64], from: usize, wave: impl Fn(f64) -> f32) -> f64 {
        (-600..=600)
            .map(|d| d as f64 * 1e-4)
            .map(|d| (d, (from..s.len()).map(|k| (s[k] - wave(when[k] - d)).powi(2)).sum::<f32>()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0
    }

    #[test]
    fn a_microphones_late_blocks_never_splice_the_macs_sound() {
        // The report: AirPods' call profile (24 kHz, 20 ms blocks), a callback up to 15 ms
        // late. Every block used to be placed by when it arrived, so the Mac's sound was cut
        // at each block's edge: some of it skipped or silence put in, many times a second.
        let tone = |t: f64| (0.5 * (std::f64::consts::TAU * 440.0 * t).sin()) as f32;
        let (heard, _) = take(24_000, 480, 6.0, 0.015, 0.0, 0.0, tone);
        let settled = 24_000; // the first second: the mix finds its place
        // 440 Hz at 24 kHz bends by at most 0.0066 a sample; a cut, far more.
        let cuts = splices(&heard[settled..], 0.01);
        assert!(cuts.is_empty(), "{} splices in 5 s, the first at {:?}", cuts.len(), &cuts[..cuts.len().min(5)]);
        let slow = |t: f64| (0.5 * (std::f64::consts::TAU * 3.0 * t).sin()) as f32;
        let (heard, when) = take(24_000, 480, 6.0, 0.015, 0.0, 0.0, slow);
        let late = lag(&heard, &when, settled, slow);
        assert!(late.abs() < 0.012, "the Mac's sound sits {:.1} ms off the voice", late * 1e3);
    }

    #[test]
    fn clocks_that_drift_apart_stay_in_step_without_a_splice() {
        // A microphone's clock and the Mac's sound card each run a little off the host's:
        // ±100 ppm is 6 ms a minute. The mix follows by stretching, never by cutting.
        let slow = |t: f64| (0.5 * (std::f64::consts::TAU * 3.0 * t).sin()) as f32;
        for (mic, mac) in [(100.0, -100.0), (-100.0, 100.0)] {
            let (heard, when) = take(48_000, 512, 90.0, 0.004, mic, mac, slow);
            // 3 Hz bends by ~2e-7 a sample; one sample dropped or doubled, ~4e-4.
            let cuts = splices(&heard[48_000..], 5e-5);
            assert!(cuts.is_empty(), "mic {mic} ppm, Mac {mac} ppm: {} splices, the first at {:?}", cuts.len(), &cuts[..cuts.len().min(5)]);
            let late = lag(&heard, &when, heard.len() - 48_000, slow);
            assert!(late.abs() < 0.008, "mic {mic} ppm, Mac {mac} ppm: after 90 s the Mac's sound sits {:.1} ms off", late * 1e3);
        }
    }

    #[test]
    fn resampling_keeps_the_count_and_the_level() {
        let mut r = Resampler::new(48_000, 44_100);
        let mut out = VecDeque::new();
        for _ in 0..100 {
            r.run(&[0.5; 480], &mut out);
        }
        assert!((out.len() as i64 - 44_100).abs() <= 2, "{}", out.len());
        assert!(out.iter().all(|s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn the_limiter_is_continuous_and_bounded() {
        assert_eq!(limit(0.5), 0.5);
        assert_eq!(limit(0.9), 0.9);
        assert!(limit(1.8) <= 1.0 && limit(1.8) > 0.99);
        assert!(limit(0.95) > 0.9 && limit(0.95) < 0.95);
        assert_eq!(limit(-1.8), -limit(1.8));
    }
}
