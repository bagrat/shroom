//! The encode leg: raw BGRA frames in, HLS-fMP4 segments + a faststart MP4 out.
//!
//! The recipe is the POC's validated one (POC-LEARNINGS §2), fed from ScreenCaptureKit
//! over a rawvideo pipe instead of avfoundation (RESEARCH, "Hands-on probe results").
//! One ffmpeg per *take* (a run between pauses); a pause ends the take, the next one
//! continues the global segment numbering, and finalize joins the takes into one
//! playlist with a discontinuity at each pause.

pub mod args;
pub mod clock;
pub mod locate;
pub mod playlist;
pub mod quality;
pub mod take;

/// File names inside a recording's directory — shared by every take.
pub mod files {
    /// The final playlist, assembled at finalize.
    pub const PLAYLIST: &str = "stream.m3u8";
    /// The recording's init segment: written by its first take, shared by every take whose
    /// picture and sound match (POC §2, B11) — and never rewritten after (`take_init`).
    pub const INIT_SEGMENT: &str = "init.mp4";
    /// printf pattern ffmpeg writes segments with.
    pub const SEGMENT_PATTERN: &str = "seg_%05d.m4s";
    /// The final single-file copy, assembled at finalize.
    pub const PREVIEW: &str = "preview.mp4";
    /// The unfurl poster (og:image): the first frame, 1200 wide.
    pub const POSTER: &str = "poster.jpg";
    /// The unfurl teaser (og:video): the first seconds, small — iMessage and Discord play it
    /// inline, so it never carries the whole recording.
    pub const TEASER: &str = "teaser.mp4";

    pub fn take_playlist(take: u32) -> String {
        format!("stream_{take}.m3u8")
    }

    /// The init segment a take's encoder writes. Only the first take writes the shared
    /// [`INIT_SEGMENT`]: ffmpeg's HLS muxer opens its init file empty when it starts and fills
    /// it only with its first segment, seconds later (measured 2026-10-02: ~0.4 s to ~6 s in),
    /// so a later take writing the shared name left it empty for that span — a crash there
    /// lost every earlier take's stream (law 2). A later take writes `init_<n>.mp4`;
    /// `take::settle_init` discards it when it matches the shared one.
    pub fn take_init(take: u32) -> String {
        if take == 0 { INIT_SEGMENT.to_string() } else { format!("init_{take}.mp4") }
    }

    /// The take a later take's own init segment name belongs to, if it is one.
    pub fn init_take(name: &str) -> Option<u32> {
        let digits = name.strip_prefix("init_")?.strip_suffix(".mp4")?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok().filter(|n| *n > 0)
    }

    pub fn take_preview(take: u32) -> String {
        format!("preview_{take}.mp4")
    }

    pub fn segment(index: u32) -> String {
        format!("seg_{index:05}.m4s")
    }

    /// The global index of a segment file name, if it is one.
    pub fn segment_index(name: &str) -> Option<u32> {
        let digits = name.strip_prefix("seg_")?.strip_suffix(".m4s")?;
        if digits.len() < 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    }
}

/// Output frame rate. Constant by construction — the frame clock duplicates frames
/// when the screen is idle, so the video's duration is the wall-clock duration.
pub const FPS: u32 = 30;

/// Segment length in seconds; keyframes are forced on these boundaries.
pub const SEGMENT_SECS: u32 = 6;
