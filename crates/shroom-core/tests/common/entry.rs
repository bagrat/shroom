//! The independent oracle (TESTING-PLAN §2.1, ARCHITECTURE §3 / §3.7 item 3; DECISIONS B89):
//! a second, simpler statement of which state each world is in, read from the fixtures'
//! entry conditions (`spec/machine.json`) and written without calling `derive` or any of its
//! helpers — then held against `derive`. It names, per surface:
//!
//! - the tray's state (`idle-unprovisioned`, `idle-ready`, `recording`) and its face;
//! - the popover: its state, its faces (the auto-pause causes, the saved-take question,
//!   disk-low, the one-shot status lines) and its one banner, by N38's order;
//! - the countdown and the bubble;
//! - the onboarding window on a Mac without an invite: the welcome, the plan's rows one to
//!   five, row six (the permissions chapter), `wizard-offline` and DEVIATIONS #25's face;
//! - Settings: the tab, and the Space tab's one mood.
//!
//! Where the fixtures don't say, it **abstains**, by name (see `ABSTAINS`) — never by
//! copying what `derive` does. A disagreement is a core bug (fixed minimally, named in
//! B89) or a spec ambiguity (DEVIATIONS, for Bagrat); the oracle is never bent to match.
//!
//! `READS` is the table: every id the oracle names, with the words of its fixture's entry
//! conditions the reading rests on — `tests/entry.rs` holds each phrase to the fixture.
#![allow(dead_code)]

use shroom_core::scene::{Face, Scene, SettingsPane};
use shroom_core::world::{
    Access, Cause, Confirm, Device, EditorMedia, EditorSave, FailClass, Fork, LaunchKind, Op, Paste, Phase, Role, SavedCause, SettingsTab, SourceId,
    StopOutcome, TokenVerdict, UpdateOffer, WindowKind, WorkerFact, World,
};

/// One reading: the state id, and the words of its fixture's `entry_conditions` it rests
/// on (or the DEVIATIONS entry that added the id).
pub struct Read {
    pub id: &'static str,
    pub rests_on: &'static str,
}

const fn r(id: &'static str, rests_on: &'static str) -> Read {
    Read { id, rests_on }
}

/// Every id the oracle names, and the fixture words it reads.
pub const READS: &[Read] = &[
    // ── the tray (r1, r3) ──
    r("idle-unprovisioned", "No space exists on this Mac and no welcome window is open."),
    r("idle-ready", "The space is provisioned and no take is open. The onboarding window is closed"),
    r("recording", "`countdown` elapsed and capture began, or Resume from `paused`."),
    r("plain", "Idle, with no banner of kinds 1–4 waiting."),
    r("tray-red-dot", "A take is recording."),
    r("tray-pause-bars", "A take is paused."),
    r("tray-amber-dot", "Idle, and a recovered take / unanswered disk-full-stop, a space-can't-take-videos banner, or recording-off waits."),
    r("tray-frost-dot", "Idle, an update is ready, and no amber-kind banner waits."),
    // ── the popover (r3, r4, r5, r6) ──
    r("config", "Not mid-take, and the tray clicked in idle-ready"),
    r("paused", "Tray click while recording, after the pause is confirmed"),
    r("display-lost", "Mid-take, the recorded display disconnects."),
    r("mic-lost", "Mid-take, the selected microphone disconnects."),
    r("camera-lost", "Mid-take with Camera as the source, and the camera disconnects."),
    r("mac-away", "Mid-take, the Mac sleeps, the lid closes, the screen locks, or a user switch happens."),
    r("mic-permission-lost", "Mid-take, the microphone grant is revoked."),
    r("discard-confirm", "Discard… from `paused` (including its auto-pause faces)."),
    r("discard-confirm-saved-take", "Discard… on the `recovered-take` banner in config."),
    r("restart-confirm", "DEVIATIONS #44"),
    r("quit-confirm", "Quit shroom (config footer, or the idle right-click menu) while uploads are draining."),
    r("stopped-offline", "the short reachability probe of the space's page (capped well under a second) can't reach it."),
    r("stopped-stranded", "At stop, the space can't take this video"),
    r("permission-lost", "Mid-take, the screen-recording grant is revoked."),
    r("disk-full-stop", "Mid-take, free disk space reaches the floor."),
    r("disk-low", "`config` is open and free disk space at the chosen quality is low."),
    r("updated", "A successful update relaunch: the popover opens by itself, unasked, under the mushroom (B155)."),
    r("update-failed", "An update's swap failed."),
    // The banners, N38's order (SM:765-774).
    r("recovered-take", "A take was interrupted (crash, forced shutdown, system-initiated termination), or disk-full-stop was dismissed unanswered."),
    r("space-degraded", "An auth- or space-class failure on a publish or the periodic self-check, any time after setup"),
    r("space-degraded-no-token-mac", "space-degraded's cause, on an owner-class Mac that does not hold the Cloudflare token."),
    r("member-revoked", "A member Mac whose mint door refuses with the removed class"),
    r("space-needs-owner", "A member Mac whose mint door fails with a space-side class"),
    r("space-gone", "A member Mac whose space's address answers as no space at all."),
    r("space-needs-update-member", "Member Mac. This app version needs newer page code than the space currently runs."),
    r("space-needs-update-no-token", "Owner-class Mac without the token. This app version needs newer page code than the space has."),
    r("owner-mac-removed", "DEVIATIONS #21"),
    r("recording-off", "`config` opens and the probe finds screen recording switched off for shroom."),
    r("update-ready", "The release is downloaded and verified against the published fingerprint. A space exists"),
    r("updating-app", "Update clicked in config's banner or Settings → General, not mid-take."),
    // ── the overlays (r3) ──
    r("countdown", "Start recording in `config`."),
    r("bubble", "A camera is picked in the camera picker (not Camera off) and the source is a display"),
    // ── the onboarding window (r1, r2, r3, r6) ──
    r("welcome", "a first launch on a Mac with no space (the installer's launch); the tray click in idle-unprovisioned"),
    r("first-launch", "The installer placed and launched the app on a Mac with no space: a first launch."),
    r("account", "nothing is probeable yet: no valid stored token."),
    r("account-signing-up", "User chose Not yet — take me there at account"),
    r("token", "Box 1 ticked (I have one / I signed up), or a re-entry/repair re-probe finds no valid stored token"),
    r("token-garbled", "A pasted value fails the shape/prefix check (pre-network)."),
    r("token-wrong-powers", "Verify passes but one harmless probe per permission fails."),
    r("token-pick-account", "Verify lists more than one account for the pasted token."),
    r("card", "Paste validated and token persisted (or account chosen at token-pick-account); probe pass ran."),
    r("gate-email", "Write-class probe returns 403 8000077"),
    r("gate-card", "R2-enabled probe returns the needs-payment class (after the email gate, if both)."),
    r("name", "the account has no workers.dev subdomain (10007) after the card row clears."),
    r("name-taken", "Claim returns 10031."),
    r("name-v-clash", "finds a non-shroom Worker named v in the account."),
    r("wiring", "The permissions chapter is done (row six ticked) and wiring hasn't finished."),
    r("wizard-offline", "Transport-level failure at any wizard step."),
    r("wizard-unexpected", "DEVIATIONS #25"),
    r("space-found", "Probes find an existing space for the pasted token's account."),
    r("done", "The link name row ticks (a name claimed, or the card step found one) — wiring starts underneath (N45) — or space-found ticked rows three to five."),
    r("perm-mic", "The microphone grant is unresolved"),
    r("perm-camera", "the camera grant is unasked, and the Mac has a camera."),
    r("perm-screen", "The mic and the camera are resolved (allowed or denied; the camera skipped on a Mac without one) and the screen-recording grant is not live (derived)."),
    r("relaunching", "the user accepted macOS's quit-and-reopen offer, or the user clicked Reopen shroom"),
    r("perm-priming", "the screen-recording grant has just become live."),
    r("all-set", "`perm-priming` finished clean and row six ticked, and wiring is done (else `wiring` is the last step, N45)."),
    r("member-done", "joining succeeded: the member is confirmed and the identity has landed locally."),
    r("member-all-set", "The permissions chapter ends on a member Mac"),
    r("member-all-set-own-mac", "The permissions chapter ends on a Mac joined through an own-Mac link."),
    // ── Settings (r5) ──
    r("settings", "A space exists, so config is reachable"),
    r("settings-general", "Settings window on the General tab (the default on first open)."),
    r("settings-space", "Settings window on the Space tab."),
    r("settings-members", "Owner-class Mac only."),
    r("settings-space-inspection", "This Mac holds the Cloudflare token and every probe passes."),
    r("settings-space-repair", "A token-holding Mac where a re-probe un-ticks a row."),
    r("repair-mood", "Reconnect your space from the space-degraded banner."),
    r("settings-space-no-token", "Owner-class Mac that does not hold the Cloudflare token."),
    r("settings-space-member", "This Mac is a member (not owner-class)."),
    // ── Send feedback (N60) ──
    r("send-feedback", "The ready popover's first header icon, Send feedback, clicked"),
    // ── the editor (N63) ──
    r("editor", "Edit… on a live row in the Library (N63), when the video is on this Mac"),
    r("editor-getting", "Edit… on a live row whose video isn't on this Mac."),
    r("editor-cant-get", "The fetch in editor-getting failed."),
    r("editor-saving", "Done (or ✕) in editor with changes, or Done again in editor-save-failed."),
    r("editor-save-failed", "A save from editor-saving failed."),
    r("editor-making", "The save landed in editor-saving with something left out."),
    r("editor-making-failed", "Making the final video failed in editor-making."),
    r("editor-keep-original", "The final video landed while the editor was open."),
];

/// Where the fixtures don't say, and the oracle names nothing (each a reason, not a copy of
/// what `derive` does).
pub const ABSTAINS: &[(&str, &str)] = &[
    ("the tray's face while a take finalizes", "`stopping` (r3/r4) is an `other`-surface beat: the fixtures give it no tray face (red dot is “recording”, pause bars “paused”)"),
    ("the bubble while a take finalizes", "the bubble's entry runs “from config's opening through countdown, recording and paused” — `stopping` isn't named"),
    ("the onboarding window with an invite held", "Region 6's invite window (N29–N31) is designed state by state from peek's and join's answers; region6.rs and the status seam walk it"),
    ("the Library window", "N35's faces read the space's records and this Mac's takes; region5.rs's inventory rows walk them"),
    ("Settings · Space's nest beneath its mood", "the nest is Region 2's own state under the row (N42) — enumerate.rs's converse walks it"),
];

/// What the oracle names for one world. `None` in a field means it abstains there.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    pub tray: Option<&'static str>,
    pub face: Option<&'static str>,
    pub popover: Option<Option<Pop>>,
    pub countdown: bool,
    pub bubble: Option<bool>,
    pub onboarding: Option<Option<Win>>,
    pub settings: Option<Option<Win>>,
    /// The Send feedback window (N60).
    pub feedback: Option<Win>,
    /// The editor (N63).
    pub editor: Option<Win>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pop {
    pub id: &'static str,
    pub faces: Vec<&'static str>,
    pub banner: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Win {
    pub id: &'static str,
    pub faces: Vec<&'static str>,
}

/// The popover faces the oracle reads (any other face `derive` adds is not its business).
pub const POPOVER_FACES: &[&str] = &["display-lost", "mic-lost", "camera-lost", "mac-away", "mic-permission-lost", "discard-confirm-saved-take", "disk-low", "updated", "update-failed"];
/// The onboarding faces the oracle reads.
pub const ONBOARDING_FACES: &[&str] = &["first-launch", "name-v-clash", "wizard-unexpected", "member-done", "member-all-set", "member-all-set-own-mac"];
/// The Settings faces the oracle reads: the tab, and the Space tab's mood.
pub const SETTINGS_FACES: &[&str] = &[
    "settings-general",
    "settings-space",
    "settings-members",
    "settings-space-inspection",
    "settings-space-repair",
    "settings-space-no-token",
    "settings-space-member",
];

// ── facts, as the fixtures speak of them ──

/// A take record exists: the countdown fired and capture was asked to begin.
fn take_phase(w: &World) -> Option<Phase> {
    w.take.as_ref().map(|t| t.phase)
}

/// “A take is recording” — capture began and isn't paused or ending (a pause or a resume
/// under way is still the recording until the recorder confirms it, r3 `paused`).
fn recording(w: &World) -> bool {
    matches!(take_phase(w), Some(Phase::Starting | Phase::Recording | Phase::Pausing(_) | Phase::Resuming))
}

fn paused(w: &World) -> Option<Cause> {
    match take_phase(w) {
        Some(Phase::Paused(c)) => Some(c),
        _ => None,
    }
}

fn finalizing(w: &World) -> bool {
    matches!(take_phase(w), Some(Phase::Finalizing(_)))
}

/// The countdown runs: Start was clicked and the count hasn't fired (or been cancelled).
fn counting(w: &World) -> bool {
    w.session.arming.as_ref().is_some_and(|a| !a.fired)
}

/// A take is open for law 4 (DEVIATIONS #3: from the countdown on).
fn open_since(w: &World) -> Option<u64> {
    match (&w.session.arming, &w.take) {
        (Some(a), _) => Some(a.started.0),
        (None, Some(t)) => Some(t.opened.0),
        _ => None,
    }
}

/// A window shows unless it opened after the take began (law 4). Law 6 reads the setup
/// window's from here (`laws::law_6`): the Dock icon's side of the World.
pub fn shows(w: &World, kind: WindowKind) -> bool {
    w.session.windows.get(&kind).is_some_and(|d| open_since(w).is_none_or(|t| d.opened.0 < t))
}

fn has_space(w: &World) -> bool {
    w.space.identity.is_some()
}

fn role(w: &World) -> Option<Role> {
    w.space.identity.as_ref().map(|i| i.role)
}

/// “Holds the Cloudflare token”: the Mac that set the space up and keeps its token.
fn holds_token(w: &World) -> bool {
    role(w) == Some(Role::TokenHolder) && w.space.token
}

/// “Owner-class Mac that does not hold the Cloudflare token” (N33): every other Mac of the
/// owner — an own-Mac join, or the token's role with the token gone from this Mac.
fn owner_class_without_token(w: &World) -> bool {
    matches!(role(w), Some(Role::OwnerClass)) || (role(w) == Some(Role::TokenHolder) && !w.space.token)
}

/// Row six is done on this Mac (the warm-up primed).
fn primed(w: &World) -> bool {
    w.mac.primed.is_some()
}

/// `config` exists: a space, and this Mac's permissions chapter done (r5 update-ready: “A
/// space exists (config exists — before that, no banner and no dot)”).
fn config_exists(w: &World) -> bool {
    has_space(w) && primed(w)
}

/// WORKER-INTERFACE §4.8 (C7): a space-side answer is a face only once two of them came at
/// least 30 s apart.
fn strike(w: &World) -> Option<bool> {
    w.members.standing.strike.filter(|s| s.last.0.saturating_sub(s.first.0) >= 30_000).map(|s| s.gone)
}

/// The banner of kind 2 (“the space can't take videos”, N38) in this Mac's role's words,
/// and N34's table: the token's Mac reads `space-degraded`; members and the owner's other
/// Macs read their standing (`/me`, a mint).
fn kind_two(w: &World) -> Option<&'static str> {
    if holds_token(w) {
        return w.space.broken.is(&true).then_some("space-degraded");
    }
    let st = &w.members.standing;
    match role(w)? {
        Role::Member => Some(if st.revoked {
            "member-revoked"
        } else {
            match strike(w) {
                Some(true) => "space-gone",
                Some(false) => "space-needs-owner",
                None if st.behind => "space-needs-update-member",
                None => return None,
            }
        }),
        Role::OwnerClass => Some(if st.revoked {
            "owner-mac-removed"
        } else if strike(w).is_some() {
            "space-degraded-no-token-mac"
        } else if st.behind {
            "space-needs-update-no-token"
        } else {
            return None;
        }),
        Role::TokenHolder => None,
    }
}

/// The space takes none of this Mac's videos (N34): removed, or its address no space at all.
fn space_refuses_this_mac(w: &World) -> bool {
    matches!(role(w), Some(Role::Member | Role::OwnerClass)) && (w.members.standing.revoked || strike(w) == Some(true))
}

/// N37: a newer release this Mac's macOS runs, downloaded and verified.
fn update_ready(w: &World) -> bool {
    matches!(w.update.offer.get(), Some(Some(UpdateOffer::Ready { version })) if w.update.staged.as_deref() == Some(version.as_str())) && config_exists(w)
}

fn recording_off(w: &World) -> bool {
    config_exists(w) && w.mac.screen.is(&false)
}

fn banner(w: &World) -> Option<&'static str> {
    if !w.saved.is_empty() {
        return Some("recovered-take");
    }
    if let Some(b) = kind_two(w) {
        return Some(b);
    }
    if recording_off(w) {
        return Some("recording-off");
    }
    update_ready(w).then_some(if w.update.applying { "updating-app" } else { "update-ready" })
}

/// The status line's upload truth (N38) speaks — so a one-shot line waits.
fn upload_truth(w: &World) -> bool {
    !w.queue.is_empty() && !space_refuses_this_mac(w)
}

/// Minutes of room at the chosen quality (N39's disk-low: the scaffold flag's numbers).
fn room_low(w: &World) -> bool {
    use shroom_core::derive::{DISK_MARGIN, ROOM_LOW_MINUTES, bytes_per_minute};
    w.mac.disk_free.get().is_some_and(|free| free.saturating_sub(DISK_MARGIN) / bytes_per_minute(w.prefs.quality) < ROOM_LOW_MINUTES)
}

fn tray(w: &World) -> Option<&'static str> {
    if recording(w) {
        return Some("recording");
    }
    if w.take.is_some() || w.session.windows.contains_key(&WindowKind::Onboarding) {
        return None;
    }
    Some(if has_space(w) { "idle-ready" } else { "idle-unprovisioned" })
}

fn face(w: &World) -> Option<&'static str> {
    if recording(w) {
        return Some("tray-red-dot");
    }
    if paused(w).is_some() {
        return Some("tray-pause-bars");
    }
    if finalizing(w) {
        return None;
    }
    let amber = !w.saved.is_empty() || kind_two(w).is_some() || recording_off(w);
    Some(if amber {
        "tray-amber-dot"
    } else if update_ready(w) {
        "tray-frost-dot"
    } else {
        "plain"
    })
}

fn popover(w: &World) -> Option<Pop> {
    w.session.popover?;
    // The count closed the popover; nothing pops over the scrim (DEVIATIONS #3).
    if counting(w) {
        return None;
    }
    let pop = |id, faces: Vec<&'static str>| Pop { id, faces, banner: None };
    if w.take.is_some() {
        let cause = paused(w)?;
        // A Mac that's away shows nothing until it's back (mac-away greets the return).
        let present = w.mac.os.get().is_none_or(|o| o.awake && o.unlocked && o.on_console);
        if matches!(cause, Cause::Asleep | Cause::Locked) && !present {
            return None;
        }
        if w.session.confirm == Some(Confirm::Discard) {
            return Some(pop("discard-confirm", vec![]));
        }
        // Restart asks first (DEVIATIONS #44).
        if w.session.confirm == Some(Confirm::Restart) {
            return Some(pop("restart-confirm", vec![]));
        }
        let face = match cause {
            Cause::User => None,
            Cause::DisplayLost => Some("display-lost"),
            Cause::MicLost => Some("mic-lost"),
            Cause::CameraLost => Some("camera-lost"),
            Cause::Asleep | Cause::Locked => Some("mac-away"),
            Cause::MicRevoked => Some("mic-permission-lost"),
        };
        return Some(pop("paused", face.into_iter().collect()));
    }
    match &w.session.confirm {
        Some(Confirm::DiscardSaved(dir)) if w.saved.iter().any(|s| &s.dir == dir) => return Some(pop("discard-confirm", vec!["discard-confirm-saved-take"])),
        // N34's stranded take, tossed: N25's question in the popover (B40 h).
        Some(Confirm::DiscardRow(dir)) if w.queue.iter().any(|q| &q.dir == dir) => return Some(pop("discard-confirm", vec!["discard-confirm-saved-take"])),
        Some(Confirm::Quit) => return Some(pop("quit-confirm", vec![])),
        _ => {}
    }
    match &w.session.last_stop {
        Some(StopOutcome::Offline(_)) => return Some(pop("stopped-offline", vec![])),
        Some(StopOutcome::Stranded(_)) => return Some(pop("stopped-stranded", vec![])),
        Some(StopOutcome::AutoStopped(dir)) => {
            if let Some(s) = w.saved.iter().find(|s| &s.dir == dir) {
                return Some(pop(if s.cause == SavedCause::ScreenOff { "permission-lost" } else { "disk-full-stop" }, vec![]));
            }
        }
        None => {}
    }
    let mut faces = Vec::new();
    if room_low(w) {
        faces.push("disk-low");
    }
    if !upload_truth(w) {
        let v = w.launch.version.as_str();
        let updated = !v.is_empty() && w.once.seen_version.as_deref().is_some_and(|s| s != v);
        if updated {
            faces.push("updated");
        } else if w.update.failed {
            faces.push("update-failed");
        }
    }
    Some(Pop { id: "config", faces, banner: banner(w) })
}

/// The source the bubble would sit on: the take's, the count's, else the one chosen —
/// remembered if it's still here, else the display with the menu bar (N39).
fn source(w: &World) -> Option<SourceId> {
    if let Some(t) = &w.take {
        return Some(t.source);
    }
    if let Some(a) = &w.session.arming {
        return Some(a.source);
    }
    let displays = w.mac.displays.get().cloned().unwrap_or_default();
    let camera_here = w.mac.cameras.get().is_some_and(|cs| cs.iter().any(|c| !c.continuity));
    match w.prefs.source {
        Some(SourceId::Camera) if camera_here => Some(SourceId::Camera),
        Some(SourceId::Display(id)) if displays.iter().any(|d| d.id == id) => Some(SourceId::Display(id)),
        _ => displays.iter().find(|d| d.is_main).or(displays.first()).map(|d| SourceId::Display(d.id)),
    }
}

fn bubble(w: &World, config: bool) -> Option<bool> {
    if finalizing(w) {
        return None;
    }
    // “through countdown, recording and paused”: one span — the count's end hands over to
    // capture beginning (`recording`'s entry: “countdown elapsed and capture began”), so the
    // beat between the two (the count fired, the recorder starting) is inside it.
    let through = config || w.session.arming.is_some() || recording(w) || paused(w).is_some();
    // N39: never with the camera off for shroom, never an iPhone's camera by itself — one the
    // person picked in the camera picker is theirs to show (DEVIATIONS #43).
    let picked = |cs: &Vec<Device>| w.prefs.camera.as_ref().is_some_and(|uid| cs.iter().any(|c| &c.uid == uid));
    let camera = w.mac.camera.is(&Access::Granted) && w.mac.cameras.get().is_some_and(|cs| picked(cs) || cs.iter().any(|c| !c.continuity));
    Some(w.prefs.show_yourself && through && camera && matches!(source(w), Some(SourceId::Display(_))))
}

// ── the onboarding window ──

/// A stored token that a probe hasn't found dead.
fn token_alive(w: &World) -> bool {
    w.space.token && !matches!(w.sweep.token.get(), Some(TokenVerdict::WrongPowers | TokenVerdict::Revoked))
}

/// The wizard's calls a face waits on fail in a way nothing classifies (DEVIATIONS #25).
fn unexpected(w: &World, ops: &[Op]) -> bool {
    ops.iter().any(|op| w.inflight.failures.get(op).is_some_and(|f| matches!(f.class, FailClass::Refused | FailClass::Unclassified | FailClass::Panicked)))
}

/// N18: offline, or a transport failure on a wizard call no answer has cleared.
fn offline(w: &World) -> bool {
    w.net.online.is(&false) || [Op::Validate, Op::SaveToken, Op::Sweep, Op::Claim, Op::Wire].iter().any(|op| w.inflight.failures.get(op).is_some_and(|f| f.class == FailClass::Transport))
}

/// The plan's step on a Mac with no space yet, by the rows' own entry conditions: the
/// step's id, whether it waits on the network (N18), and the calls it waits on.
fn plan_step(w: &World) -> (&'static str, bool, &'static [Op]) {
    // Box 1: self-reported (I have one / I signed up), or a stored token re-proves it (N14).
    let box1 = w.space.token || w.session.fork == Fork::Affirmed;
    if !box1 {
        return (if w.session.fork == Fork::SigningUp { "account-signing-up" } else { "account" }, false, &[]);
    }
    if !token_alive(w) {
        return match &w.sweep.paste {
            Some(Paste::Garbled) => ("token-garbled", false, &[]),
            Some(Paste::WrongPowers) => ("token-wrong-powers", false, &[]),
            Some(Paste::PickAccount { .. }) => ("token-pick-account", false, &[]),
            Some(Paste::Checking(_) | Paste::Saving { .. }) => ("token", true, &[Op::Validate, Op::SaveToken]),
            Some(Paste::OtherAccount) | None => ("token", false, &[]),
        };
    }
    // The card row's pass: the gates, then the account's name and the page's label (B26).
    // `perm-screen` is re-entered after `relaunching` ("resumes right here"): in the
    // chapter, a pass the relaunch hasn't re-run yet leaves the rows it ticked ticked.
    let held = w.session.chapter;
    let gates = w.sweep.gates.get();
    let gates_clear = gates.map_or(held, |g| !g.email && !g.card);
    let surveyed = (w.sweep.subdomain.get().is_some() || held) && (w.sweep.label.get().is_some() || held);
    if !(gates_clear && surveyed) {
        return match gates {
            Some(g) if g.email => ("gate-email", true, &[Op::Sweep]),
            Some(g) if g.card => ("gate-card", true, &[Op::Sweep]),
            _ => ("card", true, &[Op::Sweep]),
        };
    }
    if matches!(w.sweep.subdomain.get(), Some(None)) {
        let taken = !w.session.fields.name.is_empty() && w.sweep.taken.as_deref() == Some(w.session.fields.name.as_str());
        return (if taken { "name-taken" } else { "name" }, true, &[Op::Claim]);
    }
    if matches!(w.sweep.found.get(), Some(Some(_))) {
        return ("space-found", true, &[Op::Wire]);
    }
    // The name row ticked: the permissions chapter is the step, wiring underneath; wiring
    // is the last step only once row six has ticked (N45).
    if !primed(w) {
        return (chapter_step(w), false, &[]);
    }
    ("wiring", true, &[Op::Wire])
}

/// Where the permissions chapter stands (Region 3's entry conditions).
fn chapter_step(w: &World) -> &'static str {
    if !w.session.chapter {
        "done"
    } else if !matches!(w.mac.mic.get(), Some(Access::Granted | Access::Denied | Access::Restricted)) {
        "perm-mic"
    } else if w.mac.camera_asked && w.mac.camera.is(&Access::NotAsked) {
        // The prompt the mic's answer chained is up (N43).
        "perm-camera"
    } else if !w.mac.screen.is(&true) {
        if w.launch.ending { "relaunching" } else { "perm-screen" }
    } else {
        "perm-priming"
    }
}

fn onboarding(w: &World) -> Option<Option<Win>> {
    let Some(door) = w.session.windows.get(&WindowKind::Onboarding) else { return Some(None) };
    if !shows(w, WindowKind::Onboarding) {
        return Some(None);
    }
    if w.members.invite.is_some() {
        return None;
    }
    let win = |id, faces: Vec<&'static str>| Some(Some(Win { id, faces }));
    if has_space(w) {
        let mut faces = Vec::new();
        // Set up beside someone else's `v` (N42): the name row goes on saying so, on the
        // Mac that set the space up — the one whose window shows the plan's own rows.
        if role(w) == Some(Role::TokenHolder) && w.space.identity.as_ref().is_some_and(|i| i.foreign_v) {
            faces.push("name-v-clash");
        }
        let id = if primed(w) {
            match role(w) {
                Some(Role::Member) => faces.push("member-all-set"),
                Some(Role::OwnerClass) => faces.push("member-all-set-own-mac"),
                _ => {}
            }
            "all-set"
        } else {
            if !w.session.chapter && role(w) != Some(Role::TokenHolder) {
                faces.push("member-done");
            }
            chapter_step(w)
        };
        return win(id, faces);
    }
    // The welcome until Set up my space (DEVIATIONS #1), a stored token, or a paste.
    if !w.prefs.setup_chosen && !w.space.token && w.sweep.paste.is_none() {
        let first = matches!(w.launch.kind, LaunchKind::Install { .. }) && door.opened == w.launch.booted;
        return win("welcome", if first { vec!["first-launch"] } else { vec![] });
    }
    let (step, network, ops) = plan_step(w);
    let mut faces = Vec::new();
    // “v” already someone else's: the space's pages take another label (N42) — a derived
    // label the row's fact says, not a state. The fixture's condition is the Worker the
    // probe pass **finds**, never the label alone: a label other than `v` beside a `v`
    // nobody looked at (a build that passes `v` by) is no clash.
    if w.sweep.foreign_v.is(&true) {
        faces.push("name-v-clash");
    }
    if network && offline(w) {
        return win("wizard-offline", faces);
    }
    if unexpected(w, ops) {
        faces.insert(0, "wizard-unexpected");
        // Under the name row the field and Claim stay (the claim is only the person's click).
        return win(if matches!(step, "name" | "name-taken") { step } else { "wizard-unexpected" }, faces);
    }
    win(step, faces)
}

// ── Settings ──

fn settings(w: &World) -> Option<Option<Win>> {
    if !w.session.windows.contains_key(&WindowKind::Settings) || !shows(w, WindowKind::Settings) || !has_space(w) {
        return Some(None);
    }
    let owner_class = matches!(role(w), Some(Role::TokenHolder | Role::OwnerClass));
    let tab = match w.prefs.settings_tab {
        Some(SettingsTab::Members) if owner_class => SettingsTab::Members,
        Some(SettingsTab::Space) => SettingsTab::Space,
        _ => SettingsTab::General,
    };
    let faces = match tab {
        SettingsTab::General => vec!["settings-general"],
        SettingsTab::Members => vec!["settings-members"],
        SettingsTab::Space => {
            let mood = if role(w) == Some(Role::Member) {
                "settings-space-member"
            } else if owner_class_without_token(w) {
                "settings-space-no-token"
            } else if space_row_unticked(w) {
                "settings-space-repair"
            } else {
                "settings-space-inspection"
            };
            vec!["settings-space", mood]
        }
    };
    Some(Some(Win { id: "settings", faces }))
}

/// A row of the space itself (two to five) un-ticked on the token's Mac (N42's table), or
/// the space reported broken and this entry's re-probe not answered yet — the repair mood
/// Reconnect your space opens (r2 `repair-mood`).
fn space_row_unticked(w: &World) -> bool {
    let Some(id) = &w.space.identity else { return false };
    let token_dead = matches!(w.sweep.token.get(), Some(TokenVerdict::Revoked | TokenVerdict::WrongPowers));
    let gate = w.sweep.gates.get().is_some_and(|g| g.email || g.card);
    let renamed = w.sweep.subdomain.get().is_some_and(|s| s.as_deref() != Some(id.subdomain.as_str()));
    let page_gone = w.sweep.bucket.is(&false) || matches!(w.sweep.worker.get(), Some(WorkerFact::Absent | WorkerFact::KeyGone | WorkerFact::Foreign));
    let entered = w.session.windows.get(&WindowKind::Settings).map(|d| d.entered.0).unwrap_or(0);
    let checking = w.space.broken.is(&true) && w.sweep.pass.is_none_or(|t| t.0 < entered);
    token_dead || gate || renamed || page_gone || checking
}

/// Everything the oracle names for `w`.
pub fn name(w: &World) -> Named {
    let popover = popover(w);
    let config = popover.as_ref().is_some_and(|p| p.id == "config");
    Named {
        tray: tray(w),
        face: face(w),
        popover: Some(popover),
        countdown: counting(w),
        bubble: bubble(w, config),
        onboarding: onboarding(w),
        settings: settings(w),
        feedback: feedback(w),
        editor: editor(w),
    }
}

/// The Send feedback window: open from its icon, once a space exists — the popover's
/// header lives in `config` only (N60).
fn feedback(w: &World) -> Option<Win> {
    (shows(w, WindowKind::Feedback) && has_space(w)).then(|| Win { id: "send-feedback", faces: vec![] })
}

/// The editor, open from a live row's Edit… once a space exists (N63), its state by the
/// fixtures' entries: Done with changes is saving until the save answers, and “a save from
/// editor-saving failed” rests on its own face; “the save landed … with something left
/// out” is the final video being made, “making the final video failed” its own face, and
/// “the final video landed while the editor was open” the question (B197); before any
/// save, the video “isn't on this Mac” while the space hands it over, “the fetch … failed”
/// when it didn't, and otherwise the editor itself — the video here, or its record still
/// being read.
fn editor(w: &World) -> Option<Win> {
    if !shows(w, WindowKind::Editor) || !has_space(w) {
        return None;
    }
    let e = w.session.fields.editor.as_ref()?;
    let id = match (e.save, e.media) {
        (EditorSave::Saving, _) => "editor-saving",
        (EditorSave::Failed, _) => "editor-save-failed",
        (EditorSave::Making { .. }, _) => "editor-making",
        (EditorSave::MakingFailed, _) => "editor-making-failed",
        (EditorSave::Asking, _) => "editor-keep-original",
        (_, EditorMedia::Getting { .. }) => "editor-getting",
        (_, EditorMedia::Failed) => "editor-cant-get",
        (EditorSave::Idle, EditorMedia::Opening | EditorMedia::Ready { .. }) => "editor",
    };
    Some(Win { id, faces: vec![] })
}

fn face_name(f: Face) -> &'static str {
    match f {
        Face::Plain => "plain",
        Face::Red => "tray-red-dot",
        Face::PauseBars => "tray-pause-bars",
        Face::Amber => "tray-amber-dot",
        Face::Frost => "tray-frost-dot",
    }
}

fn read<'a>(faces: &[&'a str], known: &[&str]) -> Vec<&'a str> {
    faces.iter().copied().filter(|f| known.contains(f)).collect()
}

/// Every place the oracle and the Scene `derive` made disagree, in words.
pub fn disagreements(w: &World, s: &Scene) -> Vec<String> {
    let n = name(w);
    let mut out = Vec::new();
    if n.tray != s.tray.state {
        out.push(format!("tray: oracle {:?}, derive {:?}", n.tray, s.tray.state));
    }
    if let Some(f) = n.face {
        if f != face_name(s.tray.face) {
            out.push(format!("tray face: oracle {f}, derive {}", face_name(s.tray.face)));
        }
    }
    if let Some(p) = &n.popover {
        let got = s.popover.as_ref().map(|d| Pop { id: d.id, faces: read(&d.faces, POPOVER_FACES), banner: d.banner.as_ref().map(|b| b.id) });
        if p != &got {
            out.push(format!("popover: oracle {p:?}, derive {got:?}"));
        }
    }
    if n.countdown != s.countdown.is_some() {
        out.push(format!("countdown: oracle {}, derive {}", n.countdown, s.countdown.is_some()));
    }
    if let Some(b) = n.bubble {
        if b != s.bubble.is_some() {
            out.push(format!("bubble: oracle {b}, derive {}", s.bubble.is_some()));
        }
    }
    if let Some(o) = &n.onboarding {
        let got = s.onboarding.as_ref().map(|d| Win { id: d.id, faces: read(&d.faces, ONBOARDING_FACES) });
        if o != &got {
            out.push(format!("onboarding: oracle {o:?}, derive {got:?}"));
        }
    }
    if let Some(st) = &n.settings {
        let got = s.settings.as_ref().map(|d| Win { id: d.id, faces: read(&d.faces, SETTINGS_FACES) });
        // The pane must be the tab the faces name (the faces are what the oracle reads).
        let pane_ok = s.settings.as_ref().is_none_or(|d| match &d.body.pane {
            SettingsPane::General(_) => d.faces.contains(&"settings-general"),
            SettingsPane::Space(_) => d.faces.contains(&"settings-space"),
            SettingsPane::Members(_) => d.faces.contains(&"settings-members"),
        });
        if st != &got || !pane_ok {
            out.push(format!("settings: oracle {st:?}, derive {got:?}"));
        }
    }
    let got = s.feedback.as_ref().map(|d| Win { id: d.id, faces: d.faces.clone() });
    if n.feedback != got {
        out.push(format!("send feedback: oracle {:?}, derive {got:?}", n.feedback));
    }
    let got = s.editor.as_ref().map(|d| Win { id: d.id, faces: d.faces.clone() });
    if n.editor != got {
        out.push(format!("editor: oracle {:?}, derive {got:?}", n.editor));
    }
    out
}

/// Held at every step of every walkthrough (`support::Run::send`): the oracle and `derive`
/// agree wherever the oracle names something.
pub fn check(w: &World, s: &Scene) {
    let d = disagreements(w, s);
    assert!(d.is_empty(), "the entry oracle (tests/common/entry.rs, B89) and derive disagree:\n  {}", d.join("\n  "));
}
