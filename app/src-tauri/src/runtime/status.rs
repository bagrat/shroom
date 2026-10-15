//! The status seam (ARCHITECTURE §11.2, DECISIONS B29 and B80): `status.json` in this app's
//! home — the public, versioned interface the installer's mid-take refusal (N9) and a
//! guiding agent (Region 6's overlay) read instead of sniffing processes or screenshots.
//!
//! Two writers feed one file, under one lock: the core thread, with the core's projection
//! after every batch of events (`shroom_core::derive::status::posture`), and a quiet thread
//! that re-reads the grants from macOS every two seconds — so a permission switched off
//! after the chapter reads as off here, never a frozen `all-set`. Whichever read the
//! grants last wins. The file is rewritten (atomically) only when what it says changes,
//! each write is logged as written (it holds no secret), and it goes on exit — a file left
//! by a crash names a pid that isn't running, which is how a reader knows to ignore it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use shroom_core::derive::status::{Grants, Posture};
use shroom_core::event::OsNews;

use super::Runtime;

/// The shape's version. 1 was `{v, pid, take_open, setup}`; 2 keeps those four as they
/// were (the installer reads `pid` and `take_open`) and adds the rest.
pub const VERSION: u32 = 2;

/// How often the grants are re-read for the seam.
pub const GRANTS_EVERY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub v: u32,
    pub pid: u32,
    /// The running app's version.
    pub app: String,
    pub take_open: bool,
    pub take: &'static str,
    pub setup: &'static str,
    pub permission: Permission,
    pub join: Join,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Permission {
    /// `granted` · `not-granted` · `unknown` (not read yet).
    pub screen: &'static str,
    /// `granted` · `denied` · `restricted` · `not-asked` · `unknown`.
    pub mic: &'static str,
    pub camera: &'static str,
    /// This Mac's permissions chapter: `done` · `ahead`.
    pub chapter: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Join {
    pub role: &'static str,
    pub invite: &'static str,
    pub invite_kind: &'static str,
    pub standing: &'static str,
}

/// What the seam knows, and what it last wrote.
#[derive(Default)]
pub struct Seam {
    pid: u32,
    app: String,
    posture: Option<Posture>,
    /// The grants as last read — by the core (an entry event's probe, a report) or by the
    /// seam's own re-read, whichever came last.
    grants: Option<Grants>,
    /// The core's grants as last seen, so an unchanged core fact never overrides a newer
    /// re-read.
    core_grants: Option<Grants>,
    written: Option<Status>,
    closed: bool,
}

impl Seam {
    pub fn new(pid: u32, app: String) -> Seam {
        Seam { pid, app, ..Seam::default() }
    }

    /// The core's projection, after a batch.
    pub fn core(&mut self, p: Posture) {
        if p.grants.is_some() && p.grants != self.core_grants {
            self.grants = p.grants;
            self.core_grants = p.grants;
        }
        self.posture = Some(p);
    }

    /// The grants as macOS says them now.
    pub fn read(&mut self, g: Grants) {
        self.grants = Some(g);
    }

    /// The file's contents, once the core has spoken.
    pub fn compose(&self) -> Option<Status> {
        let p = self.posture?;
        let g = self.grants;
        Some(Status {
            v: VERSION,
            pid: self.pid,
            app: self.app.clone(),
            take_open: p.take_open,
            take: p.take,
            setup: p.setup,
            permission: Permission {
                screen: g.map_or("unknown", |g| g.screen),
                mic: g.map_or("unknown", |g| g.mic),
                camera: g.map_or("unknown", |g| g.camera),
                chapter: p.chapter,
            },
            join: Join { role: p.role, invite: p.invite, invite_kind: p.invite_kind, standing: p.standing },
        })
    }

    /// What to write now, if anything changed (and the seam isn't closed). Marks it written.
    pub fn due(&mut self) -> Option<Status> {
        if self.closed {
            return None;
        }
        let now = self.compose()?;
        if self.written.as_ref() == Some(&now) {
            return None;
        }
        self.written = Some(now.clone());
        Some(now)
    }
}

pub type Shared = Arc<Mutex<Seam>>;

/// Applies `f` and writes the file if what it says changed — under the seam's lock, so two
/// writers never race on the temp file.
pub fn publish(rt: &Runtime, f: impl FnOnce(&mut Seam)) {
    let mut seam = rt.seam.lock().unwrap();
    f(&mut seam);
    if let Some(now) = seam.due() {
        let _ = rt.home.write_json("status.json", &now);
        rt.log(&format!("seam: {}", serde_json::to_string(&now).unwrap_or_default()));
    }
}

/// The process is ending: the file goes, and nothing writes it again.
pub fn close(rt: &Runtime) {
    let mut seam = rt.seam.lock().unwrap();
    seam.closed = true;
    rt.home.remove("status.json");
}

pub fn grants_of(news: &OsNews) -> Option<Grants> {
    match news {
        OsNews::Grants { screen, mic, camera } => Some(Grants::from(*screen, *mic, *camera)),
        _ => None,
    }
}

/// Re-reads the grants for the seam every two seconds, on a quiet thread (`read` is
/// `os::grants` — cheap preflight calls, never a prompt).
pub fn watch(rt: Runtime, read: fn() -> OsNews) {
    std::thread::Builder::new()
        .name("shroom-seam".into())
        .spawn(move || {
            loop {
                if let Some(g) = grants_of(&read()) {
                    publish(&rt, |s| s.read(g));
                }
                if rt.seam.lock().unwrap().closed {
                    return;
                }
                std::thread::sleep(GRANTS_EVERY);
            }
        })
        .expect("the seam's thread starts");
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::world::Access;

    fn posture(setup: &'static str, grants: Option<Grants>) -> Posture {
        Posture {
            take_open: false,
            take: "none",
            setup,
            chapter: if setup == "all-set" { "done" } else { "ahead" },
            grants,
            role: "token-holder",
            invite: "none",
            invite_kind: "none",
            standing: "fine",
        }
    }

    fn all_on() -> Grants {
        Grants::from(true, Access::Granted, Access::Granted)
    }

    fn json(s: &Status) -> serde_json::Value {
        serde_json::to_value(s).unwrap()
    }

    /// The v2 shape, field by field — and v1's four fields exactly where the installer reads
    /// them (`plutil -extract pid` / `take_open`).
    #[test]
    fn the_shape_is_v2_and_keeps_v1s_fields() {
        let mut s = Seam::new(4242, "0.1.1".into());
        assert!(s.due().is_none(), "nothing before the core speaks");
        s.core(posture("all-set", Some(all_on())));
        let v = json(&s.due().expect("written"));
        assert_eq!(v["v"], 2);
        assert_eq!(v["pid"], 4242);
        assert_eq!(v["take_open"], false);
        assert_eq!(v["setup"], "all-set");
        assert_eq!(v["app"], "0.1.1");
        assert_eq!(v["take"], "none");
        assert_eq!(v["permission"], serde_json::json!({ "screen": "granted", "mic": "granted", "camera": "granted", "chapter": "done" }));
        assert_eq!(v["join"], serde_json::json!({ "role": "token-holder", "invite": "none", "invite_kind": "none", "standing": "fine" }));
        // No nulls anywhere: stock macOS's plutil reads every value (older ones choke on null).
        assert!(!v.to_string().contains("null"), "{v}");
    }

    /// Gap: the seam said `all-set` frozen after priming. A screen grant switched off later
    /// is re-read and said, while `setup` stays where onboarding stands.
    #[test]
    fn a_grant_switched_off_after_all_set_reads_as_off() {
        let mut s = Seam::new(1, "0.1.1".into());
        s.core(posture("all-set", Some(all_on())));
        s.due();
        // The core's facts don't move (no entry event) — the seam's own re-read does.
        s.read(Grants::from(false, Access::Denied, Access::Granted));
        s.core(posture("all-set", Some(all_on())));
        let v = json(&s.due().expect("rewritten on the change"));
        assert_eq!(v["setup"], "all-set");
        assert_eq!(v["permission"]["screen"], "not-granted");
        assert_eq!(v["permission"]["mic"], "denied");
        assert_eq!(v["permission"]["camera"], "granted");
        // A newer core report (an entry event's probe) wins over the older re-read.
        s.core(posture("all-set", Some(Grants::from(true, Access::Granted, Access::NotAsked))));
        let v = json(&s.due().expect("rewritten"));
        assert_eq!(v["permission"]["screen"], "granted");
        assert_eq!(v["permission"]["camera"], "not-asked");
    }

    #[test]
    fn unread_grants_say_unknown() {
        let mut s = Seam::new(1, "0.1.1".into());
        s.core(posture("welcome", None));
        let v = json(&s.due().unwrap());
        assert_eq!(v["permission"]["screen"], "unknown");
        assert_eq!(v["permission"]["mic"], "unknown");
        assert_eq!(v["permission"]["chapter"], "ahead");
    }

    #[test]
    fn written_only_on_change_and_never_after_close() {
        let mut s = Seam::new(1, "0.1.1".into());
        s.core(posture("welcome", None));
        assert!(s.due().is_some());
        s.core(posture("welcome", None));
        assert!(s.due().is_none(), "the same words aren't written twice");
        s.read(all_on());
        assert!(s.due().is_some(), "a grant read is a change");
        s.read(all_on());
        assert!(s.due().is_none());
        s.closed = true;
        s.core(posture("account", None));
        assert!(s.due().is_none(), "closed: the exit's removal is the last word");
    }

    /// The file on disk, through the runner's own path: mode 600, the installer's fields,
    /// the log line naming what was written; the close removes it and a late re-read
    /// never brings it back.
    #[test]
    fn the_file_is_written_logged_and_removed() {
        let dir = std::env::temp_dir().join(format!("shroom-seam-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (rt, _rx) = Runtime::for_test(dir.clone(), None, None);
        publish(&rt, |s| s.core(posture("perm-screen", Some(Grants::from(false, Access::Granted, Access::NotAsked)))));
        let path = rt.home.path("status.json");
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["setup"], "perm-screen");
        assert_eq!(v["permission"]["screen"], "not-granted");
        assert_eq!(v["pid"], std::process::id());
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        // The installer's reader, as it runs: plutil on the file.
        let pid = std::process::Command::new("plutil").args(["-extract", "pid", "raw", "-o", "-"]).arg(&path).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&pid.stdout).trim(), std::process::id().to_string());
        let open = std::process::Command::new("plutil").args(["-extract", "take_open", "raw", "-o", "-"]).arg(&path).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&open.stdout).trim(), "false");
        // The re-read: screen on now.
        publish(&rt, |s| s.read(all_on()));
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["permission"]["screen"], "granted");
        let log = std::fs::read_to_string(dir.join("logs/run.log")).unwrap();
        assert_eq!(log.matches("seam: ").count(), 2, "{log}");
        close(&rt);
        assert!(!path.exists());
        publish(&rt, |s| s.read(Grants::from(false, Access::Granted, Access::Granted)));
        assert!(!path.exists(), "nothing writes after the close");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The seam's own thread: with the core silent at `all-set`, macOS switching screen
    /// recording off reaches the file within a re-read; after the close the thread stops.
    #[test]
    fn the_re_read_thread_says_a_grant_turned_off() {
        fn off() -> OsNews {
            OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted }
        }
        let dir = std::env::temp_dir().join(format!("shroom-seam-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (rt, _rx) = Runtime::for_test(dir.clone(), None, None);
        publish(&rt, |s| s.core(posture("all-set", Some(all_on()))));
        watch(rt.clone(), off);
        let path = rt.home.path("status.json");
        let read = || serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path).ok()?).ok();
        let deadline = std::time::Instant::now() + GRANTS_EVERY * 2;
        let mut seen = None;
        while std::time::Instant::now() < deadline {
            seen = read();
            if seen.as_ref().is_some_and(|v| v["permission"]["screen"] == "not-granted") {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let v = seen.expect("the file");
        assert_eq!((v["setup"].as_str(), v["permission"]["screen"].as_str()), (Some("all-set"), Some("not-granted")), "{v}");
        close(&rt);
        std::thread::sleep(GRANTS_EVERY + Duration::from_millis(200));
        assert!(!path.exists(), "the thread never writes after the close");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `grants_of` reads only the grants news.
    #[test]
    fn grants_of_reads_the_grants_news() {
        let g = grants_of(&OsNews::Grants { screen: true, mic: Access::Restricted, camera: Access::Denied }).unwrap();
        assert_eq!((g.screen, g.mic, g.camera), ("granted", "restricted", "denied"));
        assert!(grants_of(&OsNews::Woke).is_none());
    }
}
