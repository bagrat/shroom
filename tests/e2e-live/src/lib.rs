//! L2 on the dedicated test account (TESTING-PLAN D3). Every run is **pinned** (the token
//! reaches the e2e account and nothing else, and that account isn't the main one), has
//! **its own space** (a run id, `<unix>-<hex4>`: its page `v-<run>` over its bucket
//! `shroom-space-<run>`, DECISIONS B131), is **scoped** (its close deletes its own Worker,
//! bucket and token and nothing else; a crashed run's leftovers are reaped by the age in
//! their run id), and is driven by a **per-run token** minted with exactly the product's
//! two powers. Runs side by side never touch each other's space, so there is no lock.
//!
//! The suite runs only with SHROOM_E2E=1; the heartbeat's dev space (`shroom-dev
//! dev-space`) opens and closes its run through [`open`] and [`close`]. The account token comes from SHROOM_E2E_TOKEN_FILE or
//! the repo's gitignored `.secrets/e2e-account-token`; the account id from
//! SHROOM_E2E_ACCOUNT_ID or `.secrets/e2e-account-id`. Neither is ever printed.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use shroom_cloud::sweep::{self, Verdict};
use shroom_cloud::wire;
use shroom_cloud::{Cloud, CloudError, Policy};

/// The e2e account's workers.dev name, claimed once (D3).
pub const NAME: &str = "shroom-e2e";
/// The main account's workers.dev name: a run never starts on an account that has it.
pub const MAIN_NAME: &str = "myshroom";
/// Per-run tokens are named with this and the run id; a crashed run's is reaped by its age.
pub const RUN_TOKEN: &str = "shroom-e2e-run-";
/// The product's powers (the token page's Workers Scripts Edit + Workers R2 Storage Edit).
pub const PRODUCT_POWERS: [&str; 2] = ["Workers Scripts Write", "Workers R2 Storage Write"];

pub struct Config {
    pub account_id: String,
    pub token: String,
}

/// The repo's gitignored `.secrets/` — where this Mac keeps the e2e credentials, beside the
/// checkout and outside every app's home.
fn secret_file(name: &str) -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.secrets").join(name);
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// The run's credentials, or None when the suite isn't switched on. Switched on without
/// them, it fails loudly: a run that skipped everything must never read as green.
pub fn config() -> Option<Config> {
    if std::env::var("SHROOM_E2E").ok().as_deref() != Some("1") {
        return None;
    }
    Some(credentials().unwrap_or_else(|e| panic!("SHROOM_E2E=1 but {e}")))
}

/// The account token and id, wherever they are kept — for the suite and for the dev app's
/// space (`shroom-dev dev-space`, the heartbeat's). Says what is missing, never a value.
pub fn credentials() -> Result<Config, String> {
    let token = match std::env::var("SHROOM_E2E_TOKEN_FILE") {
        Ok(path) => std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()),
        Err(_) => secret_file("e2e-account-token"),
    }
    .filter(|t| !t.is_empty())
    .ok_or("no account token (SHROOM_E2E_TOKEN_FILE or .secrets/e2e-account-token in the repo)")?;
    let account_id = std::env::var("SHROOM_E2E_ACCOUNT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .or_else(|| secret_file("e2e-account-id"))
        .filter(|a| !a.is_empty())
        .ok_or("no account id (SHROOM_E2E_ACCOUNT_ID or .secrets/e2e-account-id in the repo)")?;
    Ok(Config { account_id, token })
}

/// The pin: the token reaches exactly `account_id`, verifies there, and the account isn't
/// the main one. Says why it refused, without the ids.
pub fn pin(cloud: &Cloud, account_id: &str) -> Result<(), String> {
    let accounts = cloud.accounts().map_err(|e| format!("the token's accounts: {e:?}"))?;
    if accounts.len() != 1 || accounts[0].id != account_id {
        return Err(format!("the token reaches {} account(s), not just the e2e one", accounts.len()));
    }
    let status = cloud.verify(account_id).map_err(|e| format!("verify: {e:?}"))?;
    if status.status != "active" {
        return Err(format!("the token is {}", status.status));
    }
    if cloud.subdomain(account_id).map_err(|e| format!("the workers.dev name: {e:?}"))?.as_deref() == Some(MAIN_NAME) {
        return Err(format!("this account is {MAIN_NAME} — the main account"));
    }
    Ok(())
}

/// A fresh run id: when the run began, and four hex digits so two runs begun in the same
/// second differ (the process id and the clock's nanoseconds, mixed).
pub fn new_run_id() -> String {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let mix = (u64::from(t.subsec_nanos()) ^ u64::from(std::process::id()).rotate_left(17)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    format!("{}-{:04x}", t.as_secs(), (mix >> 48) & 0xffff)
}

/// The run's token's name.
pub fn run_token_name(run: &str) -> String {
    format!("{RUN_TOKEN}{run}")
}

/// Deletes every Worker and bucket (emptied first, with `token_id`'s derived keys) whose
/// name `takes`. Returns what it removed, sorted.
fn delete_where(cloud: &Cloud, account_id: &str, token_id: &str, takes: impl Fn(&str) -> bool) -> Result<Vec<String>, CloudError> {
    let mut gone = Vec::new();
    let scripts = cloud.raw("GET", &format!("/accounts/{account_id}/workers/scripts"), None)?;
    for s in scripts["result"].as_array().into_iter().flatten() {
        if let Some(name) = s["id"].as_str().filter(|n| takes(n)) {
            cloud.delete_worker(account_id, name)?;
            gone.push(format!("worker {name}"));
        }
    }
    for bucket in cloud.list_buckets(account_id)?.into_iter().filter(|b| takes(b)) {
        let s3 = wire::bucket_client(cloud, account_id, &bucket, token_id);
        for o in s3.list("").map_err(|e| CloudError::Transport(format!("listing {bucket}: {e:?}")))? {
            s3.delete(&o.key).map_err(|e| CloudError::Transport(format!("emptying {bucket}: {e:?}")))?;
        }
        cloud.delete_bucket(account_id, &bucket)?;
        gone.push(format!("bucket {bucket}"));
    }
    gone.sort();
    Ok(gone)
}

/// A run's own space: every Worker and bucket whose name ends in its run id (`v-<run>`,
/// `shroom-space-<run>`, the dev space's `shroom-test-dev-<run>`) — nothing else.
pub fn sweep_run(cloud: &Cloud, account_id: &str, token_id: &str, run: &str) -> Result<Vec<String>, CloudError> {
    delete_where(cloud, account_id, token_id, |name| wire::run_of(name) == Some(run))
}

/// The product's own names (`v` or its fallbacks over `shroom-space`), which only a release
/// build sets up — the AI migration check (`e2e-migration.yml`) installs a signed release,
/// so its space has them. No other lane deploys them on the test account (D3).
pub fn product_name(name: &str) -> bool {
    sweep::LABELS.contains(&name) || name == wire::BUCKET
}

/// The release's space on the test account: its page under the product's labels and the
/// `shroom-space` bucket — nothing named by a run.
pub fn sweep_product(cloud: &Cloud, account_id: &str, token_id: &str) -> Result<Vec<String>, CloudError> {
    delete_where(cloud, account_id, token_id, product_name)
}

/// A run older than this is a crashed run's leftover (a whole run takes well under it);
/// a younger one may still be going, and is never touched.
pub const RUN_FRESH_SECS: u64 = 45 * 60;

fn crashed(run: &str, now: u64) -> bool {
    wire::run_started(run).is_some_and(|at| now.saturating_sub(at) >= RUN_FRESH_SECS)
}

/// Crashed runs' spaces: every Worker and bucket whose run id is at least
/// [`RUN_FRESH_SECS`] old. A name without a run id is never a run's to take.
pub fn reap(cloud: &Cloud, account_id: &str, token_id: &str, now: u64) -> Result<Vec<String>, CloudError> {
    delete_where(cloud, account_id, token_id, |name| wire::run_of(name).is_some_and(|run| crashed(run, now)))
}

/// When a run token was issued: the run id its name ends in, else Cloudflare's
/// `issued_on`, else a bare `<unix>` after the prefix (the names before run ids). None:
/// none reads.
fn issued(t: &Value) -> Option<u64> {
    let name = t["name"].as_str()?;
    wire::run_of(name)
        .and_then(wire::run_started)
        .or_else(|| t["issued_on"].as_str().and_then(parse_rfc3339))
        .or_else(|| name.strip_prefix(RUN_TOKEN)?.parse().ok())
}

/// The run tokens a crashed run left: at least [`RUN_FRESH_SECS`] old. A token whose age
/// can't be read is left be — never delete what might be running.
pub fn stale_run_tokens(tokens: &Value, now: u64) -> Vec<String> {
    tokens["result"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| t["name"].as_str().is_some_and(|n| n.starts_with(RUN_TOKEN)))
        .filter(|t| issued(t).is_some_and(|at| now.saturating_sub(at) >= RUN_FRESH_SECS))
        .filter_map(|t| t["id"].as_str().map(String::from))
        .collect()
}

/// Deletes the run tokens in `ids`.
fn delete_tokens(parent: &Cloud, account_id: &str, ids: &[String]) -> Result<(), CloudError> {
    for id in ids {
        parent.raw("DELETE", &format!("/accounts/{account_id}/tokens/{id}"), None)?;
    }
    Ok(())
}

/// The account's tokens to delete: the run's own (by name) and every crashed run's.
fn run_tokens_to_delete(parent: &Cloud, account_id: &str, run: Option<&str>) -> Result<Vec<String>, CloudError> {
    let tokens = parent.raw("GET", &format!("/accounts/{account_id}/tokens"), None)?;
    let mut ids = stale_run_tokens(&tokens, now());
    if let Some(name) = run.map(run_token_name) {
        for t in tokens["result"].as_array().into_iter().flatten().filter(|t| t["name"] == name.as_str()) {
            ids.extend(t["id"].as_str().map(String::from));
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` as seconds since the epoch (Cloudflare's timestamps).
pub fn parse_rfc3339(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' || !s.ends_with('Z') {
        return None;
    }
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // Days-from-civil (Howard Hinnant), the inverse of `rfc3339`.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()
}

/// `secs` since the epoch as RFC 3339 UTC (Cloudflare's `expires_on`).
pub fn rfc3339(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil-from-days (Howard Hinnant).
    let z = days as i64 + 719_468;
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

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Mints the run's token, named for the run: the product's two powers on this account, two
/// hours to live. Returns (token id, value).
fn mint(parent: &Cloud, account_id: &str, run: &str) -> Result<(String, String), String> {
    let groups = parent
        .raw("GET", &format!("/accounts/{account_id}/tokens/permission_groups"), None)
        .map_err(|e| format!("permission groups: {e:?}"))?;
    let ids: Vec<Value> = PRODUCT_POWERS
        .iter()
        .map(|want| {
            groups["result"].as_array().into_iter().flatten().find(|g| g["name"] == *want).map(|g| json!({"id": g["id"]})).ok_or(format!("no permission group {want:?}"))
        })
        .collect::<Result<_, _>>()?;
    let body = json!({
        "name": run_token_name(run),
        "policies": [{"effect": "allow", "resources": {format!("com.cloudflare.api.account.{account_id}"): "*"}, "permission_groups": ids}],
        "expires_on": rfc3339(now() + 2 * 3600),
    });
    let v = parent.raw("POST", &format!("/accounts/{account_id}/tokens"), Some(body)).map_err(|e| format!("minting the run token: {e:?}"))?;
    match (v["result"]["id"].as_str(), v["result"]["value"].as_str()) {
        (Some(id), Some(value)) => Ok((id.to_string(), value.to_string())),
        _ => Err("the minted token came back without an id or value".into()),
    }
}

/// A run's opening: the pin, crashed runs' leftovers reaped, then the run's token minted
/// and seen to verify — shared by [`Run`], `e2e-run` and the heartbeat's dev space
/// (`shroom-dev dev-space`). Nothing of another live run is touched, so nothing waits.
/// The caller sets up under [`Opened::space`] and ends with [`close`] for [`Opened::run`].
pub struct Opened {
    pub parent: Cloud,
    pub parent_token_id: String,
    /// The run id: what the run's space, its token and its leftovers are named by.
    pub run: String,
    /// The run token's id (its derived S3 key) and its value — the product's token.
    pub token_id: String,
    pub value: String,
}

impl Opened {
    /// Where the run's setup puts its space: `v-<run>` over `shroom-space-<run>`.
    pub fn space(&self) -> wire::SpaceNames {
        wire::SpaceNames::of_run(&self.run).expect("a fresh run id")
    }
}

/// `run`: the run id, made by the caller (and written where the app reads it) before any
/// of it exists on the account — so a run cut short after its token is minted is still
/// closed, or reaped, by name.
pub fn open(cfg: &Config, run: &str) -> Result<Opened, String> {
    if wire::run_started(run).is_none() {
        return Err(format!("{run:?} isn't a run id"));
    }
    let parent = Cloud::new(&cfg.token, Policy::Unfenced);
    pin(&parent, &cfg.account_id)?;
    let parent_token_id = parent.verify(&cfg.account_id).map_err(|e| format!("verify: {e:?}"))?.id;
    let reaped = reap(&parent, &cfg.account_id, &parent_token_id, now()).map_err(|e| format!("reaping a crashed run's space: {e:?}"))?;
    if !reaped.is_empty() {
        eprintln!("e2e: reaped a crashed run's {}", reaped.join(", "));
    }
    let stale = run_tokens_to_delete(&parent, &cfg.account_id, None).map_err(|e| format!("reading the run tokens: {e:?}"))?;
    delete_tokens(&parent, &cfg.account_id, &stale).map_err(|e| format!("deleting a crashed run's token: {e:?}"))?;
    let (token_id, value) = mint(&parent, &cfg.account_id, run)?;
    let product = Cloud::new(&value, Policy::Unfenced);
    // A fresh token can take a few seconds to verify everywhere.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match sweep::verify(&product) {
            Ok(Verdict::Valid { .. }) => break,
            _ if Instant::now() < deadline => std::thread::sleep(Duration::from_secs(2)),
            other => {
                let _ = parent.raw("DELETE", &format!("/accounts/{}/tokens/{token_id}", cfg.account_id), None);
                return Err(format!("the run token never verified: {other:?}"));
            }
        }
    }
    Ok(Opened { parent, parent_token_id, run: run.to_string(), token_id, value })
}

/// A run's close: its own space and token deleted (`run`: None when it never got an id),
/// and any crashed run's leftovers reaped. Another live run's space is never touched.
/// Returns what went.
pub fn close(cfg: &Config, run: Option<&str>) -> Result<Vec<String>, String> {
    let parent = Cloud::new(&cfg.token, Policy::Unfenced);
    pin(&parent, &cfg.account_id)?;
    let parent_token_id = parent.verify(&cfg.account_id).map_err(|e| format!("verify: {e:?}"))?.id;
    let mut gone = match run {
        Some(run) => sweep_run(&parent, &cfg.account_id, &parent_token_id, run).map_err(|e| format!("the run's own space: {e:?}"))?,
        None => Vec::new(),
    };
    gone.extend(reap(&parent, &cfg.account_id, &parent_token_id, now()).map_err(|e| format!("reaping a crashed run's space: {e:?}"))?);
    let tokens = run_tokens_to_delete(&parent, &cfg.account_id, run).map_err(|e| format!("reading the run tokens: {e:?}"))?;
    delete_tokens(&parent, &cfg.account_id, &tokens).map_err(|e| format!("deleting the run tokens: {e:?}"))?;
    gone.sort();
    gone.dedup();
    Ok(gone)
}

/// The release's space deleted (its page and its bucket, emptied first): before the
/// migration check sets one up, so a crashed check's leftover is never reconnected, and
/// after it. Returns what went.
pub fn close_product(cfg: &Config) -> Result<Vec<String>, String> {
    let parent = Cloud::new(&cfg.token, Policy::Unfenced);
    pin(&parent, &cfg.account_id)?;
    let parent_token_id = parent.verify(&cfg.account_id).map_err(|e| format!("verify: {e:?}"))?.id;
    sweep_product(&parent, &cfg.account_id, &parent_token_id).map_err(|e| format!("the release's space: {e:?}"))
}

/// One run on the e2e account, in its own space. Dropping it deletes that space and the
/// run token.
pub struct Run {
    pub account_id: String,
    /// The run token's client — what the tests drive, as the product would.
    pub product: Cloud,
    /// The run token's id (its derived S3 key).
    pub token_id: String,
    /// Where the run sets up: `v-<run>` over `shroom-space-<run>`.
    pub space: wire::SpaceNames,
    run: String,
    parent: Cloud,
    parent_token_id: String,
}

impl Run {
    pub fn start(cfg: &Config) -> Result<Run, String> {
        let opened = open(cfg, &new_run_id())?;
        let space = opened.space();
        let Opened { parent, parent_token_id, run, token_id, value } = opened;
        let product = Cloud::new(&value, Policy::Unfenced);
        Ok(Run { account_id: cfg.account_id.clone(), product, token_id, space, run, parent, parent_token_id })
    }

    /// The account token's client — for the harness's own reads, never the product's path.
    pub fn parent(&self) -> &Cloud {
        &self.parent
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        if let Err(e) = sweep_run(&self.parent, &self.account_id, &self.parent_token_id, &self.run) {
            eprintln!("the run's close failed (a later run reaps it after {} min): {e:?}", RUN_FRESH_SECS / 60);
        }
        let _ = self.parent.raw("DELETE", &format!("/accounts/{}/tokens/{}", self.account_id, self.token_id), None);
    }
}
