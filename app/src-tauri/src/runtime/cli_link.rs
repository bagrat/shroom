//! The `shroom` command on PATH for a Mac that got shroom before the command shipped
//! (DECISIONS B239, DEVIATIONS #57). Only the install line's `link_cli` and the cask made
//! `~/.local/bin/shroom` (B217), and an update replaces the bundle without running either,
//! so a Mac installed before the command, and updated in-app since, had none. Once per Mac,
//! at launch, the app makes what `link_cli` makes, under its rules: the link only in
//! `~/.local/bin` and never over a file of the person's own; the marked profile line only
//! when that folder isn't on their PATH already; nothing when Homebrew linked it. Then
//! `cli.json` says it's done, so a link the person removes stays removed. Uninstall
//! shroom… takes back exactly what either made, as `install.sh --uninstall` does.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::Runtime;

/// The install line's `CLI_MARK` and `CLI_LINE`, word for word (a test holds them equal).
pub const MARK: &str = "# shroom's command line (shroom.am) — run shroom --help";
/// Appended, never first: nothing already in `~/.local/bin` stands in for a system command.
pub const LINE: &str = r#"export PATH="$PATH:$HOME/.local/bin""#;
/// Where the cask's `binary` puts its link, on Apple silicon and on Intel's prefix.
pub const BREW_BINS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];
/// The files a shell reads at start that could put `~/.local/bin` on PATH.
const PROFILES: [&str; 7] = [".zshenv", ".zprofile", ".zshrc", ".zlogin", ".bash_profile", ".bashrc", ".profile"];

/// What one pass did, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linked {
    /// The app carries no command (never for a bundle built with it).
    NoCommand,
    /// Homebrew's bin already links it.
    ByHomebrew,
    /// `~/.local/bin/shroom` is something of the person's own: left as it is.
    TheirsThere,
    /// The folder couldn't be made or the link written.
    Failed,
    /// Linked (or already linked), and `~/.local/bin` is on PATH by a profile already.
    OnPath,
    /// Linked, and the marked line was added to this profile (or was there already).
    Profile(&'static str),
    /// Linked, but the login shell is neither zsh nor bash: no profile is touched.
    OtherShell,
}

/// The command inside the app, as the install line links it.
pub fn target(app: &Path) -> PathBuf {
    app.join("Contents/MacOS/shroom-cli")
}

/// `link_cli`, for the person's home folder `home`, the app at `app`, their login shell
/// and the bins Homebrew links into.
pub fn link(home: &Path, app: &Path, shell: Option<&str>, brew_bins: &[&Path]) -> Linked {
    use std::os::unix::fs::PermissionsExt;
    let target = target(app);
    if !fs::metadata(&target).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0) {
        return Linked::NoCommand;
    }
    if brew_bins.iter().any(|b| fs::read_link(b.join("shroom")).is_ok_and(|t| t == target)) {
        return Linked::ByHomebrew;
    }
    let dir = home.join(".local/bin");
    let link = dir.join("shroom");
    if fs::symlink_metadata(&link).is_ok() {
        if fs::read_link(&link).ok().as_deref() != Some(target.as_path()) {
            return Linked::TheirsThere;
        }
    } else if fs::create_dir_all(&dir).is_err() || std::os::unix::fs::symlink(&target, &link).is_err() {
        return Linked::Failed;
    }
    if on_path(home) {
        return Linked::OnPath;
    }
    let profile = match shell.map(|s| s.rsplit('/').next().unwrap_or(s)) {
        Some("zsh") => ".zprofile",
        Some("bash") => ".bash_profile",
        _ => return Linked::OtherShell,
    };
    let path = home.join(profile);
    if !fs::read_to_string(&path).is_ok_and(|t| t.lines().any(|l| l == MARK)) {
        let added = fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| write!(f, "\n{MARK}\n{LINE}\n"));
        if added.is_err() {
            return Linked::Failed;
        }
    }
    Linked::Profile(profile)
}

/// `~/.local/bin` is on PATH already. The app's own PATH is launchd's, never the Terminal's,
/// so it reads the shell's start files instead: any of them naming `.local/bin` counts, so
/// the line is never added beside one of the person's own.
pub fn on_path(home: &Path) -> bool {
    PROFILES.iter().any(|p| fs::read_to_string(home.join(p)).is_ok_and(|t| t.lines().any(|l| l.contains(".local/bin") && l != LINE)))
}

/// `unlink_cli`: the link only if it points into this app, the mark, and the line after it
/// only when that is still shroom's own; the rest of each profile is left as it was.
pub fn unlink(home: &Path, app: &Path) {
    let link = home.join(".local/bin/shroom");
    if fs::read_link(&link).is_ok_and(|t| t == target(app)) {
        let _ = fs::remove_file(&link);
    }
    for profile in [".zprofile", ".bash_profile"] {
        let path = home.join(profile);
        let Ok(text) = fs::read_to_string(&path) else { continue };
        if !text.lines().any(|l| l == MARK) {
            continue;
        }
        let mut kept = String::with_capacity(text.len());
        let mut held = false;
        for line in text.split_inclusive('\n') {
            let bare = line.strip_suffix('\n').unwrap_or(line);
            if bare == MARK {
                held = true;
                continue;
            }
            if std::mem::take(&mut held) && bare == LINE {
                continue;
            }
            kept.push_str(line);
        }
        // In place, as the installer's `cat >` does: the file keeps its mode and owner.
        let _ = fs::write(&path, kept);
    }
}

/// The person's login shell: `$SHELL`, else the system's record for this user.
fn login_shell() -> Option<String> {
    if let Some(s) = std::env::var("SHELL").ok().filter(|s| !s.is_empty()) {
        return Some(s);
    }
    use std::ffi::CStr;
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0 as libc::c_char; 16 * 1024];
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    let rc = unsafe { libc::getpwuid_r(libc::getuid(), &mut pwd, buf.as_mut_ptr(), buf.len(), &mut out) };
    if rc != 0 || out.is_null() || pwd.pw_shell.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(pwd.pw_shell) }.to_string_lossy().into_owned())
}

/// `cli.json`: `{v, at, linked}` — this Mac's one pass is behind it.
#[derive(serde::Serialize, serde::Deserialize)]
struct Done {
    v: u32,
    at: i64,
    linked: String,
}

/// At launch, once per Mac: the app installed where the install line, the cask and the
/// updater keep it. A dev build never links (its command would stand in for the real one).
pub fn boot(rt: &Runtime) {
    if rt.dev || rt.home.path("cli.json").exists() {
        return;
    }
    let (Some(app), Some(home)) = (super::os::bundle(), super::home::user_home()) else { return };
    if app != home.join("Applications/shroom.app") {
        rt.log(&format!("cli: not linked — shroom runs from {}", app.display()));
        return;
    }
    let rt = rt.clone();
    std::thread::spawn(move || {
        let brew: Vec<&Path> = BREW_BINS.iter().map(Path::new).collect();
        let linked = link(&home, &app, login_shell().as_deref(), &brew);
        rt.log(&format!("cli: {linked:?}"));
        if linked != Linked::Failed {
            let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
            let _ = rt.home.write_json("cli.json", &Done { v: 1, at, linked: format!("{linked:?}") });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A home folder and an app carrying the command, in a temporary folder.
    fn mac() -> (tempfile::TempDir, PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("Users/b");
        let app = home.join("Applications/shroom.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(target(&app), b"#!/bin/sh\n").unwrap();
        fs::set_permissions(target(&app), fs::Permissions::from_mode(0o755)).unwrap();
        (tmp, home, app)
    }

    fn link_at(home: &Path) -> Option<PathBuf> {
        fs::read_link(home.join(".local/bin/shroom")).ok()
    }

    #[test]
    fn its_mark_and_line_are_the_install_lines() {
        let sh = include_str!("../../../../installer/install.sh");
        assert!(sh.contains(&format!("CLI_MARK=\"{MARK}\"")), "install.sh's CLI_MARK");
        assert!(sh.contains(&format!("CLI_LINE='{LINE}'")), "install.sh's CLI_LINE");
    }

    #[test]
    fn it_links_the_command_and_adds_the_marked_line_to_zshs_login_profile() {
        let (_t, home, app) = mac();
        fs::write(home.join(".zprofile"), "eval \"$(/opt/homebrew/bin/brew shellenv)\"\n").unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::Profile(".zprofile"));
        assert_eq!(link_at(&home), Some(target(&app)));
        assert_eq!(fs::read_to_string(home.join(".zprofile")).unwrap(), format!("eval \"$(/opt/homebrew/bin/brew shellenv)\"\n\n{MARK}\n{LINE}\n"));
        assert!(!home.join(".bash_profile").exists());
        // Again: nothing twice.
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::Profile(".zprofile"));
        assert_eq!(fs::read_to_string(home.join(".zprofile")).unwrap().matches(MARK).count(), 1);
    }

    #[test]
    fn bash_gets_its_own_profile_and_another_shell_none() {
        let (_t, home, app) = mac();
        assert_eq!(link(&home, &app, Some("/bin/bash"), &[]), Linked::Profile(".bash_profile"));
        assert!(fs::read_to_string(home.join(".bash_profile")).unwrap().contains(MARK));
        let (_t, home, app) = mac();
        assert_eq!(link(&home, &app, Some("/opt/homebrew/bin/fish"), &[]), Linked::OtherShell);
        assert_eq!(link_at(&home), Some(target(&app)), "the link still made");
        assert!(PROFILES.iter().all(|p| !home.join(p).exists()), "no profile written");
    }

    #[test]
    fn a_profile_that_already_puts_the_folder_on_path_is_left_alone() {
        let (_t, home, app) = mac();
        let theirs = "export PATH=\"$HOME/.local/bin:$PATH\"\n";
        fs::write(home.join(".zshrc"), theirs).unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::OnPath);
        assert_eq!(link_at(&home), Some(target(&app)));
        assert_eq!(fs::read_to_string(home.join(".zshrc")).unwrap(), theirs);
        assert!(!home.join(".zprofile").exists());
    }

    #[test]
    fn never_over_a_file_of_theirs() {
        let (_t, home, app) = mac();
        fs::create_dir_all(home.join(".local/bin")).unwrap();
        fs::write(home.join(".local/bin/shroom"), b"mine").unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::TheirsThere);
        assert_eq!(fs::read(home.join(".local/bin/shroom")).unwrap(), b"mine");
        assert!(!home.join(".zprofile").exists(), "nothing else either");
        // Nor over a link of theirs to somewhere else.
        let (_t, home, app) = mac();
        fs::create_dir_all(home.join(".local/bin")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/true", home.join(".local/bin/shroom")).unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::TheirsThere);
        assert_eq!(link_at(&home), Some(PathBuf::from("/usr/bin/true")));
    }

    #[test]
    fn homebrews_link_is_enough_and_an_app_without_the_command_links_nothing() {
        let (t, home, app) = mac();
        let brew = t.path().join("opt/homebrew/bin");
        fs::create_dir_all(&brew).unwrap();
        std::os::unix::fs::symlink(target(&app), brew.join("shroom")).unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[&brew]), Linked::ByHomebrew);
        assert_eq!(link_at(&home), None);
        fs::remove_file(target(&app)).unwrap();
        assert_eq!(link(&home, &app, Some("/bin/zsh"), &[]), Linked::NoCommand);
        assert_eq!(link_at(&home), None);
    }

    #[test]
    fn uninstall_takes_back_exactly_what_was_made() {
        let (_t, home, app) = mac();
        let before = "# mine\nexport FOO=1\n";
        fs::write(home.join(".zprofile"), before).unwrap();
        link(&home, &app, Some("/bin/zsh"), &[]);
        fs::write(home.join(".zprofile"), format!("{}export BAR=2\n", fs::read_to_string(home.join(".zprofile")).unwrap())).unwrap();
        unlink(&home, &app);
        assert_eq!(link_at(&home), None);
        assert_eq!(fs::read_to_string(home.join(".zprofile")).unwrap(), format!("{before}\nexport BAR=2\n"), "the mark and its line go, the rest stays");
    }

    #[test]
    fn uninstall_leaves_a_link_and_a_line_of_theirs() {
        let (_t, home, app) = mac();
        fs::create_dir_all(home.join(".local/bin")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/true", home.join(".local/bin/shroom")).unwrap();
        let edited = format!("{MARK}\nexport PATH=\"$HOME/.local/bin:$PATH\"\n");
        fs::write(home.join(".bash_profile"), &edited).unwrap();
        unlink(&home, &app);
        assert_eq!(link_at(&home), Some(PathBuf::from("/usr/bin/true")));
        assert_eq!(fs::read_to_string(home.join(".bash_profile")).unwrap(), "export PATH=\"$HOME/.local/bin:$PATH\"\n", "the mark goes; their edited line stays");
    }
}
