//! Updates (U7, N37; DECISIONS B31): checked quietly against the project's public
//! releases, downloaded in the background, verified against the published fingerprint
//! **exactly as the installer does** (`installer/install.sh`, B29 — the same
//! `manifest.txt`, the same SHA-256, the same `ditto` extract), and applied by a small
//! detached helper that waits for the app to quit, swaps, and reopens — putting the old
//! copy back if the swap fails. No shroom server to depend on: the download goes by
//! shroom.am, which counts it, and straight to GitHub when shroom.am doesn't answer
//! (DECISIONS B228); a failure never breaks anything.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Where releases live — the installer's default too.
pub const RELEASES: &str = "https://github.com/bagrat/shroom/releases";
/// Where a public release's file is downloaded through, counted (DECISIONS B228): shroom.am
/// counts the download — an update, of that version — and redirects to the same file in
/// [`RELEASES`]. Nothing else is sent: no ID, and not the version running.
pub const DOWNLOADS: &str = "https://shroom.am/download";
/// A quiet check gives up fast (the POC's fail-soft budget).
pub const CHECK_TIMEOUT: Duration = Duration::from_millis(2500);

/// A release's version: three whole numbers, and for an early build its stage (DECISIONS
/// B220): `0.1.25-early.1` < `0.1.25-early.2` < `0.1.25` < `0.1.26-early.1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32, pub Stage);

/// Where a version stands: an early build of it (`-early.<n>`, n ≥ 1), or the release
/// itself, which every early build of it sorts below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Early(u32),
    Final,
}

impl Version {
    /// A release (no early stage).
    pub const fn new(major: u32, minor: u32, patch: u32) -> Version {
        Version(major, minor, patch, Stage::Final)
    }

    /// `1.2.3`, `v1.2` or `1.2.3-early.4`; any other suffix is no version.
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.trim().trim_start_matches('v');
        let (core, stage) = match s.split_once('-') {
            None => (s, Stage::Final),
            Some((core, pre)) => {
                let n = pre.strip_prefix("early.")?;
                if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                (core, Stage::Early(n.parse().ok().filter(|n| *n >= 1)?))
            }
        };
        let mut it = core.split('.').map(|p| p.parse::<u32>().ok());
        let v = Version(it.next()??, it.next().unwrap_or(Some(0))?, it.next().unwrap_or(Some(0))?, stage);
        it.next().is_none().then_some(v)
    }

    pub fn is_early(&self) -> bool {
        matches!(self.3, Stage::Early(_))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)?;
        match self.3 {
            Stage::Early(n) => write!(f, "-early.{n}"),
            Stage::Final => Ok(()),
        }
    }
}

/// One release's `manifest.txt`: `key=value` lines — `version`, `macos` (the floor), and
/// per architecture (`uname -m`) `<file> <sha256>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub version: Version,
    pub macos: u32,
    pub assets: Vec<(String, String, String)>,
}

impl Manifest {
    pub fn parse(text: &str) -> Option<Manifest> {
        let mut version = None;
        let mut macos = 14;
        let mut assets = Vec::new();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "version" => version = Version::parse(v),
                "macos" => macos = v.trim().parse().ok()?,
                arch => {
                    let mut parts = v.split_whitespace();
                    if let (Some(file), Some(sum), None) = (parts.next(), parts.next(), parts.next()) {
                        if sum.len() == 64 && sum.bytes().all(|b| b.is_ascii_hexdigit()) && !file.contains('/') {
                            assets.push((arch.to_string(), file.to_string(), sum.to_lowercase()));
                        }
                    }
                }
            }
        }
        Some(Manifest { version: version?, macos, assets })
    }
}

/// What this Mac should be offered.
#[derive(Debug, Clone, PartialEq)]
pub enum Offer {
    /// Nothing newer (or nothing for this kind of Mac).
    Nothing,
    /// A newer version this Mac can run: the file and its fingerprint.
    Ready(Release),
    /// A newer version that needs a newer macOS: “shroom 1.4.0 needs macOS 15 — this Mac
    /// stays on 1.3.2.”
    NeedsMacos { version: Version, macos: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: Version,
    pub file: String,
    pub sha256: String,
}

pub fn offer(m: &Manifest, running: Version, arch: &str, macos_major: u32) -> Offer {
    if m.version <= running {
        return Offer::Nothing;
    }
    if macos_major < m.macos {
        return Offer::NeedsMacos { version: m.version, macos: m.macos };
    }
    match m.assets.iter().find(|(a, _, _)| a == arch) {
        Some((_, file, sum)) => Offer::Ready(Release { version: m.version, file: file.clone(), sha256: sum.clone() }),
        None => Offer::Nothing,
    }
}

#[derive(Debug)]
pub enum UpdateError {
    Transport(String),
    /// The manifest wasn't one.
    Unreadable,
    /// The download's fingerprint didn't match — discarded, never installed.
    Mismatch,
    /// The archive didn't hold `shroom.app`.
    Archive(String),
    /// The staged copy isn't signed by the app it would replace (DECISIONS B125).
    Signature(String),
    Io(std::io::Error),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Transport(e) => write!(f, "transport: {e}"),
            UpdateError::Unreadable => write!(f, "the manifest wasn't readable"),
            UpdateError::Mismatch => write!(f, "the fingerprint didn't match"),
            UpdateError::Archive(e) => write!(f, "archive: {e}"),
            UpdateError::Signature(e) => write!(f, "signature: {e}"),
            UpdateError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<std::io::Error> for UpdateError {
    fn from(e: std::io::Error) -> Self {
        UpdateError::Io(e)
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // A host that doesn't answer is given up on soon, so the next source gets its turn.
        .timeout_connect(Some(Duration::from_secs(15)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .build()
        .into()
}

/// The designated requirement in `codesign -d -r-`'s answer: the line it begins
/// `designated => ` (macOS prints it to stdout or stderr, by version).
pub fn designated_in(answer: &str) -> Option<String> {
    answer.lines().find_map(|l| l.trim().strip_prefix("designated => ")).map(str::trim).filter(|r| !r.is_empty()).map(String::from)
}

/// Whether `staged` may replace `replaced` (DECISIONS B125): its signature holds, whole,
/// and it meets the replaced app's own designated requirement — for a release, the
/// identifier and the release certificate's leaf, the same rule macOS keeps the app's
/// permissions by. A fingerprint only says the download is the one the manifest named;
/// this says who made it. Whatever can't be read is refused.
pub fn same_signer(staged: &Path, replaced: &Path) -> Result<(), UpdateError> {
    let out = Command::new("codesign").arg("-d").arg("-r-").arg(replaced).stdin(Stdio::null()).output()?;
    let answer = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let req = designated_in(&answer).filter(|_| out.status.success()).ok_or_else(|| UpdateError::Signature("the installed app's requirement couldn't be read".into()))?;
    let status = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        // `-R` and its text apart: glued (`-R==…`), codesign reads one `=` too many and
        // answers a syntax error, so no update would ever be applied.
        .arg("-R")
        .arg(format!("={req}"))
        .arg(staged)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() { Ok(()) } else { Err(UpdateError::Signature(format!("the update doesn't meet {req}"))) }
}

/// Which releases a Mac takes (DECISIONS B220): the official ones, or the early channel's
/// signed pre-releases as well. Set by a file in the app's folder, `channel`, holding the
/// one word `early` — written by `install.sh --early`; anything else, or no file, is the
/// official channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Official,
    Early,
}

impl Channel {
    /// The channel file's words (None: no file).
    pub fn from_file(text: Option<&str>) -> Channel {
        match text.map(str::trim) {
            Some("early") => Channel::Early,
            _ => Channel::Official,
        }
    }

    /// The word the log and Copy diagnostics carry.
    pub fn word(self) -> &'static str {
        match self {
            Channel::Official => "official",
            Channel::Early => "early",
        }
    }
}

/// The newest release this channel takes, as its manifest: for the official channel the
/// latest release's — the installer's first request, with a short budget. The app asks for
/// it as `update.txt` (the same bytes as `manifest.txt`, DECISIONS B136), so a release's
/// download counts tell the app's checks from the install line's runs; a release without
/// one (before 0.1.11, or a test's folder) answers 404, and then it's `manifest.txt`.
///
/// The early channel reads the `early` pointer too — a pre-release GitHub keeps beside the
/// releases, whose files `release.yml` replaces with each publish's manifest, pre-release or
/// not — and takes whichever of the two is newer, so an early Mac takes an official release
/// that is out, and a pointer that couldn't be read leaves it on the official channel's.
pub fn check(base: &str) -> Result<Manifest, UpdateError> {
    check_channel(base, Channel::Official)
}

pub fn check_channel(base: &str, channel: Channel) -> Result<Manifest, UpdateError> {
    let base = base.trim_end_matches('/');
    let agent = agent(CHECK_TIMEOUT);
    let latest = manifest_in(&agent, &format!("{base}/latest/download"));
    if channel == Channel::Official {
        return latest;
    }
    match (latest, manifest_in(&agent, &format!("{base}/download/early"))) {
        (Ok(l), Ok(e)) => Ok(if e.version > l.version { e } else { l }),
        (Ok(m), Err(_)) | (Err(_), Ok(m)) => Ok(m),
        (Err(e), Err(_)) => Err(e),
    }
}

/// One release folder's manifest: `update.txt`, or on a 404 `manifest.txt`.
fn manifest_in(agent: &ureq::Agent, dir: &str) -> Result<Manifest, UpdateError> {
    let get = |name: &str| agent.get(&format!("{dir}/{name}")).call();
    let mut res = match get("update.txt") {
        Err(ureq::Error::StatusCode(404)) => get("manifest.txt"),
        asked => asked,
    }
    .map_err(|e| UpdateError::Transport(e.to_string()))?;
    let text = res.body_mut().with_config().limit(64 * 1024).read_to_string().map_err(|e| UpdateError::Transport(e.to_string()))?;
    Manifest::parse(&text).ok_or(UpdateError::Unreadable)
}

/// Where `release`'s file is fetched from, in order. From the public releases: through
/// shroom.am's counter (DECISIONS B228), then GitHub itself, so an update never waits on
/// shroom.am. A stand-in's releases (a dev build's, a test's): only where they are.
pub fn sources(base: &str, release: &Release) -> Vec<String> {
    let base = base.trim_end_matches('/');
    let direct = format!("{base}/download/v{}/{}", release.version, release.file);
    if base == RELEASES {
        vec![format!("{DOWNLOADS}/v{}/{}?via=update", release.version, release.file), direct]
    } else {
        vec![direct]
    }
}

/// Downloads the release into `staging/<version>/`, checks its fingerprint, and opens it
/// with `ditto` (signatures intact). Returns the staged `shroom.app`. A mismatch leaves
/// nothing behind.
pub fn download(base: &str, release: &Release, staging: &Path) -> Result<PathBuf, UpdateError> {
    download_from(&sources(base, release), release, staging)
}

/// The file's bytes from the first source that gives ones matching its fingerprint; else
/// the last source's error.
fn fetch(sources: &[String], release: &Release) -> Result<Vec<u8>, UpdateError> {
    let agent = agent(Duration::from_secs(30 * 60));
    let mut last = UpdateError::Transport("no source".into());
    for url in sources {
        let got = (|| {
            let res = agent.get(url).call().map_err(|e| UpdateError::Transport(e.to_string()))?;
            let mut bytes = Vec::new();
            res.into_body().into_reader().take(1 << 30).read_to_end(&mut bytes).map_err(|e| UpdateError::Transport(e.to_string()))?;
            if shroom_storage::sigv4::sha256_hex(&bytes) != release.sha256 {
                return Err(UpdateError::Mismatch);
            }
            Ok(bytes)
        })();
        match got {
            Ok(bytes) => return Ok(bytes),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// [`download`], from these sources in order ([`sources`]).
pub fn download_from(sources: &[String], release: &Release, staging: &Path) -> Result<PathBuf, UpdateError> {
    let dir = staging.join(release.version.to_string());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let result = (|| {
        let bytes = fetch(sources, release)?;
        let zip = dir.join(&release.file);
        std::fs::write(&zip, &bytes)?;
        let status = Command::new("ditto").arg("-x").arg("-k").arg(&zip).arg(&dir).stdin(Stdio::null()).status()?;
        let _ = std::fs::remove_file(&zip);
        let app = dir.join("shroom.app");
        if !status.success() || !app.is_dir() {
            return Err(UpdateError::Archive(format!("ditto exited {status}; shroom.app present: {}", app.is_dir())));
        }
        Ok(app)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    result
}

/// The swap helper, as readable as the installer: wait for the app (`pid`) to be gone —
/// never swap under a running process — then put the staged copy in place, the old one
/// back if that fails, and reopen. `ok`/`failed` are the reopen's arguments for each
/// outcome, so the next launch can say “Updated to 1.3.0 ✓” or “The update didn't
/// finish…”.
pub const SWAP_SH: &str = r#"pid=$1; new=$2; app=$3; ok=$4; failed=$5
i=0
while kill -0 "$pid" 2>/dev/null; do
  i=$((i + 1)); [ "$i" -gt 300 ] && exit 1
  sleep 0.1
done
old="$app.old-$$"
if ! mv "$app" "$old"; then open "$app" --args "$failed"; exit 1; fi
if mv "$new" "$app"; then
  rm -rf "$old"
  open "$app" --args "$ok"
else
  mv "$old" "$app"
  open "$app" --args "$failed"
fi
"#;

/// Starts the helper detached (its own session), so it outlives the app it replaces.
pub fn spawn_swap(pid: u32, staged: &Path, app: &Path, ok: &str, failed: &str) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg(SWAP_SH).arg("shroom-swap").arg(pid.to_string()).arg(staged).arg(app).arg(ok).arg(failed);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // SAFETY: setsid is async-signal-safe and touches no memory of this process.
    unsafe {
        cmd.pre_exec(|| if setsid() == -1 { Err(std::io::Error::last_os_error()) } else { Ok(()) });
    }
    cmd.spawn().map(|_| ())
}

unsafe extern "C" {
    fn setsid() -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUM: &str = "a12e7ee1cd64a69dcdb314157e45dafcba705bfb0b1440b7935cb265d374423e";

    /// The public releases' download goes by shroom.am's counter first, saying it's an
    /// update and nothing else, then GitHub; a stand-in's never leaves it (DECISIONS B228).
    #[test]
    fn a_public_update_is_counted_on_its_way_and_a_stand_ins_never() {
        let r = Release { version: Version::parse("0.1.26-early.2").unwrap(), file: "shroom-0.1.26-early.2-arm64.zip".into(), sha256: SUM.into() };
        assert_eq!(
            sources(RELEASES, &r),
            [
                "https://shroom.am/download/v0.1.26-early.2/shroom-0.1.26-early.2-arm64.zip?via=update",
                "https://github.com/bagrat/shroom/releases/download/v0.1.26-early.2/shroom-0.1.26-early.2-arm64.zip",
            ]
        );
        assert_eq!(sources(&format!("{RELEASES}/"), &r).len(), 2);
        assert_eq!(sources("http://127.0.0.1:4321", &r), ["http://127.0.0.1:4321/download/v0.1.26-early.2/shroom-0.1.26-early.2-arm64.zip"]);
    }

    #[test]
    fn reads_the_designated_requirement() {
        let answer = "Executable=/Applications/shroom.app/Contents/MacOS/shroom\ndesignated => identifier \"am.shroom\" and certificate leaf = H\"8344cbe040722ea0a5632320c1d0d4348eaa3af3\"\n";
        assert_eq!(designated_in(answer).as_deref(), Some("identifier \"am.shroom\" and certificate leaf = H\"8344cbe040722ea0a5632320c1d0d4348eaa3af3\""));
        assert_eq!(designated_in("Executable=/x\n"), None);
        assert_eq!(designated_in("designated => \n"), None);
    }

    // The real codesign, on two of the system's own signed tools: a copy meets its own
    // requirement, and a different tool doesn't — so the check runs, and refuses.
    #[cfg(target_os = "macos")]
    #[test]
    fn same_signer_asks_codesign_in_a_form_it_reads() {
        assert!(same_signer(Path::new("/bin/ls"), Path::new("/bin/ls")).is_ok());
        assert!(matches!(same_signer(Path::new("/bin/cat"), Path::new("/bin/ls")), Err(UpdateError::Signature(_))));
    }

    #[test]
    fn the_channel_is_early_only_when_its_file_says_so() {
        assert_eq!(Channel::from_file(Some("early\n")), Channel::Early);
        assert_eq!(Channel::from_file(Some("  early ")), Channel::Early);
        for other in [None, Some(""), Some("official"), Some("Early"), Some("early please")] {
            assert_eq!(Channel::from_file(other), Channel::Official, "{other:?}");
        }
    }

    /// An early build sorts above the release before it and below its own final, and the
    /// next version's early builds above that (DECISIONS B220) — the order the updater, the
    /// page compare (`shroom_cloud::wire`), `bump-version.sh` and the installer all keep.
    #[test]
    fn early_builds_sort_between_the_releases_around_them() {
        let v = |s: &str| Version::parse(s).unwrap_or_else(|| panic!("{s}"));
        let order = ["0.1.24", "0.1.25-early.1", "0.1.25-early.2", "0.1.25-early.10", "0.1.25", "0.1.26-early.1", "0.1.26"];
        for w in order.windows(2) {
            assert!(v(w[0]) < v(w[1]), "{} < {}", w[0], w[1]);
        }
        for s in order {
            assert_eq!(v(s).to_string(), s, "it prints as it reads");
        }
        assert!(v("0.1.25-early.1").is_early() && !v("0.1.25").is_early());
        for bad in ["0.1.25-early", "0.1.25-early.0", "0.1.25-early.x", "0.1.25-beta.1", "0.1.25-early.1.2", "0.1.25-"] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn versions_order_and_parse() {
        assert_eq!(Version::parse("1.3.0"), Some(Version::new(1, 3, 0)));
        assert_eq!(Version::parse("v1.10"), Some(Version::new(1, 10, 0)));
        assert!(Version::parse("1.2.3.4").is_none() && Version::parse("x").is_none());
        assert!(Version::new(1, 10, 0) > Version::new(1, 9, 9));
    }

    #[test]
    fn reads_the_installers_manifest() {
        let m = Manifest::parse(&format!("version=1.3.0\nmacos=14\narm64=shroom-1.3.0-arm64.zip {SUM}\nx86_64=../evil {SUM}\n")).unwrap();
        assert_eq!(m.version, Version::new(1, 3, 0));
        assert_eq!(m.assets.len(), 1, "a file name with a path in it is ignored");
        assert!(Manifest::parse("macos=14\n").is_none());
    }

    #[test]
    fn offers_only_what_this_mac_can_run() {
        let m = Manifest::parse(&format!("version=1.4.0\nmacos=15\narm64=shroom-1.4.0-arm64.zip {SUM}\n")).unwrap();
        assert_eq!(offer(&m, Version::new(1, 3, 2), "arm64", 14), Offer::NeedsMacos { version: Version::new(1, 4, 0), macos: 15 });
        assert!(matches!(offer(&m, Version::new(1, 3, 2), "arm64", 15), Offer::Ready(r) if r.file == "shroom-1.4.0-arm64.zip"));
        assert_eq!(offer(&m, Version::new(1, 4, 0), "arm64", 15), Offer::Nothing);
        assert_eq!(offer(&m, Version::new(1, 3, 2), "x86_64", 15), Offer::Nothing);
    }

    const PLIST: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n\t<key>CFBundleIdentifier</key>\n\t<string>am.shroom</string>\n\t<key>CFBundleShortVersionString</key>\n\t<string>0.1.26</string>\n\t<key>CFBundleVersion</key>\n\t<string>0.1.26</string>\n</dict>\n</plist>\n";

    /// The version a bundle says it is: its Info.plist's `CFBundleShortVersionString`, as
    /// the bundler writes it (XML), an early build's too.
    #[test]
    fn reads_a_bundles_own_version() {
        let with = |v: &str| PLIST.replacen("<string>0.1.26</string>", &format!("<string>{v}</string>"), 1);
        assert_eq!(short_version_in(PLIST), Some("0.1.26"));
        assert_eq!(short_version_in(&with("0.1.27-early.2")), Some("0.1.27-early.2"));
        assert_eq!(short_version_in("<plist><dict><key>CFBundleVersion</key><string>1</string></dict></plist>"), None, "no short version");
        assert_eq!(short_version_in("<key>CFBundleShortVersionString</key><integer>3</integer>"), None, "not a string");
        let dir = scratch("bundle");
        let app = dir.join("shroom.app");
        assert_eq!(bundle_version(&app), None, "no bundle");
        std::fs::create_dir_all(app.join("Contents")).unwrap();
        std::fs::write(app.join("Contents/Info.plist"), PLIST).unwrap();
        assert_eq!(bundle_version(&app), Some(Version::new(0, 1, 26)));
        std::fs::write(app.join("Contents/Info.plist"), with("latest")).unwrap();
        assert_eq!(bundle_version(&app), None, "a version that doesn't read");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A staged copy is swapped in only as the version offered, newer than the one running,
    /// and of this Mac's channel: an official Mac takes no early build.
    #[test]
    fn a_staged_copy_fits_only_as_the_offered_newer_version_of_this_channel() {
        let v = |s: &str| Version::parse(s).unwrap();
        let fits = |bundle: Option<&str>, offered: &str, running: &str, channel| staged_fits(bundle.map(v), v(offered), v(running), channel).is_ok();
        assert!(fits(Some("0.1.26"), "0.1.26", "0.1.25", Channel::Official));
        assert!(fits(Some("0.1.26-early.1"), "0.1.26-early.1", "0.1.25", Channel::Early));
        assert!(fits(Some("0.1.26"), "0.1.26", "0.1.26-early.3", Channel::Early), "an early Mac takes the release");
        assert!(!fits(None, "0.1.26", "0.1.25", Channel::Official), "a version that can't be read");
        assert!(!fits(Some("0.1.24"), "0.1.26", "0.1.25", Channel::Official), "not the version offered");
        assert!(!fits(Some("0.1.26"), "0.1.27", "0.1.25", Channel::Official), "not the version offered");
        assert!(!fits(Some("0.1.25"), "0.1.25", "0.1.25", Channel::Official), "the version running");
        assert!(!fits(Some("0.1.24"), "0.1.24", "0.1.25", Channel::Early), "older than the one running");
        assert!(!fits(Some("0.1.26-early.1"), "0.1.26-early.1", "0.1.25", Channel::Official), "an early build, on the official channel");
        assert!(matches!(staged_fits(None, v("0.1.26"), v("0.1.25"), Channel::Official), Err(UpdateError::Staged(_))));
    }

    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("shroom-update-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
