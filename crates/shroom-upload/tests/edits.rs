//! The editor's writes against the L1 fake's S3 (N63, DECISIONS B195): a save is the next
//! revision of the record's `edits`, read-modify-write, the rest of the record kept; a
//! permanent copy is named only while its revision is the saved one, its landing keeps the
//! original two more days (B197), and the files of a copy the record no longer names go; the
//! identity drops the copy; a tombstone stops both. And
//! the editor's fetch of a recording streams its single-file copy out of the bucket.

use shroom_cloudfake::{Fake, State};
use shroom_edit::{EditCopy, EditList, History, Op};
use shroom_storage::s3::{Bucket, Client, READ_CHUNK};
use shroom_storage::sigv4::Credentials;
use shroom_upload::library;
use shroom_upload::meta::{Meta, State as MetaState};
use shroom_upload::upload::UploadError;

const A: &str = "AAAAAAAAAAAAAAAA";
/// When the copies below land.
const NOW: &str = "2026-10-18T09:30:00Z";

fn space() -> (Fake, Client) {
    let mut st = State::fresh();
    st.buckets.insert("shroom-space".into(), Default::default());
    let fake = Fake::start(st);
    let s3 = Client::new(Bucket {
        endpoint: fake.r2(),
        region: "auto".into(),
        name: "shroom-space".into(),
        credentials: Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None },
    });
    (fake, s3)
}

/// A live minute-long recording, as the Worker leaves it after a rename (a field this
/// crate doesn't know), with its original single-file copy beside it.
fn record(s3: &Client, id: &str) {
    let mut m = Meta::uploading(id, "Setting up shroom", "2026-10-02T10:00:00Z");
    m.state = MetaState::Live;
    m.duration_sec = Some(60.0);
    m.extra.insert("titled".into(), serde_json::Value::Bool(true));
    s3.put(&format!("meta/{id}.json"), &serde_json::to_vec(&m).unwrap(), "application/json").unwrap();
    s3.put(&format!("rec/{id}/video.mp4"), b"the original", "video/mp4").unwrap();
    s3.put(&format!("rec/{id}/poster.jpg"), b"its poster", "image/jpeg").unwrap();
}

fn read(s3: &Client, id: &str) -> serde_json::Value {
    serde_json::from_slice(&s3.get(&format!("meta/{id}.json")).unwrap().unwrap()).unwrap()
}

/// Trimmed to 2–50 s with 10–20 s cut: 38 s play.
fn edited() -> EditList {
    let mut h = History::new(EditList::new(60.0));
    assert!(h.act(&Op::Trim { in_: 2.0, out: 50.0 }) && h.act(&Op::Cut { start: 10.0, end: 20.0 }));
    h.now
}

fn copy(rev: u64) -> EditCopy {
    EditCopy { rev, file: EditCopy::file_for(rev), duration: 38.0, poster: Some(EditCopy::poster_for(rev)), teaser: Some(EditCopy::teaser_for(rev)) }
}

/// The copy's three files, sent as the editor lane sends them before naming the copy.
fn send(s3: &Client, id: &str, c: &EditCopy) {
    for f in c.files() {
        s3.put(&format!("rec/{id}/{f}"), f.as_bytes(), "application/octet-stream").unwrap();
    }
}

fn files(fake: &Fake, id: &str) -> Vec<String> {
    let prefix = format!("rec/{id}/");
    fake.with(|s| s.buckets["shroom-space"].keys().filter_map(|k| k.strip_prefix(&prefix).map(str::to_string)).collect())
}

#[test]
fn a_save_is_the_next_revision_and_keeps_the_rest_of_the_record() {
    let (_fake, s3) = space();
    record(&s3, A);
    assert_eq!(library::save_edits(&s3, A, &edited(), 1).unwrap(), 1);
    let v = read(&s3, A);
    assert_eq!((v["edits"]["rev"].as_u64(), v["edits"]["in"].as_f64(), v["edits"]["out"].as_f64()), (Some(1), Some(2.0), Some(50.0)));
    assert_eq!(v["edits"]["cuts"][0]["kind"], "manual");
    assert!(v["edits"].get("copy").is_none(), "no copy yet");
    assert_eq!((v["title"].as_str(), v["titled"].as_bool(), v["state"].as_str()), (Some("Setting up shroom"), Some(true), Some("live")));
    let m: Meta = serde_json::from_value(v).unwrap();
    assert_eq!(m.edited_sec(), Some(38.0));

    // Every other rewrite of the record keeps the edit (a rename here).
    library::rename(&s3, A, "Setting up shroom, edited").unwrap();
    assert_eq!(read(&s3, A)["edits"]["rev"], 1);

    // An editor opened before that save (on no edit: it asks for 1) still lands after it.
    assert_eq!(library::save_edits(&s3, A, &edited(), 1).unwrap(), 2);
    // One opened on revision 2 asks for 3 and gets it; one from the future isn't held back.
    assert_eq!(library::save_edits(&s3, A, &edited(), 3).unwrap(), 3);
    assert_eq!(library::save_edits(&s3, A, &edited(), 9).unwrap(), 9);
    assert_eq!(read(&s3, A)["edits"]["rev"], 9);
}

#[test]
fn a_copy_is_named_only_while_its_revision_is_the_saved_one() {
    let (fake, s3) = space();
    record(&s3, A);
    library::save_edits(&s3, A, &edited(), 1).unwrap();
    send(&s3, A, &copy(1));
    assert!(library::set_edit_copy(&s3, A, &copy(1), NOW).unwrap());
    let v = read(&s3, A);
    assert_eq!((v["edits"]["copy"]["rev"].as_u64(), v["edits"]["copy"]["file"].as_str()), (Some(1), Some("edit-1.mp4")));
    assert_eq!(v["edits"]["copy"]["teaser"], "edit-1-teaser.mp4");
    // Naming it again changes nothing and deletes nothing.
    assert!(library::set_edit_copy(&s3, A, &copy(1), NOW).unwrap());
    assert!(files(&fake, A).contains(&"edit-1.mp4".to_string()));

    // A newer save leaves the old copy named, now stale; its files stay until the next lands.
    assert_eq!(library::save_edits(&s3, A, &edited(), 2).unwrap(), 2);
    assert_eq!(read(&s3, A)["edits"]["copy"]["rev"], 1);
    assert!(files(&fake, A).contains(&"edit-1-poster.jpg".to_string()));
    send(&s3, A, &copy(2));
    assert!(library::set_edit_copy(&s3, A, &copy(2), NOW).unwrap());
    let mut left = files(&fake, A);
    left.sort();
    assert_eq!(left, ["edit-2-poster.jpg", "edit-2-teaser.mp4", "edit-2.mp4", "poster.jpg", "video.mp4"], "the previous copy went, the original stays");

    // Another Mac's slow cut of revision 1 lands late: nothing is written, its files go,
    // and the copy the record names stays.
    let before = s3.get_with_etag(&format!("meta/{A}.json")).unwrap().unwrap().1;
    send(&s3, A, &copy(1));
    assert!(!library::set_edit_copy(&s3, A, &copy(1), NOW).unwrap());
    assert_eq!(s3.get_with_etag(&format!("meta/{A}.json")).unwrap().unwrap().1, before, "the record wasn't rewritten");
    assert_eq!(read(&s3, A)["edits"]["copy"]["rev"], 2);
    let mut left = files(&fake, A);
    left.sort();
    assert_eq!(left, ["edit-2-poster.jpg", "edit-2-teaser.mp4", "edit-2.mp4", "poster.jpg", "video.mp4"]);

    // A copy is only ever named by a copy's own file names.
    let forged = EditCopy { file: "video.mp4".into(), ..copy(2) };
    assert!(library::set_edit_copy(&s3, A, &forged, NOW).is_err());
    assert_eq!(read(&s3, A)["edits"]["copy"]["file"], "edit-2.mp4");
    assert!(files(&fake, A).contains(&"video.mp4".to_string()));

    // A copy for a record with no edit at all is stale too.
    record(&s3, "BBBBBBBBBBBBBBBB");
    send(&s3, "BBBBBBBBBBBBBBBB", &copy(1));
    assert!(!library::set_edit_copy(&s3, "BBBBBBBBBBBBBBBB", &copy(1), NOW).unwrap());
    assert!(files(&fake, "BBBBBBBBBBBBBBBB").iter().all(|f| !f.starts_with("edit-")));
}

/// B197 (b): a copy's landing sets when the original goes, two days on; a retry keeps that
/// time, the next revision's copy sets its own, a late copy of an old revision sets none.
#[test]
fn a_copys_landing_keeps_the_original_two_more_days() {
    let (_fake, s3) = space();
    record(&s3, A);
    library::save_edits(&s3, A, &edited(), 1).unwrap();
    send(&s3, A, &copy(1));
    assert!(library::set_edit_copy(&s3, A, &copy(1), "2026-10-18T09:30:00Z").unwrap());
    assert_eq!(read(&s3, A)["edits"]["keep_until"], "2026-10-20T09:30:00Z");
    // The same copy named again later (a lost reply, retried) doesn't move it.
    assert!(library::set_edit_copy(&s3, A, &copy(1), "2026-10-18T11:00:00.250Z").unwrap());
    assert_eq!(read(&s3, A)["edits"]["keep_until"], "2026-10-20T09:30:00Z");

    // A newer save keeps it as it is, while that revision's copy is made…
    library::save_edits(&s3, A, &edited(), 2).unwrap();
    assert_eq!(read(&s3, A)["edits"]["keep_until"], "2026-10-20T09:30:00Z");
    // …a late copy of the old revision writes nothing…
    send(&s3, A, &copy(1));
    assert!(!library::set_edit_copy(&s3, A, &copy(1), "2026-10-19T08:00:00Z").unwrap());
    assert_eq!(read(&s3, A)["edits"]["keep_until"], "2026-10-20T09:30:00Z");
    // …and the new copy's landing starts its own two days.
    send(&s3, A, &copy(2));
    assert!(library::set_edit_copy(&s3, A, &copy(2), "2026-10-19T08:00:00Z").unwrap());
    let v = read(&s3, A);
    assert_eq!((v["edits"]["keep_until"].as_str(), v["edits"]["copy"]["rev"].as_u64()), (Some("2026-10-21T08:00:00Z"), Some(2)));
    let m: Meta = serde_json::from_value(v).unwrap();
    assert_eq!(m.edits.unwrap().keep_until.as_deref(), Some("2026-10-21T08:00:00Z"));

    // A time that isn't one names nothing.
    library::save_edits(&s3, A, &edited(), 3).unwrap();
    send(&s3, A, &copy(3));
    let before = s3.get_with_etag(&format!("meta/{A}.json")).unwrap().unwrap().1;
    assert!(library::set_edit_copy(&s3, A, &copy(3), "yesterday").is_err());
    assert_eq!(s3.get_with_etag(&format!("meta/{A}.json")).unwrap().unwrap().1, before);
}

#[test]
fn the_identity_drops_the_copy_and_its_files() {
    let (fake, s3) = space();
    record(&s3, A);
    library::save_edits(&s3, A, &edited(), 1).unwrap();
    send(&s3, A, &copy(1));
    assert!(library::set_edit_copy(&s3, A, &copy(1), NOW).unwrap());
    assert!(read(&s3, A)["edits"]["keep_until"].is_string());
    // Revert to original, saved.
    assert_eq!(library::save_edits(&s3, A, &EditList::new(60.0), 2).unwrap(), 2);
    let v = read(&s3, A);
    assert_eq!(v["edits"]["rev"], 2);
    assert!(v["edits"].get("copy").is_none(), "{v}");
    assert!(v["edits"].get("keep_until").is_none(), "nothing kept for: {v}");
    let m: Meta = serde_json::from_value(v).unwrap();
    assert_eq!(m.edited_sec(), None, "no edit");
    let mut left = files(&fake, A);
    left.sort();
    assert_eq!(left, ["poster.jpg", "video.mp4"]);
}

#[test]
fn an_edit_never_deletes_a_file_that_isnt_a_copys() {
    let (fake, s3) = space();
    record(&s3, A);
    // A record whose copy names the original (written by anything but this code).
    let mut v = read(&s3, A);
    v["edits"] = serde_json::json!({ "version": 1, "source_duration": 60.0, "in": 0.0, "out": 50.0, "cuts": [], "rev": 1,
        "copy": { "rev": 1, "file": "video.mp4", "duration": 50.0, "poster": "poster.jpg" } });
    s3.put(&format!("meta/{A}.json"), v.to_string().as_bytes(), "application/json").unwrap();
    library::save_edits(&s3, A, &EditList::new(60.0), 2).unwrap();
    assert!(read(&s3, A)["edits"].get("copy").is_none());
    let mut left = files(&fake, A);
    left.sort();
    assert_eq!(left, ["poster.jpg", "video.mp4"]);
}

#[test]
fn a_tombstone_stops_a_save_and_a_copy() {
    let (_fake, s3) = space();
    record(&s3, A);
    library::delete(&s3, A).unwrap();
    assert!(matches!(library::save_edits(&s3, A, &edited(), 1), Err(UploadError::Tombstoned)));
    assert!(matches!(library::set_edit_copy(&s3, A, &copy(1), NOW), Err(UploadError::Tombstoned)));
    assert_eq!(read(&s3, A)["state"], "deleted");
}

#[test]
fn the_editor_streams_a_recordings_single_file_copy_out_of_the_bucket() {
    let (_fake, s3) = space();
    let body: Vec<u8> = (0..(2 * READ_CHUNK + 4321)).map(|i| (i % 253) as u8).collect();
    s3.put(&format!("rec/{A}/video.mp4"), &body, "video/mp4").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.mp4");
    let mut last = (0, 0);
    let mut steps = 0;
    assert!(s3.get_to_file(&format!("rec/{A}/video.mp4"), &path, &mut |d, t| {
        last = (d, t);
        steps += 1;
    }, &|| false).unwrap());
    assert!(std::fs::read(&path).unwrap() == body);
    assert_eq!(last, (body.len() as u64, body.len() as u64));
    assert!(steps >= 3, "{steps} steps");
    assert!(!s3.get_to_file(&format!("rec/{A}/nothing.mp4"), &dir.path().join("x.mp4"), &mut |_, _| {}, &|| false).unwrap());
    assert!(!dir.path().join("x.mp4").exists());
}
