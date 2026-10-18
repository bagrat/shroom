//! A take's sound, read in Rust (symphonia, as the editor's waveform is, DECISIONS B195
//! (e)): the bundled encoder carries no filter for the quiet a voice cut's edges are set in
//! (spec N65), and needs none.

use std::fs::File;
use std::io;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Reads the first sound track of `path` (an MP4) from its start, handing `f` each decoded
/// block as levels — the loudest channel's magnitude at each instant, 0 to 1 — with the
/// frame of the track it starts at and the track's rate. A packet that won't decode is
/// passed over, and the blocks after it keep their own place. Returns the rate, or `None`
/// when the file has no sound to read.
pub fn each_block(path: &Path, mut f: impl FnMut(u64, u32, &[f32])) -> io::Result<Option<u32>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp4");
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("not a file this reads: {e}")))?;
    let Some(track) = format.default_track(TrackType::Audio) else { return Ok(None) };
    let Some(params) = track.codec_params.as_ref().and_then(|p| p.audio()) else { return Ok(None) };
    let Ok(mut decoder) = symphonia::default::get_codecs().make_audio_decoder(params, &AudioDecoderOptions::default()) else {
        return Ok(None);
    };
    let (id, base) = (track.id, track.time_base);
    let mut rate = params.sample_rate;
    let (mut next, mut samples, mut levels) = (0u64, Vec::<f32>::new(), Vec::<f32>::new());
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::IoError(e)) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        };
        if packet.track_id != id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(Error::DecodeError(_) | Error::IoError(_)) => continue,
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        };
        let r = buf.spec().rate();
        let channels = buf.spec().channels().count().max(1);
        rate = Some(r);
        // Where this block sits: its packet's time on the track's clock, else right after the last.
        let at = match base {
            Some(b) if packet.pts.get() >= 0 => {
                (packet.pts.get() as u128 * b.numer.get() as u128 * r as u128 / b.denom.get() as u128) as u64
            }
            _ => next,
        };
        samples.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut samples);
        levels.clear();
        levels.extend(samples.chunks(channels).map(|frame| frame.iter().fold(0.0f32, |m, s| m.max(s.abs()))));
        f(at, r, &levels);
        next = at + levels.len() as u64;
    }
    Ok(rate)
}
