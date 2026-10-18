//! Everything that can happen to the core: the person, the OS, the mechanism lanes.

use serde::{Deserialize, Serialize};

use crate::scene::Action;
use crate::world::{
    Access, Account, Blocked, Device, Display, FailClass, Gates, InviteKind, LaunchKind, LibraryRow, Once, Op, OsSession, PendingInvite, Prefs,
    QuickProbe, Queued, Reach, RecId, Restore, Roster, SavedTake, SecretId, SpaceIdentity, TakeDir, TokenVerdict, UpdateOffer, WindowKind,
    WorkerFact,
};
use crate::time::Wall;

/// What the shell read from disk at launch.
#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub launch: LaunchKind,
    pub version: String,
    pub prefs: Prefs,
    pub once: Once,
    pub space: Option<SpaceIdentity>,
    pub token: bool,
    pub device_key: bool,
    pub primed: Option<Wall>,
    /// `restore.json`, if one was waiting (read once, then deleted — bend 9).
    pub restore: Option<Restore>,
    pub quick: QuickProbe,
}

/// Which field a secret was pasted into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecretField {
    /// The wizard's "Paste your token".
    Token,
    /// The welcome's invite door, "Paste your invite link".
    Invite,
}

/// What the shell's classifier made of a paste — its shape only, never its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// Shaped like a Cloudflare token.
    Token,
    /// Token-like but damaged (lost characters in the copy).
    Garbled,
    /// A shroom join link.
    JoinLink,
    /// Anything else.
    Other,
}

/// Non-secret fields whose contents the core echoes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Field {
    /// The link name, typed into `https://v.___.workers.dev/…`.
    Name,
    /// The Library's “Search titles”.
    LibrarySearch,
    /// A Library title being renamed in place.
    Title,
    /// The Members composer's “Who's this for?” — the admin-private label (N28).
    InviteName,
    /// The name invites say they're from, while it's being changed (B158).
    FromName,
    /// Report a problem's description (B180).
    ReportWords,
    /// Report a problem's “Your email, if you'd like an answer” (B180).
    ReportReplyTo,
    /// Send feedback's words (N60).
    FeedbackWords,
    /// Send feedback's “Your email, if you'd like an answer” (N60).
    FeedbackReplyTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Surface {
    Tray,
    Popover,
    Onboarding,
    Library,
    Settings,
    /// The Send feedback window (N60).
    Feedback,
    Countdown,
    Bubble,
    /// The agent label (N62, B216): it takes no clicks and no keys; only its slice is read.
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Key {
    Return,
    Esc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitVia {
    CmdQ,
    Dock,
    Footer,
    RightClick,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Booted(Box<Disk>),
    // ── the person ──
    TrayClicked { right: bool, quick: QuickProbe },
    /// The reopen event (Spotlight, Finder, the Dock) while running — a click (N41).
    Reopened(QuickProbe),
    /// ⌥⌘P, held while a take is open (N54): the tray click's pause, or Resume.
    Shortcut(QuickProbe),
    Act { surface: Surface, action: Action, rev: u64 },
    Key { surface: Surface, key: Key },
    PopoverDismissed,
    /// Non-secret field contents, as typed (the runner only caps the length). The core
    /// filters them — and shows the charset hint when it drops a character.
    Field { field: Field, value: String },
    /// The one inbound path for secret text: the shell moved it into the stash and sends
    /// only the handle and its shape (law 13).
    SecretPasted { field: SecretField, id: SecretId, shape: Shape },
    /// A window's ✕ (accepted whatever the rev).
    WindowClosed(WindowKind),
    /// A window came to the front again — every entry re-probes (law 1).
    WindowFocused(WindowKind, QuickProbe),
    /// The first key event in a non-key popover: it becomes asked.
    PopoverKeyed,
    /// The person dragged the bubble: its top-left corner's offset, in points, from the
    /// top-left of the display it's on (N22: you place yourself before Start).
    BubbleMoved((i32, i32)),
    QuitRequested(QuitVia),
    /// The `shroom` command asked to start, pause, resume or stop a recording, through the
    /// app's control socket: `id` is the runner's, for its answer (`Effect::Cli`).
    Cli { id: u64, op: CliOp },
    /// A `shroom://settings/…` link opened with shroom (N62): the command's recording refusal
    /// carries one, for the person to click.
    SettingsLink(SettingsSpot),
    // ── the OS ──
    Os(OsNews),
    // ── the mechanism ──
    Rec(RecNews),
    /// The takes scan: saved takes, published takes still uploading, and published takes
    /// another Mac's delete stopped (tombstoned here, still this Mac's to keep or toss).
    Scanned { saved: Vec<SavedTake>, queue: Vec<Queued>, removed: Vec<Queued> },
    Cloud(CloudNews),
    Upload(UploadNews),
    Library(LibraryNews),
    Update(UpdateNews),
    /// The space's Worker answering this Mac (Region 6, WORKER-INTERFACE §4).
    Member(MemberNews),
    /// Report a problem's lane: the report gathered, or Send's answer (B180).
    Report(ReportNews),
    /// The runner's catch-all: an error, a caught panic, or a deadline passing.
    Failed { op: Op, class: FailClass },
    /// Posted at `Scene.next_wake`.
    Tick,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OsNews {
    Grants { screen: bool, mic: Access, camera: Access },
    /// The screen-recording ask landed (N43): `alert` = macOS raised its own alert (the
    /// first ask), so shroom opened nothing; else shroom opened the pane itself.
    ScreenAsked { alert: bool },
    Devices { displays: Vec<Display>, mics: Vec<Device>, cameras: Vec<Device> },
    Disk(u64),
    Session(OsSession),
    /// The Mac woke: every network fact is stale (monotonic time skipped the sleep).
    Woke,
    /// macOS is about to sleep and waits for shroom's answer (IOKit's system-will-sleep,
    /// DECISIONS B92): a take recording pauses at that instant (`mac-away`), and the answer
    /// (`ShellCmd::AllowSleep`) goes once it's paused — or two seconds on, whatever happened.
    WillSleep,
    NetPath(bool),
    StatusItemVisible(bool),
    /// Whether shroom is registered to start at login.
    LoginItem(bool),
    /// This Mac's offset from UTC, in minutes (the Library's dates).
    UtcOffset(i32),
    /// The app is being terminated; `ours: false` = by macOS (its quit-and-reopen offer).
    Terminating { ours: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecNews {
    /// Capture began; the lane minted the take's dir and recording id.
    Started { dir: TakeDir, rec: RecId },
    Paused { length_ms: u64 },
    Resumed,
    /// The take was finalized (its length), or finalizing failed.
    Finalized(Result<u64, FailClass>),
    /// Capture couldn't start or died.
    Failed,
    /// Capture stopped making progress mid-take: the encoder took no frame past the
    /// deadline, it exited, or the source reported it stopped (the recorder's watchdog,
    /// DEVIATIONS #6).
    FrameStall,
    /// The lane let a dead capture go (`Abandon`) and its folder holds nothing a repair can
    /// save — no frame reached it, or none survived: there is no take to keep, only to say
    /// (DEVIATIONS #53). In place of `Abandoned`.
    NothingKept { camera: bool },
    /// Free space reached the floor mid-take (the recorder's watchdog: both tee legs and
    /// the single-file copy still to come, `derive::take_floor_bytes`).
    DiskFloor,
    /// The take's microphone stopped delivering mid-take (its device went away, or failed):
    /// N39's `mic-lost`.
    MicLost,
    /// `Abandon` ran: capture is let go and the take's lock released, its folder kept — the
    /// scan can read it as interrupted now (a scan racing the release would see it live).
    Abandoned,
    Primed(bool),
    Deleted(TakeDir),
    SavedToMovies(TakeDir),
    /// Save to Movies couldn't make the file (no single-file copy could be built, no room,
    /// Movies not writable): nothing was lost, the take stays as it was.
    SaveFailed(TakeDir),
    /// The running length, for the paused title (sent on pause and periodically).
    Length(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloudNews {
    Reached(Reach),
    PageOpened(RecId),
    /// Verify and probe-by-use answered for a paste: the accounts it reaches.
    Validated { id: SecretId, result: Result<Vec<Account>, TokenFail> },
    /// The token is in `~/.shroom` (mode 600) — before any wiring.
    TokenSaved(SecretId),
    /// (Part of) the probe pass answered.
    Swept(SweepAnswer),
    /// The workers.dev name claim answered.
    Claimed { name: String, result: Result<(), ClaimFail> },
    /// Wiring ran to its silent self-check.
    Wired(Result<SpaceIdentity, WireFail>),
    /// The page refresh answered (§6.10): `deployed` — this app's page went up over an
    /// older one.
    Refreshed { deployed: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenFail {
    /// A live token without both powers (probe by use).
    WrongPowers,
    /// The verify step: not a live token at all — a copy that lost characters past the
    /// shape check, most likely (`token-garbled`'s line, DEVIATIONS #26).
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimFail {
    /// 10031.
    Taken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFail {
    /// The fallback classification: a gate closed between the card step and wiring.
    Gate(Gates),
    /// Anything unclassified: retried after the backoff.
    Refused,
}

/// What a probe pass answered. `None` = that part wasn't probed this time. A full pass
/// always answers `token` (its first probe), even when it stops there; the gates' poll
/// never does — that is how the core tells a whole pass from a poll.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SweepAnswer {
    pub token: Option<TokenVerdict>,
    pub gates: Option<Gates>,
    pub subdomain: Option<Option<String>>,
    pub foreign_v: Option<bool>,
    /// The survey's label (`shroom-cloud::sweep::pick_label`); `Some(None)` = every label
    /// is someone else's.
    pub label: Option<Option<String>>,
    pub bucket: Option<bool>,
    pub worker: Option<WorkerFact>,
    pub found: Option<Option<u32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UploadNews {
    Progress { dir: TakeDir, done: u32, total: u32 },
    Live(TakeDir),
    Blocked(TakeDir, Blocked),
    Tombstoned(TakeDir),
    /// The uploader ran out of work for now (ends its op).
    Idle,
    /// The takes scan's credentials: each published take holding an upload credential, and
    /// when it expires (N32's “already on its way”).
    Credentials(Vec<(TakeDir, Wall)>),
    /// The uploader minted a take's upload credential after the scan (a take recorded
    /// offline, a credential re-minted near its expiry) — `Some(until)` — or dropped one
    /// the space refused (`None`).
    Minted { dir: TakeDir, until: Option<Wall> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryNews {
    /// The space's records: `fresh: false` = the local cache, at once (the refresh still
    /// runs); `fresh: true` = the bucket refresh answered (ends its op).
    Rows { rows: Vec<LibraryRow>, fresh: bool },
    /// A rename landed (a record rewrite under `If-Match`, B21).
    Renamed { rec: RecId, title: String },
    /// Tombstones written and the bytes gone (§5.5's order).
    Deleted(Vec<RecId>),
    /// The token Mac's cleanup sweep answered (DECISIONS B91) — counts only, for the log.
    Cleaned { under_tombstones: u32, orphans: u32, keys: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNews {
    /// The background check answered: newest only, `None` = nothing newer.
    Checked(Option<UpdateOffer>),
    /// This version is downloaded and verified exactly as the installer verifies.
    Staged(String),
}

// ── Region 6 (WORKER-INTERFACE §4, typed by `shroom-cloud::member`) ──
//
// REGION 6 INTERFACE (tests/region6.rs, 2026-10-02): what the runner's member lane reports.
// A join link only ever rides as a stash handle (law 13): never its code, never a device
// secret, never `k`.

/// The space's Worker answering this Mac, or a join link arriving.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberNews {
    /// A join link arrived by a door that isn't a paste — the page's Open shroom (the
    /// `shroom://join/…` deep link), or the installer's hand-off (`~/.shroom/invite` and
    /// `--invite`, B29 amended) — already in the stash and on disk (DEVIATIONS #2). A paste
    /// arrives as `SecretPasted { field: Invite, shape: JoinLink }` instead. `remembered`:
    /// the file was found at a launch without `--invite` (Later kept it) — it opens nothing
    /// by itself; the next tray click does.
    Arrived { link: SecretId, host: String, remembered: bool },
    /// The installer's hand-off held text that isn't a join link (the file is removed
    /// already): the paste door's answer for the same text — the nameless dead-link face
    /// (`invite_unknown`), with its paste door. `opened`: an install or reopen carried it.
    Unreadable { link: SecretId, opened: bool },
    /// `join.json` was found at launch (§5): a join this Mac asked for may have landed on the
    /// space before a crash. Its link is already in the stash; the core holds it as joining,
    /// and the identical join replays (`MemberCmd::Join`) until it answers for good — so a
    /// 200 that landed is never lost, and this Mac never reads its own invite as used.
    JoinPending { link: SecretId, host: String },
    /// The arrival's ping and peek answered (§4.2, §4.4, §6.2 steps 2–3). A transport failure
    /// is `Failed { op: Peek, class: Transport }`.
    Peeked { link: SecretId, peek: PeekAnswer },
    /// join answered (§4.5's table, `JoinNext`) — or its replay from `join.json` at launch
    /// (`JoinPending`). A transport failure is `Failed { op: Join, class: Transport }`; a
    /// `JoinNext::Bug` too (`class: Unclassified` — it keeps `join.json`, §4.5).
    Joined { link: SecretId, answer: JoinAnswer },
    /// This Mac's own space answered — the ping and `/me` (§4.2, §4.7: law 1's re-probe on a
    /// Mac without the token), a mint (§4.8) or a fresh credential's first PUT (§6.7).
    ///
    /// Every answer below is about the space this Mac is in **now**. The runner's member lane
    /// calls one host at a time, and a switch is its own act (on join's 200 it deregisters
    /// from the old host and wipes that space's files before it reports `Joined`): an answer
    /// still in flight from the host it left is the lane's to drop, never reported (DECISIONS
    /// B41 h).
    Standing(StandingAnswer),
    /// `GET /members` answered (§4.11).
    Roster(Roster),
    /// `POST /invites` answered 201 (§4.12): the row, and the new link in the stash.
    Invited { invite: PendingInvite, link: SecretId },
    /// A removal or a cancel landed (§4.13–§4.15): that row (member, invite or Mac id) is gone.
    Removed(String),
    /// `POST mark` answered (§4.16): the switch as the space now has it.
    Marked(bool),
}

/// peek's answer through the shape rule (§4.4, U2), with its host.
#[derive(Debug, Clone, PartialEq)]
pub enum PeekAnswer {
    /// Unused and unexpired. `inviter`: the inviting Mac account's macOS short name (N29);
    /// `current`: the space's page has every route this app calls (ping `iface ≥
    /// IFACE_MIN`, §6.10).
    Pending { host: String, kind: InviteKind, inviter: String, current: bool },
    /// Past its days, or cancelled from Members; `inviter: None` = an unknown code.
    Expired { host: String, kind: InviteKind, inviter: Option<String> },
    Used { host: String, kind: InviteKind, inviter: String },
    /// The link's address answers as no space at all (the ping isn't shroom's).
    SpaceMissing { host: String },
}

/// join's answer (§4.5).
#[derive(Debug, Clone, PartialEq)]
pub enum JoinAnswer {
    /// 200: the identity `space.json` now holds — role `Member`, or `OwnerClass` with
    /// member #1's id for an own-Mac link. On a switch, the old space was deregistered and
    /// its files wiped only now (§5: a failed join never strands a Mac outside its space).
    Joined(SpaceIdentity),
    /// 404 `unknown`, 410 `expired` / `cancelled`.
    Expired,
    /// 409 `used`.
    Used,
    /// 409 `already-here`.
    AlreadyHere,
    /// Not shroom-shaped, and the re-ping isn't shroom's.
    SpaceMissing,
    /// `busy`, `space`, 429, 5xx: the space isn't answering right now — the identical body
    /// replays after the backoff.
    Waiting,
    /// Not shroom-shaped, and the re-ping found the space's page older than this app needs
    /// (`iface < IFACE_MIN`, U3, §6.10): the join stops on the needs-update line (open
    /// question 15). Clears `join.json` like any answer for good.
    Behind,
}

/// What this Mac's own space said about it (N34; §6.7's table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandingAnswer {
    /// `/me` 200. `current`: the page has every route this app calls (ping `iface ≥
    /// IFACE_MIN`); owner-class Macs also hear whether this Mac is its space's only
    /// token-holder (§4.7, U9) and the mark switch.
    Fine { current: bool, only_holder: Option<bool>, mark: Option<bool> },
    /// `/me` or a mint: 401, or 403 `removed` — this identity is dead here. Never debounced.
    Revoked,
    /// A mint or `/me` 503 `space`, or a fresh credential refused on its first PUT. A face
    /// only after two such answers at least 30 s apart (C7).
    SpaceSide,
    /// The ping isn't shroom's (or NXDOMAIN while online). Debounced the same way.
    Gone,
}

/// What the shell says about a report (B180).
#[derive(Debug, Clone, PartialEq)]
pub enum ReportNews {
    /// The report, gathered for the sheet: its disclosure, and what Send posts.
    Gathered(crate::report::Diagnostics),
    /// shroom.am took the report of this kind.
    Sent(crate::report::Kind),
    /// It didn't go through: offline, refused, rate-limited or switched off.
    NotSent(crate::report::Kind),
}

/// A place in Settings a `shroom://settings/…` link opens (spec N62, DECISIONS B216): General's
/// switch “Let it start and stop recordings”, the link the `shroom` command's recording
/// refusal carries. The link only opens and lights it; the person flips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSpot {
    AgentRecording,
}

impl SettingsSpot {
    const ALL: [SettingsSpot; 1] = [SettingsSpot::AgentRecording];

    /// The link that opens it.
    pub const fn link(self) -> &'static str {
        match self {
            SettingsSpot::AgentRecording => "shroom://settings/agent",
        }
    }

    /// The tab it is on.
    pub const fn tab(self) -> crate::world::SettingsTab {
        match self {
            SettingsSpot::AgentRecording => crate::world::SettingsTab::General,
        }
    }

    /// The spot a link opens, read exactly (a trailing slash and the case aside): any other
    /// link, a query or a fragment included, is not one.
    pub fn of_link(url: &str) -> Option<SettingsSpot> {
        let url = url.trim();
        let url = url.strip_suffix('/').unwrap_or(url);
        SettingsSpot::ALL.into_iter().find(|s| url.eq_ignore_ascii_case(s.link()))
    }
}

/// What the `shroom` command may ask of the recording — each one exactly what its click does
/// in the popover or the tray (dev/proposals/cli-agent-skill.md §3) — and how a start goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliOp {
    Start(StartAsk),
    Pause,
    Resume,
    Stop,
}

impl CliOp {
    /// A start as the popover's: the count, and the label while it records.
    pub const START: CliOp = CliOp::Start(StartAsk { countdown: true, label: true });
}

/// How the command's start goes (N62, B216; Bagrat, 2026-10-18): the count, unless
/// `--no-countdown`; the agent label while the take records, unless `--no-label`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartAsk {
    pub countdown: bool,
    pub label: bool,
}

impl Default for StartAsk {
    fn default() -> StartAsk {
        StartAsk { countdown: true, label: true }
    }
}
