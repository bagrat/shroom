//! The space's registry, from the Mac's side (WORKER-INTERFACE §2.2, §6.11).
//!
//! The roster (`.shroom/registry/roster.json`) holds members, Macs and invites; the
//! Worker changes it, one compare-and-swap at a time, and **never creates it**. Wiring
//! seeds it here, create-only over S3, before the page that needs it goes up — on a fresh
//! space and on the first update of a space wired before Region 6. The only other thing
//! the Mac ever does to it is [`set_aside`]: the token Mac's repair pass copying a roster
//! that won't parse to one side, kept for hand repair. Nothing here deletes or rewrites it.

use serde_json::{Value, json};
use shroom_storage::s3::{Client, S3Error};

/// The roster's key in the space's bucket.
pub const ROSTER: &str = ".shroom/registry/roster.json";
/// Where the ownership ledger lives (§2.4): `<LEDGER><id>`, written create-only.
pub const LEDGER: &str = ".shroom/registry/recs/";

/// RFC 3339 for a unix time, seconds precision.
pub fn iso(unix_seconds: u64) -> String {
    let d = shroom_storage::sigv4::amz_date(unix_seconds); // YYYYMMDDTHHMMSSZ
    format!("{}-{}-{}T{}:{}:{}Z", &d[0..4], &d[4..6], &d[6..8], &d[9..11], &d[11..13], &d[13..15])
}

/// A fresh roster (§2.3, v1): member #1 already in it, owner-class, so a link made from
/// any owner Mac has its member to join. Its name stays empty until the first enroll
/// fills it (the Worker refreshes it whenever it differs).
pub fn initial(owner_name: Option<&str>, now_unix: u64) -> Value {
    let at = iso(now_unix);
    json!({
        "v": 1,
        "createdAt": at,
        "members": { "m1": { "role": "owner", "name": owner_name, "state": "active", "joinedAt": at } },
        "devices": {},
        "invites": {},
    })
}

/// Seeds the roster, create-only (`If-None-Match: *`): `true` when this call made it,
/// `false` when one was already there (left exactly as found — idempotent wiring).
pub fn seed(s3: &Client, owner_name: Option<&str>, now_unix: u64) -> Result<bool, S3Error> {
    s3.put_if_absent(ROSTER, initial(owner_name, now_unix).to_string().as_bytes(), "application/json")
}

/// Whether `bytes` read as a roster the Worker accepts (the same test as its
/// `readRoster`: a JSON object whose `members`, `devices` and `invites` are objects).
pub fn is_roster(bytes: &[u8]) -> bool {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return false };
    ["members", "devices", "invites"].iter().all(|t| v.get(t).is_some_and(Value::is_object))
}

/// What [`set_aside`] found.
#[derive(Debug, Clone, PartialEq)]
pub enum Aside {
    /// No roster at all: `unwired` — seed it, don't set anything aside.
    Absent,
    /// It parses: not broken (a 503 `broken` seen earlier was something else). Untouched.
    Intact,
    /// Broken, and now copied to this key (create-only). The roster itself is untouched.
    SetAside(String),
}

/// The repair pass's first step for a `broken` roster (§6.11, C8): read it, confirm it
/// doesn't parse, and copy it aside to `.shroom/registry/roster.broken-<ts>.json`,
/// create-only. Never deletes or rewrites the roster — reseeding waits on Bagrat (open
/// question 19). A transient read error is an error, never "broken".
pub fn set_aside(s3: &Client, now_unix: u64) -> Result<Aside, S3Error> {
    let Some(bytes) = s3.get(ROSTER)? else { return Ok(Aside::Absent) };
    if is_roster(&bytes) {
        return Ok(Aside::Intact);
    }
    let key = format!(".shroom/registry/roster.broken-{now_unix}.json");
    s3.put_if_absent(&key, &bytes, "application/json")?;
    Ok(Aside::SetAside(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_roster_has_member_one_and_nothing_else() {
        let r = initial(None, 1_790_942_400);
        assert_eq!(r["v"], 1);
        assert_eq!(r["createdAt"], "2026-10-02T12:00:00Z");
        assert_eq!(r["members"]["m1"], json!({"role": "owner", "name": null, "state": "active", "joinedAt": "2026-10-02T12:00:00Z"}));
        assert_eq!((r["devices"].as_object().unwrap().len(), r["invites"].as_object().unwrap().len()), (0, 0));
        assert!(is_roster(r.to_string().as_bytes()));
        assert_eq!(initial(Some("Bagrat"), 0)["members"]["m1"]["name"], "Bagrat");
    }

    #[test]
    fn only_a_roster_shaped_object_is_a_roster() {
        assert!(!is_roster(b"{ hand-edited, oops"));
        assert!(!is_roster(br#"{"members":{},"devices":[],"invites":{}}"#));
        assert!(!is_roster(b"[]"));
        assert!(is_roster(br#"{"v":1,"members":{},"devices":{},"invites":{},"extra":true}"#));
    }
}
