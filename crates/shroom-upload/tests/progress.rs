//! L1 (whole-spec audit #17, #18, #29, #30; DECISIONS B49): what the Mac writes while a
//! recording goes up is what the page reads. The upload runs against the L1 fake's bucket;
//! the record it leaves there at each moment is handed to the real Worker's `status()`
//! (`worker/src/worker.js`, under node) — the same bytes, read by both sides.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use shroom_cloudfake::{Fake, State};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::meta::{Meta, State as MetaState};
use shroom_upload::upload::{self, Beat, Local};

const ID: &str = "AbCdEfGhIjKlMnOp";
const SEGMENTS: usize = 6;

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

/// A finalized recording on disk: the joined playlist, the init, six segments — and the
/// single-file copy, the one second-class file this take has.
fn recording(dir: &Path) {
    let mut playlist = String::from("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:6\n#EXT-X-MAP:URI=\"init.mp4\"\n");
    for i in 0..SEGMENTS {
        playlist.push_str(&format!("#EXTINF:6.000,\nseg_{i:05}.m4s\n"));
        std::fs::write(dir.join(format!("seg_{i:05}.m4s")), vec![i as u8; 64]).unwrap();
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    std::fs::write(dir.join("stream.m3u8"), playlist).unwrap();
    std::fs::write(dir.join("init.mp4"), b"not a real init").unwrap();
    std::fs::write(dir.join("preview.mp4"), b"the single-file copy").unwrap();
    std::fs::write(dir.join(upload::MARKERS_FILE), br#"{"v":1,"seconds":[12.5,40]}"#).unwrap();
}

fn meta(fake: &Fake) -> Meta {
    serde_json::from_slice(&fake.with(|s| s.object("shroom-space", &format!("meta/{ID}.json"))).expect("the record is there")).unwrap()
}

/// The real Worker's `status()` over each record, at `now` (unix ms) — `None` when node
/// isn't on this Mac (the Worker's own suite covers its side then).
fn worker_status(metas: &[serde_json::Value], now: i64) -> Option<Vec<serde_json::Value>> {
    let worker = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/src/worker.js").canonicalize().ok()?;
    let script = format!(
        "import {{ status }} from {:?};\nlet s='';process.stdin.on('data',d=>s+=d).on('end',()=>{{const ms=JSON.parse(s);console.log(JSON.stringify(ms.map(m=>status(m,{now}))))}});",
        format!("file://{}", worker.display())
    );
    let mut child = Command::new("node").args(["--input-type=module", "-e", &script]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().ok()?;
    use std::io::Write;
    child.stdin.take()?.write_all(serde_json::to_string(metas).unwrap().as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    assert!(out.status.success(), "node ran the Worker's status(): {}", String::from_utf8_lossy(&out.stderr));
    Some(serde_json::from_slice(&out.stdout).unwrap())
}

/// How long a test waits for the beat to write what it should — a bound on a hang, not a
/// pace. The beat writes from its own thread, over the network, so the test waits for what
/// it wrote rather than for a fixed time: on GitHub's macOS runner a beat (a read and a
/// conditional write) took longer than the 150 ms the test once slept (#56, #45).
const PATIENCE: Duration = Duration::from_secs(10);

/// The record once `holds` is true of it, polled; `None` if it isn't within [`PATIENCE`].
/// Called inside the upload's progress callback, where a panic would leave the beat running:
/// the caller asserts on the `Option` after the upload returns.
fn record_once(fake: &Fake, holds: impl Fn(&Meta) -> bool) -> Option<Meta> {
    let until = Instant::now() + PATIENCE;
    loop {
        let m = meta(fake);
        if holds(&m) {
            return Some(m);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64
}

#[test]
fn the_page_reads_the_upload_as_the_mac_writes_it() {
    let (fake, s3) = space();
    let tmp = tempfile::tempdir().unwrap();
    recording(tmp.path());
    let local = Local::read(tmp.path()).unwrap();
    let stream = local.stream_files();
    let total = stream + 1; // + the single-file copy

    // The claim: the record exists before any byte, with its title, markers and count.
    upload::claim(&s3, ID, "Untitled — Oct 2", &upload::local_markers(tmp.path()), total).unwrap();
    let claimed = meta(&fake);
    assert_eq!(claimed.state, MetaState::Uploading);
    assert_eq!(claimed.title, "Untitled — Oct 2");
    assert_eq!(claimed.markers.as_deref(), Some(&[12.5, 40.0][..]));
    let p = claimed.progress.clone().expect("the claim starts the count");
    assert_eq!((p.done, p.total), (0, total));
    assert_eq!(fake.with(|s| s.calls.iter().filter(|c| c.starts_with("PUT /s3/shroom-space/rec/")).count()), 0, "the claim comes before any media");

    // The stream, with a fast beat: mid-way, held after its fourth file, the record says
    // how far it got.
    let beat = Beat { every: Duration::from_millis(20), keepalive: Duration::from_millis(60) };
    let mut mid = None;
    let report = upload::go_live(&s3, ID, tmp.path(), &local, &Default::default(), 1, &beat, |done, _| {
        if done == 4 {
            mid = Some(record_once(&fake, |m| m.progress.as_ref().is_some_and(|p| p.done == 4)));
        }
    })
    .unwrap();
    assert_eq!(report.uploaded as usize, SEGMENTS + 1);
    let mid = mid.expect("the upload passed its fourth file").expect("the beat wrote the count while bytes moved");
    let mp = mid.progress.clone().unwrap();
    assert_eq!(mid.state, MetaState::Uploading);
    assert!(mp.done < stream, "{mp:?}");
    assert_eq!(mp.total, total);

    // Live, but the single-file copy is still to come: not drained.
    let at_live = meta(&fake);
    assert_eq!(at_live.state, MetaState::Live);
    let lp = at_live.progress.clone().unwrap();
    assert_eq!((lp.done, lp.total), (stream, total));

    // The second-class file, then drained.
    let mp4 = upload::finish(&s3, ID, tmp.path(), stream, total, &beat, |_, _| {}).unwrap();
    assert!(mp4);
    let done = meta(&fake);
    let dp = done.progress.clone().unwrap();
    assert_eq!((dp.done, dp.total), (total, total));
    assert!(done.media.as_ref().is_some_and(|m| m.mp4.is_some()));
    // Drained, the record says what the video holds in the bucket (N68's suggestions read it).
    let stored: u64 = s3.list(&format!("rec/{ID}/")).unwrap().iter().map(|o| o.size).sum();
    assert!(stored > 0);
    assert_eq!(done.extra.get("bytes").and_then(|b| b.as_u64()), Some(stored));

    // The Worker reads the same three records: uploading and moving → live, still
    // draining → drained (“Fully live ✓”).
    let records: Vec<serde_json::Value> = [&mid, &at_live, &done].iter().map(|m| serde_json::to_value(m).unwrap()).collect();
    let Some(read) = worker_status(&records, now_ms()) else {
        eprintln!("node isn't here — the Worker's half is covered by worker/test/page.test.js");
        return;
    };
    assert_eq!(read[0]["state"], "uploading");
    assert_eq!(read[0]["stalled"], false);
    assert!(read[0]["pct"].as_u64().unwrap() > 0, "{:?}", read[0]);
    assert_eq!((read[1]["state"].as_str(), read[1]["drained"].as_bool(), read[1]["stalled"].as_bool()), (Some("live"), Some(false), Some(false)));
    assert_eq!((read[2]["state"].as_str(), read[2]["drained"].as_bool()), (Some("live"), Some(true)));
    // Two minutes on with nothing written, the same mid-upload record reads stalled.
    let later = worker_status(&records[..1], now_ms() + 125_000).unwrap();
    assert_eq!(later[0]["stalled"], true);
}

#[test]
fn a_long_file_keeps_the_record_fresh_and_a_tombstone_is_never_overwritten() {
    let (fake, s3) = space();
    let tmp = tempfile::tempdir().unwrap();
    recording(tmp.path());
    let local = Local::read(tmp.path()).unwrap();
    upload::claim(&s3, ID, "t", &[], local.stream_files()).unwrap();
    // Nothing moves for a while (one long file): the keepalive still rewrites the record.
    let beat = Beat { every: Duration::from_millis(20), keepalive: Duration::from_millis(40) };
    let mut seen = None;
    upload::go_live(&s3, ID, tmp.path(), &local, &Default::default(), 0, &beat, |done, _| {
        if done == 1 {
            // The upload is held here, after its first file: one long file in flight.
            let first = record_once(&fake, |m| m.progress.as_ref().is_some_and(|p| p.done == 1));
            // updatedAt has whole seconds: the keepalive moves it past the next one.
            let later = first.as_ref().and_then(|f| {
                let at = f.progress.as_ref().unwrap().updated_at.clone();
                record_once(&fake, |m| m.progress.as_ref().is_some_and(|p| p.updated_at != at))
            });
            seen = Some((first, later));
        }
    })
    .unwrap();
    let (first, later) = seen.expect("the upload passed its first file");
    let first = first.expect("the beat wrote the count").progress.unwrap();
    let later = later.expect("the keepalive rewrote the record while nothing moved").progress.unwrap();
    assert_eq!((first.done, later.done), (1, 1), "nothing moved");
    assert_ne!(first.updated_at, later.updated_at, "the keepalive moved updatedAt");

    // Deleted meanwhile: the upload stops at the record, and nothing rewrites the tombstone.
    let mut tomb = meta(&fake);
    tomb.state = MetaState::Deleted;
    fake.with(|s| s.put_object("shroom-space", &format!("meta/{ID}.json"), &serde_json::to_vec(&tomb).unwrap()));
    let err = upload::finish(&s3, ID, tmp.path(), 1, 2, &beat, |_, _| {}).unwrap_err();
    assert!(matches!(err, upload::UploadError::Tombstoned), "{err}");
    assert_eq!(meta(&fake).state, MetaState::Deleted);
}
