//! What a report carries (spec N53, DECISIONS B180): the Mac's line, the setup timeline and
//! the last lines of shroom's log, each one cleaned — the same rules as the feedback page's
//! (`site/clean.js`), held to the same cases (`site/test/clean-cases.json`). The log never
//! holds a secret (law 13), but it does hold values: a space's host, paths in the home
//! folder, and error text from macOS and the network. Pure: the shell reads the files and
//! the system, and hands the raw text here.

use serde::Serialize;

/// The lines a report keeps: the last ones, the newest last.
pub const LOG_LINES: usize = 1000;
/// Of those, at most this many are the `shroom` command's own (its `cli.log`, spec N62):
/// its last runs, among the app's lines by their time (`clean_logs`).
pub const CLI_LINES: usize = 30;
/// One line, at most, once cleaned (in characters).
pub const LINE_CHARS: usize = 400;
/// A window's own error (`webview <surface>: <message>`): its message is cut to this.
pub const WEBVIEW_CHARS: usize = 160;

/// Hosts that name no one: shroom's own and the services every Mac running it talks to.
const PUBLIC_HOSTS: &[&str] = &[
    "shroom.am",
    "github.com",
    "api.github.com",
    "objects.githubusercontent.com",
    "dash.cloudflare.com",
    "api.cloudflare.com",
    "www.cloudflare.com",
    "cloudflare.com",
];
/// The endings a bare host in the log can have; a file name (`run.log`, `worker.js`) has
/// none of them.
const TLDS: &[&str] = &[
    "com", "net", "org", "dev", "app", "io", "am", "co", "me", "ai", "sh", "xyz", "site", "cloud", "tech", "page", "link", "eu", "uk", "de", "fr", "nl", "us",
    "ca", "au",
];

/// The report gathered from this Mac: what the disclosure shows, Copy diagnostics copies,
/// and Send posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostics {
    pub shroom: String,
    pub macos: String,
    pub chip: String,
    pub memory: String,
    /// When each setup step landed on this Mac, or "".
    pub setup: String,
    /// This Mac's place in its space, in words: owns it, a member's, or none yet.
    pub role: String,
    /// The last `LOG_LINES` lines of the log, cleaned (`clean_log`).
    pub log: String,
}

impl Diagnostics {
    /// The version alone: what Send feedback carries with its log box unticked (N60).
    pub fn version_only(version: &str) -> Diagnostics {
        Diagnostics { shroom: version.into(), macos: String::new(), chip: String::new(), memory: String::new(), setup: String::new(), role: String::new(), log: String::new() }
    }

    /// The report as words: the Mac's line, the setup, the role, then the log — or, for
    /// the version alone, its line.
    pub fn text(&self) -> String {
        let mac: Vec<String> = [
            (!self.shroom.is_empty()).then(|| format!("shroom {}", self.shroom)),
            (!self.macos.is_empty()).then(|| format!("macOS {}", self.macos)),
            (!self.chip.is_empty()).then(|| self.chip.clone()),
            (!self.memory.is_empty()).then(|| self.memory.clone()),
        ]
        .into_iter()
        .flatten()
        .collect();
        let mut out = vec![mac.join(" · ")];
        if self.role.is_empty() && self.log.is_empty() && self.setup.is_empty() {
            return out.join("\n");
        }
        if !self.setup.is_empty() {
            out.push(format!("Set up: {}", self.setup));
        }
        out.push(format!("This Mac: {}", self.role));
        out.push(String::new());
        let n = self.log.lines().count();
        out.push(format!("— the last {n} lines of shroom’s log, cleaned —"));
        out.push(self.log.clone());
        out.join("\n")
    }
}

/// Which door a report came by: Settings' Report a problem (B180), or the popover's Send
/// feedback (N60). shroom.am/report tags its message with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Problem,
    Feedback,
}

impl Kind {
    /// site/report.js's `kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Problem => "problem",
            Kind::Feedback => "feedback",
        }
    }
}

/// What Send posts to shroom.am/report: its kind, the words, the reply-to, and the
/// gathered report (the version alone for feedback sent without the log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    pub kind: Kind,
    pub words: String,
    pub reply_to: Option<String>,
    pub diagnostics: Diagnostics,
}

impl Post {
    /// What goes, as the disclosure shows it: the words, the reply-to, then the report.
    pub fn text(&self) -> String {
        preview(self.kind, &self.words, self.reply_to.as_deref(), &self.diagnostics)
    }
}

/// The disclosure's text for what's typed and what's gathered.
pub fn preview(kind: Kind, words: &str, reply_to: Option<&str>, d: &Diagnostics) -> String {
    let head = match kind {
        Kind::Problem => "What happened:",
        Kind::Feedback => "Feedback:",
    };
    let mut out = format!("{head}\n{}\n\n", words.trim());
    if let Some(r) = reply_to.filter(|r| !r.trim().is_empty()) {
        out.push_str(&format!("Reply to: {}\n\n", r.trim()));
    }
    out.push_str(&d.text());
    out
}

/// A log, cleaned: its last `LOG_LINES` lines that hold anything, each cleaned.
pub fn clean_log(text: &str) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = lines.len().saturating_sub(LOG_LINES);
    lines[from..].iter().map(|l| clean_line(l)).collect::<Vec<_>>().join("\n")
}

/// The app's log and the `shroom` command's, cleaned: the command's last `CLI_LINES` runs
/// and as many of the app's last lines as leave the whole within `LOG_LINES`, in time order
/// by the stamp each line starts with (UTC to the second). An app line without one (from
/// before the stamps) keeps its place after the stamped line before it; at the same second
/// the app's comes first. A Mac where the command never ran sends just the app's log.
pub fn clean_logs(app: &str, cli: &str) -> String {
    let cli: Vec<&str> = cli.lines().filter(|l| !l.trim().is_empty()).collect();
    let cli = &cli[cli.len().saturating_sub(CLI_LINES)..];
    let app: Vec<&str> = app.lines().filter(|l| !l.trim().is_empty()).collect();
    let app = &app[app.len().saturating_sub(LOG_LINES - cli.len())..];
    let mut out = Vec::with_capacity(app.len() + cli.len());
    let (mut a, mut c, mut at) = (0, 0, "");
    while a < app.len() || c < cli.len() {
        if a < app.len() {
            at = stamp_of(app[a]).unwrap_or(at);
        }
        let take_cli = c < cli.len() && (a == app.len() || stamp_of(cli[c]).unwrap_or("") < at);
        if take_cli {
            out.push(clean_line(cli[c]));
            c += 1;
        } else {
            out.push(clean_line(app[a]));
            a += 1;
        }
    }
    out.join("\n")
}

/// The wall clock a log line starts with, `2026-10-18T17:52:03Z`, if it has one.
fn stamp_of(line: &str) -> Option<&str> {
    let s = line.get(..20)?;
    let shape = s.bytes().enumerate().all(|(i, b)| match i {
        4 | 7 => b == b'-',
        10 => b == b'T',
        13 | 16 => b == b':',
        19 => b == b'Z',
        _ => b.is_ascii_digit(),
    });
    shape.then_some(s)
}

/// One log line, cleaned — `site/clean.js`'s `cleanLine`, rule for rule.
pub fn clean_line(line: &str) -> String {
    let mut s: String = line.chars().filter(|c| *c == '\t' || !c.is_control()).collect();
    s = cut_webview(&s);
    s = replace_urls(&s);
    s = replace_after(&s);
    s = replace_emails(&s);
    s = replace_homes(&s);
    s = replace_hosts(&s);
    s = replace_hex(&s);
    s = replace_runs(&s);
    if s.chars().count() > LINE_CHARS {
        s = s.chars().take(LINE_CHARS).collect::<String>() + "…";
    }
    s
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `2026-10-18T17:52:03Z       10 webview popover: <message>`: the message cut to
/// `WEBVIEW_CHARS`.
fn cut_webview(s: &str) -> String {
    let Some(at) = s.find("webview ") else {
        return s.into();
    };
    // Only the line's own kind: its wall clock, then digits and spaces before it, a surface
    // name after it.
    if !after_stamp(&s[..at]).chars().all(|c| c.is_ascii_digit() || c == ' ') {
        return s.into();
    }
    let rest = &s[at + "webview ".len()..];
    let Some(colon) = rest.find(": ") else {
        return s.into();
    };
    if rest[..colon].is_empty() || !rest[..colon].chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
        return s.into();
    }
    let head = &s[..at + "webview ".len() + colon + 2];
    let message = &s[head.len()..];
    if message.chars().count() <= WEBVIEW_CHARS {
        return s.into();
    }
    format!("{head}{}…", message.chars().take(WEBVIEW_CHARS).collect::<String>())
}

/// A log line past its wall clock (`2026-10-18T17:52:03Z `), when it opens with one.
fn after_stamp(s: &str) -> &str {
    let b = s.as_bytes();
    let shape = |i: usize, c: u8| match c {
        b'-' | b'T' | b':' | b'Z' | b' ' => b[i] == c,
        _ => b[i].is_ascii_digit(),
    };
    if b.len() >= 21 && b"0000-00-00T00:00:00Z ".iter().enumerate().all(|(i, &c)| shape(i, c)) { &s[21..] } else { s }
}

fn host_of(url: &str) -> String {
    let after = url.split_once("://").map(|(_, r)| r).unwrap_or("");
    let authority = after.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    host.split(':').next().unwrap_or("").to_ascii_lowercase()
}

fn is_public(host: &str) -> bool {
    PUBLIC_HOSTS.contains(&host.to_ascii_lowercase().as_str())
}

fn is_space(host: &str) -> bool {
    host.to_ascii_lowercase().ends_with(".workers.dev")
}

/// `scheme://…` up to a space, a quote, `<`, `>`, `)` or `]`.
fn replace_urls(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        // A scheme starts at a letter not preceded by a word character.
        if chars[i].is_ascii_alphabetic() && (i == 0 || !is_word(chars[i - 1])) {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || "+.-".contains(chars[j])) {
                j += 1;
            }
            if j + 2 < chars.len() && chars[j] == ':' && chars[j + 1] == '/' && chars[j + 2] == '/' {
                let mut k = j + 3;
                while k < chars.len() && !chars[k].is_whitespace() && !"\"'<>)]".contains(chars[k]) {
                    k += 1;
                }
                if k > j + 3 {
                    let url: String = chars[i..k].iter().collect();
                    let host = host_of(&url);
                    out.push_str(&if is_space(&host) {
                        "<your space>".to_string()
                    } else if is_public(&host) {
                        format!("https://{host}/…")
                    } else {
                        "<a web address>".to_string()
                    });
                    i = k;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// An email address → `<an email>`.
fn replace_emails(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_ascii_alphanumeric() || ".-".contains(c);
    let mut out = String::new();
    let mut i = 0;
    let mut emitted = 0; // chars of `chars` already in `out`
    while i < chars.len() {
        if chars[i] == '@' {
            let mut a = i;
            while a > emitted && local(chars[a - 1]) {
                a -= 1;
            }
            let mut b = i + 1;
            while b < chars.len() && domain(chars[b]) {
                b += 1;
            }
            // The domain ends at its last `.xx` run of two or more letters.
            let dom: String = chars[i + 1..b].iter().collect();
            let end = dom.char_indices().filter(|(_, c)| *c == '.').filter_map(|(d, _)| {
                let tld: String = dom[d + 1..].chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                (tld.len() >= 2).then_some(d + 1 + tld.len())
            });
            if let (true, Some(len)) = (a < i, end.last()) {
                out.extend(&chars[emitted..a]);
                out.push_str("<an email>");
                i = i + 1 + dom[..len].chars().count();
                emitted = i;
                continue;
            }
        }
        i += 1;
    }
    out.extend(&chars[emitted..]);
    out
}

/// `/Users/<name>` and `/home/<name>` → `~`.
fn replace_homes(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    loop {
        let next = ["/Users/", "/home/"].iter().filter_map(|p| rest.find(p).map(|at| (at, p.len()))).min();
        let Some((at, len)) = next else { break };
        let name_len = rest[at + len..].find(|c: char| c == '/' || c.is_whitespace() || c == '"' || c == '\'').unwrap_or(rest.len() - at - len);
        if name_len == 0 {
            out.push_str(&rest[..at + len]);
            rest = &rest[at + len..];
            continue;
        }
        out.push_str(&rest[..at]);
        out.push('~');
        rest = &rest[at + len + name_len..];
    }
    out.push_str(rest);
    out
}

fn label_ok(l: &str) -> bool {
    !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !l.starts_with('-') && !l.ends_with('-')
}

/// A bare host (`media.example.com`) not inside a path or a word: the space's →
/// `<your space>`, anyone else's → `<a host>`; shroom's and its services' stay.
fn replace_hosts(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let in_run = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-';
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_alphanumeric() && (i == 0 || !(is_word(chars[i - 1]) || "./~-".contains(chars[i - 1])));
        if starts {
            let mut j = i;
            while j < chars.len() && in_run(chars[j]) {
                j += 1;
            }
            let run: String = chars[i..j].iter().collect();
            let labels: Vec<&str> = run.split('.').collect();
            // The longest leading run of labels that ends in a known ending.
            let mut best = None;
            for k in 2..=labels.len() {
                if !labels[..k].iter().all(|l| label_ok(l)) {
                    break;
                }
                let tld = labels[k - 1].to_ascii_lowercase();
                // `\b` after the ending: the next char isn't a word character.
                let used: usize = labels[..k].iter().map(|l| l.len()).sum::<usize>() + k - 1;
                let next = chars.get(i + used).copied();
                if TLDS.contains(&tld.as_str()) && !next.is_some_and(is_word) {
                    best = Some(used);
                }
            }
            if let Some(used) = best {
                let host: String = chars[i..i + used].iter().collect();
                out.push_str(if is_public(&host) {
                    &host
                } else if is_space(&host) {
                    "<your space>"
                } else {
                    "<a host>"
                });
                i += used;
                continue;
            }
            out.extend(&chars[i..j]);
            i = j;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// A whole word of 16 or more hex digits → `<an id>`.
fn replace_hex(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if is_word(chars[i]) && (i == 0 || !is_word(chars[i - 1])) {
            let mut j = i;
            while j < chars.len() && is_word(chars[j]) {
                j += 1;
            }
            let word = &chars[i..j];
            if word.len() >= 16 && word.iter().all(|c| c.is_ascii_hexdigit()) {
                out.push_str("<an id>");
            } else {
                out.extend(word);
            }
            i = j;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// What follows a link's code (`/join/`, `?k=`, `&k=`) or a credential's scheme
/// (`Bearer `, `ShroomParent `), up to a space → `<an id>`.
fn replace_after(s: &str) -> String {
    const MARKS: &[&str] = &["/join/", "?k=", "&k=", "Bearer ", "ShroomParent "];
    let mut out = String::new();
    let mut rest = s;
    loop {
        // The earliest mark followed by something; at one place, the first that matches.
        let next = MARKS
            .iter()
            .filter_map(|m| {
                let mut from = 0;
                while let Some(at) = rest[from..].find(m) {
                    let at = from + at;
                    if rest[at + m.len()..].chars().next().is_some_and(|c| !c.is_whitespace()) {
                        return Some((at, m.len()));
                    }
                    from = at + 1;
                }
                None
            })
            .min_by_key(|&(at, _)| at);
        let Some((at, len)) = next else { break };
        out.push_str(&rest[..at + len]);
        out.push_str("<an id>");
        let after = &rest[at + len..];
        rest = &after[after.find(char::is_whitespace).unwrap_or(after.len())..];
    }
    out.push_str(rest);
    out
}

/// A part of a run between `+ / _ -` that names a thing, not a key: short, a number, a word
/// with a number after it (`sha256`), or a number with a letter or two (`26b`, `18T17`).
fn shaped(p: &[char]) -> bool {
    let digits = |from: usize| from + p[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let letter = |at: usize| at + usize::from(p.get(at).is_some_and(|c| c.is_ascii_alphabetic()));
    let letters = p.iter().take_while(|c| c.is_ascii_alphabetic()).count();
    // `[0-9]+[A-Za-z]?[0-9]*[A-Za-z]?`
    let number = {
        let a = digits(0);
        a > 0 && letter(digits(letter(a))) == p.len()
    };
    p.len() <= 5 || number || (letters > 0 && digits(letters) == p.len())
}

/// A run's letters read as words: one case, or words of three letters or more (one shorter
/// allowed), at least three letters a word (`ScreenCaptureKit`, `NSURLErrorDomain`).
fn words(r: &[char]) -> bool {
    if !r.iter().any(char::is_ascii_lowercase) || !r.iter().any(char::is_ascii_uppercase) {
        return true;
    }
    // The run's words, as `[A-Z]+(?![a-z])|[A-Z]?[a-z]+` finds them: their lengths, and
    // whether each holds a small letter.
    let mut pieces: Vec<(usize, bool)> = Vec::new();
    let mut i = 0;
    while i < r.len() {
        if r[i].is_ascii_uppercase() {
            let up = r[i..].iter().take_while(|c| c.is_ascii_uppercase()).count();
            let lower_next = r.get(i + up).is_some_and(char::is_ascii_lowercase);
            if !lower_next || up >= 2 {
                let n = if lower_next { up - 1 } else { up };
                pieces.push((n, false));
                i += n;
                continue;
            }
            let low = r[i + 1..].iter().take_while(|c| c.is_ascii_lowercase()).count();
            pieces.push((1 + low, true));
            i += 1 + low;
        } else if r[i].is_ascii_lowercase() {
            let low = r[i..].iter().take_while(|c| c.is_ascii_lowercase()).count();
            pieces.push((low, true));
            i += low;
        } else {
            i += 1;
        }
    }
    let letters: usize = pieces.iter().map(|p| p.0).sum();
    let short = pieces.iter().filter(|p| p.1 && p.0 < 3).count();
    short <= 1 && pieces.len() * 3 <= letters
}

/// Whether a run of base64's characters is key-shaped: 24 or more holding a letter and a
/// digit; 32 or more holding both cases; 16 or more holding a digit or both cases, unless it
/// reads as names (each part `shaped`, its letters `words`).
fn id_like(r: &[char]) -> bool {
    let digit = r.iter().any(char::is_ascii_digit);
    let mixed = r.iter().any(char::is_ascii_lowercase) && r.iter().any(char::is_ascii_uppercase);
    if r.len() >= 24 && digit && r.iter().any(char::is_ascii_alphabetic) {
        return true;
    }
    if r.len() >= 32 && mixed {
        return true;
    }
    if r.len() < 16 || (!digit && !mixed) {
        return false;
    }
    let names = r.split(|c| "+/_-".contains(*c)).filter(|p| !p.is_empty()).all(shaped);
    !(names && words(r))
}

/// A key-shaped run (`id_like`) of base64's characters, and up to two `=` → `<an id>`.
fn replace_runs(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let in_run = |c: char| c.is_ascii_alphanumeric() || "+/_-".contains(c);
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if in_run(chars[i]) {
            let mut j = i;
            while j < chars.len() && in_run(chars[j]) {
                j += 1;
            }
            let mut end = j;
            while end < chars.len() && end < j + 2 && chars[end] == '=' {
                end += 1;
            }
            let run = &chars[i..j];
            if id_like(run) {
                out.push_str("<an id>");
                i = end;
            } else {
                out.extend(run);
                i = j;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// This Mac's place, in the report's words.
pub fn role_words(role: Option<crate::world::Role>) -> String {
    match role {
        Some(crate::world::Role::Member) => "a member’s Mac".into(),
        Some(_) => "owns its space".into(),
        None => "no space yet".into(),
    }
}
