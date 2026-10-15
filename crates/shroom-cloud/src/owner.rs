//! Owner-ness on the page (U2, N40; DECISIONS B25). Before the app opens a page it
//! writes a one-time grant at `.shroom/keys/<sha256(k)>` and opens `/<id>?k=<k>`; the
//! Worker redeems it (deleting it), sets the owner cookie signed with
//! `.shroom/cookie.key`, and redirects to the clean URL. The cookie carries metadata
//! powers only — renaming, and for owner-class browsers the mark switch.

use base64::Engine;
use shroom_storage::s3::{Client, S3Error};

/// The cookie-signing secret's key; written once by wiring, read only by the Worker.
pub const COOKIE_KEY: &str = ".shroom/cookie.key";
/// How long an unredeemed key stays good: long enough for a slow browser launch.
pub const KEY_TTL_SECS: u64 = 600;

fn random_b64(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    getrandom::fill(&mut bytes).expect("the OS random source works");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// A fresh cookie-signing secret: 32 random bytes, base64url.
pub fn cookie_secret() -> String {
    random_b64(32)
}

fn iso(unix_seconds: u64) -> String {
    let d = shroom_storage::sigv4::amz_date(unix_seconds); // YYYYMMDDTHHMMSSZ
    format!("{}-{}-{}T{}:{}:{}Z", &d[0..4], &d[4..6], &d[6..8], &d[9..11], &d[11..13], &d[13..15])
}

/// Writes a one-time owner grant and returns the key for `?k=`. Needs a credential that
/// can write `.shroom/` — today, the token's own (a token-holding Mac).
///
/// **The B25 shape, from before the amendment:** it names no device, so a page at iface 2
/// issues no cookie for it (§3.5). Macs now ask the page for the grant
/// (`member::Space::key`); a token Mac whose page is older than iface 2 falls back to
/// [`grant_for_device`].
pub fn grant(s3: &Client, member: &str, owner_class: bool, now_unix: u64) -> Result<String, S3Error> {
    let k = random_b64(32);
    let body = serde_json::json!({
        "member": member,
        "role": if owner_class { "owner" } else { "member" },
        "exp": iso(now_unix + KEY_TTL_SECS),
    });
    let name = format!(".shroom/keys/{}", shroom_storage::sigv4::sha256_hex(k.as_bytes()));
    s3.put(&name, body.to_string().as_bytes(), "application/json")?;
    Ok(k)
}

/// The token Mac's fallback grant (WORKER-INTERFACE §3.3, C2), for a page that answers
/// `/_shroom/api/key` as not shroom-shaped (older than iface 2): written directly over S3
/// in a shape both pages read — `{member:"m1", role:"owner", device, exp}`. An iface-1
/// page reads `member` and `role`; an iface-2 page ignores `role` and checks `device`.
pub fn grant_for_device(s3: &Client, device: &str, now_unix: u64) -> Result<String, S3Error> {
    let k = random_b64(32);
    let body = serde_json::json!({ "member": "m1", "role": "owner", "device": device, "exp": iso(now_unix + KEY_TTL_SECS) });
    let name = format!(".shroom/keys/{}", shroom_storage::sigv4::sha256_hex(k.as_bytes()));
    s3.put(&name, body.to_string().as_bytes(), "application/json")?;
    Ok(k)
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys_and_secrets_are_43_url_safe_characters() {
        for s in [super::cookie_secret(), super::random_b64(32)] {
            assert_eq!(s.len(), 43);
            assert!(s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        }
        assert_ne!(super::cookie_secret(), super::cookie_secret());
        assert_eq!(super::iso(1_790_892_781), "2026-10-01T22:13:01Z");
    }
}
