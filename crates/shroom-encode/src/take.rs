//! One take: an ffmpeg process fed BGRA frames on stdin.
//!
//! The process runs in its own process group so a kill takes any children with it
//! (POC §9: kill trees, not processes). Stopping is EOF on stdin — ffmpeg then
//! finalizes both tee outputs (probe: finalize ~100 ms).

use std::fs::File;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::args::{TakeSpec, ffmpeg_args};
use crate::files;

#[derive(Debug)]
pub enum TakeError {
    Io(io::Error),
    /// ffmpeg exited with a failure; `log` is its stderr.
    Exited { status: ExitStatus, log: String },
    /// ffmpeg didn't finish within the grace period after EOF and was killed.
    Hung { log: String },
    /// ffmpeg exited but the take's outputs are missing or empty (the POC's wedge).
    NoOutput { log: String },
}

impl From<io::Error> for TakeError {
    fn from(e: io::Error) -> Self {
        TakeError::Io(e)
    }
}

impl std::fmt::Display for TakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TakeError::Io(e) => write!(f, "io: {e}"),
            TakeError::Exited { status, log } => write!(f, "ffmpeg exited {status}: {log}"),
            TakeError::Hung { .. } => write!(f, "ffmpeg did not finish after its input closed"),
            TakeError::NoOutput { log } => write!(f, "take produced no output: {log}"),
        }
    }
}

impl std::error::Error for TakeError {}

/// What a finished take left on disk.
#[derive(Debug, Clone)]
pub struct TakeOutput {
    pub take: u32,
    pub playlist: PathBuf,
    pub preview: PathBuf,
    pub frames_written: u64,
}

pub struct Take {
    child: Child,
    stdin: Option<ChildStdin>,
    dir: PathBuf,
    spec: TakeSpec,
    log: PathBuf,
    frames_written: u64,
    frame_bytes: usize,
}

impl Take {
    pub fn start(ffmpeg: &Path, dir: &Path, spec: TakeSpec) -> io::Result<Take> {
        let log = dir.join(format!("ffmpeg_{}.log", spec.take));
        let mut cmd = Command::new(ffmpeg);
        cmd.args(ffmpeg_args(&spec))
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(File::create(&log)?);
        // Own process group, so `kill` reaches everything ffmpeg started.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
            });
        }
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take();
        let frame_bytes = (spec.width * spec.height * 4) as usize;
        Ok(Take { child, stdin, dir: dir.to_path_buf(), spec, log, frames_written: 0, frame_bytes })
    }

    pub fn spec(&self) -> &TakeSpec {
        &self.spec
    }

    pub fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    /// Writes one frame: exactly `width × height × 4` bytes, tightly packed BGRA.
    pub fn write_frame(&mut self, bgra: &[u8]) -> io::Result<()> {
        if bgra.len() != self.frame_bytes {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "frame has the wrong size"));
        }
        let stdin = self.stdin.as_mut().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "take finished"))?;
        stdin.write_all(bgra)?;
        self.frames_written += 1;
        Ok(())
    }

    pub fn frames_written(&self) -> u64 {
        self.frames_written
    }

    /// Ends the frames: EOF on ffmpeg's stdin, so it flushes the picture while the sound
    /// (another input) may still be arriving. `finish` then waits for it.
    pub fn close_input(&mut self) {
        drop(self.stdin.take());
    }

    /// Closes the input and waits for ffmpeg to finalize; kills it after `grace`.
    pub fn finish(mut self, grace: Duration) -> Result<TakeOutput, TakeError> {
        self.close_input();
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait()? {
                break status;
            }
            if started.elapsed() > grace {
                self.kill();
                return Err(TakeError::Hung { log: self.read_log() });
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !status.success() {
            return Err(TakeError::Exited { status, log: self.read_log() });
        }
        let playlist = self.dir.join(files::take_playlist(self.spec.take));
        let preview = self.dir.join(files::take_preview(self.spec.take));
        let init = self.dir.join(files::take_init(self.spec.take));
        let nonempty = |p: &Path| p.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if !(nonempty(&playlist) && nonempty(&preview) && nonempty(&init)) {
            return Err(TakeError::NoOutput { log: self.read_log() });
        }
        Ok(TakeOutput { take: self.spec.take, playlist, preview, frames_written: self.frames_written })
    }

    /// Kills ffmpeg's whole process group.
    pub fn kill(&mut self) {
        let pid = self.child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = self.child.wait();
    }

    fn read_log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }
}

impl Drop for Take {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            self.kill();
        }
    }
}

/// After a later take ends: its own init segment (`files::take_init`) byte-identical to the
/// recording's shared one — the norm, B11 — is discarded, and its playlist names the shared
/// one; a different one (the picture changed size between takes) stays, for the joined
/// playlist to name for that take. Returns the init the take plays under.
pub fn settle_init(dir: &Path, take: u32) -> io::Result<String> {
    if take == 0 {
        return Ok(files::INIT_SEGMENT.to_string());
    }
    let own = files::take_init(take);
    let shared = std::fs::read(dir.join(files::INIT_SEGMENT))?;
    let mine = std::fs::read(dir.join(&own))?;
    if mine.is_empty() {
        std::fs::remove_file(dir.join(&own))?;
        return Ok(files::INIT_SEGMENT.to_string());
    }
    if mine != shared {
        return Ok(own);
    }
    let playlist = dir.join(files::take_playlist(take));
    if let Ok(text) = std::fs::read_to_string(&playlist) {
        let fixed = text.replace(&format!("URI=\"{own}\""), &format!("URI=\"{}\"", files::INIT_SEGMENT));
        if fixed != text {
            write_atomically(&playlist, fixed.as_bytes())?;
        }
    }
    std::fs::remove_file(dir.join(&own))?;
    Ok(files::INIT_SEGMENT.to_string())
}

/// The init segment a take plays under, as the disk says now: its own when one is there,
/// whole, and different from the shared one; the shared one otherwise (a take settled, or
/// one that died before writing its own).
pub fn take_map(dir: &Path, take: u32) -> String {
    if take > 0 {
        let own = files::take_init(take);
        if let Ok(mine) = std::fs::read(dir.join(&own)) {
            if !mine.is_empty() && std::fs::read(dir.join(files::INIT_SEGMENT)).map(|s| s != mine).unwrap_or(true) {
                return own;
            }
        }
    }
    files::INIT_SEGMENT.to_string()
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("m3u8.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The highest global segment index on disk, so the next take continues after it.
pub fn next_start_number(dir: &Path) -> io::Result<u32> {
    let mut next = 0;
    for entry in std::fs::read_dir(dir)? {
        if let Some(i) = entry?.file_name().to_str().and_then(files::segment_index) {
            next = next.max(i + 1);
        }
    }
    Ok(next)
}

/// Joins per-take previews (fragmented while recording) into a faststart `preview.mp4`:
/// one take is remuxed, more are concatenated (stream copy, re-timestamped joins). Never
/// on the Stop path — it's built before its own upload.
pub fn assemble_preview(ffmpeg: &Path, dir: &Path, takes: &[u32]) -> io::Result<PathBuf> {
    let present: Vec<u32> = takes
        .iter()
        .copied()
        .filter(|k| dir.join(files::take_preview(*k)).metadata().map(|m| m.len() > 0).unwrap_or(false))
        .collect();
    if present.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no take previews"));
    }
    let list = dir.join("preview_concat.txt");
    let body: String = present.iter().map(|k| format!("file '{}'\n", files::take_preview(*k))).collect();
    std::fs::write(&list, body)?;
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i", "preview_concat.txt"])
        .args(["-c", "copy", "-movflags", "+faststart", files::PREVIEW])
        .current_dir(dir)
        .stdin(Stdio::null())
        .status()?;
    let _ = std::fs::remove_file(list);
    if status.success() { Ok(dir.join(files::PREVIEW)) } else { Err(io::Error::other(format!("preview assembly exited {status}"))) }
}

/// How long the unfurl teaser runs.
pub const TEASER_SECS: u32 = 15;

/// The unfurl poster from the recording's first frame (og:image, R5.3), from the
/// single-file copy or, before it exists, the first take's preview.
pub fn poster(ffmpeg: &Path, dir: &Path) -> io::Result<PathBuf> {
    let source = if dir.join(files::PREVIEW).is_file() { files::PREVIEW.to_string() } else { files::take_preview(0) };
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i", &source])
        .args(["-frames:v", "1", "-vf", "scale=1200:-1", "-q:v", "3", files::POSTER])
        .current_dir(dir)
        .stdin(Stdio::null())
        .status()?;
    if status.success() { Ok(dir.join(files::POSTER)) } else { Err(io::Error::other(format!("poster exited {status}"))) }
}

/// The unfurl teaser (og:video): the first [`TEASER_SECS`] at 640 wide, small and
/// faststart, from the single-file copy (R5.3's option 2).
pub fn teaser(ffmpeg: &Path, dir: &Path) -> io::Result<PathBuf> {
    let secs = TEASER_SECS.to_string();
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i", files::PREVIEW, "-t", &secs])
        .args(["-vf", "scale=640:-2", "-c:v", "h264_videotoolbox", "-b:v", "700k", "-pix_fmt", "yuv420p"])
        .args(["-c:a", "aac", "-b:a", "64k", "-ac", "1", "-movflags", "+faststart", files::TEASER])
        .current_dir(dir)
        .stdin(Stdio::null())
        .status()?;
    if status.success() { Ok(dir.join(files::TEASER)) } else { Err(io::Error::other(format!("teaser exited {status}"))) }
}

/// The takes whose previews are on disk, in order.
pub fn take_numbers(dir: &Path) -> Vec<u32> {
    (0..).take_while(|k| dir.join(files::take_preview(*k)).exists()).collect()
}

/// Whether `path` is a single-file copy a player can open: ffmpeg reads its first video
/// frame. A missing, empty or cut-short file (a crash mid-assembly) is not.
pub fn playable(ffmpeg: &Path, path: &Path) -> bool {
    if path.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        return false;
    }
    Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-xerror", "-i"])
        .arg(path)
        .args(["-map", "0:v:0", "-frames:v", "1", "-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The recovery's remux: the take's HLS (`stream.m3u8` — the whole recording, one init
/// segment) as one faststart `preview.mp4`, for a take whose per-take previews are gone or
/// unreadable. Stream copy only.
pub fn remux_playlist(ffmpeg: &Path, dir: &Path) -> io::Result<PathBuf> {
    if !dir.join(files::PLAYLIST).is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no playlist"));
    }
    let status = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-allowed_extensions", "ALL", "-i", files::PLAYLIST])
        .args(["-c", "copy", "-movflags", "+faststart", files::PREVIEW])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() { Ok(dir.join(files::PREVIEW)) } else { Err(io::Error::other(format!("playlist remux exited {status}"))) }
}

/// The take's single-file copy, made if it isn't there or isn't playable: the per-take
/// previews joined, else the recording's HLS remuxed (Save to Movies on a take the
/// uploader never assembled — a stranded or refused take, N34).
pub fn ensure_preview(ffmpeg: &Path, dir: &Path) -> io::Result<PathBuf> {
    let preview = dir.join(files::PREVIEW);
    if playable(ffmpeg, &preview) {
        return Ok(preview);
    }
    let takes = take_numbers(dir);
    if !takes.is_empty() && assemble_preview(ffmpeg, dir, &takes).is_ok() && playable(ffmpeg, &preview) {
        return Ok(preview);
    }
    if remux_playlist(ffmpeg, dir).is_ok() && playable(ffmpeg, &preview) {
        return Ok(preview);
    }
    let _ = std::fs::remove_file(&preview);
    Err(io::Error::other("no single-file copy could be made from this take"))
}
