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
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).args(args).arg("--json").env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    (out.status.code().unwrap_or(-1), serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

fn item(id: &str, title: &str, age: f64, quiet: Option<f64>, plays: u64, last: Option<&str>, candidate: bool, score: f64, code: &str) -> Value {
    // As #57's worker.js `cleanup` writes an item: the record's facts (days as fractions), then
    // the verdict, whose reason carries the facts again, floored.
    json!({
        "id": id, "title": title, "createdAt": "2026-03-11T10:00:00.000Z", "owner": "m_AnnAnnAnnAnnAnnA",
        "ageDays": age, "durationSec": 1500.0, "bytes": 900_000_000u64, "usdPerMonth": 0.0135,
        "plays": plays, "viewers": 1, "medianWatched": 0.1, "lastPlayedAt": last, "plays90": 0, "quietDays": quiet,
        "candidate": candidate, "score": score,
        "reason": {"code": code, "plays": plays, "lastPlayedAt": last, "quietDays": quiet.map(f64::floor), "ageDays": age.floor()},
    })
}

/// Two pages of suggestions: three candidates and two that aren't (kept, new).
fn suggestions(path: &str, body: &Value) -> (u16, Value) {
    assert_eq!(path, format!("/{HOST}/_shroom/api/cleanup"), "only the cleanup route is asked");
    let first = body["after"].is_null();
    let items = if first {
        vec![
            item("Fr1st3v3rT3st000", "Untitled — Feb 2", 258.3, Some(258.3), 0, None, true, 9.5, "never-played"),
            item("Q3rNdPl4nW1kThr0", "Q3 roadmap walkthrough", 2.1, Some(1.2), 3, Some("2026-10-17T09:00:00.000Z"), false, 0.0, "new"),
            item("0ldD3m0F0rAcm3Co", "Old demo for a prospect", 221.4, Some(200.7), 7, Some("2026-04-01T09:00:00.000Z"), true, 4.0, "not-lately"),
        ]
    } else {
        assert_eq!(body["after"], "0ldD3m0F0rAcm3Co", "the next page starts after the last");
        vec![
            item("D3s1gnR3v13wAug1", "Design review", 60.2, Some(49.9), 4, Some("2026-08-30T09:00:00.000Z"), true, 2.5, "stopped-early"),
            item("St4ndupN0t3sJn05", "Standup notes", 135.0, None, 0, None, false, 0.0, "kept"),
        ]
    };
    let next = if first { json!("0ldD3m0F0rAcm3Co") } else { Value::Null };
    (200, json!({"shroom": 1, "countingSince": "2026-02-01T00:00:00.000Z", "counting": true, "formula": "fading-cost", "next": next, "items": items}))
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
    assert_eq!(v["counting"], true, "the space counts plays");
    let ids: Vec<&str> = v["candidates"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Fr1st3v3rT3st000", "0ldD3m0F0rAcm3Co", "D3s1gnR3v13wAug1"], "candidates only, most likely first");
    let c = &v["candidates"][1];
    assert_eq!(c["link"], format!("https://{HOST}/0ldD3m0F0rAcm3Co"));
    // The page's days are fractions (200.7, 221.4); the command gives whole days.
    assert_eq!((c["plays"].as_u64(), c["last_played"].as_str(), c["quiet_days"].as_u64()), (Some(7), Some("2026-04-01T09:00:00.000Z"), Some(200)));
    assert_eq!((c["reason"].as_str(), c["age_days"].as_u64()), (Some("not-lately"), Some(221)));
    assert_eq!((c["title"].as_str(), c["created_at"].as_str()), (Some("Old demo for a prospect"), Some("2026-03-11T10:00:00.000Z")));
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

/// A space with Count plays off (#57's switch) still answers; the command says so, for an
/// agent not to read "never played" as never watched.
#[test]
fn a_space_not_counting_plays_says_so() {
    let (base, _) = page(|path, body| {
        let (status, mut v) = suggestions(path, body);
        v["counting"] = false.into();
        (status, v)
    });
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "OwnerClass");
    let (code, v) = shroom(dir.path(), &base, &["cleanup-candidates"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["counting"], false);
}

/// With auto-cleanup on (N75): what's going comes first, soonest first, each with when; a
/// pinned video is never a candidate, even one a stale page still marks.
#[test]
fn what_auto_cleanup_will_delete_comes_first_and_a_pinned_video_never_does() {
    let (base, _) = page(|path, body| {
        let (status, mut v) = suggestions(path, body);
        if body["after"].is_null() {
            let items = v["items"].as_array_mut().unwrap();
            // The roadmap isn't worth deleting, but nobody watched it: it's going on the 27th.
            items[1]["deletesAt"] = "2026-10-27T03:00:00.000Z".into();
            items[1]["reason"]["code"] = "not-lately".into();
            items[2]["deletesAt"] = "2026-10-26T03:00:00.000Z".into();
            // The first one was pinned since: the page says keep.
            items[0]["keep"] = true.into();
        }
        (status, v)
    });
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "OwnerClass");
    let (code, v) = shroom(dir.path(), &base, &["cleanup-candidates"]);
    assert_eq!(code, 0, "{v}");
    let ids: Vec<&str> = v["candidates"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["0ldD3m0F0rAcm3Co", "Q3rNdPl4nW1kThr0", "D3s1gnR3v13wAug1"], "going first, soonest first; the pinned one never");
    assert_eq!(v["candidates"][0]["deletes_at"], "2026-10-26T03:00:00.000Z");
    assert!(v["candidates"][2]["deletes_at"].is_null());
    assert!(v["candidates"][0]["why"].as_str().unwrap().contains("unless pinned"), "{}", v["candidates"][0]);
}

/// `shroom autoclean` reads the space's setting and what's going; it never changes it (the
/// app on the Mac that set up the space does, as it gives the page its schedule).
#[test]
fn autoclean_reads_the_setting_and_whats_next_and_never_changes_it() {
    let (base, heard) = page(|path, _| {
        assert_eq!(path, format!("/{HOST}/_shroom/api/autoclean"));
        (200, json!({"shroom": 1, "days": 30, "since": "2026-10-01T00:00:00.000Z",
            "going": [{"id": "0ldD3m0F0rAcm3Co", "deletesAt": "2026-10-27T03:00:00.000Z"}, {"id": "Fr1st3v3rT3st000", "deletesAt": "2026-10-27T05:00:00.000Z"}, {"id": "D3s1gnR3v13wAug1", "deletesAt": "2026-10-28T03:00:00.000Z"}],
            "last": {"at": "2026-10-20T03:00:00.000Z", "ids": ["St4ndupN0t3sJn05"], "bytes": 900_000_000u64}}))
    });
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "Member");
    let (code, v) = shroom(dir.path(), &base, &["autoclean"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!((v["days"].as_u64(), v["on"].as_bool()), (Some(30), Some(true)));
    assert_eq!(v["next_at"], "2026-10-27T03:00:00.000Z");
    assert_eq!(v["going"].as_array().unwrap().len(), 3);
    assert_eq!(v["going"][0]["link"], format!("https://{HOST}/0ldD3m0F0rAcm3Co"));
    assert_eq!(v["last"]["ids"][0], "St4ndupN0t3sJn05");
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).arg("autoclean").env("SHROOM_HOME", dir.path()).env("SHROOM_SPACE_BASE", &base).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Auto-cleanup: videos nobody watches for 30 days are deleted."), "{text}");
    assert!(text.contains("Next: 2 videos on Oct 27. Pin one to keep it: shroom pin <video>"), "{text}");

    let before = heard.lock().unwrap().len();
    let (code, v) = shroom(dir.path(), &base, &["autoclean", "60"]);
    assert_eq!((code, v["error"].as_str()), (6, Some("refused")), "{v}");
    assert!(v["message"].as_str().unwrap().contains("Settings"));
    assert_eq!(heard.lock().unwrap().len(), before, "nothing asked of the space");
}

#[test]
fn autoclean_at_a_year_says_a_year() {
    // N77: the page's 365 reads as the landing says it; `days` stays the number.
    let (base, _) = page(|_, _| (200, json!({"shroom": 1, "days": 365, "since": "2026-10-01T00:00:00.000Z", "going": [], "last": null})));
    let dir = tempfile::tempdir().unwrap();
    mac(dir.path(), "OwnerClass");
    let (code, v) = shroom(dir.path(), &base, &["autoclean"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!((v["days"].as_u64(), v["on"].as_bool()), (Some(365), Some(true)), "{v}");
    let out = Command::new(env!("CARGO_BIN_EXE_shroom-cli")).arg("autoclean").env("SHROOM_HOME", dir.path()).env("SHROOM_SPACE_BASE", &base).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Auto-cleanup: videos nobody watches for a year are deleted."), "{text}");
}

#[test]
fn autoclean_off_or_on_a_page_without_it_says_off() {
    for answer in [(200, json!({"shroom": 1, "days": 0, "since": null, "going": [], "last": null})), (404, json!({"shroom": 1, "error": "not-found"}))] {
        let (base, _) = page(move |_, _| answer.clone());
        let dir = tempfile::tempdir().unwrap();
        mac(dir.path(), "OwnerClass");
        let (code, v) = shroom(dir.path(), &base, &["autoclean"]);
        assert_eq!(code, 0, "{v}");
        assert_eq!((v["days"].as_u64(), v["on"].as_bool()), (Some(0), Some(false)), "{v}");
        assert!(v["going"].as_array().unwrap().is_empty());
    }
}
