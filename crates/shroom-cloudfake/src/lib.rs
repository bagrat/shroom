//! L1 (TESTING-PLAN §5): an in-process stand-in for the slice of Cloudflare's REST API
//! and R2's S3 API that shroom uses, so the wizard's paths that must never run on a real
//! account — claiming a name, a name that's taken, the email and card gates, a space
//! that's already there — run on every change. Response shapes follow what the live
//! tests see on the test account (`tests/e2e-live`, whose `contract.rs` holds this fake to
//! them, and the recorded answers in `shroom-cloud/tests/fixtures/`): the v4 envelope with
//! numeric error codes.
//!
//! Tests only. Plain HTTP on 127.0.0.1, one request per connection, no auth checks
//! beyond the token's presence.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// The account as Cloudflare would describe it. Tests set it up, then read it back.
#[derive(Debug, Clone)]
pub struct State {
    pub accounts: Vec<(String, String)>,
    pub token_id: String,
    pub token_active: bool,
    /// Workers Scripts and R2 Storage.
    pub powers: (bool, bool),
    pub email_verified: bool,
    pub r2_enabled: bool,
    pub subdomain: Option<String>,
    /// Names someone else already has.
    pub taken: BTreeSet<String>,
    /// Worker name → its bindings, as deployed.
    pub scripts: BTreeMap<String, Vec<Value>>,
    /// Workers with their workers.dev route on.
    pub routes: BTreeSet<String>,
    /// Bucket → key → (bytes, etag).
    pub buckets: BTreeMap<String, BTreeMap<String, (Vec<u8>, u64)>>,
    /// "METHOD /path" for every request, in order.
    pub calls: Vec<String>,
    /// The last ETag handed out.
    pub etags: u64,
    /// Buckets deleted from the dashboard (`drop-bucket`), kept to put back.
    pub dropped: BTreeMap<String, BTreeMap<String, (Vec<u8>, u64)>>,
    /// Worker name → the `VERSION` its deployed module carries (`worker.js`'s constant), as
    /// its page's ping reports it.
    pub versions: BTreeMap<String, String>,
    /// Worker name → the secrets placed through the script-secrets endpoint (names only).
    pub secrets: BTreeMap<String, BTreeSet<String>>,
    /// workers.dev propagation (RESEARCH L2005, ~40 s on a fresh name): a route switched on
    /// answers this many pings with workers.dev's own 404 before its page does. The knob
    /// `propagation <n>`; applies to routes switched on afterwards.
    pub propagation: u32,
    /// Route → pings still to answer as not-yet-there.
    pub pending: BTreeMap<String, u32>,
    /// Pings answered, by Worker name (any answer).
    pub pings: BTreeMap<String, u32>,
    /// The transport fault seam (TESTING-PLAN §5 L3, DECISIONS B64): what the network does
    /// to the next matching requests, in order — see [`Fault`] and the `fault` knob.
    pub faults: Vec<Fault>,
    /// Buckets whose recording bytes (`rec/…`) are counted, not kept — the soak's hours of
    /// video would otherwise sit in this process's memory. Their objects list at their real
    /// size and read back empty. The knob `sink <bucket>`.
    pub sink: BTreeSet<String>,
    /// The real size of each object a sink didn't keep.
    pub sunk: BTreeMap<(String, String), u64>,
    /// Multipart uploads in progress (S3's `?uploads` → parts → complete), by upload id: a
    /// single-file copy past `SINGLE_PUT_MAX` (256 MB — a take of roughly an hour and a
    /// half) goes up this way, as R2 takes it (DECISIONS B70).
    pub multipart: BTreeMap<String, Multipart>,
    /// The page stand-in that runs the account's Workers (`worker/dev/serve.mjs`): when
    /// set, an `R2_PARENT` placed through the script-secrets endpoint reaches that space's
    /// binding (`POST <relay>/_dev/space/<label>.<name>.workers.dev/parent`), as Cloudflare
    /// hands a secret to the Worker it was placed on — so a drive can hold a stale binding
    /// and watch the app re-place it (DECISIONS B90). Loopback only. The knob
    /// `relay-parent <http://127.0.0.1:port>`.
    pub relay: Option<String>,
}

/// One multipart upload: where it lands, and its parts by number — (bytes, size, ETag);
/// a sink's parts keep only their size.
#[derive(Debug, Clone, Default)]
pub struct Multipart {
    pub bucket: String,
    pub key: String,
    pub parts: BTreeMap<u32, (Vec<u8>, u64, String)>,
}

/// What the network does to a request (L3's fault seam, at the transport, inside this fake:
/// no proxy, no system change). Every fault the fake applies is logged in `calls` as
/// `FAULT <kind> <METHOD> <path>`.
#[derive(Debug, Clone, PartialEq)]
pub enum FaultKind {
    /// The connection closes once the request is in, before anything is done with it.
    Drop,
    /// The request is done — the object written — and the reply never comes back.
    LostReply,
    /// The connection resets halfway through the request's body: nothing is done.
    ResetMidBody,
    /// The answer comes this many milliseconds late (the request then goes through).
    Delay(u64),
    /// The edge answers this status without doing anything: a 5xx (Cloudflare busy, an
    /// origin hiccup), or a 4xx a credential gone stale answers.
    Status(u16),
}

/// One rule of the seam: a kind, which requests (a method or any, a path prefix), and how
/// many more it applies to (`None`: every one).
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    pub kind: FaultKind,
    pub method: Option<String>,
    pub prefix: String,
    pub left: Option<u32>,
}

impl FaultKind {
    pub fn parse(s: &str) -> Option<FaultKind> {
        Some(match s {
            "drop" => FaultKind::Drop,
            "lost-reply" => FaultKind::LostReply,
            "reset" => FaultKind::ResetMidBody,
            _ if s.starts_with("delay:") => FaultKind::Delay(s["delay:".len()..].parse().ok()?),
            _ => match s.parse::<u16>() {
                Ok(code) if (400..600).contains(&code) => FaultKind::Status(code),
                _ => return None,
            },
        })
    }

    fn name(&self) -> String {
        match self {
            FaultKind::Drop => "drop".into(),
            FaultKind::LostReply => "lost-reply".into(),
            FaultKind::ResetMidBody => "reset".into(),
            FaultKind::Delay(ms) => format!("delay:{ms}"),
            FaultKind::Status(c) => c.to_string(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            accounts: vec![("acc0000000000000000000000000001".into(), "Bagrat's Account".into())],
            token_id: "tok00000000000000000000000000001".into(),
            token_active: true,
            powers: (true, true),
            email_verified: true,
            r2_enabled: true,
            subdomain: None,
            taken: BTreeSet::new(),
            scripts: BTreeMap::new(),
            routes: BTreeSet::new(),
            buckets: BTreeMap::new(),
            calls: Vec::new(),
            etags: 0,
            dropped: BTreeMap::new(),
            versions: BTreeMap::new(),
            secrets: BTreeMap::new(),
            propagation: 0,
            pending: BTreeMap::new(),
            pings: BTreeMap::new(),
            faults: Vec::new(),
            sink: BTreeSet::new(),
            sunk: BTreeMap::new(),
            multipart: BTreeMap::new(),
            relay: None,
        }
    }
}

impl State {
    /// A fresh account: verified, card on file, no name yet.
    pub fn fresh() -> State {
        State::default()
    }

    pub fn account_id(&self) -> String {
        self.accounts[0].0.clone()
    }

    pub fn object(&self, bucket: &str, key: &str) -> Option<Vec<u8>> {
        self.buckets.get(bucket)?.get(key).map(|(b, _)| b.clone())
    }

    /// What a person does in the dashboard meanwhile — the fake's knobs, one word each:
    /// `clear-gates` (email verified, R2 on), `email-gate`, `card-gate` (R2 off, as when
    /// the card is removed), `drop-bucket <name>` (the bucket deleted with everything in
    /// it), `restore-bucket <name>` (put back as it was).
    pub fn control(&mut self, cmd: &str) -> Result<(), String> {
        let mut words = cmd.split_whitespace();
        match (words.next(), words.next()) {
            (Some("clear-gates"), None) => {
                self.email_verified = true;
                self.r2_enabled = true;
            }
            (Some("email-gate"), None) => self.email_verified = false,
            (Some("card-gate"), None) => self.r2_enabled = false,
            (Some("drop-bucket"), Some(name)) => {
                let b = self.buckets.remove(name).ok_or_else(|| format!("no bucket {name:?}"))?;
                self.dropped.insert(name.into(), b);
            }
            (Some("restore-bucket"), Some(name)) => {
                let b = self.dropped.remove(name).ok_or_else(|| format!("no dropped bucket {name:?}"))?;
                self.buckets.insert(name.into(), b);
            }
            (Some("propagation"), Some(n)) => self.propagation = n.parse().map_err(|_| format!("not a count: {n:?}"))?,
            // `fault <drop|lost-reply|reset|delay:<ms>|4xx|5xx> <METHOD|*> <path prefix> <count|always>`
            (Some("fault"), Some(kind)) => {
                let kind = FaultKind::parse(kind).ok_or_else(|| format!("not a fault: {kind:?}"))?;
                let (Some(method), Some(prefix), Some(count), None) = (words.next(), words.next(), words.next(), words.next()) else {
                    return Err("fault <kind> <METHOD|*> <path prefix> <count|always>".into());
                };
                let method = (method != "*").then(|| method.to_uppercase());
                let left = if count == "always" { None } else { Some(count.parse().map_err(|_| format!("not a count: {count:?}"))?) };
                self.faults.push(Fault { kind, method, prefix: prefix.into(), left });
            }
            (Some("faults-clear"), None) => self.faults.clear(),
            (Some("sink"), Some(bucket)) => {
                self.sink.insert(bucket.into());
            }
            (Some("relay-parent"), Some(url)) => {
                let base = url.trim_end_matches('/');
                let host = base.strip_prefix("http://").ok_or_else(|| format!("not a plain http URL: {url:?}"))?;
                if !(host.starts_with("127.0.0.1:") || host.starts_with("localhost:")) {
                    return Err(format!("the relay is loopback only: {url:?}"));
                }
                self.relay = Some(base.into());
            }
            _ => return Err(format!("unknown command {cmd:?}")),
        }
        Ok(())
    }

    /// The first fault that applies to this request, used up by one (the seam's order).
    pub fn take_fault(&mut self, method: &str, path: &str) -> Option<FaultKind> {
        let i = self.faults.iter().position(|f| f.method.as_deref().is_none_or(|m| m == method) && path.starts_with(&f.prefix) && f.left != Some(0))?;
        let f = &mut self.faults[i];
        if let Some(n) = f.left.as_mut() {
            *n -= 1;
        }
        let kind = f.kind.clone();
        if f.left == Some(0) {
            self.faults.remove(i);
        }
        self.calls.push(format!("FAULT {} {method} {path}", kind.name()));
        Some(kind)
    }

    /// How many requests the seam has faulted.
    pub fn faulted(&self) -> usize {
        self.calls.iter().filter(|c| c.starts_with("FAULT ")).count()
    }

    pub fn put_object(&mut self, bucket: &str, key: &str, bytes: &[u8]) {
        self.etags += 1;
        let e = self.etags;
        self.buckets.entry(bucket.into()).or_default().insert(key.into(), (bytes.to_vec(), e));
    }
}

pub struct Fake {
    pub state: Arc<Mutex<State>>,
    addr: String,
}

impl Fake {
    pub fn start(state: State) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
        let addr = listener.local_addr().unwrap().to_string();
        let state = Arc::new(Mutex::new(state));
        let shared = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let s = shared.clone();
                std::thread::spawn(move || {
                    let _ = serve(stream, &s);
                });
            }
        });
        Fake { state, addr }
    }

    /// Where `Cloud::with_api` points.
    pub fn api(&self) -> String {
        format!("http://{}/client/v4", self.addr)
    }

    /// Where the bucket client points (path-style, like R2's endpoint).
    pub fn r2(&self) -> String {
        format!("http://{}/s3", self.addr)
    }

    /// Where `https://<host>` answers: the account's workers.dev pages, as far as wiring
    /// reads them — the ping (`member::Space::at(host, &fake.page(host))`).
    pub fn page(&self, host: &str) -> String {
        format!("http://{}/page/{host}", self.addr)
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut State) -> T) -> T {
        f(&mut self.state.lock().unwrap())
    }
}

struct Request {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn serve(stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut headers = BTreeMap::new();
    loop {
        let mut h = String::new();
        reader.read_line(&mut h)?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    // The fault seam: knobs themselves are never faulted.
    let raw_path = decode(target.split_once('?').map_or(target.as_str(), |(p, _)| p));
    let fault = if raw_path.starts_with("/_fake/") { None } else { state.lock().unwrap().take_fault(&method, &raw_path) };
    if fault == Some(FaultKind::ResetMidBody) {
        let mut half = vec![0; len / 2];
        let _ = reader.read_exact(&mut half);
        // Closed with the rest of the body still arriving: the kernel answers it with a
        // TCP reset, as a path that drops mid-transfer does.
        drop(reader);
        drop(stream);
        return Ok(());
    }
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    match fault {
        Some(FaultKind::Drop) => {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            return Ok(());
        }
        Some(FaultKind::Delay(ms)) => std::thread::sleep(std::time::Duration::from_millis(ms)),
        Some(FaultKind::Status(code)) => {
            let out = format!("<Error><Code>InternalError</Code><Message>the fault seam's {code}</Message></Error>");
            let mut w = stream;
            write!(w, "HTTP/1.1 {code} X\r\ncontent-type: application/xml\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{out}", out.len())?;
            return w.flush();
        }
        _ => {}
    }
    let (path, q) = target.split_once('?').unwrap_or((&target, ""));
    let query = q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (k.to_string(), decode(v))
    }).collect();
    let req = Request { method, path: decode(path), query, headers, body };
    let (status, ctype, out, extra) = {
        let mut s = state.lock().unwrap();
        s.calls.push(format!("{} {}", req.method, req.path));
        route(&mut s, &req)
    };
    if fault == Some(FaultKind::LostReply) {
        let _ = stream.shutdown(std::net::Shutdown::Both);
        return Ok(());
    }
    let mut w = stream;
    let mut head = format!("HTTP/1.1 {status} X\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\nconnection: close\r\n", out.len());
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    w.write_all(head.as_bytes())?;
    if req.method != "HEAD" {
        w.write_all(&out)?;
    }
    w.flush()
}

/// One bare HTTP POST to the loopback relay (the fake has no client dependency): its
/// status, or why it couldn't be made.
fn relay_post(base: &str, path: &str, body: &[u8]) -> Result<u16, String> {
    let addr = base.trim_start_matches("http://");
    let mut c = TcpStream::connect(addr).map_err(|e| format!("unreachable: {e}"))?;
    c.set_read_timeout(Some(std::time::Duration::from_secs(5))).map_err(|e| e.to_string())?;
    write!(c, "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    c.write_all(body).map_err(|e| e.to_string())?;
    let mut out = String::new();
    c.read_to_string(&mut out).map_err(|e| e.to_string())?;
    out.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or_else(|| "no status".to_string())
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

type Reply = (u16, &'static str, Vec<u8>, Vec<(String, String)>);

fn ok(result: Value) -> Reply {
    (200, "application/json", json!({"success": true, "errors": [], "messages": [], "result": result}).to_string().into_bytes(), vec![])
}

fn err(status: u16, code: i64, message: &str) -> Reply {
    (status, "application/json", json!({"success": false, "errors": [{"code": code, "message": message}], "messages": [], "result": null}).to_string().into_bytes(), vec![])
}

fn route(s: &mut State, r: &Request) -> Reply {
    if let Some(rest) = r.path.strip_prefix("/s3/") {
        return s3(s, r, rest);
    }
    if let Some(rest) = r.path.strip_prefix("/page/") {
        return page(s, rest);
    }
    // The dashboard's knobs, for a drive that runs the real app against this fake:
    // `POST /_fake/<command>` (spaces as `+` or `%20`) — see `State::control`.
    if let Some(cmd) = r.path.strip_prefix("/_fake/") {
        // What the fault seam has done so far, for a runner to hold a run to.
        if r.method == "GET" && cmd == "faulted" {
            let kinds: Vec<&str> = s.calls.iter().filter_map(|c| c.strip_prefix("FAULT ")).filter_map(|c| c.split_whitespace().next()).collect();
            return ok(json!({"faulted": kinds.len(), "kinds": kinds, "pending": s.faults.len()}));
        }
        // Every request so far, in order ("METHOD /path", the relay's deliveries, the seam's
        // faults) — no bodies, so no secret: a drive holds a run to what it did and didn't call.
        if r.method == "GET" && cmd == "calls" {
            return ok(json!({"calls": s.calls}));
        }
        if r.method != "POST" {
            return err(405, 0, "POST a command");
        }
        return match s.control(cmd) {
            Ok(()) => ok(json!({"done": cmd})),
            Err(e) => err(400, 0, &e),
        };
    }
    let Some(api) = r.path.strip_prefix("/client/v4") else { return err(404, 7003, "Could not route") };
    if !r.headers.get("authorization").is_some_and(|a| a.starts_with("Bearer ")) {
        return err(400, 6003, "Invalid request headers");
    }
    if !s.token_active {
        return err(401, 1000, "Invalid API Token");
    }
    let seg: Vec<&str> = api.trim_start_matches('/').split('/').collect();
    match (r.method.as_str(), seg.as_slice()) {
        ("GET", ["accounts"]) => ok(Value::Array(s.accounts.iter().map(|(id, name)| json!({"id": id, "name": name})).collect())),
        ("GET", ["user", "tokens", "verify"]) | ("GET", ["accounts", _, "tokens", "verify"]) => {
            ok(json!({"id": s.token_id, "status": "active"}))
        }
        (_, ["accounts", acc, rest @ ..]) if s.accounts.iter().any(|(id, _)| id == acc) => account(s, r, rest),
        (_, ["accounts", ..]) => err(403, 9109, "Unauthorized to access requested resource"),
        _ => err(404, 7003, "Could not route to the requested path"),
    }
}

fn account(s: &mut State, r: &Request, rest: &[&str]) -> Reply {
    let workers = |s: &State| if s.powers.0 { None } else { Some(err(403, 10000, "Authentication error")) };
    let r2 = |s: &State| {
        if !s.powers.1 {
            Some(err(403, 10000, "Authentication error"))
        } else if !s.r2_enabled {
            Some(err(403, 10042, "Please enable R2 through the Cloudflare Dashboard."))
        } else {
            None
        }
    };
    let unverified = |s: &State| if s.email_verified { None } else { Some(err(403, 8000077, "Your user email must been verified")) };
    match (r.method.as_str(), rest) {
        ("GET", ["workers", "scripts"]) => workers(s).unwrap_or_else(|| ok(Value::Array(s.scripts.keys().map(|n| json!({"id": n})).collect()))),
        ("PUT", ["workers", "scripts", name]) => {
            if let Some(e) = workers(s).or_else(|| unverified(s)) {
                return e;
            }
            let bindings = metadata(&r.body).and_then(|m| m.get("bindings").cloned()).and_then(|b| b.as_array().cloned()).unwrap_or_default();
            // Bindings to a bucket that doesn't exist fail the deploy, as on Cloudflare.
            for b in &bindings {
                if let Some(bucket) = b.get("bucket_name").and_then(Value::as_str) {
                    if !s.buckets.contains_key(bucket) {
                        return err(400, 10181, "R2 bucket not found");
                    }
                }
            }
            let shown: Vec<Value> = bindings.into_iter().map(|mut b| {
                if let Some(o) = b.as_object_mut() {
                    o.remove("text");
                }
                b
            }).collect();
            s.scripts.insert(name.to_string(), shown);
            match module_version(&r.body) {
                Some(v) => s.versions.insert(name.to_string(), v),
                None => s.versions.remove(*name),
            };
            ok(json!({"id": name, "bindings": []}))
        }
        ("PUT", ["workers", "scripts", name, "secrets"]) => {
            if let Some(e) = workers(s) {
                return e;
            }
            if !s.scripts.contains_key(*name) {
                return err(404, 10007, "workers.api.error.script_not_found");
            }
            let body = serde_json::from_slice::<Value>(&r.body).ok();
            let secret = body.as_ref().and_then(|v| v.get("name").and_then(Value::as_str).map(String::from)).unwrap_or_default();
            if secret == "R2_PARENT" {
                if let (Some(relay), Some(sub)) = (s.relay.clone(), s.subdomain.clone()) {
                    // The binding reaches the Worker it was placed on (the page stand-in).
                    let host = format!("{name}.{sub}.workers.dev");
                    let text = body.as_ref().and_then(|v| v.get("text").and_then(Value::as_str)).unwrap_or_default();
                    let status = relay_post(&relay, &format!("/_dev/space/{host}/parent"), text.as_bytes());
                    s.calls.push(format!("RELAY R2_PARENT {host} {}", status.as_ref().map_or_else(|e| e.clone(), |c| c.to_string())));
                    if status != Ok(200) {
                        return err(500, 10013, "the secret couldn't be placed on the Worker");
                    }
                }
            }
            s.secrets.entry(name.to_string()).or_default().insert(secret.clone());
            ok(json!({"name": secret, "type": "secret_text"}))
        }
        ("DELETE", ["workers", "scripts", name]) => {
            s.scripts.remove(*name);
            s.routes.remove(*name);
            ok(json!(null))
        }
        ("GET", ["workers", "scripts", name, "settings"]) => match s.scripts.get(*name) {
            Some(b) => ok(json!({"bindings": b})),
            None => err(404, 10007, "workers.api.error.script_not_found"),
        },
        ("POST", ["workers", "scripts", name, "subdomain"]) => {
            if s.subdomain.is_none() {
                return err(400, 10007, "workers.api.error.no_subdomain");
            }
            if !s.scripts.contains_key(*name) {
                return err(404, 10007, "workers.api.error.script_not_found");
            }
            if s.routes.insert(name.to_string()) && s.propagation > 0 {
                s.pending.insert(name.to_string(), s.propagation);
            }
            ok(json!({"enabled": true, "previews_enabled": false}))
        }
        ("GET", ["workers", "subdomain"]) => match &s.subdomain {
            Some(n) => ok(json!({"subdomain": n})),
            None => err(404, 10007, "workers.api.error.no_subdomain"),
        },
        ("PUT", ["workers", "subdomain"]) => {
            let want = serde_json::from_slice::<Value>(&r.body).ok().and_then(|v| v.get("subdomain").and_then(Value::as_str).map(String::from)).unwrap_or_default();
            // An account keeps the name it has: a second claim is refused whatever it asks
            // for (recorded on the e2e account, 2026-10-14 — `shroom-cloud/tests/fixtures/named`).
            if s.subdomain.is_some() {
                return err(409, 10036, "Account already has an associated subdomain.");
            }
            if s.taken.contains(&want) {
                return err(409, 10031, "workers.api.error.subdomain_unavailable");
            }
            s.subdomain = Some(want.clone());
            ok(json!({"subdomain": want}))
        }
        ("GET", ["r2", "buckets"]) => r2(s).unwrap_or_else(|| {
            let filter = r.query.get("name_contains").cloned().unwrap_or_default();
            ok(json!({"buckets": s.buckets.keys().filter(|n| n.contains(&filter)).map(|n| json!({"name": n})).collect::<Vec<_>>()}))
        }),
        ("POST", ["r2", "buckets"]) => {
            if let Some(e) = r2(s).or_else(|| unverified(s)) {
                return e;
            }
            let name = serde_json::from_slice::<Value>(&r.body).ok().and_then(|v| v.get("name").and_then(Value::as_str).map(String::from)).unwrap_or_default();
            if s.buckets.contains_key(&name) {
                return err(409, 10004, "The bucket you tried to create already exists, and you own it.");
            }
            s.buckets.insert(name.clone(), BTreeMap::new());
            ok(json!({"name": name}))
        }
        ("DELETE", ["r2", "buckets", name]) => {
            if s.buckets.get(*name).is_some_and(|b| !b.is_empty()) {
                return err(409, 10008, "The bucket you tried to delete is not empty.");
            }
            s.buckets.remove(*name);
            ok(json!(null))
        }
        ("POST", ["r2", "temp-access-credentials"]) => {
            if let Some(e) = r2(s) {
                return e;
            }
            ok(json!({"accessKeyId": "tmpkey", "secretAccessKey": "tmpsecret", "sessionToken": "tmpsession"}))
        }
        _ => err(404, 7003, "Could not route to the requested path"),
    }
}

/// `GET /page/<label>.<name>.workers.dev/_shroom/ping`: a shroom page answers once its
/// script is bound to a bucket and its route is on under the account's name (after the
/// propagation knob's not-yet answers); anything else is workers.dev's own 404.
fn page(s: &mut State, rest: &str) -> Reply {
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let not_there = || (404, "text/plain", b"error code: 1042".to_vec(), vec![]);
    let Some(label) = host.strip_suffix(".workers.dev").and_then(|h| h.rsplit_once('.')).and_then(|(label, sub)| (s.subdomain.as_deref() == Some(sub)).then_some(label.to_string())) else {
        return not_there();
    };
    *s.pings.entry(label.clone()).or_default() += 1;
    if !s.routes.contains(&label) {
        return not_there();
    }
    if let Some(n) = s.pending.get_mut(&label) {
        *n -= 1;
        if *n == 0 {
            s.pending.remove(&label);
        }
        return not_there();
    }
    let bound = s.scripts.get(&label).is_some_and(|bs| bs.iter().any(|b| b.get("type").and_then(Value::as_str) == Some("r2_bucket")));
    if path != "_shroom/ping" || !bound {
        return not_there();
    }
    let version = s.versions.get(&label).cloned().unwrap_or_else(|| "0.0.0".into());
    (200, "application/json", json!({"shroom": true, "iface": 2, "version": version}).to_string().into_bytes(), vec![])
}

/// The `VERSION` constant a deployed module carries (`export const VERSION = "x.y.z";`).
pub fn module_version(body: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    let at = text.find("const VERSION = \"")? + "const VERSION = \"".len();
    let end = text[at..].find('"')? + at;
    Some(text[at..end].to_string())
}

/// The multipart Worker upload's `metadata` part.
fn metadata(body: &[u8]) -> Option<Value> {
    let text = String::from_utf8_lossy(body);
    let at = text.find("name=\"metadata\"")?;
    let start = text[at..].find("\r\n\r\n")? + at + 4;
    let end = text[start..].find("\r\n--")? + start;
    serde_json::from_str(&text[start..end]).ok()
}

fn s3(s: &mut State, r: &Request, rest: &str) -> Reply {
    let (bucket, key) = rest.split_once('/').unwrap_or((rest, ""));
    if !s.buckets.contains_key(bucket) {
        return (404, "application/xml", b"<Error><Code>NoSuchBucket</Code></Error>".to_vec(), vec![]);
    }
    if key.is_empty() && r.method == "GET" {
        let prefix = r.query.get("prefix").cloned().unwrap_or_default();
        let mut xml = String::from("<ListBucketResult><IsTruncated>false</IsTruncated>");
        for (k, (b, e)) in s.buckets[bucket].iter().filter(|(k, _)| k.starts_with(&prefix)) {
            let size = s.sunk.get(&(bucket.to_string(), k.clone())).copied().unwrap_or(b.len() as u64);
            xml.push_str(&format!("<Contents><Key>{}</Key><Size>{size}</Size><ETag>\"{e}\"</ETag></Contents>", k.replace('&', "&amp;").replace('<', "&lt;")));
        }
        xml.push_str("</ListBucketResult>");
        return (200, "application/xml", xml.into_bytes(), vec![]);
    }
    if let Some(reply) = multipart(s, r, bucket, key) {
        return reply;
    }
    let current = s.buckets[bucket].get(key).map(|(_, e)| format!("\"{e}\""));
    match r.method.as_str() {
        "PUT" => {
            if r.headers.get("if-none-match").is_some_and(|v| v == "*") && current.is_some() {
                return (412, "application/xml", b"<Error><Code>PreconditionFailed</Code></Error>".to_vec(), vec![]);
            }
            if let Some(want) = r.headers.get("if-match") {
                if current.as_deref() != Some(want.as_str()) {
                    return (412, "application/xml", b"<Error><Code>PreconditionFailed</Code></Error>".to_vec(), vec![]);
                }
            }
            if s.sink.contains(bucket) && key.starts_with("rec/") {
                s.sunk.insert((bucket.to_string(), key.to_string()), r.body.len() as u64);
                s.put_object(bucket, key, &[]);
            } else {
                s.sunk.remove(&(bucket.to_string(), key.to_string()));
                s.put_object(bucket, key, &r.body);
            }
            let e = format!("\"{}\"", s.etags);
            (200, "application/xml", Vec::new(), vec![("etag".into(), e)])
        }
        "GET" | "HEAD" => match s.buckets[bucket].get(key) {
            Some((b, e)) => (200, "application/octet-stream", b.clone(), vec![("etag".into(), format!("\"{e}\""))]),
            None => (404, "application/xml", b"<Error><Code>NoSuchKey</Code></Error>".to_vec(), vec![]),
        },
        "DELETE" => {
            if let Some(b) = s.buckets.get_mut(bucket) {
                b.remove(key);
            }
            (204, "application/xml", Vec::new(), vec![])
        }
        _ => (405, "application/xml", Vec::new(), vec![]),
    }
}

/// S3's multipart upload, as R2 answers it: `POST ?uploads` starts one, `PUT
/// ?partNumber&uploadId` sends a part (its ETag back), `POST ?uploadId` completes it from
/// the parts' numbers and ETags in order, `DELETE ?uploadId` aborts it. `None`: not a
/// multipart request.
fn multipart(s: &mut State, r: &Request, bucket: &str, key: &str) -> Option<Reply> {
    let xml = |status: u16, body: String| (status, "application/xml", body.into_bytes(), vec![]);
    let no_such = || xml(404, "<Error><Code>NoSuchUpload</Code></Error>".into());
    let id = r.query.get("uploadId").cloned();
    match (r.method.as_str(), id) {
        ("POST", None) if r.query.contains_key("uploads") => {
            s.etags += 1;
            let id = format!("mpu-{}", s.etags);
            s.multipart.insert(id.clone(), Multipart { bucket: bucket.into(), key: key.into(), parts: BTreeMap::new() });
            Some(xml(200, format!("<InitiateMultipartUploadResult><Bucket>{bucket}</Bucket><Key>{key}</Key><UploadId>{id}</UploadId></InitiateMultipartUploadResult>")))
        }
        ("PUT", Some(id)) => {
            let Some(n) = r.query.get("partNumber").and_then(|n| n.parse::<u32>().ok()).filter(|n| (1..=10_000).contains(n)) else {
                return Some(xml(400, "<Error><Code>InvalidArgument</Code></Error>".into()));
            };
            let sink = s.sink.contains(bucket) && key.starts_with("rec/");
            s.etags += 1;
            let etag = format!("\"part-{}\"", s.etags);
            let Some(up) = s.multipart.get_mut(&id).filter(|u| u.bucket == bucket && u.key == key) else { return Some(no_such()) };
            up.parts.insert(n, (if sink { Vec::new() } else { r.body.clone() }, r.body.len() as u64, etag.clone()));
            Some((200, "application/xml", Vec::new(), vec![("etag".into(), etag)]))
        }
        ("POST", Some(id)) => {
            let Some(up) = s.multipart.get(&id).filter(|u| u.bucket == bucket && u.key == key).cloned() else { return Some(no_such()) };
            let text = String::from_utf8_lossy(&r.body).into_owned();
            let unescape = |v: &str| v.replace("&quot;", "\"").replace("&amp;", "&");
            let mut listed: Vec<(u32, String)> = Vec::new();
            for part in text.split("<Part>").skip(1) {
                let field = |tag: &str| part.split(&format!("<{tag}>")).nth(1).and_then(|v| v.split(&format!("</{tag}>")).next()).map(unescape);
                let (Some(n), Some(e)) = (field("PartNumber").and_then(|n| n.parse().ok()), field("ETag")) else { return Some(xml(400, "<Error><Code>MalformedXML</Code></Error>".into())) };
                listed.push((n, e));
            }
            let ascending = listed.windows(2).all(|w| w[0].0 < w[1].0);
            let known = listed.iter().all(|(n, e)| up.parts.get(n).is_some_and(|p| &p.2 == e));
            if listed.is_empty() || !ascending || !known {
                return Some(xml(400, "<Error><Code>InvalidPart</Code></Error>".into()));
            }
            let size: u64 = listed.iter().map(|(n, _)| up.parts[n].1).sum();
            if s.sink.contains(bucket) && key.starts_with("rec/") {
                s.sunk.insert((bucket.to_string(), key.to_string()), size);
                s.put_object(bucket, key, &[]);
            } else {
                let bytes: Vec<u8> = listed.iter().flat_map(|(n, _)| up.parts[n].0.iter().copied()).collect();
                s.sunk.remove(&(bucket.to_string(), key.to_string()));
                s.put_object(bucket, key, &bytes);
            }
            s.multipart.remove(&id);
            Some(xml(200, format!("<CompleteMultipartUploadResult><Bucket>{bucket}</Bucket><Key>{key}</Key><ETag>\"{}\"</ETag></CompleteMultipartUploadResult>", s.etags)))
        }
        ("DELETE", Some(id)) => {
            s.multipart.remove(&id);
            Some((204, "application/xml", Vec::new(), vec![]))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dashboards_knobs_break_and_mend_the_account() {
        let mut s = State::fresh();
        s.put_object("shroom-space", "meta/a.json", b"{}");
        s.control("card-gate").unwrap();
        s.control("email-gate").unwrap();
        assert!(!s.r2_enabled && !s.email_verified);
        s.control("clear-gates").unwrap();
        assert!(s.r2_enabled && s.email_verified);
        s.control("drop-bucket shroom-space").unwrap();
        assert!(!s.buckets.contains_key("shroom-space"));
        assert!(s.control("drop-bucket shroom-space").is_err());
        s.control("restore-bucket shroom-space").unwrap();
        assert_eq!(s.object("shroom-space", "meta/a.json").as_deref(), Some(&b"{}"[..]));
        assert!(s.control("format-disk").is_err());
    }

    #[test]
    fn the_knobs_answer_over_http_and_only_to_post() {
        let fake = Fake::start(State::fresh());
        let root = fake.api().trim_end_matches("/client/v4").to_string();
        assert!(ureq_like_post(&root, "/_fake/card-gate"));
        assert!(!fake.with(|s| s.r2_enabled));
        assert!(ureq_like_post(&root, "/_fake/clear-gates"));
        assert!(fake.with(|s| s.r2_enabled));
    }

    #[test]
    fn the_fault_seam_parses_counts_and_uses_itself_up() {
        let mut s = State::fresh();
        s.control("fault 503 PUT /s3/b/rec/ 2").unwrap();
        s.control("fault delay:40 * /s3/ always").unwrap();
        assert!(s.control("fault 200 PUT /s3/ 1").is_err(), "a 2xx is no fault");
        assert!(s.control("fault drop PUT /s3/").is_err(), "a count is required");
        assert_eq!(s.take_fault("GET", "/s3/b/rec/x"), Some(FaultKind::Delay(40)), "the method must match");
        assert_eq!(s.take_fault("PUT", "/s3/b/rec/x"), Some(FaultKind::Status(503)));
        assert_eq!(s.take_fault("PUT", "/s3/b/rec/y"), Some(FaultKind::Status(503)));
        assert_eq!(s.take_fault("PUT", "/s3/b/rec/z"), Some(FaultKind::Delay(40)), "the 503s are used up");
        assert_eq!(s.faulted(), 4);
        s.control("faults-clear").unwrap();
        assert_eq!(s.take_fault("PUT", "/s3/b/rec/z"), None);
    }

    /// What a client sees for each kind, over a real socket — and what the bucket keeps.
    #[test]
    fn each_fault_reaches_the_client_as_the_network_would() {
        let mut st = State::fresh();
        st.buckets.insert("b".into(), Default::default());
        let fake = Fake::start(st);
        let addr = fake.r2().trim_start_matches("http://").trim_end_matches("/s3").to_string();
        let put = |key: &str, body: &[u8]| -> std::io::Result<String> {
            let mut c = TcpStream::connect(&addr)?;
            c.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
            write!(c, "PUT /s3/b/{key} HTTP/1.1\r\nhost: x\r\ncontent-length: {}\r\n\r\n", body.len())?;
            c.write_all(body)?;
            let mut out = String::new();
            c.read_to_string(&mut out)?;
            Ok(out)
        };
        let big = vec![7u8; 4 << 20];
        for (kind, key, body, written) in [
            ("503", "a", &b"x"[..], false),
            ("drop", "b", &b"x"[..], false),
            ("lost-reply", "c", &b"x"[..], true),
            ("reset", "d", &big[..], false),
        ] {
            fake.with(|s| s.control(&format!("fault {kind} PUT /s3/b/{key} 1")).unwrap());
            let out = put(key, body);
            match kind {
                "503" => assert!(out.as_ref().unwrap().starts_with("HTTP/1.1 503"), "{out:?}"),
                _ => assert!(out.as_ref().map(|o| o.is_empty()).unwrap_or(true), "{kind}: no answer ({out:?})"),
            }
            assert_eq!(fake.with(|s| s.object("b", key).is_some()), written, "{kind}");
            // The next try goes through.
            assert!(put(key, b"y").unwrap().starts_with("HTTP/1.1 200"), "{kind}");
        }
        fake.with(|s| s.control("fault delay:300 PUT /s3/b/e 1").unwrap());
        let t = std::time::Instant::now();
        assert!(put("e", b"z").unwrap().starts_with("HTTP/1.1 200"));
        assert!(t.elapsed() >= std::time::Duration::from_millis(300));
    }

    #[test]
    fn a_sink_counts_recording_bytes_without_keeping_them() {
        let mut st = State::fresh();
        st.buckets.insert("b".into(), Default::default());
        st.control("sink b").unwrap();
        let fake = Fake::start(st);
        let addr = fake.r2().trim_start_matches("http://").trim_end_matches("/s3").to_string();
        let req = |head: String, body: &[u8]| {
            let mut c = TcpStream::connect(&addr).unwrap();
            c.write_all(head.as_bytes()).unwrap();
            c.write_all(body).unwrap();
            let mut out = String::new();
            c.read_to_string(&mut out).unwrap();
            out
        };
        let big = vec![1u8; 300_000];
        assert!(req(format!("PUT /s3/b/rec/x/seg.m4s HTTP/1.1\r\ncontent-length: {}\r\n\r\n", big.len()), &big).starts_with("HTTP/1.1 200"));
        assert!(req("PUT /s3/b/meta/x.json HTTP/1.1\r\ncontent-length: 2\r\n\r\n".into(), b"{}").starts_with("HTTP/1.1 200"));
        assert_eq!(fake.with(|s| s.object("b", "rec/x/seg.m4s")).map(|b| b.len()), Some(0), "not kept");
        assert_eq!(fake.with(|s| s.object("b", "meta/x.json")).as_deref(), Some(&b"{}"[..]), "records are kept");
        let list = req("GET /s3/b?prefix=rec/ HTTP/1.1\r\ncontent-length: 0\r\n\r\n".into(), b"");
        assert!(list.contains("<Size>300000</Size>"), "listed at its real size: {list}");
    }

    /// DECISIONS B90: an `R2_PARENT` placed on a Worker reaches the page stand-in's binding
    /// for that Worker's host — its text, once; any other secret, or no relay, reaches nothing.
    #[test]
    fn a_placed_parent_reaches_the_page_stand_in_and_the_calls_say_so() {
        let stand_in = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = stand_in.local_addr().unwrap().to_string();
        let got = std::thread::spawn(move || {
            let (mut c, _) = stand_in.accept().unwrap();
            let mut r = BufReader::new(c.try_clone().unwrap());
            let mut first = String::new();
            r.read_line(&mut first).unwrap();
            let mut len = 0;
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                if h.trim().is_empty() {
                    break;
                }
                if let Some(v) = h.to_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            c.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").unwrap();
            (first, String::from_utf8(body).unwrap())
        });
        let mut st = State::fresh();
        st.subdomain = Some("myshroom".into());
        st.scripts.insert("v".into(), vec![]);
        assert!(st.control("relay-parent http://10.0.0.1:80").is_err(), "loopback only");
        st.control(&format!("relay-parent http://{at}")).unwrap();
        let fake = Fake::start(st);
        let api = fake.api();
        let root = api.trim_end_matches("/client/v4").trim_start_matches("http://").to_string();
        let put = |name: &str, text: &str| {
            let body = json!({"name": name, "text": text, "type": "secret_text"}).to_string();
            let mut c = TcpStream::connect(&root).unwrap();
            write!(c, "PUT /client/v4/accounts/acc0000000000000000000000000001/workers/scripts/v/secrets HTTP/1.1\r\nauthorization: Bearer t\r\ncontent-length: {}\r\n\r\n{body}", body.len()).unwrap();
            let mut out = String::new();
            c.read_to_string(&mut out).unwrap();
            out
        };
        let parent = r#"{"account_id":"a","bucket":"shroom-space","key_id":"k","secret":"s"}"#;
        assert!(put("R2_PARENT", parent).starts_with("HTTP/1.1 200"));
        let (line, body) = got.join().unwrap();
        assert!(line.starts_with("POST /_dev/space/v.myshroom.workers.dev/parent "), "{line}");
        assert_eq!(body, parent, "the binding's text, as placed");
        assert!(put("OTHER", "x").starts_with("HTTP/1.1 200"), "another secret isn't relayed (nothing listens now)");
        let calls = fake.with(|s| s.calls.clone());
        assert_eq!(calls.iter().filter(|c| c.starts_with("RELAY ")).collect::<Vec<_>>(), ["RELAY R2_PARENT v.myshroom.workers.dev 200"]);
        assert!(!calls.iter().any(|c| c.contains("\"secret\"") || c.contains(parent)), "no secret text in the calls");
        // The calls knob answers GET, with no bodies in it.
        let mut c = TcpStream::connect(&root).unwrap();
        write!(c, "GET /_fake/calls HTTP/1.1\r\ncontent-length: 0\r\n\r\n").unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 200") && out.contains("RELAY R2_PARENT") && !out.contains("\\\"s\\\""), "{out}");
    }

    /// A bare HTTP POST (the fake has no client dependency).
    fn ureq_like_post(root: &str, path: &str) -> bool {
        let addr = root.trim_start_matches("http://");
        let mut c = TcpStream::connect(addr).unwrap();
        write!(c, "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-length: 0\r\n\r\n").unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).unwrap();
        out.starts_with("HTTP/1.1 200")
    }
}
