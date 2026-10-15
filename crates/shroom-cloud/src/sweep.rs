//! The wizard's probes (Region 2's backbone, ARCHITECTURE §7): what a pasted token is,
//! whether it works and has its two powers, which of Cloudflare's gates still stand,
//! and what's already in the account. Every probe is a read or an idempotent write the
//! wiring would make anyway (or a throwaway it removes); the answers are facts the core
//! derives the plan from — there is no step counter here either.

use serde_json::Value;

use crate::{Account, Cloud, CloudError, codes};
use crate::wire::{BUCKET, MANIFEST};

/// Cloudflare's "enable R2 first" refusal (R2 needs a payment method on file).
/// Observed on a fresh account (`tests/fixtures/bare-account/r2-buckets.json`;
/// DEVIATIONS #47); the text backstop below catches the wording if the code differs.
pub const R2_NOT_ENABLED: i64 = 10042;
/// Script not found — Cloudflare reuses 10007 for it.
pub const SCRIPT_NOT_FOUND: i64 = 10007;

/// What a paste is, before any network (the shape/prefix check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `cfat_…`, verified at the account's endpoint.
    Account,
    /// Any other well-formed token, verified at the user endpoint.
    User,
    /// Not a token: empty, partial, spaced, a URL, or an OAuth token (`cfoat_`/`cfort_`,
    /// which has no verify endpoint and isn't what the pre-filled page makes).
    Garbled,
}

pub fn shape(paste: &str) -> Shape {
    let t = paste.trim();
    let well_formed = (30..=200).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !well_formed || t.starts_with("cfoat_") || t.starts_with("cfort_") {
        Shape::Garbled
    } else if t.starts_with("cfat_") {
        Shape::Account
    } else {
        Shape::User
    }
}

/// The verify step's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The token works. Several accounts → `token-pick-account`.
    Valid { token_id: String, accounts: Vec<Account> },
    /// Cloudflare doesn't accept it (revoked, expired, mistyped past the shape check).
    Rejected,
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name
    }
}

/// Accounts → verify (routed by prefix, R7.6). Transport failures stay errors: the
/// caller is offline, not holding a bad token.
pub fn verify(cloud: &Cloud) -> Result<Verdict, CloudError> {
    let accounts = match cloud.accounts() {
        Ok(a) => a,
        Err(e) if rejected(&e) => return Ok(Verdict::Rejected),
        Err(e) => return Err(e),
    };
    let Some(first) = accounts.first() else { return Ok(Verdict::Rejected) };
    match cloud.verify(&first.id) {
        Ok(s) if s.status == "active" => Ok(Verdict::Valid { token_id: s.id, accounts }),
        Ok(_) => Ok(Verdict::Rejected),
        Err(e) if rejected(&e) => Ok(Verdict::Rejected),
        Err(e) => Err(e),
    }
}

fn rejected(e: &CloudError) -> bool {
    matches!(e, CloudError::Api { status: 400 | 401 | 403, .. }) && (e.has_code(codes::INVALID_TOKEN) || e.has_code(codes::AUTH) || e.has_code(9109) || e.has_code(6003) || e.has_code(6111))
}

/// What a failure of the stored token on a publish says (N42's classify, DECISIONS B51):
/// Cloudflare no longer takes the token for the space's account — revoked, expired, or the
/// account out of its reach (`1000`, `9109`, `10000`, …) — a space-class failure, never a
/// network one.
pub fn token_refused(e: &CloudError) -> bool {
    rejected(e) || matches!(e, CloudError::Api { status: 401 | 403, .. })
}

/// Cloudflare is busy or briefly failing (`429`, `5xx`): retried after a backoff, never a
/// verdict about the token or the space (DEVIATIONS #25).
pub fn transient(e: &CloudError) -> bool {
    matches!(e, CloudError::Api { status: 429 | 500..=599, .. })
}

/// Probe by use: one harmless read per power, never trusting an echo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Powers {
    pub workers: bool,
    pub r2: bool,
}

impl Powers {
    pub fn both(&self) -> bool {
        self.workers && self.r2
    }
}

pub fn powers(cloud: &Cloud, account_id: &str) -> Result<Powers, CloudError> {
    let workers = match cloud.call("GET", &format!("/accounts/{account_id}/workers/scripts"), None) {
        Ok(_) => true,
        Err(e) if lacks_power(&e) => false,
        Err(e) => return Err(e),
    };
    let r2 = match cloud.list_buckets(account_id) {
        Ok(_) => true,
        // R2 not switched on is a gate (the card row), not a missing power.
        Err(e) if gate_of(&e) == Some(Gate::Card) => true,
        Err(e) if lacks_power(&e) => false,
        Err(e) => return Err(e),
    };
    Ok(Powers { workers, r2 })
}

fn lacks_power(e: &CloudError) -> bool {
    matches!(e, CloudError::Api { status: 401 | 403, .. }) && !e.has_code(codes::EMAIL_UNVERIFIED)
}

/// Cloudflare's account gates, in the order the plan surfaces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// The account's email isn't verified (8000077, numeric match).
    Email,
    /// R2 needs a payment method on file.
    Card,
}

/// Classifies a refusal as a gate. Wiring keeps the same classification as a fallback.
pub fn gate_of(e: &CloudError) -> Option<Gate> {
    if e.has_code(codes::EMAIL_UNVERIFIED) {
        return Some(Gate::Email);
    }
    if e.has_code(R2_NOT_ENABLED) {
        return Some(Gate::Card);
    }
    if let CloudError::Api { messages, .. } = e {
        let text = messages.join(" ").to_lowercase();
        if text.contains("must be verified") || text.contains("must been verified") || text.contains("verify your email") {
            return Some(Gate::Email);
        }
        if text.contains("enable r2") || text.contains("payment method") || text.contains("r2 subscription") || text.contains("purchase r2") {
            return Some(Gate::Card);
        }
    }
    None
}

/// Which gates still stand (true = blocking). The email probe is a write: a throwaway
/// Worker named by the caller (`probe` — the app passes `shroom-probe`) is deployed and removed — Cloudflare reveals an unverified email
/// only to a write. The card probe is R2's own list call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gates {
    pub email: bool,
    pub card: bool,
}

pub fn gates(cloud: &Cloud, account_id: &str, probe: &str) -> Result<Gates, CloudError> {
    let email = match cloud.put_worker(account_id, probe, "probe.js", PROBE_MODULE, &[], "2026-09-01") {
        Ok(()) => {
            let _ = cloud.delete_worker(account_id, probe);
            false
        }
        Err(e) if gate_of(&e) == Some(Gate::Email) => true,
        Err(e) => return Err(e),
    };
    let card = match cloud.list_buckets(account_id) {
        Ok(_) => false,
        Err(e) if gate_of(&e) == Some(Gate::Card) => true,
        Err(e) => return Err(e),
    };
    Ok(Gates { email, card })
}

const PROBE_MODULE: &str = "export default { fetch() { return new Response(null, { status: 404 }); } };\n";

/// What a Worker of a given name is to us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerFact {
    Absent,
    /// Bound to the space's bucket — a shroom space's page.
    Ours,
    /// Someone else's (or the POC's, bound to another bucket): **seen** there, and never
    /// touched.
    Foreign,
}

impl WorkerFact {
    /// A label the space's page can take: free, or already its own.
    pub fn takeable(self) -> bool {
        matches!(self, WorkerFact::Absent | WorkerFact::Ours)
    }
}

/// The labels a space's page may take, in order (U8; N42's clash fallback).
pub const LABELS: &[&str] = &["v", "shroom", "shroom-v"];

/// The first label that's free or already ours.
pub fn pick_label<'a>(facts: &[(&'a str, WorkerFact)]) -> Option<&'a str> {
    facts.iter().find(|(_, f)| f.takeable()).map(|(l, _)| *l)
}

pub fn worker_fact(cloud: &Cloud, account_id: &str, name: &str, bucket: &str) -> Result<WorkerFact, CloudError> {
    cloud.fence("worker", name)?;
    match cloud.call("GET", &format!("/accounts/{account_id}/workers/scripts/{name}/settings"), None) {
        Ok(v) => {
            let ours = v.pointer("/result/bindings").and_then(Value::as_array).is_some_and(|bs| {
                bs.iter().any(|b| b.get("type").and_then(Value::as_str) == Some("r2_bucket") && b.get("bucket_name").and_then(Value::as_str) == Some(bucket))
            });
            Ok(if ours { WorkerFact::Ours } else { WorkerFact::Foreign })
        }
        Err(e) if e.has_code(SCRIPT_NOT_FOUND) || matches!(e, CloudError::Api { status: 404, .. }) => Ok(WorkerFact::Absent),
        Err(e) => Err(e),
    }
}

pub fn bucket_exists(cloud: &Cloud, account_id: &str, name: &str) -> Result<bool, CloudError> {
    let v = cloud.call("GET", &format!("/accounts/{account_id}/r2/buckets?name_contains={name}&per_page=1000"), None)?;
    Ok(v.pointer("/result/buckets")
        .and_then(Value::as_array)
        .is_some_and(|a| a.iter().any(|b| b.get("name").and_then(Value::as_str) == Some(name))))
}

/// What's already in the account: the name, the page's label, and an existing space.
#[derive(Debug, Clone, PartialEq)]
pub struct Survey {
    /// The account's workers.dev name, or None (the `name` step).
    pub subdomain: Option<String>,
    /// `v`, or the clash fallback (N42) — None when every label is someone else's.
    pub label: Option<String>,
    /// A Worker named `v` that isn't shroom's was **found** (N42) — what the name row's
    /// “v was already taken” rests on. Never true for a `v` passed by without a look.
    pub foreign_v: bool,
    pub bucket: bool,
    pub worker: WorkerFact,
    /// A whole space is already here (the bucket's manifest and our page): Some(videos).
    pub found: Option<u32>,
}

/// `labels` and `bucket` are the product's ([`LABELS`], [`BUCKET`]) or a test fence's.
pub fn survey(cloud: &Cloud, account_id: &str, token_id: &str, labels: &[&str], bucket: &str) -> Result<Survey, CloudError> {
    let subdomain = cloud.subdomain(account_id)?;
    let mut facts = Vec::new();
    for l in labels {
        let f = worker_fact(cloud, account_id, l, bucket)?;
        facts.push((*l, f));
        if f.takeable() {
            break;
        }
    }
    let label = pick_label(&facts).map(String::from);
    let foreign_v = facts.first().is_some_and(|(_, f)| *f == WorkerFact::Foreign);
    let worker = facts.iter().find(|(l, _)| Some(*l) == label.as_deref()).map(|(_, f)| *f).unwrap_or(WorkerFact::Foreign);
    finish_survey(cloud, account_id, token_id, bucket, subdomain, label, foreign_v, worker)
}

/// A provisioned space's own survey (whole-spec audit #1, DECISIONS B50): its page is
/// probed under **its own label** (`space.json`), never the label chain — a foreign `v`
/// appearing beside a space under `v` reads as that space's page being someone else's (and
/// is never wired over), and a space under `shroom` stays sound when a foreign `v` goes.
/// `v` itself isn't looked at here, so `foreign_v` is what the label says — a space under a
/// fallback was set up beside someone else's `v`. (What the name row says rests on what the
/// setup's own survey saw, kept with the space — never on this.)
pub fn survey_own(cloud: &Cloud, account_id: &str, token_id: &str, label: &str, bucket: &str) -> Result<Survey, CloudError> {
    survey_own_among(cloud, account_id, token_id, label, LABELS, bucket)
}

/// [`survey_own`] where the space's label chain is `labels` — the product's ([`LABELS`]) or
/// an e2e run's own (`wire::SpaceNames::of_run`): a fallback is one past its first.
pub fn survey_own_among(cloud: &Cloud, account_id: &str, token_id: &str, label: &str, labels: &[&str], bucket: &str) -> Result<Survey, CloudError> {
    let subdomain = cloud.subdomain(account_id)?;
    let worker = worker_fact(cloud, account_id, label, bucket)?;
    let first = labels.first().copied().unwrap_or(LABELS[0]);
    let foreign_v = label != first;
    finish_survey(cloud, account_id, token_id, bucket, subdomain, Some(label.to_string()), foreign_v, worker)
}

#[allow(clippy::too_many_arguments)]
fn finish_survey(
    cloud: &Cloud,
    account_id: &str,
    token_id: &str,
    bucket: &str,
    subdomain: Option<String>,
    label: Option<String>,
    foreign_v: bool,
    worker: WorkerFact,
) -> Result<Survey, CloudError> {
    let has_bucket = bucket_exists(cloud, account_id, bucket)?;
    let mut found = None;
    if has_bucket && worker == WorkerFact::Ours {
        let s3 = crate::wire::bucket_client(cloud, account_id, bucket, token_id);
        if s3.head(MANIFEST).map_err(|e| CloudError::Transport(e.to_string()))? {
            found = Some(videos(&s3)?);
        }
    }
    Ok(Survey { subdomain, label, foreign_v, bucket: has_bucket, worker, found })
}

/// The space's videos, as `space-found` counts them: every record but a tombstone (the
/// Library's own rule — whole-spec audit #6).
fn videos(s3: &shroom_storage::s3::Client) -> Result<u32, CloudError> {
    let transport = |e: shroom_storage::s3::S3Error| CloudError::Transport(e.to_string());
    let mut n = 0;
    for o in s3.list("meta/").map_err(transport)? {
        let deleted = s3
            .get(&o.key)
            .map_err(transport)?
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .is_some_and(|m| m.get("state").and_then(Value::as_str) == Some("deleted"));
        if !deleted {
            n += 1;
        }
    }
    Ok(n)
}

/// The product's survey: label `v` (or its fallbacks) over the `shroom-space` bucket.
pub fn survey_space(cloud: &Cloud, account_id: &str, token_id: &str) -> Result<Survey, CloudError> {
    survey(cloud, account_id, token_id, LABELS, BUCKET)
}

/// The name step's claim (explicit only — never on a typing debounce).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    Claimed,
    /// 10031: someone else has that name.
    Taken,
}

pub fn claim(cloud: &Cloud, account_id: &str, name: &str) -> Result<Claim, CloudError> {
    match cloud.claim_subdomain(account_id, name) {
        Ok(()) => Ok(Claim::Claimed),
        Err(e) if e.has_code(codes::SUBDOMAIN_TAKEN) => Ok(Claim::Taken),
        Err(e) => Err(e),
    }
}

/// The name field's local check: lowercase letters, digits and inner hyphens, 1–63
/// characters (a DNS label). Uppercase lowercases silently; other characters never land.
pub fn clean_name(typed: &str) -> String {
    typed.chars().flat_map(char::to_lowercase).filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-').take(63).collect()
}

pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.len() <= 63 && !name.starts_with('-') && !name.ends_with('-') && clean_name(name) == name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        let user = "a".repeat(40);
        assert_eq!(shape(&user), Shape::User);
        assert_eq!(shape(&format!("  {user}\n")), Shape::User);
        assert_eq!(shape(&format!("cfat_{}", "B".repeat(40))), Shape::Account);
        for bad in ["", "abc", "https://dash.cloudflare.com/profile/api-tokens", &format!("{} {}", "a".repeat(20), "b".repeat(20)), &format!("cfoat_{}", "x".repeat(40)), &format!("cfort_{}", "x".repeat(40)), &"a".repeat(201)] {
            assert_eq!(shape(bad), Shape::Garbled, "{bad:?}");
        }
    }

    #[test]
    fn gates_by_code_then_by_wording() {
        let api = |codes: Vec<i64>, msg: &str| CloudError::Api { status: 403, codes, messages: vec![msg.into()] };
        assert_eq!(gate_of(&api(vec![8000077], "")), Some(Gate::Email));
        assert_eq!(gate_of(&api(vec![1], "Your user email must been verified")), Some(Gate::Email));
        assert_eq!(gate_of(&api(vec![R2_NOT_ENABLED], "")), Some(Gate::Card));
        assert_eq!(gate_of(&api(vec![1], "Please enable R2 through the Cloudflare Dashboard.")), Some(Gate::Card));
        assert_eq!(gate_of(&api(vec![10000], "Authentication error")), None);
        assert_eq!(gate_of(&CloudError::Transport("x".into())), None);
    }

    #[test]
    fn the_label_is_the_first_free_or_ours() {
        use WorkerFact::*;
        assert_eq!(pick_label(&[("v", Absent)]), Some("v"));
        assert_eq!(pick_label(&[("v", Ours)]), Some("v"));
        assert_eq!(pick_label(&[("v", Foreign), ("shroom", Absent)]), Some("shroom"));
        assert_eq!(pick_label(&[("v", Foreign), ("shroom", Foreign), ("shroom-v", Ours)]), Some("shroom-v"));
        assert_eq!(pick_label(&[("v", Foreign), ("shroom", Foreign), ("shroom-v", Foreign)]), None);
        assert!(!WorkerFact::Foreign.takeable());
        assert!(WorkerFact::Absent.takeable() && WorkerFact::Ours.takeable());
    }

    #[test]
    fn names_lowercase_silently_and_drop_what_never_lands() {
        assert_eq!(clean_name("My Shroom!"), "myshroom");
        assert_eq!(clean_name("team-42"), "team-42");
        assert!(name_ok("myshroom") && name_ok("a") && name_ok("team-42"));
        assert!(!name_ok("") && !name_ok("-x") && !name_ok("x-") && !name_ok("Caps") && !name_ok(&"a".repeat(64)));
    }

    #[test]
    fn fenced_reads_of_foreign_workers_are_refused() {
        let c = Cloud::new("x", crate::Policy::TestNamespace { prefix: "shroom-test-".into() });
        assert!(matches!(worker_fact(&c, "acct", "v", "b"), Err(CloudError::Refused(_))));
        assert!(matches!(gates(&c, "acct", "probe"), Err(CloudError::Refused(_))));
    }
}
