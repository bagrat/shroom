//! The Library's data path (ARCHITECTURE §13, N35): the space's records listed with
//! `ListObjectsV2 prefix=meta/` and an ETag delta — only records that changed are read
//! again — plus the three writes a token-holding Mac makes from a row: rename (a record
//! rewrite), delete (a tombstone **first**, then the bytes — §5.5), and the clean delete
//! of a take that was never shared (no tombstone) — and the editor's two (N63): saving an
//! edit and naming its permanent copy. Every rewrite is read-modify-write under `If-Match`
//! (B21).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shroom_storage::s3::Client;

use crate::keys;
use crate::meta::{Meta, State, now_iso};
use crate::upload::{UploadError, update_meta, with_retry};

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
    pub poster: Option<String>,
}

impl Row {
    fn of(m: &Meta) -> Row {
        Row {
            id: m.id.clone(),
            title: m.title.clone(),
            state: m.state,
            created_at: m.created_at.clone(),
            duration_sec: m.duration_sec,
            // Member-writable: never trusted for ownership (§4.10).
            owner: None,
            poster: m.media.as_ref().and_then(|x| x.poster.clone()),
        }
    }
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

/// Removes these files from beside the recording, skipping any name that isn't a copy's.
/// Best effort: the record no longer names them, so one left behind is only bytes.
fn delete_copy_files(s3: &Client, id: &str, names: &[String]) {
    for name in names.iter().filter(|n| is_copy_file(n)) {
        let _ = with_retry(|| s3.delete(&format!("{}{name}", keys::prefix(id))));
    }
}

/// [`update_meta`] for an edit's write: a record that turned into a tombstone (which keeps no
/// title, so it never reads as a whole record) answers `Tombstoned`, not “unreadable”.
fn update_edit(s3: &Client, id: &str, change: impl FnMut(&mut Meta)) -> Result<Meta, UploadError> {
    update_meta(s3, id, change).map_err(|e| match e {
        UploadError::Incomplete(_) if tombstoned(s3, id).unwrap_or(false) => UploadError::Tombstoned,
        e => e,
    })
}

/// The revision an edit's record holds: its `edits.rev`, also when this version can't read
/// the rest of that `edits` (kept as it was in `extra`).
fn record_rev(m: &Meta) -> u64 {
    m.edits.as_ref().map(|s| s.rev).or_else(|| m.extra.get("edits").and_then(|e| e.get("rev")).and_then(serde_json::Value::as_u64)).unwrap_or(0)
}

/// Saves an edit (N63's Done; DECISIONS B195): the record's `edits` becomes `list` as the
/// next revision — `max(the record's, asked_rev - 1) + 1`, so a save from an editor that
/// opened on an older revision still lands after whatever another Mac saved since (the
/// last save wins). A copy cut from an earlier revision stays named, now stale: readers
/// ignore it until [`set_edit_copy`] names the new one. When `list` is the identity (nothing
/// cut, the whole recording) there's no copy to make: the old copy is dropped from the
/// record and then its files go. Read-modify-write under `If-Match`; a tombstone stops it.
/// Returns the revision saved.
pub fn save_edits(s3: &Client, id: &str, list: &shroom_edit::EditList, asked_rev: u64) -> Result<u64, UploadError> {
    let list = list.clone().checked();
    let identity = list.is_identity();
    let mut rev = 0;
    let mut dropped: Vec<String> = Vec::new();
    update_edit(s3, id, |m| {
        rev = record_rev(m).max(asked_rev.saturating_sub(1)) + 1;
        let before = m.edits.take();
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
        m.edits = Some(shroom_edit::Saved { copy, keep_until, extra, ..shroom_edit::Saved::new(list.clone(), rev) });
    })?;
    delete_copy_files(s3, id, &dropped);
    Ok(rev)
}

/// Names the permanent copy of a saved revision in the record (`edits.copy`), once its
/// files are up — only while the record still holds that revision. Returns `true` when it
/// did: the previous copy's files then go. `false` when the record moved on (a newer save,
/// another Mac's edit, no edit at all): nothing is written, and the files of `copy` that
/// were just sent are deleted here — never one the record still names. A tombstone stops it.
pub fn set_edit_copy(s3: &Client, id: &str, copy: &shroom_edit::EditCopy) -> Result<bool, UploadError> {
    let mine: Vec<String> = copy.files().into_iter().map(str::to_string).collect();
    if !mine.iter().all(|f| is_copy_file(f)) {
        return Err(UploadError::Incomplete(format!("not a copy's file names: {mine:?}")));
    }
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
        delete_copy_files(s3, id, &mine.into_iter().filter(|f| !keep.contains(f)).collect::<Vec<_>>());
        return Ok(false);
    }
    let mut current = false;
    let mut doomed: Vec<String> = Vec::new();
    let meta = update_edit(s3, id, |m| {
        doomed.clear();
        current = holds(m);
        let Some(saved) = m.edits.as_mut().filter(|_| current) else { return };
        if let Some(previous) = saved.copy.replace(copy.clone()) {
            doomed.extend(previous.files().into_iter().map(str::to_string).filter(|f| !mine.contains(f)));
        }
    })?;
    if !current {
        // It moved on between the look and the write.
        let keep = named(&meta);
        doomed = mine.into_iter().filter(|f| !keep.contains(f)).collect();
    }
    delete_copy_files(s3, id, &doomed);
    Ok(current)
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
pub fn sweep(s3: &Client, orphans_ok: &dyn Fn(&str) -> bool) -> Result<Swept, UploadError> {
    let mut ids = std::collections::BTreeSet::new();
    for o in with_retry(|| s3.list("rec/"))? {
        if let Some(id) = o.key.strip_prefix("rec/").and_then(|k| k.split('/').next()) {
            ids.insert(id.to_string());
        }
    }
    let mut out = Swept::default();
    for id in ids {
        match with_retry(|| s3.get(&keys::meta(&id)))? {
            Some(b) if serde_json::from_slice::<serde_json::Value>(&b).ok().is_some_and(|v| v["state"] == "deleted") => {
                remove_bytes(s3, &id)?;
                out.under_tombstones.push(id);
            }
            None if orphans_ok(&id) => {
                remove_bytes(s3, &id)?;
                out.orphans.push(id);
            }
            _ => {}
        }
    }
    Ok(out)
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
}

/// The token Mac's cleanup sweep (DECISIONS B91; ARCHITECTURE §5.5's GC, §16 #49):
/// bytes under a tombstone go (an interrupted delete, a late segment from another Mac);
/// bytes with no record go only once no Mac can still finish them (`reapable`); page keys
/// go once spent or an hour past their life. Records are never touched — a link that was
/// made keeps answering as it did.
pub fn cleanup(s3: &Client, here: &Here) -> Result<Cleaned, UploadError> {
    // An unreadable roster reaps no orphan at all (fail closed): who can still finish one
    // is the roster's to say.
    let roster: Option<serde_json::Value> = with_retry(|| s3.get(ROSTER))?.and_then(|b| serde_json::from_slice(&b).ok());
    let swept = sweep(s3, &|id: &str| {
        let ledger = s3.get(&format!("{LEDGER}{id}")).ok().flatten().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        reapable(id, ledger.as_ref(), roster.as_ref(), here)
    })?;
    let mut keys = 0;
    for o in with_retry(|| s3.list(KEYS))? {
        let Some(bytes) = with_retry(|| s3.get(&o.key))? else { continue };
        if key_done(&bytes, here.now_ms) {
            with_retry(|| s3.delete(&o.key))?;
            keys += 1;
        }
    }
    Ok(Cleaned { swept, keys })
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
}
