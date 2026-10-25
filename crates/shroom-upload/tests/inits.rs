//! A take whose picture differed plays under its own init segment (`init_<n>.mp4`, named
//! by the joined playlist after its pause — shroom-encode's `take_init`): the upload sends
//! every init the playlist names, and refuses a recording missing one rather than publish a
//! stream that can't play.

use std::path::Path;

use shroom_cloudfake::{Fake, State};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::upload::{self, Local, UploadError};

const ID: &str = "AbCdEfGhIjKlMnOq";

fn recording(dir: &Path) {
    let playlist = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:6\n#EXT-X-MAP:URI=\"init.mp4\"\n\
        #EXTINF:6.000,\nseg_00000.m4s\n#EXT-X-DISCONTINUITY\n#EXT-X-MAP:URI=\"init_1.mp4\"\n#EXTINF:4.000,\nseg_00001.m4s\n\
        #EXT-X-DISCONTINUITY\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:2.000,\nseg_00002.m4s\n#EXT-X-ENDLIST\n";
    std::fs::write(dir.join("stream.m3u8"), playlist).unwrap();
    for i in 0..3 {
        std::fs::write(dir.join(format!("seg_{i:05}.m4s")), vec![i as u8; 64]).unwrap();
    }
    std::fs::write(dir.join("init.mp4"), b"the shared init").unwrap();
    std::fs::write(dir.join("init_1.mp4"), b"the odd take's init").unwrap();
    std::fs::write(dir.join("preview.mp4"), b"the single-file copy").unwrap();
}

#[test]
fn every_init_the_playlist_names_goes_up() {
    let dir = tempfile::tempdir().unwrap();
    recording(dir.path());
    // Two inits, three segments, the variant playlist and the master.
    assert_eq!(Local::read(dir.path()).unwrap().stream_files(), 7);

    let mut st = State::fresh();
    st.buckets.insert("shroom-space".into(), Default::default());
    let fake = Fake::start(st);
    let s3 = Client::new(Bucket {
        endpoint: fake.r2(),
        region: "auto".into(),
        name: "shroom-space".into(),
        credentials: Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None },
    });
    upload::upload_recording(&s3, ID, dir.path(), "Two pictures", |_, _| {}).unwrap();
    for f in ["init.mp4", "init_1.mp4", "seg_00002.m4s", "stream.m3u8"] {
        let key = format!("rec/{ID}/v0/{f}");
        assert!(fake.with(|s| s.object("shroom-space", &key)).is_some(), "{key} is in the bucket");
    }
}

#[test]
fn a_named_init_missing_is_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    recording(dir.path());
    std::fs::remove_file(dir.path().join("init_1.mp4")).unwrap();
    assert!(matches!(Local::read(dir.path()), Err(UploadError::Incomplete(_))));
}

/// Only an init named as the encoder names them (`init.mp4`, `init_<n>.mp4`) is read and
/// sent: a playlist naming any other file — a path, a dot file — is refused whole.
#[test]
fn a_playlist_naming_any_other_init_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    recording(dir.path());
    std::fs::write(dir.path().join(".hidden.mp4"), b"x").unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/init.mp4"), b"x").unwrap();
    std::fs::write(dir.path().join("other.json"), b"x").unwrap();
    let playlist = std::fs::read_to_string(dir.path().join("stream.m3u8")).unwrap();
    for name in ["../init.mp4", "sub/init.mp4", ".hidden.mp4", "other.json", "/etc/hosts", "init_0.mp4", "init_.mp4", "init_1.mp4/", ""] {
        std::fs::write(dir.path().join("stream.m3u8"), playlist.replace("URI=\"init_1.mp4\"", &format!("URI=\"{name}\""))).unwrap();
        assert!(matches!(Local::read(dir.path()), Err(UploadError::Incomplete(_))), "{name:?}");
    }
    std::fs::write(dir.path().join("stream.m3u8"), playlist).unwrap();
    assert!(Local::read(dir.path()).is_ok(), "the encoder's own names read");
}
