//! The post-finalize upload of one recording (ARCHITECTURE §9.1), resumable: run it
//! again after any failure and it uploads only what's missing.

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use shroom_encode::files;
use shroom_encode::playlist::{maps, take_segments};
use shroom_storage::s3::{Client, S3Error};

use crate::keys;
use crate::master::{Variant, avc_codec, has_aac, peak_bandwidth, render};
use crate::meta::{Media, Meta, Progress, State, now_iso};

/// R2's single PUT tops out at 4.995 GiB (R11); stay well under, and keep memory
/// bounded: one PUT up to 256 MiB, 128 MiB parts above (ARCHITECTURE §9.2).
pub const SINGLE_PUT_MAX: u64 = 256 * 1024 * 1024;
pub const PART_SIZE: u64 = 128 * 1024 * 1024;

#[derive(Debug)]
pub enum UploadError {
    /// The recording was deleted (its meta is a tombstone) — stop, upload nothing more.
    Tombstoned,
    /// The id belongs to another recording (a create-only claim lost to a stranger).
    Collision,
    S3(S3Error),
    Io(std::io::Error),
    /// The local recording isn't complete (no playlist, no init, a missing segment).
    Incomplete(String),
    /// The record no longer plays what the editor opened on (an edit became the video since,
    /// B197): a list made on the old clock is never saved over it — nothing was written.
    MovedOn,
}

impl From<S3Error> for UploadError {
    fn from(e: S3Error) -> Self {
        UploadError::S3(e)
    }
}

impl From<std::io::Error> for UploadError {
    fn from(e: std::io::Error) -> Self {
        UploadError::Io(e)
    }
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadError::Tombstoned => write!(f, "the recording was deleted"),
            UploadError::Collision => write!(f, "the id is taken by another recording"),
            UploadError::S3(e) => write!(f, "{e}"),
            UploadError::Io(e) => write!(f, "io: {e}"),
            UploadError::Incomplete(why) => write!(f, "incomplete recording: {why}"),
            UploadError::MovedOn => write!(f, "the record no longer plays what the editor opened on"),
        }
    }
}

impl std::error::Error for UploadError {}

/// Retries an idempotent call with exponential backoff (POC: 6 tries, 500 ms base,
/// 15 s cap). Status errors that can't improve (4xx other than 408/429) aren't retried, and
/// neither is a transfer its caller stopped.
pub fn with_retry<T>(mut f: impl FnMut() -> Result<T, S3Error>) -> Result<T, S3Error> {
    let mut delay = Duration::from_millis(500);
    let mut last = None;
    for _ in 0..6 {
        match f() {
            Ok(v) => return Ok(v),
            Err(S3Error::Status { status, body }) if (400..500).contains(&status) && status != 408 && status != 429 => {
                return Err(S3Error::Status { status, body });
            }
            Err(S3Error::Cancelled) => return Err(S3Error::Cancelled),
            Err(e) => last = Some(e),
        }
        std::thread::sleep(delay);
        delay = (delay * 2).min(Duration::from_secs(15));
    }
    Err(last.expect("at least one attempt"))
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub id: String,
    /// From the call until the stream was live (playlists + meta), in milliseconds.
    pub live_ms: u64,
    /// From the call until the single-file copy was up too.
    pub done_ms: u64,
    pub uploaded: u32,
    pub already_there: u32,
    pub bytes: u64,
    pub duration_sec: f64,
    pub mp4: bool,
}

fn content_type(file: &str) -> &'static str {
    match file.rsplit('.').next() {
        Some("m3u8") => "application/vnd.apple.mpegurl",
        Some("m4s") => "video/iso.segment",
        Some("mp4") => "video/mp4",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

fn read_meta(s3: &Client, id: &str) -> Result<Option<Meta>, UploadError> {
    match with_retry(|| s3.get(&keys::meta(id)))? {
        None => Ok(None),
        Some(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}"))),
    }
}

/// Read-modify-write of the record under optimistic concurrency: the write carries the
/// ETag that was read (`If-Match`) and retries from a fresh read if anyone — another of
/// this Mac's writers, or the Worker renaming the video — wrote in between. R2 honors it
/// (verified live 2026-10-02). Stops with `Tombstoned` the moment the record says deleted.
pub fn update_meta(s3: &Client, id: &str, mut change: impl FnMut(&mut Meta)) -> Result<Meta, UploadError> {
    try_update_meta(s3, id, |m| {
        change(m);
        Ok(())
    })
}

/// [`update_meta`] whose change may refuse: an `Err` from `change`, judged on the very record
/// the write would replace (each read, again after a lost race), ends it with that error and
/// the record as it was.
pub(crate) fn try_update_meta(s3: &Client, id: &str, mut change: impl FnMut(&mut Meta) -> Result<(), UploadError>) -> Result<Meta, UploadError> {
    for _ in 0..8 {
        let (bytes, etag) = with_retry(|| s3.get_with_etag(&keys::meta(id)))?.ok_or(UploadError::Collision)?;
        let mut meta: Meta = serde_json::from_slice(&bytes).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}")))?;
        if meta.state == State::Deleted {
            return Err(UploadError::Tombstoned);
        }
        change(&mut meta)?;
        let body = serde_json::to_vec(&meta).expect("meta serializes");
        if with_retry(|| s3.put_if_match(&keys::meta(id), &body, "application/json", &etag))? {
            return Ok(meta);
        }
    }
    Err(UploadError::Incomplete("the record kept changing under us".into()))
}

/// Uploads the finalized recording in `dir` as `id`. `progress(done, total)` is called
/// after each file. Safe to call again after any failure.
pub fn upload_recording(
    s3: &Client,
    id: &str,
    dir: &Path,
    title: &str,
    progress: impl FnMut(u32, u32),
) -> Result<Report, UploadError> {
    upload_recording_after(s3, id, dir, title, &std::collections::HashMap::new(), progress)
}

/// What the page reads while bytes move (ARCHITECTURE §9.3, DECISIONS B49): the record's
/// `progress` is rewritten (read-modify-write under `If-Match`, B21) at most every
/// `every` while the count moves, and at least every `keepalive` while a long file is in
/// flight — so the page's stall clock (`STALL_SECONDS`, 120 s) only runs out when nothing
/// gets through. A beat that can't write (offline, a lost race) is simply skipped: the
/// next one says it again.
#[derive(Debug, Clone, Copy)]
pub struct Beat {
    pub every: Duration,
    pub keepalive: Duration,
}

impl Default for Beat {
    fn default() -> Self {
        Beat { every: Duration::from_secs(5), keepalive: Duration::from_secs(30) }
    }
}

/// The recording as it sits on this Mac: the joined playlist, its segments and the init.
pub struct Local {
    playlist: String,
    segments: Vec<shroom_encode::playlist::Segment>,
    init: Vec<u8>,
    /// The init segment, then every segment, in order.
    media: Vec<String>,
}

impl Local {
    pub fn read(dir: &Path) -> Result<Local, UploadError> {
        let playlist = std::fs::read_to_string(dir.join(files::PLAYLIST))
            .map_err(|_| UploadError::Incomplete("no joined playlist — finalize first".into()))?;
        let segments = take_segments(&playlist);
        if segments.is_empty() {
            return Err(UploadError::Incomplete("the playlist names no segments".into()));
        }
        // Every init segment the playlist names: the shared one, and a take's own when its
        // picture differed (`shroom_encode::files::take_init`).
        let mut inits = maps(&playlist);
        if inits.is_empty() {
            inits.push(files::INIT_SEGMENT.to_string());
        }
        for name in &inits {
            if !dir.join(name).metadata().map(|m| m.len() > 0).unwrap_or(false) {
                return Err(UploadError::Incomplete(format!("no init segment {name}")));
            }
        }
        let init = std::fs::read(dir.join(&inits[0])).map_err(|_| UploadError::Incomplete("no init segment".into()))?;
        let mut media: Vec<String> = inits;
        media.extend(segments.iter().map(|s| s.file.clone()));
        Ok(Local { playlist, segments, init, media })
    }

    /// The files the stream needs before it can go live: the media, the variant playlist
    /// and the master.
    pub fn stream_files(&self) -> u32 {
        self.media.len() as u32 + 2
    }
}

/// The markers dropped while recording (`markers.json`, written at each drop), as seconds
/// in the joined timeline.
pub fn local_markers(dir: &Path) -> Vec<f64> {
    #[derive(serde::Deserialize)]
    struct Markers {
        seconds: Vec<f64>,
    }
    std::fs::read(dir.join(MARKERS_FILE)).ok().and_then(|b| serde_json::from_slice::<Markers>(&b).ok()).map(|m| m.seconds).unwrap_or_default()
}

/// The take's marker file, beside its segments.
pub const MARKERS_FILE: &str = "markers.json";

/// Beside a take that crash repair re-cut from its fragmented copy: what went up while it
/// recorded is no longer its bytes, so its go-live sends every file again.
pub const REBUILT_FILE: &str = "rebuilt";

/// Step 1, the create-only claim — the pass's first network act for a newly published take
/// (DECISIONS B49), so the owner page finds its record (the title field, Copy link) the
/// moment it opens. It carries the title, the markers and the first `progress` (nothing
/// up yet, of `total`), which starts the page's stall clock. A resume finds its own record
/// (or a tombstone).
pub fn claim(s3: &Client, id: &str, title: &str, markers: &[f64], total: u32) -> Result<(), UploadError> {
    let mut fresh = Meta::uploading(id, title, &now_iso());
    fresh.progress = Some(Progress { done: 0, total, updated_at: now_iso() });
    if !markers.is_empty() {
        fresh.markers = Some(markers.to_vec());
    }
    let claimed = with_retry(|| s3.put_if_absent(&keys::meta(id), &serde_json::to_vec(&fresh).unwrap(), "application/json"))?;
    if !claimed {
        let existing = read_meta(s3, id)?.ok_or(UploadError::Collision)?;
        if existing.state == State::Deleted {
            return Err(UploadError::Tombstoned);
        }
        if existing.id != id {
            return Err(UploadError::Collision);
        }
    }
    Ok(())
}

/// The count the beat writes from: files confirmed of all the files this pass will send.
struct Counter {
    done: AtomicU32,
    total: u32,
}

/// One beat: the record's `progress`, rewritten once under `If-Match` — never retried, never
/// over a tombstone (a lost race or a dropped connection waits for the next beat).
fn beat_once(s3: &Client, id: &str, done: u32, total: u32) {
    let Ok(Some((bytes, etag))) = s3.get_with_etag(&keys::meta(id)) else { return };
    let Ok(mut meta) = serde_json::from_slice::<Meta>(&bytes) else { return };
    if meta.state == State::Deleted {
        return;
    }
    meta.progress = Some(Progress { done, total, updated_at: now_iso() });
    let _ = s3.put_if_match(&keys::meta(id), &serde_json::to_vec(&meta).expect("meta serializes"), "application/json", &etag);
}

/// Runs `work` with the beat alongside it; the beat has stopped when this returns, so a
/// write that must be the last word (`done = total`) comes after it.
fn with_beat<T>(s3: &Client, id: &str, beat: &Beat, counter: &Counter, work: impl FnOnce() -> Result<T, UploadError>) -> Result<T, UploadError> {
    let stop = (Mutex::new(false), Condvar::new());
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut last_done = counter.done.load(Ordering::SeqCst);
            let mut last_write = Instant::now();
            loop {
                let stopped = stop.0.lock().unwrap();
                let (stopped, _) = stop.1.wait_timeout_while(stopped, beat.every, |s| !*s).unwrap();
                if *stopped {
                    return;
                }
                drop(stopped);
                let done = counter.done.load(Ordering::SeqCst);
                if done != last_done || last_write.elapsed() >= beat.keepalive {
                    beat_once(s3, id, done, counter.total);
                    last_done = done;
                    last_write = Instant::now();
                }
            }
        });
        let out = work();
        *stop.0.lock().unwrap() = true;
        stop.1.notify_all();
        out
    })
}

/// Steps 2–5, the stream: media in order, the variant playlist last of it (the go-live — every
/// byte it names is confirmed by now), the master, the poster with it, then the record
/// flips to live. `extras` is how many second-class files (the teaser, the single-file
/// copy) [`finish`] will still send: they count in `progress.total`, so the record reads
/// drained — the page's “Fully live ✓” — only once they're up too (r4 `fully-live`: the
/// queue drained). `progress(done, total)` is called after each file. Over a video whose edit
/// already became it (B197's bake) it sends nothing ([`baked_now`]): the record says live
/// and the count moves on.
#[allow(clippy::too_many_arguments)]
pub fn go_live(
    s3: &Client,
    id: &str,
    dir: &Path,
    local: &Local,
    already: &std::collections::HashMap<String, u64>,
    extras: u32,
    beat: &Beat,
    mut progress: impl FnMut(u32, u32),
) -> Result<Report, UploadError> {
    let called = Instant::now();
    let stream = local.stream_files();
    let total = stream + extras;
    let duration: f64 = local.segments.iter().map(|s| s.seconds).sum();
    if baked_now(s3, id)? {
        progress(stream, total);
        update_meta(s3, id, |m| {
            m.state = State::Live;
            m.published_at.get_or_insert_with(now_iso);
            m.progress = Some(Progress { done: stream, total, updated_at: now_iso() });
        })?;
        let ms = called.elapsed().as_millis() as u64;
        return Ok(Report { id: id.to_string(), live_ms: ms, done_ms: ms, uploaded: 0, already_there: 0, bytes: 0, duration_sec: duration, mp4: false });
    }
    // What's already up (a resume uploads only the gap). Prefix-scoped credentials may
    // not list; then every file is simply PUT again (idempotent). A take rebuilt after a
    // crash ([`REBUILT_FILE`]) trusts nothing that went up before: its segments were
    // re-cut from the fragmented copy, so a file there under the same name and size can
    // hold other bytes — every file is PUT again (law 2: the bucket's copy is the local
    // file, byte for byte; DECISIONS B70).
    let rebuilt = dir.join(REBUILT_FILE).exists();
    let mut present: std::collections::HashMap<String, u64> = if rebuilt {
        Default::default()
    } else {
        s3.list(&keys::prefix(id)).map(|objs| objs.into_iter().map(|o| (o.key, o.size)).collect()).unwrap_or_default()
    };
    if !rebuilt {
        present.extend(already.iter().map(|(k, v)| (k.clone(), *v)));
    }
    let counter = Counter { done: AtomicU32::new(0), total };
    let mut report = Report { id: id.to_string(), live_ms: 0, done_ms: 0, uploaded: 0, already_there: 0, bytes: 0, duration_sec: 0.0, mp4: false };
    with_beat(s3, id, beat, &counter, || {
        let mut sizes = Vec::new();
        for (i, file) in local.media.iter().enumerate() {
            let local_file = dir.join(file);
            let size = std::fs::metadata(&local_file).map_err(|_| UploadError::Incomplete(format!("{file} is missing")))?.len();
            let key = keys::variant_file(id, file);
            if present.get(&key) == Some(&size) {
                report.already_there += 1;
            } else {
                let bytes = std::fs::read(&local_file)?;
                with_retry(|| s3.put(&key, &bytes, content_type(file)))?;
                report.uploaded += 1;
                report.bytes += size;
            }
            if i > 0 {
                sizes.push(size);
            }
            counter.done.store(i as u32 + 1, Ordering::SeqCst);
            progress(i as u32 + 1, total);
        }

        with_retry(|| s3.put(&keys::variant_file(id, files::PLAYLIST), local.playlist.as_bytes(), content_type(files::PLAYLIST)))?;
        counter.done.store(stream - 1, Ordering::SeqCst);
        progress(stream - 1, total);
        let (width, height) = probe_size(&local.init).unwrap_or((0, 0));
        let mut codecs = avc_codec(&local.init).unwrap_or_else(|| "avc1.640028".into());
        if has_aac(&local.init) {
            codecs.push_str(",mp4a.40.2");
        }
        let per_segment: Vec<(u64, f64)> = sizes.iter().zip(&local.segments).map(|(b, s)| (*b, s.seconds)).collect();
        let master = render(&[Variant {
            bandwidth: peak_bandwidth(&per_segment),
            width,
            height,
            codecs,
            uri: format!("{}/{}", keys::VARIANT, files::PLAYLIST),
        }]);
        with_retry(|| s3.put(&keys::master(id), master.as_bytes(), content_type("x.m3u8")))?;
        counter.done.store(stream, Ordering::SeqCst);
        progress(stream, total);

        // The poster rides with the go-live, so the first unfurl already has its image.
        let poster_file = dir.join(files::POSTER);
        let poster = if poster_file.is_file() {
            let bytes = std::fs::read(&poster_file)?;
            with_retry(|| s3.put(&keys::poster(id), &bytes, "image/jpeg"))?;
            Some(keys::poster(id))
        } else {
            None
        };

        // 5. The record flips to live (read-modify-write; stops if it was deleted meanwhile).
        // A video whose edit became it while this pass ran (B197) is never pointed back at
        // the stream, nor its length at the recording's.
        update_meta(s3, id, |m| {
            m.state = State::Live;
            m.published_at.get_or_insert_with(now_iso);
            if !baked(m) {
                m.duration_sec = Some((duration * 1000.0).round() / 1000.0);
                m.media = Some(Media { hls: Some(keys::master(id)), mp4: None, poster: poster.clone(), teaser: None, width, height });
            }
            m.progress = Some(Progress { done: stream, total, updated_at: now_iso() });
        })?;
        report.duration_sec = duration;
        Ok(())
    })?;
    report.live_ms = called.elapsed().as_millis() as u64;
    report.done_ms = report.live_ms;
    Ok(report)
}

/// Whether the record's video is one file and no stream any more: an edit became it (B197's
/// bake, `media.hls` null). The uploader never moves such a record's `media`.
fn baked(m: &Meta) -> bool {
    m.media.as_ref().is_some_and(|x| x.hls.is_none())
}

/// Whether the record is [`baked`], read before a pass sends anything: run again over such a
/// video, the pass sends none of the original's files — the bake deleted them, and nothing
/// would ever delete them again. A tombstone stops the pass here.
fn baked_now(s3: &Client, id: &str) -> Result<bool, UploadError> {
    match read_meta(s3, id)? {
        Some(m) if m.state == State::Deleted => Err(UploadError::Tombstoned),
        Some(m) => Ok(baked(&m)),
        None => Ok(false),
    }
}

/// Names `key` in the record's `media` (as `set` does) unless the video is baked; then the
/// file just sent is the original's, which the bake deletes, and it goes here too (never a
/// file `media` names).
fn name_extra(s3: &Client, id: &str, key: &str, set: impl Fn(&mut Media)) -> Result<(), UploadError> {
    let m = update_meta(s3, id, |m| {
        if let Some(media) = m.media.as_mut().filter(|x| x.hls.is_some()) {
            set(media);
        }
    })?;
    let named = m.media.as_ref().is_some_and(|x| [&x.mp4, &x.poster, &x.teaser].iter().any(|k| k.as_deref() == Some(key)));
    if baked(&m) && !named {
        with_retry(|| s3.delete(key))?;
    }
    Ok(())
}

/// Step 6, the second-class files, after the stream is live and never gating it: the unfurl
/// teaser (small), then the single-file copy — each only if it's on this Mac. Then the
/// record says the upload drained (`progress.done = total`), the page's “Fully live ✓”,
/// whether or not every extra could be made. `done`/`total` continue [`go_live`]'s count.
/// Over a video whose edit already became it (B197's bake, [`baked_now`]) it sends nothing
/// and says drained; one whose edit becomes it while this runs keeps its `media`, and what
/// this sent is removed again. Returns whether the single-file copy went up.
pub fn finish(s3: &Client, id: &str, dir: &Path, done: u32, total: u32, beat: &Beat, mut progress: impl FnMut(u32, u32)) -> Result<bool, UploadError> {
    if baked_now(s3, id)? {
        update_meta(s3, id, |m| m.progress = Some(Progress { done: total, total, updated_at: now_iso() }))?;
        progress(total, total);
        return Ok(false);
    }
    let counter = Counter { done: AtomicU32::new(done), total };
    let mp4 = with_beat(s3, id, beat, &counter, || {
        let teaser = dir.join(files::TEASER);
        if teaser.is_file() {
            let bytes = std::fs::read(&teaser)?;
            with_retry(|| s3.put(&keys::teaser(id), &bytes, "video/mp4"))?;
            name_extra(s3, id, &keys::teaser(id), |media| media.teaser = Some(keys::teaser(id)))?;
            let d = counter.done.fetch_add(1, Ordering::SeqCst) + 1;
            progress(d.min(total), total);
        }
        let mp4 = dir.join(files::PREVIEW);
        if mp4.is_file() {
            with_retry(|| s3.put_file(&keys::mp4(id), &mp4, "video/mp4", SINGLE_PUT_MAX, PART_SIZE))?;
            name_extra(s3, id, &keys::mp4(id), |media| media.mp4 = Some(keys::mp4(id)))?;
            let d = counter.done.fetch_add(1, Ordering::SeqCst) + 1;
            progress(d.min(total), total);
            return Ok(true);
        }
        Ok(false)
    })?;
    // The last word, after the beat stopped: drained — and what the video holds in the
    // bucket, read once, for the suggestions to clean up (N68). A listing that fails leaves
    // the size out: the Worker estimates it from the length.
    let stored: Option<u64> = s3.list(&keys::prefix(id)).ok().map(|objs| objs.iter().map(|o| o.size).sum());
    update_meta(s3, id, |m| {
        m.progress = Some(Progress { done: total, total, updated_at: now_iso() });
        if let Some(b) = stored {
            m.extra.insert("bytes".into(), serde_json::Value::from(b));
        }
    })?;
    progress(total, total);
    Ok(mp4)
}

/// [`upload_recording`], skipping files a [`crate::live::LiveUpload`] already confirmed:
/// [`claim`] → [`go_live`] → [`finish`] with the extras already on this Mac.
pub fn upload_recording_after(
    s3: &Client,
    id: &str,
    dir: &Path,
    title: &str,
    already: &std::collections::HashMap<String, u64>,
    mut progress: impl FnMut(u32, u32),
) -> Result<Report, UploadError> {
    let called = Instant::now();
    let local = Local::read(dir)?;
    let extras = [files::TEASER, files::PREVIEW].iter().filter(|f| dir.join(f).is_file()).count() as u32;
    let total = local.stream_files() + extras;
    claim(s3, id, title, &local_markers(dir), total)?;
    let beat = Beat::default();
    let mut report = go_live(s3, id, dir, &local, already, extras, &beat, &mut progress)?;
    report.mp4 = finish(s3, id, dir, local.stream_files(), total, &beat, &mut progress)?;
    report.done_ms = called.elapsed().as_millis() as u64;
    Ok(report)
}

/// Width and height from the init segment's track header (`tkhd`, 16.16 fixed point).
pub fn probe_size(init: &[u8]) -> Option<(u32, u32)> {
    let at = init.windows(4).position(|w| w == b"tkhd")?;
    let version = *init.get(at + 4)?;
    // tkhd body after the 4-byte type: version+flags (4), then v0: 20 bytes / v1: 32 bytes
    // of times/ids, 8 reserved, layer/alt group/volume/reserved (8), matrix (36), w, h.
    let fixed = at + 4 + 4 + if version == 1 { 32 } else { 20 } + 8 + 8 + 36;
    let w = u32::from_be_bytes(init.get(fixed..fixed + 4)?.try_into().ok()?) >> 16;
    let h = u32::from_be_bytes(init.get(fixed + 4..fixed + 8)?.try_into().ok()?) >> 16;
    (w > 0 && h > 0).then_some((w, h))
}
