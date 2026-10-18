//! The bake against the L1 fake's S3 (B197 (c)): once a saved edit's copy is current, the
//! record points at the copy's own files and forgets the edit, then the original's files
//! go — and only they, and an earlier bake's. Repeatable after any interruption; an edit of
//! a baked video bakes again; nothing a record's media names is ever deleted.

use shroom_cloudfake::{Fake, State};
use shroom_edit::{EditCopy, EditList, History, Op};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::library::{self, Baked};
use shroom_upload::meta::{Media, Meta, State as MetaState};
use shroom_upload::upload::{self, UploadError};

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

/// The picture Copy thumbnail stored for the video's title, drawn from the original's poster.
const PICTURE: &str = "thumb-0123456789abcdef.jpg";

/// A live minute-long recording, uploaded and renamed: its stream and extras, the markers
/// dropped while recording, the picture Copy thumbnail drew for its title stored beside it.
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
    m.extra.insert("thumb".into(), serde_json::json!({ "title": "Setting up shroom", "jpg": PICTURE }));
    s3.put(&format!("meta/{id}.json"), &serde_json::to_vec(&m).unwrap(), "application/json").unwrap();
    for f in ORIGINAL.iter().chain(&[PICTURE]) {
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
    assert_eq!(library::bake(&s3, A, DUE).unwrap(), Some(Baked { rev: 1, written: true, deleted: ORIGINAL.len() + 1 }));

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
    // The rest of the record is as it was — but the picture, drawn from the start the edit
    // cut (the next two tests).
    assert_eq!((v["title"].as_str(), v["titled"].as_bool(), v["state"].as_str()), (Some("Setting up shroom"), Some(true), Some("live")));
    assert!(v.get("thumb").is_none(), "{v}");
    let m: Meta = serde_json::from_value(v).unwrap();
    assert_eq!(m.edited_sec(), None, "nothing is left out of the video it is now");
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);

    // Again: nothing to write, nothing more to delete.
    let before = etag(&s3, A);
    assert_eq!(library::bake(&s3, A, "2026-10-21T00:00:00Z").unwrap(), Some(Baked { rev: 1, written: false, deleted: 0 }));
    assert_eq!(etag(&s3, A), before);
    assert_eq!(files(&fake, A).len(), 3);
}

/// Another title's picture Copy thumbnail stored earlier: B164 keeps it, so a paste sent then
/// keeps its picture.
const EARLIER: &str = "thumb-fedcba9876543210.gif";

/// The record's picture, as the Worker stamps one drawn from the copy's own poster.
fn drawn_from_the_copy(s3: &Client) {
    let mut v = read(s3, A);
    v["thumb"]["from"] = "edit-1-poster.jpg".into();
    s3.put(&format!("meta/{A}.json"), v.to_string().as_bytes(), "application/json").unwrap();
}

/// Copy thumbnail's pictures show a video's first three seconds (B164). An edit that cut into
/// them takes every picture with the original — but the record's own when it was drawn from
/// the copy's poster: that one stays named, its file with it. One drawn from the original is
/// forgotten, and the page draws a fresh one.
#[test]
fn a_bake_that_cut_into_the_start_takes_the_pictures_drawn_from_it() {
    for from_copy in [false, true] {
        let (fake, s3) = space();
        record(&s3, A);
        s3.put(&format!("rec/{A}/{EARLIER}"), b"an earlier title's", "image/gif").unwrap();
        // Trimmed to start at 2 s.
        save_and_copy(&s3, A, &edited(60.0), 38.0);
        if from_copy {
            drawn_from_the_copy(&s3);
        }
        let baked = library::bake(&s3, A, DUE).unwrap().expect("baked");
        let v = read(&s3, A);
        if from_copy {
            assert_eq!(v["thumb"], serde_json::json!({ "title": "Setting up shroom", "jpg": PICTURE, "from": "edit-1-poster.jpg" }));
            assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4", PICTURE]);
            assert_eq!(baked.deleted, ORIGINAL.len() + 1, "the earlier picture went");
        } else {
            assert!(v.get("thumb").is_none(), "{v}");
            assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);
            assert_eq!(baked.deleted, ORIGINAL.len() + 2, "both pictures went");
        }
    }
}

/// An edit that kept the first three seconds whole keeps every picture file: they show nothing
/// it cut (B164). The record still forgets a picture drawn from the original — the page draws
/// a fresh one, with the edited length — and keeps one drawn from the copy's poster.
#[test]
fn a_bake_that_kept_the_start_keeps_every_picture() {
    for from_copy in [false, true] {
        let (fake, s3) = space();
        record(&s3, A);
        s3.put(&format!("rec/{A}/{EARLIER}"), b"an earlier title's", "image/gif").unwrap();
        // 10–20 s cut and the end trimmed at 50 s: the first ten seconds play as they were.
        let mut h = History::new(EditList::new(60.0));
        assert!(h.act(&Op::Trim { in_: 0.0, out: 50.0 }) && h.act(&Op::Cut { start: 10.0, end: 20.0 }));
        save_and_copy(&s3, A, &h.now, 40.0);
        if from_copy {
            drawn_from_the_copy(&s3);
        }
        let baked = library::bake(&s3, A, DUE).unwrap().expect("baked");
        let v = read(&s3, A);
        assert_eq!(v["bakedFrom"][0]["keep"], serde_json::json!([[0.0, 10.0], [20.0, 50.0]]));
        if from_copy {
            assert_eq!(v["thumb"]["from"], "edit-1-poster.jpg");
        } else {
            assert!(v.get("thumb").is_none(), "{v}");
        }
        assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4", PICTURE, EARLIER]);
        assert_eq!(baked.deleted, ORIGINAL.len(), "the original only");
    }
}

#[test]
fn only_a_current_copy_is_baked() {
    let (fake, s3) = space();
    // No edit at all.
    record(&s3, A);
    let before = etag(&s3, A);
    assert_eq!(library::bake(&s3, A, DUE).unwrap(), None);
    // An edit whose copy isn't made yet.
    library::save_edits(&s3, A, &edited(60.0), 1).unwrap();
    let saved = etag(&s3, A);
    assert_ne!(saved, before);
    assert_eq!(library::bake(&s3, A, DUE).unwrap(), None);
    // A copy made, then a newer save: the copy is stale.
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    let mut later = edited(60.0);
    later.out = 45.0;
    library::save_edits(&s3, A, &later, 3).unwrap();
    let stale = etag(&s3, A);
    assert_eq!(library::bake(&s3, A, DUE).unwrap(), None);
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
    assert_eq!(library::bake(&s3, A, "2026-10-20T10:00:00Z").unwrap(), Some(Baked { rev: 1, written: false, deleted: ORIGINAL.len() + 1 }));
    assert_eq!(etag(&s3, A), written, "the record was already baked");
    assert_eq!(read(&s3, A)["bakedFrom"].as_array().unwrap().len(), 1);
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);
}

/// The same cut-off bake left to the token Mac's cleanup sweep (B197): the record holds no edit
/// any more, so no plan bakes it again — the sweep finds it by the original's files still
/// beside it and finishes the deletes. A video never edited, a bake that finished, and a baked
/// video edited again since (it isn't baked now, whatever lies beside it) are left alone.
#[test]
fn the_cleanup_sweep_finishes_a_bake_cut_off_after_its_write() {
    const B: &str = "BBBBBBBBBBBBBBBB";
    const C: &str = "CCCCCCCCCCCCCCCC";
    const D: &str = "DDDDDDDDDDDDDDDD";
    let (fake, s3) = space();
    record(&s3, B);
    record(&s3, C);
    save_and_copy(&s3, C, &edited(60.0), 38.0);
    assert!(library::bake(&s3, C, DUE).unwrap().is_some());
    for id in [A, D] {
        record(&s3, id);
        save_and_copy(&s3, id, &edited(60.0), 38.0);
    }
    fake.with(|s| s.control(&format!("fault 403 DELETE /s3/{BUCKET}/rec/ always")).unwrap());
    for id in [A, D] {
        assert!(library::bake(&s3, id, DUE).is_err());
        assert!(read(&s3, id).get("edits").is_none() && ORIGINAL.iter().all(|f| files(&fake, id).contains(&f.to_string())));
    }
    fake.with(|s| s.control("faults-clear").unwrap());
    // D is edited again on its baked clock: it holds an edit, its copy not made yet.
    let mut again = History::new(EditList::new(38.0));
    assert!(again.act(&Op::Trim { in_: 0.0, out: 30.0 }));
    assert_eq!(library::save_edits(&s3, D, &again.now, 1).unwrap(), 2);
    let (b, c, d) = (files(&fake, B), files(&fake, C), files(&fake, D));

    let holds = |_: &str| false;
    let here = library::Here { now_ms: library::iso_ms("2026-10-25T12:00:00Z").unwrap(), device: None, holds: &holds };
    let cleaned = library::cleanup(&s3, &here).unwrap();
    assert_eq!(cleaned.baked, vec![(A.to_string(), Baked { rev: 1, written: false, deleted: ORIGINAL.len() + 1 })]);
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);
    assert_eq!((files(&fake, B), files(&fake, C), files(&fake, D)), (b, c, d), "the others are as they were");
    // The next pass finds nothing more to do.
    assert_eq!(library::cleanup(&s3, &here).unwrap(), library::Cleaned::default());
}

/// The sweep never bakes the video this Mac's editor has open, nor finishes a bake of it
/// (B197): asked just before each — here B opens once A has been baked, mid-pass — and a later
/// pass, the editor closed, bakes what it spared.
#[test]
fn the_cleanup_sweep_never_bakes_the_video_the_editor_has_open() {
    const B: &str = "BBBBBBBBBBBBBBBB";
    const C: &str = "CCCCCCCCCCCCCCCC";
    let (fake, s3) = space();
    for id in [A, B, C] {
        record(&s3, id);
        save_and_copy(&s3, id, &edited(60.0), 38.0);
    }
    // C's bake was cut off after its write.
    fake.with(|s| s.control(&format!("fault 403 DELETE /s3/{BUCKET}/rec/{C}/ always")).unwrap());
    assert!(library::bake(&s3, C, DUE).is_err());
    fake.with(|s| s.control("faults-clear").unwrap());
    let holds = |_: &str| false;
    let here = library::Here { now_ms: library::iso_ms(DUE).unwrap(), device: None, holds: &holds };
    let open = |id: &str| id == C || (id == B && read(&s3, A).get("edits").is_none());
    let ids = |c: &library::Cleaned| c.baked.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>();
    let cleaned = library::cleanup_sparing(&s3, &here, &open).unwrap();
    assert_eq!(ids(&cleaned), [A]);
    assert!(read(&s3, B)["edits"].is_object() && ORIGINAL.iter().all(|f| files(&fake, B).contains(&f.to_string())), "B, opened mid-pass, was spared");
    assert!(ORIGINAL.iter().all(|f| files(&fake, C).contains(&f.to_string())), "C's deletes wait");
    // The editor closed: the next pass bakes B and finishes C.
    let cleaned = library::cleanup(&s3, &here).unwrap();
    assert_eq!(ids(&cleaned), [B, C]);
    assert_eq!((files(&fake, B), files(&fake, C)), (files(&fake, A), files(&fake, A)));
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
    assert!(library::bake(&s3, A, DUE).unwrap().is_some());
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

    assert_eq!(library::bake(&s3, A, "2026-10-24T08:00:00Z").unwrap(), Some(Baked { rev: 3, written: true, deleted: 3 }), "the first bake's three files");
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
    assert_eq!(files(&fake, A), ["edit-3-poster.jpg", "edit-3.mp4"], "the first bake's files went, its picture with the original");

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
    assert!(library::bake(&s3, A, DUE).unwrap().is_some());
    let late = EditCopy { rev: 1, file: EditCopy::file_for(1), duration: 38.0, poster: Some(EditCopy::poster_for(1)), teaser: Some(EditCopy::teaser_for(1)) };
    assert!(!library::set_edit_copy(&s3, A, &late, DUE).unwrap());
    assert_eq!(files(&fake, A), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);
}

/// An editor that opened before the bake holds a list on the original's clock: its save writes
/// nothing over the baked video, deletes nothing, and says why (B197) — the check made on the
/// record the write would replace. An editor that opened on the baked video saves as any
/// other, a rename meanwhile moving nothing; a writer with no editor saves unguarded.
#[test]
fn a_save_from_an_editor_opened_before_the_bake_never_lands_on_the_baked_video() {
    let (fake, s3) = space();
    record(&s3, A);
    save_and_copy(&s3, A, &edited(60.0), 38.0);
    let meta = |s3: &Client| serde_json::from_value::<Meta>(read(s3, A)).unwrap();
    // The editor opens on revision 1, then a cut is added at 30–35 s of the original.
    let opened = library::Plays::of(&meta(&s3));
    let mut h = History::new(edited(60.0));
    assert!(h.act(&Op::Cut { start: 30.0, end: 35.0 }));
    // Meanwhile the sweep makes the edit the video.
    assert!(library::bake(&s3, A, DUE).unwrap().is_some());
    let (baked, left) = (etag(&s3, A), files(&fake, A));
    assert!(matches!(library::save_edits_over(&s3, A, &h.now, 2, Some(&opened)), Err(UploadError::MovedOn)));
    assert_eq!(etag(&s3, A), baked, "nothing written");
    assert_eq!(files(&fake, A), left, "nothing deleted");
    assert!(read(&s3, A).get("edits").is_none());

    // Opened on the baked video, renamed meanwhile: it saves, past the revision baked.
    let now = library::Plays::of(&meta(&s3));
    library::rename(&s3, A, "Setting up shroom, short").unwrap();
    let mut short = History::new(EditList::new(38.0));
    assert!(short.act(&Op::Trim { in_: 0.0, out: 30.0 }));
    assert_eq!(library::save_edits_over(&s3, A, &short.now, 1, Some(&now)).unwrap(), 2);
    assert_eq!(read(&s3, A)["edits"]["out"], 30.0);
    // A writer that opened no editor: unguarded, as before.
    assert_eq!(library::save_edits(&s3, A, &short.now, 1).unwrap(), 3);
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
    assert!(library::bake(&s3, A, DUE).unwrap().is_some());
    library::refresh(&s3, &mut cache).unwrap();
    let r = row(&cache);
    assert_eq!((r.duration_sec, r.edited_sec, r.poster_file.as_deref()), (Some(38.0), None, Some("edit-1-poster.jpg")));
    assert_eq!(r.poster, Some(format!("rec/{A}/edit-1-poster.jpg")));
}

/// The uploader never moves a baked video back (B197): a take whose teaser and single-file
/// copy were still being made when its edit became the video keeps playing the edit, and
/// what it sends then goes again; so does a pass run again from the start.
#[test]
fn the_uploader_never_moves_a_baked_video_back() {
    let (fake, s3) = space();
    let take = tempfile::tempdir().unwrap();
    let dir = take.path();
    let mut playlist = String::from("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:6\n#EXT-X-MAP:URI=\"init.mp4\"\n");
    for i in 0..2 {
        playlist.push_str(&format!("#EXTINF:6.000,\nseg_{i:05}.m4s\n"));
        std::fs::write(dir.join(format!("seg_{i:05}.m4s")), vec![i as u8; 64]).unwrap();
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    std::fs::write(dir.join("stream.m3u8"), playlist).unwrap();
    std::fs::write(dir.join("init.mp4"), b"not a real init").unwrap();
    upload::upload_recording(&s3, A, dir, "Setting up shroom", |_, _| {}).unwrap();
    assert_eq!((read(&s3, A)["media"]["hls"].as_str(), read(&s3, A)["durationSec"].as_f64()), (Some(&*format!("rec/{A}/master.m3u8")), Some(12.0)));

    let mut h = History::new(EditList::new(12.0));
    assert!(h.act(&Op::Trim { in_: 2.0, out: 10.0 }));
    save_and_copy(&s3, A, &h.now, 8.0);
    assert!(library::bake(&s3, A, DUE).unwrap().is_some());
    let baked = |s3: &Client| {
        let v = read(s3, A);
        assert_eq!(v["media"]["hls"], serde_json::Value::Null, "{v}");
        assert_eq!((v["media"]["mp4"].as_str(), v["media"]["teaser"].as_str()), (Some(&*format!("rec/{A}/edit-1.mp4")), Some(&*format!("rec/{A}/edit-1-teaser.mp4"))));
        assert_eq!(v["durationSec"], 8.0);
    };
    baked(&s3);

    // The extras were still being made: now they go up.
    std::fs::write(dir.join("teaser.mp4"), b"the teaser").unwrap();
    std::fs::write(dir.join("preview.mp4"), b"the single-file copy").unwrap();
    assert!(upload::finish(&s3, A, dir, 5, 7, &upload::Beat::default(), |_, _| {}).unwrap());
    baked(&s3);
    assert_eq!(read(&s3, A)["progress"]["done"], 7, "drained all the same");
    assert!(files(&fake, A).iter().all(|f| f != "video.mp4" && f != "teaser.mp4"), "{:?}", files(&fake, A));

    // The whole pass again (its live marker lost): the record stays the edit.
    upload::upload_recording(&s3, A, dir, "Setting up shroom", |_, _| {}).unwrap();
    baked(&s3);
    assert!(files(&fake, A).iter().all(|f| f != "video.mp4" && f != "teaser.mp4"));
    assert!(["edit-1.mp4", "edit-1-poster.jpg", "edit-1-teaser.mp4"].iter().all(|f| files(&fake, A).contains(&f.to_string())));
}
