//! A microphone opened ahead of its take. A Bluetooth headset switches to its call profile
//! when its mic opens and delivers nothing for about two seconds (AirPods, 2026-10-15), so
//! a mic opened at the countdown's zero gave every take a silent head — and, with the Mac's
//! sound riding the voice's blocks (B172), cut that too. Opened when the count starts, it's
//! delivering by the take's zero; held through a pause, Resume starts with sound.
//!
//! [`WarmMic`] is a cheap handle: the recorder keeps one for as long as the device should
//! stay open, and each take gets a clone as its audio source. Starting a take binds its sink
//! (what the device heard before goes nowhere); stopping it unbinds, the device stays open.
//! The last handle dropped closes the device.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::audio::{AudioSink, AudioSource};

struct Device {
    source: Mutex<Box<dyn AudioSource>>,
    rate: u32,
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.source.get_mut().unwrap().stop();
    }
}

#[derive(Clone)]
pub struct WarmMic {
    device: Arc<Device>,
    /// The sink of the take running now, shared by every handle.
    take: Arc<Mutex<Option<AudioSink>>>,
}

impl WarmMic {
    /// Opens `source` now, listening into nothing until a take starts.
    pub fn warm(mut source: Box<dyn AudioSource>) -> Result<WarmMic, String> {
        let take: Arc<Mutex<Option<AudioSink>>> = Arc::default();
        let bound = take.clone();
        let relay = AudioSink::with_capacity(0).routed(Arc::new(move |samples: &[f32]| {
            if let Some(sink) = bound.lock().unwrap().as_ref() {
                sink.push(samples);
            }
        }));
        source.start(relay)?;
        let rate = source.sample_rate();
        Ok(WarmMic { device: Arc::new(Device { source: Mutex::new(source), rate }), take })
    }
}

impl AudioSource for WarmMic {
    fn sample_rate(&self) -> u32 {
        self.device.rate
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        *self.take.lock().unwrap() = Some(sink);
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        *self.take.lock().unwrap() = None;
        Ok(())
    }
    fn latency(&self) -> Duration {
        self.device.source.lock().unwrap().latency()
    }
}
