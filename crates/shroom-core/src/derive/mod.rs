//! `derive(&World) -> Scene`: pure and total. Every surface comes from here.

pub mod editor;
pub mod feedback;
pub mod library;
pub mod members;
pub mod onboarding;
mod popover;
pub mod settings;
pub mod status;
mod tray;

pub(crate) use popover::{OneShot, one_shot_showing, selected_mic as popover_selected_mic, selected_source as popover_selected_source};
pub use popover::{
    DISK_MARGIN, ROOM_FLOOR_MINUTES, ROOM_LOW_MINUTES, bytes_per_minute, marker_offered, resume_mic, resume_ok, selected_camera, source_present,
    take_floor_bytes,
};

use crate::copy;
use crate::scene::{Bubble, Countdown, Preview, Scene};
use crate::time::Mono;
use crate::world::{Op, SourceId, UpdateOffer, World};

/// How long "Copied" shows (DEVIATIONS #8).
pub const COPIED_MS: u64 = 1_500;
/// The countdown: 3 · 2 · 1 (N22).
pub const COUNTDOWN_MS: u64 = 3_000;
/// “Every few hours”: how often the background update check runs, and how long a failed
/// check or download waits before the next try (DECISIONS B34).
pub const UPDATE_EVERY_MS: u64 = 4 * 3_600_000;
/// Opening the popover checks for an update when the last check, by the wall clock, is
/// at least this old (B154) — a Mac that sleeps between uses still hears of a release.
pub const POPOVER_CHECK_MS: i64 = 15 * 60_000;

pub fn derive(w: &World) -> Scene {
    let popover = popover::derive(w);
    let countdown = countdown(w);
    let config_showing = popover.as_ref().is_some_and(|p| p.id == "config");
    let bubble = bubble(w, config_showing);
    let preview = preview(w, config_showing);
    let onboarding = onboarding::derive(w);
    let library = library::derive(w);
    let settings = settings::derive(w);
    let feedback = feedback::derive(w);
    let editor = editor::derive(w);
    // Law 6 (SM:133), as ratified: the Dock icon iff a window of shroom's is open —
    // onboarding, the Library, Settings, Send feedback or the editor (DEVIATIONS #38's
    // setup-only Dock was rejected).
    let dock = onboarding.is_some() || library.is_some() || settings.is_some() || feedback.is_some() || editor.is_some();
    Scene { tray: tray::derive(w), popover, onboarding, library, settings, feedback, editor, countdown, bubble, preview, dock, next_wake: next_wake(w) }
}

/// The update to offer now: downloaded, verified, a space to carry it (never before
/// `config` exists, never past this Mac's macOS — N37).
pub fn update_ready(w: &World) -> Option<&str> {
    settings::ready_version(w).filter(|_| w.config_reachable())
}

/// The version an `--updated` relaunch landed, until its one status line was shown.
pub fn updated_pending(w: &World) -> Option<&str> {
    let v = w.launch.version.as_str();
    (!v.is_empty() && w.once.seen_version.as_deref().is_some_and(|s| s != v)).then_some(v)
}

/// When the next background update check is due: at launch, then every few hours (a
/// failed check waits as long — fail-soft). Only once a space exists.
pub fn next_check(w: &World) -> Option<Mono> {
    if !w.provisioned() || w.launch.version.is_empty() {
        return None;
    }
    let failed = w.inflight.failures.get(&Op::UpdateCheck).map(|f| f.at);
    Some(match w.update.checked.max(failed) {
        Some(t) => t.plus(UPDATE_EVERY_MS),
        None => Mono(0),
    })
}

/// The popover is open and the last check answered (or failed) a while ago by the wall
/// clock: check now (B154). The few-hours cadence counts awake time only, so a Mac that
/// sleeps between uses could otherwise go days without looking.
pub fn popover_check_due(w: &World) -> bool {
    next_check(w).is_some() && w.session.popover.is_some() && w.update.tried.is_none_or(|t| w.wall.0.saturating_sub(t.0) >= POPOVER_CHECK_MS)
}

/// How often a token Mac re-proves its whole space on its own (DECISIONS B51; ARCHITECTURE
/// §5 row 9): six hours of awake time.
pub const SELF_CHECK_MS: u64 = 6 * 3_600_000;
/// How long a page refresh that couldn't run waits before the next try (B50).
pub const REFRESH_RETRY_MS: u64 = 3_600_000;

/// When a token Mac's periodic self-check is due: at launch, then six hours after the last
/// whole pass (any pass: the wizard's, Settings', its own) or the last one that couldn't run
/// — and at once when a stop found its page isn't the space's since that pass. Only once
/// booted, provisioned, on the Mac that keeps the token.
pub fn next_self_check(w: &World) -> Option<Mono> {
    if !w.provisioned() || !w.space.token_holder() || w.launch.version.is_empty() {
        return None;
    }
    let failed = w.inflight.failures.get(&Op::Sweep).map(|f| f.at);
    let last = w.sweep.pass.max(failed);
    // A stop's probe found the link answering as no space at all: look now.
    // Only a held verdict (B60): one answer while a fresh address propagates isn't a finding.
    let gone = w.net.not_shroom_held() && matches!(&w.net.reach, crate::time::Fact::Known { v: crate::world::Reach::NotShroom, at } if last.is_none_or(|l| *at > l));
    Some(match last {
        _ if gone => Mono(0),
        Some(t) => t.plus(SELF_CHECK_MS),
        None => Mono(0),
    })
}

/// How often the token Mac's cleanup sweep runs (DECISIONS B91): with the self-check's
/// cadence — six hours of awake time — after the last one that answered or couldn't run.
pub const CLEANUP_MS: u64 = SELF_CHECK_MS;
/// The first sweep of a launch waits this long after the launch's whole pass proved the
/// space sound, so it never rides the launch's own burst of calls.
pub const CLEANUP_FIRST_MS: u64 = 10 * 60_000;
/// The longest shroom holds macOS's will-sleep (ARCHITECTURE §3.3's power row, B92): the
/// take's pause lands well inside it; past it, sleep goes ahead whatever happened.
pub const SLEEP_HOLD_MS: u64 = 2_000;

/// When the token Mac's cleanup sweep is due (B91): never before a whole probe pass has
/// answered, and never while that pass says the space is broken — then `CLEANUP_FIRST_MS`
/// after the pass, and `CLEANUP_MS` after the last sweep (fail-soft: one that couldn't run
/// waits as long). Only once booted, provisioned, on the Mac that keeps the token.
pub fn next_cleanup(w: &World) -> Option<Mono> {
    if !w.provisioned() || !w.space.token_holder() || w.launch.version.is_empty() || w.space.broken.is(&true) {
        return None;
    }
    let pass = w.sweep.pass?;
    let failed = w.inflight.failures.get(&Op::Cleanup).map(|f| f.at);
    Some(match w.library_ops.cleaned.max(failed) {
        Some(t) => t.plus(CLEANUP_MS),
        None => pass.plus(CLEANUP_FIRST_MS),
    })
}

/// How often every Mac's editor upkeep runs (B197 (d)): with the cleanup sweep's cadence —
/// six hours of awake time — after the last one that answered or couldn't run.
pub const UPKEEP_MS: u64 = CLEANUP_MS;

/// When the editor's upkeep is due (B197 (d)): once a launch — at once, booted on a Mac in a
/// space — then `UPKEEP_MS` after the last one (fail-soft: one that couldn't run waits as
/// long). Every Mac, token or not: each bakes and copies its own videos.
pub fn next_upkeep(w: &World) -> Option<Mono> {
    if !w.provisioned() || w.launch.version.is_empty() {
        return None;
    }
    let failed = w.inflight.failures.get(&Op::EditorUpkeep).map(|f| f.at);
    Some(match w.library_ops.upkept.max(failed) {
        Some(t) => t.plus(UPKEEP_MS),
        None => Mono(0),
    })
}

/// When the token Mac's page refresh is due (§6.10, B50): once a launch, an hour after one
/// that couldn't run. Only once booted, provisioned, on the Mac that keeps the token.
pub fn next_refresh(w: &World) -> Option<Mono> {
    if !w.provisioned() || !w.space.token_holder() || w.launch.version.is_empty() || w.space.page_checked.is_some() {
        return None;
    }
    Some(w.inflight.failures.get(&Op::Refresh).map(|f| f.at.plus(REFRESH_RETRY_MS)).unwrap_or(Mono(0)))
}

/// When the offered version's download is due: now, or a few hours after a failed one
/// (a fingerprint mismatch is discarded silently and retried later).
pub fn next_download(w: &World) -> Option<(Mono, String)> {
    let Some(Some(UpdateOffer::Ready { version })) = w.update.offer.get() else { return None };
    if w.update.staged.as_deref() == Some(version.as_str()) {
        return None;
    }
    let at = w.inflight.failures.get(&Op::UpdateDownload).map(|f| f.at.plus(UPDATE_EVERY_MS)).unwrap_or(Mono(0));
    Some((at, version.clone()))
}

fn countdown(w: &World) -> Option<Countdown> {
    let a = w.session.arming.as_ref().filter(|a| !a.fired)?;
    let elapsed = w.now.since(a.started);
    let digit = 3u64.saturating_sub(elapsed / 1000).clamp(1, 3) as u8;
    // Camera as the source: the count plays on the display holding its preview (N39).
    let display = match a.source {
        SourceId::Display(id) => id,
        SourceId::Camera => preview_display(w).unwrap_or(0),
    };
    Some(Countdown { display, digit, line: copy::countdown_cancel() })
}

/// The bubble is born when `config` opens with a camera picked for it (N22) and stays through
/// the count and the take; never with Camera as the source, never with the camera off for
/// shroom, never on an iPhone's camera by itself — and when its camera goes, it simply
/// goes (N39). It sits on the display being recorded, and follows the choice.
fn bubble(w: &World, config_showing: bool) -> Option<Bubble> {
    let source = match w.take.as_ref() {
        Some(t) => Some(t.source),
        None => w.session.arming.as_ref().map(|a| a.source).or_else(|| popover::selected_source(w)),
    };
    let Some(SourceId::Display(display)) = source else { return None };
    let granted = w.mac.camera.is(&crate::world::Access::Granted);
    let showing = config_showing || w.take_open();
    let camera = popover::selected_camera(w)?;
    (w.prefs.show_yourself && granted && showing).then(|| Bubble { at: w.prefs.bubble_at, display, camera: camera.uid.clone() })
}

/// Camera as the source shows its own preview at `config`, like the bubble, and through
/// the count (N39) — its last moment too, while the recorder opens the camera — and through
/// the take, on the camera it records, until Stop ends it (DEVIATIONS #42). Never a gap
/// between them: a gap closes its window, which comes back at the default place, and
/// reopens its camera beside the one the take just opened.
fn preview(w: &World, config_showing: bool) -> Option<Preview> {
    if let Some(t) = w.take.as_ref().filter(|t| t.source == SourceId::Camera) {
        if matches!(t.phase, crate::world::Phase::Finalizing(_)) || !w.mac.camera.is(&crate::world::Access::Granted) {
            return None;
        }
        let uid = t.camera.as_ref()?;
        w.mac.cameras.get()?.iter().find(|c| &c.uid == uid)?;
        return Some(Preview { display: preview_display(w)?, camera: uid.clone() });
    }
    let counting = w.session.arming.as_ref().is_some_and(|a| a.source == SourceId::Camera);
    let configuring = config_showing && w.take.is_none() && popover::selected_source(w) == Some(SourceId::Camera);
    if !(counting || configuring) || !w.mac.camera.is(&crate::world::Access::Granted) {
        return None;
    }
    let camera = popover::selected_camera(w)?;
    Some(Preview { display: preview_display(w)?, camera: camera.uid.clone() })
}

/// Where Camera's preview sits: the display the mushroom was clicked on, where `config`
/// hangs (N56); else the main display.
fn preview_display(w: &World) -> Option<u32> {
    let here = |id: u32| w.mac.displays.get().is_some_and(|ds| ds.iter().any(|d| d.id == id));
    w.display_pick.clicked.filter(|&id| here(id)).or_else(|| main_display(w))
}

pub(crate) fn main_display(w: &World) -> Option<u32> {
    let displays = w.mac.displays.get()?;
    displays.iter().find(|d| d.is_main).or(displays.first()).map(|d| d.id)
}

fn next_wake(w: &World) -> Option<Mono> {
    let mut wakes: Vec<Mono> = Vec::new();
    if let Some(a) = w.session.arming.as_ref().filter(|a| !a.fired) {
        let next_digit = a.started.plus((w.now.since(a.started) / 1000 + 1) * 1000);
        wakes.push(next_digit.min(a.deadline));
    }
    if let Some((_, at)) = &w.session.copied {
        wakes.push(at.plus(COPIED_MS));
    }
    wakes.extend(w.inflight.ops.values().copied());
    // The stop's probe asking again after a “not shroom” that hasn't held (B60).
    if w.session.page_open.is_some() {
        wakes.extend(w.net.not_shroom_reprobe());
    }
    wakes.extend(w.inflight.failures.values().map(|f| f.retry_at()));
    // The AI banner's Turn on, tried again quietly (N66).
    if w.space.ai_turn == Some(crate::world::AiTurn::Turning) {
        wakes.extend(w.inflight.failures.get(&Op::Ai).map(|f| f.ai_retry_at()));
    }
    onboarding::wakes(w, &mut wakes);
    settings::wakes(w, &mut wakes);
    members::wakes(w, &mut wakes);
    wakes.extend(next_check(w));
    wakes.extend(next_self_check(w));
    wakes.extend(next_refresh(w));
    wakes.extend(next_cleanup(w));
    wakes.extend(next_upkeep(w));
    // The editor's time-left line by Revert changes its words (B197 (e)).
    editor::wakes(w, &mut wakes);
    // macOS waits on the will-sleep answer: it goes at the hold's end at the latest.
    wakes.extend(w.mac.sleep_hold.map(|t| t.plus(SLEEP_HOLD_MS)));
    wakes.extend(next_download(w).map(|(t, _)| t));
    wakes.into_iter().filter(|t| *t > w.now).min()
}

/// Naming's last re-wiring didn't go through (N66, law 12) and nothing runs since: said in the
/// window it was asked from, for this visit to it.
pub fn ai_fell_through(w: &World, window: crate::world::WindowKind) -> bool {
    let entered = w.session.windows.get(&window).map(|d| d.entered);
    !w.inflight.ops.contains_key(&Op::Ai) && w.inflight.failures.get(&Op::Ai).is_some_and(|f| entered.is_some_and(|e| f.at >= e))
}
