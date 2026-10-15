//! The Mac's sound mixed with the voice (spec N47, DECISIONS B172): what a take's one sound
//! track carries when the screen's stream brings the Mac's sound along. Runs anywhere (no
//! capture, no encoder).
//!
//! The mix runs `LATENCY` behind and places the Mac's sound by when it was heard; a take's
//! end flushes what it holds (`AudioSink::flush`, which the writer calls). These tests
//! flush to read the mix at once.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_record::mix::{LATENCY, Mixed, SYSTEM_RATE, SystemAudio};
use shroom_record::{AudioSink, AudioSource};

/// A microphone the test drives by hand: it keeps the sink it was started with.
struct HandMic {
    rate: u32,
    sink: Arc<Mutex<Option<AudioSink>>>,
}

impl AudioSource for HandMic {
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        *self.sink.lock().unwrap() = Some(sink);
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        *self.sink.lock().unwrap() = None;
        Ok(())
    }
}

fn hand_mic(rate: u32) -> (Box<dyn AudioSource>, Arc<Mutex<Option<AudioSink>>>) {
    let sink = Arc::new(Mutex::new(None));
    (Box::new(HandMic { rate, sink: sink.clone() }), sink)
}

fn speak(mic: &Arc<Mutex<Option<AudioSink>>>, samples: &[f32]) {
    mic.lock().unwrap().as_ref().expect("the mic was started").push(samples);
}

const MS: usize = SYSTEM_RATE as usize / 1000;

fn ago(ms: u64) -> Instant {
    Instant::now() - Duration::from_millis(ms)
}

/// Every sample in `s[from..to]` is `want`, give or take.
fn all_near(s: &[f32], from: usize, to: usize, want: f32) -> bool {
    s[from..to].iter().all(|x| (x - want).abs() < 1e-3)
}

#[test]
fn the_mix_declares_how_far_behind_it_runs() {
    let mixed = Mixed::new(None, SystemAudio::new());
    assert_eq!(mixed.latency(), LATENCY);
    assert!(LATENCY >= Duration::from_millis(80) && LATENCY <= Duration::from_millis(200));
}

#[test]
fn the_take_hears_the_voice_and_the_mac_together() {
    let system = SystemAudio::new();
    let (mic, hand) = hand_mic(48_000);
    let mut mixed = Mixed::new(Some(mic), system.clone());
    assert_eq!(mixed.sample_rate(), 48_000, "the mic's rate is the track's");
    let out = AudioSink::with_capacity(480_000);
    mixed.start(out.clone()).unwrap();

    // 100 ms of the Mac's sound and 100 ms of voice, heard over the same 100 ms.
    system.feed_heard(&vec![0.25; 100 * MS], ago(100));
    speak(&hand, &vec![0.5; 100 * MS]);
    assert!(out.drain().is_empty(), "nothing leaves before the mix's latency");
    out.flush();
    let heard = out.drain();
    assert_eq!(heard.len(), 100 * MS, "one sample out for every sample the mic gave");
    assert!(all_near(&heard, 5 * MS, 95 * MS, 0.75), "voice + the Mac: {:?}", &heard[50 * MS..50 * MS + 4]);
}

#[test]
fn the_macs_sound_lands_where_it_was_heard() {
    let system = SystemAudio::new();
    let (mic, hand) = hand_mic(48_000);
    let mut mixed = Mixed::new(Some(mic), system.clone());
    let out = AudioSink::with_capacity(480_000);
    mixed.start(out.clone()).unwrap();

    // A 10 ms click heard 60 ms ago, delivered late (as ScreenCaptureKit does); the voice
    // is 100 ms of silence ending now. The click sits 40 ms into the voice's 100 ms.
    system.feed_heard(&vec![0.5; 10 * MS], ago(60));
    speak(&hand, &vec![0.0; 100 * MS]);
    out.flush();
    let heard = out.drain();
    assert!(all_near(&heard, 41 * MS, 49 * MS, 0.5), "the click, in its place");
    assert!(all_near(&heard, 0, 39 * MS, 0.0) && all_near(&heard, 51 * MS, 100 * MS, 0.0), "and only there");
}

#[test]
fn the_voice_alone_comes_out_sample_for_sample() {
    let system = SystemAudio::new();
    let (mic, hand) = hand_mic(48_000);
    let mut mixed = Mixed::new(Some(mic), system.clone());
    let out = AudioSink::with_capacity(480_000);
    mixed.start(out.clone()).unwrap();

    // The Mac plays nothing: the voice is passed through untouched, whatever the blocks.
    let voice: Vec<f32> = (0..48_000).map(|i| ((i % 97) as f32 / 97.0) - 0.5).collect();
    let mut heard = Vec::new();
    for block in voice.chunks(512) {
        speak(&hand, block);
        heard.extend(out.drain());
    }
    out.flush();
    heard.extend(out.drain());
    assert_eq!(heard, voice);
}

#[test]
fn with_no_microphone_the_mac_alone_is_the_track() {
    let system = SystemAudio::new();
    let mut mixed = Mixed::new(None, system.clone());
    assert_eq!(mixed.sample_rate(), SYSTEM_RATE);
    let out = AudioSink::with_capacity(48_000);
    mixed.start(out.clone()).unwrap();

    let tone: Vec<f32> = (0..50 * MS).map(|i| (i as f32 * 0.01).sin() * 0.3).collect();
    system.feed_heard(&tone, ago(50));
    out.flush();
    let heard = out.drain();
    assert!(heard.len() >= tone.len() - 2 * MS, "{} of {}", heard.len(), tone.len());
    assert_eq!(&heard[..40 * MS], &tone[..40 * MS], "the Mac's sound, as it was");
}

#[test]
fn the_mac_is_brought_to_the_microphones_rate() {
    let system = SystemAudio::new();
    let (mic, hand) = hand_mic(24_000);
    let mut mixed = Mixed::new(Some(mic), system.clone());
    assert_eq!(mixed.sample_rate(), 24_000);
    let out = AudioSink::with_capacity(48_000);
    mixed.start(out.clone()).unwrap();

    system.feed_heard(&vec![0.2; 100 * MS], ago(100));
    speak(&hand, &vec![0.1; 2_400]);
    out.flush();
    let heard = out.drain();
    assert_eq!(heard.len(), 2_400);
    assert!(all_near(&heard, 120, 2_280, 0.3), "every sample is both: {:?}", &heard[1_200..1_204]);
}

#[test]
fn loud_together_is_limited_never_wrapped() {
    let system = SystemAudio::new();
    let (mic, hand) = hand_mic(48_000);
    let mut mixed = Mixed::new(Some(mic), system.clone());
    let out = AudioSink::with_capacity(48_000);
    mixed.start(out.clone()).unwrap();

    let at = ago(40);
    system.feed_heard(&vec![0.9; 20 * MS], at);
    system.feed_heard(&vec![-0.9; 20 * MS], at + Duration::from_millis(20));
    speak(&hand, &vec![0.9; 20 * MS]);
    speak(&hand, &vec![-0.9; 20 * MS]);
    out.flush();
    let heard = out.drain();
    assert!(heard.iter().all(|s| s.abs() <= 1.0), "never past full scale");
    assert!(heard[5 * MS] > 0.9 && heard[35 * MS] < -0.9, "loud stays loud: {} {}", heard[5 * MS], heard[35 * MS]);
}

#[test]
fn after_stop_the_mac_reaches_nothing() {
    let system = SystemAudio::new();
    let mut mixed = Mixed::new(None, system.clone());
    let out = AudioSink::with_capacity(48_000);
    mixed.start(out.clone()).unwrap();
    mixed.stop().unwrap();
    system.feed_heard(&[0.5; 480], ago(10));
    assert!(out.drain().is_empty());
}
