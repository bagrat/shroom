//! Report a problem and Copy diagnostics (spec N53, DECISIONS B180): the shell's half. It
//! gathers the report — this Mac's line, the setup's times, the log's last lines, cleaned by
//! `shroom_core::report` — copies it, or posts it to shroom.am/report, which emails it to
//! the person who makes shroom and keeps nothing. Each runs off the core thread, only ever
//! from a click (law 10), and answers once. A dev build never reaches shroom.am.

use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use shroom_core::event::{Event, ReportNews};
use shroom_core::report::{self, Diagnostics, Post};
#[cfg(test)]
use shroom_core::report::Kind;

use super::Runtime;

/// Where a release posts (site/report.js).
pub const ENDPOINT: &str = "https://shroom.am/report";
/// The Worker's bounds (site/report.js): the log and the setup line, in characters.
const LOG_CHARS: usize = 120_000;
const SETUP_CHARS: usize = 300;

/// The report, gathered: never fails — a fact it can't read is left out.
pub fn gather(rt: &Runtime, role: &str) -> Diagnostics {
    Diagnostics {
        // An early-channel Mac says so beside its version (B220): it may run a pre-release.
        shroom: match super::updater::mac_channel(rt) {
            shroom_update::Channel::Early => format!("{} (early channel)", rt.version),
            shroom_update::Channel::Official => rt.version.clone(),
        },
        macos: command("sw_vers", &["-productVersion"]).unwrap_or_default(),
        chip: command("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default(),
        memory: command("sysctl", &["-n", "hw.memsize"]).and_then(|b| b.parse::<u64>().ok()).map(|b| format!("{} GB", b / (1 << 30))).unwrap_or_default(),
        setup: setup(&rt.home.root),
        role: role.to_string(),
        log: report::clean_log(&log_tail(&rt.home.root.join("logs"))),
    }
}

/// The report for the sheet: its disclosure, and what Send posts.
pub fn gather_for_sheet(rt: &Runtime, role: String) {
    let rt = rt.clone();
    std::thread::spawn(move || {
        let d = gather(&rt, &role);
        rt.log(&format!("report: gathered ({} log lines)", d.log.lines().count()));
        rt.post(Event::Report(ReportNews::Gathered(d)));
    });
}

/// Copy diagnostics: the report onto the clipboard (a drive's own pasteboard in a harness).
pub fn copy(rt: &Runtime, role: String) {
    let rt = rt.clone();
    std::thread::spawn(move || {
        let d = gather(&rt, &role);
        rt.log("shell: copied the diagnostics");
        super::os::put_on_pasteboard(&rt, d.text());
    });
}

/// Send: one POST to shroom.am/report. A dev build never reaches it: a harness run takes
/// it as sent (its drives walk on), any other dev build as not sent.
pub fn send(rt: &Runtime, post: Post) {
    let rt = rt.clone();
    std::thread::spawn(move || {
        let body = body(&post).to_string();
        let (sent, not_sent) = (ReportNews::Sent(post.kind), ReportNews::NotSent(post.kind));
        let news = if rt.dev {
            rt.log(&format!("report: a dev build would send {} bytes", body.len()));
            if rt.harness { sent.clone() } else { not_sent }
        } else {
            match post_json(ENDPOINT, &body) {
                Ok(200) => sent.clone(),
                Ok(status) => {
                    rt.log(&format!("report: not taken ({status})"));
                    not_sent
                }
                Err(e) => {
                    rt.log(&format!("report: didn't go through ({e})"));
                    not_sent
                }
            }
        };
        if news == sent {
            rt.log("report: sent");
        }
        rt.post(Event::Report(news));
    });
}

/// The JSON site/report.js takes: kind, from, the words, the reply-to, the versions, the
/// setup line (with this Mac's role) and the cleaned log, each within its bound. Feedback
/// sent without the log (N60) carries the version alone: no setup, no log.
pub fn body(p: &Post) -> serde_json::Value {
    let d = &p.diagnostics;
    let setup = [d.setup.as_str(), d.role.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    let versions: serde_json::Map<String, serde_json::Value> = [("shroom", &d.shroom), ("macos", &d.macos), ("chip", &d.chip), ("memory", &d.memory)]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.clone())))
        .collect();
    let mut v = serde_json::json!({
        "kind": p.kind.as_str(),
        "from": "app",
        "words": p.words.chars().take(shroom_core::copy::REPORT_WORDS_MAX).collect::<String>(),
        "versions": versions,
    });
    if !setup.is_empty() {
        v["setup"] = serde_json::Value::String(setup.chars().take(SETUP_CHARS).collect());
    }
    if !d.log.is_empty() {
        v["log"] = serde_json::Value::String(bounded_log(&d.log));
    }
    if let Some(r) = &p.reply_to {
        v["reply_to"] = serde_json::Value::String(r.chars().take(254).collect());
    }
    v
}

/// The newest lines that fit the Worker's bound.
fn bounded_log(log: &str) -> String {
    let mut lines: Vec<&str> = log.lines().collect();
    while lines.iter().map(|l| l.chars().count() + 1).sum::<usize>() > LOG_CHARS && !lines.is_empty() {
        lines.remove(0);
    }
    lines.join("\n")
}

fn post_json(url: &str, body: &str) -> Result<u16, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .build()
        .into();
    let res = agent.post(url).header("content-type", "application/json").send(body).map_err(|e| e.to_string())?;
    Ok(res.status().as_u16())
}

/// One line of a system command's output, trimmed.
fn command(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).stdin(std::process::Stdio::null()).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !s.is_empty()).then_some(s)
}

/// The log's last lines: `run.log`, reaching into `run.log.1` when the current one is short.
fn log_tail(logs: &Path) -> String {
    let read = |name: &str| std::fs::read(logs.join(name)).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let now = read("run.log");
    if now.lines().filter(|l| !l.trim().is_empty()).count() >= report::LOG_LINES {
        return now;
    }
    format!("{}\n{now}", read("run.log.1"))
}

/// When each setup step landed on this Mac, from its files' times, oldest first:
/// “2026-10-12 10:06 UTC key · 10:14 recording allowed · 10:15 wired”.
fn setup(root: &Path) -> String {
    let mut steps: Vec<(i64, &str)> = [("token.json", "key"), ("primed.json", "recording allowed"), ("space.json", "wired")]
        .into_iter()
        .filter_map(|(file, step)| {
            let t = std::fs::metadata(root.join(file)).and_then(|m| m.modified()).ok()?;
            Some((t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64, step))
        })
        .collect();
    steps.sort();
    let mut day = String::new();
    let parts: Vec<String> = steps
        .into_iter()
        .map(|(t, step)| {
            let (date, time) = utc(t);
            if date == day {
                format!("{time} {step}")
            } else {
                day = date.clone();
                format!("{date} {time} UTC {step}")
            }
        })
        .collect();
    parts.join(" · ")
}

/// A time as UTC's date and hour:minute.
pub fn utc(secs: i64) -> (String, String) {
    let tm = gmtime(secs);
    (format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday), format!("{:02}:{:02}", tm.tm_hour, tm.tm_min))
}

fn gmtime(secs: i64) -> libc::tm {
    let t: libc::time_t = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: gmtime_r writes only into `tm`.
    unsafe { libc::gmtime_r(&t, &mut tm) };
    tm
}

/// A log line's wall clock, UTC to the second: “2026-10-18T17:52:03Z”.
pub fn stamp(secs: i64) -> String {
    let tm = gmtime(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec)
}

/// Which build runs on which Mac, for the log's boot and roll lines:
/// “shroom 0.1.17 · macOS 15.5 · Apple M2” (a fact it can't read is left out).
pub fn build(version: &str) -> String {
    let macos = command("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}"));
    let chip = command("sysctl", &["-n", "machdep.cpu.brand_string"]);
    std::iter::once(format!("shroom {version}")).chain(macos).chain(chip).collect::<Vec<_>>().join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(log: String) -> Post {
        Post {
            kind: Kind::Problem,
            words: "It stopped.".into(),
            reply_to: Some("you@example.com".into()),
            diagnostics: Diagnostics {
                shroom: "0.1.18".into(),
                macos: "15.3.1".into(),
                chip: "Apple M2".into(),
                memory: "16 GB".into(),
                setup: "2026-10-12 10:06 UTC key".into(),
                role: "owns its space".into(),
                log,
            },
        }
    }

    /// The body is what site/report.js takes: known fields only, each within its bound.
    #[test]
    fn the_body_is_the_endpoints_shape() {
        let v = body(&post("  1 boot: launch login".into()));
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["from", "kind", "log", "reply_to", "setup", "versions", "words"]);
        assert_eq!(v["kind"], "problem");
        assert_eq!(v["from"], "app");
        assert_eq!(v["setup"], "2026-10-12 10:06 UTC key · owns its space");
        assert_eq!(v["versions"]["macos"], "15.3.1");
        let v = body(&Post { reply_to: None, ..post(String::new()) });
        assert!(v.get("reply_to").is_none());
    }

    /// Send feedback (N60) posts as feedback; without the log, the version alone.
    #[test]
    fn feedback_without_the_log_carries_the_version_alone() {
        let v = body(&Post { kind: Kind::Feedback, words: "Love it.".into(), reply_to: None, diagnostics: Diagnostics::version_only("0.1.18") });
        assert_eq!(v, serde_json::json!({ "kind": "feedback", "from": "app", "words": "Love it.", "versions": { "shroom": "0.1.18" } }));
        let v = body(&Post { kind: Kind::Feedback, ..post("  1 boot: launch login".into()) });
        assert_eq!((v["kind"].as_str(), v["log"].as_str()), (Some("feedback"), Some("  1 boot: launch login")));
    }

    #[test]
    fn a_long_log_keeps_its_newest_lines_within_the_bound() {
        let log = (0..400).map(|i| format!("{i:>6} {}", "x".repeat(390))).collect::<Vec<_>>().join("\n");
        let v = body(&post(log));
        let kept = v["log"].as_str().unwrap();
        assert!(kept.chars().count() <= LOG_CHARS);
        // The Worker takes as much of the log as the app sends.
        assert!(include_str!("../../../../site/report.js").contains(&format!("log: {LOG_CHARS},")));
        assert!(kept.ends_with(&"x".repeat(10)) && kept.lines().last().unwrap().starts_with("   399"));
        assert!(!kept.starts_with("     0"), "the oldest lines go first");
    }

    /// An early-channel Mac's report says so beside its version (B220).
    #[test]
    fn the_report_names_an_early_channel() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut rt, _rx) = super::super::Runtime::for_test(tmp.path().join("h"), None, None);
        rt.version = "0.1.25".into();
        assert_eq!(gather(&rt, "none").shroom, "0.1.25");
        std::fs::write(rt.home.path("channel"), "early\n").unwrap();
        assert_eq!(gather(&rt, "none").shroom, "0.1.25 (early channel)");
        assert!(gather(&rt, "none").text().starts_with("shroom 0.1.25 (early channel)"));
    }

    #[test]
    fn utc_reads_the_calendar() {
        assert_eq!(utc(1_792_150_000), ("2026-10-16".to_string(), "11:26".to_string()));
        assert_eq!(stamp(1_792_150_003), "2026-10-16T11:26:43Z");
    }
}
