//! Dev only: a space with videos in it, on the local stand-in (`worker/dev/serve.mjs`),
//! and a shroom home that is in it — what the CLI's cloud demo and its end-to-end test
//! run against (dev/proposals/cli-agent-skill.md §7). Every value here is invented.
//!
//!   cargo run -q -p shroom-cli --example demo_space -- <serve url> <home dir>
//!
//! The home gets `space.json` and `device.key` (an owner's Mac) and no token: the CLI
//! never reads one.

use shroom_cloud::device::DeviceKey;
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_upload::meta::{Media, Meta, State};

pub const HOST: &str = "v.myshroom.workers.dev";
const PARENT_SECRET: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const KEY_ID: &str = "demo0000000000000000000000000000";
const ACCOUNT: &str = "acc00000000000000000000000000000";

fn post(base: &str, what: &str, body: serde_json::Value) {
    let url = format!("{base}/_dev/space/{HOST}/{what}");
    let res = ureq::post(&url).header("content-type", "application/json").send(body.to_string()).expect("the stand-in answers");
    assert!(res.status().is_success(), "{what}: {}", res.status());
}

/// The bucket under the space's parent pair (a token Mac's own reach).
pub fn bucket(base: &str) -> Client {
    Client::new(Bucket {
        endpoint: format!("{base}/_s3/{HOST}"),
        region: "auto".into(),
        name: "shroom-space".into(),
        credentials: Credentials { access_key_id: KEY_ID.into(), secret_access_key: PARENT_SECRET.into(), session_token: None },
    })
}

pub fn put_video(s3: &Client, id: &str, title: &str, created: &str, secs: f64) {
    let mut m = Meta::uploading(id, title, created);
    m.state = State::Live;
    m.published_at = Some(created.into());
    m.duration_sec = Some(secs);
    m.media = Some(Media { hls: format!("rec/{id}/master.m3u8"), mp4: None, poster: None, teaser: None, width: 1920, height: 1080 });
    s3.put(&format!("meta/{id}.json"), serde_json::to_string(&m).unwrap().as_bytes(), "application/json").expect("the record lands");
    s3.put(&format!("rec/{id}/master.m3u8"), b"#EXTM3U\n", "application/vnd.apple.mpegurl").expect("bytes land");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [base, home] = &args[..] else { panic!("demo_space <serve url> <home dir>") };
    seed(base.trim_end_matches('/'), std::path::Path::new(home));
    println!("{}", serde_json::json!({"home": home, "host": HOST, "videos": 7}));
}

/// The space's parent, an owner's Mac and a member's in its roster, seven videos, and
/// `home` as the owner's Mac's shroom home.
pub fn seed(base: &str, home: &std::path::Path) {
    post(base, "parent", serde_json::json!({"account_id": ACCOUNT, "bucket": "shroom-space", "key_id": KEY_ID, "secret": PARENT_SECRET}));
    let key = DeviceKey::generate(HOST);
    post(base, "device", serde_json::json!({"id": key.device, "verifier": key.verifier(), "member": "m1", "name": "MacBook Pro", "token": true}));
    post(base, "device", serde_json::json!({"id": "d_MherMacMherMac01", "verifier": "00", "member": "m_MherMherMherMher", "label": "Mher", "name": "Mher's Mac"}));
    let s3 = bucket(base);
    for (id, title, created, secs) in [
        ("Q3rNdPl4nW1kThr0", "Q3 roadmap walkthrough", "2026-10-16T09:12:00Z", 412.0),
        ("BugRepr0L0g1nLp5", "Bug repro: login loops on Safari", "2026-10-14T15:40:00Z", 96.0),
        ("0nb0ard1ngFl0w22", "Onboarding flow, first pass", "2026-09-30T11:05:00Z", 248.0),
        ("D3s1gnR3v13wAug1", "Design review — pricing page", "2026-08-21T16:30:00Z", 731.0),
        ("St4ndupN0t3sJn05", "Standup notes", "2026-06-05T08:58:00Z", 133.0),
        ("0ldD3m0F0rAcm3Co", "Old demo for a prospect", "2026-03-11T13:20:00Z", 1520.0),
        ("Fr1st3v3rT3st000", "Untitled — Feb 2", "2026-02-02T10:00:00Z", 12.0),
    ] {
        put_video(&s3, id, title, created, secs);
    }
    std::fs::create_dir_all(home).unwrap();
    let space = serde_json::json!({
        "account_id": ACCOUNT, "label": "v", "subdomain": "myshroom", "host": HOST, "bucket": "shroom-space",
        "role": "TokenHolder", "member_id": "m1", "owner_name": "Ann", "device_id": key.device,
    });
    std::fs::write(home.join("space.json"), space.to_string()).unwrap();
    std::fs::write(home.join("device.key"), serde_json::to_string(&key).unwrap()).unwrap();
}
