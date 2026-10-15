//! A virtual loopback device as a microphone that hears only a signal shroom plays into it
//! (DECISIONS B59 e): the native input path — CoreAudio, `mic::MicSource`, the recorder's
//! sound writer — measured with no room at all. The signal goes to the loopback's *output*
//! (never the speakers, never the system's default devices, which are left as they are),
//! and the recorder opens the loopback's *input* exactly as it opens a microphone.
//!
//! Only BlackHole is used (a virtual device whose output comes back on its input and goes
//! nowhere else). When it is the system's default output or input, other apps' sound
//! would be on it: refused. Whatever does reach the line is checked by the analysis (the
//! signal and nothing else), and the recordings these checks make are local and deleted.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::mic::{MicDevice, Transport};

/// What the loopback carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    /// The drives' beep (`beep::sample_at`): 1 kHz for the first 100 ms of every wall-clock
    /// second — the instant the test pattern's counter crosses a whole second.
    Beep,
    /// A continuous sine at this frequency, phase-continuous sample to sample: any sample
    /// lost or added on the way in shows as a jump in its phase (TESTING-PLAN §3's tone).
    Tone(f64),
}

/// The tone's level, of full scale.
pub const TONE_LEVEL: f32 = 0.5;

/// A loopback being played into; dropping it stops the signal.
pub struct Loopback {
    _stream: Live,
    /// The loopback's input, to open as the take's microphone.
    pub input: MicDevice,
    /// The rate the signal is played at.
    pub rate: u32,
    /// Times the output's next buffer didn't follow on from the last (the player itself
    /// fell behind — a gap that would otherwise read as the input's).
    pub output_gaps: Arc<AtomicU64>,
}

struct Live(#[allow(dead_code)] cpal::Stream);
// cpal's CoreAudio stream runs on CoreAudio's own thread; this handle is only kept alive
// and dropped.
unsafe impl Send for Live {}

fn wall() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// The loopback's input device, when this Mac has one: BlackHole.
pub fn device() -> Option<MicDevice> {
    crate::mic::input_devices().into_iter().find(|d| d.transport == Transport::Virtual && d.name.starts_with("BlackHole"))
}

/// Starts playing `signal` into the loopback. Refuses when there's no loopback, or when it
/// is the system's default output or input (other sound would be on the line).
pub fn play(signal: Signal) -> Result<Loopback, String> {
    let input = device().ok_or("no BlackHole loopback device on this Mac")?;
    let host = cpal::default_host();
    let uid_of = |d: &cpal::Device| d.id().ok().map(|id| id.to_string()).map(|id| id.split_once(':').map(|(_, u)| u.to_string()).unwrap_or(id));
    if host.default_output_device().and_then(|d| uid_of(&d)).is_some_and(|u| u == input.uid) {
        return Err("BlackHole is the system's default output — other apps' sound would be on it".into());
    }
    if crate::mic::default_input_uid().is_some_and(|u| u == input.uid) {
        return Err("BlackHole is the system's default input".into());
    }
    let device = host
        .output_devices()
        .map_err(|e| e.to_string())?
        .find(|d| uid_of(d).is_some_and(|u| u == input.uid))
        .ok_or("BlackHole has no output side")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("unsupported sample format {:?}", supported.sample_format()));
    }
    let config = supported.config();
    let channels = config.channels.max(1) as usize;
    let rate = config.sample_rate;
    let output_gaps = Arc::new(AtomicU64::new(0));
    let gaps = output_gaps.clone();
    // The tone's phase runs on from sample to sample; the beep follows the wall clock.
    let mut n: u64 = 0;
    let mut next: Option<cpal::StreamInstant> = None;
    let stream = device
        .build_output_stream::<f32, _, _>(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                let ts = info.timestamp();
                let frames = data.len() / channels;
                if let Some(expected) = next {
                    let off = if ts.playback > expected { ts.playback.duration_since(expected) } else { expected.duration_since(ts.playback) };
                    if off.as_secs_f64() * rate as f64 > 1.5 {
                        gaps.fetch_add(1, Ordering::Relaxed);
                    }
                }
                next = ts.playback.checked_add(std::time::Duration::from_secs_f64(frames as f64 / rate as f64));
                // The wall-clock instant the first sample of this buffer plays.
                let t0 = wall() + ts.playback.duration_since(ts.callback).as_secs_f64();
                for (i, frame) in data.chunks_exact_mut(channels).enumerate() {
                    let v = match signal {
                        Signal::Beep => crate::beep::sample_at(t0 + i as f64 / rate as f64),
                        Signal::Tone(hz) => {
                            let k = n + i as u64;
                            // The phase in whole cycles, reduced before the sine (f64 keeps it exact
                            // for a 30-minute run).
                            let cycles = (hz * k as f64 / rate as f64).fract();
                            TONE_LEVEL * (2.0 * std::f64::consts::PI * cycles).sin() as f32
                        }
                    };
                    frame.fill(v);
                }
                n += frames as u64;
            },
            |_err| {},
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Loopback { _stream: Live(stream), input, rate, output_gaps })
}
