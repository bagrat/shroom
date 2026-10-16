//! What a report carries (spec N53, DECISIONS B180): the Mac's line, the setup timeline and
//! the last lines of shroom's log, each one cleaned — the same rules as the feedback page's
//! (`site/clean.js`), held to the same cases (`site/test/clean-cases.json`). The log never
//! holds a secret (law 13), but it does hold values: a space's host, paths in the home
//! folder, and error text from macOS and the network. Pure: the shell reads the files and
//! the system, and hands the raw text here.

use serde::Serialize;

/// The lines a report keeps: the last ones, the newest last.
pub const LOG_LINES: usize = 200;
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
    /// The report as words: the Mac's line, the setup, the role, then the log.
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

/// What Send posts to shroom.am/report: the words, the reply-to, and the gathered report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    pub words: String,
    pub reply_to: Option<String>,
    pub diagnostics: Diagnostics,
}

impl Post {
    /// What goes, as the disclosure shows it: the words, the reply-to, then the report.
    pub fn text(&self) -> String {
        preview(&self.words, self.reply_to.as_deref(), &self.diagnostics)
    }
}

/// The disclosure's text for what's typed and what's gathered.
pub fn preview(words: &str, reply_to: Option<&str>, d: &Diagnostics) -> String {
    let mut out = format!("What happened:\n{}\n\n", words.trim());
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

/// One log line, cleaned — `site/clean.js`'s `cleanLine`, rule for rule.
pub fn clean_line(line: &str) -> String {
    let mut s: String = line.chars().filter(|c| *c == '\t' || !c.is_control()).collect();
    s = cut_webview(&s);
    s = replace_urls(&s);
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

/// `   10 webview popover: <message>`: the message cut to `WEBVIEW_CHARS`.
fn cut_webview(s: &str) -> String {
    let Some(at) = s.find("webview ") else {
        return s.into();
    };
    // Only the line's own kind: digits and spaces before it, a surface name after it.
    if !s[..at].chars().all(|c| c.is_ascii_digit() || c == ' ') {
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

/// `local@domain.tld` → `<an email>`.
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

/// A key-shaped run: 24 or more of base64's characters (and up to two `=`), holding a
/// letter and a digit → `<an id>`.
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
            if run.len() >= 24 && run.iter().any(|c| c.is_ascii_alphabetic()) && run.iter().any(|c| c.is_ascii_digit()) {
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
