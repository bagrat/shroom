//! The runner (ARCHITECTURE §3.3): one core thread owns the World — events in, `update`,
//! `derive`, present, dispatch effects to lanes whose results come back as events.

pub mod camera;
pub mod cloud;
pub mod editor;
pub mod control;
pub mod home;
pub mod hotkey;
pub mod launch;
pub mod library;
pub mod media;
pub mod member;
pub mod net;
pub mod observe;
pub mod os;
pub mod overlay;
pub mod posters;
pub mod power;
pub mod presenter;
pub mod probe;
pub mod recorder;
pub mod report;
pub mod ring;
pub mod stash;
pub mod status;
pub mod takes;
pub mod uninstall;
pub mod updater;
pub mod uploader;
pub mod voice;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use shroom_cloud::{Cloud, Policy};
use shroom_core::effect::{Effect, PersistOp, ShellCmd, TakeCmd};
use shroom_core::event::{Disk, Event, Field, SecretField};
use shroom_core::scene::Scene;
use shroom_core::time::{Envelope, Mono, Wall};
use shroom_core::world::{LaunchKind, Once, Prefs, Restore, SpaceIdentity, World};
use tauri::AppHandle;

use home::Home;

/// What this build is — a property of the bundle (its identifier), never of how it was
/// opened: the app is the same app from Spotlight, the Dock, at login and after a relaunch.
///
/// - **Release** (`am.shroom`): the product. `~/.shroom`, the account unfenced, the public
///   releases.
/// - **Dev** (`….dev`): the build's own — fenced to the test namespace, quiet unless asked,
///   the only flavor that takes the dev flags and the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Release,
    Dev,
}

impl Flavor {
    pub fn of(identifier: &str) -> Flavor {
        if identifier.ends_with(".dev") {
            Flavor::Dev
        } else {
            Flavor::Release
        }
    }

    /// The only flavor that takes the dev flags and the harness.
    pub fn dev(self) -> bool {
        self == Flavor::Dev
    }

    /// The folder this flavor keeps in the home folder — never shared between flavors (B4).
    pub fn home_dir(self) -> &'static str {
        match self {
            Flavor::Release => ".shroom",
            Flavor::Dev => ".shroom-dev",
        }
    }
}

/// Dev-only flags (refused by release builds): a throwaway home for automated runs, the
/// L1 Cloudflare fake's endpoints for the wizard, and `--live-shell` — a dev build is
/// quiet by default (no browser, no System Settings, no prompts, no login item, no
/// relaunch: each is logged), and only this flag lets it do them for real. For updates:
/// `--releases <loopback url>` (a release stand-in on this Mac — a dev build never checks
/// the real releases), `--update-target <throwaway .app>` (the only bundle a dev build's
/// update may ever swap), and `--as-version <x.y.z>` (the running version it reports). For
/// Region 6: `--space-base <loopback url>` — a local stand-in for every space's Worker
/// (`worker/dev/serve.mjs`): the member client calls `<base>/<host>/…` instead of
/// `https://<host>/…`, and minted credentials upload to `<base>/_s3/<host>`.
///
/// And `--e2e` — the pre-merge e2e run on the dedicated test account (TESTING-PLAN D3): the
/// product's reach, unfenced, on that one account only (`Policy::Account`, its id from
/// `~/.shroom-dev/e2e-account-id`, which the run writes) — every call addressed to another
/// account is refused before it's made — and its space set up under the run's own id
/// (`~/.shroom-dev/e2e-space-id`, also the run's): `v-<run>` over `shroom-space-<run>`, so
/// runs side by side never share a space (DECISIONS B131). Never beside the fake, a local
/// Worker or a release stand-in.
#[derive(Debug, Clone, Default)]
pub struct DevFlags {
    pub home: Option<PathBuf>,
    pub cloud_fake: Option<(String, String)>,
    pub live_shell: bool,
    pub releases: Option<String>,
    pub update_target: Option<PathBuf>,
    pub as_version: Option<String>,
    pub space_base: Option<String>,
    pub e2e: bool,
}

/// The e2e run's space id (`--e2e`): `~/.shroom-dev/e2e-space-id`, written by the run beside
/// the account id — the setup's label and bucket are the run's own (B131).
pub fn e2e_space_id() -> Option<String> {
    e2e_space_id_in(&Path::new(&std::env::var_os("HOME")?).join(".shroom-dev"))
}

/// The run id in `dir`'s `e2e-space-id`, if it is one.
pub(crate) fn e2e_space_id_in(dir: &Path) -> Option<String> {
    let id = std::fs::read_to_string(dir.join("e2e-space-id")).ok()?;
    let id = id.trim();
    shroom_cloud::wire::run_started(id).map(|_| id.to_string())
}

/// The e2e run's account id (`--e2e`): `~/.shroom-dev/e2e-account-id`, written by the run
/// — whatever `--home` a drive has (the heartbeat's member keeps a throwaway one).
pub fn e2e_account_id() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let id = std::fs::read_to_string(Path::new(&home).join(".shroom-dev/e2e-account-id")).ok()?;
    let id = id.trim();
    (id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())).then(|| id.to_ascii_lowercase())
}

impl DevFlags {
    pub fn from_args(args: &[String]) -> DevFlags {
        let at = |flag: &str| args.iter().position(|a| a == flag);
        let value = |flag: &str| at(flag).and_then(|i| args.get(i + 1)).cloned();
        DevFlags {
            home: value("--home").map(PathBuf::from),
            cloud_fake: at("--cloud-fake").and_then(|i| Some((args.get(i + 1)?.clone(), args.get(i + 2)?.clone()))),
            live_shell: at("--live-shell").is_some(),
            releases: value("--releases"),
            update_target: value("--update-target").map(PathBuf::from),
            as_version: value("--as-version"),
            space_base: value("--space-base"),
            e2e: at("--e2e").is_some(),
        }
    }

    pub fn any(&self) -> bool {
        self.home.is_some()
            || self.cloud_fake.is_some()
            || self.live_shell
            || self.releases.is_some()
            || self.update_target.is_some()
            || self.as_version.is_some()
            || self.space_base.is_some()
            || self.e2e
    }

    /// Why these flags can't run, if they can't. `--cloud-fake` turns the Cloudflare
    /// client's fence off, so it is taken only toward this Mac (a loopback host — the L1
    /// fake) and only with a throwaway `--home`: never the real account, never the dev home
    /// (whose harness token is the real account's), whatever the mode. The release
    /// stand-in is loopback only, and an update target is a throwaway copy — never an
    /// installed app — swapped only from a throwaway home.
    pub fn refusal(&self) -> Option<&'static str> {
        // The e2e run reaches the test account: no stand-in rides along, and it needs its id.
        if self.e2e && (self.cloud_fake.is_some() || self.space_base.is_some() || self.releases.is_some() || self.update_target.is_some() || self.as_version.is_some()) {
            return Some("--e2e runs on the test account — no --cloud-fake, --space-base, --releases, --update-target or --as-version");
        }
        if self.e2e && e2e_account_id().is_none() {
            return Some("--e2e needs the test account's id in ~/.shroom-dev/e2e-account-id");
        }
        if self.e2e && e2e_space_id().is_none() {
            return Some("--e2e needs the run's space id in ~/.shroom-dev/e2e-space-id");
        }
        // A throwaway home is a temp folder — never ~/.shroom, ~/.shroom-dev or any alias of
        // them (Disconnect's wipe and the updater's staging run there).
        if self.home.as_deref().is_some_and(|h| throwaway_place(h).is_none()) {
            return Some("--home takes only a throwaway folder in a temp folder (never a shroom home)");
        }
        match &self.cloud_fake {
            Some(_) if self.home.is_none() => return Some("--cloud-fake needs --home <dir>"),
            Some((api, r2)) if !loopback(api) || !loopback(r2) => return Some("--cloud-fake takes only loopback endpoints (127.0.0.1, ::1, localhost)"),
            _ => {}
        }
        if self.releases.as_deref().is_some_and(|r| !loopback(r)) {
            return Some("--releases takes only a loopback release stand-in (127.0.0.1, ::1, localhost)");
        }
        if let Some(t) = &self.update_target {
            if self.home.is_none() {
                return Some("--update-target needs --home <dir>");
            }
            if throwaway_app(t).is_none() {
                return Some("--update-target takes only a throwaway copy of the app (an existing *.app in a temp folder — never an installed copy, nor anything in a shroom home)");
            }
        }
        if self.as_version.as_deref().is_some_and(|v| shroom_update::Version::parse(v).is_none()) {
            return Some("--as-version takes a version like 1.2.3");
        }
        // The local Worker joins, mints and uploads: only toward this Mac, only from a
        // throwaway home (a dev home's space is a real, fenced one).
        match &self.space_base {
            Some(_) if self.home.is_none() => return Some("--space-base needs --home <dir>"),
            Some(b) if !loopback(b) => return Some("--space-base takes only a loopback Worker stand-in (127.0.0.1, ::1, localhost)"),
            _ => {}
        }
        None
    }

    /// The fake's endpoints, only when the flags are sound.
    fn fake(&self) -> Option<(String, String)> {
        self.refusal().is_none().then(|| self.cloud_fake.clone()).flatten()
    }

    /// The local Worker stand-in, only when the flags are sound.
    fn fake_space_base(&self) -> Option<String> {
        self.refusal().is_none().then(|| self.space_base.clone()).flatten()
    }
}

/// The only places a dev build's throwaways may live — a `--home`, an update target: the
/// system's temp folders, as the disk names them (symlinks, firmlinks and letter case
/// resolved first, so `/tmp`, `$TMPDIR` and the scratch folders all qualify, and no alias
/// of an installed app or a real home ever does).
const TEMP_ROOTS: [&str; 2] = ["/private/tmp/", "/private/var/folders/"];

/// `path` as the disk names it — every symlink, firmlink and letter case resolved — even
/// when its last components don't exist yet (they're then taken as written). A link whose
/// target isn't there yet is still a link, and is followed: a link into a shroom home that
/// was never made names that home, not the temp folder holding the link. `None` for a
/// relative path, one with `.` or `..` (a link's target included), or links that never end.
pub fn resolved(path: &std::path::Path) -> Option<PathBuf> {
    resolved_within(path, 0)
}

/// How many links-to-nowhere `resolved` follows in a row before calling it a loop (the
/// system's own limit for links that do lead somewhere is 32).
const DANGLING_LINKS: u8 = 32;

fn resolved_within(path: &std::path::Path, followed: u8) -> Option<PathBuf> {
    use std::path::Component;
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return None;
    }
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        let mut out = match std::fs::canonicalize(existing) {
            Ok(out) => out,
            // Not there — or a link to somewhere that isn't (`read_link` reads only a link,
            // and only one the disk can reach). Its target is resolved in its place.
            Err(_) => match std::fs::read_link(existing) {
                Ok(target) if followed < DANGLING_LINKS => {
                    let target = if target.is_absolute() { target } else { resolved_within(existing.parent()?, followed)?.join(target) };
                    resolved_within(&target, followed + 1)?
                }
                Ok(_) => return None,
                Err(_) => {
                    rest.push(existing.file_name()?.to_os_string());
                    existing = existing.parent()?;
                    continue;
                }
            },
        };
        for part in rest.iter().rev() {
            out.push(part);
        }
        return Some(out);
    }
}

/// This user's home folders: `$HOME` and the account's own (a changed `$HOME` never hides
/// the real one).
fn user_homes() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::env::var_os("HOME").map(PathBuf::from).into_iter().collect();
    // SAFETY: getpwuid_r fills the caller's struct from the caller's buffer.
    unsafe {
        let mut pw: libc::passwd = std::mem::zeroed();
        let mut buf = vec![0 as libc::c_char; 4096];
        let mut res: *mut libc::passwd = std::ptr::null_mut();
        if libc::getpwuid_r(libc::getuid(), &mut pw, buf.as_mut_ptr(), buf.len(), &mut res) == 0 && !res.is_null() && !pw.pw_dir.is_null() {
            out.push(PathBuf::from(std::ffi::CStr::from_ptr(pw.pw_dir).to_string_lossy().into_owned()));
        }
    }
    out
}

/// A file's identity on disk (device, inode) — what no alias can fake.
fn identity(path: &std::path::Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| (m.dev(), m.ino()))
}

/// What a throwaway may never be, nor sit inside: the Applications folders, the shroom
/// homes, a home folder itself — by identity, whatever path names them.
fn forbidden() -> Vec<(u64, u64)> {
    let mut roots = vec![PathBuf::from("/Applications"), PathBuf::from("/System")];
    for h in user_homes() {
        roots.extend([h.join("Applications"), h.join(".shroom"), h.join(".shroom-dev"), h]);
    }
    roots.iter().filter_map(|p| identity(p)).collect()
}

/// A throwaway place for a dev build (`--home`, an update target's folder): strictly inside
/// a temp root once resolved, and nowhere inside an Applications folder, a shroom home or a
/// home folder (by identity). The resolved path, or `None`.
pub fn throwaway_place(path: &std::path::Path) -> Option<PathBuf> {
    let r = resolved(path)?;
    let s = r.to_str()?;
    if !TEMP_ROOTS.iter().any(|root| s.starts_with(root) && s.len() > root.len()) {
        return None;
    }
    let no = forbidden();
    if r.ancestors().any(|a| identity(a).is_some_and(|id| no.contains(&id))) {
        return None;
    }
    Some(r)
}

/// A bundle a dev build's update may swap: an existing `*.app` folder in a throwaway place
/// — never an installed copy (nothing under `/Applications` or `~/Applications`, by any
/// alias, case or link), never inside a shroom home, and never the same folder as an
/// installed shroom. The resolved path (the one to swap), or `None`.
pub fn throwaway_app(path: &std::path::Path) -> Option<PathBuf> {
    if path.extension().is_none_or(|e| e != "app") {
        return None;
    }
    let r = throwaway_place(path)?;
    if r.extension().is_none_or(|e| e != "app") || !r.is_dir() || std::fs::symlink_metadata(&r).map(|m| m.file_type().is_symlink()).unwrap_or(true) {
        return None;
    }
    let mut installed = vec![PathBuf::from("/Applications/shroom.app"), PathBuf::from("/Applications/shroom-dev.app")];
    for h in user_homes() {
        installed.extend([h.join("Applications/shroom.app"), h.join("Applications/shroom-dev.app")]);
    }
    let me = identity(&r)?;
    if installed.iter().any(|i| identity(i) == Some(me)) {
        return None;
    }
    Some(r)
}

/// `http(s)://<loopback host>[:port][/path]`.
pub fn loopback(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")) else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(v6) => match v6.split_once(']') {
            Some((h, port)) if port.is_empty() || port.starts_with(':') => h,
            _ => return false,
        },
        None => authority.split(':').next().unwrap_or_default(),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// A dev build reopened by its own update (`--relaunched=update…`, no `--home`): the
/// arguments its updater left in a note beside the bundle — read once, then deleted.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn dev_relaunch_args(mut args: Vec<String>) -> Vec<String> {
    let relaunched = args.iter().any(|a| a == updater::RELAUNCHED_OK || a == updater::RELAUNCHED_FAILED);
    if !relaunched || args.iter().any(|a| a == "--home") {
        return args;
    }
    let Some(bundle) = os::bundle() else { return args };
    // The note sits beside the target as the updater resolved it (`/private/var/…`), which
    // may not be how this process names its own bundle (`/var/…`).
    for b in [Some(bundle.clone()), resolved(&bundle)].into_iter().flatten() {
        let note = updater::dev_note(&b);
        if let Some(n) = home::read_json::<updater::DevNote>(&note) {
            let _ = std::fs::remove_file(&note);
            args.extend(n.args);
            break;
        }
    }
    args
}

/// The source the config popover's Start (or Return) would record: its selected choice.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn records(s: &Scene) -> Option<shroom_core::world::SourceId> {
    let p = s.popover.as_ref().filter(|p| p.id == "config")?;
    let shroom_core::scene::PopoverBody::Config(c) = &p.body else { return None };
    c.sources.iter().find(|c| c.selected).and_then(|c| match &c.action {
        shroom_core::scene::Action::SetSource(id) => Some(*id),
        _ => None,
    })
}

/// The heartbeat drive's guard on what an unattended take will record: the display the
/// pattern covers and was proven on, nothing else — never a camera (the room), never
/// another display (b4 #16).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn guard_source(s: &Scene, covered: u32) -> Result<(), String> {
    match records(s) {
        Some(shroom_core::world::SourceId::Display(id)) if id == covered => Ok(()),
        Some(shroom_core::world::SourceId::Camera) => Err("the take would record the camera — refused (only the pattern is ever recorded)".into()),
        other => Err(format!("the take would record {other:?}, not the pattern's display {covered} — refused")),
    }
}

/// The test harness's modes (`harness::from_args`): each records, drives or probes this
/// Mac, so none ever runs in a build anyone else gets.
pub const HARNESS_FLAGS: &[&str] = &["--selftest", "--camera-check", "--record-check", "--guard-probe", "--drive", "--bubble-probe", "--shell-probe", "--tray-probe", "--grant", "--play-out"];

/// Why this process must not run its harness mode, if it asked for one: a build whose
/// identifier isn't a dev one never does (B5), and a build without the harness can't —
/// it would start as the app instead, with a drive's flags (b4 #18).
pub fn harness_refusal(args: &[String], dev: bool, built_with_harness: bool) -> Option<&'static str> {
    let asked = args.iter().skip(1).any(|a| HARNESS_FLAGS.contains(&a.as_str()));
    match (asked, dev, built_with_harness) {
        (false, _, _) => None,
        (true, false, _) => Some("the test harness's modes are for dev builds only"),
        (true, true, false) => Some("this build has no test harness — build it with `--features dev-harness`"),
        (true, true, true) => None,
    }
}

/// How this process was launched: `--relaunched=permission|update` is the provenance a
/// relaunch shroom asked for carries (ARCHITECTURE §3.4, `restore.json`); the install
/// script's launches say `--relaunched=install` (a fresh install) or
/// `--relaunched=install-update` (its update lane), with `--invite` when it handed over a
/// join link; `at_login` is the launch event's login mark (`launch::launched_at_login`) —
/// the login item starts shroom with no arguments.
pub fn launch_kind(args: &[String], at_login: bool) -> LaunchKind {
    if args.iter().any(|a| a == "--relaunched=permission") {
        LaunchKind::PermissionRelaunch
    } else if args.iter().any(|a| a == "--relaunched=update") {
        LaunchKind::UpdateRelaunch
    } else if args.iter().any(|a| a == "--relaunched=update-failed") {
        LaunchKind::UpdateFailed
    } else if args.iter().any(|a| a == "--invite") {
        // The installer opened shroom with an invite in `~/.shroom/invite` (B29 amended,
        // §6.12): the file's arrival — the flag carries no secret.
        LaunchKind::Install { join: true }
    } else if args.iter().any(|a| a == "--relaunched=install") {
        LaunchKind::Install { join: false }
    } else if args.iter().any(|a| a == "--relaunched=install-update") {
        // The `updating` lane stays silent (N9, N41).
        LaunchKind::InstallUpdate
    } else if at_login {
        LaunchKind::LoginItem
    } else {
        LaunchKind::User
    }
}

/// The handle everything uses to talk to the core.
#[derive(Clone)]
pub struct Runtime {
    tx: Sender<Envelope>,
    epoch: Instant,
    pub home: Home,
    pub snapshot: Arc<Mutex<Option<Scene>>>,
    /// Test runs: open no browser, no System Settings, no prompts, no login item, no
    /// relaunch — each is logged instead.
    pub quiet: Arc<AtomicBool>,
    /// Pasted secrets, by handle (law 13).
    pub stash: Arc<Mutex<stash::Stash>>,
    /// A dev build (identifier ending `.dev`).
    pub dev: bool,
    /// Dev only: the L1 fake's API and R2 endpoints.
    pub cloud_fake: Option<(String, String)>,
    /// Dev only: the e2e run's test account (`--e2e`) — the only account its client reaches.
    pub e2e_account: Option<String>,
    /// Dev only: the e2e run's space id (`--e2e`) — the setup's own label and bucket (B131).
    pub e2e_space: Option<String>,
    /// This process asked to end (Quit, Relaunch, a harness run) — any other end is
    /// macOS's (`Terminating { ours: false }`).
    pub ending_ours: Arc<AtomicBool>,
    /// Envelopes posted / handled, so an exit can wait for its last event to land.
    posted: Arc<AtomicU64>,
    handled: Arc<(Mutex<u64>, Condvar)>,
    /// The harness's trace of state ids (`None` unless a drive asked): what it reads from
    /// each Scene, and the sequence so far (a repeat isn't recorded).
    pub trace: Arc<Mutex<Option<Trace>>>,
    /// Dev only: the home is a throwaway (`--home`) — the only place Disconnect's wipe runs.
    pub throwaway: bool,
    /// The version this process reports as running (dev: `--as-version` may say otherwise).
    pub version: String,
    /// Dev only: the loopback release stand-in and the throwaway update target.
    pub releases: Option<String>,
    pub update_target: Option<PathBuf>,
    /// Dev only: what the relaunch after a dev update carries besides the home and the
    /// fake (a drive's second half).
    pub relaunch_extra: Arc<Mutex<Vec<String>>>,
    /// The flags this process was started with (a dev update's relaunch carries them).
    pub flags: DevFlags,
    /// A harness run (a drive): Copy link lands on shroom's own private pasteboard
    /// (`HARNESS_PASTEBOARD`), never the person's clipboard.
    pub harness: bool,
    /// Which space this Mac knows, as a generation: Disconnect's wipe bumps it while holding
    /// the lock, and a lane writing for a space (the Library's cache) checks it under the
    /// same lock — an answer that lands after the wipe never writes the old space back.
    pub space_gen: Arc<Mutex<u64>>,
    /// Dev only: what a drive does as the process ends (an update's quit ends a drive's
    /// first half mid-step).
    pub on_exit: Arc<Mutex<Option<OnExit>>>,
    /// Dev only: the local stand-in for every space's Worker (`--space-base`).
    pub space_base: Option<String>,
    /// shroom's own windows left out of every screen capture, by window number (the
    /// presenter adds each at creation; the recorder hands them to ScreenCaptureKit).
    #[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
    pub excluded: Arc<Mutex<Vec<u32>>>,
    /// The status seam (`status.json`, ARCHITECTURE §11.2): what it knows and last wrote.
    pub seam: status::Shared,
    /// The `shroom` command's asks waiting on the core's answer (`control.rs`).
    pub cli: control::Pending,
}

/// A drive's last words, run as the process ends.
pub type OnExit = Box<dyn FnOnce() + Send>;

/// A drive's trace: what it reads from each Scene, and the sequence so far.
pub struct Trace {
    pub probe: fn(&Scene) -> String,
    pub ids: Vec<String>,
}

impl Trace {
    pub fn new(probe: fn(&Scene) -> String) -> Trace {
        Trace { probe, ids: Vec::new() }
    }

    /// The onboarding window's state id (`closed` = no window).
    #[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
    pub fn onboarding(s: &Scene) -> String {
        s.onboarding.as_ref().map(|o| o.id).unwrap_or("closed").to_string()
    }
}

impl Runtime {
    /// A dev runtime over a throwaway home with no lanes and no core — a lane's unit test
    /// calls its functions directly and reads what they posted from the receiver.
    #[cfg(test)]
    pub(crate) fn for_test(root: PathBuf, cloud_fake: Option<(String, String)>, space_base: Option<String>) -> (Runtime, Receiver<Envelope>) {
        let home = Home::at(root);
        let _ = home.ensure();
        let (tx, rx) = channel();
        let rt = Runtime {
            tx,
            epoch: Instant::now(),
            home,
            snapshot: Arc::default(),
            quiet: Arc::new(AtomicBool::new(true)),
            stash: Arc::default(),
            dev: true,
            cloud_fake,
            e2e_account: None,
            e2e_space: None,
            ending_ours: Arc::default(),
            posted: Arc::default(),
            handled: Arc::default(),
            trace: Arc::default(),
            throwaway: true,
            version: String::from("0.0.0"),
            releases: None,
            update_target: None,
            relaunch_extra: Arc::default(),
            flags: DevFlags::default(),
            harness: true,
            space_gen: Arc::default(),
            on_exit: Arc::default(),
            space_base,
            excluded: Arc::default(),
            seam: Arc::new(Mutex::new(status::Seam::new(std::process::id(), String::from("0.0.0")))),
            cli: control::Pending::default(),
        };
        (rt, rx)
    }

    pub fn mono(&self) -> Mono {
        Mono(self.epoch.elapsed().as_millis() as u64)
    }

    /// One line in `~/.shroom/logs/run.log` (kinds and names only — never a secret).
    pub fn log(&self, line: &str) {
        log_line(&self.home.root.join("logs"), &format!("{:>8} {line}\n", self.mono().0));
    }

    pub fn post(&self, event: Event) {
        let wall = Wall(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0));
        self.posted.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx.send(Envelope { mono: self.mono(), wall, event });
    }

    /// Posts, then waits (at most `timeout`) until the core has handled it and run its
    /// effects — for the last event before the process ends.
    pub fn post_and_wait(&self, event: Event, timeout: Duration) {
        self.post(event);
        let want = self.posted.load(Ordering::SeqCst);
        let (lock, cv) = &*self.handled;
        let done = lock.lock().unwrap();
        let _ = cv.wait_timeout_while(done, timeout, |n| *n < want);
    }

    /// The one secret inbound path (law 13): the text moves into the stash; the core
    /// gets the handle and the paste's shape. Nothing about the text is logged. The text
    /// is kept only when the Scene shows that paste field now (the core takes the paste).
    pub fn paste_secret(&self, field: SecretField, text: String) {
        let lands = self
            .snapshot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.onboarding.as_ref().is_some_and(|o| o.takes_paste(field)) || s.settings.as_ref().is_some_and(|w| w.takes_paste(field)));
        let (id, shape) = self.stash.lock().unwrap().put(field, text, lands);
        self.log(&format!("paste {} into {field:?}: {shape:?}", id.0));
        self.post(Event::SecretPasted { field, id, shape });
    }

    /// A non-secret field's contents, as typed (the core filters; filtering here would
    /// hide the charset hint, which the core shows when it drops a character).
    pub fn field(&self, field: Field, value: String) {
        // A report's words may run long (B180); every other field is short.
        let max = if field == Field::ReportWords { shroom_core::copy::REPORT_WORDS_MAX } else { 256 };
        let value: String = value.chars().take(max).collect();
        self.post(Event::Field { field, value });
    }

    /// A client for the person's Cloudflare account. Release builds: unfenced. Dev builds:
    /// the L1 fake when `--cloud-fake` was given, the e2e run's one test account (`--e2e`),
    /// else the test fence — which refuses the product's own labels, so a dev build can
    /// never wire a real space.
    pub fn cloud(&self, token: &str) -> Cloud {
        match (&self.cloud_fake, self.dev) {
            (_, false) => Cloud::new(token, Policy::Unfenced),
            (Some((api, r2)), true) => Cloud::new(token, Policy::Unfenced).with_endpoints(api, r2),
            (None, true) if self.e2e_account.is_some() => Cloud::new(token, Policy::Account { id: self.e2e_account.clone().unwrap_or_default() }),
            (None, true) => Cloud::new(token, Policy::TestNamespace { prefix: TEST_PREFIX.into() }),
        }
    }

    /// Whether this build may reach `bucket` over S3. The token's derived S3 keys go
    /// straight to the bucket — past the Cloudflare client's test fence — so a dev build
    /// keeps the fence here: only a `shroom-test-…` bucket, or anything on the L1 fake.
    pub fn bucket_allowed(&self, bucket: &str) -> bool {
        // The e2e run: its client and its token's keys reach the test account alone.
        self.e2e() || bucket_allowed(self.dev, self.cloud_fake.is_some(), bucket)
    }

    /// Where setup puts the space: the product's names (`v` or its fallbacks, over
    /// `shroom-space`) — or, a dev build's `--e2e` run, the run's own (B131). A release
    /// build has no harness and never takes another.
    pub fn space_names(&self) -> shroom_cloud::wire::SpaceNames {
        use shroom_cloud::wire::SpaceNames;
        match &self.e2e_space {
            Some(run) if self.e2e() => SpaceNames::of_run(run).unwrap_or_else(SpaceNames::product),
            _ => SpaceNames::product(),
        }
    }

    /// A dev build's `--e2e` run, pinned to the dedicated test account (D3).
    pub fn e2e(&self) -> bool {
        self.dev && self.e2e_account.is_some()
    }

    /// The space's Worker at `host`, as this build may reach it (Region 6). A release build:
    /// `https://<host>`. A dev build: the local stand-in (`--space-base`), or — with neither
    /// it nor the L1 fake — only a page in the test fence's namespace (`shroom-test-…`): a
    /// dev build never joins, mints or deregisters on a real space.
    pub fn space(&self, host: &str) -> Option<shroom_cloud::member::Space> {
        self.space_url(host).map(|base| shroom_cloud::member::Space::at(host, &base))
    }

    /// Where `space()` reaches `host`'s Worker, if it may.
    fn space_url(&self, host: &str) -> Option<String> {
        let real = format!("https://{host}");
        if !self.dev {
            return Some(real);
        }
        if self.e2e() {
            return e2e_host(host).then_some(real);
        }
        if let Some(base) = &self.space_base {
            return Some(format!("{}/{host}", base.trim_end_matches('/')));
        }
        (self.cloud_fake.is_none() && host.starts_with(TEST_PREFIX)).then_some(real)
    }

    /// The space's page as wiring and the page refresh read it (the ping, §6.10): a release
    /// build's `https://<host>`; a dev build's local stand-in
    /// (`--space-base`), else the L1 fake's workers.dev pages (`--cloud-fake`), else only the
    /// test fence's real page.
    pub fn page_at(&self, host: &str) -> shroom_cloud::member::Space {
        shroom_cloud::member::Space::at(host, &self.page_base(host))
    }

    /// Where `host`'s page answers (the stop's reach probe): `https://<host>` in a release
    /// build (a read). A dev build is fenced like `space()`: its local
    /// stand-in (`--space-base`), the L1 fake's pages (`--cloud-fake`), a `shroom-test-…` page
    /// — else an address that can't exist, so a dev run never pings a real space (b4 #19, B42).
    pub fn page_base(&self, host: &str) -> String {
        if self.e2e() && e2e_host(host) {
            return format!("https://{host}");
        }
        page_base(self.dev, self.space_base.as_deref(), self.cloud_fake.as_ref().map(|(api, _)| api.as_str()), host).unwrap_or_else(|| format!("https://{host}"))
    }

    /// Where a minted credential's bytes go: the credential's own endpoint — or, on the
    /// local stand-in (dev), that space's bucket there.
    pub fn s3_endpoint(&self, host: &str, endpoint: &str) -> String {
        match (&self.space_base, self.dev) {
            (Some(base), true) => format!("{}/_s3/{host}", base.trim_end_matches('/')),
            _ => endpoint.to_string(),
        }
    }

    /// The space's bucket under the token's own derived pair (the token Mac's Library, its
    /// ledger entries, the owner grant, the roster seed): the account's R2 endpoint — or,
    /// on the local stand-in (dev), that space's bucket there.
    pub fn token_bucket(&self, cloud: &Cloud, space: &SpaceIdentity, token_id: &str) -> shroom_storage::s3::Client {
        self.token_bucket_at(cloud, &space.account_id, &space.bucket, &space.host, token_id)
    }

    pub fn token_bucket_at(&self, cloud: &Cloud, account_id: &str, bucket: &str, host: &str, token_id: &str) -> shroom_storage::s3::Client {
        shroom_storage::s3::Client::new(shroom_storage::s3::Bucket {
            endpoint: self.s3_endpoint(host, &cloud.r2_endpoint(account_id)),
            region: "auto".into(),
            name: bucket.into(),
            credentials: cloud.s3_credentials(token_id),
        })
    }

    /// Ends the process on shroom's own terms (not macOS's).
    pub fn exit(&self, app: &AppHandle) {
        self.ending_ours.store(true, Ordering::SeqCst);
        app.exit(0);
    }
}

/// How big `run.log` grows before it rolls to `run.log.1` (one old log kept).
pub const LOG_ROLL_BYTES: u64 = 5 * 1024 * 1024;

/// Which build wrote the log (`shroom <version> · macOS <version> · <chip>`), set at boot:
/// the first line of every fresh `run.log`, so any tail of it names its build.
static BUILD: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Appends one line to `<logs>/run.log`: the folder 700, the file 600 (home.rs's rule —
/// every file 600), rolled to `run.log.1` past `LOG_ROLL_BYTES` so it never grows without
/// bound. Each line opens with the UTC wall clock (`report::stamp`); a fresh log after a
/// roll opens with the build. One write per call, so lines from several threads never
/// interleave.
pub fn log_line(dir: &Path, line: &str) {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
    match std::fs::symlink_metadata(dir) {
        // A folder from before the rule (or loosened since): closed to everyone else, the
        // way the log itself is. Never through a link — only the folder that is ours.
        Ok(m) if m.is_dir() && m.permissions().mode() & 0o077 != 0 => {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        Ok(_) => {}
        Err(_) => {
            let _ = std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir);
        }
    }
    let stamp = report::stamp(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0));
    let mut out: String = line.split_inclusive('\n').map(|l| format!("{stamp} {l}")).collect();
    let log = dir.join("run.log");
    if let Ok(m) = std::fs::metadata(&log) {
        if m.len() >= LOG_ROLL_BYTES && std::fs::rename(&log, dir.join("run.log.1")).is_ok() {
            if let Some(build) = BUILD.get() {
                out = format!("{stamp} rolled: {build}\n{out}");
            }
        } else if m.permissions().mode() & 0o077 != 0 {
            // A log written before this rule: closed to everyone else.
            let _ = std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o600));
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).mode(0o600).open(&log) {
        let _ = f.write_all(out.as_bytes());
    }
}

/// One `run.log` line as `Runtime::log` wrote it: its monotonic ms and its text, past the
/// wall clock `log_line` opens it with (a line from before the stamps has none). A line
/// with no ms (the trace ring's, a continuation) is `None`.
pub fn read_log_line(line: &str) -> Option<(u64, &str)> {
    let line = line.split_once(' ').filter(|(wall, _)| wall.ends_with('Z')).map_or(line, |(_, rest)| rest);
    let (mono, text) = line.trim_start().split_once(' ')?;
    Some((mono.parse().ok()?, text))
}

/// A recording id as the run log names it: enough to tell takes apart, never the whole
/// link (`AbCd…`).
pub fn short(id: &str) -> String {
    let head: String = id.chars().take(4).collect();
    if head.len() < id.len() { format!("{head}…") } else { head }
}

/// The test fence's namespace: every cloud resource a dev build may touch on a real account.
pub const TEST_PREFIX: &str = "shroom-test-";

/// The dedicated test account's workers.dev name (TESTING-PLAN D3): the only real space a
/// dev build's `--e2e` run reaches — its page, its Worker, its storage.
pub const E2E_NAME: &str = "shroom-e2e";

/// `<label>.shroom-e2e.workers.dev`: a page on the e2e run's test account.
pub(crate) fn e2e_host(host: &str) -> bool {
    host.strip_suffix(&format!(".{E2E_NAME}.workers.dev")).is_some_and(|label| !label.is_empty() && !label.contains('.'))
}

/// Where wiring's ping of `host` goes (`None`: `https://<host>` itself). A dev build: the
/// local Worker stand-in, else the L1 fake's workers.dev pages, else — outside the test
/// fence's namespace — an address that can't exist, so a dev build never reads (nor waits
/// on) a real space's page.
fn page_base(dev: bool, space_base: Option<&str>, fake_api: Option<&str>, host: &str) -> Option<String> {
    if !dev {
        return None;
    }
    if let Some(base) = space_base {
        return Some(format!("{}/{host}", base.trim_end_matches('/')));
    }
    if let Some(api) = fake_api {
        return Some(format!("{}/page/{host}", api.trim_end_matches("/client/v4")));
    }
    (!host.starts_with(TEST_PREFIX)).then(|| "http://127.0.0.1:9".to_string())
}

/// A release build reaches its space's bucket; a dev build only the L1 fake's, or a
/// `shroom-test-…` bucket on a real account.
fn bucket_allowed(dev: bool, fake: bool, bucket: &str) -> bool {
    !dev || fake || bucket.starts_with(TEST_PREFIX)
}

/// Where effects go.
struct Lanes {
    recorder: Sender<shroom_core::effect::RecCmd>,
    uploader: Sender<shroom_core::effect::UpCmd>,
    cloud: Sender<shroom_core::effect::CloudCmd>,
    takes: Sender<TakeCmd>,
    library: Sender<shroom_core::effect::LibCmd>,
    updater: Sender<shroom_core::effect::UpdCmd>,
    member: Sender<(Instant, shroom_core::effect::MemberCmd)>,
    editor: Sender<shroom_core::effect::EditCmd>,
}

/// Starts the core and its lanes; the first event is `Booted` with what's on disk.
pub fn start(app: &AppHandle, launch: LaunchKind, flags: &DevFlags) -> Runtime {
    start_with(app, launch, false, flags)
}

pub fn start_with(app: &AppHandle, launch: LaunchKind, quiet: bool, flags: &DevFlags) -> Runtime {
    start_inner(app, launch, quiet, flags, None)
}

/// A quiet run whose state ids (as `probe` reads them) are traced from the very first
/// Scene (the harness's drives).
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn start_traced(app: &AppHandle, launch: LaunchKind, flags: &DevFlags, probe: fn(&Scene) -> String) -> Runtime {
    start_inner(app, launch, true, flags, Some(probe))
}

/// A link macOS handed shroom as it opened (`RunEvent::Opened`, or one held from a cold
/// launch): Settings' own — `shroom://settings/agent`, which the command's recording refusal
/// carries (spec N62, DECISIONS B216) — goes to the core, which opens Settings there; the
/// owner page's Edit in shroom (`shroom://edit/<host>/<id>`, N69) goes to the core, which
/// decides what it opens; anything else is the member lane's, an invite (N50). The log
/// never names the link, its space or its video (B221 (e)).
pub fn opened(rt: &Runtime, url: &str) {
    if let Some(spot) = shroom_core::event::SettingsSpot::of_link(url) {
        rt.log(&format!("link: Settings at {spot:?}"));
        rt.post(Event::SettingsLink(spot));
    } else if let Some(event) = Event::edit_link(url) {
        rt.log("link: the page's Edit in shroom");
        rt.post(event);
    } else {
        member::deep_link(rt, url);
    }
}

/// The process is ending. An end shroom didn't ask for is macOS's (Dock quit, logout,
/// System Settings' quit-and-reopen): the core hears `Terminating { ours: false }` and
/// gets a moment to write what it must (restore.json at perm-screen). The status seam
/// goes either way — a reader then knows nothing is running.
pub fn exiting(rt: &Runtime) {
    if !rt.ending_ours.load(Ordering::SeqCst) {
        rt.log("exit: not ours");
        rt.post_and_wait(Event::Os(shroom_core::event::OsNews::Terminating { ours: false }), Duration::from_millis(800));
    }
    // A harness run's log always ends on the core's trace ring (DECISIONS B93).
    if rt.harness {
        ring::dump(rt, "a harness run ends");
    }
    status::close(rt);
    if let Some(f) = rt.on_exit.lock().unwrap().take() {
        f();
    }
}

/// The home this process keeps: a dev build's throwaway `--home`, else the build's by its
/// identifier.
fn home_for(flags: &DevFlags, dev: bool, identifier: &str) -> std::io::Result<Home> {
    match (&flags.home, dev) {
        (Some(root), true) => Ok(Home::at(root.clone())),
        _ => Home::for_identifier(identifier),
    }
}

/// Whether the shell stays quiet: always in a harness run; in a dev build unless asked
/// (`--live-shell`).
fn shell_quiet(harness: bool, rt: &Runtime) -> bool {
    harness || (rt.dev && !rt.flags.live_shell)
}

/// How a launch reads in the run log: `login` is macOS's login launch (the `lgit` mark,
/// B45 a) — what the checkpoint's login step looks for.
fn launch_word(launch: LaunchKind) -> &'static str {
    match launch {
        LaunchKind::User => "user",
        LaunchKind::LoginItem => "login",
        LaunchKind::UpdateRelaunch => "relaunch-update",
        LaunchKind::PermissionRelaunch => "relaunch-permission",
        LaunchKind::Install { join: false } => "install",
        LaunchKind::Install { join: true } => "install-invite",
        LaunchKind::InstallUpdate => "install-update",
        LaunchKind::UpdateFailed => "relaunch-update-failed",
    }
}

/// The boot lines: kinds only — no path, no name, no secret.
fn log_boot(rt: &Runtime, home_override: bool, quiet: bool, launch: LaunchKind) {
    // Which shroom wrote the lines that follow, on which Mac: a report's log reads by it
    // (B180), and a roll writes it again at the top of the fresh log.
    let build = report::build(&rt.version);
    let _ = BUILD.set(build.clone());
    rt.log(&format!("boot: {build}"));
    rt.log(&format!("boot: launch {}", launch_word(launch)));
    // Which releases this Mac takes (B220): a report's log says whether it ran a pre-release.
    rt.log(&format!("boot: channel {}", updater::mac_channel(rt).word()));
    rt.log(&format!(
        "boot: home {} · shell {} · cloud {}",
        if home_override { "override" } else { "default" },
        if quiet { "quiet" } else { "live" },
        match (&rt.cloud_fake, rt.dev) {
            (_, false) => "unfenced",
            (Some(_), true) => "the L1 fake",
            (None, true) if rt.e2e() => "the e2e test account",
            (None, true) => "test fence",
        }
    ));
}

fn start_inner(app: &AppHandle, launch: LaunchKind, quiet: bool, flags: &DevFlags, traced: Option<fn(&Scene) -> String>) -> Runtime {
    let identifier = app.config().identifier.clone();
    let flavor = Flavor::of(&identifier);
    let dev = flavor.dev();
    // Release builds never take the dev flags (main refuses them before this point).
    // The home is this account's own, private folder — or nothing runs (law 13): no home
    // folder, one that can't be made, a link, another user's, or one left open is fatal.
    let home = match home_for(flags, dev, &identifier).and_then(|h| h.ensure_private().map(|()| h)) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("shroom: can't use its folder in your home ({e})");
            std::process::exit(70);
        }
    };
    let version = match (&flags.as_version, dev) {
        (Some(v), true) => v.clone(),
        _ => app.package_info().version.to_string(),
    };
    let (tx, rx) = channel();
    let excluded = Arc::new(Mutex::new(Vec::new()));
    let rt = Runtime {
        tx,
        epoch: Instant::now(),
        home: home.clone(),
        snapshot: Arc::default(),
        quiet: Arc::default(),
        stash: Arc::default(),
        dev,
        cloud_fake: if dev { flags.fake() } else { None },
        e2e_account: if dev && flags.e2e && flags.refusal().is_none() { e2e_account_id() } else { None },
        e2e_space: if dev && flags.e2e && flags.refusal().is_none() { e2e_space_id() } else { None },
        ending_ours: Arc::default(),
        posted: Arc::default(),
        handled: Arc::default(),
        trace: Arc::new(Mutex::new(traced.map(Trace::new))),
        throwaway: dev && flags.home.as_deref().is_some_and(|h| throwaway_place(h).is_some()),
        version: version.clone(),
        releases: if dev { flags.releases.clone() } else { None },
        update_target: if dev { flags.update_target.clone() } else { None },
        relaunch_extra: Arc::default(),
        flags: if dev { flags.clone() } else { DevFlags::default() },
        harness: dev && quiet,
        space_gen: Arc::default(),
        on_exit: Arc::default(),
        space_base: if dev { flags.fake_space_base() } else { None },
        excluded: excluded.clone(),
        seam: Arc::new(Mutex::new(status::Seam::new(std::process::id(), version.clone()))),
        cli: control::Pending::default(),
    };

    let quiet = shell_quiet(quiet, &rt);
    rt.quiet.store(quiet, Ordering::SeqCst);
    log_boot(&rt, flags.home.is_some() && dev, quiet, launch);
    if rt.space_base.is_some() {
        rt.log("boot: spaces' pages · the local stand-in (--space-base)");
    }
    // Which encoder a take will run, by word (B82) — the release proof reads it (L5).
    rt.log(&format!("boot: encoder {}", find_encoder().map(|(s, _)| s.word()).unwrap_or("none")));
    let live = Arc::new(Mutex::new(None));
    let lanes = Lanes {
        recorder: recorder::spawn(rt.clone(), live.clone(), excluded.clone()),
        uploader: uploader::spawn(rt.clone(), live),
        cloud: cloud::spawn(rt.clone(), app.clone()),
        takes: takes::spawn_lane(rt.clone()),
        library: library::spawn(rt.clone()),
        updater: updater::spawn(rt.clone(), app.clone()),
        member: member::spawn(rt.clone()),
        editor: editor::spawn(rt.clone()),
    };
    // The voice cuts' sweep (N65): takes whose words came late.
    voice::spawn_sweep(rt.clone());
    // The core's trace ring, and the panic hook that writes it down (DECISIONS B93).
    ring::install(&rt);
    let presenter = presenter::Presenter::new(app.clone(), rt.clone(), excluded);
    let core_rt = rt.clone();
    let app2 = app.clone();
    std::thread::Builder::new()
        .name("shroom-core".into())
        .spawn(move || run(core_rt, rx, lanes, presenter, app2))
        .expect("the core thread starts");

    // Boot: what's on disk, plus the first local facts (gathered on the main thread).
    // restore.json is read once, then deleted (bend 9).
    let restore = rt.home.read_json::<RestoreFile>("restore.json").map(|f| f.restore);
    rt.home.remove("restore.json");
    let rt2 = rt.clone();
    let _ = app.run_on_main_thread(move || {
        let mtm = objc2_foundation::MainThreadMarker::new().expect("main thread");
        let (quick, _) = probe::quick(mtm, &rt2.home.root);
        let disk = Disk {
            launch,
            version: rt2.version.clone(),
            prefs: rt2.home.read_json::<Prefs>("prefs.json").unwrap_or_default(),
            once: rt2.home.read_json::<Once>("once.json").unwrap_or_default(),
            space: rt2.home.read_json::<SpaceIdentity>("space.json"),
            // Only a pasted token counts: the harness's test-account token (dev) is the
            // uploader's, never the wizard's.
            token: rt2.home.saved_token().is_some(),
            device_key: rt2.home.path("device.key").exists(),
            primed: rt2.home.read_json::<home_files::Primed>("primed.json").map(|p| Wall(p.at)),
            restore,
            quick,
        };
        rt2.post(Event::Booted(Box::new(disk)));
        // Region 6: a join asked for before a crash, or an invite waiting (§5, §6.12).
        member::boot(&rt2, launch);
        // Whether shroom starts at login, as macOS says now (a read — never a change).
        if let Some(on) = os::login_item() {
            rt2.post(Event::Os(shroom_core::event::OsNews::LoginItem(on)));
        }
        // The Library's dates are this Mac's calendar.
        rt2.post(Event::Os(shroom_core::event::OsNews::UtcOffset(os::utc_offset_min())));
    });
    net::watch(rt.clone());
    observe::watch(rt.clone(), rt.home.root.clone());
    // macOS's will-sleep, held for a take's pause (DECISIONS B92).
    power::watch(rt.clone());
    // The seam's own re-read of the grants (the core's facts move only on entry events).
    status::watch(rt.clone(), os::grants);
    rt
}

pub mod home_files {
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Primed {
        pub v: u32,
        pub at: i64,
    }
}

/// `restore.json`: `{v, why, window, written}`.
#[derive(serde::Serialize, serde::Deserialize)]
struct RestoreFile {
    v: u32,
    #[serde(flatten)]
    restore: Restore,
}

fn run(rt: Runtime, rx: Receiver<Envelope>, lanes: Lanes, mut presenter: presenter::Presenter, app: AppHandle) {
    let mut w = World::blank();
    let mut wake: Option<Mono> = None;
    // The screen before each batch, and when to write the trace ring down (B93).
    let mut last: Option<Scene> = None;
    let mut watch = ring::Watch::default();
    // The `shroom` command's door (spec N62, DECISIONS B216) opens once the core has what's
    // on disk: an ask before `Booted` would be answered from an empty World — "not set up",
    // the switch off — and the command that just opened shroom asks the moment it can.
    let mut door = false;
    loop {
        let timeout = wake.map(|t| Duration::from_millis(t.0.saturating_sub(rt.mono().0))).unwrap_or(Duration::from_secs(3600));
        let first = match rx.recv_timeout(timeout) {
            Ok(env) => env,
            Err(RecvTimeoutError::Timeout) => {
                let wall = Wall(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0));
                Envelope { mono: rt.mono(), wall, event: Event::Tick }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut effects = Vec::new();
        let mut count = 0u64;
        let take_was_open = w.take_open();
        for env in std::iter::once(first).chain(rx.try_iter()) {
            let tick = matches!(env.event, Event::Tick);
            // Named before `update` takes it: the ring keeps kinds only (B93, law 13).
            let (mono, kind) = (env.mono.0, shroom_core::trace::event_kind(&env.event));
            let booted = matches!(env.event, Event::Booted(_));
            let (fx, arm) = shroom_core::update(&mut w, env);
            if booted && !door {
                control::spawn(rt.clone(), rt.cli.clone());
                door = true;
            }
            if let Some(line) = core_line(tick, &arm, &fx) {
                rt.log(&line);
            }
            ring::step(mono, kind, &arm, &fx);
            effects.extend(fx);
            count += u64::from(!tick);
        }
        let scene = shroom_core::derive(&w);
        // N68: the Library's cleanup line coming and going while it's open (the core's arm
        // just before says why: suggested, put away, deleted…).
        let cleanup_line = |s: &Scene| s.library.as_ref().map(|l| l.body.cleanup_line.is_some());
        if let (Some(Some(was)), Some(now)) = (last.as_ref().map(cleanup_line), cleanup_line(&scene)) {
            if was != now {
                rt.log(if now { "library: the cleanup line shows" } else { "library: the cleanup line is gone" });
            }
        }
        watch.after(&rt, &w, take_was_open, last.as_ref(), &scene, &effects);
        last = Some(scene.clone());
        presenter.present(&scene);
        if let Some(t) = rt.trace.lock().unwrap().as_mut() {
            let id = (t.probe)(&scene);
            if t.ids.last() != Some(&id) {
                t.ids.push(id);
            }
        }
        *rt.snapshot.lock().unwrap() = Some(scene.clone());
        wake = scene.next_wake;
        // The status seam (§11.2): the core's projection, written when what it says changed.
        status::publish(&rt, |s| s.core(shroom_core::derive::status::posture(&w)));
        for e in effects {
            dispatch(&rt, &lanes, &app, e);
        }
        let (lock, cv) = &*rt.handled;
        *lock.lock().unwrap() += count;
        cv.notify_all();
    }
}

/// A core step's run-log line: its arm and its effects' kinds. A tick that asked for
/// nothing is left out, so a day of waiting doesn't push the day's events out of a
/// report's tail (the trace ring still keeps it).
fn core_line(tick: bool, arm: &shroom_core::Arm, fx: &[Effect]) -> Option<String> {
    (!tick || !fx.is_empty()).then(|| format!("core {} {:?}", arm.name, fx.iter().map(effect_kind).collect::<Vec<_>>()))
}

fn effect_kind(e: &Effect) -> &'static str {
    match e {
        Effect::Recorder(_) => "recorder",
        Effect::Publish { .. } => "publish",
        Effect::Uploader(_) => "uploader",
        Effect::Cloud(_) => "cloud",
        Effect::Shell(_) => "shell",
        Effect::Persist(_) => "persist",
        Effect::Takes(_) => "takes",
        Effect::Library(_) => "library",
        Effect::Updater(_) => "updater",
        Effect::Member(_) => "member",
        Effect::Editor(_) => "editor",
        Effect::Cli(_) => "cli",
    }
}

fn dispatch(rt: &Runtime, lanes: &Lanes, app: &AppHandle, e: Effect) {
    match e {
        Effect::Recorder(cmd) => {
            let _ = lanes.recorder.send(cmd);
        }
        Effect::Publish { dir, rec } => {
            // law 3's single gate, written on the core thread before anything else runs
            let path = rt.home.takes().join(&dir.0).join("publish.json");
            let wall = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
            let title = untitled(wall);
            let _ = home::write_json(&path, &takes::PublishJson { v: 1, rec: rec.0, title, stopped: wall });
            let _ = lanes.takes.send(TakeCmd::Rescan);
        }
        Effect::Uploader(cmd) => {
            let _ = lanes.uploader.send(cmd);
        }
        Effect::Cloud(cmd) => {
            let _ = lanes.cloud.send(cmd);
        }
        Effect::Takes(cmd) => {
            let _ = lanes.takes.send(cmd);
        }
        Effect::Persist(PersistOp::Prefs(p)) => {
            let _ = rt.home.write_json("prefs.json", &p);
        }
        Effect::Persist(PersistOp::Space(identity)) => {
            let _ = member::write_space(rt, &identity);
        }
        Effect::Persist(PersistOp::Primed(at)) => {
            let _ = rt.home.write_json("primed.json", &home_files::Primed { v: 1, at: at.0 });
        }
        Effect::Persist(PersistOp::Restore(Some(restore))) => {
            let _ = rt.home.write_json("restore.json", &RestoreFile { v: 1, restore });
        }
        Effect::Persist(PersistOp::Restore(None)) => rt.home.remove("restore.json"),
        Effect::Persist(PersistOp::Once(once)) => {
            let _ = rt.home.write_json("once.json", &once);
        }
        Effect::Persist(PersistOp::Wipe) => wipe(rt),
        Effect::Shell(cmd) => shell(rt, app, cmd),
        Effect::Library(cmd) => {
            let _ = lanes.library.send(cmd);
        }
        Effect::Updater(cmd) => {
            let _ = lanes.updater.send(cmd);
        }
        Effect::Member(cmd) => {
            let _ = lanes.member.send((Instant::now(), cmd));
        }
        Effect::Editor(cmd) => {
            let _ = lanes.editor.send(cmd);
        }
        Effect::Cli(a) => control::answer(rt, &rt.cli, a),
    }
}

/// Disconnect and Leave (N36, WORKER-INTERFACE §5's recipe): this Mac deregisters from
/// its space (`DELETE devices/self`, best-effort — the wipe never waits on it) and forgets
/// it — `space.json`, the token, the device key, the Library's cache and posters, the
/// editor's folder, an invite waiting, a join asked for, `restore.json`, the stash, and each
/// remaining take's per-recording credential go. The prefs, `once.json`, `primed.json` (this Mac's permissions chapter),
/// the takes themselves and the login item stay. The core is already at
/// `idle-unprovisioned`. A dev build wipes only a throwaway home (`--home`).
fn wipe(rt: &Runtime) {
    if rt.dev && !rt.throwaway {
        rt.log("persist would wipe the space (a dev build wipes only a --home throwaway)");
        return;
    }
    if let (Some(space), Some(key)) = (rt.home.read_json::<SpaceIdentity>("space.json"), member::device_key(rt)) {
        if key.host == space.host {
            member::deregister(rt, &space.host, key, None);
        }
    }
    // Held through the removal: a lane mid-write for the old space finishes first or never.
    let mut generation = rt.space_gen.lock().unwrap();
    *generation += 1;
    for name in ["space.json", "token.json", "device.key", "library.json", "invite", "join.json", "restore.json"] {
        rt.home.remove(name);
    }
    // The space's posters go with its list, and what the editor fetched and read of its
    // videos with them (N63).
    let _ = std::fs::remove_dir_all(rt.home.root.join("posters"));
    media::MEDIA.forget_all();
    let _ = std::fs::remove_dir_all(rt.home.edits());
    drop(generation);
    for dir in takes::dirs(&rt.home.takes()) {
        let _ = std::fs::remove_file(dir.join("cred.json"));
    }
    rt.stash.lock().unwrap().forget_all();
    rt.log("persist: wiped the space from this Mac");
}

/// N26: an untouched title is "Untitled — <the day's date>", e.g. "Untitled — Aug 21" —
/// the person's local date, not UTC's.
pub fn untitled(wall_ms: i64) -> String {
    let t: libc::time_t = (wall_ms / 1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][tm.tm_mon.clamp(0, 11) as usize];
    format!("Untitled — {month} {}", tm.tm_mday)
}

fn shell(rt: &Runtime, app: &AppHandle, cmd: ShellCmd) {
    use shroom_core::effect::Pane;
    let quiet = rt.quiet.load(Ordering::SeqCst);
    match cmd {
        ShellCmd::Quit => rt.exit(app),
        ShellCmd::SettingsPane(pane) => {
            let url = match pane {
                Pane::ScreenRecording => "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
                Pane::Camera => "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera",
                Pane::Microphone => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
            };
            if quiet {
                rt.log(&format!("shell would open {url}"));
            } else {
                os::open_detached(url.to_string());
            }
        }
        ShellCmd::RequestPermission(perm) => os::request_permission(rt, perm),
        ShellCmd::Copy(rec) => os::copy_link(rt, &rec),
        ShellCmd::CopyThumb(rec, title) => os::copy_thumb(rt, &rec, &title),
        ShellCmd::CopyAgentPrompt => {
            rt.log("shell: copied the prompt for a coding agent");
            os::put_on_pasteboard(rt, shroom_core::copy::agent_prompt());
        }
        ShellCmd::Reveal(dir) => os::reveal(rt, &dir),
        ShellCmd::PowerAssertion(on) => os::power_assertion(rt, on),
        // The take is safe: macOS's waiting will-sleep goes ahead (B92).
        ShellCmd::AllowSleep => power::allow_pending(rt),
        // The window opens with the Scene; a footer click on one already open brings it
        // forward with the keyboard (the person reached for it).
        ShellCmd::OpenLibrary => presenter::bring_forward_kind(app, shroom_core::world::WindowKind::Library),
        ShellCmd::OpenSettings => presenter::bring_forward_kind(app, shroom_core::world::WindowKind::Settings),
        ShellCmd::OpenFeedback => presenter::bring_forward_kind(app, shroom_core::world::WindowKind::Feedback),
        ShellCmd::OpenEditor(id) => presenter::bring_forward_kind(app, shroom_core::world::WindowKind::Editor(id)),
        ShellCmd::OpenUrl(page) => os::open_url(rt, &page),
        ShellCmd::SetLoginItem(on) => os::set_login_item(rt, on),
        ShellCmd::Relaunch(why) => os::relaunch(rt, app, why),
        ShellCmd::Uninstall => uninstall::uninstall(rt, app),
        // A join link's Copy link (Region 6): its code lives in the stash.
        ShellCmd::CopyInvite(id) => os::copy_invite(rt, id),
        // Report a problem and Copy diagnostics (B180), each off the core thread.
        ShellCmd::GatherReport { role } => report::gather_for_sheet(rt, role),
        ShellCmd::CopyDiagnostics { role } => report::copy(rt, role),
        ShellCmd::SendReport(post) => report::send(rt, post),
    }
}

/// Whether this process is a dev build, for the encoder's road — set by `main` from the
/// identifier before anything else runs. Unset (a unit test's process) reads as dev.
static DEV_ENCODER_ROAD: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Says which encoder road this process takes (DECISIONS B82): `main` calls it once, first.
pub fn set_encoder_road(dev: bool) {
    let _ = DEV_ENCODER_ROAD.set(dev);
}

/// The encoder and where it came from (DECISIONS B82, amending B12): a release runs only the
/// sidecar beside its binary (`Contents/MacOS/ffmpeg`); a dev build takes `SHROOM_FFMPEG`,
/// then a sidecar if it carries one, then Homebrew's (LaunchServices starts apps with a bare
/// PATH).
pub fn find_encoder() -> Option<(shroom_encode::locate::Source, std::path::PathBuf)> {
    let dev = *DEV_ENCODER_ROAD.get().unwrap_or(&true);
    let exe = std::env::current_exe().ok();
    let env = std::env::var_os(shroom_encode::locate::ENV);
    shroom_encode::locate::locate(exe.as_deref().and_then(std::path::Path::parent), env.as_deref(), dev)
}

pub fn locate_ffmpeg() -> Option<std::path::PathBuf> {
    find_encoder().map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tests-sys #2 (DECISIONS B66): the release bundle claims `shroom://` — the join page's Open
    /// shroom (SM:1442, 1751) — and only the release bundle: a dev build never does (a real invite opened on
    /// this Mac can't launch one). The release plist says what the dev one says, plus the URL
    /// type; the release config is never a dev identifier.
    #[test]
    fn only_the_release_bundle_claims_the_shroom_scheme() {
        let dev = include_str!("../../Info.plist");
        let release = include_str!("../../Info.release.plist");
        assert!(!dev.contains("CFBundleURLTypes"), "a dev bundle never claims the scheme");
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert!(conf["bundle"]["macOS"].get("infoPlist").is_none(), "the dev config merges no other plist");
        let rel: serde_json::Value = serde_json::from_str(include_str!("../../tauri.release.conf.json")).unwrap();
        assert_eq!(rel["bundle"]["macOS"]["infoPlist"], "Info.release.plist");
        let id = rel["identifier"].as_str().unwrap();
        assert!(!id.ends_with(".dev") && id != conf["identifier"].as_str().unwrap(), "{id}");
        assert!(rel["bundle"]["macOS"]["signingIdentity"].is_null(), "the release config never signs with the dev key");
        // The scheme the member lane reads (`shroom://join/<host>/<code>`, WORKER-INTERFACE §6.2).
        let types = release.split("<key>CFBundleURLSchemes</key>").nth(1).expect("a URL type");
        assert!(types.trim_start().starts_with("<array>\n        <string>shroom</string>"), "{types}");
        assert_eq!(super::member::loose_host("shroom://join/v.x.workers.dev/short"), "v.x.workers.dev");
        // Everything the dev plist says, the release one says verbatim (the permission prompts).
        for line in dev.lines().map(str::trim).filter(|l| l.starts_with("<string>") || l.starts_with("<key>")) {
            assert!(release.contains(line), "the release plist lost {line}");
        }
    }

    /// Every flavor is a menu-bar app from the moment macOS hears of it (DECISIONS B113): the
    /// bundle says so — the Dock shows a launching app by what its bundle says, before the
    /// app has run a line — in the dev plist and in the release one; and the app keeps it so through its own start (the windowing layer
    /// would otherwise make it an ordinary app for a moment as it finishes launching). The
    /// Dock icon comes with a window, raised when a Scene asks (law 6).
    #[test]
    fn every_flavor_is_born_a_menu_bar_app() {
        for (name, plist) in [("Info.plist", include_str!("../../Info.plist")), ("Info.release.plist", include_str!("../../Info.release.plist"))] {
            let after = plist.split("<key>LSUIElement</key>").nth(1).unwrap_or_else(|| panic!("{name} doesn't say shroom is a menu-bar app"));
            assert!(after.trim_start().starts_with("<true/>"), "{name}: {after}");
        }
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.release.conf.json")).unwrap();
        assert_eq!(conf["bundle"]["macOS"]["infoPlist"], "Info.release.plist");
        // Set on the app before its event loop runs — never only once it has launched.
        let main = include_str!("../main.rs");
        let at = |what: &str| main.find(what).unwrap_or_else(|| panic!("main.rs has no `{what}`"));
        let policy = at("app.set_activation_policy(tauri::ActivationPolicy::Accessory)");
        assert!(at(".build(context)") < policy && policy < at("app.run("), "the policy is set between building the app and running it");
    }

    /// Encode cluster #1 (DECISIONS B82): the release bundle carries its own encoder — the
    /// sidecar `tools/build-ffmpeg.sh` makes, under the name Tauri's `externalBin` wants — and
    /// FFmpeg's notice and licence beside it; the notice names the tarball the script pins. A
    /// dev bundle carries none (B12's road), and the release keeps the dev config's floor.
    #[test]
    fn the_release_bundle_carries_its_encoder_and_its_licence() {
        let rel: serde_json::Value = serde_json::from_str(include_str!("../../tauri.release.conf.json")).unwrap();
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let script = include_str!("../../../../tools/build-ffmpeg.sh");
        let notice = include_str!("../../../../tools/ffmpeg-NOTICE.txt");
        // Beside it, the `shroom` command (DECISIONS B217): Contents/MacOS/shroom-cli, built by
        // tools/release-build.sh where the bundler reads it.
        assert_eq!(rel["bundle"]["externalBin"], serde_json::json!(["../../build/ffmpeg/sidecar/ffmpeg", "../../build/cli/shroom-cli"]));
        let build = include_str!("../../../../tools/release-build.sh");
        assert!(build.contains("CLI=$ROOT/build/cli/shroom-cli-aarch64-apple-darwin") && build.contains("cargo build --release -p shroom-cli"), "release-build makes the command the bundle reads");
        assert!(script.contains("OUT=$ROOT/build/ffmpeg") && script.contains("\"$OUT/sidecar/ffmpeg-$TRIPLE\"") && script.contains("TRIPLE=aarch64-apple-darwin"), "the script writes what the bundle reads");
        assert_eq!(std::path::Path::new("../../build/ffmpeg/sidecar/ffmpeg").file_name().unwrap(), shroom_encode::locate::SIDECAR, "Tauri bundles it as Contents/MacOS/ffmpeg, where a release looks");
        let res = &rel["bundle"]["resources"];
        assert_eq!(res["../../tools/ffmpeg-NOTICE.txt"], "licenses/ffmpeg/NOTICE.txt");
        assert_eq!(res["../../build/ffmpeg/sidecar/COPYING.LGPLv2.1"], "licenses/ffmpeg/COPYING.LGPLv2.1");
        // Every third party's licence ships beside it (B123, B124): the file
        // tools/third-party-notices.py writes (CI holds it current), naming the git
        // dependencies, the webviews' bundled packages and the player's hls.js among the rest.
        assert_eq!(res["../../THIRD-PARTY-NOTICES.md"], "licenses/third-party/NOTICES.md");
        let notices = include_str!("../../../../THIRD-PARTY-NOTICES.md");
        assert!(notices.contains("*Generated by `tools/third-party-notices.py` — don't edit by hand.*"));
        for dep in ["scap-screencapturekit", "cap-camera-avfoundation", "cidre"] {
            assert!(include_str!("../../../../crates/shroom-capture/Cargo.toml").contains(&format!("{dep} = {{ git")), "{dep} is a git dependency");
            assert!(notices.contains(&format!("| {dep} | ")), "the notices list {dep}");
        }
        for dep in ["tauri", "svelte", "@tauri-apps/api", "hls.js"] {
            assert!(notices.contains(&format!("| {dep} | ")), "the notices list {dep}");
        }
        let pin = |key: &str| script.lines().find_map(|l| l.strip_prefix(&format!("{key}="))).unwrap().trim_matches('"').to_string();
        let (version, sha) = (pin("VERSION"), pin("SHA256"));
        assert!(notice.contains(&format!("https://ffmpeg.org/releases/ffmpeg-{version}.tar.xz")) && notice.contains(&sha), "the notice names the pinned tarball");
        let configure = script.split("set -- \\\n").nth(1).and_then(|r| r.split("configure_line=").next()).expect("the configure line");
        assert!(configure.contains("--disable-autodetect") && configure.contains("--disable-network"), "{configure}");
        assert!(!configure.contains("--enable-gpl") && !configure.contains("--enable-nonfree") && !configure.contains("--enable-version3"), "LGPL v2.1+ only");
        assert!(conf["bundle"].get("externalBin").is_none(), "a dev bundle carries no encoder (B12's road)");
        // A dev build carries the command, and only it (DECISIONS B217): npm run build:dev
        // builds it first (tools/dev-cli.sh) and bundles it through tauri.dev-cli.conf.json.
        let dev_cli: serde_json::Value = serde_json::from_str(include_str!("../../tauri.dev-cli.conf.json")).unwrap();
        assert_eq!(dev_cli["bundle"]["externalBin"], serde_json::json!(["../../build/cli/shroom-cli"]));
        let scripts: serde_json::Value = serde_json::from_str(include_str!("../../../package.json")).unwrap();
        let dev = scripts["scripts"]["build:dev"].as_str().unwrap();
        assert!(dev.starts_with("bash ../tools/dev-cli.sh && ") && dev.contains("--config src-tauri/tauri.dev-cli.conf.json"), "{dev}");
        assert!(include_str!("../../../../tools/dev-cli.sh").contains("\"$ROOT/build/cli/shroom-cli-$triple\""), "dev-cli.sh writes what the dev bundle reads");
        assert!(rel["bundle"]["macOS"].get("minimumSystemVersion").is_none(), "the release keeps the one floor");
    }

    /// Encode cluster #2 (DECISIONS B83, B6 amended): one macOS floor — the bundle's
    /// `minimumSystemVersion` (LSMinimumSystemVersion, the binaries' deployment target, and the
    /// encoder's `-mmacosx-version-min`, which `tools/build-ffmpeg.sh` reads from it) is the
    /// installer's `MIN_MACOS`, whole, as the updater reads it. `tests/l5/minos-probe.sh` holds
    /// the built bundle to it.
    #[test]
    fn one_macos_floor_everywhere() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let bundle = conf["bundle"]["macOS"]["minimumSystemVersion"].as_str().unwrap();
        let installer = include_str!("../../../../installer/install.sh");
        let min = installer.lines().find_map(|l| l.strip_prefix("MIN_MACOS=")).unwrap();
        assert_eq!(bundle, format!("{min}.0"), "the bundle's floor is the installer's, whole");
        let script = include_str!("../../../../tools/build-ffmpeg.sh");
        assert!(script.contains("plutil -extract bundle.macOS.minimumSystemVersion raw -o - \"$ROOT/app/src-tauri/tauri.conf.json\"") && script.contains("-mmacosx-version-min=$FLOOR"), "the encoder is built for the bundle's floor");
        // The one release build runs the probe, and every lane that makes a release-config
        // app is that build (DECISIONS B102): this Mac's L5 lane, CI, and the tag workflow.
        let release = include_str!("../../../../tools/release-build.sh");
        assert!(release.contains("\"$ROOT/tests/l5/minos-probe.sh\" \"$app\""), "every release build runs the floor's probe");
        let local = include_str!("../../../../tests/l5/build-release.sh");
        assert!(local.contains("\"$ROOT/tools/release-build.sh\" \"$out\" --identity shroom-dev"), "this Mac's release lane is the one release build");
        assert!(!local.contains("tauri build"), "and builds nothing by itself");
    }

    /// DECISIONS B102: a tag's build is fast only because it starts from what the default
    /// branch's CI saved — GitHub lets a tag's run read `main`'s caches, never another tag's.
    /// So the two workflows must build the same thing the same way (the one release build,
    /// the same toolchain and environment) under the same cache names, CI saving from `main`
    /// only and the tag run never saving. Every action is pinned to a commit, and the
    /// signing secrets are named in the tag workflow alone.
    #[test]
    fn the_tag_workflow_builds_what_ci_cached() {
        let ci = include_str!("../../../../.github/workflows/ci.yml");
        let rel = include_str!("../../../../.github/workflows/release.yml");
        // One value of a `key: value` line, wherever it sits.
        let all = |text: &str, key: &str| -> Vec<String> {
            text.lines().filter(|l| !l.trim_start().starts_with('#')).filter_map(|l| l.trim().trim_start_matches("- ").strip_prefix(&format!("{key}:")).map(|v| v.trim().to_string())).collect()
        };
        for wf in [ci, rel] {
            assert!(wf.contains("tools/release-build.sh"), "both build through the one release build");
            assert!(!wf.contains("tauri build"), "and never by another road");
            assert!(!wf.contains("pull_request_target"), "a pull request's code never runs with the repo's secrets");
            for uses in all(wf, "uses") {
                let (_, at) = uses.split_once('@').unwrap_or_else(|| panic!("{uses} isn't pinned"));
                let sha = at.split_whitespace().next().unwrap();
                assert!(sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()), "{uses} is pinned to a full commit");
                assert!(at.contains("# v"), "{uses} says which version that commit is");
            }
        }
        // The same runner image, toolchain pin and build environment…
        for key in ["RUNNER_IMAGE", "NODE_VERSION", "CARGO_INCREMENTAL", "CARGO_TERM_COLOR", "RELEASE_CACHE"] {
            let (a, b) = (all(ci, key), all(rel, key));
            assert!(a.len() == 1 && a == b, "{key} is set once in each workflow, to the same value: {a:?} vs {b:?}");
        }
        let image = all(ci, "RUNNER_IMAGE")[0].clone();
        for wf in [ci, rel] {
            for on in all(wf, "runs-on").iter().filter(|r| r.starts_with("macos")) {
                assert_eq!(on, &image, "RUNNER_IMAGE names the image the jobs run on (the caches are keyed by it)");
            }
        }
        // …under the same cache names: the Rust cache's shared key and the encoder's key.
        let shared = |wf: &str| all(wf, "shared-key").into_iter().filter(|k| k.contains("RELEASE_CACHE")).collect::<Vec<_>>();
        assert!(shared(ci).len() == 1 && shared(ci) == shared(rel), "{:?} vs {:?}", shared(ci), shared(rel));
        let encoder = |wf: &str| all(wf, "key").into_iter().filter(|k| k.starts_with("ffmpeg-sidecar-")).collect::<std::collections::BTreeSet<_>>();
        assert!(encoder(ci).len() == 1 && encoder(ci) == encoder(rel), "{:?} vs {:?}", encoder(ci), encoder(rel));
        // CI saves only from the default branch; the tag run only restores.
        assert!(all(ci, "save-if").iter().all(|v| v.contains("github.ref == 'refs/heads/main'")) && !all(ci, "save-if").is_empty());
        assert_eq!(all(rel, "save-if"), ["false"], "a tag's cache is invisible to the next tag — never saved");
        assert!(rel.contains("actions/cache/restore@") && !rel.contains("actions/cache@"), "the tag run restores, never saves");
        // The signing secrets: the tag workflow's alone, in the `release` environment.
        assert!(!ci.contains("secrets."), "CI reads no secret");
        assert!(rel.contains("environment: release"));
        for name in ["RELEASE_CERT_P12_BASE64", "RELEASE_CERT_PASSWORD"] {
            assert!(rel.contains(&format!("secrets.{name}")) && include_str!("../../../../docs/RELEASING.md").contains(name), "{name}");
        }
    }

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn dev_flags_and_launch_provenance() {
        let f = DevFlags::from_args(&args("shroom --home /tmp/h --cloud-fake http://a/client/v4 http://a/s3 --drive onboarding /tmp/o.json"));
        assert_eq!(f.home, Some(PathBuf::from("/tmp/h")));
        assert_eq!(f.cloud_fake, Some(("http://a/client/v4".into(), "http://a/s3".into())));
        assert!(f.any());
        assert!(!DevFlags::from_args(&args("shroom")).any());
        assert!(DevFlags::from_args(&args("shroom --cloud-fake only-one")).cloud_fake.is_none());
        assert!(DevFlags::from_args(&args("shroom --live-shell")).any());
        assert_eq!(launch_kind(&args("shroom"), false), LaunchKind::User);
        assert_eq!(launch_kind(&args("shroom --relaunched=permission"), false), LaunchKind::PermissionRelaunch);
        assert_eq!(launch_kind(&args("shroom --relaunched=update"), false), LaunchKind::UpdateRelaunch);
        assert_eq!(launch_kind(&args("shroom --invite"), false), LaunchKind::Install { join: true }, "the installer's hand-off: no secret on the line");
    }

    #[test]
    fn the_installers_and_the_login_items_launches_are_told_apart() {
        // The install script's fresh install is the first launch; its update lane is silent.
        assert_eq!(launch_kind(&args("shroom --relaunched=install"), false), LaunchKind::Install { join: false });
        assert_eq!(launch_kind(&args("shroom --relaunched=install-update"), false), LaunchKind::InstallUpdate);
        // A join link handed over by file opens the invite, on either lane (N9's `--join`).
        assert_eq!(launch_kind(&args("shroom --relaunched=install --invite"), false), LaunchKind::Install { join: true });
        assert_eq!(launch_kind(&args("shroom --relaunched=install-update --invite"), false), LaunchKind::Install { join: true });
        // The login item passes no arguments: the launch event's mark says it.
        assert_eq!(launch_kind(&args("shroom"), true), LaunchKind::LoginItem);
        assert_eq!(launch_kind(&args("shroom --relaunched=update"), true), LaunchKind::UpdateRelaunch, "a relaunch's own flag wins");
        // Nothing near the flags is taken for them.
        assert_eq!(launch_kind(&args("shroom --relaunched=installer"), false), LaunchKind::User);
        assert_eq!(launch_kind(&args("shroom --relaunched=install-updated"), false), LaunchKind::User);
    }

    #[test]
    fn the_drive_records_only_the_display_the_pattern_covers() {
        use shroom_core::world::{Device, Display, Door, SourceId};
        let mut w = shroom_core::world::World::blank();
        w.space.identity = Some(shroom_core::world::SpaceIdentity {
            account_id: "a".into(),
            label: "v".into(),
            subdomain: "s".into(),
            host: "v.s.workers.dev".into(),
            bucket: "b".into(),
            role: shroom_core::world::Role::TokenHolder,
            member_id: "m1".into(),
            owner_name: "B".into(),
            account_name: None,
            foreign_v: false,
        });
        w.space.token = true;
        w.mac.primed = Some(Wall(1));
        w.mac.screen = shroom_core::time::Fact::known(true, Mono(0));
        w.mac.displays = shroom_core::time::Fact::known(vec![Display { id: 1, name: "Built-in".into(), is_main: true, x: 0, y: 0 }, Display { id: 7, name: "LG".into(), is_main: false, x: 0, y: 0 }], Mono(0));
        w.mac.cameras = shroom_core::time::Fact::known(vec![Device { uid: "cam".into(), name: "FaceTime".into(), builtin: true, continuity: false, system_default: false }], Mono(0));
        w.session.popover = Some(Door::asked(Mono(0)));
        for (pick, ok) in [(SourceId::Display(1), true), (SourceId::Display(7), false), (SourceId::Camera, false)] {
            w.prefs.source = Some(pick);
            let s = shroom_core::derive(&w);
            assert_eq!(records(&s), Some(pick));
            assert_eq!(guard_source(&s, 1).is_ok(), ok, "{pick:?}");
        }
    }

    #[test]
    fn harness_modes_run_only_in_a_dev_build_with_the_harness() {
        for flag in HARNESS_FLAGS {
            let a = args(&format!("shroom {flag} /tmp/x.json"));
            assert!(harness_refusal(&a, false, true).is_some(), "{flag}: never in a release identifier");
            assert!(harness_refusal(&a, false, false).is_some(), "{flag}");
            assert!(harness_refusal(&a, true, false).is_some(), "{flag}: a dev build without the harness says so");
            assert_eq!(harness_refusal(&a, true, true), None, "{flag}");
        }
        assert_eq!(harness_refusal(&args("shroom --relaunched=install"), false, false), None, "the app's own flags aren't the harness's");
        assert_eq!(harness_refusal(&args("--drive"), false, false), None, "argv[0] is the program");
    }

    #[test]
    fn the_run_log_is_private_rolls_over_and_names_recordings_short() {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let tmp = tempfile::tempdir().unwrap();
        let logs = tmp.path().join("logs");
        log_line(&logs, "one\n");
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!((mode(&logs), mode(&logs.join("run.log"))), (0o700, 0o600));
        // A log — and its folder — from before the rule are closed on the next line.
        std::fs::set_permissions(logs.join("run.log"), std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(&logs, std::fs::Permissions::from_mode(0o755)).unwrap();
        log_line(&logs, "two\n");
        assert_eq!((mode(&logs), mode(&logs.join("run.log"))), (0o700, 0o600));
        // A folder that already existed loose (made before the app ever wrote to it).
        let old = tmp.path().join("old-logs");
        std::fs::DirBuilder::new().mode(0o755).create(&old).unwrap();
        std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o755)).unwrap();
        log_line(&old, "one\n");
        assert_eq!((mode(&old), mode(&old.join("run.log"))), (0o700, 0o600));
        // Past the roll size: one old log kept, a fresh one begun.
        std::fs::write(logs.join("run.log"), vec![b'x'; LOG_ROLL_BYTES as usize]).unwrap();
        log_line(&logs, "three\n");
        assert!(std::fs::read_to_string(logs.join("run.log")).unwrap().ends_with("Z three\n"));
        assert_eq!(std::fs::metadata(logs.join("run.log.1")).unwrap().len(), LOG_ROLL_BYTES);
        log_line(&logs, "four\n");
        assert!(!logs.join("run.log.2").exists());
        assert_eq!(short("AbCdEfGhIjKlMnOp"), "AbCd…");
        assert_eq!(short("ab"), "ab");
    }

    /// A tick that changed nothing outward isn't logged; every other step is.
    #[test]
    fn a_tick_that_asks_for_nothing_stays_out_of_the_run_log() {
        let arm = shroom_core::Arm::named("tick");
        assert_eq!(core_line(true, &arm, &[]), None);
        assert_eq!(core_line(false, &shroom_core::Arm::named("click-start"), &[]).as_deref(), Some("core click-start []"));
        let fx = [Effect::Persist(PersistOp::Wipe)];
        assert!(core_line(true, &arm, &fx).is_some_and(|l| l.starts_with("core tick [")));
    }

    /// Every line carries the UTC wall clock; the boot's first line names the build, and so
    /// does the first line of a fresh log after a roll — any tail of it says who wrote it.
    #[test]
    fn the_run_log_stamps_each_line_and_names_its_build_at_boot_and_after_a_roll() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("home"), None, None);
        log_boot(&rt, false, true, LaunchKind::User);
        rt.log("a line");
        let logs = rt.home.root.join("logs");
        let log = std::fs::read_to_string(logs.join("run.log")).unwrap();
        let stamped = |l: &str| {
            let (stamp, _) = l.split_at(20);
            let b = stamp.as_bytes();
            stamp.ends_with('Z') && b[4] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':' && l.as_bytes()[20] == b' '
        };
        assert!(log.lines().all(stamped), "{log}");
        let first = log.lines().next().unwrap();
        assert!(first.contains(&format!(" boot: shroom {}", rt.version)), "{first}");
        assert!(log.lines().last().unwrap().ends_with(" a line"));
        // Past the roll size, the fresh log opens with the build.
        std::fs::write(logs.join("run.log"), vec![b'x'; LOG_ROLL_BYTES as usize]).unwrap();
        rt.log("after the roll");
        let fresh = std::fs::read_to_string(logs.join("run.log")).unwrap();
        let lines: Vec<&str> = fresh.lines().collect();
        assert_eq!(lines.len(), 2, "{fresh}");
        assert!(lines.iter().all(|l| stamped(l)), "{fresh}");
        assert!(lines[0].ends_with(&format!(" rolled: {}", BUILD.get().unwrap())) && lines[0].contains(" rolled: shroom "), "{fresh}");
        assert!(lines[1].ends_with(" after the roll"));
    }

    /// The harness's live drive reads the run log back: each line's monotonic ms and its
    /// text, past the wall clock it opens with, as a line from before the stamps reads —
    /// the drive waits on these to see the page opened after Stop.
    #[test]
    fn a_run_log_line_reads_back_its_ms_and_text_with_or_without_its_wall_clock() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("home"), None, None);
        rt.log("cloud would open the owner page for AbCd…");
        let log = std::fs::read_to_string(rt.home.root.join("logs/run.log")).unwrap();
        let stamped = log.lines().last().unwrap();
        assert!(matches!(read_log_line(stamped), Some((_, "cloud would open the owner page for AbCd…"))), "{stamped}");
        assert_eq!(read_log_line("2026-10-18T18:40:03Z    19175 recorder: finalized in 412 ms"), Some((19175, "recorder: finalized in 412 ms")));
        // A line from before the stamps reads the same.
        assert_eq!(read_log_line("   19175 recorder: finalized in 412 ms"), Some((19175, "recorder: finalized in 412 ms")));
        // A line with no ms (the trace ring's, a stamp alone) is none of the drive's.
        for l in ["2026-10-18T18:40:03Z trace     1535 Booted.Disk → first-launch", "trace: end", "2026-10-18T18:40:03Z", ""] {
            assert_eq!(read_log_line(l), None, "{l}");
        }
    }

    /// Law 13 at the door: of a pasted token the core is told its handle and its shape,
    /// nothing else — so nothing the Scene says of a held paste (the field's dots,
    /// DECISIONS B104) can carry the text or its length. Two tokens of different lengths
    /// are the same paste to the core, and to the run log.
    #[test]
    fn a_pasted_token_reaches_the_core_as_a_handle_and_a_shape_whatever_its_text_or_length() {
        let tmp = tempfile::tempdir().unwrap();
        let hear = |home: &str, token: &str| {
            let (rt, rx) = Runtime::for_test(tmp.path().join(home), None, None);
            rt.paste_secret(SecretField::Token, token.to_string());
            let event = format!("{:?}", rx.try_recv().expect("the paste was posted").event);
            assert!(rx.try_recv().is_err(), "one event a paste");
            // Each line after its wall clock and its monotonic stamp.
            let log = std::fs::read_to_string(rt.home.root.join("logs").join("run.log")).expect("the paste was logged");
            let log: Vec<String> = log.lines().map(|l| l.split_whitespace().skip(2).collect::<Vec<_>>().join(" ")).collect();
            (event, log)
        };
        let (short, long) = (format!("cfut_{}", "a".repeat(35)), format!("cfat_{}", "Zx".repeat(48)));
        assert_eq!((short.len(), long.len()), (40, 101));
        let (a, b) = (hear("a", &short), hear("b", &long));
        assert_eq!(a, b, "the same paste, whichever token it was");
        assert_eq!(a.0, "SecretPasted { field: Token, id: SecretId(1), shape: Token }");
        assert_eq!(a.1, ["paste 1 into Token: Token"]);
    }

    #[test]
    fn the_e2e_run_reaches_its_test_account_and_nothing_else() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = Runtime::for_test(tmp.path().join("a"), None, None);
        rt.e2e_account = Some("0123456789abcdef0123456789abcdef".into());
        assert!(rt.e2e());
        // Its own space: the page, the Worker, the storage.
        assert_eq!(rt.page_base("v.shroom-e2e.workers.dev"), "https://v.shroom-e2e.workers.dev");
        assert!(rt.space("v.shroom-e2e.workers.dev").is_some());
        assert!(rt.bucket_allowed("shroom-space"));
        // Any other real page stays out of reach, as in every dev build.
        for other in ["v.myshroom.workers.dev", "shroom-e2e.workers.dev", "a.b.shroom-e2e.workers.dev", "v.shroom-e2e.workers.dev.evil.dev"] {
            assert_eq!(rt.page_base(other), "http://127.0.0.1:9", "{other}");
            assert!(rt.space(other).is_none(), "{other}");
        }
        // And its client is pinned: another account is refused before any request.
        let c = rt.cloud("cfat_x");
        assert!(matches!(c.list_buckets("fedcba9876543210fedcba9876543210"), Err(shroom_cloud::CloudError::Refused(_))));
    }

    /// Runs side by side on the one test account (D3, B131): an `--e2e` run sets up under
    /// its own space id — `v-<run>` over `shroom-space-<run>`, as a pair — and every other
    /// build keeps the product's names, whatever is on disk.
    #[test]
    fn an_e2e_run_sets_up_its_own_space_and_every_other_build_the_products() {
        use shroom_cloud::wire::SpaceNames;
        const RUN: &str = "1792000000-ab12";
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = Runtime::for_test(tmp.path().join("a"), None, None);
        rt.e2e_space = Some(RUN.into());
        assert_eq!(rt.space_names(), SpaceNames::product(), "a space id without --e2e changes nothing");
        rt.e2e_account = Some("0123456789abcdef0123456789abcdef".into());
        assert_eq!(rt.space_names(), SpaceNames::of_run(RUN).unwrap());
        // The run's page passes the e2e fence; its storage is reachable.
        let host = format!("v-{RUN}.shroom-e2e.workers.dev");
        assert_eq!(rt.page_base(&host), format!("https://{host}"));
        assert!(rt.space(&host).is_some());
        assert!(rt.bucket_allowed(&format!("shroom-space-{RUN}")));
        // A release build has no harness: the defaults, whatever the fields say.
        rt.dev = false;
        assert_eq!(rt.space_names(), SpaceNames::product());
        // The id is the run's file beside the account id's, read only when well formed.
        let dir = tmp.path().join("dev");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(e2e_space_id_in(&dir), None, "no file");
        std::fs::write(dir.join("e2e-space-id"), format!("{RUN}\n")).unwrap();
        assert_eq!(e2e_space_id_in(&dir).as_deref(), Some(RUN));
        for bad in ["v", "shroom-space", "1792000000-AB12", "../1792000000-ab12", ""] {
            std::fs::write(dir.join("e2e-space-id"), bad).unwrap();
            assert_eq!(e2e_space_id_in(&dir), None, "{bad:?}");
        }
    }

    #[test]
    fn the_stops_reach_probe_is_fenced_like_the_space() {
        let tmp = tempfile::tempdir().unwrap();
        // The drives' fake: its own pages.
        let (rt, _rx) = Runtime::for_test(tmp.path().join("a"), Some(("http://127.0.0.1:1/client/v4".into(), "http://127.0.0.1:2".into())), None);
        assert_eq!(rt.page_base("v.myshroom.workers.dev"), "http://127.0.0.1:1/page/v.myshroom.workers.dev");
        // No stand-in: a real space is never pinged from a dev build; a test page is.
        let (rt, _rx) = Runtime::for_test(tmp.path().join("b"), None, None);
        assert_eq!(rt.page_base("v.myshroom.workers.dev"), "http://127.0.0.1:9");
        assert_eq!(rt.page_base("shroom-test-x.a.workers.dev"), "https://shroom-test-x.a.workers.dev");
        let (rt, _rx) = Runtime::for_test(tmp.path().join("c"), None, Some("http://127.0.0.1:8787".into()));
        assert_eq!(rt.page_base("v.a.workers.dev"), "http://127.0.0.1:8787/v.a.workers.dev");
    }

    #[test]
    fn wirings_ping_reaches_only_the_stand_ins_in_a_dev_build() {
        assert_eq!(page_base(false, None, None, "v.myshroom.workers.dev"), None, "a release build: the real page");
        assert_eq!(page_base(true, Some("http://127.0.0.1:8787/"), Some("http://127.0.0.1:1/client/v4"), "v.a.workers.dev").as_deref(), Some("http://127.0.0.1:8787/v.a.workers.dev"));
        assert_eq!(page_base(true, None, Some("http://127.0.0.1:1/client/v4"), "v.a.workers.dev").as_deref(), Some("http://127.0.0.1:1/page/v.a.workers.dev"));
        assert_eq!(page_base(true, None, None, "shroom-test-x.a.workers.dev"), None, "the test fence's page");
        assert_eq!(page_base(true, None, None, "v.a.workers.dev").as_deref(), Some("http://127.0.0.1:9"), "never a real space from a dev build");
    }

    #[test]
    fn the_fake_is_taken_only_toward_this_mac_and_with_a_throwaway_home() {
        let ok = DevFlags::from_args(&args("shroom --home /tmp/h --cloud-fake http://127.0.0.1:5001/client/v4 http://localhost:5002"));
        assert_eq!(ok.refusal(), None);
        assert!(ok.fake().is_some());
        let no_home = DevFlags::from_args(&args("shroom --cloud-fake http://127.0.0.1:1/client/v4 http://127.0.0.1:2"));
        assert!(no_home.refusal().is_some() && no_home.fake().is_none());
        let real = DevFlags::from_args(&args("shroom --home /tmp/h --cloud-fake https://api.cloudflare.com/client/v4 https://acc.r2.cloudflarestorage.com"));
        assert!(real.refusal().is_some() && real.fake().is_none());
        let half = DevFlags::from_args(&args("shroom --home /tmp/h --cloud-fake http://127.0.0.1:1/client/v4 https://acc.r2.cloudflarestorage.com"));
        assert!(half.refusal().is_some());
        // The local Worker stand-in: this Mac only, a throwaway home only.
        let base = DevFlags::from_args(&args("shroom --home /tmp/h --space-base http://127.0.0.1:8787"));
        assert_eq!((base.refusal(), base.fake_space_base().as_deref()), (None, Some("http://127.0.0.1:8787")));
        assert!(base.any());
        for bad in ["shroom --space-base http://127.0.0.1:8787", "shroom --home /tmp/h --space-base https://v.myshroom.workers.dev"] {
            let f = DevFlags::from_args(&args(bad));
            assert!(f.refusal().is_some() && f.fake_space_base().is_none(), "{bad}");
        }
        for good in ["http://127.0.0.1", "http://127.0.0.1:8787/x", "https://localhost:1", "http://[::1]:9/a"] {
            assert!(loopback(good), "{good}");
        }
        for bad in ["127.0.0.1", "http://127.0.0.1.evil.com", "http://localhost.evil.com/", "http://evil.com/127.0.0.1", "http://user@127.0.0.1@evil.com", "http://[::1].evil.com", "ftp://127.0.0.1", "http://10.0.0.1"] {
            assert!(!loopback(bad), "{bad}");
        }
    }

    #[test]
    fn updates_in_a_dev_build_reach_only_this_mac_and_swap_only_a_throwaway() {
        let home = std::env::var("HOME").unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path().display().to_string();
        std::fs::create_dir_all(tmp.path().join("x/shroom-dev.app")).unwrap();
        let ok = DevFlags::from_args(&args(&format!("shroom --home {t}/h --releases http://127.0.0.1:9/rel --update-target {t}/x/shroom-dev.app --as-version 0.0.1")));
        assert_eq!(ok.refusal(), None);
        assert!(ok.any());
        // The swap takes the resolved path (`/private/var/…`), the same folder.
        let r = throwaway_app(&tmp.path().join("x/shroom-dev.app")).expect("a throwaway copy");
        assert!(r.starts_with("/private/") && identity(&r) == identity(&tmp.path().join("x/shroom-dev.app")));
        assert!(DevFlags::from_args(&args("shroom --releases https://github.com/bagrat/shroom/releases")).refusal().is_some(), "never the real releases");
        assert!(DevFlags::from_args(&args(&format!("shroom --update-target {t}/x/shroom-dev.app"))).refusal().is_some(), "only from a throwaway home");
        assert!(DevFlags::from_args(&args(&format!("shroom --home {t}/h --as-version banana"))).refusal().is_some());
        // Links into the Applications folders, made in the temp folder (nothing is touched).
        std::os::unix::fs::symlink(format!("{home}/Applications"), tmp.path().join("apps")).unwrap();
        std::os::unix::fs::symlink("/Applications/Safari.app", tmp.path().join("safari.app")).unwrap();
        let mut bad = vec![
            "/Applications/shroom.app".to_string(),
            "/Applications/Safari.app".into(),
            // The disk ignores letter case; the guard doesn't get fooled by it.
            "/APPLICATIONS/Safari.app".into(),
            "/applications/Safari.app".into(),
            format!("{home}/Applications/shroom-dev.app"),
            format!("{home}/applications/shroom-dev.app"),
            format!("{home}/APPLICATIONS/shroom-dev.app"),
            format!("{}/Applications/shroom-dev.app", home.to_uppercase()),
            format!("{home}/.shroom-dev/x.app"),
            format!("{home}/.SHROOM-dev/x.app"),
            format!("{home}/x.app"),
            format!("{t}/apps/shroom-dev.app"),
            format!("{t}/safari.app"),
            format!("{t}/x/missing.app"),
            "relative/shroom.app".into(),
            format!("{t}/x/../../Applications/shroom.app"),
            format!("{t}/x/shroom-dev"),
            "/x.app".into(),
            "/tmp".into(),
        ];
        // The boot volume by its other name (`/Volumes/Macintosh HD` → `/`).
        for v in std::fs::read_dir("/Volumes").into_iter().flatten().flatten() {
            if std::fs::canonicalize(v.path()).is_ok_and(|c| c == std::path::Path::new("/")) {
                bad.push(format!("{}{home}/Applications/shroom-dev.app", v.path().display()));
            }
        }
        for bad in bad {
            assert!(throwaway_app(std::path::Path::new(&bad)).is_none(), "{bad}");
            let f = DevFlags::from_args(&args(&format!("shroom --home {t}/h --update-target {bad}")));
            assert!(f.refusal().is_some(), "{bad}");
        }
        assert_eq!(launch_kind(&args("shroom --relaunched=update-failed"), false), LaunchKind::UpdateFailed);
    }

    #[test]
    fn a_dev_build_reaches_a_bucket_over_s3_only_inside_the_test_fence() {
        assert!(bucket_allowed(false, false, "shroom-space"), "a release build: its own space");
        assert!(bucket_allowed(true, true, "shroom-space"), "the L1 fake");
        assert!(bucket_allowed(true, false, "shroom-test-dev-1790905072"));
        assert!(!bucket_allowed(true, false, "shroom-space"), "never the product bucket on a real account");
        assert!(!bucket_allowed(true, false, "SHROOM-TEST-x") && !bucket_allowed(true, false, "x-shroom-test-"));
    }

    /// The checkpoint mode is retired (B150): `--checkpoint` is no flag, so a dev build given
    /// it is a plain dev build — its own home, a quiet shell, the test fence on every reach.
    #[test]
    fn a_dev_build_runs_only_under_the_test_fence_whatever_it_is_given() {
        use shroom_cloud::CloudError;
        let tmp = tempfile::tempdir().unwrap();
        let once = DevFlags::from_args(&args("shroom --checkpoint"));
        assert!(!once.any() && once.refusal().is_none(), "not a flag: nothing turns on");
        assert!(home_for(&once, true, "am.shroom.dev").unwrap().root.ends_with(".shroom-dev"), "the dev home");
        assert!(home_for(&DevFlags::default(), false, "am.shroom").unwrap().root.ends_with(".shroom"));

        let (plain, _rx) = Runtime::for_test(tmp.path().join("plain"), None, None);
        let fenced = plain.cloud("t").with_endpoints("http://127.0.0.1:9/client/v4", "http://127.0.0.1:9");
        for r in [fenced.create_bucket("acct", "shroom-space"), fenced.delete_worker("acct", "v"), fenced.claim_subdomain("acct", "myshroom")] {
            assert!(matches!(r, Err(CloudError::Refused(_))), "{r:?}");
        }
        assert!(!plain.bucket_allowed("shroom-space") && !plain.bucket_allowed("shroom"));
        assert!(plain.space("shroom.myshroom.workers.dev").is_none() && plain.space("v.myshroom.workers.dev").is_none());
        assert_eq!(plain.page_base("shroom.myshroom.workers.dev"), "http://127.0.0.1:9");
        assert!(shell_quiet(false, &plain), "a plain dev run stays quiet");
        assert!(shell_quiet(true, &plain), "a harness run is quiet whatever it says");

        log_boot(&plain, false, true, LaunchKind::LoginItem);
        let log = std::fs::read_to_string(plain.home.root.join("logs/run.log")).unwrap();
        assert!(log.contains("boot: launch login") && log.contains("boot: home default · shell quiet · cloud test fence"), "{log}");
        assert!(!log.contains(&tmp.path().display().to_string()), "no paths: {log}");
        // Every launch has its own word (the login one is what docs/CHECKPOINT.md looks for).
        let kinds = [LaunchKind::User, LaunchKind::LoginItem, LaunchKind::UpdateRelaunch, LaunchKind::PermissionRelaunch, LaunchKind::Install { join: false }, LaunchKind::Install { join: true }, LaunchKind::InstallUpdate, LaunchKind::UpdateFailed];
        let words: std::collections::HashSet<_> = kinds.iter().map(|k| launch_word(*k)).collect();
        assert_eq!(words.len(), kinds.len());
    }

    /// What a build is comes from its identifier alone — the same answer from Spotlight, at
    /// login and after a relaunch: a `….dev` identifier is a dev build, anything else the
    /// release (the preview flavor is retired, B148).
    #[test]
    fn the_identifier_alone_says_what_a_build_is() {
        let conf = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();
        let (dev, rel) = (conf(include_str!("../../tauri.conf.json")), conf(include_str!("../../tauri.release.conf.json")));
        assert_eq!(Flavor::of(dev["identifier"].as_str().unwrap()), Flavor::Dev);
        assert_eq!(Flavor::of(rel["identifier"].as_str().unwrap()), Flavor::Release);
        for (id, want) in [("am.shroom", Flavor::Release), ("am.shroom.dev", Flavor::Dev), ("am.shroom.preview", Flavor::Release), ("am.shroom.preview.dev", Flavor::Dev), ("", Flavor::Release)] {
            assert_eq!(Flavor::of(id), want, "{id:?}");
        }
        assert!(Flavor::Dev.dev() && !Flavor::Release.dev());
        assert_eq!([Flavor::Release.home_dir(), Flavor::Dev.home_dir()], [".shroom", ".shroom-dev"]);
        // `main` and the runner both ask the identifier, and nothing else.
        assert!(include_str!("../main.rs").contains("let dev = runtime::Flavor::of(&context.config().identifier).dev();"));
        let me = include_str!("mod.rs");
        let start = me.split("fn start_inner(").nth(1).unwrap().split("\nfn ").next().unwrap();
        assert!(start.contains("let flavor = Flavor::of(&identifier);") && start.contains("let dev = flavor.dev();"), "start_inner reads the flavor from the identifier");
        // One release config: the preview's is gone.
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(!src.join("tauri.preview.conf.json").exists(), "the preview build is retired (B148)");
    }

    /// TESTING-PLAN's quiet guard: no unattended run plays sound through this Mac. Tart hands
    /// a guest's sound and its microphone through to the Mac it runs on unless the VM is
    /// started with `--no-audio` — so every `tart run` in the tree's scripts carries it. The
    /// VMs Bagrat sits at (the checkpoint's Tart clones, CHECKPOINT step 10) he starts by
    /// hand from the doc, never from a script here.
    #[test]
    fn every_vm_a_script_starts_has_its_sound_off() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut stack: Vec<PathBuf> = ["tests", "tools", "app/test", "worker/dev", "worker/test", "installer", ".github"].iter().map(|d| root.join(d)).collect();
        // Every line that starts a VM: (the file, the line), comments left out.
        let mut launches: Vec<(String, String)> = Vec::new();
        while let Some(p) = stack.pop() {
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if m.is_dir() {
                if p.file_name().is_some_and(|n| n == "node_modules" || n == "target") {
                    continue;
                }
                stack.extend(std::fs::read_dir(&p).into_iter().flatten().flatten().map(|e| e.path()));
            } else if let Ok(text) = std::fs::read_to_string(&p) {
                let file = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
                for l in text.lines().map(str::trim).filter(|l| !l.starts_with('#') && !l.starts_with("//")) {
                    let words: Vec<&str> = l.split(|c: char| c.is_whitespace() || c == ';' || c == '(' || c == '|' || c == '&').filter(|w| !w.is_empty()).collect();
                    if words.windows(2).any(|w| w == ["tart", "run"]) {
                        launches.push((file.clone(), l.to_string()));
                    }
                }
            }
        }
        launches.sort();
        // The walk found the lanes that are known to start one (it isn't passing on nothing).
        for lane in ["tests/l5/installer-vm.sh", "tests/l5/dock-vm.sh", "tests/l5/surfaces-vm.sh", "tests/l5/take-vm.sh", "tests/l5/update-vm.sh", "tests/l5/setup-vm.sh"] {
            assert!(launches.iter().any(|(f, _)| f == lane), "{lane} starts a VM: {launches:?}");
        }
        // Nobody sits at a VM a script starts: none has this Mac's sound.
        for (file, line) in &launches {
            assert!(line.contains("--no-audio"), "{file} starts a VM with this Mac's sound and microphone: {line}");
        }
        // The plan says it where it says the rest of the guard.
        let plan = std::fs::read_to_string(root.join("docs/TESTING-PLAN.md")).unwrap();
        let guard = plan.split("\n\n").find(|p| p.starts_with("**Quiet guard for unattended runs:**")).expect("TESTING-PLAN's quiet guard");
        assert!(guard.contains("`--no-audio`"), "the quiet guard says how a VM is started: {guard}");
    }

    #[test]
    fn the_checkpoint_script_tells_the_truth_about_the_build() {
        // docs/CHECKPOINT.md is walked by hand: a wrong expectation there reads as a failure of
        // a right build. Its labels are the core's copy, its log lines the runner's, its
        // commands the official release's, and its expected outcomes the core's
        // (region3::quit_is_declined_mid_take, law 4). It walks the from-scratch flow on the
        // official release, the only shroom on Bagrat's Mac (CLAUDE.md), with the stranger's
        // run in Tart clones (step 10).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |rel: &str| std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let doc = read("docs/CHECKPOINT.md");
        let flat = doc.split_whitespace().collect::<Vec<_>>().join(" ");
        use shroom_core::copy;
        // The update step names the version one patch above the tree's: the next release.
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let cur = shroom_update::Version::parse(conf["version"].as_str().unwrap()).unwrap();
        let next = shroom_update::Version::new(cur.0, cur.1, cur.2 + 1).to_string();
        // Not Camera: it's the camera row's accessible name, shown nowhere.
        for label in [
            copy::login_box(),
            copy::tab_general(),
            copy::tab_space(),
            copy::tab_members(),
            copy::copy_link(),
            copy::who_for(),
            copy::invite_someone(),
            copy::invite(),
            copy::library(),
            copy::update(),
            copy::whats_new(),
            copy::update_ready(&next),
            copy::updating(),
            copy::updated(&next),
            copy::up_to_date(&next),
        ] {
            assert!(flat.contains(&label), "docs/CHECKPOINT.md should name {label:?} as shroom says it");
        }
        // Every runtime file's log lines; a grep may name a line by its opening words. Each
        // reads the release's own log.
        let src: String = std::fs::read_dir(root.join("app/src-tauri/src/runtime")).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "rs")).map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
        let kinds = [LaunchKind::User, LaunchKind::LoginItem, LaunchKind::UpdateRelaunch, LaunchKind::PermissionRelaunch, LaunchKind::Install { join: false }, LaunchKind::Install { join: true }, LaunchKind::InstallUpdate, LaunchKind::UpdateFailed];
        let grep_lines: Vec<&str> = doc.lines().filter(|l| l.starts_with("grep '")).collect();
        let greps: Vec<&str> = grep_lines.iter().map(|l| l.strip_prefix("grep '").unwrap().split('\'').next().unwrap()).collect();
        for want in ["boot: launch login", "power: macOS will sleep", "updater staged", "updater not applied"] {
            assert!(greps.contains(&want), "docs/CHECKPOINT.md should grep for {want:?}: {greps:?}");
        }
        assert!(!greps.iter().any(|g| g.starts_with("boot: checkpoint")), "the sitting runs the official release");
        for line in &greps {
            let logged = match line.strip_prefix("boot: launch ") {
                Some(word) => kinds.iter().any(|k| launch_word(*k) == word),
                None => src.contains(&format!("\"{line}")),
            };
            assert!(logged, "docs/CHECKPOINT.md greps for {line:?}, which the runner never logs");
        }
        let log = format!("~/{}/logs/run.log", Flavor::Release.home_dir());
        for l in &grep_lines {
            assert!(l.ends_with(&format!("' {log}")), "{l} should read the release's log, {log}");
        }

        // Each command sits alone in its own block: the official install line, the app, and
        // Tart — nothing from the tree runs on his Mac.
        let blocks: Vec<&str> = doc.split("```bash\n").skip(1).map(|b| b.split("```").next().unwrap().trim_end()).collect();
        assert!(blocks.len() >= 10, "{blocks:?}");
        let tart = |b: &&str| b.lines().all(|l| l.starts_with("tart ") || l.starts_with("mkdir -p ~/shroom-handoff") || l.starts_with("rm -rf ~/shroom-handoff") || l.starts_with("pbpaste > "));
        assert!(blocks.iter().all(|b| b.lines().count() == 1 || tart(b)), "one command to a block, Tart's set apart: {blocks:?}");
        let has = |cmd: &str| blocks.iter().any(|b| b.lines().any(|l| l == cmd));
        for cmd in [
            "curl -fsSL https://shroom.am/install | sh -s -- --uninstall",
            "open ~/Applications/shroom.app",
            "tart clone shroom-clean-tahoe shroom-owner",
            "tart clone shroom-clean-tahoe shroom-friend",
            "tart delete shroom-owner",
            "tart delete shroom-friend",
        ] {
            assert!(has(cmd), "docs/CHECKPOINT.md should carry `{cmd}` in a block");
        }
        assert!(flat.contains("curl -fsSL https://shroom.am/install | sh"), "the install line, as the landing shows it");
        assert!(read("site/index.html").contains("curl -fsSL https://shroom.am/install | sh</code>"));
        assert!(read("installer/install.sh").contains("if [ \"${1:-}\" = \"--uninstall\" ]; then uninstall; return; fi"), "the install line takes --uninstall (B135)");
        for b in &blocks {
            assert!(!b.contains("tests/") && !b.contains("cargo ") && !b.contains("dev/"), "nothing from the tree runs on his Mac: {b}");
        }
        assert!(!doc.contains("--args"), "nothing in the sitting opens the app with a flag");
        assert!(!doc.contains("shroom-dev.app") && !doc.contains("~/.shroom-dev"), "the sitting never touches a dev build");
        assert!(blocks.iter().all(|b| !b.starts_with("rm -rf ~/.shroom") && !b.contains("rm -rf ~/Applications")), "the uninstall line removes shroom, never a hand-typed rm");
        // What the installer says, as the doc quotes it.
        let installer = read("installer/install.sh");
        for said in ["Getting shroom…", "verified ✓", "Installed — opening shroom now."] {
            assert!(flat.contains(said) && installer.contains(said), "the installer says {said:?}");
        }
        assert!(read("site/index.html").contains(">Read it first</a>") && flat.contains("\"Read it first\""));

        // Step 5's name row: the uninstall leaves the space, so setup finds `v` again.
        {
            use shroom_cloud::wire;
            let step5 = flat.split("## 5. First launch").nth(1).and_then(|s| s.split("## 6. ").next()).expect("step 5");
            let host = format!("{}.myshroom.workers.dev", wire::LABEL);
            let row = copy::row_done(4, Some(&copy::name_fact(&host, false)));
            assert!(step5.contains(&format!("\"{row}\"")), "step 5 should quote the name row as shroom shows it — {row:?}: {step5}");
            assert!(!step5.contains("as taken"), "{step5}");
        }

        // The Dock icon iff a window of shroom's is open (law 6, SM:133): step 8 says so,
        // and sends the tester to a full-screen app to see the windows open on a desktop.
        let step8 = flat.split("## 8. The same app, every time").nth(1).and_then(|s| s.split("## 9. ").next()).expect("step 8");
        let dock = step8.split("- The Dock icon:").nth(1).and_then(|s| s.split("- ⌘Q, mid-take").next()).expect("step 8's The Dock icon");
        assert!(dock.contains("only while one of its windows is open") && dock.contains("should appear in the Dock"), "{dock}");
        assert!(dock.contains("from the mushroom's popover") && dock.contains("Press ⌘W") && dock.contains("the Dock icon goes"), "the way back to a window, and its keys: {dock}");
        assert!(dock.contains("Over full screen, the windows") && dock.contains("on a desktop, not over the full-screen app"), "{dock}");
        // ⌘Q mid-take is declined silently (B45 b, law 4): no question is expected.
        let quit = step8.split("- ⌘Q, mid-take").nth(1).and_then(|s| s.split("- The notch").next()).expect("step 8's ⌘Q");
        assert!(quit.contains("keeps recording") && quit.contains("paused popover should come back"), "{quit}");
        assert!(!quit.contains("should ask"), "⌘Q mid-take never asks: {quit}");
        // Step 10's invite is made from the owner's wizard and crosses to the friend.
        let step10 = flat.split("## 10. ").nth(1).and_then(|s| s.split("## 11. ").next()).expect("step 10");
        assert!(step10.contains(&copy::invite_someone()) && step10.contains(&copy::who_for()) && step10.contains(&copy::copy_link()), "{step10}");
        // Disconnect is real on the release: the doc says so, once.
        assert!(flat.contains("Disconnect is real here") && !flat.contains("aren't real here"));
    }

    #[test]
    fn a_throwaway_home_is_a_temp_folder_never_a_shroom_home() {
        let home = std::env::var("HOME").unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path().display().to_string();
        std::os::unix::fs::symlink(format!("{home}/.shroom-dev"), tmp.path().join("dev-home")).unwrap();
        std::os::unix::fs::symlink(&home, tmp.path().join("me")).unwrap();
        for good in [format!("{t}/h"), format!("{t}/a/b/c"), "/tmp/shroom-test-home".into()] {
            assert!(throwaway_place(std::path::Path::new(&good)).is_some(), "{good}");
            assert_eq!(DevFlags::from_args(&args(&format!("shroom --home {good}"))).refusal(), None, "{good}");
        }
        for bad in [
            format!("{home}/.shroom"),
            format!("{home}/.shroom-dev"),
            format!("{home}/.SHROOM"),
            format!("{home}/.Shroom-Dev"),
            format!("{home}/.shroom/sub"),
            home.clone(),
            format!("{home}/somewhere"),
            format!("{t}/dev-home"),
            format!("{t}/dev-home/sub"),
            format!("{t}/me/.shroom"),
            "/tmp/".into(),
            "/private/tmp".into(),
            "/private/var/folders".into(),
            "relative/home".into(),
            format!("{t}/../x"),
        ] {
            assert!(throwaway_place(std::path::Path::new(&bad)).is_none(), "{bad}");
            assert!(DevFlags::from_args(&args(&format!("shroom --home {bad}"))).refusal().is_some(), "{bad}");
        }
    }

    /// A link is followed whether or not its target is there yet: on a Mac that never ran a
    /// dev build `~/.shroom-dev` doesn't exist, and a link to it out of a temp folder must
    /// still name that home — not pass for the temp folder it sits in (found by CI's clean
    /// runner, DECISIONS B102). Nothing here depends on what this Mac's home holds.
    #[test]
    fn a_link_to_a_place_not_made_yet_is_still_a_link() {
        use std::os::unix::fs::symlink;
        use std::path::Path;
        // A home outside every temp root, and a shroom home in it that was never made.
        let outside = Path::new("/shroom-never-made/.shroom-dev");
        assert!(!outside.exists());
        let tmp = tempfile::tempdir().unwrap();
        let real = std::fs::canonicalize(tmp.path()).unwrap();
        let t = tmp.path();
        symlink(outside, t.join("out")).unwrap();
        symlink(t.join("later"), t.join("in")).unwrap();
        symlink("later", t.join("beside")).unwrap();
        symlink(t.join("in"), t.join("twice")).unwrap();
        symlink("../elsewhere", t.join("up")).unwrap();
        symlink(t.join("b"), t.join("a")).unwrap();
        symlink(t.join("a"), t.join("b")).unwrap();

        // Out of the temp folder: resolved to where it points, so never a throwaway place.
        assert_eq!(resolved(&t.join("out")), Some(outside.to_path_buf()));
        assert_eq!(resolved(&t.join("out/sub")), Some(outside.join("sub")));
        for bad in ["out", "out/sub"] {
            let bad = t.join(bad).display().to_string();
            assert!(throwaway_place(Path::new(&bad)).is_none(), "{bad}");
            assert!(DevFlags::from_args(&args(&format!("shroom --home {bad}"))).refusal().is_some(), "{bad}");
        }
        // Within it: the place it points to, by an absolute target, a relative one, or a
        // link to a link.
        for (link, to) in [("in", "later"), ("beside", "later"), ("twice", "later"), ("in/sub", "later/sub")] {
            assert_eq!(resolved(&t.join(link)), Some(real.join(to)), "{link}");
            assert_eq!(throwaway_place(&t.join(link)), Some(real.join(to)), "{link}");
        }
        // A target the disk can't be asked about (`..`), and links that never end: refused.
        for bad in ["up", "a", "b", "a/sub"] {
            assert_eq!(resolved(&t.join(bad)), None, "{bad}");
            assert!(throwaway_place(&t.join(bad)).is_none(), "{bad}");
        }
        // And what was already so: a place that is there, and one whose end isn't.
        std::fs::create_dir(t.join("here")).unwrap();
        symlink(t.join("here"), t.join("to-here")).unwrap();
        assert_eq!(resolved(&t.join("to-here/x/y")), Some(real.join("here/x/y")));
        assert_eq!(resolved(&t.join("nothing/yet")), Some(real.join("nothing/yet")));
    }

    #[test]
    fn restore_json_round_trips_with_its_version() {
        use shroom_core::world::{RelaunchWhy, WindowKind};
        let f = RestoreFile { v: 1, restore: Restore { why: RelaunchWhy::Permission, window: WindowKind::Onboarding, written: Wall(42) } };
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(json, r#"{"v":1,"why":"permission","window":"onboarding","written":42}"#);
        let back: RestoreFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.restore.written, Wall(42));
    }
}
