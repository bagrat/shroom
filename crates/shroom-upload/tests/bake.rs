//! The bake against the L1 fake's S3 (B197 (c)): once a saved edit's copy is current, the
//! record points at the copy's own files and forgets the edit, then the original's files
//! go — and only they, and an earlier bake's. Repeatable after any interruption; an edit of
//! a baked video bakes again; nothing a record's media names is ever deleted.

use shroom_cloudfake::{Fake, State};
use shroom_edit::{EditCopy, EditList, History, Op};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::library;
use shroom_upload::meta::{Media, Meta, State as MetaState};
use shroom_upload::upload::UploadError;

const A: &str = "AAAAAAAAAAAAAAAA";
const BUCKET: &str = "shroom-space";
/// When the first copy lands, and two days on, when it's baked.
const LANDED: &str = "2026-10-18T09:30:00Z";
const DUE: &str = "2026-10-20T09:30:00Z";

fn space() -> (Fake, Client) {
    let mut st = State::fresh();
    st.buckets.insert(BUCKET.into(), Default::default());
    let fake = Fake::start(st);
    let s3 = Client::new(Bucket {
        endpoint: fake.r2(),
        region: "auto".into(),
        name: BUCKET.into(),
        credentials: Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None },
    });
    (fake, s3)
}

/// The original's files as an upload leaves them: the stream, the single-file copy, the
/// poster and the teaser.
const ORIGINAL: [&str; 8] = ["master.m3u8", "v0/init.mp4", "v0/seg_00000.m4s", "v0/seg_00001.m4s", "v0/stream.m3u8", "video.mp4", "poster.jpg", "teaser.mp4"];

/// A live minute-long recording, uploaded and renamed: its stream and extras, the markers
/// dropped while recording, a picture Copy thumbnail stored beside it.
fn record(s3: &Client, id: &str) {
    let mut m = Meta::uploading(id, "Setting up shroom", "2026-10-02T10:00:00Z");
    m.state = MetaState::Live;
    m.duration_sec = Some(60.0);
    m.markers = Some(vec![5.0, 15.0, 45.0, 49.99, 55.0]);
    m.media = Some(Media {
        hls: Some(format!("rec/{id}/master.m3u8")),
        mp4: Some(format!("rec/{id}/video.mp4")),
        poster: Some(format!("rec/{id}/poster.jpg")),
        teaser: Some(format!("rec/{id}/teaser.mp4")),
        width: 1920,
        height: 1080,
    });
    m.extra.insert("titled".into(), true.into());
    m.extra.insert("thumb".into(), serde_json::json!({ "title": "Setting up shroom", "jpg": "thumb-0123456789abcdef.jpg" }));
    s3.put(&format!("meta/{id}.json"), &serde_json::to_vec(&m).unwrap(), "application/json").unwrap();
    for f in ORIGINAL.iter().chain(&["thumb-0123456789abcdef.jpg"]) {
        s3.put(&format!("rec/{id}/{f}"), f.as_bytes(), "application/octet-stream").unwrap();
    }
}

fn read(s3: &Client, id: &str) -> serde_json::Value {
    serde_json::from_slice(&s3.get(&format!("meta/{id}.json")).unwrap().unwrap()).unwrap()
}

fn etag(s3: &Client, id: &str) -> String {
    s3.get_with_etag(&format!("meta/{id}.json")).unwrap().unwrap().1
}

fn files(fake: &Fake, id: &str) -> Vec<String> {
    let prefix = format!("rec/{id}/");
    let mut v: Vec<String> = fake.with(|s| s.buckets[BUCKET].keys().filter_map(|k| k.strip_prefix(&prefix).map(str::to_string)).collect());
    v.sort();
    v
}

/// Trimmed to 2–50 s with 10–20 s cut: 38 s play, in two ranges.
fn edited(len: f64) -> EditList {
    let mut h = History::new(EditList::new(len));
    assert!(h.act(&Op::Trim { in_: 2.0, out: 50.0f64.min(len) }) && h.act(&Op::Cut { start: 10.0, end: 20.0 }));
    h.now
}

/// Saves `list`, makes its copy (`duration` long, as the encoder measured it), sends the
/// copy's files and names it, as the editor lane does. Returns the revision.
fn save_and_copy(s3: &Client, id: &str, list: &EditList, duration: f64) -> u64 {
    let rev = library::save_edits(s3, id, list, 1).unwrap();
    let copy = EditCopy { rev, file: EditCopy::file_for(rev), duration, poster: Some(EditCopy::poster_for(rev)), teaser: Some(EditCopy::teaser_for(rev)) };
    for f in copy.files() {
        s3.put(&format!("rec/{id}/{f}"), f.as_bytes(), "application/octet-stream").unwrap();
    }
    assert!(library::set_edit_copy(s3, id, &copy, LANDED).unwrap());
    rev
}

#[test]
fn a_bake_makes_the_edit_the_video_and_deletes_the_original() {
    let (fake, s3) = space();
    record(&s3, A);
    assert_eq!(save_and_copy(&s3, A, &edited(60.0), 37.96), 1);
    assert!(library::bake(&s3, A, DUE).unwrap());

    let v = read(&s3, A);
    assert_eq!(
        v["media"],
        serde_json::json!({ "hls": null, "mp4": format!("rec/{A}/edit-1.mp4"), "poster": format!("rec/{A}/edit-1-poster.jpg"),
            "teaser": format!("rec/{A}/edit-1-teaser.mp4"), "width": 1920, "height": 1080 })
    );
    assert_eq!(v["durationSec"], 37.96);
    // 5 s → 3 s; 15 s is in the cut; 45 s → 33 s; 49.99 s → 37.99 s, past the copy's end;
    // 55 s is past the trim.
    assert_eq!(v["markers"], serde_json::json!([3.0, 33.0, 37.96]));
    assert!(v.get("edits").is_none(), "{v}");
    assert_eq!(v["bakedFrom"], serde_json::json!([{ "at": DUE, "rev": 1, "keep": [[2.0, 10.0], [20.0, 50.0]] }]));
    // The rest of the record is as it was.
    assert_eq!((v["title"].as_str(), v["titled"].as_bool(), v["state"].as_str()), (Some("Setting up shroom"), Some(true), Some("live")));
    assert_eq!(v["thumb"]["jpg"], "thumb-0123456789abcdef.jpg");
    let m: Meta = serde_json::from_value(v).unwrap();
    assert_eq!(m.edited_sec(), None, "nothing is left out of the video it is now");
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4", "thumb-0123456789abcdef.jpg"]);

    // Again: nothing to write, nothing more to delete.
    let before = etag(&s3, A);
    assert!(library::bake(&s3, A, "2026-10-21T00:00:00Z").unwrap());
    assert_eq!(etag(&s3, A), before);
    assert_eq!(files(&fake, A).len(), 4);
}

#[test]
fn only_a_current_copy_is_baked() {
    let (fake, s3) = space();
    // No edit at all.
    record(&s3, A);
    let before = etag(&s3, A);
    assert!(!library::bake(&s3, A, DUE).unwrap());
    // An edit whose copy isn't made yet.
    library::save_edits(&s3, A, &edited(60.0), 1).unwrap();
    let saved = etag(&s3, A);
    assert_ne!(saved, before);
    assert!(!library::bake(&s3, A, DUE).unwrap());
    // A copy made, then a newer save: the copy is stale.
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    let mut later = edited(60.0);
    later.out = 45.0;
    library::save_edits(&s3, A, &later, 3).unwrap();
    let stale = etag(&s3, A);
    assert!(!library::bake(&s3, A, DUE).unwrap());
    assert_eq!(etag(&s3, A), stale, "nothing written");
    assert!(read(&s3, A)["media"]["hls"].is_string());
    for f in ORIGINAL {
        assert!(files(&fake, A).contains(&f.to_string()), "{f} was deleted");
    }
    // Not a time: nothing at all.
    assert!(library::bake(&s3, A, "soon").is_err());
}

#[test]
fn a_copy_the_space_doesnt_hold_is_never_baked() {
    let (fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    s3.delete(&format!("rec/{A}/edit-1.mp4")).unwrap();
    let before = etag(&s3, A);
    assert!(library::bake(&s3, A, DUE).is_err());
    assert_eq!(etag(&s3, A), before, "nothing written");
    assert!(ORIGINAL.iter().all(|f| files(&fake, A).contains(&f.to_string())), "nothing deleted");
}

/// The record first, then the deletes: a bake cut off after its write (every delete
/// refused) leaves a video that plays its copy and the original's bytes beside it; the next
/// bake writes nothing and finishes the deletes.
#[test]
fn a_bake_interrupted_after_its_write_finishes_when_repeated() {
    let (fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    fake.with(|s| s.control(&format!("fault 403 DELETE /s3/{BUCKET}/rec/ always")).unwrap());
    assert!(library::bake(&s3, A, DUE).is_err());
    let v = read(&s3, A);
    assert_eq!((v["media"]["mp4"].as_str(), v["media"]["hls"].clone()), (Some(&*format!("rec/{A}/edit-1.mp4")), serde_json::Value::Null));
    assert!(ORIGINAL.iter().all(|f| files(&fake, A).contains(&f.to_string())));
    let written = etag(&s3, A);

    fake.with(|s| s.control("faults-clear").unwrap());
    assert!(library::bake(&s3, A, "2026-10-20T10:00:00Z").unwrap());
    assert_eq!(etag(&s3, A), written, "the record was already baked");
    assert_eq!(read(&s3, A)["bakedFrom"].as_array().unwrap().len(), 1);
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4", "thumb-0123456789abcdef.jpg"]);
}

/// A baked video is an ordinary one-file recording: edited again, its revisions carry on
/// past the one it plays (so no copy is ever cut over the file it plays), a copy record
/// naming the file it plays never gets that file deleted, the second bake moves it to the
/// new copy and deletes the first, and every bake is remembered — the newest twenty.
#[test]
fn a_baked_video_edited_again_bakes_again() {
    let (fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    assert!(library::bake(&s3, A, DUE).unwrap());
    let plays_edit_1 = |fake: &Fake| ["edit-1.mp4", "edit-1-poster.jpg", "edit-1-teaser.mp4"].iter().all(|f| files(fake, A).contains(&f.to_string()));
    // A stray copy record naming the files the video plays (as an older writer could leave).
    let stray = |s3: &Client| {
        let mut v = read(s3, A);
        let mut edits = v.get("edits").cloned().unwrap_or_else(|| serde_json::json!({ "version": 1, "source_duration": 38.0, "in": 0.0, "out": 30.0, "cuts": [], "rev": 1 }));
        edits["copy"] = serde_json::json!({ "rev": 1, "file": "edit-1.mp4", "duration": 38.0, "poster": "edit-1-poster.jpg", "teaser": "edit-1-teaser.mp4" });
        v["edits"] = edits;
        s3.put(&format!("meta/{A}.json"), v.to_string().as_bytes(), "application/json").unwrap();
    };

    // Revert to original on the baked video, from an editor that opened on it (no edit:
    // it asks for revision 1): the stray copy is dropped from the record, its files stay.
    stray(&s3);
    assert_eq!(library::save_edits(&s3, A, &EditList::new(38.0), 1).unwrap(), 2, "past the revision baked");
    assert!(read(&s3, A)["edits"].get("copy").is_none());
    assert!(plays_edit_1(&fake));

    // Edited again on its own clock: 2–30 s plays. The stray copy is still named, stale.
    stray(&s3);
    let mut again = History::new(EditList::new(38.0));
    assert!(again.act(&Op::Trim { in_: 2.0, out: 30.0 }));
    let rev = library::save_edits(&s3, A, &again.now, 1).unwrap();
    assert_eq!(rev, 3, "never a revision the video has played from");
    let copy = EditCopy { rev, file: EditCopy::file_for(rev), duration: 28.0, poster: Some(EditCopy::poster_for(rev)), teaser: None };
    for f in copy.files() {
        s3.put(&format!("rec/{A}/{f}"), f.as_bytes(), "application/octet-stream").unwrap();
    }
    assert!(library::set_edit_copy(&s3, A, &copy, "2026-10-22T08:00:00Z").unwrap());
    assert!(plays_edit_1(&fake), "naming a new copy never takes the files the video plays");

    // Nineteen older bakes remembered before this one.
    let older: Vec<serde_json::Value> = (0..19).map(|i| serde_json::json!({ "at": format!("2026-09-{:02}T00:00:00Z", i + 1), "rev": 0, "keep": [[0.0, 1.0]] })).collect();
    let mut v = read(&s3, A);
    let mut from = older.clone();
    from.extend(v["bakedFrom"].as_array().unwrap().iter().cloned());
    v["bakedFrom"] = serde_json::Value::Array(from);
    s3.put(&format!("meta/{A}.json"), v.to_string().as_bytes(), "application/json").unwrap();

    assert!(library::bake(&s3, A, "2026-10-24T08:00:00Z").unwrap());
    let v = read(&s3, A);
    assert_eq!(
        v["media"],
        serde_json::json!({ "hls": null, "mp4": format!("rec/{A}/edit-3.mp4"), "poster": format!("rec/{A}/edit-3-poster.jpg"),
            "width": 1920, "height": 1080 }),
        "a copy with no teaser: none named"
    );
    // 3 s → 1 s; 33 s and 37.99 s are past the new trim.
    assert_eq!((v["durationSec"].as_f64(), v["markers"].clone()), (Some(28.0), serde_json::json!([1.0])));
    let from = v["bakedFrom"].as_array().unwrap();
    assert_eq!(from.len(), library::BAKED_FROM_MAX);
    assert_eq!(from[0], older[1], "the oldest dropped off");
    assert_eq!((from[18]["rev"].as_u64(), from[18]["at"].as_str()), (Some(1), Some(DUE)));
    assert_eq!(from[19], serde_json::json!({ "at": "2026-10-24T08:00:00Z", "rev": 3, "keep": [[2.0, 30.0]] }));
    assert_eq!(files(&fake, A), ["edit-3-poster.jpg", "edit-3.mp4", "thumb-0123456789abcdef.jpg"], "the first bake's files went");

    // A third edit after that: the revisions still carry on.
    assert_eq!(library::save_edits(&s3, A, &edited(28.0), 1).unwrap(), 4);
}

/// Another Mac's late copy of the revision the video now plays — the same names, cut
/// twice — never deletes the file it plays.
#[test]
fn a_late_copy_of_the_baked_revision_never_takes_its_files() {
    let (fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    assert!(library::bake(&s3, A, DUE).unwrap());
    let late = EditCopy { rev: 1, file: EditCopy::file_for(1), duration: 38.0, poster: Some(EditCopy::poster_for(1)), teaser: Some(EditCopy::teaser_for(1)) };
    assert!(!library::set_edit_copy(&s3, A, &late, DUE).unwrap());
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4", "thumb-0123456789abcdef.jpg"]);
}

#[test]
fn a_tombstone_stops_a_bake() {
    let (_fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    library::delete(&s3, A).unwrap();
    assert!(matches!(library::bake(&s3, A, DUE), Err(UploadError::Tombstoned)));
    assert_eq!(read(&s3, A)["state"], "deleted");
}

/// The Library's rows follow the record: the edited length once an edit is saved, the
/// copy's poster once baked (and then the length is the video's own).
#[test]
fn a_librarys_row_follows_the_edit_and_the_bake() {
    let (_fake, s3) = space();
    record(&s3, A);
    let mut cache = library::Cache::default();
    library::refresh(&s3, &mut cache).unwrap();
    let row = |c: &library::Cache| c.rows[A].1.clone();
    assert_eq!((row(&cache).edited_sec, row(&cache).poster_file.as_deref()), (None, Some("poster.jpg")));
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    library::refresh(&s3, &mut cache).unwrap();
    assert_eq!((row(&cache).duration_sec, row(&cache).edited_sec), (Some(60.0), Some(38.0)));
    assert!(library::bake(&s3, A, DUE).unwrap());
    library::refresh(&s3, &mut cache).unwrap();
    let r = row(&cache);
    assert_eq!((r.duration_sec, r.edited_sec, r.poster_file.as_deref()), (Some(38.0), None, Some("edit-1-poster.jpg")));
    assert_eq!(r.poster, Some(format!("rec/{A}/edit-1-poster.jpg")));
}
