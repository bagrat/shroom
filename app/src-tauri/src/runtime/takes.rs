//! A take directory is its own truth (ARCHITECTURE §3.4): its marker files say what it is,
//! and the scan classifies it with a pure function.
//!
//!   LOCK (flock held)  live — skipped, never touched
//!   take.json          written when capture starts {rec, created}
//!   finalized.json     finalized {length_ms, why}
//!   publish.json       the one durable intent: a link was made {rec, title, stopped}
//!   uploaded           done
//!   tombstoned         deleted elsewhere: never re-queued
//!   set-aside          a switch's take whose copy to Movies failed: kept, never sent
//!                      anywhere (read as tombstoned — DEVIATIONS #27)

use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shroom_core::time::Wall;
use shroom_core::world::{Queued, RecId, SavedCause, SavedTake, TakeDir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeJson {
    pub v: u32,
    pub rec: String,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizedJson {
    pub v: u32,
    pub length_ms: u64,
    /// `stop`, `interrupted`, `disk-full`, `screen-off`
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishJson {
    pub v: u32,
    pub rec: String,
    pub title: String,
    pub stopped: i64,
}

/// When a take's upload credential (`cred.json`) expires, if it holds one.
pub fn minted_until(dir: &Path) -> Option<Wall> {
    let f: shroom_cloud::device::CredFile = crate::runtime::home::read_json(&dir.join("cred.json"))?;
    (f.purpose == "upload").then(|| crate::runtime::member::iso_ms(&f.expires_at)).flatten().map(Wall)
}

/// A new take directory name: `yyyymmdd-hhmmss-<4 random>`.
pub fn new_dir_name(wall_ms: i64) -> String {
    let stamp = shroom_storage::sigv4::amz_date((wall_ms / 1000) as u64); // YYYYMMDDTHHMMSSZ
    let mut r = [0u8; 2];
    let _ = getrandom::fill(&mut r);
    format!("{}-{}-{:02x}{:02x}", &stamp[0..8], &stamp[9..15], r[0], r[1])
}

/// Holds `LOCK` for as long as the value lives (released when the process dies, too).
pub struct TakeLock {
    _file: File,
}

impl TakeLock {
    pub fn acquire(dir: &Path) -> std::io::Result<TakeLock> {
        let file = OpenOptions::new().create(true).write(true).truncate(false).open(dir.join("LOCK"))?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(TakeLock { _file: file })
    }
}

fn is_locked(dir: &Path) -> bool {
    match OpenOptions::new().write(true).open(dir.join("LOCK")) {
        Ok(f) => {
            let busy = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0;
            if !busy {
                unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_UN) };
            }
            busy
        }
        Err(_) => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Live,
    /// Unlocked with no `finalized.json`: a crash mid-take. Needs repair before it's saved.
    Interrupted,
    Saved(SavedTake),
    Queued(Queued),
    Done,
    /// Deleted elsewhere: never re-queued, but still this Mac's to keep or toss — the
    /// published take it was, when its `publish.json` is readable (DEVIATIONS #7).
    Tombstoned(Option<Queued>),
    Unreadable,
}

pub fn classify(dir: &Path) -> Kind {
    if is_locked(dir) {
        return Kind::Live;
    }
    classify_held(dir)
}

/// What a take directory is, read by the one holding its lock (the recorder, deciding what
/// an abandoned take kept): everything `classify` reads but the lock.
pub fn classify_held(dir: &Path) -> Kind {
    let name = TakeDir(dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let tombstoned = dir.join("tombstoned").exists() || dir.join("set-aside").exists();
    if !tombstoned && dir.join("uploaded").exists() {
        return Kind::Done;
    }
    let take: Option<TakeJson> = crate::runtime::home::read_json(&dir.join("take.json"));
    let finalized: Option<FinalizedJson> = crate::runtime::home::read_json(&dir.join("finalized.json"));
    let publish: Option<PublishJson> = crate::runtime::home::read_json(&dir.join("publish.json"));
    let queued = |p: PublishJson, f: Option<&FinalizedJson>| Queued {
        dir: name.clone(),
        rec: RecId(p.rec),
        title: p.title,
        stopped: Wall(p.stopped),
        done: 0,
        total: 0,
        blocked: None,
        length_ms: f.map(|f| f.length_ms),
    };
    if tombstoned {
        return Kind::Tombstoned(publish.map(|p| queued(p, finalized.as_ref())));
    }
    match (take, finalized, publish) {
        (_, f, Some(p)) => Kind::Queued(queued(p, f.as_ref())),
        (Some(t), Some(f), None) => Kind::Saved(SavedTake {
            dir: name,
            rec: RecId(t.rec),
            length_ms: f.length_ms,
            cause: match f.why.as_str() {
                "disk-full" => SavedCause::DiskFull,
                "screen-off" => SavedCause::ScreenOff,
                _ => SavedCause::Interrupted,
            },
            created: Wall(t.created),
            movies: dir.join("movies.json").exists(),
        }),
        (Some(_), None, None) => Kind::Interrupted,
        _ => Kind::Unreadable,
    }
}

/// Every take directory, oldest first: the real folders in `takes/` (a link is never
/// followed) named as a take's are (`posters::valid_dir`).
pub fn dirs(takes: &Path) -> Vec<PathBuf> {
    let take = |e: &fs::DirEntry| e.file_type().is_ok_and(|t| t.is_dir()) && e.file_name().to_str().is_some_and(crate::runtime::posters::valid_dir);
    let mut v: Vec<PathBuf> = fs::read_dir(takes)
        .map(|it| it.filter_map(|e| e.ok()).filter(take).map(|e| e.path()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

// ── the takes lane ──

use std::sync::mpsc::{Sender, channel};

use shroom_core::effect::TakeCmd;
use shroom_core::event::{Event, RecNews};

pub fn spawn_lane(rt: crate::runtime::Runtime) -> Sender<TakeCmd> {
    let (tx, rx) = channel::<TakeCmd>();
    std::thread::Builder::new()
        .name("shroom-takes".into())
        .spawn(move || {
            for cmd in rx {
                match cmd {
                    TakeCmd::Rescan => {
                        let mut saved = Vec::new();
                        let mut queue = Vec::new();
                        let mut removed = Vec::new();
                        for dir in dirs(&rt.home.takes()) {
                            let mut kind = classify(&dir);
                            if kind == Kind::Interrupted {
                                repair(&dir);
                                kind = classify(&dir);
                            }
                            match kind {
                                Kind::Saved(s) => saved.push(s),
                                Kind::Queued(q) => queue.push(q),
                                Kind::Tombstoned(Some(q)) => removed.push(q),
                                _ => {}
                            }
                        }
                        // Which of them already hold an upload credential, and until when: a
                        // space that stops minting for this Mac lets those finish (N32, #22).
                        let minted = queue.iter().filter_map(|q| minted_until(&rt.home.takes().join(&q.dir.0)).map(|w| (q.dir.clone(), w))).collect();
                        rt.post(Event::Scanned { saved, queue, removed });
                        rt.post(Event::Upload(shroom_core::event::UploadNews::Credentials(minted)));
                    }
                    TakeCmd::Delete(dir) => {
                        let _ = fs::remove_dir_all(rt.home.takes().join(&dir.0));
                        rt.post(Event::Rec(RecNews::Deleted(dir)));
                    }
                    TakeCmd::SaveToMovies(copies) => {
                        let quiet = rt.quiet.load(std::sync::atomic::Ordering::SeqCst);
                        let movies = movies_dir(&rt);
                        for c in copies {
                            match save_one(&rt, movies.as_deref(), &c) {
                                Ok(dest) => {
                                    let _ = crate::runtime::home::write_json(&rt.home.takes().join(&c.dir.0).join("movies.json"), &serde_json::json!({ "v": 1, "path": dest }));
                                    if quiet {
                                        rt.log(&format!("shell would reveal {}", dest.display()));
                                    } else {
                                        let _ = std::process::Command::new("open").arg("-R").arg(&dest).status();
                                    }
                                    rt.post(Event::Rec(RecNews::SavedToMovies(c.dir)));
                                }
                                Err(e) => {
                                    // Said on the face (the button asks again), never only here.
                                    rt.log(&format!("takes: {} couldn't be saved to Movies ({e})", c.dir.0));
                                    rt.post(Event::Rec(RecNews::SaveFailed(c.dir)));
                                }
                            }
                        }
                    }
                    TakeCmd::SetAside(dir) => {
                        let path = rt.home.takes().join(&dir.0);
                        if path.is_dir() {
                            let _ = fs::write(path.join("set-aside"), b"");
                        }
                        rt.log(&format!("takes: {} set aside (its copy to Movies failed) — kept, never sent", dir.0));
                    }
                }
            }
        })
        .expect("the takes lane starts");
    tx
}

/// Where Save to Movies puts files: `~/Movies` — a throwaway home's own `Movies/` in a dev
/// run (a test never writes into the person's).
pub fn movies_dir(rt: &crate::runtime::Runtime) -> Option<PathBuf> {
    if rt.throwaway { Some(rt.home.root.join("Movies")) } else { crate::runtime::home::user_home().map(|h| h.join("Movies")) }
}

/// One take into Movies: its single-file copy (made from the take's own files if the
/// uploader never made it, N34), copied under a safe, unused name — never over a file.
fn save_one(rt: &crate::runtime::Runtime, movies: Option<&Path>, c: &shroom_core::effect::MoviesCopy) -> std::io::Result<PathBuf> {
    let movies = movies.ok_or_else(|| std::io::Error::other("no Movies folder"))?;
    let dir = rt.home.takes().join(&c.dir.0);
    if !dir.is_dir() || is_locked(&dir) {
        return Err(std::io::Error::other("no finished take there"));
    }
    let ffmpeg = crate::runtime::locate_ffmpeg().ok_or_else(|| std::io::Error::other("no ffmpeg"))?;
    let src = shroom_encode::take::ensure_preview(&ffmpeg, &dir)?;
    fs::create_dir_all(movies)?;
    place(&src, movies, &file_stem(&c.name))
}

/// A take's name as a file name: no `/` or `:` (Finder's separators), no leading dots (a
/// hidden file), no control characters, at most ~200 bytes; “shroom recording” if nothing
/// is left.
pub fn file_stem(name: &str) -> String {
    let cleaned: String = name.chars().map(|ch| if ch == '/' || ch == ':' { '-' } else if ch.is_control() { ' ' } else { ch }).collect();
    let mut stem = cleaned.trim().trim_start_matches('.').trim().to_string();
    while stem.len() > 200 {
        stem.pop();
    }
    if stem.is_empty() { "shroom recording".into() } else { stem }
}

/// Copies `src` into `movies` as `<stem>.mp4`, else `<stem> 2.mp4`, `<stem> 3.mp4`, … —
/// never over an existing file, even one that appears meanwhile (the copy lands under a
/// hidden temporary name, then is linked in, which fails rather than replace).
pub fn place(src: &Path, movies: &Path, stem: &str) -> std::io::Result<PathBuf> {
    let tmp = movies.join(format!(".{stem}.shroom-partial"));
    fs::copy(src, &tmp)?;
    let result = (1..10_000)
        .map(|n| movies.join(if n == 1 { format!("{stem}.mp4") } else { format!("{stem} {n}.mp4") }))
        .find_map(|dest| match fs::hard_link(&tmp, &dest) {
            Ok(()) => Some(Ok(dest)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
            Err(e) => Some(Err(e)),
        })
        .unwrap_or_else(|| Err(std::io::Error::other("no free name")));
    let _ = fs::remove_file(&tmp);
    result
}

/// A crash mid-take: rebuild what's on disk into a saved take (law 2) and record it.
pub fn repair(dir: &Path) {
    let Some(ffmpeg) = crate::runtime::locate_ffmpeg() else { return };
    if let Some(r) = shroom_record::repair::repair(&ffmpeg, dir) {
        // Re-cut from the fragmented copy: segments sent while it recorded are other bytes
        // now — its upload trusts none of them (B70). Written before finalized.json, so a
        // take that reads as saved always carries it.
        if r.from_fragments {
            let _ = fs::write(dir.join(shroom_upload::upload::REBUILT_FILE), b"");
        }
        let length_ms = (r.seconds * 1000.0) as u64;
        let _ = crate::runtime::home::write_json(&dir.join("finalized.json"), &FinalizedJson { v: 1, length_ms, why: "interrupted".into() });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_scan_reads_each_takes_upload_credential_and_its_expiry() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(minted_until(tmp.path()), None);
        let f = |purpose: &str| serde_json::json!({ "v": 1, "purpose": purpose, "accessKeyId": "k", "secretAccessKey": "s", "sessionToken": "t", "expiresAt": "2026-10-09T12:00:00Z", "bucket": "b", "endpoint": "e" });
        crate::runtime::home::write_json(&tmp.path().join("cred.json"), &f("upload")).unwrap();
        assert_eq!(minted_until(tmp.path()), Some(Wall(1_791_547_200_000)));
        crate::runtime::home::write_json(&tmp.path().join("cred.json"), &f("manage")).unwrap();
        assert_eq!(minted_until(tmp.path()), None, "only an upload credential");
    }

    /// The take folders are the real folders in `takes/` named as a take's are: a link to a
    /// folder elsewhere, a file, or a folder of another name is no take.
    #[test]
    fn the_takes_are_real_folders_named_as_takes_are() {
        let tmp = tempfile::tempdir().unwrap();
        let takes = tmp.path().join("takes");
        let elsewhere = tmp.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        for d in ["20261002-120000-ab12", "20261002-101500-ab12", ".hidden", "-lead", "has space", "under_score"] {
            fs::create_dir_all(takes.join(d)).unwrap();
        }
        fs::write(takes.join("20261002-130000-ab12"), b"a file").unwrap();
        std::os::unix::fs::symlink(&elsewhere, takes.join("20261002-140000-ab12")).unwrap();
        assert_eq!(dirs(&takes), [takes.join("20261002-101500-ab12"), takes.join("20261002-120000-ab12")]);
        assert!(dirs(&tmp.path().join("none")).is_empty());
    }

    use super::*;

    fn touch(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn classifies_by_markers() {
        let root = tempfile::tempdir().unwrap();
        let d = root.path().join("20261002-120000-ab12");
        fs::create_dir(&d).unwrap();
        touch(&d, "take.json", r#"{"v":1,"rec":"AbCdEfGhIjKlMnOp","created":1}"#);
        assert_eq!(classify(&d), Kind::Interrupted, "no finalized.json and no lock: a crash");

        let lock = TakeLock::acquire(&d).unwrap();
        assert_eq!(classify(&d), Kind::Live, "a held lock is live");
        assert_eq!(classify_held(&d), Kind::Interrupted, "but its holder reads what it is (the recorder, abandoning it — #53)");
        drop(lock);
        // flock belongs to the open file, and a process another test thread is starting
        // holds a copy of every open file until its exec closes it: for that moment the
        // released lock still reads held (macOS CI, 2026-10-14). Wait it out; a lock that
        // stays held after the drop is still a failure.
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while is_locked(&d) {
            assert!(std::time::Instant::now() < until, "the lock outlived its holder");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        touch(&d, "finalized.json", r#"{"v":1,"length_ms":272000,"why":"interrupted"}"#);
        assert!(matches!(classify(&d), Kind::Saved(s) if s.length_ms == 272_000 && s.cause == SavedCause::Interrupted));

        touch(&d, "publish.json", r#"{"v":1,"rec":"AbCdEfGhIjKlMnOp","title":"Untitled — Oct 2","stopped":5}"#);
        assert!(matches!(classify(&d), Kind::Queued(q) if q.title == "Untitled — Oct 2"));

        assert!(matches!(classify(&d), Kind::Queued(q) if q.length_ms == Some(272_000)), "its length, known since the stop");

        touch(&d, "uploaded", "");
        assert_eq!(classify(&d), Kind::Done);

        touch(&d, "tombstoned", "");
        assert!(matches!(classify(&d), Kind::Tombstoned(Some(q)) if q.title == "Untitled — Oct 2"), "still its maker's, across a relaunch");
    }

    #[test]
    fn a_takes_file_name_is_safe_and_never_overwrites() {
        assert_eq!(file_stem("Where the 502s come from"), "Where the 502s come from");
        assert_eq!(file_stem("shroom recording — Aug 21"), "shroom recording — Aug 21");
        assert_eq!(file_stem("a/b: c"), "a-b- c");
        assert_eq!(file_stem("..hidden"), "hidden");
        assert_eq!(file_stem(" . "), "shroom recording");
        assert_eq!(file_stem("tab\there"), "tab here");
        assert!(file_stem(&"x".repeat(500)).len() <= 200);

        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("preview.mp4");
        fs::write(&src, b"movie").unwrap();
        let movies = root.path().join("Movies");
        fs::create_dir(&movies).unwrap();
        fs::write(movies.join("Demo.mp4"), b"someone else's").unwrap();
        let a = place(&src, &movies, "Demo").unwrap();
        let b = place(&src, &movies, "Demo").unwrap();
        assert_eq!((a.file_name().unwrap().to_str(), b.file_name().unwrap().to_str()), (Some("Demo 2.mp4"), Some("Demo 3.mp4")));
        assert_eq!(fs::read(movies.join("Demo.mp4")).unwrap(), b"someone else's", "never overwritten");
        assert_eq!(fs::read(&b).unwrap(), b"movie");
        let left: Vec<_> = fs::read_dir(&movies).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert!(!left.iter().any(|n| n.starts_with('.')), "no temporary left behind: {left:?}");
    }

    #[test]
    fn a_set_aside_take_is_never_requeued() {
        let root = tempfile::tempdir().unwrap();
        let d = root.path().join("20261002-120000-ab12");
        fs::create_dir(&d).unwrap();
        touch(&d, "take.json", r#"{"v":1,"rec":"AbCdEfGhIjKlMnOp","created":1}"#);
        touch(&d, "finalized.json", r#"{"v":1,"length_ms":5000,"why":"stop"}"#);
        touch(&d, "publish.json", r#"{"v":1,"rec":"AbCdEfGhIjKlMnOp","title":"Raced","stopped":5}"#);
        touch(&d, "set-aside", "");
        assert!(matches!(classify(&d), Kind::Tombstoned(Some(q)) if q.title == "Raced"));
    }

    #[test]
    fn dir_names_sort_by_time() {
        assert!(new_dir_name(1_790_000_000_000) < new_dir_name(1_790_000_100_000));
        assert_eq!(new_dir_name(1_790_000_000_000).len(), "20260921-120000-ab12".len());
    }
}
