//! The voice-cut drive's microphone (spec N65): stock-voice speech from a WAV the drive is
//! handed, in real time, then silence, in place of the beep, so a drive's take says
//! “shroom cut” where the space's hearing and the voice lane can find it. Never the room, as
//! the beep never is (whole-spec audit b4 #17): what it plays is a file made by the Mac's own
//! stock voice.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use shroom_record::audio::{AudioSink, AudioSource};

/// A microphone that plays its speech (mono, at its rate) from the take's start, then
/// silence; a take resumed after a pause goes on where it left off.
pub struct SpeechSource {
    rate: u32,
    samples: Arc<Vec<f32>>,
    played: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SpeechSource {
    /// The speech in `path` (a WAV of 16-bit PCM or 32-bit float, any channels, any rate),
    /// mixed to mono and played at `rate`.
    pub fn open(path: &Path, rate: u32) -> Result<SpeechSource, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("the drive's speech {}: {e}", path.display()))?;
        let (from, mono) = wav_mono(&bytes)?;
        Ok(SpeechSource::new(resample(&mono, from, rate), rate))
    }

    /// Speech already at `rate`.
    pub fn new(samples: Vec<f32>, rate: u32) -> SpeechSource {
        SpeechSource { rate, samples: Arc::new(samples), played: Arc::default(), stop: Arc::default(), thread: None }
    }

    /// How long the speech lasts (seconds).
    pub fn seconds(&self) -> f64 {
        self.samples.len() as f64 / self.rate as f64
    }
}

/// A WAV's sound mixed to mono: its rate and its samples.
pub fn wav_mono(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("the drive's speech isn't a WAV".into());
    }
    let mut at = 12;
    // (format, channels, rate, bits)
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = &bytes[at + 8..(at + 8 + len).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            let u16_at = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
            // WAVE_FORMAT_EXTENSIBLE names the format in its subformat's first two bytes.
            let format = if u16_at(0) == 0xFFFE && body.len() >= 26 { u16_at(24) } else { u16_at(0) };
            fmt = Some((format, u16_at(2), u32::from_le_bytes([body[4], body[5], body[6], body[7]]), u16_at(14)));
        } else if id == b"data" {
            let (format, channels, rate, bits) = fmt.ok_or("the drive's speech has its sound before its format")?;
            let samples: Vec<f32> = match (format, bits) {
                (1, 16) => body.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect(),
                (3, 32) => body.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect(),
                _ => return Err(format!("the drive's speech is format {format} at {bits} bits: only 16-bit PCM or 32-bit float")),
            };
            let ch = channels.max(1) as usize;
            return Ok((rate, samples.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect()));
        }
        at += 8 + len + (len & 1);
    }
    Err("the drive's speech has no sound".into())
}

/// `samples` at `from` Hz, at `to` Hz: linear, enough for speech a transcriber hears.
fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || samples.is_empty() {
        return samples.to_vec();
    }
    let n = (samples.len() as u64 * to as u64 / from as u64) as usize;
    let last = samples.len() - 1;
    (0..n)
        .map(|i| {
            let x = i as f64 * from as f64 / to as f64;
            let k = x.floor() as usize;
            let f = (x - k as f64) as f32;
            let (a, b) = (samples[k.min(last)], samples[(k + 1).min(last)]);
            a + (b - a) * f
        })
        .collect()
}

impl AudioSource for SpeechSource {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn start(&mut self, sink: AudioSink) -> Result<(), String> {
        self.stop.store(false, Ordering::SeqCst);
        let (stop, samples, played, rate) = (self.stop.clone(), self.samples.clone(), self.played.clone(), self.rate as f64);
        self.thread = Some(
            std::thread::Builder::new()
                .name("shroom-drive-speech".into())
                .spawn(move || {
                    // Paced by the host's steady clock, 10 ms at a time, as the beep is (B174).
                    let start = Instant::now();
                    let from = played.load(Ordering::SeqCst);
                    let mut buf = Vec::new();
                    while !stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(10));
                        let due = from + (start.elapsed().as_secs_f64() * rate).floor() as usize;
                        let at = played.load(Ordering::SeqCst);
                        buf.clear();
                        buf.extend((at..due).map(|i| samples.get(i).copied().unwrap_or(0.0)));
                        sink.push(&buf);
                        played.store(due, Ordering::SeqCst);
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        Ok(())
    }
}

impl Drop for SpeechSource {
    fn drop(&mut self) {
        let _ = AudioSource::stop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(format: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend(((36 + data.len()) as u32).to_le_bytes());
        v.extend(b"WAVEfmt ");
        v.extend(16u32.to_le_bytes());
        v.extend(format.to_le_bytes());
        v.extend(channels.to_le_bytes());
        v.extend(rate.to_le_bytes());
        v.extend((rate * channels as u32 * bits as u32 / 8).to_le_bytes());
        v.extend((channels * bits / 8).to_le_bytes());
        v.extend(bits.to_le_bytes());
        v.extend(b"data");
        v.extend((data.len() as u32).to_le_bytes());
        v.extend(data);
        v
    }

    #[test]
    fn a_wav_reads_as_mono_whatever_its_channels_and_samples() {
        let pcm: Vec<u8> = [16384i16, -16384, 0, 32767].iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(wav_mono(&wav(1, 1, 22_050, 16, &pcm)).unwrap(), (22_050, vec![0.5, -0.5, 0.0, 32767.0 / 32768.0]));
        let float: Vec<u8> = [0.25f32, 0.75, -1.0, 0.0].iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(wav_mono(&wav(3, 2, 48_000, 32, &float)).unwrap(), (48_000, vec![0.5, -0.5]));
        assert!(wav_mono(b"not a wav at all").is_err());
        assert!(wav_mono(&wav(1, 1, 48_000, 24, &[0; 6])).is_err());
    }

    #[test]
    fn speech_at_another_rate_lasts_as_long() {
        let s = resample(&vec![0.1; 22_050], 22_050, 48_000);
        assert_eq!(s.len(), 48_000);
        assert!(s.iter().all(|&x| (x - 0.1).abs() < 1e-6));
    }

    #[test]
    fn it_plays_the_speech_from_the_start_then_silence() {
        let speech: Vec<f32> = (0..480).map(|i| (i + 1) as f32 / 1000.0).collect();
        let mut src = SpeechSource::new(speech.clone(), 48_000);
        let sink = AudioSink::with_capacity(48_000);
        src.start(sink.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(120));
        src.stop().unwrap();
        let heard = sink.drain();
        assert!(heard.len() >= 480 * 5, "{} samples in 120 ms", heard.len());
        assert_eq!(&heard[..480], &speech[..]);
        assert!(heard[480..].iter().all(|&x| x == 0.0));
    }
}
