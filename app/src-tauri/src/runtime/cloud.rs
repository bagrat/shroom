//! The cloud lane: the stop's reachability probe (capped at 600 ms so a stop never waits
//! on it), opening a video's page, and — on a sub-lane of its own, so a long wiring never
//! holds a stop's probe — the wizard's calls: validate a paste, save the token, the probe
//! pass, the name claim, and wiring. Every op answers exactly once: its result, or
//! `Failed` with its class.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use shroom_cloud::device::{self, DeviceKey};
use shroom_cloud::member::{Answer, EnrollNext, Enrolled};
use shroom_cloud::sweep::{self, Gate, Verdict};
use shroom_cloud::wire::{self, EnrollFail, PageCheck, Refresh, WireError, WireSpec};
use shroom_cloud::{Cloud, CloudError, registry};
use shroom_core::effect::{CloudCmd, SweepPart};
use shroom_core::event::{CloudNews, Event, SweepAnswer, TokenFail, WireFail};
use shroom_core::world::{Account, FailClass, Gates, Op, Reach, Role, SecretId, SpaceIdentity, TokenVerdict, WorkerFact};
use tauri::AppHandle;

use super::Runtime;

/// The page's module and the player it loads from the bucket (B30), built in.
const WORKER_MODULE: &str = include_str!("../../../../worker/src/worker.js");
const HLS_JS: &[u8] = include_bytes!("../../../../worker/vendor/hls.min.js");
const COMPATIBILITY_DATE: &str = "2026-09-01";
/// The email gate's throwaway Worker (deployed and removed by the probe).
const PROBE_WORKER: &str = "shroom-probe";
/// How long opening a stopped video's page waits for its record's claim to land (the
/// uploader claims first, before any ffmpeg work): the owner page then finds its title
/// field and Copy link (whole-spec audit #17, DECISIONS B49).
const CLAIM_WAIT: Duration = Duration::from_secs(3);

/// The `VERSION` this app's page carries (`worker.js`'s constant) — what the page refresh
/// and wiring compare a live page's ping against (§6.10, B50).
pub fn page_version() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        let at = WORKER_MODULE.find("const VERSION = \"").map(|i| i + "const VERSION = \"".len()).unwrap_or(0);
        WORKER_MODULE[at..].split('"').next().unwrap_or("0.0.0").to_string()
    })
}

pub fn spawn(rt: Runtime, _app: AppHandle) -> Sender<CloudCmd> {
    let (tx, rx) = channel::<CloudCmd>();
    let wizard = spawn_wizard(rt.clone());
    std::thread::Builder::new()
        .name("shroom-cloud".into())
        .spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(Duration::from_millis(600)))
                .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
                .build()
                .into();
            // The token's derived S3 key id, learned once per process (a verify call).
            let mut parent: Option<(String, String)> = None;
            for cmd in rx {
                let identity: Option<SpaceIdentity> = rt.home.read_json("space.json");
                match cmd {
                    CloudCmd::Reach => {
                        let reach = match &identity {
                            None => Reach::Unreachable,
                            Some(s) => match agent.get(&format!("{}/_shroom/ping", rt.page_base(&s.host))).call() {
                                Ok(mut res) => {
                                    let status = res.status().as_u16();
                                    let body = if status == 200 { res.body_mut().read_to_string().unwrap_or_default() } else { String::new() };
                                    reach_of(status, &body)
                                }
                                Err(_) => Reach::Unreachable,
                            },
                        };
                        rt.post(Event::Cloud(CloudNews::Reached(reach)));
                    }
                    CloudCmd::OpenOwnerPage(rec) => {
                        let quiet = rt.quiet.load(Ordering::SeqCst);
                        match &identity {
                            Some(s) if !quiet || MEASURE_OPEN.load(Ordering::SeqCst) => {
                                // The one-time key makes this browser the owner's (B25); without
                                // one (no token on this Mac, storage unreachable) the page opens
                                // plainly — still the right page.
                                let page = format!("https://{}/{}", s.host, rec.0);
                                let target = match open_key(&rt, s, &rec.0, &mut parent) {
                                    Some(k) => format!("{page}?k={k}"),
                                    None => page,
                                };
                                if quiet {
                                    // The heartbeat's drive does the browser's part (B59 b).
                                    *HARNESS_OPEN.lock().unwrap() = Some(target);
                                    rt.log(&format!("cloud would open the owner page for {}", super::short(&rec.0)));
                                } else {
                                    open_privately(target);
                                }
                            }
                            Some(_) => {
                                await_claim(&rt, &rec.0);
                                // Never a keyed URL in a log (law 13).
                                rt.log(&format!("cloud would open the owner page for {}", super::short(&rec.0)));
                            }
                            None => await_claim(&rt, &rec.0),
                        }
                        rt.post(Event::Cloud(CloudNews::PageOpened(rec)));
                    }
                    wizard_cmd => {
                        let _ = wizard.send((Instant::now(), wizard_cmd));
                    }
                }
            }
        })
        .expect("the cloud lane starts");
    tx
}

/// What the stop's probe heard (N40, B52, B60): the space's ping → `Shroom`; an answer
/// that isn't one — workers.dev's 404 for an address with no Worker, someone else's page —
/// → `NotShroom`, which the core believes only once it holds; a 5xx is Cloudflare's edge or
/// the Worker struggling, not a verdict on what the address is → `Unreachable`.
pub(crate) fn reach_of(status: u16, body: &str) -> Reach {
    match status {
        200 if body.contains("\"shroom\":true") => Reach::Shroom,
        500..=599 => Reach::Unreachable,
        _ => Reach::NotShroom,
    }
}

/// The key a stopped video's page opens with (DECISIONS B59 g). One minted ahead, while the
/// take ran, opens it at once: the redeem's own round trip covers the claim's (the uploader
/// claims at once), and the Worker gives the maker's browser the owner chrome over a record
/// still to come (B49 b). Without one, it's minted now, beside the claim's wait (B49 b's
/// 3 s) rather than after it.
fn open_key(rt: &Runtime, space: &SpaceIdentity, rec: &str, parent: &mut Option<(String, String)>) -> Option<String> {
    if let Some(k) = AHEAD.take(&space.host, Instant::now()) {
        rt.log("cloud: the page opens with the owner key minted ahead");
        return Some(k);
    }
    std::thread::scope(|scope| {
        let minting = scope.spawn(|| owner_key(rt, space, parent));
        await_claim(rt, rec);
        minting.join().ok().flatten()
    })
}

/// How long a key minted ahead is used: its grant lives 10 minutes (the Worker's
/// `KEY_SECONDS`), so a browser opened with it has two left to redeem it.
const AHEAD_FOR: Duration = Duration::from_secs(8 * 60);
/// After a mint ahead that didn't land, the next try waits this long.
const AHEAD_RETRY: Duration = Duration::from_secs(30);

/// The owner key minted ahead (DECISIONS B59 g): while a take runs, the recorder's watchdog
/// asks for one each second ([`mint_ahead`]); a mint starts when none is held for this
/// space or the one held is older than [`AHEAD_FOR`] (a long take re-mints — while
/// recording only, never while paused). Stop's page takes it, once. `k` lives only here,
/// never in a log or a file.
pub(crate) struct AheadKey {
    state: Mutex<AheadState>,
}

struct AheadState {
    /// The space's host, the key, and when its mint began.
    held: Option<(String, String, Instant)>,
    minting: bool,
    failed: Option<Instant>,
}

impl AheadKey {
    const fn new() -> AheadKey {
        AheadKey { state: Mutex::new(AheadState { held: None, minting: false, failed: None }) }
    }

    /// Whether a mint for `host` should begin now; if so, it's marked as under way.
    fn begin(&self, host: &str, now: Instant) -> bool {
        let mut s = self.state.lock().unwrap();
        let fresh = s.held.as_ref().is_some_and(|(h, _, at)| h == host && now.saturating_duration_since(*at) < AHEAD_FOR);
        let resting = s.failed.is_some_and(|at| now.saturating_duration_since(at) < AHEAD_RETRY);
        if s.minting || fresh || resting {
            return false;
        }
        s.minting = true;
        true
    }

    /// A mint begun at `began` answered: the key it made, or none.
    fn landed(&self, host: &str, k: Option<String>, began: Instant) {
        let mut s = self.state.lock().unwrap();
        s.minting = false;
        match k {
            Some(k) => {
                s.held = Some((host.to_string(), k, began));
                s.failed = None;
            }
            None => s.failed = Some(began),
        }
    }

    /// The key held for `host`, if it's still fresh — gone from here either way.
    fn take(&self, host: &str, now: Instant) -> Option<String> {
        let (h, k, at) = self.state.lock().unwrap().held.take()?;
        (h == host && now.saturating_duration_since(at) < AHEAD_FOR).then_some(k)
    }
}

static AHEAD: AheadKey = AheadKey::new();

/// Dev only: the heartbeat's drive measures Stop → the owner's page through the app's own
/// path — the key minted ahead, the open — and does the browser's part itself, so a quiet
/// run mints ahead and leaves the keyed URL in [`HARNESS_OPEN`] instead of opening it.
pub static MEASURE_OPEN: AtomicBool = AtomicBool::new(false);
static HARNESS_OPEN: Mutex<Option<String>> = Mutex::new(None);

/// Dev only: the URL the app would have opened (with its key), once.
#[cfg_attr(not(feature = "dev-harness"), allow(dead_code))]
pub fn take_harness_open() -> Option<String> {
    HARNESS_OPEN.lock().unwrap().take()
}

/// While a take records: a key for Stop's page, minted ahead off the recorder's thread
/// (DECISIONS B59 g). A quiet run opens no page, so it mints none — unless the heartbeat is
/// measuring the open.
pub fn mint_ahead(rt: &Runtime) {
    if rt.quiet.load(Ordering::SeqCst) && !MEASURE_OPEN.load(Ordering::SeqCst) {
        return;
    }
    let Some(space) = rt.home.read_json::<SpaceIdentity>("space.json") else { return };
    let now = Instant::now();
    if !AHEAD.begin(&space.host, now) {
        return;
    }
    let rt = rt.clone();
    let spawned = std::thread::Builder::new().name("shroom-key-ahead".into()).spawn(move || {
        let k = owner_key(&rt, &space, &mut None);
        if k.is_none() {
            rt.log("owner key ahead: none this time — Stop mints its own");
        }
        AHEAD.landed(&space.host, k, now);
    });
    if spawned.is_err() {
        AHEAD.landed("", None, now);
    }
}

/// A stopped video's page waits a moment for its record's claim (the uploader's first
/// network act for a newly published take), so the owner page opens with its title field
/// and Copy link — never the recipient's face (#17). Only for a take on this Mac that's
/// published and not yet claimed; at most [`CLAIM_WAIT`].
fn await_claim(rt: &Runtime, rec: &str) {
    let Some(dir) = super::takes::dirs(&rt.home.takes())
        .into_iter()
        .find(|d| super::home::read_json::<super::takes::TakeJson>(&d.join("take.json")).is_some_and(|t| t.rec == rec))
    else {
        return;
    };
    let settled = |d: &std::path::Path| ["claimed", "live", "uploaded", "tombstoned"].iter().any(|f| d.join(f).exists());
    if !dir.join("publish.json").exists() || settled(&dir) {
        return;
    }
    let until = Instant::now() + CLAIM_WAIT;
    while !settled(&dir) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    if !settled(&dir) {
        rt.log("cloud: the record's claim hasn't landed yet — the page opens on its own chrome");
    }
}

/// The one-time key that makes this browser the owner's on the page (B36, §6.9): `k` is
/// made here and only its hash travels — `POST /_shroom/api/key` under this Mac's device
/// key, from any Mac in the space. A token Mac's device key is the space's only once
/// enrolled (best-effort at wiring): one the page doesn't know yet (no key, or a 401)
/// enrolls first — the token proves it — and `/key` is asked once more. A token Mac whose
/// page answers `/key` as not shroom-shaped (older than iface 2) writes the grant directly
/// over S3 instead (C2). `k` is never logged; `None` opens the clean page.
pub(crate) fn owner_key(rt: &super::Runtime, space: &SpaceIdentity, parent: &mut Option<(String, String)>) -> Option<String> {
    let token_mac = space.role == Role::TokenHolder && rt.home.token().is_some();
    let Some(page) = rt.space(&space.host) else {
        let key = super::member::device_key(rt).filter(|k| k.host == space.host);
        return token_grant(rt, space, key.as_ref(), parent);
    };
    let mut enrolled = false;
    loop {
        let key = super::member::device_key(rt).filter(|k| k.host == space.host);
        let Some(dk) = key else {
            if token_mac && !enrolled && held(rt).is_ok() {
                enrolled = true;
                if enroll_token_mac(rt).is_ok() {
                    continue;
                }
            }
            // B25's grant (no device yet) — an older page's only door.
            return token_mac.then(|| token_grant(rt, space, None, parent)).flatten();
        };
        let k = device::new_page_key();
        match page.key(&dk, &device::hash(&k)) {
            Answer::Ok(_) => return Some(k),
            Answer::NotShroom { status } => {
                rt.log(&format!("owner key: the page has no /key ({status}) — the token's grant instead"));
                return token_grant(rt, space, Some(&dk), parent);
            }
            Answer::Refused(r) if token_mac && !enrolled && r.class == shroom_cloud::member::Class::Unauthorized => {
                rt.log("owner key: the page doesn't know this token Mac's device key yet — enrolling");
                enrolled = true;
                if let Err(e) = enroll_token_mac(rt) {
                    rt.log(&format!("owner key: the enroll didn't land ({}) — the page opens clean", redact(&e)));
                    return None;
                }
            }
            Answer::Refused(r) => {
                rt.log(&format!("owner key: refused {} {:?} — the page opens clean", r.status, r.class));
                return None;
            }
            Answer::Unanswered(_) => {
                rt.log("owner key: the page didn't answer — it opens clean");
                return None;
            }
        }
    }
}

/// The token Mac's own grant over S3 (C2; B25's shape when there's no device key yet).
fn token_grant(rt: &super::Runtime, space: &SpaceIdentity, key: Option<&DeviceKey>, parent: &mut Option<(String, String)>) -> Option<String> {
    // A dev build's S3 stays inside the test fence (the token's keys would pass it by).
    if !rt.bucket_allowed(&space.bucket) {
        rt.log("owner key: a dev build reaches only a shroom-test-… bucket (or the L1 fake) — refused");
        return None;
    }
    let token = rt.home.token()?;
    let cloud = rt.cloud(&token);
    let fingerprint = shroom_storage::sigv4::sha256_hex(token.as_bytes());
    let token_id = match parent {
        Some((f, id)) if *f == fingerprint => id.clone(),
        _ => {
            let id = cloud.verify(&space.account_id).ok()?.id;
            *parent = Some((fingerprint, id.clone()));
            id
        }
    };
    let s3 = rt.token_bucket(&cloud, space, &token_id);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    match key {
        Some(dk) => shroom_cloud::owner::grant_for_device(&s3, &dk.device, now).ok(),
        None => shroom_cloud::owner::grant(&s3, &space.member_id, space.role != Role::Member, now).ok(),
    }
}

/// Opens a URL through NSWorkspace — not `open <url>`, whose argv any process can read
/// while it runs (the URL may carry a one-time key).
fn open_privately(url: String) {
    super::observe::dispatch_main(move || {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};
        if let Some(u) = NSURL::URLWithString(&NSString::from_str(&url)) {
            NSWorkspace::sharedWorkspace().openURL(&u);
        }
    });
}

/// The wizard's sub-lane. Calls run one at a time; a call can outlive the core's deadline
/// for it (a slow link), and the core then issues it again — which queues behind the slow
/// one. The slow one's answer satisfies the re-issue, so a queued command that the same
/// command already answered after it was sent is skipped: every issue gets one answer.
fn spawn_wizard(rt: Runtime) -> Sender<(Instant, CloudCmd)> {
    let (tx, rx) = channel::<(Instant, CloudCmd)>();
    std::thread::Builder::new()
        .name("shroom-wizard".into())
        .spawn(move || {
            let mut answered: HashMap<String, Instant> = HashMap::new();
            for (sent, cmd) in rx {
                // Debug of a command is kinds, names and handles — never a secret.
                let key = format!("{cmd:?}");
                if answered.get(&key).is_some_and(|at| *at > sent) {
                    rt.log(&format!("wizard: {key} already answered since it was sent — skipped"));
                    continue;
                }
                let (op, kind) = match &cmd {
                    CloudCmd::Validate(_) => (Op::Validate, "validate"),
                    CloudCmd::SaveToken(..) => (Op::SaveToken, "save-token"),
                    CloudCmd::Sweep(SweepPart::Full) => (Op::Sweep, "sweep"),
                    CloudCmd::Sweep(SweepPart::Gates) => (Op::Sweep, "sweep-gates"),
                    CloudCmd::Claim(_) => (Op::Claim, "claim"),
                    CloudCmd::Wire { .. } => (Op::Wire, "wire"),
                    CloudCmd::Refresh => (Op::Refresh, "refresh"),
                    CloudCmd::SetAi(_) => (Op::Ai, "naming"),
                    CloudCmd::Reach | CloudCmd::OpenOwnerPage(_) => continue,
                };
                // A whole pass that found this token live over its own page: this Mac enrolls
                // after the answer is out (§6.11's every entry).
                let mut proved: Option<(String, String)> = None;
                let began = Instant::now();
                let news = match cmd {
                    CloudCmd::Validate(id) => validate(&rt, id),
                    CloudCmd::SaveToken(id, account_id) => save_token(&rt, id, &account_id),
                    CloudCmd::Sweep(part) => sweep_pass(&rt, part, &mut proved),
                    CloudCmd::Claim(name) => claim(&rt, name),
                    CloudCmd::Wire { label, fresh } => wire_space(&rt, &label, fresh),
                    CloudCmd::Refresh => refresh(&rt),
                    CloudCmd::SetAi(on) => set_ai(&rt, on),
                    CloudCmd::Reach | CloudCmd::OpenOwnerPage(_) => continue,
                };
                // Turning the AI on or off says how long the page took to go up again (B206 f).
                let took = if kind == "naming" { format!(" in {} ms", began.elapsed().as_millis()) } else { String::new() };
                match news {
                    Ok(n) => {
                        rt.log(&format!("wizard {kind}: {}{took}", summary(&n)));
                        rt.post(Event::Cloud(n));
                        if let Some((account_id, token_id)) = proved {
                            enroll_on_entry(&rt, &account_id, &token_id);
                        }
                    }
                    Err(class) => {
                        rt.log(&format!("wizard {kind}: failed {class:?}{took}"));
                        rt.post(Event::Failed { op, class });
                    }
                }
                answered.insert(key, Instant::now());
            }
        })
        .expect("the wizard lane starts");
    tx
}

/// What the log may say about an answer: kinds and names, never a secret.
fn summary(n: &CloudNews) -> String {
    match n {
        CloudNews::Validated { id, result } => format!("paste {} → {}", id.0, match result {
            Ok(a) => format!("{} account(s)", a.len()),
            Err(_) => "wrong powers".into(),
        }),
        CloudNews::TokenSaved(id) => format!("paste {} saved", id.0),
        CloudNews::Swept(a) => format!(
            "token {:?} gates {:?} subdomain {:?} label {:?} bucket {:?} worker {:?} found {:?}",
            a.token.as_ref().map(|t| match t {
                TokenVerdict::Valid { .. } => "valid",
                TokenVerdict::WrongPowers => "wrong-powers",
                TokenVerdict::Revoked => "revoked",
            }),
            a.gates,
            a.subdomain,
            a.label,
            a.bucket,
            a.worker,
            a.found
        ),
        CloudNews::Claimed { name, result } => format!("{name} → {result:?}"),
        CloudNews::Wired(r) => format!("{:?}", r.as_ref().map(|s| &s.host)),
        CloudNews::Reached(r) => format!("{r:?}"),
        CloudNews::PageOpened(r) => r.0.clone(),
        CloudNews::Refreshed { deployed, ai } => format!("deployed {deployed}, naming {ai:?}"),
        CloudNews::AiSet(on) => format!("naming {}", if *on { "on" } else { "off" }),
        CloudNews::Part { op, part } => format!("{op:?} part {part}"),
    }
}

/// A run's part underway (N74): the window shows it as it goes, and the log keeps when.
fn post_part(rt: &Runtime, op: Op, part: u8) {
    rt.log(&format!("wizard {}: part {part}", if op == Op::Wire { "wire" } else { "sweep" }));
    rt.post(Event::Cloud(CloudNews::Part { op, part }));
}

/// A Cloudflare error's failure class (the lanes that verify the token share it).
pub fn class_of(e: &CloudError) -> FailClass {
    class(e)
}

fn class(e: &CloudError) -> FailClass {
    match e {
        CloudError::Transport(_) => FailClass::Transport,
        CloudError::Refused(_) => FailClass::Refused,
        // Cloudflare busy or briefly failing: a calm retry after the backoff, never the
        // “something unexpected” face (DEVIATIONS #25).
        e if sweep::transient(e) => FailClass::Timeout,
        CloudError::Api { .. } => FailClass::Unclassified,
    }
}

fn account(a: &shroom_cloud::Account) -> Account {
    Account { id: a.id.clone(), name: a.name.clone() }
}

/// A paste's verify and probe by use: the accounts it reaches with both powers.
fn validate(rt: &Runtime, id: SecretId) -> Result<CloudNews, FailClass> {
    let cloud = {
        let mut stash = rt.stash.lock().unwrap();
        // The core validates the paste it holds: anything pasted before it is behind it.
        stash.superseded_by(id);
        let Some(secret) = stash.get(id) else { return Err(FailClass::Unclassified) };
        rt.cloud(secret.text())
    };
    let wrong = |rt: &Runtime| {
        // A token that can't do the job is worth nothing: forget it now.
        rt.stash.lock().unwrap().forget(id);
        Ok(CloudNews::Validated { id, result: Err(TokenFail::WrongPowers) })
    };
    let accounts = match sweep::verify(&cloud).map_err(|e| class(&e))? {
        // Not a live token at all: a copy that lost characters past the shape check —
        // `token-garbled`'s “copy it once more” (DEVIATIONS #26).
        Verdict::Rejected => {
            rt.stash.lock().unwrap().forget(id);
            return Ok(CloudNews::Validated { id, result: Err(TokenFail::Rejected) });
        }
        Verdict::Valid { accounts, .. } => accounts,
    };
    let mut able = Vec::new();
    for a in &accounts {
        if sweep::powers(&cloud, &a.id).map_err(|e| class(&e))?.both() {
            able.push(account(a));
        }
    }
    if able.is_empty() {
        return wrong(rt);
    }
    // Whether the person is offered a choice — token.json keeps it for every later proof.
    rt.stash.lock().unwrap().validated(id, able.len() > 1);
    Ok(CloudNews::Validated { id, result: Ok(able) })
}

#[derive(serde::Serialize)]
struct TokenFile<'a> {
    v: u32,
    token: &'a str,
    account_id: &'a str,
    /// The paste reached more than one usable account (the person chose this one).
    several: bool,
}

/// The verified token goes to `token.json` (mode 600, atomic) before any wiring; the
/// stash forgets it, and everything pasted before it.
fn save_token(rt: &Runtime, id: SecretId, account_id: &str) -> Result<CloudNews, FailClass> {
    let mut stash = rt.stash.lock().unwrap();
    let Some(secret) = stash.get(id) else { return Err(FailClass::Unclassified) };
    rt.home
        .write_json("token.json", &TokenFile { v: 1, token: secret.text().trim(), account_id, several: secret.several() })
        .map_err(|_| FailClass::Unclassified)?;
    stash.forget_tokens();
    Ok(CloudNews::TokenSaved(id))
}

/// The stored token, its account, and a client for both.
struct Held {
    cloud: Cloud,
    account_id: Option<String>,
    several: bool,
}

/// Only the token the person pasted (`token.json`) — never the harness's test-account
/// token, so the wizard never calls the real account from a dev build.
fn held(rt: &Runtime) -> Result<Held, FailClass> {
    let saved = rt.home.saved_token().ok_or(FailClass::Unclassified)?;
    Ok(Held { cloud: rt.cloud(&saved.token), account_id: saved.account_id, several: saved.several })
}

/// The stored token's verdict for its account, with the token's own id.
fn prove(h: &Held) -> Result<(TokenVerdict, Option<(String, String)>), FailClass> {
    let (token_id, accounts) = match sweep::verify(&h.cloud).map_err(|e| class(&e))? {
        Verdict::Rejected => return Ok((TokenVerdict::Revoked, None)),
        Verdict::Valid { token_id, accounts } => (token_id, accounts),
    };
    let found = match &h.account_id {
        Some(id) => accounts.iter().find(|a| &a.id == id),
        None => accounts.first(),
    };
    let Some(a) = found else { return Ok((TokenVerdict::Revoked, None)) };
    if !sweep::powers(&h.cloud, &a.id).map_err(|e| class(&e))?.both() {
        return Ok((TokenVerdict::WrongPowers, None));
    }
    // `several` is what the paste offered (only accounts with both powers count), not
    // how many accounts the token can see.
    let verdict = TokenVerdict::Valid { account: account(a), several: h.several };
    Ok((verdict, Some((a.id.clone(), token_id))))
}

fn gates(g: sweep::Gates) -> Gates {
    Gates { email: g.email, card: g.card }
}

fn worker_fact(f: sweep::WorkerFact) -> WorkerFact {
    match f {
        sweep::WorkerFact::Absent => WorkerFact::Absent,
        sweep::WorkerFact::Ours => WorkerFact::Ours,
        sweep::WorkerFact::Foreign => WorkerFact::Foreign,
    }
}

/// The probe pass in the plan's order — token → gates → the survey — stopping at the
/// first thing that blocks (a dead token, a gate up): a stopped pass is still a whole
/// answer. The gates' poll answers only the gates.
fn sweep_pass(rt: &Runtime, part: SweepPart, proved_here: &mut Option<(String, String)>) -> Result<CloudNews, FailClass> {
    let h = held(rt)?;
    if part == SweepPart::Gates {
        let account_id = match &h.account_id {
            Some(a) => a.clone(),
            None => match prove(&h)? {
                (_, Some((a, _))) => a,
                (verdict, None) => return Ok(CloudNews::Swept(SweepAnswer { token: Some(verdict), ..SweepAnswer::default() })),
            },
        };
        let g = sweep::gates(&h.cloud, &account_id, PROBE_WORKER).map_err(|e| class(&e))?;
        return Ok(CloudNews::Swept(SweepAnswer { gates: Some(gates(g)), ..SweepAnswer::default() }));
    }
    // The check's parts, each said as it begins (N74): the token, the email gate, storage,
    // then the look around.
    post_part(rt, Op::Sweep, 1);
    let (verdict, proved) = prove(&h)?;
    let mut ans = SweepAnswer { token: Some(verdict), ..SweepAnswer::default() };
    let Some((account_id, token_id)) = proved else { return Ok(CloudNews::Swept(ans)) };
    post_part(rt, Op::Sweep, 2);
    let email = sweep::email_gate(&h.cloud, &account_id, PROBE_WORKER).map_err(|e| class(&e))?;
    post_part(rt, Op::Sweep, 3);
    let card = sweep::card_gate(&h.cloud, &account_id).map_err(|e| class(&e))?;
    let g = sweep::Gates { email, card };
    ans.gates = Some(gates(g));
    if g.email || g.card {
        return Ok(CloudNews::Swept(ans));
    }
    post_part(rt, Op::Sweep, 4);
    // A provisioned space is probed under its own label, never the label chain (#1, B50):
    // a foreign `v` beside a space under `v` is never wired over, and a space under
    // `shroom` never reads broken when a foreign `v` goes.
    // The labels and the bucket are the product's — or an e2e run's own (B131).
    let names = rt.space_names();
    let own = rt.home.read_json::<SpaceIdentity>("space.json").filter(|s| s.account_id == account_id);
    let s = match &own {
        Some(id) => sweep::survey_own_among(&h.cloud, &account_id, &token_id, &id.label, &names.labels(), &id.bucket),
        None => sweep::survey(&h.cloud, &account_id, &token_id, &names.labels(), &names.bucket),
    }
    .map_err(|e| class(&e))?;
    ans.subdomain = Some(s.subdomain);
    ans.foreign_v = Some(s.foreign_v);
    ans.label = Some(s.label);
    ans.bucket = Some(s.bucket);
    ans.worker = Some(worker_fact(s.worker));
    ans.found = Some(s.found);
    if s.worker == sweep::WorkerFact::Ours {
        *proved_here = Some((account_id, token_id));
    }
    Ok(CloudNews::Swept(ans))
}

/// A token Mac's entry (§6.11): the sweep found its token live and its page there — enroll
/// (the Worker stamps the proof at most daily, C5), re-placing `R2_PARENT` once on
/// `stale-parent` / `no-parent` (H1). Only for the space this Mac holds the token of.
fn enroll_on_entry(rt: &Runtime, account_id: &str, token_id: &str) {
    let Some(space) = rt.home.read_json::<SpaceIdentity>("space.json").filter(|s| s.role == Role::TokenHolder && s.account_id == account_id) else { return };
    let Ok(h) = held(rt) else { return };
    let place = Place { account_id, token_id, label: &space.label, bucket: &space.bucket, host: &space.host, owner_name: &space.owner_name };
    match enroll_this_mac(rt, &h.cloud, &place) {
        Ok(e) => rt.log(&format!("enroll on entry: {} as {} (token flag {})", e.device, e.member, e.token)),
        Err(e) => rt.log(&format!("enroll on entry: {e}")),
    }
}

/// A token Mac's enroll when its device key turns out not to be the space's (the member
/// lane's 401, the owner page's `/key`): the token proved live first, then §6.11's enroll.
pub(crate) fn enroll_token_mac(rt: &Runtime) -> Result<Enrolled, String> {
    let space = rt.home.read_json::<SpaceIdentity>("space.json").filter(|s| s.role == Role::TokenHolder).ok_or("not a token Mac")?;
    let h = held(rt).map_err(|_| "no token pasted on this Mac".to_string())?;
    let (_, proved) = prove(&h).map_err(|c| format!("the token's proof: {c:?}"))?;
    let (account_id, token_id) = proved.ok_or("the token isn't live")?;
    if account_id != space.account_id {
        return Err("the token is for another account".into());
    }
    let place = Place { account_id: &account_id, token_id: &token_id, label: &space.label, bucket: &space.bucket, host: &space.host, owner_name: &space.owner_name };
    enroll_this_mac(rt, &h.cloud, &place)
}

/// Where a token Mac enrolls: its account and token, the page's label and host, the bucket,
/// and the owner's short name (N29).
struct Place<'a> {
    account_id: &'a str,
    token_id: &'a str,
    label: &'a str,
    bucket: &'a str,
    host: &'a str,
    owner_name: &'a str,
}

/// This Mac's device key (made once, before the first enroll, §5) enrolled under member #1
/// — a fresh key once if its id turned out to be someone else's.
fn enroll_this_mac(rt: &Runtime, cloud: &Cloud, place: &Place) -> Result<Enrolled, String> {
    for attempt in 0..2 {
        let key = super::member::ensure_device_key(rt, place.host).map_err(|e| format!("device.key: {e}"))?;
        match enroll(rt, cloud, place, &key) {
            Ok(e) => return Ok(e),
            Err(EnrollFail::Conflict) if attempt == 0 => rt.home.remove("device.key"),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("the device id is taken".into())
}

/// `enroll` with a parent proof (§3.2, §4.6) — `wire::enroll_here`'s loop, through this
/// build's route to the page (`Runtime::space`: a dev build reaches only the local stand-in
/// or a test-fenced page): `clock` corrects the offset once, `stale-parent`/`no-parent`
/// re-place the parent once and wait out its propagation, `unwired` seeds the roster,
/// `busy`/429 back off a few times.
fn enroll(rt: &Runtime, cloud: &Cloud, place: &Place, key: &DeviceKey) -> Result<Enrolled, EnrollFail> {
    let Place { account_id, token_id, label, bucket, host, owner_name } = *place;
    let page = rt.space(host).ok_or_else(|| EnrollFail::Unanswered("a dev build reaches this page only through --space-base".into()))?;
    let secret = cloud.s3_credentials(token_id).secret_access_key;
    let name = super::os::sharing_name();
    let machine = super::os::hardware_uuid().zip(super::home::user_home()).map(|(hw, home)| shroom_cloud::member::machine_stamp(&hw, &home.display().to_string(), host));
    let unix = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (mut offset, mut clocked, mut seeded, mut transient) = (0i64, false, false, 0u32);
    let mut replaced: Option<Instant> = None;
    loop {
        let ts = (unix() + offset).max(0) as u64;
        match EnrollNext::of(page.enroll(&secret, key, name.as_deref(), machine.as_deref(), Some(owner_name), ts)) {
            EnrollNext::Enrolled(e) => return Ok(e),
            EnrollNext::Clock(date) if !clocked => {
                clocked = true;
                if let Some(server) = date.as_deref().and_then(wire::http_date) {
                    offset = server as i64 - unix();
                }
            }
            EnrollNext::ReplaceParent => match replaced {
                None => {
                    wire::place_parent(cloud, account_id, label, bucket, token_id).map_err(EnrollFail::Cloud)?;
                    rt.log("enroll: the page held another parent — re-placed");
                    replaced = Some(Instant::now());
                    std::thread::sleep(Duration::from_secs(2));
                }
                Some(at) if at.elapsed() < Duration::from_secs(60) => std::thread::sleep(Duration::from_secs(3)),
                Some(_) => return Err(EnrollFail::StaleAfterReplace),
            },
            EnrollNext::Seed if !seeded => {
                seeded = true;
                let s3 = rt.token_bucket_at(cloud, account_id, bucket, host, token_id);
                registry::seed(&s3, Some(owner_name), unix() as u64).map_err(EnrollFail::Storage)?;
            }
            EnrollNext::Broken => return Err(EnrollFail::Broken),
            EnrollNext::Conflict => return Err(EnrollFail::Conflict),
            EnrollNext::Transient if transient < 4 => {
                transient += 1;
                std::thread::sleep(Duration::from_millis(500 << (transient - 1)));
            }
            EnrollNext::Transient => return Err(EnrollFail::Transient),
            EnrollNext::Reping => return Err(EnrollFail::NotShroom),
            EnrollNext::Unanswered(e) => return Err(EnrollFail::Unanswered(e)),
            EnrollNext::Repair(r) => return Err(EnrollFail::Refused(Box::new(r))),
            EnrollNext::Clock(_) | EnrollNext::Seed => return Err(EnrollFail::NotShroom),
        }
    }
}

/// The name claim — only ever from the person's click.
fn claim(rt: &Runtime, name: String) -> Result<CloudNews, FailClass> {
    let h = held(rt)?;
    let account_id = match &h.account_id {
        Some(a) => a.clone(),
        None => prove(&h)?.1.map(|(a, _)| a).ok_or(FailClass::Refused)?,
    };
    let result = match sweep::claim(&h.cloud, &account_id, &name).map_err(|e| class(&e))? {
        sweep::Claim::Claimed => Ok(()),
        sweep::Claim::Taken => Err(shroom_core::event::ClaimFail::Taken),
    };
    Ok(CloudNews::Claimed { name, result })
}

/// Wiring under the survey's label: the bucket, the player, the page, its route, the
/// self-check, then the space's marker — idempotent end to end. `fresh`: the setup's own
/// wiring, whose new page carries the AI (N75).
fn wire_space(rt: &Runtime, label: &str, fresh: bool) -> Result<CloudNews, FailClass> {
    // Wiring's parts, each said as it begins (N74); the window keeps the furthest.
    post_part(rt, Op::Wire, 1);
    let h = held(rt)?;
    let (_, proved) = prove(&h)?;
    let Some((account_id, token_id)) = proved else { return Ok(CloudNews::Wired(Err(WireFail::Refused))) };
    let assets: [(&str, &[u8], &str); 1] = [(".shroom/player/hls.min.js", HLS_JS, "text/javascript")];
    let names = rt.space_names();
    let spec = WireSpec {
        account_id: &account_id,
        token_id: &token_id,
        label,
        bucket: &names.bucket,
        worker_module: WORKER_MODULE,
        compatibility_date: COMPATIBILITY_DATE,
        assets: &assets,
    };
    // The page read first and after (§6.10, B50): never written over a newer page, and
    // done only once it answers.
    let at = |host: &str| rt.page_at(host);
    let page = PageCheck { at: &at, version: page_version(), every: PageCheck::EVERY, within: PageCheck::WITHIN };
    let wiring = if fresh { wire::wire_fresh_parts } else { wire::wire_parts };
    match wiring(&h.cloud, &spec, Some(&page), &|p| post_part(rt, Op::Wire, p as u8)) {
        Ok(w) => {
            rt.log(&format!("wizard wire: the page {}", if w.deployed { "went up" } else { "was current — kept" }));
            // N29: the owner's name in macOS's short form, everywhere it's written.
            let owner_name = super::os::short_user_name();
            // Wiring's enroll (§6.11 step 8): this Mac's device key, written first, made
            // member #1's. A failure here doesn't undo a working space — the token's own
            // uploads need no enroll — and the next entry's enroll tries again.
            let place = Place { account_id: &account_id, token_id: &token_id, label, bucket: &names.bucket, host: &w.host, owner_name: &owner_name };
            post_part(rt, Op::Wire, 5);
            match enroll_this_mac(rt, &h.cloud, &place) {
                Ok(e) => rt.log(&format!("wizard wire: enrolled {} as {}", e.device, e.member)),
                Err(e) => rt.log(&format!("wizard wire: enroll didn't land ({}) — the next entry tries again", redact(&e))),
            }
            Ok(CloudNews::Wired(Ok(SpaceIdentity {
                account_id,
                label: label.to_string(),
                subdomain: w.subdomain,
                host: w.host,
                bucket: names.bucket.clone(),
                role: Role::TokenHolder,
                // The token holder is the space's first member.
                member_id: "m1".into(),
                owner_name,
                account_name: None,
                // Whether someone else's `v` was found is the survey's to say: the core
                // records it with the space as this answer lands.
                foreign_v: false,
            })))
        }
        Err(WireError::Cloud(e)) => match sweep::gate_of(&e) {
            Some(Gate::Email) => Ok(CloudNews::Wired(Err(WireFail::Gate(Gates { email: true, card: false })))),
            Some(Gate::Card) => Ok(CloudNews::Wired(Err(WireFail::Gate(Gates { email: false, card: true })))),
            None if matches!(e, CloudError::Transport(_)) => Err(unreached(rt, "Cloudflare")),
            None => {
                rt.log(&format!("wizard wire: refused ({})", redact(&e.to_string())));
                Ok(CloudNews::Wired(Err(WireFail::Refused)))
            }
        },
        Err(WireError::Storage(shroom_storage::s3::S3Error::Transport(_))) => Err(unreached(rt, "the storage endpoint")),
        // workers.dev hasn't switched the name on yet: wiring tries again after the backoff,
        // calmly — never `done` before the page answers (#4, #25).
        Err(WireError::NotAnswering) => {
            rt.log("wizard wire: the page didn't answer within the propagation wait — tried again");
            Err(FailClass::Timeout)
        }
        Err(e) => {
            rt.log(&format!("wizard wire: refused ({})", redact(&e.to_string())));
            Ok(CloudNews::Wired(Err(WireFail::Refused)))
        }
    }
}

/// A call inside wiring that went unanswered. Wiring runs only after the token's proof
/// answered moments ago, so the network was up: on a brand-new account the storage
/// endpoint and the fresh name come up over the first minutes (CHECKPOINT §10, 3.5), and
/// that is a calm retry under the wiring line — never `wizard-offline`'s “back online”.
/// A network that really dropped fails the next attempt's proof, which is offline.
fn unreached(rt: &Runtime, what: &str) -> FailClass {
    rt.log(&format!("wizard wire: {what} didn't answer yet — tried again"));
    FailClass::Timeout
}

/// The page follows the app (§6.10, B50): a token Mac's launch pings its space's page and,
/// only when it answers older than this app's, puts this app's page up — the roster seeded
/// first, the parent riding the script PUT, then this Mac's enroll. A page that doesn't
/// answer as shroom is the self-check's and the repair mood's, never a refresh's.
fn refresh(rt: &Runtime) -> Result<CloudNews, FailClass> {
    let Some(space) = rt.home.read_json::<SpaceIdentity>("space.json").filter(|s| s.role == Role::TokenHolder) else {
        return Ok(CloudNews::Refreshed { deployed: false, ai: None });
    };
    // The pasted token only (never a harness's): no token here, nothing to deploy with.
    let h = held(rt)?;
    let at = |host: &str| rt.page_at(host);
    let page = PageCheck { at: &at, version: page_version(), every: PageCheck::EVERY, within: PageCheck::WITHIN };
    let ping = rt.page_at(&space.host).ping();
    // Whether the page names videos (N66): the switch in Settings · Space reads it.
    let ai = match &ping {
        shroom_cloud::member::Ping::Shroom { ai, .. } => Some(*ai),
        _ => None,
    };
    let live = live_version(ping).inspect_err(|class| {
        rt.log(&format!("refresh: the page didn't answer as the space's ({class:?}) — tried again later"));
    })?;
    if !wire::may_deploy_over(&live, page_version()) {
        rt.log(&format!("refresh: the page is at {live} — current for {}", page_version()));
        return Ok(CloudNews::Refreshed { deployed: false, ai });
    }
    let (_, proved) = prove(&h)?;
    let Some((account_id, token_id)) = proved.filter(|(a, _)| *a == space.account_id) else {
        // The self-check and the repair mood look after the token; the refresh still owes
        // this launch its check, an hour on (B50 c).
        rt.log("refresh: the token isn't live for this space — tried again later");
        return Err(FailClass::Refused);
    };
    let assets: [(&str, &[u8], &str); 1] = [(".shroom/player/hls.min.js", HLS_JS, "text/javascript")];
    let spec = WireSpec {
        account_id: &account_id,
        token_id: &token_id,
        label: &space.label,
        bucket: &space.bucket,
        worker_module: WORKER_MODULE,
        compatibility_date: COMPATIBILITY_DATE,
        assets: &assets,
    };
    match wire::refresh_page(&h.cloud, &spec, &page, &space.host) {
        Ok(Refresh::Deployed { was }) => {
            rt.log(&format!("refresh: the page went from {was} to {}", page_version()));
            enroll_on_entry(rt, &account_id, &token_id);
            Ok(CloudNews::Refreshed { deployed: true, ai })
        }
        Ok(_) => Ok(CloudNews::Refreshed { deployed: false, ai }),
        Err(WireError::Cloud(e)) => Err(class(&e)),
        Err(WireError::Storage(shroom_storage::s3::S3Error::Transport(_))) => Err(FailClass::Transport),
        Err(e) => {
            rt.log(&format!("refresh: refused ({})", redact(&e.to_string())));
            Err(FailClass::Refused)
        }
    }
}

/// Naming on or off (N66, DECISIONS B206 a): this app's page put up again over the space's
/// own with Cloudflare's AI and its schedule, or without them — only on the Mac that holds
/// the setup key, and never over a newer page (another Mac's update): that page keeps its
/// script and the switch says it couldn't change it.
fn set_ai(rt: &Runtime, on: bool) -> Result<CloudNews, FailClass> {
    let space = rt.home.read_json::<SpaceIdentity>("space.json").filter(|s| s.role == Role::TokenHolder).ok_or(FailClass::Refused)?;
    let h = held(rt)?;
    let live = live_version(rt.page_at(&space.host).ping())?;
    if wire::may_deploy_over(page_version(), &live) {
        rt.log(&format!("naming: the page is at {live}, newer than {} — kept as it is", page_version()));
        return Err(FailClass::Refused);
    }
    let (_, proved) = prove(&h)?;
    let Some((account_id, token_id)) = proved.filter(|(a, _)| *a == space.account_id) else {
        rt.log("naming: the token isn't live for this space");
        return Err(FailClass::Refused);
    };
    let assets: [(&str, &[u8], &str); 1] = [(".shroom/player/hls.min.js", HLS_JS, "text/javascript")];
    let spec = WireSpec {
        account_id: &account_id,
        token_id: &token_id,
        label: &space.label,
        bucket: &space.bucket,
        worker_module: WORKER_MODULE,
        compatibility_date: COMPATIBILITY_DATE,
        assets: &assets,
    };
    match wire::set_ai(&h.cloud, &spec, on) {
        Ok(()) => Ok(CloudNews::AiSet(on)),
        Err(WireError::Cloud(e)) => Err(class(&e)),
        Err(e) => {
            rt.log(&format!("naming: refused ({})", redact(&e.to_string())));
            Err(FailClass::Refused)
        }
    }
}

/// The live page's version, or why this launch's refresh couldn't run yet (B50 c): a page
/// that didn't answer at all (a launch before the network is up, a transport failure) is a
/// transport failure — tried again the moment the network is back, or an hour on; one that
/// answered as something other than the space's page is refused, an hour on (the
/// self-check and the repair mood look after a page that's really gone). Neither counts as
/// this launch's check.
fn live_version(ping: shroom_cloud::member::Ping) -> Result<String, FailClass> {
    use shroom_cloud::member::Ping;
    match ping {
        Ping::Shroom { version, .. } => Ok(version),
        Ping::Unanswered(_) => Err(FailClass::Transport),
        Ping::NotShroom { .. } => Err(FailClass::Refused),
    }
}

/// Error text from Cloudflare stays short in the log.
fn redact(s: &str) -> String {
    s.chars().take(160).collect()
}

#[cfg(test)]
mod tests {
    /// B60: only a 5xx is the edge's trouble; a 404 or a page that isn't the space's is the
    /// (unconfirmed) “not shroom” the core holds before it strands.
    #[test]
    fn the_stop_probe_reads_its_answer() {
        use super::reach_of;
        use shroom_core::world::Reach;
        assert_eq!(reach_of(200, r#"{"shroom":true,"version":"0.1.0"}"#), Reach::Shroom);
        assert_eq!(reach_of(200, "<html>someone else's page</html>"), Reach::NotShroom);
        assert_eq!(reach_of(404, ""), Reach::NotShroom, "workers.dev's “nothing here yet”");
        for edge in [500, 502, 503, 522, 524] {
            assert_eq!(reach_of(edge, ""), Reach::Unreachable, "{edge} says nothing about what the address is");
        }
    }

    /// B59 g: a key minted ahead is used once, only for its own space, only while fresh; a
    /// long take re-mints it; a mint that didn't land rests before the next.
    #[test]
    fn the_owner_key_minted_ahead() {
        use super::{AHEAD_FOR, AHEAD_RETRY, AheadKey};
        use std::time::{Duration, Instant};
        let ahead = AheadKey::new();
        let t0 = Instant::now();
        let host = "shroom-test-x.myshroom.workers.dev";
        assert!(ahead.begin(host, t0), "nothing held: mint");
        assert!(!ahead.begin(host, t0 + Duration::from_secs(1)), "one mint at a time");
        ahead.landed(host, Some("k1".into()), t0);
        assert!(!ahead.begin(host, t0 + Duration::from_secs(60)), "a fresh key is kept");
        assert!(ahead.begin("other.example", t0 + Duration::from_secs(60)), "another space's key isn't this one's");
        ahead.landed(host, Some("k1".into()), t0);
        assert_eq!(ahead.take("other.example", t0 + Duration::from_secs(61)), None, "never another space's key");
        assert_eq!(ahead.take(host, t0 + Duration::from_secs(61)), None, "taken (and dropped) by the miss");

        ahead.landed(host, Some("k2".into()), t0);
        assert!(ahead.begin(host, t0 + AHEAD_FOR), "a long take re-mints before the grant runs out");
        ahead.landed(host, Some("k3".into()), t0 + AHEAD_FOR);
        assert_eq!(ahead.take(host, t0 + AHEAD_FOR + Duration::from_secs(1)), Some("k3".into()));
        assert_eq!(ahead.take(host, t0 + AHEAD_FOR + Duration::from_secs(2)), None, "once");

        ahead.landed(host, Some("k4".into()), t0);
        assert_eq!(ahead.take(host, t0 + AHEAD_FOR), None, "a stale key is never opened with");

        let t1 = t0 + Duration::from_secs(3600);
        assert!(ahead.begin(host, t1));
        ahead.landed(host, None, t1);
        assert!(!ahead.begin(host, t1 + Duration::from_secs(1)), "a mint that didn't land rests");
        assert!(ahead.begin(host, t1 + AHEAD_RETRY), "then tries again");
    }

    #[test]
    fn the_bundled_pages_version_is_the_apps() {
        let v = super::page_version();
        assert!(shroom_cloud::wire::semver(v).is_some(), "{v}");
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(Some(v), conf["version"].as_str(), "wiring compares a live page against this app's own");
    }

    /// B50 c / whole-spec audit #21, #23, #27: a launch whose page didn't answer never
    /// counts as this launch's refresh — the core retries it (network back, or an hour on).
    #[test]
    fn a_page_that_didnt_answer_fails_the_refresh_so_it_runs_again() {
        use shroom_cloud::member::Ping;
        use shroom_core::world::FailClass;
        assert_eq!(super::live_version(Ping::Shroom { iface: 2, version: "1.2.3".into(), ai: false }), Ok("1.2.3".into()));
        assert_eq!(super::live_version(Ping::Unanswered("connection refused".into())), Err(FailClass::Transport));
        assert_eq!(super::live_version(Ping::NotShroom { status: 404 }), Err(FailClass::Refused));

        // Through the lane's own call: a token Mac whose page can't be reached at launch.
        let tmp = tempfile::tempdir().unwrap();
        let fake = Some(("http://127.0.0.1:9/client/v4".to_string(), "http://127.0.0.1:9/s3".to_string()));
        let (rt, _rx) = super::Runtime::for_test(tmp.path().join("home"), fake, None);
        let space = shroom_core::world::SpaceIdentity {
            account_id: "acc".into(),
            label: "v".into(),
            subdomain: "myshroom".into(),
            host: "v.myshroom.workers.dev".into(),
            bucket: "shroom-space".into(),
            role: shroom_core::world::Role::TokenHolder,
            member_id: "m".into(),
            owner_name: "Bagrat".into(),
            account_name: None,
            foreign_v: false,
        };
        super::super::home::write_json(&rt.home.root.join("space.json"), &space).unwrap();
        super::super::home::write_json(&rt.home.root.join("token.json"), &serde_json::json!({"v": 1, "token": "t", "account_id": "acc"})).unwrap();
        assert!(matches!(super::refresh(&rt), Err(FailClass::Transport)), "never Refreshed — that would end the launch's refresh");
    }
}
