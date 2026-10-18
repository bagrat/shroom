//! The Mac's client for the space's Worker (WORKER-INTERFACE §4): every route the app
//! calls — ping, peek, join, enroll, me, invites, members, removals, mint, key, library,
//! mark, health — and the shape rule (U2) that turns each answer into something the core
//! can route on.
//!
//! **The shape rule.** An answer is the space's own only when its body is JSON carrying
//! `"shroom":1`. Anything else — workers.dev's HTML 404, another Worker's JSON, a
//! Cloudflare error page, an older page's empty 405 — is [`Answer::NotShroom`]: the app
//! re-pings ([`Space::ping`]) and routes on that, never on the status code. A transport
//! failure is [`Answer::Unanswered`].
//!
//! No call sends an `Origin` header (the Worker refuses any request carrying one), and
//! nothing here logs: the device key rides only the `Authorization` header.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use shroom_storage::sigv4::{hmac, sha256_hex};

use crate::device::{DeviceKey, Previous, is_b14};

/// The oldest page interface this build's routes need (§6.10).
pub const IFACE_MIN: u32 = 2;

/// Normalizes a join link in any of its three forms — bare `host/join/<code>`,
/// `https://host/join/<code>`, `shroom://join/<host>/<code>` — to `(host, code)` (§6.2).
pub fn parse_link(link: &str) -> Option<(String, String)> {
    let s = link.trim();
    let (host, code) = if let Some(rest) = s.strip_prefix("shroom://join/") {
        rest.split_once('/')?
    } else {
        let rest = s.strip_prefix("https://").unwrap_or(s);
        let (host, path) = rest.split_once('/')?;
        (host, path.strip_prefix("join/")?)
    };
    let code = code.trim_end_matches('/');
    let host_ok = host.contains('.')
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        && !host.starts_with('.')
        && !host.ends_with('.');
    (host_ok && is_b14(code)).then(|| (host.to_ascii_lowercase(), code.to_string()))
}

/// Whether a URL macOS hands the app as it opens is taken as an invite (N50): the page's
/// Open shroom (`shroom:` — a non-join one is still reported, then ignored), and a web
/// address only when it reads as a join link (`open -a shroom <link>`). Any other is
/// ignored unread: shroom claims no web addresses, so only a local open can send one.
pub fn takes_opened(url: &str) -> bool {
    url.starts_with("shroom:") || (url.starts_with("https://") && parse_link(url).is_some())
}

/// The parent proof's `Authorization` value (§3.2): an HMAC under the parent's S3 secret
/// over a domain-separated message binding the method, host, path, time and body.
pub fn parent_proof(secret: &str, method: &str, host: &str, path: &str, ts: u64, body: &[u8]) -> String {
    let msg = format!("shroom-parent-v1\n{method}\n{host}\n{path}\n{ts}\n{}", sha256_hex(body));
    let sig = hmac(secret.as_bytes(), msg.as_bytes());
    format!("ShroomParent {ts}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig))
}

/// An error class the Worker names (§4's tables). `Other` keeps a class this build
/// doesn't know (a newer page), so it is logged rather than misread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    Unknown,
    Expired,
    Cancelled,
    Used,
    AlreadyHere,
    Conflict,
    Full,
    BadRequest,
    Origin,
    Unauthorized,
    Removed,
    Forbidden,
    NotYours,
    Deleted,
    Absent,
    TooMany,
    Busy,
    Space,
    Clock,
    StaleParent,
    Owner,
    SelfDevice,
    TokenHolder,
    NotOwnerMac,
    TooLarge,
    Method,
    NoRoute,
    Other(String),
}

impl Class {
    fn of(s: &str) -> Class {
        match s {
            "unknown" => Class::Unknown,
            "expired" => Class::Expired,
            "cancelled" => Class::Cancelled,
            "used" => Class::Used,
            "already-here" => Class::AlreadyHere,
            "conflict" => Class::Conflict,
            "full" => Class::Full,
            "bad-request" => Class::BadRequest,
            "origin" => Class::Origin,
            "unauthorized" => Class::Unauthorized,
            "removed" => Class::Removed,
            "forbidden" => Class::Forbidden,
            "not-yours" => Class::NotYours,
            "deleted" => Class::Deleted,
            "absent" => Class::Absent,
            "too-many" => Class::TooMany,
            "busy" => Class::Busy,
            "space" => Class::Space,
            "clock" => Class::Clock,
            "stale-parent" => Class::StaleParent,
            "owner" => Class::Owner,
            "self" => Class::SelfDevice,
            "token-holder" => Class::TokenHolder,
            "not-owner-mac" => Class::NotOwnerMac,
            "too-large" => Class::TooLarge,
            "method" => Class::Method,
            "no-route" => Class::NoRoute,
            other => Class::Other(other.to_string()),
        }
    }
}

/// A shroom-shaped refusal: the status, the class and its qualifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub status: u16,
    pub class: Class,
    /// `space`'s reason: `unwired` · `broken` · `storage` · `no-parent`.
    pub reason: Option<String>,
    /// `removed`'s why: `removed` · `left`.
    pub why: Option<String>,
    /// A cancel that lost to a join: the member who joined.
    pub member: Option<String>,
    /// `too-many`: seconds until the budget's next hour.
    pub retry_after: Option<u64>,
    /// The response's `Date` header — `clock`'s one-time offset correction.
    pub date: Option<String>,
}

impl Refusal {
    /// A space-side answer (§4.8): `space-needs-owner` after the debounce (C7).
    pub fn is_space(&self) -> bool {
        self.class == Class::Space
    }
    /// A transient answer: a silent backoff, never a face (busy, 429, the clock).
    pub fn is_transient(&self) -> bool {
        matches!(self.class, Class::Busy | Class::TooMany | Class::Clock) || self.status == 429
    }
    /// This identity is dead here (`/me` and mint: 401, or 403 `removed`).
    pub fn is_revoked(&self) -> bool {
        self.class == Class::Unauthorized || self.class == Class::Removed
    }
}

/// One call's answer, classified by the shape rule (U2).
#[derive(Debug, Clone, PartialEq)]
pub enum Answer<T> {
    Ok(T),
    /// The space's own refusal.
    Refused(Refusal),
    /// Not shroom-shaped, whatever the status: re-ping and route on the ping.
    NotShroom { status: u16 },
    /// No answer at all (offline, DNS, a timeout).
    Unanswered(String),
}

impl<T> Answer<T> {
    pub fn ok(self) -> Option<T> {
        match self {
            Answer::Ok(t) => Some(t),
            _ => None,
        }
    }
    pub fn refusal(&self) -> Option<&Refusal> {
        match self {
            Answer::Refused(r) => Some(r),
            _ => None,
        }
    }
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Answer<U> {
        match self {
            Answer::Ok(t) => Answer::Ok(f(t)),
            Answer::Refused(r) => Answer::Refused(r),
            Answer::NotShroom { status } => Answer::NotShroom { status },
            Answer::Unanswered(e) => Answer::Unanswered(e),
        }
    }
}

/// The ping (§4.2) — the one route that keeps `"shroom":true`.
#[derive(Debug, Clone, PartialEq)]
pub enum Ping {
    /// A shroom space, its page's interface and the version of the app that deployed it.
    Shroom { iface: u32, version: String },
    /// Answered, but not as a shroom space: workers.dev's 404 (`error code: 1042`), another
    /// Worker, an error page.
    NotShroom { status: u16 },
    Unanswered(String),
}

impl Ping {
    /// Whether this app's routes are all there (§6.10): a page behind reads the
    /// needs-update line; a page ahead is always fine.
    pub fn current_enough(&self) -> bool {
        matches!(self, Ping::Shroom { iface, .. } if *iface >= IFACE_MIN)
    }
}

/// `peek` (§4.4). Never the label.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Peek {
    /// `pending` · `expired` · `cancelled` · `used`.
    pub state: String,
    /// `member` · `own_mac`.
    pub kind: String,
    pub inviter: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
    pub now: String,
    pub iface: u32,
}

/// The space as the joining Mac stores it (§4.5).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct JoinedSpace {
    pub host: String,
    pub account_id: Option<String>,
    pub bucket: Option<String>,
    pub label: String,
    pub subdomain: Option<String>,
    pub owner_name: String,
    pub iface: u32,
}

/// `join`'s 200 (§4.5). The member never receives its own label.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Joined {
    pub kind: String,
    /// `member` · `owner` (an own-Mac link).
    pub role: String,
    pub member: String,
    pub device: String,
    pub inviter: String,
    pub space: JoinedSpace,
}

/// `enroll`'s 200 (§4.6).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Enrolled {
    pub member: String,
    pub device: String,
    pub iface: u32,
    pub token: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct You {
    pub member: String,
    pub device: String,
    pub role: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MeSpace {
    pub host: String,
    pub account_id: Option<String>,
    pub bucket: Option<String>,
    pub owner_name: Option<String>,
    pub iface: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OwnerFacts {
    pub mark: bool,
    #[serde(rename = "tokenHolders")]
    pub token_holders: u32,
    pub token: bool,
}

/// `me` (§4.7). `owner` only for owner-class Macs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Me {
    pub you: You,
    pub space: MeSpace,
    #[serde(default)]
    pub owner: Option<OwnerFacts>,
}

impl Me {
    /// The only-token-holder lines show when this Mac is counted alone (§4.7, U9).
    pub fn only_token_holder(&self) -> bool {
        self.owner.as_ref().is_some_and(|o| o.token && o.token_holders <= 1)
    }
}

/// An upload or manage credential (§4.8). `owner` is the ledger's member — written into
/// `meta.owner` for readability only.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Minted {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expires_at: String,
    pub bucket: String,
    pub endpoint: String,
    pub owner: String,
}

impl Minted {
    pub fn credentials(&self) -> shroom_storage::sigv4::Credentials {
        shroom_storage::sigv4::Credentials {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: Some(self.session_token.clone()),
        }
    }
    /// The file a take keeps it in (§5).
    pub fn cred_file(&self, purpose: Purpose) -> crate::device::CredFile {
        crate::device::CredFile {
            v: 1,
            purpose: purpose.as_str().into(),
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: self.session_token.clone(),
            expires_at: self.expires_at.clone(),
            bucket: self.bucket.clone(),
            endpoint: self.endpoint.clone(),
        }
    }
    /// The JWT inside the session token (`base64("jwt/" + jwt)`).
    pub fn jwt(&self) -> Option<String> {
        let raw = base64::engine::general_purpose::STANDARD.decode(&self.session_token).ok()?;
        String::from_utf8(raw).ok()?.strip_prefix("jwt/").map(String::from)
    }
}

impl std::fmt::Debug for Minted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Minted").field("access_key_id", &self.access_key_id).field("expires_at", &self.expires_at).field("owner", &self.owner).finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Seven days, a recording's bytes and record (capture start).
    Upload,
    /// Fifteen minutes, rename or delete one recording.
    Manage,
}

impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Purpose::Upload => "upload",
            Purpose::Manage => "manage",
        }
    }
}

/// `invites`' row (§4.12) and `members`' pending rows (§4.11).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InviteRow {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
    pub expired: bool,
}

#[derive(Deserialize)]
struct InviteAnswer {
    invite: InviteRow,
}

/// A new invite (§4.12). The code stays on the Mac; only its hash is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct NewInvite {
    pub id: String,
    pub code_hash: String,
    /// `member` · `own_mac`.
    pub kind: String,
    /// The invitee's admin-private label (member invites); `None` for own-Mac links.
    pub label: Option<String>,
    /// The inviter's macOS short name (N29).
    pub inviter: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OwnerMac {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "joinedAt")]
    pub joined_at: String,
    /// Keeps the Cloudflare token: no Remove… (SM:1393-1399).
    pub token: bool,
    #[serde(rename = "self")]
    pub this_mac: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OwnerRow {
    pub id: String,
    pub name: Option<String>,
    pub devices: Vec<OwnerMac>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MemberRow {
    pub id: String,
    pub label: String,
    #[serde(rename = "joinedAt")]
    pub joined_at: String,
}

/// `members` (§4.11): the Members face's rows, and every label ever for the Library.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Members {
    pub now: String,
    pub owner: OwnerRow,
    pub members: Vec<MemberRow>,
    pub invites: Vec<InviteRow>,
    pub labels: std::collections::BTreeMap<String, String>,
}

/// `library`'s scope (§4.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Mine,
    Everyone,
    /// The id → member map from the ledger alone.
    Owners,
}

/// One `library` call's request: the next slice of this Mac's cache (≤ 1,000 ids after
/// `after`, `known_through` naming the last of them).
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryRequest {
    pub scope: Scope,
    pub known: std::collections::BTreeMap<String, String>,
    pub known_through: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LibraryItem {
    pub id: String,
    pub etag: String,
    pub owner: String,
    /// Only when its ETag differs from `known`.
    #[serde(default)]
    pub meta: Option<Value>,
}

/// One page. The client computes "gone" as known minus every id seen, once `next` is null.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LibraryPage {
    #[serde(default)]
    pub items: Vec<LibraryItem>,
    #[serde(default)]
    pub owners: std::collections::BTreeMap<String, String>,
    pub next: Option<String>,
}

/// One page of `cleanup`, the page's own suggestions of what to delete (where the page makes
/// them: an older page answers 404). Read by the `shroom` command (spec N62, DECISIONS B215).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPage {
    /// Since when plays are counted; `None` before counting began.
    #[serde(default)]
    pub counting_since: Option<String>,
    #[serde(default)]
    pub items: Vec<CleanupItem>,
    pub next: Option<String>,
}

/// One live video, with how it's watched and whether it's worth a look before deleting.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub age_days: Option<u64>,
    #[serde(default)]
    pub duration_sec: Option<f64>,
    #[serde(default)]
    pub plays: Option<u64>,
    #[serde(default)]
    pub last_played_at: Option<String>,
    /// Days since it was last played (or made); `None` where counting can't tell yet.
    #[serde(default)]
    pub quiet_days: Option<u64>,
    /// The page suggests it.
    #[serde(default)]
    pub candidate: bool,
    /// Only a ranking key: higher is sooner.
    #[serde(default)]
    pub score: f64,
    /// `{code, …facts}`: the code is stable, the words are the reader's.
    #[serde(default)]
    pub reason: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct KeyGrant {
    pub exp: String,
}

#[derive(Deserialize)]
struct MarkAnswer {
    on: bool,
}

/// One space's Worker, by host.
pub struct Space {
    agent: ureq::Agent,
    host: String,
    base: String,
}

impl std::fmt::Debug for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Space").field("host", &self.host).finish()
    }
}

/// What a body-less call carries.
const EMPTY: &[u8] = b"";

impl Space {
    /// `https://<host>`.
    pub fn new(host: &str) -> Space {
        Space::at(host, &format!("https://{host}"))
    }

    /// A Worker reached at `base` but signed for `host` (a local test server).
    pub fn at(host: &str, base: &str) -> Space {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(60)))
            .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
            .build();
        Space { agent: config.into(), host: host.to_string(), base: base.trim_end_matches('/').to_string() }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// One request: `(status, body, Date)`, or the transport error.
    fn send(&self, method: &str, path: &str, body: &[u8], auth: Option<&str>) -> Result<(u16, Vec<u8>, Option<String>), String> {
        let mut req = ureq::http::Request::builder().method(method).uri(format!("{}{path}", self.base));
        if !body.is_empty() {
            req = req.header("content-type", "application/json");
        }
        if let Some(a) = auth {
            req = req.header("authorization", a);
        }
        let req = req.body(body.to_vec()).map_err(|e| e.to_string())?;
        let mut res = self.agent.run(req).map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let date = res.headers().get("date").and_then(|v| v.to_str().ok()).map(String::from);
        let bytes = res.body_mut().with_config().limit(4 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
        Ok((status, bytes, date))
    }

    /// The shape rule (U2).
    fn classify<T: DeserializeOwned>(sent: Result<(u16, Vec<u8>, Option<String>), String>) -> Answer<T> {
        let (status, bytes, date) = match sent {
            Ok(r) => r,
            Err(e) => return Answer::Unanswered(e),
        };
        let body: Value = if bytes.is_empty() && status == 204 { json!({"shroom": 1}) } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
        if body.get("shroom").and_then(Value::as_u64) != Some(1) {
            return Answer::NotShroom { status };
        }
        if (200..300).contains(&status) {
            return match serde_json::from_value::<T>(body) {
                Ok(t) => Answer::Ok(t),
                // Shroom-shaped but not the shape this build reads: treat as not ours.
                Err(_) => Answer::NotShroom { status },
            };
        }
        let text = |k: &str| body.get(k).and_then(Value::as_str).map(String::from);
        Answer::Refused(Refusal {
            status,
            class: Class::of(&text("error").unwrap_or_default()),
            reason: text("reason"),
            why: text("why"),
            member: text("member"),
            retry_after: body.get("retryAfter").and_then(Value::as_u64),
            date,
        })
    }

    fn call<T: DeserializeOwned>(&self, method: &str, route: &str, body: Option<Value>, key: Option<&DeviceKey>) -> Answer<T> {
        let bytes = body.map(|b| b.to_string().into_bytes()).unwrap_or_default();
        let auth = key.map(DeviceKey::bearer);
        Self::classify(self.send(method, &format!("/_shroom/api/{route}"), &bytes, auth.as_deref()))
    }

    /// `GET /_shroom/ping` (§4.2).
    pub fn ping(&self) -> Ping {
        match self.send("GET", "/_shroom/ping", EMPTY, None) {
            Err(e) => Ping::Unanswered(e),
            Ok((status, bytes, _)) => {
                let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                match (status, v.get("shroom").and_then(Value::as_bool)) {
                    (200, Some(true)) => Ping::Shroom {
                        iface: v.get("iface").and_then(Value::as_u64).unwrap_or(1) as u32,
                        version: v.get("version").and_then(Value::as_str).unwrap_or("0.0.0").to_string(),
                    },
                    _ => Ping::NotShroom { status },
                }
            }
        }
    }

    /// `POST peek` (§4.4): the code rides the body, never a URL.
    pub fn peek(&self, code: &str) -> Answer<Peek> {
        self.call("POST", "peek", Some(json!({"code": code})), None)
    }

    /// `POST join` (§4.5), from `join.json`'s facts — replayed identically after a crash.
    pub fn join(&self, f: &crate::device::JoinFile) -> Answer<Joined> {
        let previous = f.previous.as_ref().map(|Previous { id, secret }| json!({"id": id, "secret": secret}));
        let body = json!({
            "code": f.code,
            "device": {"id": f.device, "verifier": crate::device::hash(&f.secret), "name": f.name},
            "previous": previous,
        });
        self.call("POST", "join", Some(body), None)
    }

    /// `POST enroll` (§4.6) under a parent proof signed with `parent_secret` (the token's
    /// derived S3 secret) at `ts` (unix seconds, offset-corrected by the caller on `clock`).
    /// `machine` is this Mac's stamp in this space ([`machine_stamp`], DECISIONS B144): the
    /// same Mac set up again under a new key retires its old record.
    pub fn enroll(&self, parent_secret: &str, key: &DeviceKey, name: Option<&str>, machine: Option<&str>, owner_name: Option<&str>, ts: u64) -> Answer<Enrolled> {
        let path = "/_shroom/api/enroll";
        let mut device = json!({"id": key.device, "verifier": key.verifier(), "name": name});
        if let Some(m) = machine {
            device["machine"] = json!(m);
        }
        let body = json!({"device": device, "ownerName": owner_name}).to_string().into_bytes();
        let proof = parent_proof(parent_secret, "POST", &self.host, path, ts, &body);
        Self::classify(self.send("POST", path, &body, Some(&proof)))
    }

    /// `GET me` (§4.7): law 1's re-probe.
    pub fn me(&self, key: &DeviceKey) -> Answer<Me> {
        self.call("GET", "me", None, Some(key))
    }

    /// `POST mint` (§4.8).
    pub fn mint(&self, key: &DeviceKey, rec: &str, purpose: Purpose) -> Answer<Minted> {
        self.call("POST", "mint", Some(json!({"rec": rec, "purpose": purpose.as_str()})), Some(key))
    }

    /// `POST key` (§4.9): the grant behind `?k=` (send `device::hash(k)`, never `k`).
    pub fn key(&self, key: &DeviceKey, k_hash: &str) -> Answer<KeyGrant> {
        self.call("POST", "key", Some(json!({"kHash": k_hash})), Some(key))
    }

    /// `POST library` (§4.10): one page.
    pub fn library(&self, key: &DeviceKey, req: &LibraryRequest) -> Answer<LibraryPage> {
        let scope = match req.scope {
            Scope::Mine => "mine",
            Scope::Everyone => "everyone",
            Scope::Owners => "owners",
        };
        let body = json!({"scope": scope, "known": req.known, "knownThrough": req.known_through, "after": req.after});
        self.call("POST", "library", Some(body), Some(key))
    }

    /// `POST cleanup`: the page's suggestions of what to delete, `after` the last id of the
    /// page before. A page that doesn't make them answers 404.
    pub fn cleanup(&self, key: &DeviceKey, scope: Scope, after: Option<&str>) -> Answer<CleanupPage> {
        let mut body = json!({"scope": if scope == Scope::Mine { "mine" } else { "everyone" }});
        if let Some(a) = after {
            body["after"] = a.into();
        }
        self.call("POST", "cleanup", Some(body), Some(key))
    }

    /// `GET members` (§4.11), owner-class.
    pub fn members(&self, key: &DeviceKey) -> Answer<Members> {
        self.call("GET", "members", None, Some(key))
    }

    /// `POST invites` (§4.12): create. 201 (200 on a replay).
    pub fn invite(&self, key: &DeviceKey, i: &NewInvite) -> Answer<InviteRow> {
        let body = json!({"id": i.id, "codeHash": i.code_hash, "kind": i.kind, "label": i.label, "inviter": i.inviter});
        self.call::<InviteAnswer>("POST", "invites", Some(body), Some(key)).map(|a| a.invite)
    }

    /// `POST invites` with `renew` — [ New link ]: the same row, a new code.
    pub fn renew(&self, key: &DeviceKey, id: &str, code_hash: &str) -> Answer<InviteRow> {
        self.call::<InviteAnswer>("POST", "invites", Some(json!({"id": id, "codeHash": code_hash, "renew": true})), Some(key)).map(|a| a.invite)
    }

    /// `DELETE invites/<iid>` (§4.13).
    pub fn cancel_invite(&self, key: &DeviceKey, iid: &str) -> Answer<()> {
        self.call::<Value>("DELETE", &format!("invites/{iid}"), None, Some(key)).map(|_| ())
    }

    /// `DELETE members/<mid>` (§4.14).
    pub fn remove_member(&self, key: &DeviceKey, mid: &str) -> Answer<()> {
        self.call::<Value>("DELETE", &format!("members/{mid}"), None, Some(key)).map(|_| ())
    }

    /// `DELETE devices/<did>` (§4.15): another owner Mac.
    pub fn remove_device(&self, key: &DeviceKey, did: &str) -> Answer<()> {
        self.call::<Value>("DELETE", &format!("devices/{did}"), None, Some(key)).map(|_| ())
    }

    /// `DELETE devices/self` — Leave, Disconnect, the switch's deregistration (§16 #57).
    /// 401 / 403 `removed` count as already done (§4.15): a retry after a lost reply, or a
    /// Mac someone already removed, has nothing left to deregister.
    pub fn leave(&self, key: &DeviceKey) -> Answer<()> {
        Self::left(self.call::<Value>("DELETE", "devices/self", None, Some(key)).map(|_| ()))
    }

    fn left(a: Answer<()>) -> Answer<()> {
        match a {
            Answer::Refused(r) if r.is_revoked() => Answer::Ok(()),
            other => other,
        }
    }

    /// `POST mark` (§4.16), owner-class.
    pub fn mark(&self, key: &DeviceKey, on: bool) -> Answer<bool> {
        self.call::<MarkAnswer>("POST", "mark", Some(json!({"on": on})), Some(key)).map(|a| a.on)
    }

    /// `POST health` (§4.18): `refused` after the debounce's second answer, `ok` once the
    /// next PUT lands.
    pub fn health(&self, key: &DeviceKey, rec: &str, refused: bool) -> Answer<()> {
        let uploads = if refused { "refused" } else { "ok" };
        self.call::<Value>("POST", "health", Some(json!({"rec": rec, "uploads": uploads})), Some(key)).map(|_| ())
    }

    /// `POST gone` (DECISIONS B125): seals a deleted recording — the Worker records the
    /// deletion where no recording credential reaches, so the video's own upload
    /// credential can't bring it back. Only once the record reads as a tombstone.
    pub fn gone(&self, key: &DeviceKey, rec: &str) -> Answer<()> {
        self.call::<Value>("POST", "gone", Some(json!({"rec": rec})), Some(key)).map(|_| ())
    }
}

/// Where a join answer leads (§4.5's table; the same table classifies the boot replay of
/// `join.json`).
#[derive(Debug, Clone, PartialEq)]
pub enum JoinNext {
    Joined(Joined),
    /// `join-expired`: 404 `unknown`, 410 `expired` / `cancelled`. Clears `join.json`.
    Expired,
    /// `join-used`. Clears `join.json`.
    Used,
    /// `join-already-here`, worded by this Mac's role (U5). Clears `join.json`.
    AlreadyHere,
    /// The device id is taken: make a fresh one and retry.
    NewDevice,
    /// `join-waiting`: replay the identical body on reconnect.
    Waiting,
    /// Not shroom-shaped: re-ping (`join-space-missing`, or the needs-update face).
    Reping,
    /// A bug (400, 403 `origin`, 409 `full`, a class this build doesn't know): logged.
    Bug(Refusal),
}

impl JoinNext {
    pub fn of(a: Answer<Joined>) -> JoinNext {
        match a {
            Answer::Ok(j) => JoinNext::Joined(j),
            Answer::NotShroom { .. } => JoinNext::Reping,
            Answer::Unanswered(_) => JoinNext::Waiting,
            Answer::Refused(r) => match r.class {
                Class::Unknown | Class::Expired | Class::Cancelled => JoinNext::Expired,
                Class::Used => JoinNext::Used,
                Class::AlreadyHere => JoinNext::AlreadyHere,
                Class::Conflict => JoinNext::NewDevice,
                Class::Busy | Class::Space | Class::TooMany => JoinNext::Waiting,
                _ if r.status == 429 || r.status >= 500 => JoinNext::Waiting,
                _ => JoinNext::Bug(r),
            },
        }
    }

    /// Whether `join.json` is cleared (a shroom-shaped terminal answer or a 200).
    pub fn clears_join_file(&self) -> bool {
        matches!(self, JoinNext::Joined(_) | JoinNext::Expired | JoinNext::Used | JoinNext::AlreadyHere)
    }
}

/// Where a mint answer leads (§4.8's table).
#[derive(Debug, Clone, PartialEq)]
pub enum MintNext {
    Minted(Minted),
    /// 401 / 403 `removed`: `member-revoked` (or the removed owner Mac's face). Not debounced.
    Revoked,
    /// A logged anomaly: never re-mint this id; the take is kept.
    NotYours,
    /// Tombstone-before-resume: the take stops, "Removed from the space".
    Deleted,
    /// `manage` only: no record yet.
    Absent,
    /// A space-side answer: counts toward the debounce (two, 30 s apart → the face).
    SpaceSide(Refusal),
    /// `busy`, 429, transport: a silent backoff.
    Backoff,
    /// Not shroom-shaped: re-ping (`space-gone` if not shroom).
    Reping,
    /// 400 and anything unexpected: a bug, logged, never retried.
    Bug(Refusal),
}

impl MintNext {
    pub fn of(a: Answer<Minted>) -> MintNext {
        match a {
            Answer::Ok(m) => MintNext::Minted(m),
            Answer::NotShroom { .. } => MintNext::Reping,
            Answer::Unanswered(_) => MintNext::Backoff,
            Answer::Refused(r) => match r.class {
                Class::Unauthorized | Class::Removed => MintNext::Revoked,
                Class::NotYours => MintNext::NotYours,
                Class::Deleted => MintNext::Deleted,
                Class::Absent => MintNext::Absent,
                Class::Space => MintNext::SpaceSide(r),
                Class::Busy | Class::TooMany => MintNext::Backoff,
                _ if r.status == 429 => MintNext::Backoff,
                _ => MintNext::Bug(r),
            },
        }
    }
}

/// Where an enroll answer leads (§4.6).
#[derive(Debug, Clone, PartialEq)]
pub enum EnrollNext {
    Enrolled(Enrolled),
    /// Transient: correct the offset once from the `Date` header, then retry.
    Clock(Option<String>),
    /// `stale-parent` or `no-parent`: a Mac whose token is live re-places `R2_PARENT`,
    /// then enrolls once more (H1).
    ReplaceParent,
    /// `unwired`: seed the roster create-only, then enroll again.
    Seed,
    /// `broken`: the repair pass's roster recovery (C8).
    Broken,
    /// A different verifier, or a member's device: this Mac needs a fresh device id.
    Conflict,
    /// `busy`, or a 429 (the Worker's or Cloudflare's, shroom-shaped or not): a silent
    /// backoff, retried — never the repair mood, never a re-wire (§3.8, §4.8).
    Transient,
    /// Storage, or anything else space-side: the repair mood.
    Repair(Refusal),
    Reping,
    Unanswered(String),
}

impl EnrollNext {
    pub fn of(a: Answer<Enrolled>) -> EnrollNext {
        match a {
            Answer::Ok(e) => EnrollNext::Enrolled(e),
            Answer::NotShroom { status: 429 } => EnrollNext::Transient,
            Answer::NotShroom { .. } => EnrollNext::Reping,
            Answer::Unanswered(e) => EnrollNext::Unanswered(e),
            Answer::Refused(r) => match (&r.class, r.reason.as_deref()) {
                (Class::Clock, _) => EnrollNext::Clock(r.date),
                (Class::Busy | Class::TooMany, _) => EnrollNext::Transient,
                _ if r.status == 429 => EnrollNext::Transient,
                (Class::StaleParent, _) | (Class::Space, Some("no-parent")) => EnrollNext::ReplaceParent,
                (Class::Space, Some("unwired")) => EnrollNext::Seed,
                (Class::Space, Some("broken")) => EnrollNext::Broken,
                (Class::Conflict, _) => EnrollNext::Conflict,
                _ => EnrollNext::Repair(r),
            },
        }
    }
}

/// Where `/me` leads (§4.7).
#[derive(Debug, Clone, PartialEq)]
pub enum MeNext {
    Me(Me),
    /// 401 / 403 `removed`: `member-revoked` on a member's Mac; the removed-Mac face on an
    /// owner-class one (U8). Kept `device.key` proves `previous` on a re-invite.
    Dead,
    /// 503: `space-needs-owner`, debounced (C7).
    SpaceSide(Refusal),
    Reping,
    /// Transport: keep the cached identity.
    Keep,
}

impl MeNext {
    pub fn of(a: Answer<Me>) -> MeNext {
        match a {
            Answer::Ok(m) => MeNext::Me(m),
            Answer::NotShroom { .. } => MeNext::Reping,
            Answer::Unanswered(_) => MeNext::Keep,
            Answer::Refused(r) if r.is_revoked() => MeNext::Dead,
            Answer::Refused(r) => MeNext::SpaceSide(r),
        }
    }
}

/// This Mac's stamp in one space (DECISIONS B144): sha-256 over the hardware's id, the
/// macOS account's home folder and the space's host — so the space can tell the same Mac set
/// up again, two accounts on one Mac stay apart, and the hardware's own id never leaves.
pub fn machine_stamp(hardware: &str, home: &str, host: &str) -> String {
    crate::device::hash(&format!("shroom-machine-v1\n{hardware}\n{home}\n{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_join_link_parses_in_all_three_forms() {
        let want = Some(("v.myshroom.workers.dev".to_string(), "AbCdEfGhIjKlMnOp".to_string()));
        assert_eq!(parse_link("v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp"), want);
        assert_eq!(parse_link(" https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp\n"), want);
        assert_eq!(parse_link("shroom://join/v.myshroom.workers.dev/AbCdEfGhIjKlMnOp"), want);
        assert_eq!(parse_link("https://V.MyShroom.workers.dev/join/AbCdEfGhIjKlMnOp/"), want);
        for bad in ["v.myshroom.workers.dev/join/7xk2pm", "http://v.myshroom.workers.dev/AbCdEfGhIjKlMnOp", "localhost/join/AbCdEfGhIjKlMnOp",
            "v.myshroom.workers.dev/join/AbCdEfGh/jKlMnOp", "ev il.example/join/AbCdEfGhIjKlMnOp", ""] {
            assert_eq!(parse_link(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_opened_url_is_taken_only_as_a_join_link() {
        // N50: what macOS hands the app as it opens. Open shroom's scheme is always taken
        // (a non-join one is reported, then ignored); a web address only as a join link.
        for take in ["https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp", "https://V.MyShroom.workers.dev/join/AbCdEfGhIjKlMnOp/",
            "shroom://join/v.myshroom.workers.dev/AbCdEfGhIjKlMnOp", "shroom://join/short"] {
            assert!(takes_opened(take), "{take}");
        }
        for leave in ["https://shroom.am/", "https://v.myshroom.workers.dev/AbCdEfGhIjKlMnOp", "https://v.myshroom.workers.dev/join/7xk2pm",
            "https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp?x=1", "http://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp",
            "file:///Users/x/v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp", "v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp", ""] {
            assert!(!takes_opened(leave), "{leave}");
        }
    }

    #[test]
    fn the_parent_proof_is_domain_separated_and_bound_to_the_request() {
        let p = parent_proof("secret", "POST", "v.example.workers.dev", "/_shroom/api/enroll", 1_790_942_400, b"{}");
        assert!(p.starts_with("ShroomParent 1790942400."));
        let msg = format!("shroom-parent-v1\nPOST\nv.example.workers.dev\n/_shroom/api/enroll\n1790942400\n{}", sha256_hex(b"{}"));
        let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hmac(b"secret", msg.as_bytes()));
        assert_eq!(p, format!("ShroomParent 1790942400.{sig}"));
        assert_ne!(p, parent_proof("secret", "POST", "v.example.workers.dev", "/_shroom/api/enroll", 1_790_942_400, b"{ }"));
    }

    fn answer<T: DeserializeOwned>(status: u16, body: &str) -> Answer<T> {
        Space::classify(Ok((status, body.as_bytes().to_vec(), Some("Fri, 02 Oct 2026 12:00:00 GMT".into()))))
    }

    #[test]
    fn the_shape_rule_classes_only_shroom_answers() {
        // workers.dev's answer for a missing Worker (B35 P7), another Worker, an old page's 405.
        assert_eq!(answer::<Peek>(404, "error code: 1042"), Answer::NotShroom { status: 404 });
        assert_eq!(answer::<Peek>(404, r#"{"error":"unknown"}"#), Answer::NotShroom { status: 404 });
        assert_eq!(answer::<Peek>(405, ""), Answer::NotShroom { status: 405 });
        assert_eq!(answer::<Peek>(200, r#"{"shroom":true}"#), Answer::NotShroom { status: 200 });
        let Answer::Refused(r) = answer::<Peek>(404, r#"{"shroom":1,"error":"unknown"}"#) else { panic!() };
        assert_eq!((r.status, r.class), (404, Class::Unknown));
        let Answer::Refused(r) = answer::<Me>(403, r#"{"shroom":1,"error":"removed","why":"left"}"#) else { panic!() };
        assert!(r.is_revoked());
        assert_eq!(r.why.as_deref(), Some("left"));
        let Answer::Refused(r) = answer::<Minted>(429, r#"{"shroom":1,"error":"too-many","retryAfter":1200}"#) else { panic!() };
        assert_eq!((r.retry_after, r.is_transient()), (Some(1200), true));
        assert_eq!(answer::<Value>(204, ""), Answer::Ok(json!({"shroom": 1})));
        assert!(matches!(Space::classify::<Peek>(Err("dns".into())), Answer::Unanswered(_)));
    }

    #[test]
    fn join_answers_lead_where_the_table_says() {
        let refused = |status, error: &str| answer::<Joined>(status, &format!(r#"{{"shroom":1,"error":"{error}"}}"#));
        assert_eq!(JoinNext::of(refused(404, "unknown")), JoinNext::Expired);
        assert_eq!(JoinNext::of(refused(410, "cancelled")), JoinNext::Expired);
        assert_eq!(JoinNext::of(refused(410, "expired")), JoinNext::Expired);
        assert_eq!(JoinNext::of(refused(409, "used")), JoinNext::Used);
        assert_eq!(JoinNext::of(refused(409, "already-here")), JoinNext::AlreadyHere);
        assert_eq!(JoinNext::of(refused(409, "conflict")), JoinNext::NewDevice);
        assert_eq!(JoinNext::of(refused(503, "busy")), JoinNext::Waiting);
        assert_eq!(JoinNext::of(answer(404, "<html>Not found</html>")), JoinNext::Reping);
        assert!(matches!(JoinNext::of(refused(400, "bad-request")), JoinNext::Bug(_)));
        assert!(JoinNext::Expired.clears_join_file() && !JoinNext::Waiting.clears_join_file() && !JoinNext::Reping.clears_join_file());
    }

    #[test]
    fn a_machine_stamp_is_one_mac_one_account_one_space_and_never_the_hardware_id() {
        let hw = "1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D";
        let s = machine_stamp(hw, "/Users/b", "v.myshroom.workers.dev");
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(s, machine_stamp(hw, "/Users/b", "v.myshroom.workers.dev"), "the same Mac, set up again");
        assert_ne!(s, machine_stamp(hw, "/Users/ani", "v.myshroom.workers.dev"), "another account on it");
        assert_ne!(s, machine_stamp(hw, "/Users/b", "v.other.workers.dev"), "another space");
        assert!(!s.contains("1a2b3c4d") && !s.contains(hw));
    }

    #[test]
    fn mint_enroll_and_me_answers_lead_where_the_tables_say() {
        let mint = |status, body: &str| MintNext::of(answer(status, body));
        assert_eq!(mint(403, r#"{"shroom":1,"error":"removed","why":"removed"}"#), MintNext::Revoked);
        assert_eq!(mint(401, r#"{"shroom":1,"error":"unauthorized"}"#), MintNext::Revoked);
        assert_eq!(mint(403, r#"{"shroom":1,"error":"not-yours"}"#), MintNext::NotYours);
        assert_eq!(mint(410, r#"{"shroom":1,"error":"deleted"}"#), MintNext::Deleted);
        assert!(matches!(mint(503, r#"{"shroom":1,"error":"space","reason":"no-parent"}"#), MintNext::SpaceSide(_)));
        assert_eq!(mint(429, r#"{"shroom":1,"error":"too-many","retryAfter":5}"#), MintNext::Backoff);
        assert_eq!(mint(502, "bad gateway"), MintNext::Reping);

        let enroll = |status, body: &str| EnrollNext::of(answer(status, body));
        assert_eq!(enroll(401, r#"{"shroom":1,"error":"stale-parent"}"#), EnrollNext::ReplaceParent);
        assert_eq!(enroll(503, r#"{"shroom":1,"error":"space","reason":"no-parent"}"#), EnrollNext::ReplaceParent);
        assert_eq!(enroll(503, r#"{"shroom":1,"error":"space","reason":"unwired"}"#), EnrollNext::Seed);
        assert_eq!(enroll(503, r#"{"shroom":1,"error":"space","reason":"broken"}"#), EnrollNext::Broken);
        assert_eq!(enroll(401, r#"{"shroom":1,"error":"clock"}"#), EnrollNext::Clock(Some("Fri, 02 Oct 2026 12:00:00 GMT".into())));
        assert_eq!(enroll(503, r#"{"shroom":1,"error":"busy"}"#), EnrollNext::Transient);
        assert_eq!(enroll(429, "error code: 1015"), EnrollNext::Transient);
        assert_eq!(enroll(502, "bad gateway"), EnrollNext::Reping);
        assert!(matches!(enroll(503, r#"{"shroom":1,"error":"space","reason":"storage"}"#), EnrollNext::Repair(_)));

        // Leave: 401 / 403 removed count as already done (§4.15).
        assert_eq!(Space::left(answer(403, r#"{"shroom":1,"error":"removed","why":"left"}"#)), Answer::Ok(()));
        assert_eq!(Space::left(answer(401, r#"{"shroom":1,"error":"unauthorized"}"#)), Answer::Ok(()));
        assert!(matches!(Space::left(answer(503, r#"{"shroom":1,"error":"busy"}"#)), Answer::Refused(_)));

        assert_eq!(MeNext::of(answer(403, r#"{"shroom":1,"error":"removed","why":"removed"}"#)), MeNext::Dead);
        assert!(matches!(MeNext::of(answer(503, r#"{"shroom":1,"error":"space","reason":"storage"}"#)), MeNext::SpaceSide(_)));
        assert_eq!(MeNext::of(Space::classify(Err("offline".into()))), MeNext::Keep);
    }

    #[test]
    fn a_minted_credential_never_prints_its_secrets() {
        let m = Minted { access_key_id: "id".into(), secret_access_key: "SECRET".into(), session_token: base64::engine::general_purpose::STANDARD.encode("jwt/a.b.c"),
            expires_at: "x".into(), bucket: "b".into(), endpoint: "e".into(), owner: "m1".into() };
        let printed = format!("{m:?}");
        assert!(!printed.contains("SECRET") && !printed.contains(&m.session_token));
        assert_eq!(m.jwt().as_deref(), Some("a.b.c"));
    }
}
