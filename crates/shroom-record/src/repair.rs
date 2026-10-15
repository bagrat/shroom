//! Rebuilding a take after a crash (law 2: the recording is sacred).
//!
//! Each take's single-file leg is fragmented MP4 with 1 s fragments, readable up to the
//! last second however the take ended; HLS segments, written whole at 6 s boundaries, can
//! lag by up to 6 s. So repair rebuilds the whole recording from the fragmented files —
//! concatenated, remuxed to faststart, then re-segmented into HLS with one init segment and
//! continuous timestamps — and falls back to the segments only if no file is readable.

use std::path::Path;
use std::process::{Command, Stdio};

use shroom_encode::files;
use shroom_encode::playlist::{assemble_mapped, take_segments};
use shroom_encode::take::{assemble_preview, take_map, take_numbers};

/// What repair recovered.
#[derive(Debug, Clone, PartialEq)]
pub struct Repaired {
    pub seconds: f64,
    pub segments: usize,
    pub from_fragments: bool,
}

fn run(ffmpeg: &Path, dir: &Path, args: &[&str]) -> bool {
    Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Repairs the recording in `dir`; `None` if nothing on disk could be saved (the take is
/// left as it is — never deleted).
pub fn repair(ffmpeg: &Path, dir: &Path) -> Option<Repaired> {
    let takes = take_numbers(dir);
    if !takes.is_empty() && assemble_preview(ffmpeg, dir, &takes).is_ok() {
        // Re-segment from the whole recording into a staging dir, then swap it in.
        let stage = dir.join("rebuild");
        let _ = std::fs::remove_dir_all(&stage);
        let _ = std::fs::create_dir_all(&stage);
        let ok = run(
            ffmpeg,
            dir,
            &[
                "-i", files::PREVIEW, "-c", "copy", "-f", "hls", "-hls_segment_type", "fmp4", "-hls_time", "6",
                "-hls_playlist_type", "vod", "-hls_fmp4_init_filename", files::INIT_SEGMENT,
                "-hls_segment_filename", "rebuild/seg_%05d.m4s", "rebuild/stream.m3u8",
            ],
        );
        if ok {
            for entry in std::fs::read_dir(dir).ok()?.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if files::segment_index(&name).is_some()
                    || name == files::INIT_SEGMENT
                    || files::init_take(&name).is_some()
                    || name.starts_with("stream") && name.ends_with(".m3u8")
                {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
            for entry in std::fs::read_dir(&stage).ok()?.flatten() {
                let name = entry.file_name();
                let target = if name == "stream.m3u8" { dir.join(files::PLAYLIST) } else { dir.join(&name) };
                let _ = std::fs::rename(entry.path(), target);
            }
            let _ = std::fs::remove_dir_all(&stage);
            let text = std::fs::read_to_string(dir.join(files::PLAYLIST)).ok()?;
            let segs = take_segments(&text);
            return Some(Repaired { seconds: segs.iter().map(|s| s.seconds).sum(), segments: segs.len(), from_fragments: true });
        }
    }
    // Fallback: whatever per-take playlists survived, each under the init it was made with.
    let per_take: Vec<_> = (0..)
        .map_while(|k| std::fs::read_to_string(dir.join(files::take_playlist(k))).ok().map(|t| (take_map(dir, k), take_segments(&t))))
        .collect();
    let master = assemble_mapped(&per_take);
    if master.segments.is_empty() {
        return None;
    }
    std::fs::write(dir.join(files::PLAYLIST), &master.text).ok()?;
    Some(Repaired { seconds: master.seconds, segments: master.segments.len(), from_fragments: false })
}
