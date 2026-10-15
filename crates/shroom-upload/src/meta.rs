//! `meta/<id>.json`, schema 1 (ARCHITECTURE §5.3): the recording's small, mutable,
//! authoritative record — and, once deleted, its tombstone. Additive evolution only;
//! readers ignore unknown fields; every rewrite is read-modify-write.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Uploading,
    Live,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub hls: String,
    pub mp4: Option<String>,
    pub poster: Option<String>,
    /// The unfurl teaser's key (additive, B27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaser: Option<String>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub schema: u32,
    pub id: String,
    pub state: State,
    pub created_at: String,
    pub published_at: Option<String>,
    pub deleted_at: Option<String>,
    pub title: String,
    pub duration_sec: Option<f64>,
    pub media: Option<Media>,
    pub captions: Option<Value>,
    /// The member who made it (N3/N40); filled once members exist.
    pub owner: Option<String>,
    /// Upload truth for the page (ARCHITECTURE §9.3): files confirmed / files known, and
    /// when it last moved — the page's percentage, its stall clock, and (with every file
    /// counted, the second-class ones too) its “Fully live ✓” (DECISIONS B49).
    pub progress: Option<Progress>,
    /// The markers dropped while recording, as seconds in the joined timeline (N24's
    /// “metadata-only in v1”, ARCHITECTURE §16 #30, DECISIONS B49). Additive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markers: Option<Vec<f64>>,
    /// Fields this version doesn't know — kept on rewrite.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: u32,
    pub total: u32,
    pub updated_at: String,
}

impl Meta {
    pub fn uploading(id: &str, title: &str, created_at: &str) -> Meta {
        Meta {
            schema: 1,
            id: id.to_string(),
            state: State::Uploading,
            created_at: created_at.to_string(),
            published_at: None,
            deleted_at: None,
            title: title.to_string(),
            duration_sec: None,
            media: None,
            captions: None,
            owner: None,
            progress: None,
            markers: None,
            extra: Map::new(),
        }
    }
}

/// A record as a listing hands it over. A tombstone carries no title (and may carry a null
/// `createdAt`), so it never reads as a whole [`Meta`]; read as one anyway, it would leave a
/// deleted video's cached row live. It reads as a deleted record instead.
pub fn listed(v: Value) -> Option<Meta> {
    if v.get("state").and_then(Value::as_str) != Some("deleted") {
        return serde_json::from_value(v).ok();
    }
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let mut m = Meta::uploading(&text("id")?, "", &text("createdAt").unwrap_or_default());
    m.state = State::Deleted;
    m.deleted_at = text("deletedAt");
    Some(m)
}

/// ISO-8601 UTC, seconds precision.
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let d = shroom_storage::sigv4::amz_date(secs); // YYYYMMDDTHHMMSSZ
    format!("{}-{}-{}T{}:{}:{}Z", &d[0..4], &d[4..6], &d[6..8], &d[9..11], &d[11..13], &d[13..15])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_unknown_fields() {
        let json = r#"{"schema":1,"id":"x","state":"live","createdAt":"2026-10-02T00:00:00Z","publishedAt":null,
            "deletedAt":null,"title":"Untitled — Oct 2","durationSec":6.0,"media":null,"captions":null,"owner":null,
            "progress":null,"markers":[12.5]}"#;
        let m: Meta = serde_json::from_str(json).unwrap();
        assert_eq!(m.state, State::Live);
        let back = serde_json::to_value(&m).unwrap();
        assert_eq!(back["markers"], serde_json::json!([12.5]));
        assert_eq!(back["createdAt"], "2026-10-02T00:00:00Z");
    }

    #[test]
    fn a_listed_tombstone_reads_as_deleted() {
        // As the Mac writes one (crates/shroom-upload/src/library.rs `delete`), with or
        // without the old createdAt, and as the Worker lists a sealed id (B125).
        for json in [
            r#"{"schema":1,"id":"x","state":"deleted","deletedAt":"2026-10-03T00:00:00Z","createdAt":"2026-10-02T00:00:00Z","owner":"m1"}"#,
            r#"{"schema":1,"id":"x","state":"deleted","deletedAt":"2026-10-03T00:00:00Z","createdAt":null,"owner":null}"#,
            r#"{"schema":1,"id":"x","state":"deleted","createdAt":null}"#,
        ] {
            assert!(serde_json::from_str::<Meta>(json).is_err(), "a tombstone isn't a whole Meta: {json}");
            let m = listed(serde_json::from_str(json).unwrap()).expect(json);
            assert_eq!(m.state, State::Deleted, "{json}");
            assert_eq!(m.id, "x");
        }
        let live = r#"{"schema":1,"id":"x","state":"live","createdAt":"2026-10-02T00:00:00Z","title":"T"}"#;
        assert_eq!(listed(serde_json::from_str(live).unwrap()).unwrap().title, "T");
        // A record that is neither whole nor a tombstone still reads as nothing.
        assert!(listed(serde_json::json!({"state": "live"})).is_none());
    }

    #[test]
    fn formats_now() {
        let n = now_iso();
        assert_eq!(n.len(), 20);
        assert!(n.ends_with('Z') && &n[10..11] == "T");
    }
}
