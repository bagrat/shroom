//! The status seam's projection (ARCHITECTURE §11.2, DECISIONS B80): where this Mac stands,
//! as the words a reader outside the app can count on — the installer's mid-take refusal
//! (N9) and a guiding agent (Region 6's overlay). Pure, like the rest of `derive`; the
//! runner adds what only the process knows (its pid, its version, the grants as macOS says
//! them now) and writes `status.json`.
//!
//! Every field is a `&'static str` from a fixed vocabulary or a bool: nothing the person
//! typed, no host, no invite, no code, no secret can reach it (law 13, by construction).

use serde::Serialize;

use crate::world::{Access, InviteKind, Phase, Role, World};

use super::members::{self, Life};
use super::onboarding;

/// The seam's view of this Mac, from the World alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Posture {
    /// A take is open — from the countdown to the end of the stop (law 4's "a take").
    pub take_open: bool,
    /// `none` · `countdown` · `recording` · `paused` · `stopping`.
    pub take: &'static str,
    /// Where onboarding stands, by its state id (`onboarding::At::id`): `welcome`, the
    /// plan's states, the permissions chapter's, the invite window's, `all-set`.
    pub setup: &'static str,
    /// This Mac's permissions chapter (row six): `done` once the warm-up ran clean, else
    /// `ahead`.
    pub chapter: &'static str,
    /// The grants as the core last heard them (`None` before the first report).
    pub grants: Option<Grants>,
    /// `none` (no space on this Mac) · `token-holder` · `owner-class` · `member`.
    pub role: &'static str,
    /// The invite window's state id while this Mac holds an invite (`member-welcome`,
    /// `joining`, `join-waiting`, `join-expired`, `join-used`, `join-space-missing`,
    /// `join-already-here`, `join-switch-confirm`), else `none`.
    pub invite: &'static str,
    /// The held invite's kind once its space said: `member` · `own-mac`; else `none`.
    pub invite_kind: &'static str,
    /// What this Mac's space last said about it (N34): `fine` · `removed` ·
    /// `needs-owner` · `gone` · `needs-update`. Always `fine` on the token's Mac (it
    /// repairs its space instead, N42) and with no space.
    pub standing: &'static str,
}

/// The three grants, in the seam's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Grants {
    /// `granted` · `not-granted` (macOS says no more than that about screen recording).
    pub screen: &'static str,
    /// `granted` · `denied` · `restricted` · `not-asked`.
    pub mic: &'static str,
    pub camera: &'static str,
}

impl Grants {
    pub fn from(screen: bool, mic: Access, camera: Access) -> Grants {
        Grants { screen: if screen { "granted" } else { "not-granted" }, mic: access(mic), camera: access(camera) }
    }
}

pub fn access(a: Access) -> &'static str {
    match a {
        Access::Granted => "granted",
        Access::Denied => "denied",
        Access::Restricted => "restricted",
        Access::NotAsked => "not-asked",
    }
}

fn take(w: &World) -> &'static str {
    if let Some(t) = &w.take {
        return match t.phase {
            Phase::Starting | Phase::Recording | Phase::Resuming => "recording",
            Phase::Pausing(_) | Phase::Paused(_) => "paused",
            Phase::Finalizing(_) => "stopping",
        };
    }
    if w.session.arming.is_some() { "countdown" } else { "none" }
}

pub fn posture(w: &World) -> Posture {
    let grants = match (w.mac.screen.get(), w.mac.mic.get(), w.mac.camera.get()) {
        (Some(s), Some(m), Some(c)) => Some(Grants::from(*s, *m, *c)),
        _ => None,
    };
    let role = match w.space.role() {
        None => "none",
        Some(Role::TokenHolder) => "token-holder",
        Some(Role::OwnerClass) => "owner-class",
        Some(Role::Member) => "member",
    };
    let invite_kind = match w.members.invite.as_ref().and_then(|i| i.peek.get()) {
        Some(crate::event::PeekAnswer::Pending { kind, .. } | crate::event::PeekAnswer::Expired { kind, .. } | crate::event::PeekAnswer::Used { kind, .. }) => match kind {
            InviteKind::Member => "member",
            InviteKind::OwnMac => "own-mac",
        },
        _ => "none",
    };
    let standing = match members::life(w) {
        None => "fine",
        Some(Life::Revoked) => "removed",
        Some(Life::NeedsOwner) => "needs-owner",
        Some(Life::Gone) => "gone",
        Some(Life::Behind) => "needs-update",
    };
    Posture {
        take_open: w.take_open(),
        take: take(w),
        setup: onboarding::at(w).id(),
        chapter: if w.row_six_done() { "done" } else { "ahead" },
        grants,
        role,
        invite: members::invite_at(w).map_or("none", |a| a.id()),
        invite_kind,
        standing,
    }
}
