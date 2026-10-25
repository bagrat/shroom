//! Uninstall shroom… (DECISIONS B135): shroom removes itself from this Mac, never the space.
//!
//! In order: the login item off, then a helper started in its own session, and the app
//! quits. The helper — as readable as the installer, like the updater's swap (B31) — waits
//! for the app to be gone, resets every permission, lets LaunchServices forget the app and
//! removes it, shroom's folder and what macOS keeps for its bundle id; the app takes back
//! the `shroom` command's link and profile line itself (`cli_link.rs`). Copies saved to
//! Movies are never touched.

use std::path::{Path, PathBuf};

use super::Runtime;

/// The helper: wait for `pid` to be gone (never remove a running app), then — as the
/// installer's `--uninstall` does — register the app with LaunchServices, reset every
/// permission (tccutil finds the app only through LaunchServices), forget it and remove. Its arguments after
/// `pid`, `bundle id` and `app` are the paths to remove.
pub const UNINSTALL_SH: &str = r#"pid=$1; id=$2; app=$3; shift 3
i=0
while kill -0 "$pid" 2>/dev/null; do
  i=$((i + 1)); [ "$i" -gt 300 ] && exit 1
  sleep 0.1
done
ls=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
"$ls" -f "$app" 2>/dev/null
tccutil reset All "$id" >/dev/null 2>&1
"$ls" -u "$app" 2>/dev/null
defaults delete "$id" 2>/dev/null
rm -rf "$app" "$@"
"#;

/// What goes besides the app: shroom's own folder and macOS's keeping for its bundle id
/// (the installer's `--uninstall` removes the same, `installer/install.sh`).
pub fn leftovers(user_home: &Path, shroom_home: &Path, id: &str) -> Vec<PathBuf> {
    let lib = user_home.join("Library");
    vec![
        shroom_home.to_path_buf(),
        lib.join("WebKit").join(id),
        lib.join("Caches").join(id),
        lib.join("HTTPStorages").join(id),
        lib.join("Saved Application State").join(format!("{id}.savedState")),
        lib.join("Preferences").join(format!("{id}.plist")),
    ]
}

/// Only an app bundle named for shroom, and only a shroom folder: anything else is refused
/// before a single path is handed to `rm`.
pub fn safe(app: &Path, shroom_home: &Path, id: &str) -> bool {
    let home_ok = shroom_home.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".shroom"));
    safe_app(app, id) && home_ok
}

fn safe_app(app: &Path, id: &str) -> bool {
    let app_ok = app.extension().is_some_and(|e| e == "app") && app.file_name().is_some_and(|n| n.to_string_lossy().starts_with("shroom"));
    let id_ok = id.starts_with("am.shroom") && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
    app_ok && id_ok
}

pub fn uninstall(rt: &Runtime, app: &tauri::AppHandle) {
    use tauri::Manager;
    let id = app.config().identifier.clone();
    let Some(bundle) = super::os::bundle() else {
        rt.log("shell: not running from an .app — no uninstall");
        return;
    };
    let Some(user_home) = super::home::user_home() else {
        rt.log("shell: no home folder — no uninstall");
        return;
    };
    // A dev build removes only a throwaway copy of itself, run from a throwaway home — the
    // updater's rule (B31): its own home goes, never the dev build's Library or the real one.
    let paths = if rt.dev {
        let throwaway = rt.throwaway && super::throwaway_app(&bundle).is_some() && safe_app(&bundle, &id);
        if !throwaway {
            rt.log("shell would uninstall (a dev build removes only a throwaway copy, from a throwaway home)");
            return;
        }
        vec![rt.home.root.clone()]
    } else {
        if !safe(&bundle, &rt.home.root, &id) {
            rt.log(&format!("shell: uninstall refused for {} / {}", bundle.display(), rt.home.root.display()));
            return;
        }
        leftovers(&user_home, &rt.home.root, &id)
    };
    // The service-management call is synchronous: off the core thread.
    let rt = rt.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        super::os::unregister_login_item(&rt);
        match spawn_helper(std::process::id(), &id, &bundle, &paths) {
            Ok(disclaimed) => {
                if !disclaimed {
                    rt.log("uninstall: the helper couldn't disclaim shroom — its permissions may stay");
                }
                // The `shroom` command's link and its profile line, as `install.sh --uninstall`
                // takes them back (B236) — never a dev build's, whose home is a throwaway.
                if !rt.dev {
                    super::cli_link::unlink(&user_home, &bundle);
                }
                rt.log("uninstall: the helper waits; shroom quits");
                rt.exit(&app);
            }
            Err(e) => rt.log(&format!("uninstall: the helper didn't start: {e}")),
        }
    });
}

/// Starts the helper detached — its own session, and its own responsible process (the
/// POC's disclaim, history/POC-LEARNINGS.md §4: a private symbol looked up at run time),
/// so the permission reset is the helper's own, as it is from a shell. Without the symbol
/// the helper still runs, and the log says the grants may stay. Proven on the hosted Mac
/// (DECISIONS B135): the drive's grants all reset once the app's id resolved the way an
/// installed app's does — LaunchServices never answers with an app under /tmp or
/// /var/folders, which is what had failed every earlier run.
/// `<spawn.h>`'s POSIX_SPAWN_SETSID (Apple's; the libc crate doesn't carry it).
const POSIX_SPAWN_SETSID: libc::c_short = 0x0400;

fn spawn_helper(pid: u32, id: &str, app: &Path, paths: &[PathBuf]) -> std::io::Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = |s: &[u8]| CString::new(s).map_err(|_| std::io::Error::other("a NUL in an argument"));
    let mut args = vec![c(b"/bin/sh")?, c(b"-c")?, c(UNINSTALL_SH.as_bytes())?, c(b"shroom-uninstall")?, c(pid.to_string().as_bytes())?, c(id.as_bytes())?, c(app.as_os_str().as_bytes())?];
    for p in paths {
        args.push(c(p.as_os_str().as_bytes())?);
    }
    let env: Vec<CString> = std::env::vars_os().filter_map(|(k, v)| CString::new([k.as_bytes(), b"=", v.as_bytes()].concat()).ok()).collect();
    let mut argv: Vec<*mut libc::c_char> = args.iter().map(|a| a.as_ptr() as *mut _).collect();
    argv.push(std::ptr::null_mut());
    let mut envp: Vec<*mut libc::c_char> = env.iter().map(|e| e.as_ptr() as *mut _).collect();
    envp.push(std::ptr::null_mut());
    type Disclaim = unsafe extern "C" fn(*mut libc::posix_spawnattr_t, libc::c_int) -> libc::c_int;
    // SAFETY: the attribute is initialised before use and destroyed after; argv and envp
    // are NUL-terminated arrays of C strings that outlive the call; the symbol, when found,
    // has the signature libsystem gives it.
    unsafe {
        let mut attr: libc::posix_spawnattr_t = std::mem::zeroed();
        if libc::posix_spawnattr_init(&mut attr) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        libc::posix_spawnattr_setflags(&mut attr, POSIX_SPAWN_SETSID);
        let sym = libc::dlsym(libc::RTLD_DEFAULT, c"responsibility_spawnattrs_setdisclaim".as_ptr());
        let disclaimed = !sym.is_null() && std::mem::transmute::<*mut libc::c_void, Disclaim>(sym)(&mut attr, 1) == 0;
        let mut child: libc::pid_t = 0;
        let r = libc::posix_spawn(&mut child, args[0].as_ptr(), std::ptr::null(), &attr, argv.as_ptr(), envp.as_ptr());
        libc::posix_spawnattr_destroy(&mut attr);
        if r != 0 {
            return Err(std::io::Error::from_raw_os_error(r));
        }
        Ok(disclaimed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_removes_shrooms_own_places_and_nothing_else() {
        let home = Path::new("/Users/b");
        let got = leftovers(home, &home.join(".shroom"), "am.shroom");
        assert_eq!(got, [
            "/Users/b/.shroom",
            "/Users/b/Library/WebKit/am.shroom",
            "/Users/b/Library/Caches/am.shroom",
            "/Users/b/Library/HTTPStorages/am.shroom",
            "/Users/b/Library/Saved Application State/am.shroom.savedState",
            "/Users/b/Library/Preferences/am.shroom.plist",
        ].map(PathBuf::from));
        assert!(!got.iter().any(|p| p.starts_with("/Users/b/Movies")), "copies saved to Movies stay");
    }

    #[test]
    fn only_a_shroom_app_and_a_shroom_folder_are_removed() {
        let home = Path::new("/Users/b/.shroom");
        assert!(safe(Path::new("/Users/b/Applications/shroom.app"), home, "am.shroom"));
        assert!(safe(Path::new("/tmp/x/shroom-dev.app"), Path::new("/tmp/h/.shroom-dev"), "am.shroom.dev"));
        assert!(!safe(Path::new("/Users/b/Applications"), home, "am.shroom"), "a folder, not the app");
        assert!(!safe(Path::new("/Applications/Safari.app"), home, "am.shroom"));
        assert!(!safe(Path::new("/Users/b/Applications/shroom.app"), Path::new("/Users/b"), "am.shroom"), "never a home folder itself");
        assert!(!safe(Path::new("/Users/b/Applications/shroom.app"), home, "com.apple.Safari"));
        assert!(!safe(Path::new("/Users/b/Applications/shroom.app"), home, "am.shroom; rm -rf ~"));
    }

    /// The helper waits for the app's process, then removes the app and the paths — and
    /// leaves what it wasn't given.
    #[test]
    fn the_helper_waits_for_the_app_then_removes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("shroom.app");
        let home = tmp.path().join(".shroom");
        let movies = tmp.path().join("Movies/shroom recording.mp4");
        for d in [&app, &home] {
            std::fs::create_dir_all(d.join("x")).unwrap();
        }
        std::fs::create_dir_all(movies.parent().unwrap()).unwrap();
        std::fs::write(&movies, b"").unwrap();
        let mut running = std::process::Command::new("/bin/sleep").arg("1").spawn().unwrap();
        let mut helper = std::process::Command::new("/bin/sh")
            .arg("-c").arg(UNINSTALL_SH).arg("shroom-uninstall").arg(running.id().to_string()).arg("am.shroom.test-nothing").arg(&app).arg(&home)
            .spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));
        assert!(app.exists() && home.exists(), "nothing goes while the app runs");
        running.wait().unwrap();
        helper.wait().unwrap();
        assert!(!app.exists() && !home.exists(), "both gone once it quit");
        assert!(movies.exists(), "Movies untouched");
    }
}
