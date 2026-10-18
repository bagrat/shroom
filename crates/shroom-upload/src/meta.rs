//! `meta/<id>.json`, schema 1 (ARCHITECTURE §5.3): the recording's small, mutable,
//! authoritative record — and, once deleted, its tombstone. Additive evolution only;
//! readers ignore unknown fields; every rewrite is read-modify-write.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Uploading,
    Live,
    Deleted,
}

/// The recording's files, as keys in the bucket. Readers follow these keys, never the
/// names they'd expect: once an edit becomes the video (B197's bake) `hls` is null and `mp4`,
/// `poster` and `teaser` name the edit's own files (`rec/<id>/edit-<rev>…`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// The stream's master playlist; null once the video is one file (baked).
    pub hls: Option<String>,
    pub mp4: Option<String>,
    pub poster: Option<String>,
    /// The unfurl teaser's key (additive, B27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaser: Option<String>,
    pub width: u32,
    pub height: u32,
}

/// The record. Read through [`Meta`]'s own `Deserialize` (below): an `edits` this version
/// can't read stays in [`Meta::extra`] as it was, so the record still reads, and a rewrite
/// keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", remote = "Self")]
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
    /// The saved edit (N63, DECISIONS B195): the list the page and the embed apply as they
    /// play, its revision, and the permanent copy once one is up. Written only by a Mac's
    /// editor ([`crate::library::save_edits`], [`crate::library::set_edit_copy`]). Additive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edits: Option<shroom_edit::Saved>,
    /// Fields this version doesn't know — kept on rewrite.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Serialize for Meta {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.edits.is_some() && self.extra.contains_key("edits") {
            // A typed edit replaces one kept as it was read: never two `edits` keys.
            let mut clean = self.clone();
            clean.extra.remove("edits");
            return Meta::serialize(&clean, s);
        }
        Meta::serialize(self, s)
    }
}

impl<'de> Deserialize<'de> for Meta {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Meta, D::Error> {
        let mut fields = Map::<String, Value>::deserialize(d)?;
        let edits = fields.remove("edits");
        let mut meta = Meta::deserialize(Value::Object(fields)).map_err(serde::de::Error::custom)?;
        match edits {
            None | Some(Value::Null) => {}
            Some(raw) => match serde_json::from_value::<shroom_edit::Saved>(raw.clone()) {
                Ok(saved) => meta.edits = Some(saved),
                Err(_) => {
                    meta.extra.insert("edits".into(), raw);
                }
            },
        }
        Ok(meta)
    }
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
            edits: None,
            extra: Map::new(),
        }
    }

    /// How long the video plays edited (seconds), when the record holds an edit that changes
    /// what plays: its list made safe against the record's own `durationSec` (the Worker
    /// trusts that clock too), its edited length. `None` for no edit or the identity.
    pub fn edited_sec(&self) -> Option<f64> {
        let saved = self.edits.as_ref()?;
        let mut list = saved.list.clone();
        if let Some(d) = self.duration_sec.filter(|d| d.is_finite() && *d > 0.0) {
            list.source_duration = d;
        }
        let list = list.checked();
        (!list.is_identity()).then(|| list.edited_duration())
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
    iso_at(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// Seconds since the Unix epoch as [`now_iso`] writes a time: `2026-10-02T12:00:00Z`.
pub fn iso_at(unix_seconds: u64) -> String {
    let d = shroom_storage::sigv4::amz_date(unix_seconds); // YYYYMMDDTHHMMSSZ
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

    const EDITED: &str = r#"{"schema":1,"id":"x","state":"live","createdAt":"2026-10-02T00:00:00Z","publishedAt":null,
        "deletedAt":null,"title":"T","durationSec":272.4,"media":null,"captions":null,"owner":null,"progress":null,
        "titled":true,"thumb":{"at":3},
        "edits":{"version":1,"source_duration":272.4,"in":8.0,"out":241.0,
          "cuts":[{"start":150.0,"end":162.0,"kind":"manual","confidence":"high","applied":true,"removed":""}],
          "rev":3,"copy":{"rev":3,"file":"edit-3.mp4","duration":221.0,"poster":"edit-3-poster.jpg"}}}"#;

    #[test]
    fn an_edit_round_trips_with_the_record_and_its_unknown_fields() {
        let m: Meta = serde_json::from_str(EDITED).unwrap();
        let saved = m.edits.as_ref().expect("the edit reads");
        assert_eq!((saved.list.in_, saved.list.out, saved.rev, saved.list.cuts.len()), (8.0, 241.0, 3, 1));
        assert_eq!(saved.current_copy().map(|c| c.file.as_str()), Some("edit-3.mp4"));
        assert!(!m.extra.contains_key("edits"), "a readable edit isn't kept twice");
        let back = serde_json::to_value(&m).unwrap();
        let was: Value = serde_json::from_str(EDITED).unwrap();
        assert_eq!(back["edits"], was["edits"], "written back as it was read");
        assert_eq!((back["titled"].clone(), back["thumb"].clone()), (Value::Bool(true), serde_json::json!({"at": 3})));
        assert_eq!(m.edited_sec(), Some(221.0));
        // No edit: no key at all.
        let plain = serde_json::to_value(Meta::uploading("x", "T", "2026-10-02T00:00:00Z")).unwrap();
        assert!(plain.get("edits").is_none());
    }

    #[test]
    fn an_edit_this_version_cant_read_never_costs_the_record() {
        let mut v: Value = serde_json::from_str(EDITED).unwrap();
        v["edits"] = serde_json::json!({ "version": 2, "spans": [[1, 2]], "rev": 7 });
        let m: Meta = serde_json::from_value(v.clone()).expect("the record still reads");
        assert!(m.edits.is_none());
        assert_eq!(m.edited_sec(), None);
        assert_eq!(serde_json::to_value(&m).unwrap()["edits"], v["edits"], "kept as it was on a rewrite");
        // A typed edit written over it replaces it: one key, the new one.
        let mut m2 = m.clone();
        m2.edits = serde_json::from_str::<Meta>(EDITED).unwrap().edits;
        let text = serde_json::to_string(&m2).unwrap();
        assert_eq!(text.matches("\"edits\"").count(), 1, "{text}");
        assert_eq!(serde_json::from_str::<Meta>(&text).unwrap().edits.unwrap().rev, 3);
        // A null reads as none.
        v["edits"] = Value::Null;
        let m: Meta = serde_json::from_value(v).unwrap();
        assert!(m.edits.is_none() && !m.extra.contains_key("edits"));
    }

    #[test]
    fn the_edited_length_is_on_the_records_own_clock() {
        let mut m: Meta = serde_json::from_str(EDITED).unwrap();
        // The list says 272.4 s; the record says 200: the record's clock wins, as the page's.
        m.duration_sec = Some(200.0);
        assert_eq!(m.edited_sec(), Some(200.0 - 8.0 - 12.0));
        let s = m.edits.as_mut().unwrap();
        s.list.in_ = 0.0;
        s.list.out = 272.4;
        s.list.cuts[0].applied = false;
        m.duration_sec = Some(272.4);
        assert_eq!(m.edited_sec(), None, "the identity is no edit");
    }

    /// A baked record (B197): no stream, the edit's own files. It reads, and writes back
    /// with its stream still null, never dropped.
    #[test]
    fn a_record_whose_media_names_no_stream_reads() {
        let json = r#"{"schema":1,"id":"x","state":"live","createdAt":"2026-10-02T00:00:00Z","publishedAt":null,
            "deletedAt":null,"title":"T","durationSec":38.0,"captions":null,"owner":null,"progress":null,
            "media":{"hls":null,"mp4":"rec/x/edit-3.mp4","poster":"rec/x/edit-3-poster.jpg","teaser":"rec/x/edit-3-teaser.mp4","width":1920,"height":1080},
            "bakedFrom":[{"at":"2026-10-20T09:30:00Z","keep":[[2.0,10.0],[20.0,50.0]]}]}"#;
        let m: Meta = serde_json::from_str(json).unwrap();
        let media = m.media.as_ref().unwrap();
        assert_eq!((media.hls.as_deref(), media.mp4.as_deref()), (None, Some("rec/x/edit-3.mp4")));
        let back = serde_json::to_value(&m).unwrap();
        assert_eq!(back["media"]["hls"], Value::Null);
        assert_eq!(back["bakedFrom"][0]["keep"][1], serde_json::json!([20.0, 50.0]));
        // An older record still reads its stream.
        let old: Media = serde_json::from_str(r#"{"hls":"rec/x/master.m3u8","mp4":null,"poster":null,"width":1,"height":1}"#).unwrap();
        assert_eq!(old.hls.as_deref(), Some("rec/x/master.m3u8"));
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
        assert_eq!(iso_at(1_790_942_400), "2026-10-02T12:00:00Z");
        assert_eq!(crate::library::iso_ms(&iso_at(1_791_115_199)), Some(1_791_115_199_000));
    }
}
