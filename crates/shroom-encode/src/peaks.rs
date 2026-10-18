//! The editor's waveform (N63, DECISIONS B195 (e)): a recording's sound as one peak per
//! 20 ms, read in Rust from the MP4 itself — the bundled ffmpeg writes no raw sound, and the
//! editor draws the whole recording at once.
//!
//! The default audio track is decoded with symphonia (AAC in an MP4, fragmented as a take's
//! `preview_<k>.mp4` is written, or faststart as `preview.mp4` and the bucket's `video.mp4`
//! are). Each bucket holds the loudest sample in its 20 ms over every channel, scaled to a
//! byte. The length is the picture's (the clock the record's `durationSec` and the edit list
//! use), the sound's when there is no picture.
//!
//! symphonia doesn't apply an MP4's edit list, so the sound reads late by the AAC encoder's
//! priming (1024 samples, 21 ms at 48 kHz — about one bucket). The waveform is a picture of
//! the sound, never a clock: nothing is placed by it.

use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as Sym;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Buckets per second: 20 ms each.
pub const PER_SECOND: u32 = 50;

/// A recording's sound, one byte per bucket: `round(255 × min(1, peak))`, the peak being
/// the largest |sample| in the bucket over every channel. `values.len()` covers `duration`
/// (the last bucket may be partial).
#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    pub per_second: u32,
    /// Seconds: the picture's length (the sound's, when there's no picture).
    pub duration: f64,
    pub values: Vec<u8>,
}

#[derive(Debug)]
pub enum PeaksError {
    Io(io::Error),
    /// Not an MP4 this reader can open, or its sound can't be decoded.
    Unreadable(String),
    /// No sound, and no picture to say how long the silence lasts.
    NoLength,
}

impl From<io::Error> for PeaksError {
    fn from(e: io::Error) -> Self {
        PeaksError::Io(e)
    }
}

impl fmt::Display for PeaksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeaksError::Io(e) => write!(f, "io: {e}"),
            PeaksError::Unreadable(why) => write!(f, "unreadable media: {why}"),
            PeaksError::NoLength => write!(f, "no sound and no picture length"),
        }
    }
}

impl std::error::Error for PeaksError {}

fn unreadable(e: Sym) -> PeaksError {
    match e {
        Sym::IoError(e) => PeaksError::Io(e),
        e => PeaksError::Unreadable(e.to_string()),
    }
}

/// How many buckets cover `duration` seconds.
pub fn buckets(duration: f64) -> usize {
    if !duration.is_finite() || duration <= 0.0 {
        return 0;
    }
    // A hair under, so 4.0 s is 200 buckets, never 201 by rounding.
    (duration * f64::from(PER_SECOND) - 1e-6).ceil().max(0.0) as usize
}

/// Reads the waveform of the MP4 at `path`.
pub fn read(path: &Path) -> Result<Peaks, PeaksError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp4");
    let mut format = symphonia::default::get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).map_err(unreadable)?;

    // Seconds per tick of each track's clock, and whether it's the picture.
    let clocks: Vec<(u32, f64, bool)> = format
        .tracks()
        .iter()
        .filter_map(|t| {
            let tb = f64::from(t.time_base?);
            let video = t.codec_params.as_ref().and_then(|p| p.video()).is_some();
            Some((t.id, tb, video))
        })
        .collect();
    // What the headers say, for a file whose packets end before saying it themselves.
    let header_video = format
        .tracks()
        .iter()
        .filter(|t| t.codec_params.as_ref().and_then(|p| p.video()).is_some())
        .filter_map(|t| Some(t.time_base?.calc_duration(t.duration?)?.as_secs_f64()))
        .fold(0.0f64, f64::max);

    let audio = format.default_track(TrackType::Audio).cloned();
    let mut decoder = match &audio {
        Some(track) => {
            let params = track.codec_params.as_ref().and_then(|p| p.audio()).ok_or_else(|| PeaksError::Unreadable("the sound track has no codec parameters".into()))?;
            Some(symphonia::default::get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default()).map_err(unreadable)?)
        }
        None => None,
    };
    let audio_id = audio.as_ref().map(|t| t.id);
    let audio_tb = audio.as_ref().and_then(|t| t.time_base).map(f64::from).unwrap_or(0.0);

    let mut peaks: Vec<f32> = Vec::new();
    let mut planes: Vec<Vec<f32>> = Vec::new();
    let (mut video_end, mut audio_end) = (0.0f64, 0.0f64);
    // Where the next packet's sound starts when the track has no clock: right after the last.
    let mut next_sample = 0i64;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            // A file cut short (a take that crashed mid-write) reads up to where it ends.
            Err(Sym::IoError(e)) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(Sym::ResetRequired) => break,
            Err(e) => return Err(unreadable(e)),
        };
        if let Some(&(_, tb, video)) = clocks.iter().find(|c| c.0 == packet.track_id) {
            let end = (packet.pts.get() as f64 + packet.dur.get() as f64) * tb;
            if video {
                video_end = video_end.max(end);
            } else if Some(packet.track_id) == audio_id {
                audio_end = audio_end.max(end);
            }
        }
        if Some(packet.track_id) != audio_id {
            continue;
        }
        let Some(dec) = decoder.as_mut() else { continue };
        let buf = match dec.decode(&packet) {
            Ok(buf) => buf,
            // A packet that won't decode leaves its buckets quiet; the rest still reads.
            Err(Sym::DecodeError(_)) => continue,
            Err(e) => return Err(unreadable(e)),
        };
        let rate = u64::from(buf.spec().rate());
        if rate == 0 || buf.frames() == 0 {
            continue;
        }
        buf.copy_to_vecs_planar::<f32>(&mut planes);
        // Placed by its own timestamp, so a gap in the sound stays a gap.
        let first = if audio_tb > 0.0 { (packet.pts.get() as f64 * audio_tb * rate as f64).round() as i64 } else { next_sample };
        next_sample = first + buf.frames() as i64;
        for plane in &planes {
            fold_plane(&mut peaks, plane, first, rate);
        }
    }

    let duration = if video_end > 0.0 {
        video_end
    } else if header_video > 0.0 {
        header_video
    } else if audio_id.is_some() {
        audio_end
    } else {
        0.0
    };
    if audio_id.is_none() && duration <= 0.0 {
        return Err(PeaksError::NoLength);
    }
    let n = buckets(duration);
    let mut values: Vec<u8> = peaks.iter().take(n).map(|p| (255.0 * p.min(1.0)).round() as u8).collect();
    values.resize(n, 0);
    Ok(Peaks { per_second: PER_SECOND, duration, values })
}

/// Folds one channel's samples, the first of them at sample `first` of the track, into the
/// buckets: each bucket keeps its loudest |sample|.
fn fold_plane(peaks: &mut Vec<f32>, plane: &[f32], first: i64, rate: u64) {
    let per = u64::from(PER_SECOND);
    let mut n = if first < 0 { (-first) as usize } else { 0 };
    while n < plane.len() {
        let at = (first + n as i64) as u64;
        let bucket = (at * per / rate) as usize;
        // The first sample of the next bucket.
        let next = ((bucket as u64 + 1) * rate).div_ceil(per);
        let end = ((next as i64 - first) as usize).min(plane.len());
        let loudest = plane[n..end].iter().fold(0.0f32, |m, s| if s.abs() > m { s.abs() } else { m });
        if peaks.len() <= bucket {
            peaks.resize(bucket + 1, 0.0);
        }
        if loudest > peaks[bucket] {
            peaks[bucket] = loudest;
        }
        n = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_cover_the_length() {
        assert_eq!(buckets(4.0), 200);
        assert_eq!(buckets(4.001), 201, "a partial bucket counts");
        assert_eq!(buckets(0.0), 0);
        assert_eq!(buckets(f64::NAN), 0);
        assert_eq!(buckets(-3.0), 0);
    }

    #[test]
    fn each_bucket_keeps_its_loudest_sample() {
        let mut p = Vec::new();
        // 48 kHz: 960 samples a bucket. Two packets, the second starting mid-bucket.
        let mut a = vec![0.1f32; 1024];
        a[10] = -0.9;
        a[1000] = 0.5;
        fold_plane(&mut p, &a, 0, 48_000);
        let mut b = vec![0.0f32; 1024];
        b[0] = 0.7;
        fold_plane(&mut p, &b, 1024, 48_000);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], 0.9);
        assert_eq!(p[1], 0.7, "the later packet's louder sample wins in the shared bucket");
        assert_eq!(p[2], 0.0);
    }

    #[test]
    fn samples_before_zero_are_dropped() {
        let mut p = Vec::new();
        let mut a = vec![0.0f32; 1024];
        a[0] = 1.0; // at -24: before the start
        a[30] = 0.25;
        fold_plane(&mut p, &a, -24, 48_000);
        assert_eq!(p[0], 0.25);
    }

    #[test]
    fn a_file_that_isnt_a_movie_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.mp4");
        std::fs::write(&path, b"not a movie at all, just some bytes").unwrap();
        assert!(matches!(read(&path), Err(PeaksError::Unreadable(_))), "{:?}", read(&path));
        assert!(matches!(read(&dir.path().join("missing.mp4")), Err(PeaksError::Io(_))));
    }
}
