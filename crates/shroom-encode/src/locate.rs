//! Which ffmpeg the app runs (DECISIONS B82, amending B12).
//!
//! A release carries its own encoder — a minimal LGPL FFmpeg built by
//! `tools/build-ffmpeg.sh`, bundled as `Contents/MacOS/ffmpeg` beside the app's own binary
//! (Tauri's `externalBin`, signed with the bundle) — and runs **only** that one: a clean Mac
//! has no Homebrew, and a release that quietly preferred some other ffmpeg on the machine
//! would record differently from the one that was tested.
//!
//! A dev build keeps B12's road: `SHROOM_FFMPEG` when set (the media tests and the release
//! proof point it at the built sidecar), then a sidecar beside the binary if the build
//! carries one, then the Homebrew ffmpeg at its usual paths (LaunchServices starts apps with
//! a bare `PATH`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Names the encoder in a dev build and in the media tests.
pub const ENV: &str = "SHROOM_FFMPEG";

/// The sidecar's file name in the bundle's `Contents/MacOS` (Tauri strips the target triple
/// from `externalBin` names when it bundles them).
pub const SIDECAR: &str = "ffmpeg";

/// Where a dev build looks after `SHROOM_FFMPEG` and its own sidecar.
pub const DEV_FALLBACKS: [&str; 2] = ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"];

/// Where the encoder that was found came from — logged by word, never by path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The sidecar beside the app's binary.
    Bundled,
    /// `SHROOM_FFMPEG` (dev builds only).
    Override,
    /// A Homebrew ffmpeg (dev builds only).
    Homebrew,
}

impl Source {
    pub fn word(self) -> &'static str {
        match self {
            Source::Bundled => "bundled",
            Source::Override => "override",
            Source::Homebrew => "homebrew",
        }
    }
}

/// The places to look, in order. `exe_dir` is the folder holding the running binary
/// (`Contents/MacOS` in a bundle); `env` is `SHROOM_FFMPEG`'s value. A release looks
/// beside itself and nowhere else.
pub fn candidates(exe_dir: Option<&Path>, env: Option<&OsStr>, dev: bool) -> Vec<(Source, PathBuf)> {
    let mut out = Vec::new();
    if dev {
        if let Some(p) = env.filter(|p| !p.is_empty()) {
            out.push((Source::Override, PathBuf::from(p)));
        }
    }
    if let Some(dir) = exe_dir {
        out.push((Source::Bundled, dir.join(SIDECAR)));
    }
    if dev {
        out.extend(DEV_FALLBACKS.iter().map(|p| (Source::Homebrew, PathBuf::from(p))));
    }
    out
}

/// The first candidate that is a file.
pub fn locate(exe_dir: Option<&Path>, env: Option<&OsStr>, dev: bool) -> Option<(Source, PathBuf)> {
    candidates(exe_dir, env, dev).into_iter().find(|(_, p)| p.is_file())
}

/// The encoder the media tests run: `SHROOM_FFMPEG` when set (the release's sidecar, to
/// prove it), else the ffmpeg on `PATH`. Decoding and probing in those tests stay with the
/// `PATH` ffmpeg/ffprobe — an independent oracle. Panics, naming the fix, when there's none:
/// a media test never passes without an encoder.
pub fn under_test() -> PathBuf {
    if let Some(p) = std::env::var_os(ENV).filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "{ENV} names {} — not a file", p.display());
        return p;
    }
    let out = std::process::Command::new("sh").args(["-c", "command -v ffmpeg"]).output().expect("sh runs");
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!p.is_empty(), "no encoder: set {ENV} or put ffmpeg on PATH (brew install ffmpeg)");
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(c: Vec<(Source, PathBuf)>) -> Vec<String> {
        c.into_iter().map(|(s, p)| format!("{}:{}", s.word(), p.display())).collect()
    }

    #[test]
    fn a_release_runs_only_its_bundled_encoder() {
        let exe = Path::new("/Applications/shroom.app/Contents/MacOS");
        let got = paths(candidates(Some(exe), Some(OsStr::new("/tmp/other-ffmpeg")), false));
        assert_eq!(got, ["bundled:/Applications/shroom.app/Contents/MacOS/ffmpeg"], "no override, no Homebrew");
        assert!(candidates(None, None, false).is_empty(), "nothing to fall back on");
    }

    #[test]
    fn a_dev_build_takes_the_override_then_its_sidecar_then_homebrew() {
        let exe = Path::new("/x/shroom-dev.app/Contents/MacOS");
        let got = paths(candidates(Some(exe), Some(OsStr::new("/b/ffmpeg-aarch64-apple-darwin")), true));
        assert_eq!(
            got,
            [
                "override:/b/ffmpeg-aarch64-apple-darwin",
                "bundled:/x/shroom-dev.app/Contents/MacOS/ffmpeg",
                "homebrew:/opt/homebrew/bin/ffmpeg",
                "homebrew:/usr/local/bin/ffmpeg",
            ]
        );
        assert_eq!(paths(candidates(None, Some(OsStr::new("")), true)), ["homebrew:/opt/homebrew/bin/ffmpeg", "homebrew:/usr/local/bin/ffmpeg"], "an empty override is none");
    }

    #[test]
    fn the_first_file_wins_and_a_missing_one_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(locate(Some(dir.path()), None, false), None, "a release with no sidecar has no encoder");
        std::fs::write(dir.path().join(SIDECAR), b"").unwrap();
        assert_eq!(locate(Some(dir.path()), None, false), Some((Source::Bundled, dir.path().join(SIDECAR))));
        let gone = dir.path().join("nothing-here");
        assert_eq!(locate(Some(dir.path()), Some(gone.as_os_str()), true), Some((Source::Bundled, dir.path().join(SIDECAR))), "a dev override that isn't a file is passed over");
    }
}
