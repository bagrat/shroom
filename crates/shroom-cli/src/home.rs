//! What the command reads from shroom's folder (ARCHITECTURE §3.4). It only reads: the app
//! is the one writer of every file there, so the command never races it.
//!
//! The folder is the one of the app the command ships in (spec N62, DECISIONS B215): from
//! its own path, `…/X.app/Contents/MacOS/shroom-cli`, the bundle's identifier picks it by
//! the app's own rule (`am.shroom` → `~/.shroom`, `am.shroom.dev` → `~/.shroom-dev`), and
//! that identifier is the app it opens to record. Off a bundle (a `cargo` build), the
//! release app's.
//!
//! - `space.json`: which space this Mac is in, and its role;
//! - `device.key`: this Mac's key in that space — every call the CLI makes goes through the
//!   space's own page under it, as a member's Mac does (WORKER-INTERFACE §6.8). The
//!   Cloudflare token (`token.json`) is never read;
//! - `status.json`: the app's public status seam (§11.2), live only while its `pid` is a
//!   running shroom;
//! - `library.json`: the Library's cache, so a list reads only the records that changed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use shroom_cloud::device::DeviceKey;
use shroom_core::world::SpaceIdentity;
use shroom_upload::library::Row;

pub struct Home {
    pub dir: PathBuf,
    /// The app this command ships in, by its bundle identifier: the one it opens to record.
    pub app_id: String,
}

/// The release app's identifier: the command's when it isn't inside a bundle.
const RELEASE: &str = "am.shroom";

impl Home {
    /// The folder of the app this command ships in, or — a debug build only, for tests and
    /// the cloud demo — `--home` or `SHROOM_HOME`.
    pub fn locate(flag: Option<&str>) -> Option<Home> {
        let app_id = std::env::current_exe().ok().and_then(|exe| bundle_id(&exe)).unwrap_or_else(|| RELEASE.to_string());
        if cfg!(debug_assertions)
            && let Some(d) = flag.map(PathBuf::from).or_else(|| std::env::var_os("SHROOM_HOME").map(PathBuf::from))
        {
            return Some(Home { dir: d, app_id });
        }
        let home = std::env::var_os("HOME").map(PathBuf::from).filter(|h| h.is_absolute())?;
        Some(Home { dir: home.join(folder_of(&app_id)), app_id })
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.dir.join(name)).ok()
    }

    fn json<T: serde::de::DeserializeOwned>(&self, name: &str) -> Option<T> {
        serde_json::from_slice(&self.read(name)?).ok()
    }

    pub fn space(&self) -> Option<SpaceIdentity> {
        self.json("space.json")
    }

    /// This Mac's key, only when it's for the space this Mac is in.
    pub fn device_key(&self, space: &SpaceIdentity) -> Option<DeviceKey> {
        self.json::<DeviceKey>("device.key").filter(|k| k.host == space.host)
    }

    /// The app's status, when the app that wrote it is running (a stale file reads as
    /// absent: the app removes it on exit, a crash leaves it behind).
    pub fn app_status(&self) -> Option<Value> {
        let v: Value = self.json("status.json")?;
        let pid = v.get("pid")?.as_u64()?;
        running_shroom(pid).then_some(v)
    }

    /// The Library's cached rows for this space (`library.json`, `{v, space, cache}`):
    /// id → (ETag, row). Another space's cache is never used.
    pub fn cached_rows(&self, space: &SpaceIdentity) -> BTreeMap<String, (String, Row)> {
        #[derive(serde::Deserialize)]
        struct File {
            v: u32,
            space: String,
            cache: shroom_upload::library::Cache,
        }
        self.json::<File>("library.json")
            .filter(|f| f.v == 1 && f.space == format!("{}/{}", space.account_id, space.bucket))
            .map(|f| f.cache.rows)
            .unwrap_or_default()
    }

    /// Where the app listens for the CLI (dev/proposals/cli-agent-skill.md §3).
    pub fn control_socket(&self) -> PathBuf {
        self.dir.join("control.sock")
    }
}

/// The app's own rule (`app/src-tauri/src/runtime/mod.rs`, `Flavor::of`): a dev build's
/// identifier ends in `.dev`, and its folder is `~/.shroom-dev` — never the release app's
/// (B4).
fn folder_of(app_id: &str) -> &'static str {
    if app_id.ends_with(".dev") { ".shroom-dev" } else { ".shroom" }
}

/// The identifier of the bundle `exe` sits in (`<X>.app/Contents/MacOS/<exe>`), links
/// resolved — the install line's `~/.local/bin/shroom` is a link to the one in the bundle.
fn bundle_id(exe: &Path) -> Option<String> {
    let exe = std::fs::canonicalize(exe).ok()?;
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" {
        return None;
    }
    plist_string(&std::fs::read_to_string(contents.join("Info.plist")).ok()?, "CFBundleIdentifier")
}

/// A string value of an XML property list (the bundler writes XML).
fn plist_string(xml: &str, key: &str) -> Option<String> {
    let after = &xml[xml.find(&format!("<key>{key}</key>"))? + key.len() + 11..];
    let start = after.find("<string>")? + "<string>".len();
    // Only the next value: no other key between.
    if after[..start].contains("<key>") {
        return None;
    }
    let len = after[start..].find("</string>")?;
    Some(after[start..start + len].trim().to_string()).filter(|s| !s.is_empty())
}

/// Whether `pid` is a live process named `shroom` (the seam's own rule, §11.2).
fn running_shroom(pid: u64) -> bool {
    let out = std::process::Command::new("ps").args(["-p", &pid.to_string(), "-o", "comm="]).output();
    out.ok().is_some_and(|o| o.status.success() && Path::new(String::from_utf8_lossy(&o.stdout).trim()).file_name().is_some_and(|n| n == "shroom"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Home) {
        let d = tempfile::tempdir().unwrap();
        for (n, body) in files {
            std::fs::write(d.path().join(n), body).unwrap();
        }
        let h = Home { dir: d.path().to_path_buf(), app_id: RELEASE.into() };
        (d, h)
    }

    const SPACE: &str = r#"{"account_id":"acc","label":"v","subdomain":"myshroom","host":"v.myshroom.workers.dev","bucket":"shroom-space","role":"Member","member_id":"m_x","owner_name":"Ann"}"#;

    #[test]
    fn a_device_key_for_another_space_is_never_used() {
        let (_d, h) = home_with(&[("space.json", SPACE), ("device.key", r#"{"v":1,"host":"v.other.workers.dev","device":"d_AAAAAAAAAAAAAAAA","secret":"s"}"#)]);
        let space = h.space().unwrap();
        assert!(h.device_key(&space).is_none());
    }

    #[test]
    fn a_status_file_left_by_a_dead_app_reads_as_no_app() {
        let (_d, h) = home_with(&[("status.json", r#"{"v":2,"pid":999999999,"take_open":false,"take":"none"}"#)]);
        assert!(h.app_status().is_none());
    }

    #[test]
    fn the_identifier_is_read_from_the_bundles_property_list() {
        let xml = "<dict>\n\t<key>CFBundleExecutable</key>\n\t<string>shroom</string>\n\t<key>CFBundleIdentifier</key>\n\t<string>am.shroom.dev</string>\n</dict>";
        assert_eq!(plist_string(xml, "CFBundleIdentifier").as_deref(), Some("am.shroom.dev"));
        assert_eq!(plist_string(xml, "CFBundleName"), None);
        assert_eq!(plist_string("<key>CFBundleIdentifier</key><true/><key>X</key><string>no</string>", "CFBundleIdentifier"), None, "only its own value");
        assert_eq!((folder_of("am.shroom"), folder_of("am.shroom.dev"), folder_of("am.shroom.preview.dev")), (".shroom", ".shroom-dev", ".shroom-dev"));
    }

    #[test]
    fn another_spaces_library_cache_is_ignored() {
        let (_d, h) = home_with(&[("space.json", SPACE), ("library.json", r#"{"v":1,"space":"other/shroom-space","cache":{"rows":{}}}"#)]);
        assert!(h.cached_rows(&h.space().unwrap()).is_empty());
    }
}
