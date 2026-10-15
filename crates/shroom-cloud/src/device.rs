//! A Mac's identity in a space and the secrets it makes (WORKER-INTERFACE §3.2, §3.3, §5).
//!
//! Every secret is made on the Mac, before the first call that needs it, and lands in a
//! file first, so a lost reply is retried with the same values (every call is idempotent).
//! Only hashes travel at first: a device's `verifier` (join, enroll), an invite's
//! `codeHash`, a page key's `kHash`. The device secret first travels as the bearer of
//! later calls — over TLS, to the owner's own Worker, never in a URL or argv.
//!
//! The on-disk shapes (§5) are serde structs here; the app writes them (mode 600,
//! atomically) and this crate never touches `~/.shroom`. Every struct holding a secret
//! prints it redacted, so a stray `{:?}` can't put it in a log.

use base64::Engine;
use serde::{Deserialize, Serialize};
use shroom_storage::sigv4::sha256_hex;

/// `n` bytes from the OS's CSPRNG, base64url without padding.
pub fn random_b64(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    getrandom::fill(&mut bytes).expect("the OS random source works");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// A registry id: the prefix, `_`, and 16 base64url characters (12 bytes) — told apart
/// from a recording id by its length, never by its prefix (§2.3, C3).
fn registry_id(prefix: char) -> String {
    format!("{prefix}_{}", random_b64(12))
}

/// A new device id, `d_` + 16.
pub fn new_device_id() -> String {
    registry_id('d')
}

/// A new invite id, `i_` + 16 (the inviting Mac makes it).
pub fn new_invite_id() -> String {
    registry_id('i')
}

/// A new device secret: 32 random bytes, 43 characters.
pub fn new_device_secret() -> String {
    random_b64(32)
}

/// A new join code: 12 random bytes, 16 characters, 96 bits (§3.3).
pub fn new_join_code() -> String {
    random_b64(12)
}

/// A new one-time page key `k`: 32 random bytes, 43 characters. It lives only inside the
/// open that uses it, never on disk or in a log (H7).
pub fn new_page_key() -> String {
    random_b64(32)
}

/// What the Worker stores for a secret: lower-case hex SHA-256 (a device's `verifier`, an
/// invite's `codeHash`, a page key's `kHash`).
pub fn hash(secret: &str) -> String {
    sha256_hex(secret.as_bytes())
}

/// Whether `s` has a registry id's shape for `prefix` (`d`, `m` or `i`).
pub fn is_registry_id(s: &str, prefix: char) -> bool {
    let mut chars = s.chars();
    chars.next() == Some(prefix)
        && chars.next() == Some('_')
        && s.len() == 18
        && s[2..].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Whether `s` is a recording id or join code: 16 base64url characters (B14).
pub fn is_b14(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `device.key` (§5): `{v, host, device, secret}`. Written after join's 200 (from
/// `join.json`) or before enroll (token Macs: made once, reused on retries). **Kept while
/// revoked**, until Leave, so a re-invite can prove `previous`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceKey {
    pub v: u32,
    pub host: String,
    pub device: String,
    pub secret: String,
}

impl DeviceKey {
    /// A fresh identity for `host`.
    pub fn generate(host: &str) -> DeviceKey {
        DeviceKey { v: 1, host: host.to_string(), device: new_device_id(), secret: new_device_secret() }
    }

    /// `hex(sha256(secret))` — what join and enroll send instead of the secret.
    pub fn verifier(&self) -> String {
        hash(&self.secret)
    }

    /// The `Authorization` header's value. Never logged (app logs redact the header).
    pub fn bearer(&self) -> String {
        format!("Bearer {}.{}", self.device, self.secret)
    }

    /// The `previous` a re-invited Mac sends from the identity it kept (§4.5).
    pub fn previous(&self) -> Previous {
        Previous { id: self.device.clone(), secret: self.secret.clone() }
    }
}

impl std::fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceKey").field("host", &self.host).field("device", &self.device).field("secret", &"…").finish()
    }
}

/// A dead identity's proof, sent with a join (§4.5 step 2).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Previous {
    pub id: String,
    pub secret: String,
}

impl std::fmt::Debug for Previous {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Previous").field("id", &self.id).field("secret", &"…").finish()
    }
}

/// `join.json` (§5): a fact file written **before** POST join, so a crash mid-join
/// replays the identical body at boot. Cleared on 200, on a shroom-shaped terminal answer
/// (404 / 410 / 409 `used` / `already-here`), or on `join-space-missing`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinFile {
    pub v: u32,
    pub host: String,
    pub code: String,
    pub device: String,
    pub secret: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub previous: Option<Previous>,
}

impl JoinFile {
    /// The new Mac's key once the join lands (`device.key`, from this file).
    pub fn device_key(&self) -> DeviceKey {
        DeviceKey { v: 1, host: self.host.clone(), device: self.device.clone(), secret: self.secret.clone() }
    }
}

impl std::fmt::Debug for JoinFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinFile")
            .field("host", &self.host)
            .field("code", &"…")
            .field("device", &self.device)
            .field("secret", &"…")
            .field("name", &self.name)
            .field("previous", &self.previous)
            .finish()
    }
}

/// `invite` (§5) in the app's shape: `{v, code, host, inviter?, own_mac?, expires?}` —
/// peek fills the optional fields. The installer writes `{v, link, arrived}` instead
/// (§6.12); [`InviteFile::read`] takes either.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteFile {
    pub v: u32,
    pub code: String,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inviter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_mac: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

#[derive(Deserialize)]
struct InstallerInvite {
    link: String,
}

impl InviteFile {
    /// Reads either shape — the app's, or the installer's `{v, link, arrived}`, normalized.
    /// `None` when it is neither, or its link isn't a join link.
    pub fn read(bytes: &[u8]) -> Option<InviteFile> {
        if let Ok(f) = serde_json::from_slice::<InviteFile>(bytes) {
            return (is_b14(&f.code) && !f.host.is_empty()).then_some(f);
        }
        let i: InstallerInvite = serde_json::from_slice(bytes).ok()?;
        let (host, code) = crate::member::parse_link(&i.link)?;
        Some(InviteFile { v: 1, code, host, inviter: None, own_mac: None, expires: None })
    }
}

impl std::fmt::Debug for InviteFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InviteFile")
            .field("host", &self.host)
            .field("code", &"…")
            .field("inviter", &self.inviter)
            .field("own_mac", &self.own_mac)
            .field("expires", &self.expires)
            .finish()
    }
}

/// `takes/<dir>/cred.json` (§5): a recording's upload credential, so a relaunch resumes
/// within its life. Replaced on re-mint; deleted with the take.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredFile {
    pub v: u32,
    pub purpose: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expires_at: String,
    pub bucket: String,
    pub endpoint: String,
}

impl CredFile {
    /// The S3 credentials it holds.
    pub fn credentials(&self) -> shroom_storage::sigv4::Credentials {
        shroom_storage::sigv4::Credentials {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: Some(self.session_token.clone()),
        }
    }
}

impl std::fmt::Debug for CredFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredFile")
            .field("purpose", &self.purpose)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"…")
            .field("session_token", &"…")
            .field("expires_at", &self.expires_at)
            .field("bucket", &self.bucket)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_secrets_have_their_shapes() {
        let d = new_device_id();
        assert!(is_registry_id(&d, 'd'), "{d}");
        assert!(is_registry_id(&new_invite_id(), 'i'));
        assert!(!is_registry_id(&d, 'm'));
        assert!(!is_registry_id("d_short", 'd'));
        assert_eq!(new_device_secret().len(), 43);
        assert_eq!(new_page_key().len(), 43);
        let code = new_join_code();
        assert!(is_b14(&code), "{code}");
        assert!(!is_b14(&d), "a registry id is never a recording id: told apart by length");
        assert_ne!(new_join_code(), new_join_code());
    }

    #[test]
    fn the_verifier_is_the_secrets_sha256() {
        let k = DeviceKey { v: 1, host: "v.example.workers.dev".into(), device: "d_AAAAAAAAAAAAAAAA".into(), secret: "abc".into() };
        assert_eq!(k.verifier(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(k.bearer(), "Bearer d_AAAAAAAAAAAAAAAA.abc");
    }

    #[test]
    fn no_secret_ever_prints() {
        let k = DeviceKey::generate("v.example.workers.dev");
        let j = JoinFile { v: 1, host: k.host.clone(), code: new_join_code(), device: k.device.clone(), secret: k.secret.clone(), name: None, previous: Some(k.previous()) };
        let c = CredFile { v: 1, purpose: "upload".into(), access_key_id: "id".into(), secret_access_key: "SECRETKEY".into(), session_token: "SESSION".into(), expires_at: "x".into(), bucket: "b".into(), endpoint: "e".into() };
        let i = InviteFile { v: 1, code: j.code.clone(), host: k.host.clone(), inviter: None, own_mac: None, expires: None };
        let printed = format!("{k:?} {j:?} {c:?} {i:?}");
        for secret in [&k.secret, &j.code, &"SECRETKEY".to_string(), &"SESSION".to_string()] {
            assert!(!printed.contains(secret.as_str()), "{printed}");
        }
    }

    #[test]
    fn the_on_disk_shapes_round_trip() {
        let k = DeviceKey::generate("v.example.workers.dev");
        let back: DeviceKey = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
        let c: CredFile = serde_json::from_str(r#"{"v":1,"purpose":"upload","accessKeyId":"a","secretAccessKey":"b","sessionToken":"c","expiresAt":"2026-10-09T12:00:00.000Z","bucket":"shroom-space","endpoint":"https://x.r2.cloudflarestorage.com"}"#).unwrap();
        assert_eq!(c.credentials().session_token.as_deref(), Some("c"));
        let j: JoinFile = serde_json::from_str(r#"{"v":1,"host":"h","code":"AbCdEfGhIjKlMnOp","device":"d_AAAAAAAAAAAAAAAA","secret":"s"}"#).unwrap();
        assert_eq!((j.name, j.previous), (None, None));
    }

    #[test]
    fn the_invite_file_takes_the_installers_shape_too() {
        let app = InviteFile::read(br#"{"v":1,"code":"AbCdEfGhIjKlMnOp","host":"v.myshroom.workers.dev","inviter":"Bagrat"}"#).unwrap();
        assert_eq!((app.host.as_str(), app.inviter.as_deref()), ("v.myshroom.workers.dev", Some("Bagrat")));
        let installer = InviteFile::read(br#"{"v":1,"link":"v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp","arrived":"2026-10-02T12:00:00Z"}"#).unwrap();
        assert_eq!((installer.host.as_str(), installer.code.as_str()), ("v.myshroom.workers.dev", "AbCdEfGhIjKlMnOp"));
        assert!(InviteFile::read(br#"{"v":1,"link":"https://evil.example/elsewhere","arrived":"x"}"#).is_none());
        assert!(InviteFile::read(b"not json").is_none());
    }
}
