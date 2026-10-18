//! The World: what is true (facts) kept apart from what the person asked for (prefs,
//! once, session). No type here can name a screen — law 1 (a fitness test greps this file).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::time::{Fact, Mono, Wall};

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    pub now: Mono,
    pub wall: Wall,
    // ── facts ──
    pub launch: Launch,
    pub mac: Mac,
    pub net: Net,
    pub space: Space,
    /// The one provisioning probe pass and what the wizard's calls answered (Region 2's
    /// backbone): every row of the plan is marked from these, never from a stored step.
    pub sweep: Sweep,
    /// Mirrors the live recorder; never persisted.
    pub take: Option<OpenTake>,
    /// Finalized takes without a link, oldest first (from the takes scan).
    pub saved: Vec<SavedTake>,
    /// Published takes still uploading (from the takes scan).
    pub queue: Vec<Queued>,
    /// Published takes whose upload another Mac's delete stopped (the tombstone found
    /// before a resume): kept on this Mac — “Removed from the space” (N35, DEVIATIONS #7).
    pub removed: Vec<Queued>,
    /// The space's records as the Library knows them: the local cache at once, then the
    /// bucket refresh (ARCHITECTURE §13). Tombstones never arrive here.
    pub library: Fact<Arc<Vec<LibraryRow>>>,
    /// The Library's writes under way, and when its last refresh answered.
    pub library_ops: LibraryOps,
    /// What the background update check found, and how applying it went (N37).
    pub update: Updates,
    /// Region 6: the invite, this Mac's standing in its space, the roster.
    pub members: Members,
    pub inflight: Inflight,
    /// When the popover last closed — ages one-shot status lines.
    pub popover_closed: Option<Mono>,
    // ── what the person asked for ──
    pub prefs: Prefs,
    pub once: Once,
    pub session: Session,
    /// Which display a click put `config` on, and whether one was picked by hand since
    /// (N56). In memory only, like the session; kept beside it, since it outlives the
    /// popover and the session holds at most twelve intents.
    pub display_pick: DisplayPick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchKind {
    /// The person launched it (Spotlight, Finder, the Dock) — a launch you did is a click (N41).
    #[default]
    User,
    /// macOS started it at login (the launch event says so) — silent, tray-only (N41).
    LoginItem,
    UpdateRelaunch,
    PermissionRelaunch,
    /// The install script's launch (`--relaunched=install`, or `--invite` when it handed
    /// over a join link): a fresh install acts as a click, and its welcome is the first launch's.
    Install { join: bool },
    /// The install script reopened shroom after replacing it (`--relaunched=install-update`):
    /// the `updating` lane — silent, tray-only, and no “Updated to … ✓” (N9, N41).
    InstallUpdate,
    /// The swap helper couldn't put the new version in place and reopened the old one
    /// (`--update-failed`): the running version is unchanged.
    UpdateFailed,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Launch {
    pub kind: LaunchKind,
    pub version: String,
    /// This process is quitting so macOS's screen grant can take effect (Reopen shroom, or
    /// macOS's own quit-and-reopen) — the `relaunching` beat. Ends with the process, or
    /// clears when the relaunch's deadline passes (`Op::Relaunch`), so a relaunch that never
    /// comes re-derives `perm-screen` with Reopen shroom offered again.
    pub ending: bool,
    /// When this process booted: the installer's launch opens the welcome then, and only
    /// that window is `first-launch` (a later tray-click welcome is the any-day one).
    pub booted: Mono,
}

/// Why the app relaunched itself, written to `restore.json` on the way out (bend 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelaunchWhy {
    Update,
    Permission,
}

/// `restore.json`: the window to bring back after a relaunch shroom asked for. Honored for
/// two minutes when its provenance matches (DEVIATIONS #8, #9), then deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Restore {
    pub why: RelaunchWhy,
    /// The window to reopen — after an update, the one that was in front (DEVIATIONS #14).
    pub window: WindowKind,
    pub written: Wall,
}

/// A handle into the runner's stash of pasted secrets. The core never holds secret text
/// (law 13): only these handles, and whether a token or device key exists. A join link
/// rides the same way — pasted, opened by the deep link, handed over by the installer, or
/// made by Invite — so its code never reaches the core, the Scene or an effect (Region 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecretId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    #[default]
    NotAsked,
    Restricted,
    Denied,
    Granted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Display {
    pub id: u32,
    pub name: String,
    pub is_main: bool,
    /// Left edge in global coordinates — tells two displays of one name apart.
    pub x: i32,
    /// Top edge in global coordinates (y down) — orders two stacked displays of one name.
    #[serde(default)]
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub uid: String,
    pub name: String,
    pub builtin: bool,
    /// An iPhone over Continuity — never chosen automatically (N39).
    pub continuity: bool,
    /// macOS's default now — the input device, or the preferred camera (N44): what System
    /// default means.
    #[serde(default)]
    pub system_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsSession {
    pub awake: bool,
    pub unlocked: bool,
    pub on_console: bool,
}

impl Default for OsSession {
    fn default() -> Self {
        OsSession { awake: true, unlocked: true, on_console: true }
    }
}

impl OsSession {
    pub fn present(&self) -> bool {
        self.awake && self.unlocked && self.on_console
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mac {
    pub screen: Fact<bool>,
    pub mic: Fact<Access>,
    pub camera: Fact<Access>,
    pub displays: Fact<Vec<Display>>,
    pub mics: Fact<Vec<Device>>,
    pub cameras: Fact<Vec<Device>>,
    pub disk_free: Fact<u64>,
    pub os: Fact<OsSession>,
    pub status_item_visible: bool,
    /// Whether shroom is registered to start at login (the OS's answer).
    pub login_item: Fact<bool>,
    /// The Allow recording click put the microphone prompt to macOS in this process and
    /// its answer hasn't come: that answer chains the screen step (law 11's one chained
    /// prompt). Consumed by the chain; never set by anything but the click.
    pub mic_asked: bool,
    /// The screen-recording step was put to macOS in this process — the chapter asks it
    /// once, never again on every grants report.
    pub screen_asked: bool,
    /// The mic's answer asked the camera in this process (N43): its answer chains the
    /// screen step. Consumed by the chain.
    pub camera_asked: bool,
    /// How macOS took the last screen-recording ask in this process (N43): `Some(true)` =
    /// its own alert is up and leads, `Some(false)` = shroom opened the pane itself.
    pub screen_alert: Option<bool>,
    /// The last clean warm-up capture; `Some` means this Mac's permissions chapter is done.
    pub primed: Option<Wall>,
    /// This Mac's offset from UTC, in minutes — the Library's dates are its calendar's.
    pub utc_offset_min: i32,
    /// shroom holds off idle sleep now (N41: uploads hold off idle sleep, never lid
    /// sleep) — what the core last asked the shell, set when the effect is emitted, so the
    /// rule asks once per change and never again.
    pub awake_held: bool,
    /// macOS said it will sleep and is waiting for shroom's answer (ARCHITECTURE §3.3's
    /// power row, DECISIONS B92): since when. The answer (`ShellCmd::AllowSleep`) goes the
    /// moment the take is safe — paused, or no take — and at most `SLEEP_HOLD_MS` later
    /// whatever happened: shroom never holds sleep beyond the pause.
    pub sleep_hold: Option<Mono>,
}

/// Local facts the shell gathers synchronously with every entry event, so a surface never
/// opens on a stale grant and the core never probes inline.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickProbe {
    pub screen: bool,
    pub mic: Access,
    pub camera: Access,
    pub displays: Vec<Display>,
    pub mics: Vec<Device>,
    pub cameras: Vec<Device>,
    pub disk_free: u64,
    pub online: Option<bool>,
    pub os: OsSession,
    pub status_item_visible: bool,
    /// Whether shroom starts at login, as macOS says now (`SMAppService`, read only) —
    /// `None` when it couldn't be read. Re-read on every entry, so Settings never shows a
    /// box the person changed in System Settings (law 1, N36).
    pub login_item: Option<bool>,
    /// The display whose menu bar the mushroom was clicked in (each display has its own
    /// with “Displays have separate Spaces”), when the shell can tell: the source a click
    /// opens `config` with (N56). Read at every probe; only a click, or a launch you did,
    /// uses it. `None` when the shell can't tell.
    pub clicked_display: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The link's address answers as this space.
    Shroom,
    /// It answers, but as no space at all.
    NotShroom,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Net {
    pub online: Fact<bool>,
    pub reach: Fact<Reach>,
    /// The stop's probe's run of consecutive “answers, but as no space at all” since the
    /// stop (DECISIONS B60, amends B52 b): a freshly wired workers.dev address answers so
    /// for a moment while it propagates, so one answer never strands a stop — the verdict
    /// must hold for [`NOT_SHROOM_HOLD`] answers in a row, [`NOT_SHROOM_REPROBE_MS`] apart.
    pub not_shroom: Option<Streak>,
}

/// A run of the same probe answer: how many in a row, and when the last came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Streak {
    pub count: u8,
    pub last: Mono,
}

/// How many “not shroom” answers in a row make the stop's verdict (B60).
pub const NOT_SHROOM_HOLD: u8 = 3;
/// How long after an unconfirmed “not shroom” answer the stop's probe asks again (B60) —
/// B50's propagation poll's own cadence.
pub const NOT_SHROOM_REPROBE_MS: u64 = 3_000;

impl Net {
    /// The stop's probe has said “not shroom” often enough, in a row, to believe it.
    pub fn not_shroom_held(&self) -> bool {
        self.reach.is(&Reach::NotShroom) && self.not_shroom.is_some_and(|s| s.count >= NOT_SHROOM_HOLD)
    }

    /// The last answer was “not shroom”, but not yet often enough to believe: when to ask again.
    pub fn not_shroom_reprobe(&self) -> Option<Mono> {
        if !self.reach.is(&Reach::NotShroom) || self.not_shroom_held() {
            return None;
        }
        self.not_shroom.map(|s| s.last.plus(NOT_SHROOM_REPROBE_MS))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    TokenHolder,
    OwnerClass,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceIdentity {
    pub account_id: String,
    pub label: String,
    pub subdomain: String,
    /// e.g. `v.myshroom.workers.dev`
    pub host: String,
    pub bucket: String,
    pub role: Role,
    pub member_id: String,
    pub owner_name: String,
    /// The Cloudflare account's name, kept when the token reached several accounts and
    /// this one was chosen (N42): row one's fact, for good.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    /// The setup's survey **found** a Worker named `v` that isn't shroom's, so the space
    /// took another label (N42): the name row's “v was already taken”, for good. The label
    /// alone never says so — a build that spares `v` passes it by without a look, and a
    /// space kept before this was recorded says nothing about `v` either.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub foreign_v: bool,
}

/// A Cloudflare account a token reaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}

/// What a pasted token turned out to be, before it's saved.
#[derive(Debug, Clone, PartialEq)]
pub enum Paste {
    /// Shape passed; verify and probe-by-use are due.
    Checking(SecretId),
    /// Caught by the shape check, before any network call.
    Garbled,
    /// A real token without the two powers (probe by use, never echo).
    WrongPowers,
    /// Verified, and it reaches several accounts (N42): which one holds the videos?
    PickAccount { id: SecretId, accounts: Vec<Account> },
    /// The repair mood only: a live token, but for a Cloudflare account that isn't the
    /// space's — never builds a second space by accident (N42).
    OtherAccount,
    /// Verified for one account (or the one picked); writing it to `~/.shroom` is due.
    Saving { id: SecretId, account: Account, several: bool },
}

impl Paste {
    /// The pasted token is still in hand: being checked, waiting on which account, or
    /// being put away. A refused paste holds nothing — its text is already forgotten.
    pub fn held(&self) -> bool {
        matches!(self, Paste::Checking(_) | Paste::PickAccount { .. } | Paste::Saving { .. })
    }
}

/// The stored token's verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenVerdict {
    /// Live, with both powers, for this account. `several`: it reached more than one and
    /// this one was picked (row one then names it).
    Valid { account: Account, several: bool },
    WrongPowers,
    Revoked,
}

/// Cloudflare's two dashboard gates; `true` = still blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gates {
    pub email: bool,
    pub card: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerFact {
    Absent,
    Ours,
    KeyGone,
    Foreign,
}

/// The probe pass, in the plan's order: token verify → gates → bucket → Worker →
/// subdomain. `Unknown` = not answered since this process (or this token) began.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sweep {
    /// The last paste and its verdict. Clears when the token is saved.
    pub paste: Option<Paste>,
    pub token: Fact<TokenVerdict>,
    pub gates: Fact<Gates>,
    /// The account's workers.dev name; `None` = it has none yet (the name row).
    pub subdomain: Fact<Option<String>>,
    /// The survey found a Worker named `v` that isn't shroom's (N42) — seen, never
    /// inferred from the label it picked.
    pub foreign_v: Fact<bool>,
    /// The label the survey picked — `v`, else the first fallback that's free or already
    /// ours (B24, B26); `None` = every label is someone else's.
    pub label: Fact<Option<String>>,
    pub bucket: Fact<bool>,
    pub worker: Fact<WorkerFact>,
    /// A space is already there (its marker found): `Some(videos)` — `space-found`.
    pub found: Fact<Option<u32>>,
    pub self_check: Fact<bool>,
    /// A name a claim found taken; `name-taken` while the field still holds it.
    pub taken: Option<String>,
    /// When the last full probe pass answered. A pass that stops early (a dead token, a
    /// gate still up) is still a whole answer: the plan re-probes on its next entry, the
    /// gate on its poll — never in a loop.
    pub pass: Option<Mono>,
}

impl Sweep {
    /// The label every link starts with, as the survey picked it (B24, B26): `v`, or a
    /// fallback beside someone else's `v` (N42). `None` until the survey answered, or
    /// when every label is taken.
    pub fn label(&self) -> Option<&str> {
        self.label.get().and_then(|l| l.as_deref())
    }

    pub fn account(&self) -> Option<&Account> {
        match self.token.get() {
            Some(TokenVerdict::Valid { account, .. }) => Some(account),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Space {
    /// `Some` means provisioned (written only after the self-check or a join).
    pub identity: Option<SpaceIdentity>,
    pub token: bool,
    pub device_key: bool,
    /// An auth- or space-class failure on a publish (or a re-probe) says the space can't
    /// take videos right now — `space-degraded`'s cause. Cleared by an upload going through,
    /// or by a whole re-probe that finds every row of the space sound.
    pub broken: Fact<bool>,
    /// When this process last heard the token Mac's page refresh answer (§6.10: the page
    /// follows the app — at launch, so after every update). `None`: not yet this process.
    pub page_checked: Option<Mono>,
    /// Whether the space's page names its videos (N66: it carries Cloudflare's AI), as the
    /// token Mac's page check or its own switch last left it. `None`: not known yet.
    pub ai: Option<bool>,
    /// The AI banner's Turn on (N66): re-wiring the page until it answers or the try fails;
    /// then, once through, `config` says so once — behind us when the popover that said it
    /// closes.
    pub ai_turn: Option<AiTurn>,
}

impl Space {
    /// The Mac that set the space up and keeps its token — the repair tooling's home (N42).
    pub fn token_holder(&self) -> bool {
        self.token && self.identity.as_ref().is_some_and(|i| i.role == Role::TokenHolder)
    }

    pub fn role(&self) -> Option<Role> {
        self.identity.as_ref().map(|i| i.role)
    }

    /// Every Mac of the owner is owner-class (law 14, N33): the token's Mac and the others.
    pub fn owner_class(&self) -> bool {
        matches!(self.role(), Some(Role::TokenHolder | Role::OwnerClass))
    }
}

/// A take directory under `~/.shroom/takes/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TakeDir(pub String);

/// A recording id (B14): the bucket prefix from capture start; a link only once published.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RecId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "id")]
pub enum SourceId {
    Display(u32),
    Camera,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Quality {
    #[default]
    Normal,
    #[serde(rename = "2k")]
    TwoK,
    #[serde(rename = "4k")]
    FourK,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Recording,
    Pausing(Cause),
    Paused(Cause),
    Resuming,
    Finalizing(Why),
}

/// Why a take paused. Only `User` is the person's own click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    User,
    DisplayLost,
    MicLost,
    CameraLost,
    Asleep,
    Locked,
    MicRevoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Stop,
    GetLink,
    DiskFull,
    ScreenRevoked,
    Discard,
    CaptureDied,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenTake {
    pub dir: TakeDir,
    pub rec: RecId,
    /// When the take opened (law 4's reference instant).
    pub opened: Mono,
    pub source: SourceId,
    pub phase: Phase,
    /// When the phase began (recorder deadlines).
    pub since: Mono,
    pub length_ms: u64,
    pub markers: u32,
    /// The microphone this take records from, by uid (`None` = silent): the one chosen when
    /// capture began, then whichever Resume used (N39's `mic-lost` fallback).
    pub mic: Option<String>,
    /// Camera as the source: the camera recorded, by uid — `camera-lost` is about this one
    /// leaving, never about some other camera (N39).
    pub camera: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedCause {
    /// A crash or forced shutdown mid-take, or capture dying.
    Interrupted,
    DiskFull,
    ScreenOff,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedTake {
    pub dir: TakeDir,
    pub rec: RecId,
    pub length_ms: u64,
    pub cause: SavedCause,
    pub created: Wall,
    /// Copied to ~/Movies ("Saved to Movies ✓").
    pub movies: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    Offline,
    Space,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Queued {
    pub dir: TakeDir,
    pub rec: RecId,
    pub title: String,
    pub stopped: Wall,
    pub done: u32,
    pub total: u32,
    pub blocked: Option<Blocked>,
    /// Its length, known here since the stop (`finalized.json`) — the Library's “date ·
    /// length” before the space's record has it, and N25's question on a removed row.
    pub length_ms: Option<u64>,
}

impl Queued {
    /// It can't go anywhere but Movies: the space refused it (N36's stranded takes).
    pub fn stranded(&self) -> bool {
        self.blocked == Some(Blocked::Space)
    }
}

/// One record of the space, as the Library's lane reports it (`shroom-upload::library::Row`).
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryRow {
    pub rec: RecId,
    pub title: String,
    pub created: Wall,
    pub length_ms: Option<u64>,
    /// Every byte is up (`state: live`); `false` = still uploading, from whichever Mac.
    pub live: bool,
    /// The member who made it (N5: members see their own) — from the space's ledger, never
    /// the record's member-writable `owner` (WORKER-INTERFACE §4.10); `None` until known.
    pub owner: Option<String>,
    /// The record names a poster (`media.poster`): the row shows its thumbnail.
    pub poster: bool,
}

/// The Library's writes under way — facts about what was asked and not yet answered, so
/// a row reads its new title (or is gone) while the write runs, and comes back if it fails.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibraryOps {
    /// When the last refresh from the bucket answered (law 1: every entry re-probes).
    pub refreshed: Option<Mono>,
    /// Deletes confirmed and under way: the tombstone first, then the bytes (§5.5).
    pub deleting: BTreeSet<RecId>,
    /// Renames under way: the new title.
    pub renaming: BTreeMap<RecId, String>,
    /// Writes that landed, and when: a refresh that began before one landed lists the space
    /// as it was, so the landed write still reads over it (a deleted row stays gone, a new
    /// title stays new) until a refresh that began after it answers.
    pub landed: BTreeMap<RecId, (Landed, Mono)>,
    /// When the token Mac's cleanup sweep last answered (DECISIONS B91): bytes under a
    /// tombstone, bytes no Mac can still finish, and spent or expired page keys reaped.
    pub cleaned: Option<Mono>,
}

/// A Library write the space confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landed {
    Deleted,
    Renamed(String),
}

// ── Region 6: members (N28–N34) ──
//
// What the space's Worker said (WORKER-INTERFACE §4, typed by `shroom-cloud::member`), as
// the core keeps it: the invite this Mac holds and what peek and join answered, this Mac's
// standing in its own space, and the roster the Members face reads (DECISIONS B39, B40).

/// Region 6's facts: everything here is something a lane reported, or a file on disk.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Members {
    /// The join link this Mac holds — `~/.shroom/invite` (DEVIATIONS #2) — until it's
    /// behind us (a 200 from join, Stay, a dead invite's face closed).
    pub invite: Option<Invite>,
    /// This Mac's own space about this Mac: `/me`, a mint, the ping (N34, §6.7).
    pub standing: Standing,
    /// The Members face's rows (`GET /members`) — owner-class Macs; never on disk.
    pub roster: Fact<Roster>,
    /// Takes that can't reach the space, copied to ~/Movies (“Saved to Movies ✓”).
    pub movies: BTreeSet<TakeDir>,
    /// Save to Movies asked and not yet answered: no exit deletes these (law 2) — the
    /// exits wait for the copy to land (DECISIONS B54).
    pub saving: BTreeSet<TakeDir>,
    /// Save to Movies answered that it couldn't: its button says so and asks again.
    pub save_failed: BTreeSet<TakeDir>,
    /// A switch's takes waiting on their copy to Movies: deleted once it lands, set aside
    /// (kept, never sent) if it can't (DECISIONS B54, DEVIATIONS #27).
    pub toss: BTreeMap<TakeDir, Queued>,
    /// The takes the switch's face named as never reaching the old space when Join was
    /// clicked there (its line, and Movies first): the only ones its 200 may delete (law 2,
    /// N36's rule for the switch — DEVIATIONS #27).
    pub left_behind: BTreeSet<TakeDir>,
    /// Published takes already holding an upload credential, and until when (each take's
    /// `cred.json`, from the takes scan): a space that stops minting for this Mac still
    /// lets these finish (N32: “a video already on its way can still finish”).
    pub minted: BTreeMap<TakeDir, Wall>,
}

/// A join link this Mac holds: its stash handle only (law 13), never its code.
#[derive(Debug, Clone, PartialEq)]
pub struct Invite {
    pub link: SecretId,
    /// The link's space, as it arrived (a paste learns it from peek).
    pub host: Option<String>,
    /// The person opened it (the page's Open shroom, the installer's launch, a paste): its
    /// window opens as soon as there's something to show — never over a take (law 4). A
    /// remembered one (Later, or the file found at a quiet launch) waits for a click.
    pub wants_window: bool,
    /// It came through a paste door — never the page's Open shroom, never the installer's
    /// launch. While it waits for its window, the Space tab's door says a paste was taken
    /// (D:B104); a link that came another way says nothing there.
    pub pasted: bool,
    /// peek's answer, and when it came (every entry re-peeks, law 1).
    pub peek: Fact<crate::event::PeekAnswer>,
    /// Join was clicked: `join.json` is written and the call runs — and replays the
    /// identical body after a transient answer — until it answers for good.
    pub joining: bool,
    /// join's answer for good, short of the 200 (§4.5).
    pub refused: Option<JoinRefusal>,
    /// The invite's space has a page older than this app needs (ping `iface < IFACE_MIN`,
    /// U3): peek's `current: false`, or a join whose re-ping said so. Join rests greyed on
    /// the needs-update line until a peek finds the page caught up (WORKER-INTERFACE open
    /// question 15, built as recommended — DECISIONS B41).
    pub behind: bool,
}

/// A join that answered for good without a 200 (§4.5's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinRefusal {
    /// 404 `unknown`, 410 `expired` / `cancelled`.
    Expired,
    /// 409 `used`.
    Used,
    /// 409 `already-here`.
    AlreadyHere,
    /// The answer isn't shroom's, nor is the re-ping.
    SpaceMissing,
}

/// This Mac's standing in its own space (N34; §6.7's table). `Fine` clears every face.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Standing {
    /// When the space last answered about this Mac (any answer).
    pub at: Option<Mono>,
    /// When it last answered that all is well (`/me` 200): an invite for this same space
    /// finds this Mac already here only on an answer newer than its peek (§6.2 step 4).
    pub fine: Option<Mono>,
    /// 401, or 403 `removed`: this identity is dead here. Never debounced.
    pub revoked: bool,
    /// Space-side answers since the last `Fine` (C7's debounce).
    pub strike: Option<Strike>,
    /// The page lacks a route this app calls (ping `iface < IFACE_MIN`).
    pub behind: bool,
    /// Owner-class Macs: whether this Mac is its space's only token-holder (U9). `None`
    /// until the space said — and then the line is said (it's true for any token Mac).
    pub only_holder: Option<bool>,
    /// Owner-class Macs: the space's “made with shroom” switch (U3).
    pub mark: Option<bool>,
    /// This Mac told the space its uploads are refused (`POST /health`, §4.18) and hasn't
    /// yet said they move again: the `ok` report is owed on the next upload that lands.
    pub reported: bool,
}

/// Answers of one space-side kind since the last `Fine`: a face only once two of them are
/// at least [`STRIKE_GAP_MS`] apart (C7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strike {
    /// The page isn't shroom's (or the name is gone while online); else the space's own
    /// side refused (503 `space`, a fresh credential refused on its first PUT).
    pub gone: bool,
    pub first: Mono,
    pub last: Mono,
}

/// C7: a space-side face needs two answers at least this far apart (WORKER-INTERFACE §4.8).
pub const STRIKE_GAP_MS: u64 = 30_000;

impl Strike {
    pub fn confirmed(&self) -> bool {
        self.last.since(self.first) >= STRIKE_GAP_MS
    }
}

/// A Members row a removal is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    Member(String),
    Invite(String),
    Mac(String),
}

/// What a join link is for (peek's `kind`, §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InviteKind {
    /// One person, seven days (N29).
    Member,
    /// One of the owner's own Macs, one hour (N33): owner-class.
    OwnMac,
}

/// The Members face's rows (`GET /members`, §4.11) — owner-class Macs only; never on disk.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Roster {
    /// The owner's Macs — member #1's devices, this one among them.
    pub macs: Vec<RosterMac>,
    pub members: Vec<RosterMember>,
    /// Pending invites, own-Mac links included (the face skips those, §6.4).
    pub invites: Vec<PendingInvite>,
    /// Every member's label ever, by member id: the Library's *Everyone* and its delete
    /// sheet (“This is Mher's video.”).
    pub labels: BTreeMap<String, String>,
}

/// One of the owner's Macs (N33): “You · MacBook Air — joined Sep 2”.
#[derive(Debug, Clone, PartialEq)]
pub struct RosterMac {
    pub id: String,
    /// The Mac's sharing name, as it joined.
    pub name: String,
    pub joined: Wall,
    /// It keeps the Cloudflare token: “keeps your Cloudflare token”, and no Remove….
    pub token: bool,
    /// The Mac reading the roster.
    pub this_mac: bool,
}

/// A joined member: “Mher — joined Aug 3”.
#[derive(Debug, Clone, PartialEq)]
pub struct RosterMember {
    pub id: String,
    /// The admin-private label typed at Invite, forever (N28).
    pub label: String,
    pub joined: Wall,
}

/// A pending invite: “Ani — invited just now”, then “invited Aug 21 · 5 days left”, then
/// expired with [ New link ] (a row face, N18).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingInvite {
    pub id: String,
    /// `None` for an own-Mac link.
    pub label: Option<String>,
    pub created: Wall,
    pub expires: Wall,
    pub own_mac: bool,
}

/// The Library's *Mine · Everyone* (N33, N35): owner-class Macs, once someone else has a
/// video here. Mine is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryScope {
    #[default]
    Mine,
    Everyone,
}

/// What the background check offered (`shroom-update::Offer`, newest only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOffer {
    /// A newer version this Mac can run.
    Ready { version: String },
    /// A newer version that needs a newer macOS than this Mac's.
    NeedsMacos { version: String, macos: u32 },
}

/// Updates (U7, N37): notify, then apply — only on the click, never mid-take.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Updates {
    /// The last check's answer; `Known(None)` = nothing newer.
    pub offer: Fact<Option<UpdateOffer>>,
    /// When the last check answered (the next is due a few hours later).
    pub checked: Option<Mono>,
    /// The wall clock at the last check's answer or failure. Opening the popover checks
    /// again once this is a while ago (B154): `checked` is awake time, which a sleeping
    /// Mac doesn't count.
    pub tried: Option<Wall>,
    /// The version downloaded and verified, ready to swap in.
    pub staged: Option<String>,
    /// Update was clicked: quit → swap → relaunch is under way.
    pub applying: bool,
    /// The last swap failed (an `--update-failed` relaunch, or the app never quit): one
    /// status line on the next `config` open, then it's behind us.
    pub failed: bool,
}

/// Every effect the core starts is an op with a deadline; it ends in a result, a
/// `Failed`, or the deadline passing (nothing waits forever).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op {
    RecStart,
    RecPause,
    RecResume,
    RecFinalize,
    Prime,
    Reach,
    OpenPage,
    Upload,
    Validate,
    SaveToken,
    Sweep,
    Claim,
    Wire,
    /// Quitting and reopening for the screen grant (`relaunching`).
    Relaunch,
    /// The Library's bucket refresh.
    LibRefresh,
    /// The Library's remembered list (`library.json`), read on entry whatever the network.
    LibCache,
    Rename,
    Delete,
    UpdateCheck,
    UpdateDownload,
    /// Update clicked: the app should be gone (quit → swap) before this deadline.
    UpdateApply,
    // ── Region 6 (WORKER-INTERFACE §4) ──
    /// An arrived link's ping and peek (§4.2, §4.4).
    Peek,
    /// The join call (§4.5), replayed identically from `join.json`.
    Join,
    /// This Mac's own space re-probed: the ping and `/me` (§4.2, §4.7).
    Standing,
    /// The Members face's rows (`GET /members`, §4.11).
    Roster,
    /// A new link: an invite, an own-Mac link, or [ New link ] (§4.12).
    Invite,
    /// A removal or a cancel (§4.13–§4.15).
    Remove,
    /// The mark switch (§4.16).
    Mark,
    /// The token Mac's page refresh: ping, and put this app's page up over an older one
    /// (§6.10, DECISIONS B50).
    Refresh,
    /// The token Mac's cleanup sweep over its own S3 pair (DECISIONS B91).
    Cleanup,
    /// Naming on or off (N66): the space's page put up again with or without the AI.
    Ai,
}

impl Op {
    /// Deadlines (DEVIATIONS #8): recorder steps 5 s; the stop's reach probe 600 ms;
    /// cloud calls 60 s; wiring 3 min. Updates (DECISIONS B34): the check 10 s, the
    /// download 30 min, the quit before the swap 30 s.
    pub fn deadline_ms(self) -> u64 {
        match self {
            Op::RecStart | Op::RecPause | Op::RecResume | Op::RecFinalize | Op::Prime => 5_000,
            Op::Reach => 600,
            Op::OpenPage | Op::Upload | Op::Validate | Op::SaveToken | Op::Sweep | Op::Claim => 60_000,
            Op::Wire | Op::Refresh | Op::Ai => 180_000,
            Op::Relaunch => 10_000,
            Op::LibRefresh | Op::Rename | Op::Delete => 60_000,
            Op::LibCache => 5_000,
            Op::UpdateCheck => 10_000,
            Op::UpdateDownload => 1_800_000,
            Op::UpdateApply => 30_000,
            Op::Peek | Op::Join | Op::Standing | Op::Roster | Op::Invite | Op::Remove | Op::Mark => 60_000,
            // A listing of every recording's bytes and each page key: slow on a big space.
            Op::Cleanup => 600_000,
        }
    }

    /// The wizard's network-bound calls (a transport failure of one is `wizard-offline`).
    pub const WIZARD: [Op; 5] = [Op::Validate, Op::SaveToken, Op::Sweep, Op::Claim, Op::Wire];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClass {
    Transport,
    Refused,
    Timeout,
    Panicked,
    Unclassified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub at: Mono,
    pub class: FailClass,
    pub tries: u8,
}

impl Failure {
    /// Backoff before the next try: 5 s doubling, capped at 2 min (DEVIATIONS #8).
    pub fn retry_at(&self) -> Mono {
        let ms = (5_000u64 << self.tries.min(5)).min(120_000);
        self.at.plus(ms)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Inflight {
    pub ops: BTreeMap<Op, Mono>,
    pub failures: BTreeMap<Op, Failure>,
}

// ── what the person asked for ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    pub source: Option<SourceId>,
    /// On from the first start (N43, Bagrat: “yes on by default”) — the camera is asked in
    /// the chapter; a camera denied there turns it off.
    pub show_yourself: bool,
    /// The camera picked in the camera picker, by device uid — the bubble's, and Camera only's;
    /// `None`, or one that isn't here, = the one shroom picks by itself (N39, DEVIATIONS #43).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    /// The microphone picked, by device uid; `None` = the safe default (N39).
    pub mic: Option<String>,
    /// No mic, picked: the take records silently (DEVIATIONS #43).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mic_off: bool,
    pub quality: Quality,
    /// Where the person put the bubble: its top-left corner's offset, in points, from the
    /// top-left of the display it sits on — so it follows the chosen display (N22, N39).
    pub bubble_at: Option<(i32, i32)>,
    /// "Set up my space" was pressed (DEVIATIONS #1).
    pub setup_chosen: bool,
    /// The Settings tab last used; `None` = never opened → General (N36).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_tab: Option<SettingsTab>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            source: None,
            show_yourself: true,
            camera: None,
            mic: None,
            mic_off: false,
            quality: Quality::default(),
            bubble_at: None,
            setup_chosen: false,
            settings_tab: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettingsTab {
    #[default]
    General,
    Space,
    /// Owner-class Macs only — Region 6's tab.
    Members,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Once {
    /// The version whose “Updated to … ✓” was shown (or that this Mac first ran, or that a
    /// launch other than an update relaunch found running): only an `--updated` relaunch
    /// leaves a newer running version unacknowledged, and that says once that it landed.
    pub seen_version: Option<String>,
    /// The popover's AI banner on this Mac (N66).
    #[serde(default)]
    pub ai_banner: AiBanner,
}

/// How the popover's AI banner was answered on this Mac (N66; Bagrat, 2026-10-18: off by
/// default, but visible): Not now once hides it until five more recordings, twice for good;
/// AI on, from the banner or anywhere else, answers it for good.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiBanner {
    #[serde(default)]
    pub not_now: u8,
    /// Recordings that made a link on this Mac since the first Not now.
    #[serde(default)]
    pub takes: u32,
    #[serde(default)]
    pub done: bool,
}

/// Recordings after a first Not now before the AI banner comes back, once (N66).
pub const AI_BANNER_AGAIN: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowKind {
    Onboarding,
    Library,
    Settings,
    /// Send feedback, from the popover's header (N60).
    Feedback,
}

/// An open surface: when it opened (law 4 visibility), when it was last entered (probe
/// freshness), and whether the person asked for it (an unasked one never takes the keyboard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Door {
    pub opened: Mono,
    pub entered: Mono,
    pub asked: bool,
}

impl Door {
    pub fn asked(at: Mono) -> Door {
        Door { opened: at, entered: at, asked: true }
    }
    pub fn unasked(at: Mono) -> Door {
        Door { opened: at, entered: at, asked: false }
    }
}

/// A question on screen. Clears on Back/Esc, on its answer, when its door closes, or when
/// its subject vanishes.
#[derive(Debug, Clone, PartialEq)]
pub enum Confirm {
    Discard,
    /// Restart's question (the canvas's, DEVIATIONS #44): the count only once it's answered.
    Restart,
    DiscardSaved(TakeDir),
    Quit,
    /// The Library's delete sheet, for these published videos.
    Delete(Vec<RecId>),
    /// The Library's Discard… on a take that never reached anyone (a recovered take, or a
    /// row another Mac's delete stopped): N25's question, as a sheet.
    DiscardRow(TakeDir),
    /// Settings · Space's Disconnect sheet — or a member's Leave sheet.
    Exit,
    /// Settings · General's Uninstall sheet (B135); `going` once answered, until shroom quits.
    Uninstall { going: bool },
    /// member-remove-confirm: a member, a pending invite, or another of the owner's Macs.
    Remove(Removal),
    /// Settings · General's Report a problem sheet (B180); `sending` once Send was clicked,
    /// until shroom.am answers.
    Report { sending: bool },
    /// The AI banner's Turn on… (N66): what it does and costs, before it costs anything.
    Ai,
}

/// The countdown: armed by Start, fired at its deadline, cancelled by any click or Esc.
#[derive(Debug, Clone, PartialEq)]
pub struct Arming {
    pub started: Mono,
    pub deadline: Mono,
    pub source: SourceId,
    /// Restart: the old take is discarded only once the new capture begins (N39).
    pub restart_of: Option<TakeDir>,
    pub fired: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopOutcome {
    /// Stopped while offline: the link sits in the popover; the page opens on reconnect.
    Offline(RecId),
    /// A stop the person didn't make (disk full, screen permission lost).
    AutoStopped(TakeDir),
    /// A stop the space can't take (N34): the link waits, the take is safe here.
    Stranded(TakeDir),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageOpen {
    pub rec: RecId,
    /// Its take, here: a stop whose page turns out gone becomes `stopped-stranded` about it.
    pub dir: TakeDir,
    pub at: Wall,
    /// The stop's instant: the page opens only on a reachability answer newer than this.
    pub stopped: Mono,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyTarget {
    Link(RecId),
    /// A born join link (Region 6), by its stash handle.
    Invite(SecretId),
    /// Settings · General's Copy diagnostics (B180).
    Diagnostics,
}

/// The account fork (N14's box 1, N19): the plan's one self-reported tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fork {
    /// Nothing said yet — the clean question.
    #[default]
    Virgin,
    /// "Not yet — take me there": Cloudflare's signup is open in the browser.
    SigningUp,
    /// "I have one" / "I signed up" (or a token pasted, which proves it).
    Affirmed,
}

/// Echoes of non-secret field contents in the onboarding window. Secret text never lands
/// here — a paste arrives as a `SecretId`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fields {
    /// The account came by “I signed up”: the token step reminds that Cloudflare makes no
    /// token before the email is confirmed (N45).
    pub signed_up: bool,
    /// Add credit card was clicked in this window: the card gate's quiet line says the
    /// billing page is open (N45).
    pub billing_opened: bool,
    /// The link name as typed, already filtered (lowercase, charset, length).
    pub name: String,
    /// How many edits of the name were taken in — the Scene carries it, so the window can
    /// tell the answer to an older edit from the answer to its newest (DECISIONS B107).
    pub name_edits: u32,
    /// When a character outside the set was typed (the quiet line fades after a moment).
    pub name_hint: Option<Mono>,
    /// The account picked in `token-pick-account`, by id.
    pub account: Option<String>,
    /// The welcome's login box was unchecked (it starts checked, U4).
    pub login_off: bool,
    /// The Library window's own fields — kept apart, they clear when it closes.
    pub library: LibraryFields,
    /// The Settings window's own fields — kept apart, they clear when it closes.
    pub settings: SettingsFields,
    /// The Send feedback window's own fields (N60) — they clear when it closes.
    pub feedback: FeedbackFields,
}

/// What the Send feedback window holds (N60): the words and the reply-to as typed, the log
/// box, the report the shell gathered, and where Send stands. All clear when it closes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeedbackFields {
    pub words: String,
    pub words_edits: u32,
    pub reply_to: String,
    pub reply_edits: u32,
    /// The log box was unticked: the report carries the version alone (it opens ticked).
    pub without_log: bool,
    pub gathered: Option<crate::report::Diagnostics>,
    /// Send was clicked and shroom.am hasn't answered.
    pub sending: bool,
    /// The last Send didn't go through.
    pub failed: bool,
    /// shroom.am took it: the window says so and offers Done.
    pub sent: bool,
}

/// What the Settings window holds while it's open: the Members composer, the link just
/// born (shown once — only its hash exists afterwards, §6.1.7), and the token row asked
/// for on a Mac without the token. All clear when the window closes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SettingsFields {
    /// “Who's this for?” as typed — the admin-private label (N28).
    pub invite_name: String,
    /// How many edits of it were taken in (B107, as `Fields::name_edits`).
    pub invite_edits: u32,
    /// The name invites say they're from, as typed while it's being changed (B158).
    pub from: Option<String>,
    pub from_edits: u32,
    /// The link Invite, [ New link ] or Add another Mac just made.
    pub born: Option<Born>,
    /// “Add a token here” / “Reconnect here” was clicked: the wizard's token row.
    pub add_token: bool,
    /// space-storage-gone's “Set it up fresh” was clicked — the person's consent to new
    /// storage: wiring runs, and retries, under the calm line until it lands (#10).
    pub fresh: bool,
    /// The Report a problem sheet's own fields (B180): they clear with the sheet.
    pub report: ReportFields,
    /// A report went through: General says so until the window closes (B180).
    pub sent: bool,
}

/// What the Report a problem sheet holds (B180): the words and the reply-to as typed, the
/// report the shell gathered, and whether the last Send didn't go through.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportFields {
    pub words: String,
    pub words_edits: u32,
    pub reply_to: String,
    pub reply_edits: u32,
    pub gathered: Option<crate::report::Diagnostics>,
    pub failed: bool,
}

/// A link just made: its row (by invite id) and its stash handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Born {
    pub invite: String,
    pub link: SecretId,
    pub own_mac: bool,
}

/// The Library's search, selection and the title being typed (N35). Clear when the Library
/// window closes; a selected row that vanishes simply stops counting (derived).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibraryFields {
    /// Local and instant, over the titles this Mac knows.
    pub search: String,
    /// How many edits of the search were taken in (B107, as `Fields::name_edits`).
    pub search_edits: u32,
    /// ⌘/⇧-click several rows, then Delete… once.
    pub selected: Vec<RecId>,
    /// A title being renamed in place, and what's typed so far. Esc drops it; Return or
    /// click-away saves it (an emptied title brings the previous one back).
    pub renaming: Option<(RecId, String)>,
    /// How many edits of a title being renamed were taken in since the window opened
    /// (B107, as `Fields::name_edits`). Starting a rename isn't one.
    pub title_edits: u32,
    /// *Mine · Everyone* (owner-class Macs, once someone else records here). Mine first.
    pub scope: LibraryScope,
}

/// In memory only, never persisted (law 1). At most 12 fields (a fitness test counts).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Session {
    /// Clears on dismiss, Esc, Start, Stop, quit, and when a footer door opens a window.
    pub popover: Option<Door>,
    /// Clears on ✕ / Close, on a morph out of the window (Record now), and on Disconnect.
    pub windows: BTreeMap<WindowKind, Door>,
    /// Clears whenever the onboarding window closes (Later, ✕, Close, Record now) and
    /// with the process: Set up my space always lands on the clean question.
    pub fork: Fork,
    /// "Allow recording" (or Settings' “Turn it back on”) was clicked: the chapter runs
    /// instead of resting. Clears when the warm-up primes, or when the window it runs in
    /// (onboarding or Settings) closes.
    pub chapter: bool,
    /// The onboarding fields clear when the onboarding window closes; the Library's when
    /// the Library window closes.
    pub fields: Fields,
    /// Clears on Back/Esc, on its answer, when its door (popover or window) closes, or
    /// when its subject vanishes.
    pub confirm: Option<Confirm>,
    pub arming: Option<Arming>,
    /// Clears when the popover closes.
    pub last_stop: Option<StopOutcome>,
    /// The offline stop's page: newest only, within the hour (N40). Clears when opened,
    /// past the hour, or on quit.
    pub page_open: Option<PageOpen>,
    /// "Copied" for 1.5 s (the popover's link or a Library row's). Clears on the Tick at
    /// its end.
    pub copied: Option<(CopyTarget, Mono)>,
    /// A take capture let go of mid-take, until the lane says whether anything of it was
    /// kept (DEVIATIONS #53). Said at `config` once nothing was; clears when the popover
    /// that said it closes, and when the lane says something was kept.
    pub take_lost: Option<TakeLost>,
    /// The popover an update relaunch opened by itself (its door's opening instant): news
    /// that a click away puts away. A popover opened since is another door.
    pub news: Option<Mono>,
}

/// The AI banner's Turn on, after it's pressed (N66): turning on — re-wiring the page, or
/// waiting to try again after a try that didn't go through — then on, said once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTurn {
    Turning,
    On,
}

impl Failure {
    /// When the AI banner's Turn on that didn't go through is tried again, quietly (N66, law
    /// 12): ten minutes, doubling, at most an hour — until it goes through, or Not now.
    pub fn ai_retry_at(&self) -> Mono {
        self.at.plus((600_000u64 << self.tries.min(3)).min(3_600_000))
    }
}

/// The display a click brings to `config` (N56).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DisplayPick {
    /// The display the mushroom was clicked on to open `config`: the source, unless a display
    /// was picked by hand or Camera only is. Set by each click that opens `config`.
    pub clicked: Option<u32>,
    /// A display was picked by hand in Record: it holds over the click's display until the
    /// take ends.
    pub by_hand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TakeLost {
    /// Capture died; the lane hasn't said what it kept.
    Pending,
    /// Nothing was kept: `config` says so, in the camera's words or the screen's.
    Said { camera: bool },
}

impl World {
    /// When the current take (or its countdown) opened — law 4's reference instant
    /// (DEVIATIONS #3: the countdown counts).
    pub fn open_since(&self) -> Option<Mono> {
        if let Some(a) = &self.session.arming {
            return Some(a.started);
        }
        self.take.as_ref().map(|t| t.opened)
    }

    pub fn take_open(&self) -> bool {
        self.open_since().is_some()
    }

    /// Row six — this Mac's permissions chapter — is done.
    pub fn row_six_done(&self) -> bool {
        self.mac.primed.is_some()
    }

    pub fn provisioned(&self) -> bool {
        self.space.identity.is_some()
    }

    /// Whether a window may show: mid-take, only one opened before the take (law 4).
    pub fn window_visible(&self, kind: WindowKind) -> bool {
        self.session.windows.get(&kind).is_some_and(|d| self.open_since().is_none_or(|t| d.opened < t))
    }

    /// `config` exists: a space, and this Mac's chapter done (the update banner's and the
    /// Frost dot's precondition — never before a space exists, N37).
    pub fn config_reachable(&self) -> bool {
        self.provisioned() && self.row_six_done()
    }

    /// The window in front: the one most recently entered.
    pub fn front_window(&self) -> Option<WindowKind> {
        self.session.windows.iter().max_by_key(|(_, d)| d.entered).map(|(k, _)| *k)
    }

    pub fn online(&self) -> Option<bool> {
        self.net.online.get().copied()
    }

    /// A blank World at boot, before anything was probed.
    pub fn blank() -> World {
        World {
            now: Mono(0),
            wall: Wall(0),
            launch: Launch::default(),
            mac: Mac::default(),
            net: Net::default(),
            space: Space::default(),
            sweep: Sweep::default(),
            take: None,
            saved: Vec::new(),
            queue: Vec::new(),
            removed: Vec::new(),
            library: Fact::Unknown,
            library_ops: LibraryOps::default(),
            update: Updates::default(),
            members: Members::default(),
            inflight: Inflight::default(),
            popover_closed: None,
            prefs: Prefs::default(),
            once: Once::default(),
            session: Session::default(),
            display_pick: DisplayPick::default(),
        }
    }
}
