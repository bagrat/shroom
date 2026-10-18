//! Every line shroom shows, in one place (TESTING-PLAN §2.4). Lines are the spec's,
//! verbatim; `tests/copy.rs` renders every one with the spec's example arguments and finds
//! it among its state's said-lines in `spec/r*.json`. Write the said-line, never the
//! mechanism (CLAUDE.md).

/// Law 16: in sentences, one to nine are words; compact facts keep figures.
pub fn count_word(n: u64) -> String {
    const WORDS: [&str; 10] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
    WORDS.get(n as usize).map(|w| w.to_string()).unwrap_or_else(|| n.to_string())
}

/// A take's length as the popover shows it: 4:32, 1:02:09.
pub fn length(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, ss) = (s / 3600, s % 3600 / 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{ss:02}") } else { format!("{m}:{ss:02}") }
}

// ── macOS prompts (Info.plist usage strings — said-lines, ratified) ──
pub fn mic_purpose() -> String { "shroom records your voice with your video.".into() }
pub fn camera_purpose() -> String { "shroom shows and records your camera only when you choose it.".into() }

// ── config ──
pub fn record() -> String { "Record".into() }
/// The source picker's camera choice, and the camera picker's and the microphone's “none”
/// choices — Bagrat's words (2026-10-13, DEVIATIONS #43), not yet in the spec.
pub fn camera_only() -> String { "Camera only".into() }
pub fn camera_off_pick() -> String { "Camera off".into() }
pub fn no_mic_pick() -> String { "No mic".into() }
/// The camera picker's accessible name; no row shows it (DEVIATIONS #45). Was “Show yourself”
/// until Bagrat renamed it (B181).
pub fn camera() -> String { "Camera".into() }
/// The first item of the microphone's and the camera's menus (N44).
pub fn system_default() -> String { "System default".into() }
pub fn microphone() -> String { "Microphone".into() }
pub fn quality() -> String { "Quality".into() }
pub fn quality_name(q: crate::world::Quality) -> String {
    match q {
        crate::world::Quality::Normal => "Normal",
        crate::world::Quality::TwoK => "2K",
        crate::world::Quality::FourK => "4K",
    }
    .into()
}
pub fn start() -> String { "Start recording".into() }
pub fn countdown_hint() -> String { "Starts after 3-2-1 — click anywhere to cancel.".into() }
pub fn library() -> String { "Library".into() }
pub fn settings() -> String { "Settings".into() }
pub fn quit() -> String { "Quit shroom".into() }
pub fn no_mic() -> String { "No microphone — your video will be silent.".into() }
pub fn allowed_in_settings() -> String { "allowed in System Settings".into() }
pub fn camera_off() -> String { "Camera is off for shroom —".into() }
pub fn open_system_settings() -> String { "Open System Settings".into() }
/// Displays of one name that place can't tell apart are numbered (N39): "LG UltraFine 2".
pub fn display_numbered(name: &str, n: u64) -> String { format!("{name} {n}") }
/// Two displays of one name are told apart by side (N39): "LG UltraFine (left)".
pub fn display_name(name: &str, side: Option<&str>) -> String {
    match side {
        Some(s) => format!("{name} ({s})"),
        None => name.to_string(),
    }
}
pub fn uploading_one(pct: u32) -> String { format!("Uploading — {pct}%. Your link already works.") }
pub fn uploading_many(n: u64, pct: u32) -> String { format!("Uploading {} videos — {pct}%.", count_word(n)) }
pub fn waiting_connection() -> String { "Waiting for connection — your links already work.".into() }
/// Law 16: one to nine are words (“About seven minutes…”), and one minute is one.
pub fn room_left(minutes: u64) -> String {
    match minutes {
        1 => "About one minute of room left on this Mac.".into(),
        m => format!("About {} minutes of room left on this Mac.", count_word(m)),
    }
}
pub fn out_of_room() -> String { "This Mac is out of room for recording — free up some space first.".into() }

// ── banners ──
pub fn recording_off() -> String { "macOS has screen recording switched off for shroom.".into() }
pub fn turn_back_on() -> String { "Turn it back on".into() }
pub fn recovered(len: &str) -> String { format!("We saved your interrupted recording ({len}).") }
pub fn get_link() -> String { "Get link".into() }
pub fn discard_ellipsis() -> String { "Discard…".into() }

// ── countdown ──
pub fn countdown_cancel() -> String { "Click anywhere to cancel.".into() }

// ── paused ──
pub fn paused_title(len: &str) -> String { format!("Paused — {len}") }
pub fn resume() -> String { "Resume".into() }
pub fn restart() -> String { "Restart".into() }
pub fn stop() -> String { "Finish & Share".into() }
/// Restart's question — the canvas's draft (DEVIATIONS #44), awaiting ratification.
pub fn restart_title() -> String { "Start over?".into() }
pub fn restart_line(len: &str) -> String { format!("This replaces the {len} you've recorded so far.") }
pub fn display_lost() -> String { "Your display disconnected — recording paused.".into() }
pub fn mic_lost() -> String { "Your microphone disconnected — recording paused.".into() }
pub fn mic_fallback(mic: &str) -> String { format!("Resume uses {mic}.") }
pub fn mic_none() -> String { "Resume continues without your microphone.".into() }
/// A take that died with nothing a repair could save (DEVIATIONS #53, ratified 2026-10-14).
pub fn nothing_recorded(camera: bool) -> String {
    if camera {
        "Your camera didn't send a picture, so nothing was recorded — try again.".into()
    } else {
        "Your screen didn't send a picture, so nothing was recorded — try again.".into()
    }
}
pub fn camera_lost() -> String { "Your camera disconnected — recording paused.".into() }
pub fn slept() -> String { "Your Mac went to sleep — recording paused.".into() }
pub fn locked() -> String { "Your screen locked — recording paused.".into() }
pub fn mic_permission_lost() -> String { "macOS switched off the microphone — recording paused.".into() }

// ── discard ──
pub fn toss_title() -> String { "Toss this recording?".into() }
pub fn toss_so_far(len: &str) -> String { format!("{len} recorded so far. There's no undo.") }
pub fn toss_saved(len: &str) -> String { format!("{len} was saved. There's no undo.") }
pub fn back() -> String { "Back".into() }
pub fn restart_instead() -> String { "Restart instead".into() }
pub fn discard() -> String { "Discard".into() }

// ── stops the person didn't make ──
pub fn screen_stopped(len: &str) -> String {
    format!("macOS switched off screen recording, so the recording stopped at {len} — everything so far is saved.")
}
pub fn disk_stopped(len: &str) -> String {
    format!("This Mac ran out of room, so the recording stopped at {len} — everything so far is saved.")
}

// ── offline stop ──
pub fn link_ready() -> String { "Your link is ready.".into() }
pub fn offline_line() -> String {
    "You're offline — it's already safe to share. The page opens by itself the moment you're back.".into()
}
pub fn copy_link() -> String { "Copy link".into() }
pub fn copied() -> String { "Copied".into() }
pub fn copy_thumbnail() -> String { "Copy thumbnail".into() }

// ── quit ──
pub fn quit_uploading(n: u64) -> String { format!("Quit with {} videos still uploading?", count_word(n)) }
pub fn quit_line() -> String {
    "They'll finish the next time shroom opens — until then, their links say “on its way.”".into()
}
pub fn quit_anyway() -> String { "Quit anyway".into() }

// ── the welcome (Region 1) ──
pub fn welcome_what() -> String {
    "shroom records your screen and hands you a link to share — no account with us, no subscription. Your videos live in your own Cloudflare storage: free for about three hours of video, and pennies after that — about 30 hours for $1.50 a month.".into()
}
/// The first paragraph's ratified emphasis: “shroom” bold, “your own” italic.
pub const WELCOME_WHAT_MARKS: &[(&str, crate::scene::Em)] = &[("shroom", crate::scene::Em::Strong), ("your own", crate::scene::Em::Em)];
pub fn welcome_steps() -> String {
    "Two steps before your first video: set up your Cloudflare, and let macOS allow recording. About ten minutes, once.".into()
}
pub fn login_box() -> String { "Start shroom when you log in".into() }
pub fn set_up() -> String { "Set up my space".into() }
pub fn later() -> String { "Later".into() }

// ── the plan (Region 2's frame; row six is Region 3's chapter) ──
pub fn plan_header() -> String { "Your space — set up once, yours forever:".into() }
fn row_title(row: u8) -> &'static str {
    match row {
        1 => "A Cloudflare account",
        2 => "One access token for shroom",
        3 => "A credit card for storage",
        4 => "Your link name",
        5 => "Wiring it up",
        _ => "Letting macOS allow recording",
    }
}
fn row_paren(row: u8) -> &'static str {
    match row {
        1 => "free — they'll want a verified email or a Google sign-in",
        2 => "a pre-filled page — review, create, paste",
        3 => "their rule, not a charge — free up to about three hours of video",
        4 => "part of every link you share — v.yourname.workers.dev",
        5 => "shroom does this part",
        _ => "microphone, camera and screen — a one-time yes each",
    }
}
/// A row still ahead: "• title (parenthetical)".
pub fn row_ahead(row: u8) -> String { format!("• {} ({})", row_title(row), row_paren(row)) }
/// The wiring row while wiring runs out of sight, under the permissions chapter (N45).
pub fn row_working() -> String { format!("• {} (shroom is doing this part now)", row_title(5)) }
/// A row ahead's emphasis: the name row's example name is italic (the spec's *yourname*).
pub fn row_ahead_marks(row: u8) -> &'static [(&'static str, crate::scene::Em)] {
    if row == 4 { &[("yourname", crate::scene::Em::Em)] } else { &[] }
}
/// The current row. The name row's parenthetical sheds only its example — the live URL
/// below replaces it (N15).
pub fn row_current(row: u8) -> String {
    let paren = if row == 4 { "part of every link you share" } else { row_paren(row) };
    format!("→ {} ({paren})", row_title(row))
}
/// A row behind you: its parenthetical drops off (N13); some rows carry a fact.
pub fn row_done(row: u8, fact: Option<&str>) -> String {
    match fact {
        Some(f) => format!("✓ {} — {f}", row_title(row)),
        None => format!("✓ {}", row_title(row)),
    }
}
/// A row as Settings → Space shows it once it's done: its own done-state words, not the
/// wizard's (Bagrat, 2026-10-14, B139). A row still to do keeps the wizard's.
pub fn space_done(row: u8, fact: Option<&str>) -> String {
    let title = match row {
        1 => "Cloudflare account",
        2 => "Access token valid",
        3 => "Cloudflare billing set up",
        4 => "Your link name",
        5 => "Wired up",
        _ => "macOS permissions granted",
    };
    match fact {
        Some(f) => format!("✓ {title} — {f}"),
        None => format!("✓ {title}"),
    }
}
/// The account row's fact in Settings: the email alone when the account still has
/// Cloudflare's own name for it (“You@example.com's Account”), lowercased as
/// an email is written; any other name as it is (B139).
pub fn account_fact(name: &str) -> String {
    match name.strip_suffix("'s Account").or_else(|| name.strip_suffix("’s Account")) {
        Some(email) if email.contains('@') && !email.contains(char::is_whitespace) => email.to_lowercase(),
        _ => name.to_string(),
    }
}
/// The link-name row's fact; beside a foreign `v` it says so, once (N42).
pub fn name_fact(host: &str, foreign_v: bool) -> String {
    if foreign_v { format!("{host} (“v” was already taken in your Cloudflare)") } else { host.to_string() }
}
pub fn mic_off_fact() -> String { "microphone off for now".into() }

pub fn have_account() -> String { "Do you already have a Cloudflare account?".into() }
pub fn not_yet() -> String { "Not yet — take me there".into() }
pub fn have_one() -> String { "I have one".into() }
pub fn signup_open() -> String {
    "Cloudflare's signup is open in your browser. Finish there, then confirm your email from the message they send — they make no token until you do. I'll wait.".into()
}
pub fn signup_again() -> String { "Take me there again".into() }
pub fn signed_up() -> String { "I signed up".into() }
pub fn token_line() -> String {
    "Your space runs on one access token. The page comes pre-filled — review what it grants, two narrow permissions and nothing more, create the token, and paste it here.".into()
}
pub fn create_token() -> String { "Create token in Cloudflare".into() }
pub fn paste_token() -> String { "Paste your token".into() }
/// N45: quiet, under the token field, on the sign-up path until a paste lands.
pub fn token_email_first() -> String { "Cloudflare won't make it yet? Confirm your email first — their message is in your inbox.".into() }
pub fn token_garbled() -> String { "Tokens sometimes lose characters in the copy — copy it once more.".into() }
pub fn token_wrong_powers() -> String { "This token can't manage your video space — create a fresh one and paste that.".into() }
pub fn pick_account() -> String {
    "This token works for more than one Cloudflare account — which one should hold your videos?".into()
}
pub fn use_account() -> String { "Use this account".into() }
pub fn gate_email() -> String {
    "Cloudflare wants your email verified before it opens storage. Check your inbox for their message — I'll notice the moment it's done and carry on.".into()
}
pub fn gate_card() -> String {
    "One step left on Cloudflare's side: adding a card switches storage on. The free tier stays free — the card is their rule, not a charge.".into()
}
pub fn add_card() -> String { "Add credit card".into() }
/// N45: quiet, under Add credit card — before it's clicked, and after.
pub fn card_where() -> String { "Add credit card opens your Cloudflare billing page. I'll notice the card the moment it's there and carry on.".into() }
pub fn card_opened() -> String { "Billing is open in your browser — add the card there. I'll carry on by myself.".into() }
pub fn name_prefix(label: &str) -> String { format!("https://{label}.") }
pub fn name_suffix() -> String { ".workers.dev/…".into() }
pub fn keeper() -> String { "This one's a keeper: choose a name you'll be glad to see in a year.".into() }
pub fn claim() -> String { "Claim this name".into() }
pub fn charset() -> String { "Lowercase letters, numbers, and dashes.".into() }
pub fn name_taken() -> String { "That name's taken — try another.".into() }
pub fn wiring() -> String { "Setting up your space… switching your name on takes about a minute, one time only.".into() }
/// N45: the setup's last step, when the permissions finished before wiring.
pub fn wiring_last() -> String { "Almost there — switching your name on takes a few minutes the first time, and only once.".into() }
pub fn offline_wizard() -> String { "Everything so far is saved — I'll pick up right here when you're back online.".into() }
/// The wizard's one face for a failure nothing else classifies (DEVIATIONS #25) — written
/// during the build, awaiting ratification (DECISIONS' copy table).
pub fn unexpected_wizard() -> String { "Something unexpected got in the way — everything so far is saved. I'll keep trying.".into() }
pub fn try_again() -> String { "Try again".into() }
/// Law 16: a compact count keeps its figures ("14 videos").
pub fn found(host: &str, videos: u32) -> String {
    let n = if videos == 1 { "1 video".to_string() } else { format!("{videos} videos") };
    format!("Found your space — {host}, {n}. This Mac is joined.")
}
pub fn done_ask() -> String {
    "macOS will ask three times: your microphone and your camera — one click each — then screen recording, which goes through System Settings.".into()
}
pub fn done_flip() -> String {
    "Flip the switch next to shroom there. When macOS offers to quit and reopen the app, say yes — I'll be right where you left off.".into()
}
pub fn allow_recording() -> String { "Allow recording".into() }
pub fn perm_screen() -> String {
    "macOS handles screen recording in System Settings — it's open. Flip the switch next to shroom, and when macOS offers to quit and reopen it, say yes. I'll be right where you left me.".into()
}
/// perm-screen while macOS's own alert is up (N43): it leads, shroom opens nothing.
pub fn perm_screen_asking() -> String {
    "macOS is asking about screen recording now — choose Open System Settings in its window, then flip the switch next to shroom. When macOS offers to quit and reopen it, say yes. I'll be right where you left me.".into()
}
pub fn closed_its_window() -> String { "Closed macOS's window?".into() }
pub fn flipped_already() -> String { "Flipped it already?".into() }
pub fn reopen() -> String { "Reopen shroom".into() }
pub fn priming() -> String { "If macOS double-checks about recording, approve it — that's the last one for a good while.".into() }
pub fn all_set() -> String { "You're all set — ready to record.".into() }
pub fn menu_bar() -> String { "shroom lives in your menu bar now — look for the little mushroom.".into() }
pub fn close() -> String { "Close".into() }
pub fn invite_someone() -> String { "Invite someone".into() }
pub fn record_now() -> String { "Record now".into() }

// ── the Library (N35) ──
pub fn search_titles() -> String { "Search titles".into() }
pub fn open_page() -> String { "Open page".into() }
pub fn delete_ellipsis() -> String { "Delete…".into() }
pub fn rename() -> String { "Rename".into() }
pub fn row_uploading(pct: u32) -> String { format!("Uploading — {pct}%") }
/// Another Mac's upload: no percentage is known here (DECISIONS, copy awaiting ratification).
pub fn row_uploading_elsewhere() -> String { "Uploading".into() }
pub fn row_waiting_connection() -> String { "Waiting for connection".into() }
pub fn row_waiting_space() -> String { "Waiting for your space".into() }
pub fn save_to_movies() -> String { "Save to Movies".into() }
pub fn row_interrupted(len: &str) -> String { format!("Interrupted recording — {len} saved") }
pub fn row_removed() -> String { "Removed from the space".into() }
/// Law 16: the footer's count is a compact fact — figures. The singular is the build's.
pub fn video_count(n: u64) -> String {
    if n == 1 { "1 video".into() } else { format!("{n} videos") }
}
pub fn library_empty() -> String { "No videos yet — click the mushroom in your menu bar to record your first.".into() }
pub fn no_match(search: &str) -> String { format!("No titles match “{search}”.") }
pub fn library_offline() -> String {
    "You're offline — showing what this Mac remembers. Renaming and deleting wait until you're back.".into()
}
pub fn delete_one(title: &str) -> String { format!("Delete “{title}”?") }
pub fn delete_one_line() -> String { "It's removed everywhere. The link stays, and says the video is gone.".into() }
/// Law 16: one to nine are words; above nine the build keeps figures (DECISIONS).
pub fn delete_many(n: u64) -> String { format!("Delete {} videos?", count_word(n)) }
pub fn delete_many_line() -> String { "They're removed everywhere. Their links stay, and say the videos are gone.".into() }
pub fn cancel() -> String { "Cancel".into() }
pub fn delete() -> String { "Delete".into() }
/// A take whose length isn't known here (a row another Mac's delete stopped): N25's
/// question with only its last sentence (DECISIONS).
pub fn toss_unknown_length() -> String { "There's no undo.".into() }

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The day a video was made, as this Mac's calendar names it: “Aug 21”.
pub fn day(wall: crate::time::Wall, utc_offset_min: i32) -> String {
    let secs = wall.0.div_euclid(1000) + utc_offset_min as i64 * 60;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{} {d}", MONTHS[(m - 1) as usize])
}

/// A row's facts after its title: “Aug 21 · 4:32”.
pub fn row_meta(day: &str, length: Option<&str>) -> String {
    match length {
        Some(l) => format!("{day} · {l}"),
        None => day.to_string(),
    }
}

// ── Suggested to clean up (N68) ──

/// A size as the suggestions say it: “11 GB”, “3.2 GB”, “780 MB” (compact facts, law 16).
pub fn size(bytes: u64) -> String {
    let mb = (bytes as f64 / 1e6).round() as u64;
    if mb < 1000 {
        return format!("{mb} MB");
    }
    let gb = bytes as f64 / 1e9;
    if gb >= 9.95 {
        return format!("{} GB", gb.round() as u64);
    }
    let tenths = (gb * 10.0).round() as u64;
    if tenths % 10 == 0 { format!("{} GB", tenths / 10) } else { format!("{}.{} GB", tenths / 10, tenths % 10) }
}

/// Money a month, as the suggestions say it: “17¢”, “$1.35”.
pub fn money(cents: u64) -> String {
    if cents < 100 { format!("{cents}¢") } else { format!("${}.{:02}", cents / 100, cents % 100) }
}

/// The Library's line above the rows.
pub fn cleanup_line(n: u64, bytes: u64) -> String {
    if n == 1 {
        format!("One video you've stopped watching takes {}.", size(bytes))
    } else {
        format!("{} videos you've stopped watching take {}.", count_word_cap(n), size(bytes))
    }
}
pub fn review() -> String { "Review…".into() }
pub fn not_now() -> String { "Not now".into() }
pub fn cleanup_title() -> String { "Suggested to clean up".into() }
pub fn cleanup_back() -> String { "‹ Library".into() }

const CLEANUP_WHY: &str = "Videos you've stopped watching that take real room.";

/// “Deleting all six” — or “Deleting it”, for one.
fn deleting_all(n: u64) -> String {
    if n == 1 { "Deleting it".into() } else { format!("Deleting all {}", count_word(n)) }
}

/// The suggestions' line, past the free 10 GB: what deleting them saves.
pub fn cleanup_saves(n: u64, freed: u64, cents: u64) -> String {
    format!("{CLEANUP_WHY} {} frees {} — about {} a month off your Cloudflare bill.", deleting_all(n), size(freed), money(cents))
}
/// Inside the free 10 GB: deleting saves nothing yet.
pub fn cleanup_inside_free(n: u64, freed: u64) -> String {
    format!("{CLEANUP_WHY} {} frees {}. Your space is inside Cloudflare's free 10 GB, so this saves nothing yet.", deleting_all(n), size(freed))
}
/// Deleting them all would bring the space back under the free 10 GB.
pub fn cleanup_back_inside(n: u64, freed: u64) -> String {
    format!("{CLEANUP_WHY} {} frees {} and brings your space back inside Cloudflare's free 10 GB.", deleting_all(n), size(freed))
}
/// On a member's Mac: the bill is the owner's.
pub fn cleanup_frees(n: u64, freed: u64) -> String {
    format!("{CLEANUP_WHY} {} frees {}.", deleting_all(n), size(freed))
}

/// How long ago, as a row fact: days under 60, months under 24, then years.
fn ago(days: u32) -> String {
    if days < 60 {
        format!("{days} days")
    } else if days < 730 {
        format!("{} months", ((days as f64) / 30.0).round() as u32)
    } else {
        format!("{} years", days / 365)
    }
}

/// Why a video is suggested (B211): its plays, and when the last was.
pub fn cleanup_reason(plays: u32, quiet_days: u32, stopped_early: bool) -> String {
    let early = if stopped_early { "stopped early, " } else { "" };
    match plays {
        0 => "Nobody has watched it".into(),
        1 => format!("Watched once, {early}{} ago", ago(quiet_days)),
        2 => format!("Watched twice, {early}the last {} ago", ago(quiet_days)),
        n => format!("Watched {n} times, {early}the last {} ago", ago(quiet_days)),
    }
}
pub fn keep() -> String { "Keep".into() }
pub fn deleted_title(title: &str) -> String { format!("Deleted “{title}”.") }
pub fn undo() -> String { "Undo".into() }
pub fn delete_all(n: u64) -> String { format!("Delete all {}…", count_word(n)) }
pub fn cleanup_foot() -> String { "Keep takes a video off this list for good. Plays are counted in your own Cloudflare account.".into() }
pub fn cleanup_empty() -> String { "Nothing to clean up right now.".into() }

pub fn your_videos() -> String { "Your videos".into() }
pub fn count_plays() -> String { "Count plays".into() }
pub fn count_plays_hint() -> String { "How often each video is played and how far, kept in your Cloudflare account. No cookies, nothing about who watched.".into() }
/// The play-count switch didn't go through (law 12).
pub fn plays_failed() -> String { "Couldn't change that just now — try again in a moment.".into() }
pub fn suggest_cleanup() -> String { "Suggest videos to clean up".into() }
pub fn suggest_cleanup_hint() -> String { "Videos you've stopped watching that take real room, in the Library.".into() }
pub fn space_used() -> String { "Space used".into() }
/// The space past the free 10 GB, and what it costs a month.
pub fn space_used_past(bytes: u64, cents: u64) -> String { format!("{} — about {} a month past Cloudflare's free 10 GB", size(bytes), money(cents)) }
pub fn space_used_free(bytes: u64) -> String { format!("{} — inside Cloudflare's free 10 GB", size(bytes)) }

// ── Settings (N36) ──
pub fn tab_general() -> String { "General".into() }
pub fn tab_space() -> String { "Space".into() }
pub fn updates() -> String { "Updates".into() }
pub fn up_to_date(version: &str) -> String { format!("shroom {version} — up to date.") }
/// The running version when nothing is known about a newer one yet (no check answered, or
/// one couldn't run): no claim (the build's, DECISIONS B54).
pub fn running_version(version: &str) -> String { format!("shroom {version}.") }
pub fn update_ready(version: &str) -> String { format!("shroom {version} is ready.") }
pub fn update() -> String { "Update".into() }
pub fn whats_new() -> String { "What's new".into() }
/// The line under General's card: GitHub is the public door (N8, B180).
pub fn help_line() -> String { "Or tell us in public, on GitHub.".into() }
pub fn github() -> String { "GitHub".into() }
// ── Report a problem (B180) ──
/// The most a report's words hold (characters); the page and the Worker take as many.
pub const REPORT_WORDS_MAX: usize = 8000;
pub fn help_label() -> String { "Something off?".into() }
pub fn copy_diagnostics() -> String { "Copy diagnostics".into() }
pub fn report_a_problem() -> String { "Report a problem…".into() }
pub fn report_sent() -> String { "Sent — thank you. If you left your email, you’ll hear back.".into() }
pub fn report_title() -> String { "Report a problem".into() }
pub fn report_line() -> String {
    "Tell us what happened. shroom adds its version, your Mac’s model and its recent log — never your videos, links, names or keys. It goes straight to the person who makes shroom.".into()
}
pub fn report_placeholder() -> String { "What were you doing, and what went wrong?".into() }
pub fn report_reply_label() -> String { "Your email, if you’d like an answer (optional)".into() }
pub fn report_disclosure() -> String { "See exactly what’s sent".into() }
pub fn report_preparing() -> String { "Putting it together…".into() }
pub fn send() -> String { "Send".into() }
pub fn sending() -> String { "Sending…".into() }
pub fn report_failed() -> String { "That didn’t go through — check your connection and try again. Or email it to hi@shroom.am instead.".into() }
pub fn report_mail() -> String { "email it to hi@shroom.am".into() }
// ── Send feedback (N60): ratified by Bagrat, 2026-10-17 ──
/// The header icon's name (hover, VoiceOver) and the window's title: the site's label (N55).
pub fn send_feedback() -> String { "Send feedback".into() }
pub fn feedback_line() -> String { "An idea, something that bugged you, something that broke — it goes straight to the person who makes shroom.".into() }
pub fn feedback_placeholder() -> String { "What’s on your mind?".into() }
pub fn include_log() -> String { "Include shroom’s recent log".into() }
pub fn include_log_note() -> String { "Its version, your Mac’s model and its recent log, cleaned — never your videos, links, names or keys.".into() }
pub fn feedback_sent() -> String { "Sent — thank you.".into() }
pub fn feedback_sent_line() -> String { "If you left your email, you’ll hear back.".into() }
pub fn done() -> String { "Done".into() }
pub fn needs_macos(version: &str, macos: u32, running: &str) -> String {
    format!("shroom {version} needs macOS {macos} — this Mac stays on {running}.")
}
/// Law 4: what greys mid-take says why.
pub fn after_take() -> String { "You can do this once your recording is finished.".into() }
pub fn disconnect_ellipsis() -> String { "Disconnect this Mac…".into() }
pub fn row_six_off() -> String { "→ Letting macOS allow recording — screen recording is off".into() }
pub fn turn_on_mic() -> String { "Turn on the microphone".into() }
pub fn on_this_mac() -> String { "on this Mac".into() }
pub fn nothing_to_redo() -> String { "Your space is ready — nothing to redo.".into() }
pub fn disconnect_title() -> String { "Disconnect this Mac?".into() }
pub fn disconnect_line() -> String {
    "Your space and every video in it stay exactly as they are. This Mac forgets your space and goes back to the start.".into()
}
pub fn disconnect_only_holder() -> String {
    "To manage your space again, set it up with a fresh token — everything will be there.".into()
}
/// Law 16: “Two videos are still uploading…”; the singular is the build's (DECISIONS).
pub fn disconnect_uploading(n: u64) -> String {
    if n == 1 {
        return "One video is still uploading — you can disconnect once it's done.".into();
    }
    let w = count_word(n);
    let mut c = w.chars();
    let cap = c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default();
    format!("{cap} videos are still uploading — you can disconnect once they're done.")
}
/// A recovered take waiting blocks the exit too; the spec gives no line — join-switch's
/// (N31) is borrowed (DECISIONS).
pub fn disconnect_saved_waits() -> String { "Get the link for your saved recording first, or discard it.".into() }
pub fn disconnect() -> String { "Disconnect".into() }
/// Stranded takes don't block an exit (N36): “Two recordings never reached the space…”;
/// the singular is the build's (DECISIONS).
pub fn stranded_line(n: u64) -> String {
    if n == 1 {
        return "One recording never reached the space — leaving deletes it from this Mac.".into();
    }
    let w = count_word(n);
    let mut c = w.chars();
    let cap = c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default();
    format!("{cap} recordings never reached the space — leaving deletes them from this Mac.")
}
/// “Save them to Movies first”; for one, the build's “Save it to Movies first” (DECISIONS).
// ── uninstall (B135) ──
pub fn uninstall_ellipsis() -> String { "Uninstall shroom…".into() }
pub fn uninstall_title() -> String { "Uninstall shroom?".into() }
pub fn uninstall_space_stays() -> String { "Your space, every video in it and every link stay exactly as they are.".into() }
/// A member's Mac: their videos lead, and the space is the owner's (B156).
pub fn uninstall_owners_space_stays(name: &str) -> String { format!("Your videos and every link stay exactly as they are, in {name}'s space.") }
pub fn uninstall_lets_go() -> String {
    "This Mac lets go of shroom: the app, its settings, and its permission to record your screen, microphone and camera.".into()
}
pub fn uninstall_back_owner() -> String { "To come back, install shroom again and set it up with a fresh token — everything will be there.".into() }
pub fn uninstall_back_member() -> String { "To come back, install shroom again and ask for a new invite.".into() }
/// Law 16: “Two videos are still uploading…”.
pub fn uninstall_uploading(n: u64) -> String {
    if n == 1 {
        return "One video is still uploading — you can uninstall once it's done.".into();
    }
    format!("{} videos are still uploading — you can uninstall once they're done.", capitalized(&count_word(n)))
}
/// Stranded takes don't block it (N36): “Two recordings never reached the space…”.
pub fn uninstall_stranded(n: u64) -> String {
    if n == 1 {
        return "One recording never reached the space — uninstalling deletes it from this Mac.".into();
    }
    format!("{} recordings never reached the space — uninstalling deletes them from this Mac.", capitalized(&count_word(n)))
}
pub fn uninstall() -> String { "Uninstall".into() }
pub fn uninstalling() -> String { "Uninstalling…".into() }

fn capitalized(w: &str) -> String {
    let mut c = w.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
}

pub fn save_them_to_movies(n: u64) -> String {
    if n == 1 { "Save it to Movies first".into() } else { "Save them to Movies first".into() }
}

// ── updates (N37) ──
pub fn updating() -> String { "Updating — back in a few seconds.".into() }
pub fn updated(version: &str) -> String { format!("Updated to {version} ✓") }
pub fn update_failed() -> String { "The update didn't finish — shroom will try again later.".into() }

// ── Region 2's afterlife: space-degraded and the repair mood (N42) ──
pub fn space_degraded() -> String { "Your space needs you — new videos wait safely on this Mac.".into() }
pub fn reconnect_space() -> String { "Reconnect your space".into() }
pub fn token_repair() -> String {
    "Cloudflare stopped accepting shroom's token — create a fresh one and paste it here. Nothing else needs redoing.".into()
}
pub fn token_other_account(host: &str) -> String {
    format!("This token is for a different Cloudflare account than your space — create it in the account that holds {host}.")
}
pub fn storage_gone() -> String {
    "Your space's storage is gone from Cloudflare — the videos that were in it can't come back. Set up fresh storage?".into()
}
pub fn set_it_up_fresh() -> String { "Set it up fresh".into() }
pub fn name_changed() -> String {
    "Your link name changed in Cloudflare, so links you've shared stopped opening. Changing it back brings them back.".into()
}

// ── Region 6: members (N28–N34) ──

/// Law 16, capitalised at a sentence's start: “Two videos…”.
fn count_word_cap(n: u64) -> String {
    let w = count_word(n);
    let mut c = w.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
}

// The Members face (N28, N33).
pub fn tab_members() -> String { "Members".into() }
pub fn members_title() -> String { "Members".into() }
pub fn members_truth() -> String {
    "Everyone here records to your space — their videos live in your storage, and you can see and delete any of them.".into()
}
pub fn you() -> String { "You".into() }
pub fn add_another_mac() -> String { "Add another Mac".into() }
pub fn joined_row(label: &str, day: &str) -> String { format!("{label} — joined {day}") }
pub fn invited_now(label: &str) -> String { format!("{label} — invited just now") }
/// A pending row's fact later in its life — figures (law 16). Days left round up.
pub fn invited_fact(day: &str, days_left: u64) -> String { format!("invited {day} · {days_left} days left") }
/// The pending row: its label, then its fact (the spec gives the fact; the dash is the row's).
pub fn invited_row(label: &str, day: &str, days_left: u64) -> String { format!("{label} — {}", invited_fact(day, days_left)) }
/// An expired invite's row: no days left, [ New link ] beside it (the build's, DECISIONS B40).
pub fn invited_expired(label: &str, day: &str) -> String { format!("{label} — invited {day}") }
/// One of the owner's other Macs under You (N33): the spec's sub-row without its Remove….
pub fn your_mac(name: &str, day: &str) -> String { format!("You · {name} — joined {day}") }
pub fn keeps_token() -> String { "keeps your Cloudflare token".into() }
pub fn remove_ellipsis() -> String { "Remove…".into() }
pub fn send_link(label: &str) -> String { format!("Send {label} this link — it's good for one person, for seven days.") }
pub fn who_for() -> String { "Who's this for?".into() }
pub fn invite() -> String { "Invite".into() }
pub fn invite_line() -> String { "You'll get a link to send them — over any chat.".into() }
/// Who invites say they're from, under the composer (B158; Bagrat, 2026-10-15).
pub fn invites_from(name: &str) -> String { format!("Invites will say they're from {name}.") }
/// The same line's head, as the label of the field that changes the name.
pub fn invites_from_label() -> String { "Invites will say they're from".into() }
pub fn change() -> String { "Change".into() }
pub fn save() -> String { "Save".into() }
/// The longest name an invite carries (the Worker's cap on `inviter`).
pub const FROM_MAX: usize = 40;
pub fn new_link() -> String { "New link".into() }
/// A link the Members face asked for (Invite, Add another Mac, New link) that didn't come
/// back, said in the composer's line (law 12; the build's, DEVIATIONS #28).
pub fn invite_failed() -> String { "Couldn't make a link just now — try again in a moment.".into() }
/// A Remove… that didn't go through: the row stays, and the composer's line says so (law 12;
/// the build's, DEVIATIONS #28).
pub fn remove_failed() -> String { "Couldn't remove that just now — try again in a moment.".into() }
/// A born link's pill, up to its code (law 13: the code is drawn from the stash).
pub fn link_prefix(host: &str) -> String { format!("{host}/join/") }
pub fn own_mac_link_line() -> String { "Open this link on your other Mac — it's good for one hour.".into() }
pub fn remove_title(name: &str) -> String { format!("Remove {name}?") }
pub fn remove_member_line(label: &str) -> String {
    format!("{label} won't be able to record to your space anymore — a video already on its way can still finish. {label}'s videos stay; you'll find them in your Library.")
}
pub fn remove() -> String { "Remove".into() }
pub fn cancel_invite_title(label: &str) -> String { format!("Cancel {label}'s invite?") }
pub fn cancel_invite_line() -> String { "The link stops working right away.".into() }
pub fn keep_it() -> String { "Keep it".into() }
pub fn cancel_invite() -> String { "Cancel invite".into() }
pub fn remove_mac_line() -> String { "That Mac stops recording to your space. Its videos stay.".into() }

// The join machine (N29–N31).
pub fn invited_you(name: &str) -> String { format!("{name} invited you to record with shroom.") }
pub fn member_truth(name: &str) -> String {
    format!("shroom records your screen and hands you a link to share. Your videos will live in {name}'s space, where {name} can see them too — nothing to sign up for, nothing to pay.")
}
pub fn member_step() -> String { "One step before your first video: let macOS allow recording. About a minute, once.".into() }
pub fn join() -> String { "Join".into() }
pub fn add_this_mac() -> String { "Add this Mac to your space.".into() }
pub fn same_videos(host: &str) -> String { format!("Same videos, same members — {host}, on this Mac too.") }
pub fn member_step_here() -> String {
    "One step before your first video here: let macOS allow recording. About a minute, once.".into()
}
/// The member plan's header: “Bagrat's space, on this Mac:” — “Your space, …” on the
/// owner's own Mac.
pub fn space_header(name: Option<&str>) -> String {
    match name {
        Some(n) => format!("{n}'s space, on this Mac:"),
        None => "Your space, on this Mac:".into(),
    }
}
/// The member plan's first row, current. The own-Mac row mirrors the member's
/// parenthetical (the spec leaves it open; DECISIONS B40).
pub fn joining_row(name: Option<&str>) -> String {
    match name {
        Some(n) => format!("→ Joining {n}'s space (shroom does this part)"),
        None => "→ Joining your space (shroom does this part)".into(),
    }
}
/// The member plan's first row, ticked with its fact (N15).
/// The space's row once it can't take this Mac's videos (N34): un-ticked, the same words
/// (DECISIONS B157).
pub fn space_row_off(name: Option<&str>, host: &str) -> String {
    match name {
        Some(n) => format!("• {n}'s space — {host}"),
        None => format!("• Your space — {host}"),
    }
}
pub fn space_row_done(name: Option<&str>, host: &str) -> String {
    match name {
        Some(n) => format!("✓ {n}'s space — {host}"),
        None => format!("✓ Your space — {host}"),
    }
}
pub fn joining_line() -> String { "Joining — a few seconds.".into() }
pub fn join_offline() -> String { "You're offline — I'll finish joining the moment you're back.".into() }
pub fn join_not_answering(name: &str) -> String { format!("{name}'s space isn't answering right now — I'll keep trying.") }
/// No inviter known (peek never got through): the line without a name (DECISIONS B41).
pub fn join_not_answering_unnamed() -> String { "This invite's space isn't answering right now — I'll keep trying.".into() }
pub fn invite_expired(name: &str) -> String { format!("This invite has expired — ask {name} for a fresh one.") }
/// An unknown code: the join page's own nameless words (B39 b).
pub fn invite_unknown() -> String { "This invite link doesn't work — ask whoever sent it for a fresh one.".into() }
/// An own-Mac link past its hour: the page's words, in the app too (B40).
pub fn own_mac_expired() -> String { "This link has expired — make a fresh one from Members on your other Mac.".into() }
pub fn new_link_door() -> String { "Have a new link? Paste it here.".into() }
pub fn invite_used(name: &str) -> String { format!("This invite was already used. If that wasn't you, ask {name} for a fresh one.") }
/// An own-Mac link already used: the page's own words (`ownUsed`), in the app too (B54).
pub fn own_mac_used() -> String { "This link was already used — make a fresh one from Members on your other Mac.".into() }
/// `join-waiting` on an own-Mac link: the space is the owner's own (the build's, B54).
pub fn own_space_not_answering() -> String { "Your space isn't answering right now — I'll keep trying.".into() }
pub fn space_missing(name: &str) -> String { format!("This invite's space isn't there anymore — ask {name} about it.") }
/// The space is missing before peek could name the inviter (the build's, DECISIONS B40).
pub fn space_missing_unnamed() -> String { "This invite's space isn't there anymore — ask whoever sent it about it.".into() }
pub fn already_here(name: &str) -> String {
    format!("This Mac is already in {name}'s space — nothing to do. The invite is still unused, so it can go to someone else.")
}
pub fn already_here_own() -> String { "This Mac is already in your space — open the link on the Mac you want to add.".into() }
pub fn switch_line(current: &str, name: &str) -> String {
    format!("This Mac records to {current} now. Joining moves it to {name}'s space — your videos in {current} stay right where they are.")
}
pub fn switch_only_holder() -> String {
    "You can come back to your own space anytime by setting it up again with a fresh token — everything will be there.".into()
}
pub fn switch_half_set_up(name: &str) -> String {
    format!("You've started setting up your own space. Joining {name}'s instead sets that aside — your Cloudflare stays as it is.")
}
pub fn join_space(name: &str) -> String { format!("Join {name}'s space") }
pub fn stay() -> String { "Stay".into() }
/// Law 16: “Two videos are still uploading…”; the singular is the build's (DECISIONS B40).
pub fn switch_uploading(n: u64) -> String {
    if n == 1 {
        return "One video is still uploading — you can switch once it's done.".into();
    }
    format!("{} videos are still uploading — you can switch once they're done.", count_word_cap(n))
}
pub fn switch_saved_waits() -> String { "Get the link for your saved recording first, or discard it.".into() }
/// Takes the old space can't take don't block a switch (they can't drain): Leave's line,
/// worded for the switch (DECISIONS B41).
pub fn switch_stranded(n: u64, host: &str) -> String {
    if n == 1 {
        return format!("One recording never reached {host} — joining deletes it from this Mac.");
    }
    format!("{} recordings never reached {host} — joining deletes them from this Mac.", count_word_cap(n))
}
/// Start greys while a switch's join runs (DECISIONS B41).
pub fn record_after_switch(name: &str) -> String { format!("You can record once this Mac has joined {name}'s space.") }

// Member life (N34).
pub fn member_revoked(name: &str) -> String {
    format!("Your access to {name}'s space was turned off. Recordings on this Mac are safe — ask {name} if that's a surprise.")
}
pub fn needs_owner(name: &str) -> String {
    format!("{name}'s space needs a fix on {name}'s side. Your videos wait safely on this Mac and upload by themselves once it's done.")
}
pub fn space_gone(name: &str) -> String {
    format!("{name}'s space isn't there anymore. Recordings on this Mac are safe — save any you want to keep.")
}
/// A removed owner-class Mac (WORKER-INTERFACE open question 18, built as recommended —
/// new copy, DECISIONS B40).
pub fn owner_mac_removed() -> String {
    "This Mac was removed from your space. Recordings on this Mac are safe — add it again from Members on your other Mac.".into()
}
/// “Save to Movies” for one; “Save three to Movies” for several (law 16).
pub fn save_n_to_movies(n: u64) -> String {
    if n <= 1 { save_to_movies() } else { format!("Save {} to Movies", count_word(n)) }
}
pub fn saved_to_movies() -> String { "Saved to Movies ✓".into() }
/// A copy to Movies under way: the button waits on it (the build's, DECISIONS B54).
pub fn saving_to_movies() -> String { "Saving to Movies…".into() }
/// A copy to Movies that couldn't be made: the button says so and asks again (the build's,
/// DECISIONS B54).
pub fn save_to_movies_again() -> String { "Couldn't save to Movies — try again".into() }
/// The file Save to Movies makes for a take with no title (N34's words).
pub fn movies_untitled(day: &str) -> String { format!("shroom recording — {day}") }
pub fn leave_space_ellipsis() -> String { "Leave this space…".into() }
pub fn stranded_title() -> String { "Your recording is safe on this Mac.".into() }
pub fn needs_update_member(name: &str) -> String { format!("{name}'s space needs an update — it happens when {name} updates shroom.") }
pub fn needs_update_owner_class() -> String {
    "Your space needs an update — open shroom on the Mac you set it up on, or add a fresh token here.".into()
}
pub fn add_token_here() -> String { "Add a token here".into() }
pub fn degraded_no_token() -> String {
    "Your space needs a fix — open shroom on the Mac you set it up on, or reconnect here with a fresh token.".into()
}
pub fn reconnect_here() -> String { "Reconnect here".into() }

// Settings · Space on every kind of Mac, and the Library across members.
pub fn mark_label() -> String { "“made with shroom” on your pages".into() }
pub fn mark_line() -> String { "The small mark on every video page. You can flip it right on a page, too.".into() }
pub fn repairs_elsewhere() -> String {
    "Repairs happen where your Cloudflare token lives — on the Mac you set your space up on, or here if you add a fresh one.".into()
}
pub fn member_space_line(name: &str) -> String { format!("Your videos live there, where {name} can see them too.") }
pub fn leave_title(name: &str) -> String { format!("Leave {name}'s space?") }
pub fn leave_line(name: &str) -> String {
    format!("Your videos stay there, where {name} can still see them — delete any you want gone first. This Mac goes back to the start.")
}
pub fn leave() -> String { "Leave".into() }
/// Leave greys like Disconnect; the spec words the line only for Disconnect (DECISIONS B40).
pub fn leave_uploading(n: u64) -> String {
    if n == 1 {
        return "One video is still uploading — you can leave once it's done.".into();
    }
    format!("{} videos are still uploading — you can leave once they're done.", count_word_cap(n))
}
pub fn disconnect_owner_class_line() -> String {
    "Your space and every video stay as they are — add this Mac again anytime from Members.".into()
}
pub fn mine() -> String { "Mine".into() }
pub fn everyone() -> String { "Everyone".into() }
pub fn their_video(label: &str) -> String { format!("This is {label}'s video.") }

/// The link-name guard (N15): uppercase lowercases silently, anything outside the set never
/// lands, and the name stays within a DNS label. (The exact edges are a scaffold-time fact.)
pub fn filter_name(raw: &str) -> String {
    raw.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .take(63)
        .collect()
}

/// Whether typing dropped a character outside the set (the quiet line's cause, N15) —
/// never a name merely cut at the DNS label's 63 characters (whole-spec audit #12).
pub fn name_charset_dropped(raw: &str) -> bool {
    let lowered: String = raw.chars().flat_map(char::to_lowercase).collect();
    let kept: String = lowered.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-').collect();
    kept != lowered
}

/// Whether a typed name may be claimed: non-empty, and no dash at either end.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.ends_with('-') && filter_name(name) == name
}

/// Every line with the spec's example arguments and its state — what `tests/copy.rs` checks.
pub fn registry() -> Vec<(&'static str, String)> {
    vec![
        ("perm-mic", mic_purpose()),
        ("perm-mic", camera_purpose()),
        ("config", record()),
        ("config", camera()),
        ("config", system_default()),
        ("config", microphone()),
        ("config", quality()),
        ("config", quality_name(crate::world::Quality::Normal)),
        ("config", quality_name(crate::world::Quality::TwoK)),
        ("config", quality_name(crate::world::Quality::FourK)),
        ("config", start()),
        ("config", countdown_hint()),
        ("config", nothing_recorded(true)),
        ("config", nothing_recorded(false)),
        ("config", library()),
        ("config", settings()),
        ("config", quit()),
        ("idle-ready", quit()),
        ("config", no_mic()),
        ("config", camera_off()),
        ("config", allowed_in_settings()),
        ("config", open_system_settings()),
        ("config", display_name("LG UltraFine", Some("left"))),
        ("config", uploading_one(40)),
        ("config", uploading_many(2, 40)),
        ("config", waiting_connection()),
        ("disk-low", room_left(20)),
        ("disk-low", out_of_room()),
        ("recording-off", recording_off()),
        ("recording-off", turn_back_on()),
        ("recovered-take", recovered("4:32")),
        ("recovered-take", get_link()),
        ("recovered-take", discard_ellipsis()),
        ("countdown", countdown_cancel()),
        ("paused", paused_title("4:32")),
        ("paused", resume()),
        ("paused", restart()),
        ("paused", stop()),
        ("paused", discard_ellipsis()),
        ("display-lost", display_lost()),
        ("mic-lost", mic_lost()),
        ("mic-lost", mic_fallback("MacBook Pro Microphone")),
        ("mic-lost", mic_none()),
        ("camera-lost", camera_lost()),
        ("mac-away", slept()),
        ("mac-away", locked()),
        ("mic-permission-lost", mic_permission_lost()),
        ("discard-confirm", toss_title()),
        ("discard-confirm", toss_so_far("4:32")),
        ("discard-confirm-saved-take", toss_saved("4:32")),
        ("discard-confirm", back()),
        ("discard-confirm", restart_instead()),
        ("discard-confirm", discard()),
        ("permission-lost", screen_stopped("7:15")),
        ("disk-full-stop", disk_stopped("12:04")),
        ("disk-full-stop", get_link()),
        ("stopped-offline", link_ready()),
        ("stopped-offline", offline_line()),
        ("stopped-offline", copy_link()),
        ("stopped-offline", copied()),
        ("quit-confirm", quit_uploading(2)),
        ("quit-confirm", quit_line()),
        ("quit-confirm", back()),
        ("quit-confirm", quit_anyway()),
        ("idle-unprovisioned", quit()),
        ("welcome", welcome_what()),
        ("welcome", welcome_steps()),
        ("welcome", login_box()),
        ("welcome", set_up()),
        ("welcome", later()),
        ("welcome", uninstall_ellipsis()),
        ("account", plan_header()),
        ("account", row_current(1)),
        ("account", row_ahead(2)),
        ("account", row_ahead(3)),
        ("account", row_ahead(4)),
        ("account", row_ahead(5)),
        ("account", row_ahead(6)),
        ("account", have_account()),
        ("account", not_yet()),
        ("account", have_one()),
        ("account-signing-up", signup_open()),
        ("account-signing-up", signup_again()),
        ("account-signing-up", signed_up()),
        ("token", token_line()),
        ("token", create_token()),
        ("token", paste_token()),
        ("token", token_email_first()),
        ("token-garbled", token_garbled()),
        ("token-wrong-powers", token_wrong_powers()),
        ("token-pick-account", pick_account()),
        ("token-pick-account", use_account()),
        ("token-pick-account", row_done(1, Some("Bagrat's Account"))),
        ("gate-email", gate_email()),
        ("gate-card", gate_card()),
        ("gate-card", add_card()),
        ("gate-card", card_where()),
        ("gate-card", card_opened()),
        ("name", name_prefix("v")),
        ("name", name_suffix()),
        ("name", keeper()),
        ("name", claim()),
        ("name", charset()),
        ("name", row_done(4, Some(&name_fact("v.myshroom.workers.dev", false)))),
        ("name-taken", name_taken()),
        ("name-v-clash", row_done(4, Some(&name_fact("shroom.myname.workers.dev", true)))),
        ("wiring", wiring()),
        ("wiring", wiring_last()),
        ("wizard-offline", offline_wizard()),
        ("space-found", found("v.myshroom.workers.dev", 14)),
        ("done", done_ask()),
        ("done", done_flip()),
        ("done", allow_recording()),
        ("done", row_working()),
        ("perm-screen", perm_screen()),
        ("perm-screen", perm_screen_asking()),
        ("perm-screen", closed_its_window()),
        ("perm-screen", open_system_settings()),
        ("perm-screen", flipped_already()),
        ("perm-screen", reopen()),
        ("perm-priming", priming()),
        ("all-set", all_set()),
        ("all-set", menu_bar()),
        ("all-set", close()),
        ("all-set", invite_someone()),
        ("all-set", record_now()),
        ("all-set", row_done(6, Some(&mic_off_fact()))),
        // ── Region 5: the Library ──
        ("library", library()),
        ("library", search_titles()),
        ("library", copy_link()),
        ("library", copy_thumbnail()),
        ("library", copied()),
        ("library", open_page()),
        ("library", delete_ellipsis()),
        ("library", rename()),
        ("library", row_uploading(40)),
        ("library", row_waiting_connection()),
        ("library", row_waiting_space()),
        ("library", save_to_movies()),
        ("library", row_interrupted("4:32")),
        ("library", get_link()),
        ("library", discard_ellipsis()),
        ("library", row_removed()),
        ("library", video_count(14)),
        ("library", cleanup_line(6, 11_000_000_000)),
        ("library", review()),
        ("library", not_now()),
        ("library-cleanup", cleanup_title()),
        ("library-cleanup", cleanup_back()),
        ("library-cleanup", cleanup_saves(6, 11_000_000_000, 17)),
        ("library-cleanup", cleanup_inside_free(6, 5_000_000_000)),
        ("library-cleanup", cleanup_back_inside(6, 3_000_000_000)),
        ("library-cleanup", cleanup_frees(6, 11_000_000_000)),
        ("library-cleanup", cleanup_reason(4, 300, false)),
        ("library-cleanup", cleanup_reason(0, 45, false)),
        ("library-cleanup", open_page()),
        ("library-cleanup", keep()),
        ("library-cleanup", delete()),
        ("library-cleanup", deleted_title("Old onboarding take")),
        ("library-cleanup", undo()),
        ("library-cleanup", delete_all(6)),
        ("library-cleanup", cleanup_foot()),
        ("library-cleanup", cleanup_empty()),
        ("library-empty", library_empty()),
        ("library-no-match", no_match("502")),
        ("library-offline", library_offline()),
        ("delete-confirm", delete_one("Where the 502s come from")),
        ("delete-confirm", delete_one_line()),
        ("delete-confirm", cancel()),
        ("delete-confirm", delete()),
        ("delete-confirm-several", delete_many(6)),
        ("delete-confirm-several", delete_many_line()),
        // ── Settings ──
        ("settings", tab_general()),
        ("settings", tab_space()),
        ("settings-general", login_box()),
        ("settings-general", your_videos()),
        ("settings-general", count_plays()),
        ("settings-general", count_plays_hint()),
        ("settings-general", plays_failed()),
        ("settings-general", suggest_cleanup()),
        ("settings-general", suggest_cleanup_hint()),
        ("settings-general", space_used()),
        ("settings-general", space_used_past(34_000_000_000, 36)),
        ("settings-general", space_used_free(8_200_000_000)),
        ("settings-general", updates()),
        ("settings-general", up_to_date("1.2.0")),
        ("settings-general", update_ready("1.3.0")),
        ("settings-general", update()),
        ("settings-general", whats_new()),
        ("settings-general", help_line()),
        ("settings-general", github()),
        ("settings-general", help_label()),
        ("settings-general", copy_diagnostics()),
        ("settings-general", copied()),
        ("settings-general", report_a_problem()),
        ("settings-general", report_sent()),
        ("report-problem", report_title()),
        ("report-problem", report_line()),
        ("report-problem", report_placeholder()),
        ("report-problem", report_reply_label()),
        ("report-problem", report_disclosure()),
        ("report-problem", report_preparing()),
        ("report-problem", cancel()),
        ("report-problem", send()),
        ("report-problem", sending()),
        ("report-problem", report_failed()),
        ("report-problem", report_mail()),
        ("config", send_feedback()),
        ("send-feedback", send_feedback()),
        ("send-feedback", feedback_line()),
        ("send-feedback", feedback_placeholder()),
        ("send-feedback", report_reply_label()),
        ("send-feedback", include_log()),
        ("send-feedback", include_log_note()),
        ("send-feedback", report_disclosure()),
        ("send-feedback", report_preparing()),
        ("send-feedback", cancel()),
        ("send-feedback", send()),
        ("send-feedback", sending()),
        ("send-feedback", report_failed()),
        ("send-feedback", report_mail()),
        ("send-feedback", feedback_sent()),
        ("send-feedback", feedback_sent_line()),
        ("send-feedback", done()),
        ("settings-general", needs_macos("1.4.0", 15, "1.3.2")),
        ("settings-general", after_take()),
        ("settings-space", disconnect_ellipsis()),
        ("settings-space", row_six_off()),
        ("settings-space", turn_back_on()),
        ("settings-space", turn_on_mic()),
        ("settings-space", after_take()),
        ("settings-space-inspection", space_done(1, Some(&account_fact("you@example.com's Account")))),
        ("settings-space-inspection", space_done(2, Some(&on_this_mac()))),
        ("settings-space-inspection", space_done(3, None)),
        ("settings-space-inspection", space_done(4, Some(&name_fact("v.myshroom.workers.dev", false)))),
        ("settings-space-inspection", space_done(5, None)),
        ("settings-space-inspection", space_done(6, Some(&on_this_mac()))),
        ("settings-space-inspection", nothing_to_redo()),
        ("re-entry-as-settings", nothing_to_redo()),
        ("re-entry-as-settings", disconnect_ellipsis()),
        ("disconnect-confirm", disconnect_title()),
        ("disconnect-confirm", disconnect_line()),
        ("disconnect-confirm", disconnect_only_holder()),
        ("disconnect-confirm", disconnect_uploading(2)),
        // N36 gives the stranded line once, generically for both exits; the fixture files it
        // on the member's sheet (its open note (c)) — the owner's sheet carries it too.
        ("leave-confirm", stranded_line(2)),
        ("leave-confirm", save_them_to_movies(2)),
        ("disconnect-confirm", cancel()),
        ("disconnect-confirm", disconnect()),
        ("settings-general", uninstall_ellipsis()),
        ("uninstall-confirm", uninstall_title()),
        ("uninstall-confirm", uninstall_space_stays()),
        ("uninstall-confirm", uninstall_owners_space_stays("Bagrat")),
        ("uninstall-confirm", uninstall_lets_go()),
        ("uninstall-confirm", uninstall_back_owner()),
        ("uninstall-confirm", uninstall_back_member()),
        ("uninstall-confirm", uninstall_uploading(1)),
        ("uninstall-confirm", uninstall_uploading(2)),
        ("uninstall-confirm", uninstall_stranded(1)),
        ("uninstall-confirm", uninstall_stranded(2)),
        ("uninstall-confirm", disconnect_saved_waits()),
        ("uninstall-confirm", save_them_to_movies(2)),
        ("uninstall-confirm", cancel()),
        ("uninstall-confirm", uninstall()),
        ("uninstall-confirm", uninstalling()),
        // ── updates ──
        ("update-ready", update_ready("1.3.0")),
        ("update-ready", update()),
        ("update-ready", whats_new()),
        ("updating-app", updating()),
        ("updated", updated("1.3.0")),
        ("update-failed", update_failed()),
        // ── Region 2's afterlife ──
        ("space-degraded", space_degraded()),
        ("space-degraded", reconnect_space()),
        ("token-repair-mood", token_repair()),
        ("token-repair-mood", create_token()),
        ("token-other-account", token_other_account("v.myshroom.workers.dev")),
        ("space-storage-gone", storage_gone()),
        ("space-storage-gone", set_it_up_fresh()),
        ("space-storage-gone", disconnect_ellipsis()),
        ("name-changed", name_changed()),
        // ── Region 6: the Members face ──
        ("settings", tab_members()),
        ("invite-born", members_title()),
        ("invite-born", members_truth()),
        ("invite-born", you()),
        ("invite-born", add_another_mac()),
        ("invite-born", joined_row("Mher", "Aug 3")),
        ("invite-born", invited_now("Ani")),
        ("invite-born", invited_fact("Aug 21", 5)),
        ("invite-born", remove_ellipsis()),
        ("invite-born", copy_link()),
        ("invite-born", copied()),
        ("invite-born", send_link("Ani")),
        ("invite-born", who_for()),
        ("invite-born", invite()),
        ("invite-born", invite_line()),
        ("invite-born", invites_from("Bagrat")),
        ("invite-born", change()),
        ("invite-born", invites_from_label()),
        ("invite-born", save()),
        ("invite-born", new_link()),
        ("invite-born", keeps_token()),
        ("add-another-mac", copy_link()),
        ("add-another-mac", own_mac_link_line()),
        ("member-remove-confirm-member", remove_title("Mher")),
        ("member-remove-confirm-member", remove_member_line("Mher")),
        ("member-remove-confirm-member", cancel()),
        ("member-remove-confirm-member", remove()),
        ("member-remove-confirm-invite", cancel_invite_title("Ani")),
        ("member-remove-confirm-invite", cancel_invite_line()),
        ("member-remove-confirm-invite", keep_it()),
        ("member-remove-confirm-invite", cancel_invite()),
        ("member-remove-confirm-own-mac", remove_title("MacBook Air")),
        ("member-remove-confirm-own-mac", remove_mac_line()),
        // ── the join machine ──
        ("member-welcome", invited_you("Bagrat")),
        ("member-welcome", member_truth("Bagrat")),
        ("member-welcome", member_step()),
        ("member-welcome", login_box()),
        ("member-welcome", join()),
        ("member-welcome", later()),
        ("member-welcome-own-mac", add_this_mac()),
        ("member-welcome-own-mac", same_videos("v.myshroom.workers.dev")),
        ("member-welcome-own-mac", member_step_here()),
        ("joining", space_header(Some("Bagrat"))),
        ("joining", joining_row(Some("Bagrat"))),
        ("joining", row_ahead(6)),
        ("joining", joining_line()),
        ("joining-own-mac", space_header(None)),
        ("joining-own-mac", space_row_done(None, "v.myshroom.workers.dev")),
        ("member-done", space_row_done(Some("Bagrat"), "v.myshroom.workers.dev")),
        ("member-done", done_ask()),
        ("member-done", done_flip()),
        ("member-done", allow_recording()),
        ("member-all-set", all_set()),
        ("member-all-set", menu_bar()),
        ("member-all-set", close()),
        ("member-all-set", record_now()),
        ("member-all-set-own-mac", invite_someone()),
        ("join-waiting", join_offline()),
        ("join-waiting", join_not_answering("Bagrat")),
        ("join-expired", invite_expired("Bagrat")),
        ("join-expired", new_link_door()),
        ("join-expired-page", invite_unknown()),
        ("join-expired-page", own_mac_expired()),
        ("join-used", invite_used("Bagrat")),
        ("join-space-missing", space_missing("Bagrat")),
        ("join-already-here", already_here("Bagrat")),
        ("join-already-here", already_here_own()),
        ("join-already-here", close()),
        ("join-switch-confirm", invited_you("Bagrat")),
        ("join-switch-confirm", switch_line("v.mine.workers.dev", "Bagrat")),
        ("join-switch-confirm", switch_only_holder()),
        ("join-switch-confirm", switch_half_set_up("Bagrat")),
        ("join-switch-confirm", join_space("Bagrat")),
        ("join-switch-confirm", stay()),
        ("join-switch-confirm", after_take()),
        ("join-switch-confirm-uploads", switch_uploading(2)),
        ("join-switch-confirm-saved-take", switch_saved_waits()),
        // ── member life ──
        ("member-revoked", member_revoked("Bagrat")),
        ("member-revoked", save_n_to_movies(1)),
        ("member-revoked", save_n_to_movies(3)),
        ("member-revoked", saved_to_movies()),
        ("member-revoked", movies_untitled("Aug 21")),
        ("member-revoked", discard_ellipsis()),
        ("member-revoked", leave_space_ellipsis()),
        ("space-needs-owner", needs_owner("Bagrat")),
        ("space-gone", space_gone("Bagrat")),
        ("space-gone", save_n_to_movies(3)),
        ("space-gone", saved_to_movies()),
        ("space-gone", leave_space_ellipsis()),
        ("stopped-stranded", stranded_title()),
        ("stopped-stranded", member_revoked("Bagrat")),
        ("stopped-stranded", space_gone("Bagrat")),
        ("stopped-stranded", save_to_movies()),
        ("stopped-stranded", reconnect_space()),
        ("space-needs-update-member", needs_update_member("Bagrat")),
        ("space-needs-update-no-token", needs_update_owner_class()),
        ("space-needs-update-no-token", add_token_here()),
        ("space-degraded-no-token-mac", degraded_no_token()),
        ("space-degraded-no-token-mac", reconnect_here()),
        // ── Region 5's member parts ──
        ("settings-space", mark_label()),
        ("settings-space", mark_line()),
        ("settings-space", leave_space_ellipsis()),
        ("settings-space-no-token", space_row_done(None, "v.myshroom.workers.dev")),
        ("settings-space-no-token", space_done(6, Some(&on_this_mac()))),
        ("settings-space-no-token", repairs_elsewhere()),
        ("settings-space-no-token", add_token_here()),
        ("settings-space-member", space_row_done(Some("Bagrat"), "v.myshroom.workers.dev")),
        ("settings-space-member", member_space_line("Bagrat")),
        ("settings-space-member", space_row_off(Some("Bagrat"), "v.myshroom.workers.dev")),
        ("settings-space-member", member_revoked("Bagrat")),
        ("settings-space-member", space_gone("Bagrat")),
        ("settings-space-no-token", space_row_off(None, "v.myshroom.workers.dev")),
        ("settings-space-no-token", owner_mac_removed()),
        ("leave-confirm", leave_title("Bagrat")),
        ("leave-confirm", leave_line("Bagrat")),
        ("leave-confirm", cancel()),
        ("leave-confirm", leave()),
        ("disconnect-confirm-no-token", disconnect_title()),
        ("disconnect-confirm-no-token", disconnect_owner_class_line()),
        ("disconnect-confirm-no-token", disconnect_uploading(2)),
        ("disconnect-confirm-no-token", disconnect()),
        ("library", mine()),
        ("library", everyone()),
        ("delete-confirm-other-member", their_video("Mher")),
        // The build's lines, said-lines of spec/machine.json since B133 (the copy table's,
        // and the states only the code had).
        ("config", camera_only()),
        ("config", camera_off_pick()),
        ("config", no_mic_pick()),
        ("config", display_numbered("LG UltraFine", 1)),
        ("config", record_after_switch("Bagrat")),
        ("restart-confirm", restart_title()),
        ("restart-confirm", restart_line("4:32")),
        ("wizard-unexpected", unexpected_wizard()),
        ("wizard-unexpected", try_again()),
        ("library", row_uploading_elsewhere()),
        ("discard-confirm-saved-take", toss_unknown_length()),
        ("settings-general", running_version("1.2.0")),
        ("disconnect-confirm", disconnect_saved_waits()),
        ("invite-born", invited_expired("Sona", "Aug 10")),
        ("invite-born", your_mac("MacBook Air", "Sep 2")),
        ("invite-born", invite_failed()),
        ("invite-born", remove_failed()),
        ("invite-born", link_prefix("v.myshroom.workers.dev")),
        ("join-waiting", join_not_answering_unnamed()),
        ("join-waiting", own_space_not_answering()),
        ("join-used", own_mac_used()),
        ("join-space-missing", space_missing_unnamed()),
        ("join-switch-confirm", switch_stranded(2, "v.mine.workers.dev")),
        ("owner-mac-removed", owner_mac_removed()),
        ("stopped-stranded", saving_to_movies()),
        ("stopped-stranded", save_to_movies_again()),
        ("leave-confirm", leave_uploading(2)),
    ]
}
