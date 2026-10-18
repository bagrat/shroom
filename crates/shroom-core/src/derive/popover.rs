//! The popover: which state shows, its one banner and one status line (law 7), what
//! Return and Esc do (laws 8, 9), and whether it may take the keyboard (law 5).

use crate::copy;
use crate::scene::{
    Action, Banner, Button, Choice, ConfigBody, PausedBody, Popover, PopoverBody, QuestionBody, StoppedBody, Style, Tone,
};
use crate::world::{Access, AiTurn, Cause, Confirm, Device, Op, Phase, Quality, SavedCause, SourceId, StopOutcome, TakeLost, World};

pub fn derive(w: &World) -> Option<Popover> {
    let door = w.session.popover?;
    // The countdown closed the popover; nothing pops over the scrim (law 4, DEVIATIONS #3).
    if w.session.arming.as_ref().is_some_and(|a| !a.fired) {
        return None;
    }
    let mut p = if let Some(t) = &w.take {
        match t.phase {
            Phase::Paused(cause) => {
                // A Mac that's asleep, locked or switched away shows nothing until it's back.
                if matches!(cause, Cause::Asleep | Cause::Locked) && !w.mac.os.get().is_some_and(|o| o.present()) {
                    return None;
                }
                if w.session.confirm == Some(Confirm::Discard) {
                    discard_question(&copy::length(t.length_ms))
                } else if w.session.confirm == Some(Confirm::Restart) {
                    restart_question(&copy::length(t.length_ms))
                } else {
                    paused(w, cause, t.length_ms)
                }
            }
            // Recording has no popover; the in-between beats show nothing either.
            _ => return None,
        }
    } else if let Some(Confirm::DiscardSaved(dir)) = &w.session.confirm {
        let take = w.saved.iter().find(|s| &s.dir == dir)?;
        saved_discard_question(&copy::length(take.length_ms))
    } else if let Some(Confirm::DiscardRow(dir)) = &w.session.confirm {
        // A take the space can't take, saved to Movies: tossing it is N25's question (N34).
        let take = w.queue.iter().find(|q| &q.dir == dir)?;
        stranded_discard_question(take.length_ms)
    } else if w.session.confirm == Some(Confirm::Quit) {
        quit_question(w)
    } else if w.session.confirm == Some(Confirm::Ai) {
        ai_question()
    } else if let Some(stop) = &w.session.last_stop {
        stopped(w, stop)?
    } else {
        config(w)
    };
    p.activates = door.asked;
    // The update relaunch's popover is news a click away puts away (N41, as amended).
    p.away_closes = !door.asked && w.session.news == Some(door.opened);
    Some(p)
}

/// The source to record: Camera only while a camera shroom may pick by itself is here
/// (N39); a display picked by hand for this take; else the display the mushroom was clicked
/// on (N56); else the remembered one if it's still here; else the main display.
pub fn selected_source(w: &World) -> Option<SourceId> {
    let displays = w.mac.displays.get().cloned().unwrap_or_default();
    let here = |id: u32| displays.iter().any(|d| d.id == id);
    let clicked = w.display_pick.clicked.filter(|&id| here(id));
    match w.prefs.source {
        Some(SourceId::Camera) if selected_camera(w).is_some() => Some(SourceId::Camera),
        Some(SourceId::Display(id)) if w.display_pick.by_hand && here(id) => Some(SourceId::Display(id)),
        _ if clicked.is_some() => clicked.map(SourceId::Display),
        Some(SourceId::Display(id)) if here(id) => Some(SourceId::Display(id)),
        _ => super::main_display(w).map(SourceId::Display),
    }
}

/// The camera shroom shows and records: the one picked in the camera picker while it's here
/// (an iPhone too, when the person picked it — DEVIATIONS #43), else the built-in one, else
/// the first that isn't an iPhone over Continuity — never an iPhone by itself (N39:
/// Continuity devices come and go). `None` when only an iPhone is around, unpicked.
pub fn selected_camera(w: &World) -> Option<&Device> {
    let cams = w.mac.cameras.get()?;
    if let Some(c) = w.prefs.camera.as_ref().and_then(|uid| cams.iter().find(|c| &c.uid == uid)) {
        return Some(c);
    }
    system_default(cams)
}

/// What System default means now (N44): macOS's default device — never an iPhone over
/// Continuity by itself (N39), so then the built-in one, else the first that isn't one.
pub fn system_default(devices: &[Device]) -> Option<&Device> {
    devices
        .iter()
        .find(|d| d.system_default && !d.continuity)
        .or_else(|| devices.iter().find(|d| d.builtin && !d.continuity))
        .or_else(|| devices.iter().find(|d| !d.continuity))
}

/// The microphone to record with: none when No mic is picked (DEVIATIONS #43); else the
/// remembered one if present, else the built-in one, else any that isn't an iPhone (N39:
/// never auto-pick Continuity).
pub fn selected_mic(w: &World) -> Option<String> {
    if w.prefs.mic_off {
        return None;
    }
    let mics = w.mac.mics.get()?;
    if let Some(uid) = &w.prefs.mic {
        if mics.iter().any(|m| &m.uid == uid) {
            return Some(uid.clone());
        }
    }
    system_default(mics).map(|m| m.uid.clone())
}

/// The microphone Resume uses: the take's own while it's here, else the one `config` would
/// pick (the `mic-lost` line names it) — none while macOS has the microphone off.
pub fn resume_mic(w: &World) -> Option<String> {
    if !w.mac.mic.is(&Access::Granted) {
        return None;
    }
    let mics = w.mac.mics.get()?;
    match w.take.as_ref().and_then(|t| t.mic.as_ref()) {
        Some(uid) if mics.iter().any(|m| &m.uid == uid) => Some(uid.clone()),
        _ => selected_mic(w),
    }
}

/// Bytes a minute of recording takes on disk at `q` — the segments and the single-file
/// copy (the tee writes both), plus audio.
pub fn bytes_per_minute(q: Quality) -> u64 {
    let video_kbps = match q {
        Quality::Normal => 3_500,
        Quality::TwoK => 6_000,
        Quality::FourK => 12_000,
    };
    (video_kbps + 128) * 1000 / 8 * 60 * 2
}

/// Free space macOS needs to stay healthy; never recorded into.
pub const DISK_MARGIN: u64 = 1_000_000_000;
/// Below this many minutes of room, `config` says so (N39 `disk-low`).
pub const ROOM_LOW_MINUTES: u64 = 30;
/// Below this, Start is greyed and a take finalizes itself — the spec's “under about a
/// minute” (N39), on top of the margin.
pub const ROOM_FLOOR_MINUTES: u64 = 1;

pub fn room_minutes(w: &World) -> Option<u64> {
    let free = *w.mac.disk_free.get()?;
    Some(free.saturating_sub(DISK_MARGIN) / bytes_per_minute(w.prefs.quality))
}

/// The free space under which a take at `q`, `length_ms` long so far, finalizes itself:
/// the margin, a floor's worth of both tee legs, and the single-file copy still to be made
/// from what's recorded (one leg's worth of the whole take) — so the take ends whole, never
/// by a surprise ENOSPC (N39 `disk-full-stop`).
pub fn take_floor_bytes(q: Quality, length_ms: u64) -> u64 {
    let per_minute = bytes_per_minute(q);
    DISK_MARGIN + ROOM_FLOOR_MINUTES * per_minute + per_minute / 2 * length_ms / 60_000
}

fn config(w: &World) -> Popover {
    let source = selected_source(w);
    let displays = w.mac.displays.get().cloned().unwrap_or_default();
    // With screen recording switched off, displays grey out; Camera still records (N38).
    let screen = w.mac.screen.is(&true);
    let mut sources: Vec<Choice> = displays
        .iter()
        .map(|d| Choice {
            action: Action::SetSource(SourceId::Display(d.id)),
            label: display_label(&displays, d),
            selected: source == Some(SourceId::Display(d.id)),
            enabled: screen, detail: None
        })
        .collect();
    if selected_camera(w).is_some() {
        sources.push(Choice {
            action: Action::SetSource(SourceId::Camera),
            label: copy::camera_only(),
            selected: source == Some(SourceId::Camera),
            enabled: true, detail: None
        });
    }

    let mic = selected_mic(w);
    // A denied mic never blocks: the mic shows No mic, none to pick, with a quiet hint
    // (Region 3). No mic at all records silently, and says so (N39); No mic picked is the
    // person's own choice and needs no line (DEVIATIONS #43).
    let mic_denied = matches!(w.mac.mic.get(), Some(Access::Denied | Access::Restricted));
    // System default first (N44), the device it means beneath; a device picked by name is
    // the one checked while it's here.
    let all_mics = w.mac.mics.get().cloned().unwrap_or_default();
    let mic_named = w.prefs.mic.as_ref().filter(|uid| all_mics.iter().any(|m| &m.uid == *uid));
    let mut mics: Vec<Choice> = Vec::new();
    if let Some(d) = system_default(&all_mics) {
        mics.push(Choice {
            action: Action::SetMicDefault,
            label: copy::system_default(),
            selected: !mic_denied && !w.prefs.mic_off && mic_named.is_none(),
            enabled: !mic_denied,
            detail: Some(d.name.clone()),
        });
    }
    mics.extend(all_mics.iter().map(|m| Choice {
        action: Action::SetMic(Some(m.uid.clone())),
        label: m.name.clone(),
        selected: !mic_denied && !w.prefs.mic_off && mic_named == Some(&m.uid),
        enabled: !mic_denied,
        detail: None,
    }));
    mics.push(Choice { action: Action::SetMic(None), label: copy::no_mic_pick(), selected: mic_denied || mic.is_none(), enabled: !mic_denied, detail: None });
    let mic_line = (mic.is_none() && !mic_denied && !w.prefs.mic_off).then(copy::no_mic);
    let mic_hint = mic_denied.then(|| Button::new(Action::OpenMicSettings, copy::allowed_in_settings(), Style::Link));

    let camera_source = source == Some(SourceId::Camera);
    let wants_camera = camera_source || w.prefs.show_yourself;
    let camera_denied = matches!(w.mac.camera.get(), Some(Access::Denied | Access::Restricted));
    // The camera picker (DEVIATIONS #43): every camera, then Camera off. A denied camera
    // flips it back off — the quiet line says why (N39); the wish is kept, so allowing the
    // camera again brings the bubble back. Camera only picks the camera it records: no
    // Camera off then.
    let camera = selected_camera(w).map(|c| c.uid.clone());
    let showing = (camera_source || (w.prefs.show_yourself && !camera_denied)).then_some(camera.as_deref()).flatten();
    // System default first (N44), as for the microphone.
    let all_cameras = w.mac.cameras.get().cloned().unwrap_or_default();
    let camera_named = w.prefs.camera.as_ref().filter(|uid| all_cameras.iter().any(|c| &c.uid == *uid));
    let mut cameras: Vec<Choice> = Vec::new();
    if let Some(d) = system_default(&all_cameras) {
        cameras.push(Choice {
            action: Action::SetCameraDefault,
            label: copy::system_default(),
            selected: showing.is_some() && camera_named.is_none(),
            enabled: true,
            detail: Some(d.name.clone()),
        });
    }
    cameras.extend(all_cameras.iter().map(|c| Choice {
        action: Action::SetCamera(Some(c.uid.clone())),
        label: c.name.clone(),
        selected: showing.is_some() && camera_named == Some(&c.uid),
        enabled: true,
        detail: None,
    }));
    if !camera_source {
        cameras.push(Choice { action: Action::SetCamera(None), label: copy::camera_off_pick(), selected: showing.is_none(), enabled: true, detail: None });
    }
    let (camera_line, camera_settings) = if wants_camera && camera_denied {
        (Some(copy::camera_off()), Some(Button::new(Action::OpenCameraSettings, copy::open_system_settings(), Style::Link)))
    } else {
        (None, None)
    };

    let room = room_minutes(w);
    let out_of_room = room.is_some_and(|m| m < ROOM_FLOOR_MINUTES);
    let room_line = match room {
        Some(m) if m < ROOM_FLOOR_MINUTES => Some(copy::out_of_room()),
        Some(m) if m < ROOM_LOW_MINUTES => Some(copy::room_left(m)),
        _ => None,
    };
    // An update applying quits the app any moment now: no take opens under it (law 4). A
    // space that can't take videos at all greys it too: a new video would have nowhere to
    // go (N34).
    let nowhere = super::members::start_greyed(w);
    // What the source needs from macOS: a display, screen recording; Camera, the camera —
    // a denied camera greys Start with its quiet line (N39).
    let source_allowed = match source {
        Some(SourceId::Display(_)) => screen,
        Some(SourceId::Camera) => !camera_denied,
        None => false,
    };
    let can_start = source_allowed && !out_of_room && !w.update.applying && !nowhere;

    let mut faces = Vec::new();
    if room_line.is_some() {
        faces.push("disk-low");
    }
    let status_line = status_line(w);
    if let Some(shot) = one_shot(w).filter(|s| status_line.as_deref() == Some(one_shot_line(w, *s).as_str())) {
        faces.push(match shot {
            OneShot::Updated => "updated",
            OneShot::Failed => "update-failed",
            OneShot::AiOn => "ai-on",
        });
    }
    if lost_line(w).is_some() {
        faces.push("take-lost");
    }
    let banner = banner(w);
    if banner.as_ref().is_some_and(|b| b.id == "updating-app") {
        faces.push("update-ready");
    }
    if banner.as_ref().is_some_and(|b| matches!(b.id, "ai-turning-on" | "ai-offer-failed")) {
        faces.push("ai-offer");
    }

    Popover {
        id: "config",
        faces,
        body: PopoverBody::Config(ConfigBody {
            record_label: copy::record(),
            sources,
            camera_label: copy::camera(),
            cameras,
            mic_label: copy::microphone(),
            mics,
            mic_line,
            mic_hint,
            quality_label: copy::quality(),
            qualities: [Quality::Normal, Quality::TwoK, Quality::FourK]
                .into_iter()
                .map(|q| Choice { action: Action::SetQuality(q), label: copy::quality_name(q), selected: w.prefs.quality == q, enabled: true, detail: None })
                .collect(),
            camera_line,
            camera_settings,
            start: Button { note: super::members::start_note(w), ..Button::new(Action::Start, copy::start(), Style::Filled).enabled(can_start) },
            hint: copy::countdown_hint(),
            room_line,
            footer: vec![
                Button::new(Action::OpenFeedback, copy::send_feedback(), Style::Quiet),
                Button::new(Action::OpenLibrary, copy::library(), Style::Quiet),
                Button::new(Action::OpenSettings, copy::settings(), Style::Quiet),
                Button::new(Action::Quit, copy::quit(), Style::Quiet),
            ],
        }),
        banner,
        status_tone: status_line.as_ref().map(|_| status_tone(w)),
        status_line,
        // A banner's own buttons are never the default (law 8): with Start greyed for a
        // space that can't take videos, Return does nothing.
        ret: (!nowhere).then_some(Action::Start),
        esc: Some(Action::Dismiss),
        activates: true,
        away_closes: false,
    }
}

/// One banner, by N38's priority: a saved take → the space can't take videos (in this
/// Mac's role's words) → recording switched off → an update.
fn banner(w: &World) -> Option<Banner> {
    banner_saved(w)
        .or_else(|| banner_degraded(w))
        .or_else(|| super::members::banner(w))
        .or_else(|| banner_recording_off(w))
        .or_else(|| banner_update(w))
        .or_else(|| banner_ai(w))
}

/// The AI offer (N66; Bagrat, 2026-10-18: off by default, but visible), the slimmest and the
/// lowest: on the Mac that holds the setup key while the space's AI is off. It says what
/// shroom can do, never the cost (Turn on…'s confirm says that); it says it's turning on
/// while the page is re-wired, and that it didn't go through when it didn't (law 12).
fn banner_ai(w: &World) -> Option<Banner> {
    if !w.space.token_holder() || w.space.ai != Some(false) {
        return None;
    }
    let ask = || vec![Button::new(Action::AiAsk, copy::turn_on_ask(), Style::Filled), Button::new(Action::AiNotNow, copy::not_now(), Style::Link)];
    if w.space.ai_turn == Some(AiTurn::Turning) {
        // Re-wiring now; or a try that didn't go through, said in place of the line until
        // the next (quiet, or the person's own Turn on…).
        if w.inflight.ops.contains_key(&Op::Ai) {
            return Some(Banner { id: "ai-turning-on", tone: Tone::Frost, line: copy::ai_turning_on(), buttons: vec![] });
        }
        if w.inflight.failures.contains_key(&Op::Ai) {
            return Some(Banner { id: "ai-offer-failed", tone: Tone::Frost, line: copy::ai_failed(), buttons: ask() });
        }
    }
    (!w.once.ai_offer_answered).then(|| Banner { id: "ai-offer", tone: Tone::Frost, line: copy::ai_banner(), buttons: ask() })
}

/// Kind 2: the space can't take videos — on the token-holding Mac, `space-degraded` (N42).
/// Start stays live beneath it (capture is local).
fn banner_degraded(w: &World) -> Option<Banner> {
    super::settings::degraded(w).then(|| Banner {
        id: "space-degraded",
        tone: Tone::Amber,
        line: copy::space_degraded(),
        buttons: vec![Button::new(Action::ReconnectSpace, copy::reconnect_space(), Style::Filled)],
    })
}

/// Kind 4, the slimmest: an update, downloaded and verified (N37).
fn banner_update(w: &World) -> Option<Banner> {
    let v = super::update_ready(w)?;
    Some(if w.update.applying {
        Banner { id: "updating-app", tone: Tone::Frost, line: copy::updating(), buttons: vec![] }
    } else {
        Banner {
            id: "update-ready",
            tone: Tone::Frost,
            line: copy::update_ready(v),
            buttons: vec![Button::new(Action::Update, copy::update(), Style::Filled), Button::new(Action::WhatsNew, copy::whats_new(), Style::Link)],
        }
    })
}

fn banner_saved(w: &World) -> Option<Banner> {
    if let Some(take) = w.saved.first() {
        return Some(Banner {
            id: "recovered-take",
            tone: Tone::Amber,
            line: copy::recovered(&copy::length(take.length_ms)),
            buttons: vec![
                get_link(w),
                Button::new(Action::DiscardSaved, copy::discard_ellipsis(), Style::RedText),
            ],
        });
    }
    None
}

/// A saved take's Get link — greyed while an update applies: the app quits any moment now,
/// so no take opens under it (law 4).
fn get_link(w: &World) -> Button {
    let b = Button::new(Action::GetLink, copy::get_link(), Style::Filled);
    if w.update.applying { Button { enabled: false, note: Some(copy::updating()), ..b } } else { b }
}

fn banner_recording_off(w: &World) -> Option<Banner> {
    w.mac.screen.is(&false).then(|| Banner {
        id: "recording-off",
        tone: Tone::Amber,
        line: copy::recording_off(),
        buttons: vec![Button::new(Action::TurnBackOn, copy::turn_back_on(), Style::Filled)],
    })
}

/// The one-shot status lines after the upload truth (N38): the update landed, or didn't.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneShot {
    Updated,
    Failed,
    /// The AI banner's Turn on went through (N66).
    AiOn,
}

fn one_shot(w: &World) -> Option<OneShot> {
    if super::updated_pending(w).is_some() {
        Some(OneShot::Updated)
    } else if w.update.failed {
        Some(OneShot::Failed)
    } else if w.space.ai_turn == Some(AiTurn::On) {
        Some(OneShot::AiOn)
    } else {
        None
    }
}

/// The one-shot line `config` shows right now, if any — once shown, it's behind us when
/// the popover closes.
pub fn one_shot_showing(w: &World) -> Option<OneShot> {
    let p = derive(w)?;
    let shot = one_shot(w)?;
    (p.id == "config" && p.status_line.as_deref() == Some(one_shot_line(w, shot).as_str())).then_some(shot)
}

fn one_shot_line(w: &World, shot: OneShot) -> String {
    match shot {
        OneShot::Updated => copy::updated(super::updated_pending(w).unwrap_or_default()),
        OneShot::Failed => copy::update_failed(),
        OneShot::AiOn => copy::ai_on_said(),
    }
}

/// The status line's dot (DEVIATIONS #45): uploads moving, waiting on something, or done.
fn status_tone(w: &World) -> crate::scene::StatusTone {
    use crate::scene::StatusTone;
    if lost_line(w).is_some() {
        return StatusTone::Waiting;
    }
    if !w.queue.is_empty() && !super::members::stranded_by_space(w) {
        return if w.online() == Some(false) { StatusTone::Waiting } else { StatusTone::Busy };
    }
    // As `status_line` reads it: the one-shot — only a failed update waits.
    if one_shot(w) == Some(OneShot::Failed) { StatusTone::Waiting } else { StatusTone::Done }
}

/// One status line — a fact: uploads in flight, waiting, or an update just done. A drained
/// queue says nothing: the line just goes (DEVIATIONS #52).
fn status_line(w: &World) -> Option<String> {
    // A take that died with nothing to keep comes first: it's what just happened (#53).
    if let Some(line) = lost_line(w) {
        return Some(line);
    }
    // Nothing is uploading while the space can't take videos: the banner says so.
    if !w.queue.is_empty() && !super::members::stranded_by_space(w) {
        if w.online() == Some(false) {
            return Some(copy::waiting_connection());
        }
        let (done, total) = w.queue.iter().fold((0u64, 0u64), |(d, t), q| (d + q.done as u64, t + q.total as u64));
        let pct = if total == 0 { 0 } else { (done * 100 / total) as u32 };
        return Some(if w.queue.len() == 1 { copy::uploading_one(pct) } else { copy::uploading_many(w.queue.len() as u64, pct) });
    }
    one_shot(w).map(|s| one_shot_line(w, s))
}

/// The line about a take capture let go of with nothing to keep (DEVIATIONS #53).
fn lost_line(w: &World) -> Option<String> {
    match w.session.take_lost {
        Some(TakeLost::Said { camera }) => Some(copy::nothing_recorded(camera)),
        _ => None,
    }
}

fn paused(w: &World, cause: Cause, length_ms: u64) -> Popover {
    let resume_ok = resume_ok(w, cause);
    let (face, context, sub, ret) = match cause {
        Cause::User => (None, None, None, Some(Action::Resume)),
        Cause::DisplayLost => (Some("display-lost"), Some(copy::display_lost()), None, None),
        Cause::CameraLost => (Some("camera-lost"), Some(copy::camera_lost()), None, None),
        Cause::MicLost => {
            let fallback = resume_mic(w).and_then(|uid| w.mac.mics.get()?.iter().find(|m| m.uid == uid).map(|m| m.name.clone()));
            let sub = Some(match fallback {
                Some(name) => copy::mic_fallback(&name),
                None => copy::mic_none(),
            });
            (Some("mic-lost"), Some(copy::mic_lost()), sub, Some(Action::Resume))
        }
        Cause::Asleep => (Some("mac-away"), Some(copy::slept()), None, None),
        Cause::Locked => (Some("mac-away"), Some(copy::locked()), None, None),
        Cause::MicRevoked => (Some("mic-permission-lost"), Some(copy::mic_permission_lost()), None, Some(Action::Resume)),
    };
    Popover {
        id: "paused",
        faces: face.into_iter().collect(),
        body: PopoverBody::Paused(PausedBody {
            title: copy::paused_title(&copy::length(length_ms)),
            context,
            sub,
            resume: Button::new(Action::Resume, copy::resume(), Style::Filled).enabled(resume_ok),
            restart: Button::new(Action::Restart, copy::restart(), Style::Plain),
            stop: Button::new(Action::Stop, copy::stop(), Style::Plain),
            // N64: at a pause you made; the auto-pause faces stay as they were. It says Marker
            // dropped once one is dropped at this pause, and only then; a question open in a
            // window greys it in its own words (B199).
            marker: (cause == Cause::User).then(|| {
                let dropped = w.take.as_ref().is_some_and(|t| t.marked_at_pause);
                let label = if dropped { copy::marker_dropped() } else { copy::drop_marker() };
                Button::new(Action::DropMarker, label, Style::Plain).enabled(marker_offered(w))
            }),
            discard: Button::new(Action::Discard, copy::discard_ellipsis(), Style::RedText),
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret,
        esc: Some(Action::Dismiss),
        activates: true,
        away_closes: false,
    }
}

/// Drop a marker is live (N64): a pause you made, no marker dropped at it yet, and no
/// question open over it — the paused popover's row, and what ⌥⌘M does while paused.
pub fn marker_offered(w: &World) -> bool {
    w.session.confirm.is_none() && w.take.as_ref().is_some_and(|t| t.phase == Phase::Paused(Cause::User) && !t.marked_at_pause)
}

/// Whether the paused popover's Resume is live: the display, camera or Mac it waits for is
/// back (the paused faces); the microphone faces resume with the fallback (N39).
pub fn resume_ok(w: &World, cause: Cause) -> bool {
    match cause {
        Cause::DisplayLost | Cause::CameraLost => w.take.as_ref().is_some_and(|t| source_present(w, t)),
        Cause::Asleep | Cause::Locked => w.mac.os.get().is_none_or(|o| o.present()),
        Cause::User | Cause::MicLost | Cause::MicRevoked => true,
    }
}

/// The take's source is here: its display, or — Camera as the source — the very camera it
/// records (N39 `camera-lost`: another camera coming or going changes nothing).
pub fn source_present(w: &World, t: &crate::world::OpenTake) -> bool {
    match t.source {
        SourceId::Display(id) => w.mac.displays.get().is_some_and(|ds| ds.iter().any(|d| d.id == id)),
        SourceId::Camera => match &t.camera {
            Some(uid) => w.mac.cameras.get().is_some_and(|cs| cs.iter().any(|c| &c.uid == uid)),
            None => selected_camera(w).is_some(),
        },
    }
}

/// A display's name in the source picker: its own, or — beside a twin of the same name —
/// its place (“LG UltraFine (left)”), numbered when place can't tell them apart: three or
/// more, or two that share a left edge (N39).
fn display_label(displays: &[crate::world::Display], d: &crate::world::Display) -> String {
    let mut twins: Vec<&crate::world::Display> = displays.iter().filter(|o| o.name == d.name).collect();
    match twins.len() {
        1 => copy::display_name(&d.name, None),
        2 if twins[0].x != twins[1].x => {
            let left = twins.iter().all(|o| o.x >= d.x);
            copy::display_name(&d.name, Some(if left { "left" } else { "right" }))
        }
        _ => {
            twins.sort_by_key(|o| (o.x, o.y, o.id));
            let n = twins.iter().position(|o| o.id == d.id).unwrap_or(0) + 1;
            copy::display_numbered(&d.name, n as u64)
        }
    }
}

fn discard_question(len: &str) -> Popover {
    Popover {
        id: "discard-confirm",
        faces: vec![],
        body: PopoverBody::Question(QuestionBody {
            title: copy::toss_title(),
            line: copy::toss_so_far(len),
            buttons: vec![
                Button::new(Action::Back, copy::back(), Style::Plain),
                Button::new(Action::RestartInstead, copy::restart_instead(), Style::Plain),
                Button::new(Action::ConfirmDiscard, copy::discard(), Style::RedOutline),
            ],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

/// Restart's question (the canvas's, DEVIATIONS #44): it replaces the take, so no Return
/// default (law 9); Esc is Back; the count it starts is still Restart's undo (N39).
fn restart_question(len: &str) -> Popover {
    Popover {
        id: "restart-confirm",
        faces: vec![],
        body: PopoverBody::Question(QuestionBody {
            title: copy::restart_title(),
            line: copy::restart_line(len),
            buttons: vec![
                Button::new(Action::Back, copy::back(), Style::Plain),
                Button::new(Action::ConfirmRestart, copy::restart(), Style::RedOutline),
            ],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

fn saved_discard_question(len: &str) -> Popover {
    Popover {
        id: "discard-confirm",
        faces: vec!["discard-confirm-saved-take"],
        body: PopoverBody::Question(QuestionBody {
            title: copy::toss_title(),
            line: copy::toss_saved(len),
            buttons: vec![
                Button::new(Action::Back, copy::back(), Style::Plain),
                Button::new(Action::ConfirmDiscardSaved, copy::discard(), Style::RedOutline),
            ],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

fn stranded_discard_question(length_ms: Option<u64>) -> Popover {
    Popover {
        id: "discard-confirm",
        faces: vec!["discard-confirm-saved-take"],
        body: PopoverBody::Question(QuestionBody {
            title: copy::toss_title(),
            line: length_ms.map(|l| copy::toss_saved(&copy::length(l))).unwrap_or_else(copy::toss_unknown_length),
            buttons: vec![
                Button::new(Action::Back, copy::back(), Style::Plain),
                Button::new(Action::ConfirmDiscardRow, copy::discard(), Style::RedOutline),
            ],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

/// The AI banner's Turn on… (N66): what turning it on does and costs, said once before it
/// costs anything. Like every question here, it takes a click, not Return; Back and Esc go
/// back to the banner.
fn ai_question() -> Popover {
    Popover {
        id: "ai-confirm",
        faces: vec![],
        body: PopoverBody::Question(QuestionBody {
            title: copy::ai_confirm_title(),
            line: copy::ai_cost(),
            buttons: vec![Button::new(Action::Back, copy::back(), Style::Plain), Button::new(Action::AiOn, copy::turn_on(), Style::Filled)],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

fn quit_question(w: &World) -> Popover {
    Popover {
        id: "quit-confirm",
        faces: vec![],
        body: PopoverBody::Question(QuestionBody {
            title: copy::quit_uploading(w.queue.len() as u64),
            line: copy::quit_line(),
            buttons: vec![
                Button::new(Action::Back, copy::back(), Style::Plain),
                Button::new(Action::QuitAnyway, copy::quit_anyway(), Style::Plain),
            ],
        }),
        banner: None,
        status_line: None,
        status_tone: None,
        ret: None,
        esc: Some(Action::Back),
        activates: true,
        away_closes: false,
    }
}

fn stopped(w: &World, stop: &StopOutcome) -> Option<Popover> {
    Some(match stop {
        StopOutcome::Offline(rec) => {
            let copied = matches!(&w.session.copied, Some((crate::world::CopyTarget::Link(r), _)) if r == rec);
            Popover {
                id: "stopped-offline",
                faces: vec![],
                body: PopoverBody::Stopped(StoppedBody {
                    title: Some(copy::link_ready()),
                    line: copy::offline_line(),
                    link: w.space.identity.as_ref().map(|s| format!("{}/{}", s.host, rec.0)),
                    buttons: vec![Button::new(Action::CopyLink, if copied { copy::copied() } else { copy::copy_link() }, Style::Filled)],
                }),
                banner: None,
                status_line: None,
                status_tone: None,
                ret: Some(Action::CopyLink),
                esc: Some(Action::Dismiss),
                activates: true,
                away_closes: false,
            }
        }
        // The space can't take this stop (N34): the take is safe here, a link that waits.
        // Owners also get [ Reconnect your space ] (r4): on an owner-class Mac whose space is
        // gone, the token row here (N33) — a removed owner Mac has nothing to reconnect to
        // (DEVIATIONS #21).
        StopOutcome::Stranded(_) => Popover {
            id: "stopped-stranded",
            faces: vec![],
            body: PopoverBody::Stopped(StoppedBody {
                title: Some(copy::stranded_title()),
                line: super::members::stranded_reason(w).unwrap_or_default(),
                link: None,
                buttons: {
                    let dir = match &w.session.last_stop {
                        Some(crate::world::StopOutcome::Stranded(d)) => Some(d),
                        _ => None,
                    };
                    let mut b = vec![super::members::save_button(w, Action::SaveToMovies, dir.as_slice(), copy::save_to_movies(), Style::Filled)];
                    if let Some(a) = super::members::reconnect_action(w) {
                        b.push(Button::new(a, copy::reconnect_space(), Style::Plain));
                    }
                    b
                },
            }),
            banner: None,
            status_line: None,
            status_tone: None,
            ret: Some(Action::SaveToMovies),
            esc: Some(Action::Dismiss),
            activates: true,
            away_closes: false,
        },
        StopOutcome::AutoStopped(dir) => {
            let take = w.saved.iter().find(|s| &s.dir == dir)?;
            let len = copy::length(take.length_ms);
            let (id, line) = match take.cause {
                SavedCause::ScreenOff => ("permission-lost", copy::screen_stopped(&len)),
                _ => ("disk-full-stop", copy::disk_stopped(&len)),
            };
            Popover {
                id,
                faces: vec![],
                body: PopoverBody::Stopped(StoppedBody {
                    title: None,
                    line,
                    link: None,
                    buttons: vec![get_link(w), Button::new(Action::DiscardSaved, copy::discard_ellipsis(), Style::RedText)],
                }),
                banner: None,
                status_line: None,
                status_tone: None,
                ret: Some(Action::GetLink),
                esc: Some(Action::Dismiss),
                activates: true,
                away_closes: false,
            }
        }
    })
}
