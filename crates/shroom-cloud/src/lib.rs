//! The user's Cloudflare account over its REST API — the wizard's whole job is about six
//! calls (RESEARCH R7.1, run live in R7.6). No wrangler, no node, one pasted token.
//!
//! A [`Policy`] can fence the client in: the test harness runs on Bagrat's real account
//! (TESTING-PLAN §9, D3), so its client may only touch `shroom-test-…` resources and may
//! never write the account's workers.dev name. The fence lives here, in code, so no test
//! can forget it.

pub mod device;
pub mod member;
pub mod mint;
pub mod owner;
pub mod registry;
pub mod sweep;
pub mod wire;

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use shroom_storage::sigv4::{Credentials, sha256_hex};

const API: &str = "https://api.cloudflare.com/client/v4";

/// What a pasted token is, read from its prefix (R7.6: verify must route by prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// `cfat_…` — owned by the account (preferred, R7.2).
    Account,
    /// Anything else — a user token.
    User,
}

pub fn classify(token: &str) -> TokenKind {
    if token.trim().starts_with("cfat_") { TokenKind::Account } else { TokenKind::User }
}

/// What the client may touch.
#[derive(Debug, Clone, PartialEq)]
pub enum Policy {
    /// The product: whatever the user's token allows.
    Unfenced,
    /// The test harness on a shared account: only Workers and buckets named with `prefix`,
    /// and never the account's workers.dev name.
    TestNamespace { prefix: String },
    /// The pre-merge e2e run on the dedicated test account (`--e2e`, dev builds only;
    /// TESTING-PLAN D3): the product's reach on that one account, and a call addressed to
    /// any other account refused before it's made.
    Account { id: String },
}

#[derive(Debug)]
pub enum CloudError {
    /// Cloudflare answered with errors (codes are the classifiable part — R7.4).
    Api { status: u16, codes: Vec<i64>, messages: Vec<String> },
    Transport(String),
    /// The policy refused the call before it was made.
    Refused(String),
}

impl CloudError {
    pub fn has_code(&self, code: i64) -> bool {
        matches!(self, CloudError::Api { codes, .. } if codes.contains(&code))
    }
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudError::Api { status, codes, messages } => write!(f, "HTTP {status} {codes:?} {messages:?}"),
            CloudError::Transport(e) => write!(f, "transport: {e}"),
            CloudError::Refused(why) => write!(f, "refused by policy: {why}"),
        }
    }
}

impl std::error::Error for CloudError {}

/// Known error codes (RESEARCH R7.4–R7.6).
pub mod codes {
    pub const AUTH: i64 = 10000;
    pub const INVALID_TOKEN: i64 = 1000;
    pub const NO_SUBDOMAIN: i64 = 10007;
    pub const SUBDOMAIN_TAKEN: i64 = 10031;
    pub const EMAIL_UNVERIFIED: i64 = 8000077;
    pub const BUCKET_EXISTS: i64 = 10004;
}

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenStatus {
    pub id: String,
    pub status: String,
    pub expires_on: Option<String>,
}

/// A Worker binding (multipart metadata, R7.1 call 3).
#[derive(Debug, Clone)]
pub enum Binding {
    R2Bucket { name: String, bucket: String },
    SecretText { name: String, text: String },
    PlainText { name: String, text: String },
}

impl Binding {
    fn to_json(&self) -> Value {
        match self {
            Binding::R2Bucket { name, bucket } => json!({"type": "r2_bucket", "name": name, "bucket_name": bucket}),
            Binding::SecretText { name, text } => json!({"type": "secret_text", "name": name, "text": text}),
            Binding::PlainText { name, text } => json!({"type": "plain_text", "name": name, "text": text}),
        }
    }
}

pub struct Cloud {
    agent: ureq::Agent,
    token: String,
    policy: Policy,
    api: String,
    r2: Option<String>,
}

impl std::fmt::Debug for Cloud {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cloud").field("kind", &classify(&self.token)).field("policy", &self.policy).finish_non_exhaustive()
    }
}

impl Cloud {
    pub fn new(token: &str, policy: Policy) -> Cloud {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(60)))
            .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
            .build();
        Cloud { agent: config.into(), token: token.trim().to_string(), policy, api: API.to_string(), r2: None }
    }

    /// Points the client at another API and R2 endpoint — the L1 fake
    /// (`shroom-cloudfake`); the product never calls it.
    pub fn with_endpoints(mut self, api: &str, r2: &str) -> Cloud {
        self.api = api.trim_end_matches('/').to_string();
        self.r2 = Some(r2.trim_end_matches('/').to_string());
        self
    }

    /// The account's S3 endpoint for its buckets.
    pub fn r2_endpoint(&self, account_id: &str) -> String {
        self.r2.clone().unwrap_or_else(|| format!("https://{account_id}.r2.cloudflarestorage.com"))
    }

    pub fn kind(&self) -> TokenKind {
        classify(&self.token)
    }

    /// The bucket keys the token implies: access key = token id, secret = SHA-256 of the
    /// token (R7.6, confirmed live).
    pub fn s3_credentials(&self, token_id: &str) -> Credentials {
        Credentials {
            access_key_id: token_id.to_string(),
            secret_access_key: sha256_hex(self.token.as_bytes()),
            session_token: None,
        }
    }

    fn fence(&self, what: &str, name: &str) -> Result<(), CloudError> {
        match &self.policy {
            Policy::Unfenced | Policy::Account { .. } => Ok(()),
            Policy::TestNamespace { prefix } if name.starts_with(prefix.as_str()) => Ok(()),
            Policy::TestNamespace { prefix } => Err(CloudError::Refused(format!("{what} {name:?} is outside {prefix}…"))),
        }
    }

    fn call(&self, method: &str, path: &str, body: Option<(String, Vec<u8>)>) -> Result<Value, CloudError> {
        if let Policy::Account { id } = &self.policy {
            let addressed = path.strip_prefix("/accounts/").map(|rest| rest.split(['/', '?']).next().unwrap_or(""));
            if addressed.is_some_and(|a| !a.is_empty() && a != id) {
                return Err(CloudError::Refused("another account than the e2e run's".into()));
            }
        }
        let mut req = ureq::http::Request::builder()
            .method(method)
            .uri(format!("{}{path}", self.api))
            .header("authorization", format!("Bearer {}", self.token));
        let bytes = match body {
            Some((content_type, bytes)) => {
                req = req.header("content-type", content_type);
                bytes
            }
            None => Vec::new(),
        };
        let req = req.body(bytes).map_err(|e| CloudError::Transport(e.to_string()))?;
        let mut res = self.agent.run(req).map_err(|e| CloudError::Transport(e.to_string()))?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().map_err(|e| CloudError::Transport(e.to_string()))?;
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if (200..300).contains(&status) && v.get("success").and_then(Value::as_bool).unwrap_or(true) {
            return Ok(v);
        }
        let errors = v.get("errors").and_then(Value::as_array).cloned().unwrap_or_default();
        Err(CloudError::Api {
            status,
            codes: errors.iter().filter_map(|e| e.get("code").and_then(Value::as_i64)).collect(),
            messages: errors.iter().filter_map(|e| e.get("message").and_then(Value::as_str).map(String::from)).collect(),
        })
    }

/// One raw REST call, for the L1 contract test that holds the fake to Cloudflare's shapes.
    /// Reads anywhere; under a test policy, writes only to a body `name` inside the fence.
    #[doc(hidden)]
    pub fn raw(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, CloudError> {
        // The fence holds here too: under a test policy a write must name a fenced resource.
        if method != "GET" {
            let named = body.as_ref().and_then(|b| b.get("name")).and_then(Value::as_str).unwrap_or("");
            self.fence("resource", named)?;
        }
        match body {
            Some(b) => self.json(method, path, b),
            None => self.call(method, path, None),
        }
    }

    fn json(&self, method: &str, path: &str, body: Value) -> Result<Value, CloudError> {
        self.call(method, path, Some(("application/json".into(), body.to_string().into_bytes())))
    }

    fn result<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T, CloudError> {
        serde_json::from_value(v.get("result").cloned().unwrap_or(Value::Null))
            .map_err(|e| CloudError::Transport(format!("unexpected response: {e}")))
    }

    /// Call 1: the accounts this token reaches.
    pub fn accounts(&self) -> Result<Vec<Account>, CloudError> {
        Self::result(self.call("GET", "/accounts", None)?)
    }

    /// Call 0: the token's own id and status, routed by kind (R7.6).
    pub fn verify(&self, account_id: &str) -> Result<TokenStatus, CloudError> {
        let path = match self.kind() {
            TokenKind::Account => format!("/accounts/{account_id}/tokens/verify"),
            TokenKind::User => "/user/tokens/verify".to_string(),
        };
        Self::result(self.call("GET", &path, None)?)
    }

    /// Call 2: the private bucket. An existing bucket of that name counts as success.
    pub fn create_bucket(&self, account_id: &str, name: &str) -> Result<(), CloudError> {
        self.fence("bucket", name)?;
        match self.json("POST", &format!("/accounts/{account_id}/r2/buckets"), json!({"name": name})) {
            Ok(_) => Ok(()),
            Err(e) if e.has_code(codes::BUCKET_EXISTS) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn delete_bucket(&self, account_id: &str, name: &str) -> Result<(), CloudError> {
        self.fence("bucket", name)?;
        self.call("DELETE", &format!("/accounts/{account_id}/r2/buckets/{name}"), None).map(|_| ())
    }

    pub fn list_buckets(&self, account_id: &str) -> Result<Vec<String>, CloudError> {
        let v = self.call("GET", &format!("/accounts/{account_id}/r2/buckets"), None)?;
        Ok(v.pointer("/result/buckets")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|b| b.get("name").and_then(Value::as_str).map(String::from)).collect())
            .unwrap_or_default())
    }

    /// Call 3: deploys one dependency-free ES module with its bindings, in one multipart
    /// PUT. (The response's echoed bindings can be empty even when they applied — R7.6 —
    /// so callers probe the Worker instead of trusting the echo.)
    pub fn put_worker(
        &self,
        account_id: &str,
        name: &str,
        module_name: &str,
        module: &str,
        bindings: &[Binding],
        compatibility_date: &str,
    ) -> Result<(), CloudError> {
        self.fence("worker", name)?;
        let metadata = json!({
            "main_module": module_name,
            "compatibility_date": compatibility_date,
            "bindings": bindings.iter().map(Binding::to_json).collect::<Vec<_>>(),
        });
        let boundary = format!("shroom-{}", &sha256_hex(module.as_bytes())[..24]);
        let mut body = Vec::new();
        body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n").bytes());
        body.extend(metadata.to_string().bytes());
        body.extend(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"{module_name}\"; filename=\"{module_name}\"\r\nContent-Type: application/javascript+module\r\n\r\n").bytes());
        body.extend(module.bytes());
        body.extend(format!("\r\n--{boundary}--\r\n").bytes());
        self.call(
            "PUT",
            &format!("/accounts/{account_id}/workers/scripts/{name}"),
            Some((format!("multipart/form-data; boundary={boundary}"), body)),
        )
        .map(|_| ())
    }

    /// Places (or replaces) one secret on a deployed Worker without redeploying it — the
    /// script-secrets endpoint (WORKER-INTERFACE §3.2, probe P5).
    pub fn put_secret(&self, account_id: &str, script: &str, name: &str, text: &str) -> Result<(), CloudError> {
        self.fence("worker", script)?;
        self.json(
            "PUT",
            &format!("/accounts/{account_id}/workers/scripts/{script}/secrets"),
            json!({"name": name, "text": text, "type": "secret_text"}),
        )
        .map(|_| ())
    }

    pub fn delete_worker(&self, account_id: &str, name: &str) -> Result<(), CloudError> {
        self.fence("worker", name)?;
        self.call("DELETE", &format!("/accounts/{account_id}/workers/scripts/{name}?force=true"), None).map(|_| ())
    }

    /// Call 4 (read): the account's workers.dev name, or `None` if it has none yet (10007).
    pub fn subdomain(&self, account_id: &str) -> Result<Option<String>, CloudError> {
        match self.call("GET", &format!("/accounts/{account_id}/workers/subdomain"), None) {
            Ok(v) => Ok(v.pointer("/result/subdomain").and_then(Value::as_str).map(String::from)),
            Err(e) if e.has_code(codes::NO_SUBDOMAIN) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Call 4 (write): claims the account's workers.dev name. Taken → code 10031.
    /// Never allowed under a test fence: it would rename every link on the account.
    pub fn claim_subdomain(&self, account_id: &str, name: &str) -> Result<(), CloudError> {
        if let Policy::TestNamespace { .. } = &self.policy {
            return Err(CloudError::Refused("the workers.dev name is never written by tests".into()));
        }
        self.json("PUT", &format!("/accounts/{account_id}/workers/subdomain"), json!({"subdomain": name})).map(|_| ())
    }

    /// Mints short-lived bucket credentials scoped to `prefixes` and single `objects`
    /// (N4: one per recording, minted when its upload begins; ≤ 7 days). The parent is
    /// the token's derived key id. Probed live 2026-07-20: scoping is enforced.
    pub fn mint_temp_credentials(
        &self,
        account_id: &str,
        bucket: &str,
        parent_access_key_id: &str,
        ttl_seconds: u32,
        prefixes: &[String],
        objects: &[String],
    ) -> Result<Credentials, CloudError> {
        self.fence("bucket", bucket)?;
        let v = self.json(
            "POST",
            &format!("/accounts/{account_id}/r2/temp-access-credentials"),
            json!({
                "bucket": bucket,
                "parentAccessKeyId": parent_access_key_id,
                "permission": "object-read-write",
                "ttlSeconds": ttl_seconds,
                "prefixes": prefixes,
                "objects": objects,
            }),
        )?;
        let field = |name: &str| {
            v.pointer(&format!("/result/{name}"))
                .and_then(Value::as_str)
                .map(String::from)
                .ok_or_else(|| CloudError::Transport(format!("temp credentials missing {name}")))
        };
        Ok(Credentials {
            access_key_id: field("accessKeyId")?,
            secret_access_key: field("secretAccessKey")?,
            session_token: Some(field("sessionToken")?),
        })
    }

    /// Call 5: turns the Worker's workers.dev route on (previews off).
    pub fn enable_workers_dev(&self, account_id: &str, script: &str) -> Result<(), CloudError> {
        self.fence("worker", script)?;
        self.json(
            "POST",
            &format!("/accounts/{account_id}/workers/scripts/{script}/subdomain"),
            json!({"enabled": true, "previews_enabled": false}),
        )
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_prefix() {
        assert_eq!(classify("cfat_abc"), TokenKind::Account);
        assert_eq!(classify("  cfat_abc\n"), TokenKind::Account);
        assert_eq!(classify("abc"), TokenKind::User);
    }

    #[test]
    fn the_test_fence_refuses_before_any_request() {
        let c = Cloud::new("x", Policy::TestNamespace { prefix: "shroom-test-".into() });
        for result in [
            c.create_bucket("acct", "shroom"),
            c.delete_bucket("acct", "shroom"),
            c.delete_worker("acct", "v"),
            c.put_worker("acct", "v", "w.js", "", &[], "2026-01-01"),
            c.enable_workers_dev("acct", "v"),
            c.claim_subdomain("acct", "shroom-test-anything"),
            c.mint_temp_credentials("acct", "shroom", "id", 60, &[], &[]).map(|_| ()),
            c.raw("POST", "/accounts/acct/r2/buckets", Some(json!({"name": "shroom"}))).map(|_| ()),
            c.raw("PUT", "/accounts/acct/workers/subdomain", Some(json!({"subdomain": "x"}))).map(|_| ()),
        ] {
            assert!(matches!(result, Err(CloudError::Refused(_))), "{result:?}");
        }
    }

    #[test]
    fn the_e2e_pin_refuses_another_account_before_any_request() {
        let c = Cloud::new("cfat_x", Policy::Account { id: "e2e".into() }).with_endpoints("http://127.0.0.1:9/client/v4", "http://127.0.0.1:9");
        for result in [
            c.create_bucket("main", "shroom-space"),
            c.list_buckets("main").map(|_| ()),
            c.put_worker("main", "v", "w.js", "", &[], "2026-01-01"),
            c.claim_subdomain("main", "x"),
            c.verify("main").map(|_| ()),
            c.raw("GET", "/accounts/main?x=1", None).map(|_| ()),
        ] {
            assert!(matches!(result, Err(CloudError::Refused(_))), "{result:?}");
        }
        // Its own account, and the account-less calls, get as far as the network.
        for result in [c.create_bucket("e2e", "shroom-space"), c.claim_subdomain("e2e", "x"), c.accounts().map(|_| ())] {
            assert!(matches!(result, Err(CloudError::Transport(_))), "{result:?}");
        }
    }

    #[test]
    fn s3_secret_is_the_tokens_sha256() {
        let c = Cloud::new("abc\n", Policy::Unfenced);
        let creds = c.s3_credentials("tokid");
        assert_eq!(creds.access_key_id, "tokid");
        assert_eq!(creds.secret_access_key, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn debug_never_prints_the_token() {
        assert!(!format!("{:?}", Cloud::new("cfat_secret", Policy::Unfenced)).contains("secret"));
    }
}
