//! `shroom` — shroom on the command line, for a person or the coding agent they ask (spec
//! N62; DECISIONS B215–B219; the write-up, dev/proposals/cli-agent-skill.md). Everything a
//! list, a link, a rename or a delete needs works whether the app is running or not: the
//! command reads its app's folder and asks the space's own page under this Mac's key.
//! Recording goes through the running app.
//!
//! The mechanism is exact and the judgment is the agent's: the command never guesses which
//! video was meant for a delete, never deletes without `--yes`, and says why in a stable
//! code (`--json`) when it won't.

mod app;
mod batch;
mod edit;
mod home;
mod log;
mod out;
mod say;
mod space;

use serde_json::{Value, json};

use home::Home;
use out::{Done, Fail, col, day, length};
use space::{Door, Video};

const GUIDE: &str = include_str!("../../../site/cli.md");


/// The command line, parsed: the verb, its words, and the options.
#[derive(Debug, Default)]
struct Args {
    words: Vec<String>,
    json: bool,
    yes: bool,
    dry_run: bool,
    mine: bool,
    no_launch: bool,
    /// `record start` without the count, and without the agent label (N62, B216).
    no_countdown: bool,
    no_label: bool,
    limit: Option<usize>,
    search: Option<String>,
    older_than: Option<u64>,
    home: Option<String>,
    space_base: Option<String>,
    /// Each argument is one item (`batch.rs`).
    batch: bool,
    /// The items from a file (`-`: stdin).
    from: Option<String>,
    /// Batch invites: a name already invited gets a new link (its old one stops working).
    renew: bool,
    /// `list`: only videos made since this many days ago, or this date (YYYY-MM-DD).
    since: Option<String>,
}

impl Args {
    /// The options given, by name only: the log's (`log.rs`). A value can be words (a
    /// search, a file's path), so none is named.
    fn options(&self) -> Vec<&'static str> {
        [
            (self.json, "json"),
            (self.yes, "yes"),
            (self.dry_run, "dry-run"),
            (self.mine, "mine"),
            (self.no_launch, "no-launch"),
            (self.no_countdown, "no-countdown"),
            (self.no_label, "no-label"),
            (self.limit.is_some(), "limit"),
            (self.search.is_some(), "search"),
            (self.older_than.is_some(), "older-than"),
            (self.batch, "batch"),
            (self.from.is_some(), "from"),
            (self.renew, "renew"),
            (self.since.is_some(), "since"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect()
    }
}

fn parse(argv: Vec<String>) -> Result<Args, Fail> {
    let mut a = Args::default();
    let mut it = argv.into_iter();
    while let Some(w) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| Fail::Usage(say::needs_value(name)));
        match w.as_str() {
            "--json" => a.json = true,
            "--yes" | "-y" => a.yes = true,
            "--dry-run" => a.dry_run = true,
            "--mine" => a.mine = true,
            "--no-launch" => a.no_launch = true,
            "--no-countdown" => a.no_countdown = true,
            "--no-label" => a.no_label = true,
            "--limit" => a.limit = Some(value("--limit")?.parse().map_err(|_| Fail::Usage(say::limit_is_a_number()))?),
            "--search" => a.search = Some(value("--search")?),
            "--batch" => a.batch = true,
            "--from" => a.from = Some(value("--from")?),
            "--renew" => a.renew = true,
            "--since" => a.since = Some(value("--since")?),
            "--older-than" => {
                let v = value("--older-than")?;
                a.older_than = Some(v.trim_end_matches('d').parse().map_err(|_| Fail::Usage(say::older_than_is_days()))?)
            }
            // Debug builds only (tests and the cloud demo): another home, a local space.
            "--home" if cfg!(debug_assertions) => a.home = Some(value("--home")?),
            "--space-base" if cfg!(debug_assertions) => a.space_base = Some(value("--space-base")?),
            "-h" | "--help" => a.words.insert(0, "help".into()),
            "-V" | "--version" => a.words.insert(0, "version".into()),
            s if s.starts_with("--") => return Err(Fail::Usage(say::not_an_option(s))),
            _ => a.words.push(w),
        }
    }
    Ok(a)
}

fn main() {
    let started = std::time::Instant::now();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let json_wanted = argv.iter().any(|a| a == "--json");
    let words: Vec<String> = argv.iter().filter(|w| !w.starts_with('-')).cloned().collect();
    let parsed = parse(argv);
    let result = match &parsed {
        Ok(a) => run(a).map(|d| (a.json, d)),
        Err(f) => Err(f.clone()),
    };
    // One line in the command's own log: what was asked, how it ended, what it counted
    // (log.rs) — never a word of what it was given or said.
    let (command, options, home) = match &parsed {
        Ok(a) => (log::command(&a.words), a.options(), Home::locate(a.home.as_deref())),
        Err(_) => (log::command(&words), vec![], Home::locate(None)),
    };
    let (ended, counted) = match &result {
        Ok((_, done)) => ("ok".to_string(), log::counted(&done.json)),
        Err(f) => (format!("{}, exit {}", f.code(), f.exit()), log::counted(&f.json())),
    };
    if let Some(h) = home {
        log::write(&h.dir, &command, &options, &ended, &counted, started.elapsed());
    }
    match result {
        // Titles and names come from other people in the space: nothing a terminal would act
        // on gets through (out::clean, out::json_safe).
        Ok((json, done)) => {
            if json {
                println!("{}", out::json_safe(&done.json.to_string()));
            } else if !done.text.is_empty() {
                println!("{}", out::clean(done.text.trim_end()));
            }
        }
        Err(f) => {
            if json_wanted {
                println!("{}", out::json_safe(&f.json().to_string()));
            } else {
                eprintln!("{}", out::clean(f.message()));
            }
            std::process::exit(f.exit());
        }
    }
}

fn run(a: &Args) -> Result<Done, Fail> {
    let verb = a.words.first().map(String::as_str).unwrap_or("help");
    let rest = &a.words[a.words.len().min(1)..];
    match verb {
        "help" => Ok(Done::new(json!({"help": say::HELP}), say::HELP)),
        "version" => Ok(Done::new(json!({"version": env!("SHROOM_VERSION")}), say::version(env!("SHROOM_VERSION")))),
        "guide" | "skill" => Ok(Done::new(json!({"guide": GUIDE}), GUIDE)),
        "status" => status(a),
        "list" | "ls" => list(a),
        "info" | "show" => one(a, rest, |v, _| Ok(Done::new(json!({"video": v.json()}), info_text(v)))),
        "link" | "links" if a.batch || a.from.is_some() || verb == "links" => links(a, rest),
        "link" => one(a, rest, |v, _| Ok(Done::new(json!({"id": v.row.id, "link": v.link}), v.link.clone()))),
        "open" => one(a, rest, |v, _| {
            open_url(&v.link)?;
            Ok(Done::new(json!({"id": v.row.id, "link": v.link, "opened": true}), say::opened(&v.link)))
        }),
        "rename" if a.batch || a.from.is_some() => rename_batch(a, rest),
        "rename" => rename(a, rest),
        "delete" | "rm" => delete(a, rest),
        "cleanup-candidates" | "cleanup" => cleanup(a),
        "members" => match rest.first().map(String::as_str) {
            Some("remove") => remove_member(a, &rest[1..]),
            Some(other) => Err(Fail::Usage(say::not_a_members_command(other))),
            None => members(a),
        },
        "invite" => match rest.first().map(String::as_str) {
            Some("cancel") => cancel_invite(a, &rest[1..]),
            _ if a.batch || a.from.is_some() => invite_batch(a, rest),
            _ => invite(a, rest),
        },
        "record" => record(a, rest),
        "edit" => edit::edit(a, rest),
        other => Err(Fail::Usage(say::not_a_command(other))),
    }
}

fn home(a: &Args) -> Result<Home, Fail> {
    Home::locate(a.home.as_deref()).ok_or_else(|| Fail::NotSetUp(say::no_home_folder()))
}

fn door(a: &Args) -> Result<(Home, Door), Fail> {
    let h = home(a)?;
    let space = h.space().ok_or_else(|| Fail::NotSetUp(say::not_set_up()))?;
    let key = h.device_key(&space).ok_or_else(|| Fail::NotSetUp(say::not_joined()))?;
    let base = if cfg!(debug_assertions) { a.space_base.clone().or_else(|| std::env::var("SHROOM_SPACE_BASE").ok()) } else { None };
    Ok((h, Door::open(space, key, base)))
}

/// The ids a command acts on: its arguments, or `--from`'s items, each one an id (a line, a
/// string, or an object's `"id"`). An item of another shape is refused before anything is
/// done, never quietly dropped.
fn ids(a: &Args, rest: &[String], usage: fn() -> String) -> Result<Vec<String>, Fail> {
    let ids: Vec<String> = match &a.from {
        Some(_) => batch::items(rest, a.from.as_deref())?
            .iter()
            .map(|i| i.field("id").filter(|s| !s.is_empty()).ok_or_else(|| Fail::Usage(say::item_needs_an_id())))
            .collect::<Result<_, _>>()?,
        None => rest.to_vec(),
    };
    if ids.is_empty() {
        return Err(Fail::Usage(usage()));
    }
    Ok(ids)
}

/// A change waits while a recording is open: shroom stays quiet mid-take (law 4).
fn quiet(h: &Home) -> Result<(), Fail> {
    match h.app_status() {
        Some(s) if s["take_open"] == true => Err(Fail::TakeOpen(say::take_open())),
        _ => Ok(()),
    }
}

fn status(a: &Args) -> Result<Done, Fail> {
    let h = home(a)?;
    let app = h.app_status();
    let space = h.space();
    let key = space.as_ref().and_then(|s| h.device_key(s));
    let owner = space.as_ref().map(|s| s.role != shroom_core::world::Role::Member);
    let take = app.as_ref().and_then(|s| s["take"].as_str()).unwrap_or("none").to_string();
    let json = json!({
        "app": {
            "id": h.app_id,
            "running": app.is_some(),
            "version": app.as_ref().and_then(|s| s["app"].as_str()),
            "take": take,
            "setup": app.as_ref().and_then(|s| s["setup"].as_str()),
        },
        "space": space.as_ref().map(|s| json!({"host": s.host, "role": if owner == Some(true) { "owner" } else { "member" }, "ready": key.is_some()})),
        "cli": env!("SHROOM_VERSION"),
    });
    let text = [say::status_app(app.is_some(), &take), say::status_space(space.as_ref().map(|s| s.host.as_str()), key.is_some(), owner == Some(true))];
    Ok(Done::new(json, text.join("\n")))
}

fn videos(a: &Args) -> Result<(Home, Door, Vec<Video>), Fail> {
    let (h, d) = door(a)?;
    let cached = h.cached_rows(&d.identity);
    let mut v = d.list(&cached)?;
    if a.mine {
        let me = d.identity.member_id.clone();
        v.retain(|x| x.row.owner.as_deref() == Some(me.as_str()));
    }
    if let Some(s) = &a.search {
        v.retain(|x| titled(&x.row.title, s));
    }
    if let Some(since) = &a.since {
        let from = since_ms(since)?;
        v.retain(|x| shroom_upload::library::iso_ms(&x.row.created_at).is_some_and(|ms| ms >= from));
    }
    Ok((h, d, v))
}

/// `--since 7d` (that many days back from now) or `--since 2026-10-01` (that day, UTC), as
/// Unix milliseconds.
fn since_ms(since: &str) -> Result<i64, Fail> {
    let bad = || Fail::Usage(say::since_is_days_or_a_date());
    if let Some(days) = since.strip_suffix('d') {
        let n: i64 = days.parse().map_err(|_| bad())?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        return Ok(now - n * 86_400_000);
    }
    shroom_upload::library::iso_ms(&format!("{since}T00:00:00Z")).ok_or_else(bad)
}

fn list(a: &Args) -> Result<Done, Fail> {
    let (_, d, mut v) = videos(a)?;
    let total = v.len();
    if let Some(n) = a.limit {
        v.truncate(n);
    }
    // What was asked for, as shown: a member's list is always their own; --mine makes an
    // owner's theirs too; --since and --search say what they kept.
    let scope = if a.mine || d.identity.role == shroom_core::world::Role::Member { "mine" } else { "everyone" };
    let mut text = String::new();
    if v.is_empty() {
        text.push_str(&if a.search.is_some() || a.mine || a.since.is_some() { say::none_match() } else { say::no_videos_yet() });
    } else {
        let [id, date, len, title] = say::LIST_HEAD;
        text.push_str(&format!("{}  {}  {}  {title}\n", col(id, 16), col(date, 10), col(len, 7)));
        for x in &v {
            text.push_str(&format!("{}  {}  {}  {}\n", x.row.id, day(&x.row.created_at), col(&length(x.row.duration_sec), 7), x.row.title));
        }
        if v.len() < total {
            text.push_str(&say::shown_of(v.len(), total));
        }
    }
    let mut j = json!({"scope": scope, "count": v.len(), "total": total, "videos": v.iter().map(Video::json).collect::<Vec<_>>()});
    if let Some(since) = &a.since {
        j["since"] = since.clone().into();
    }
    if let Some(search) = &a.search {
        j["search"] = search.clone().into();
    }
    Ok(Done::new(j, text))
}

fn info_text(v: &Video) -> String {
    format!("{}\n{} · {}\n{}", v.row.title, day(&v.row.created_at), length(v.row.duration_sec), v.link)
}

/// The id a word names, when it's an id or a link to this space.
fn id_of(word: &str, host: &str) -> Option<String> {
    let w = word.trim().trim_end_matches('/');
    let w = w.strip_prefix("https://").or_else(|| w.strip_prefix("http://")).unwrap_or(w);
    let id = match w.split_once('/') {
        Some((h, rest)) if h.eq_ignore_ascii_case(host) => rest.split(['?', '#']).next().unwrap_or(""),
        Some(_) => return None,
        None => w,
    };
    shroom_cloud::device::is_b14(id).then(|| id.to_string())
}

/// Whether every one of `words` is in `title`, in any order, case aside: “login bug repro”
/// finds “Bug repro: login loops on Safari”.
fn titled(title: &str, words: &str) -> bool {
    let title = title.to_lowercase();
    let mut words = words.split_whitespace().map(str::to_lowercase).peekable();
    words.peek().is_some() && words.all(|w| title.contains(&w))
}

/// Finds the one video `words` names: an id, a link, or words from its title.
fn find(v: &[Video], words: &str, host: &str, titles: bool) -> Result<Video, Fail> {
    if let Some(id) = id_of(words, host) {
        return v.iter().find(|x| x.row.id == id).cloned().ok_or_else(|| Fail::NotFound(say::no_video(&id)));
    }
    if !titles {
        return Err(Fail::Usage(say::by_id_or_link()));
    }
    let hits: Vec<&Video> = v.iter().filter(|x| titled(&x.row.title, words)).collect();
    match hits.as_slice() {
        [] => Err(Fail::NotFound(say::no_title_has(words))),
        [one] => Ok((*one).clone()),
        _ => Err(Fail::Ambiguous { message: say::several_match(hits.len(), words), matches: hits.iter().map(|x| x.json()).collect() }),
    }
}

fn one(a: &Args, rest: &[String], then: impl FnOnce(&Video, &Door) -> Result<Done, Fail>) -> Result<Done, Fail> {
    if rest.is_empty() {
        return Err(Fail::Usage(say::which_video()));
    }
    let (_, d, v) = videos(a)?;
    let hit = find(&v, &rest.join(" "), &d.identity.host, true)?;
    then(&hit, &d)
}

fn rename(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let [which, title @ ..] = rest else { return Err(Fail::Usage(say::rename_usage())) };
    if title.is_empty() {
        return Err(Fail::Usage(say::rename_usage()));
    }
    let (h, d, v) = videos(a)?;
    let hit = find(&v, which, &d.identity.host, true)?;
    if a.dry_run {
        let title = title.join(" ");
        return Ok(Done::new(json!({"dry_run": true, "id": hit.row.id, "was": hit.row.title, "title": title, "link": hit.link}), say::would_rename(&hit.row.title, &title)));
    }
    quiet(&h)?;
    let saved = d.rename(&hit.row.id, &title.join(" "))?;
    Ok(Done::new(json!({"id": hit.row.id, "title": saved, "link": hit.link}), say::renamed(&saved)))
}

/// Several titles at once: `--batch "<id>=<title>" …`, or `--from` a JSON array of
/// `{"id", "title"}` (or lines `<id>=<title>`). By id or link only, as a delete is. A
/// video that already has its new title is `unchanged`.
fn rename_batch(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let items = batch::items(rest, a.from.as_deref())?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for i in &items {
        let pair = match i {
            batch::Item::Obj(_) => i.field("id").zip(i.field("title")),
            batch::Item::Text(t) => t.split_once('=').map(|(x, y)| (x.trim().to_string(), y.trim().to_string())),
        };
        match pair {
            Some((id, title)) if !id.is_empty() && !title.is_empty() => pairs.push((id, title)),
            _ => return Err(Fail::Usage(say::rename_item_shape())),
        }
    }
    let (h, d, v) = videos(a)?;
    if !a.dry_run {
        quiet(&h)?;
    }
    let mut outcomes = Vec::new();
    for (w, title) in &pairs {
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let hit = match find(&v, w, &d.identity.host, false) {
            Ok(h) => h,
            Err(f) => {
                outcomes.push(batch::Outcome::failed(json!({"id": w, "title": title}), &f, say::line_failed(w, f.message())));
                continue;
            }
        };
        let base = json!({"id": hit.row.id, "was": hit.row.title, "title": title, "link": hit.link});
        if hit.row.title == title {
            let mut j = base;
            j["status"] = "unchanged".into();
            outcomes.push(batch::Outcome::ok(j, say::line_already_so(&title)));
        } else if a.dry_run {
            let mut j = base;
            j["status"] = "would_rename".into();
            outcomes.push(batch::Outcome::ok(j, say::line_renamed(&hit.row.title, &title)));
        } else {
            match d.rename(&hit.row.id, &title) {
                Ok(saved) => {
                    let mut j = base;
                    j["title"] = saved.clone().into();
                    j["status"] = "renamed".into();
                    outcomes.push(batch::Outcome::ok(j, say::line_renamed(&hit.row.title, &saved)));
                }
                Err(f) => outcomes.push(batch::Outcome::failed(base, &f, say::line_failed(&format!("“{}”", hit.row.title), f.message()))),
            }
        }
    }
    let mut done = batch::finish("results", outcomes, say::renames_head(a.dry_run), "")?;
    if a.dry_run {
        done.json["dry_run"] = true.into();
    }
    Ok(done)
}

/// The links of several videos, each named as one argument (an id, a link, or its title
/// words) — for a digest or a message. A name that matches none, or several, says so and
/// the rest still come back.
fn links(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let words: Vec<String> = batch::items(rest, a.from.as_deref())?.iter().filter_map(|i| i.field("id").or_else(|| i.field("title"))).collect();
    let (_, d, v) = videos(a)?;
    let mut outcomes = Vec::new();
    for w in &words {
        match find(&v, w, &d.identity.host, true) {
            Ok(hit) => {
                let mut j = hit.json();
                j["asked"] = w.clone().into();
                j["status"] = "found".into();
                outcomes.push(batch::Outcome::ok(j, say::line_link(&hit.row.title, &hit.link)));
            }
            Err(f) => {
                let mut j = json!({"asked": w});
                if let Fail::Ambiguous { matches, .. } = &f {
                    j["matches"] = Value::Array(matches.clone());
                }
                outcomes.push(batch::Outcome::failed(j, &f, say::line_failed(w, f.message())));
            }
        }
    }
    batch::finish("links", outcomes, say::links_head(), "")
}

fn delete(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let words = ids(a, rest, say::delete_usage)?;
    let (h, d, v) = videos(a)?;
    // Each word: a video of this space (by id or link — never title words), or one that
    // isn't there, which a re-run after a delete will meet: reported, never fatal.
    let mut targets: Vec<Video> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for w in &words {
        match find(&v, w, &d.identity.host, false) {
            Ok(hit) if !targets.iter().any(|t| t.row.id == hit.row.id) => targets.push(hit),
            Ok(_) => {}
            Err(f) if f.code() == "not_found" => missing.push(id_of(w, &d.identity.host).unwrap_or_else(|| w.clone())),
            Err(f) => return Err(f),
        }
    }
    let would: Vec<Value> = targets.iter().map(Video::json).collect();
    let names = targets.iter().map(|t| say::line_video(&t.row.id, &t.row.title)).collect::<Vec<_>>().join("\n");
    let gone_note = say::not_here_already_gone(&missing);
    if a.dry_run {
        return Ok(Done::new(json!({"dry_run": true, "would_delete": would, "not_found": missing}), say::would_delete(&names, &gone_note)));
    }
    if targets.is_empty() {
        let outcomes = missing.iter().map(|id| batch::Outcome::ok(json!({"id": id, "status": "not_found"}), say::line_not_found(id))).collect();
        let mut done = batch::finish("results", outcomes, say::nothing_to_delete(), "")?;
        done.json["deleted"] = json!([]);
        return Ok(done);
    }
    if !a.yes {
        return Err(Fail::NeedsConfirmation {
            message: say::confirm_delete(&names, &gone_note),
            what: "would_delete",
            would,
        });
    }
    quiet(&h)?;
    // Every one is tried; the ones done stay done, and a failed one doesn't stop the rest.
    let mut outcomes = Vec::new();
    let mut deleted = Vec::new();
    for t in &targets {
        let item = json!({"id": t.row.id, "title": t.row.title});
        match d.delete(&t.row.id) {
            Ok(()) => {
                deleted.push(t.row.id.clone());
                let mut j = item;
                j["status"] = "deleted".into();
                outcomes.push(batch::Outcome::ok(j, say::line_deleted(&t.row.id, &t.row.title)));
            }
            Err(f) => outcomes.push(batch::Outcome::failed(item, &f, say::line_failed(&say::line_video(&t.row.id, &t.row.title), f.message()))),
        }
    }
    for id in &missing {
        outcomes.push(batch::Outcome::ok(json!({"id": id, "status": "not_found"}), say::line_not_found(id)));
    }
    let mut done = batch::finish("results", outcomes, say::deleted(deleted.len()), "")?;
    done.json["deleted"] = json!(deleted);
    Ok(done)
}

/// What may be worth deleting (spec N62, DECISIONS B215): the page's own suggestions where
/// the page makes them — with plays, when last played and why — most likely first, narrowed
/// to `--older-than` days when given; and on a page that doesn't make them, the videos older
/// than `--older-than` days (90), oldest first. The same fields either way, unknown `null`,
/// and `by` says which it was.
fn cleanup(a: &Args) -> Result<Done, Fail> {
    let (_, d) = door(a)?;
    let Some(s) = d.suggestions(a.mine)? else { return cleanup_by_age(a) };
    // The page's days are fractions; a person reads whole ones.
    let whole = |d: Option<f64>| d.map(|d| d.max(0.0).floor() as u64);
    let mut rows: Vec<_> = s.items.into_iter().filter(|i| i.candidate && a.older_than.is_none_or(|n| whole(i.age_days).is_some_and(|age| age >= n))).collect();
    // Most likely first; between equals, the older.
    rows.sort_by(|x, y| y.score.total_cmp(&x.score).then(y.age_days.unwrap_or(0.0).total_cmp(&x.age_days.unwrap_or(0.0))));
    if let Some(n) = a.limit {
        rows.truncate(n);
    }
    let candidates: Vec<Value> = rows
        .iter()
        .map(|i| {
            let code = i.reason.as_ref().map_or("suggested", |r| r.code.as_str());
            json!({
                "id": i.id, "title": i.title, "state": "live", "created_at": i.created_at, "duration_sec": i.duration_sec,
                "owner": i.owner, "link": d.link(&i.id), "age_days": whole(i.age_days), "plays": i.plays, "last_played": i.last_played_at,
                "quiet_days": whole(i.quiet_days), "score": i.score, "reason": code, "why": say::why(code, whole(i.quiet_days)),
            })
        })
        .collect();
    let text = if rows.is_empty() {
        say::no_suggestions(a.older_than)
    } else {
        let lines: Vec<String> = candidates
            .iter()
            .map(|c| say::line_candidate(c["id"].as_str().unwrap_or(""), day(c["created_at"].as_str().unwrap_or("")), c["title"].as_str().unwrap_or(""), c["why"].as_str().unwrap_or("")))
            .collect();
        format!("{}\n{}\n{}", say::suggestions_head(rows.len()), lines.join("\n"), say::nothing_was_deleted())
    };
    Ok(Done::new(json!({"by": "suggestions", "counting": s.counting, "counting_since": s.since, "older_than_days": a.older_than, "candidates": candidates}), text))
}

/// Without the page's suggestions: age alone, oldest first.
fn cleanup_by_age(a: &Args) -> Result<Done, Fail> {
    let days = a.older_than.unwrap_or(90);
    let (_, _, v) = videos(a)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let mut rows: Vec<(i64, &Video)> = v
        .iter()
        .filter_map(|x| shroom_upload::library::iso_ms(&x.row.created_at).map(|ms| ((now - ms) / 86_400_000, x)))
        .filter(|(age, _)| *age >= days as i64)
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.0));
    if let Some(n) = a.limit {
        rows.truncate(n);
    }
    let candidates: Vec<Value> = rows
        .iter()
        .map(|(age, x)| {
            let mut j = x.json();
            for unknown in ["plays", "last_played", "quiet_days", "score"] {
                j[unknown] = Value::Null;
            }
            j["age_days"] = json!(age);
            j["reason"] = "old".into();
            j["why"] = say::why_old(days).into();
            j
        })
        .collect();
    let text = if rows.is_empty() {
        say::nothing_older(days)
    } else {
        let lines: Vec<String> = rows.iter().map(|(_, x)| say::line_candidate(&x.row.id, day(&x.row.created_at), &x.row.title, &say::why_old(days))).collect();
        format!("{}\n{}\n{}", say::old_head(rows.len(), days), lines.join("\n"), say::nothing_was_deleted())
    };
    Ok(Done::new(json!({"by": "age", "counting_since": null, "older_than_days": days, "candidates": candidates}), text))
}

fn members(a: &Args) -> Result<Done, Fail> {
    let (_, d) = door(a)?;
    let m = d.members()?;
    let people: Vec<Value> = m.members.iter().map(|x| json!({"id": x.id, "name": x.label, "joined_at": x.joined_at})).collect();
    let invites: Vec<Value> =
        m.invites.iter().map(|i| json!({"id": i.id, "kind": i.kind, "name": i.label, "expires_at": i.expires_at, "expired": i.expired})).collect();
    let macs: Vec<Value> = m.owner.devices.iter().map(|x| json!({"id": x.id, "name": x.name, "this_mac": x.this_mac})).collect();
    let mut text = format!("{}\n", say::owner_line(m.owner.name.as_deref(), macs.len()));
    for x in &m.members {
        text.push_str(&format!("{}\n", say::member_line(&x.id, &x.label, day(&x.joined_at))));
    }
    for i in m.invites.iter().filter(|i| !i.expired) {
        text.push_str(&format!("{}\n", say::invited_line(&i.id, i.label.as_deref())));
    }
    Ok(Done::new(json!({"owner": {"id": m.owner.id, "name": m.owner.name, "macs": macs}, "members": people, "invites": invites}), text))
}

/// A member invite. The link is a key to the space until it's used, so it goes to the
/// person who asked, once; the guide tells an agent to hand it over and not repeat it
/// (Bagrat, 2026-10-18: the CLI may make invites).
fn invite(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let name = rest.join(" ");
    let name = name.trim();
    if name.is_empty() {
        return Err(Fail::Usage(say::invite_who()));
    }
    let (h, d) = door(a)?;
    if a.dry_run {
        return Ok(Done::new(json!({"dry_run": true, "would_invite": name}), say::would_invite(name)));
    }
    quiet(&h)?;
    let (row, link) = d.invite(name)?;
    Ok(Done::new(
        json!({"id": row.id, "name": row.label, "link": link, "expires_at": row.expires_at}),
        say::invite_made(name, day(&row.expires_at), &link),
    ))
}

/// Cancels invites, one or several (say, for people no longer on the team list): each
/// link stops working. One already cancelled is `cancelled` again (the space says so); one
/// the space doesn't know is `not_found`; one already used is `failed` — they're in, and
/// `members remove` is the way out.
fn cancel_invite(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let ids = ids(a, rest, say::cancel_usage)?;
    // An invite's id, and only that: never a path in its place (`../members/…`).
    if let Some(bad) = ids.iter().find(|id| !shroom_cloud::device::is_registry_id(id, 'i')) {
        return Err(Fail::Usage(say::not_an_invite_id(bad)));
    }
    let (h, d) = door(a)?;
    if a.dry_run {
        let m = d.members()?;
        let outcomes = ids
            .iter()
            .map(|id| match m.invites.iter().find(|i| &i.id == id) {
                Some(i) => batch::Outcome::ok(json!({"id": id, "name": i.label, "status": "would_cancel"}), say::line_would_cancel(id, i.label.as_deref())),
                None => batch::Outcome::ok(json!({"id": id, "status": "not_found"}), say::line_not_found(id)),
            })
            .collect();
        let mut done = batch::finish("results", outcomes, say::would_cancel_head(), "")?;
        done.json["dry_run"] = true.into();
        return Ok(done);
    }
    quiet(&h)?;
    let mut outcomes = Vec::new();
    for id in &ids {
        let item = json!({"id": id});
        match d.cancel_invite(id) {
            Ok(()) => {
                let mut j = item;
                j["status"] = "cancelled".into();
                outcomes.push(batch::Outcome::ok(j, say::line_cancelled(id)));
            }
            Err(f) if f.code() == "not_found" => {
                let mut j = item;
                j["status"] = "not_found".into();
                outcomes.push(batch::Outcome::ok(j, say::line_not_found(id)));
            }
            Err(f) => outcomes.push(batch::Outcome::failed(item, &f, say::line_failed(id, f.message()))),
        }
    }
    let mut done = batch::finish("results", outcomes, say::cancelled_head(), "")?;
    if let [one] = ids.as_slice() {
        done.json["cancelled"] = one.clone().into();
    }
    Ok(done)
}

/// Removes members: like a delete, it asks first (`--yes`), and names them by id only. Each
/// is tried; one already gone is reported as such.
fn remove_member(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let ids = ids(a, rest, say::remove_usage)?;
    let (h, d) = door(a)?;
    let m = d.members()?;
    let mut who = Vec::new();
    let mut missing = Vec::new();
    for id in &ids {
        match m.members.iter().find(|x| &x.id == id) {
            Some(x) if !who.iter().any(|w: &&shroom_cloud::member::MemberRow| w.id == x.id) => who.push(x),
            Some(_) => {}
            None => missing.push(id.clone()),
        }
    }
    let would: Vec<Value> = who.iter().map(|x| json!({"id": x.id, "name": x.label})).collect();
    let names = who.iter().map(|x| say::line_video(&x.id, &x.label)).collect::<Vec<_>>().join("\n");
    let gone_note = say::not_here_already_removed(&missing);
    if a.dry_run {
        return Ok(Done::new(json!({"dry_run": true, "would_remove": would, "not_found": missing}), say::would_remove(&names, &gone_note)));
    }
    if who.is_empty() {
        // Nobody to remove: a re-run after it went through, or ids no longer here — said item
        // by item, as a delete's are.
        let outcomes = missing.iter().map(|id| batch::Outcome::ok(json!({"id": id, "status": "not_found"}), say::line_not_found(id))).collect();
        return batch::finish("results", outcomes, say::nobody_to_remove(), "");
    }
    if !a.yes {
        return Err(Fail::NeedsConfirmation {
            message: say::confirm_remove(&names, &gone_note),
            what: "would_remove",
            would,
        });
    }
    quiet(&h)?;
    let mut outcomes = Vec::new();
    for x in &who {
        let item = json!({"id": x.id, "name": x.label});
        match d.remove_member(&x.id) {
            Ok(()) => {
                let mut j = item;
                j["status"] = "removed".into();
                outcomes.push(batch::Outcome::ok(j, say::line_removed(&x.label)));
            }
            Err(f) => outcomes.push(batch::Outcome::failed(item, &f, say::line_failed(&x.label, f.message()))),
        }
    }
    for id in &missing {
        outcomes.push(batch::Outcome::ok(json!({"id": id, "status": "not_found"}), say::line_not_found(id)));
    }
    let mut done = batch::finish("results", outcomes, say::removed_head(), "")?;
    if let [one] = who.as_slice() {
        done.json["removed"] = json!(one.id);
    }
    Ok(done)
}

/// Invites for many people in one call (Bagrat, 2026-10-18: a team lead's agent invites the
/// team, then mails each link itself). Each name gets its own link — shroom sends nothing.
/// Safe to run again: a name already in the space is `already_member`, one already invited
/// and waiting is `already_invited` (its link can't be shown again: only its hash is kept;
/// `--renew` makes it a new one, and the old one stops working).
///
/// Like a delete, it asks first: without `--yes` it makes nothing and answers
/// `needs_confirmation` with the list. A batch gives several people a way into the space,
/// often from a list the agent looked up — the batch demo's agent, told "the engineering
/// team", invited everyone its directory named, someone on leave included, without showing
/// the list (2026-10-18). The guide's rule wasn't enough; this makes the ask the mechanism's.
fn invite_batch(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let items = batch::items(rest, a.from.as_deref())?;
    let mut names: Vec<String> = Vec::new();
    for i in &items {
        let Some(n) = i.field("name").map(|n| n.split_whitespace().collect::<Vec<_>>().join(" ")) else {
            return Err(Fail::Usage(say::invite_item_shape()));
        };
        if n.is_empty() || n.chars().count() > 64 {
            return Err(Fail::Usage(say::not_a_name(&n)));
        }
        if !names.iter().any(|x| x.to_lowercase() == n.to_lowercase()) {
            names.push(n);
        }
    }
    let (h, d) = door(a)?;
    let m = d.members()?;
    let key = |s: &str| s.to_lowercase();
    // Without --yes, what it would do is the answer — and nothing is made.
    let plan_only = a.dry_run || !a.yes;
    let member = |n: &str| m.members.iter().find(|x| key(&x.label) == key(n));
    let waiting = |n: &str| m.invites.iter().find(|i| !i.expired && i.kind == "member" && i.label.as_deref().map(key) == Some(key(n)));
    // The space's room for waiting invites (at most 20, WORKER-INTERFACE §4.12): only the
    // people this would invite anew take one — never someone already in, or already invited.
    let new = names.iter().filter(|n| member(n).is_none() && waiting(n).is_none()).count();
    let room = batch::MAX_INVITES.saturating_sub(m.invites.iter().filter(|i| !i.expired).count());
    if new > room {
        return Err(Fail::Usage(say::no_room(new, room)));
    }
    if !plan_only {
        quiet(&h)?;
    }
    let mut outcomes = Vec::new();
    for n in &names {
        let base = json!({"name": n});
        if let Some(x) = member(n) {
            let mut j = base;
            j["status"] = "already_member".into();
            j["member_id"] = x.id.clone().into();
            outcomes.push(batch::Outcome::ok(j, say::line_already_member(n)));
            continue;
        }
        if let Some(i) = waiting(n) {
            if !a.renew {
                let mut j = base;
                j["status"] = "already_invited".into();
                j["invite_id"] = i.id.clone().into();
                j["expires_at"] = i.expires_at.clone().into();
                outcomes.push(batch::Outcome::ok(j, say::line_already_invited(n)));
                continue;
            }
            if plan_only {
                let mut j = base;
                j["status"] = "would_renew".into();
                outcomes.push(batch::Outcome::ok(j, say::line_would_renew(n)));
                continue;
            }
            match d.renew(&i.id) {
                Ok((row, link)) => {
                    let mut j = base;
                    j["status"] = "renewed".into();
                    j["invite_id"] = row.id.into();
                    j["link"] = link.clone().into();
                    j["expires_at"] = row.expires_at.into();
                    outcomes.push(batch::Outcome::ok(j, say::line_invite(n, &link)));
                }
                Err(f) => outcomes.push(batch::Outcome::failed(base, &f, say::line_failed(n, f.message()))),
            }
            continue;
        }
        if plan_only {
            let mut j = base;
            j["status"] = "would_invite".into();
            outcomes.push(batch::Outcome::ok(j, say::line_would_invite(n)));
            continue;
        }
        match d.invite(n) {
            Ok((row, link)) => {
                let mut j = base;
                j["status"] = "invited".into();
                j["invite_id"] = row.id.into();
                j["link"] = link.clone().into();
                j["expires_at"] = row.expires_at.into();
                outcomes.push(batch::Outcome::ok(j, say::line_invite(n, &link)));
            }
            Err(f) => outcomes.push(batch::Outcome::failed(base, &f, say::line_failed(n, f.message()))),
        }
    }
    let would = |o: &batch::Outcome| matches!(o.json["status"].as_str(), Some("would_invite" | "would_renew"));
    if !a.dry_run && !a.yes && outcomes.iter().any(would) {
        let lines = outcomes.iter().map(|o| o.line.clone()).collect::<Vec<_>>().join("\n");
        return Err(Fail::NeedsConfirmation {
            message: say::confirm_invites(&lines),
            what: "would_invite",
            would: outcomes.into_iter().map(|o| o.json).collect(),
        });
    }
    let tail = if plan_only { String::new() } else { say::invites_tail() };
    let mut done = batch::finish("invites", outcomes, say::invites_head(a.dry_run, plan_only), &tail)?;
    if a.dry_run {
        done.json["dry_run"] = true.into();
    }
    Ok(done)
}

fn record(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let what = rest.first().map(String::as_str).unwrap_or("");
    let op = format!("record.{what}");
    if !app::OPS.contains(&op.as_str()) {
        return Err(Fail::Usage(say::record_usage()));
    }
    let h = home(a)?;
    let launch = (!a.no_launch && what == "start").then_some(h.app_id.as_str());
    // How a start goes, said only when it isn't the app's default (the count, the label).
    let mut how = serde_json::Map::new();
    if what == "start" {
        if a.no_countdown {
            how.insert("countdown".into(), false.into());
        }
        if a.no_label {
            how.insert("label".into(), false.into());
        }
    }
    let v = app::ask(&h.control_socket(), &op, &how, launch)?;
    let take = v["take"].as_str().unwrap_or("").to_string();
    let text = match what {
        // Without the count, the take is already recording.
        "start" if take == "recording" => say::recording_now(),
        "start" => say::started(),
        "pause" => say::paused(),
        "resume" => say::resumed(),
        _ => say::stopped(v["link"].as_str()),
    };
    let mut j = json!({"op": op, "take": take});
    if let Some(l) = v.get("link") {
        j["link"] = l.clone();
    }
    Ok(Done::new(j, text))
}

fn open_url(url: &str) -> Result<(), Fail> {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    match std::process::Command::new(opener).arg(url).status() {
        Ok(s) if s.success() => Ok(()),
        _ => Err(Fail::Other(say::no_browser(url))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "v.myshroom.workers.dev";

    fn video(id: &str, title: &str) -> Video {
        Video {
            row: shroom_upload::library::Row {
                id: id.into(),
                title: title.into(),
                state: shroom_upload::meta::State::Live,
                created_at: "2026-10-02T12:00:00Z".into(),
                duration_sec: Some(10.0),
                owner: Some("m1".into()),
                poster: None,
                edited_sec: None,
                poster_file: None,
                ai: None,
            },
            link: format!("https://{HOST}/{id}"),
        }
    }

    #[test]
    fn a_video_is_named_by_id_link_or_title_words() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Onboarding walkthrough"), video("QrStUvWxYz012345", "Bug repro")];
        assert_eq!(find(&v, "AbCdEfGhIjKlMnOp", HOST, true).unwrap().row.title, "Onboarding walkthrough");
        assert_eq!(find(&v, "https://v.myshroom.workers.dev/QrStUvWxYz012345", HOST, true).unwrap().row.title, "Bug repro");
        assert_eq!(find(&v, "onboarding", HOST, true).unwrap().row.id, "AbCdEfGhIjKlMnOp");
        assert_eq!(find(&v, "repro bug", HOST, true).unwrap().row.id, "QrStUvWxYz012345", "words in any order");
        assert_eq!(find(&v, "nothing", HOST, true).unwrap_err().code(), "not_found");
    }

    #[test]
    fn a_delete_never_guesses_from_title_words() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Onboarding walkthrough")];
        assert_eq!(find(&v, "onboarding", HOST, false).unwrap_err().code(), "usage");
    }

    #[test]
    fn two_title_matches_name_both_and_pick_neither() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Demo one"), video("QrStUvWxYz012345", "Demo two")];
        let f = find(&v, "demo", HOST, true).unwrap_err();
        assert_eq!(f.code(), "ambiguous");
        assert_eq!(f.json()["matches"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_link_to_another_space_is_not_this_spaces_video() {
        assert_eq!(id_of("https://v.other.workers.dev/AbCdEfGhIjKlMnOp", HOST), None);
        assert_eq!(id_of("v.myshroom.workers.dev/AbCdEfGhIjKlMnOp?t=3", HOST).as_deref(), Some("AbCdEfGhIjKlMnOp"));
    }

    #[test]
    fn unknown_options_are_refused_not_ignored() {
        assert_eq!(parse(vec!["list".into(), "--force".into()]).unwrap_err().code(), "usage");
        let a = parse(vec!["delete".into(), "x".into(), "--yes".into(), "--json".into()]).unwrap();
        assert!(a.yes && a.json);
    }
}
