//! The editor's lane (N63, DECISIONS B195): the record, the video and its sound for the
//! editor window, the save, and the permanent copy. Each `EditCmd` is answered once — its
//! news, or `Failed` with the editor's op, classified as the Library's lane classifies; a
//! Cancel answers nothing:
//!
//! - `Open`: the record read over the door a rename uses (`Opened`); the video found on this
//!   Mac — a take folder whose `take.json` or `publish.json` names the recording, with its
//!   `preview.mp4`; else one fetched before, `edits/<rec>/source.mp4` — or fetched now from
//!   the space into that file (`Getting`, at most every 250 ms); its sound read into
//!   `edits/<rec>/peaks.u8` (`shroom_encode::peaks`); both registered for `shroom-media:`
//!   (`media.rs`); `Ready`. Each open runs on a thread of its own, so a Cancel lands mid-fetch.
//! - `Cancel`: the open in flight stops — a cancelled open says nothing more, a failure
//!   included — and its video is no longer served.
//! - `Save`: `meta.edits` rewritten as the next revision (`shroom_upload::library::save_edits`,
//!   read-modify-write as a rename) — `Saved`.
//! - `Copy`: on a lane of its own, one at a time, never holding the window's commands up:
//!   the saved list's kept ranges cut from the video (the registered one, else found or
//!   fetched again — an app that restarted has none registered) by the encoder
//!   `find_encoder` finds, with the copy's own poster and teaser; all three put beside the
//!   recording, then named in the record (`set_edit_copy`) — `CopyDone`. A copy of a revision
//!   the record no longer holds is dropped, and answered `CopyDone` too.
//!
//! `edits/` (folder 700, files 600) holds what this Mac fetched and read, and a copy while
//! it's made; Disconnect and Leave's wipe removes it. Nothing here logs more than a
//! recording's short id.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_core::effect::EditCmd;
use shroom_core::event::{EditorNews, Event};
use shroom_core::world::{FailClass, Op, RecId, SpaceIdentity};
use shroom_storage::s3::S3Error;
use shroom_upload::meta::{Meta, State};
use shroom_upload::upload::{PART_SIZE, SINGLE_PUT_MAX, UploadError, with_retry};
use shroom_upload::{keys, library as records};

use super::library::{Doorway, class};
use super::media::{MEDIA, Registered};
use super::{Runtime, short};

/// The video fetched from the space, kept for the next edit of it.
pub const SOURCE: &str = "source.mp4";
/// Its sound, one byte per 20 ms (`shroom_encode::peaks`).
pub const PEAKS: &str = "peaks.u8";
/// The fetch's progress, at most this often.
const GETTING_EVERY: Duration = Duration::from_millis(250);

/// An open's stop: set by a Cancel (or the next Open), read between its steps and while it
/// fetches — and held while the open registers its video or says how it ended, so nothing
/// lands after a Cancel.
#[derive(Clone, Default)]
struct Stop(Arc<Mutex<bool>>);

impl Stop {
    fn stop(&self) {
        *self.0.lock().unwrap() = true;
    }

    fn stopped(&self) -> bool {
        *self.0.lock().unwrap()
    }

    /// Runs `f` unless stopped, the stop held meanwhile; whether it ran.
    fn unless_stopped(&self, f: impl FnOnce()) -> bool {
        let g = self.0.lock().unwrap();
        if *g {
            return false;
        }
        f();
        drop(g);
        true
    }
}

/// Why an open or a copy ended early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    /// A Cancel: nothing more is said.
    Stopped,
    Failed(FailClass),
}

impl From<FailClass> for Ended {
    fn from(c: FailClass) -> Ended {
        Ended::Failed(c)
    }
}

pub fn spawn(rt: Runtime) -> Sender<EditCmd> {
    let copies = spawn_copier(rt.clone());
    let (tx, rx) = channel::<EditCmd>();
    std::thread::Builder::new()
        .name("shroom-editor".into())
        .spawn(move || {
            let mut door = Doorway::default();
            // The open in flight: its recording, and its stop.
            let mut opening: Option<(RecId, Stop)> = None;
            for cmd in rx {
                match cmd {
                    EditCmd::Open { rec } => {
                        if let Some((_, stop)) = opening.take() {
                            stop.stop();
                        }
                        // One video at a time: the one before isn't served any more.
                        MEDIA.forget_all();
                        let stop = Stop::default();
                        opening = Some((rec.clone(), stop.clone()));
                        let rt2 = rt.clone();
                        let (rec2, stop2) = (rec.clone(), stop.clone());
                        if std::thread::Builder::new().name("shroom-editor-open".into()).spawn(move || open(&rt2, &rec2, &stop2)).is_err() {
                            stop.unless_stopped(|| rt.post(Event::Failed { op: Op::EditorOpen, class: FailClass::Unclassified }));
                        }
                    }
                    EditCmd::Cancel { rec } => {
                        if let Some((_, stop)) = opening.take_if(|(r, _)| *r == rec) {
                            stop.stop();
                        }
                        MEDIA.forget(&rec.0);
                        rt.log(&format!("editor: {} closed — its open stopped, its video no longer served", short(&rec.0)));
                    }
                    EditCmd::Save { rec, saved } => save(&rt, &mut door, &rec, &saved),
                    EditCmd::Copy { rec, rev } => {
                        if copies.send((rec, rev)).is_err() {
                            rt.post(Event::Failed { op: Op::EditorCopy, class: FailClass::Unclassified });
                        }
                    }
                }
            }
        })
        .expect("the editor lane starts");
    tx
}

/// The copies' own lane: one at a time, each minutes long, never in the window's way.
fn spawn_copier(rt: Runtime) -> Sender<(RecId, u64)> {
    let (tx, rx) = channel::<(RecId, u64)>();
    std::thread::Builder::new()
        .name("shroom-editor-copy".into())
        .spawn(move || {
            let mut door = Doorway::default();
            for (rec, rev) in rx {
                match copy(&rt, &mut door, &rec, rev) {
                    Ok(()) => rt.post(Event::Editor(EditorNews::CopyDone { rec, rev })),
                    Err(e) => {
                        door.forget();
                        let class = match e {
                            Ended::Failed(c) => c,
                            Ended::Stopped => FailClass::Unclassified,
                        };
                        rt.post(Event::Failed { op: Op::EditorCopy, class });
                    }
                }
            }
        })
        .expect("the editor's copy lane starts");
    tx
}

fn space(rt: &Runtime) -> Result<SpaceIdentity, FailClass> {
    rt.home.read_json::<SpaceIdentity>("space.json").ok_or_else(|| {
        rt.log("editor: no space on this Mac");
        FailClass::Refused
    })
}

/// `edits/<rec>/`, made private (folder 700) — only for a recording id as the space makes them.
fn folder(rt: &Runtime, rec: &str) -> Result<PathBuf, FailClass> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    if !super::posters::valid_id(rec) {
        rt.log("editor: not a recording id — refused");
        return Err(FailClass::Refused);
    }
    let dir = rt.home.edits().join(rec);
    let made = std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&dir).and_then(|()| {
        std::fs::set_permissions(rt.home.edits(), std::fs::Permissions::from_mode(0o700))?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
    });
    made.map(|()| dir).map_err(|e| {
        rt.log(&format!("editor: edits folder: {e}"));
        FailClass::Unclassified
    })
}

/// The record, read over `door`: never a tombstone.
fn record(rt: &Runtime, door: &mut Doorway, space: &SpaceIdentity, rec: &str) -> Result<Meta, FailClass> {
    let s3 = door.client(rt, space, rec)?;
    let read = with_retry(|| s3.get(&keys::meta(rec))).map_err(UploadError::S3);
    let bytes = match read {
        Ok(Some(b)) => b,
        Ok(None) => {
            rt.log(&format!("editor: {} has no record", short(rec)));
            return Err(FailClass::Refused);
        }
        Err(e) => {
            rt.log(&format!("editor: {}'s record: {e}", short(rec)));
            return Err(class(&e));
        }
    };
    let meta = serde_json::from_slice::<serde_json::Value>(&bytes).ok().and_then(shroom_upload::meta::listed).ok_or_else(|| {
        rt.log(&format!("editor: {}'s record doesn't read", short(rec)));
        FailClass::Unclassified
    })?;
    if meta.state == State::Deleted {
        rt.log(&format!("editor: {} was deleted", short(rec)));
        return Err(FailClass::Refused);
    }
    Ok(meta)
}

/// Whether a take folder names `rec` (its `take.json` or `publish.json`), as
/// `library::held_ids` reads them.
fn names(dir: &Path, rec: &str) -> bool {
    ["take.json", "publish.json"].iter().any(|f| {
        std::fs::read(dir.join(f)).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).is_some_and(|v| v.get("rec").and_then(|r| r.as_str()) == Some(rec))
    })
}

/// The video on this Mac, best first: a take's single-file copy, then one fetched before.
pub fn on_this_mac(takes: &Path, edits: &Path, rec: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = super::takes::dirs(takes)
        .into_iter()
        .filter(|d| names(d, rec))
        .map(|d| d.join(shroom_encode::files::PREVIEW))
        .filter(|p| p.is_file())
        .collect();
    let fetched = edits.join(rec).join(SOURCE);
    if fetched.is_file() {
        found.push(fetched);
    }
    found
}

/// Where the space keeps the video as one file: the record's `media.mp4` while it sits beside
/// the recording, else `rec/<id>/video.mp4`.
pub fn video_key(meta: &Meta) -> String {
    let prefix = keys::prefix(&meta.id);
    meta.media
        .as_ref()
        .and_then(|m| m.mp4.clone())
        .filter(|k| k.starts_with(&prefix) && !k.contains("..") && !k[prefix.len()..].is_empty())
        .unwrap_or_else(|| keys::mp4(&meta.id))
}

/// How far a fetch has come, in whole percent.
pub fn percent(done: u64, total: u64) -> u8 {
    if total == 0 { 0 } else { (done.min(total) as u128 * 100 / total as u128) as u8 }
}

/// Fetches the video into `edits/<rec>/source.mp4` (whole or not at all). With `getting`,
/// the editor hears how far it has come.
fn fetch(rt: &Runtime, door: &mut Doorway, space: &SpaceIdentity, meta: &Meta, dir: &Path, stop: Option<&Stop>, getting: bool) -> Result<PathBuf, Ended> {
    use std::os::unix::fs::PermissionsExt;
    let rec = RecId(meta.id.clone());
    let key = video_key(meta);
    let path = dir.join(SOURCE);
    let s3 = door.client(rt, space, &meta.id)?;
    if getting {
        rt.post(Event::Editor(EditorNews::Getting { rec: rec.clone(), pct: 0 }));
    }
    let t0 = Instant::now();
    let (mut said, mut at) = (0u8, Instant::now());
    let mut progress = |done: u64, total: u64| {
        let pct = percent(done, total);
        if getting && pct != said && at.elapsed() >= GETTING_EVERY {
            rt.post(Event::Editor(EditorNews::Getting { rec: rec.clone(), pct }));
            (said, at) = (pct, Instant::now());
        }
    };
    let cancel = || stop.is_some_and(Stop::stopped);
    match s3.get_to_file(&key, &path, &mut progress, &cancel) {
        Ok(true) => {
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            let mb = std::fs::metadata(&path).map(|m| m.len() / 1_000_000).unwrap_or(0);
            rt.log(&format!("editor: {} fetched from the space ({mb} MB in {} ms)", short(&meta.id), t0.elapsed().as_millis()));
            Ok(path)
        }
        Ok(false) => {
            rt.log(&format!("editor: {} has no single-file copy in the space", short(&meta.id)));
            Err(Ended::Failed(FailClass::Refused))
        }
        Err(S3Error::Cancelled) => Err(Ended::Stopped),
        Err(e) => {
            rt.log(&format!("editor: {}'s fetch: {e}", short(&meta.id)));
            Err(Ended::Failed(class(&UploadError::S3(e))))
        }
    }
}

/// `bytes` into `path`, atomically, mode 600.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(bytes)?;
    }
    std::fs::rename(&tmp, path)
}

fn open(rt: &Runtime, rec: &RecId, stop: &Stop) {
    match opened(rt, rec, stop) {
        Ok(()) => {}
        Err(Ended::Stopped) => rt.log(&format!("editor: {}'s open stopped", short(&rec.0))),
        Err(Ended::Failed(class)) => {
            stop.unless_stopped(|| rt.post(Event::Failed { op: Op::EditorOpen, class }));
        }
    }
}

fn opened(rt: &Runtime, rec: &RecId, stop: &Stop) -> Result<(), Ended> {
    let t0 = Instant::now();
    let space = space(rt)?;
    let dir = folder(rt, &rec.0)?;
    let mut door = Doorway::default();
    let meta = record(rt, &mut door, &space, &rec.0)?;
    let said = stop.unless_stopped(|| {
        rt.post(Event::Editor(EditorNews::Opened {
            rec: rec.clone(),
            saved: meta.edits.clone(),
            markers: meta.markers.clone().unwrap_or_default(),
            duration_sec: meta.duration_sec,
        }))
    });
    if !said {
        return Err(Ended::Stopped);
    }
    // The first copy on this Mac whose sound reads; else the space's.
    let mut read = None;
    for candidate in on_this_mac(&rt.home.takes(), &rt.home.edits(), &rec.0) {
        if stop.stopped() {
            return Err(Ended::Stopped);
        }
        match shroom_encode::peaks::read(&candidate) {
            Ok(p) => {
                read = Some((candidate, p));
                break;
            }
            Err(e) => rt.log(&format!("editor: a copy of {} here doesn't read ({e})", short(&rec.0))),
        }
    }
    let (video, peaks) = match read {
        Some(found) => found,
        None => {
            let video = fetch(rt, &mut door, &space, &meta, &dir, Some(stop), true)?;
            let peaks = shroom_encode::peaks::read(&video).map_err(|e| {
                rt.log(&format!("editor: {}'s sound doesn't read ({e})", short(&rec.0)));
                let _ = std::fs::remove_file(&video);
                Ended::Failed(FailClass::Unclassified)
            })?;
            (video, peaks)
        }
    };
    let peaks_file = dir.join(PEAKS);
    write_private(&peaks_file, &peaks.values).map_err(|e| {
        rt.log(&format!("editor: the peaks: {e}"));
        Ended::Failed(FailClass::Unclassified)
    })?;
    let ready = stop.unless_stopped(|| {
        MEDIA.register(Registered { rec: rec.0.clone(), video: video.clone(), peaks: peaks_file });
        rt.post(Event::Editor(EditorNews::Ready { rec: rec.clone(), duration: peaks.duration }));
    });
    if !ready {
        return Err(Ended::Stopped);
    }
    let from = if video.starts_with(rt.home.takes()) { "a take on this Mac" } else { "this Mac's fetched copy" };
    rt.log(&format!("editor: {} ready from {from} — {:.1} s, {} peaks, in {} ms", short(&rec.0), peaks.duration, peaks.values.len(), t0.elapsed().as_millis()));
    Ok(())
}

fn save(rt: &Runtime, door: &mut Doorway, rec: &RecId, saved: &shroom_edit::Saved) {
    let result = space(rt).and_then(|space| {
        let s3 = door.client(rt, &space, &rec.0)?;
        records::save_edits(&s3, &rec.0, &saved.list, saved.rev).map_err(|e| {
            rt.log(&format!("editor: {}'s save: {e}", short(&rec.0)));
            class(&e)
        })
    });
    match result {
        Ok(rev) => {
            rt.log(&format!("editor: {} saved as revision {rev}", short(&rec.0)));
            rt.post(Event::Editor(EditorNews::Saved { rec: rec.clone(), rev }));
        }
        Err(class) => {
            door.forget();
            rt.post(Event::Failed { op: Op::EditorSave, class });
        }
    }
}

/// The picture encoder for the copy: VideoToolbox (the release's encoder has it), else x264
/// where a dev build's encoder lacks it.
fn encoder(ffmpeg: &Path) -> shroom_encode::cut::Encoder {
    use shroom_encode::cut::{Encoder, has_encoder};
    if !has_encoder(ffmpeg, Encoder::VideoToolbox.codec()) && has_encoder(ffmpeg, Encoder::Libx264.codec()) { Encoder::Libx264 } else { Encoder::VideoToolbox }
}

fn copy(rt: &Runtime, door: &mut Doorway, rec: &RecId, rev: u64) -> Result<(), Ended> {
    let t0 = Instant::now();
    let space = space(rt)?;
    let dir = folder(rt, &rec.0)?;
    let meta = record(rt, door, &space, &rec.0)?;
    // Only the revision asked for: a newer save came first, or the edit went.
    let Some(saved) = meta.edits.as_ref().filter(|s| s.rev == rev) else {
        rt.log(&format!("editor: {}'s copy of revision {rev} dropped — the record moved on", short(&rec.0)));
        return Ok(());
    };
    let mut list = saved.list.clone();
    if let Some(d) = meta.duration_sec.filter(|d| d.is_finite() && *d > 0.0) {
        list.source_duration = d;
    }
    let list = list.checked();
    if list.is_identity() {
        rt.log(&format!("editor: {}'s revision {rev} is the original — no copy", short(&rec.0)));
        return Ok(());
    }
    let keep = list.keep();
    let source = match MEDIA.video(&rec.0).filter(|p| p.is_file()).or_else(|| on_this_mac(&rt.home.takes(), &rt.home.edits(), &rec.0).into_iter().next()) {
        Some(p) => p,
        None => fetch(rt, door, &space, &meta, &dir, None, false)?,
    };
    let ffmpeg = super::locate_ffmpeg().ok_or_else(|| {
        rt.log("editor: no encoder for the copy");
        Ended::Failed(FailClass::Unclassified)
    })?;
    let enc = encoder(&ffmpeg);
    let names = [shroom_edit::EditCopy::file_for(rev), shroom_edit::EditCopy::poster_for(rev), shroom_edit::EditCopy::teaser_for(rev)];
    let made = (|| -> Result<(), Ended> {
        let out = dir.join(&names[0]);
        let duration = shroom_encode::cut::render(&ffmpeg, &source, &keep, &out, &enc).map_err(|e| {
            rt.log(&format!("editor: {}'s copy didn't cut ({e})", short(&rec.0)));
            Ended::Failed(FailClass::Unclassified)
        })?;
        // Its own first frame and teaser: a trimmed-off start never shows in an unfurl.
        let poster = shroom_encode::take::poster_of(&ffmpeg, &out, &dir.join(&names[1])).map_err(|e| rt.log(&format!("editor: the copy's poster: {e}"))).ok().map(|()| names[1].clone());
        let teaser = shroom_encode::take::teaser_of(&ffmpeg, &out, &dir.join(&names[2]), &enc).map_err(|e| rt.log(&format!("editor: the copy's teaser: {e}"))).ok().map(|()| names[2].clone());
        rt.log(&format!("editor: {}'s copy of revision {rev} cut — {duration:.1} s, in {} ms", short(&rec.0), t0.elapsed().as_millis()));
        // The bytes go up under a door opened now: a `manage` credential lives 15 minutes,
        // and a long recording's cut can take longer.
        let s3 = door.client(rt, &space, &rec.0)?;
        let up = |name: &str, kind: &str| -> Result<(), Ended> {
            with_retry(|| s3.put_file(&format!("{}{name}", keys::prefix(&rec.0)), &dir.join(name), kind, SINGLE_PUT_MAX, PART_SIZE)).map_err(|e| {
                rt.log(&format!("editor: {}'s {name} didn't go up ({e})", short(&rec.0)));
                Ended::Failed(class(&UploadError::S3(e)))
            })
        };
        up(&names[0], "video/mp4")?;
        if let Some(p) = &poster {
            up(p, "image/jpeg")?;
        }
        if let Some(t) = &teaser {
            up(t, "video/mp4")?;
        }
        let copy = shroom_edit::EditCopy { rev, file: names[0].clone(), duration, poster, teaser };
        let current = records::set_edit_copy(&s3, &rec.0, &copy).map_err(|e| {
            rt.log(&format!("editor: {}'s copy wasn't named ({e})", short(&rec.0)));
            Ended::Failed(class(&e))
        })?;
        rt.log(&format!(
            "editor: {}'s copy of revision {rev} {} in {} ms",
            short(&rec.0),
            if current { "is up and named" } else { "was dropped — a newer save came first" },
            t0.elapsed().as_millis()
        ));
        Ok(())
    })();
    for name in &names {
        let _ = std::fs::remove_file(dir.join(name));
    }
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_video_is_found_on_this_mac_by_its_takes_then_a_fetch_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let (takes, edits) = (tmp.path().join("takes"), tmp.path().join("edits"));
        let take = |name: &str, file: &str, rec: &str, preview: bool| {
            let d = takes.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(file), format!(r#"{{"v":1,"rec":"{rec}","created":1,"title":"t","stopped":1}}"#)).unwrap();
            if preview {
                std::fs::write(d.join("preview.mp4"), b"mp4").unwrap();
            }
            d
        };
        let none: Vec<PathBuf> = vec![];
        assert_eq!(on_this_mac(&takes, &edits, "AbCd"), none, "nothing here");
        take("20261002-120000-0001", "take.json", "Other", true);
        take("20261002-120000-0002", "take.json", "AbCd", false);
        assert_eq!(on_this_mac(&takes, &edits, "AbCd"), none, "a take without its single-file copy");
        let pub3 = take("20261002-120000-0003", "publish.json", "AbCd", true);
        std::fs::create_dir_all(edits.join("AbCd")).unwrap();
        std::fs::write(edits.join("AbCd").join(SOURCE), b"mp4").unwrap();
        assert_eq!(on_this_mac(&takes, &edits, "AbCd"), vec![pub3.join("preview.mp4"), edits.join("AbCd").join(SOURCE)], "the take's first, then the fetched one");
        assert_eq!(on_this_mac(&takes, &edits, "Other").len(), 1);
    }

    #[test]
    fn the_space_keeps_the_video_beside_the_recording() {
        let mut m = Meta::uploading("AbCd", "t", "2026-10-02T00:00:00Z");
        assert_eq!(video_key(&m), "rec/AbCd/video.mp4", "no media: the uploader's name");
        let media = |mp4: Option<&str>| shroom_upload::meta::Media { hls: "rec/AbCd/master.m3u8".into(), mp4: mp4.map(String::from), poster: None, teaser: None, width: 1, height: 1 };
        m.media = Some(media(Some("rec/AbCd/edit-3.mp4")));
        assert_eq!(video_key(&m), "rec/AbCd/edit-3.mp4", "the record's own");
        for elsewhere in ["rec/Other/video.mp4", "rec/AbCd/../Other/video.mp4", "meta/AbCd.json", "rec/AbCd/"] {
            m.media = Some(media(Some(elsewhere)));
            assert_eq!(video_key(&m), "rec/AbCd/video.mp4", "{elsewhere}: never outside the recording");
        }
        m.media = Some(media(None));
        assert_eq!(video_key(&m), "rec/AbCd/video.mp4");
    }

    #[test]
    fn a_fetch_reads_in_whole_percent() {
        assert_eq!(percent(0, 0), 0, "a store that doesn't say");
        assert_eq!(percent(0, 1000), 0);
        assert_eq!(percent(399, 1000), 39);
        assert_eq!(percent(1000, 1000), 100);
        assert_eq!(percent(2000, 1000), 100);
        assert_eq!(percent(u64::MAX / 2, u64::MAX), 49);
    }

    #[test]
    fn a_stopped_open_says_nothing_more() {
        let stop = Stop::default();
        let mut said = 0;
        assert!(stop.unless_stopped(|| said += 1));
        stop.stop();
        assert!(!stop.unless_stopped(|| said += 1));
        assert_eq!(said, 1);
        assert!(stop.stopped());
    }

    /// The lane against no space: an open fails as the core waits to hear, a save too; a
    /// Cancel answers nothing, and a cancelled open's failure never lands.
    #[test]
    fn every_command_is_answered_once_and_a_cancel_never() {
        use std::time::Duration;
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        let lane = spawn(rt.clone());
        let next = || rx.recv_timeout(Duration::from_secs(5)).map(|e| e.event);
        let rec = RecId("AbCdEfGhIjKlMnOp".into());
        lane.send(EditCmd::Open { rec: rec.clone() }).unwrap();
        assert!(matches!(next(), Ok(Event::Failed { op: Op::EditorOpen, class: FailClass::Refused })));
        lane.send(EditCmd::Save { rec: rec.clone(), saved: shroom_edit::Saved::new(shroom_edit::EditList::new(10.0), 1) }).unwrap();
        assert!(matches!(next(), Ok(Event::Failed { op: Op::EditorSave, class: FailClass::Refused })));
        lane.send(EditCmd::Copy { rec: rec.clone(), rev: 1 }).unwrap();
        assert!(matches!(next(), Ok(Event::Failed { op: Op::EditorCopy, class: FailClass::Refused })));
        lane.send(EditCmd::Cancel { rec: rec.clone() }).unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "a Cancel answers nothing");
    }
}
