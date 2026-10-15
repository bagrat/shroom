//! The L1 fake as a process, for driving the real app (whose wizard points at it with the
//! dev-only `--cloud-fake <api> <r2>`):
//!
//!   cloudfake [--state fresh|taken:<name>|email-gate|card-gate|found]... [--relay-parent <url>]
//!
//! `--relay-parent <http://127.0.0.1:port>`: an `R2_PARENT` placed on a Worker reaches the
//! page stand-in's binding (`worker/dev/serve.mjs`, DECISIONS B90).
//!
//! Prints one JSON line `{"api":…,"r2":…}`, then serves until stdin closes. States
//! combine (`--state taken:myshroom --state card-gate`); no `--state` is `fresh`.
//!
//! While it serves, stdin takes one command per line — what a person does in the
//! dashboard meanwhile:
//!   clear-gates   the email is verified and R2 is on (both gates clear)
//!   calls         prints every request so far to stderr, one per line
//!   email-gate · card-gate · drop-bucket <name> · restore-bucket <name>
//!                 a gate goes up again; the bucket is deleted (with everything in it)
//!                 or put back — also `POST /_fake/<command>` on the fake's own port

use std::io::{BufRead, Write};

use serde_json::json;
use shroom_cloudfake::{Fake, State};

/// What a wired space leaves behind (shroom-cloud's `wire::BUCKET`, label `v`).
const BUCKET: &str = "shroom-space";

fn apply(state: &mut State, what: &str) -> Result<(), String> {
    match what {
        "fresh" => {}
        "email-gate" => state.email_verified = false,
        "card-gate" => state.r2_enabled = false,
        "found" => {
            state.subdomain = Some("myshroom".into());
            state.buckets.entry(BUCKET.into()).or_default();
            state.put_object(BUCKET, ".shroom/manifest.json", br#"{"schema":1,"label":"v"}"#);
            state.put_object(BUCKET, ".shroom/cookie.key", b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFG");
            state.put_object(BUCKET, "meta/AbCdEfGhIjKlMnOp.json", b"{}");
            state.scripts.insert("v".into(), vec![json!({"type": "r2_bucket", "name": "BUCKET", "bucket_name": BUCKET})]);
            state.routes.insert("v".into());
            // The page this app ships, already up: a token Mac's launch finds it current.
            if let Some(v) = shroom_cloudfake::module_version(include_str!("../../../../worker/src/worker.js").as_bytes()) {
                state.versions.insert("v".into(), v);
            }
        }
        other => match other.strip_prefix("taken:") {
            Some(name) if !name.is_empty() => {
                state.taken.insert(name.into());
            }
            _ => return Err(format!("unknown state {other:?}")),
        },
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut state = State::fresh();
    let mut i = 0;
    while i < args.len() {
        match (args[i].as_str(), args.get(i + 1)) {
            ("--relay-parent", Some(url)) => {
                if let Err(e) = state.control(&format!("relay-parent {url}")) {
                    eprintln!("cloudfake: {e}");
                    std::process::exit(2);
                }
                i += 2;
            }
            ("--state", Some(what)) => {
                if let Err(e) = apply(&mut state, what) {
                    eprintln!("cloudfake: {e}");
                    std::process::exit(2);
                }
                i += 2;
            }
            _ => {
                eprintln!("usage: cloudfake [--state fresh|taken:<name>|email-gate|card-gate|found]... [--relay-parent <url>]");
                std::process::exit(2);
            }
        }
    }
    let fake = Fake::start(state);
    let mut out = std::io::stdout();
    let _ = writeln!(out, "{}", json!({"api": fake.api(), "r2": fake.r2()}));
    let _ = out.flush();
    // Serve until whoever started us closes our stdin; its lines are commands.
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        match line.trim() {
            "clear-gates" => {
                fake.with(|s| {
                    s.email_verified = true;
                    s.r2_enabled = true;
                });
                eprintln!("cloudfake: gates cleared");
            }
            "calls" => fake.with(|s| s.calls.iter().for_each(|c| eprintln!("cloudfake call: {c}"))),
            "" => {}
            other => match fake.with(|s| s.control(other)) {
                Ok(()) => eprintln!("cloudfake: {other}"),
                Err(e) => eprintln!("cloudfake: {e}"),
            },
        }
    }
}
