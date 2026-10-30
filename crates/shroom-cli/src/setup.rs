//! `shroom setup …` — the setup a coding agent drives, with no window (spec N88; the
//! write-up, dev/proposals/agent-setup.md). The app runs the wizard's own states with its
//! window shut; each command here is one of the window's clicks or its paste, asked over the
//! control socket (`app.rs`), and every answer says where the setup stands:
//!
//!   {"ok": true, "at": "token", "say": "…", "do": "…", "next": "shroom setup token --json"}
//!
//! `say` is the step's agent line (the spec's, verbatim), for the agent to say to the
//! person; `do` is what the person does, `null` when it's shroom's turn; `next` is the
//! command to run once they have. The person clicks every button in Cloudflare and macOS.
//!
//! The token never passes through the agent: `shroom setup token` reads it from the
//! clipboard (as `pbpaste` does) or from stdin when it's piped, hands it to the app once
//! (DEVIATIONS #59), clears the clipboard if it still holds it, and prints nothing of it.

use std::io::{IsTerminal, Read, Write};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::app;
use crate::home::Home;
use crate::out::{Done, Fail};
use crate::say;

/// Every step the app can answer with: each has its agent line in the spec (N88; the spec
/// test reads it).
#[cfg_attr(not(test), allow(dead_code))]
pub const STEPS: [&str; 23] = [
    "welcome",
    "account",
    "account-signing-up",
    "token",
    "token-garbled",
    "token-wrong-powers",
    "token-pick-account",
    "card",
    "gate-email",
    "gate-card",
    "name",
    "name-taken",
    "wiring",
    "space-found",
    "done",
    "perm-mic",
    "perm-camera",
    "perm-screen",
    "relaunching",
    "perm-priming",
    "all-set",
    "wizard-offline",
    "wizard-unexpected",
];

/// The command to run after this step; `None` once it's all set. `ran` is the verb that
/// answered: a page just opened waits on the person, so the next run reads what they did.
fn next(at: &str, ran: &str) -> Option<String> {
    let cmd = |v: &str| Some(say::setup_next(v));
    match at {
        "all-set" => None,
        "welcome" | "account" => cmd("account have"),
        "account-signing-up" => cmd("account have"),
        "token" if ran == "token open" => cmd("token"),
        "token" => cmd("token open"),
        "token-garbled" | "token-wrong-powers" => cmd("token"),
        "token-pick-account" => Some(say::setup_next_pick()),
        "gate-card" if ran != "card open" => cmd("card open"),
        "name" | "name-taken" => Some(say::setup_next_name()),
        "done" | "perm-mic" | "perm-camera" | "perm-screen" => cmd("permissions"),
        _ => cmd("wait"),
    }
}

/// A step where something is on its way (a check, a gate's watch, a prompt, a relaunch):
/// `shroom setup wait` waits until it moves.
fn waits(at: &str, busy: bool) -> bool {
    busy || matches!(at, "gate-email" | "gate-card" | "wizard-offline" | "wizard-unexpected" | "perm-mic" | "perm-camera" | "perm-screen" | "relaunching" | "perm-priming")
}

/// How long one `shroom setup wait`, or a step's own wait for its answer, lasts at most.
const WAIT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_secs(2);

pub fn setup(h: &Home, rest: &[String], no_launch: bool) -> Result<Done, Fail> {
    let words: Vec<&str> = rest.iter().map(String::as_str).collect();
    let launch = (!no_launch).then_some(h.app_id.as_str());
    let ask = |op: &str, how: Map<String, Value>| -> Result<Value, Fail> { app::ask(&h.control_socket(), op, &how, launch) };
    let (ran, answer) = match words.as_slice() {
        [] => ("status", ask("setup.status", Map::new())?),
        ["wait"] => ("wait", wait_moved(h, None)?),
        ["account", "new"] => ("account new", ask("setup.account", how([("new", json!(true))]))?),
        ["account", "have"] => ("account have", ask("setup.account", how([("new", json!(false))]))?),
        ["account", "pick", id] => ("account pick", settled(h, ask("setup.pick", how([("account", json!(id))]))?)?),
        ["token", "open"] => ("token open", ask("setup.token-page", Map::new())?),
        ["token"] => ("token", token(h, launch)?),
        ["card", "open"] => ("card open", ask("setup.card-page", Map::new())?),
        ["name", name] => ("name", settled(h, ask("setup.name", how([("name", json!(name))]))?)?),
        ["permissions"] => ("permissions", ask("setup.permissions", Map::new())?),
        _ => return Err(Fail::Usage(say::setup_usage())),
    };
    Ok(done(&answer, ran))
}

fn how<const N: usize>(pairs: [(&str, Value); N]) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// The answer, as the agent reads it and the person would.
fn done(answer: &Value, ran: &str) -> Done {
    let s = &answer["setup"];
    let at = s["at"].as_str().unwrap_or("");
    let line = say::setup_says(at, s["host"].as_str(), s["videos"].as_u64().map(|n| n as u32));
    let mut j = json!({"at": at, "say": line, "do": say::setup_does(at), "next": next(at, ran), "took": s["took"] == true});
    if at == "welcome" || at == "account" {
        // The other answer to the account question.
        j["or"] = json!(say::setup_next("account new"));
    }
    if let Some(h) = s["host"].as_str() {
        j["host"] = json!(h);
    }
    let accounts = s["accounts"].as_array().cloned().unwrap_or_default();
    if !accounts.is_empty() {
        j["accounts"] = Value::Array(accounts.clone());
    }
    let mut text = line;
    for a in &accounts {
        text.push('\n');
        text.push_str(&say::setup_account_row(a["name"].as_str().unwrap_or(""), a["id"].as_str().unwrap_or("")));
    }
    if let Some(n) = next(at, ran) {
        text.push('\n');
        text.push_str(&say::setup_next_line(&n));
    }
    Done::new(j, text)
}

/// A step's own answer, once shroom is done working on it (a token's check, a claim): the
/// agent hears the result, not "checking". At most `WAIT`.
fn settled(h: &Home, first: Value) -> Result<Value, Fail> {
    if first["setup"]["busy"] != true {
        return Ok(first);
    }
    wait_moved(h, Some(first))
}

/// Polls where the setup stands until it moves on from where it was (or, for a step that
/// waits on nothing, at once). The app may be reopening (screen recording's Quit & Reopen):
/// a poll it can't answer is tried again.
fn wait_moved(h: &Home, from: Option<Value>) -> Result<Value, Fail> {
    let peek = || app::ask(&h.control_socket(), "setup.peek", &Map::new(), None);
    let first = match from {
        Some(v) => v,
        None => peek()?,
    };
    let at0 = first["setup"]["at"].as_str().unwrap_or("").to_string();
    if !waits(&at0, first["setup"]["busy"] == true) {
        return Ok(first);
    }
    let start = Instant::now();
    let mut last = first;
    while start.elapsed() < WAIT {
        std::thread::sleep(POLL);
        match peek() {
            Ok(v) => {
                let moved = v["setup"]["at"].as_str() != Some(at0.as_str());
                let idle = v["setup"]["busy"] != true && !matches!(at0.as_str(), "gate-email" | "gate-card" | "wizard-offline" | "wizard-unexpected" | "perm-mic" | "perm-camera" | "perm-screen" | "relaunching" | "perm-priming");
                last = v;
                if moved || idle {
                    return Ok(last);
                }
            }
            Err(Fail::AppUnreachable(_)) => continue,
            Err(f) => return Err(f),
        }
    }
    Ok(last)
}

/// `shroom setup token`: the token from stdin when it's piped, else from the clipboard —
/// read here, handed to the app once, never printed (DEVIATIONS #59).
fn token(h: &Home, launch: Option<&str>) -> Result<Value, Fail> {
    let piped = read_stdin();
    let (text, from_clipboard) = match piped {
        Some(t) => (t, false),
        None => (clipboard().unwrap_or_default(), true),
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(Fail::Refused(say::setup_no_token()));
    }
    let answer = app::ask(&h.control_socket(), "setup.token", &how([("token", json!(text))]), launch)?;
    // Taken: the clipboard lets it go — only if it still holds this token.
    if from_clipboard && answer["setup"]["took"] == true && clipboard().is_some_and(|now| now.trim() == text) {
        clear_clipboard();
    }
    settled(h, answer)
}

/// What was piped in, when something was: a terminal, nothing, or a pipe left open with
/// nothing in it (some agents run every command so) reads as none, after a moment.
fn read_stdin() -> Option<String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return None;
    }
    let mut fd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
    // SAFETY: one pollfd for stdin, and its count.
    let ready = unsafe { libc::poll(&mut fd, 1, 300) };
    if ready <= 0 || fd.revents & libc::POLLIN == 0 {
        return None;
    }
    let mut s = String::new();
    stdin.lock().take(4096).read_to_string(&mut s).ok()?;
    (!s.trim().is_empty()).then_some(s)
}

/// The clipboard's text, as `pbpaste` reads it; `None` off a Mac or when it can't be read.
fn clipboard() -> Option<String> {
    let out = std::process::Command::new("pbpaste").stderr(std::process::Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn clear_clipboard() {
    if let Ok(mut c) = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn() {
        if let Some(mut i) = c.stdin.take() {
            let _ = i.write_all(b"");
        }
        let _ = c.wait();
    }
}

#[cfg(test)]
#[path = "setup_spec.rs"]
mod spec;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_step_says_what_the_person_does_and_what_runs_next() {
        assert_eq!(next("token", "status").as_deref(), Some("shroom setup token open --json"));
        assert_eq!(next("token", "token open").as_deref(), Some("shroom setup token --json"), "the page is open: they copy, then the token");
        assert_eq!(next("gate-card", "status").as_deref(), Some("shroom setup card open --json"));
        assert_eq!(next("gate-card", "card open").as_deref(), Some("shroom setup wait --json"));
        assert_eq!(next("perm-screen", "permissions").as_deref(), Some("shroom setup permissions --json"));
        assert_eq!(next("all-set", "status"), None);
        assert_eq!(say::setup_does("card"), None, "shroom's turn");
        assert!(say::setup_does("token").unwrap().contains("never the token itself"));
        for s in STEPS {
            assert!(next(s, "status").is_some() || s == "all-set", "{s}: no next");
        }
    }

    #[test]
    fn an_answer_reads_as_its_step() {
        let a = json!({"ok": true, "take": "none", "setup": {"at": "token-pick-account", "took": true, "busy": false, "signed_up": false,
            "accounts": [{"id": "0a1b", "name": "Ann's Account"}, {"id": "2c3d", "name": "Acme Engineering"}]}});
        let d = done(&a, "token");
        assert_eq!(d.json["at"], "token-pick-account");
        assert_eq!(d.json["next"], "shroom setup account pick <account id> --json");
        assert_eq!(d.json["accounts"][1]["id"], "2c3d");
        assert!(d.text.contains("Acme Engineering  (shroom setup account pick 2c3d)"));
        let a = json!({"ok": true, "setup": {"at": "all-set", "took": false, "busy": false, "host": "v.myshroom.workers.dev"}});
        let d = done(&a, "status");
        assert_eq!(d.json["say"], "You're set: v.myshroom.workers.dev. Click the mushroom in the menu bar to record.");
        assert_eq!(d.json["next"], Value::Null);
        assert_eq!(d.json["do"], Value::Null);
    }

    /// A stand-in app: answers each ask with the next of `answers`, and keeps what it was
    /// asked.
    fn fake_app(dir: &std::path::Path, answers: Vec<Value>) -> std::thread::JoinHandle<Vec<String>> {
        let listener = std::os::unix::net::UnixListener::bind(dir.join("control.sock")).unwrap();
        std::thread::spawn(move || {
            let mut asked = Vec::new();
            for a in answers {
                let (mut c, _) = listener.accept().unwrap();
                let mut line = String::new();
                std::io::BufRead::read_line(&mut std::io::BufReader::new(c.try_clone().unwrap()), &mut line).unwrap();
                asked.push(line);
                c.write_all(format!("{a}\n").as_bytes()).unwrap();
            }
            asked
        })
    }

    #[test]
    fn a_step_that_is_being_worked_on_is_waited_for() {
        let dir = tempfile::tempdir().unwrap();
        let h = Home { dir: dir.path().into(), app_id: "am.shroom.dev".into() };
        let app = fake_app(
            dir.path(),
            vec![
                json!({"ok": true, "take": "none", "setup": {"at": "name", "took": true, "busy": true}}),
                json!({"ok": true, "take": "none", "setup": {"at": "done", "took": false, "busy": false, "host": "v.myshroom.workers.dev"}}),
            ],
        );
        let d = setup(&h, &["name".into(), "myshroom".into()], true).unwrap();
        let asked = app.join().unwrap();
        assert!(asked[0].contains(r#""op":"setup.name""#) && asked[0].contains(r#""name":"myshroom""#));
        assert!(asked[1].contains(r#""op":"setup.peek""#), "the claim's answer is waited for");
        assert_eq!(d.json["at"], "done");
        assert_eq!(d.json["next"], "shroom setup permissions --json");
    }

    #[test]
    fn the_app_not_running_and_not_to_be_opened_is_unreachable() {
        let dir = tempfile::tempdir().unwrap();
        let h = Home { dir: dir.path().into(), app_id: "am.shroom.dev".into() };
        let f = setup(&h, &[], true).unwrap_err();
        assert_eq!(f.code(), "app_unreachable");
        let f = setup(&h, &["bake".into()], true).unwrap_err();
        assert_eq!(f.code(), "usage");
    }
}
