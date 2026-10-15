//! L3 (TESTING-PLAN §5; DECISIONS B64; tests-sys #4): the uploader under the transport
//! fault seam (the L1 fake's `fault` rules — dropped connections, replies that never come
//! back, resets mid-body, delays, a busy edge, a stale credential) and killed mid-upload
//! (`kill -9` of a process that was sending). Law 2 at the bucket: whatever happened on the
//! way, the next pass ends with every file of the take there exactly once, byte for byte —
//! no stray key, no second record, no object that isn't the local file.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use shroom_cloudfake::{Fake, State};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::meta::{Meta, State as MetaState};
use shroom_upload::upload;

const ID: &str = "FaUlTsFaUlTsFaUl";
const BUCKET: &str = "shroom-space";
const SEGMENTS: usize = 12;

fn space() -> Fake {
    let mut st = State::fresh();
    st.buckets.insert(BUCKET.into(), Default::default());
    Fake::start(st)
}

fn client(r2: &str) -> Client {
    Client::new(Bucket {
        endpoint: r2.into(),
        region: "auto".into(),
        name: BUCKET.into(),
        credentials: Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None },
    })
}

/// A finalized take on disk: the joined playlist, the init, twelve 96 KiB segments (each
/// distinct, so a swapped or partial object can't pass for another), the single-file copy.
fn recording(dir: &Path) {
    let mut playlist = String::from("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:6\n#EXT-X-MAP:URI=\"init.mp4\"\n");
    for i in 0..SEGMENTS {
        playlist.push_str(&format!("#EXTINF:6.000,\nseg_{i:05}.m4s\n"));
        let bytes: Vec<u8> = (0..96 * 1024).map(|j| ((i * 31 + j) % 251) as u8).collect();
        std::fs::write(dir.join(format!("seg_{i:05}.m4s")), bytes).unwrap();
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    std::fs::write(dir.join("stream.m3u8"), playlist).unwrap();
    std::fs::write(dir.join("init.mp4"), b"not a real init, but this take's").unwrap();
    std::fs::write(dir.join("preview.mp4"), vec![9u8; 200 * 1024]).unwrap();
}

/// Law 2 at the bucket: exactly the take's keys, each the local file's bytes; one record,
/// live and drained.
fn whole_and_once(fake: &Fake, dir: &Path) {
    let objects: BTreeMap<String, Vec<u8>> = fake.with(|s| s.buckets[BUCKET].iter().map(|(k, (b, _))| (k.clone(), b.clone())).collect());
    let mut want: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    want.insert(format!("rec/{ID}/v0/init.mp4"), std::fs::read(dir.join("init.mp4")).unwrap());
    for i in 0..SEGMENTS {
        let f = format!("seg_{i:05}.m4s");
        want.insert(format!("rec/{ID}/v0/{f}"), std::fs::read(dir.join(&f)).unwrap());
    }
    want.insert(format!("rec/{ID}/v0/stream.m3u8"), std::fs::read(dir.join("stream.m3u8")).unwrap());
    want.insert(format!("rec/{ID}/video.mp4"), std::fs::read(dir.join("preview.mp4")).unwrap());
    for (k, bytes) in &want {
        let got = objects.get(k).unwrap_or_else(|| panic!("{k} is missing"));
        assert!(got == bytes, "{k} isn't the local file ({} bytes up, {} here)", got.len(), bytes.len());
    }
    let extra: Vec<&String> = objects.keys().filter(|k| !want.contains_key(*k) && *k != &format!("rec/{ID}/master.m3u8") && *k != &format!("meta/{ID}.json")).collect();
    assert!(extra.is_empty(), "stray objects: {extra:?}");
    assert_eq!(objects.keys().filter(|k| k.starts_with("meta/")).count(), 1, "one record");
    let meta: Meta = serde_json::from_slice(&objects[&format!("meta/{ID}.json")]).unwrap();
    assert_eq!(meta.state, MetaState::Live);
    let p = meta.progress.expect("progress");
    assert_eq!(p.done, p.total, "drained");
    assert!(objects.contains_key(&format!("rec/{ID}/master.m3u8")));
}

#[test]
fn every_transient_fault_on_the_way_up_still_lands_every_byte_once() {
    // Each kind, twice on the media and once on the record's claim — inside the uploader's
    // own retries (six tries, 0.5 s doubling), so one pass lands it all.
    for kind in ["503", "drop", "lost-reply", "reset", "delay:300"] {
        let fake = space();
        let tmp = tempfile::tempdir().unwrap();
        recording(tmp.path());
        fake.with(|s| {
            s.control(&format!("fault {kind} PUT /s3/{BUCKET}/meta/{ID}.json 1")).unwrap();
            s.control(&format!("fault {kind} PUT /s3/{BUCKET}/rec/{ID}/v0/seg_00004.m4s 1")).unwrap();
            s.control(&format!("fault {kind} PUT /s3/{BUCKET}/rec/{ID}/video.mp4 1")).unwrap();
        });
        let report = upload::upload_recording(&client(&fake.r2()), ID, tmp.path(), "Untitled — Oct 2", |_, _| {})
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
        assert!(report.mp4, "{kind}");
        assert_eq!(fake.with(|s| s.faulted()), 3, "{kind}: every fault was met");
        whole_and_once(&fake, tmp.path());
    }
}

#[test]
fn a_fault_that_outlasts_the_pass_fails_it_and_the_next_pass_sends_only_the_gap() {
    // A credential gone stale halfway (403, never retried): the pass stops with what it has.
    let fake = space();
    let tmp = tempfile::tempdir().unwrap();
    recording(tmp.path());
    fake.with(|s| s.control(&format!("fault 403 PUT /s3/{BUCKET}/rec/{ID}/v0/seg_00007.m4s always")).unwrap());
    let err = upload::upload_recording(&client(&fake.r2()), ID, tmp.path(), "Untitled — Oct 2", |_, _| {}).unwrap_err();
    assert!(err.to_string().contains("403"), "{err}");
    let up = fake.with(|s| s.buckets[BUCKET].keys().filter(|k| k.starts_with(&format!("rec/{ID}/v0/seg_"))).count());
    assert_eq!(up, 7, "seg 0–6 made it");
    let meta: Meta = serde_json::from_slice(&fake.with(|s| s.object(BUCKET, &format!("meta/{ID}.json"))).unwrap()).unwrap();
    assert_eq!(meta.state, MetaState::Uploading, "never live with a gap");
    // The next pass, with a fresh credential: only the gap goes.
    fake.with(|s| s.control("faults-clear").unwrap());
    let report = upload::upload_recording(&client(&fake.r2()), ID, tmp.path(), "Untitled — Oct 2", |_, _| {}).unwrap();
    assert_eq!(report.already_there, 8, "the init and seg 0–6 weren't sent again");
    whole_and_once(&fake, tmp.path());
}

/// The process the kill test kills: one upload pass, from the environment the parent set.
#[test]
#[ignore = "run only as the kill test's child"]
fn child_upload_pass() {
    let (Ok(r2), Ok(dir)) = (std::env::var("SHROOM_FAULTS_R2"), std::env::var("SHROOM_FAULTS_DIR")) else { return };
    let _ = upload::upload_recording(&client(&r2), ID, Path::new(&dir), "Untitled — Oct 2", |_, _| {});
}

#[test]
fn kill_9_mid_upload_then_the_next_pass_resumes_with_nothing_duplicated_or_torn() {
    let fake = space();
    let tmp = tempfile::tempdir().unwrap();
    recording(tmp.path());
    // A slow path, so the kill lands with the take half up.
    fake.with(|s| s.control(&format!("fault delay:150 PUT /s3/{BUCKET}/rec/ always")).unwrap());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_upload_pass", "--ignored", "--nocapture", "--test-threads=1"])
        .env("SHROOM_FAULTS_R2", fake.r2())
        .env("SHROOM_FAULTS_DIR", tmp.path())
        .spawn()
        .unwrap();
    let rec_keys = || fake.with(|s| s.buckets[BUCKET].keys().filter(|k| k.starts_with("rec/")).count());
    let until = Instant::now() + Duration::from_secs(30);
    while rec_keys() < 5 {
        assert!(Instant::now() < until, "the child never got going");
        assert!(child.try_wait().unwrap().is_none(), "the child finished before the kill");
        std::thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap(); // SIGKILL
    child.wait().unwrap();
    let at_kill = rec_keys();
    assert!(at_kill < SEGMENTS, "killed mid-way ({at_kill} up)");
    let meta: Meta = serde_json::from_slice(&fake.with(|s| s.object(BUCKET, &format!("meta/{ID}.json"))).unwrap()).unwrap();
    assert_eq!(meta.state, MetaState::Uploading);
    // The relaunch's pass: resumes, sends the gap, lands the take whole.
    fake.with(|s| s.control("faults-clear").unwrap());
    let report = upload::upload_recording(&client(&fake.r2()), ID, tmp.path(), "Untitled — Oct 2", |_, _| {}).unwrap();
    assert!(report.already_there as usize >= at_kill.saturating_sub(1), "resumed: {} already there of {at_kill} at the kill", report.already_there);
    whole_and_once(&fake, tmp.path());
}

/// The single-file copy past the single-PUT limit goes up in parts, as R2 takes it — the
/// fake answered S3's multipart calls with 405 until the 2-hour soak's 1.6 GB copy met it
/// (DECISIONS B70). Byte for byte, through a lost part reply and a busy edge;
/// a sink bucket counts the copy at its real size without holding it.
#[test]
fn a_copy_past_the_single_put_limit_goes_up_in_parts_byte_for_byte() {
    let fake = space();
    let s3 = client(&fake.r2());
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("video.mp4");
    let bytes: Vec<u8> = (0..1_000_003u32).map(|i| (i % 253) as u8).collect();
    std::fs::write(&file, &bytes).unwrap();
    let key = format!("rec/{ID}/video.mp4");
    // Parts of 256 KiB: four whole ones and a short last.
    s3.put_file(&key, &file, "video/mp4", 300_000, 256 * 1024).unwrap();
    assert_eq!(fake.with(|s| s.object(BUCKET, &key)).as_deref(), Some(&bytes[..]), "the parts in order, nothing torn");
    assert!(fake.with(|s| s.multipart.is_empty()), "the upload is completed, none left open");

    // A part whose reply is lost (the upload is aborted, nothing left open), and a busy
    // edge on the start: the uploader's own retry (`with_retry`) sends the whole copy again
    // and it still lands whole.
    fake.with(|s| s.control(&format!("fault lost-reply PUT /s3/{BUCKET}/{key} 1")).unwrap());
    assert!(s3.put_file(&key, &file, "video/mp4", 300_000, 256 * 1024).is_err());
    assert!(fake.with(|s| s.multipart.is_empty()), "the failed upload was aborted");
    fake.with(|s| s.control(&format!("fault 503 POST /s3/{BUCKET}/{key} 1")).unwrap());
    assert!(s3.put_file(&key, &file, "video/mp4", 300_000, 256 * 1024).is_err(), "the start refused");
    s3.put_file(&key, &file, "video/mp4", 300_000, 256 * 1024).unwrap();
    assert_eq!(fake.with(|s| s.object(BUCKET, &key)).as_deref(), Some(&bytes[..]));

    // A sink keeps only the size.
    fake.with(|s| s.control(&format!("sink {BUCKET}")).unwrap());
    s3.put_file(&key, &file, "video/mp4", 300_000, 256 * 1024).unwrap();
    assert_eq!(fake.with(|s| s.object(BUCKET, &key)).map(|b| b.len()), Some(0));
    let listed = s3.list(&key).unwrap();
    assert_eq!(listed.iter().find(|o| o.key == key).map(|o| o.size), Some(bytes.len() as u64), "listed at its real size");
}

/// A take crash repair re-cut (its segments rebuilt from the fragmented copy, same names,
/// same sizes, other bytes) after some went up while it recorded: its go-live trusts none
/// of them and the bucket ends with the local files, byte for byte — found by the in-app
/// capture kill (`tests/e2e/kill-relaunch.mjs capture`, DECISIONS B70).
#[test]
fn a_take_rebuilt_after_a_crash_replaces_what_went_up_while_it_recorded() {
    let fake = space();
    let tmp = tempfile::tempdir().unwrap();
    recording(tmp.path());
    // Sent while recording: the init and the first two segments, as the encoder wrote them.
    let s3 = client(&fake.r2());
    for f in ["init.mp4", "seg_00000.m4s", "seg_00001.m4s"] {
        s3.put(&format!("rec/{ID}/v0/{f}"), &std::fs::read(tmp.path().join(f)).unwrap(), "video/mp4").unwrap();
    }
    // The crash, the repair: the same names and sizes, re-cut bytes.
    for f in ["init.mp4", "seg_00000.m4s", "seg_00001.m4s"] {
        let p = tmp.path().join(f);
        let recut: Vec<u8> = std::fs::read(&p).unwrap().iter().map(|b| b.wrapping_add(1)).collect();
        std::fs::write(&p, recut).unwrap();
    }
    // Repair marks the take; without the mark a same-size object passed for the file.
    std::fs::write(tmp.path().join(upload::REBUILT_FILE), b"").unwrap();
    let report = upload::upload_recording(&s3, ID, tmp.path(), "Untitled — Oct 2", |_, _| {}).unwrap();
    assert_eq!(report.already_there, 0, "nothing sent before the crash is trusted");
    whole_and_once(&fake, tmp.path());
}
