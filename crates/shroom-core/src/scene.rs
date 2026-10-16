//! The Scene: everything every surface shows, derived (never stored). Webviews receive
//! their slice and render it — final copy included; they compose no sentences and decide
//! nothing (ARCHITECTURE §3.5).

use serde::{Deserialize, Serialize};

use crate::event::Surface;
use crate::time::Mono;
use crate::world::{LibraryScope, Quality, RecId, SecretId, SettingsTab, SourceId, TakeDir};

/// Every clickable thing. No secrets, no free text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "do", content = "arg")]
pub enum Action {
    /// Close the popover (Esc, click-away).
    Dismiss,
    Start,
    CancelCountdown,
    Resume,
    Stop,
    /// Opens the Restart question (DEVIATIONS #44).
    Restart,
    /// The Restart question's Restart: the count, Restart's undo (N39).
    ConfirmRestart,
    DropMarker,
    /// Opens the Discard question.
    Discard,
    ConfirmDiscard,
    Back,
    RestartInstead,
    /// On the oldest saved take (the banner shows the oldest first).
    GetLink,
    /// Opens the Discard question for the oldest saved take.
    DiscardSaved,
    ConfirmDiscardSaved,
    SaveToMovies,
    OpenLibrary,
    OpenSettings,
    Quit,
    QuitAnyway,
    CopyLink,
    SetSource(SourceId),
    /// Show yourself's pick: a camera by uid, or `None` — Camera off (DEVIATIONS #43).
    SetCamera(Option<String>),
    /// Show yourself's System default: follows macOS's preferred camera (N44).
    SetCameraDefault,
    /// The microphone's pick: one by uid, or `None` — No mic (DEVIATIONS #43).
    SetMic(Option<String>),
    /// The microphone's System default: follows macOS's default input (N44).
    SetMicDefault,
    SetQuality(Quality),
    /// recording-off's banner: opens System Settings at Screen Recording.
    TurnBackOn,
    OpenCameraSettings,
    /// The denied mic's quiet hint (Region 3: a denied mic never blocks).
    OpenMicSettings,
    // ── onboarding: the welcome ──
    /// "Set up my space" — the window morphs into the plan.
    SetUp,
    Later,
    /// The welcome's "Start shroom when you log in" box.
    SetLoginBox(bool),
    // ── onboarding: the plan ──
    /// "Not yet — take me there": opens Cloudflare's signup.
    NotYet,
    HaveOne,
    /// "Take me there again".
    SignupAgain,
    SignedUp,
    CreateToken,
    /// A radio in `token-pick-account`, by account id.
    PickAccount(String),
    UseAccount,
    AddCard,
    /// "Claim this name" — the only trigger of a claim.
    Claim,
    AllowRecording,
    /// perm-screen's quiet "Reopen shroom".
    ReopenShroom,
    /// perm-screen's quiet door while macOS's alert led (N43): the exact pane.
    OpenScreenSettings,
    // ── onboarding: all-set ──
    /// all-set's Close (≡ ✕).
    CloseWindow,
    InviteSomeone,
    RecordNow,
    // ── the update (config's banner, Settings · General) ──
    /// Quit → swap → relaunch (N37) — never mid-take.
    Update,
    /// The release notes in the browser.
    WhatsNew,
    // ── space-degraded ──
    /// Opens Settings on Space, in the repair mood.
    ReconnectSpace,
    // ── the Library ──
    /// A click on a row: it alone is selected.
    SelectRow(RecId),
    /// ⌘-click: in or out of the selection.
    ToggleRow(RecId),
    /// ⇧-click: the selection runs to this row.
    ExtendRow(RecId),
    /// Copy link (also ⌘C on a selected row): the clipboard, and “Copied” for a moment.
    CopyRow(RecId),
    /// Copy thumbnail, in a live row's menu (N46): the video's picture wrapped in its link.
    CopyThumbRow(RecId),
    /// Open page (also a double-click): the owner-keyed URL (B25).
    OpenPage(RecId),
    /// A click on the title: rename in place.
    StartRename(RecId),
    /// Return in the title field, or a click away: save it.
    CommitRename,
    /// Esc in the title field.
    CancelRename,
    /// Delete… on a row (several selected: all of them).
    DeleteRow(RecId),
    /// The Delete key on the selection.
    DeleteSelected,
    /// The delete sheet's Delete.
    ConfirmDelete,
    /// A recovered take's Get link, from its pinned row.
    GetLinkRow(TakeDir),
    /// Discard… on a take that never reached anyone.
    DiscardRow(TakeDir),
    ConfirmDiscardRow,
    SaveRowToMovies(TakeDir),
    // ── Settings ──
    /// The exit sheet's “Save them to Movies first”: every take that can't reach the space.
    SaveStrandedToMovies,
    SettingsTab(SettingsTab),
    /// General's “Start shroom when you log in” — sets the system's login item.
    SetLoginItem(bool),
    /// The help line's GitHub door.
    ReportIssue,
    /// General's “Copy diagnostics”: the report onto the clipboard (B180, law 10).
    CopyDiagnostics,
    /// General's “Report a problem…” — opens its sheet (B180).
    ReportProblem,
    /// The report sheet's Send (⌘Return).
    SendReport,
    /// A failed send's “email it to hi@shroom.am”.
    MailReport,
    /// “Disconnect this Mac…” — opens its sheet.
    Disconnect,
    ConfirmDisconnect,
    /// General's “Uninstall shroom…” — opens its sheet (B135).
    Uninstall,
    ConfirmUninstall,
    /// space-storage-gone's “Set it up fresh”.
    SetUpFresh,
    /// The wizard's “something unexpected” face: ask every wizard call again now
    /// (DEVIATIONS #25).
    TryAgain,
    // ── Region 6 (REGION 6 INTERFACE, tests/region6.rs, 2026-10-02) ──
    /// member-welcome's [ Join ], and join-switch-confirm's [ Join Bagrat's space ].
    Join,
    /// join-switch-confirm's [ Stay ] (≡ ✕): the invite stays unused, and is behind us.
    Stay,
    /// The Members composer's [ Invite ] (Return in “Who's this for?”).
    Invite,
    /// *Change*, after “Invites will say they're from …”: the name becomes a field (B158).
    ChangeFrom,
    /// *Save* beside that field (Return in it): the name new invites carry (B158).
    SaveFrom,
    /// Esc in that field: the name stays as it was (B158).
    KeepFrom,
    /// A born link's [ Copy link ] — this link only (law 10).
    CopyInvite(SecretId),
    /// *Add another Mac*, beside You (N33).
    AddMac,
    /// [ New link ] on an expired invite's row, by invite id.
    NewLink(String),
    /// Remove… on a Members row (never You, never a Mac with the token).
    Remove(RowRef),
    /// member-remove-confirm's [ Remove ] / [ Cancel invite ].
    ConfirmRemove,
    /// “Add a token here” / “Reconnect here”: the token row, on a Mac without the token.
    AddToken,
    /// Settings · Space's “made with shroom” switch (owner-class, U3).
    SetMark(bool),
    /// The Library's *Mine · Everyone*.
    SetScope(LibraryScope),
}

/// A Members row a Remove… is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "id")]
pub enum RowRef {
    /// A joined member, by member id.
    Member(String),
    /// A pending invite, by invite id.
    Invite(String),
    /// Another of the owner's Macs, by device id.
    Mac(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
    Filled,
    Plain,
    Quiet,
    RedText,
    RedOutline,
    Link,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Button {
    pub action: Action,
    pub label: String,
    pub style: Style,
    pub enabled: bool,
    /// A grey note under a greyed control (law 4's "You can do this once…").
    pub note: Option<String>,
}

impl Button {
    pub fn new(action: Action, label: String, style: Style) -> Button {
        Button { action, label, style, enabled: true, note: None }
    }
    pub fn enabled(mut self, on: bool) -> Button {
        self.enabled = on;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Choice {
    pub action: Action,
    pub label: String,
    pub selected: bool,
    pub enabled: bool,
    /// What the choice means now, in grey beneath it — System default's device (N44).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Toggle {
    pub action: Action,
    pub label: String,
    pub on: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Face {
    Plain,
    Red,
    PauseBars,
    Amber,
    Frost,
}

/// What a click on the mushroom means right now — "the click is the door" (law 5), decided
/// in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Click {
    OpenPopover,
    OpenOnboarding,
    Pause,
    ShowPaused,
    CancelCountdown,
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tray {
    /// The tray's own state: `idle-unprovisioned`, `idle-ready`, `recording`, or none.
    pub state: Option<&'static str>,
    pub face: Face,
    pub click: Click,
    /// The one-item right-click menu ("Quit shroom") — idle only.
    pub right_menu: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    Amber,
    Frost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusTone {
    /// Uploads moving, an update applying.
    Busy,
    /// Waiting for a connection, an update that didn't finish.
    Waiting,
    /// Updated.
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Banner {
    pub id: &'static str,
    pub tone: Tone,
    pub line: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfigBody {
    pub record_label: String,
    pub sources: Vec<Choice>,
    /// Show yourself, a pick like the microphone's: every camera here, then Camera off —
    /// none with Camera only as the source, whose camera it picks (DEVIATIONS #43).
    pub camera_label: String,
    pub cameras: Vec<Choice>,
    pub mic_label: String,
    pub mics: Vec<Choice>,
    pub mic_line: Option<String>,
    pub mic_hint: Option<Button>,
    pub quality_label: String,
    pub qualities: Vec<Choice>,
    pub camera_line: Option<String>,
    pub camera_settings: Option<Button>,
    pub start: Button,
    pub hint: String,
    pub room_line: Option<String>,
    pub footer: Vec<Button>,
}

impl ConfigBody {
    /// The camera Show yourself shows, by uid — `None` at Camera off (DEVIATIONS #43). With
    /// Camera only as the source, the camera the take records.
    /// System default (N44) is the camera named beneath it.
    pub fn camera_picked(&self) -> Option<&str> {
        fn uid_of(c: &Choice) -> Option<&str> {
            match &c.action {
                Action::SetCamera(Some(uid)) => Some(uid.as_str()),
                _ => None,
            }
        }
        let picked = self.cameras.iter().find(|c| c.selected)?;
        match &picked.action {
            Action::SetCameraDefault => self.cameras.iter().find(|c| Some(&c.label) == picked.detail.as_ref()).and_then(uid_of),
            _ => uid_of(picked),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PausedBody {
    pub title: String,
    /// The auto-pause context line (display-lost, mic-lost, …).
    pub context: Option<String>,
    pub sub: Option<String>,
    pub resume: Button,
    pub restart: Button,
    pub stop: Button,
    pub discard: Button,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuestionBody {
    pub title: String,
    pub line: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoppedBody {
    pub title: Option<String>,
    pub line: String,
    pub link: Option<String>,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum PopoverBody {
    Config(ConfigBody),
    Paused(PausedBody),
    Question(QuestionBody),
    Stopped(StoppedBody),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Popover {
    /// The fixture's state id, verbatim.
    pub id: &'static str,
    /// Faces of that state showing now (N18: faces are render properties of their state).
    pub faces: Vec<&'static str>,
    pub body: PopoverBody,
    /// At most one — an ask (law 7).
    pub banner: Option<Banner>,
    /// At most one — a fact (law 7).
    pub status_line: Option<String>,
    /// The status line's dot: moving, waiting, or done (DEVIATIONS #45).
    pub status_tone: Option<StatusTone>,
    /// What Return does (law 8); `None` on destructive questions (law 9).
    pub ret: Option<Action>,
    pub esc: Option<Action>,
    /// `false`: shown without taking the keyboard (law 5).
    pub activates: bool,
    /// An unasked popover that a click anywhere else puts away, like one the person
    /// opened: the update relaunch's (N41, as amended 2026-10-15). The others that open
    /// by themselves stay until touched (DEVIATIONS #39).
    pub away_closes: bool,
}

impl Popover {
    pub fn buttons(&self) -> Vec<&Button> {
        let mut out: Vec<&Button> = Vec::new();
        if let Some(b) = &self.banner {
            out.extend(&b.buttons);
        }
        match &self.body {
            PopoverBody::Config(c) => {
                out.push(&c.start);
                out.extend(&c.footer);
                out.extend(c.camera_settings.iter());
                out.extend(c.mic_hint.iter());
            }
            PopoverBody::Paused(p) => out.extend([&p.resume, &p.stop, &p.restart, &p.discard]),
            PopoverBody::Question(q) => out.extend(&q.buttons),
            PopoverBody::Stopped(s) => out.extend(&s.buttons),
        }
        out
    }

    /// Everything clickable in the popover, enabled.
    pub fn offers(&self, action: &Action) -> bool {
        if self.buttons().iter().any(|b| b.enabled && &b.action == action) {
            return true;
        }
        match &self.body {
            PopoverBody::Config(c) => c
                .sources
                .iter()
                .chain(&c.cameras)
                .chain(&c.mics)
                .chain(&c.qualities)
                .any(|ch| ch.enabled && &ch.action == action),
            _ => false,
        }
    }
}

/// A window (ARCHITECTURE §3.2): the fixture's state id, its faces, its body, what Return
/// and Esc do, and whether it may take the keyboard.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Window<B> {
    pub id: &'static str,
    pub faces: Vec<&'static str>,
    pub body: B,
    pub ret: Option<Action>,
    pub esc: Option<Action>,
    pub activates: bool,
}

/// A run's ratified emphasis (the spec's bold and italic words).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Em {
    Strong,
    Em,
}

/// One run of a line: its text, plain or set in its emphasis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub text: String,
    pub em: Option<Em>,
}

/// `text` cut into runs at each mark's first occurrence, the mark set in its emphasis; the
/// runs, joined, are `text`. Webviews render the runs and add no emphasis of their own.
pub fn runs(text: &str, marks: &[(&str, Em)]) -> Vec<Run> {
    let mut cuts: Vec<(usize, usize, Em)> =
        marks.iter().filter_map(|(m, em)| (!m.is_empty()).then(|| text.find(m).map(|i| (i, i + m.len(), *em))).flatten()).collect();
    cuts.sort_by_key(|c| c.0);
    let mut out = Vec::new();
    let mut at = 0;
    for (from, to, em) in cuts {
        if from < at {
            continue; // overlapping marks: the first one wins
        }
        if from > at {
            out.push(Run { text: text[at..from].to_string(), em: None });
        }
        out.push(Run { text: text[from..to].to_string(), em: Some(em) });
        at = to;
    }
    if at < text.len() || out.is_empty() {
        out.push(Run { text: text[at..].to_string(), em: None });
    }
    out
}

/// A line with its emphasis: `text` whole, `runs` the same text cut where emphasis
/// starts and ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rich {
    pub text: String,
    pub runs: Vec<Run>,
}

impl Rich {
    pub fn new(text: String, marks: &[(&str, Em)]) -> Rich {
        let runs = runs(&text, marks);
        Rich { text, runs }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WelcomeBody {
    pub paragraphs: Vec<Rich>,
    pub login: Toggle,
    /// The ratified pair, filled action first (law 8's local exception).
    pub buttons: Vec<Button>,
    /// “Uninstall shroom…”, quiet at the foot where the invite line was (B160). An invite
    /// link pasted anywhere in the window still opens its invite (B159): no door shows.
    pub uninstall: Button,
    /// The Uninstall sheet over the welcome, once asked (B160).
    pub sheet: Option<Sheet>,
}

/// → current (Frost) · ✓ done (green) · • still ahead (N13) · a small spinner: a row ahead
/// shroom is already doing, out of sight (wiring under the permissions chapter, N45).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Glyph {
    Current,
    Done,
    Ahead,
    Working,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanRow {
    pub glyph: Glyph,
    /// The whole row as shown, its glyph first.
    pub line: String,
    /// The row after its glyph (and the space after it), in runs with its emphasis.
    pub runs: Vec<Run>,
    /// The row's parts, as the canvas lays them out (DEVIATIONS #45): its title; under it, a
    /// row ahead's or the current row's parenthetical (or the dash's rest), in runs with its
    /// emphasis; and at its end a done row's fact. The words are the line's, split — never
    /// new ones.
    pub title: String,
    pub aside: Vec<Run>,
    pub fact: Option<String>,
}

impl PlanRow {
    pub fn new(glyph: Glyph, line: String, marks: &[(&str, Em)]) -> PlanRow {
        let rest = line.char_indices().nth(1).map(|(i, _)| line[i..].trim_start()).unwrap_or_default().to_string();
        let runs = runs(&rest, marks);
        // A done row: “title — fact”. A row ahead or current: “title (aside)”, or “title — aside”.
        let dash = rest.split_once(" — ");
        let (title, aside, fact) = match glyph {
            Glyph::Done => match dash {
                Some((t, f)) => (t.to_string(), String::new(), Some(f.to_string())),
                None => (rest.clone(), String::new(), None),
            },
            _ => match rest.split_once(" (").filter(|_| rest.ends_with(')')) {
                Some((t, a)) => (t.to_string(), a[..a.len() - 1].to_string(), None),
                None => match dash {
                    Some((t, a)) => (t.to_string(), a.to_string(), None),
                    None => (rest.clone(), String::new(), None),
                },
            },
        };
        let aside = if aside.is_empty() { vec![] } else { self::runs(&aside, marks) };
        PlanRow { glyph, line, runs, title, aside, fact }
    }
}

/// The plan panel (shared with Settings · Space later): the header, six rows, and the
/// nest under the current row (or below the plan when every row is done).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanView {
    pub header: String,
    pub rows: Vec<PlanRow>,
    /// 1–6; `None` when every row is done.
    pub current: Option<u8>,
    pub nest: Nest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Nest {
    Account { question: String, buttons: Vec<Button> },
    SigningUp { line: String, buttons: Vec<Button> },
    Token {
        line: String,
        create: Button,
        placeholder: String,
        /// A paste was taken and shroom holds it — while it's checked, and while it's put
        /// away. The field shows it as dots (D:B104). A yes or a no and nothing more: the
        /// core never has the text or its length to say (law 13). No once it's refused.
        held: bool,
        checking: bool,
        error: Option<String>,
        /// The quiet reminder under the field (N45): on the sign-up path, until a paste lands.
        quiet: Option<String>,
    },
    PickAccount { line: String, accounts: Vec<Choice>, choose: Button },
    /// The row's spinner: an answer this row needs isn't in yet.
    Checking,
    /// A dashboard gate under the card row: its line, its button, the auto-poll spinner.
    /// `quiet`: the card gate's line under its button — where it goes, and that shroom is
    /// watching (N45).
    Gate { line: String, button: Option<Button>, quiet: Option<String> },
    Name {
        prefix: String,
        value: String,
        /// The edits of `value` taken in so far, as `TextField::edits` (DECISIONS B107).
        edits: u32,
        suffix: String,
        line: String,
        claim: Button,
        /// The charset guard's quiet fading line.
        hint: Option<String>,
        /// name-taken: red edges and its line.
        taken: Option<String>,
        /// A claim that failed in a way nothing classifies (DEVIATIONS #25): its amber line;
        /// Claim stays the way on.
        unexpected: Option<String>,
    },
    /// Wiring: the spinner and the calm line, zero affordances.
    Calm { line: String },
    Found { line: String },
    /// wizard-offline: the spinner rests, one amber line, no retry.
    Offline { line: String },
    /// A failure nothing else classifies (DEVIATIONS #25): the spinner rests, one amber
    /// line, and [ Try again ] — the backoff retries by itself meanwhile.
    Unexpected { line: String, button: Button },
    Done { lines: Vec<String>, allow: Button },
    /// macOS's own microphone prompt is up; nothing of ours.
    PermMic,
    /// macOS's own camera prompt is up (N43); nothing of ours.
    PermCamera,
    /// `settings`: while macOS's alert leads, “Closed macOS's window?” and its door (N43).
    PermScreen { line: String, quiet: String, reopen: Button, settings: Option<QuietDoor> },
    /// No render: the app is quitting to reopen.
    Relaunching,
    Priming { line: String },
    AllSet { lines: Vec<String>, buttons: Vec<Button> },
    // ── Settings · Space (the plan as the space's settings surface) ──
    /// Every row ticked: nothing to redo.
    Ready { line: String },
    /// name-changed: the line, no affordance — re-probed when the window regains focus.
    NameChanged { line: String },
    /// space-storage-gone: the line, [ Set it up fresh ] · Disconnect this Mac….
    StorageGone { line: String, buttons: Vec<Button> },
    /// Row six un-ticked by a revoked screen grant: [ Turn it back on ].
    ScreenOff { button: Button },
    /// A Mac without the token, every row ticked: where the space is looked after (an
    /// owner-class Mac's “Repairs happen…” and [ Add a token here ], a member's line).
    NoToken { line: String, button: Option<Button> },
    /// Join's answer for good, short of the 200 (N31): under the joining row, its line, and
    /// the paste door where a new link is the way on (“Have a new link? Paste it here.”).
    JoinRefused { line: String, door: Option<String> },
}

/// The invite window's faces that aren't a plan (Region 6, N29–N31): member-welcome, the
/// switch, and the dead or already-here invite.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InviteBody {
    pub title: Option<String>,
    pub lines: Vec<String>,
    /// member-welcome's “Start shroom when you log in” (U4).
    pub login: Option<Toggle>,
    /// Filled action first, as the welcome's twin (law 8's local exception).
    pub buttons: Vec<Button>,
    /// A dead invite's paste door: “Have a new link? Paste it here.”
    pub door: Option<String>,
    /// The spinner rests here (an arrival waiting on its space).
    pub spinner: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum OnboardingBody {
    Welcome(WelcomeBody),
    Plan(PlanView),
    Invite(InviteBody),
}

impl Window<OnboardingBody> {
    pub fn buttons(&self) -> Vec<&Button> {
        match &self.body {
            OnboardingBody::Welcome(b) => match &b.sheet {
                Some(s) => s.buttons().collect(),
                None => b.buttons.iter().chain(std::iter::once(&b.uninstall)).collect(),
            },
            OnboardingBody::Plan(p) => nest_buttons(&p.nest),
            OnboardingBody::Invite(b) => b.buttons.iter().collect(),
        }
    }

    /// Everything clickable in the window, enabled.
    pub fn offers(&self, action: &Action) -> bool {
        if self.buttons().iter().any(|b| b.enabled && &b.action == action) {
            return true;
        }
        match &self.body {
            OnboardingBody::Welcome(b) => b.sheet.is_none() && b.login.enabled && &b.login.action == action,
            OnboardingBody::Plan(p) => match &p.nest {
                Nest::PickAccount { accounts, .. } => accounts.iter().any(|c| c.enabled && &c.action == action),
                _ => false,
            },
            OnboardingBody::Invite(b) => b.login.as_ref().is_some_and(|t| t.enabled && &t.action == action),
        }
    }

    /// The paste field a secret may land in now, if any.
    pub fn takes_paste(&self, field: crate::event::SecretField) -> bool {
        use crate::event::SecretField;
        match (&self.body, field) {
            // No door shows (B159): a paste anywhere in the welcome is classified.
            (OnboardingBody::Welcome(b), SecretField::Invite) => b.sheet.is_none(),
            (OnboardingBody::Plan(p), SecretField::Token) => matches!(p.nest, Nest::Token { .. }),
            (OnboardingBody::Plan(p), SecretField::Invite) => matches!(p.nest, Nest::JoinRefused { door: Some(_), .. }),
            (OnboardingBody::Invite(b), SecretField::Invite) => b.door.is_some(),
            _ => false,
        }
    }
}

/// A sheet over a window (N23's law for the destructive ones: no Return default, Esc ≡
/// Cancel, the destructive button red-outlined and rightmost).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sheet {
    pub id: &'static str,
    pub faces: Vec<&'static str>,
    pub title: String,
    pub lines: Vec<String>,
    /// Why the destructive button is greyed, when it is (one line).
    pub blocked: Option<String>,
    /// Cancel first, the answer rightmost.
    pub buttons: Vec<Button>,
}

/// A text field: its placeholder and what it holds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextField {
    pub placeholder: String,
    pub value: String,
    pub enabled: bool,
    /// How many edits of this field the core has taken in since its window's fields were
    /// last cleared. A window counts the edits it sends, and lets `value` replace what's
    /// typed only once this has caught up: a Scene that counts fewer answers an older edit
    /// (DECISIONS B107).
    pub edits: u32,
}

/// What a click on a row means, by gesture (the full macOS idiom, N35).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Gestures {
    pub click: Option<Action>,
    pub cmd_click: Option<Action>,
    pub shift_click: Option<Action>,
    pub title_click: Option<Action>,
    pub double_click: Option<Action>,
    /// ⌘C on a selected row.
    pub copy: Option<Action>,
}

/// One Library row: title, then date · length, then a state word only when it isn't live.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LibraryRowView {
    /// Stable for the webview's list.
    pub key: String,
    pub title: Option<String>,
    /// The title being renamed in place.
    pub rename: Option<TextField>,
    /// “Aug 21 · 4:32”.
    pub meta: String,
    /// Silence means live.
    pub state: Option<String>,
    pub tone: Option<Tone>,
    /// The thin bar under an uploading row.
    pub progress: Option<u32>,
    /// A recovered take, pinned to the top under the amber dot.
    pub pinned: bool,
    pub selected: bool,
    pub gestures: Gestures,
    /// Always visible.
    pub buttons: Vec<Button>,
    /// On hover or selection.
    pub hover: Vec<Button>,
    /// The right-click menu.
    pub menu: Vec<Button>,
    /// *Everyone*'s rows add the member's label (N33): another member's video.
    pub member: Option<String>,
    /// The row's poster (N35: “title, date · length, the poster”), as the shell serves it
    /// to this window: `<rec>` for the space's own, `<rec>/<take dir>` when this Mac holds
    /// the take (its own `poster.jpg` first). `None`: a neutral placeholder.
    pub poster: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LibraryBody {
    pub title: String,
    pub search: TextField,
    /// *Mine · Everyone* — owner-class Macs once someone else has a video here; else empty.
    pub scope: Vec<Choice>,
    /// One quiet line on top (offline).
    pub notice: Option<String>,
    /// The empty or no-match line, in place of the list.
    pub empty: Option<String>,
    pub rows: Vec<LibraryRowView>,
    /// The count only.
    pub footer: Option<String>,
    pub sheet: Option<Sheet>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneralBody {
    pub login: Toggle,
    pub updates_label: String,
    pub update_line: String,
    pub update: Option<Button>,
    pub whats_new: Option<Button>,
    /// The help row (B180): “Something off?”, Copy diagnostics, Report a problem….
    pub help_label: String,
    pub copy_diagnostics: Button,
    pub report: Button,
    /// After a report went through, until the window closes.
    pub sent: Option<String>,
    /// The line under the card; its doors are `help_links`, each label a run of the line.
    pub help: String,
    pub help_links: Vec<Button>,
    /// “Uninstall shroom…”, quiet text at the tab's foot (B135).
    pub uninstall: Button,
}

/// The Report a problem sheet's form, between its line and its buttons (B180).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportForm {
    pub words: TextField,
    pub reply_label: String,
    pub reply_to: TextField,
    /// “See exactly what's sent”.
    pub disclosure: String,
    /// What goes, word for word — once the report is gathered.
    pub preview: Option<String>,
    /// Until then: “Putting it together…”.
    pub preparing: Option<String>,
    /// The last Send didn't go through: its line, and the mail address as a link.
    pub failed: Option<String>,
    pub mail: Option<Button>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpaceBody {
    /// The re-probed plan (Region 2's own panel): every row ticked with its fact, or the
    /// first broken row current with Region 2's state beneath it.
    pub plan: PlanView,
    /// A denied microphone's quiet door (the row stays ticked).
    pub mic: Option<Button>,
    /// The “made with shroom” switch and its line — owner-class Macs, once the space said
    /// where it stands (U3, DEVIATIONS #19).
    pub mark: Option<Toggle>,
    pub mark_line: Option<String>,
    /// Quiet text at the foot.
    pub exits: Vec<Button>,
}

/// A born link's pill: the address up to `/join/`, and the stash handle the webview
/// draws the code from (law 13) — with its one action and its line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinkPill {
    pub prefix: String,
    pub link: SecretId,
    pub copy: Button,
    pub line: String,
}

/// One Members row: its line, a fact, its buttons, and the link just born under it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MemberRowView {
    pub key: String,
    pub line: String,
    pub fact: Option<String>,
    pub buttons: Vec<Button>,
    pub born: Option<LinkPill>,
}

/// The Members tab (N28, N33): You and the owner's other Macs, the members, the pending
/// invites, and the composer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MembersBody {
    pub title: String,
    /// N5's truth, said once, here.
    pub line: String,
    /// “You” — it carries no fact (N28).
    pub you: String,
    /// *Add another Mac*: quiet Frost text beside You.
    pub add_mac: Button,
    /// The own-Mac link's nest under You, while it shows.
    pub mac_link: Option<LinkPill>,
    /// The owner's other Macs, as sub-rows under You.
    pub macs: Vec<MemberRowView>,
    /// Joined members, then pending invites.
    pub rows: Vec<MemberRowView>,
    /// “Who's this for?” — Return in it ≡ Invite.
    pub name: TextField,
    pub invite: Button,
    pub invite_line: String,
    /// Who invites say they're from, under the composer (B158).
    pub from: FromName,
}

/// “Invites will say they're from **Bagrat**.” and *Change* — or, while it's being
/// changed, the line's head as the field's label, the field, and *Save* (B158).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum FromName {
    Shown { line: Rich, change: Button },
    Changing { label: String, field: TextField, save: Button },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum SettingsPane {
    General(GeneralBody),
    Space(SpaceBody),
    Members(MembersBody),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettingsBody {
    /// The toolbar.
    pub tabs: Vec<Choice>,
    pub pane: SettingsPane,
    pub sheet: Option<Sheet>,
    /// The Report a problem sheet's form, with its `sheet` (B180).
    pub report: Option<ReportForm>,
}

impl Sheet {
    fn buttons(&self) -> impl Iterator<Item = &Button> {
        self.buttons.iter()
    }
}

impl Window<LibraryBody> {
    pub fn buttons(&self) -> Vec<&Button> {
        if let Some(s) = &self.body.sheet {
            return s.buttons().collect();
        }
        self.body.rows.iter().flat_map(|r| r.buttons.iter().chain(&r.hover).chain(&r.menu)).collect()
    }

    pub fn offers(&self, action: &Action) -> bool {
        if self.buttons().iter().any(|b| b.enabled && &b.action == action) {
            return true;
        }
        if self.body.sheet.is_some() {
            return false;
        }
        if self.body.scope.iter().any(|c| c.enabled && &c.action == action) {
            return true;
        }
        // Esc always drops the typing; saving it needs the field live (offline it greys).
        let field = self.body.rows.iter().find_map(|r| r.rename.as_ref());
        match (field, action) {
            (Some(_), Action::CancelRename) => return true,
            (Some(f), Action::CommitRename) => return f.enabled,
            _ => {}
        }
        if *action == Action::DeleteSelected {
            return self.body.rows.iter().any(|r| r.selected && r.hover.iter().any(|b| b.enabled && matches!(b.action, Action::DeleteRow(_))));
        }
        self.body.rows.iter().any(|r| {
            let g = &r.gestures;
            [&g.click, &g.cmd_click, &g.shift_click, &g.title_click, &g.double_click, &g.copy].into_iter().any(|a| a.as_ref() == Some(action))
        })
    }
}

impl Window<SettingsBody> {
    pub fn buttons(&self) -> Vec<&Button> {
        if let Some(s) = &self.body.sheet {
            return s.buttons().chain(self.body.report.iter().flat_map(|f| &f.mail)).collect();
        }
        match &self.body.pane {
            SettingsPane::General(g) => {
                g.update.iter().chain(&g.whats_new).chain([&g.copy_diagnostics, &g.report]).chain(&g.help_links).chain(std::iter::once(&g.uninstall)).collect()
            }
            SettingsPane::Space(sp) => {
                let mut out = nest_buttons(&sp.plan.nest);
                out.extend(sp.mic.iter());
                out.extend(&sp.exits);
                out
            }
            SettingsPane::Members(m) => {
                let mut out = vec![&m.add_mac];
                out.extend(m.mac_link.iter().map(|p| &p.copy));
                for row in m.macs.iter().chain(&m.rows) {
                    out.extend(&row.buttons);
                    out.extend(row.born.iter().map(|p| &p.copy));
                }
                out.push(&m.invite);
                match &m.from {
                    FromName::Shown { change, .. } => out.push(change),
                    FromName::Changing { save, .. } => out.push(save),
                }
                out
            }
        }
    }

    pub fn offers(&self, action: &Action) -> bool {
        if self.buttons().iter().any(|b| b.enabled && &b.action == action) {
            return true;
        }
        if self.body.sheet.is_some() {
            return false;
        }
        if self.body.tabs.iter().any(|c| c.enabled && &c.action == action) {
            return true;
        }
        match &self.body.pane {
            SettingsPane::General(g) => g.login.enabled && &g.login.action == action,
            SettingsPane::Space(sp) => sp.mark.as_ref().is_some_and(|t| t.enabled && &t.action == action),
            // Esc in the name being changed puts it back (B158).
            SettingsPane::Members(m) => *action == Action::KeepFrom && matches!(m.from, FromName::Changing { .. }),
        }
    }

    /// The token field takes a token (the repair mood's, or Add a token here — the same
    /// machine as the wizard's); the Space tab's invite door takes a join link (N29).
    pub fn takes_paste(&self, field: crate::event::SecretField) -> bool {
        let SettingsPane::Space(sp) = &self.body.pane else { return false };
        self.body.sheet.is_none()
            && match field {
                crate::event::SecretField::Token => matches!(sp.plan.nest, Nest::Token { .. }),
                // No door shows (B159): a link pasted anywhere in the tab opens its invite.
                crate::event::SecretField::Invite => !matches!(sp.plan.nest, Nest::Token { .. }),
            }
    }
}

/// A nest's buttons — shared by the onboarding window and Settings · Space.
/// A quiet line and the door beside it (perm-screen's “Closed macOS's window?”, N43).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuietDoor {
    pub quiet: String,
    pub button: Button,
}

pub fn nest_buttons(nest: &Nest) -> Vec<&Button> {
    match nest {
        Nest::Account { buttons, .. } | Nest::SigningUp { buttons, .. } | Nest::AllSet { buttons, .. } | Nest::StorageGone { buttons, .. } => {
            buttons.iter().collect()
        }
        Nest::Token { create, .. } => vec![create],
        Nest::PickAccount { choose, .. } => vec![choose],
        Nest::Gate { button, .. } => button.iter().collect(),
        Nest::Name { claim, .. } => vec![claim],
        Nest::Done { allow, .. } => vec![allow],
        Nest::PermScreen { reopen, settings, .. } => std::iter::once(reopen).chain(settings.iter().map(|d| &d.button)).collect(),
        Nest::ScreenOff { button } => vec![button],
        Nest::NoToken { button, .. } => button.iter().collect(),
        Nest::Unexpected { button, .. } => vec![button],
        Nest::Checking
        | Nest::JoinRefused { .. }
        | Nest::Calm { .. }
        | Nest::Found { .. }
        | Nest::Offline { .. }
        | Nest::PermMic
        | Nest::PermCamera
        | Nest::Relaunching
        | Nest::Priming { .. }
        | Nest::Ready { .. }
        | Nest::NameChanged { .. } => vec![],
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Countdown {
    pub display: u32,
    pub digit: u8,
    pub line: String,
}

/// The Show yourself bubble (N22): round, mirrored, on the display being recorded —
/// recorded as-is (never excluded from capture), never key (law 5), draggable.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bubble {
    /// Where the person put it: an offset from its display's top-left (`None` = the
    /// default corner).
    pub at: Option<(i32, i32)>,
    /// The display it sits on — the chosen display source (it follows the choice).
    pub display: u32,
    /// The camera it shows, by uid: never an iPhone by itself (N39).
    pub camera: String,
}

/// Camera as the source: its own preview at `config`, through the count and the take, on
/// the display the count plays on (N39, DEVIATIONS #42) — the picture the take records,
/// not mirrored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Preview {
    pub display: u32,
    pub camera: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scene {
    pub tray: Tray,
    pub popover: Option<Popover>,
    /// The onboarding window: the welcome, then the plan (Regions 1–3's chapter).
    pub onboarding: Option<Window<OnboardingBody>>,
    /// The Library window (N35).
    pub library: Option<Window<LibraryBody>>,
    /// The Settings window (N36).
    pub settings: Option<Window<SettingsBody>>,
    pub countdown: Option<Countdown>,
    pub bubble: Option<Bubble>,
    pub preview: Option<Preview>,
    /// The Dock icon shows while a window of shroom's is open — the setup window, the
    /// Library or Settings — never for a popover alone (law 6, SM:133).
    pub dock: bool,
    /// The next instant `derive` would answer differently on its own (the countdown digit,
    /// "Copied" reverting, a deadline); the runner posts `Tick` then.
    pub next_wake: Option<Mono>,
}

impl Scene {
    /// Every fixture state id showing, for the inventory tests.
    pub fn state_ids(&self) -> Vec<&'static str> {
        let mut ids = vec![match self.tray.face {
            Face::Plain => "plain",
            Face::Red => "tray-red-dot",
            Face::PauseBars => "tray-pause-bars",
            Face::Amber => "tray-amber-dot",
            Face::Frost => "tray-frost-dot",
        }];
        ids.extend(self.tray.state);
        if let Some(p) = &self.popover {
            ids.push(p.id);
            ids.extend(&p.faces);
            if let Some(b) = &p.banner {
                ids.push(b.id);
            }
        }
        if let Some(o) = &self.onboarding {
            ids.push(o.id);
            ids.extend(&o.faces);
        }
        if let Some(l) = &self.library {
            ids.push(l.id);
            ids.extend(&l.faces);
            if let Some(s) = &l.body.sheet {
                ids.push(s.id);
                ids.extend(&s.faces);
            }
        }
        if let Some(st) = &self.settings {
            ids.push(st.id);
            ids.extend(&st.faces);
            if let Some(s) = &st.body.sheet {
                ids.push(s.id);
                ids.extend(&s.faces);
            }
        }
        if self.countdown.is_some() {
            ids.push("countdown");
        }
        if self.bubble.is_some() {
            ids.push("bubble");
        }
        ids
    }

    pub fn offers(&self, surface: Surface, action: &Action) -> bool {
        match surface {
            Surface::Popover => self.popover.as_ref().is_some_and(|p| p.offers(action)),
            Surface::Countdown => self.countdown.is_some() && *action == Action::CancelCountdown,
            Surface::Onboarding => self.onboarding.as_ref().is_some_and(|o| o.offers(action)),
            Surface::Library => self.library.as_ref().is_some_and(|l| l.offers(action)),
            Surface::Settings => self.settings.as_ref().is_some_and(|s| s.offers(action)),
            _ => false,
        }
    }

    pub fn ret(&self, surface: Surface) -> Option<Action> {
        match surface {
            Surface::Popover => self.popover.as_ref().and_then(|p| p.ret.clone()),
            Surface::Onboarding => self.onboarding.as_ref().and_then(|o| o.ret.clone()),
            Surface::Library => self.library.as_ref().and_then(|o| o.ret.clone()),
            Surface::Settings => self.settings.as_ref().and_then(|o| o.ret.clone()),
            _ => None,
        }
    }

    pub fn esc(&self, surface: Surface) -> Option<Action> {
        match surface {
            Surface::Popover => self.popover.as_ref().and_then(|p| p.esc.clone()),
            Surface::Onboarding => self.onboarding.as_ref().and_then(|o| o.esc.clone()),
            Surface::Library => self.library.as_ref().and_then(|o| o.esc.clone()),
            Surface::Settings => self.settings.as_ref().and_then(|o| o.esc.clone()),
            Surface::Countdown => self.countdown.as_ref().map(|_| Action::CancelCountdown),
            _ => None,
        }
    }
}

#[cfg(test)]
mod emphasis {
    use super::*;

    #[test]
    fn runs_cut_at_each_mark_and_join_back_to_the_line() {
        let text = crate::copy::welcome_what();
        let r = runs(&text, crate::copy::WELCOME_WHAT_MARKS);
        assert_eq!(r.iter().map(|x| x.text.as_str()).collect::<String>(), text);
        let set: Vec<_> = r.iter().filter_map(|x| x.em.map(|e| (x.text.as_str(), e))).collect();
        assert_eq!(set, vec![("shroom", Em::Strong), ("your own", Em::Em)]);
        assert_eq!(runs("plain", &[]), vec![Run { text: "plain".into(), em: None }]);
        assert_eq!(runs("", &[]), vec![Run { text: "".into(), em: None }]);
        // Overlaps: the first mark wins; a missing mark is ignored.
        let r = runs("abcdef", &[("bcd", Em::Em), ("cde", Em::Strong), ("zz", Em::Em)]);
        assert_eq!(r.iter().map(|x| (x.text.as_str(), x.em)).collect::<Vec<_>>(), vec![("a", None), ("bcd", Some(Em::Em)), ("ef", None)]);
    }

    #[test]
    fn a_row_is_its_title_its_aside_and_its_fact() {
        // The canvas draws a row as its title, a quiet line under it for a row ahead's or
        // the current row's parenthetical, and a done row's fact at its end (DEVIATIONS #45).
        let text = |r: &[Run]| r.iter().map(|x| x.text.as_str()).collect::<String>();
        let ahead = PlanRow::new(Glyph::Ahead, crate::copy::row_ahead(4), crate::copy::row_ahead_marks(4));
        assert_eq!(ahead.title, "Your link name");
        assert_eq!(text(&ahead.aside), "part of every link you share — v.yourname.workers.dev");
        assert!(ahead.aside.iter().any(|x| x.text == "yourname" && x.em == Some(Em::Em)));
        assert_eq!(ahead.fact, None);
        let current = PlanRow::new(Glyph::Current, crate::copy::row_current(2), &[]);
        assert_eq!((current.title.as_str(), text(&current.aside).as_str()), ("One access token for shroom", "a pre-filled page — review, create, paste"));
        let done = PlanRow::new(Glyph::Done, crate::copy::row_done(4, Some("v.myshroom.workers.dev")), &[]);
        assert_eq!((done.title.as_str(), done.fact.as_deref(), done.aside.is_empty()), ("Your link name", Some("v.myshroom.workers.dev"), true));
        let bare = PlanRow::new(Glyph::Done, crate::copy::row_done(5, None), &[]);
        assert_eq!((bare.title.as_str(), bare.fact.as_deref()), ("Wiring it up", None));
        // A fact with its own parentheses stays whole (N42's “v” note).
        let v = PlanRow::new(Glyph::Done, crate::copy::row_done(4, Some(&crate::copy::name_fact("shroom.myshroom.workers.dev", true))), &[]);
        assert_eq!(v.fact.as_deref(), Some("shroom.myshroom.workers.dev (“v” was already taken in your Cloudflare)"));
        // A current row whose aside is a dash's (row six with screen recording off).
        let off = PlanRow::new(Glyph::Current, crate::copy::row_six_off(), &[]);
        assert_eq!((off.title.as_str(), text(&off.aside).as_str()), ("Letting macOS allow recording", "screen recording is off"));
        // The member's rows.
        let joining = PlanRow::new(Glyph::Current, crate::copy::joining_row(Some("Bagrat")), &[]);
        assert_eq!((joining.title.as_str(), text(&joining.aside).as_str()), ("Joining Bagrat's space", "shroom does this part"));
    }

    #[test]
    fn a_rows_runs_are_its_line_after_the_glyph() {
        let row = PlanRow::new(Glyph::Ahead, crate::copy::row_ahead(4), crate::copy::row_ahead_marks(4));
        let joined: String = row.runs.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(format!("• {joined}"), row.line);
        assert!(row.runs.iter().any(|x| x.text == "yourname" && x.em == Some(Em::Em)));
    }
}
