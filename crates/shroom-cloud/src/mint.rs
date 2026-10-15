//! Temporary bucket credentials minted **locally** (ARCHITECTURE §16 #1 option (a);
//! Cloudflare's documented local signing for R2 temporary credentials): an HS256 JWT
//! signed with the parent's S3 secret — no API call, no API token. The parent's key id
//! is reused as the access key id, the secret is the JWT's SHA-256 (hex), and the session
//! token is `base64("jwt/" + jwt)`. This is what lets the page's Worker hand a member a
//! credential scoped to one recording while holding only an R2 key, never the token.

use base64::Engine;
use serde_json::{Value, json};
use shroom_storage::sigv4::{Credentials, hmac, sha256_hex};

/// What the credential may touch: whole prefixes and single objects.
#[derive(Debug, Clone, Default)]
pub struct Paths {
    pub prefixes: Vec<String>,
    pub objects: Vec<String>,
}

pub struct Grant<'a> {
    pub account_id: &'a str,
    pub bucket: &'a str,
    /// `object-read-only` | `object-read-write`.
    pub scope: &'a str,
    pub paths: Paths,
    pub issued_at: u64,
    pub ttl_seconds: u64,
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// The signed JWT, exactly as Cloudflare's example builds it.
pub fn jwt(parent: &Credentials, g: &Grant) -> String {
    let header = json!({"alg": "HS256", "typ": "JWT"});
    let mut claims = json!({
        "bucket": g.bucket,
        "scope": g.scope,
        "sub": g.account_id,
        "iss": parent.access_key_id,
        "aud": format!("{}.r2.cloudflarestorage.com", g.account_id),
        "iat": g.issued_at,
        "exp": g.issued_at + g.ttl_seconds,
    });
    if !g.paths.prefixes.is_empty() || !g.paths.objects.is_empty() {
        claims["paths"] = json!({"prefixPaths": g.paths.prefixes, "objectPaths": g.paths.objects});
    }
    let signing_input = format!("{}.{}", b64(header.to_string().as_bytes()), b64(claims.to_string().as_bytes()));
    let sig = hmac(parent.secret_access_key.as_bytes(), signing_input.as_bytes());
    format!("{signing_input}.{}", b64(&sig))
}

/// The temporary credential for `g`, signed with the parent's S3 pair.
pub fn mint(parent: &Credentials, g: &Grant) -> Credentials {
    let token = jwt(parent, g);
    Credentials {
        access_key_id: parent.access_key_id.clone(),
        secret_access_key: sha256_hex(token.as_bytes()),
        session_token: Some(base64::engine::general_purpose::STANDARD.encode(format!("jwt/{token}"))),
    }
}

/// The claims of a JWT this module made (tests and logs read them back; never the key).
pub fn claims(jwt: &str) -> Option<Value> {
    let part = jwt.split('.').nth(1)?;
    serde_json::from_slice(&base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(part).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_credential_has_cloudflares_shape() {
        let parent = Credentials { access_key_id: "pid".into(), secret_access_key: "psecret".into(), session_token: None };
        let g = Grant {
            account_id: "acct", bucket: "b", scope: "object-read-write",
            paths: Paths { prefixes: vec!["rec/X/".into()], objects: vec!["meta/X.json".into()] },
            issued_at: 1_000, ttl_seconds: 60,
        };
        let t = jwt(&parent, &g);
        let c = claims(&t).unwrap();
        assert_eq!((c["iss"].as_str(), c["sub"].as_str(), c["aud"].as_str(), c["exp"].as_u64()), (Some("pid"), Some("acct"), Some("acct.r2.cloudflarestorage.com"), Some(1_060)));
        assert_eq!(c["paths"]["prefixPaths"][0], "rec/X/");
        let cred = mint(&parent, &g);
        assert_eq!(cred.access_key_id, "pid");
        assert_eq!(cred.secret_access_key, sha256_hex(t.as_bytes()));
        let session = base64::engine::general_purpose::STANDARD.decode(cred.session_token.unwrap()).unwrap();
        assert_eq!(String::from_utf8(session).unwrap(), format!("jwt/{t}"));
    }
}
