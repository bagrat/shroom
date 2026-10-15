//! The ffmpeg argument list for one take — pure, so the recipe is testable byte for byte.

use crate::{FPS, SEGMENT_SECS, files};

#[derive(Debug, Clone, PartialEq)]
pub struct TakeSpec {
    /// Frame size on the rawvideo pipe (already the output size — no scaling here).
    pub width: u32,
    pub height: u32,
    pub video_kbps: u32,
    /// This take's number, 0-based.
    pub take: u32,
    /// Global index of this take's first segment, so names stay contiguous across takes.
    pub start_number: u32,
    pub audio: Option<AudioInput>,
}

/// Mono 32-bit float PCM, read from a named pipe in the recording directory.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInput {
    pub sample_rate: u32,
    pub fifo: String,
    /// The rate the take's audio is encoded at when it isn't the input's: every take of a
    /// recording shares one `init.mp4` (B11), so a later take keeps the first take's rate —
    /// a microphone at another rate is resampled to it.
    pub out_rate: Option<u32>,
}

pub fn ffmpeg_args(spec: &TakeSpec) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(|s| s.to_string()));

    // Never -nostdin: stdin carries the frames, and EOF on it is the stop signal.
    push(&["-hide_banner", "-loglevel", "error", "-y"]);
    push(&["-f", "rawvideo", "-pix_fmt", "bgra"]);
    let size = format!("{}x{}", spec.width, spec.height);
    let fps = FPS.to_string();
    push(&["-s", &size, "-framerate", &fps, "-i", "pipe:0"]);
    if let Some(audio) = &spec.audio {
        let rate = audio.sample_rate.to_string();
        push(&["-thread_queue_size", "4096", "-f", "f32le", "-ar", &rate, "-ac", "1", "-i", &audio.fifo]);
    }

    let kbps = format!("{}k", spec.video_kbps);
    push(&["-c:v", "h264_videotoolbox", "-b:v", &kbps, "-pix_fmt", "yuv420p"]);
    // A constant output rate + keyframes forced on segment boundaries are what let the
    // HLS muxer slice cleanly (POC §2: without them it wedges with a 0-byte init.mp4).
    let keyframes = format!("expr:gte(t,n_forced*{SEGMENT_SECS})");
    push(&["-r", &fps, "-force_key_frames", &keyframes]);
    if let Some(audio) = &spec.audio {
        push(&["-c:a", "aac", "-b:a", "128k"]);
        if let Some(out) = audio.out_rate.filter(|r| *r != audio.sample_rate) {
            let out = out.to_string();
            push(&["-ar", &out]);
        }
    }

    push(&["-f", "tee", "-map", "0:v"]);
    if spec.audio.is_some() {
        push(&["-map", "1:a"]);
    }
    let hls = format!(
        "[f=hls:hls_segment_type=fmp4:hls_time={SEGMENT_SECS}:hls_playlist_type=vod:start_number={}:hls_fmp4_init_filename={}:hls_segment_filename={}]{}",
        spec.start_number,
        files::take_init(spec.take),
        files::SEGMENT_PATTERN,
        files::take_playlist(spec.take),
    );
    // The single-file leg is fragmented MP4 with 1 s fragments: readable up to the last
    // second if the Mac loses power mid-take (law 2). `delay_moov` waits for the encoder's
    // codec configuration before writing the header — `empty_moov` alone writes a header
    // nothing can decode — and `flush_packets` writes each fragment through ffmpeg's 256 KiB
    // output buffer at once (without it, seconds of video sat in memory; both measured).
    // It's remuxed to faststart off the Stop path.
    let mp4 = format!(
        "[f=mp4:movflags=+frag_keyframe+empty_moov+delay_moov+default_base_moof:frag_duration=1000000:flush_packets=1]{}",
        files::take_preview(spec.take)
    );
    a.push(format!("{hls}|{mp4}"));
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> TakeSpec {
        TakeSpec { width: 1920, height: 1080, video_kbps: 3500, take: 0, start_number: 0, audio: None }
    }

    #[test]
    fn video_only_take_matches_the_probed_recipe() {
        let args = ffmpeg_args(&spec()).join(" ");
        assert_eq!(
            args,
            "-hide_banner -loglevel error -y -f rawvideo -pix_fmt bgra -s 1920x1080 -framerate 30 -i pipe:0 \
             -c:v h264_videotoolbox -b:v 3500k -pix_fmt yuv420p -r 30 -force_key_frames expr:gte(t,n_forced*6) \
             -f tee -map 0:v \
             [f=hls:hls_segment_type=fmp4:hls_time=6:hls_playlist_type=vod:start_number=0:hls_fmp4_init_filename=init.mp4:hls_segment_filename=seg_%05d.m4s]stream_0.m3u8|[f=mp4:movflags=+frag_keyframe+empty_moov+delay_moov+default_base_moof:frag_duration=1000000:flush_packets=1]preview_0.mp4"
        );
    }

    #[test]
    fn never_disables_stdin() {
        assert!(!ffmpeg_args(&spec()).iter().any(|a| a == "-nostdin"));
    }

    #[test]
    fn later_takes_continue_the_numbering_with_their_own_playlist() {
        let args = ffmpeg_args(&TakeSpec { take: 2, start_number: 17, ..spec() });
        let tee = args.last().unwrap();
        assert!(tee.contains(":start_number=17:"));
        // Its own init segment: a later take never opens the shared one (law 2).
        assert!(tee.contains(":hls_fmp4_init_filename=init_2.mp4:"), "{tee}");
        assert!(tee.ends_with("]stream_2.m3u8|[f=mp4:movflags=+frag_keyframe+empty_moov+delay_moov+default_base_moof:frag_duration=1000000:flush_packets=1]preview_2.mp4"));
    }

    #[test]
    fn audio_adds_the_second_input_codec_and_map() {
        let args = ffmpeg_args(&TakeSpec {
            audio: Some(AudioInput { sample_rate: 48000, fifo: "mic.f32".into(), out_rate: None }),
            ..spec()
        })
        .join(" ");
        assert!(args.contains("-thread_queue_size 4096 -f f32le -ar 48000 -ac 1 -i mic.f32"));
        assert!(args.contains("-c:a aac -b:a 128k"));
        assert!(args.contains("-map 0:v -map 1:a"));
    }

    #[test]
    fn a_later_take_keeps_the_recordings_audio_rate() {
        let at = |rate: u32, out: Option<u32>| {
            ffmpeg_args(&TakeSpec { audio: Some(AudioInput { sample_rate: rate, fifo: "mic_1.f32".into(), out_rate: out }), ..spec() }).join(" ")
        };
        assert!(at(44100, Some(48000)).contains("-c:a aac -b:a 128k -ar 48000"), "resampled to the first take's rate");
        assert!(!at(48000, Some(48000)).contains("-b:a 128k -ar"), "the same rate needs nothing");
        assert!(at(44100, Some(48000)).contains("-f f32le -ar 44100 -ac 1"), "the input is read at its own rate");
    }
}
