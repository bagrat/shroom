//! Upload while recording (ARCHITECTURE §9.1 step 4): segments go up as soon as they're
//! complete, so Stop only has to send the tail and the playlists — and a long recording
//! is mostly in the bucket by the time it ends.
//!
//! ffmpeg writes an fMP4 segment's file only when the segment is done, and the playlist
//! only when the take ends (measured 2026-10-02). So a segment whose file exists and hasn't
//! changed for a moment is complete (the moment guards the write itself); the init segment
//! is written when the take starts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use shroom_encode::files;
use shroom_storage::s3::Client;

use crate::keys;
use crate::meta::{Meta, now_iso};
use crate::upload::{UploadError, with_retry};

/// Which of `files` (a recording directory's names, each with how long ago it last
/// changed) are complete and uploadable: anything settled for `settle`, and everything once
/// no take is running.
pub fn ready_files(files: &[(String, std::time::Duration)], take_running: bool, settle: std::time::Duration) -> Vec<String> {
    let ok = |age: &std::time::Duration| !take_running || *age >= settle;
    let mut ready: Vec<String> = Vec::new();
    if files.iter().any(|(n, age)| n == files::INIT_SEGMENT && ok(age)) {
        ready.push(files::INIT_SEGMENT.to_string());
    }
    let mut segments: Vec<u32> = files.iter().filter(|(_, age)| ok(age)).filter_map(|(n, _)| files::segment_index(n)).collect();
    segments.sort_unstable();
    ready.extend(segments.into_iter().map(files::segment));
    ready
}

/// The uploader for one recording while it's being made.
pub struct LiveUpload {
    s3: Client,
    id: String,
    dir: PathBuf,
    /// Key → size of everything already confirmed in the bucket.
    sent: HashMap<String, u64>,
    claimed: bool,
}

impl LiveUpload {
    pub fn new(s3: Client, id: &str, dir: &Path) -> LiveUpload {
        LiveUpload { s3, id: id.to_string(), dir: dir.to_path_buf(), sent: HashMap::new(), claimed: false }
    }

    /// Goes on with a fresh credential (the take's was re-minted near its end); what was sent
    /// stays sent.
    pub fn renew(&mut self, s3: Client) {
        self.s3 = s3;
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// What's confirmed in the bucket, for the final step to skip.
    pub fn sent(&self) -> &HashMap<String, u64> {
        &self.sent
    }

    /// How many of the recording's pieces are confirmed up, unbroken from the first — what
    /// this Mac tells its space for naming (N66: another minute is up every ten).
    pub fn pieces_up(&self) -> u32 {
        (0..).take_while(|n| self.sent.contains_key(&keys::variant_file(&self.id, &files::segment(*n)))).count() as u32
    }

    /// Claims the id (create-only) with the record in `uploading` state — so the link
    /// shows the title and "on its way" from the first second.
    pub fn claim(&mut self, title: &str) -> Result<(), UploadError> {
        if self.claimed {
            return Ok(());
        }
        let meta = Meta::uploading(&self.id, title, &now_iso());
        let created = with_retry(|| self.s3.put_if_absent(&keys::meta(&self.id), &serde_json::to_vec(&meta).unwrap(), "application/json"))?;
        if !created {
            let existing = with_retry(|| self.s3.get(&keys::meta(&self.id)))?.ok_or(UploadError::Collision)?;
            let existing: Meta = serde_json::from_slice(&existing).map_err(|_| UploadError::Collision)?;
            if existing.state == crate::meta::State::Deleted {
                return Err(UploadError::Tombstoned);
            }
        }
        self.claimed = true;
        Ok(())
    }

    /// Uploads every complete file not yet sent. Returns how many went up this time.
    pub fn tick(&mut self, take_running: bool) -> Result<u32, UploadError> {
        let now = std::time::SystemTime::now();
        let names: Vec<(String, std::time::Duration)> = std::fs::read_dir(&self.dir)?
            .filter_map(|e| {
                let e = e.ok()?;
                let age = e.metadata().ok()?.modified().ok().and_then(|m| now.duration_since(m).ok()).unwrap_or_default();
                Some((e.file_name().into_string().ok()?, age))
            })
            .collect();
        let mut sent = 0;
        for file in ready_files(&names, take_running, std::time::Duration::from_secs(1)) {
            let key = keys::variant_file(&self.id, &file);
            let local = self.dir.join(&file);
            let size = std::fs::metadata(&local)?.len();
            if self.sent.get(&key) == Some(&size) || size == 0 {
                continue;
            }
            let bytes = std::fs::read(&local)?;
            with_retry(|| self.s3.put(&key, &bytes, "video/iso.segment"))?;
            self.sent.insert(key, size);
            sent += 1;
        }
        Ok(sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn files(v: &[(&str, u64)]) -> Vec<(String, Duration)> {
        v.iter().map(|(n, ms)| (n.to_string(), Duration::from_millis(*ms))).collect()
    }
    const SETTLE: Duration = Duration::from_secs(1);

    #[test]
    fn the_pieces_up_are_counted_unbroken_from_the_first() {
        let credentials = shroom_storage::sigv4::Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None };
        let s3 = Client::new(shroom_storage::s3::Bucket { endpoint: "http://127.0.0.1:9".into(), region: "auto".into(), name: "b".into(), credentials });
        let mut up = LiveUpload::new(s3, "AbCdEfGhIjKlMnOp", Path::new("/nowhere"));
        assert_eq!(up.pieces_up(), 0);
        for n in [0, 1, 2, 4] {
            up.sent.insert(keys::variant_file("AbCdEfGhIjKlMnOp", &files::segment(n)), 10);
        }
        up.sent.insert(keys::variant_file("AbCdEfGhIjKlMnOp", files::INIT_SEGMENT), 10);
        assert_eq!(up.pieces_up(), 3, "piece 3 isn't up yet: only 0–2 count");
    }

    #[test]
    fn a_settled_segment_is_ready_while_recording() {
        let f = files(&[("init.mp4", 9_000), ("preview_0.mp4", 0), ("seg_00000.m4s", 2_000)]);
        assert_eq!(ready_files(&f, true, SETTLE), vec!["init.mp4", "seg_00000.m4s"]);
    }

    #[test]
    fn a_segment_still_being_written_waits() {
        let f = files(&[("init.mp4", 9_000), ("seg_00000.m4s", 3_000), ("seg_00001.m4s", 100)]);
        assert_eq!(ready_files(&f, true, SETTLE), vec!["init.mp4", "seg_00000.m4s"]);
    }

    #[test]
    fn everything_is_ready_once_no_take_runs() {
        let f = files(&[("init.mp4", 0), ("seg_00000.m4s", 0), ("seg_00001.m4s", 0)]);
        assert_eq!(ready_files(&f, false, SETTLE).len(), 3);
    }

    #[test]
    fn the_preview_and_playlists_are_never_segments() {
        let f = files(&[("preview_0.mp4", 5_000), ("stream_0.m3u8", 5_000)]);
        assert!(ready_files(&f, true, SETTLE).is_empty());
    }
}
