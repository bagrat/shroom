//! Law 15's copy lint (TESTING-PLAN §2.4) over every line shroom ships outside a Scene:
//! the core's copy registry, the Worker's `COPY` and its templated lines, and the
//! installer's said-lines and header comment block (N11: ratifiable copy). Every Scene
//! string is linted where Scenes are made — at every walkthrough step (`laws::check`) and
//! over every inventory row (`laws::voice_failures`). Law 16's figures rule runs there too.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::path::PathBuf;

fn repo(path: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn lint(lines: impl IntoIterator<Item = (String, String)>) -> Vec<String> {
    lines.into_iter().filter_map(|(at, text)| laws::voice(&text).map(|why| format!("{at}: “{text}” {why}"))).collect()
}

/// The spec's example arguments for a template's holes.
fn fill(template: &str) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else { break };
        let hole = &rest[i + 2..i + j];
        out.push_str(match hole {
            h if h.contains("pct") => "40",
            h if h.contains("link") => "v.myshroom.workers.dev/join/7xk2pm",
            h if h.contains("host") => "v.myshroom.workers.dev",
            _ => "Bagrat",
        });
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

/// The Worker's said-lines: every string value of `export const COPY = { … };`, and every
/// one-line template export (`export const x = (…) => \`…\`;`) with its holes filled.
fn worker_lines() -> Vec<(String, String)> {
    let src = repo("worker/src/worker.js");
    let start = src.find("export const COPY = {").expect("worker.js exports COPY");
    let body = &src[start..start + src[start..].find("\n};").expect("COPY ends")];
    let mut out = Vec::new();
    for line in body.lines().skip(1) {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        if let Some((key, value)) = line.split_once(": \"") {
            let text = value.trim_end_matches(',').trim_end_matches('"');
            out.push((format!("worker COPY.{key}"), text.to_string()));
        }
    }
    for line in src.lines() {
        if let (Some(name), Some(t)) = (line.strip_prefix("export const "), line.split_once("=> `")) {
            if let Some(template) = t.1.strip_suffix("`;") {
                out.push((format!("worker {}", name.split(' ').next().unwrap_or(name)), fill(template)));
            }
        }
    }
    out
}

/// The installer's said-lines: the header comment block (ratifiable copy, N11 — the inline
/// comments below it are exempt), the `NAME="…"` lines it prints, and every literal it
/// `say`s or `fail`s with.
fn installer_lines() -> Vec<(String, String)> {
    let src = repo("installer/install.sh");
    let mut out = Vec::new();
    let header: Vec<&str> = src.lines().skip(1).take_while(|l| l.starts_with('#')).map(|l| l.trim_start_matches('#').trim()).collect();
    out.push(("install.sh header".to_string(), header.join(" ")));
    let quoted = |s: &str| -> Vec<String> {
        s.split('"').skip(1).step_by(2).map(|q| q.replace("$MIN_MACOS", "14").replace("$floor", "14").replace("$macos", "13.6").replace("$version", "1.3.0")).collect()
    };
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            continue;
        }
        let is_line = t.split_once("=\"").is_some_and(|(name, _)| !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c == '_'));
        if is_line && !t.starts_with("RELEASES=") && !t.starts_with("APP=") {
            out.extend(quoted(t).into_iter().take(1).map(|q| ("install.sh".to_string(), q)));
        }
        for verb in ["say \"", "fail \""] {
            if let Some(i) = t.find(verb) {
                out.extend(quoted(&t[i..]).into_iter().filter(|q| !q.starts_with('$')).map(|q| ("install.sh".to_string(), q)));
            }
        }
    }
    out
}

#[test]
fn every_line_in_the_core_registry_speaks_product() {
    let lines = shroom_core::copy::registry().into_iter().map(|(state, text)| (format!("copy.rs ({state})"), text));
    let bad = lint(lines);
    assert!(bad.is_empty(), "law 15:\n{}", bad.join("\n"));
}

#[test]
fn every_worker_line_speaks_product() {
    let lines = worker_lines();
    assert!(lines.len() >= 40, "found only {} Worker lines — has COPY's shape changed?", lines.len());
    assert!(lines.iter().any(|(_, t)| t == "Waiting for connection — your link already works."));
    assert!(lines.iter().any(|(_, t)| t == "Bagrat invited you to record with shroom."));
    let bad = lint(lines);
    assert!(bad.is_empty(), "law 15:\n{}", bad.join("\n"));
}

#[test]
fn every_installer_line_speaks_product() {
    let lines = installer_lines();
    assert!(lines.len() >= 10, "found only {} installer lines — has install.sh's shape changed?", lines.len());
    assert!(lines.iter().any(|(_, t)| t == "Getting shroom…"));
    assert!(lines.iter().any(|(at, t)| at == "install.sh header" && t.contains("published fingerprint")));
    let bad = lint(lines);
    assert!(bad.is_empty(), "law 15:\n{}", bad.join("\n"));
}

/// The lint itself: what it catches, and what the spec says is product voice.
#[test]
fn the_lint_catches_internals_and_codes_and_spares_the_product() {
    for bad in [
        "Couldn't reach the Worker.",
        "The bucket is gone.",
        "Your R2 storage is full.",
        "Your creds expired.",
        "Saving credentials…",
        "Error 10031",
        "Something failed (8000077).",
        "Cloudflare said #403",
        "The HLS playlist is missing.",
        "ffmpeg exited",
        "Create an API token",
        "Your subdomain is taken.",
        "The server said 403 Forbidden",
        "Error 502 occurred",
        "Got 500 from the space",
        "Upload failed: 413 Payload Too Large",
        "status 404 videos",
        "Something went wrong (code 1001 hours)",
        "Your space answered 503.",
        "It said Forbidden.",
    ] {
        assert!(laws::voice(bad).is_some(), "the lint missed “{bad}”");
    }
    for good in [
        "One access token for shroom",
        "A Cloudflare account",
        "v.myshroom.workers.dev",
        "https://v.myshroom.workers.dev/AbCdEfGhIjKlMnOp",
        "Paused — 4:32",
        "shroom 1.3.0 is ready.",
        "Uploading — 40%. Your link already works.",
        "about 30 hours for $1.50 a month.",
        "Found your space — v.myshroom.workers.dev, 14 videos. This Mac is joined.",
        "For macOS 14 or newer, Apple silicon and Intel.",
        "140 videos in your space",
        "About 500 MB left — enough for about 2 minutes.",
        "1000 hours of video",
    ] {
        assert_eq!(laws::voice(good), None, "the lint flags product voice “{good}”");
    }
    let _ = support::quick;
}
