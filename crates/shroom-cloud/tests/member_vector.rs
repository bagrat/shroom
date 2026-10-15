//! The member mint's golden vector (WORKER-INTERFACE §3.4, C1; §7.1 #16). The page's
//! Worker mints recording credentials for Macs without the token exactly as
//! `shroom_cloud::mint` does for token Macs, and the two signers must agree byte for byte.
//! `worker/test/region6.test.js` pins the very same inputs and the very same JWT.
//!
//! The claims serialize alphabetically (serde_json without `preserve_order`), nested
//! objects included: `aud, bucket, exp, iat, iss, paths{objectPaths, prefixPaths}, scope,
//! sub`. If this workspace ever turns `preserve_order` on, this test breaks loudly — which
//! is the point: the Worker's literal order would then have to move with it.

use base64::Engine;
use shroom_cloud::mint::{Grant, Paths, claims, jwt, mint};
use shroom_storage::sigv4::{Credentials, sha256_hex};

/// The fixed inputs, shared with the Worker's test (`GOLDEN` there).
const ACCOUNT: &str = "0123456789abcdef0123456789abcdef";
const BUCKET: &str = "shroom-space";
const KEY_ID: &str = "00112233445566778899aabbccddeeff";
const SECRET: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
const REC: &str = "AbCdEfGhIjKlMnOp";
/// 2026-10-02T12:00:00Z.
const IAT: u64 = 1_790_942_400;
/// An upload credential lives seven days (§3.4; B35 caps it there).
const TTL: u64 = 604_800;

/// The expected JWT, byte for byte.
const GOLDEN: &str = concat!(
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.",
    "eyJhdWQiOiIwMTIzNDU2Nzg5YWJjZGVmMDEyMzQ1Njc4OWFiY2RlZi5yMi5jbG91ZGZsYXJlc3RvcmFnZS5jb20iLCJidWNrZXQiOiJzaHJvb20tc3BhY2UiLCJleHAiOjE3OTE1NDcyMDAsImlhdCI6MTc5MDk0MjQwMCwiaXNzIjoiMDAxMTIyMzM0NDU1NjY3Nzg4OTlhYWJiY2NkZGVlZmYiLCJwYXRocyI6eyJvYmplY3RQYXRocyI6WyJtZXRhL0FiQ2RFZkdoSWpLbE1uT3AuanNvbiJdLCJwcmVmaXhQYXRocyI6WyJyZWMvQWJDZEVmR2hJaktsTW5PcC8iXX0sInNjb3BlIjoib2JqZWN0LXJlYWQtd3JpdGUiLCJzdWIiOiIwMTIzNDU2Nzg5YWJjZGVmMDEyMzQ1Njc4OWFiY2RlZiJ9.",
    "SjlFTu38wdAOOr-s_PB-X6_Qy8vwi7sf2DhE4c2TNA4",
);

fn parent() -> Credentials {
    Credentials { access_key_id: KEY_ID.into(), secret_access_key: SECRET.into(), session_token: None }
}

fn grant() -> Grant<'static> {
    Grant {
        account_id: ACCOUNT,
        bucket: BUCKET,
        scope: "object-read-write",
        paths: Paths { prefixes: vec![format!("rec/{REC}/")], objects: vec![format!("meta/{REC}.json")] },
        issued_at: IAT,
        ttl_seconds: TTL,
    }
}

#[test]
fn the_member_mint_is_the_golden_jwt() {
    let token = jwt(&parent(), &grant());
    assert_eq!(token, GOLDEN);
}

#[test]
fn the_golden_claims_are_alphabetical_with_exactly_two_paths() {
    let payload = GOLDEN.split('.').nth(1).unwrap();
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).unwrap();
    assert_eq!(
        String::from_utf8(raw).unwrap(),
        format!(
            r#"{{"aud":"{ACCOUNT}.r2.cloudflarestorage.com","bucket":"{BUCKET}","exp":{},"iat":{IAT},"iss":"{KEY_ID}","paths":{{"objectPaths":["meta/{REC}.json"],"prefixPaths":["rec/{REC}/"]}},"scope":"object-read-write","sub":"{ACCOUNT}"}}"#,
            IAT + TTL
        )
    );
    let header = GOLDEN.split('.').next().unwrap();
    assert_eq!(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(header).unwrap(), br#"{"alg":"HS256","typ":"JWT"}"#);
    let c = claims(GOLDEN).unwrap();
    assert_eq!(c["exp"].as_u64(), Some(IAT + TTL));
}

#[test]
fn the_golden_credential_derives_from_the_jwt() {
    let cred = mint(&parent(), &grant());
    assert_eq!(cred.access_key_id, KEY_ID);
    assert_eq!(cred.secret_access_key, sha256_hex(GOLDEN.as_bytes()));
    let session = base64::engine::general_purpose::STANDARD.decode(cred.session_token.unwrap()).unwrap();
    assert_eq!(String::from_utf8(session).unwrap(), format!("jwt/{GOLDEN}"));
}
