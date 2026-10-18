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

/// What the space's AI did with a take, as the record says it, in one line for the Mac's log
/// (N66, DECISIONS B206 f): named, the early title, let go, or waiting for the free day — its
/// model, time, counts and codes, never a word said or a title. None for a take it hasn't touched.
pub fn ai_note(m: &Meta) -> Option<String> {
    let model = |n: &Value| n.get("model").and_then(Value::as_str).map(|s| s.rsplit('/').next().unwrap_or(s).to_string());
    let num = |n: &Value, k: &str| n.get(k).and_then(Value::as_u64);
    let text = |n: &Value, k: &str| n.get(k).and_then(Value::as_str).map(|s| s.chars().filter(|c| c.is_ascii_alphanumeric() || "-:.".contains(*c)).take(40).collect::<String>());
    if let Some(n) = m.extra.get("named").filter(|n| n.is_object()) {
        if n.get("failed").and_then(Value::as_bool) == Some(true) {
            return Some(format!("let go: error {}", text(n, "code").unwrap_or_else(|| "no code".into())));
        }
        let ms = num(n, "ms").map(|ms| format!(" in {ms} ms")).unwrap_or_default();
        let Some(by) = model(n) else {
            return Some(format!("not named: {} words", num(n, "words").unwrap_or(0)));
        };
        if n.get("early").and_then(Value::as_bool) == Some(true) {
            return Some(format!("early title by {by}{ms}"));
        }
        let chapters = n.get("chapters").and_then(Value::as_array).map_or(0, Vec::len);
        let tokens = n.get("tokens").and_then(Value::as_array).and_then(|t| Some(format!(", {}+{} tokens", t.first()?.as_u64()?, t.get(1)?.as_u64()?))).unwrap_or_default();
        let fallback = text(n, "fallback").map(|c| format!(" (the first model failed: {c})")).unwrap_or_default();
        return Some(format!("named by {by}{ms}: {chapters} chapters from {} words{tokens}{fallback}", num(n, "words").unwrap_or(0)));
    }
    let naming = m.extra.get("naming")?;
    (naming.get("later").and_then(Value::as_str) == Some("daily")).then(|| {
        let since = text(naming, "at").map(|a| format!(" since {a}")).unwrap_or_default();
        let code = text(naming, "code").map(|c| format!(" ({c})")).unwrap_or_default();
        format!("waiting for the free day{since}{code}")
    })
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

    /// Bagrat, 2026-10-18: a report from a Mac shows what the space did with a take — the
    /// steps its schedule took too, which reach the Mac only as the record. One line, no word
    /// said and no title, as the report's cleaning leaves it.
    #[test]
    fn a_records_ai_state_is_one_line_with_nothing_said_in_it() {
        let with = |extra: serde_json::Value| -> Meta {
            let mut v = serde_json::json!({"schema":1,"id":"x","state":"live","createdAt":"2026-10-18T14:00:00Z","title":"Planning the spring launch"});
            v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            serde_json::from_value(v).unwrap()
        };
        let named = serde_json::json!({"by":"ai","model":"@cf/openai/gpt-oss-120b","title":"Planning the spring launch","chapters":[[0,"Where we are"],[62,"The budget"]],"words":238,"ms":1530,"tokens":[900,80],"at":"2026-10-18T14:05:00Z"});
        let cases = [
            (serde_json::json!({"named": named}), Some("named by gpt-oss-120b in 1530 ms: 2 chapters from 238 words, 900+80 tokens")),
            (serde_json::json!({"named": {"by":"ai","model":"@cf/google/gemma-4-26b-a4b-it","title":"T","chapters":[],"words":40,"ms":800,"fallback":"5007"}}), Some("named by gemma-4-26b-a4b-it in 800 ms: 0 chapters from 40 words (the first model failed: 5007)")),
            (serde_json::json!({"named": {"by":"ai","early":true,"model":"@cf/openai/gpt-oss-120b","title":"Planning","chapters":[],"words":30,"ms":812}}), Some("early title by gpt-oss-120b in 812 ms")),
            (serde_json::json!({"named": {"by":"ai","model":null,"title":null,"chapters":[],"words":0,"failed":true,"code":"5007"}}), Some("let go: error 5007")),
            (serde_json::json!({"named": {"by":"ai","model":null,"title":null,"chapters":[],"words":8}}), Some("not named: 8 words")),
            (serde_json::json!({"naming": {"later":"daily","at":"2026-10-18T23:14:00Z","code":"4006"}}), Some("waiting for the free day since 2026-10-18T23:14:00Z (4006)")),
            (serde_json::json!({}), None),
        ];
        for (extra, want) in cases {
            let got = ai_note(&with(extra.clone()));
            assert_eq!(got.as_deref(), want, "{extra}");
            if let Some(line) = got {
                assert!(!line.contains("spring") && !line.contains("budget") && !line.contains("Where"), "{line}");
                assert_eq!(shroom_core::report::clean_line(&line), line);
            }
        }
    }

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
