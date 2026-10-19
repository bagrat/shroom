//! The Library's data path (ARCHITECTURE §13, N35): the space's records listed with
//! `ListObjectsV2 prefix=meta/` and an ETag delta — only records that changed are read
//! again — plus the three writes a token-holding Mac makes from a row: rename (a record
//! rewrite), delete (a tombstone **first**, then the bytes — §5.5), and the clean delete
//! of a take that was never shared (no tombstone) — and the editor's three (N63, B197):
//! saving an edit, naming its permanent copy, and making that copy the video (the bake).
//! Every rewrite is read-modify-write under `If-Match` (B21).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shroom_storage::s3::{Client, Object};

use crate::keys;
use crate::meta::{Media, Meta, State, iso_at, now_iso};
use crate::upload::{UploadError, try_update_meta, update_meta, with_retry};

/// What a Library row needs from a record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub state: State,
    pub created_at: String,
    pub duration_sec: Option<f64>,
    /// The member who made it (N5 scoping: members see their own) — from the space's
    /// ownership ledger only, filled by the caller; never the record's own `owner`, which a
    /// member can rewrite on their records (WORKER-INTERFACE §4.10, H3).
    pub owner: Option<String>,
    /// The poster's key (`media.poster`).
    pub poster: Option<String>,
    /// How long it plays edited, when the record holds an edit that leaves something out
    /// ([`Meta::edited_sec`]): the row shows it in place of the length (N63).
    #[serde(default)]
    pub edited_sec: Option<f64>,
    /// The poster's file name beside the recording (`poster.jpg`, or a baked edit's
    /// `edit-3-poster.jpg`): what a Mac without the token fetches as `/m/<id>/<name>`.
    /// `None` when the record names no poster, or one that isn't beside it.
    #[serde(default)]
    pub poster_file: Option<String>,
    /// What the space's AI did with it, in one line for the Mac's log (`meta::ai_note`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
}

impl Row {
    /// A record's row (the ledger's owner is the caller's to fill).
    pub fn of(m: &Meta) -> Row {
        Row {
            id: m.id.clone(),
            title: m.title.clone(),
            state: m.state,
            created_at: m.created_at.clone(),
            duration_sec: m.duration_sec,
            // Member-writable: never trusted for ownership (§4.10).
            owner: None,
            poster: m.media.as_ref().and_then(|x| x.poster.clone()),
            edited_sec: m.edited_sec(),
            poster_file: poster_file(m),
            ai: crate::meta::ai_note(m),
        }
    }
}

/// The name under `rec/<id>/` of the record's poster: one plain file name, never a path. An
/// edit's current copy has its own (the page shows it, and the original's may be one the
/// edit trimmed off and the page no longer serves); else the media's.
pub fn poster_file(m: &Meta) -> Option<String> {
    let plain = |name: &str| (!name.is_empty() && !name.starts_with('.') && !name.contains('/')).then(|| name.to_string());
    if let Some(name) = m.edits.as_ref().and_then(|s| s.current_copy()).and_then(|c| c.poster.as_deref()).and_then(plain) {
        return Some(name);
    }
    plain(m.media.as_ref()?.poster.as_deref()?.strip_prefix(&keys::prefix(&m.id))?)
}

/// The local cache the Library opens on at once (kept by the caller, e.g. a file).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    /// id → (the record's ETag when read, its row).
    pub rows: BTreeMap<String, (String, Row)>,
}

impl Cache {
    /// The rows a person sees: everything but tombstones, newest first.
    pub fn visible(&self) -> Vec<&Row> {
        let mut v: Vec<&Row> = self.rows.values().map(|(_, r)| r).filter(|r| r.state != State::Deleted).collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }
}

/// What one refresh changed.
#[derive(Debug, Default, PartialEq)]
pub struct Delta {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub gone: Vec<String>,
    /// Records that were read (the cost of the refresh).
    pub reads: usize,
}

/// Lists `meta/` and the deletion records, reads only the records whose ETag changed, and
/// drops ids that no longer exist. A sealed id is deleted whatever its record says now
/// (DECISIONS B125) and costs no read. A record that can't be parsed is skipped, never fatal.
pub fn refresh(s3: &Client, cache: &mut Cache) -> Result<Delta, UploadError> {
    let listed = with_retry(|| s3.list("meta/"))?;
    let sealed: std::collections::BTreeSet<String> =
        with_retry(|| s3.list(keys::GONE))?.into_iter().filter_map(|o| o.key.strip_prefix(keys::GONE).map(str::to_string)).collect();
    let mut delta = Delta::default();
    let mut seen = std::collections::BTreeSet::new();
    for o in listed {
        let Some(id) = o.key.strip_prefix("meta/").and_then(|k| k.strip_suffix(".json")) else { continue };
        seen.insert(id.to_string());
        let etag = o.etag.clone().unwrap_or_default();
        if sealed.contains(id) {
            let before = cache.rows.get(id).map(|(_, r)| r.clone());
            if before.as_ref().is_some_and(|r| r.state == State::Deleted) {
                continue;
            }
            let mut row = before.unwrap_or_else(|| Row::of(&Meta::uploading(id, "", "")));
            row.state = State::Deleted;
            let fresh = !cache.rows.contains_key(id);
            cache.rows.insert(id.to_string(), (etag, row));
            if fresh { delta.added.push(id.to_string()) } else { delta.changed.push(id.to_string()) }
            continue;
        }
        if cache.rows.get(id).is_some_and(|(e, _)| !etag.is_empty() && *e == etag) {
            continue;
        }
        delta.reads += 1;
        let Some((bytes, got)) = with_retry(|| s3.get_with_etag(&o.key))? else { continue };
        let Some(meta) = serde_json::from_slice(&bytes).ok().and_then(crate::meta::listed) else { continue };
        let fresh = !cache.rows.contains_key(id);
        cache.rows.insert(id.to_string(), (got, Row::of(&meta)));
        if fresh { delta.added.push(id.to_string()) } else { delta.changed.push(id.to_string()) }
    }
    let gone: Vec<String> = cache.rows.keys().filter(|id| !seen.contains(*id)).cloned().collect();
    for id in gone {
        cache.rows.remove(&id);
        delta.gone.push(id);
    }
    Ok(delta)
}

/// A recording's record as it is now (`meta/<id>.json`), `None` when there is none: what
/// `shroom edit` reads before it changes an edit (N70).
pub fn record(s3: &Client, id: &str) -> Result<Option<Meta>, UploadError> {
    crate::upload::read_meta(s3, id)
}

/// Renames a recording — live worldwide on the next page load. An emptied title is
/// refused (the row restores the previous one, N35); a tombstone stops it.
pub fn rename(s3: &Client, id: &str, title: &str) -> Result<Meta, UploadError> {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return Err(UploadError::Incomplete("an empty title is never saved".into()));
    }
    update_meta(s3, id, |m| {
        m.title = title.clone();
        m.extra.insert("titled".into(), serde_json::Value::Bool(true));
    })
}

/// Whether `name` is one of a permanent copy's files (`edit-<rev>.mp4`, `edit-<rev>-poster.jpg`,
/// `edit-<rev>-teaser.mp4`) — the only names beside a recording an edit's write ever
/// deletes, whatever a record says (the Worker serves only these, `^edit-\d{1,9}…$`).
pub fn is_copy_file(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("edit-") else { return false };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (1..=9).contains(&digits) && matches!(&rest[digits..], ".mp4" | "-poster.jpg" | "-teaser.mp4")
}

/// Removes these files from beside the recording, skipping any name that isn't a copy's and
/// any the record's `media` names (a baked video plays a copy's files: a late or repeated
/// write about that copy must never take them). Best effort: the record no longer names
/// them, so one left behind is only bytes.
fn delete_copy_files(s3: &Client, id: &str, names: &[String], record: &Meta) {
    let playing = media_keys(record);
    for name in names.iter().filter(|n| is_copy_file(n)) {
        let key = format!("{}{name}", keys::prefix(id));
        if !playing.contains(&key) {
            let _ = with_retry(|| s3.delete(&key));
        }
    }
}

/// Every key the record's `media` names: the files the link plays and shows.
fn media_keys(m: &Meta) -> Vec<String> {
    m.media.iter().flat_map(|x| [&x.hls, &x.mp4, &x.poster, &x.teaser]).flatten().cloned().collect()
}

/// [`update_meta`] for an edit's write: a record that turned into a tombstone (which keeps no
/// title, so it never reads as a whole record) answers `Tombstoned`, not “unreadable”.
fn update_edit(s3: &Client, id: &str, mut change: impl FnMut(&mut Meta)) -> Result<Meta, UploadError> {
    try_update_edit(s3, id, |m| {
        change(m);
        Ok(())
    })
}

/// [`update_edit`] whose change may refuse, as [`try_update_meta`]'s does.
fn try_update_edit(s3: &Client, id: &str, change: impl FnMut(&mut Meta) -> Result<(), UploadError>) -> Result<Meta, UploadError> {
    try_update_meta(s3, id, change).map_err(|e| match e {
        UploadError::Incomplete(_) if tombstoned(s3, id).unwrap_or(false) => UploadError::Tombstoned,
        e => e,
    })
}

/// What a record plays: which file, and how many bakes it has been through (B197). An editor
/// remembers it as it opens; once the record plays something else — an edit became the video
/// since, on this Mac or another — a list the editor made is on a clock the record no longer
/// has. A rename, a voice plan or a copy landing moves nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plays {
    /// `media.mp4`: the original's single-file copy, or a baked edit's own.
    pub mp4: Option<String>,
    /// How many bakes `bakedFrom` remembers.
    pub bakes: usize,
}

impl Plays {
    pub fn of(m: &Meta) -> Plays {
        let bakes = m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).map_or(0, Vec::len);
        Plays { mp4: m.media.as_ref().and_then(|x| x.mp4.clone()), bakes }
    }
}

/// The revision an edit's record holds: its `edits.rev`, also when this version can't read
/// the rest of that `edits` (kept as it was in `extra`) — and never below a revision a bake
/// made the video (`bakedFrom`), so an edit of a baked video never reuses the file names the
/// video now plays from.
fn record_rev(m: &Meta) -> u64 {
    let edits = m.edits.as_ref().map(|s| s.rev).or_else(|| m.extra.get("edits").and_then(|e| e.get("rev")).and_then(serde_json::Value::as_u64)).unwrap_or(0);
    let baked = m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|b| b.get("rev")?.as_u64()).max().unwrap_or(0);
    edits.max(baked)
}

/// Saves an edit (N63's Done; DECISIONS B195): the record's `edits` becomes `list` as the
/// next revision — `max(the record's, asked_rev - 1) + 1`, so a save from an editor that
/// opened on an older revision still lands after whatever another Mac saved since (the
/// last save wins). A voice plan that landed while the editor was open is kept
/// ([`with_voice_plan`]). Whatever else the record's `edits` holds is kept: `keep_until`,
/// and fields this build doesn't know. A copy cut from an earlier revision stays named, now
/// stale: readers ignore it until [`set_edit_copy`] names the new one. When the list saved
/// is the identity (nothing cut, the whole recording) there's no copy to make: the old copy
/// and `keep_until` are dropped from the record and then the copy's files go.
/// Read-modify-write under `If-Match`; a tombstone stops it. Returns the revision saved.
///
/// Unguarded: whatever the record plays now, the list lands on it — for a writer that opened
/// no editor on it. An editor's save is [`save_edits_over`].
pub fn save_edits(s3: &Client, id: &str, list: &shroom_edit::EditList, asked_rev: u64) -> Result<u64, UploadError> {
    save_edits_over(s3, id, list, asked_rev, None)
}

/// [`save_edits`] from an editor that opened on the record when it played `opened`: judged
/// inside the read-modify-write, on the very record the save would replace, a record that
/// plays anything else now ([`Plays`] — an edit became the video since) is never written:
/// `MovedOn`, nothing changed, no file deleted. `None` saves unguarded.
pub fn save_edits_over(s3: &Client, id: &str, list: &shroom_edit::EditList, asked_rev: u64, opened: Option<&Plays>) -> Result<u64, UploadError> {
    let mut rev = 0;
    let mut dropped: Vec<String> = Vec::new();
    let meta = try_update_edit(s3, id, |m| {
        if opened.is_some_and(|o| *o != Plays::of(m)) {
            return Err(UploadError::MovedOn);
        }
        rev = record_rev(m).max(asked_rev.saturating_sub(1)) + 1;
        let before = m.edits.take();
        let list = with_voice_plan(list, before.as_ref().map(|s| &s.list)).checked();
        let identity = list.is_identity();
        let (old, keep_until, extra) = match before {
            Some(s) => (s.copy, s.keep_until, s.extra),
            None => (None, None, serde_json::Map::new()),
        };
        dropped.clear();
        let (copy, keep_until) = if identity {
            dropped.extend(old.iter().flat_map(|c| c.files().into_iter().map(str::to_string)));
            (None, None)
        } else {
            (old, keep_until)
        };
        m.extra.remove("edits");
        m.edits = Some(shroom_edit::Saved { copy, keep_until, extra, ..shroom_edit::Saved::new(list, rev) });
        Ok(())
    })?;
    delete_copy_files(s3, id, &dropped, &meta);
    Ok(rev)
}

/// The list an editor saves, merged with the record's by cut id (agreed with the voice-cut
/// build): when the record holds a voice plan and the list none — the editor opened before
/// the plan landed — the list takes the record's plan and every planned cut it doesn't
/// have (a cut with an id, not drawn by hand). Otherwise the list wins as it is: a hand
/// cut put back in the editor stays out, and so does a suggestion the editor saw and left.
fn with_voice_plan(list: &shroom_edit::EditList, record: Option<&shroom_edit::EditList>) -> shroom_edit::EditList {
    let mut list = list.clone();
    let Some(record) = record else { return list };
    if let (Some(voice), None) = (record.voice, list.voice) {
        list.voice = Some(voice);
        let planned: Vec<shroom_edit::Cut> = record
            .cuts
            .iter()
            .filter(|c| !c.id.is_empty() && c.kind != shroom_edit::CutKind::Manual && !list.cuts.iter().any(|x| x.id == c.id))
            .cloned()
            .collect();
        list.cuts.extend(planned);
    }
    list
}

/// How long the original stays beside a landed copy before a bake deletes it (B197 (b)):
/// two days from the copy's landing, `edits.keep_until`.
pub const KEEP_ORIGINAL_MS: i64 = 2 * DAY_MS;

/// Names the permanent copy of a saved revision in the record (`edits.copy`), once its
/// files are up — only while the record still holds that revision — and sets when the
/// original goes, `edits.keep_until`: `now` (ISO 8601, UTC) plus [`KEEP_ORIGINAL_MS`]
/// (B197). Naming the same copy again (a retry) keeps the time it first set. Returns `true`
/// when it named it: the previous copy's files then go. `false` when the record moved on (a
/// newer save, another Mac's edit, no edit at all): nothing is written, and the files of
/// `copy` that were just sent are deleted here — never one the record still names. A
/// tombstone stops it.
pub fn set_edit_copy(s3: &Client, id: &str, copy: &shroom_edit::EditCopy, now: &str) -> Result<bool, UploadError> {
    let mine: Vec<String> = copy.files().into_iter().map(str::to_string).collect();
    if !mine.iter().all(|f| is_copy_file(f)) {
        return Err(UploadError::Incomplete(format!("not a copy's file names: {mine:?}")));
    }
    let at = iso_ms(now).ok_or_else(|| UploadError::Incomplete(format!("not a time: {now:?}")))?;
    let keep_until = iso_at(u64::try_from((at + KEEP_ORIGINAL_MS).div_euclid(1000)).unwrap_or(0));
    let holds = |m: &Meta| m.edits.as_ref().is_some_and(|s| s.rev == copy.rev);
    // What the record names, so a stale copy's cleanup never takes a file it still names.
    let named = |m: &Meta| -> Vec<String> { m.edits.as_ref().and_then(|s| s.copy.as_ref()).map(|c| c.files().into_iter().map(str::to_string).collect()).unwrap_or_default() };
    // A look first: a copy of a revision the record no longer holds writes nothing.
    let bytes = with_retry(|| s3.get(&keys::meta(id)))?.ok_or(UploadError::Collision)?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}")))?;
    if raw.get("state").and_then(|s| s.as_str()) == Some("deleted") {
        return Err(UploadError::Tombstoned);
    }
    let seen: Meta = serde_json::from_value(raw).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}")))?;
    if !holds(&seen) {
        let keep = named(&seen);
        delete_copy_files(s3, id, &mine.into_iter().filter(|f| !keep.contains(f)).collect::<Vec<_>>(), &seen);
        return Ok(false);
    }
    let mut current = false;
    let mut doomed: Vec<String> = Vec::new();
    let meta = update_edit(s3, id, |m| {
        doomed.clear();
        current = holds(m);
        let Some(saved) = m.edits.as_mut().filter(|_| current) else { return };
        let again = saved.copy.as_ref() == Some(copy) && saved.keep_until.is_some();
        if let Some(previous) = saved.copy.replace(copy.clone()) {
            doomed.extend(previous.files().into_iter().map(str::to_string).filter(|f| !mine.contains(f)));
        }
        if !again {
            saved.keep_until = Some(keep_until.clone());
        }
    })?;
    if !current {
        // It moved on between the look and the write.
        let keep = named(&meta);
        doomed = mine.into_iter().filter(|f| !keep.contains(f)).collect();
    }
    delete_copy_files(s3, id, &doomed, &meta);
    Ok(current)
}

/// Where a record remembers its bakes (B197, agreed with the AI build): a list, oldest
/// first, `{"at": <iso>, "rev": <the edit's revision>, "keep": [[s, e], …]}` — the ranges
/// that played on the clock before that bake — so data kept on an older clock (a
/// transcript) maps through every later bake in order.
pub const BAKED_FROM: &str = "bakedFrom";
/// The most bakes a record remembers: the oldest drop off (a reader that finds its stamp
/// gone re-derives from the video).
pub const BAKED_FROM_MAX: usize = 20;

/// A file of the original recording beside its record: the stream (`master.m3u8`, `v0/…`),
/// its single-file copy, poster and teaser (B13). Never a thumbnail (`thumb-…`).
fn is_original_file(name: &str) -> bool {
    matches!(name, "master.m3u8" | "video.mp4" | "poster.jpg" | "teaser.mp4") || name.strip_prefix(keys::VARIANT).is_some_and(|r| r.starts_with('/'))
}

/// Copy thumbnail's picture as the record names it (the Worker writes it, B164):
/// `{"title", "gif"?, "jpg"?, "from"?}`. `from`, agreed with the Worker, is the file beside
/// the recording of the poster the picture was drawn from — `edit-<rev>-poster.jpg` for a
/// copy's, `poster.jpg` (the original's) when it doesn't say.
pub const THUMB: &str = "thumb";

/// How much of a video's start Copy thumbnail's pictures show (B164): its first frame, and a
/// loop of its first three seconds.
pub const PICTURE_SECONDS: f64 = 3.0;

/// Whether `name` is one of Copy thumbnail's pictures beside a recording (`thumb-…`).
fn is_picture_file(name: &str) -> bool {
    name.starts_with("thumb-") && !name.contains('/')
}

/// The poster the record's picture was drawn from, when it has one (`poster.jpg` when the
/// picture doesn't say).
fn picture_from(m: &Meta) -> Option<&str> {
    let t = m.extra.get(THUMB)?;
    Some(t.get("from").and_then(serde_json::Value::as_str).unwrap_or("poster.jpg"))
}

/// The record's own picture files (its loop, its still) when they were drawn from the poster it
/// shows now — the only pictures a bake that cut into the start keeps.
fn own_pictures(m: &Meta) -> Vec<String> {
    let Some(t) = m.extra.get(THUMB).filter(|_| picture_from(m).is_some_and(|f| poster_file(m).as_deref() == Some(f))) else { return vec![] };
    ["gif", "jpg"].iter().filter_map(|k| t.get(*k)?.as_str()).map(str::to_string).collect()
}

/// Whether the newest bake the record remembers kept the video's start whole — the part Copy
/// thumbnail's pictures show: its first kept range starts at 0 and runs [`PICTURE_SECONDS`],
/// or to the end of a video `before` seconds long that was shorter. A length not known asks
/// for the whole three seconds; a keep that doesn't read never kept it.
fn kept_the_start(m: &Meta, before: Option<f64>) -> bool {
    const EDGE: f64 = 1e-6;
    let newest = m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).and_then(|from| from.last());
    let range = newest.and_then(|b| b.get("keep")?.as_array()?.first()?.as_array().cloned()).unwrap_or_default();
    let (Some(s), Some(e)) = (range.first().and_then(serde_json::Value::as_f64), range.get(1).and_then(serde_json::Value::as_f64)) else { return false };
    let span = before.filter(|d| d.is_finite() && *d > 0.0).map_or(PICTURE_SECONDS, |d| d.min(PICTURE_SECONDS));
    s <= EDGE && e >= span - EDGE
}

/// The copy a bake makes the video: the saved revision's, with a copy's own file names and a
/// length.
fn bakeable(saved: &shroom_edit::Saved) -> Option<&shroom_edit::EditCopy> {
    saved.current_copy().filter(|c| c.files().iter().all(|f| is_copy_file(f)) && c.duration.is_finite() && c.duration > 0.0)
}

/// Whether the record is already baked: no edit, no stream, and its file one of its own
/// copies' (its poster and teaser too, when it names them).
fn baked(m: &Meta, id: &str) -> bool {
    let prefix = keys::prefix(id);
    let own = |k: &String| k.strip_prefix(&prefix).is_some_and(is_copy_file);
    m.edits.is_none()
        && !m.extra.contains_key("edits")
        && m.media.as_ref().is_some_and(|x| x.hls.is_none() && x.mp4.as_ref().is_some_and(own) && x.poster.as_ref().is_none_or(own) && x.teaser.as_ref().is_none_or(own))
}

/// The record's half of a bake: `media` becomes `copy`'s own files (its stream null, the
/// picture's size kept), the length the copy's, every marker moved onto the edited clock
/// (one inside a cut or outside the trim goes; none past the copy's end), the edit gone,
/// the bake remembered in [`BAKED_FROM`], and Copy thumbnail's picture ([`THUMB`]) gone unless
/// it was drawn from the copy's own poster — the page draws a fresh one. `false`, changing
/// nothing, unless `copy` is still the record's current copy.
fn bake_record(m: &mut Meta, id: &str, copy: &shroom_edit::EditCopy, now: &str) -> bool {
    let Some(saved) = m.edits.clone().filter(|s| bakeable(s) == Some(copy)) else { return false };
    // The list as the page plays it: on the record's own clock.
    let mut list = saved.list;
    if let Some(d) = m.duration_sec.filter(|d| d.is_finite() && *d > 0.0) {
        list.source_duration = d;
    }
    let list = list.checked();
    let key = |name: &String| format!("{}{name}", keys::prefix(id));
    let (width, height) = m.media.as_ref().map_or((0, 0), |x| (x.width, x.height));
    m.media = Some(Media { hls: None, mp4: Some(key(&copy.file)), poster: copy.poster.as_ref().map(key), teaser: copy.teaser.as_ref().map(key), width, height });
    m.duration_sec = Some(copy.duration);
    m.markers = m.markers.take().map(|all| all.into_iter().filter_map(|t| list.to_edited(t)).map(|e| ((e * 1000.0).round() / 1000.0).min(copy.duration)).collect());
    let keep: Vec<[f64; 2]> = list.keep().into_iter().map(|(s, e)| [s, e]).collect();
    let mut from = m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).cloned().unwrap_or_default();
    from.push(serde_json::json!({ "at": now, "rev": saved.rev, "keep": keep }));
    let over = from.len().saturating_sub(BAKED_FROM_MAX);
    from.drain(..over);
    m.extra.insert(BAKED_FROM.into(), serde_json::Value::Array(from));
    m.edits = None;
    if !picture_from(m).is_some_and(|f| copy.poster.as_deref() == Some(f)) {
        m.extra.remove(THUMB);
    }
    true
}

/// What a bake did (B197 (c)), for its caller's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Baked {
    /// The revision of the edit the video is now (the newest bake [`BAKED_FROM`] remembers).
    pub rev: u64,
    /// Whether this call made the edit the video. `false`: it already was — a bake repeated,
    /// or another Mac's that came first — and this call only finished its deletes.
    pub written: bool,
    /// How many files beside the record this call deleted.
    pub deleted: usize,
}

/// “The edit becomes the video” (B197 (c)): once the record's copy is current, one
/// read-modify-write points `media` at the copy's own files ([`bake_record`]) and drops the
/// edit; then the original's files go (`master.m3u8`, `v0/…`, `video.mp4`, `poster.jpg`,
/// `teaser.mp4`) and every copy's file the record no longer names (an earlier bake's).
/// Never a file `media` names after the write. Copy thumbnail's pictures (`thumb-…`) show the
/// video's first three seconds: they go only when the edit cut into those — all but the
/// record's own, drawn from the copy's poster — and otherwise stay (B164: a sent paste keeps
/// its picture). Nothing is renamed, so any of the space's Macs can do it without the video
/// in hand.
///
/// Idempotent and safe to repeat after any interruption — the record first, then the
/// deletes: a record already baked (no edit, `media` one of its own copies) writes nothing
/// and finishes the deletes. `Ok(Some(_))`: baked, now or before ([`Baked`] says which, and
/// what this call deleted). `Ok(None)`, nothing written or deleted: no edit, no current
/// copy (not made yet, or a newer save came first), or an edit this build can't read. A
/// copy the record names but the space doesn't hold is an error, before anything is
/// written. A tombstone stops it. `now` (ISO 8601, UTC) is when.
pub fn bake(s3: &Client, id: &str, now: &str) -> Result<Option<Baked>, UploadError> {
    bake_as(s3, id, now, false)
}

/// The upkeep's and the cleanup sweep's [`bake`]: only while the record's current copy is
/// past its `keep_until`, checked again inside the write. A newer revision whose copy landed
/// after the pass read the record keeps its own two days (`Ok(None)`, nothing written).
pub fn bake_when_due(s3: &Client, id: &str, now: &str) -> Result<Option<Baked>, UploadError> {
    bake_as(s3, id, now, true)
}

/// Whether a saved edit's original has had its two days by `at` (ms): a `keep_until` that
/// reads, at or before it.
fn due(saved: &shroom_edit::Saved, at: i64) -> bool {
    saved.keep_until.as_deref().and_then(iso_ms).is_some_and(|k| k <= at)
}

fn bake_as(s3: &Client, id: &str, now: &str, due_only: bool) -> Result<Option<Baked>, UploadError> {
    let at = iso_ms(now).ok_or_else(|| UploadError::Incomplete(format!("not a time: {now:?}")))?;
    let prefix = keys::prefix(id);
    // A look first: a record with nothing to bake costs one read.
    let seen = look(s3, id)?;
    let copy = match seen.edits.as_ref() {
        Some(saved) => match bakeable(saved) {
            Some(c) if !due_only || due(saved, at) => c.clone(),
            _ => return Ok(None),
        },
        None if baked(&seen, id) => return finish(s3, id, &seen),
        None => return Ok(None),
    };
    // What's beside the record before the write: the deletes take only from this, so a
    // file sent after it (a later edit's copy) is never one of them.
    let listed = with_retry(|| s3.list(&prefix))?;
    let file = format!("{prefix}{}", copy.file);
    if !listed.iter().any(|o| o.key == file && o.size > 0) {
        return Err(UploadError::Incomplete(format!("the copy {} isn't in the space", copy.file)));
    }
    // The length the video had before the bake, on whose clock its list plays.
    let before = seen.duration_sec.filter(|d| d.is_finite() && *d > 0.0).or_else(|| seen.edits.as_ref().map(|s| s.list.source_duration));
    let mut done = false;
    let meta = update_edit(s3, id, |m| done = (!due_only || m.edits.as_ref().is_some_and(|s| due(s, at))) && bake_record(m, id, &copy, now))?;
    // Between the look and the write: another Mac's bake (finish it), or a newer save
    // (nothing to do).
    if !done && !baked(&meta, id) {
        return Ok(None);
    }
    let deleted = bake_deletes(s3, id, &listed, &meta, !kept_the_start(&meta, before))?;
    Ok(Some(Baked { rev: baked_rev(&meta), written: done, deleted }))
}

/// The record, read once before a write: a tombstone stops what was asked.
fn look(s3: &Client, id: &str) -> Result<Meta, UploadError> {
    let bytes = with_retry(|| s3.get(&keys::meta(id)))?.ok_or(UploadError::Collision)?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}")))?;
    if raw.get("state").and_then(|s| s.as_str()) == Some("deleted") {
        return Err(UploadError::Tombstoned);
    }
    serde_json::from_value(raw).map_err(|e| UploadError::Incomplete(format!("unreadable meta: {e}")))
}

/// The deletes of a video already baked (`record`), finished: what its bake deletes, from
/// what's beside it now (the length it had before isn't known any more).
fn finish(s3: &Client, id: &str, record: &Meta) -> Result<Option<Baked>, UploadError> {
    let listed = with_retry(|| s3.list(&keys::prefix(id)))?;
    let deleted = bake_deletes(s3, id, &listed, record, !kept_the_start(record, None))?;
    Ok(Some(Baked { rev: baked_rev(record), written: false, deleted }))
}

/// A bake cut off after its write, finished by the cleanup sweep: the record read again, and
/// only a video baked — no edit, a bake [`BAKED_FROM`] remembers, `media` its own copy's —
/// has its deletes finished. Never a bake of its own: a record edited again since, or never
/// baked, is left as it is (`None`).
fn finish_bake(s3: &Client, id: &str) -> Result<Option<Baked>, UploadError> {
    let seen = look(s3, id)?;
    if !(remembers_a_bake(&seen) && baked(&seen, id)) {
        return Ok(None);
    }
    finish(s3, id, &seen)
}

/// Whether the record remembers a bake ([`BAKED_FROM`] holds one).
fn remembers_a_bake(m: &Meta) -> bool {
    m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).is_some_and(|from| !from.is_empty())
}

/// A bake's deletes, from `listed` (what was beside the record before its write): when the
/// edit cut into the video's start (`start_cut`), Copy thumbnail's pictures — all but the
/// record's own, drawn from the poster it shows now — then the original's files and every
/// copy's file `record`'s media doesn't name. The pictures go first: a bake cut off part-way
/// always leaves one of the others behind, which the cleanup sweep finds. How many went.
fn bake_deletes(s3: &Client, id: &str, listed: &[Object], record: &Meta, start_cut: bool) -> Result<usize, UploadError> {
    let prefix = keys::prefix(id);
    let playing = media_keys(record);
    let own = own_pictures(record);
    let names = || listed.iter().filter_map(|o| Some((o.key.as_str(), o.key.strip_prefix(&prefix)?)));
    let pictures = names().filter(|(_, n)| start_cut && is_picture_file(n) && !own.iter().any(|p| p == n));
    let files = names().filter(|(k, n)| (is_original_file(n) || is_copy_file(n)) && !playing.iter().any(|p| p == k));
    let mut deleted = 0;
    for (key, _) in pictures.chain(files) {
        with_retry(|| s3.delete(key))?;
        deleted += 1;
    }
    Ok(deleted)
}

/// The revision of the newest bake the record remembers (0 when it doesn't say).
fn baked_rev(m: &Meta) -> u64 {
    m.extra.get(BAKED_FROM).and_then(serde_json::Value::as_array).and_then(|from| from.last()).and_then(|b| b.get("rev")?.as_u64()).unwrap_or(0)
}

/// What one upkeep pass does with the space's edited videos (B197 (d)).
#[derive(Debug, Default, PartialEq)]
pub struct UpkeepPlan {
    /// Videos to [`bake`]: their copy is current and the two days their original is kept
    /// are up.
    pub bake: Vec<String>,
    /// Copies to make, `(id, the saved revision)`: an edit that leaves something out with no
    /// current copy (a copy that failed, an edit saved by another tool), whose video is on
    /// this Mac.
    pub copy: Vec<(String, u64)>,
}

/// The upkeep pass's plan (B197 (d)), pure: of the live records, those `mine` names, the
/// ones to bake — a current copy, `keep_until` at or before `now` (ISO 8601, UTC) — and the
/// copies to make — a non-identity edit without a current copy, its video on this Mac
/// (`local`). A Mac passes its own videos as `mine` (a member's own; every video on an
/// owner-class Mac); the token Mac's cleanup sweep passes every record (`|_| true`, and no
/// video as local). Never a bake without a `keep_until` it can read: the original goes
/// only when a copy's landing set when.
pub fn upkeep_plan(records: &[(String, Meta)], now: &str, mine: &dyn Fn(&str) -> bool, local: &dyn Fn(&str) -> bool) -> UpkeepPlan {
    let now = iso_ms(now);
    let mut plan = UpkeepPlan::default();
    for (id, m) in records {
        if m.state != State::Live || !mine(id) {
            continue;
        }
        let Some(saved) = m.edits.as_ref() else { continue };
        if bakeable(saved).is_some() {
            let due = saved.keep_until.as_deref().and_then(iso_ms);
            if due.zip(now).is_some_and(|(due, now)| due <= now) {
                plan.bake.push(id.clone());
            }
        } else if saved.current_copy().is_none() && m.edited_sec().is_some() && local(id) {
            plan.copy.push((id.clone(), saved.rev));
        }
    }
    plan
}

/// Keeps a recording off the suggestions to clean up for good (N68): `keep: true` on its
/// record, nothing else changed; a tombstone stops it.
pub fn keep(s3: &Client, id: &str) -> Result<Meta, UploadError> {
    update_meta(s3, id, |m| {
        m.extra.insert("keep".into(), serde_json::Value::Bool(true));
    })
}

/// Deletes a shared recording: the record becomes a tombstone first (the link answers
/// “removed” forever and every uploading Mac stops), then every byte under `rec/<id>/`.
/// Safe to run again after an interruption.
pub fn delete(s3: &Client, id: &str) -> Result<(), UploadError> {
    let key = keys::meta(id);
    // The bytes go only once the record says deleted (or there never was a record): a
    // delete that can't write its tombstone must not leave a link to nothing.
    let mut tombstoned = false;
    for _ in 0..8 {
        let Some((bytes, etag)) = with_retry(|| s3.get_with_etag(&key))? else {
            tombstoned = true;
            break;
        };
        let old: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if old.get("state").and_then(|s| s.as_str()) == Some("deleted") {
            tombstoned = true;
            break;
        }
        let tomb = serde_json::json!({
            "schema": 1, "id": id, "state": "deleted", "deletedAt": now_iso(),
            "createdAt": old.get("createdAt"), "owner": old.get("owner"),
        });
        if with_retry(|| s3.put_if_match(&key, tomb.to_string().as_bytes(), "application/json", &etag))? {
            tombstoned = true;
            break;
        }
    }
    if !tombstoned {
        return Err(UploadError::Incomplete("the record kept changing; nothing was removed".into()));
    }
    remove_bytes(s3, id)
}

/// Discards a take that was never shared: its bytes and its claimed record go, with no
/// tombstone (§5.5 — nobody ever had the link).
pub fn discard(s3: &Client, id: &str) -> Result<(), UploadError> {
    remove_bytes(s3, id)?;
    with_retry(|| s3.delete(&keys::meta(id)))?;
    Ok(())
}

fn remove_bytes(s3: &Client, id: &str) -> Result<(), UploadError> {
    for o in with_retry(|| s3.list(&keys::prefix(id)))? {
        with_retry(|| s3.delete(&o.key))?;
    }
    Ok(())
}

/// What a cleanup pass removed.
#[derive(Debug, Default, PartialEq)]
pub struct Swept {
    /// Recordings whose bytes outlived their tombstone (a late upload from another Mac).
    pub under_tombstones: Vec<String>,
    /// Bytes with no record at all, once the claim had every chance to land.
    pub orphans: Vec<String>,
}

/// The cleanup sweep for interrupted deletes (§5.5): compares `rec/` against the records
/// and removes bytes under a tombstone, and bytes with no record whose ids appear in
/// `orphans_ok` (the caller passes ids it knows are stale — the claim is written before
/// any byte, so a record-less id is either mid-claim or abandoned; only the caller can tell).
/// A live record's bytes are never touched here: its stream, its copies, its thumbnails.
pub fn sweep(s3: &Client, orphans_ok: &dyn Fn(&str) -> bool) -> Result<Swept, UploadError> {
    sweep_reading(s3, orphans_ok, &mut |_, _, _| {})
}

/// What the sweep's listing holds beside one recording that a bake deletes: any of the
/// original's files, and its copies' files by name.
#[derive(Debug, Default)]
struct Beside {
    original: bool,
    copies: Vec<String>,
}

/// [`sweep`], handing every other record it reads (one with bytes, not a tombstone) to
/// `live` with what its listing holds beside it, so a caller that needs them costs no second
/// read.
fn sweep_reading(s3: &Client, orphans_ok: &dyn Fn(&str) -> bool, live: &mut dyn FnMut(&str, &[u8], &Beside)) -> Result<Swept, UploadError> {
    let mut ids: BTreeMap<String, Beside> = BTreeMap::new();
    for o in with_retry(|| s3.list("rec/"))? {
        let Some(rest) = o.key.strip_prefix("rec/") else { continue };
        let (id, name) = rest.split_once('/').unwrap_or((rest, ""));
        let beside = ids.entry(id.to_string()).or_default();
        if is_original_file(name) {
            beside.original = true;
        } else if is_copy_file(name) {
            beside.copies.push(name.to_string());
        }
    }
    let mut out = Swept::default();
    for (id, beside) in ids {
        match with_retry(|| s3.get(&keys::meta(&id)))? {
            Some(b) if serde_json::from_slice::<serde_json::Value>(&b).ok().is_some_and(|v| v["state"] == "deleted") => {
                remove_bytes(s3, &id)?;
                out.under_tombstones.push(id);
            }
            None if orphans_ok(&id) => {
                remove_bytes(s3, &id)?;
                out.orphans.push(id);
            }
            Some(b) => live(&id, &b, &beside),
            None => {}
        }
    }
    Ok(out)
}

/// Whether a record is a live video baked (B197) whose listing still holds a file its bake
/// deletes — one of the original's, or a copy's its `media` doesn't name: a bake whose
/// deletes were cut off after its write. A record that isn't baked never is.
fn bake_unfinished(m: &Meta, id: &str, beside: &Beside) -> bool {
    if m.state != State::Live || !remembers_a_bake(m) || !baked(m, id) {
        return false;
    }
    let playing = media_keys(m);
    beside.original || beside.copies.iter().any(|name| !playing.contains(&format!("{}{name}", keys::prefix(id))))
}

/// Where the space keeps its roster, its ownership ledger and the page's one-time keys
/// (WORKER-INTERFACE §2.4, §3.3) — the Worker's own constants, read here only by the token
/// Mac's cleanup sweep.
pub const ROSTER: &str = ".shroom/registry/roster.json";
pub const LEDGER: &str = ".shroom/registry/recs/";
pub const KEYS: &str = ".shroom/keys/";

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
/// The longest an upload credential lives (N4, B35 P6): a removed Mac still finishes the
/// takes whose credential outlives its removal (N32), so its bytes wait out that life.
pub const UPLOAD_CREDENTIAL_MS: i64 = 7 * DAY_MS;
/// How long a page key outlives its `exp` before the sweep takes it (a clock a little off
/// never takes a key still being redeemed).
pub const KEY_SLACK_MS: i64 = HOUR_MS;

/// What the token Mac's cleanup sweep knows of this Mac (DECISIONS B91).
pub struct Here<'a> {
    /// Now, Unix milliseconds.
    pub now_ms: i64,
    /// This Mac's device id in the space, when it has one.
    pub device: Option<&'a str>,
    /// Whether this Mac still holds a take with this recording id — any take folder at all
    /// (recording, paused, saved, queued, done): law 2, never a recording a Mac holds.
    pub holds: &'a dyn Fn(&str) -> bool,
}

/// What one cleanup pass removed.
#[derive(Debug, Default, PartialEq)]
pub struct Cleaned {
    pub swept: Swept,
    /// Page keys (`.shroom/keys/<hash>`) spent or past their life.
    pub keys: usize,
    /// Videos whose original's two days were up: their edit became the video (B197 (d)); and
    /// videos whose bake was cut off after its write, its deletes finished now (`written`
    /// false) — each with what its bake did.
    pub baked: Vec<(String, Baked)>,
}

/// The token Mac's cleanup sweep (DECISIONS B91; ARCHITECTURE §5.5's GC, §16 #49):
/// bytes under a tombstone go (an interrupted delete, a late segment from another Mac);
/// bytes with no record go only once no Mac can still finish them (`reapable`); page keys
/// go once spent or an hour past their life; and every video in the space whose original's
/// two days are up is baked (B197 (d), [`upkeep_plan`] over every record the sweep reads,
/// [`bake`]) — a bake that fails waits for the next pass, the rest of the pass stands. A
/// video whose bake was cut off after its write (no edit left for the plan to read, the
/// original's files still beside it) has its deletes finished. No other record is touched,
/// and nothing a live record names: a link that was made keeps answering as it did.
pub fn cleanup(s3: &Client, here: &Here) -> Result<Cleaned, UploadError> {
    cleanup_sparing(s3, here, &|_| false)
}

/// [`cleanup`], never baking — nor finishing a bake of — a recording `open` says this Mac's
/// editor has open: its list is on the clock it opened on (B197). Asked just before each, so
/// a video opened while the sweep reads the space is spared too; a later pass bakes it once
/// the editor has closed.
pub fn cleanup_sparing(s3: &Client, here: &Here, open: &dyn Fn(&str) -> bool) -> Result<Cleaned, UploadError> {
    // An unreadable roster reaps no orphan at all (fail closed): who can still finish one
    // is the roster's to say.
    let roster: Option<serde_json::Value> = with_retry(|| s3.get(ROSTER))?.and_then(|b| serde_json::from_slice(&b).ok());
    let mut edited: Vec<(String, Meta)> = Vec::new();
    let mut unfinished: Vec<String> = Vec::new();
    let reapable_here = |id: &str| {
        let ledger = s3.get(&format!("{LEDGER}{id}")).ok().flatten().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        reapable(id, ledger.as_ref(), roster.as_ref(), here)
    };
    let swept = sweep_reading(s3, &reapable_here, &mut |id, bytes, beside| {
        let Ok(m) = serde_json::from_slice::<Meta>(bytes) else { return };
        if m.edits.is_some() {
            edited.push((id.to_string(), m));
        } else if bake_unfinished(&m, id, beside) {
            unfinished.push(id.to_string());
        }
    })?;
    let mut keys = 0;
    for o in with_retry(|| s3.list(KEYS))? {
        let Some(bytes) = with_retry(|| s3.get(&o.key))? else { continue };
        if key_done(&bytes, here.now_ms) {
            with_retry(|| s3.delete(&o.key))?;
            keys += 1;
        }
    }
    let now = iso_at(u64::try_from(here.now_ms.div_euclid(1000)).unwrap_or(0));
    let mut baked = Vec::new();
    for id in upkeep_plan(&edited, &now, &|_| true, &|_| false).bake {
        if open(&id) {
            continue;
        }
        // One that fails waits for the next pass; the rest of this one stands.
        if let Ok(Some(b)) = bake_when_due(s3, &id, &now) {
            baked.push((id, b));
        }
    }
    for id in unfinished {
        if open(&id) {
            continue;
        }
        if let Ok(Some(b)) = finish_bake(s3, &id) {
            baked.push((id, b));
        }
    }
    Ok(Cleaned { swept, keys, baked })
}

/// Whether a recording's bytes with no record may go (B91): a recording only ever gets
/// bytes under a credential, and a credential only after its ledger entry (§2.4), so the
/// entry says who could still finish it. Never a recording this Mac holds (law 2), never
/// one younger than a day, never without a ledger entry (it can't be placed), and only
/// when no Mac can still send it: this Mac made it and no longer holds it (discarded, the
/// owed delete lost), or the Mac that made it is out of the space — removed or left — for
/// longer than an upload credential lives (N32: a removed Mac still finishes the takes its
/// credential covers). An active Mac's take may sit saved or queued for weeks: it stays.
pub fn reapable(id: &str, ledger: Option<&serde_json::Value>, roster: Option<&serde_json::Value>, here: &Here) -> bool {
    if (here.holds)(id) {
        return false;
    }
    let Some(entry) = ledger else { return false };
    let Some(at) = entry.get("at").and_then(|v| v.as_str()).and_then(iso_ms) else { return false };
    if here.now_ms - at < DAY_MS {
        return false;
    }
    let Some(device) = entry.get("device").and_then(|v| v.as_str()).filter(|d| !d.is_empty()) else { return false };
    if here.device == Some(device) {
        return true;
    }
    let Some(d) = roster.and_then(|r| r.get("devices")).and_then(|ds| ds.get(device)) else { return false };
    if d.get("state").and_then(|v| v.as_str()) == Some("active") {
        return false;
    }
    d.get("endedAt").and_then(|v| v.as_str()).and_then(iso_ms).is_some_and(|ended| here.now_ms - ended > UPLOAD_CREDENTIAL_MS + DAY_MS)
}

/// A page key that can never be redeemed again: spent, an hour past its `exp`, or not a
/// grant at all.
pub fn key_done(bytes: &[u8], now_ms: i64) -> bool {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) else { return true };
    if v.get("spent").is_some_and(|s| !s.is_null() && s != &serde_json::Value::Bool(false)) {
        return true;
    }
    match v.get("exp").and_then(|e| e.as_str()).and_then(iso_ms) {
        Some(exp) => now_ms - exp > KEY_SLACK_MS,
        None => true,
    }
}

/// `2026-10-02T12:00:00Z` (fractional seconds allowed) → Unix milliseconds.
pub fn iso_ms(s: &str) -> Option<i64> {
    let s = s.trim().strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let (hms, frac) = time.split_once('.').unwrap_or((time, ""));
    let mut t = hms.split(':').map(|p| p.parse::<i64>().ok());
    let (h, mi, sec) = (t.next()??, t.next()??, t.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    let ms: i64 = format!("{:0<3}", frac.chars().take(3).collect::<String>()).parse().unwrap_or(0);
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + h * 3_600 + mi * 60 + sec) * 1_000) + ms)
}

/// The tombstone-before-resume check: true when the record says deleted.
pub fn tombstoned(s3: &Client, id: &str) -> Result<bool, UploadError> {
    Ok(with_retry(|| s3.get(&keys::meta(id)))?
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|v| v.get("state").and_then(|s| s.as_str()) == Some("deleted")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row carries its record's AI line (`meta::ai_note`), so a refresh can log a change in
    /// what the space did; a cache written before it still reads.
    #[test]
    fn a_row_carries_its_records_ai_line() {
        let m: Meta = serde_json::from_value(serde_json::json!({"schema":1,"id":"x","state":"live","createdAt":"2026-10-18T14:00:00Z","title":"T",
            "naming":{"later":"daily","at":"2026-10-18T23:14:00Z","code":"4006"}})).unwrap();
        assert_eq!(Row::of(&m).ai.as_deref(), Some("waiting for the free day since 2026-10-18T23:14:00Z (4006)"));
        let old: Row = serde_json::from_str(r#"{"id":"x","title":"T","state":"live","created_at":"2026-10-18T14:00:00Z","duration_sec":null,"owner":null,"poster":null}"#).unwrap();
        assert_eq!(old.ai, None);
    }

    const NOW: i64 = 1_790_942_400_000; // 2026-10-02T12:00:00Z
    fn iso(ms: i64) -> String {
        // Days → civil date (Hinnant), enough for the tests' fixed range.
        let secs = ms.div_euclid(1000);
        let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
    }

    #[test]
    fn iso_round_trips() {
        assert_eq!(iso_ms("2026-10-02T12:00:00Z"), Some(NOW));
        assert_eq!(iso_ms(&iso(NOW - 9 * DAY_MS)), Some(NOW - 9 * DAY_MS));
        assert_eq!(iso_ms("2026-10-02T12:00:00.5Z"), Some(NOW + 500));
        assert_eq!(iso_ms("garbage"), None);
    }

    /// B91: who may lose bytes with no record — only what no Mac can still finish.
    #[test]
    fn only_bytes_no_mac_can_still_finish_are_reapable() {
        let held = |id: &str| id == "HELDHELDHELDHELD";
        let here = Here { now_ms: NOW, device: Some("d_this"), holds: &held };
        let entry = |device: Option<&str>, age: i64| serde_json::json!({ "v": 1, "member": "m1", "device": device, "at": iso(NOW - age) });
        let roster = serde_json::json!({ "devices": {
            "d_active": { "state": "active", "endedAt": null },
            "d_gone_long": { "state": "removed", "endedAt": iso(NOW - 9 * DAY_MS) },
            "d_gone_lately": { "state": "removed", "endedAt": iso(NOW - 3 * DAY_MS) },
            "d_left_long": { "state": "left", "endedAt": iso(NOW - 30 * DAY_MS) },
            "d_no_end": { "state": "removed" },
        }});
        let r = |id: &str, e: Option<&serde_json::Value>, ro: Option<&serde_json::Value>| reapable(id, e, ro, &here);
        let old = 2 * DAY_MS;
        // This Mac's own, no longer held: it goes.
        assert!(r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_this"), old)), Some(&roster)));
        // …but never one it still holds (law 2), and never one younger than a day.
        assert!(!r("HELDHELDHELDHELD", Some(&entry(Some("d_this"), old)), Some(&roster)));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_this"), HOUR_MS)), Some(&roster)));
        // An active Mac's take may sit saved or queued for weeks: it stays.
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_active"), 60 * DAY_MS)), Some(&roster)));
        // A Mac out of the space past a credential's life: its bytes go.
        assert!(r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_gone_long"), 20 * DAY_MS)), Some(&roster)));
        assert!(r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_left_long"), 40 * DAY_MS)), Some(&roster)));
        // …but not while a credential minted before its removal may still be sending (N32).
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_gone_lately"), 20 * DAY_MS)), Some(&roster)));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_no_end"), 20 * DAY_MS)), Some(&roster)));
        // Nothing that can't be placed: no entry, no device, a device the roster doesn't
        // name, no roster at all.
        assert!(!r("AAAAAAAAAAAAAAAA", None, Some(&roster)));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(None, old)), Some(&roster)));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_stranger"), 20 * DAY_MS)), Some(&roster)));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&entry(Some("d_gone_long"), 20 * DAY_MS)), None));
        assert!(!r("AAAAAAAAAAAAAAAA", Some(&serde_json::json!({ "device": "d_this" })), Some(&roster)), "no date: can't tell its age");
    }

    #[test]
    fn a_page_key_goes_once_spent_or_an_hour_past_its_life() {
        let grant = |exp: i64, spent: bool| {
            let mut v = serde_json::json!({ "member": "m1", "device": "d_x", "exp": iso(exp) });
            if spent {
                v["spent"] = serde_json::json!(iso(exp - 60_000));
            }
            v.to_string().into_bytes()
        };
        assert!(!key_done(&grant(NOW + 5 * 60_000, false), NOW), "a live key stays");
        assert!(!key_done(&grant(NOW - 30 * 60_000, false), NOW), "a little past its life: a clock may be off");
        assert!(key_done(&grant(NOW - 2 * HOUR_MS, false), NOW), "expired, never redeemed");
        assert!(key_done(&grant(NOW + 5 * 60_000, true), NOW), "spent");
        assert!(key_done(b"not json", NOW));
        assert!(key_done(br#"{"member":"m1"}"#, NOW));
    }

    /// An edit's writes delete only a copy's own files, whatever a record names.
    #[test]
    fn only_a_copys_files_are_ever_deleted_by_an_edit() {
        for ok in ["edit-1.mp4", "edit-3-poster.jpg", "edit-123456789-teaser.mp4"] {
            assert!(is_copy_file(ok), "{ok}");
        }
        for not in ["video.mp4", "poster.jpg", "master.m3u8", "edit-.mp4", "edit-1234567890.mp4", "edit-3.mp4.tmp", "v0/edit-3.mp4", "../meta/x.json", "edit-3-poster.png", "edit-x.mp4"] {
            assert!(!is_copy_file(not), "{not}");
        }
    }

    /// A member can rewrite `owner` on their own records: a Library row never takes it for
    /// the video's owner (WORKER-INTERFACE §4.10, whole-spec audit b4 #5).
    #[test]
    fn a_records_own_owner_is_never_taken_for_its_owner() {
        let mut m = Meta::uploading("AbCdEfGhIjKlMnOp", "Mher's notes", "2026-10-02T12:00:00Z");
        m.owner = Some("m1".into());
        assert_eq!(Row::of(&m).owner, None);
    }

    /// B197 (d): which videos the upkeep pass bakes and which copies it makes.
    #[test]
    fn the_upkeep_plan_bakes_what_is_due_and_copies_what_is_missing() {
        let now = "2026-10-20T09:30:00Z";
        let rec = |id: &str, state: State, rev: u64, copy: Option<u64>, keep_until: Option<&str>, trimmed: bool| {
            let mut m = Meta::uploading(id, id, "2026-10-02T12:00:00Z");
            m.state = state;
            m.duration_sec = Some(60.0);
            let mut list = shroom_edit::EditList::new(60.0);
            if trimmed {
                list.in_ = 8.0;
            }
            let mut saved = shroom_edit::Saved::new(list, rev);
            saved.copy = copy.map(|r| shroom_edit::EditCopy { rev: r, file: shroom_edit::EditCopy::file_for(r), duration: 52.0, poster: None, teaser: None });
            saved.keep_until = keep_until.map(str::to_string);
            m.edits = Some(saved);
            (id.to_string(), m)
        };
        let records = vec![
            rec("DueLongAgo000000", State::Live, 2, Some(2), Some("2026-10-19T09:30:00Z"), true),
            rec("DueNow0000000000", State::Live, 1, Some(1), Some(now), true),
            rec("NotYet0000000000", State::Live, 1, Some(1), Some("2026-10-20T09:30:01Z"), true),
            rec("NoKeepUntil00000", State::Live, 1, Some(1), None, true),
            rec("GarbledKeep00000", State::Live, 1, Some(1), Some("Tuesday"), true),
            rec("AnisDueVideo0000", State::Live, 1, Some(1), Some("2026-10-01T00:00:00Z"), true),
            rec("NoCopyYet0000000", State::Live, 3, None, None, true),
            rec("StaleCopy0000000", State::Live, 4, Some(3), Some("2026-10-01T00:00:00Z"), true),
            rec("NoCopyElsewhere0", State::Live, 1, None, None, true),
            rec("Identity00000000", State::Live, 2, None, None, false),
            rec("StillUploading00", State::Uploading, 1, None, None, true),
            rec("Tombstone0000000", State::Deleted, 1, Some(1), Some("2026-10-01T00:00:00Z"), true),
            ("NoEdit0000000000".to_string(), Meta { state: State::Live, ..Meta::uploading("NoEdit0000000000", "x", "2026-10-02T12:00:00Z") }),
        ];
        let mine = |id: &str| !id.starts_with("Anis");
        let local = |id: &str| id != "NoCopyElsewhere0";
        let plan = upkeep_plan(&records, now, &mine, &local);
        assert_eq!(plan.bake, ["DueLongAgo000000", "DueNow0000000000"]);
        assert_eq!(plan.copy, [("NoCopyYet0000000".to_string(), 3), ("StaleCopy0000000".to_string(), 4)]);

        // The token Mac's sweep: every record in the space, no copies (it holds no videos).
        let plan = upkeep_plan(&records, now, &|_| true, &|_| false);
        assert_eq!(plan.bake, ["DueLongAgo000000", "DueNow0000000000", "AnisDueVideo0000"]);
        assert!(plan.copy.is_empty());

        // A clock it can't read bakes nothing; the copies are still made.
        let plan = upkeep_plan(&records, "later", &mine, &local);
        assert!(plan.bake.is_empty());
        assert_eq!(plan.copy.len(), 2);
    }

    /// B197: what a record plays moves with a bake only — never with a rename, a save or a
    /// copy's landing — and with every bake, its file named alike or not.
    #[test]
    fn what_a_record_plays_moves_only_with_a_bake() {
        let id = "AbCdEfGhIjKlMnOp";
        let mut m = Meta::uploading(id, "Demo", "2026-10-02T12:00:00Z");
        m.media = Some(Media { hls: Some(format!("rec/{id}/master.m3u8")), mp4: Some(format!("rec/{id}/video.mp4")), poster: None, teaser: None, width: 1, height: 1 });
        let opened = Plays::of(&m);
        assert_eq!(opened, Plays { mp4: Some(format!("rec/{id}/video.mp4")), bakes: 0 });
        let mut edited = m.clone();
        edited.title = "Demo, edited".into();
        let mut saved = shroom_edit::Saved::new(shroom_edit::EditList::new(60.0), 3);
        saved.copy = Some(shroom_edit::EditCopy { rev: 3, file: "edit-3.mp4".into(), duration: 50.0, poster: None, teaser: None });
        edited.edits = Some(saved);
        assert_eq!(Plays::of(&edited), opened, "a rename, a save, a copy landing");
        let mut baked = m.clone();
        baked.media.as_mut().unwrap().hls = None;
        baked.media.as_mut().unwrap().mp4 = Some(format!("rec/{id}/edit-3.mp4"));
        baked.extra.insert(BAKED_FROM.into(), serde_json::json!([{ "at": "2026-10-18T00:00:00Z", "rev": 3, "keep": [[0, 50]] }]));
        assert_ne!(Plays::of(&baked), opened, "a bake");
        let mut again = baked.clone();
        again.extra.insert(BAKED_FROM.into(), serde_json::json!([{}, {}]));
        assert_ne!(Plays::of(&again), Plays::of(&baked), "a second bake, its file named alike or not");
    }

    /// B164: a bake kept the start Copy thumbnail's pictures show when the newest bake's first
    /// kept range runs from 0 for three seconds — or to the end of a shorter video; a length not
    /// known asks for all three.
    #[test]
    fn a_bake_kept_the_start_when_its_first_three_seconds_play() {
        let baked = |keep: serde_json::Value| {
            let mut m = Meta::uploading("AbCdEfGhIjKlMnOp", "t", "2026-10-02T12:00:00Z");
            m.extra.insert(BAKED_FROM.into(), serde_json::json!([{ "at": "2026-10-01T00:00:00Z", "rev": 1, "keep": [[4.0, 9.0]] }, { "at": "2026-10-02T00:00:00Z", "rev": 2, "keep": keep }]));
            m
        };
        assert!(kept_the_start(&baked(serde_json::json!([[0.0, 3.0], [5.0, 9.0]])), Some(60.0)), "the newest bake's, not the first's");
        assert!(kept_the_start(&baked(serde_json::json!([[0.0, 50.0]])), None));
        assert!(!kept_the_start(&baked(serde_json::json!([[0.0, 2.9], [4.0, 9.0]])), Some(60.0)), "a cut inside the first three seconds");
        assert!(!kept_the_start(&baked(serde_json::json!([[0.5, 9.0]])), Some(60.0)), "the start trimmed");
        assert!(kept_the_start(&baked(serde_json::json!([[0.0, 2.5]])), Some(2.5)), "a shorter video kept to its end");
        assert!(!kept_the_start(&baked(serde_json::json!([[0.0, 2.0]])), Some(2.5)), "a shorter video, its end cut");
        assert!(!kept_the_start(&baked(serde_json::json!([[0.0, 2.5]])), None), "its length not known");
        assert!(!kept_the_start(&baked(serde_json::json!([])), Some(60.0)), "nothing plays");
        assert!(!kept_the_start(&baked(serde_json::json!("garbled")), Some(60.0)));
        assert!(!kept_the_start(&Meta::uploading("AbCdEfGhIjKlMnOp", "t", "2026-10-02T12:00:00Z"), Some(60.0)), "never baked");
    }

    /// N63, B197: a row carries the edited length (none for the identity) and its poster's
    /// file name beside the recording, the original's or a baked edit's — never a path out.
    #[test]
    fn a_row_carries_its_edited_length_and_its_posters_file_name() {
        let id = "AbCdEfGhIjKlMnOp";
        let mut m = Meta::uploading(id, "Demo", "2026-10-02T12:00:00Z");
        m.duration_sec = Some(60.0);
        let media = |poster: Option<String>| crate::meta::Media { hls: None, mp4: None, poster, teaser: None, width: 1, height: 1 };
        assert_eq!((Row::of(&m).edited_sec, Row::of(&m).poster_file), (None, None));
        m.media = Some(media(Some(format!("rec/{id}/poster.jpg"))));
        assert_eq!(Row::of(&m).poster_file.as_deref(), Some("poster.jpg"));
        m.media = Some(media(Some(format!("rec/{id}/edit-3-poster.jpg"))));
        assert_eq!(Row::of(&m).poster_file.as_deref(), Some("edit-3-poster.jpg"));
        for elsewhere in ["rec/ZZZZZZZZZZZZZZZZ/poster.jpg".to_string(), format!("rec/{id}/v0/poster.jpg"), format!("rec/{id}/"), format!("rec/{id}/..")] {
            m.media = Some(media(Some(elsewhere.clone())));
            assert_eq!(Row::of(&m).poster_file, None, "{elsewhere}");
        }
        let mut list = shroom_edit::EditList::new(60.0);
        list.in_ = 8.0;
        m.edits = Some(shroom_edit::Saved::new(list.clone(), 1));
        assert_eq!(Row::of(&m).edited_sec, Some(52.0));
        list.in_ = 0.0;
        m.edits = Some(shroom_edit::Saved::new(list, 2));
        assert_eq!(Row::of(&m).edited_sec, None, "the identity is no edit");
        // A cache written before these fields still reads.
        let old: Row = serde_json::from_str(r#"{"id":"x","title":"T","state":"live","created_at":"2026-10-02T12:00:00Z","duration_sec":6.0,"owner":null,"poster":null}"#).unwrap();
        assert_eq!((old.edited_sec, old.poster_file), (None, None));
    }
}
