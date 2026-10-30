//! The mushroom: its face and what a click means — "the click is the door" (law 5),
//! decided here and nowhere else.

use crate::scene::{Click, Face, Tray};
use crate::world::{Phase, WindowKind, World};

pub fn derive(w: &World) -> Tray {
    Tray { state: state(w), face: face(w), click: click(w), right_menu: (!w.take_open()).then(crate::copy::quit), shortcut: shortcut(w) }
}

fn state(w: &World) -> Option<&'static str> {
    match w.take.as_ref().map(|t| t.phase) {
        Some(Phase::Starting | Phase::Recording | Phase::Pausing(_) | Phase::Resuming) => Some("recording"),
        Some(_) => None,
        // Both idle states are tray-only life: the onboarding window is closed.
        // An agent's setup has no window (N89): the tray's own idle states.
        None if w.session.windows.get(&WindowKind::Onboarding).is_some_and(|d| !d.headless) => None,
        // A space whose chapter isn't done on this Mac is idle-ready too (its entry is
        // "provisioned, no take, the window closed"); only its click differs (bend 5).
        None if w.provisioned() => Some("idle-ready"),
        None => Some("idle-unprovisioned"),
    }
}

/// Priority: the recording faces, then amber, then Frost, then plain (N38).
fn face(w: &World) -> Face {
    if let Some(t) = &w.take {
        return match t.phase {
            Phase::Paused(_) | Phase::Finalizing(_) => Face::PauseBars,
            Phase::Starting | Phase::Recording | Phase::Pausing(_) | Phase::Resuming => Face::Red,
        };
    }
    if needs_you(w) {
        Face::Amber
    } else if super::update_ready(w).is_some() {
        // The lowest face: something new from us (N37).
        Face::Frost
    } else {
        Face::Plain
    }
}

/// Something needs you: a banner of kinds 1–3 waits (N38) — a saved take, the space
/// unable to take videos (in any role's words, N34), or screen recording switched off.
pub fn needs_you(w: &World) -> bool {
    !w.saved.is_empty()
        || super::settings::degraded(w)
        || super::members::life(w).is_some()
        || (w.provisioned() && w.row_six_done() && w.mac.screen.is(&false))
}

fn click(w: &World) -> Click {
    match &w.session.arming {
        Some(a) if !a.fired => return Click::CancelCountdown,
        // The count fired and capture is starting (the recorder hasn't answered): the take's
        // in-between beat, like `Starting` — a click opens nothing over it (law 4, DEVIATIONS
        // #3; DECISIONS B89: the popover/tray enumeration found config opening here).
        Some(_) => return Click::Nothing,
        None => {}
    }
    if let Some(t) = &w.take {
        return match t.phase {
            Phase::Recording => Click::Pause,
            Phase::Paused(_) => Click::ShowPaused,
            Phase::Starting | Phase::Pausing(_) | Phase::Resuming | Phase::Finalizing(_) => Click::Nothing,
        };
    }
    if !w.provisioned() || !w.row_six_done() {
        return Click::OpenOnboarding;
    }
    Click::OpenPopover
}

/// ⌥⌘P is held while a take is recording or paused (N54), not through a count or a finish.
fn shortcut(w: &World) -> bool {
    w.session.arming.is_none()
        && w.take.as_ref().is_some_and(|t| matches!(t.phase, Phase::Recording | Phase::Pausing(_) | Phase::Paused(_) | Phase::Resuming))
}
