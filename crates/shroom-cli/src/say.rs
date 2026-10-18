//! Every line the `shroom` command says (spec N62, DECISIONS B215) — to a person in a
//! terminal, and in `--json`'s `message` to their coding agent — in one place, for Bagrat to
//! read and ratify with N62. The product's voice: warm, brief, product words only; a code
//! an agent branches on is never here (`out.rs`). No other file of the command writes a
//! sentence of its own (`tests/voice.rs`).

// ── help ──

pub const HELP: &str = "\
shroom — your shroom videos from the command line.

  shroom list                         your videos, newest first
  shroom list --search demo --limit 5
  shroom info <video>                 one video: title, date, length, link
  shroom link <video>                 just its link
  shroom open <video>                 open it in the browser
  shroom rename <video> <new title>
  shroom delete <id-or-link>... --yes delete for good (try --dry-run first)
  shroom cleanup-candidates           videos worth a look before deleting
  shroom members                      who is in your space (the owner's Macs)
  shroom invite <name>                a link that lets one person join (7 days, once)
  shroom invite cancel <invite-id>... those links stop working
  shroom members remove <member-id>... --yes
                                      they leave the space; their videos stay
  shroom record start|pause|resume|stop
                                      asks the shroom app (opens it if needed)
  shroom status                       is shroom running, recording, set up?
  shroom guide                        how a coding agent should use this tool

Many at once (each item its own result; safe to run again; --dry-run first)
  shroom invite --batch \"Ann Lee\" \"Sam Ng\" ... --yes   a link each (at most 20)
  shroom delete <id>... --yes                     several videos, one confirmation
  shroom rename --batch \"<id>=<new title>\" ...    several titles
  shroom links <video>...                         the links of several videos
  shroom list --mine --since 7d                   what you made this week
  --from <file>   the items from a file (- for stdin): a JSON array, or one per line

<video> is an id, a link, or words from its title (one match only).
A delete takes an id or a link, never title words.

Options
  --json          one JSON object on stdout: {\"ok\":true,…} or
                  {\"ok\":false,\"error\":\"<code>\",\"message\":\"…\"}
  --limit N       at most N rows          --search TEXT  titles containing TEXT
  --mine          only videos you made    --older-than DAYS (cleanup; default 90
                                          where your space doesn't suggest)
  --since 7d|DATE only videos since then  --renew  a new link for someone already invited
  --dry-run       say what would change, change nothing
  --yes           confirm a delete, a removal or a batch invite
  --no-launch     don't open the app to record

Exit codes
  0 ok · 1 failed · 2 usage · 3 not set up · 4 not found or ambiguous · 5 offline
  6 refused · 7 needs --yes · 8 the app is needed and didn't answer · 9 recording now
  10 recording from here is off (Settings → General) · 11 a batch partly went through
";

pub fn version(v: &str) -> String {
    format!("shroom {v}")
}

// ── the command line ──

pub fn needs_value(option: &str) -> String {
    format!("{option} needs a value.")
}

pub fn limit_is_a_number() -> String {
    "--limit takes a number.".into()
}

pub fn older_than_is_days() -> String {
    "--older-than takes a number of days.".into()
}

pub fn since_is_days_or_a_date() -> String {
    "--since takes days (7d) or a date (2026-10-01).".into()
}

pub fn not_an_option(option: &str) -> String {
    format!("{option} isn't an option. See shroom --help.")
}

pub fn not_a_command(word: &str) -> String {
    format!("{word} isn't a shroom command. See shroom --help.")
}

pub fn not_a_members_command(word: &str) -> String {
    format!("members {word} isn't a command. See shroom --help.")
}

// ── this Mac ──

pub fn no_home_folder() -> String {
    "This Mac's home folder couldn't be found.".into()
}

pub fn not_set_up() -> String {
    "shroom isn't set up on this Mac yet. Open shroom to set it up, or install it from https://shroom.am.".into()
}

pub fn not_joined() -> String {
    "This Mac hasn't finished joining its space. Open shroom once, then try again.".into()
}

pub fn take_open() -> String {
    "A recording is open. Try again once it has stopped.".into()
}

/// `status`: the app's line.
pub fn status_app(running: bool, take: &str) -> String {
    match (running, take) {
        (false, _) => "shroom isn't running.".into(),
        (true, "none") => "shroom is running.".into(),
        (true, t) => format!("shroom is running — {t}."),
    }
}

/// `status`: the space's line.
pub fn status_space(host: Option<&str>, joined: bool, owner: bool) -> String {
    match (host, joined) {
        (None, _) => "Not set up on this Mac yet.".into(),
        (Some(h), false) => format!("In {h} — not finished joining; open shroom once."),
        (Some(h), true) => format!("Your space: {h} ({}).", if owner { "owner" } else { "member" }),
    }
}

// ── the space ──

pub fn unreachable() -> String {
    "The space can't be reached. Check the connection and try again.".into()
}

pub fn busy_space() -> String {
    "The space is busy. Try again in a minute.".into()
}

pub fn busy_or_unreachable() -> String {
    "The space is busy or out of reach. Try again in a minute.".into()
}

pub fn not_in_the_space() -> String {
    "This Mac is no longer in the space.".into()
}

pub fn unknown_mac() -> String {
    "The space doesn't know this Mac yet. Open shroom once, then try again.".into()
}

pub fn owner_only() -> String {
    "Only the space's owner can do that.".into()
}

pub fn no_such_invite_or_member() -> String {
    "There's no such invite or member.".into()
}

pub fn too_many_invites() -> String {
    "Too many invites are waiting. Cancel one first.".into()
}

pub fn cant_change_now() -> String {
    "That can't change now: the invite was already used, or that's the owner.".into()
}

pub fn name_too_long() -> String {
    "The space didn't take that. A name is 1 to 64 characters.".into()
}

pub fn space_said_no() -> String {
    "The space said no.".into()
}

pub fn not_answering_as_shroom() -> String {
    "The space's page isn't answering as shroom. Open shroom to check on it.".into()
}

pub fn unexpected_answer() -> String {
    "The space didn't answer as expected. Try again in a minute.".into()
}

pub fn someone_elses() -> String {
    "That video is someone else's: only they or the space's owner can change it.".into()
}

pub fn already_gone() -> String {
    "That video is already gone.".into()
}

/// A change that stopped partway: the next run finishes what this one started.
pub fn didnt_finish() -> String {
    "That didn't finish. Run it again: what's done stays done.".into()
}

// ── finding a video ──

pub fn no_video(id: &str) -> String {
    format!("No video {id} in your space.")
}

pub fn by_id_or_link() -> String {
    "Name the video by its id or link (see shroom list).".into()
}

pub fn no_title_has(words: &str) -> String {
    format!("No video's title has “{words}” in it.")
}

pub fn several_match(n: usize, words: &str) -> String {
    format!("{n} videos match “{words}”. Name one by its id.")
}

pub fn which_video() -> String {
    "Which video? Give its id, link or title words.".into()
}

// ── list, info, open ──

pub fn none_match() -> String {
    "No videos match.".into()
}

pub fn no_videos_yet() -> String {
    "No videos yet.".into()
}

/// The list's head row.
pub const LIST_HEAD: [&str; 4] = ["ID", "DATE", "LENGTH", "TITLE"];

pub fn shown_of(shown: usize, total: usize) -> String {
    format!("({shown} of {total})")
}

pub fn opened(link: &str) -> String {
    format!("Opened {link}")
}

pub fn no_browser(link: &str) -> String {
    format!("Couldn't open a browser here. The link: {link}")
}

// ── rename ──

pub fn rename_usage() -> String {
    "shroom rename <video> <new title>".into()
}

pub fn renamed(title: &str) -> String {
    format!("Renamed to “{title}”.")
}

pub fn rename_item_shape() -> String {
    "Each rename is <id>=<new title> (or {\"id\", \"title\"}).".into()
}

pub fn renames_head(dry_run: bool) -> String {
    if dry_run { "Would rename (nothing was changed):".into() } else { "Renamed:".into() }
}

pub fn line_renamed(was: &str, title: &str) -> String {
    format!("  “{was}” → “{title}”")
}

pub fn line_already_so(title: &str) -> String {
    format!("  “{title}” — already so")
}

/// One item of a batch that didn't go through, and why.
pub fn line_failed(what: &str, why: &str) -> String {
    format!("  {what} — {why}")
}

// ── links ──

pub fn links_head() -> String {
    "Links:".into()
}

pub fn line_link(title: &str, link: &str) -> String {
    format!("  {title}  {link}")
}

// ── delete ──

pub fn delete_usage() -> String {
    "shroom delete <id-or-link>... --yes".into()
}

pub fn line_video(id: &str, title: &str) -> String {
    format!("  {id}  {title}")
}

pub fn not_here_already_gone(ids: &[String]) -> String {
    if ids.is_empty() { String::new() } else { format!("\nNot in your space (already gone?): {}", ids.join(", ")) }
}

pub fn would_delete(names: &str, gone: &str) -> String {
    format!("Would delete, for good:\n{names}{gone}")
}

pub fn confirm_delete(names: &str, gone: &str) -> String {
    format!("This deletes for good — each link will say it was removed:\n{names}{gone}\nRun it again with --yes once that's what you want.")
}

pub fn nothing_to_delete() -> String {
    "Nothing to delete: none of these is in your space (already gone?).".into()
}

pub fn line_not_found(id: &str) -> String {
    format!("  not found  {id}")
}

pub fn line_deleted(id: &str, title: &str) -> String {
    format!("  deleted  {id}  {title}")
}

pub fn deleted(n: usize) -> String {
    format!("Deleted {n} video{}.", if n == 1 { "" } else { "s" })
}

// ── cleanup candidates ──

/// The page's own suggestion, in words (its codes are stable; the words are ours).
pub fn why(code: &str, quiet_days: Option<u64>) -> String {
    match (code, quiet_days) {
        ("never-played", _) => "never played".into(),
        ("stopped-early", _) => "people stop watching early".into(),
        ("not-lately", Some(d)) => format!("not played in {d} days"),
        ("not-lately", None) => "not played lately".into(),
        ("big-and-quiet", _) => "large, and quiet lately".into(),
        _ => "suggested by your space".into(),
    }
}

/// Without the page's suggestions: its age alone.
pub fn why_old(days: u64) -> String {
    format!("older than {days} days")
}

pub fn suggestions_head(n: usize) -> String {
    format!("{n} video{} worth a look before deleting (most likely first):", if n == 1 { "" } else { "s" })
}

pub fn old_head(n: usize, days: u64) -> String {
    format!("{n} video{} older than {days} days (oldest first):", if n == 1 { "" } else { "s" })
}

pub fn line_candidate(id: &str, day: &str, title: &str, why: &str) -> String {
    format!("  {id}  {day}  {title} — {why}")
}

pub fn no_suggestions(older_than: Option<u64>) -> String {
    match older_than {
        Some(d) => format!("Nothing to suggest older than {d} days."),
        None => "Nothing to suggest: your videos are being watched, kept, or new.".into(),
    }
}

pub fn nothing_older(days: u64) -> String {
    format!("Nothing older than {days} days.")
}

pub fn nothing_was_deleted() -> String {
    "Nothing was deleted. shroom delete <id> --yes removes one for good.".into()
}

// ── members ──

pub fn owner_line(name: Option<&str>, macs: usize) -> String {
    format!("{} (owner) · {macs} Mac{}", name.unwrap_or("You"), if macs == 1 { "" } else { "s" })
}

pub fn member_line(id: &str, name: &str, joined: &str) -> String {
    format!("{id}  {name}  joined {joined}")
}

pub fn invited_line(id: &str, name: Option<&str>) -> String {
    format!("{id}  {}  invited, waiting", name.unwrap_or("Another Mac"))
}

pub fn remove_usage() -> String {
    "shroom members remove <member-id>... --yes (see shroom members)".into()
}

pub fn not_here_already_removed(ids: &[String]) -> String {
    if ids.is_empty() { String::new() } else { format!("\nNot in your space (already removed?): {}", ids.join(", ")) }
}

pub fn would_remove(names: &str, gone: &str) -> String {
    format!("Would remove from the space:\n{names}{gone}")
}

pub fn no_such_member(gone: &str) -> String {
    format!("No such member in your space (see shroom members).{gone}")
}

pub fn confirm_remove(names: &str, gone: &str) -> String {
    format!("This removes them from the space: their Macs stop recording into it. Their videos stay.\n{names}{gone}\nRun it again with --yes once that's what you want.")
}

pub fn line_removed(name: &str) -> String {
    format!("  removed  {name}")
}

pub fn removed_head() -> String {
    "No longer in the space:".into()
}

// ── invites ──

pub fn invite_who() -> String {
    "Who is it for? shroom invite <their name>".into()
}

pub fn invite_made(name: &str, until: &str, link: &str) -> String {
    format!("An invite for {name}, good until {until} and for one use:\n{link}\nSend it to them yourself.")
}

pub fn cancel_usage() -> String {
    "shroom invite cancel <invite-id>... (see shroom members)".into()
}

pub fn line_cancelled(id: &str) -> String {
    format!("  cancelled  {id}")
}

pub fn cancelled_head() -> String {
    "Those invite links no longer work:".into()
}

pub fn invite_item_shape() -> String {
    "Each person is a name (or {\"name\": …}).".into()
}

pub fn not_a_name(name: &str) -> String {
    format!("“{name}” isn't a name shroom can keep: 1 to 64 characters.")
}

pub fn too_many_at_once(max: usize, asked: usize) -> String {
    format!("At most {max} invites wait at once in a space; this asks for {asked}. Send the first {max} now.")
}

pub fn line_already_member(name: &str) -> String {
    format!("  {name} — already in the space")
}

pub fn line_already_invited(name: &str) -> String {
    format!("  {name} — already invited, waiting (--renew for a new link)")
}

pub fn line_would_renew(name: &str) -> String {
    format!("  {name} — would get a new link")
}

pub fn line_would_invite(name: &str) -> String {
    format!("  {name} — would get a link")
}

pub fn line_invite(name: &str, link: &str) -> String {
    format!("  {name}  {link}")
}

pub fn confirm_invites(lines: &str) -> String {
    format!("This gives each of these people their own link to join your space, for 7 days:\n{lines}\nRun it again with --yes once that's who you mean.")
}

pub fn invites_head(dry_run: bool, nothing_to_make: bool) -> String {
    match (dry_run, nothing_to_make) {
        (true, _) => "Would invite (nothing was made):".into(),
        (false, true) => "Nothing to make: each is already in the space or invited.".into(),
        (false, false) => "Invites, each for one person, good for 7 days:".into(),
    }
}

pub fn invites_tail() -> String {
    "Send each link to its person yourself; each works once.".into()
}

// ── many at once ──

pub fn couldnt_read_stdin() -> String {
    "Couldn't read the list from stdin.".into()
}

pub fn couldnt_read(path: &str) -> String {
    format!("Couldn't read {path}.")
}

pub fn nothing_to_do() -> String {
    "Nothing to do: give the items as arguments with --batch, or --from <file>.".into()
}

pub fn too_many_items(max: usize, asked: usize) -> String {
    format!("At most {max} at a time; this has {asked}. Split it.")
}

pub fn not_a_json_array() -> String {
    "The list starts like JSON but isn't a JSON array.".into()
}

pub fn item_shape() -> String {
    "Each item is a string or an object.".into()
}

pub fn partly(failed: usize, of: usize, lines: &str) -> String {
    format!("{failed} of {of} didn't go through; the rest did. Run it again for the ones that failed.\n{lines}")
}

// ── recording, through the app ──

pub fn record_usage() -> String {
    "shroom record start|pause|resume|stop".into()
}

pub fn app_not_running() -> String {
    "shroom isn't running. Open it, or leave out --no-launch.".into()
}

pub fn app_only_on_a_mac() -> String {
    "shroom isn't running, and it runs only on a Mac.".into()
}

pub fn app_wont_open() -> String {
    "shroom couldn't be opened. Is it installed?".into()
}

pub fn app_slow_to_answer() -> String {
    "shroom opened but didn't answer in time.".into()
}

pub fn app_didnt_take_it() -> String {
    "shroom didn't take the request.".into()
}

pub fn app_didnt_answer() -> String {
    "shroom didn't answer.".into()
}

pub fn app_answer_unreadable() -> String {
    "shroom's answer wasn't readable.".into()
}

pub fn already_recording() -> String {
    "A recording is already going.".into()
}

pub fn app_not_set_up() -> String {
    "shroom isn't set up on this Mac yet. Open it to set it up.".into()
}

pub fn recording_off() -> String {
    "shroom isn't set to let your coding agent record. Turn on “Let it start and stop recordings” in shroom's Settings, under General.".into()
}

pub fn between_two_steps() -> String {
    "shroom is between two steps. Try again in a moment.".into()
}

pub fn nothing_recording() -> String {
    "Nothing is recording.".into()
}

pub fn nothing_paused() -> String {
    "Nothing is paused.".into()
}

/// Where the popover wouldn't offer what was asked right now.
pub fn not_now(op: &str) -> String {
    match op {
        "record.start" => "shroom can't start a recording right now. Open it from the menu bar to see why.".into(),
        "record.resume" => "shroom can't go on recording right now. Open it from the menu bar to see why.".into(),
        _ => "shroom can't stop this recording from here right now. Open it from the menu bar to see why.".into(),
    }
}

pub fn app_said_no() -> String {
    "shroom said no.".into()
}

pub fn started() -> String {
    "Recording starts after the countdown.".into()
}

pub fn paused() -> String {
    "Paused.".into()
}

pub fn resumed() -> String {
    "Recording again.".into()
}

pub fn stopped(link: Option<&str>) -> String {
    match link {
        Some(l) => format!("Stopped. Its link: {l}"),
        None => "Stopped.".into(),
    }
}
