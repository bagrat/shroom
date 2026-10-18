//! The permanent copy of an edit (N63, DECISIONS B195 (f)): the kept ranges of a recording
//! cut out of its single-file copy and joined into one faststart MP4, by the bundled ffmpeg.
//!
//! Each kept range is re-encoded on its own (`-ss <start> -i <src> -t <len>`: an input seek
//! that the CLI makes frame-exact with its own `trim`), the picture at the source's own
//! bitrate, the sound as AAC 128k; the parts are then joined by the concat *demuxer* with
//! stream copy. Nothing here needs a filter, codec or format the release's encoder lacks
//! (`tools/build-ffmpeg.sh`: no concat filter; the concat demuxer, mov, mp4, h264, aac,
//! trim/atrim, scale and format are in it).

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// The picture encoder: VideoToolbox on a Mac (the release's), x264 where it doesn't exist
/// (the media tests on Linux).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoder {
    #[default]
    VideoToolbox,
    Libx264,
}

impl Encoder {
    pub fn codec(self) -> &'static str {
        match self {
            Encoder::VideoToolbox => "h264_videotoolbox",
            Encoder::Libx264 => "libx264",
        }
    }

    /// `-c:v <codec> -b:v <kbps>k -pix_fmt yuv420p`.
    pub fn video_args(self, kbps: u32) -> Vec<OsString> {
        ["-c:v", self.codec(), "-b:v", &format!("{kbps}k"), "-pix_fmt", "yuv420p"].iter().map(OsString::from).collect()
    }
}

/// The copy's sound.
pub const AUDIO_KBPS: u32 = 128;
/// The picture's bitrate is the source's, held to this range (kbps).
pub const MIN_VIDEO_KBPS: u32 = 500;
pub const MAX_VIDEO_KBPS: u32 = 20_000;

/// The source's picture bitrate, estimated: the whole file's bits over its length, less the
/// sound's 128k, held to [`MIN_VIDEO_KBPS`]–[`MAX_VIDEO_KBPS`]. An unknown length gives the
/// floor.
pub fn video_kbps(bytes: u64, duration: f64) -> u32 {
    if !duration.is_finite() || duration <= 0.0 {
        return MIN_VIDEO_KBPS;
    }
    let total = bytes as f64 * 8.0 / duration / 1000.0;
    let video = total - f64::from(AUDIO_KBPS);
    video.round().clamp(f64::from(MIN_VIDEO_KBPS), f64::from(MAX_VIDEO_KBPS)) as u32
}

/// Seconds as ffmpeg reads them, to the millisecond.
fn secs(t: f64) -> String {
    format!("{:.3}", t.max(0.0))
}

/// A path as one argument ffmpeg reads as a file: a relative one that would read as an
/// option (`-x`) or a protocol (`a:b`) is spelled from `./`.
pub fn path_arg(p: &Path) -> OsString {
    let s = p.as_os_str();
    let first = p.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default();
    if p.is_relative() && (first.starts_with('-') || first.contains(':')) {
        let mut o = OsString::from("./");
        o.push(s);
        return o;
    }
    s.to_os_string()
}

fn os(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

/// One kept range, `[start, start + len)` of `src`, re-encoded into `out` (faststart MP4).
pub fn part_args(src: &Path, start: f64, len: f64, kbps: u32, enc: Encoder, out: &Path) -> Vec<OsString> {
    let mut a = os(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-ss", &secs(start), "-i"]);
    a.push(path_arg(src));
    a.extend(os(&["-t", &secs(len), "-map", "0:v:0", "-map", "0:a:0?"]));
    a.extend(enc.video_args(kbps));
    a.extend(os(&["-c:a", "aac", "-b:a", &format!("{AUDIO_KBPS}k"), "-movflags", "+faststart", "-f", "mp4"]));
    a.push(path_arg(out));
    a
}

/// The parts named in `list` (see [`concat_list`]) joined into `out`: stream copy, faststart.
pub fn concat_args(list: &Path, out: &Path) -> Vec<OsString> {
    let mut a = os(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-f", "concat", "-safe", "0", "-i"]);
    a.push(path_arg(list));
    a.extend(os(&["-map", "0", "-c", "copy", "-movflags", "+faststart", "-f", "mp4"]));
    a.push(path_arg(out));
    a
}

/// A path inside the concat demuxer's list: single-quoted, a quote inside written `'\''`
/// (the demuxer's own escaping). A path that can't be one line of the list is refused.
pub fn quote(path: &Path) -> io::Result<String> {
    let s = path.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "a part's path isn't UTF-8"))?;
    if s.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a part's path holds a line break"));
    }
    Ok(format!("'{}'", s.replace('\'', r"'\''")))
}

/// The concat demuxer's list: one `file '<path>'` line per part, in order.
pub fn concat_list(parts: &[PathBuf]) -> io::Result<String> {
    let mut out = String::from("ffconcat version 1.0\n");
    for p in parts {
        out.push_str(&format!("file {}\n", quote(p)?));
    }
    Ok(out)
}

/// The kept ranges that can be cut: finite, non-negative, in order, each at least a frame long.
fn usable(keep: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = keep
        .iter()
        .filter(|(s, e)| s.is_finite() && e.is_finite())
        .map(|(s, e)| (s.max(0.0), *e))
        .filter(|(s, e)| e - s >= 0.001)
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The picture's length as the file's header gives it (seconds), when it does: a faststart
/// MP4's video track, read without touching its media. `None` for a fragmented file, a file
/// with no picture, or one that isn't an MP4.
pub fn picture_length(path: &Path) -> Option<f64> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp4");
    let format = symphonia::default::get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).ok()?;
    let track = format.default_track(TrackType::Video)?;
    let secs = track.time_base?.calc_duration(track.duration?)?.as_secs_f64();
    (secs > 0.0).then_some(secs)
}

fn run(ffmpeg: &Path, args: &[OsString], what: &str) -> io::Result<()> {
    let out = Command::new(ffmpeg).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output()?;
    if out.status.success() {
        return Ok(());
    }
    let log = String::from_utf8_lossy(&out.stderr);
    let tail: String = log.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" / ");
    Err(io::Error::other(format!("{what} exited {}: {tail}", out.status)))
}

/// The work folder beside `out`, so the joined file is renamed into place on one disk.
fn work_dir(out: &Path) -> PathBuf {
    let name = out.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "copy".into());
    out.with_file_name(format!(".{name}.parts"))
}

/// How far a part's encode has come, from one line of ffmpeg's `-progress` report: its
/// output's time (`out_time_us=`, or the older `out_time_ms=`, which counts microseconds
/// too), in seconds. `None` for every other line, and for a time not yet known.
pub fn progress_secs(line: &str) -> Option<f64> {
    let (key, value) = line.trim().split_once('=')?;
    if key != "out_time_us" && key != "out_time_ms" {
        return None;
    }
    let us: i64 = value.trim().parse().ok()?;
    (us >= 0).then(|| us as f64 / 1_000_000.0)
}

/// [`run`], telling `seconds` how much of the output is written as ffmpeg reports it.
fn run_reporting(ffmpeg: &Path, args: &[OsString], what: &str, seconds: &mut dyn FnMut(f64)) -> io::Result<()> {
    use std::io::{BufRead, BufReader, Read};
    let mut all = os(&["-progress", "pipe:1", "-nostats"]);
    all.extend(args.iter().cloned());
    let mut child = Command::new(ffmpeg).args(&all).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    // The log is read beside the report, so neither pipe ever fills and stalls ffmpeg.
    let mut err = child.stderr.take().expect("piped");
    let log = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(t) = progress_secs(&line) {
                seconds(t);
            }
        }
    }
    let status = child.wait()?;
    let log = log.join().unwrap_or_default();
    if status.success() {
        return Ok(());
    }
    let tail: String = log.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" / ");
    Err(io::Error::other(format!("{what} exited {status}: {tail}")))
}

/// Cuts the permanent copy: the `keep` ranges of `src` (seconds on its own clock), in order,
/// into `out`, a faststart MP4. Returns the copy's length in seconds — its picture's, as the
/// written file's header says, else the sum of the kept ranges. `out` appears whole or not
/// at all; the parts are made beside it and removed.
pub fn render(ffmpeg: &Path, src: &Path, keep: &[(f64, f64)], out: &Path, enc: &Encoder) -> io::Result<f64> {
    render_with(ffmpeg, src, keep, out, enc, &mut |_| {})
}

/// [`render`], telling `progress` how far it has come (0–1: the kept seconds encoded, as
/// ffmpeg reports each part, of all of them; 1 once the copy is whole), never going back.
pub fn render_with(ffmpeg: &Path, src: &Path, keep: &[(f64, f64)], out: &Path, enc: &Encoder, progress: &mut dyn FnMut(f64)) -> io::Result<f64> {
    let keep = usable(keep);
    if keep.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "nothing is kept"));
    }
    let bytes = std::fs::metadata(src)?.len();
    let src_len = picture_length(src).unwrap_or_else(|| keep.iter().map(|k| k.1).fold(0.0, f64::max));
    let kbps = video_kbps(bytes, src_len);

    let work = work_dir(out);
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let total: f64 = keep.iter().map(|(s, e)| e - s).sum();
    let mut said = 0.0_f64;
    let mut tell = |f: f64| {
        let f = f.clamp(0.0, 1.0);
        if f > said {
            said = f;
            progress(f);
        }
    };
    let result = (|| {
        let mut parts = Vec::with_capacity(keep.len());
        let mut before = 0.0;
        for (k, (s, e)) in keep.iter().enumerate() {
            let part = work.join(format!("part-{k:03}.mp4"));
            let len = e - s;
            run_reporting(ffmpeg, &part_args(src, *s, len, kbps, *enc, &part), &format!("part {k}"), &mut |t| tell((before + t.min(len)) / total))?;
            before += len;
            tell(before / total);
            parts.push(part);
        }
        let joined = if parts.len() == 1 {
            parts.remove(0)
        } else {
            let list = work.join("parts.ffconcat");
            std::fs::write(&list, concat_list(&parts)?)?;
            let joined = work.join("joined.mp4");
            run(ffmpeg, &concat_args(&list, &joined), "the join")?;
            joined
        };
        std::fs::rename(&joined, out)
    })();
    let _ = std::fs::remove_dir_all(&work);
    result?;
    tell(1.0);
    let kept: f64 = keep.iter().map(|(s, e)| e - s).sum();
    Ok(picture_length(out).unwrap_or(kept))
}

/// Whether `ffmpeg` has the encoder `name` (`ffmpeg -encoders`).
pub fn has_encoder(ffmpeg: &Path, name: &str) -> bool {
    Command::new(ffmpeg)
        .args([OsStr::new("-hide_banner"), OsStr::new("-encoders")])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.split_whitespace().nth(1) == Some(name)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_progress_line_reads_as_the_output_written_so_far() {
        assert_eq!(progress_secs("out_time_us=1500000"), Some(1.5));
        assert_eq!(progress_secs("out_time_ms=250000\n"), Some(0.25), "the older key counts microseconds too");
        assert_eq!(progress_secs("out_time_us=N/A"), None);
        assert_eq!(progress_secs("out_time_us=-9223372036854775807"), None, "not known yet");
        assert_eq!(progress_secs("out_time=00:00:01.500000"), None);
        assert_eq!(progress_secs("progress=continue"), None);
    }

    fn line(a: &[OsString]) -> String {
        a.iter().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn a_part_is_an_input_seek_and_a_length_re_encoded() {
        let a = part_args(Path::new("/Users/x/.shroom/edits/AbCdEfGhIjKlMnOp/source.mp4"), 150.0, 12.25, 3372, Encoder::VideoToolbox, Path::new("/w/part-001.mp4"));
        assert_eq!(
            line(&a),
            "-hide_banner -loglevel error -nostdin -y -ss 150.000 -i /Users/x/.shroom/edits/AbCdEfGhIjKlMnOp/source.mp4 \
             -t 12.250 -map 0:v:0 -map 0:a:0? -c:v h264_videotoolbox -b:v 3372k -pix_fmt yuv420p \
             -c:a aac -b:a 128k -movflags +faststart -f mp4 /w/part-001.mp4"
        );
        let x = line(&part_args(Path::new("s.mp4"), 0.0, 1.0, 500, Encoder::Libx264, Path::new("p.mp4")));
        assert!(x.contains("-ss 0.000 -i s.mp4 -t 1.000 ") && x.contains("-c:v libx264 -b:v 500k"), "{x}");
    }

    #[test]
    fn the_join_is_the_concat_demuxer_with_stream_copy() {
        let a = concat_args(Path::new("/w/parts.ffconcat"), Path::new("/w/joined.mp4"));
        assert_eq!(line(&a), "-hide_banner -loglevel error -nostdin -y -f concat -safe 0 -i /w/parts.ffconcat -map 0 -c copy -movflags +faststart -f mp4 /w/joined.mp4");
        // No filter graph at all: the release's encoder has no concat filter.
        assert!(!a.iter().any(|s| s == "-filter_complex" || s == "-vf" || s == "-af"));
    }

    #[test]
    fn the_list_quotes_every_path() {
        let list = concat_list(&[PathBuf::from("/w/part-000.mp4"), PathBuf::from("/Users/Ani's Mac/it's here.mp4")]).unwrap();
        assert_eq!(list, "ffconcat version 1.0\nfile '/w/part-000.mp4'\nfile '/Users/Ani'\\''s Mac/it'\\''s here.mp4'\n");
        assert!(quote(Path::new("/w/two\nlines.mp4")).is_err());
    }

    #[test]
    fn a_path_never_reads_as_an_option_or_a_protocol() {
        assert_eq!(path_arg(Path::new("-y.mp4")), OsString::from("./-y.mp4"));
        assert_eq!(path_arg(Path::new("http:x.mp4")), OsString::from("./http:x.mp4"));
        assert_eq!(path_arg(Path::new("part-000.mp4")), OsString::from("part-000.mp4"));
        assert_eq!(path_arg(Path::new("/a:b/-c.mp4")), OsString::from("/a:b/-c.mp4"));
    }

    #[test]
    fn the_bitrate_is_the_sources_less_the_sound() {
        // 100 MB over 200 s: 4000 kbps in all, 3872 of it picture.
        assert_eq!(video_kbps(100_000_000, 200.0), 3872);
        assert_eq!(video_kbps(1_000_000, 200.0), MIN_VIDEO_KBPS, "held to the floor");
        assert_eq!(video_kbps(10_000_000_000, 60.0), MAX_VIDEO_KBPS, "held to the ceiling");
        assert_eq!(video_kbps(100_000_000, 0.0), MIN_VIDEO_KBPS);
        assert_eq!(video_kbps(100_000_000, f64::NAN), MIN_VIDEO_KBPS);
    }

    #[test]
    fn only_ranges_with_length_are_cut_in_order() {
        assert_eq!(usable(&[(5.0, 9.0), (-1.0, 2.0), (3.0, 3.0), (f64::NAN, 4.0)]), vec![(0.0, 2.0), (5.0, 9.0)]);
    }

    #[test]
    fn the_work_folder_sits_beside_the_copy() {
        assert_eq!(work_dir(Path::new("/x/edits/A/edit-3.mp4")), PathBuf::from("/x/edits/A/.edit-3.mp4.parts"));
    }
}
