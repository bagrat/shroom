//! A microphone opened ahead of its take (the AirPods bug, 2026-10-15): a Bluetooth headset
//! switches to its call profile when its mic opens and delivers nothing for about two
//! seconds, so a mic opened at the countdown's zero gave every take a silent head. Opened
//! when the count starts, it is already delivering at the take's zero; held through a
//! pause, Resume starts with sound too. Written before `WarmMic`; runs anywhere (no
//! capture, no encoder).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use shroom_record::mix::{Mixed, SYSTEM_RATE, SystemAudio};
use shroom_record::warm::WarmMic;
use shroom_record::{AudioSink, AudioSource};

const RATE: u32 = 24_000;

/// A headset mic: silent for `wake` after it's opened, then 10 ms of 0.25 every 10 ms.
struct SlowMic {
    wake: Duration,
    stop: Arc<AtomicBool>,
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
}

fn slow_mic(wake: Duration) -> (SlowMic, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let (opened, closed) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    (SlowMic { wake, stop: Arc::default(), opened: opened.clone(), closed: closed.clone(), thread: None }, opened, closed)
}

impl AudioSource for SlowMic {
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        self.opened.fetch_add(1, Ordering::SeqCst);
        self.stop.store(false, Ordering::SeqCst);
        let (wake, stop) = (self.wake, self.stop.clone());
        self.thread = Some(std::thread::spawn(move || {
            let awake = Instant::now() + wake;
            while !stop.load(Ordering::SeqCst) {
                if Instant::now() >= awake {
                    sink.push(&[0.25; RATE as usize / 100]);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.closed.fetch_add(1, Ordering::SeqCst);
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().map(|t| t.join());
        Ok(())
    }
}

/// How long after `start` the take's sink first held a sample, within `limit`.
fn first_sound(source: &mut dyn AudioSource, limit: Duration) -> Option<Duration> {
    let sink = AudioSink::with_capacity(RATE as usize * 10);
    let t0 = Instant::now();
    source.start(sink.clone()).unwrap();
    let heard = loop {
        if !sink.drain().is_empty() {
            break Some(t0.elapsed());
        }
        if t0.elapsed() > limit {
            break None;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    source.stop().unwrap();
    heard
}

const WAKE: Duration = Duration::from_millis(400);

#[test]
fn a_mic_opened_at_the_takes_zero_starts_late() {
    // The bug, as shipped: the take's zero is the moment the mic opens.
    let (mut mic, _, _) = slow_mic(WAKE);
    let heard = first_sound(&mut mic, Duration::from_secs(2)).expect("it wakes");
    assert!(heard >= WAKE, "{heard:?}");
}

#[test]
fn a_mic_warmed_through_the_count_is_heard_at_the_takes_zero() {
    let (mic, opened, _) = slow_mic(WAKE);
    let warm = WarmMic::warm(Box::new(mic)).unwrap();
    // The count: three seconds in the app, longer than the headset's wake here.
    std::thread::sleep(WAKE + Duration::from_millis(150));
    let heard = first_sound(&mut warm.clone(), Duration::from_secs(2)).expect("sound");
    assert!(heard < Duration::from_millis(60), "the take's first sound came {heard:?} after its zero");
    assert_eq!(opened.load(Ordering::SeqCst), 1, "opened once, at the count");
}

#[test]
fn nothing_heard_before_the_take_reaches_it() {
    let (mic, _, _) = slow_mic(Duration::ZERO);
    let warm = WarmMic::warm(Box::new(mic)).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let sink = AudioSink::with_capacity(RATE as usize * 10);
    let mut take = warm.clone();
    take.start(sink.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    take.stop().unwrap();
    let got = sink.drain().len();
    // About 50 ms (plus a callback's worth of jitter) — never the 200 ms of the count.
    assert!(got <= RATE as usize * 100 / 1000, "the take got {got} samples: the count leaked in");
}

#[test]
fn a_pause_keeps_the_mic_open_and_resume_is_heard_at_once() {
    let (mic, opened, closed) = slow_mic(WAKE);
    let warm = WarmMic::warm(Box::new(mic)).unwrap();
    std::thread::sleep(WAKE + Duration::from_millis(150));
    // Take 0, then its pause: the take's handle stops and goes; the recorder's stays.
    assert!(first_sound(&mut warm.clone(), Duration::from_secs(1)).is_some());
    assert_eq!(closed.load(Ordering::SeqCst), 0, "a pause doesn't close the device");
    std::thread::sleep(Duration::from_millis(300));
    // Resume: take 1 hears it at its zero, without a second wake.
    let heard = first_sound(&mut warm.clone(), Duration::from_secs(1)).expect("sound");
    assert!(heard < Duration::from_millis(60), "{heard:?}");
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    // The recording ends (or the count is cancelled): the last handle closes the device.
    drop(warm);
    assert_eq!(closed.load(Ordering::SeqCst), 1);
}

#[test]
fn a_cancelled_count_closes_the_mic() {
    let (mic, _, closed) = slow_mic(WAKE);
    let warm = WarmMic::warm(Box::new(mic)).unwrap();
    drop(warm);
    assert_eq!(closed.load(Ordering::SeqCst), 1);
}

#[test]
fn the_macs_sound_is_mixed_from_the_takes_zero_with_a_warmed_mic() {
    // With a cold headset the mix (B172) had no voice blocks to ride for two seconds and the
    // Mac's sound was cut back to its jitter buffer; warmed, it's there from the first block.
    let (mic, _, _) = slow_mic(WAKE);
    let warm = WarmMic::warm(Box::new(mic)).unwrap();
    std::thread::sleep(WAKE + Duration::from_millis(150));
    let system = SystemAudio::new();
    let mut take = Mixed::new(Some(Box::new(warm.clone())), system.clone());
    let sink = AudioSink::with_capacity(RATE as usize * 10);
    take.start(sink.clone()).unwrap();
    let got: Arc<Mutex<Vec<f32>>> = Arc::default();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(300) {
        system.feed(&[0.5; SYSTEM_RATE as usize / 100]);
        got.lock().unwrap().extend(sink.drain());
        std::thread::sleep(Duration::from_millis(10));
    }
    take.stop().unwrap();
    let got = got.lock().unwrap();
    // The first 100 ms of the take already carry voice + the Mac's sound (0.25 + 0.5).
    let head = &got[..(RATE as usize / 10).min(got.len())];
    assert!(head.len() == RATE as usize / 10, "only {} samples in the first 100 ms", head.len());
    let mixed = head.iter().filter(|s| (**s - 0.75).abs() < 0.01).count();
    assert!(mixed > head.len() / 2, "{mixed} of {} head samples carried both", head.len());
}
