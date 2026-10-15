//! AWS Signature Version 4 for S3, ported from the POC's signer
//! (`archived-poc/scripts/uploader/lib/s3.mjs` at tag `v0.1.5`), which matched botocore byte for byte.
//!
//! Signs `host`, `x-amz-content-sha256`, `x-amz-date` and, for temporary credentials,
//! `x-amz-security-token` — the shape of botocore's S3SigV4Auth for a request with no
//! other headers. Path-style URLs (`<endpoint>/<bucket>/<key>`).

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

const ALGORITHM: &str = "AWS4-HMAC-SHA256";

#[derive(Clone)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    /// Present for temporary credentials (N4's per-recording, prefix-scoped keys).
    pub session_token: Option<String>,
}

impl std::fmt::Debug for Credentials {
    // Never print secrets (Law 13).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("access_key_id", &self.access_key_id)
            .field("temporary", &self.session_token.is_some())
            .finish_non_exhaustive()
    }
}

/// The headers to send with a signed request (besides `host`, which the HTTP client
/// derives from the URL).
#[derive(Debug, Clone, PartialEq)]
pub struct Signed {
    pub authorization: String,
    pub amz_date: String,
    pub payload_hash: String,
    pub session_token: Option<String>,
    pub canonical_request: String,
    pub string_to_sign: String,
}

impl Signed {
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut h = vec![
            ("authorization", self.authorization.clone()),
            ("x-amz-content-sha256", self.payload_hash.clone()),
            ("x-amz-date", self.amz_date.clone()),
        ];
        if let Some(t) = &self.session_token {
            h.push(("x-amz-security-token", t.clone()));
        }
        h
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

pub fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// RFC 3986 percent-encoding of one path segment or query component (S3 rules:
/// unreserved characters pass, everything else is encoded, `/` handled by the caller).
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Splits `scheme://host[:port]/path?query` into (host with port, path, query pairs).
fn split_url(url: &str) -> (String, String, Vec<(String, String)>) {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let (authority, path_query) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
    let pairs = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect();
    (authority.to_string(), percent_decode(path), pairs)
}

/// Decodes `%XX` escapes; everything else (including `+`, literal in S3 keys) passes.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Signs a request. `amz_date` is `YYYYMMDDTHHMMSSZ` (see [`amz_date_now`]).
pub fn sign(method: &str, url: &str, body: &[u8], region: &str, creds: &Credentials, amz_date: &str) -> Signed {
    sign_with(method, url, body, region, creds, amz_date, &[])
}

/// Signs a request that also carries `extra` headers (lower-case names, e.g.
/// `x-amz-meta-m`): S3 refuses an `x-amz-*` header it wasn't asked to sign, so each one
/// joins the canonical headers and `SignedHeaders`. The caller still sends them.
pub fn sign_with(method: &str, url: &str, body: &[u8], region: &str, creds: &Credentials, amz_date: &str, extra: &[(&str, &str)]) -> Signed {
    let service = "s3";
    let (host, path, mut query) = split_url(url);
    let date = &amz_date[..8];
    let payload_hash = sha256_hex(body);

    let canonical_uri = path.split('/').map(encode).collect::<Vec<_>>().join("/");
    query.sort();
    let canonical_query =
        query.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect::<Vec<_>>().join("&");

    let mut headers: Vec<(&str, String)> = vec![
        ("host", host),
        ("x-amz-content-sha256", payload_hash.clone()),
        ("x-amz-date", amz_date.to_string()),
    ];
    if let Some(t) = &creds.session_token {
        headers.push(("x-amz-security-token", t.clone()));
    }
    for (k, v) in extra {
        headers.push((k, v.to_string()));
    }
    headers.sort_by(|a, b| a.0.cmp(b.0));
    let canonical_headers: String = headers.iter().map(|(k, v)| format!("{k}:{}\n", v.trim())).collect();
    let signed_headers = headers.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(";");

    let canonical_request =
        [method, &canonical_uri, &canonical_query, &canonical_headers, &signed_headers, &payload_hash].join("\n");
    let scope = format!("{date}/{region}/{service}/aws4_request");
    let string_to_sign = [ALGORITHM, amz_date, &scope, &sha256_hex(canonical_request.as_bytes())].join("\n");

    let k_date = hmac(format!("AWS4{}", creds.secret_access_key).as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, service.as_bytes());
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex::encode(hmac(&k_signing, string_to_sign.as_bytes()));

    Signed {
        authorization: format!(
            "{ALGORITHM} Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            creds.access_key_id
        ),
        amz_date: amz_date.to_string(),
        payload_hash,
        session_token: creds.session_token.clone(),
        canonical_request,
        string_to_sign,
    }
}

/// Now, as `YYYYMMDDTHHMMSSZ`.
pub fn amz_date_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    amz_date(secs)
}

/// Seconds since the Unix epoch as `YYYYMMDDTHHMMSSZ` (UTC; civil-from-days).
pub fn amz_date(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let sod = unix_seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z", sod / 3600, sod % 3600 / 60, sod % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden vectors produced by botocore's S3SigV4Auth under a frozen clock (the
    /// POC's `test/sigv4.test.mjs`) — a reference implementation, not our own output.
    const VECTORS: &[(&str, &str, &str, &str, &str)] = &[
        (
            "PUT",
            "auto",
            "https://acct123.r2.cloudflarestorage.com/mybucket/AbC123/seg_00000.m4s",
            "hello-shroom-segment",
            "e913d6b0c8eebcfc69aa7a4ef01d83f9bd4bdae0b4da50203d993479d0bc6da1",
        ),
        (
            "PUT",
            "us-east-1",
            "https://s3.amazonaws.com/mybucket/AbC123/init.mp4",
            "",
            "c1c6af81e6943e9b10b65cfc631ce437c453f787ca8269a2aa2237acec5e7c1e",
        ),
        (
            "HEAD",
            "auto",
            "https://acct123.r2.cloudflarestorage.com/mybucket/AbC123/seg_00007.m4s",
            "",
            "9d861ba823e1f2753565c7698c13aa6b8b2a439916465808c9600e3445887e2c",
        ),
        (
            "PUT",
            "us-east-1",
            "http://127.0.0.1:9000/mybucket/AbC123/stream.m3u8",
            "#EXTM3U\n#EXT-X-ENDLIST\n",
            "a8537d04b8d82460d26b5e5fc5fed7e17639f7ffe9a392ebfea0ca0eca54d532",
        ),
    ];

    fn creds() -> Credentials {
        Credentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
        }
    }

    #[test]
    fn matches_botocore_byte_for_byte() {
        for (method, region, url, body, signature) in VECTORS {
            let s = sign(method, url, body.as_bytes(), region, &creds(), "20260102T030405Z");
            let scope = format!("20260102/{region}/s3/aws4_request");
            assert_eq!(
                s.authorization,
                format!(
                    "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/{scope}, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={signature}"
                ),
                "{method} {url}"
            );
        }
    }

    #[test]
    fn a_session_token_is_signed_and_sent() {
        let c = Credentials { session_token: Some("tok".into()), ..creds() };
        let s = sign("PUT", "https://a.example/b/k", b"x", "auto", &c, "20260102T030405Z");
        assert!(s.authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token,"));
        assert!(s.canonical_request.contains("x-amz-security-token:tok\n"));
        assert!(s.headers().contains(&("x-amz-security-token", "tok".to_string())));
    }

    #[test]
    fn query_parameters_are_sorted_and_encoded() {
        let s = sign("GET", "https://a.example/b?prefix=v%2F1&list-type=2", b"", "auto", &creds(), "20260102T030405Z");
        assert_eq!(s.canonical_request.lines().nth(2), Some("list-type=2&prefix=v%2F1"));
    }

    #[test]
    fn a_plus_in_a_key_stays_a_plus() {
        let s = sign("PUT", "https://a.example/b/a+b", b"", "auto", &creds(), "20260102T030405Z");
        assert_eq!(s.canonical_request.lines().nth(1), Some("/b/a%2Bb"));
    }

    #[test]
    fn extra_headers_are_signed_in_order() {
        let c = Credentials { session_token: Some("tok".into()), ..creds() };
        let s = sign_with("PUT", "https://a.example/b/k", b"x", "auto", &c, "20260102T030405Z", &[("x-amz-meta-m", " m1 ")]);
        assert!(s.authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-meta-m;x-amz-security-token,"), "{}", s.authorization);
        assert!(s.canonical_request.contains("x-amz-meta-m:m1\n"));
        // Without extras, the signature is the plain one (the botocore vectors still hold).
        assert_eq!(sign_with("PUT", "https://a.example/b/k", b"x", "auto", &c, "20260102T030405Z", &[]), sign("PUT", "https://a.example/b/k", b"x", "auto", &c, "20260102T030405Z"));
    }

    #[test]
    fn formats_amz_dates() {
        assert_eq!(amz_date(0), "19700101T000000Z");
        assert_eq!(amz_date(1_767_323_045), "20260102T030405Z");
        assert_eq!(amz_date(951_782_400), "20000229T000000Z");
    }

    #[test]
    fn debug_never_prints_the_secret() {
        let shown = format!("{:?}", Credentials { session_token: Some("tok".into()), ..creds() });
        assert!(!shown.contains("EXAMPLEKEY") && !shown.contains("tok"));
    }
}
