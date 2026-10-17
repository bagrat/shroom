//! Everything the core can ask for — data, executed by the runner's lanes, whose results
//! come back as events. Effects never cross to a webview.

use crate::world::{Op, Prefs, Quality, RecId, RelaunchWhy, Restore, SecretId, SourceId, SpaceIdentity, TakeDir, Why};
use crate::time::Wall;

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Recorder(RecCmd),
    /// Writes the take's `publish.json` — law 3's single gate: only Stop or Get link make
    /// a link.
    Publish { dir: TakeDir, rec: RecId },
    Uploader(UpCmd),
    Cloud(CloudCmd),
    Shell(ShellCmd),
    Persist(PersistOp),
    Takes(TakeCmd),
    Library(LibCmd),
    Updater(UpdCmd),
    /// The space's Worker, Region 6 (WORKER-INTERFACE §4).
    Member(MemberCmd),
}

impl Effect {
    /// The op this effect starts, if it's one the core waits on.
    pub fn op(&self) -> Option<Op> {
        match self {
            Effect::Recorder(RecCmd::Start { .. }) => Some(Op::RecStart),
            Effect::Recorder(RecCmd::Pause) => Some(Op::RecPause),
            Effect::Recorder(RecCmd::Resume { .. }) => Some(Op::RecResume),
            Effect::Recorder(RecCmd::Finalize { .. } | RecCmd::Confirm(_)) => Some(Op::RecFinalize),
            Effect::Recorder(RecCmd::Prime) => Some(Op::Prime),
            Effect::Cloud(CloudCmd::Reach) => Some(Op::Reach),
            Effect::Cloud(CloudCmd::OpenOwnerPage(_)) => Some(Op::OpenPage),
            Effect::Uploader(UpCmd::Kick) => Some(Op::Upload),
            Effect::Cloud(CloudCmd::Validate(_)) => Some(Op::Validate),
            Effect::Cloud(CloudCmd::SaveToken(..)) => Some(Op::SaveToken),
            Effect::Cloud(CloudCmd::Sweep(_)) => Some(Op::Sweep),
            Effect::Cloud(CloudCmd::Claim(_)) => Some(Op::Claim),
            Effect::Cloud(CloudCmd::Wire { .. }) => Some(Op::Wire),
            Effect::Cloud(CloudCmd::Refresh) => Some(Op::Refresh),
            Effect::Shell(ShellCmd::Relaunch(_)) => Some(Op::Relaunch),
            Effect::Library(LibCmd::Refresh) => Some(Op::LibRefresh),
            Effect::Library(LibCmd::Cleanup) => Some(Op::Cleanup),
            Effect::Library(LibCmd::LoadCache) => Some(Op::LibCache),
            Effect::Library(LibCmd::Rename(..)) => Some(Op::Rename),
            Effect::Library(LibCmd::Delete(_)) => Some(Op::Delete),
            Effect::Updater(UpdCmd::Check) => Some(Op::UpdateCheck),
            Effect::Updater(UpdCmd::Download(_)) => Some(Op::UpdateDownload),
            Effect::Updater(UpdCmd::Apply(_)) => Some(Op::UpdateApply),
            Effect::Member(MemberCmd::Peek(_)) => Some(Op::Peek),
            Effect::Member(MemberCmd::Join(_)) => Some(Op::Join),
            Effect::Member(MemberCmd::Probe) => Some(Op::Standing),
            Effect::Member(MemberCmd::Roster) => Some(Op::Roster),
            Effect::Member(MemberCmd::Invite { .. } | MemberCmd::AddMac | MemberCmd::Renew(_)) => Some(Op::Invite),
            Effect::Member(MemberCmd::CancelInvite(_) | MemberCmd::RemoveMember(_) | MemberCmd::RemoveMac(_)) => Some(Op::Remove),
            Effect::Member(MemberCmd::Mark(_)) => Some(Op::Mark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecCmd {
    /// The lane mints the take's dir and recording id and answers `Rec(Started)`. `camera`:
    /// Camera as the source, the camera to record, by uid — the core's pick, never the
    /// lane's (N39: never an iPhone by itself).
    Start { source: SourceId, quality: Quality, mic: Option<String>, camera: Option<String> },
    Pause,
    /// Resume with this microphone — the take's own if it's still here, else the fallback
    /// the paused popover named (N39 `mic-lost`), or none: silence, in the take's own audio
    /// format (DECISIONS B47).
    Resume { mic: Option<String> },
    Marker,
    /// Finalize the open take; `why` is written into the take (a saved take's reason
    /// survives a relaunch).
    Finalize { why: Why },
    /// Get link on a saved take: confirm it's finalized (a no-op for a repaired take) and
    /// answer `Rec(Finalized)` — so it shares Stop's single publish arm (law 3).
    Confirm(TakeDir),
    /// Kill capture, keep the dir — it reads as an interrupted take (never deleted).
    Abandon,
    Discard(TakeDir),
    Prime,
    /// The count began: open this microphone now, so it's already heard at the take's zero
    /// (a Bluetooth headset wakes about two seconds after its mic opens; B173). Nothing it
    /// hears before the take begins is kept.
    Warm { mic: Option<String> },
    /// The count was cancelled: close what `Warm` opened.
    Cool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpCmd {
    Kick,
    /// Discard's clean delete of bytes already uploaded (N23: no tombstone).
    ForgetRemote(RecId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloudCmd {
    /// The stop's reachability probe (capped at 600 ms).
    Reach,
    /// Mint the one-time owner key and open the page (the keyed URL never leaves the lane).
    OpenOwnerPage(RecId),
    /// Verify the pasted token and probe it by use; answers `Validated`.
    Validate(SecretId),
    /// Write the token to `~/.shroom` (mode 600) for this account id; answers `TokenSaved`.
    SaveToken(SecretId, String),
    /// The probe pass with the stored token; answers `Swept`.
    Sweep(SweepPart),
    /// Claim this workers.dev name (only ever on the person's click); answers `Claimed`.
    Claim(String),
    /// The idempotent wiring calls and the self-check, under this label; answers `Wired`.
    Wire { label: String },
    /// The token Mac's page refresh (§6.10, B50): ping the space's page and, only when it
    /// answers older than this app's, put this app's page up (the roster seeded first);
    /// never over a newer one, never any storage. Answers `Refreshed`.
    Refresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepPart {
    /// Token verify → gates → bucket → Worker → subdomain (and the space marker).
    Full,
    /// Only the two gates — the auto-poll under the card row.
    Gates,
}

/// A page shroom may open in the browser — destinations, not URLs (the runner owns those).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicUrl {
    /// Cloudflare's signup.
    Signup,
    /// The pre-filled token page (name + the two permissions).
    TokenPage,
    /// The account's own billing page (card or PayPal — shroom never sees payment data).
    Billing { account_id: String },
    /// What's new: this release's notes.
    ReleaseNotes { version: String },
    /// A new GitHub issue, prefilled with the running version (N8).
    NewIssue { version: String },
    /// A send that didn't go through, by mail instead: a draft to hi@shroom.am (N48) with the
    /// words and the version (B180).
    MailReport { version: String, words: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShellCmd {
    /// Gather the report for the sheet: the Mac, the setup, this Mac's role (in words, from
    /// the core) and the log, cleaned — answered by `ReportNews::Gathered` (B180).
    GatherReport { role: String },
    /// The same report onto the clipboard — only ever from Copy diagnostics' click (law 10).
    CopyDiagnostics { role: String },
    /// Post it to shroom.am/report — only ever from Send's click; answered by
    /// `ReportNews::Sent` or `NotSent`. A dev build never reaches shroom.am.
    SendReport(crate::report::Post),
    /// A public link onto the clipboard — only ever from a Copy click (law 10).
    Copy(RecId),
    /// The video's picture wrapped in its link, and the link as text — only ever from a
    /// Copy thumbnail click (law 10, N46). The title names the picture for screen readers.
    CopyThumb(RecId, String),
    SettingsPane(Pane),
    RequestPermission(Perm),
    Reveal(TakeDir),
    Quit,
    PowerAssertion(bool),
    /// Answer macOS's waiting will-sleep: it may sleep now (DECISIONS B92). Emitted once
    /// per hold, the moment the take is safe or the hold's two seconds are up.
    AllowSleep,
    OpenLibrary,
    OpenSettings,
    /// The Send feedback window, brought forward when it's already open (N60).
    OpenFeedback,
    /// Open a public page in the browser — only ever right after the click that asked.
    OpenUrl(PublicUrl),
    /// Register (or not) shroom to start at login.
    SetLoginItem(bool),
    /// Quit and reopen shroom.
    Relaunch(RelaunchWhy),
    /// A join link onto the clipboard, from the stash — only ever from its Copy link click
    /// (law 10); the code never passes through the core (law 13).
    CopyInvite(SecretId),
    /// Remove shroom from this Mac (B135): the login item off, the permissions reset, then
    /// quit, leaving a helper that removes the app and its folder once it has. Never the space.
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    ScreenRecording,
    Camera,
    Microphone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perm {
    Mic,
    Screen,
    Camera,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PersistOp {
    Prefs(Prefs),
    /// `space.json`: its existence means provisioned (written after the self-check).
    Space(SpaceIdentity),
    /// `primed.json`: row six done on this Mac.
    Primed(Wall),
    /// `restore.json`: `Some` before a relaunch shroom asks for; `None` deletes it.
    Restore(Option<Restore>),
    /// `once.json`.
    Once(crate::world::Once),
    /// Disconnect: the `~/.shroom` secrets and the space's identity go; takes, prefs and
    /// the login item stay (N36).
    Wipe,
}

/// The Library's data path (`shroom-upload::library`, ARCHITECTURE §13).
#[derive(Debug, Clone, PartialEq)]
pub enum LibCmd {
    /// What this Mac remembers (`library.json`), answered at once — no network (N35's
    /// offline face shows it). Answers `Rows { fresh: false }`.
    LoadCache,
    /// List `meta/` with the ETag delta.
    Refresh,
    /// Rewrite the record's title (read-modify-write under `If-Match`, B21).
    Rename(RecId, String),
    /// The tombstone first, then the bytes — for each.
    Delete(Vec<RecId>),
    /// The token Mac's cleanup sweep (DECISIONS B91): bytes under a tombstone, bytes no Mac
    /// can still finish (never one this Mac holds), spent or expired page keys. Answers
    /// `Cleaned`.
    Cleanup,
}

/// The updater (`shroom-update`, B31).
#[derive(Debug, Clone, PartialEq)]
pub enum UpdCmd {
    /// Read the releases' manifest (fail-soft); answers `Update(Checked)`.
    Check,
    /// Download and verify this version in the background; answers `Update(Staged)`.
    Download(String),
    /// Quit, swap this staged version in, and reopen (`--relaunched=update`, or
    /// `--relaunched=update-failed` with the old copy put back).
    Apply(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TakeCmd {
    /// Each take's single-file copy into ~/Movies under its name (built from the take's
    /// own segments when it has none yet), never over another file. Answers
    /// `SavedToMovies` or `SaveFailed` for each.
    SaveToMovies(Vec<MoviesCopy>),
    Delete(TakeDir),
    /// A switch's take whose copy to Movies failed: marked so it's never sent anywhere,
    /// and kept (DEVIATIONS #27).
    SetAside(TakeDir),
    Rescan,
}

/// One take for Movies: its folder and the file's name — its title, or “shroom recording
/// — Aug 21” in this Mac's calendar (N34). The lane makes it a safe file name.
#[derive(Debug, Clone, PartialEq)]
pub struct MoviesCopy {
    pub dir: TakeDir,
    pub name: String,
}

/// The space's Worker (`shroom-cloud::member`, WORKER-INTERFACE §4, §6). Links ride as
/// stash handles; labels are the only free text (admin-private, N28).
///
/// REGION 6 INTERFACE (tests/region6.rs, 2026-10-02): the runner's member lane
/// (`app/src-tauri/src/runtime/member.rs`) answers each one.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberCmd {
    /// An arrived link: ping its host and peek it (§6.2 steps 2–3) — a pasted one is first
    /// normalized and written to `~/.shroom/invite` (DEVIATIONS #2). Answers `Peeked`.
    Peek(SecretId),
    /// Write `join.json` (unless it's already there — the boot replay), then `POST join`
    /// (§4.5). Answers `Joined`. On a 200 the lane writes `device.key` and `space.json`,
    /// deregisters from and wipes the old space on a switch, and deletes `join.json` (§5).
    Join(SecretId),
    /// The invite is behind us: delete `~/.shroom/invite` and drop the link from the stash
    /// (join's 200, Stay, ✕ on the switch, a dead invite's face closed).
    ForgetInvite,
    /// The ping and `/me` with this Mac's device key (§4.2, §4.7). Answers `Standing`.
    Probe,
    /// `GET /members`. Answers `Roster`.
    Roster,
    /// `POST /invites` for this label; the runner makes the code into the stash. Answers
    /// `Invited`.
    Invite { label: String },
    /// *Add another Mac*: an own-Mac link, one hour (N33). Answers `Invited`.
    AddMac,
    /// [ New link ] on an expired invite's row: the same row, a new code. Answers `Invited`.
    Renew(String),
    /// `DELETE /invites/<iid>`. Answers `Removed`.
    CancelInvite(String),
    /// `DELETE /members/<mid>`. Answers `Removed`.
    RemoveMember(String),
    /// `DELETE /devices/<did>` — another of the owner's Macs, never one with the token.
    /// Answers `Removed`.
    RemoveMac(String),
    /// `POST mark` (§4.16). Answers `Marked`.
    Mark(bool),
    /// `POST /health {rec, uploads}` (§4.18, §6.7 step 5): `refused` once C7's debounce
    /// confirms the space refuses this Mac's uploads, `ok` on the next upload that lands
    /// after. Fire and forget: a lost report is repeated by the next one (§4.18). Never
    /// answers.
    Health { rec: crate::world::RecId, refused: bool },
}
