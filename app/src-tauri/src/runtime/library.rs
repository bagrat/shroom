//! The Library lane (ARCHITECTURE §13, N35): the space's records for the Library window,
//! and the two writes a row makes. Every command answers the core exactly once — its
//! result, or `Failed` with its class (a deadline the core keeps covers a lane that never
//! answers):
//!
//! - `Refresh`: the local cache (`library.json`) at once, then `meta/` listed with the
//!   ETag delta (`shroom-upload::library::refresh`) — `Rows { fresh: true }`;
//! - `Rename`: the record rewritten under `If-Match` — `Renamed`;
//! - `Delete`: per video, the tombstone first, then the bytes — `Deleted`;
//! - `Cleanup`: the token Mac's sweep (B91), handed at once to a lane of its own — a sweep
//!   reads the whole space and never holds the commands above up — `Cleaned`.
//!
//! A token Mac reaches the bucket the way it does everywhere else: the token's own derived
//! S3 credentials over the space's bucket (as the owner page's grant does). Those keys go
//! straight to the bucket, past the Cloudflare client's test fence, so a dev build checks
//! the fence itself: only a `shroom-test-…` bucket, or the L1 fake with `--cloud-fake`.
//!
//! Every other Mac (a member, an owner's Mac without the token — WORKER-INTERFACE §6.8)
//! asks its space's Worker under its device key: `POST /library` pages through the ids in
//! id order with this Mac's cached ETags (`mine` for a member, `everyone` for the owner's
//! Macs; each item's owner comes from the ledger, never the record), and a rename or delete
//! runs under a `manage` credential minted for that one recording (15 minutes, in memory
//! only). Nothing here is ever a secret in a log.

use std::sync::mpsc::{Sender, channel};

use std::collections::{BTreeMap, BTreeSet};

use shroom_cloud::device::DeviceKey;
use shroom_cloud::member::{Answer, LibraryRequest, MintNext, Purpose, Scope, Space};
use shroom_core::effect::LibCmd;
use shroom_core::event::{Event, LibraryNews};
use shroom_core::time::Wall;
use shroom_core::world::{FailClass, LibraryRow, Op, RecId, Role, SpaceIdentity};
use shroom_storage::s3::{Client, S3Error};
use shroom_upload::library::{self, Cache, Row};
use shroom_upload::meta::State;
use shroom_upload::upload::UploadError;

use super::Runtime;

/// `library.json`: `{v, space, cache}` — the cache belongs to one space (account/bucket);
/// another space's cache is never shown.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CacheFile {
    v: u32,
    space: String,
    cache: Cache,
    /// The ownership ledger's id → member map as this Mac learned it (a token Mac's: the
    /// `owners` scope, or `.shroom/registry/recs/<id>` over S3). Ledger entries never
    /// change, so it's kept for good (WORKER-INTERFACE §4.10).
    #[serde(default)]
    owners: BTreeMap<String, String>,
}

fn space_key(s: &SpaceIdentity) -> String {
    format!("{}/{}", s.account_id, s.bucket)
}

fn load_file(rt: &Runtime, space: &SpaceIdentity) -> CacheFile {
    rt.home
        .read_json::<CacheFile>("library.json")
        .filter(|f| f.v == 1 && f.space == space_key(space))
        .unwrap_or_else(|| CacheFile { v: 1, space: space_key(space), ..CacheFile::default() })
}

fn load(rt: &Runtime, space: &SpaceIdentity) -> Cache {
    load_file(rt, space).cache
}

/// Writes the cache — only while this Mac still knows the space the command began with
/// (`generation`): a refresh, rename or delete answering after Disconnect writes nothing.
/// The ledger's owners ride along unchanged unless given.
fn save(rt: &Runtime, space: &SpaceIdentity, cache: &Cache, generation: u64) -> bool {
    let owners = load_file(rt, space).owners;
    save_with(rt, space, cache, owners, generation)
}

fn save_with(rt: &Runtime, space: &SpaceIdentity, cache: &Cache, owners: BTreeMap<String, String>, generation: u64) -> bool {
    let now = rt.space_gen.lock().unwrap();
    if *now != generation {
        rt.log("library: the space was disconnected meanwhile — the cache isn't written");
        return false;
    }
    let _ = rt.home.write_json("library.json", &CacheFile { v: 1, space: space_key(space), cache: cache.clone(), owners });
    true
}

/// The rows the core gets: each record's owner from the ledger (a token Mac's learned map;
/// a device-door Mac's rows carry the ledger's owner already) — `None` until it's known —
/// and whether its poster is on this Mac.
fn rows_of(rt: &Runtime, space: &SpaceIdentity, file: &CacheFile) -> Vec<LibraryRow> {
    let token = super::uploader::token_door(rt, space).is_some();
    file.cache
        .visible()
        .into_iter()
        .map(|r| {
            let mut l = row(r);
            if token {
                l.owner = file.owners.get(&r.id).cloned();
            }
            l.poster = super::posters::cached(rt, &r.id);
            l
        })
        .collect()
}

/// How this Mac reaches the space's records — by token presence, as the uploader does.
enum Door {
    /// The token Mac: the bucket itself under the token's derived pair.
    Token(Client),
    /// Every other Mac: the space's Worker under this Mac's device key (§4.10, §6.8).
    Device { key: DeviceKey, page: Space },
}

/// A verified door, kept while the space and the token (or device key) stay the same.
struct Ctx {
    key: (String, String),
    door: Door,
}

fn ctx(rt: &Runtime, space: &SpaceIdentity, cached: &mut Option<Ctx>) -> Result<(), FailClass> {
    let Some(token) = super::uploader::token_door(rt, space) else {
        let dk = super::member::device_key(rt).filter(|k| k.host == space.host).ok_or_else(|| {
            rt.log("library: no device key for this Mac's space");
            FailClass::Refused
        })?;
        let key = (space_key(space), dk.device.clone());
        if cached.as_ref().is_some_and(|c| c.key == key) {
            return Ok(());
        }
        let page = rt.space(&space.host).ok_or_else(|| {
            rt.log("library: a dev build reaches a space's page only through --space-base — refused");
            FailClass::Refused
        })?;
        *cached = Some(Ctx { key, door: Door::Device { key: dk, page } });
        return Ok(());
    };
    // A dev build's S3 stays inside the test fence (the token's keys would pass it by).
    if !rt.bucket_allowed(&space.bucket) {
        rt.log("library: a dev build reaches only a shroom-test-… bucket (or the L1 fake) — refused");
        return Err(FailClass::Refused);
    }
    let key = (space_key(space), shroom_storage::sigv4::sha256_hex(token.as_bytes()));
    if cached.as_ref().is_some_and(|c| c.key == key) {
        return Ok(());
    }
    let cloud = rt.cloud(&token);
    let token_id = cloud.verify(&space.account_id).map_err(|e| super::cloud::class_of(&e))?.id;
    *cached = Some(Ctx { key, door: Door::Token(rt.token_bucket(&cloud, space, &token_id)) });
    Ok(())
}

/// The door to one recording's record and files, as a rename reaches them — the editor's
/// lane's (N63): the token Mac's own pair, verified once and kept; any other Mac a `manage`
/// credential minted for that one recording (15 minutes, in memory only).
#[derive(Default)]
pub(super) struct Doorway(Option<Ctx>);

impl Doorway {
    /// A client for `rec`'s record and files.
    pub(super) fn client(&mut self, rt: &Runtime, space: &SpaceIdentity, rec: &str) -> Result<Held<'_>, FailClass> {
        ctx(rt, space, &mut self.0)?;
        match &self.0.as_ref().expect("a door").door {
            Door::Token(s3) => Ok(Held::Shared(s3)),
            Door::Device { key, page } => manage(rt, space, key, page, rec).map(Held::Own),
        }
    }

    /// After a failure: the next call re-verifies (a revoked token, a new one).
    pub(super) fn forget(&mut self) {
        self.0 = None;
    }
}

/// A space's answer that isn't the one wanted, as a Library failure class.
fn answer_class<T>(rt: &Runtime, what: &str, a: &Answer<T>) -> FailClass {
    match a {
        Answer::Ok(_) => FailClass::Unclassified,
        // 503 reads offline-like: the cached list stays (§4.10).
        Answer::Refused(r) if r.status == 503 || r.is_transient() => FailClass::Transport,
        Answer::Refused(r) => {
            rt.log(&format!("library {what}: refused {} {:?}", r.status, r.class));
            FailClass::Refused
        }
        Answer::NotShroom { status } => {
            rt.log(&format!("library {what}: answered as no shroom page ({status})"));
            FailClass::Unclassified
        }
        Answer::Unanswered(_) => FailClass::Transport,
    }
}

/// A `manage` credential for one recording (§6.8): 15 minutes, never written down.
fn manage(rt: &Runtime, space: &SpaceIdentity, key: &DeviceKey, page: &Space, rec: &str) -> Result<Client, FailClass> {
    match MintNext::of(page.mint(key, rec, Purpose::Manage)) {
        MintNext::Minted(m) => Ok(super::uploader::client(rt, space, &m.cred_file(Purpose::Manage))),
        MintNext::Absent | MintNext::Backoff => Err(FailClass::Transport),
        other => {
            rt.log(&format!("library: the manage mint for {} was refused ({})", super::short(rec), match other {
                MintNext::Revoked => "this Mac isn't in the space",
                MintNext::NotYours => "not this member's video",
                MintNext::Deleted => "already deleted",
                MintNext::SpaceSide(_) => "the space's side",
                _ => "unexpected",
            }));
            Err(FailClass::Refused)
        }
    }
}

/// The Library's failure classes: a dropped connection is `Transport` (the offline face),
/// the store saying no is `Refused`, anything else unclassified. The editor's lane's too.
pub(super) fn class(e: &UploadError) -> FailClass {
    match e {
        UploadError::S3(S3Error::Transport(_)) => FailClass::Transport,
        UploadError::S3(S3Error::Status { status: 401 | 403 | 404, .. }) | UploadError::Tombstoned => FailClass::Refused,
        _ => FailClass::Unclassified,
    }
}

/// `2026-10-02T12:00:00Z` (fractional seconds allowed) → Unix milliseconds.
pub fn parse_iso(s: &str) -> Option<i64> {
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
    // Days from the civil date (Howard Hinnant's algorithm).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + h * 3_600 + mi * 60 + sec) * 1_000) + ms)
}

/// A record as the core knows a Library row (its poster as `rows_of` finds it), its edited
/// length when the record holds an edit that leaves something out (N63).
pub fn row(r: &Row) -> LibraryRow {
    LibraryRow {
        rec: RecId(r.id.clone()),
        title: r.title.clone(),
        created: Wall(parse_iso(&r.created_at).unwrap_or(0)),
        length_ms: r.duration_sec.filter(|s| s.is_finite() && *s >= 0.0).map(|s| (s * 1000.0).round() as u64),
        live: r.state == State::Live,
        owner: r.owner.clone(),
        poster: false,
        edited_ms: r.edited_sec.filter(|s| s.is_finite() && *s >= 0.0).map(|s| (s * 1000.0).round() as u64),
    }
}

/// The records this Mac's Library shows, as cached, each with its owner as the rows read it
/// (a token Mac's from the ledger it learned): what the editor's upkeep walks (B197 (d)).
pub(super) fn cached(rt: &Runtime, space: &SpaceIdentity) -> Vec<Row> {
    let file = load_file(rt, space);
    let token = super::uploader::token_door(rt, space).is_some();
    file.cache
        .visible()
        .into_iter()
        .map(|r| {
            let mut r = r.clone();
            if token {
                r.owner = file.owners.get(&r.id).cloned();
            }
            r
        })
        .collect()
}

/// What the core is told after a write or a refresh: the file as it now stands.
fn post_rows(rt: &Runtime, space: &SpaceIdentity, fresh: bool) {
    let file = load_file(rt, space);
    rt.post(Event::Library(LibraryNews::Rows { rows: rows_of(rt, space, &file), fresh }));
}

pub fn spawn(rt: Runtime) -> Sender<LibCmd> {
    let mut door: Option<Ctx> = None;
    spawn_with(rt, move |rt: &Runtime, space: &SpaceIdentity| sweep(rt, space, &mut door))
}

/// The sweep, behind its own verified door (B91): only the token Mac's pair lists `rec/`.
fn sweep(rt: &Runtime, space: &SpaceIdentity, door: &mut Option<Ctx>) -> Result<(), FailClass> {
    ctx(rt, space, door)?;
    let swept = match &door.as_ref().expect("a door").door {
        Door::Token(s3) => cleanup(rt, space, s3),
        Door::Device { .. } => {
            rt.log("library: the cleanup sweep is the token Mac's — refused");
            Err(FailClass::Refused)
        }
    };
    if swept.is_err() {
        // A failed sweep re-verifies next time (a revoked token, a new one).
        *door = None;
    }
    swept
}

/// The sweep's own lane (B91): on a real space it reads one record per recording — tens of
/// seconds for a hundred — so it never runs on the Library's lane, where a cache load, a
/// refresh, a rename or a delete would wait behind it. One sweep at a time; it answers only
/// when it fails (`Cleaned` comes from the sweep itself).
fn spawn_sweeper(rt: Runtime, mut sweep: impl FnMut(&Runtime, &SpaceIdentity) -> Result<(), FailClass> + Send + 'static) -> Sender<()> {
    let (tx, rx) = channel::<()>();
    std::thread::Builder::new()
        .name("shroom-cleanup".into())
        .spawn(move || {
            for () in rx {
                let Some(space) = rt.home.read_json::<SpaceIdentity>("space.json") else {
                    rt.log("library Cleanup: no space on this Mac");
                    rt.post(Event::Failed { op: Op::Cleanup, class: FailClass::Refused });
                    continue;
                };
                if let Err(class) = sweep(&rt, &space) {
                    rt.post(Event::Failed { op: Op::Cleanup, class });
                }
            }
        })
        .expect("the cleanup lane starts");
    tx
}

/// The Library lane, with the sweep handed to a lane of its own (`spawn_sweeper`).
fn spawn_with(rt: Runtime, sweep: impl FnMut(&Runtime, &SpaceIdentity) -> Result<(), FailClass> + Send + 'static) -> Sender<LibCmd> {
    let sweeper = spawn_sweeper(rt.clone(), sweep);
    let (tx, rx) = channel::<LibCmd>();
    std::thread::Builder::new()
        .name("shroom-library".into())
        .spawn(move || {
            let mut cached: Option<Ctx> = None;
            for cmd in rx {
                // The sweep never holds the Library's commands up: it goes to its own lane.
                if matches!(cmd, LibCmd::Cleanup) {
                    if sweeper.send(()).is_err() {
                        rt.post(Event::Failed { op: Op::Cleanup, class: FailClass::Unclassified });
                    }
                    continue;
                }
                let op = match &cmd {
                    LibCmd::LoadCache => Op::LibCache,
                    LibCmd::Refresh => Op::LibRefresh,
                    LibCmd::Rename(..) => Op::Rename,
                    LibCmd::Delete(_) => Op::Delete,
                    LibCmd::Cleanup => Op::Cleanup,
                };
                // Read before the space: a wipe between the two shows as a newer generation.
                let generation = *rt.space_gen.lock().unwrap();
                let Some(space) = rt.home.read_json::<SpaceIdentity>("space.json") else {
                    rt.log(&format!("library {op:?}: no space on this Mac"));
                    rt.post(Event::Failed { op, class: FailClass::Refused });
                    continue;
                };
                // What this Mac remembers answers at once, with no network at all — whatever
                // the reach (N35's offline face shows it, b4 #4).
                if matches!(cmd, LibCmd::LoadCache) {
                    post_rows(&rt, &space, false);
                    continue;
                }
                if let Err(class) = ctx(&rt, &space, &mut cached) {
                    rt.log(&format!("library {op:?}: no client ({class:?})"));
                    rt.post(Event::Failed { op, class });
                    continue;
                }
                let ok = match (&cached.as_ref().expect("a door").door, cmd) {
                    // Both answered above: a cache load at once, a sweep on its own lane.
                    (_, LibCmd::LoadCache | LibCmd::Cleanup) => Ok(()),
                    (Door::Token(s3), LibCmd::Refresh) => refresh(&rt, &space, s3, generation),
                    (Door::Token(s3), LibCmd::Rename(rec, title)) => rename(&rt, &space, s3, rec, &title, generation),
                    (Door::Token(s3), LibCmd::Delete(recs)) => delete(&rt, &space, |_| Ok(Held::Shared(s3)), |id| seal_here(s3, id), recs, generation),
                    (Door::Device { key, page }, LibCmd::Refresh) => refresh_device(&rt, &space, key, page, generation),
                    (Door::Device { key, page }, LibCmd::Rename(rec, title)) => {
                        manage(&rt, &space, key, page, &rec.0).and_then(|s3| rename(&rt, &space, &s3, rec, &title, generation))
                    }
                    (Door::Device { key, page }, LibCmd::Delete(recs)) => {
                        delete(&rt, &space, |rec| manage(&rt, &space, key, page, rec).map(Held::Own), |id| match page.gone(key, id) {
                            shroom_cloud::member::Answer::Ok(()) => Ok(()),
                            other => Err(format!("{other:?}")),
                        }, recs, generation)
                    }
                };
                if let Err(class) = ok {
                    // A failed call re-verifies next time (a revoked token, a new one).
                    cached = None;
                    rt.post(Event::Failed { op, class });
                }
            }
        })
        .expect("the library lane starts");
    tx
}

fn refresh(rt: &Runtime, space: &SpaceIdentity, s3: &Client, generation: u64) -> Result<(), FailClass> {
    let mut file = load_file(rt, space);
    match library::refresh(s3, &mut file.cache) {
        Ok(d) => {
            rt.log(&format!("library refreshed: {} new, {} changed, {} gone, {} read", d.added.len(), d.changed.len(), d.gone.len(), d.reads));
            // Who made each video: the ledger, never the record (§4.10, b4 #5).
            learn_owners(rt, space, s3, &mut file);
            save_with(rt, space, &file.cache, file.owners.clone(), generation);
            rt.post(Event::Library(LibraryNews::Rows { rows: rows_of(rt, space, &file), fresh: true }));
            if super::posters::fetch_missing(rt, space, Some(s3), &file.cache) {
                post_rows(rt, space, true);
            }
            Ok(())
        }
        Err(e) => {
            rt.log(&format!("library refresh: {e}"));
            Err(class(&e))
        }
    }
}

/// A Mac without the token (§4.10, §6.8): `POST /library` pages until `next` is null, each
/// call carrying the next slice (≤ 1,000 ids after `after`) of this Mac's cached ETags;
/// only changed records come back with a body, every item with its ledger owner. Gone =
/// the cache minus every id seen.
fn refresh_device(rt: &Runtime, space: &SpaceIdentity, key: &DeviceKey, page: &Space, generation: u64) -> Result<(), FailClass> {
    const SLICE: usize = 1000;
    let mut cache = load(rt, space);
    let scope = if space.role == Role::Member { Scope::Mine } else { Scope::Everyone };
    let ids: Vec<String> = cache.rows.keys().cloned().collect();
    let mut seen = BTreeSet::new();
    let (mut added, mut changed, mut calls) = (0usize, 0usize, 0usize);
    let mut after: Option<String> = None;
    loop {
        calls += 1;
        let start = after.as_ref().map_or(0, |a| ids.partition_point(|x| x <= a));
        let slice = &ids[start..(start + SLICE).min(ids.len())];
        let known: BTreeMap<String, String> = slice.iter().map(|id| (id.clone(), cache.rows[id].0.clone())).collect();
        let known_through = (start + slice.len() < ids.len()).then(|| slice.last().cloned()).flatten();
        let answer = page.library(key, &LibraryRequest { scope, known, known_through, after: after.clone() });
        let p = match answer {
            Answer::Ok(p) => p,
            other => return Err(answer_class(rt, "refresh", &other)),
        };
        for item in p.items {
            seen.insert(item.id.clone());
            let meta = item.meta.and_then(shroom_upload::meta::listed);
            match meta {
                Some(m) => {
                    // The ledger's owner, never the record's (member-writable, H3).
                    let row = Row { id: item.id.clone(), owner: Some(item.owner), ..Row::of(&m) };
                    if cache.rows.insert(item.id, (item.etag, row)).is_some() {
                        changed += 1;
                    } else {
                        added += 1;
                    }
                }
                None => {
                    if let Some((_, r)) = cache.rows.get_mut(&item.id) {
                        r.owner = Some(item.owner);
                    }
                }
            }
        }
        match p.next {
            // A page that doesn't move on would loop forever: treated as the end.
            Some(n) if after.as_deref() != Some(n.as_str()) && calls < 10_000 => after = Some(n),
            _ => break,
        }
    }
    let gone: Vec<String> = ids.into_iter().filter(|id| !seen.contains(id)).collect();
    for g in &gone {
        cache.rows.remove(g);
    }
    rt.log(&format!("library refreshed through the space ({scope:?}): {added} new, {changed} changed, {} gone, {calls} call(s)", gone.len()));
    save(rt, space, &cache, generation);
    post_rows(rt, space, true);
    if super::posters::fetch_missing(rt, space, None, &cache) {
        post_rows(rt, space, true);
    }
    Ok(())
}

/// A token Mac's owners, for the ids it doesn't know yet: the `owners` scope through the
/// space's Worker under this Mac's device key, paged; for whatever that leaves (no device
/// key, the Worker can't answer — the repair mood), each id's ledger entry over S3, once.
/// An id whose owner can't be read stays unknown: Everyone, unlabeled, never Mine.
fn learn_owners(rt: &Runtime, space: &SpaceIdentity, s3: &Client, file: &mut CacheFile) {
    let unknown = |f: &CacheFile| f.cache.rows.keys().filter(|id| !f.owners.contains_key(*id)).cloned().collect::<Vec<_>>();
    if unknown(file).is_empty() {
        return;
    }
    let door = super::member::device_key(rt).filter(|k| k.host == space.host).zip(rt.space(&space.host));
    if let Some((key, page)) = door {
        let mut after: Option<String> = None;
        for _ in 0..1_000 {
            match page.library(&key, &LibraryRequest { scope: Scope::Owners, known: BTreeMap::new(), known_through: None, after: after.clone() }) {
                Answer::Ok(p) => {
                    file.owners.extend(p.owners);
                    match p.next {
                        Some(n) if after.as_deref() != Some(n.as_str()) => after = Some(n),
                        _ => break,
                    }
                }
                other => {
                    let _ = answer_class(rt, "owners", &other);
                    break;
                }
            }
        }
    }
    let mut read = 0usize;
    for id in unknown(file) {
        if let Some(m) = ledger_owner(s3, &id) {
            file.owners.insert(id, m);
            read += 1;
        }
    }
    rt.log(&format!("library: owners from the ledger ({} known, {read} read directly)", file.owners.len()));
}

/// One ledger entry (`.shroom/registry/recs/<id>`, `{v, member, device, at}`): its member.
fn ledger_owner(s3: &Client, id: &str) -> Option<String> {
    let bytes = s3.get(&format!("{}{id}", shroom_cloud::registry::LEDGER)).ok()??;
    ledger_member(&bytes)
}

fn ledger_member(bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(bytes).ok()?.get("member")?.as_str().filter(|m| !m.is_empty()).map(String::from)
}

fn rename(rt: &Runtime, space: &SpaceIdentity, s3: &Client, rec: RecId, title: &str, generation: u64) -> Result<(), FailClass> {
    match library::rename(s3, &rec.0, title) {
        Ok(meta) => {
            let mut cache = load(rt, space);
            if let Some((etag, r)) = cache.rows.get_mut(&rec.0) {
                r.title = meta.title.clone();
                // The record changed under a new ETag: the next refresh reads it again.
                etag.clear();
            }
            save(rt, space, &cache, generation);
            rt.log(&format!("library renamed {}", super::short(&rec.0)));
            rt.post(Event::Library(LibraryNews::Renamed { rec, title: meta.title }));
            Ok(())
        }
        Err(e) => {
            rt.log(&format!("library rename {}: {e}", super::short(&rec.0)));
            Err(class(&e))
        }
    }
}

/// A delete counts only once its record reads as a tombstone (or is gone): should the
/// tombstone have lost every race to another writer, the row comes back and a retry
/// writes it — the link never stays un-removed while its bytes are gone.
fn tombstoned(s3: &Client, id: &str) -> Result<(), UploadError> {
    match s3.get(&shroom_upload::keys::meta(id)).map_err(UploadError::S3)? {
        None => Ok(()),
        Some(b) if serde_json::from_slice::<serde_json::Value>(&b).ok().is_some_and(|v| v["state"] == "deleted") => Ok(()),
        Some(_) => Err(UploadError::Incomplete("the record isn't a tombstone yet".into())),
    }
}

/// Each video in turn: its tombstone, then its bytes. The ones that landed are answered
/// even when a later one fails (they're gone everywhere), then the failure ends the op.
/// The client a delete runs under: the token Mac's one, or a video's own `manage` mint.
pub(super) enum Held<'a> {
    Shared(&'a Client),
    Own(Client),
}

impl std::ops::Deref for Held<'_> {
    type Target = Client;
    fn deref(&self) -> &Client {
        match self {
            Held::Shared(c) => c,
            Held::Own(c) => c,
        }
    }
}

/// A token Mac seals a delete itself: the deletion record (DECISIONS B125) is a `.shroom/`
/// key, which its own pair reaches and no recording credential does.
fn seal_here(s3: &Client, id: &str) -> Result<(), String> {
    let at = shroom_cloud::registry::iso(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let body = serde_json::json!({"v": 1, "by": "token", "at": at}).to_string();
    s3.put(&format!("{GONE}{id}"), body.as_bytes(), "application/json").map_err(|e| e.to_string())
}

/// Where the Worker keeps deletion records (worker.js `GONE`, DECISIONS B125).
const GONE: &str = ".shroom/gone/";

fn delete<'a>(
    rt: &Runtime,
    space: &SpaceIdentity,
    client: impl Fn(&str) -> Result<Held<'a>, FailClass>,
    seal: impl Fn(&str) -> Result<(), String>,
    recs: Vec<RecId>,
    generation: u64,
) -> Result<(), FailClass> {
    let mut done = Vec::new();
    let mut failed = None;
    for rec in recs {
        // A token Mac's one client; any other Mac's `manage` mint, one per video (§6.8).
        let s3 = match client(&rec.0) {
            Ok(c) => c,
            Err(class) => {
                failed = Some(class);
                break;
            }
        };
        match library::delete(&s3, &rec.0).and_then(|()| tombstoned(&s3, &rec.0)) {
            Ok(()) => {
                // Sealed where the video's own upload credential can't reach (B125). A space
                // whose page predates the record answers no route: logged, and the delete
                // still stands — the next page refresh brings the route.
                if let Err(e) = seal(&rec.0) {
                    rt.log(&format!("library delete {}: not sealed ({e})", super::short(&rec.0)));
                }
                done.push(rec)
            }
            Err(e) => {
                rt.log(&format!("library delete {}: {e}", super::short(&rec.0)));
                failed = Some(class(&e));
                break;
            }
        }
    }
    if !done.is_empty() {
        let mut cache = load(rt, space);
        for r in &done {
            cache.rows.remove(&r.0);
        }
        save(rt, space, &cache, generation);
        rt.log(&format!("library deleted {}", done.iter().map(|r| super::short(&r.0)).collect::<Vec<_>>().join(", ")));
        rt.post(Event::Library(LibraryNews::Deleted(done)));
    }
    match failed {
        Some(c) => Err(c),
        None => Ok(()),
    }
}

/// Every recording id a take folder on this Mac names — recording, paused, saved, queued,
/// uploaded or set aside alike (`take.json`, `publish.json`): law 2's line for the cleanup
/// sweep, which never takes the bytes of a recording this Mac still holds (B91).
pub fn held_ids(takes: &std::path::Path) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for dir in super::takes::dirs(takes) {
        for file in ["take.json", "publish.json"] {
            let rec = std::fs::read(dir.join(file)).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|v| v.get("rec")?.as_str().map(String::from));
            ids.extend(rec);
        }
    }
    ids
}

/// The token Mac's cleanup sweep (DECISIONS B91): over the token's own pair, sparing every
/// recording a take folder here names; counts only in the log.
fn cleanup(rt: &Runtime, space: &SpaceIdentity, s3: &Client) -> Result<(), FailClass> {
    let held = held_ids(&rt.home.takes());
    let device = super::member::device_key(rt).filter(|k| k.host == space.host).map(|k| k.device);
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let here = library::Here { now_ms, device: device.as_deref(), holds: &|id: &str| held.contains(id) };
    match library::cleanup(s3, &here) {
        Ok(c) => {
            rt.log(&format!(
                "library: cleanup reaped {} under tombstones, {} no Mac can finish, {} page keys ({} held here spared)",
                c.swept.under_tombstones.len(),
                c.swept.orphans.len(),
                c.keys,
                held.len()
            ));
            let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
            rt.post(Event::Library(LibraryNews::Cleaned { under_tombstones: n(c.swept.under_tombstones.len()), orphans: n(c.swept.orphans.len()), keys: n(c.keys) }));
            Ok(())
        }
        Err(e) => {
            rt.log(&format!("library cleanup: {e}"));
            Err(class(&e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B91 / law 2: every take folder's recording is spared by the sweep — the take being
    /// recorded, a saved one, a queued one, one uploaded, one set aside.
    #[test]
    fn every_take_this_mac_holds_is_spared_by_the_cleanup() {
        let tmp = tempfile::tempdir().unwrap();
        let takes = tmp.path();
        let dir = |name: &str, files: &[(&str, &str)]| {
            let d = takes.join(name);
            std::fs::create_dir_all(&d).unwrap();
            for (f, body) in files {
                std::fs::write(d.join(f), body).unwrap();
            }
        };
        dir("20261002-120000-0001", &[("take.json", r#"{"v":1,"rec":"RecordingNow0000","created":1}"#), ("LOCK", "")]);
        dir("20261002-120000-0002", &[("take.json", r#"{"v":1,"rec":"SavedTake0000000","created":1}"#), ("finalized.json", r#"{"v":1,"length_ms":5,"why":"interrupted"}"#)]);
        dir("20261002-120000-0003", &[("take.json", r#"{"v":1,"rec":"QueuedTake000000","created":1}"#), ("publish.json", r#"{"v":1,"rec":"QueuedTake000000","title":"t","stopped":1}"#)]);
        dir("20261002-120000-0004", &[("publish.json", r#"{"v":1,"rec":"UploadedTake0000","title":"t","stopped":1}"#), ("uploaded", "")]);
        dir("20261002-120000-0005", &[("take.json", r#"{"v":1,"rec":"SetAside00000000","created":1}"#), ("set-aside", "")]);
        dir("20261002-120000-0006", &[("take.json", "not json")]);
        let held = held_ids(takes);
        assert_eq!(
            held.into_iter().collect::<Vec<_>>(),
            ["QueuedTake000000", "RecordingNow0000", "SavedTake0000000", "SetAside00000000", "UploadedTake0000"]
        );
        assert!(held_ids(&takes.join("none")).is_empty());
    }

    /// B91: a sweep under way never holds the Library up — a cache load, a refresh, a
    /// rename and a delete each answer while the sweep is still running; the sweep answers
    /// on its own (only a failure, as `Failed { Cleanup }`), one at a time.
    #[test]
    fn library_commands_are_never_queued_behind_the_cleanup_sweep() {
        use std::sync::mpsc::{Receiver, RecvTimeoutError};
        use std::time::Duration;
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        let space = SpaceIdentity {
            account_id: "acct".into(),
            label: "v".into(),
            subdomain: "myshroom".into(),
            host: "v.myshroom.workers.dev".into(),
            bucket: "shroom-test-x".into(),
            role: Role::TokenHolder,
            member_id: "m1".into(),
            owner_name: "Bagrat".into(),
            account_name: None,
            foreign_v: false,
        };
        rt.home.write_json("space.json", &space).unwrap();
        let (started_tx, started) = channel::<()>();
        let (release, held) = channel::<Result<(), FailClass>>();
        let held = std::sync::Mutex::new(held);
        let lane = spawn_with(rt.clone(), move |_: &Runtime, _: &SpaceIdentity| {
            started_tx.send(()).unwrap();
            held.lock().unwrap().recv().unwrap()
        });
        let next = |rx: &Receiver<shroom_core::time::Envelope>| rx.recv_timeout(Duration::from_secs(2)).map(|e| e.event);

        lane.send(LibCmd::Cleanup).unwrap();
        started.recv_timeout(Duration::from_secs(2)).expect("the sweep starts");
        // The sweep is running (held) — every Library command still answers at once.
        lane.send(LibCmd::LoadCache).unwrap();
        assert!(matches!(next(&rx), Ok(Event::Library(LibraryNews::Rows { fresh: false, .. }))), "the cache answers behind no sweep");
        for (cmd, want) in [
            (LibCmd::Refresh, Op::LibRefresh),
            (LibCmd::Rename(RecId("r1".into()), "t".into()), Op::Rename),
            (LibCmd::Delete(vec![RecId("r1".into())]), Op::Delete),
        ] {
            lane.send(cmd).unwrap();
            // No door on this Mac (no token, no device key): refused — but answered now.
            match next(&rx) {
                Ok(Event::Failed { op, .. }) => assert_eq!(op, want),
                other => panic!("{want:?} waited behind the sweep: {other:?}"),
            }
        }
        // A second sweep asked meanwhile waits for the first (one at a time).
        lane.send(LibCmd::Cleanup).unwrap();
        assert!(started.recv_timeout(Duration::from_millis(300)).is_err(), "one sweep at a time");
        release.send(Ok(())).unwrap();
        started.recv_timeout(Duration::from_secs(2)).expect("the second sweep starts after the first");
        // A sweep that answered posts nothing of its own here (the sweep posts `Cleaned`).
        assert!(matches!(rx.recv_timeout(Duration::from_millis(200)), Err(RecvTimeoutError::Timeout)));
        release.send(Err(FailClass::Transport)).unwrap();
        match next(&rx) {
            Ok(Event::Failed { op: Op::Cleanup, class: FailClass::Transport }) => {}
            other => panic!("a failed sweep answers as Cleanup: {other:?}"),
        }
    }

    /// The real sweep on a Mac without the token is refused on its own lane (B91): with no
    /// space, and with a space but no token (a member's Mac, an owner's other Mac).
    #[test]
    fn a_mac_without_the_token_never_sweeps() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        let lane = spawn(rt.clone());
        let refused = || match rx.recv_timeout(std::time::Duration::from_secs(2)).map(|e| e.event) {
            Ok(Event::Failed { op: Op::Cleanup, class: FailClass::Refused }) => {}
            other => panic!("{other:?}"),
        };
        lane.send(LibCmd::Cleanup).unwrap();
        refused();
        let space = SpaceIdentity {
            account_id: "acct".into(),
            label: "v".into(),
            subdomain: "myshroom".into(),
            host: "v.myshroom.workers.dev".into(),
            bucket: "shroom-test-x".into(),
            role: Role::Member,
            member_id: "m2".into(),
            owner_name: "Bagrat".into(),
            account_name: None,
            foreign_v: false,
        };
        rt.home.write_json("space.json", &space).unwrap();
        lane.send(LibCmd::Cleanup).unwrap();
        refused();
    }

    #[test]
    fn iso_dates_read_as_unix_milliseconds() {
        assert_eq!(parse_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_iso("2026-10-02T12:00:00Z"), Some(1_790_942_400_000));
        assert_eq!(parse_iso("2026-10-02T12:00:00.25Z"), Some(1_790_942_400_250));
        assert_eq!(parse_iso("2024-02-29T23:59:59Z"), Some(1_709_251_199_000));
        for bad in ["", "2026-10-02", "2026-13-02T00:00:00Z", "2026-10-02T12:00:00", "x-y-zT1:2:3Z"] {
            assert_eq!(parse_iso(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_ledger_entry_names_its_member() {
        assert_eq!(ledger_member(br#"{"v":1,"member":"m_mher","device":"d_1","at":"2026-10-02T12:00:00Z"}"#).as_deref(), Some("m_mher"));
        assert_eq!(ledger_member(b"{}"), None);
        assert_eq!(ledger_member(br#"{"member":""}"#), None);
        assert_eq!(ledger_member(b"not json"), None);
    }

    /// A token Mac's rows take their owner from the ledger it learned — a forged
    /// `meta.owner` names nobody — and an unknown one stays unknown (b4 #5).
    #[test]
    fn a_token_macs_rows_key_on_the_ledger_never_the_record() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), Some(("http://127.0.0.1:1/client/v4".into(), "http://127.0.0.1:2".into())), None);
        let space = SpaceIdentity {
            account_id: "acct".into(),
            label: "v".into(),
            subdomain: "myshroom".into(),
            host: "v.myshroom.workers.dev".into(),
            bucket: "shroom".into(),
            role: Role::TokenHolder,
            member_id: "m1".into(),
            owner_name: "Bagrat".into(),
            account_name: None,
            foreign_v: false,
        };
        rt.home.write_json("token.json", &serde_json::json!({ "v": 1, "token": "t", "account_id": "acct" })).unwrap();
        let rec = |id: &str, forged: Option<&str>| Row { id: id.into(), title: id.into(), state: State::Live, created_at: "2026-10-02T12:00:00Z".into(), duration_sec: None, owner: forged.map(Into::into), poster: None, edited_sec: None, poster_file: None };
        let mut file = CacheFile { v: 1, space: space_key(&space), ..CacheFile::default() };
        file.cache.rows.insert("mine".into(), ("e".into(), rec("mine", None)));
        file.cache.rows.insert("forged".into(), ("e".into(), rec("forged", Some("m1"))));
        file.owners.insert("mine".into(), "m1".into());
        let rows = rows_of(&rt, &space, &file);
        let owner = |id: &str| rows.iter().find(|r| r.rec.0 == id).unwrap().owner.clone();
        assert_eq!(owner("mine").as_deref(), Some("m1"));
        assert_eq!(owner("forged"), None, "the record's own owner is never taken");
    }

    #[test]
    fn a_record_becomes_the_cores_row() {
        let r = Row {
            id: "AbCd".into(),
            title: "Where the 502s come from".into(),
            state: State::Live,
            created_at: "2026-08-21T12:00:00Z".into(),
            duration_sec: Some(272.4),
            owner: Some("m1".into()),
            poster: None,
            edited_sec: None,
            poster_file: None,
        };
        let l = row(&r);
        assert_eq!(l.rec, RecId("AbCd".into()));
        assert_eq!(l.length_ms, Some(272_400));
        assert_eq!(l.edited_ms, None);
        assert!(l.live);
        // An edit that leaves something out: the row reads its length after every refresh.
        assert_eq!(row(&Row { edited_sec: Some(221.04), ..r.clone() }).edited_ms, Some(221_040));
        assert_eq!(l.created, Wall(parse_iso("2026-08-21T12:00:00Z").unwrap()));
        assert!(!row(&Row { state: State::Uploading, duration_sec: Some(f64::NAN), ..r }).live);
    }
}
