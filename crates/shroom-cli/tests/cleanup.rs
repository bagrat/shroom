//! Cleanup candidates are the page's own suggestions where the page makes them (spec N62,
//! DECISIONS B215: `POST /_shroom/api/cleanup`, paged by `after`, under this Mac's device
//! key), with plays, when last played and why; a page without them (404, an older page)
//! gets the videos older than `--older-than` days, in the same shape with what isn't known
//! `null` (`tests/space.rs`, on the real worker.js). Here the page is a stand-in that answers
//! the route as the page that makes suggestions does.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

const HOST: &str = "v.myshroom.workers.dev";

/// What the stand-in heard: each request's path, its authorization, whether it carried an
/// Origin, and its body.
type Heard = Arc<Mutex<Vec<(String, String, bool, Value)>>>;

/// A page on loopback answering `answer(path, body)`; its base for `SHROOM_SPACE_BASE`.
fn page(answer: impl Fn(&str, &Value) -> (u16, Value) + Send + 'static) -> (String, Heard) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let heard: Heard = Arc::default();
    let log = heard.clone();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            let mut r = BufReader::new(conn.try_clone().unwrap());
            let mut line = String::new();
            r.read_line(&mut line).unwrap();
            let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
            let (mut length, mut auth, mut origin) = (0, String::new(), false);
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                if h.trim().is_empty() {
                    break;
                }
                let (k, v) = h.split_once(':').unwrap_or((&h, ""));
                match k.to_ascii_lowercase().as_str() {
                    "content-length" => length = v.trim().parse().unwrap_or(0),
                    "authorization" => auth = v.trim().to_string(),
                    "origin" => origin = true,
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            r.read_exact(&mut body).unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            log.lock().unwrap().push((path.clone(), auth, origin, body.clone()));
            let (status, out) = answer(&path, &body);
            let out = out.to_string();
            let _ = write!(conn, "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{out}", out.len());
        }
    });
    (base, heard)
}

/// A Mac in the space at `home`: its space (an owner's, or a member's) and its device key.
fn mac(home: &Path, role: &str) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::write(
        home.join("space.json"),
        format!(r#"{{"account_id":"acc","label":"v","subdomain":"myshroom","host":"{HOST}","bucket":"shroom-space","role":"{role}","member_id":"m_AnnAnnAnnAnnAnnA","owner_name":"Ann"}}"#),
    )
    .unwrap();
    std::fs::write(home.join("device.key"), format!(r#"{{"v":1,"host":"{HOST}","device":"d_AAAAAAAAAAAAAAAA","secret":"s3cr3t"}}"#)).unwrap();
}

fn shroom(home: &Path, base: &str, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_shroom")).args(args).arg("--json").env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    (out.status.code().unwrap_or(-1), serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

fn item(id: &str, title: &str, age: u64, candidate: bool, score: f64, reason: Value) -> Value {
    json!({
        "id": id, "title": title, "createdAt": "2026-03-11T10:00:00.000Z", "owner": "m_AnnAnnAnnAnnAnnA",
        "ageDays": age, "durationSec": 1500.0, "bytes": 900_000_000u64, "usdPerMonth": 0.0,
        "plays": reason["plays"].clone(), "viewers": 1, "medianWatched": 0.1, "lastPlayedAt": reason["lastPlayedAt"].clone(),
        "plays90": 0, "quietDays": reason["quietDays"].clone(), "keep": false, "candidate": candidate, "score": score, "reason": reason,
    })
}

/// Two pages of suggestions: three candidates and two that aren't (kept, new).
fn suggestions(path: &str, body: &Value) -> (u16, Value) {
    assert_eq!(path, format!("/{HOST}/_shroom/api/cleanup"), "only the cleanup route is asked");
    let first = body["after"].is_null();
    let items = if first {
        vec![
            item("Fr1st3v3rT3st000", "Untitled — Feb 2", 258, true, 9.5, json!({"code": "never-played", "plays": 0, "lastPlayedAt": null, "quietDays": 258, "ageDays": 258})),
            item("Q3rNdPl4nW1kThr0", "Q3 roadmap walkthrough", 2, false, 0.0, json!({"code": "new", "plays": 3, "lastPlayedAt": "2026-10-17T09:00:00.000Z", "quietDays": 1})),
            item("0ldD3m0F0rAcm3Co", "Old demo for a prospect", 221, true, 4.0, json!({"code": "not-lately", "plays": 7, "lastPlayedAt": "2026-04-01T09:00:00.000Z", "quietDays": 200, "ageDays": 221})),
        ]
    } else {
        assert_eq!(body["after"], "0ldD3m0F0rAcm3Co", "the next page starts after the last");
        vec![
            item("D3s1gnR3v13wAug1", "Design review", 60, true, 2.5, json!({"code": "stopped-early", "plays": 4, "lastPlayedAt": "2026-08-30T09:00:00.000Z", "quietDays": 49, "medianWatched": 0.12})),
            item("St4ndupN0t3sJn05", "Standup notes", 135, false, 0.0, json!({"code": "kept"})),
        ]
    };
    let next = if first { json!("0ldD3m0F0rAcm3Co") } else { Value::Null };
    (200, json!({"shroom": 1, "countingSince": "2026-02-01T00:00:00.000Z", "formula": "idle", "next": next, "items": items}))
}

#[test]
fn the_pages_own_suggestions_every_page_of_them_most_likely_first() {
    let (base, heard) = page(suggestions);
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "OwnerClass");
    let (code, v) = shroom(dir.path(), &base, &["cleanup-candidates"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["by"], "suggestions", "{v}");
    assert_eq!(v["counting_since"], "2026-02-01T00:00:00.000Z");
    let ids: Vec<&str> = v["candidates"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Fr1st3v3rT3st000", "0ldD3m0F0rAcm3Co", "D3s1gnR3v13wAug1"], "candidates only, most likely first");
    let c = &v["candidates"][1];
    assert_eq!(c["link"], format!("https://{HOST}/0ldD3m0F0rAcm3Co"));
    assert_eq!((c["plays"].as_u64(), c["last_played"].as_str(), c["quiet_days"].as_u64()), (Some(7), Some("2026-04-01T09:00:00.000Z"), Some(200)));
    assert_eq!((c["reason"].as_str(), c["age_days"].as_u64()), (Some("not-lately"), Some(221)));
    assert!(c["why"].as_str().is_some_and(|w| w.contains("200 days")), "its why, in words: {c}");
    assert_eq!(v["candidates"][0]["reason"], "never-played");
    // Under this Mac's device key, no Origin (as the app calls its page); an owner's Mac
    // asks for everyone's videos.
    let heard = heard.lock().unwrap();
    assert_eq!(heard.len(), 2, "two pages");
    assert!(heard.iter().all(|(_, auth, origin, _)| auth == "Bearer d_AAAAAAAAAAAAAAAA.s3cr3t" && !origin));
    assert_eq!(heard[0].3["scope"], "everyone");
}

#[test]
fn older_than_narrows_the_suggestions_and_a_members_mac_asks_for_its_own() {
    let (base, heard) = page(suggestions);
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "Member");
    let (code, v) = shroom(dir.path(), &base, &["cleanup-candidates", "--older-than", "200"]);
    assert_eq!(code, 0, "{v}");
    let ids: Vec<&str> = v["candidates"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Fr1st3v3rT3st000", "0ldD3m0F0rAcm3Co"], "only the suggestions at least 200 days old");
    assert_eq!(v["older_than_days"], 200);
    assert_eq!(heard.lock().unwrap()[0].3["scope"], "mine");
}
