//! The uploader lane (ARCHITECTURE §9.1): segments of the open take go up while it records;
//! a published take (publish.json) goes up in full, then is marked `uploaded`. Credentials
//! are minted per recording (N4) and kept in the take's own `cred.json` (mode 600), so an
//! upload resumes after a relaunch within their lifetime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_cloud::Cloud;
use shroom_cloud::device::{CredFile, DeviceKey};
use shroom_cloud::member::{AiAuth, AiSent, AiSignal, MintNext, Ping, Purpose, Space, ai_log_lines, ai_signal_noted};
use shroom_cloud::mint::{self, Grant, Paths};
use shroom_core::effect::UpCmd;
use shroom_core::event::{Event, MemberNews, StandingAnswer, UploadNews};
use shroom_core::world::{Blocked, FailClass, Op, Role, SpaceIdentity, TakeDir};
use shroom_storage::s3::{Bucket, Client, S3Error};
use shroom_storage::sigv4::Credentials;
use shroom_upload::live::LiveUpload;
use shroom_upload::upload::{self, Beat, Local, UploadError};

use super::Runtime;
use super::recorder::LiveTake;
use super::takes::{Kind, classify, dirs};

/// Seven days — the most shroom allows (R2 sets no ceiling, B35 P6; N4); keeps N32's “a
/// video already on its way can still finish” true without re-minting.
const TTL_SECONDS: u64 = 604_800;
/// Discard's credential: long enough for one delete (§4.8's `manage` life).
const MANAGE_SECONDS: u64 = 900;

/// How this Mac gets a recording's credential — by token presence (#58, §6.7).
enum Door {
    /// The token's Mac: minted locally from the token's derived pair (B32, no REST call),
    /// the ownership ledger written create-only over S3 first (§6.7 step 2).
    Token { cloud: Cloud, token_id: String },
    /// Every other Mac: `POST /mint` under this Mac's device key (§4.8).
    Device { key: DeviceKey, page: Space },
}

struct Ctx {
    identity: SpaceIdentity,
    door: Door,
}

/// Why this Mac can't upload at all right now — classified, never a bare silence (the
/// whole-spec audit's #2, #19; DECISIONS B51).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtxFail {
    /// Not reachable now (a transport failure, Cloudflare busy): the next try waits its backoff.
    Offline,
    /// The space can't take videos — the token Cloudflare no longer accepts, or its account
    /// out of reach (N42's auth- and space-class failures): `space-degraded`.
    Space,
    /// Nothing this Mac can send through (no space, no device key, a dev build's fence).
    Nowhere,
}

fn ctx(rt: &Runtime) -> Result<Ctx, CtxFail> {
    let identity: SpaceIdentity = rt.home.read_json("space.json").ok_or(CtxFail::Nowhere)?;
    if let Some(token) = token_door(rt, &identity) {
        // The token's derived keys go straight to the bucket, past the Cloudflare client's
        // test fence: a dev build keeps the fence itself (TESTING-PLAN D3).
        if !rt.bucket_allowed(&identity.bucket) {
            rt.log("uploader: a dev build reaches only a shroom-test-… bucket (or the L1 fake) — refused");
            return Err(CtxFail::Nowhere);
        }
        let cloud = rt.cloud(&token);
        let token_id = match cloud.verify(&identity.account_id) {
            Ok(s) if s.status == "active" => s.id,
            Ok(s) => {
                rt.log(&format!("uploader: the token reads {:?} — the space can't take videos", s.status));
                return Err(CtxFail::Space);
            }
            Err(e) if shroom_cloud::sweep::token_refused(&e) => {
                rt.log("uploader: Cloudflare refused the token — the space can't take videos");
                return Err(CtxFail::Space);
            }
            Err(e) => {
                rt.log(&format!("uploader: the token's check didn't get through ({})", e.to_string().chars().take(120).collect::<String>()));
                return Err(CtxFail::Offline);
            }
        };
        return Ok(Ctx { identity, door: Door::Token { cloud, token_id } });
    }
    let key = super::member::device_key(rt).filter(|k| k.host == identity.host).ok_or(CtxFail::Nowhere)?;
    let Some(page) = rt.space(&identity.host) else {
        rt.log("uploader: a dev build mints only through --space-base — refused");
        return Err(CtxFail::Nowhere);
    };
    Ok(Ctx { identity, door: Door::Device { key, page } })
}

/// The oldest published take still to go up, if any.
fn head_take(rt: &Runtime) -> Option<shroom_core::world::Queued> {
    dirs(&rt.home.takes()).into_iter().find_map(|d| match classify(&d) {
        Kind::Queued(q) => Some(q),
        _ => None,
    })
}

/// No context: the core hears why, about the take that waits on it — so the space-class
/// answer raises `space-degraded` and every answer waits its backoff (never a Kick loop).
fn report_no_ctx(rt: &Runtime, why: CtxFail) {
    let head = head_take(rt);
    match (why, head) {
        (CtxFail::Space, Some(q)) => rt.post(Event::Upload(UploadNews::Blocked(q.dir, Blocked::Space))),
        (CtxFail::Offline, Some(q)) => rt.post(Event::Upload(UploadNews::Blocked(q.dir, Blocked::Offline))),
        (CtxFail::Offline, None) => rt.post(Event::Failed { op: Op::Upload, class: FailClass::Transport }),
        (CtxFail::Space | CtxFail::Nowhere, _) => rt.post(Event::Failed { op: Op::Upload, class: FailClass::Refused }),
    }
}

/// The token, when this Mac mints from it: it holds its space's token (the identity's
/// role) or has one pasted. The dev home's harness token (`Home::token`'s fallback) counts
/// only for a token Mac's identity — a dev build joined as a member mints through `/mint`.
pub(crate) fn token_door(rt: &Runtime, identity: &SpaceIdentity) -> Option<String> {
    (identity.role == Role::TokenHolder || rt.home.saved_token().is_some()).then(|| rt.home.token()).flatten()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Why a recording has no credential now.
enum NoCred {
    /// Not reachable now: the next pass tries again.
    Offline,
    /// The token Mac's ledger write was refused by the bucket itself (gone, or the token's
    /// keys no longer take): the space can't take videos.
    Space,
    /// The space said something about this Mac (`/me`'s table, §6.7): the core hears it.
    Standing(StandingAnswer),
    /// The ledger names someone else (a logged anomaly): never re-minted, the take is kept.
    NotYours,
    /// Deleted from the space before the bytes went up.
    Deleted,
}

/// A recording's credential: the one in its take (`cred.json`) while it has more than an
/// hour left, else a fresh mint written there first. `fresh`: minted this call. Without a
/// take (`dir: None`, Discard) the credential stays in memory.
fn creds_for(rt: &Runtime, c: &Ctx, dir: Option<&Path>, rec: &str, purpose: Purpose) -> Result<(Client, bool), NoCred> {
    let path = dir.map(|d| d.join("cred.json"));
    if let Some(path) = &path {
        if let Some(f) = super::home::read_json::<CredFile>(path) {
            let left = super::member::iso_ms(&f.expires_at).unwrap_or(0) - now_secs() as i64 * 1000;
            if f.purpose == "upload" && left > 3_600_000 {
                return Ok((client(rt, &c.identity, &f), false));
            }
        }
    }
    let f = match &c.door {
        Door::Token { cloud, token_id } => mint_locally(rt, c, cloud, token_id, rec, purpose)?,
        Door::Device { key, page } => match MintNext::of(page.mint(key, rec, purpose)) {
            MintNext::Minted(m) => m.cred_file(purpose),
            MintNext::Revoked => return Err(NoCred::Standing(StandingAnswer::Revoked)),
            MintNext::SpaceSide(_) => return Err(NoCred::Standing(StandingAnswer::SpaceSide)),
            MintNext::NotYours => return Err(NoCred::NotYours),
            MintNext::Deleted => return Err(NoCred::Deleted),
            MintNext::Absent | MintNext::Backoff => return Err(NoCred::Offline),
            MintNext::Reping => match page.ping() {
                Ping::NotShroom { .. } => return Err(NoCred::Standing(StandingAnswer::Gone)),
                _ => return Err(NoCred::Offline),
            },
            MintNext::Bug(r) => {
                rt.log(&format!("uploader: mint refused {} {:?}", r.status, r.class));
                return Err(NoCred::Offline);
            }
        },
    };
    if let (Some(dir), Some(path)) = (dir, &path) {
        let _ = super::home::write_json(path, &f);
        // The core learns of it at once, not at the next scan: a space that stops minting
        // for this Mac still lets this take finish (N32, whole-spec audit #22).
        if let Some(until) = super::takes::minted_until(dir) {
            rt.post(Event::Upload(UploadNews::Minted { dir: take_dir(dir), until: Some(until) }));
        }
    }
    Ok((client(rt, &c.identity, &f), true))
}

fn take_dir(dir: &Path) -> TakeDir {
    TakeDir(dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

/// The token Mac's mint (B32): its ledger entry first — create-only, its member in
/// `x-amz-meta-m` (§2.4) — then an HS256 credential for this recording alone, signed
/// locally with the token's derived pair.
fn mint_locally(rt: &Runtime, c: &Ctx, cloud: &Cloud, token_id: &str, rec: &str, purpose: Purpose) -> Result<CredFile, NoCred> {
    let id = &c.identity;
    let parent = cloud.s3_credentials(token_id);
    let now = now_secs();
    if purpose == Purpose::Upload {
        let s3 = rt.token_bucket(cloud, id, token_id);
        let device = super::member::device_key(rt).filter(|k| k.host == id.host).map(|k| k.device);
        let entry = serde_json::json!({ "v": 1, "member": id.member_id, "device": device, "at": shroom_cloud::registry::iso(now) });
        let key = format!("{}{rec}", shroom_cloud::registry::LEDGER);
        if let Err(e) = s3.put_if_absent_with_meta(&key, entry.to_string().as_bytes(), "application/json", &[("m", &id.member_id)]) {
            rt.log(&format!("uploader: the ledger entry for {} didn't land ({e})", super::short(rec)));
            // `NoSuchBucket` (404) or a refused key (401/403) is the space's own answer.
            return Err(match e {
                S3Error::Status { status: 401 | 403 | 404, .. } => NoCred::Space,
                _ => NoCred::Offline,
            });
        }
    }
    let ttl = if purpose == Purpose::Upload { TTL_SECONDS } else { MANAGE_SECONDS };
    let grant = Grant {
        account_id: &id.account_id,
        bucket: &id.bucket,
        scope: "object-read-write",
        paths: Paths { prefixes: vec![shroom_upload::keys::prefix(rec)], objects: vec![shroom_upload::keys::meta(rec)] },
        issued_at: now,
        ttl_seconds: ttl,
    };
    let creds = mint::mint(&parent, &grant);
    Ok(CredFile {
        v: 1,
        purpose: purpose.as_str().into(),
        access_key_id: creds.access_key_id,
        secret_access_key: creds.secret_access_key,
        session_token: creds.session_token.unwrap_or_default(),
        expires_at: shroom_cloud::registry::iso(now + ttl),
        bucket: id.bucket.clone(),
        endpoint: cloud.r2_endpoint(&id.account_id),
    })
}

pub(crate) fn client(rt: &Runtime, identity: &SpaceIdentity, f: &CredFile) -> Client {
    Client::new(Bucket {
        endpoint: rt.s3_endpoint(&identity.host, &f.endpoint),
        region: "auto".into(),
        name: f.bucket.clone(),
        credentials: Credentials { access_key_id: f.access_key_id.clone(), secret_access_key: f.secret_access_key.clone(), session_token: Some(f.session_token.clone()) },
    })
}

/// A recording that can't go up now: what the core hears, and the pass ends.
fn stop(rt: &Runtime, dir: &Path, q: &shroom_core::world::Queued, why: NoCred) {
    match why {
        NoCred::Offline => rt.post(Event::Upload(UploadNews::Blocked(q.dir.clone(), Blocked::Offline))),
        NoCred::Space => rt.post(Event::Upload(UploadNews::Blocked(q.dir.clone(), Blocked::Space))),
        NoCred::Standing(a) => rt.post(Event::Member(MemberNews::Standing(a))),
        NoCred::NotYours => {
            rt.log(&format!("uploader: the space's ledger names someone else for {} — kept here, never re-minted", super::short(&q.rec.0)));
            rt.post(Event::Upload(UploadNews::Blocked(q.dir.clone(), Blocked::Space)));
        }
        NoCred::Deleted => {
            let _ = std::fs::write(dir.join("tombstoned"), b"");
            rt.post(Event::Upload(UploadNews::Tombstoned(q.dir.clone())));
        }
    }
    rt.post(Event::Upload(UploadNews::Idle));
}

pub fn spawn(rt: Runtime, live: Arc<Mutex<Option<LiveTake>>>) -> Sender<UpCmd> {
    let (tx, rx) = channel::<UpCmd>();
    std::thread::Builder::new()
        .name("shroom-uploader".into())
        .spawn(move || {
            let mut lives: HashMap<PathBuf, LiveUpload> = HashMap::new();
            let mut naming = Naming::new(&rt);
            let mut cached: Option<Ctx> = None;
            let mut working = false;
            // Discards whose remote delete hasn't landed (`forget/<rec>`): retried on a
            // backoff, from a minute after launch (whole-spec audit #13, DECISIONS B52).
            let mut forgets = Forgets::at_launch(&rt);
            loop {
                // While a take is open (or work remains), pass every 2 s; a new Kick wakes
                // it at once — never a blind sleep in front of fresh work.
                let wait = [working.then_some(Duration::from_secs(2)), forgets.wait()].into_iter().flatten().min();
                let msg = match wait {
                    Some(t) => match rx.recv_timeout(t) {
                        Ok(m) => Some(m),
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                        Err(_) => return,
                    },
                    None => match rx.recv() {
                        Ok(m) => Some(m),
                        Err(_) => return,
                    },
                };
                if forgets.due() {
                    forgets.retry(&rt);
                }
                match msg {
                    Some(UpCmd::ForgetRemote(rec)) => forgets.forget(&rt, &rec.0),
                    // A timeout only for the forgets' backoff: no pass is owed.
                    None if !working => {}
                    Some(UpCmd::Kick) | None => {
                        // The verified context is reused; a failed pass re-verifies.
                        if cached.is_none() {
                            match ctx(&rt) {
                                Ok(c) => cached = Some(c),
                                Err(why) => {
                                    report_no_ctx(&rt, why);
                                    working = false;
                                    continue;
                                }
                            }
                        }
                        let (more, ok) = pass(&rt, cached.as_ref().expect("set above"), &live, &mut lives, &mut naming);
                        working = more;
                        if !ok {
                            cached = None;
                        }
                    }
                }
            }
        })
        .expect("the uploader lane starts");
    tx
}

/// Whether the take holds an upload credential with more than an hour left — one a space
/// that refuses new mints still honors (N32: “a video already on its way can still finish”).
fn has_live_cred(dir: &Path) -> bool {
    super::home::read_json::<CredFile>(&dir.join("cred.json")).is_some_and(|f| {
        f.purpose == "upload" && super::member::iso_ms(&f.expires_at).unwrap_or(0) - now_secs() as i64 * 1000 > 3_600_000
    })
}

/// One pass over the open take and the published ones. Returns whether work remains
/// (a take still open) — otherwise it reports Idle, ending the core's upload op — and
/// whether the pass went cleanly.
fn pass(rt: &Runtime, c: &Ctx, live: &Arc<Mutex<Option<LiveTake>>>, lives: &mut HashMap<PathBuf, LiveUpload>, naming: &mut Naming) -> (bool, bool) {
    // Takes whose space hasn't yet answered that they're done (N66): said again now and then.
    naming.owed(rt, c);
    // The open take: its finished segments, as they finish. No meta claim — a recording
    // becomes a link only by Stop or Get link (law 3, DEVIATIONS #4).
    let open = live.lock().unwrap().clone();
    if let Some(l) = &open {
        if !lives.contains_key(&l.dir) {
            if let Ok((s3, _)) = creds_for(rt, c, Some(&l.dir), &l.rec, Purpose::Upload) {
                lives.insert(l.dir.clone(), LiveUpload::new(s3, &l.rec, &l.dir));
            }
        }
        if let Some(up) = lives.get_mut(&l.dir) {
            if let Ok(n) = up.tick(true) {
                if n > 0 {
                    rt.log(&format!("uploader sent {n} while recording"));
                }
            }
            // Another ten pieces up: another minute for the space to hear (N66).
            naming.minute(c, &l.dir, &l.rec, up.pieces_up());
        }
        let name = TakeDir(l.dir.file_name().unwrap().to_string_lossy().into_owned());
        rt.post(Event::Upload(UploadNews::Progress { dir: name, done: 0, total: 0 }));
    }
    // The space refused this Mac a new credential (removed, or gone): takes already holding
    // one still finish; the rest wait (N32, whole-spec audit #22).
    let mut no_mint = false;
    // Published takes, oldest first.
    for dir in dirs(&rt.home.takes()) {
        let Kind::Queued(q) = classify(&dir) else { continue };
        if no_mint && !has_live_cred(&dir) {
            continue;
        }
        let (s3, fresh) = match creds_for(rt, c, Some(&dir), &q.rec.0, Purpose::Upload) {
            Ok(t) => t,
            Err(NoCred::Standing(a @ (StandingAnswer::Revoked | StandingAnswer::Gone))) => {
                rt.post(Event::Member(MemberNews::Standing(a)));
                no_mint = true;
                continue;
            }
            Err(why) => {
                stop(rt, &dir, &q, why);
                return (false, false);
            }
        };
        let already = lives.get(&dir).map(|u| u.sent().clone()).unwrap_or_default();
        rt.log(&format!("uploader publishing {} ({} already up)", q.dir.0, already.len()));
        match publish(rt, &s3, &dir, &q, &already) {
            Ok(()) => {
                let _ = std::fs::write(dir.join("uploaded"), b"");
                lives.remove(&dir);
                // Every piece is up: the space hears the rest and names it (N66).
                naming.done(c, &dir, &q.rec.0);
                rt.post(Event::Upload(UploadNews::Live(q.dir)));
            }
            Err(UploadError::Tombstoned) => {
                let _ = std::fs::write(dir.join("tombstoned"), b"");
                rt.post(Event::Upload(UploadNews::Tombstoned(q.dir)));
            }
            Err(e) => {
                rt.log(&format!("uploader {}: {e}", q.dir.0));
                let refused = matches!(e, UploadError::S3(S3Error::Status { status: 403 | 404, .. }));
                let blocked = match (&c.door, refused) {
                    // A fresh credential refused on its first PUT is the space's own side
                    // (§6.7 step 5) — debounced by the core (C7), never a stranded take.
                    (Door::Device { .. }, true) if fresh => {
                        rt.post(Event::Member(MemberNews::Standing(StandingAnswer::SpaceSide)));
                        Blocked::Offline
                    }
                    // An older one: minted again on the next pass.
                    (Door::Device { .. }, true) => {
                        let _ = std::fs::remove_file(dir.join("cred.json"));
                        rt.post(Event::Upload(UploadNews::Minted { dir: q.dir.clone(), until: None }));
                        Blocked::Offline
                    }
                    (Door::Token { .. }, true) => Blocked::Space,
                    _ => Blocked::Offline,
                };
                rt.post(Event::Upload(UploadNews::Blocked(q.dir, blocked)));
                rt.post(Event::Upload(UploadNews::Idle));
                return (false, false);
            }
        }
    }
    if open.is_some() {
        return (true, true);
    }
    rt.post(Event::Upload(UploadNews::Idle));
    (false, true)
}

/// A take whose space hasn't yet answered that it's done (N66): `ai-owed` beside it.
const AI_OWED: &str = "ai-owed";
/// How often an owed done is said again: the space's own schedule does the rest once it's in.
const AI_AGAIN: Duration = Duration::from_secs(600);

/// One word for the space about a take (N66, DECISIONS B206 b).
struct AiJob {
    host: String,
    door: AiDoor,
    rec: String,
    /// The take, for its owed marker and the log (never the link's id).
    dir: PathBuf,
    signal: AiSignal,
}

/// The door naming's words go through: this Mac's device key, or the setup Mac's parent
/// proof — the token's derived S3 secret, as enroll signs with it.
enum AiDoor {
    Device(DeviceKey),
    Parent(String),
}

/// What the uploader keeps for naming: its own lane — one word at a time, off the
/// uploader's thread, since a step can take half a minute and the upload never waits on it —
/// how far each open take was told, and when an owed done was last said.
struct Naming {
    lane: Sender<AiJob>,
    told: HashMap<PathBuf, u32>,
    tried: HashMap<PathBuf, Instant>,
}

impl Naming {
    fn new(rt: &Runtime) -> Naming {
        Naming { lane: ai_lane(rt.clone()), told: HashMap::new(), tried: HashMap::new() }
    }

    fn door(c: &Ctx) -> AiDoor {
        match &c.door {
            Door::Device { key, .. } => AiDoor::Device(key.clone()),
            Door::Token { cloud, token_id } => AiDoor::Parent(cloud.s3_credentials(token_id).secret_access_key),
        }
    }

    /// The open take has another ten pieces up: another minute to hear.
    fn minute(&mut self, c: &Ctx, dir: &Path, rec: &str, pieces: u32) {
        let told = self.told.get(dir).copied().unwrap_or(0);
        if pieces / 10 > told / 10 {
            self.told.insert(dir.to_path_buf(), pieces);
            self.send(c, dir, rec, AiSignal::Through(pieces));
        }
    }

    /// The take is live: owed until the space answers.
    fn done(&mut self, c: &Ctx, dir: &Path, rec: &str) {
        self.told.remove(dir);
        let _ = std::fs::write(dir.join(AI_OWED), b"");
        self.tried.insert(dir.to_path_buf(), Instant::now());
        self.send(c, dir, rec, AiSignal::Done);
    }

    /// Owed dones, said again — from launch, then no oftener than every ten minutes a take.
    fn owed(&mut self, rt: &Runtime, c: &Ctx) {
        for dir in dirs(&rt.home.takes()) {
            if !dir.join(AI_OWED).is_file() || self.tried.get(&dir).is_some_and(|t| t.elapsed() < AI_AGAIN) {
                continue;
            }
            let Some(p) = super::home::read_json::<super::takes::PublishJson>(&dir.join("publish.json")) else { continue };
            self.tried.insert(dir.clone(), Instant::now());
            self.send(c, &dir, &p.rec, AiSignal::Done);
        }
    }

    fn send(&self, c: &Ctx, dir: &Path, rec: &str, signal: AiSignal) {
        let _ = self.lane.send(AiJob { host: c.identity.host.clone(), door: Self::door(c), rec: rec.to_string(), dir: dir.to_path_buf(), signal });
    }
}

/// Naming's lane: each word in turn. A take whose space doesn't name videos is left be; a
/// done take's marker goes once the space has answered at all.
fn ai_lane(rt: Runtime) -> Sender<AiJob> {
    let (tx, rx) = channel::<AiJob>();
    std::thread::Builder::new()
        .name("shroom-naming".into())
        .spawn(move || {
            let mut off: std::collections::HashSet<String> = std::collections::HashSet::new();
            for job in rx {
                let take = job.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let sent = if off.contains(&job.rec) {
                    AiSent::Off
                } else if let Some(space) = rt.space(&job.host) {
                    let auth = match &job.door {
                        AiDoor::Device(k) => AiAuth::Device(k),
                        AiDoor::Parent(secret) => AiAuth::Parent(secret),
                    };
                    // Each step's note from the space (B206 f): what a report shows of it.
                    let (sent, notes) = ai_signal_noted(&space, &auth, &job.rec, job.signal);
                    for line in ai_log_lines(&take, job.signal, sent, &notes) {
                        rt.log(&line);
                    }
                    sent
                } else {
                    continue;
                };
                if sent == AiSent::Off {
                    off.insert(job.rec.clone());
                }
                if job.signal == AiSignal::Done && sent != AiSent::Owed {
                    let _ = std::fs::remove_file(job.dir.join(AI_OWED));
                }
            }
        })
        .expect("the naming lane starts");
    tx
}

/// `live`: the stream went live on an earlier pass, with this count — a resume sends only
/// the second-class files.
#[derive(serde::Serialize, serde::Deserialize)]
struct LiveMarker {
    stream: u32,
    total: u32,
}

/// One published take, in the order the page needs it (whole-spec audit #17, #18;
/// DECISIONS B49): the record's claim first — before any ffmpeg work, so the owner page
/// finds its title field and Copy link the moment it opens — then the poster (from the
/// first take's preview when the single-file copy isn't made yet), the stream and the
/// go-live; only then the second-class files are made and sent (the single-file copy, the
/// teaser), so a slow remux never holds the go-live. `progress` reaches the core and,
/// rate-limited, the record (the page's percentage and stall clock).
fn publish(rt: &Runtime, s3: &Client, dir: &Path, q: &shroom_core::world::Queued, already: &HashMap<String, u64>) -> Result<(), UploadError> {
    use shroom_encode::{files, take};
    let ffmpeg = super::locate_ffmpeg();
    let beat = Beat::default();
    let name = q.dir.clone();
    let progress = |done: u32, total: u32| rt.post(Event::Upload(UploadNews::Progress { dir: name.clone(), done, total }));
    let started = Instant::now();
    let marker: Option<LiveMarker> = super::home::read_json(&dir.join("live"));
    let (stream, total) = match marker {
        Some(m) => (m.stream, m.total),
        None => {
            let local = Local::read(dir)?;
            // The second-class files this pass will send: both when ffmpeg can make them,
            // else whichever are already here.
            let extras = if ffmpeg.is_some() { 2 } else { [files::TEASER, files::PREVIEW].iter().filter(|f| dir.join(f).is_file()).count() as u32 };
            let total = local.stream_files() + extras;
            upload::claim(s3, &q.rec.0, &q.title, &upload::local_markers(dir), total)?;
            let _ = std::fs::write(dir.join("claimed"), b"");
            if let Some(ff) = &ffmpeg {
                if !dir.join(files::POSTER).exists() {
                    let _ = take::poster(ff, dir);
                }
            }
            let r = upload::go_live(s3, &q.rec.0, dir, &local, already, extras, &beat, progress)?;
            rt.log(&format!("uploader live in {} ms ({} files, {} bytes)", r.live_ms, r.uploaded, r.bytes));
            let m = LiveMarker { stream: local.stream_files(), total };
            let _ = super::home::write_json(&dir.join("live"), &m);
            (m.stream, m.total)
        }
    };
    if let Some(ff) = &ffmpeg {
        if !dir.join(files::PREVIEW).exists() {
            let _ = take::assemble_preview(ff, dir, &take::take_numbers(dir));
        }
        if !dir.join(files::TEASER).exists() && dir.join(files::PREVIEW).exists() {
            let _ = take::teaser(ff, dir);
        }
    }
    // Live, its sound on this Mac: the take's voice cuts are looked for now (N65), beside
    // the second-class files, which never wait on them.
    super::voice::after_go_live(rt, dir, &q.rec.0);
    let mp4 = upload::finish(s3, &q.rec.0, dir, stream, total, &beat, progress)?;
    rt.log(&format!("uploader drained in {} ms (single-file copy: {mp4})", started.elapsed().as_millis()));
    Ok(())
}

/// The discards whose remote delete still owes (N23's clean delete, retried until it lands —
/// an offline Discard never leaves the bytes behind for good).
struct Forgets {
    next: Option<Instant>,
    delay: Duration,
}

/// The first retry, and the most a retry waits.
const FORGET_FIRST: Duration = Duration::from_secs(60);
const FORGET_MAX: Duration = Duration::from_secs(1800);

impl Forgets {
    fn at_launch(rt: &Runtime) -> Forgets {
        let pending = !pending_forgets(&rt.home.forgets()).is_empty();
        Forgets { next: pending.then(|| Instant::now() + FORGET_FIRST), delay: FORGET_FIRST }
    }

    fn wait(&self) -> Option<Duration> {
        self.next.map(|t| t.saturating_duration_since(Instant::now()))
    }

    fn due(&self) -> bool {
        self.next.is_some_and(|t| Instant::now() >= t)
    }

    /// Discard's delete: written down first, then tried once now.
    fn forget(&mut self, rt: &Runtime, rec: &str) {
        let dir = rt.home.forgets();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(rec), b"");
        if !try_forget(rt, rec) && self.next.is_none() {
            self.delay = FORGET_FIRST;
            self.next = Some(Instant::now() + self.delay);
        }
    }

    fn retry(&mut self, rt: &Runtime) {
        let left = pending_forgets(&rt.home.forgets()).into_iter().filter(|rec| !try_forget(rt, rec)).count();
        if left == 0 {
            self.next = None;
            self.delay = FORGET_FIRST;
        } else {
            self.delay = (self.delay * 2).min(FORGET_MAX);
            self.next = Some(Instant::now() + self.delay);
        }
    }
}

/// The recording ids whose discard is still owed (`forget/<rec>`).
fn pending_forgets(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str().map(String::from)).filter(|n| n.len() == 16).collect())
        .unwrap_or_default()
}

/// Discard's clean delete of whatever already went up (N23: no tombstone). A token Mac
/// mints a short credential locally; any other Mac re-mints `upload` for the same id (the
/// ledger matches; `manage` needs a record a never-shared take lacks, §6.8).
/// Returns whether it's behind us (its `forget/<rec>` marker removed): deleted, or nothing
/// of it was ever up — a space this Mac no longer has, or one that says the id isn't its.
fn try_forget(rt: &Runtime, rec: &str) -> bool {
    let done = |rt: &Runtime| {
        let _ = std::fs::remove_file(rt.home.forgets().join(rec));
        true
    };
    let c = match ctx(rt) {
        Ok(c) => c,
        Err(CtxFail::Nowhere) => return done(rt),
        Err(_) => return false,
    };
    let purpose = if matches!(c.door, Door::Token { .. }) { Purpose::Manage } else { Purpose::Upload };
    let s3 = match creds_for(rt, &c, None, rec, purpose) {
        Ok((s3, _)) => s3,
        // The space says it isn't this Mac's to touch, or it's already gone from it.
        Err(NoCred::NotYours | NoCred::Deleted) => return done(rt),
        Err(_) => return false,
    };
    // Its bytes and its claimed record, no tombstone (§5.5 — nobody ever had the link).
    match shroom_upload::library::discard(&s3, rec) {
        Ok(_) => done(rt),
        Err(e) => {
            rt.log(&format!("uploader: discard of {} left something behind ({e}) — tried again later", super::short(rec)));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cred(dir: &Path, purpose: &str, expires_in_s: i64) {
        let f = CredFile {
            v: 1,
            purpose: purpose.into(),
            access_key_id: "k".into(),
            secret_access_key: "s".into(),
            session_token: "t".into(),
            expires_at: shroom_cloud::registry::iso((now_secs() as i64 + expires_in_s) as u64),
            bucket: "shroom-space".into(),
            endpoint: "http://127.0.0.1:9".into(),
        };
        super::super::home::write_json(&dir.join("cred.json"), &f).unwrap();
    }

    #[test]
    fn a_take_holding_a_credential_with_an_hour_left_can_still_finish() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_live_cred(tmp.path()), "no credential");
        cred(tmp.path(), "upload", 5 * 86_400);
        assert!(has_live_cred(tmp.path()));
        cred(tmp.path(), "upload", 30 * 60);
        assert!(!has_live_cred(tmp.path()), "under an hour: re-minted, so it needs the space");
        cred(tmp.path(), "manage", 5 * 86_400);
        assert!(!has_live_cred(tmp.path()), "only an upload credential sends bytes");
    }

    #[test]
    fn owed_discards_are_read_back_from_their_markers() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(pending_forgets(&tmp.path().join("forget")).is_empty(), "none yet");
        let dir = tmp.path().join("forget");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("AbCdEfGhIjKlMnOp"), b"").unwrap();
        std::fs::write(dir.join(".DS_Store"), b"").unwrap();
        assert_eq!(pending_forgets(&dir), ["AbCdEfGhIjKlMnOp"]);
    }

    /// One answer from a stand-in page on loopback: `POST /_shroom/api/mint` → a credential.
    fn one_mint(expires_at: String) -> String {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            // The whole request, so closing never resets the connection under the client.
            let (mut got, mut buf) = (Vec::new(), [0u8; 4096]);
            loop {
                let n = c.read(&mut buf).unwrap_or(0);
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_lowercase();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text.lines().find_map(|l| l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                    if got.len() >= end + 4 + len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let body = serde_json::json!({
                "shroom": 1, "accessKeyId": "k", "secretAccessKey": "s", "sessionToken": "t",
                "expiresAt": expires_at, "bucket": "shroom-space", "endpoint": "http://127.0.0.1:9", "owner": "m_mher",
            })
            .to_string();
            let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        });
        base
    }

    /// N32 / whole-spec audit #22: a credential minted after the takes scan reaches the core
    /// at once — a space that removes this Mac afterwards still lets that take finish, even
    /// when its upload then meets a transient failure.
    #[test]
    fn a_fresh_mint_tells_the_core_the_take_is_on_its_way() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, rx) = super::super::Runtime::for_test(tmp.path().join("home"), None, None);
        let dir = rt.home.takes().join("20261002-101500-ab12");
        std::fs::create_dir_all(&dir).unwrap();
        let expires = shroom_cloud::registry::iso(now_secs() + 5 * 86_400);
        let base = one_mint(expires.clone());
        let identity = SpaceIdentity {
            account_id: "acc".into(),
            label: "v".into(),
            subdomain: "myshroom".into(),
            host: "v.myshroom.workers.dev".into(),
            bucket: "shroom-space".into(),
            role: Role::Member,
            member_id: "m_mher".into(),
            owner_name: "Bagrat".into(),
            account_name: None,
            foreign_v: false,
        };
        let key = DeviceKey { v: 1, host: identity.host.clone(), device: "d".into(), secret: "x".into() };
        let c = Ctx { identity: identity.clone(), door: Door::Device { key, page: Space::at(&identity.host, &base) } };
        let (_, fresh) = creds_for(&rt, &c, Some(&dir), "QrStUvWxYz012345", Purpose::Upload).unwrap_or_else(|_| panic!("minted"));
        assert!(fresh, "minted this call");
        let posted: Vec<Event> = rx.try_iter().map(|e| e.event).collect();
        let want = Event::Upload(UploadNews::Minted {
            dir: TakeDir("20261002-101500-ab12".into()),
            until: super::super::member::iso_ms(&expires).map(shroom_core::time::Wall),
        });
        assert_eq!(posted, [want], "the core hears of the mint, not only the next scan");

        // A credential already held (more than an hour left) is reused — nothing new to say.
        let (_, fresh) = creds_for(&rt, &c, Some(&dir), "QrStUvWxYz012345", Purpose::Upload).unwrap_or_else(|_| panic!("held"));
        assert!(!fresh);
        assert_eq!(rx.try_iter().count(), 0);
    }

    #[test]
    fn a_live_marker_keeps_the_count_for_the_resume() {
        let tmp = tempfile::tempdir().unwrap();
        super::super::home::write_json(&tmp.path().join("live"), &LiveMarker { stream: 9, total: 11 }).unwrap();
        let m: LiveMarker = super::super::home::read_json(&tmp.path().join("live")).unwrap();
        assert_eq!((m.stream, m.total), (9, 11));
    }
}
