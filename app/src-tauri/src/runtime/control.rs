//! The `shroom` command's door into the running app (DECISIONS B216,
//! spec N62): `control.sock` in this flavor's home, one JSON line each way.
//!
//!   → {"v":1,"op":"record.start"}       ← {"ok":true,"take":"countdown"}
//!   → {"v":1,"op":"record.stop"}        ← {"ok":true,"take":"stopping","link":"https://…"}
//!                                       ← {"ok":false,"take":"none","error":"recording_off"}
//!
//! Who may ask: this user alone — the socket lives in the home folder (700) and is 600
//! itself, and every connection's peer uid is checked against ours. What it may ask: only
//! start, pause, resume and stop, and the core answers each exactly as the click it stands
//! for, refusing all of them while General's “Let it start and stop recordings” is off.
//! The answer is a code, never words: the command says them. Nothing secret crosses here.
//!
//! `shroom setup` (spec N89) asks the same way: `setup.status`, `setup.peek`, `setup.account`
//! (`"new"`), `setup.token-page`, `setup.pick` (`"account"`), `setup.card-page`, `setup.name`
//! (`"name"`), `setup.permissions` — each the setup window's own click, with the window
//! shut — answered with where the setup stands (`"setup": {"at", …}`). And once, `setup.token`
//! with the token the command read from the clipboard: the one secret that crosses here
//! (DEVIATIONS #59, ratified 2026-10-30). It goes straight into the stash, as a paste does;
//! the core gets its handle and shape, and the run log only the shape.
//!
//! The run log has every ask and its answer, by the ask's number: the op, how a start goes
//! and who asked — the calling process's pid and program (`caller`) — then the answer's code
//! and how long the core took (`asked`, `answered`) — and any connection dropped or ask not
//! read, with why. Never the link a stop answers with.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_core::effect::CliAnswer;
use shroom_core::event::{CliOp, Event, SecretField, SetupAsk, StartAsk};
use shroom_core::world::RecId;

use super::Runtime;

/// The connections waiting on the core's answer, by the id their `Event::Cli` carries.
#[derive(Clone, Default)]
pub struct Pending {
    next: Arc<AtomicU64>,
    open: Arc<Mutex<HashMap<u64, (UnixStream, Instant)>>>,
}

/// The longest ask read: the op and its version, nothing more.
const MAX_ASK: u64 = 1024;

pub fn op_of(line: &str) -> Option<CliOp> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if v["v"] != 1 {
        return None;
    }
    match v["op"].as_str()? {
        // How a start goes: the count and the label unless the command turned one off (N62).
        "record.start" => Some(CliOp::Start(StartAsk { countdown: v["countdown"] != false, label: v["label"] != false })),
        "record.pause" => Some(CliOp::Pause),
        "record.resume" => Some(CliOp::Resume),
        "record.stop" => Some(CliOp::Stop),
        // `shroom edit` (N72): before it changes a video's edit, and once it saved one.
        "edit.check" => Some(CliOp::Editing { rec: rec_of(&v)? }),
        "edit.copy" => Some(CliOp::Copy { rec: rec_of(&v)?, rev: v["rev"].as_u64()? }),
        // `shroom setup` (N89): the setup window's clicks, with the window shut. Its token
        // is `token_of`'s, never an op's.
        "setup.status" => Some(CliOp::Setup(SetupAsk::Status)),
        "setup.peek" => Some(CliOp::Setup(SetupAsk::Peek)),
        "setup.account" => Some(CliOp::Setup(SetupAsk::Account { new: v["new"].as_bool()? })),
        "setup.token-page" => Some(CliOp::Setup(SetupAsk::TokenPage)),
        "setup.card-page" => Some(CliOp::Setup(SetupAsk::CardPage)),
        "setup.permissions" => Some(CliOp::Setup(SetupAsk::Permissions)),
        "setup.pick" => Some(CliOp::Setup(SetupAsk::Pick(account_of(&v)?))),
        "setup.name" => Some(CliOp::Setup(SetupAsk::Name(v["name"].as_str()?.chars().take(256).collect()))),
        _ => None,
    }
}

/// `setup.token`'s token (N89, DEVIATIONS #59): the text the command read, for the stash —
/// never an op, never logged.
pub fn token_of(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    if v["v"] != 1 || v["op"] != "setup.token" {
        return None;
    }
    let token = v["token"].as_str()?;
    (!token.is_empty() && token.len() <= 512).then(|| token.to_string())
}

/// An account's id as the token's verify named it: Cloudflare's hex, never anything else.
fn account_of(v: &serde_json::Value) -> Option<String> {
    let id = v["account"].as_str()?;
    ((1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then(|| id.to_string())
}

/// A video's id as an ask names it: a recording id's shape, never anything else.
fn rec_of(v: &serde_json::Value) -> Option<RecId> {
    let id = v["id"].as_str()?;
    let shaped = (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    shaped.then(|| RecId(id.to_string()))
}

/// An ask, as the run log has it: its number, the op, and how a start goes when it isn't
/// the app's default.
pub fn asked(id: u64, op: &CliOp) -> String {
    let (name, how) = match op {
        CliOp::Start(a) => ("record.start", [(!a.countdown).then_some(" no-count"), (!a.label).then_some(" no-label")].into_iter().flatten().collect::<String>()),
        CliOp::Pause => ("record.pause", String::new()),
        CliOp::Resume => ("record.resume", String::new()),
        CliOp::Stop => ("record.stop", String::new()),
        CliOp::Editing { rec } => ("edit.check", format!(" {}", super::short(&rec.0))),
        CliOp::Copy { rec, rev } => ("edit.copy", format!(" {} rev {rev}", super::short(&rec.0))),
        CliOp::Setup(ask) => match ask {
            SetupAsk::Status => ("setup.status", String::new()),
            SetupAsk::Peek => ("setup.peek", String::new()),
            SetupAsk::Account { new } => ("setup.account", if *new { " new" } else { " have" }.into()),
            SetupAsk::TokenPage => ("setup.token-page", String::new()),
            // The token's shape only (law 13).
            SetupAsk::Token { shape, .. } => ("setup.token", format!(" {shape:?}").to_lowercase()),
            SetupAsk::Pick(_) => ("setup.pick", String::new()),
            SetupAsk::CardPage => ("setup.card-page", String::new()),
            SetupAsk::Name(n) => ("setup.name", format!(" ({} chars)", n.chars().count())),
            SetupAsk::Permissions => ("setup.permissions", String::new()),
        },
    };
    format!("control: ask {id} {name}{how}")
}

/// An answer, as the run log has it: its ask's number, ok or no, the take or the refusal's
/// code, and how long it took — never the link.
pub fn answered(a: &CliAnswer, took: Duration) -> String {
    // A setup ask's answer is where the setup stands (N89).
    let face = a.setup.as_ref().map_or(a.take, |s| s.at);
    format!("control: answer {} {} {} ({} ms)", a.id, if a.ok { "ok" } else { "no" }, a.error.unwrap_or(face), took.as_millis())
}

pub fn line_of(a: &CliAnswer) -> String {
    let mut v = serde_json::json!({"ok": a.ok, "take": a.take});
    if let Some(e) = a.error {
        v["error"] = e.into();
    }
    if let Some(l) = &a.link {
        v["link"] = l.clone().into();
    }
    if let Some(r) = &a.setup {
        let accounts: Vec<_> = r.accounts.iter().map(|x| serde_json::json!({"id": x.id, "name": x.name})).collect();
        let mut s = serde_json::json!({"at": r.at, "took": r.took, "busy": r.busy, "signed_up": r.signed_up, "accounts": accounts});
        if let Some(h) = &r.host {
            s["host"] = h.clone().into();
        }
        if let Some(n) = r.videos {
            s["videos"] = n.into();
        }
        v["setup"] = s;
    }
    format!("{v}\n")
}

/// Whether the other end of `s` runs as this user.
fn same_user(s: &UnixStream) -> bool {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: a connected socket's descriptor and two out-parameters the call fills.
    let ok = unsafe { libc::getpeereid(s.as_raw_fd(), &mut uid, &mut gid) } == 0;
    // SAFETY: getuid has no failure mode.
    ok && uid == unsafe { libc::getuid() }
}

/// The calling process as macOS has it: its pid (`LOCAL_PEERPID`) and the program it runs
/// (`proc_pidpath`; empty when that can't be read). None: no pid.
fn peer(s: &UnixStream) -> Option<(i32, String)> {
    let mut pid: libc::pid_t = 0;
    let mut len = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
    // SAFETY: a connected socket's descriptor, and an out-parameter of the size given.
    let ok = unsafe { libc::getsockopt(s.as_raw_fd(), libc::SOL_LOCAL, libc::LOCAL_PEERPID, (&mut pid as *mut libc::pid_t).cast(), &mut len) } == 0;
    if !ok || pid <= 0 {
        return None;
    }
    let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: a buffer of the size given, which the call fills and measures.
    let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    buf.truncate(n.max(0) as usize);
    Some((pid, String::from_utf8_lossy(&buf).into_owned()))
}

/// Who asked, as the run log has it beside the ask: the pid and the program's path.
pub fn caller(peer: Option<(i32, String)>) -> String {
    match peer {
        Some((pid, path)) if path.is_empty() => format!("from pid {pid} (no path)"),
        Some((pid, path)) => format!("from pid {pid} {path}"),
        None => "from an unknown process".into(),
    }
}

/// Listens on `control.sock` for as long as the app runs. A socket another run left
/// behind is replaced; one that can't be made is logged and the app runs without it.
pub fn spawn(rt: Runtime, pending: Pending) {
    let path = rt.home.path("control.sock");
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            rt.log(&format!("control: no socket ({e}) — the shroom command can't reach this app"));
            return;
        }
    };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    rt.log("control: listening for the shroom command");
    let _ = std::thread::Builder::new().name("shroom-control".into()).spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            if !same_user(&conn) {
                rt.log("control: a connection from another user, dropped");
                continue;
            }
            let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = conn.set_write_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let read = conn.try_clone().map(|c| BufReader::new(c.take(MAX_ASK)).read_line(&mut line));
            let read = read.ok().and_then(|r| r.ok()).is_some();
            // The command's token (N89): into the stash, where a paste lands; the line that
            // carried it goes with this turn.
            let token = read.then(|| token_of(&line)).flatten().map(|text| {
                let (id, shape) = rt.stash.lock().unwrap().put(SecretField::Token, text, true);
                CliOp::Setup(SetupAsk::Token { id, shape })
            });
            let Some(op) = token.or_else(|| read.then(|| op_of(&line)).flatten()) else {
                rt.log(&format!("control: an ask it couldn't read ({} bytes), answered usage", line.len()));
                let _ = conn.write_all(b"{\"ok\":false,\"take\":\"none\",\"error\":\"usage\"}\n");
                continue;
            };
            // Start is the popover's Start: the grants as macOS has them now go first, so
            // the core arms on the truth (commands::arms) — and so does a setup's look at
            // where it stands, and its permissions step (N89).
            if matches!(op, CliOp::Start(_) | CliOp::Setup(SetupAsk::Status | SetupAsk::Permissions)) {
                rt.post(Event::Os(super::os::grants()));
            }
            let id = pending.next.fetch_add(1, Ordering::SeqCst) + 1;
            let from = caller(peer(&conn));
            pending.open.lock().unwrap().insert(id, (conn, Instant::now()));
            rt.log(&format!("{} {from}", asked(id, &op)));
            rt.post(Event::Cli { id, op });
        }
    });
}

/// The core's answer, onto the connection that asked; then it closes.
pub fn answer(rt: &Runtime, pending: &Pending, a: CliAnswer) {
    let Some((mut conn, at)) = pending.open.lock().unwrap().remove(&a.id) else { return };
    rt.log(&answered(&a, at.elapsed()));
    if conn.write_all(line_of(&a).as_bytes()).is_err() {
        rt.log(&format!("control: answer {} not delivered: the command had gone", a.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setups_asks_are_read_and_its_token_is_never_an_op() {
        assert_eq!(op_of(r#"{"v":1,"op":"setup.status"}"#), Some(CliOp::Setup(SetupAsk::Status)));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.peek"}"#), Some(CliOp::Setup(SetupAsk::Peek)));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.account","new":true}"#), Some(CliOp::Setup(SetupAsk::Account { new: true })));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.account"}"#), None, "new or have, said");
        assert_eq!(op_of(r#"{"v":1,"op":"setup.token-page"}"#), Some(CliOp::Setup(SetupAsk::TokenPage)));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.card-page"}"#), Some(CliOp::Setup(SetupAsk::CardPage)));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.permissions"}"#), Some(CliOp::Setup(SetupAsk::Permissions)));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.pick","account":"0123abcd"}"#), Some(CliOp::Setup(SetupAsk::Pick("0123abcd".into()))));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.pick","account":"a b"}"#), None, "an account id's shape only");
        assert_eq!(op_of(r#"{"v":1,"op":"setup.name","name":"myshroom"}"#), Some(CliOp::Setup(SetupAsk::Name("myshroom".into()))));
        assert_eq!(op_of(r#"{"v":1,"op":"setup.token","token":"cfat_abc"}"#), None, "the token is token_of's, never an op");
        assert_eq!(token_of(r#"{"v":1,"op":"setup.token","token":"cfat_abc"}"#).as_deref(), Some("cfat_abc"));
        assert_eq!(token_of(r#"{"v":1,"op":"setup.status"}"#), None);
        assert_eq!(token_of(r#"{"v":2,"op":"setup.token","token":"cfat_abc"}"#), None);
        let tok = CliOp::Setup(SetupAsk::Token { id: shroom_core::world::SecretId(4), shape: shroom_core::event::Shape::Token });
        assert_eq!(asked(9, &tok), "control: ask 9 setup.token token", "the log has its shape only");
        assert_eq!(asked(9, &CliOp::Setup(SetupAsk::Name("myshroom".into()))), "control: ask 9 setup.name (8 chars)");
    }

    #[test]
    fn a_setup_answer_says_where_it_stands() {
        use shroom_core::effect::SetupReport;
        let report = SetupReport { at: "space-found", took: false, busy: true, host: Some("v.myshroom.workers.dev".into()), videos: Some(14), ..SetupReport::default() };
        let a = CliAnswer { id: 3, ok: true, take: "none", error: None, link: None, setup: Some(report) };
        let v: serde_json::Value = serde_json::from_str(&line_of(&a)).unwrap();
        assert_eq!(v["setup"]["at"], "space-found");
        assert_eq!(v["setup"]["host"], "v.myshroom.workers.dev");
        assert_eq!(v["setup"]["videos"], 14);
        assert_eq!(v["setup"]["busy"], true);
        assert_eq!(answered(&a, Duration::from_millis(5)), "control: answer 3 ok space-found (5 ms)");
    }

    #[test]
    fn only_the_ops_of_this_version_are_read() {
        assert_eq!(op_of(r#"{"v":1,"op":"record.start"}"#), Some(CliOp::START));
        // The command's flags (N62, B216): each only when the person or agent turned it off.
        assert_eq!(op_of(r#"{"v":1,"op":"record.start","countdown":false,"label":false}"#), Some(CliOp::Start(StartAsk { countdown: false, label: false })));
        assert_eq!(op_of(r#"{"v":1,"op":"record.start","label":false}"#), Some(CliOp::Start(StartAsk { countdown: true, label: false })));
        assert_eq!(op_of(r#"{"v":1,"op":"record.stop"}"#), Some(CliOp::Stop));
        assert_eq!(op_of(r#"{"v":2,"op":"record.start"}"#), None);
        assert_eq!(op_of(r#"{"v":1,"op":"delete"}"#), None);
        // `shroom edit`'s (N72): a recording id's shape, and a revision.
        let rec = || RecId("AbCdEfGhIjKlMnOp".into());
        assert_eq!(op_of(r#"{"v":1,"op":"edit.check","id":"AbCdEfGhIjKlMnOp"}"#), Some(CliOp::Editing { rec: rec() }));
        assert_eq!(op_of(r#"{"v":1,"op":"edit.copy","id":"AbCdEfGhIjKlMnOp","rev":3}"#), Some(CliOp::Copy { rec: rec(), rev: 3 }));
        assert_eq!(op_of(r#"{"v":1,"op":"edit.copy","id":"AbCdEfGhIjKlMnOp"}"#), None, "no revision");
        assert_eq!(op_of(r#"{"v":1,"op":"edit.check","id":"../space.json"}"#), None, "not an id");
        assert_eq!(op_of(r#"{"v":1,"op":"edit.check","id":""}"#), None);
        assert_eq!(op_of("not json"), None);
    }

    /// What the run log keeps of an ask and its answer (Bagrat, 2026-10-18: enough to report
    /// a problem): the op, how a start goes, the code, the time — never the link.
    #[test]
    fn the_log_has_each_ask_and_its_answer_by_number() {
        assert_eq!(asked(3, &CliOp::START), "control: ask 3 record.start");
        assert_eq!(asked(4, &CliOp::Start(StartAsk { countdown: false, label: false })), "control: ask 4 record.start no-count no-label");
        assert_eq!(asked(5, &CliOp::Stop), "control: ask 5 record.stop");
        assert_eq!(asked(7, &CliOp::Copy { rec: RecId("AbCdEfGhIjKlMnOp".into()), rev: 2 }), "control: ask 7 edit.copy AbCd… rev 2", "the id short");
        let ok = CliAnswer { id: 5, ok: true, take: "stopping", error: None, link: Some("https://v.x.workers.dev/AbCdEfGhIjKlMnOp".into()), setup: None };
        assert_eq!(answered(&ok, Duration::from_millis(41)), "control: answer 5 ok stopping (41 ms)");
        let no = CliAnswer { id: 6, ok: false, take: "none", error: Some("recording_off"), link: None, setup: None };
        assert_eq!(answered(&no, Duration::from_millis(3)), "control: answer 6 no recording_off (3 ms)");
        for l in [asked(4, &CliOp::Start(StartAsk { countdown: false, label: true })), answered(&ok, Duration::ZERO)] {
            assert_eq!(shroom_core::report::clean_line(&l), l, "nothing in it for the report to take out");
        }
    }

    /// Beside each ask, the run log has who asked: the calling process's pid and the program
    /// it runs, as macOS has them — so a recording the person didn't start says what did.
    #[test]
    fn the_log_says_which_process_asked() {
        assert_eq!(caller(Some((812, "/Applications/shroom.app/Contents/MacOS/shroom-cli".into()))), "from pid 812 /Applications/shroom.app/Contents/MacOS/shroom-cli");
        assert_eq!(caller(Some((812, String::new()))), "from pid 812 (no path)");
        assert_eq!(caller(None), "from an unknown process");
        let line = format!("{} {}", asked(4, &CliOp::Start(StartAsk { countdown: false, label: false })), caller(Some((812, "/usr/local/bin/shroom".into()))));
        assert_eq!(line, "control: ask 4 record.start no-count no-label from pid 812 /usr/local/bin/shroom");
        assert_eq!(shroom_core::report::clean_line(&line), line, "nothing in it for the report to take out");
        // The real call, on a connection to a listening socket: this process, and its own
        // program.
        let tmp = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(tmp.path().join("c.sock")).unwrap();
        let _client = UnixStream::connect(tmp.path().join("c.sock")).unwrap();
        let (conn, _) = listener.accept().unwrap();
        let (pid, path) = peer(&conn).expect("the peer's pid");
        assert_eq!(pid, std::process::id() as i32);
        let exe = std::env::current_exe().unwrap();
        assert!(path.ends_with(&*exe.file_name().unwrap().to_string_lossy()), "{path} is {}", exe.display());
    }

    #[test]
    fn an_answer_is_a_code_and_a_link_never_words() {
        let a = CliAnswer { id: 3, ok: true, take: "stopping", error: None, link: Some("https://v.x.workers.dev/AbCdEfGhIjKlMnOp".into()), setup: None };
        assert_eq!(line_of(&a), "{\"link\":\"https://v.x.workers.dev/AbCdEfGhIjKlMnOp\",\"ok\":true,\"take\":\"stopping\"}\n");
        let no = CliAnswer { id: 4, ok: false, take: "none", error: Some("recording_off"), link: None, setup: None };
        assert_eq!(line_of(&no), "{\"error\":\"recording_off\",\"ok\":false,\"take\":\"none\"}\n");
    }
}
