//! The updater lane (U7, N37; `shroom-update`, B31): the background check against the
//! project's public releases, the download verified exactly as the installer verifies,
//! and Update's quit → swap → relaunch. Every command answers the core exactly once: its
//! news, or `Failed` with its class — a failure never breaks anything (the running version
//! stays; the next try is hours away).
//!
//! Dev builds never reach the real releases and never swap an installed app: a check goes
//! only to the loopback release stand-in named by `--releases`, and Update swaps only the
//! throwaway copy named by `--update-target` (from a `--home` throwaway). Its relaunch
//! carries the throwaway home with it — a note beside the copy that only a dev build
//! reads — so the reopened copy never runs on the real dev home.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};

use shroom_core::effect::UpdCmd;
use shroom_core::event::{Event, UpdateNews};
use shroom_core::world::{FailClass, Op, UpdateOffer};
use shroom_update::{Channel, Offer, Release, UpdateError, Version};
use tauri::AppHandle;

use super::Runtime;

/// The reopen's provenance for each outcome of the swap (`launch_kind` reads them).
pub const RELAUNCHED_OK: &str = "--relaunched=update";
pub const RELAUNCHED_FAILED: &str = "--relaunched=update-failed";

/// What `uname -m` calls this Mac (the manifest's architecture keys).
fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    }
}

/// Where releases are read from: the public releases (a release build), or only the
/// loopback stand-in a dev build was given.
fn base(rt: &Runtime) -> Option<String> {
    releases_base(rt.dev, rt.releases.as_deref())
}

fn releases_base(dev: bool, stand_in: Option<&str>) -> Option<String> {
    if dev {
        stand_in.filter(|r| super::loopback(r)).map(str::to_string)
    } else {
        Some(shroom_update::RELEASES.to_string())
    }
}

/// This Mac's channel (DECISIONS B220): `early` in the folder's `channel` file (written by
/// `install.sh --early`), else the official one. Read at every check, so changing the file
/// needs no restart.
pub fn mac_channel(rt: &Runtime) -> Channel {
    Channel::from_file(std::fs::read_to_string(rt.home.path("channel")).ok().as_deref())
}

fn class(e: &UpdateError) -> FailClass {
    match e {
        UpdateError::Transport(_) => FailClass::Transport,
        // A fingerprint that doesn't match is discarded, never installed (the installer's law).
        UpdateError::Mismatch => FailClass::Refused,
        // Nor is a copy someone other than this app's own signer made (B125).
        UpdateError::Signature(_) => FailClass::Refused,
        _ => FailClass::Unclassified,
    }
}

/// The dev relaunch note beside a throwaway update target.
pub fn dev_note(target: &Path) -> PathBuf {
    PathBuf::from(format!("{}.dev-relaunch.json", target.display()))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DevNote {
    pub v: u32,
    pub args: Vec<String>,
}

pub fn spawn(rt: Runtime, app: AppHandle) -> Sender<UpdCmd> {
    let (tx, rx) = channel::<UpdCmd>();
    std::thread::Builder::new()
        .name("shroom-updater".into())
        .spawn(move || {
            // A staged copy outlives nothing: what an earlier run left is gone (the swap
            // moved the one it applied).
            let _ = std::fs::remove_dir_all(rt.home.updates());
            let mut offered: Option<Release> = None;
            for cmd in rx {
                let (op, result) = match cmd {
                    UpdCmd::Check => (Op::UpdateCheck, check(&rt, &mut offered)),
                    UpdCmd::Download(version) => (Op::UpdateDownload, download(&rt, &mut offered, &version)),
                    UpdCmd::Apply(version) => (Op::UpdateApply, apply(&rt, &app, &version)),
                };
                if let Err(class) = result {
                    rt.log(&format!("updater {op:?}: failed {class:?}"));
                    rt.post(Event::Failed { op, class });
                }
            }
        })
        .expect("the updater lane starts");
    tx
}

fn running(rt: &Runtime) -> Result<Version, FailClass> {
    Version::parse(&rt.version).ok_or(FailClass::Unclassified)
}

fn check(rt: &Runtime, offered: &mut Option<Release>) -> Result<(), FailClass> {
    let Some(base) = base(rt) else {
        rt.log("updater: no releases to check (a dev build checks only a --releases stand-in)");
        return Err(FailClass::Refused);
    };
    let manifest = shroom_update::check_channel(&base, mac_channel(rt)).map_err(|e| {
        rt.log(&format!("updater check: {e}"));
        class(&e)
    })?;
    let offer = match shroom_update::offer(&manifest, running(rt)?, arch(), super::os::macos_major()) {
        Offer::Nothing => None,
        Offer::Ready(r) => {
            let version = r.version.to_string();
            *offered = Some(r);
            Some(UpdateOffer::Ready { version })
        }
        Offer::NeedsMacos { version, macos } => Some(UpdateOffer::NeedsMacos { version: version.to_string(), macos }),
    };
    rt.log(&format!("updater check: {} running, {} channel, {:?}", rt.version, mac_channel(rt).word(), offer));
    rt.post(Event::Update(UpdateNews::Checked(offer)));
    Ok(())
}

fn download(rt: &Runtime, offered: &mut Option<Release>, version: &str) -> Result<(), FailClass> {
    let base = base(rt).ok_or(FailClass::Refused)?;
    // The offer this lane read; else read the manifest again (a relaunch since the check).
    if offered.as_ref().is_none_or(|r| r.version.to_string() != version) {
        let manifest = shroom_update::check_channel(&base, mac_channel(rt)).map_err(|e| class(&e))?;
        *offered = match shroom_update::offer(&manifest, running(rt)?, arch(), super::os::macos_major()) {
            Offer::Ready(r) => Some(r),
            _ => None,
        };
    }
    let release = offered.clone().filter(|r| r.version.to_string() == version).ok_or(FailClass::Refused)?;
    let staging = rt.home.updates();
    std::fs::create_dir_all(&staging).map_err(|_| FailClass::Unclassified)?;
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o700));
    }
    let app = shroom_update::download(&base, &release, &staging).map_err(|e| {
        rt.log(&format!("updater download {version}: {e}"));
        class(&e)
    })?;
    // Only the newest is ever kept (versions roll up).
    for entry in std::fs::read_dir(&staging).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy() != version {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    rt.log(&format!("updater staged {version} ({})", app.display()));
    rt.post(Event::Update(UpdateNews::Staged(version.to_string())));
    Ok(())
}

/// The bundle Update replaces: the running app (a release build), or only a dev build's
/// throwaway `--update-target`.
fn target(rt: &Runtime) -> Result<PathBuf, FailClass> {
    if rt.dev {
        // Checked again here, at the moment of the swap, by identity: a throwaway `.app` in a
        // temp folder, from a throwaway home — resolved, so no alias reaches an installed app.
        let t = rt.update_target.as_deref().and_then(super::throwaway_app).filter(|_| rt.throwaway);
        let Some(t) = t else {
            rt.log("updater would swap the app (a dev build swaps only a --update-target throwaway)");
            return Err(FailClass::Refused);
        };
        // Never the bundle this process runs from (whatever path names it).
        if super::os::bundle().and_then(|b| super::resolved(&b)).is_some_and(|b| b == t) {
            rt.log("updater: the update target is the running bundle — refused");
            return Err(FailClass::Refused);
        }
        return Ok(t);
    }
    super::os::bundle().ok_or_else(|| {
        rt.log("updater: not running from an .app — nothing to swap");
        FailClass::Refused
    })
}

/// Quit → swap → relaunch: the detached helper waits for this process to be gone, swaps
/// the staged copy in (the old one back if that fails), and reopens with the outcome.
fn apply(rt: &Runtime, app: &AppHandle, version: &str) -> Result<(), FailClass> {
    let staged = rt.home.updates().join(version).join("shroom.app");
    if !staged.is_dir() {
        rt.log(&format!("updater: {version} isn't staged"));
        return Err(FailClass::Refused);
    }
    let target = target(rt)?;
    // Only a copy signed as the app it replaces is ever swapped in (DECISIONS B125): a
    // release someone else published, fingerprint and all, stays staged and unused.
    if let Err(e) = shroom_update::same_signer(&staged, &target) {
        rt.log(&format!("updater not applied {version}: {e}"));
        let _ = std::fs::remove_dir_all(rt.home.updates().join(version));
        return Err(class(&e));
    }
    if rt.dev {
        // The reopened copy carries the throwaway home (and the fake, and a drive's second
        // half) — it never runs on the real dev home.
        let mut args = Vec::new();
        if let Some(h) = &rt.flags.home {
            args.extend(["--home".to_string(), h.display().to_string()]);
        }
        if let Some((api, r2)) = &rt.cloud_fake {
            args.extend(["--cloud-fake".to_string(), api.clone(), r2.clone()]);
        }
        if let Some(r) = &rt.releases {
            args.extend(["--releases".to_string(), r.clone()]);
        }
        args.extend(rt.relaunch_extra.lock().unwrap().iter().cloned());
        super::home::write_json(&dev_note(&target), &DevNote { v: 1, args }).map_err(|_| FailClass::Unclassified)?;
    }
    shroom_update::spawn_swap(std::process::id(), &staged, &target, RELAUNCHED_OK, RELAUNCHED_FAILED).map_err(|e| {
        rt.log(&format!("updater: the swap helper didn't start: {e}"));
        FailClass::Unclassified
    })?;
    rt.log(&format!("updater applies {version}: quitting for the swap"));
    rt.exit(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A release reads only the public releases; a dev build only the loopback stand-in it
    /// was given.
    #[test]
    fn each_flavor_reads_releases_from_its_own_place() {
        assert_eq!(releases_base(false, None).as_deref(), Some(shroom_update::RELEASES));
        assert_eq!(releases_base(false, Some("http://127.0.0.1:9/rel")).as_deref(), Some(shroom_update::RELEASES), "a release never takes a stand-in");
        assert_eq!(releases_base(true, None), None, "a dev build checks nothing by itself");
        assert_eq!(releases_base(true, Some("http://127.0.0.1:9/rel")).as_deref(), Some("http://127.0.0.1:9/rel"));
        assert_eq!(releases_base(true, Some("https://github.com/bagrat/shroom/releases")), None);
        // What the runtime hands it: a dev runtime, then a release one.
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        assert_eq!(base(&rt), None);
        rt.dev = false;
        assert_eq!(base(&rt).as_deref(), Some(shroom_update::RELEASES));
    }

    /// A staged copy that isn't the version offered, newer, of this channel, is refused like
    /// one signed by someone else — and a dev build's stand-in may serve an early build to a
    /// Mac on the official channel (the tree's own version), a release build's never.
    #[test]
    fn a_staged_copy_of_another_version_is_refused() {
        assert_eq!(class(&UpdateError::Staged("0.1.24, not 0.1.26".into())), FailClass::Refused);
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        assert_eq!(staged_channel(&rt), Channel::Early);
        rt.dev = false;
        assert_eq!(staged_channel(&rt), Channel::Official);
        std::fs::write(rt.home.path("channel"), "early\n").unwrap();
        assert_eq!(staged_channel(&rt), Channel::Early);
    }

    /// The channel is the folder's `channel` file's (B220), read at each check.
    #[test]
    fn the_channel_is_the_folders_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        assert_eq!(mac_channel(&rt), Channel::Official);
        std::fs::write(rt.home.path("channel"), "early\n").unwrap();
        assert_eq!(mac_channel(&rt), Channel::Early);
        std::fs::remove_file(rt.home.path("channel")).unwrap();
        assert_eq!(mac_channel(&rt), Channel::Official);
    }

    /// A release build's Update swaps the app it runs from — the dev build's throwaway-only
    /// rule isn't its (run from a test binary there is no bundle, so both refuse — but for
    /// different reasons, and only the dev one asks for a target).
    #[test]
    fn a_release_build_swaps_the_app_it_runs_from() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        rt.dev = false;
        rt.throwaway = false;
        assert_eq!(target(&rt).ok(), crate::runtime::os::bundle());
        let log = std::fs::read_to_string(rt.home.root.join("logs/run.log")).unwrap_or_default();
        assert!(!log.contains("--update-target"), "{log}");
    }
}
