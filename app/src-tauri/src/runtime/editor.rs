//! The editor's lane (N63, DECISIONS B195, B197): the record, the video and its sound for the
//! editor window, the save, the permanent copy, the bake and the upkeep. Each `EditCmd` is
//! answered once — its news, or `Failed` with the editor's op, classified as the Library's
//! lane classifies; a Cancel and a Bake answer nothing:
//!
//! - `Open`: the record read over the door a rename uses (`Opened`); the video found on this
//!   Mac — a take folder whose `take.json` or `publish.json` names the recording, with its
//!   `preview.mp4`, while the record still plays the original; else the file fetched before
//!   (`edits/<rec>/source.mp4`, kept with the key it came from, `source.key`) when it is the
//!   one the record plays — or fetched now from the space into that file (`Getting`, at most
//!   every 250 ms); its sound read into `edits/<rec>/peaks.u8` (`shroom_encode::peaks`);
//!   both registered for `shroom-media:` (`media.rs`); `Ready`. Each open runs on a thread of
//!   its own, so a Cancel lands mid-fetch. A baked record never opens on its original.
//! - `Cancel`: the open in flight stops — a cancelled open says nothing more, a failure
//!   included — and its video is no longer served.
//! - `Save`: `meta.edits` rewritten as the next revision
//!   (`shroom_upload::library::save_edits_over`, read-modify-write as a rename) — `Saved`.
//!   Refused, nothing written, when the record no longer plays what the editor opened on (an
//!   edit became the video since, on this Mac or another): the list was made on a clock the
//!   record no longer has. Judged inside the write, so no bake slips in between.
//! - `Copy`: on a lane of its own, one at a time, never holding the window's commands up:
//!   the saved list's kept ranges cut from the video (the registered one, else found or
//!   fetched again — an app that restarted has none registered) by the encoder
//!   `find_encoder` finds, with the copy's own poster and teaser; all three put beside the
//!   recording, then named in the record with when the original goes (`set_edit_copy`,
//!   two days on) — `Making` as it goes (the cut to 80%, the files sent to 100%), then
//!   `CopyDone`. A copy of a revision the record no longer holds is dropped: `CopyDone` too.
//! - `Bake`: “Yes, delete” — the edit becomes the video (`library::bake`), on a thread of its
//!   own; its outcome is logged, and one that doesn't land is the upkeep's in two days.
//! - `Upkeep` (B197 (d)): on a lane of its own, never in the window's way — of the records
//!   the space lists for this Mac, read now (a member's own, every one on an owner-class
//!   Mac: `Doorway::listed`), the ones past their two days are baked and the copies they
//!   lack are made where the video is on this Mac (`library::upkeep_plan`); then `edits/`
//!   drops what nobody needs — `UpkeepDone`.
//!
//! `edits/` (folder 700, files 600) holds what this Mac fetched and read, and a copy while
//! it's made; a recording's folder goes once it's baked (here or elsewhere) or gone from the
//! space, or after a week unused, and the whole of it with the space (Disconnect, Leave).
//! Nothing here logs more than a recording's short id.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use shroom_core::effect::EditCmd;
use shroom_core::event::{EditorNews, Event};
use shroom_core::time::Wall;
use shroom_core::world::{FailClass, Op, RecId, SpaceIdentity};
use shroom_storage::s3::S3Error;
use shroom_upload::library::Plays;
use shroom_upload::meta::{Meta, State, now_iso};
use shroom_upload::upload::{PART_SIZE, SINGLE_PUT_MAX, UploadError, with_retry};
use shroom_upload::{keys, library as records};

use super::library::{Doorway, class};
use super::media::{MEDIA, Registered};
use super::{Runtime, short};

/// The video fetched from the space, kept for the next edit of it.
pub const SOURCE: &str = "source.mp4";
/// Which of the record's files `source.mp4` is (`rec/<id>/video.mp4`, or a baked edit's).
pub const SOURCE_KEY: &str = "source.key";
/// Its sound, one byte per 20 ms (`shroom_encode::peaks`).
pub const PEAKS: &str = "peaks.u8";
/// Progress, at most this often.
const EVERY: Duration = Duration::from_millis(250);
/// A recording's folder in `edits/` unused this long goes.
const UNUSED: Duration = Duration::from_secs(7 * 24 * 3600);
/// A copy on this Mac whose picture is this far from the record's length isn't the video
/// the record plays.
const LENGTH_SLACK: f64 = 1.0;

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

impl Ended {
    fn class(self) -> FailClass {
        match self {
            Ended::Failed(c) => c,
            Ended::Stopped => FailClass::Unclassified,
        }
    }
}

/// What each record the editor opened on played then (`records::Plays`: which file, how many
/// bakes), by id: a save over a record that moved on from it is refused.
static OPENED: Mutex<Option<HashMap<String, Plays>>> = Mutex::new(None);

fn opened_as(rec: &str) -> Option<Plays> {
    OPENED.lock().unwrap().as_ref()?.get(rec).cloned()
}

pub fn spawn(rt: Runtime) -> Sender<EditCmd> {
    let copies = spawn_copier(rt.clone());
    let upkeeps = spawn_upkeeper(rt.clone());
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
                    EditCmd::Bake { rec } => {
                        let rt2 = rt.clone();
                        if std::thread::Builder::new().name("shroom-editor-bake".into()).spawn(move || bake(&rt2, &rec)).is_err() {
                            rt.log("editor: the bake's thread didn't start — the upkeep bakes it in two days");
                        }
                    }
                    EditCmd::Upkeep => {
                        if upkeeps.send(()).is_err() {
                            rt.post(Event::Failed { op: Op::EditorUpkeep, class: FailClass::Unclassified });
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
                let mut making = Making::window(&rt, &rec);
                match copy(&rt, &mut door, &rec, rev, &mut making) {
                    Ok(_) => rt.post(Event::Editor(EditorNews::CopyDone { rec, rev })),
                    Err(e) => {
                        door.forget();
                        rt.post(Event::Failed { op: Op::EditorCopy, class: e.class() });
                    }
                }
            }
        })
        .expect("the editor's copy lane starts");
    tx
}

/// The upkeep's own lane: one pass at a time, never in the window's way.
fn spawn_upkeeper(rt: Runtime) -> Sender<()> {
    let (tx, rx) = channel::<()>();
    std::thread::Builder::new()
        .name("shroom-editor-upkeep".into())
        .spawn(move || {
            let mut door = Doorway::default();
            for () in rx {
                match upkeep(&rt, &mut door) {
                    Ok((baked, copies)) => rt.post(Event::Editor(EditorNews::UpkeepDone { baked, copies })),
                    Err(class) => {
                        door.forget();
                        rt.post(Event::Failed { op: Op::EditorUpkeep, class });
                    }
                }
            }
        })
        .expect("the editor's upkeep lane starts");
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

/// The video on this Mac that the record plays, best first: a take's single-file copy while
/// the record plays the original (a baked record plays its edit's copy, which no take
/// holds), then the file fetched before, when it's the one the record plays now.
pub fn on_this_mac(takes: &Path, edits: &Path, meta: &Meta) -> Vec<PathBuf> {
    let rec = meta.id.as_str();
    let key = video_key(meta);
    let mut found: Vec<PathBuf> = Vec::new();
    if key == keys::mp4(rec) {
        found.extend(super::takes::dirs(takes).into_iter().filter(|d| names(d, rec)).map(|d| d.join(shroom_encode::files::PREVIEW)).filter(|p| p.is_file()));
    }
    let fetched = edits.join(rec).join(SOURCE);
    if fetched.is_file() && std::fs::read_to_string(edits.join(rec).join(SOURCE_KEY)).is_ok_and(|k| k.trim() == key) {
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

/// When the original goes, while the record keeps it beside a current copy (B197 (e)).
pub fn kept_until(meta: &Meta) -> Option<Wall> {
    let saved = meta.edits.as_ref().filter(|s| s.current_copy().is_some())?;
    records::iso_ms(saved.keep_until.as_deref()?).map(Wall)
}

/// How far a fetch has come, in whole percent.
pub fn percent(done: u64, total: u64) -> u8 {
    if total == 0 { 0 } else { (done.min(total) as u128 * 100 / total as u128) as u8 }
}

/// Fetches the video into `edits/<rec>/source.mp4` (whole or not at all), and remembers which
/// of the record's files it is. With `getting`, the editor hears how far it has come.
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
        if getting && pct != said && at.elapsed() >= EVERY {
            rt.post(Event::Editor(EditorNews::Getting { rec: rec.clone(), pct }));
            (said, at) = (pct, Instant::now());
        }
    };
    let cancel = || stop.is_some_and(Stop::stopped);
    let _ = std::fs::remove_file(dir.join(SOURCE_KEY));
    match s3.get_to_file(&key, &path, &mut progress, &cancel) {
        Ok(true) => {
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            let _ = write_private(&dir.join(SOURCE_KEY), key.as_bytes());
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

/// Whether a copy's picture is as long as the record says the video is (when it says).
fn as_long(meta: &Meta, seconds: f64) -> bool {
    meta.duration_sec.filter(|d| d.is_finite() && *d > 0.0).is_none_or(|d| (d - seconds).abs() <= LENGTH_SLACK)
}

fn opened(rt: &Runtime, rec: &RecId, stop: &Stop) -> Result<(), Ended> {
    let t0 = Instant::now();
    let space = space(rt)?;
    let dir = folder(rt, &rec.0)?;
    let mut door = Doorway::default();
    let meta = record(rt, &mut door, &space, &rec.0)?;
    let said = stop.unless_stopped(|| {
        OPENED.lock().unwrap().get_or_insert_with(HashMap::new).insert(rec.0.clone(), Plays::of(&meta));
        rt.post(Event::Editor(EditorNews::Opened {
            rec: rec.clone(),
            saved: meta.edits.clone(),
            markers: meta.markers.clone().unwrap_or_default(),
            duration_sec: meta.duration_sec,
            keep_until: kept_until(&meta),
        }))
    });
    if !said {
        return Err(Ended::Stopped);
    }
    // The first copy on this Mac of what the record plays whose sound reads; else the
    // space's.
    let mut read = None;
    for candidate in on_this_mac(&rt.home.takes(), &rt.home.edits(), &meta) {
        if stop.stopped() {
            return Err(Ended::Stopped);
        }
        match shroom_encode::peaks::read(&candidate) {
            Ok(p) if as_long(&meta, p.duration) => {
                read = Some((candidate, p));
                break;
            }
            Ok(p) => rt.log(&format!("editor: a copy of {} here is {:.1} s, not the record's {:?} s", short(&rec.0), p.duration, meta.duration_sec)),
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
        // The guard against a bake under the editor (another Mac's, or this one's sweep) is
        // inside the write: none can slip in between a look and it.
        records::save_edits_over(&s3, &rec.0, &saved.list, saved.rev, opened_as(&rec.0).as_ref()).map_err(|e| {
            match e {
                UploadError::MovedOn => rt.log(&format!("editor: {} was made the video from its edit while the editor was open — this edit wasn't saved over it", short(&rec.0))),
                _ => rt.log(&format!("editor: {}'s save: {e}", short(&rec.0))),
            }
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

/// The final video's progress, for the window making it (B197 (a)): the cut to 80%, the
/// files sent to 100% — at most every 250 ms. The upkeep's copies say nothing.
struct Making<'a> {
    to: Option<(&'a Runtime, &'a RecId)>,
    said: u8,
    at: Option<Instant>,
}

impl<'a> Making<'a> {
    fn window(rt: &'a Runtime, rec: &'a RecId) -> Making<'a> {
        Making { to: Some((rt, rec)), said: 0, at: None }
    }

    fn quiet() -> Making<'a> {
        Making { to: None, said: 0, at: None }
    }

    fn tell(&mut self, pct: u8) {
        let Some((rt, rec)) = self.to else { return };
        let pct = pct.min(100);
        if pct <= self.said || self.at.is_some_and(|t| t.elapsed() < EVERY) {
            return;
        }
        rt.post(Event::Editor(EditorNews::Making { rec: rec.clone(), pct }));
        (self.said, self.at) = (pct, Some(Instant::now()));
    }
}

/// The share of the final video's progress the cut is; the files sent are the rest.
const CUT_SHARE: f64 = 80.0;

/// What a copy's sent files count for, once the cut is done: the copy, its poster, its
/// teaser, then naming them.
const SENT: [u8; 3] = [95, 97, 99];

/// The picture encoders the copy may take, best first: VideoToolbox (the release's encoder
/// has only it), then x264 where a dev build's encoder has it — a virtual Mac's VideoToolbox
/// can refuse to encode.
fn encoders(ffmpeg: &Path) -> Vec<shroom_encode::cut::Encoder> {
    use shroom_encode::cut::{Encoder, has_encoder};
    std::iter::once(Encoder::VideoToolbox).chain(has_encoder(ffmpeg, Encoder::Libx264.codec()).then_some(Encoder::Libx264)).collect()
}

/// Cuts the permanent copy of `rev` and puts it up. `Ok(true)`: named in the record (with
/// when the original goes); `Ok(false)`: dropped — the record moved on, or there's nothing
/// to cut.
fn copy(rt: &Runtime, door: &mut Doorway, rec: &RecId, rev: u64, making: &mut Making) -> Result<bool, Ended> {
    let t0 = Instant::now();
    let space = space(rt)?;
    let dir = folder(rt, &rec.0)?;
    let meta = record(rt, door, &space, &rec.0)?;
    // Only the revision asked for: a newer save came first, or the edit went.
    let Some(saved) = meta.edits.as_ref().filter(|s| s.rev == rev) else {
        rt.log(&format!("editor: {}'s copy of revision {rev} dropped — the record moved on", short(&rec.0)));
        return Ok(false);
    };
    let mut list = saved.list.clone();
    if let Some(d) = meta.duration_sec.filter(|d| d.is_finite() && *d > 0.0) {
        list.source_duration = d;
    }
    let list = list.checked();
    if list.is_identity() {
        rt.log(&format!("editor: {}'s revision {rev} is the original — no copy", short(&rec.0)));
        return Ok(false);
    }
    let keep = list.keep();
    // The registered video (the window's, found or fetched as the record plays now), else
    // the same search an open makes, else the space's.
    let here = MEDIA.video(&rec.0).filter(|p| p.is_file()).into_iter().chain(on_this_mac(&rt.home.takes(), &rt.home.edits(), &meta)).next();
    let source = match here {
        Some(p) => p,
        None => fetch(rt, door, &space, &meta, &dir, None, false)?,
    };
    let ffmpeg = super::locate_ffmpeg().ok_or_else(|| {
        rt.log("editor: no encoder for the copy");
        Ended::Failed(FailClass::Unclassified)
    })?;
    let names = [shroom_edit::EditCopy::file_for(rev), shroom_edit::EditCopy::poster_for(rev), shroom_edit::EditCopy::teaser_for(rev)];
    let made = (|| -> Result<bool, Ended> {
        let out = dir.join(&names[0]);
        let mut cut = None;
        making.tell(0);
        for enc in encoders(&ffmpeg) {
            match shroom_encode::cut::render_with(&ffmpeg, &source, &keep, &out, &enc, &mut |f| making.tell((f * CUT_SHARE) as u8)) {
                Ok(duration) => {
                    cut = Some((duration, enc));
                    break;
                }
                Err(e) => rt.log(&format!("editor: {}'s copy didn't cut with {} ({e})", short(&rec.0), enc.codec())),
            }
        }
        let (duration, enc) = cut.ok_or(Ended::Failed(FailClass::Unclassified))?;
        // Its own first frame and teaser: a trimmed-off start never shows in an unfurl.
        let poster = shroom_encode::take::poster_of(&ffmpeg, &out, &dir.join(&names[1])).map_err(|e| rt.log(&format!("editor: the copy's poster: {e}"))).ok().map(|()| names[1].clone());
        let teaser = shroom_encode::take::teaser_of(&ffmpeg, &out, &dir.join(&names[2]), &enc).map_err(|e| rt.log(&format!("editor: the copy's teaser: {e}"))).ok().map(|()| names[2].clone());
        making.tell(CUT_SHARE as u8);
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
        making.tell(SENT[0]);
        if let Some(p) = &poster {
            up(p, "image/jpeg")?;
        }
        making.tell(SENT[1]);
        if let Some(t) = &teaser {
            up(t, "video/mp4")?;
        }
        making.tell(SENT[2]);
        let copy = shroom_edit::EditCopy { rev, file: names[0].clone(), duration, poster, teaser };
        let current = records::set_edit_copy(&s3, &rec.0, &copy, &now_iso()).map_err(|e| {
            rt.log(&format!("editor: {}'s copy wasn't named ({e})", short(&rec.0)));
            Ended::Failed(class(&e))
        })?;
        making.tell(100);
        rt.log(&format!(
            "editor: {}'s copy of revision {rev} {} in {} ms",
            short(&rec.0),
            if current { "is up and named, its original kept two days" } else { "was dropped — a newer save came first" },
            t0.elapsed().as_millis()
        ));
        Ok(current)
    })();
    for name in &names {
        let _ = std::fs::remove_file(dir.join(name));
    }
    made
}

/// “Yes, delete” (B197 (c)): the edit becomes the video, now. What this Mac kept of the
/// original goes with it.
fn bake(rt: &Runtime, rec: &RecId) {
    let mut door = Doorway::default();
    let baked = space(rt).and_then(|space| {
        let s3 = door.client(rt, &space, &rec.0)?;
        records::bake(&s3, &rec.0, &now_iso()).map_err(|e| {
            rt.log(&format!("editor: {}'s bake: {e}", short(&rec.0)));
            class(&e)
        })
    });
    match baked {
        Ok(true) => {
            rt.log(&format!("editor: {}'s edit is the video now — its original deleted", short(&rec.0)));
            forget_folder(rt, &rec.0);
        }
        Ok(false) => rt.log(&format!("editor: {} had nothing to bake (no current copy)", short(&rec.0))),
        Err(class) => rt.log(&format!("editor: {}'s bake didn't land ({class:?}) — the upkeep bakes it once its two days are up", short(&rec.0))),
    }
}

/// A recording's folder in `edits/` goes (its video isn't served any more either).
fn forget_folder(rt: &Runtime, rec: &str) {
    if !super::posters::valid_id(rec) {
        return;
    }
    MEDIA.forget(rec);
    let _ = std::fs::remove_dir_all(rt.home.edits().join(rec));
}

/// One upkeep pass, as the lane runs it, from the drive's own thread (the editor drive: the
/// core asks for one at launch and every six hours, a drive can't wait for that).
#[cfg(feature = "dev-harness")]
pub fn upkeep_pass(rt: &Runtime) -> Result<(u32, u32), FailClass> {
    upkeep(rt, &mut Doorway::default())
}

/// One upkeep pass (B197 (d)): `(baked, copies made)`.
fn upkeep(rt: &Runtime, door: &mut Doorway) -> Result<(u32, u32), FailClass> {
    let t0 = Instant::now();
    let space = space(rt)?;
    // The space's own list, read now (a member's videos; every one on an owner-class Mac):
    // the Library's cache hears of an edit only when the Library refreshes. Offline, the
    // pass waits for the next.
    let listed = door.listed(rt, &space).inspect_err(|_| door.forget())?;
    let read = listed.edited;
    let now = now_iso();
    let local: HashMap<String, bool> = read.iter().map(|(id, m)| (id.clone(), !on_this_mac(&rt.home.takes(), &rt.home.edits(), m).is_empty())).collect();
    let plan = records::upkeep_plan(&read, &now, &|_| true, &|id| local.get(id).copied().unwrap_or(false));
    let (mut baked, mut copies) = (0u32, 0u32);
    for id in &plan.bake {
        let done = door.client(rt, &space, id).and_then(|s3| records::bake(&s3, id, &now).map_err(|e| class(&e)));
        match done {
            Ok(true) => {
                baked += 1;
                forget_folder(rt, id);
            }
            Ok(false) => {}
            Err(c) => {
                rt.log(&format!("editor upkeep: {}'s bake didn't land ({c:?})", short(id)));
                door.forget();
            }
        }
    }
    for (id, rev) in &plan.copy {
        match copy(rt, door, &RecId(id.clone()), *rev, &mut Making::quiet()) {
            Ok(true) => copies += 1,
            Ok(false) => {}
            Err(e) => {
                rt.log(&format!("editor upkeep: {}'s copy of revision {rev} didn't land ({:?})", short(id), e.class()));
                door.forget();
            }
        }
    }
    let ids: Vec<&str> = listed.ids.iter().map(String::as_str).collect();
    let pruned = prune(rt, &ids, &mut |id| {
        let dir = rt.home.edits().join(id);
        if !dir.join(SOURCE_KEY).is_file() {
            return false;
        }
        match read.iter().find(|(r, _)| r == id).map(|(_, m)| Ok(m.clone())).unwrap_or_else(|| record(rt, door, &space, id)) {
            Ok(m) => outdated(&dir, &m),
            // Deleted, or no record at all.
            Err(FailClass::Refused) => true,
            Err(_) => false,
        }
    });
    rt.log(&format!(
        "editor upkeep: {} edited record(s) read, {baked} baked, {copies} cop{} made, {pruned} folder(s) dropped, in {} ms",
        read.len(),
        if copies == 1 { "y" } else { "ies" },
        t0.elapsed().as_millis()
    ));
    Ok((baked, copies))
}

/// The recordings' folders in `edits/` that nobody needs: a recording the space no longer
/// lists (deleted, or another space's), one unused a week, or one whose fetched video its
/// record no longer plays (`stale`: baked on another Mac, deleted) — never the one the
/// editor serves now. How many went.
fn prune(rt: &Runtime, listed: &[&str], stale: &mut dyn FnMut(&str) -> bool) -> usize {
    let Ok(entries) = std::fs::read_dir(rt.home.edits()) else { return 0 };
    let now = SystemTime::now();
    let mut gone = 0;
    for e in entries.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        if !super::posters::valid_id(&name) || MEDIA.video(&name).is_some() {
            continue;
        }
        let used = last_used(&e.path());
        if !listed.contains(&name.as_str()) || unused(used, now) || stale(&name) {
            let _ = std::fs::remove_dir_all(e.path());
            gone += 1;
        }
    }
    gone
}

/// Whether a folder's fetched video is one its record no longer plays (baked since): it would
/// never be used again.
pub fn outdated(dir: &Path, meta: &Meta) -> bool {
    std::fs::read_to_string(dir.join(SOURCE_KEY)).is_ok_and(|k| k.trim() != video_key(meta))
}

/// When anything in a folder was last written (an open rewrites its peaks).
fn last_used(dir: &Path) -> Option<SystemTime> {
    std::fs::read_dir(dir).ok()?.filter_map(Result::ok).filter_map(|e| e.metadata().ok()?.modified().ok()).max()
}

/// Unused for a week (a folder holding nothing at all counts as unused).
pub fn unused(last: Option<SystemTime>, now: SystemTime) -> bool {
    last.is_none_or(|t| now.duration_since(t).is_ok_and(|d| d >= UNUSED))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, mp4: Option<&str>, duration: Option<f64>) -> Meta {
        let mut m = Meta::uploading(id, "t", "2026-10-02T00:00:00Z");
        m.duration_sec = duration;
        m.media = Some(shroom_upload::meta::Media { hls: Some(format!("rec/{id}/master.m3u8")), mp4: mp4.map(String::from), poster: None, teaser: None, width: 1, height: 1 });
        m
    }

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
        let original = meta("AbCd", Some("rec/AbCd/video.mp4"), Some(10.0));
        let none: Vec<PathBuf> = vec![];
        assert_eq!(on_this_mac(&takes, &edits, &original), none, "nothing here");
        take("20261002-120000-0001", "take.json", "Other", true);
        take("20261002-120000-0002", "take.json", "AbCd", false);
        assert_eq!(on_this_mac(&takes, &edits, &original), none, "a take without its single-file copy");
        let pub3 = take("20261002-120000-0003", "publish.json", "AbCd", true);
        std::fs::create_dir_all(edits.join("AbCd")).unwrap();
        std::fs::write(edits.join("AbCd").join(SOURCE), b"mp4").unwrap();
        assert_eq!(on_this_mac(&takes, &edits, &original), vec![pub3.join("preview.mp4")], "a fetched file of no known key is never taken");
        std::fs::write(edits.join("AbCd").join(SOURCE_KEY), b"rec/AbCd/video.mp4").unwrap();
        assert_eq!(on_this_mac(&takes, &edits, &original), vec![pub3.join("preview.mp4"), edits.join("AbCd").join(SOURCE)], "the take's first, then the fetched one");
        assert_eq!(on_this_mac(&takes, &edits, &meta("AbCd", None, None)).len(), 2, "no single file named: the uploader's");

        // Baked: the record plays its edit's copy, which no take holds, and the original
        // fetched before isn't it.
        let baked = meta("AbCd", Some("rec/AbCd/edit-1.mp4"), Some(6.0));
        assert_eq!(on_this_mac(&takes, &edits, &baked), none);
        std::fs::write(edits.join("AbCd").join(SOURCE_KEY), b"rec/AbCd/edit-1.mp4").unwrap();
        assert_eq!(on_this_mac(&takes, &edits, &baked), vec![edits.join("AbCd").join(SOURCE)], "the baked file, once fetched");
    }

    #[test]
    fn the_space_keeps_the_video_beside_the_recording() {
        let mut m = Meta::uploading("AbCd", "t", "2026-10-02T00:00:00Z");
        assert_eq!(video_key(&m), "rec/AbCd/video.mp4", "no media: the uploader's name");
        let media = |mp4: Option<&str>| shroom_upload::meta::Media { hls: Some("rec/AbCd/master.m3u8".into()), mp4: mp4.map(String::from), poster: None, teaser: None, width: 1, height: 1 };
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
    fn the_original_is_kept_until_the_current_copys_time() {
        let mut m = meta("AbCd", Some("rec/AbCd/video.mp4"), Some(10.0));
        assert_eq!(kept_until(&m), None, "no edit");
        let mut saved = shroom_edit::Saved::new(shroom_edit::EditList::new(10.0), 2);
        saved.keep_until = Some("2026-10-20T12:00:00Z".into());
        m.edits = Some(saved.clone());
        assert_eq!(kept_until(&m), None, "no copy yet: nothing is kept for it");
        saved.copy = Some(shroom_edit::EditCopy { rev: 1, file: "edit-1.mp4".into(), duration: 5.0, poster: None, teaser: None });
        m.edits = Some(saved.clone());
        assert_eq!(kept_until(&m), None, "an older revision's copy");
        saved.copy.as_mut().unwrap().rev = 2;
        m.edits = Some(saved.clone());
        assert_eq!(kept_until(&m), Some(Wall(1_792_497_600_000)));
        saved.keep_until = Some("soon".into());
        m.edits = Some(saved);
        assert_eq!(kept_until(&m), None, "a time that doesn't read");
    }

    #[test]
    fn a_fetched_video_its_record_no_longer_plays_is_outdated() {
        let tmp = tempfile::tempdir().unwrap();
        let original = meta("AbCd", Some("rec/AbCd/video.mp4"), Some(10.0));
        let baked = meta("AbCd", Some("rec/AbCd/edit-1.mp4"), Some(6.0));
        assert!(!outdated(tmp.path(), &original), "nothing fetched: a take's peaks only");
        std::fs::write(tmp.path().join(SOURCE_KEY), b"rec/AbCd/video.mp4\n").unwrap();
        assert!(!outdated(tmp.path(), &original));
        assert!(outdated(tmp.path(), &baked), "baked on another Mac");
        std::fs::write(tmp.path().join(SOURCE_KEY), b"rec/AbCd/edit-1.mp4").unwrap();
        assert!(!outdated(tmp.path(), &baked), "the baked file, fetched since");
    }

    #[test]
    fn a_folder_unused_a_week_goes() {
        let now = SystemTime::now();
        assert!(!unused(Some(now - Duration::from_secs(6 * 24 * 3600)), now));
        assert!(unused(Some(now - Duration::from_secs(7 * 24 * 3600)), now));
        assert!(unused(None, now), "nothing in it");
        assert!(!unused(Some(now + Duration::from_secs(60)), now), "a clock that went back");
    }

    #[test]
    fn progress_is_told_forward_only_and_never_more_than_every_250_ms() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        let rec = RecId("AbCd".into());
        let mut m = Making::window(&rt, &rec);
        m.tell(0);
        m.tell(10);
        m.tell(5);
        std::thread::sleep(EVERY);
        m.tell(4);
        m.tell(40);
        let mut q = Making::quiet();
        q.tell(50);
        let told: Vec<u8> = rx.try_iter().filter_map(|e| if let Event::Editor(EditorNews::Making { pct, .. }) = e.event { Some(pct) } else { None }).collect();
        assert_eq!(told, vec![10, 40], "0 is where it starts; 5 and 4 go back; 40 after the wait; the upkeep's never");
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

    /// The lane against no space: an open fails as the core waits to hear, a save, a copy
    /// and an upkeep too; a Cancel and a Bake answer nothing, and a cancelled open's failure
    /// never lands.
    #[test]
    fn every_command_is_answered_once_and_a_cancel_or_a_bake_never() {
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
        lane.send(EditCmd::Upkeep).unwrap();
        assert!(matches!(next(), Ok(Event::Failed { op: Op::EditorUpkeep, class: FailClass::Refused })));
        lane.send(EditCmd::Cancel { rec: rec.clone() }).unwrap();
        lane.send(EditCmd::Bake { rec: rec.clone() }).unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "a Cancel and a Bake answer nothing");
    }
}
