//! The CLI against the real Worker (`worker/dev/serve.mjs`: worker.js over an in-memory
//! space, its S3 surface checking every credential) — list, find, rename, and a delete
//! that asks first, then tombstones, removes the bytes and seals. Needs node; without it
//! the test says so and passes as skipped.

#[path = "../examples/demo_space.rs"]
#[allow(dead_code)]
mod demo;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn serve() -> Option<(Serve, String)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut child = Command::new("node")
        .args([&format!("{root}/worker/dev/serve.mjs"), "--space", "v.myshroom.workers.dev=Ann"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut line = String::new();
    BufReader::new(child.stdout.take()?).read_line(&mut line).ok()?;
    let url = serde_json::from_str::<Value>(&line).ok()?["url"].as_str()?.to_string();
    Some((Serve(child), url))
}

fn shroom(home: &std::path::Path, base: &str, args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_shroom")).args(args).arg("--json").env("SHROOM_HOME", home).env("SHROOM_SPACE_BASE", base).output().unwrap();
    (out.status.code().unwrap_or(-1), serde_json::from_slice(&out.stdout).unwrap_or(Value::Null))
}

#[test]
fn an_agent_lists_finds_renames_and_deletes_through_the_space() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    let (code, v) = shroom(&home, &base, &["list"]);
    assert_eq!((code, v["total"].as_u64()), (0, Some(7)), "{v}");
    assert_eq!(v["videos"][0]["title"], "Q3 roadmap walkthrough", "newest first");

    let (code, v) = shroom(&home, &base, &["link", "login loops"]);
    assert_eq!(code, 0);
    assert_eq!(v["link"], "https://v.myshroom.workers.dev/BugRepr0L0g1nLp5");

    let (code, v) = shroom(&home, &base, &["info", "o"]);
    assert_eq!((code, v["error"].as_str()), (4, Some("ambiguous")), "words matching several pick none");

    let (code, v) = shroom(&home, &base, &["cleanup-candidates", "--older-than", "200"]);
    assert_eq!(code, 0);
    let ids: Vec<&str> = v["candidates"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"Fr1st3v3rT3st000") && ids.contains(&"0ldD3m0F0rAcm3Co") && ids.len() == 2, "{ids:?}");
    assert!(v["candidates"][0]["score"].is_null(), "no score until the space counts views");

    let (code, v) = shroom(&home, &base, &["rename", "St4ndupN0t3sJn05", "Standup", "—", "June", "5"]);
    assert_eq!((code, v["title"].as_str()), (0, Some("Standup — June 5")), "{v}");

    // A delete asks first and changes nothing; then deletes for good.
    let (code, v) = shroom(&home, &base, &["delete", "0ldD3m0F0rAcm3Co"]);
    assert_eq!((code, v["error"].as_str()), (7, Some("needs_confirmation")));
    assert_eq!(shroom(&home, &base, &["list"]).1["total"], 7);
    let (code, v) = shroom(&home, &base, &["delete", "old demo", "--yes"]);
    assert_eq!((code, v["error"].as_str()), (2, Some("usage")), "a delete never goes by title words");
    let (code, v) = shroom(&home, &base, &["delete", "https://v.myshroom.workers.dev/0ldD3m0F0rAcm3Co", "--yes"]);
    assert_eq!((code, v["deleted"][0].as_str()), (0, Some("0ldD3m0F0rAcm3Co")), "{v}");
    assert_eq!(shroom(&home, &base, &["list"]).1["total"], 6);

    let text = ureq::get(&format!("{base}/_dev/space/{}/objects", demo::HOST)).call().unwrap().body_mut().read_to_string().unwrap();
    let keys: Vec<String> = serde_json::from_str::<Vec<Value>>(&text)
        .unwrap()
        .iter()
        .map(|o| o["key"].as_str().unwrap().to_string())
        .collect();
    assert!(keys.contains(&".shroom/gone/0ldD3m0F0rAcm3Co".to_string()), "sealed");
    assert!(!keys.iter().any(|k| k.starts_with("rec/0ldD3m0F0rAcm3Co/")), "its bytes are gone");

    let (code, v) = shroom(&home, &base, &["members"]);
    assert_eq!((code, v["members"][0]["name"].as_str()), (0, Some("Mher")));
}

#[test]
fn an_agent_invites_cancels_and_removes_with_a_yes() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    let (code, v) = shroom(&home, &base, &["invite", "Sam", "Lee"]);
    assert_eq!(code, 0, "{v}");
    let link = v["link"].as_str().unwrap();
    assert!(link.starts_with("https://v.myshroom.workers.dev/join/") && link.len() == "https://v.myshroom.workers.dev/join/".len() + 16, "{link}");
    let iid = v["id"].as_str().unwrap().to_string();
    let m = shroom(&home, &base, &["members"]).1;
    assert_eq!(m["invites"][0]["name"], "Sam Lee");
    // The page knows the link by its code's hash: the join page answers it.
    let code_part = link.rsplit('/').next().unwrap();
    let peek = ureq::get(&format!("{base}/{}/join/{code_part}", demo::HOST)).call().unwrap().status();
    assert_eq!(peek.as_u16(), 200);

    let (code, v) = shroom(&home, &base, &["invite", "cancel", &iid]);
    assert_eq!((code, v["cancelled"].as_str()), (0, Some(iid.as_str())), "{v}");

    let (code, v) = shroom(&home, &base, &["members", "remove", "m_MherMherMherMher"]);
    assert_eq!((code, v["error"].as_str()), (7, Some("needs_confirmation")));
    assert_eq!(v["would_remove"][0]["name"], "Mher");
    let (code, _) = shroom(&home, &base, &["members", "remove", "m_MherMherMherMher", "--yes"]);
    assert_eq!(code, 0);
    let roster: Value = serde_json::from_str(
        &ureq::get(&format!("{base}/_dev/space/{}/roster", demo::HOST)).call().unwrap().body_mut().read_to_string().unwrap(),
    )
    .unwrap();
    assert_eq!(roster["members"]["m_MherMherMherMher"]["state"], "removed");
}

#[test]
fn a_mac_that_isnt_set_up_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let (code, v) = shroom(dir.path(), "http://127.0.0.1:9", &["list"]);
    assert_eq!((code, v["error"].as_str()), (3, Some("not_set_up")));
}

#[test]
fn a_team_lead_invites_a_list_in_one_call_and_a_rerun_is_safe() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    // A dry run makes nothing.
    let (code, v) = shroom(&home, &base, &["invite", "--batch", "Sam Lee", "Ana Ruiz", "Mher", "sam lee", "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    let st: Vec<&str> = v["invites"].as_array().unwrap().iter().map(|x| x["status"].as_str().unwrap()).collect();
    assert_eq!(st, ["would_invite", "would_invite", "already_member"], "a name given twice is asked once");
    assert!(shroom(&home, &base, &["members"]).1["invites"].as_array().unwrap().is_empty());

    // One call: a link for each new person, in the order given.
    let (code, v) = shroom(&home, &base, &["invite", "--batch", "Sam Lee", "Ana Ruiz", "Mher"]);
    assert_eq!(code, 0, "{v}");
    let inv = v["invites"].as_array().unwrap();
    assert_eq!((inv[0]["name"].as_str(), inv[0]["status"].as_str()), (Some("Sam Lee"), Some("invited")));
    assert_eq!(inv[1]["status"], "invited");
    assert_eq!(inv[2]["status"], "already_member");
    let (l0, l1) = (inv[0]["link"].as_str().unwrap().to_string(), inv[1]["link"].as_str().unwrap().to_string());
    assert!(l0.contains("/join/") && l0 != l1, "each person their own link");
    assert_eq!(v["counts"]["invited"], 2);

    // Again: nothing is made twice; the links already given can't be shown again.
    let (code, v) = shroom(&home, &base, &["invite", "--batch", "Sam Lee", "Ana Ruiz"]);
    assert_eq!(code, 0);
    assert!(v["invites"].as_array().unwrap().iter().all(|x| x["status"] == "already_invited" && x.get("link").is_none()));
    assert_eq!(shroom(&home, &base, &["members"]).1["invites"].as_array().unwrap().len(), 2);

    // --renew: a new link for one already invited; the old one stops working.
    let (code, v) = shroom(&home, &base, &["invite", "--batch", "Sam Lee", "--renew"]);
    assert_eq!((code, v["invites"][0]["status"].as_str()), (0, Some("renewed")));
    let fresh = v["invites"][0]["link"].as_str().unwrap();
    assert_ne!(fresh, l0);
    let old = l0.rsplit('/').next().unwrap();
    let new = fresh.rsplit('/').next().unwrap();
    let page = |code: &str| ureq::get(&format!("{base}/{}/join/{code}", demo::HOST)).call().unwrap().body_mut().read_to_string().unwrap();
    assert_ne!(page(old), page(new), "the old link no longer opens the invite");

    // Too many at once is refused before anything is made.
    let many: Vec<String> = (0..21).map(|i| format!("Person {i}")).collect();
    let mut args: Vec<&str> = vec!["invite", "--batch"];
    args.extend(many.iter().map(String::as_str));
    assert_eq!(shroom(&home, &base, &args).1["error"], "usage");
}

#[test]
fn many_videos_at_once_each_with_its_own_result() {
    let Some((_serve, base)) = serve() else {
        eprintln!("skipped: node or worker/dev/serve.mjs isn't available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    demo::seed(&base, &home);

    // Renames: several, one already so, dry run first.
    let ren = ["rename", "--batch", "St4ndupN0t3sJn05=Standup — June 5", "Fr1st3v3rT3st000=First test", "Q3rNdPl4nW1kThr0=Q3 roadmap walkthrough"];
    let (code, v) = shroom(&home, &base, &[&ren[..], &["--dry-run"]].concat());
    assert_eq!(code, 0, "{v}");
    let st: Vec<&str> = v["results"].as_array().unwrap().iter().map(|x| x["status"].as_str().unwrap()).collect();
    assert_eq!(st, ["would_rename", "would_rename", "unchanged"]);
    let (code, v) = shroom(&home, &base, &ren);
    assert_eq!((code, v["counts"]["renamed"].as_u64()), (0, Some(2)), "{v}");
    assert_eq!(shroom(&home, &base, &["info", "St4ndupN0t3sJn05"]).1["video"]["title"], "Standup — June 5");

    // Links for a digest: by title words; one that matches several says so, the rest come back.
    let (code, v) = shroom(&home, &base, &["links", "roadmap", "login loops", "o"]);
    assert_eq!((code, v["error"].as_str()), (11, Some("partial")), "{v}");
    assert_eq!(v["links"][0]["link"], "https://v.myshroom.workers.dev/Q3rNdPl4nW1kThr0");
    assert_eq!(v["links"][2]["error"], "ambiguous");

    // This week's: only what's new.
    let (_, all) = shroom(&home, &base, &["list"]);
    let (_, week) = shroom(&home, &base, &["list", "--since", "3650d"]);
    assert_eq!(all["total"], week["total"]);
    assert_eq!(shroom(&home, &base, &["list", "--since", "2026-09-01"]).1["total"], 3);
    // The answer says what it holds: an owner's --mine is "mine", and the filter is echoed.
    let (_, mine) = shroom(&home, &base, &["list", "--mine", "--since", "10d"]);
    assert_eq!((mine["scope"].as_str(), mine["since"].as_str()), (Some("mine"), Some("10d")), "{mine}");
    assert_eq!(shroom(&home, &base, &["list"]).1["scope"], "everyone");

    // Several deletes, one confirmation; an id that isn't there is reported, not fatal; a
    // re-run finds them gone.
    let del = ["delete", "0ldD3m0F0rAcm3Co", "Fr1st3v3rT3st000", "NoSuchVide0Here1"];
    let (code, v) = shroom(&home, &base, &del);
    assert_eq!((code, v["error"].as_str()), (7, Some("needs_confirmation")));
    assert_eq!(v["would_delete"].as_array().unwrap().len(), 2);
    let (code, v) = shroom(&home, &base, &[&del[..], &["--yes"]].concat());
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["deleted"].as_array().unwrap().len(), 2);
    assert_eq!(v["counts"]["not_found"], 1);
    let (code, v) = shroom(&home, &base, &[&del[..], &["--yes"]].concat());
    assert_eq!((code, v["counts"]["not_found"].as_u64()), (0, Some(3)), "a re-run deletes nothing twice");
}
