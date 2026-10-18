//! `shroom` — shroom on the command line, for a person or the coding agent they ask
//! (dev/proposals/cli-agent-skill.md). Everything a list, a link, a rename or a delete
//! needs works whether the app is running or not: the CLI reads shroom's home and asks
//! the space's own page under this Mac's key. Recording goes through the running app.
//!
//! The mechanism is exact and the judgment is the agent's: the CLI never guesses which
//! video was meant for a delete, never deletes without `--yes`, and says why in a stable
//! code (`--json`) when it won't.

mod app;
mod home;
mod out;
mod space;

use serde_json::{Value, json};

use home::Home;
use out::{Done, Fail, col, day, length};
use space::{Door, Video};

const GUIDE: &str = include_str!("../../../site/cli.md");

const HELP: &str = "\
shroom — your shroom videos from the command line.

  shroom list                         your videos, newest first
  shroom list --search demo --limit 5
  shroom info <video>                 one video: title, date, length, link
  shroom link <video>                 just its link
  shroom open <video>                 open it in the browser
  shroom rename <video> <new title>
  shroom delete <id-or-link>... --yes delete for good (try --dry-run first)
  shroom cleanup-candidates           old videos worth a look before deleting
  shroom members                      who is in your space (the owner's Macs)
  shroom invite <name>                a link that lets one person join (7 days, once)
  shroom invite cancel <invite-id>    that link stops working
  shroom members remove <member-id> --yes
                                      they leave the space; their videos stay
  shroom record start|pause|resume|stop
                                      asks the shroom app (opens it if needed)
  shroom status                       is shroom running, recording, set up?
  shroom guide                        how a coding agent should use this tool

<video> is an id, a link, or words from its title (one match only).
A delete takes an id or a link, never title words.

Options
  --json          one JSON object on stdout: {\"ok\":true,…} or
                  {\"ok\":false,\"error\":\"<code>\",\"message\":\"…\"}
  --limit N       at most N rows          --search TEXT  titles containing TEXT
  --mine          only videos you made    --older-than DAYS (cleanup, default 90)
  --dry-run       say what a delete would remove, remove nothing
  --yes           confirm a delete        --no-launch   don't open the app to record

Exit codes
  0 ok · 1 failed · 2 usage · 3 not set up · 4 not found or ambiguous · 5 offline
  6 refused · 7 needs --yes · 8 the app is needed and didn't answer · 9 recording now
  10 recording from here is off (Settings → General)
";

/// The command line, parsed: the verb, its words, and the options.
#[derive(Debug, Default)]
struct Args {
    words: Vec<String>,
    json: bool,
    yes: bool,
    dry_run: bool,
    mine: bool,
    no_launch: bool,
    limit: Option<usize>,
    search: Option<String>,
    older_than: Option<u64>,
    home: Option<String>,
    space_base: Option<String>,
}

fn parse(argv: Vec<String>) -> Result<Args, Fail> {
    let mut a = Args::default();
    let mut it = argv.into_iter();
    while let Some(w) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| Fail::Usage(format!("{name} needs a value.")));
        match w.as_str() {
            "--json" => a.json = true,
            "--yes" | "-y" => a.yes = true,
            "--dry-run" => a.dry_run = true,
            "--mine" => a.mine = true,
            "--no-launch" => a.no_launch = true,
            "--limit" => a.limit = Some(value("--limit")?.parse().map_err(|_| Fail::Usage("--limit takes a number.".into()))?),
            "--search" => a.search = Some(value("--search")?),
            "--older-than" => {
                let v = value("--older-than")?;
                a.older_than = Some(v.trim_end_matches('d').parse().map_err(|_| Fail::Usage("--older-than takes a number of days.".into()))?)
            }
            // Debug builds only (tests and the cloud demo): another home, a local space.
            "--home" if cfg!(debug_assertions) => a.home = Some(value("--home")?),
            "--space-base" if cfg!(debug_assertions) => a.space_base = Some(value("--space-base")?),
            "-h" | "--help" => a.words.insert(0, "help".into()),
            "-V" | "--version" => a.words.insert(0, "version".into()),
            s if s.starts_with("--") => return Err(Fail::Usage(format!("{s} isn't an option. See shroom --help."))),
            _ => a.words.push(w),
        }
    }
    Ok(a)
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let json_wanted = argv.iter().any(|a| a == "--json");
    let result = parse(argv).and_then(|a| run(&a).map(|d| (a.json, d)));
    match result {
        Ok((json, done)) => {
            if json {
                println!("{}", done.json);
            } else if !done.text.is_empty() {
                println!("{}", done.text.trim_end());
            }
        }
        Err(f) => {
            if json_wanted {
                println!("{}", f.json());
            } else {
                eprintln!("{}", f.message());
            }
            std::process::exit(f.exit());
        }
    }
}

fn run(a: &Args) -> Result<Done, Fail> {
    let verb = a.words.first().map(String::as_str).unwrap_or("help");
    let rest = &a.words[a.words.len().min(1)..];
    match verb {
        "help" => Ok(Done::new(json!({"help": HELP}), HELP)),
        "version" => Ok(Done::new(json!({"version": env!("CARGO_PKG_VERSION")}), format!("shroom {}", env!("CARGO_PKG_VERSION")))),
        "guide" | "skill" => Ok(Done::new(json!({"guide": GUIDE}), GUIDE)),
        "status" => status(a),
        "list" | "ls" => list(a),
        "info" | "show" => one(a, rest, |v, _| Ok(Done::new(json!({"video": v.json()}), info_text(v)))),
        "link" => one(a, rest, |v, _| Ok(Done::new(json!({"id": v.row.id, "link": v.link}), v.link.clone()))),
        "open" => one(a, rest, |v, _| {
            open_url(&v.link)?;
            Ok(Done::new(json!({"id": v.row.id, "link": v.link, "opened": true}), format!("Opened {}", v.link)))
        }),
        "rename" => rename(a, rest),
        "delete" | "rm" => delete(a, rest),
        "cleanup-candidates" | "cleanup" => cleanup(a),
        "members" => match rest.first().map(String::as_str) {
            Some("remove") => remove_member(a, &rest[1..]),
            Some(other) => Err(Fail::Usage(format!("members {other} isn't a command. See shroom --help."))),
            None => members(a),
        },
        "invite" => match rest.first().map(String::as_str) {
            Some("cancel") => cancel_invite(a, &rest[1..]),
            _ => invite(a, rest),
        },
        "record" => record(a, rest),
        other => Err(Fail::Usage(format!("{other} isn't a shroom command. See shroom --help."))),
    }
}

fn home(a: &Args) -> Result<Home, Fail> {
    Home::locate(a.home.as_deref()).ok_or_else(|| Fail::NotSetUp("No home folder found.".into()))
}

fn door(a: &Args) -> Result<(Home, Door), Fail> {
    let h = home(a)?;
    let space = h.space().ok_or_else(|| Fail::NotSetUp("shroom isn't set up on this Mac yet. Open shroom to set it up, or install it from https://shroom.am.".into()))?;
    let key = h.device_key(&space).ok_or_else(|| Fail::NotSetUp("This Mac hasn't finished joining its space. Open shroom once, then try again.".into()))?;
    let base = if cfg!(debug_assertions) { a.space_base.clone().or_else(|| std::env::var("SHROOM_SPACE_BASE").ok()) } else { None };
    Ok((h, Door::open(space, key, base)))
}

/// A change waits while a recording is open: shroom stays quiet mid-take (law 4).
fn quiet(h: &Home) -> Result<(), Fail> {
    match h.app_status() {
        Some(s) if s["take_open"] == true => Err(Fail::TakeOpen("A recording is open. Try again once it has stopped.".into())),
        _ => Ok(()),
    }
}

fn status(a: &Args) -> Result<Done, Fail> {
    let h = home(a)?;
    let app = h.app_status();
    let space = h.space();
    let key = space.as_ref().and_then(|s| h.device_key(s));
    let role = space.as_ref().map(|s| match s.role {
        shroom_core::world::Role::TokenHolder | shroom_core::world::Role::OwnerClass => "owner",
        shroom_core::world::Role::Member => "member",
    });
    let take = app.as_ref().and_then(|s| s["take"].as_str()).unwrap_or("none").to_string();
    let json = json!({
        "app": {
            "running": app.is_some(),
            "version": app.as_ref().and_then(|s| s["app"].as_str()),
            "take": take,
            "setup": app.as_ref().and_then(|s| s["setup"].as_str()),
        },
        "space": space.as_ref().map(|s| json!({"host": s.host, "role": role, "ready": key.is_some()})),
        "cli": env!("CARGO_PKG_VERSION"),
    });
    let mut text = Vec::new();
    text.push(match (&app, take.as_str()) {
        (None, _) => "shroom isn't running.".to_string(),
        (Some(_), "none") => "shroom is running.".to_string(),
        (Some(_), t) => format!("shroom is running — {t}."),
    });
    text.push(match (&space, &key) {
        (None, _) => "Not set up on this Mac yet.".to_string(),
        (Some(s), None) => format!("In {} — not finished joining; open shroom once.", s.host),
        (Some(s), Some(_)) => format!("Your space: {} ({}).", s.host, role.unwrap_or("")),
    });
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
    Ok((h, d, v))
}

fn list(a: &Args) -> Result<Done, Fail> {
    let (_, d, mut v) = videos(a)?;
    let total = v.len();
    if let Some(n) = a.limit {
        v.truncate(n);
    }
    let scope = if d.identity.role == shroom_core::world::Role::Member { "mine" } else { "everyone" };
    let mut text = String::new();
    if v.is_empty() {
        text.push_str(if a.search.is_some() || a.mine { "No videos match." } else { "No videos yet." });
    } else {
        text.push_str(&format!("{}  {}  {}  TITLE\n", col("ID", 16), col("DATE", 10), col("LENGTH", 7)));
        for x in &v {
            text.push_str(&format!("{}  {}  {}  {}\n", x.row.id, day(&x.row.created_at), col(&length(x.row.duration_sec), 7), x.row.title));
        }
        if v.len() < total {
            text.push_str(&format!("({} of {total})", v.len()));
        }
    }
    Ok(Done::new(json!({"scope": scope, "count": v.len(), "total": total, "videos": v.iter().map(Video::json).collect::<Vec<_>>()}), text))
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
fn find(v: Vec<Video>, words: &str, host: &str, titles: bool) -> Result<Video, Fail> {
    if let Some(id) = id_of(words, host) {
        return v.into_iter().find(|x| x.row.id == id).ok_or_else(|| Fail::NotFound(format!("No video {id} in your space.")));
    }
    if !titles {
        return Err(Fail::Usage("Name the video by its id or link (see shroom list).".into()));
    }
    let mut hits: Vec<Video> = v.into_iter().filter(|x| titled(&x.row.title, words)).collect();
    match hits.len() {
        0 => Err(Fail::NotFound(format!("No video's title has “{words}” in it."))),
        1 => Ok(hits.remove(0)),
        n => Err(Fail::Ambiguous {
            message: format!("{n} videos match “{words}”. Name one by its id."),
            matches: hits.iter().map(Video::json).collect(),
        }),
    }
}

fn one(a: &Args, rest: &[String], then: impl FnOnce(&Video, &Door) -> Result<Done, Fail>) -> Result<Done, Fail> {
    if rest.is_empty() {
        return Err(Fail::Usage("Which video? Give its id, link or title words.".into()));
    }
    let (_, d, v) = videos(a)?;
    let hit = find(v, &rest.join(" "), &d.identity.host, true)?;
    then(&hit, &d)
}

fn rename(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let [which, title @ ..] = rest else { return Err(Fail::Usage("shroom rename <video> <new title>".into())) };
    if title.is_empty() {
        return Err(Fail::Usage("shroom rename <video> <new title>".into()));
    }
    let (h, d, v) = videos(a)?;
    quiet(&h)?;
    let hit = find(v, which, &d.identity.host, true)?;
    let saved = d.rename(&hit.row.id, &title.join(" "))?;
    Ok(Done::new(json!({"id": hit.row.id, "title": saved, "link": hit.link}), format!("Renamed to “{saved}”.")))
}

fn delete(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    if rest.is_empty() {
        return Err(Fail::Usage("shroom delete <id-or-link>... --yes".into()));
    }
    let (h, d, v) = videos(a)?;
    let mut targets: Vec<Video> = Vec::new();
    for w in rest {
        let hit = find(v.clone(), w, &d.identity.host, false)?;
        if !targets.iter().any(|t| t.row.id == hit.row.id) {
            targets.push(hit);
        }
    }
    let would: Vec<Value> = targets.iter().map(Video::json).collect();
    let names = targets.iter().map(|t| format!("  {}  {}", t.row.id, t.row.title)).collect::<Vec<_>>().join("\n");
    if a.dry_run {
        return Ok(Done::new(json!({"dry_run": true, "would_delete": would}), format!("Would delete, for good:\n{names}")));
    }
    if !a.yes {
        return Err(Fail::NeedsConfirmation {
            message: format!("This deletes for good — its link will say it was removed:\n{names}\nRun it again with --yes once that's what you want."),
            what: "would_delete",
            would,
        });
    }
    quiet(&h)?;
    let mut deleted = Vec::new();
    for t in &targets {
        if let Err(f) = d.delete(&t.row.id) {
            // The ones done stay done; say which.
            return Err(match deleted.is_empty() {
                true => f,
                false => Fail::Other(format!("Deleted {} of {}, then: {}", deleted.len(), targets.len(), f.message())),
            });
        }
        deleted.push(t.row.id.clone());
    }
    Ok(Done::new(json!({"deleted": deleted}), format!("Deleted {} video{}.", deleted.len(), if deleted.len() == 1 { "" } else { "s" })))
}

/// What says a video may be worth deleting. Today only its age; when the space counts
/// views (the cleanup-score proposal, a sibling POC), `views`, `last_viewed` and `score`
/// come from there and the ranking follows the score. The JSON keeps the fields now so an
/// agent's reading of it doesn't change shape later.
fn cleanup(a: &Args) -> Result<Done, Fail> {
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
            j["age_days"] = json!(age);
            j["views"] = Value::Null;
            j["last_viewed"] = Value::Null;
            j["score"] = Value::Null;
            j["reasons"] = json!([format!("older than {days} days")]);
            j
        })
        .collect();
    let text = if rows.is_empty() {
        format!("Nothing older than {days} days.")
    } else {
        let mut t = format!("{} video{} older than {days} days (oldest first):\n", rows.len(), if rows.len() == 1 { "" } else { "s" });
        for (age, x) in &rows {
            t.push_str(&format!("  {}  {}  {:>4} days  {}\n", x.row.id, day(&x.row.created_at), age, x.row.title));
        }
        t.push_str("Nothing was deleted. shroom delete <id> --yes removes one for good.");
        t
    };
    Ok(Done::new(json!({"signals": "age", "older_than_days": days, "candidates": candidates}), text))
}

fn members(a: &Args) -> Result<Done, Fail> {
    let (_, d) = door(a)?;
    let m = d.members()?;
    let people: Vec<Value> = m.members.iter().map(|x| json!({"id": x.id, "name": x.label, "joined_at": x.joined_at})).collect();
    let invites: Vec<Value> =
        m.invites.iter().map(|i| json!({"id": i.id, "kind": i.kind, "name": i.label, "expires_at": i.expires_at, "expired": i.expired})).collect();
    let macs: Vec<Value> = m.owner.devices.iter().map(|x| json!({"id": x.id, "name": x.name, "this_mac": x.this_mac})).collect();
    let mut text = format!("{} (owner) · {} Mac{}\n", m.owner.name.clone().unwrap_or_else(|| "You".into()), macs.len(), if macs.len() == 1 { "" } else { "s" });
    for x in &m.members {
        text.push_str(&format!("{}  {}  joined {}\n", x.id, x.label, day(&x.joined_at)));
    }
    for i in m.invites.iter().filter(|i| !i.expired) {
        text.push_str(&format!("{}  {}  invited, waiting\n", i.id, i.label.clone().unwrap_or_else(|| "Another Mac".into())));
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
        return Err(Fail::Usage("Who is it for? shroom invite <their name>".into()));
    }
    let (h, d) = door(a)?;
    quiet(&h)?;
    let (row, link) = d.invite(name)?;
    Ok(Done::new(
        json!({"id": row.id, "name": row.label, "link": link, "expires_at": row.expires_at}),
        format!("An invite for {name}, good until {} and for one use:\n{link}\nSend it to them yourself.", day(&row.expires_at)),
    ))
}

fn cancel_invite(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let [id] = rest else { return Err(Fail::Usage("shroom invite cancel <invite-id> (see shroom members)".into())) };
    let (h, d) = door(a)?;
    quiet(&h)?;
    d.cancel_invite(id)?;
    Ok(Done::new(json!({"cancelled": id}), "Cancelled: that invite link no longer works."))
}

/// Removes a member: like a delete, it asks first (`--yes`), and names them by id only.
fn remove_member(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let [id] = rest else { return Err(Fail::Usage("shroom members remove <member-id> --yes (see shroom members)".into())) };
    let (h, d) = door(a)?;
    let m = d.members()?;
    let Some(who) = m.members.iter().find(|x| &x.id == id) else {
        return Err(Fail::NotFound(format!("No member {id} in your space (see shroom members).")));
    };
    let would = json!({"id": who.id, "name": who.label});
    if a.dry_run {
        return Ok(Done::new(json!({"dry_run": true, "would_remove": would}), format!("Would remove {} from the space.", who.label)));
    }
    if !a.yes {
        return Err(Fail::NeedsConfirmation {
            message: format!("This removes {} from the space: their Macs stop recording into it. Their videos stay.\nRun it again with --yes once that's what you want.", who.label),
            what: "would_remove",
            would: vec![would],
        });
    }
    quiet(&h)?;
    d.remove_member(id)?;
    Ok(Done::new(json!({"removed": id}), format!("{} is no longer in the space.", who.label)))
}

fn record(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let what = rest.first().map(String::as_str).unwrap_or("");
    let op = format!("record.{what}");
    if !app::OPS.contains(&op.as_str()) {
        return Err(Fail::Usage("shroom record start|pause|resume|stop".into()));
    }
    let h = home(a)?;
    let v = app::ask(&h.control_socket(), &op, !a.no_launch && what == "start")?;
    let take = v["take"].as_str().unwrap_or("").to_string();
    let text = match what {
        "start" => "Recording starts after the countdown.".to_string(),
        "pause" => "Paused.".to_string(),
        "resume" => "Recording again.".to_string(),
        _ => match v["link"].as_str() {
            Some(l) => format!("Stopped. Its link: {l}"),
            None => "Stopped.".to_string(),
        },
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
        _ => Err(Fail::Other(format!("Couldn't open a browser here. The link: {url}"))),
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
            },
            link: format!("https://{HOST}/{id}"),
        }
    }

    #[test]
    fn a_video_is_named_by_id_link_or_title_words() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Onboarding walkthrough"), video("QrStUvWxYz012345", "Bug repro")];
        assert_eq!(find(v.clone(), "AbCdEfGhIjKlMnOp", HOST, true).unwrap().row.title, "Onboarding walkthrough");
        assert_eq!(find(v.clone(), "https://v.myshroom.workers.dev/QrStUvWxYz012345", HOST, true).unwrap().row.title, "Bug repro");
        assert_eq!(find(v.clone(), "onboarding", HOST, true).unwrap().row.id, "AbCdEfGhIjKlMnOp");
        assert_eq!(find(v.clone(), "repro bug", HOST, true).unwrap().row.id, "QrStUvWxYz012345", "words in any order");
        assert_eq!(find(v, "nothing", HOST, true).unwrap_err().code(), "not_found");
    }

    #[test]
    fn a_delete_never_guesses_from_title_words() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Onboarding walkthrough")];
        assert_eq!(find(v, "onboarding", HOST, false).unwrap_err().code(), "usage");
    }

    #[test]
    fn two_title_matches_name_both_and_pick_neither() {
        let v = vec![video("AbCdEfGhIjKlMnOp", "Demo one"), video("QrStUvWxYz012345", "Demo two")];
        let f = find(v, "demo", HOST, true).unwrap_err();
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
