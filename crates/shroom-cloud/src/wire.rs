//! Wiring a space (R7.1, ARCHITECTURE §7): the private bucket, the player assets and the
//! space's manifest in it, the roster, the Worker with its bindings (the bucket and the
//! mint parent), its workers.dev route, and the silent self-check through the real path.
//! Every call is idempotent — re-running after any interruption resumes (Region 2's
//! backbone). Then [`enroll_here`] makes this Mac member #1's (WORKER-INTERFACE §6.11).

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use shroom_storage::s3::{Bucket, Client};

use crate::device::DeviceKey;
use crate::member::{EnrollNext, Enrolled, Ping, Refusal, Space};
use crate::{Binding, Cloud, CloudError, registry};

/// The bucket every space keeps its videos in (DECISIONS B24). Named so it never collides
/// with anything else in the account — a bucket is shroom's only if it holds
/// `.shroom/manifest.json`.
pub const BUCKET: &str = "shroom-space";
/// The Worker's label: every link is `v.<name>.workers.dev/<id>` (U8), or `shroom` when a
/// Worker named `v` that isn't shroom's already exists (N42).
pub const LABEL: &str = "v";
pub const FALLBACK_LABEL: &str = "shroom";
pub const MANIFEST: &str = ".shroom/manifest.json";
/// Where a space lives on its account: the labels its page may take, in the order tried,
/// and its bucket. The two move together — a bucket shared between two spaces would mix
/// their videos and their deletes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceNames {
    pub labels: Vec<String>,
    pub bucket: String,
}

impl SpaceNames {
    /// The product's, in every build but a dev build's e2e run: `v` (or its fallbacks,
    /// N42) over `shroom-space`.
    pub fn product() -> SpaceNames {
        SpaceNames { labels: crate::sweep::LABELS.iter().map(|l| l.to_string()).collect(), bucket: BUCKET.into() }
    }

    /// An e2e run's own space on the shared test account (TESTING-PLAN D3, DECISIONS
    /// B131): `v-<run>` over `shroom-space-<run>`, with no fallback. None: not a run id.
    pub fn of_run(run: &str) -> Option<SpaceNames> {
        run_started(run)?;
        Some(SpaceNames { labels: vec![format!("{LABEL}-{run}")], bucket: format!("{BUCKET}-{run}") })
    }

    pub fn labels(&self) -> Vec<&str> {
        self.labels.iter().map(String::as_str).collect()
    }
}

/// When an e2e run began: a run id is `<unix seconds, ten digits>-<four lowercase hex
/// digits>`. None: not a run id.
pub fn run_started(run: &str) -> Option<u64> {
    let (secs, tag) = run.split_once('-')?;
    let hex = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
    (secs.len() == 10 && secs.bytes().all(|b| b.is_ascii_digit()) && tag.len() == 4 && tag.bytes().all(hex)).then(|| secs.parse().ok())?
}

/// The run id a name ends in (`v-<run>`, `shroom-space-<run>`, a run token's
/// `shroom-e2e-run-<run>`), if it ends in one after a hyphen.
pub fn run_of(name: &str) -> Option<&str> {
    let cut = name.len().checked_sub(15)?;
    let (head, run) = (name.get(..cut)?, name.get(cut..)?);
    (head.len() > 1 && head.ends_with('-') && run_started(run).is_some()).then_some(run)
}

/// The Worker's mint parent (WORKER-INTERFACE §3.4): one `secret_text` binding holding
/// `{account_id, bucket, key_id, secret}` — the token's derived S3 pair (B32).
pub const PARENT_BINDING: &str = "R2_PARENT";

pub struct WireSpec<'a> {
    pub account_id: &'a str,
    /// The token's own id — the derived S3 key (R7.6).
    pub token_id: &'a str,
    pub label: &'a str,
    pub bucket: &'a str,
    pub worker_module: &'a str,
    pub compatibility_date: &'a str,
    /// Files the page needs from the bucket: (key, bytes, content type).
    pub assets: &'a [(&'a str, &'a [u8], &'a str)],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wired {
    pub subdomain: String,
    pub host: String,
    /// The script went up: the page was absent, not answering, or older than this app's
    /// (§6.10). `false`: the page already answered at this version or a newer one, so only
    /// the parent was re-placed — never an older version over a newer one.
    pub deployed: bool,
}

/// How wiring reads the space's page (WORKER-INTERFACE §6.10, DECISIONS B50).
pub struct PageCheck<'a> {
    /// The page at a host: `member::Space::new` in the app, a stand-in in tests.
    pub at: &'a dyn Fn(&str) -> Space,
    /// The `VERSION` this app's own page carries (`worker.js`).
    pub version: &'a str,
    /// workers.dev's propagation wait (~40 s measured on a fresh name, RESEARCH L2005):
    /// after the route is switched on, the ping is asked every `every` until it answers as
    /// shroom, for at most `within` — inside `Op::Wire`'s 3-minute deadline.
    pub every: Duration,
    pub within: Duration,
}

impl PageCheck<'_> {
    /// The product's cadence: every 3 s, for at most 90 s.
    pub const EVERY: Duration = Duration::from_secs(3);
    pub const WITHIN: Duration = Duration::from_secs(90);
}

/// A dotted version as numbers (`0.1.0` → `[0, 1, 0]`); a pre-release suffix is ignored.
/// `None` when it isn't one.
pub fn semver(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().split(['-', '+']).next()?;
    let parts: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

/// Where a version stands among its own builds (DECISIONS B220): an early build
/// (`-early.<n>`) below the release, the release itself highest; any other suffix lowest.
fn stage(v: &str) -> u64 {
    match v.trim().split_once('-') {
        None => u64::MAX,
        Some((_, pre)) => pre.strip_prefix("early.").and_then(|n| n.parse().ok()).unwrap_or(0),
    }
}

/// Whether a page answering at `live` may be replaced by this app's `bundled` page: only
/// when it's strictly older — by its numbers, then its stage, so `0.1.25-early.1` <
/// `0.1.25-early.2` < `0.1.25` (B220). A version that can't be read is never overwritten
/// (the page refuses a downgrade, SM:1754-1756).
pub fn may_deploy_over(live: &str, bundled: &str) -> bool {
    match (semver(live), semver(bundled)) {
        (Some(l), Some(b)) => {
            let n = l.len().max(b.len());
            let pad = |mut v: Vec<u64>| {
                v.resize(n, 0);
                v
            };
            (pad(l), stage(live)) < (pad(b), stage(bundled))
        }
        _ => false,
    }
}

#[derive(Debug)]
pub enum WireError {
    Cloud(CloudError),
    Storage(shroom_storage::s3::S3Error),
    /// The account has no workers.dev name yet — the `name` step comes first.
    NoName,
    /// The self-check's object didn't come back.
    SelfCheck,
    /// A Worker under this label isn't shroom's: never written over (B24, N42).
    Foreign,
    /// The page didn't answer as shroom within the propagation wait: a retry, never done.
    NotAnswering,
}

impl From<CloudError> for WireError {
    fn from(e: CloudError) -> Self {
        WireError::Cloud(e)
    }
}

impl From<shroom_storage::s3::S3Error> for WireError {
    fn from(e: shroom_storage::s3::S3Error) -> Self {
        WireError::Storage(e)
    }
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Cloud(e) => write!(f, "{e}"),
            WireError::Storage(e) => write!(f, "{e}"),
            WireError::NoName => write!(f, "the account has no workers.dev name"),
            WireError::SelfCheck => write!(f, "the self-check's object didn't come back"),
            WireError::Foreign => write!(f, "the Worker under this label isn't shroom's"),
            WireError::NotAnswering => write!(f, "the page didn't answer within the propagation wait"),
        }
    }
}

impl std::error::Error for WireError {}

pub fn bucket_client(cloud: &Cloud, account_id: &str, bucket: &str, token_id: &str) -> Client {
    Client::new(Bucket {
        endpoint: cloud.r2_endpoint(account_id),
        region: "auto".into(),
        name: bucket.into(),
        credentials: cloud.s3_credentials(token_id),
    })
}

/// The `R2_PARENT` secret's text: the token's derived S3 pair, scoped by name to the
/// space's bucket (every JWT the Worker signs names it).
pub fn parent_text(cloud: &Cloud, account_id: &str, bucket: &str, token_id: &str) -> String {
    let pair = cloud.s3_credentials(token_id);
    serde_json::json!({ "account_id": account_id, "bucket": bucket, "key_id": pair.access_key_id, "secret": pair.secret_access_key }).to_string()
}

/// Re-places `R2_PARENT` alone through the script-secrets endpoint — no redeploy (B35 P5).
/// The second and third of §3.4's placements: `space-found` (a fresh paste), and a
/// `stale-parent` or `no-parent` answer to a Mac whose token is live (H1).
pub fn place_parent(cloud: &Cloud, account_id: &str, label: &str, bucket: &str, token_id: &str) -> Result<(), CloudError> {
    cloud.put_secret(account_id, label, PARENT_BINDING, &parent_text(cloud, account_id, bucket, token_id))
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// [`wire_with`] without reading the page: no version guard, no propagation wait — tests
/// and tools only. The app always wires through [`wire_with`].
pub fn wire(cloud: &Cloud, spec: &WireSpec) -> Result<Wired, WireError> {
    wire_with(cloud, spec, None)
}

/// Waits out workers.dev's propagation: the page at `host` answers as shroom.
fn await_page(page: &PageCheck, host: &str) -> Result<(), WireError> {
    let space = (page.at)(host);
    let until = Instant::now() + page.within;
    loop {
        if matches!(space.ping(), Ping::Shroom { .. }) {
            return Ok(());
        }
        if Instant::now() + page.every > until {
            return Err(WireError::NotAnswering);
        }
        std::thread::sleep(page.every);
    }
}

/// Wiring (R7.1, §6.11), idempotent end to end, with the page read first (§6.10, B50):
/// a Worker under the label that isn't shroom's is refused (B24); a shroom page already
/// answering at this app's version or newer keeps its script — only `R2_PARENT` is
/// re-placed (§3.4) — so an older app never lowers a newer page; and after the route is
/// switched on, wiring waits for the page to answer before it reports done.
pub fn wire_with(cloud: &Cloud, spec: &WireSpec, page: Option<&PageCheck>) -> Result<Wired, WireError> {
    wire_parts(cloud, spec, page, &|_| {})
}

/// Wiring's parts as the setup names them (N74), each said as it begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WirePart {
    /// The bucket, the player, the cookie key and the roster.
    Storage = 1,
    /// The script with its bindings (or the parent alone over a current page).
    Page = 2,
    /// The workers.dev route switched on, and the wait until the page answers.
    Name = 3,
    /// The write-read-delete self-check and the space's marker.
    Check = 4,
}

/// [`wire_with`], saying each part as it begins (N74) — the caller shows the wait.
pub fn wire_parts(cloud: &Cloud, spec: &WireSpec, page: Option<&PageCheck>, part: &dyn Fn(WirePart)) -> Result<Wired, WireError> {
    wire_any(cloud, spec, page, part, false)
}

/// [`wire_parts`] for a space being set up (N76; Bagrat, 2026-10-25: AI on by default): a
/// page put up where there was none carries the AI and its schedule. A page already there
/// keeps whatever it had, as every wiring does.
pub fn wire_fresh_parts(cloud: &Cloud, spec: &WireSpec, page: Option<&PageCheck>, part: &dyn Fn(WirePart)) -> Result<Wired, WireError> {
    wire_any(cloud, spec, page, part, true)
}

fn wire_any(cloud: &Cloud, spec: &WireSpec, page: Option<&PageCheck>, part: &dyn Fn(WirePart), fresh: bool) -> Result<Wired, WireError> {
    let subdomain = cloud.subdomain(spec.account_id)?.ok_or(WireError::NoName)?;
    let host = format!("{}.{subdomain}.workers.dev", spec.label);
    let (fact, ai) = crate::sweep::worker_fact_ai(cloud, spec.account_id, spec.label, spec.bucket)?;
    if !matches!(fact, crate::sweep::WorkerFact::Absent | crate::sweep::WorkerFact::Ours) {
        return Err(WireError::Foreign);
    }
    let ai = ai || fresh && fact == crate::sweep::WorkerFact::Absent;
    // Our page, answering at this version or a newer one: its script stays.
    let current = fact == crate::sweep::WorkerFact::Ours
        && page.is_some_and(|p| match (p.at)(&host).ping() {
            Ping::Shroom { version, .. } => !may_deploy_over(&version, p.version),
            _ => false,
        });
    part(WirePart::Storage);
    cloud.create_bucket(spec.account_id, spec.bucket)?;
    let s3 = bucket_client(cloud, spec.account_id, spec.bucket, spec.token_id);
    for (key, bytes, ct) in spec.assets {
        s3.put(key, bytes, ct)?;
    }
    // The owner cookie's signing secret, once: rewriting it would sign every browser out.
    let _ = s3.put_if_absent(crate::owner::COOKIE_KEY, crate::owner::cookie_secret().as_bytes(), "text/plain")?;
    // The roster, create-only, before any page that needs it goes up: a space wired before
    // Region 6 gets it from the token Mac's page refresh (`refresh_page`, DECISIONS B50 c)
    // before an iface-2 page goes up; an existing one is left exactly as found.
    let _ = registry::seed(&s3, None, unix_now())?;
    part(WirePart::Page);
    if current {
        // The page stays as it is; its parent is this token's (space-found's second
        // placement, §3.4) — a fresh paste's pair, never the previous token's.
        place_parent(cloud, spec.account_id, spec.label, spec.bucket, spec.token_id)?;
    } else {
        // Every script PUT carries the parent (§3.4's first placement), and the AI when the
        // page had it (N66: a re-wiring never turns naming off).
        put_page(cloud, spec, ai)?;
    }
    part(WirePart::Name);
    cloud.enable_workers_dev(spec.account_id, spec.label)?;
    if let Some(p) = page {
        await_page(p, &host)?;
    }
    part(WirePart::Check);
    // The silent self-check: write, read, delete one tiny object through the real path.
    let probe = ".shroom/self-check";
    s3.put(probe, b"ok", "text/plain")?;
    let back = s3.get(probe)?;
    let _ = s3.delete(probe);
    if back.as_deref() != Some(&b"ok"[..]) {
        return Err(WireError::SelfCheck);
    }
    // Last: the manifest marks the bucket as a whole shroom space (create-only). Written
    // only after everything else proved itself, so a wiring cut short never reads as a
    // space that's already there (space-found) on re-entry.
    let manifest = format!(r#"{{"schema":1,"label":"{}"}}"#, spec.label);
    let _ = s3.put_if_absent(MANIFEST, manifest.as_bytes(), "application/json")?;
    Ok(Wired { host, subdomain, deployed: !current })
}

/// The schedule a space with naming on runs: each minute (N66, DECISIONS B206 c).
pub const AI_SCHEDULE: &str = "* * * * *";

/// The script PUT with its bindings: the bucket and the parent, and with naming on (N66)
/// Cloudflare's AI, whose schedule is put beside it.
fn put_page(cloud: &Cloud, spec: &WireSpec, ai: bool) -> Result<(), CloudError> {
    put_script(cloud, spec, ai)?;
    if ai {
        cloud.put_schedules(spec.account_id, spec.label, &[AI_SCHEDULE])?;
    }
    Ok(())
}

/// The script alone, with its bindings (the AI's when `ai`).
fn put_script(cloud: &Cloud, spec: &WireSpec, ai: bool) -> Result<(), CloudError> {
    let mut bindings = vec![
        Binding::R2Bucket { name: "BUCKET".into(), bucket: spec.bucket.into() },
        Binding::SecretText { name: PARENT_BINDING.into(), text: parent_text(cloud, spec.account_id, spec.bucket, spec.token_id) },
    ];
    if ai {
        bindings.push(Binding::Ai { name: "AI".into() });
    }
    cloud.put_worker(spec.account_id, spec.label, "worker.js", spec.worker_module, &bindings, spec.compatibility_date)
}

/// Turning the AI on or off, part by part as the switch names them (N80); part 1, checking
/// the space, is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiPart {
    /// The page put up again with Cloudflare's AI, or without it.
    Page = 2,
    /// Its schedule, then the page saying so.
    Schedule = 3,
}

/// Naming on or off (N66, DECISIONS B206 a): the space's own page put up again with
/// Cloudflare's AI and its schedule, or without them. Only ever over shroom's own page; the
/// names, chapters and captions already made stay where they are. `autoclean`: the space's
/// auto-cleanup is on (N75), so naming off keeps the one schedule both use.
pub fn set_ai(cloud: &Cloud, spec: &WireSpec, on: bool, autoclean: bool) -> Result<(), WireError> {
    set_ai_parts(cloud, spec, on, autoclean, None, &|_| {})
}

/// [`set_ai`], saying each part as it begins (N80), and with `confirm` (the page's check and
/// its host) done only once the page itself says its AI is on, or off — asked every
/// `every`, for at most `within`: a page that never says so didn't go through.
pub fn set_ai_parts(cloud: &Cloud, spec: &WireSpec, on: bool, autoclean: bool, confirm: Option<(&PageCheck, &str)>, part: &dyn Fn(AiPart)) -> Result<(), WireError> {
    if crate::sweep::worker_fact(cloud, spec.account_id, spec.label, spec.bucket)? != crate::sweep::WorkerFact::Ours {
        return Err(WireError::Foreign);
    }
    part(AiPart::Page);
    put_script(cloud, spec, on)?;
    part(AiPart::Schedule);
    if on {
        cloud.put_schedules(spec.account_id, spec.label, &[AI_SCHEDULE])?;
    } else if !autoclean {
        cloud.put_schedules(spec.account_id, spec.label, &[])?;
    }
    if let Some((page, host)) = confirm {
        let space = (page.at)(host);
        let until = Instant::now() + page.within;
        while !matches!(space.ping(), Ping::Shroom { ai, .. } if ai == on) {
            if Instant::now() >= until {
                return Err(WireError::NotAnswering);
            }
            std::thread::sleep(page.every);
        }
    }
    Ok(())
}

/// The space's page given its schedule, or not (N75, DECISIONS B232): the one Cron Trigger
/// naming uses (`AI_SCHEDULE`), while auto-cleanup or naming is on. Only ever over shroom's
/// own page; the page itself isn't put up again.
pub fn set_schedule(cloud: &Cloud, spec: &WireSpec, on: bool) -> Result<(), WireError> {
    if crate::sweep::worker_fact(cloud, spec.account_id, spec.label, spec.bucket)? != crate::sweep::WorkerFact::Ours {
        return Err(WireError::Foreign);
    }
    cloud.put_schedules(spec.account_id, spec.label, if on { &[AI_SCHEDULE] } else { &[] })?;
    Ok(())
}

/// What a page refresh found (§6.10's update apply, B50).
#[derive(Debug, Clone, PartialEq)]
pub enum Refresh {
    /// The page answers at this app's version or a newer one: nothing to do.
    Current { version: String },
    /// The page answered older: the roster seeded (create-only) and this app's page put up.
    Deployed { was: String },
    /// The page doesn't answer as shroom: not a refresh's business — the self-check and the
    /// repair mood look after a page that's gone.
    NotAnswering,
}

/// The token Mac's page refresh — after an update, and at every launch (§6.10, §6.11's
/// update path): when the space's own page answers older than this app's, put this app's
/// page up — the roster seeded create-only first (a space wired before Region 6 gets it
/// before any iface-2 script), the parent riding the script PUT. Never anything else: no
/// bucket, no route, no manifest — a missing piece is the repair mood's, by the person's
/// click; and never an older version over a newer one.
pub fn refresh_page(cloud: &Cloud, spec: &WireSpec, page: &PageCheck, host: &str) -> Result<Refresh, WireError> {
    let version = match (page.at)(host).ping() {
        Ping::Shroom { version, .. } => version,
        _ => return Ok(Refresh::NotAnswering),
    };
    if !may_deploy_over(&version, page.version) {
        return Ok(Refresh::Current { version });
    }
    let (fact, ai) = crate::sweep::worker_fact_ai(cloud, spec.account_id, spec.label, spec.bucket)?;
    if fact != crate::sweep::WorkerFact::Ours {
        return Err(WireError::Foreign);
    }
    let s3 = bucket_client(cloud, spec.account_id, spec.bucket, spec.token_id);
    let _ = registry::seed(&s3, None, unix_now())?;
    // The page follows the app with naming as it was (N66).
    put_page(cloud, spec, ai)?;
    Ok(Refresh::Deployed { was: version })
}

/// Who [`enroll_here`] enrolls, and where.
pub struct EnrollSpec<'a> {
    pub account_id: &'a str,
    pub token_id: &'a str,
    pub label: &'a str,
    pub bucket: &'a str,
    /// The page's host, `<label>.<name>.workers.dev`.
    pub host: &'a str,
    /// This Mac's device key (`device.key`, made once and reused on retries).
    pub device: &'a DeviceKey,
    /// This Mac's sharing name.
    pub name: Option<&'a str>,
    /// This Mac's stamp in this space (`member::machine_stamp`, B144).
    pub machine: Option<&'a str>,
    /// The owner's macOS short name (N29).
    pub owner_name: Option<&'a str>,
}

#[derive(Debug)]
pub enum EnrollFail {
    /// The roster won't parse: the repair pass's recovery (C8, `registry::set_aside`).
    Broken,
    /// The device id belongs to someone else, or another verifier: make a fresh key.
    Conflict,
    /// The parent was re-placed and the page still says `stale-parent`: the repair mood,
    /// never a loop.
    StaleAfterReplace,
    /// Another space-side refusal (storage, a class this build doesn't read).
    Refused(Box<Refusal>),
    /// Still `busy` or 429 after the bounded retries: back off and try at the next entry —
    /// never the repair mood, never a re-wire.
    Transient,
    /// The page doesn't answer as a shroom space (re-ping; the repair mood re-wires).
    NotShroom,
    Unanswered(String),
    Cloud(CloudError),
    Storage(shroom_storage::s3::S3Error),
}

impl std::fmt::Display for EnrollFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnrollFail::Broken => write!(f, "the roster won't parse"),
            EnrollFail::Conflict => write!(f, "the device id is taken"),
            EnrollFail::StaleAfterReplace => write!(f, "the page kept another parent after a re-place"),
            EnrollFail::Refused(r) => write!(f, "refused: {} {:?} {:?}", r.status, r.class, r.reason),
            EnrollFail::NotShroom => write!(f, "the page doesn't answer as a shroom space"),
            EnrollFail::Transient => write!(f, "the space is busy; try again later"),
            EnrollFail::Unanswered(e) => write!(f, "unanswered: {e}"),
            EnrollFail::Cloud(e) => write!(f, "{e}"),
            EnrollFail::Storage(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EnrollFail {}

/// How long a re-placed parent may take to reach the page (P5 saw a few seconds).
const PARENT_PROPAGATION: Duration = Duration::from_secs(60);
/// `busy` / 429 retries inside one enroll before handing back `Transient`.
const TRANSIENT_TRIES: u32 = 4;

/// A jittered backoff for the `n`th transient retry (1-based): 0.5 s, 1 s, 2 s, … plus up
/// to half again, so two owner Macs entering together drift apart.
fn transient_pause(n: u32) -> Duration {
    let base = 500u64 << (n.saturating_sub(1)).min(4);
    let mut b = [0u8; 2];
    let jitter = if getrandom::fill(&mut b).is_ok() { u64::from(u16::from_le_bytes(b)) % (base / 2 + 1) } else { 0 };
    Duration::from_millis(base + jitter)
}

/// Wiring's step 8 and every token Mac's entry (§6.11): enroll this Mac under member #1
/// with a parent proof. On `stale-parent` or `no-parent` it re-places `R2_PARENT` with this
/// token's pair **once** and enrolls again, waiting out the secret's propagation; on
/// `unwired` it seeds the roster and enrolls again; on `clock` it corrects the offset once
/// from the page's `Date`. Call it only after the sweep found this token live — a dead
/// token never re-places (H1).
pub fn enroll_here(cloud: &Cloud, spec: &EnrollSpec) -> Result<Enrolled, EnrollFail> {
    let space = Space::new(spec.host);
    let secret = cloud.s3_credentials(spec.token_id).secret_access_key;
    let mut offset: i64 = 0;
    let (mut clocked, mut seeded) = (false, false);
    let mut transient = 0u32;
    let mut replaced: Option<Instant> = None;
    loop {
        let ts = (unix_now() as i64 + offset).max(0) as u64;
        match EnrollNext::of(space.enroll(&secret, spec.device, spec.name, spec.machine, spec.owner_name, ts)) {
            EnrollNext::Enrolled(e) => return Ok(e),
            EnrollNext::Clock(date) if !clocked => {
                clocked = true;
                if let Some(server) = date.as_deref().and_then(http_date) {
                    offset = server as i64 - unix_now() as i64;
                }
            }
            EnrollNext::ReplaceParent => match replaced {
                None => {
                    place_parent(cloud, spec.account_id, spec.label, spec.bucket, spec.token_id).map_err(EnrollFail::Cloud)?;
                    replaced = Some(Instant::now());
                    std::thread::sleep(Duration::from_secs(2));
                }
                Some(at) if at.elapsed() < PARENT_PROPAGATION => std::thread::sleep(Duration::from_secs(3)),
                Some(_) => return Err(EnrollFail::StaleAfterReplace),
            },
            EnrollNext::Seed if !seeded => {
                seeded = true;
                let s3 = bucket_client(cloud, spec.account_id, spec.bucket, spec.token_id);
                registry::seed(&s3, spec.owner_name, unix_now()).map_err(EnrollFail::Storage)?;
            }
            EnrollNext::Broken => return Err(EnrollFail::Broken),
            EnrollNext::Conflict => return Err(EnrollFail::Conflict),
            EnrollNext::Transient if transient < TRANSIENT_TRIES => {
                transient += 1;
                std::thread::sleep(transient_pause(transient));
            }
            EnrollNext::Transient => return Err(EnrollFail::Transient),
            EnrollNext::Reping => return Err(EnrollFail::NotShroom),
            EnrollNext::Unanswered(e) => return Err(EnrollFail::Unanswered(e)),
            EnrollNext::Repair(r) => return Err(EnrollFail::Refused(Box::new(r))),
            EnrollNext::Clock(_) | EnrollNext::Seed => return Err(EnrollFail::NotShroom),
        }
    }
}

/// An HTTP date (`Fri, 02 Oct 2026 12:00:00 GMT`, RFC 9110's IMF-fixdate) as unix seconds.
pub fn http_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let [_, day, month, year, time, "GMT"] = parts.as_slice() else { return None };
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"].iter().position(|m| m == month)? as i64 + 1;
    let (day, year): (i64, i64) = (day.parse().ok()?, year.parse().ok()?);
    let hms: Vec<i64> = time.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let [h, m, sec] = hms.as_slice() else { return None };
    // Days from civil (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + h * 3600 + m * 60 + sec).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_an_http_date() {
        assert_eq!(super::http_date("Fri, 02 Oct 2026 12:00:00 GMT"), Some(1_790_942_400));
        assert_eq!(super::http_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(super::http_date("Sat, 29 Feb 2028 23:59:59 GMT"), Some(1_835_481_599));
        assert_eq!(super::http_date("yesterday"), None);
    }
}
