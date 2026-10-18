//! `~/.shroom` (dev builds: `~/.shroom-dev`):
//! directory 700, every file 600, written
//! atomically (temp, fsync, rename). The core reads files only through `Booted` and
//! `Scanned`, and writes only through effects (ARCHITECTURE §3.4).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};

/// The only file names this app writes at the top of its home (a test can whitelist).
pub const FILES: &[&str] = &[
    "prefs.json",
    "once.json",
    "space.json",
    "token.json",
    "device.key",
    "primed.json",
    // macOS was asked for screen recording once on this Mac (N43, DECISIONS B146): later
    // asks open System Settings themselves.
    "screen-asked.json",
    "status.json",
    "restore.json",
    // The Library's cache of the space's records (ETags and rows — no secret).
    "library.json",
    // An invite this Mac holds (WORKER-INTERFACE §5): the installer's hand-off, the deep
    // link or the paste door — normalized into the app's shape; removed once it's behind us.
    "invite",
    // A join asked for, written before the call (§5): replayed identically at launch.
    "join.json",
];

#[derive(Debug, Clone)]
pub struct Home {
    pub root: PathBuf,
}

impl Home {
    /// `~/.shroom` for the app, `~/.shroom-dev` for dev builds — never mixed (B4). The person's home folder as the system knows
    /// it (`user_home`): never a shared place.
    pub fn for_identifier(identifier: &str) -> std::io::Result<Home> {
        let home = user_home().ok_or_else(|| std::io::Error::other("this account has no home folder"))?;
        Ok(Home::for_identifier_in(identifier, &home))
    }

    /// The build's home inside the home folder `home`.
    pub fn for_identifier_in(identifier: &str, home: &Path) -> Home {
        Home { root: home.join(super::Flavor::of(identifier).home_dir()) }
    }

    /// A home somewhere else — the dev-only `--home <dir>` for automated runs, so a test
    /// never touches the real `~/.shroom` or `~/.shroom-dev`.
    pub fn at(root: PathBuf) -> Home {
        Home { root }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.takes())?;
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(self.takes(), fs::Permissions::from_mode(0o700))
    }

    /// The home is this account's own and private (law 13): `ensure` ran, and the folder is
    /// a real directory (not a link) owned by this user, mode 700 — anything else is fatal
    /// at startup, never a home shared with someone else.
    pub fn ensure_private(&self) -> std::io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        if !self.root.is_absolute() {
            return Err(std::io::Error::other("the home isn't an absolute path"));
        }
        self.ensure()?;
        let m = fs::symlink_metadata(&self.root)?;
        let uid = unsafe { libc::getuid() };
        if !m.is_dir() || m.uid() != uid || m.mode() & 0o777 != 0o700 {
            return Err(std::io::Error::other(format!("{} isn't this account's private folder", self.root.display())));
        }
        Ok(())
    }

    pub fn takes(&self) -> PathBuf {
        self.root.join("takes")
    }

    /// Discards whose remote delete is still owed: `forget/<rec>`, empty files the uploader
    /// retries until the delete lands (N23).
    pub fn forgets(&self) -> PathBuf {
        self.root.join("forget")
    }

    /// The editor's folder (N63): per recording, the video fetched from the space
    /// (`source.mp4`), its sound read (`peaks.u8`), and a permanent copy while it's made.
    /// Disconnect and Leave remove it with the space's other traces.
    pub fn edits(&self) -> PathBuf {
        self.root.join("edits")
    }

    /// Where verified updates wait to be swapped in: `updates/<version>/shroom.app`.
    pub fn updates(&self) -> PathBuf {
        self.root.join("updates")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn read_json<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        read_json(&self.path(name))
    }

    pub fn write_json<T: Serialize>(&self, name: &str, value: &T) -> std::io::Result<()> {
        debug_assert!(FILES.contains(&name), "{name} isn't a file this app writes");
        write_json(&self.path(name), value)
    }

    /// Removes one of this app's files (a missing file is fine).
    pub fn remove(&self, name: &str) {
        debug_assert!(FILES.contains(&name), "{name} isn't a file this app writes");
        let _ = fs::remove_file(self.path(name));
    }

    /// The token the person pasted (`token.json`) and what was saved with it — the
    /// wizard's only token source.
    pub fn saved_token(&self) -> Option<SavedToken> {
        self.read_json::<SavedToken>("token.json")
    }

    /// The pasted token, if this Mac holds one — in every home, the dev build's too (its
    /// old fallback, the main-account test token, retired with main-account testing: D3).
    pub fn token(&self) -> Option<String> {
        #[derive(serde::Deserialize)]
        struct TokenFile {
            token: String,
        }
        self.read_json::<TokenFile>("token.json").map(|t| t.token)
    }
}

/// `token.json`: `{v, token, account_id, several}`.
#[derive(serde::Deserialize)]
pub struct SavedToken {
    pub token: String,
    pub account_id: Option<String>,
    #[serde(default)]
    pub several: bool,
}

/// This account's home folder: `$HOME` when it's set to an absolute path, else the
/// system's record for this user (`getpwuid_r`) — never a shared fallback like `/tmp`.
pub fn user_home() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("HOME").map(PathBuf::from).filter(|h| h.is_absolute()) {
        return Some(h);
    }
    passwd_home(unsafe { libc::getuid() })
}

/// The home folder the system records for `uid`.
pub fn passwd_home(uid: libc::uid_t) -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0 as libc::c_char; 16 * 1024];
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    let rc = unsafe { libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buf.len(), &mut out) };
    if rc != 0 || out.is_null() || pwd.pw_dir.is_null() {
        return None;
    }
    let dir = unsafe { CStr::from_ptr(pwd.pw_dir) };
    let p = PathBuf::from(std::ffi::OsStr::from_bytes(dir.to_bytes()));
    p.is_absolute().then_some(p)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// Atomic, mode-600 write: a reader sees the old file or the new one, never half of one.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f: File = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_folder_comes_from_the_system_never_tmp() {
        let mine = passwd_home(unsafe { libc::getuid() }).expect("this account has a home");
        assert!(mine.is_absolute() && !mine.starts_with("/tmp") && !mine.starts_with("/private/tmp"), "{mine:?}");
        assert!(Home::for_identifier("am.shroom.dev").unwrap().root.ends_with(".shroom-dev"));
    }

    /// The identifier alone picks the folder: `~/.shroom` for the release, `~/.shroom-dev`
    /// for a dev build, and nothing else — the retired preview's name (B148) is the release's.
    #[test]
    fn the_identifier_alone_picks_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        for (id, dir) in [("am.shroom", ".shroom"), ("am.shroom.dev", ".shroom-dev"), ("am.shroom.preview", ".shroom"), ("am.shroom.preview.dev", ".shroom-dev")] {
            assert_eq!(Home::for_identifier_in(id, tmp.path()).root, tmp.path().join(dir), "{id}");
        }
    }

    #[test]
    fn a_home_that_isnt_private_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let h = Home::at(tmp.path().join("h"));
        h.ensure_private().expect("a fresh folder is made private");
        assert_eq!(fs::metadata(&h.root).unwrap().permissions().mode() & 0o777, 0o700);
        // A link in its place (someone else's folder, say) is never taken as the home.
        let other = tmp.path().join("other");
        fs::create_dir(&other).unwrap();
        let link = tmp.path().join("linked");
        std::os::unix::fs::symlink(&other, &link).unwrap();
        assert!(Home::at(link).ensure_private().is_err());
        assert!(Home::at(PathBuf::from("relative/home")).ensure_private().is_err());
    }

    #[test]
    fn only_a_pasted_token_counts_never_the_harness_file() {
        // The old harness file retired with D3's main-account testing: in the dev home a
        // token file other than a pasted one is never taken.
        let tmp = tempfile::tempdir().unwrap();
        let dev = Home::at(tmp.path().join(".shroom-dev"));
        dev.ensure().unwrap();
        fs::write(dev.path("test-account-token"), "harness").unwrap();
        assert_eq!(dev.token(), None, "only a pasted token counts, in the dev home too");
    }
}
