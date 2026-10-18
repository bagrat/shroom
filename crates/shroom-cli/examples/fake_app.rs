//! Dev only: a stand-in for shroom.app's side of the CLI's control socket, for the cloud
//! demo — the real one is the app's (proposed, dev/proposals/cli-agent-skill.md §3), and
//! the app runs only on a Mac. It keeps `status.json` as the app does (§11.2) and answers
//! one JSON line per connection; a stop puts an invented video into the demo space.
//!
//!   cp target/debug/examples/fake_app <dir>/shroom   # the seam wants a process named shroom
//!   <dir>/shroom <serve url> <home dir>               # serves until stdin closes

#[path = "demo_space.rs"]
#[allow(dead_code)]
mod demo;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{Value, json};

struct Take {
    state: &'static str,
    since: Instant,
}

fn write_status(home: &std::path::Path, take: &Option<Take>) {
    let state = take.as_ref().map_or("none", |t| t.state);
    let s = json!({
        "v": 2, "pid": std::process::id(), "app": "0.1.24", "take_open": take.is_some(), "take": state, "setup": "all-set",
        "permission": {"screen": "granted", "mic": "granted", "camera": "not-asked", "chapter": "done"},
        "join": {"role": "token-holder", "invite": "none", "invite_kind": "none", "standing": "fine"},
    });
    std::fs::write(home.join("status.json"), serde_json::to_string_pretty(&s).unwrap()).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [base, home] = &args[..] else { panic!("shroom <serve url> <home dir>") };
    let base = base.trim_end_matches('/').to_string();
    let home = std::path::PathBuf::from(home);
    let sock = home.join("control.sock");
    let _ = std::fs::remove_file(&sock);
    let listener = UnixListener::bind(&sock).unwrap();
    let take: Arc<Mutex<Option<Take>>> = Arc::new(Mutex::new(None));
    write_status(&home, &None);
    {
        let (home, sock) = (home.clone(), sock.clone());
        std::thread::spawn(move || {
            let _ = std::io::stdin().read_to_end(&mut Vec::new());
            let _ = std::fs::remove_file(home.join("status.json"));
            let _ = std::fs::remove_file(sock);
            std::process::exit(0);
        });
    }
    println!("{}", json!({"socket": sock}));
    for conn in listener.incoming().flatten() {
        let mut line = String::new();
        let mut r = BufReader::new(&conn);
        if r.read_line(&mut line).is_err() {
            continue;
        }
        let op = serde_json::from_str::<Value>(&line).ok().and_then(|v| v["op"].as_str().map(String::from)).unwrap_or_default();
        let mut t = take.lock().unwrap();
        // Settings → General's “Let it start and stop recordings”, off by default: the app
        // keeps it in prefs.json (`agent_recording`).
        let allowed = std::fs::read(home.join("prefs.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .is_some_and(|p| p["agent_recording"] == true);
        let answer = match (op.as_str(), t.as_ref().map(|x| x.state)) {
            // The app answers with a code; the command says the words (control.rs).
            _ if !allowed => json!({"ok": false, "take": "none", "error": "recording_off"}),
            ("record.start", None) => {
                *t = Some(Take { state: "countdown", since: Instant::now() });
                let (take, home) = (take.clone(), home.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let mut t = take.lock().unwrap();
                    if let Some(x) = t.as_mut().filter(|x| x.state == "countdown") {
                        x.state = "recording";
                        x.since = Instant::now();
                    }
                    write_status(&home, &t);
                });
                json!({"ok": true, "take": "countdown"})
            }
            ("record.start", Some(_)) => json!({"ok": false, "take": "recording", "error": "take_open"}),
            ("record.pause", Some("recording")) => {
                t.as_mut().unwrap().state = "paused";
                json!({"ok": true, "take": "paused"})
            }
            ("record.resume", Some("paused")) => {
                t.as_mut().unwrap().state = "recording";
                json!({"ok": true, "take": "recording"})
            }
            ("record.stop", Some(_)) => {
                let secs = t.take().unwrap().since.elapsed().as_secs_f64().max(1.0);
                let id = shroom_cloud::device::random_b64(12);
                demo::put_video(&demo::bucket(&base), &id, "Untitled — Oct 18", &shroom_upload::meta::now_iso(), secs.round());
                json!({"ok": true, "take": "none", "link": format!("https://{}/{id}", demo::HOST)})
            }
            (_, None) => json!({"ok": false, "take": "none", "error": "no_take"}),
            _ => json!({"ok": false, "take": "recording", "error": "busy"}),
        };
        write_status(&home, &t);
        let _ = (&conn).write_all(format!("{answer}\n").as_bytes());
    }
}
