//! The Library's data path against the L1 fake's S3: a refresh reads only what changed,
//! rename is a record rewrite the page shows, delete leaves a tombstone first and then
//! no bytes, discard leaves nothing at all.

use shroom_cloudfake::{Fake, State};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::library::{self, Cache};
use shroom_upload::meta::{Meta, State as MetaState};

const A: &str = "AAAAAAAAAAAAAAAA";
const B: &str = "BBBBBBBBBBBBBBBB";

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

fn record(s3: &Client, id: &str, title: &str, created: &str) {
    let mut m = Meta::uploading(id, title, created);
    m.state = MetaState::Live;
    s3.put(&format!("meta/{id}.json"), &serde_json::to_vec(&m).unwrap(), "application/json").unwrap();
    s3.put(&format!("rec/{id}/master.m3u8"), b"#EXTM3U", "application/vnd.apple.mpegurl").unwrap();
    s3.put(&format!("rec/{id}/v0/seg_00000.m4s"), b"seg", "video/iso.segment").unwrap();
}

#[test]
fn a_refresh_reads_only_what_changed() {
    let (_fake, s3) = space();
    record(&s3, A, "First", "2026-10-01T10:00:00Z");
    record(&s3, B, "Second", "2026-10-02T10:00:00Z");
    let mut cache = Cache::default();
    let d = library::refresh(&s3, &mut cache).unwrap();
    assert_eq!((d.added.len(), d.reads), (2, 2));
    assert_eq!(cache.visible().iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), ["Second", "First"], "newest first");

    let again = library::refresh(&s3, &mut cache).unwrap();
    assert_eq!(again.reads, 0, "nothing changed, nothing read");

    library::rename(&s3, A, "  The   plan ").unwrap();
    let d = library::refresh(&s3, &mut cache).unwrap();
    assert_eq!((d.changed, d.reads), (vec![A.to_string()], 1));
    assert_eq!(cache.rows[A].1.title, "The plan");

    s3.delete(&format!("meta/{B}.json")).unwrap();
    assert_eq!(library::refresh(&s3, &mut cache).unwrap().gone, vec![B.to_string()]);
}

#[test]
fn rename_marks_a_chosen_title_and_never_saves_an_empty_one() {
    let (_fake, s3) = space();
    record(&s3, A, "Untitled — Oct 2", "2026-10-02T10:00:00Z");
    assert!(library::rename(&s3, A, "   ").is_err());
    library::rename(&s3, A, "Demo").unwrap();
    let v: serde_json::Value = serde_json::from_slice(&s3.get(&format!("meta/{A}.json")).unwrap().unwrap()).unwrap();
    assert_eq!((v["title"].as_str(), v["titled"].as_bool(), v["state"].as_str()), (Some("Demo"), Some(true), Some("live")));
}

#[test]
fn delete_leaves_a_tombstone_first_and_then_no_bytes() {
    let (fake, s3) = space();
    record(&s3, A, "Secret plans", "2026-10-02T10:00:00Z");
    library::delete(&s3, A).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&s3.get(&format!("meta/{A}.json")).unwrap().unwrap()).unwrap();
    assert_eq!(v["state"], "deleted");
    assert!(v.get("title").is_none(), "the tombstone keeps no title");
    assert!(library::tombstoned(&s3, A).unwrap());
    assert!(fake.with(|s| s.buckets["shroom-space"].keys().all(|k| !k.starts_with("rec/"))));
    // A rename after the delete is refused; deleting again is harmless.
    assert!(library::rename(&s3, A, "Back").is_err());
    library::delete(&s3, A).unwrap();
    let mut cache = Cache::default();
    library::refresh(&s3, &mut cache).unwrap();
    assert!(cache.visible().is_empty(), "tombstones aren't rows");
}

#[test]
fn a_row_already_cached_goes_once_its_tombstone_lands() {
    let (_fake, s3) = space();
    record(&s3, A, "Secret plans", "2026-10-02T10:00:00Z");
    let mut cache = Cache::default();
    library::refresh(&s3, &mut cache).unwrap();
    assert_eq!(cache.visible().len(), 1);
    // Another Mac deletes it: the record is now a tombstone, which keeps no title.
    library::delete(&s3, A).unwrap();
    let d = library::refresh(&s3, &mut cache).unwrap();
    assert_eq!(d.changed, vec![A.to_string()]);
    assert!(cache.visible().is_empty(), "a tombstone replaces the cached row");
}

#[test]
fn a_sealed_video_stays_deleted_whatever_its_record_says_now() {
    let (_fake, s3) = space();
    record(&s3, A, "Secret plans", "2026-10-02T10:00:00Z");
    record(&s3, B, "Second", "2026-10-03T10:00:00Z");
    let mut cache = Cache::default();
    library::refresh(&s3, &mut cache).unwrap();
    library::delete(&s3, A).unwrap();
    s3.put(&format!(".shroom/gone/{A}"), br#"{"v":1,"by":"token","at":"2026-10-04T10:00:00Z"}"#, "application/json").unwrap();
    // A stale upload credential writes the record back to live after the seal (DECISIONS B125).
    record(&s3, A, "Secret plans", "2026-10-02T10:00:00Z");
    library::refresh(&s3, &mut cache).unwrap();
    assert_eq!(cache.visible().iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), [B]);
    assert_eq!(cache.rows[A].1.state, MetaState::Deleted);
    assert_eq!(library::refresh(&s3, &mut cache).unwrap().reads, 0, "a sealed id costs no read once known");
}

#[test]
fn discard_of_a_never_shared_take_leaves_nothing() {
    let (fake, s3) = space();
    record(&s3, A, "Oops", "2026-10-02T10:00:00Z");
    library::discard(&s3, A).unwrap();
    assert!(fake.with(|s| s.buckets["shroom-space"].is_empty()));
    assert!(!library::tombstoned(&s3, A).unwrap());
}

#[test]
fn the_sweep_reaps_bytes_under_tombstones_and_known_orphans_only() {
    let (fake, s3) = space();
    record(&s3, A, "Kept", "2026-10-02T10:00:00Z");
    record(&s3, B, "Deleted", "2026-10-02T10:00:00Z");
    library::delete(&s3, B).unwrap();
    // Another Mac's late segment lands after the delete.
    s3.put(&format!("rec/{B}/v0/seg_00009.m4s"), b"late", "video/iso.segment").unwrap();
    // Bytes with no record: one abandoned, one mid-claim.
    s3.put("rec/CCCCCCCCCCCCCCCC/v0/init.mp4", b"x", "video/mp4").unwrap();
    s3.put("rec/DDDDDDDDDDDDDDDD/v0/init.mp4", b"x", "video/mp4").unwrap();
    let swept = library::sweep(&s3, &|id: &str| id == "CCCCCCCCCCCCCCCC").unwrap();
    assert_eq!(swept.under_tombstones, vec![B.to_string()]);
    assert_eq!(swept.orphans, vec!["CCCCCCCCCCCCCCCC".to_string()]);
    let left: Vec<String> = fake.with(|s| s.buckets["shroom-space"].keys().filter(|k| k.starts_with("rec/")).cloned().collect());
    assert!(left.iter().all(|k| k.starts_with(&format!("rec/{A}/")) || k.starts_with("rec/DDDDDDDDDDDDDDDD/")), "{left:?}");
}

/// DECISIONS B91: the token Mac's cleanup sweep against the L1 bucket — bytes under a
/// tombstone go; bytes with no record go only when no Mac can still finish them (this
/// Mac's own, no longer held; a Mac out of the space past a credential's life); records,
/// the ledger and the roster are never touched; spent and expired page keys go, live ones
/// stay.
#[test]
fn the_cleanup_sweep_reaps_only_what_no_mac_can_still_finish() {
    const NOW: i64 = 1_790_942_400_000; // 2026-10-02T12:00:00Z
    const DAY: i64 = 86_400_000;
    let (fake, s3) = space();
    // Unix milliseconds → an ISO date (Howard Hinnant's civil-from-days).
    let iso = |ms: i64| {
        let secs = ms.div_euclid(1000);
        let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
    };
    assert_eq!(library::iso_ms(&iso(NOW - 25 * DAY)), Some(NOW - 25 * DAY));
    let put = |k: &str, v: serde_json::Value| s3.put(k, v.to_string().as_bytes(), "application/json").unwrap();
    put(library::ROSTER, serde_json::json!({ "v": 1, "members": {}, "devices": {
        "d_this": { "member": "m1", "state": "active", "endedAt": null },
        "d_ani": { "member": "m_ani", "state": "active", "endedAt": null },
        "d_gone": { "member": "m_old", "state": "removed", "endedAt": iso(NOW - 20 * DAY) },
        "d_lately": { "member": "m_new", "state": "removed", "endedAt": iso(NOW - 3 * DAY) },
    }}));
    let orphan = |id: &str, device: &str, age_days: i64| {
        s3.put(&format!("rec/{id}/v0/init.mp4"), b"x", "video/mp4").unwrap();
        put(&format!("{}{id}", library::LEDGER), serde_json::json!({ "v": 1, "member": "m", "device": device, "at": iso(NOW - age_days * DAY) }));
    };
    record(&s3, A, "Kept", "2026-10-02T10:00:00Z");
    record(&s3, B, "Deleted", "2026-10-02T10:00:00Z");
    library::delete(&s3, B).unwrap();
    s3.put(&format!("rec/{B}/v0/seg_00009.m4s"), b"late", "video/iso.segment").unwrap();
    orphan("OwnDiscardedOld0", "d_this", 3); // this Mac's, discarded: goes
    orphan("OwnStillHeld0000", "d_this", 3); // this Mac's, a saved take still here: stays
    orphan("OwnFreshToday000", "d_this", 0); // under a day old: stays
    orphan("AnisSavedTake000", "d_ani", 30); // an active member's saved take: stays
    orphan("GoneMacsOldTake0", "d_gone", 25); // removed long ago: goes
    orphan("LatelyRemoved000", "d_lately", 25); // removed lately, its credential may live: stays
    s3.put("rec/NoLedgerAtAll000/v0/init.mp4", b"x", "video/mp4").unwrap(); // can't be placed: stays
    put(&format!("{}spentkey", library::KEYS), serde_json::json!({ "member": "m1", "device": "d_this", "exp": iso(NOW + DAY), "spent": iso(NOW) }));
    put(&format!("{}expiredkey", library::KEYS), serde_json::json!({ "member": "m1", "device": "d_this", "exp": iso(NOW - DAY) }));
    put(&format!("{}livekey", library::KEYS), serde_json::json!({ "member": "m1", "device": "d_this", "exp": iso(NOW + DAY) }));

    let holds = |id: &str| id == "OwnStillHeld0000";
    let cleaned = library::cleanup(&s3, &library::Here { now_ms: NOW, device: Some("d_this"), holds: &holds }).unwrap();
    assert_eq!(cleaned.swept.under_tombstones, vec![B.to_string()]);
    assert_eq!(cleaned.swept.orphans, vec!["GoneMacsOldTake0".to_string(), "OwnDiscardedOld0".to_string()]);
    assert_eq!(cleaned.keys, 2);

    let keys: Vec<String> = fake.with(|s| s.buckets["shroom-space"].keys().cloned().collect());
    let rec_left: Vec<&str> = keys.iter().filter_map(|k| k.strip_prefix("rec/")).filter_map(|k| k.split('/').next()).collect();
    for kept in [A, "OwnStillHeld0000", "OwnFreshToday000", "AnisSavedTake000", "LatelyRemoved000", "NoLedgerAtAll000"] {
        assert!(rec_left.contains(&kept), "{kept} lost its bytes: {keys:?}");
    }
    for gone in [B, "GoneMacsOldTake0", "OwnDiscardedOld0"] {
        assert!(!rec_left.contains(&gone), "{gone} kept its bytes");
    }
    // Records, the tombstone, the ledger and the roster stay as they were.
    for k in [format!("meta/{A}.json"), format!("meta/{B}.json"), library::ROSTER.to_string(), format!("{}GoneMacsOldTake0", library::LEDGER)] {
        assert!(keys.contains(&k), "{k} was touched");
    }
    assert!(keys.contains(&format!("{}livekey", library::KEYS)) && !keys.iter().any(|k| k.ends_with("spentkey") || k.ends_with("expiredkey")));

    // A second pass finds nothing more to do.
    let again = library::cleanup(&s3, &library::Here { now_ms: NOW, device: Some("d_this"), holds: &holds }).unwrap();
    assert_eq!(again, library::Cleaned::default());

    // No roster to read: no orphan goes (fail closed), tombstones still do.
    s3.delete(library::ROSTER).unwrap();
    s3.put(&format!("rec/{B}/v0/seg_00010.m4s"), b"later", "video/iso.segment").unwrap();
    orphan("GoneMacsOtherOne", "d_gone", 25);
    let blind = library::cleanup(&s3, &library::Here { now_ms: NOW, device: Some("d_this"), holds: &holds }).unwrap();
    assert_eq!((blind.swept.under_tombstones, blind.swept.orphans), (vec![B.to_string()], vec![]));
}

#[test]
fn a_delete_that_cant_write_its_tombstone_removes_nothing() {
    let (fake, s3) = space();
    record(&s3, A, "Busy", "2026-10-02T10:00:00Z");
    // Another writer rewrites the record before every one of our conditional writes. It
    // reads and writes in one hold of the bucket — what it puts back is always what is
    // there, under a new ETag. Read in one hold and written in another, it could put the
    // record it read *before* our tombstone back *after* it: an unconditional write no
    // writer of shroom's makes (DECISIONS B21), and a failure of this stand-in, not of the
    // delete — seen on a Mac with few cores, the file's tests running side by side.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (f2, s2) = (fake.state.clone(), stop.clone());
    let churn = std::thread::spawn(move || {
        while !s2.load(std::sync::atomic::Ordering::SeqCst) {
            let mut bucket = f2.lock().unwrap();
            let body = bucket.object("shroom-space", &format!("meta/{A}.json")).unwrap();
            bucket.put_object("shroom-space", &format!("meta/{A}.json"), &body);
        }
    });
    let r = library::delete(&s3, A);
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    churn.join().unwrap();
    if r.is_err() {
        assert!(fake.with(|s| s.object("shroom-space", &format!("rec/{A}/master.m3u8")).is_some()), "no bytes removed without a tombstone");
    } else {
        assert!(library::tombstoned(&s3, A).unwrap(), "a successful delete always leaves the tombstone");
    }
}

/// B197 (d): the token Mac's sweep bakes every video in the space whose original's two days
/// are up — whoever's it is — and nothing else: a video still in its two days keeps its
/// original, a baked one keeps the copy it plays, a copy that isn't there is never baked
/// (and doesn't stop the pass), and no live record's file is reaped.
#[test]
fn the_cleanup_sweep_bakes_every_video_past_its_two_days() {
    const NOW: i64 = 1_790_942_400_000; // 2026-10-02T12:00:00Z
    let (fake, s3) = space();
    let put = |k: &str, v: serde_json::Value| s3.put(k, v.to_string().as_bytes(), "application/json").unwrap();
    let original = ["master.m3u8", "v0/init.mp4", "v0/seg_00000.m4s", "v0/stream.m3u8", "video.mp4", "poster.jpg", "teaser.mp4"];
    let copy = ["edit-1.mp4", "edit-1-poster.jpg", "edit-1-teaser.mp4"];
    let edited = |id: &str, keep_until: &str, sent: &[&str]| {
        put(&format!("meta/{id}.json"), serde_json::json!({
            "schema": 1, "id": id, "state": "live", "createdAt": "2026-09-28T10:00:00Z", "publishedAt": "2026-09-28T10:01:00Z",
            "deletedAt": null, "title": id, "durationSec": 60.0, "captions": null, "owner": null, "progress": null,
            "media": { "hls": format!("rec/{id}/master.m3u8"), "mp4": format!("rec/{id}/video.mp4"), "poster": format!("rec/{id}/poster.jpg"),
                "teaser": format!("rec/{id}/teaser.mp4"), "width": 1280, "height": 720 },
            "edits": { "version": 1, "source_duration": 60.0, "in": 8.0, "out": 60.0, "cuts": [], "rev": 1, "keep_until": keep_until,
                "copy": { "rev": 1, "file": "edit-1.mp4", "duration": 52.0, "poster": "edit-1-poster.jpg", "teaser": "edit-1-teaser.mp4" } },
        }));
        for f in original.iter().chain(sent) {
            s3.put(&format!("rec/{id}/{f}"), f.as_bytes(), "application/octet-stream").unwrap();
        }
    };
    let files = |id: &str| -> Vec<String> {
        let prefix = format!("rec/{id}/");
        let mut v: Vec<String> = fake.with(|s| s.buckets["shroom-space"].keys().filter_map(|k| k.strip_prefix(&prefix).map(str::to_string)).collect());
        v.sort();
        v
    };
    edited("DueToBakeVideo00", "2026-10-02T12:00:00Z", &copy);
    edited("KeptTwoMoreDays0", "2026-10-04T08:00:00Z", &copy);
    edited("CopyGoneMissing0", "2026-10-01T00:00:00Z", &copy[1..]);
    put("meta/BakedAlready0000.json", serde_json::json!({
        "schema": 1, "id": "BakedAlready0000", "state": "live", "createdAt": "2026-09-20T10:00:00Z", "publishedAt": null, "deletedAt": null,
        "title": "Baked", "durationSec": 40.0, "captions": null, "owner": null, "progress": null,
        "media": { "hls": null, "mp4": "rec/BakedAlready0000/edit-2.mp4", "poster": "rec/BakedAlready0000/edit-2-poster.jpg", "width": 1280, "height": 720 },
        "bakedFrom": [{ "at": "2026-09-25T00:00:00Z", "rev": 2, "keep": [[0.0, 40.0]] }],
    }));
    for f in ["edit-2.mp4", "edit-2-poster.jpg", "thumb-00112233aabbccdd.jpg"] {
        s3.put(&format!("rec/BakedAlready0000/{f}"), b"x", "application/octet-stream").unwrap();
    }

    let holds = |_: &str| false;
    let here = library::Here { now_ms: NOW, device: Some("d_this"), holds: &holds };
    let cleaned = library::cleanup(&s3, &here).unwrap();
    assert_eq!(cleaned.baked, vec![("DueToBakeVideo00".to_string(), library::Baked { rev: 1, written: true, deleted: original.len() })]);
    assert_eq!((cleaned.swept.under_tombstones.len(), cleaned.swept.orphans.len()), (0, 0));

    let v: serde_json::Value = serde_json::from_slice(&s3.get("meta/DueToBakeVideo00.json").unwrap().unwrap()).unwrap();
    assert_eq!((v["media"]["mp4"].as_str(), v["durationSec"].as_f64()), (Some("rec/DueToBakeVideo00/edit-1.mp4"), Some(52.0)));
    assert_eq!(v["bakedFrom"][0]["at"], "2026-10-02T12:00:00Z");
    assert_eq!(files("DueToBakeVideo00"), ["edit-1-poster.jpg", "edit-1-teaser.mp4", "edit-1.mp4"]);
    for id in ["KeptTwoMoreDays0", "CopyGoneMissing0"] {
        let v: serde_json::Value = serde_json::from_slice(&s3.get(&format!("meta/{id}.json")).unwrap().unwrap()).unwrap();
        assert!(v["edits"].is_object() && v["media"]["hls"].is_string(), "{id} was baked");
        assert!(original.iter().all(|f| files(id).contains(&f.to_string())), "{id} lost its original");
    }
    assert!(copy.iter().all(|f| files("KeptTwoMoreDays0").contains(&f.to_string())), "a live record's copy is never reaped");
    assert_eq!(files("BakedAlready0000"), ["edit-2-poster.jpg", "edit-2.mp4", "thumb-00112233aabbccdd.jpg"], "a baked video's files are never reaped");

    // The next pass: nothing more is due.
    assert_eq!(library::cleanup(&s3, &here).unwrap(), library::Cleaned::default());
}
