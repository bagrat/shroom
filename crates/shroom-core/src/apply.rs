//! `apply`: the only writer of the World. Fact arms copy reports in; input arms resolve
//! their meaning through the current Scene; intent arms record what the person asked
//! for. No arm names a screen (law 1).

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::copy;
use crate::derive::onboarding::{self, At};
use crate::derive::settings::{self as settings_view, SpaceAt};
use crate::derive::{COPIED_MS, COUNTDOWN_MS, LIT_MS, OneShot, UNDO_MS, derive, library as library_view};
use crate::effect::{CloudCmd, EditCmd, Effect, LibCmd, MemberCmd, MoviesCopy, Perm, PersistOp, PublicUrl, RecCmd, ShellCmd, TakeCmd, UpCmd, UpdCmd};
use crate::event::{
    ClaimFail, CloudNews, EditStep, EditorNews, Event, Field, JoinAnswer, Key, LibraryNews, MemberNews, OsNews, PeekAnswer, QuitVia, RecNews, ReportNews,
    SecretField, Shape, StandingAnswer, Surface, TokenFail, UpdateNews, UploadNews, WireFail,
};
use crate::report::{Kind, Post};
use crate::scene::{Action, Click};
use crate::time::{Fact, Mono};
use crate::world::{
    Access, AiTurn, Arming, Blocked, Born, Cause, Confirm, CopyOwed, CopyTarget, Door, EditLink, EditLinkNo, EditorFields, EditorId, EditorMedia, EditorSave, FailClass, Failure, Fields, Fork, Invite,
    JoinRefusal, Landed, LaunchKind, Leaving, RecId,
    LibraryFields, LibraryOps, Members, OpenTake, Op, PageOpen, Paste, Phase, Progress, QuickProbe, Reach, RelaunchWhy, Removal, ReportFields, FeedbackFields, Restore, Role, SavedCause,
    SavedTake, SecretId, SettingsFields, SettingsTab, SourceId, Space, SpaceIdentity, StopOutcome, Strike, Sweep, TakeDir, TakeLost, TokenVerdict,
    UpdateOffer, Why, WindowKind, WorkerFact, World,
};

/// How long a `restore.json` is honored after it was written (DEVIATIONS #8, #9).
pub const RESTORE_MS: i64 = 120_000;

/// Which arm ran, and the machine's edge it realizes (spec/machine.json; for the trace and the edge tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arm {
    pub name: &'static str,
    pub edge: Option<(&'static str, &'static str)>,
}

impl Arm {
    pub const fn named(name: &'static str) -> Arm {
        Arm { name, edge: None }
    }
    pub const DROPPED: Arm = Arm::named("dropped");
}

type Out = (Vec<Effect>, Arm);

pub fn apply(w: &mut World, event: Event) -> Out {
    // A one-shot status line (“Updated to … ✓”, the failed update) shows once: when the
    // popover that showed it closes, it's behind us.
    let shot = crate::derive::one_shot_showing(w);
    // So does the line about a take that kept nothing (DEVIATIONS #53), and the reason the
    // page's Edit in shroom opened nothing (N69).
    let lost_said = w.session.popover.is_some() && matches!(w.session.take_lost, Some(TakeLost::Said { .. }));
    let edit_said = w.session.popover.is_some() && matches!(w.session.fields.edit_link, Some(EditLink::Said(_)));
    let (mut fx, arm) = apply_event(w, event);
    if lost_said && w.session.popover.is_none() {
        w.session.take_lost = None;
    }
    if edit_said && w.session.popover.is_none() {
        w.session.fields.edit_link = None;
    }
    // AI on by default (N75; Bagrat, 2026-10-25): the setup Mac's popover, about to show “AI
    // features are turned on.”, turns it on — so it's said before anything is heard.
    if w.online() != Some(false) && crate::derive::popover::ai_default_due(w) {
        w.space.ai_turn = Some(AiTurn::TurningOn);
        fx.push(Effect::Cloud(CloudCmd::SetAi(true)));
    }
    // A press on the page's Edit in shroom held for a take goes on once it's over (law 4).
    if !w.take_open() && matches!(w.session.fields.edit_link, Some(EditLink::Waiting { asked: false, .. })) {
        if let Some(EditLink::Waiting { host, rec, .. }) = w.session.fields.edit_link.take() {
            fx.extend(edit_link(w, host, rec).0);
        }
    }
    if let Some(shot) = shot.filter(|_| w.session.popover.is_none()) {
        match shot {
            OneShot::Updated => {
                w.once.seen_version = Some(w.launch.version.clone());
                fx.push(Effect::Persist(PersistOp::Once(w.once.clone())));
            }
            OneShot::Failed => w.update.failed = false,
            OneShot::AiOff => w.space.ai_turn = None,
        }
    }
    (fx, arm)
}

/// The space's AI, settled on this Mac for good (N75): never turned on by default again,
/// and its banner never shown again here.
fn settle_ai(w: &mut World) -> Vec<Effect> {
    if w.once.ai_offer_answered {
        return vec![];
    }
    w.once.ai_offer_answered = true;
    vec![Effect::Persist(PersistOp::Once(w.once.clone()))]
}

fn apply_event(w: &mut World, event: Event) -> Out {
    match event {
        Event::Booted(disk) => {
            let disk = *disk;
            w.launch.kind = disk.launch;
            w.launch.version = disk.version;
            w.launch.booted = w.now;
            w.prefs = disk.prefs;
            w.once = disk.once;
            w.space.identity = disk.space;
            w.space.token = disk.token;
            w.space.device_key = disk.device_key;
            w.mac.primed = disk.primed;
            quick(w, &disk.quick);
            let mut fx = vec![Effect::Takes(TakeCmd::Rescan)];
            // The first version this Mac runs is the one it has seen; so is any version a
            // launch other than an `--updated` relaunch finds running (the install script's
            // manual update says nothing): only an in-app update says “Updated to … ✓” (N37).
            let acknowledged = w.once.seen_version.as_deref() == Some(w.launch.version.as_str());
            if !acknowledged && !w.launch.version.is_empty() && (w.once.seen_version.is_none() || disk.launch != LaunchKind::UpdateRelaunch) {
                w.once.seen_version = Some(w.launch.version.clone());
                fx.push(Effect::Persist(PersistOp::Once(w.once.clone())));
            }
            let fresh = |r: &Restore, why: RelaunchWhy| r.why == why && (0..=RESTORE_MS).contains(&(w.wall.0 - r.written.0));
            // A permission relaunch resumes the chapter (bend 9: macOS's own quit-and-reopen
            // is recognized by a fresh restore.json, since it looks like a user launch).
            let permission = disk.restore.as_ref().filter(|r| fresh(r, RelaunchWhy::Permission)).map(|r| r.window);
            let updated = disk.restore.as_ref().filter(|r| fresh(r, RelaunchWhy::Update)).map(|r| r.window);
            let mut arm = Arm::named("booted");
            match disk.launch {
                LaunchKind::PermissionRelaunch => arm = resume_chapter(w, permission.unwrap_or(WindowKind::Onboarding)),
                LaunchKind::User | LaunchKind::Install { join: false } if permission.is_some() => {
                    arm = resume_chapter(w, permission.unwrap_or(WindowKind::Onboarding))
                }
                // An update relaunch brings back the window that was in front (law 5,
                // N37) — without the keyboard…
                LaunchKind::UpdateRelaunch | LaunchKind::UpdateFailed => {
                    // Never the editor: its steps lived in the process that quit (N63).
                    if let Some(kind) = updated.filter(|k| !matches!(k, WindowKind::Editor(_))) {
                        w.session.windows.insert(kind, Door::unasked(w.now));
                    }
                    if disk.launch == LaunchKind::UpdateFailed {
                        // The running version is unchanged: say so once, and try again
                        // later — the next check, a few hours from now.
                        w.update.failed = true;
                        w.update.checked = Some(w.now);
                        w.update.tried = Some(w.wall);
                        arm = Arm { name: "update-failed", edge: Some(("updating-app", "update-failed")) };
                    } else {
                        arm = Arm { name: "updated", edge: Some(("updating-app", "updated")) };
                    }
                    // …and the popover opens by itself to say how it went, without the
                    // keyboard (Bagrat, 2026-10-15: “after update it is too silent,
                    // probably should open the popover after relaunch” — amends N41).
                    if derive(w).tray.click == Click::OpenPopover {
                        let door = Door::unasked(w.now);
                        w.session.popover = Some(door);
                        w.session.news = Some(door.opened);
                    }
                }
                // A launch the person did is a click (N41); login and update launches stay
                // silent — the install script's update too (N9) — and so does an install
                // carrying an invite (its window is the member lane's to open).
                LaunchKind::User | LaunchKind::Install { join: false } => {
                    let (more, a) = click(w, disk.quick.clicked_display);
                    fx.extend(more);
                    if matches!(disk.launch, LaunchKind::Install { .. }) && onboarding_at(w) == Some(At::Welcome) {
                        arm = Arm { name: "first-launch", edge: Some(("first-launch", "welcome")) };
                    } else {
                        arm = Arm { name: a.name, edge: None };
                    }
                }
                LaunchKind::LoginItem | LaunchKind::InstallUpdate | LaunchKind::Install { join: true } => {}
            }
            if disk.restore.is_some() {
                fx.push(Effect::Persist(PersistOp::Restore(None)));
            }
            (fx, arm)
        }
        Event::TrayClicked { right, quick: q } => {
            quick(w, &q);
            let at = q.clicked_display;
            // What the click's probe found about the take comes first (a grant switched off,
            // a device gone): that pause or stop is the answer, and its popover opens itself.
            if let Some(out) = guard_take(w) {
                return out;
            }
            // Right-click mid-take is a left click (N39): the pause while recording, the
            // paused popover while paused. Idle, it's the one-item Quit menu's (the shell's).
            if right && !w.take_open() {
                return (vec![], Arm::named("tray-right-click"));
            }
            click(w, at)
        }
        Event::Reopened(q) => {
            quick(w, &q);
            if let Some(out) = guard_take(w) {
                return out;
            }
            // A launch you did is a click (N41): from the display it came from (N56).
            click(w, q.clicked_display)
        }
        Event::Shortcut(q) => {
            // ⌥⌘P (N54): recording, the tray click's pause; paused, Resume. Nothing else.
            quick(w, &q);
            if let Some(out) = guard_take(w) {
                return out;
            }
            match w.take.as_ref().map(|t| t.phase) {
                _ if !derive(w).tray.shortcut => (vec![], Arm::DROPPED),
                Some(Phase::Recording) => click(w, None),
                Some(Phase::Paused(cause)) if w.session.confirm.is_none() && crate::derive::resume_ok(w, cause) => act(w, Action::Resume),
                _ => (vec![], Arm::DROPPED),
            }
        }
        Event::MarkerShortcut(q) => {
            // ⌥⌘M (N64), under ⌥⌘P's rule: recording, a marker at this moment — nothing on
            // screen answers, the take goes on; paused, Drop a marker where it's offered.
            quick(w, &q);
            if let Some(out) = guard_take(w) {
                return out;
            }
            if !derive(w).tray.shortcut {
                return (vec![], Arm::DROPPED);
            }
            match w.take.as_ref().map(|t| t.phase) {
                Some(Phase::Recording) => {
                    if let Some(t) = w.take.as_mut() {
                        t.markers += 1;
                    }
                    (vec![Effect::Recorder(RecCmd::Marker)], Arm::named("marker-shortcut"))
                }
                Some(Phase::Paused(_)) if crate::derive::marker_offered(w) => act(w, Action::DropMarker),
                _ => (vec![], Arm::DROPPED),
            }
        }
        Event::Act { surface, action, .. } => {
            if action != Action::Dismiss && !derive(w).offers(surface, &action) {
                return (vec![], Arm::DROPPED);
            }
            act_from(w, surface, action)
        }
        Event::Key { surface, key } => {
            let scene = derive(w);
            let action = match key {
                Key::Return => scene.ret(surface),
                Key::Esc => scene.esc(surface),
            };
            match action {
                Some(a) if a == Action::Dismiss || scene.offers(surface, &a) || surface == Surface::Countdown => act_from(w, surface, a),
                _ => (vec![], Arm::DROPPED),
            }
        }
        Event::PopoverDismissed => dismiss(w),
        Event::Cli { id, op } => cli(w, id, op),
        Event::SettingsLink(spot) => settings_link(w, spot),
        Event::Field { field: Field::LibrarySearch, value } => {
            if derive(w).library.is_none() {
                return (vec![], Arm::DROPPED);
            }
            w.session.fields.library.search = value;
            w.session.fields.library.search_edits += 1;
            (vec![], Arm::named("library-search"))
        }
        Event::Field { field: Field::InviteName, value } => {
            // The Members composer's field: it echoes only while the Members face shows.
            let members = w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::Members;
            if !members {
                return (vec![], Arm::DROPPED);
            }
            w.session.fields.settings.invite_name = value.chars().take(80).collect();
            w.session.fields.settings.invite_edits += 1;
            (vec![], Arm::named("invite-name-typed"))
        }
        Event::Field { field: field @ (Field::ReportWords | Field::ReportReplyTo), value } => {
            // The report sheet's fields: typed only while it shows and nothing is on its way.
            if w.session.confirm != Some(Confirm::Report { sending: false }) || !w.window_visible(WindowKind::Settings) {
                return (vec![], Arm::DROPPED);
            }
            let f = &mut w.session.fields.settings.report;
            if field == Field::ReportWords {
                f.words = value.chars().take(crate::copy::REPORT_WORDS_MAX).collect();
                f.words_edits += 1;
            } else {
                f.reply_to = value.chars().take(254).collect();
                f.reply_edits += 1;
            }
            (vec![], Arm::named("report-typed"))
        }
        Event::Field { field: field @ (Field::FeedbackWords | Field::FeedbackReplyTo), value } => {
            // Send feedback's fields: typed only while its form shows and nothing is on its way.
            let open = w.window_visible(WindowKind::Feedback);
            let f = &mut w.session.fields.feedback;
            if !open || f.sending || f.sent {
                return (vec![], Arm::DROPPED);
            }
            if field == Field::FeedbackWords {
                f.words = value.chars().take(crate::copy::REPORT_WORDS_MAX).collect();
                f.words_edits += 1;
            } else {
                f.reply_to = value.chars().take(254).collect();
                f.reply_edits += 1;
            }
            (vec![], Arm::named("feedback-typed"))
        }
        Event::Field { field: Field::FromName, value } => {
            // The name invites carry, while it's being changed on the Members face (B158).
            let members = w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::Members;
            let Some(typed) = w.session.fields.settings.from.as_mut().filter(|_| members) else { return (vec![], Arm::DROPPED) };
            *typed = value.chars().take(crate::copy::FROM_MAX).collect();
            w.session.fields.settings.from_edits += 1;
            (vec![], Arm::named("from-typed"))
        }
        Event::Field { field: Field::Title, value } => {
            let Some((_, typed)) = w.session.fields.library.renaming.as_mut() else { return (vec![], Arm::DROPPED) };
            *typed = value;
            w.session.fields.library.title_edits += 1;
            (vec![], Arm::named("title-typed"))
        }
        Event::Field { field: Field::Name, value } => {
            if !matches!(onboarding_at(w), Some(At::Name | At::NameTaken)) {
                return (vec![], Arm::DROPPED);
            }
            let name = copy::filter_name(&value);
            // The quiet line is for a character outside the set (N15); a name past the
            // DNS label's 63 characters is simply cut, silently (whole-spec audit #12).
            let filtered_out = copy::name_charset_dropped(&value);
            if filtered_out {
                w.session.fields.name_hint = Some(w.now);
            }
            w.session.fields.name = name;
            // Counted even when the guard gave the old value back: the window typed, and
            // the Scene that answers has to say so (DECISIONS B107).
            w.session.fields.name_edits += 1;
            // The charset guard is a note on the name row, not a move (N15): no edge either way.
            (vec![], Arm::named("name-typed"))
        }
        Event::Edit(id, step) => edit_step(w, id, step),
        Event::EditLink { host, rec } => edit_link(w, host, rec),
        Event::SecretPasted { field, id, shape } => pasted(w, field, id, shape),
        Event::WindowClosed(kind) => window_closed(w, kind),
        Event::WindowFocused(kind, q) => {
            quick(w, &q);
            let now = w.now;
            if let Some(d) = w.session.windows.get_mut(&kind) {
                d.entered = now;
            }
            if let Some(out) = guard_take(w) {
                return out;
            }
            (vec![], Arm::named("window-focused"))
        }
        Event::BubbleMoved(at) => {
            // Only a bubble on screen can be put somewhere.
            if derive(w).bubble.is_none() || w.prefs.bubble_at == Some(at) {
                return (vec![], Arm::DROPPED);
            }
            w.prefs.bubble_at = Some(at);
            (vec![persist_prefs(w)], Arm::named("bubble-moved"))
        }
        Event::PopoverKeyed => {
            if let Some(d) = w.session.popover.as_mut() {
                d.asked = true;
            }
            (vec![], Arm::named("popover-keyed"))
        }
        Event::QuitRequested(via) => quit_requested(w, via),
        Event::Os(news) => os(w, news),
        Event::Rec(news) => rec(w, news),
        Event::Scanned { saved, queue, removed } => {
            w.saved = saved;
            // A take another Mac's delete stopped stays “Removed from the space” — across a
            // relaunch too (its tombstone marker is on disk) — and never goes back in the
            // queue (a scan never un-tombstones).
            let mut kept: Vec<_> = std::mem::take(&mut w.removed).into_iter().filter(|r| !removed.iter().any(|x| x.dir == r.dir)).collect();
            kept.extend(removed);
            kept.sort_by(|a, b| a.dir.cmp(&b.dir));
            w.removed = kept;
            w.queue = queue.into_iter().filter(|q| !w.removed.iter().any(|r| r.dir == q.dir)).collect();
            (vec![], Arm::named("scanned"))
        }
        Event::Cloud(news) => cloud(w, news),
        Event::Upload(news) => upload(w, news),
        Event::Library(news) => library(w, news),
        Event::Update(news) => update_news(w, news),
        Event::Member(news) => member(w, news),
        Event::Report(news) => report(w, news),
        Event::Editor(news) => editor_news(w, news),
        Event::Failed { op, class } => failed(w, op, class),
        Event::Tick => tick(w),
    }
}

/// The local facts every entry brings with it (law 1: every entry re-probes).
fn quick(w: &mut World, q: &QuickProbe) {
    let at = w.now;
    w.mac.screen = Fact::known(q.screen, at);
    w.mac.mic = Fact::known(q.mic, at);
    w.mac.camera = Fact::known(q.camera, at);
    w.mac.displays = Fact::known(q.displays.clone(), at);
    w.mac.mics = Fact::known(q.mics.clone(), at);
    w.mac.cameras = Fact::known(q.cameras.clone(), at);
    w.mac.disk_free = Fact::known(q.disk_free, at);
    w.mac.os = Fact::known(q.os, at);
    w.mac.status_item_visible = q.status_item_visible;
    // The login item as macOS says now: a change made in System Settings shows on entry.
    if let Some(on) = q.login_item {
        w.mac.login_item = Fact::known(on, at);
    }
    if let Some(online) = q.online {
        w.net.online = Fact::known(online, at);
    }
}

/// The mushroom was clicked (or a launch you did) — in the menu bar of display `at`, when
/// the shell could tell: open the one door the moment needs.
fn click(w: &mut World, at: Option<u32>) -> Out {
    match derive(w).tray.click {
        Click::OpenPopover => {
            if w.session.popover.is_some() {
                return dismiss(w);
            }
            // `config` opens on the clicked display's source (N56); mid-take the click is
            // the pause and moves nothing.
            if !w.take_open() && at.is_some() {
                w.display_pick.clicked = at;
            }
            w.session.popover = Some(Door::asked(w.now));
            (vec![], Arm::named("click-open-popover"))
        }
        Click::Pause => {
            pause(w, Cause::User);
            (vec![Effect::Recorder(RecCmd::Pause)], Arm { name: "click-pause", edge: Some(("recording", "paused")) })
        }
        Click::ShowPaused => {
            w.session.popover = Some(Door::asked(w.now));
            (vec![], Arm::named("click-show-paused"))
        }
        Click::CancelCountdown => {
            w.session.arming = None;
            w.session.popover = Some(Door::asked(w.now));
            (vec![Effect::Recorder(RecCmd::Cool)], Arm { name: "click-cancel-countdown", edge: Some(("countdown", "config")) })
        }
        Click::OpenOnboarding => {
            let now = w.now;
            // An open window comes forward (entered moves, so the plan re-probes); it keeps
            // the instant it opened (law 4's visibility).
            w.session
                .windows
                .entry(WindowKind::Onboarding)
                .and_modify(|d| {
                    d.entered = now;
                    d.asked = true;
                })
                .or_insert(Door::asked(now));
            let edge = (onboarding::at(w) == At::Welcome).then_some(("idle-unprovisioned", "welcome"));
            (vec![], Arm { name: "click-open-onboarding", edge })
        }
        Click::Nothing => (vec![], Arm::named("click-nothing")),
    }
}

fn dismiss(w: &mut World) -> Out {
    w.session.popover = None;
    // The popover's own questions go with it; a window's sheet is that window's.
    let stranded_question = matches!(&w.session.confirm, Some(Confirm::DiscardRow(d)) if w.queue.iter().any(|q| &q.dir == d));
    if matches!(w.session.confirm, Some(Confirm::Discard | Confirm::DiscardSaved(_) | Confirm::Quit)) || stranded_question {
        w.session.confirm = None;
    }
    w.session.last_stop = None;
    w.popover_closed = Some(w.now);
    (vec![], Arm::named("popover-dismissed"))
}

fn persist_prefs(w: &World) -> Effect {
    Effect::Persist(PersistOp::Prefs(w.prefs.clone()))
}

/// The saved take an action means: the one a stop popover is about, else the oldest.
fn saved_target(w: &World) -> Option<TakeDir> {
    if let Some(StopOutcome::AutoStopped(dir) | StopOutcome::Stranded(dir)) = &w.session.last_stop {
        return Some(dir.clone());
    }
    w.saved.first().map(|s| s.dir.clone())
}

/// Save to Movies for these takes: each named (its title, or “shroom recording — Aug 21”
/// in this Mac's calendar, N34), and held as saving until the lane answers — no exit deletes
/// a take whose copy is under way (law 2, DECISIONS B54). A take already saving isn't asked
/// twice.
fn save_to_movies(w: &mut World, dirs: Vec<TakeDir>) -> Vec<Effect> {
    let copies: Vec<MoviesCopy> = dirs
        .into_iter()
        .filter(|d| !w.members.saving.contains(d))
        .map(|dir| MoviesCopy { name: crate::derive::members::movies_name(w, &dir), dir })
        .collect();
    if copies.is_empty() {
        return vec![];
    }
    for c in &copies {
        w.members.saving.insert(c.dir.clone());
        w.members.save_failed.remove(&c.dir);
    }
    vec![Effect::Takes(TakeCmd::SaveToMovies(copies))]
}

fn saving(fx: Vec<Effect>, name: &'static str) -> Out {
    if fx.is_empty() { (fx, Arm::DROPPED) } else { (fx, Arm::named(name)) }
}

/// What a space change keeps of Region 6's facts: the takes' own (saved to Movies, a copy
/// under way or failed, a switch's takes waiting on theirs).
fn carried(w: &mut World) -> Members {
    Members {
        movies: std::mem::take(&mut w.members.movies),
        saving: std::mem::take(&mut w.members.saving),
        save_failed: std::mem::take(&mut w.members.save_failed),
        toss: std::mem::take(&mut w.members.toss),
        ..Members::default()
    }
}

/// The count began: the microphone the count will fire capture with opens now (B173).
fn warm(w: &World) -> Effect {
    Effect::Recorder(RecCmd::Warm { mic: crate::derive::popover_selected_mic(w) })
}

fn pause(w: &mut World, cause: Cause) {
    if let Some(t) = w.take.as_mut() {
        t.phase = Phase::Pausing(cause);
        t.since = w.now;
    }
}

/// The `shroom` command's ask (spec N62, DECISIONS B216): each op is exactly what its click
/// does — Start as the popover's (its countdown, its permission beat), pause as the tray's,
/// Resume and Stop as the paused popover's — and only while the person has turned on
/// General's “Let it start and stop recordings”. Answered once, always, with a code (the
/// command says the words): the app holds the connection until it is.
fn cli(w: &mut World, id: u64, op: crate::event::CliOp) -> Out {
    use crate::event::CliOp;
    let face = |w: &World| -> &'static str {
        match (w.session.arming.as_ref(), w.take.as_ref().map(|t| t.phase)) {
            (Some(a), _) if !a.fired => "countdown",
            // The count's end fired (at once, without the count): capture is beginning.
            (Some(_), _) => "recording",
            (_, Some(Phase::Paused(_) | Phase::Pausing(_))) => "paused",
            (_, Some(Phase::Finalizing(_))) => "stopping",
            (_, Some(_)) => "recording",
            _ => "none",
        }
    };
    let answer = |w: &World, error: Option<&'static str>, link: Option<String>| {
        Effect::Cli(crate::effect::CliAnswer { id, ok: error.is_none(), take: face(w), error, link })
    };
    let refuse = |w: &World, code: &'static str| (vec![answer(w, Some(code), None)], Arm::named("cli-refused"));
    // Set up first: the switch lives in Settings, and Settings needs a space (B160) — so
    // the refusal's link always has a Settings to open (B216).
    if w.space.identity.is_none() {
        return refuse(w, "not_set_up");
    }
    // `shroom edit` (N72) changes a video's edit itself, through the space's page; it asks
    // here first, and after it saved. No recording, so no switch to ask. While that video is
    // open in an editor window, that window's Done is the edit: the command changes nothing,
    // and nothing is owed.
    let editing = |w: &World, rec: &RecId| editor_of(w, rec).is_some();
    match op {
        CliOp::Editing { rec } if editing(w, &rec) => return refuse(w, "editor_open"),
        CliOp::Editing { .. } => return (vec![answer(w, None, None)], Arm::named("cli-editing")),
        CliOp::Copy { rec, .. } if editing(w, &rec) => return refuse(w, "editor_open"),
        // Its final video is owed, as after the editor's Done: cut when the lane is free, one
        // at a time and never over a take (`settle::due_edit_copy`), the video coming from
        // the space first when this Mac hasn't got it. A newer revision already owed stays.
        CliOp::Copy { rec, rev } => {
            if !w.library_ops.copies.get(&rec).is_some_and(|c| c.rev >= rev) {
                w.library_ops.copies.insert(rec, CopyOwed { rev, asked: false });
            }
            return (vec![answer(w, None, None)], Arm::named("cli-copy-owed"));
        }
        _ => {}
    }
    if !w.prefs.agent_recording {
        return refuse(w, "recording_off");
    }
    let phase = w.take.as_ref().map(|t| t.phase);
    // Between two steps (pausing, resuming, stopping): ask again in a moment.
    if matches!(phase, Some(Phase::Pausing(_) | Phase::Resuming | Phase::Finalizing(_))) {
        return refuse(w, "busy");
    }
    // The count, and the recorder starting after it: the take is on its way.
    let coming = w.session.arming.is_some() || matches!(phase, Some(Phase::Starting));
    let stop = op == CliOp::Stop;
    let (mut fx, arm) = match op {
        CliOp::Start(ask) => {
            if w.take_open() {
                return refuse(w, "take_open");
            }
            // Start only where the popover would offer it, as if it were open now. With it
            // closed, the move is the machine's own edge idle-ready → countdown (N62).
            let idle = w.session.popover.is_none() && derive(w).state_ids().contains(&"idle-ready");
            let was = w.session.popover.take();
            w.session.popover = Some(Door::asked(w.now));
            let offered = derive(w).popover.as_ref().is_some_and(|p| p.offers(&Action::Start));
            if !offered {
                w.session.popover = was;
                return refuse(w, "not_ready");
            }
            let (fx, mut arm) = act(w, Action::Start);
            if w.session.arming.is_none() {
                // Not started: a permission beat is asked first, or nothing at all.
                w.session.popover = was;
                let mut fx = fx;
                fx.push(answer(w, Some("not_ready"), None));
                return (fx, arm);
            }
            if was.is_none() {
                arm.edge = idle.then_some(("idle-ready", "countdown"));
            }
            let mut fx = fx;
            // The take this start makes wears the label while it records (N62, B216).
            if ask.label {
                w.agent = w.session.arming.as_ref().map(|a| a.started);
            }
            // `--no-countdown`: the count's end, now. The tray goes from idle-ready to
            // recording when capture begins (the machine's own edge, on the recorder's word).
            if !ask.countdown {
                if let Some(a) = w.session.arming.as_mut() {
                    a.deadline = w.now;
                }
                fx.extend(fire(w));
                arm.edge = None;
            }
            (fx, arm)
        }
        _ if coming => return refuse(w, "busy"),
        CliOp::Pause => match phase {
            Some(Phase::Recording) => click(w, None),
            _ => return refuse(w, "no_take"),
        },
        CliOp::Resume => match phase {
            Some(Phase::Paused(cause)) if w.session.confirm.is_none() && crate::derive::resume_ok(w, cause) => act(w, Action::Resume),
            // Paused, but its popover wouldn't offer Resume now: its source is gone, or it
            // asks a question.
            Some(Phase::Paused(_)) => return refuse(w, "not_ready"),
            _ => return refuse(w, "no_take"),
        },
        CliOp::Editing { .. } | CliOp::Copy { .. } => unreachable!("an edit's asks are answered above"),
        CliOp::Stop => match phase {
            Some(Phase::Paused(_)) if w.session.confirm.is_some() => return refuse(w, "not_ready"),
            Some(Phase::Paused(_)) => act(w, Action::Stop),
            // Recording, not paused: the take's own stop, the machine's recording → stopping.
            Some(Phase::Recording) => {
                let (fx, mut arm) = act(w, Action::Stop);
                arm.edge = arm.edge.map(|_| ("recording", "stopping"));
                (fx, arm)
            }
            _ => return refuse(w, "no_take"),
        },
    };
    let link = stop.then(|| crate::derive::cli_link(w)).flatten();
    fx.push(answer(w, None, link));
    (fx, arm)
}

/// A `shroom://settings/…` link (N62, B216): Settings on the spot's tab, the spot lit for a
/// moment — General's switch, for the command's recording refusal. It changes nothing there:
/// the person flips the switch. Mid-take it opens nothing (law 4); a Mac with no space has no
/// Settings (B160). A sheet open in Settings is a question the person is in the middle of:
/// it stays, and Settings comes forward as it stands. An idle popover closes, as a click
/// away closes it.
fn settings_link(w: &mut World, spot: crate::event::SettingsSpot) -> Out {
    if w.take_open() || !w.provisioned() {
        return (vec![], Arm::DROPPED);
    }
    let sheet = matches!(w.session.confirm, Some(Confirm::Exit | Confirm::Uninstall { .. } | Confirm::Remove(_) | Confirm::Report { .. }));
    if sheet && w.window_visible(WindowKind::Settings) {
        open_window(w, WindowKind::Settings);
        return (vec![Effect::Shell(ShellCmd::OpenSettings)], Arm::named("settings-link-forward"));
    }
    let at_rest = w.session.popover.is_none() && !w.window_visible(WindowKind::Settings) && derive(w).state_ids().contains(&"idle-ready");
    if w.session.popover.is_some() {
        dismiss(w);
    }
    let mut fx = Vec::new();
    if w.window_visible(WindowKind::Settings) && settings_view::tab(w) != spot.tab() {
        // A tab change, as a click on the tab: a name being changed and left is put back (B158).
        w.session.fields.settings.from = None;
    }
    open_settings(w, Some(spot.tab()), &mut fx);
    fx.push(Effect::Shell(ShellCmd::OpenSettings));
    w.session.fields.settings.lit = Some((spot, w.now));
    let edge = (at_rest && spot.tab() == SettingsTab::General).then_some(("idle-ready", "settings-general"));
    (fx, Arm { name: "settings-link", edge })
}

/// A click or a key's action, from where it was: an editor window's are its own editor's
/// (N73); every other surface's are the app's.
fn act_from(w: &mut World, surface: Surface, action: Action) -> Out {
    match surface {
        Surface::Editor(id) => editor_act(w, id, action),
        _ => act(w, action),
    }
}

fn act(w: &mut World, action: Action) -> Out {
    let now = w.now;
    match action {
        Action::Dismiss => dismiss(w),
        Action::Start => {
            // An update applying quits the app any moment now: no take opens under it (law 4).
            if w.update.applying {
                return (vec![], Arm::DROPPED);
            }
            let Some(source) = crate::derive::popover_selected_source(w) else { return (vec![], Arm::DROPPED) };
            // The arm-time guard (Region 3): every arm re-probes grants (the runner reads them
            // just before it posts Start) and replays exactly the missing beat — one prompt,
            // never stacked, and never under the count (law 11). The person starts again once
            // macOS has its answer.
            if let Some(perm) = missing_beat(w, source) {
                return (vec![Effect::Shell(ShellCmd::RequestPermission(perm))], Arm::named("arm-guard-beat"));
            }
            w.prefs.source = Some(source);
            w.session.arming = Some(Arming { started: now, deadline: now.plus(COUNTDOWN_MS), source, restart_of: None, fired: false });
            w.session.popover = None;
            (vec![persist_prefs(w), warm(w)], Arm { name: "start", edge: Some(("config", "countdown")) })
        }
        Action::CancelCountdown => {
            w.session.arming = None;
            w.session.popover = Some(Door::asked(now));
            (vec![Effect::Recorder(RecCmd::Cool)], Arm { name: "cancel-countdown", edge: Some(("countdown", "config")) })
        }
        Action::Resume => {
            // The take's own microphone while it's here, else the one the paused popover
            // named (N39 `mic-lost`), else silence.
            let mic = crate::derive::resume_mic(w);
            if let Some(t) = w.take.as_mut() {
                t.phase = Phase::Resuming;
                t.since = now;
                t.mic = mic.clone();
                // The next pause offers Drop a marker again (N64).
                t.marked_at_pause = false;
            }
            w.session.popover = None;
            (vec![Effect::Recorder(RecCmd::Resume { mic })], Arm { name: "resume", edge: Some(("paused", "recording")) })
        }
        Action::Stop => {
            if let Some(t) = w.take.as_mut() {
                t.phase = Phase::Finalizing(Why::Stop);
                t.since = now;
            }
            w.session.popover = None;
            let mut fx = vec![Effect::Recorder(RecCmd::Finalize { why: Why::Stop })];
            // A space that can't take the video has no page to open (N34). A new stop's
            // probe starts afresh, its backoff and its “not shroom” run behind it (B60).
            w.inflight.failures.remove(&Op::Reach);
            w.net.not_shroom = None;
            if w.online() != Some(false) && !crate::derive::members::stranded_by_space(w) {
                fx.push(Effect::Cloud(CloudCmd::Reach));
            }
            (fx, Arm { name: "stop", edge: Some(("paused", "stopping")) })
        }
        // Restart asks first (the canvas's question, DEVIATIONS #44).
        Action::Restart => {
            if !matches!(w.take.as_ref().map(|t| t.phase), Some(Phase::Paused(_))) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = Some(Confirm::Restart);
            (vec![], Arm { name: "restart-asked", edge: Some(("paused", "restart-confirm")) })
        }
        Action::ConfirmRestart | Action::RestartInstead => {
            let Some(t) = &w.take else { return (vec![], Arm::DROPPED) };
            let source = t.source;
            // The edge's name only (the trace): Restart instead answers the Discard question;
            // Restart's own question is restart-confirm (DEVIATIONS #44, B134).
            let from = if w.session.confirm == Some(Confirm::Discard) { "discard-confirm" } else { "restart-confirm" };
            w.session.confirm = None;
            w.session.popover = None;
            // The count is Restart's undo: the old take dies only when the new capture
            // begins (N39).
            w.session.arming = Some(Arming {
                started: now,
                deadline: now.plus(COUNTDOWN_MS),
                source,
                restart_of: Some(t.dir.clone()),
                fired: false,
            });
            (vec![warm(w)], Arm { name: "restart", edge: Some((from, "countdown")) })
        }
        // N64: a marker at the pause point, once — the row then reads Marker dropped until
        // Resume (⌥⌘M while paused comes here too, past the Scene's gate).
        Action::DropMarker => {
            if !crate::derive::marker_offered(w) {
                return (vec![], Arm::DROPPED);
            }
            if let Some(t) = w.take.as_mut() {
                t.markers += 1;
                t.marked_at_pause = true;
            }
            (vec![Effect::Recorder(RecCmd::Marker)], Arm::named("drop-marker"))
        }
        Action::Discard => {
            w.session.confirm = Some(Confirm::Discard);
            (vec![], Arm { name: "discard-asked", edge: Some(("paused", "discard-confirm")) })
        }
        Action::ConfirmDiscard => {
            let Some(t) = w.take.take() else { return (vec![], Arm::DROPPED) };
            w.session.confirm = None;
            w.session.popover = None;
            (
                vec![Effect::Recorder(RecCmd::Discard(t.dir)), Effect::Uploader(UpCmd::ForgetRemote(t.rec))],
                Arm { name: "discard", edge: Some(("discard-confirm", "idle-ready")) },
            )
        }
        Action::Back => {
            // An uninstall under way isn't taken back: shroom is removing itself (B135); nor
            // is a report on its way (B180).
            if matches!(w.session.confirm, Some(Confirm::Uninstall { going: true } | Confirm::Report { sending: true })) {
                return (vec![], Arm::DROPPED);
            }
            let was = w.session.confirm.take();
            if was == Some(Confirm::Report { sending: false }) {
                w.session.fields.settings.report = ReportFields::default();
                return (vec![], Arm { name: "report-cancelled", edge: Some(("report-problem", "settings-general")) });
            }
            // No sheet to take back: it's Suggested to clean up's ‹ Library (N68).
            if was.is_none() && w.session.fields.library.cleanup {
                w.session.fields.library.cleanup = false;
                return (vec![], Arm { name: "cleanup-closed", edge: Some(("library-cleanup", "library")) });
            }
            (vec![], Arm::named("back"))
        }
        Action::GetLink => {
            let Some(dir) = saved_target(w) else { return (vec![], Arm::DROPPED) };
            if w.update.applying {
                return (vec![], Arm::DROPPED);
            }
            get_link(w, dir)
        }
        Action::DiscardSaved => {
            let Some(dir) = saved_target(w) else { return (vec![], Arm::DROPPED) };
            w.session.last_stop = None;
            w.session.confirm = Some(Confirm::DiscardSaved(dir));
            (vec![], Arm::named("discard-saved-asked"))
        }
        Action::ConfirmDiscardSaved => {
            let Some(Confirm::DiscardSaved(dir)) = w.session.confirm.take() else { return (vec![], Arm::DROPPED) };
            let rec = w.saved.iter().find(|s| s.dir == dir).map(|s| s.rec.clone());
            w.saved.retain(|s| s.dir != dir);
            let mut fx = vec![Effect::Takes(TakeCmd::Delete(dir))];
            fx.extend(rec.map(|r| Effect::Uploader(UpCmd::ForgetRemote(r))));
            (fx, Arm::named("discard-saved"))
        }
        Action::SaveToMovies => {
            let Some(dir) = saved_target(w) else { return (vec![], Arm::DROPPED) };
            saving(save_to_movies(w, vec![dir]), "save-to-movies")
        }
        Action::OpenLibrary => {
            w.session.popover = None;
            open_window(w, WindowKind::Library);
            (vec![Effect::Shell(ShellCmd::OpenLibrary)], Arm { name: "open-library", edge: Some(("config", "library")) })
        }
        Action::OpenSettings => {
            w.session.popover = None;
            let mut fx = Vec::new();
            open_settings(w, None, &mut fx);
            fx.push(Effect::Shell(ShellCmd::OpenSettings));
            let edge = (settings_view::tab(w) == SettingsTab::General).then_some(("config", "settings-general"));
            (fx, Arm { name: "open-settings", edge })
        }
        Action::OpenFeedback => {
            w.session.popover = None;
            // A window already open comes forward as typed; a new one starts empty, its
            // report gathered for the log box (ticked) and the disclosure.
            let fresh = !w.session.windows.contains_key(&WindowKind::Feedback);
            open_window(w, WindowKind::Feedback);
            let mut fx = Vec::new();
            if fresh {
                w.session.fields.feedback = FeedbackFields::default();
                fx.push(Effect::Shell(ShellCmd::GatherReport { role: crate::report::role_words(w.space.role()) }));
            }
            fx.push(Effect::Shell(ShellCmd::OpenFeedback));
            (fx, Arm { name: "open-feedback", edge: Some(("config", "send-feedback")) })
        }
        Action::Quit => quit_requested(w, QuitVia::Footer),
        Action::QuitAnyway => quit_now(w, Leaving::QuitAnyway, "quit-anyway"),
        Action::CopyLink => {
            let Some(StopOutcome::Offline(rec)) = &w.session.last_stop else { return (vec![], Arm::DROPPED) };
            let rec = rec.clone();
            w.session.copied = Some((CopyTarget::Link(rec.clone()), now));
            (vec![Effect::Shell(ShellCmd::Copy(rec))], Arm::named("copy-link"))
        }
        Action::SetSource(s) => {
            w.prefs.source = Some(s);
            // A display picked by hand holds over the click's until the take ends (N56).
            w.display_pick.by_hand = matches!(s, SourceId::Display(_));
            let mut fx = vec![persist_prefs(w)];
            // Choosing Camera as the source is a camera-permission moment too (N22) — right
            // after the click.
            if s == SourceId::Camera && w.mac.camera.is(&Access::NotAsked) {
                fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Camera)));
            }
            (fx, Arm::named("set-source"))
        }
        // The camera picker's pick (DEVIATIONS #43): a camera turns the bubble on with it, Camera
        // off turns it off and keeps the pick for next time. With Camera only as the source
        // the pick is the camera it records — the bubble's wish stays as it was.
        Action::SetCamera(c) => {
            let camera_source = crate::derive::popover_selected_source(w) == Some(SourceId::Camera);
            let on = c.is_some();
            if let Some(uid) = c {
                w.prefs.camera = Some(uid);
            }
            if !camera_source {
                w.prefs.show_yourself = on;
            }
            let mut fx = vec![persist_prefs(w)];
            // Picking a camera is the camera-permission moment (N22) — right after the click.
            if on && w.mac.camera.is(&Access::NotAsked) {
                fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Camera)));
            }
            (fx, Arm::named("set-camera"))
        }
        // System default, in the camera picker (N44): the camera follows macOS's preferred one,
        // and it turns the bubble on like any camera pick — the permission moment if unasked.
        Action::SetCameraDefault => {
            let camera_source = crate::derive::popover_selected_source(w) == Some(SourceId::Camera);
            w.prefs.camera = None;
            if !camera_source {
                w.prefs.show_yourself = true;
            }
            let mut fx = vec![persist_prefs(w)];
            if w.mac.camera.is(&Access::NotAsked) {
                fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Camera)));
            }
            (fx, Arm::named("set-camera-default"))
        }
        // System default, for the microphone (N44): it follows macOS's default input.
        Action::SetMicDefault => {
            w.prefs.mic = None;
            w.prefs.mic_off = false;
            (vec![persist_prefs(w)], Arm::named("set-mic-default"))
        }
        // The microphone's pick: one by uid, or No mic — the take records silently (DEVIATIONS #43).
        Action::SetMic(m) => {
            w.prefs.mic_off = m.is_none();
            if m.is_some() {
                w.prefs.mic = m;
            }
            (vec![persist_prefs(w)], Arm::named("set-mic"))
        }
        Action::SetQuality(q) => {
            w.prefs.quality = q;
            (vec![persist_prefs(w)], Arm::named("set-quality"))
        }
        // recording-off's banner and Settings · Space's row six: the permissions chapter at
        // exactly the missing beat, in the Settings window (N36, N38).
        Action::TurnBackOn => {
            if w.take_open() {
                return (vec![], Arm::DROPPED);
            }
            w.session.popover = None;
            let mut fx = Vec::new();
            open_settings(w, Some(SettingsTab::Space), &mut fx);
            w.session.chapter = true;
            ask_screen(w, &mut fx);
            (fx, Arm { name: "turn-back-on", edge: Some(("recording-off", "perm-screen")) })
        }
        Action::OpenCameraSettings => {
            (vec![Effect::Shell(ShellCmd::SettingsPane(crate::effect::Pane::Camera))], Arm::named("camera-settings"))
        }
        Action::OpenMicSettings => {
            (vec![Effect::Shell(ShellCmd::SettingsPane(crate::effect::Pane::Microphone))], Arm::named("mic-settings"))
        }
        // ── the welcome ──
        Action::SetUp => {
            let mut fx = Vec::new();
            register_login(w, &mut fx);
            w.prefs.setup_chosen = true;
            fx.push(persist_prefs(w));
            (fx, Arm { name: "set-up", edge: Some(("welcome", "account")) })
        }
        Action::Later => {
            let mut fx = Vec::new();
            later(w, &mut fx);
            (fx, Arm { name: "later", edge: Some(("welcome", "idle-unprovisioned")) })
        }
        Action::SetLoginBox(on) => {
            w.session.fields.login_off = !on;
            (vec![], Arm::named("login-box"))
        }
        // ── the plan ──
        Action::NotYet => {
            w.session.fork = Fork::SigningUp;
            (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Signup))], Arm { name: "not-yet", edge: Some(("account", "account-signing-up")) })
        }
        Action::HaveOne => {
            w.session.fork = Fork::Affirmed;
            w.session.fields.signed_up = false;
            (vec![], Arm { name: "have-one", edge: Some(("account", "token")) })
        }
        Action::SignupAgain => (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::Signup))], Arm::named("signup-again")),
        Action::SignedUp => {
            w.session.fork = Fork::Affirmed;
            // A fresh account: the token step says the email comes first (N45).
            w.session.fields.signed_up = true;
            (vec![], Arm { name: "signed-up", edge: Some(("account-signing-up", "token")) })
        }
        Action::CreateToken => (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::TokenPage))], Arm::named("create-token")),
        Action::PickAccount(id) => {
            w.session.fields.account = Some(id);
            (vec![], Arm::named("pick-account"))
        }
        Action::UseAccount => {
            let chosen = onboarding::picked(w);
            let Some(Paste::PickAccount { id, accounts }) = &w.sweep.paste else { return (vec![], Arm::DROPPED) };
            let Some(account) = accounts.iter().find(|a| Some(&a.id) == chosen.as_ref()).cloned() else { return (vec![], Arm::DROPPED) };
            w.sweep.paste = Some(Paste::Saving { id: *id, account, several: true });
            (vec![], Arm::named("use-account"))
        }
        Action::AddCard => {
            let Some(account) = w.sweep.account() else { return (vec![], Arm::DROPPED) };
            let url = PublicUrl::Billing { account_id: account.id.clone() };
            w.session.fields.billing_opened = true;
            // The watch's clock counts from the first click (N74).
            w.session.fields.billing_at.get_or_insert(w.now);
            (vec![Effect::Shell(ShellCmd::OpenUrl(url))], Arm::named("add-card"))
        }
        Action::Claim => {
            let name = w.session.fields.name.clone();
            (vec![Effect::Cloud(CloudCmd::Claim(name))], Arm::named("claim"))
        }
        Action::AllowRecording => {
            w.session.chapter = true;
            let mut fx = Vec::new();
            // One prompt at a time, the microphone first (POC §4); a resolved mic goes
            // straight to the screen step.
            if w.mac.mic.get().is_none_or(|m| *m == Access::NotAsked) {
                // Its answer chains the camera, then the screen — this click's later halves
                // (law 11).
                w.mac.mic_asked = true;
                fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Mic)));
            } else if camera_unasked(w) {
                w.mac.camera_asked = true;
                fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Camera)));
            } else if !w.mac.screen.is(&true) {
                ask_screen(w, &mut fx);
            }
            (fx, Arm { name: "allow-recording", edge: Some(("done", "perm-mic")) })
        }
        // The alert led and was closed (N43): the exact pane, and the face becomes its.
        Action::OpenScreenSettings => {
            w.mac.screen_alert = Some(false);
            (vec![Effect::Shell(ShellCmd::SettingsPane(crate::effect::Pane::ScreenRecording))], Arm::named("open-screen-settings"))
        }
        Action::ReopenShroom => {
            w.launch.ending = true;
            // The chapter runs where it was opened: the onboarding window, or Settings.
            let window = if w.window_visible(WindowKind::Onboarding) { WindowKind::Onboarding } else { WindowKind::Settings };
            let restore = Restore { why: RelaunchWhy::Permission, window, written: w.wall };
            (
                vec![Effect::Persist(PersistOp::Restore(Some(restore))), Effect::Shell(ShellCmd::Relaunch(RelaunchWhy::Permission))],
                Arm { name: "reopen-shroom", edge: Some(("perm-screen", "relaunching")) },
            )
        }
        // ── all-set ──
        Action::CloseWindow => close_onboarding(w),
        // all-set's Invite someone: the same window morphs into Settings on Members, the
        // name field focused (N28).
        Action::InviteSomeone => {
            leave_onboarding(w);
            let mut fx = Vec::new();
            open_settings(w, Some(SettingsTab::Members), &mut fx);
            fx.push(Effect::Shell(ShellCmd::OpenSettings));
            (fx, Arm { name: "invite-someone", edge: Some(("all-set", "invite-born")) })
        }
        Action::RecordNow => {
            leave_onboarding(w);
            w.session.popover = Some(Door::asked(now));
            (vec![], Arm { name: "record-now", edge: Some(("all-set", "config")) })
        }
        // ── the update ──
        Action::Update => apply_update(w),
        Action::WhatsNew => {
            let version = match w.update.offer.get() {
                Some(Some(UpdateOffer::Ready { version } | UpdateOffer::NeedsMacos { version, .. })) => version.clone(),
                _ => w.launch.version.clone(),
            };
            (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::ReleaseNotes { version }))], Arm::named("whats-new"))
        }
        // ── space-degraded ──
        Action::ReconnectSpace => {
            w.session.popover = None;
            let mut fx = Vec::new();
            open_settings(w, Some(SettingsTab::Space), &mut fx);
            // An open Settings window comes forward, as the footer's door brings it (#11).
            fx.push(Effect::Shell(ShellCmd::OpenSettings));
            (fx, Arm { name: "reconnect-space", edge: Some(("space-degraded", "repair-mood")) })
        }
        // ── the Library ──
        Action::SelectRow(rec) => {
            w.session.fields.library.selected = vec![rec];
            (vec![], Arm::named("select-row"))
        }
        Action::ToggleRow(rec) => {
            let sel = &mut w.session.fields.library.selected;
            if let Some(i) = sel.iter().position(|r| *r == rec) {
                sel.remove(i);
            } else {
                sel.push(rec);
            }
            (vec![], Arm::named("toggle-row"))
        }
        Action::ExtendRow(rec) => {
            let order: Vec<_> = library_view::shown(w).into_iter().filter(|r| r.published()).map(|r| r.rec).collect();
            let anchor = w.session.fields.library.selected.first().cloned();
            let range = match (anchor.and_then(|a| order.iter().position(|r| *r == a)), order.iter().position(|r| *r == rec)) {
                (Some(a), Some(b)) => {
                    let (lo, hi) = (a.min(b), a.max(b));
                    let mut v: Vec<_> = order[lo..=hi].to_vec();
                    if b < a {
                        v.reverse();
                    }
                    v
                }
                _ => vec![rec],
            };
            w.session.fields.library.selected = range;
            (vec![], Arm::named("extend-row"))
        }
        Action::CopyRow(rec) => {
            w.session.copied = Some((CopyTarget::Link(rec.clone()), now));
            (vec![Effect::Shell(ShellCmd::Copy(rec))], Arm::named("copy-row"))
        }
        Action::CopyThumbRow(rec) => {
            let title = library_view::title_of(w, &rec).unwrap_or_default();
            (vec![Effect::Shell(ShellCmd::CopyThumb(rec, title))], Arm::named("copy-thumb-row"))
        }
        Action::OpenPage(rec) => (vec![Effect::Cloud(CloudCmd::OpenOwnerPage(rec))], Arm { name: "open-page", edge: Some(("library", "owner-page")) }),
        Action::EditRow(rec) => open_editor(w, rec),
        Action::StartRename(rec) => {
            let title = library_view::title_of(w, &rec).unwrap_or_default();
            w.session.fields.library.renaming = Some((rec, title));
            (vec![], Arm::named("start-rename"))
        }
        Action::CommitRename => {
            // Offline, renaming waits until you're back (N35): the typing stays in its field.
            if w.online() == Some(false) {
                return (vec![], Arm::DROPPED);
            }
            let Some((rec, typed)) = w.session.fields.library.renaming.take() else { return (vec![], Arm::DROPPED) };
            let title = typed.split_whitespace().collect::<Vec<_>>().join(" ");
            // Emptied → the previous title comes back, never blank (N35).
            if title.is_empty() || library_view::title_of(w, &rec).as_deref() == Some(title.as_str()) {
                return (vec![], Arm::named("rename-unchanged"));
            }
            w.library_ops.renaming.insert(rec.clone(), title.clone());
            (vec![Effect::Library(LibCmd::Rename(rec, title))], Arm::named("rename"))
        }
        Action::CancelRename => {
            w.session.fields.library.renaming = None;
            (vec![], Arm::named("cancel-rename"))
        }
        Action::DeleteRow(rec) => {
            let sel = &w.session.fields.library.selected;
            let targets = if sel.len() > 1 && sel.contains(&rec) { library_view::deletable(w, sel) } else { library_view::deletable(w, &[rec]) };
            ask_delete(w, targets)
        }
        Action::DeleteSelected => {
            let targets = library_view::deletable(w, &w.session.fields.library.selected);
            ask_delete(w, targets)
        }
        Action::ConfirmDelete => {
            let Some(Confirm::Delete(recs)) = w.session.confirm.take() else { return (vec![], Arm::DROPPED) };
            w.library_ops.deleting.extend(recs.iter().cloned());
            w.session.fields.library.selected.clear();
            // The editor open on one of them closes, nothing saved (B199).
            let gone = editor_video_gone(w, &recs);
            let mut fx = vec![Effect::Library(LibCmd::Delete(recs))];
            fx.extend(gone);
            (fx, Arm { name: "delete", edge: Some(("delete-confirm", "removed")) })
        }
        Action::GetLinkRow(dir) => {
            if w.take_open() || w.update.applying {
                return (vec![], Arm::DROPPED);
            }
            get_link(w, dir)
        }
        Action::DiscardRow(dir) => {
            w.session.confirm = Some(Confirm::DiscardRow(dir));
            (vec![], Arm::named("discard-row-asked"))
        }
        Action::ConfirmDiscardRow => {
            let Some(Confirm::DiscardRow(dir)) = w.session.confirm.take() else { return (vec![], Arm::DROPPED) };
            let mut fx = Vec::new();
            if let Some(pos) = w.saved.iter().position(|s| s.dir == dir) {
                // A recovered take: N25's clean delete (no link was ever handed out).
                let s = w.saved.remove(pos);
                fx.push(Effect::Takes(TakeCmd::Delete(dir)));
                fx.push(Effect::Uploader(UpCmd::ForgetRemote(s.rec)));
            } else if let Some(pos) = w.removed.iter().position(|q| q.dir == dir) {
                // Already gone from the space: only this Mac's copy is left to toss.
                w.removed.remove(pos);
                fx.push(Effect::Takes(TakeCmd::Delete(dir)));
            } else if let Some(pos) = w.queue.iter().position(|q| q.dir == dir && settings_view::stranded(w).iter().any(|s| s.dir == dir)) {
                // A take the space can't take, saved to Movies: this Mac's copy is tossed (N34).
                w.queue.remove(pos);
                w.members.movies.remove(&dir);
                fx.push(Effect::Takes(TakeCmd::Delete(dir)));
            } else {
                return (vec![], Arm::DROPPED);
            }
            (fx, Arm::named("discard-row"))
        }
        Action::SaveRowToMovies(dir) => saving(save_to_movies(w, vec![dir]), "save-row-to-movies"),
        // The exit sheet's “Save them to Movies first”: the takes that can't reach the space.
        Action::SaveStrandedToMovies => {
            let dirs: Vec<TakeDir> = settings_view::stranded(w).into_iter().map(|q| q.dir.clone()).collect();
            if dirs.is_empty() {
                return (vec![], Arm::DROPPED);
            }
            saving(save_to_movies(w, dirs), "save-stranded-to-movies")
        }
        // ── Settings ──
        Action::SettingsTab(tab) => {
            w.prefs.settings_tab = Some(tab);
            // Another tab puts the link's light out: coming back never lights it again.
            w.session.fields.settings.lit = None;
            // A name being changed and left is put back (B158).
            w.session.fields.settings.from = None;
            // Entering a tab is an entry: Space re-probes (law 1).
            if let Some(d) = w.session.windows.get_mut(&WindowKind::Settings) {
                d.entered = now;
            }
            let edge = (tab == SettingsTab::Space).then_some(("settings-general", "settings-space"));
            (vec![persist_prefs(w)], Arm { name: "settings-tab", edge })
        }
        Action::SetLoginItem(on) => (vec![Effect::Shell(ShellCmd::SetLoginItem(on))], Arm::named("login-item")),
        Action::ReportIssue => {
            let version = w.launch.version.clone();
            (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::NewIssue { version }))], Arm::named("report-issue"))
        }
        // ── Your coding agent (dev/proposals/cli-agent-skill.md) ──
        Action::CopyAgentPrompt => {
            if !w.window_visible(WindowKind::Settings) {
                return (vec![], Arm::DROPPED);
            }
            w.session.copied = Some((CopyTarget::AgentPrompt, w.now));
            (vec![Effect::Shell(ShellCmd::CopyAgentPrompt)], Arm::named("copy-agent-prompt"))
        }
        Action::OpenAgentGuide => (vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::AgentGuide))], Arm::named("agent-guide")),
        Action::SetAgentRecording(on) => {
            w.prefs.agent_recording = on;
            // The link's light pointed here; the person's own flip puts it out.
            w.session.fields.settings.lit = None;
            (vec![persist_prefs(w)], Arm::named("agent-recording"))
        }
        // ── Report a problem (B180) ──
        Action::CopyDiagnostics => {
            if !w.window_visible(WindowKind::Settings) {
                return (vec![], Arm::DROPPED);
            }
            w.session.copied = Some((CopyTarget::Diagnostics, w.now));
            (vec![Effect::Shell(ShellCmd::CopyDiagnostics { role: crate::report::role_words(w.space.role()) })], Arm::named("copy-diagnostics"))
        }
        Action::ReportProblem => {
            if !(w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::General) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = Some(Confirm::Report { sending: false });
            w.session.fields.settings.report = ReportFields::default();
            w.session.fields.settings.sent = false;
            let fx = vec![Effect::Shell(ShellCmd::GatherReport { role: crate::report::role_words(w.space.role()) })];
            (fx, Arm { name: "report-asked", edge: Some(("settings-general", "report-problem")) })
        }
        Action::SendReport => {
            let f = &w.session.fields.settings.report;
            let Some(diagnostics) = f.gathered.clone() else { return (vec![], Arm::DROPPED) };
            if w.session.confirm != Some(Confirm::Report { sending: false }) || f.words.trim().is_empty() {
                return (vec![], Arm::DROPPED);
            }
            let reply = f.reply_to.trim();
            let post = Post { kind: Kind::Problem, words: f.words.trim().to_string(), reply_to: (!reply.is_empty()).then(|| reply.to_string()), diagnostics };
            w.session.confirm = Some(Confirm::Report { sending: true });
            w.session.fields.settings.report.failed = false;
            (vec![Effect::Shell(ShellCmd::SendReport(post))], Arm::named("report-sending"))
        }
        Action::MailReport => {
            let f = &w.session.fields.settings.report;
            if w.session.confirm != Some(Confirm::Report { sending: false }) || !f.failed {
                return (vec![], Arm::DROPPED);
            }
            let version = f.gathered.as_ref().map_or_else(|| w.launch.version.clone(), |d| d.shroom.clone());
            let url = PublicUrl::MailReport { version, words: f.words.trim().to_string() };
            (vec![Effect::Shell(ShellCmd::OpenUrl(url))], Arm::named("report-by-mail"))
        }
        // ── Send feedback (N60) ──
        Action::FeedbackLog(on) => {
            let open = w.window_visible(WindowKind::Feedback);
            let f = &mut w.session.fields.feedback;
            if !open || f.sending || f.sent {
                return (vec![], Arm::DROPPED);
            }
            f.without_log = !on;
            (vec![], Arm::named("feedback-log"))
        }
        Action::SendFeedback => {
            let f = &w.session.fields.feedback;
            if !w.window_visible(WindowKind::Feedback) || f.sending || f.sent || f.words.trim().is_empty() {
                return (vec![], Arm::DROPPED);
            }
            let Some(diagnostics) = crate::derive::feedback::goes(w, f) else { return (vec![], Arm::DROPPED) };
            let reply = f.reply_to.trim();
            let post = Post { kind: Kind::Feedback, words: f.words.trim().to_string(), reply_to: (!reply.is_empty()).then(|| reply.to_string()), diagnostics };
            let f = &mut w.session.fields.feedback;
            f.sending = true;
            f.failed = false;
            (vec![Effect::Shell(ShellCmd::SendReport(post))], Arm::named("feedback-sending"))
        }
        Action::MailFeedback => {
            let f = &w.session.fields.feedback;
            if !w.window_visible(WindowKind::Feedback) || f.sending || !f.failed {
                return (vec![], Arm::DROPPED);
            }
            let url = PublicUrl::MailReport { version: w.launch.version.clone(), words: f.words.trim().to_string() };
            (vec![Effect::Shell(ShellCmd::OpenUrl(url))], Arm::named("feedback-by-mail"))
        }
        Action::CloseFeedback => {
            if !w.window_visible(WindowKind::Feedback) || w.session.fields.feedback.sending {
                return (vec![], Arm::DROPPED);
            }
            window_closed(w, WindowKind::Feedback)
        }
        // ── the editor (N63): an editor window's own, `editor_act`'s ──
        Action::EditorDone
        | Action::EditorRevert
        | Action::EditorKeepEditing
        | Action::EditorDiscard
        | Action::EditorRetry
        | Action::EditorClose
        | Action::DeleteOriginal
        | Action::KeepOriginal => (vec![], Arm::DROPPED),
        Action::Disconnect => {
            if w.take_open() {
                return (vec![], Arm::DROPPED);
            }
            // The member banners' “Leave this space…” opens its sheet in Settings · Space.
            let mut fx = Vec::new();
            if !(w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::Space) {
                w.session.popover = None;
                open_settings(w, Some(SettingsTab::Space), &mut fx);
                fx.push(Effect::Shell(ShellCmd::OpenSettings));
            }
            w.session.confirm = Some(Confirm::Exit);
            (fx, Arm { name: "disconnect-asked", edge: Some(("settings-space", "disconnect-confirm")) })
        }
        Action::ConfirmDisconnect => {
            // Uploads that can still drain and a recovered take waiting block it (N31's
            // rule); stranded takes don't — they can't drain (N36).
            // A copy to Movies still under way holds it too: nothing it was asked to keep
            // is deleted before it lands (law 2, DECISIONS B54).
            if w.session.confirm != Some(Confirm::Exit) || w.take_open() || settings_view::draining(w) > 0 || !w.saved.is_empty() || settings_view::saving_stranded(w) {
                return (vec![], Arm::DROPPED);
            }
            disconnect(w)
        }
        Action::Uninstall => {
            if w.take_open() {
                return (vec![], Arm::DROPPED);
            }
            // A Mac with no space asks over the welcome: Settings needs a space (B160).
            if !w.provisioned() {
                if !w.window_visible(WindowKind::Onboarding) {
                    return (vec![], Arm::DROPPED);
                }
                w.session.confirm = Some(Confirm::Uninstall { going: false });
                return (vec![], Arm { name: "uninstall-asked", edge: Some(("welcome", "uninstall-confirm")) });
            }
            let mut fx = Vec::new();
            if !(w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::General) {
                w.session.popover = None;
                open_settings(w, Some(SettingsTab::General), &mut fx);
                fx.push(Effect::Shell(ShellCmd::OpenSettings));
            }
            w.session.confirm = Some(Confirm::Uninstall { going: false });
            (fx, Arm { name: "uninstall-asked", edge: Some(("settings-general", "uninstall-confirm")) })
        }
        Action::ConfirmUninstall => {
            // Disconnect's guards (N31's rule, B54): nothing that can still drain, no
            // recovered take waiting, no copy to Movies under way — and only once.
            if w.session.confirm != Some(Confirm::Uninstall { going: false }) || w.take_open() || settings_view::draining(w) > 0 || !w.saved.is_empty() || settings_view::saving_stranded(w) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = Some(Confirm::Uninstall { going: true });
            (vec![Effect::Shell(ShellCmd::Uninstall)], Arm::named("uninstall"))
        }
        Action::SetUpFresh => {
            let Some(id) = &w.space.identity else { return (vec![], Arm::DROPPED) };
            // One wiring at a time: a second click while it runs adds nothing (#10).
            if w.inflight.ops.contains_key(&Op::Wire) {
                return (vec![], Arm::DROPPED);
            }
            let label = id.label.clone();
            w.session.fields.settings.fresh = true;
            (vec![Effect::Cloud(CloudCmd::Wire { label, fresh: false })], Arm { name: "set-up-fresh", edge: Some(("space-storage-gone", "wiring")) })
        }
        // The wizard's “something unexpected” face (DEVIATIONS #25): its Try again asks
        // every wizard call again now — the backoff is behind it, and the window re-probes.
        Action::TryAgain => {
            w.inflight.failures.retain(|op, f| !(Op::WIZARD.contains(op) && f.class != FailClass::Transport));
            for kind in [WindowKind::Onboarding, WindowKind::Settings] {
                if let Some(d) = w.session.windows.get_mut(&kind) {
                    d.entered = now;
                }
            }
            (vec![], Arm::named("try-again"))
        }
        // ── Region 6: the join machine ──
        Action::Join => {
            // Never mid-take (law 4); the switch's Join waits for the old space (N31).
            if w.take_open() {
                return (vec![], Arm::DROPPED);
            }
            let welcome = onboarding::at(w) == At::MemberWelcome;
            let mut fx = Vec::new();
            // member-welcome's Join is a dismissal of its login box (U4).
            if welcome {
                register_login(w, &mut fx);
            }
            // The switch's face named the takes the old space can't take — “joining deletes
            // them from this Mac” — and offered Movies first: the click consents for those.
            let left_behind: BTreeSet<TakeDir> = if welcome { BTreeSet::new() } else { settings_view::stranded(w).into_iter().map(|q| q.dir.clone()).collect() };
            if w.members.invite.is_none() {
                return (vec![], Arm::DROPPED);
            }
            w.members.left_behind = left_behind;
            let Some(inv) = w.members.invite.as_mut() else { return (vec![], Arm::DROPPED) };
            inv.joining = true;
            inv.refused = None;
            w.inflight.failures.remove(&Op::Join);
            let from = if welcome { "member-welcome" } else { "join-switch-confirm" };
            (fx, Arm { name: "join", edge: Some((from, "joining")) })
        }
        Action::Stay => {
            let mut fx = Vec::new();
            forget_invite(w, &mut fx);
            leave_onboarding(w);
            (fx, Arm::named("stay"))
        }
        // ── Region 6: the Members face ──
        Action::Invite => {
            let label = w.session.fields.settings.invite_name.split_whitespace().collect::<Vec<_>>().join(" ");
            if label.is_empty() {
                return (vec![], Arm::DROPPED);
            }
            (vec![Effect::Member(MemberCmd::Invite { label })], Arm::named("invite"))
        }
        // Who invites say they're from (B158): Change opens the name as a field, Save keeps
        // it in the space's identity (space.json), Esc puts it back.
        Action::ChangeFrom => {
            let name = w.space.identity.as_ref().map(|i| i.owner_name.clone()).unwrap_or_default();
            w.session.fields.settings.from = Some(name);
            w.session.fields.settings.from_edits = 0;
            (vec![], Arm::named("from-change"))
        }
        Action::SaveFrom => {
            let Some(typed) = w.session.fields.settings.from.take() else { return (vec![], Arm::DROPPED) };
            let name = typed.split_whitespace().collect::<Vec<_>>().join(" ");
            let Some(id) = w.space.identity.as_mut().filter(|_| !name.is_empty()) else {
                w.session.fields.settings.from = Some(typed);
                return (vec![], Arm::DROPPED);
            };
            if id.owner_name == name {
                return (vec![], Arm::named("from-kept"));
            }
            id.owner_name = name;
            (vec![Effect::Persist(PersistOp::Space(id.clone()))], Arm::named("from-saved"))
        }
        Action::KeepFrom => {
            w.session.fields.settings.from = None;
            (vec![], Arm::named("from-kept"))
        }
        Action::CopyInvite(id) => {
            w.session.copied = Some((CopyTarget::Invite(id), now));
            (vec![Effect::Shell(ShellCmd::CopyInvite(id))], Arm::named("copy-invite"))
        }
        Action::AddMac => (vec![Effect::Member(MemberCmd::AddMac)], Arm { name: "add-mac", edge: Some(("invite-born", "add-another-mac")) }),
        Action::NewLink(iid) => (vec![Effect::Member(MemberCmd::Renew(iid))], Arm::named("new-link")),
        Action::Remove(row) => {
            w.session.confirm = Some(Confirm::Remove(match row {
                crate::scene::RowRef::Member(id) => Removal::Member(id),
                crate::scene::RowRef::Invite(id) => Removal::Invite(id),
                crate::scene::RowRef::Mac(id) => Removal::Mac(id),
            }));
            (vec![], Arm { name: "remove-asked", edge: Some(("invite-born", "member-remove-confirm")) })
        }
        Action::ConfirmRemove => {
            let Some(Confirm::Remove(r)) = w.session.confirm.take() else { return (vec![], Arm::DROPPED) };
            let cmd = match r {
                Removal::Member(id) => MemberCmd::RemoveMember(id),
                Removal::Invite(id) => MemberCmd::CancelInvite(id),
                Removal::Mac(id) => MemberCmd::RemoveMac(id),
            };
            (vec![Effect::Member(cmd)], Arm::named("remove"))
        }
        // “Add a token here” / “Reconnect here”: Settings · Space at the wizard's token row.
        Action::AddToken => {
            w.session.popover = None;
            let mut fx = Vec::new();
            open_settings(w, Some(SettingsTab::Space), &mut fx);
            w.session.fields.settings.add_token = true;
            fx.push(Effect::Shell(ShellCmd::OpenSettings));
            (fx, Arm { name: "add-token", edge: Some(("space-degraded-no-token-mac", "token")) })
        }
        Action::SetMark(on) => (vec![Effect::Member(MemberCmd::Mark(on))], Arm::named("set-mark")),
        // Naming (N66): the setup Mac re-wires its page; the switch reads the page's answer.
        // A new try starts clean: its own failure, if any, is the one said. The switch is a
        // choice: AI is settled on this Mac, never turned on again by default (N75).
        Action::SetAi(on) => {
            w.inflight.failures.remove(&Op::Ai);
            w.space.ai_turn = None;
            let mut fx = vec![Effect::Cloud(CloudCmd::SetAi(on))];
            fx.extend(settle_ai(w));
            (fx, Arm::named("set-ai"))
        }
        // The AI banner's OK (N75): AI stays as it is, and the banner is answered for good on
        // this Mac — the setup Mac's, or a member's.
        Action::AiOk => {
            let from = derive(w).popover.and_then(|p| p.banner).map(|b| b.id).filter(|id| matches!(*id, "ai-turned-on" | "ai-turn-off-failed" | "ai-on-in-space"));
            let Some(from) = from else { return (vec![], Arm::DROPPED) };
            // A turn-on still on its way goes on (quietly, if it must be tried again); a Turn
            // off that didn't go through stops being tried.
            if w.space.ai_turn != Some(AiTurn::TurningOn) {
                w.space.ai_turn = None;
            }
            (settle_ai(w), Arm { name: "ai-ok", edge: Some((from, "config")) })
        }
        // The banner's Turn off (N75): the switch's off — the banner says it's turning off
        // until the page answers. Never over a re-wiring already on its way (the turn-on's
        // few seconds): the press waits for its answer.
        Action::AiTurnOff => {
            let from = derive(w).popover.and_then(|p| p.banner).map(|b| b.id).filter(|id| matches!(*id, "ai-turned-on" | "ai-turn-off-failed"));
            let Some(from) = from else { return (vec![], Arm::DROPPED) };
            if w.take_open() || w.inflight.ops.contains_key(&Op::Ai) {
                return (vec![], Arm::DROPPED);
            }
            w.inflight.failures.remove(&Op::Ai);
            w.space.ai_turn = Some(AiTurn::TurningOff);
            (vec![Effect::Cloud(CloudCmd::SetAi(false))], Arm { name: "ai-turn-off", edge: Some((from, "ai-turning-off")) })
        }
        Action::SetScope(scope) => {
            w.session.fields.library.scope = scope;
            (vec![], Arm::named("set-scope"))
        }
        // ── Suggested to clean up (N68) ──
        Action::ReviewCleanup => {
            w.session.fields.library.cleanup = true;
            w.session.fields.library.renaming = None;
            (vec![], Arm { name: "cleanup-opened", edge: Some(("library", "library-cleanup")) })
        }
        Action::DismissCleanup => {
            w.prefs.cleanup_dismissed = library_view::suggested(w).iter().map(|(s, _)| s.rec.0.clone()).collect();
            w.prefs.cleanup_dismissed.sort();
            (vec![persist_prefs(w)], Arm::named("cleanup-dismissed"))
        }
        Action::KeepRow(rec) => {
            w.library_ops.keeping.insert(rec.clone());
            (vec![Effect::Library(LibCmd::Keep(rec))], Arm::named("keep"))
        }
        Action::DeleteSuggested(rec) => {
            // One Undo line at a time: a delete it was holding goes now.
            let fx = send_undone(w);
            w.session.fields.library.undo = Some((rec, now));
            (fx, Arm::named("delete-suggested"))
        }
        Action::UndoDelete => {
            w.session.fields.library.undo = None;
            (vec![], Arm::named("delete-undone"))
        }
        Action::DeleteAllSuggested => {
            let targets: Vec<_> = library_view::suggested(w).into_iter().map(|(s, _)| s.rec).collect();
            if targets.is_empty() || w.online() == Some(false) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = Some(Confirm::Delete(targets));
            (vec![], Arm { name: "delete-all-asked", edge: Some(("library-cleanup", "delete-confirm-several")) })
        }
        Action::SetPlays(on) => {
            w.library_ops.plays = Some(on);
            (vec![Effect::Library(LibCmd::Plays(on))], Arm::named("set-plays"))
        }
        Action::SetSuggestCleanup(on) => {
            w.prefs.suggest_cleanup = on;
            (vec![persist_prefs(w)], Arm::named("set-suggest-cleanup"))
        }
    }
}

/// An editor window's own buttons and keys (N63): each acts on that window's editor alone
/// (N73); what isn't the editor's is the app's.
fn editor_act(w: &mut World, id: EditorId, action: Action) -> Out {
    match action {
        Action::EditorDone => editor_done(w, id),
        Action::EditorRevert => edit_step(w, id, EditStep::Op(shroom_edit::Op::Revert)),
        Action::EditorKeepEditing => {
            // Keep editing (Esc; N71): the question goes, every change still there, back to
            // the face it asked from.
            let Some(e) = w.session.fields.editors.get_mut(&id).filter(|e| e.closing) else { return (vec![], Arm::DROPPED) };
            e.closing = false;
            let edge = if e.save == EditorSave::Failed { ("editor-closing", "editor-save-failed") } else { ("editor-closing", "editor") };
            (vec![], Arm { name: "editor-close-kept", edge: Some(edge) })
        }
        Action::EditorDiscard => {
            // Discard (a click only; N71): the window closes, and what wasn't saved is
            // dropped — the video stays as it was last saved. Nothing is coming to stop.
            if !w.session.fields.editors.get(&id).is_some_and(|e| e.closing) {
                return (vec![], Arm::DROPPED);
            }
            (close_editor(w, id, false), Arm { name: "editor-discarded", edge: Some(("editor-closing", "library")) })
        }
        Action::EditorRetry => {
            // Try again: the video is asked for again — the person's retry, never one behind it.
            let Some(e) = w.session.fields.editors.get_mut(&id).filter(|e| e.media == EditorMedia::Failed) else { return (vec![], Arm::DROPPED) };
            e.media = EditorMedia::Opening;
            let rec = e.rec.clone();
            w.inflight.failures.remove(&Op::EditorOpen(id));
            (vec![Effect::Editor(EditCmd::Open { id, rec })], Arm { name: "editor-retry", edge: Some(("editor-cant-get", "editor-getting")) })
        }
        Action::EditorClose => {
            // editor-cant-get's Close (Return, Esc): the window goes, and nothing more comes.
            // After a save (B197): the window goes, and the final video is still made. After
            // the space turned the save down (B199): the window goes, nothing saved.
            let Some(e) = w.session.fields.editors.get(&id) else { return (vec![], Arm::DROPPED) };
            if crate::derive::editor::after_save(e) {
                return close_after_save(w, id);
            }
            let (edge, coming) = match crate::derive::editor::state(e) {
                "editor-cant-get" => (Some(("editor-cant-get", "library")), true),
                "editor-save-refused" => (Some(("editor-save-refused", "library")), false),
                _ => (None, true),
            };
            (close_editor(w, id, coming), Arm { name: "editor-closed", edge })
        }
        Action::DeleteOriginal => {
            // Yes, delete (B197 (c)): the edit becomes the video, and the original goes now.
            let Some(rec) = w.session.fields.editors.get(&id).filter(|e| e.save == EditorSave::Asking).map(|e| e.rec.clone()) else { return (vec![], Arm::DROPPED) };
            close_editor(w, id, false);
            (vec![Effect::Editor(EditCmd::Bake { rec })], Arm { name: "original-deleted", edge: Some(("editor-keep-original", "library")) })
        }
        Action::KeepOriginal => {
            // Keep for 2 days (Esc, ✕): the copy's landing already set when it goes.
            if !w.session.fields.editors.get(&id).is_some_and(|e| e.save == EditorSave::Asking) {
                return (vec![], Arm::DROPPED);
            }
            let (fx, arm) = close_after_save(w, id);
            (fx, Arm { name: "original-kept", ..arm })
        }
        other => act(w, other),
    }
}

/// A one-tap delete waiting on its Undo line, sent now (N68): the line ran out, another
/// tap took its place, or its window closed.
fn send_undone(w: &mut World) -> Vec<Effect> {
    let Some((rec, _)) = w.session.fields.library.undo.take() else { return vec![] };
    w.library_ops.deleting.insert(rec.clone());
    vec![Effect::Library(LibCmd::Delete(vec![rec]))]
}

/// An invite this Mac holds, from any door: a new one starts afresh. Offline, its window
/// opens at once on the offline line (DEVIATIONS #20) — never over a take (law 4).
///
/// A join already asked for (`join.json` written) is never replaced: its answer — a 200 the
/// space may already have recorded — is the one that counts. A second arrival meanwhile
/// (Open shroom clicked again) only brings its window forward (DECISIONS B41 f).
///
/// `pasted`: it came through a paste door — what the Space tab's door shows of it while
/// its window waits (DECISIONS B104).
fn hold_invite(w: &mut World, link: SecretId, host: Option<String>, opened: bool, pasted: bool) {
    if w.members.invite.as_ref().is_some_and(|i| i.joining) {
        if opened && !w.take_open() {
            open_window(w, WindowKind::Onboarding);
        }
        return;
    }
    for op in [Op::Peek, Op::Join] {
        w.inflight.ops.remove(&op);
        w.inflight.failures.remove(&op);
    }
    let mut inv = Invite { link, host, wants_window: opened, pasted, peek: Fact::Unknown, joining: false, refused: None, behind: false };
    if opened && w.online() == Some(false) && !w.take_open() {
        inv.wants_window = false;
        open_window(w, WindowKind::Onboarding);
    }
    w.members.invite = Some(inv);
}

/// The invite is behind us: `~/.shroom/invite` goes and the link leaves the stash.
fn forget_invite(w: &mut World, fx: &mut Vec<Effect>) {
    if w.members.invite.take().is_some() {
        fx.push(Effect::Member(MemberCmd::ForgetInvite));
    }
    for op in [Op::Peek, Op::Join] {
        w.inflight.ops.remove(&op);
        w.inflight.failures.remove(&op);
    }
}

/// The ops that speak for one space: a late answer after this Mac left it is stale.
const SPACE_OPS: [Op; 19] = [
    Op::Ai,
    Op::Cleanup,
    Op::Suggest,
    Op::Keep,
    Op::Plays,
    Op::Validate,
    Op::SaveToken,
    Op::Sweep,
    Op::Claim,
    Op::Wire,
    Op::LibRefresh,
    Op::Rename,
    Op::Delete,
    Op::Standing,
    Op::Roster,
    Op::Invite,
    Op::Remove,
    Op::Mark,
    Op::Peek,
];

/// join answered 200 (§4.5): this Mac is in the invite's space now. On a switch, the old
/// space's token is set aside and its records forgotten (§5) — only now: a failed join never
/// strands a Mac outside its old space.
///
/// A switch leaves nothing of the old space on this Mac's queue (law 14): the takes that
/// never reached it go the way Leave sends them — the switch said so, and offered Movies
/// first (N36's rule, DECISIONS B41 b). A take whose copy to Movies hasn't landed, or one
/// that raced in, waits in `toss`: deleted once its copy lands, set aside (kept, never
/// sent) if it can't — never deleted unsaved (law 2, DECISIONS B54). The same space joined
/// again (a re-invite, §6.3) keeps its queue: those takes drain now. The invite is behind
/// us (§5).
fn joined(w: &mut World, identity: SpaceIdentity) -> Out {
    let switch = w.space.identity.as_ref().is_some_and(|old| old.host != identity.host);
    let mut gone = Vec::new();
    if switch {
        let stranded: BTreeSet<TakeDir> = settings_view::stranded(w).into_iter().map(|q| q.dir.clone()).collect();
        let queue = std::mem::take(&mut w.queue);
        let mut save = Vec::new();
        for q in queue {
            let pending = w.members.saving.contains(&q.dir);
            if stranded.contains(&q.dir) && w.members.left_behind.contains(&q.dir) && !pending {
                gone.push(Effect::Takes(TakeCmd::Delete(q.dir)));
            } else {
                // A take still moving can't be here — the ask waits for the queue, and Start
                // greys while a switch joins — but one that raced in is copied to Movies
                // before it goes: never lost, never sent to a space it wasn't made for.
                if !pending {
                    save.push(q.dir.clone());
                }
                w.members.toss.insert(q.dir.clone(), q);
            }
        }
        if !save.is_empty() {
            gone.extend(save_to_movies(w, save));
        }
    }
    for op in SPACE_OPS.into_iter().chain([Op::Join]) {
        w.inflight.ops.remove(&op);
        w.inflight.failures.remove(&op);
    }
    if switch {
        w.inflight.ops.remove(&Op::Upload);
        w.inflight.failures.remove(&Op::Upload);
        // The editor is on a video of the space left behind (N63).
        gone.extend(leave_editor(w));
    }
    w.space = Space { identity: Some(identity.clone()), token: false, device_key: true, broken: Fact::Unknown, page_checked: None, ai: None, ai_turn: None };
    w.sweep = Sweep::default();
    w.library = Fact::Unknown;
    w.library_ops = LibraryOps::default();
    w.members = carried(w);
    w.session.fields.settings = SettingsFields::default();
    let mut fx = vec![Effect::Persist(PersistOp::Space(identity)), Effect::Member(MemberCmd::ForgetInvite)];
    fx.extend(gone);
    if w.prefs.setup_chosen {
        w.prefs.setup_chosen = false;
        fx.push(persist_prefs(w));
    }
    (fx, Arm { name: "joined", edge: Some(("joining", "member-done")) })
}

/// The space's Worker answering this Mac (Region 6, WORKER-INTERFACE §4).
fn member(w: &mut World, news: MemberNews) -> Out {
    let now = w.now;
    match news {
        MemberNews::Arrived { link, host, remembered } => {
            hold_invite(w, link, Some(host), !remembered, false);
            (vec![], Arm::named(if remembered { "invite-remembered" } else { "invite-arrived" }))
        }
        // The installer's hand-off that isn't a join link reads as the paste door reads the
        // same text: a link that doesn't work, with nobody to name (B39 b, DECISIONS B54 e).
        MemberNews::Unreadable { link, opened } => {
            if w.members.invite.as_ref().is_some_and(|i| i.joining) {
                return (vec![Effect::Member(MemberCmd::ForgetInvite)], Arm::named("invite-unreadable-ignored"));
            }
            for op in [Op::Peek, Op::Join] {
                w.inflight.ops.remove(&op);
                w.inflight.failures.remove(&op);
            }
            w.members.invite = Some(Invite { link, host: None, wants_window: false, pasted: false, peek: Fact::Unknown, joining: false, refused: Some(JoinRefusal::Expired), behind: false });
            if opened && !w.take_open() {
                open_window(w, WindowKind::Onboarding);
            }
            (vec![], Arm::named("invite-unreadable"))
        }
        MemberNews::JoinPending { link, host } => {
            // The join asked for before the crash replays — its answer, whatever it is,
            // is this Mac's (§5). Its window waits for a click, like a remembered invite.
            for op in [Op::Peek, Op::Join] {
                w.inflight.ops.remove(&op);
                w.inflight.failures.remove(&op);
            }
            w.members.invite = Some(Invite { link, host: Some(host), wants_window: false, pasted: false, peek: Fact::Unknown, joining: true, refused: None, behind: false });
            (vec![], Arm::named("join-pending"))
        }
        MemberNews::Peeked { link, peek } => {
            // An answer for a link this Mac no longer holds touches nothing — not the
            // current link's own peek in flight.
            if w.members.invite.as_ref().is_none_or(|i| i.link != link) {
                return (vec![], Arm::named("peeked-stale"));
            }
            w.inflight.ops.remove(&Op::Peek);
            w.inflight.failures.remove(&Op::Peek);
            let take_open = w.take_open();
            let inv = w.members.invite.as_mut().expect("checked above");
            let host = match &peek {
                PeekAnswer::Pending { host, .. } | PeekAnswer::Expired { host, .. } | PeekAnswer::Used { host, .. } | PeekAnswer::SpaceMissing { host } => host.clone(),
            };
            inv.host = Some(host);
            // The page caught up, or never was behind (U3).
            inv.behind = matches!(peek, PeekAnswer::Pending { current: false, .. });
            inv.peek = Fact::known(peek, now);
            // The person opened it: its window opens now that there's something to show —
            // never over a take (law 4: the take ends, and it's asked again).
            let open = inv.wants_window && !take_open;
            if open {
                inv.wants_window = false;
                open_window(w, WindowKind::Onboarding);
            }
            (vec![], Arm { name: "peeked", edge: open.then_some(("join-landing", "member-welcome")) })
        }
        MemberNews::Joined { link, answer } => {
            // A 200 for the link this Mac holds is never dropped: the space has recorded
            // this Mac (and the lane has written it down), whatever the window shows now.
            let held = w.members.invite.as_ref().is_some_and(|i| i.link == link);
            if let (true, JoinAnswer::Joined(identity)) = (held, &answer) {
                return joined(w, identity.clone());
            }
            if !held || !w.members.invite.as_ref().is_some_and(|i| i.joining) {
                return (vec![], Arm::named("joined-stale"));
            }
            w.inflight.ops.remove(&Op::Join);
            let refusal = match answer {
                JoinAnswer::Joined(_) => unreachable!("applied above"),
                // The page is older than this app needs: Join rests greyed on the
                // needs-update line until a peek finds it caught up (open question 15).
                JoinAnswer::Behind => {
                    w.inflight.failures.remove(&Op::Join);
                    if let Some(inv) = w.members.invite.as_mut() {
                        inv.joining = false;
                        inv.behind = true;
                    }
                    return (vec![], Arm { name: "join-behind", edge: Some(("joining", "member-welcome")) });
                }
                // The space isn't answering right now: the identical body replays after the
                // backoff (never straight back).
                JoinAnswer::Waiting => {
                    note_failure(w, Op::Join, FailClass::Refused);
                    return (vec![], Arm { name: "join-waiting", edge: Some(("joining", "join-waiting")) });
                }
                JoinAnswer::Expired => JoinRefusal::Expired,
                JoinAnswer::Used => JoinRefusal::Used,
                JoinAnswer::AlreadyHere => JoinRefusal::AlreadyHere,
                JoinAnswer::SpaceMissing => JoinRefusal::SpaceMissing,
            };
            w.inflight.failures.remove(&Op::Join);
            if let Some(inv) = w.members.invite.as_mut() {
                inv.joining = false;
                inv.refused = Some(refusal);
            }
            (vec![], Arm::named("join-refused"))
        }
        MemberNews::Standing(answer) => {
            w.inflight.ops.remove(&Op::Standing);
            w.inflight.failures.remove(&Op::Standing);
            // Asked before this Mac left its space: about a space it no longer knows.
            if !w.provisioned() {
                return (vec![], Arm::named("standing-stale"));
            }
            let st = &mut w.members.standing;
            st.at = Some(now);
            let arm = match answer {
                // The cause clears, the face clears (law 1).
                StandingAnswer::Fine { current, only_holder, mark, ai } => {
                    st.fine = Some(now);
                    st.revoked = false;
                    st.strike = None;
                    st.behind = !current;
                    if only_holder.is_some() {
                        st.only_holder = only_holder;
                    }
                    if mark.is_some() {
                        st.mark = mark;
                    }
                    // A Mac without the token learns its space's AI from the ping (N75): a
                    // member's popover says once whose space hears its videos.
                    if ai.is_some() && !w.space.token_holder() {
                        w.space.ai = ai;
                    }
                    "standing-fine"
                }
                // Never debounced.
                StandingAnswer::Revoked => {
                    st.revoked = true;
                    st.strike = None;
                    "standing-revoked"
                }
                // C7: one answer is a silent backoff; a face only after a second at least
                // 30 s later.
                StandingAnswer::SpaceSide | StandingAnswer::Gone => {
                    let gone = answer == StandingAnswer::Gone;
                    let was = st.strike.is_some_and(|s| s.confirmed());
                    st.strike = Some(match st.strike {
                        Some(s) if s.gone == gone => Strike { last: now, ..s },
                        _ => Strike { gone, first: now, last: now },
                    });
                    // The debounce confirms the space refuses this Mac's uploads: the owner
                    // page learns to say “Waiting for your space” (§4.18, §6.7 step 5) — once,
                    // about a take that's waiting for it.
                    let confirmed = st.strike.is_some_and(|s| s.confirmed() && !s.gone);
                    if confirmed && !was && !st.reported {
                        if let Some(q) = w.queue.first() {
                            w.members.standing.reported = true;
                            return (vec![Effect::Member(MemberCmd::Health { rec: q.rec.clone(), refused: true })], Arm::named("standing-strike"));
                        }
                    }
                    "standing-strike"
                }
            };
            (vec![], Arm::named(arm))
        }
        MemberNews::Roster(roster) => {
            answered(w, Op::Roster);
            if !w.space.owner_class() {
                return (vec![], Arm::named("roster-stale"));
            }
            w.members.roster = Fact::known(roster, now);
            (vec![], Arm::named("roster"))
        }
        MemberNews::Invited { invite, link } => {
            answered(w, Op::Invite);
            if !w.space.owner_class() {
                return (vec![], Arm::named("invited-stale"));
            }
            let born = Born { invite: invite.id.clone(), link, own_mac: invite.own_mac };
            // The same row for [ New link ] (§6.1.6); a new row for a new invite.
            let (mut roster, at) = match std::mem::take(&mut w.members.roster) {
                crate::time::Fact::Known { v, at } => (v, at),
                crate::time::Fact::Unknown => (crate::world::Roster::default(), Mono(0)),
            };
            match roster.invites.iter_mut().find(|i| i.id == invite.id) {
                Some(row) => *row = invite,
                None => roster.invites.push(invite),
            }
            w.members.roster = Fact::known(roster, at);
            // Shown once, under its row — only while the face it was asked from is open.
            if w.window_visible(WindowKind::Settings) {
                if !born.own_mac {
                    w.session.fields.settings.invite_name.clear();
                }
                w.session.fields.settings.born = Some(born);
            }
            (vec![], Arm { name: "invited", edge: Some(("invite-born", "invite-born")) })
        }
        MemberNews::Removed(id) => {
            answered(w, Op::Remove);
            if let crate::time::Fact::Known { v, .. } = &mut w.members.roster {
                v.members.retain(|m| m.id != id);
                v.invites.retain(|i| i.id != id);
                v.macs.retain(|m| m.id != id);
            }
            if w.session.fields.settings.born.as_ref().is_some_and(|b| b.invite == id) {
                w.session.fields.settings.born = None;
            }
            (vec![], Arm { name: "removed", edge: Some(("member-remove-confirm", "invite-born")) })
        }
        MemberNews::Marked(on) => {
            answered(w, Op::Mark);
            w.members.standing.mark = Some(on);
            (vec![], Arm::named("marked"))
        }
    }
}

/// Get link on a saved take: the normal stop sequence from the saved file (N25).
fn get_link(w: &mut World, dir: TakeDir) -> Out {
    let now = w.now;
    let Some(pos) = w.saved.iter().position(|s| s.dir == dir) else { return (vec![], Arm::DROPPED) };
    let saved = w.saved.remove(pos);
    w.session.last_stop = None;
    w.session.popover = None;
    w.take = Some(OpenTake {
        dir: saved.dir.clone(),
        rec: saved.rec,
        opened: now,
        source: crate::derive::popover_selected_source(w).unwrap_or(SourceId::Camera),
        phase: Phase::Finalizing(Why::GetLink),
        since: now,
        length_ms: saved.length_ms,
        markers: 0,
        marked_at_pause: false,
        mic: None,
        camera: None,
    });
    let mut fx = vec![Effect::Recorder(RecCmd::Confirm(saved.dir))];
    w.inflight.failures.remove(&Op::Reach);
    w.net.not_shroom = None;
    if w.online() != Some(false) {
        fx.push(Effect::Cloud(CloudCmd::Reach));
    }
    (fx, Arm::named("get-link"))
}

/// A window opens, or the open one comes forward (one instance each): it keeps the
/// instant it opened (law 4's visibility); entering it re-probes (law 1).
fn open_window(w: &mut World, kind: WindowKind) {
    let now = w.now;
    w.session
        .windows
        .entry(kind)
        .and_modify(|d| {
            d.entered = now;
            d.asked = true;
        })
        .or_insert(Door::asked(now));
}

/// A paste the repair mood already turned down (garbled, wrong powers, another account)
/// is behind the person once they leave Settings or come back to it: the next entry
/// re-probes from scratch and reads the token row afresh (law 1, N36).
fn forget_failed_paste(w: &mut World) {
    if w.provisioned() && matches!(w.sweep.paste, Some(Paste::Garbled | Paste::WrongPowers | Paste::OtherAccount)) {
        w.sweep.paste = None;
    }
}

/// Settings opens on the last tab used, or on the one a door asks for.
fn open_settings(w: &mut World, tab: Option<SettingsTab>, fx: &mut Vec<Effect>) {
    forget_failed_paste(w);
    if let Some(t) = tab {
        if w.prefs.settings_tab != Some(t) {
            w.prefs.settings_tab = Some(t);
            fx.push(persist_prefs(w));
        }
    }
    open_window(w, WindowKind::Settings);
}

/// Delete… asks first, always (law 9) — and only for published videos, online.
fn ask_delete(w: &mut World, targets: Vec<crate::world::RecId>) -> Out {
    if targets.is_empty() || w.online() == Some(false) {
        return (vec![], Arm::DROPPED);
    }
    w.session.fields.library.renaming = None;
    w.session.confirm = Some(Confirm::Delete(targets));
    (vec![], Arm { name: "delete-asked", edge: Some(("library", "delete-confirm")) })
}

/// Disconnect: the `~/.shroom` secrets are wiped and the app is back at
/// `idle-unprovisioned`; the takes, the prefs and the login item stay (N36).
fn disconnect(w: &mut World) -> Out {
    w.session.confirm = None;
    // Leaving deletes the takes that never reached the space — the sheet said so, and
    // offered Movies first (N36). Nothing else of the person's goes.
    let mut gone = Vec::new();
    for q in settings_view::stranded(w).into_iter().cloned().collect::<Vec<_>>() {
        w.queue.retain(|x| x.dir != q.dir);
        gone.push(Effect::Takes(TakeCmd::Delete(q.dir)));
    }
    // What was under way for this space ends with it: a late answer is stale, never a
    // space set up again behind the person's back.
    for op in SPACE_OPS.into_iter().chain([Op::Upload, Op::Join]) {
        w.inflight.ops.remove(&op);
        w.inflight.failures.remove(&op);
    }
    gone.extend(leave_editor(w));
    w.space = Space::default();
    // The invite, the roster and this Mac's standing go with the space (§5's exit recipe);
    // what this Mac's takes are waiting on (a copy to Movies) stays with them.
    w.members = Members { movies: BTreeSet::new(), ..carried(w) };
    w.sweep = Sweep::default();
    w.library = Fact::Unknown;
    w.library_ops = LibraryOps::default();
    w.session.windows.clear();
    w.session.popover = None;
    w.session.chapter = false;
    w.session.fork = Fork::Virgin;
    // The editor windows' numbers go on: none is ever given another's (N73).
    w.session.fields = Fields { editors_opened: w.session.fields.editors_opened, ..Fields::default() };
    let mut fx = vec![Effect::Persist(PersistOp::Wipe)];
    fx.extend(gone);
    if w.prefs.setup_chosen {
        w.prefs.setup_chosen = false;
        fx.push(persist_prefs(w));
    }
    (fx, Arm { name: "disconnect", edge: Some(("disconnect-confirm", "idle-unprovisioned")) })
}

/// Where the onboarding window stands, if it shows.
fn onboarding_at(w: &World) -> Option<At> {
    w.window_visible(WindowKind::Onboarding).then(|| onboarding::at(w))
}

fn resume_chapter(w: &mut World, window: WindowKind) -> Arm {
    w.session.windows.insert(window, Door::asked(w.now));
    w.session.chapter = true;
    Arm { name: "permission-relaunch", edge: Some(("relaunching", "perm-screen")) }
}

/// The login item is registered when the welcome is dismissed — either button or ✕ —
/// iff its box is still checked (U4/N12); never silently.
/// Both ways (U4: the box's value is applied at dismissal): an unchecked box on a Mac whose
/// login item is still on (a welcome after Disconnect or Leave, B34/N36) switches it off.
fn register_login(w: &World, fx: &mut Vec<Effect>) {
    let want = !w.session.fields.login_off;
    let differs = if want { !w.mac.login_item.is(&true) } else { w.mac.login_item.is(&true) };
    if differs {
        fx.push(Effect::Shell(ShellCmd::SetLoginItem(want)));
    }
}

/// Later (and the welcome's ✕, exactly): back to the tray.
fn later(w: &mut World, fx: &mut Vec<Effect>) {
    register_login(w, fx);
    if w.prefs.setup_chosen {
        w.prefs.setup_chosen = false;
        fx.push(persist_prefs(w));
    }
    leave_onboarding(w);
}

/// The onboarding window goes: what was asked in it goes with it — the fork too, so
/// welcome → Set up my space always lands on the clean question (`account`).
fn leave_onboarding(w: &mut World) {
    w.session.windows.remove(&WindowKind::Onboarding);
    w.session.chapter = false;
    w.session.fork = Fork::Virgin;
    // The Library's, Settings', Send feedback's and the editors' fields are their windows';
    // only the onboarding's go. A press on the page's Edit in shroom isn't the setup's.
    let library = std::mem::take(&mut w.session.fields.library);
    let settings = std::mem::take(&mut w.session.fields.settings);
    let feedback = std::mem::take(&mut w.session.fields.feedback);
    let editors = std::mem::take(&mut w.session.fields.editors);
    let (editors_opened, leaving) = (w.session.fields.editors_opened, w.session.fields.leaving.take());
    let edit_link = w.session.fields.edit_link.take();
    w.session.fields = Fields { library, settings, feedback, editors, editors_opened, leaving, edit_link, ..Fields::default() };
    w.mac.mic_asked = false;
    w.mac.camera_asked = false;
}

/// The camera is asked in the chapter (N43) when macOS hasn't been asked yet and this Mac
/// has one — no prompt for hardware that isn't there.
fn camera_unasked(w: &World) -> bool {
    w.mac.camera.get().is_none_or(|c| *c == Access::NotAsked) && !w.mac.cameras.get().is_some_and(|c| c.is_empty())
}

/// The screen step put to macOS: how it took it (its own alert, or the pane shroom opens)
/// comes back as `ScreenAsked` (N43).
fn ask_screen(w: &mut World, fx: &mut Vec<Effect>) {
    w.mac.screen_asked = true;
    w.mac.screen_alert = None;
    fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Screen)));
}

/// The onboarding window's ✕: ≡ Later on the welcome, ≡ Close at all-set; on the plan it
/// is always safe (state is derived, every call idempotent).
fn close_onboarding(w: &mut World) -> Out {
    let mut fx = Vec::new();
    let at = onboarding::at(w);
    let edge = match at {
        At::Welcome => {
            // An unanswered Uninstall sheet goes with its window (B160).
            if w.session.confirm == Some(Confirm::Uninstall { going: false }) {
                w.session.confirm = None;
            }
            later(w, &mut fx);
            return (fx, Arm { name: "welcome-closed", edge: Some(("welcome", "idle-unprovisioned")) });
        }
        // ✕ ≡ Later: the invite is remembered (N29).
        At::MemberWelcome => {
            later(w, &mut fx);
            return (fx, Arm { name: "member-welcome-closed", edge: Some(("member-welcome", "idle-unprovisioned")) });
        }
        // A dead invite's face shown once, the switch's ✕ ≡ Stay, join-already-here's Close:
        // the invite is behind us (B39 c).
        At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing | At::AlreadyHere | At::SwitchConfirm => {
            forget_invite(w, &mut fx);
            None
        }
        At::AllSet => Some(("all-set", "idle-ready")),
        _ => None,
    };
    if w.prefs.setup_chosen {
        w.prefs.setup_chosen = false;
        fx.push(persist_prefs(w));
    }
    leave_onboarding(w);
    (fx, Arm { name: "onboarding-closed", edge })
}

/// A secret landed in a paste field: only its handle and shape reach the core (law 13).
fn pasted(w: &mut World, field: SecretField, id: crate::world::SecretId, shape: Shape) -> Out {
    let scene = derive(w);
    let takes = scene.onboarding.as_ref().is_some_and(|o| o.takes_paste(field)) || scene.settings.as_ref().is_some_and(|s| s.takes_paste(field));
    if !takes {
        return (vec![], Arm::DROPPED);
    }
    let welcome_door = matches!(scene.onboarding.as_ref().map(|o| &o.body), Some(crate::scene::OnboardingBody::Welcome(_)));
    match (field, shape) {
        (SecretField::Token, Shape::Token) => {
            w.sweep.paste = Some(Paste::Checking(id));
            (vec![], Arm::named("token-pasted"))
        }
        (SecretField::Token, _) => {
            w.sweep.paste = Some(Paste::Garbled);
            (vec![], Arm { name: "token-garbled", edge: Some(("token", "token-garbled")) })
        }
        // A Cloudflare token pasted in the welcome (no door shows, B159): the wizard, at
        // `token`, the paste landed (N30) — it proves the account, so box 1 ticks. Anything
        // else pasted there is nothing.
        (SecretField::Invite, Shape::Token) if welcome_door => {
            let mut fx = Vec::new();
            register_login(w, &mut fx);
            w.prefs.setup_chosen = true;
            w.session.fork = Fork::Affirmed;
            w.sweep.paste = Some(Paste::Checking(id));
            fx.push(persist_prefs(w));
            (fx, Arm { name: "invite-door-token", edge: Some(("welcome", "token")) })
        }
        // A join link, pasted in the welcome or Settings · Space (anywhere, B159) or a dead
        // invite's door:
        // the same machine as the page's Open shroom (N29, N30).
        (SecretField::Invite, Shape::JoinLink) => {
            hold_invite(w, id, None, true, true);
            (vec![], Arm::named("invite-pasted"))
        }
        (SecretField::Invite, _) => (vec![], Arm::DROPPED),
    }
}

/// A failure of one op: its backoff grows (5 s doubling, 2 min cap).
fn note_failure(w: &mut World, op: Op, class: FailClass) {
    let tries = w.inflight.failures.get(&op).map(|f| f.tries.saturating_add(1)).unwrap_or(0);
    w.inflight.failures.insert(op, Failure { at: w.now, class, tries });
    if op == Op::UpdateCheck {
        w.update.tried = Some(w.wall);
    }
}

/// An op answered: it's no longer in flight, and its failures are behind it. A wizard
/// call answering proves the network is back, so every wizard transport failure before it
/// is behind us too — `wizard-offline` lasts only while the newest failure is Transport and
/// newer than the newest success (ARCHITECTURE §3.3).
fn answered(w: &mut World, op: Op) {
    w.inflight.ops.remove(&op);
    w.inflight.failures.remove(&op);
    if Op::WIZARD.contains(&op) {
        w.inflight.failures.retain(|o, f| !(Op::WIZARD.contains(o) && f.class == FailClass::Transport));
    }
}

/// A window's ✕ — or ⌘Q with the window in front and no take (B145).
fn window_closed(w: &mut World, kind: WindowKind) -> Out {
    match kind {
        WindowKind::Onboarding => close_onboarding(w),
        WindowKind::Library => {
            w.session.windows.remove(&WindowKind::Library);
            // A one-tap delete still on its Undo line goes: the tap meant delete (N68).
            let fx = send_undone(w);
            w.session.fields.library = LibraryFields::default();
            if matches!(w.session.confirm, Some(Confirm::Delete(_) | Confirm::DiscardRow(_))) {
                w.session.confirm = None;
            }
            (fx, Arm::named("library-closed"))
        }
        WindowKind::Settings => {
            w.session.windows.remove(&WindowKind::Settings);
            // A report sheet goes with its window; one on its way still goes (B180).
            if matches!(w.session.confirm, Some(Confirm::Exit | Confirm::Remove(_) | Confirm::Report { .. })) {
                w.session.confirm = None;
            }
            // The born link is shown once (§6.1.7); the composer starts empty next time.
            w.session.fields.settings = SettingsFields::default();
            forget_failed_paste(w);
            // The chapter ran in this window; the onboarding window keeps its own.
            if !w.session.windows.contains_key(&WindowKind::Onboarding) {
                w.session.chapter = false;
            }
            (vec![], Arm::named("settings-closed"))
        }
        WindowKind::Feedback => {
            // The words go with the window; a send on its way still goes, and its answer
            // finds no one waiting.
            w.session.windows.remove(&WindowKind::Feedback);
            w.session.fields.feedback = FeedbackFields::default();
            (vec![], Arm::named("feedback-closed"))
        }
        // An editor's ✕ asks first with changes not saved (N71); otherwise it's Done.
        WindowKind::Editor(id) => editor_close(w, id),
    }
}

// ── the editor (N63, B195) ──

/// Edit… on a live row: a window of its own opens on it, in front, and asks for its record
/// and its video — or, its window open already, that window comes forward as it is (N73).
/// Every other window stays as it is.
fn open_editor(w: &mut World, rec: RecId) -> Out {
    if let Some(id) = editor_of(w, &rec) {
        return (forward_editor(w, id), Arm::named("editor-forward"));
    }
    let Some(row) = library_view::rows(w).into_iter().find(|r| r.rec == rec && r.kind == library_view::Kind::Live) else { return (vec![], Arm::DROPPED) };
    open_editor_on(w, row)
}

/// The editor window open on `rec`, if one is: one window per video (N73).
fn editor_of(w: &World, rec: &RecId) -> Option<EditorId> {
    w.session.fields.editors.iter().find(|(_, e)| &e.rec == rec).map(|(id, _)| *id)
}

/// An editor window comes forward as it is, with the keyboard.
fn forward_editor(w: &mut World, id: EditorId) -> Vec<Effect> {
    open_window(w, WindowKind::Editor(id));
    vec![Effect::Shell(ShellCmd::OpenEditor(id))]
}

/// An editor window opening on a live row, under the next number (N73): Edit…'s, and the
/// page's Edit in shroom's (N69).
fn open_editor_on(w: &mut World, row: library_view::Row) -> Out {
    let rec = row.rec.clone();
    w.session.fields.editors_opened += 1;
    let id = EditorId(w.session.fields.editors_opened);
    open_window(w, WindowKind::Editor(id));
    w.session.fields.editors.insert(
        id,
        EditorFields {
            rec: rec.clone(),
            title: row.title.unwrap_or_default(),
            length_ms: row.length_ms,
            source_duration: row.length_ms.map_or(0.0, |ms| ms as f64 / 1000.0),
            markers: Vec::new(),
            history: None,
            rev: 0,
            kept_until: None,
            media: EditorMedia::Opening,
            save: EditorSave::Idle,
            closing: false,
        },
    );
    (vec![Effect::Editor(EditCmd::Open { id, rec }), Effect::Shell(ShellCmd::OpenEditor(id))], Arm { name: "open-editor", edge: Some(("library", "editor")) })
}

/// The owner page's Edit in shroom (N69, B221 (c)): it only asks — the Mac decides by its
/// own state, in this order.
fn edit_link(w: &mut World, host: String, rec: RecId) -> Out {
    // Not set up: the welcome window, as a click on the mushroom opens it.
    if !w.provisioned() {
        w.session.fields.edit_link = None;
        return click(w, None);
    }
    // Nothing new while a take is open (law 4): the latest press waits for it to end.
    if w.take_open() {
        w.session.fields.edit_link = Some(EditLink::Waiting { host, rec, asked: false });
        return (vec![], Arm::named("edit-link-held"));
    }
    w.session.fields.edit_link = None;
    let ours = w.space.identity.as_ref().is_some_and(|i| i.host.eq_ignore_ascii_case(host.trim_end_matches('.')));
    if !ours {
        return edit_link_no(w, EditLinkNo::OtherSpace);
    }
    // The video's window open: it comes forward as it is (N73).
    if let Some(id) = editor_of(w, &rec) {
        return (forward_editor(w, id), Arm { name: "edit-link-forward", edge: Some(("owner-page", "editor")) });
    }
    if let Some(row) = library_view::editable(w, &rec) {
        let (fx, _) = open_editor_on(w, row);
        return (fx, Arm { name: "edit-link-open", edge: Some(("owner-page", "editor")) });
    }
    // Not among the videos this Mac knows: the space's own list decides, asked once.
    w.session.fields.edit_link = Some(EditLink::Waiting { host, rec, asked: true });
    let mut fx = Vec::new();
    if w.library.get().is_none() && !w.inflight.ops.contains_key(&Op::LibCache) {
        fx.push(Effect::Library(LibCmd::LoadCache));
    }
    if !w.inflight.ops.contains_key(&Op::LibRefresh) {
        fx.push(Effect::Library(LibCmd::Refresh));
    }
    (fx, Arm::named("edit-link-asking"))
}

/// The Library answered while the page's press waited on it (N69): there now, it opens;
/// still not there once the space has answered, the popover says why.
fn edit_link_answered(w: &mut World, fresh: bool) -> Vec<Effect> {
    let Some(EditLink::Waiting { host, rec, asked: true }) = w.session.fields.edit_link.clone() else { return vec![] };
    // A take began meanwhile: it waits for that too (law 4), and asks again after.
    if w.take_open() {
        w.session.fields.edit_link = Some(EditLink::Waiting { host, rec, asked: false });
        return vec![];
    }
    // Its window open since (one window per video, N73): forward, as it is.
    if let Some(id) = editor_of(w, &rec) {
        w.session.fields.edit_link = None;
        return forward_editor(w, id);
    }
    if let Some(row) = library_view::editable(w, &rec) {
        w.session.fields.edit_link = None;
        return open_editor_on(w, row).0;
    }
    if !fresh {
        return vec![];
    }
    // An owner's Mac lists every video in the space; a member's, only its own.
    let no = if w.space.owner_class() { EditLinkNo::Gone } else { EditLinkNo::NotYours };
    edit_link_no(w, no).0
}

/// Why this Mac can't edit the video the page asked for (N69): the popover opens by itself
/// under the mushroom, without the keyboard (law 5), put away by a click elsewhere, and says
/// it once in its status line.
fn edit_link_no(w: &mut World, no: EditLinkNo) -> Out {
    if w.session.popover.is_none() {
        if derive(w).tray.click != Click::OpenPopover {
            w.session.fields.edit_link = None;
            return (vec![], Arm::named("edit-link-unsaid"));
        }
        let door = Door::unasked(w.now);
        w.session.popover = Some(door);
        w.session.news = Some(door.opened);
    }
    w.session.fields.edit_link = Some(EditLink::Said(no));
    (vec![], Arm { name: "edit-link-no", edge: Some(("owner-page", "config")) })
}

/// The editor window open on `rec`, and its fields: news about a video no window holds is
/// stale. There's one window per video (N73).
fn editor_on<'a>(w: &'a mut World, rec: &RecId) -> Option<(EditorId, &'a mut EditorFields)> {
    w.session.fields.editors.iter_mut().find(|(_, e)| &e.rec == rec).map(|(id, e)| (*id, e))
}

/// Whether any editor window's save is on its way.
fn editor_saving(w: &World) -> bool {
    w.session.fields.editors.values().any(|e| e.save == EditorSave::Saving)
}

/// An editor window closes: its fields and its door go. `cancel` stops what its open asked
/// for (a video still coming) and lets the lane forget the video.
fn close_editor(w: &mut World, id: EditorId, cancel: bool) -> Vec<Effect> {
    w.session.windows.remove(&WindowKind::Editor(id));
    w.inflight.ops.remove(&Op::EditorOpen(id));
    w.inflight.failures.remove(&Op::EditorOpen(id));
    match w.session.fields.editors.remove(&id) {
        Some(e) if cancel => vec![Effect::Editor(EditCmd::Cancel { id, rec: e.rec })],
        _ => vec![],
    }
}

/// After a save (B197): Close, Esc, ✕ or Keep for 2 days — the window goes, and nothing
/// stops: a final video still being made is finished behind it, and the original is kept
/// until the two days its landing set are up.
fn close_after_save(w: &mut World, id: EditorId) -> Out {
    let from = w.session.fields.editors.get(&id).map(crate::derive::editor::state);
    close_editor(w, id, false);
    let edge = match from {
        Some("editor-making") => Some(("editor-making", "library")),
        Some("editor-making-failed") => Some(("editor-making-failed", "library")),
        Some("editor-keep-original") => Some(("editor-keep-original", "library")),
        _ => None,
    };
    (vec![], Arm { name: "editor-closed", edge })
}

/// The space is left (Disconnect, Leave, a switch): every editor window closes with it, a
/// video still coming stops, and a save, a copy or an upkeep in flight was the old space's —
/// its answer is stale, and nothing waits on it.
fn leave_editor(w: &mut World) -> Vec<Effect> {
    let mut fx = Vec::new();
    for id in w.session.fields.editors.keys().copied().collect::<Vec<_>>() {
        let coming = w.session.fields.editors.get(&id).is_some_and(|e| !crate::derive::editor::ready(e));
        fx.extend(close_editor(w, id, coming));
    }
    let lane = |op: &Op| matches!(op, Op::EditorSave(_) | Op::EditorCopy | Op::EditorUpkeep);
    w.inflight.ops.retain(|op, _| !lane(op));
    w.inflight.failures.retain(|op, _| !lane(op));
    w.session.fields.leaving = None;
    fx
}

/// A video an editor window holds is deleted — Delete… confirmed here, or this Mac's delete
/// landing (B199): that window closes, and no other (N73); what its open asked for stops, and
/// nothing is saved — a save on its way was for a video that's gone, and its answer is stale.
/// A quit or an update that waited on it, and on no other window's save, is dropped.
fn editor_video_gone(w: &mut World, recs: &[RecId]) -> Vec<Effect> {
    let gone: Vec<EditorId> = w.session.fields.editors.iter().filter(|(_, e)| recs.contains(&e.rec)).map(|(id, _)| *id).collect();
    let mut fx = Vec::new();
    for id in gone {
        w.inflight.ops.remove(&Op::EditorSave(id));
        w.inflight.failures.remove(&Op::EditorSave(id));
        fx.extend(close_editor(w, id, true));
    }
    if !editor_saving(w) {
        w.session.fields.leaving = None;
    }
    fx
}

/// An editor window's ✕ (N71): with changes not saved it asks first — Save, Keep editing or
/// Discard — the edit held behind the question, and ✕ again leaves the question as it is.
/// Otherwise it is Done: with nothing to save the window closes, while the edit saves it
/// waits, and after a save it is Close.
fn editor_close(w: &mut World, id: EditorId) -> Out {
    if let Some(e) = w.session.fields.editors.get_mut(&id) {
        if e.closing {
            return (vec![], Arm::DROPPED);
        }
        if crate::derive::editor::editable(e) && e.history.as_ref().is_some_and(|h| h.dirty()) {
            let edge = if e.save == EditorSave::Failed { ("editor-save-failed", "editor-closing") } else { ("editor", "editor-closing") };
            e.closing = true;
            return (vec![], Arm { name: "editor-close-asked", edge: Some(edge) });
        }
    }
    editor_done(w, id)
}

/// Done — its button, Return, the close prompt's Save (N71), or the window's ✕ when there's
/// nothing to ask about: with no video to save over, the window
/// closes at once and what's coming stops; with nothing changed it closes; with changes, the
/// checked list is saved as the next revision, and the window stays until the save answers.
/// While it saves, nothing. After the save (the window's ✕ only: Done is gone by then), it
/// closes and leaves the original for two days (B197 (b)); after the space turned it down,
/// it closes and nothing is saved (B199). Each window's Done is its own (N73).
fn editor_done(w: &mut World, id: EditorId) -> Out {
    let Some(e) = w.session.fields.editors.get(&id) else {
        w.session.windows.remove(&WindowKind::Editor(id));
        return (vec![], Arm::DROPPED);
    };
    let face = crate::derive::editor::state(e);
    if e.save == EditorSave::Saving {
        return (vec![], Arm::named("editor-done-while-saving"));
    }
    if crate::derive::editor::after_save(e) {
        return close_after_save(w, id);
    }
    if e.save == EditorSave::Refused {
        return (close_editor(w, id, false), Arm { name: "editor-closed", edge: Some(("editor-save-refused", "library")) });
    }
    let Some(h) = e.history.as_ref().filter(|_| crate::derive::editor::ready(e)) else {
        let edge = match face {
            "editor" => Some(("editor", "library")),
            "editor-cant-get" => Some(("editor-cant-get", "library")),
            _ => None,
        };
        return (close_editor(w, id, true), Arm { name: "editor-closed", edge });
    };
    if !h.dirty() {
        let edge = (face == "editor").then_some(("editor", "library"));
        return (close_editor(w, id, false), Arm { name: "editor-closed", edge });
    }
    let saved = shroom_edit::Saved::new(h.now.clone().checked(), e.rev + 1);
    let rec = e.rec.clone();
    if let Some(e) = w.session.fields.editors.get_mut(&id) {
        e.save = EditorSave::Saving;
        e.closing = false;
    }
    w.inflight.failures.remove(&Op::EditorSave(id));
    let edge = match face {
        // The close prompt's Save (N71).
        "editor-closing" => ("editor-closing", "editor-saving"),
        "editor-save-failed" => ("editor-save-failed", "editor-saving"),
        _ => ("editor", "editor-saving"),
    };
    (vec![Effect::Editor(EditCmd::Save { id, rec, saved })], Arm { name: "editor-saving", edge: Some(edge) })
}

/// A step from an editor window's webview, or its Revert button: taken while that editor can
/// take steps (the video here, the record read, nothing saving), else dropped; the edit model
/// refuses what changes nothing or leaves too little (`edit-refused`).
fn edit_step(w: &mut World, id: EditorId, step: EditStep) -> Out {
    let open = w.window_visible(WindowKind::Editor(id));
    let Some(e) = w.session.fields.editors.get_mut(&id) else { return (vec![], Arm::DROPPED) };
    // While ✕ asks, the edit is held (N71).
    if e.closing {
        return (vec![], Arm::DROPPED);
    }
    let Some(h) = Some(e).filter(|e| open && crate::derive::editor::editable(e)).and_then(|e| e.history.as_mut()) else {
        return (vec![], Arm::DROPPED);
    };
    let taken = match &step {
        EditStep::Op(op) => h.act(op),
        EditStep::Undo => h.undo(),
        EditStep::Redo => h.redo(),
    };
    // Named by the step, so the run log says what was done (DECISIONS B199 (t)).
    let name = match &step {
        _ if !taken => "edit-refused",
        EditStep::Op(shroom_edit::Op::Trim { .. }) => "edit-trim",
        EditStep::Op(shroom_edit::Op::Cut { .. }) => "edit-cut",
        EditStep::Op(shroom_edit::Op::PutBack { .. }) => "edit-put-back",
        EditStep::Op(shroom_edit::Op::Apply { .. }) => "edit-apply",
        EditStep::Op(shroom_edit::Op::Resize { .. }) => "edit-resize",
        EditStep::Op(shroom_edit::Op::Revert) => "edit-revert",
        EditStep::Undo => "edit-undo",
        EditStep::Redo => "edit-redo",
    };
    (vec![], Arm::named(name))
}

/// The editor's lane (N63): each answer lands on the editor open on its video, else it's stale.
fn editor_news(w: &mut World, news: EditorNews) -> Out {
    let now = w.now;
    let stale = || (vec![], Arm::named("editor-stale"));
    let seconds = |d: f64| (d.is_finite() && d > 0.0).then_some(d);
    match news {
        EditorNews::Opened { rec, saved, markers, duration_sec, keep_until } => {
            // Read once per open: a second reading over steps already taken changes nothing.
            let Some((_, e)) = editor_on(w, &rec).filter(|(_, e)| e.history.is_none() || matches!(e.media, EditorMedia::Opening | EditorMedia::Getting { .. })) else {
                return stale();
            };
            let duration = duration_sec
                .and_then(seconds)
                .or_else(|| saved.as_ref().and_then(|s| seconds(s.list.source_duration)))
                .or_else(|| e.length_ms.and_then(|ms| seconds(ms as f64 / 1000.0)))
                .or_else(|| if let EditorMedia::Ready { duration } = e.media { seconds(duration) } else { None })
                .unwrap_or(0.0);
            e.rev = saved.as_ref().map_or(0, |s| s.rev);
            // The original is kept beside a copy only while that copy is the saved edit's
            // (B197): a stale one is no one's to count down.
            e.kept_until = keep_until.filter(|_| saved.as_ref().is_some_and(|s| s.current_copy().is_some()));
            let mut list = saved.map_or_else(|| shroom_edit::EditList::new(duration), |s| s.list);
            if seconds(list.source_duration).is_none() {
                list.source_duration = duration;
            }
            e.source_duration = duration;
            e.markers = markers.into_iter().filter(|m| m.is_finite() && *m >= 0.0).collect();
            e.history = Some(shroom_edit::History::new(list));
            (vec![], Arm::named("editor-opened"))
        }
        EditorNews::Getting { rec, pct } => {
            let Some((id, e)) = editor_on(w, &rec).filter(|(_, e)| matches!(e.media, EditorMedia::Opening | EditorMedia::Getting { .. })) else { return stale() };
            e.media = EditorMedia::Getting { pct: pct.min(100) };
            // Bytes moving keep the open alive: a long video isn't cut off mid-way.
            let op = Op::EditorOpen(id);
            if w.inflight.ops.contains_key(&op) {
                w.inflight.ops.insert(op, now.plus(op.deadline_ms()));
            }
            (vec![], Arm::named("editor-getting"))
        }
        EditorNews::Ready { rec, duration } => {
            let Some((id, e)) = editor_on(w, &rec).filter(|(_, e)| matches!(e.media, EditorMedia::Opening | EditorMedia::Getting { .. })) else { return stale() };
            let came = matches!(e.media, EditorMedia::Getting { .. });
            let duration = seconds(duration).unwrap_or(0.0);
            e.media = EditorMedia::Ready { duration };
            // A record that knew no length takes the video's own (no step could be taken yet).
            if e.source_duration <= 0.0 {
                e.source_duration = duration;
            }
            if duration > 0.0 && e.history.as_ref().is_some_and(|h| h.saved.source_duration <= 0.0) {
                e.history = Some(shroom_edit::History::new(shroom_edit::EditList::new(duration)));
            }
            answered(w, Op::EditorOpen(id));
            (vec![], Arm { name: "editor-ready", edge: came.then_some(("editor-getting", "editor")) })
        }
        EditorNews::Saved { rec, rev } => {
            let Some((id, e)) = editor_on(w, &rec).filter(|(_, e)| e.save == EditorSave::Saving) else { return stale() };
            e.rev = rev;
            // Whatever copy the record held is an older revision's now.
            e.kept_until = None;
            let list = e.history.as_mut().map(|h| {
                h.saved_as_now();
                h.now.clone()
            });
            answered(w, Op::EditorSave(id));
            // The row reads what plays now, at once; the permanent copy of this revision is
            // owed — none for a list back to the original (the lane dropped the old one).
            let edited_ms = list.as_ref().filter(|l| !l.is_identity()).map(|l| (l.edited_duration() * 1000.0).round() as u64);
            w.library_ops.edited.insert(rec.clone(), (edited_ms, now));
            if let Fact::Known { v, .. } = &mut w.library {
                if let Some(r) = Arc::make_mut(v).iter_mut().find(|r| r.rec == rec) {
                    r.edited_ms = edited_ms;
                }
            }
            let edge = if edited_ms.is_none() {
                // Nothing left out any more: the window closes.
                w.library_ops.copies.remove(&rec);
                close_editor(w, id, false);
                ("editor-saving", "library")
            } else {
                // Something left out (B197 (a)): the window stays and makes the final video —
                // asked as any owed copy is, one at a time and never over a take.
                if let Some(e) = w.session.fields.editors.get_mut(&id) {
                    e.save = EditorSave::Making { pct: 0 };
                }
                w.library_ops.copies.insert(rec, CopyOwed { rev, asked: false });
                ("editor-saving", "editor-making")
            };
            // A quit or an update that waited on the saves goes on once none is on its way
            // (B199).
            let leaving = if editor_saving(w) { None } else { w.session.fields.leaving.take() };
            (go_on(w, leaving), Arm { name: "editor-saved", edge: Some(edge) })
        }
        EditorNews::Making { rec, pct } => {
            // Bytes moving keep the copy alive: a long one isn't cut off mid-way — unless
            // it's news of another video's than the one being cut.
            let another = w.library_ops.copies.iter().any(|(r, c)| c.asked && *r != rec);
            if another || !w.inflight.ops.contains_key(&Op::EditorCopy) {
                return stale();
            }
            w.inflight.ops.insert(Op::EditorCopy, now.plus(Op::EditorCopy.deadline_ms()));
            // The window fills with the copy it waits for: its own revision's, being cut.
            let cutting = w.library_ops.copies.get(&rec).copied().filter(|c| c.asked);
            let Some((_, e)) = editor_on(w, &rec).filter(|(_, e)| matches!(e.save, EditorSave::Making { .. }) && cutting.is_some_and(|c| c.rev == e.rev)) else {
                return (vec![], Arm::named("editor-making-unseen"));
            };
            e.save = EditorSave::Making { pct: pct.min(100) };
            (vec![], Arm::named("editor-making"))
        }
        EditorNews::CopyDone { rec, rev, named } => {
            // Copies go one at a time: news of another video's, while this one is cut (a
            // late answer from one that ran out of time), leaves the running one alone.
            if w.library_ops.copies.iter().any(|(r, c)| c.asked && *r != rec) {
                return stale();
            }
            answered(w, Op::EditorCopy);
            // A newer save of the same video while it ran is still owed.
            if w.library_ops.copies.get(&rec).is_some_and(|c| c.asked && c.rev <= rev) {
                w.library_ops.copies.remove(&rec);
            }
            if let Some((_, e)) = editor_on(w, &rec).filter(|(_, e)| matches!(e.save, EditorSave::Making { .. }) && e.rev == rev) {
                // The final video the window is making is up: it asks about the original
                // (B197 (b)).
                if named {
                    e.save = EditorSave::Asking;
                    return (vec![], Arm { name: "editor-copied", edge: Some(("editor-making", "editor-keep-original")) });
                }
                // Dropped, never named — the record moved on first: there's no final video to
                // ask about, and the window says it couldn't be made (B199); an upkeep makes
                // the newer revision's.
                e.save = EditorSave::MakingFailed;
                return (vec![], Arm { name: "editor-copy-dropped", edge: Some(("editor-making", "editor-making-failed")) });
            }
            (vec![], Arm::named("editor-copied"))
        }
        EditorNews::UpkeepDone { .. } => {
            // Nothing on screen moves: the next one is due `UPKEEP_MS` from now. One asked
            // before this Mac left its space is stale.
            if !w.inflight.ops.contains_key(&Op::EditorUpkeep) {
                return stale();
            }
            answered(w, Op::EditorUpkeep);
            w.library_ops.upkept = Some(now);
            (vec![], Arm::named("editor-upkept"))
        }
    }
}

fn quit_requested(w: &mut World, via: QuitVia) -> Out {
    // Nothing that ends the app runs mid-take (law 4): declined silently while recording,
    // the paused popover (with its note) while paused.
    if let Some(t) = &w.take {
        if matches!(t.phase, Phase::Paused(_)) {
            w.session.popover = Some(Door::asked(w.now));
            return (vec![], Arm::named("quit-declined-paused"));
        }
        return (vec![], Arm::named("quit-declined"));
    }
    if w.session.arming.is_some() {
        return (vec![], Arm::named("quit-declined"));
    }
    // ⌘Q with one of shroom's windows in front closes that window, as its ✕ does; shroom
    // stays in the menu bar (B145). The Dock's Quit, the popover's and the right-click
    // menu's still quit.
    if via == QuitVia::CmdQ {
        if let Some(kind) = w.front_window() {
            return window_closed(w, kind);
        }
    }
    quit(w)
}

/// The quit goes on: uploads still draining ask first; then shroom quits — once an edit the
/// editor holds is saved (B199).
fn quit(w: &mut World) -> Out {
    if !w.queue.is_empty() {
        // The edge's name only (the trace): from config's footer when the popover shows,
        // from idle-ready's right-click or ⌘Q otherwise (map: config → quit-confirm).
        let from = if w.session.popover.is_some() { "config" } else { "idle-ready" };
        w.session.popover = Some(Door::asked(w.now));
        w.session.confirm = Some(Confirm::Quit);
        return (vec![], Arm { name: "quit-asked", edge: Some((from, "quit-confirm")) });
    }
    quit_now(w, Leaving::Quit, "quit")
}

/// shroom quits — unless the editor holds an edit not saved yet: it's saved first, and the
/// quit waits on it (B199).
fn quit_now(w: &mut World, leaving: Leaving, name: &'static str) -> Out {
    if let Some(out) = save_first(w, leaving) {
        return out;
    }
    (vec![Effect::Shell(ShellCmd::Quit)], Arm::named(name))
}

/// Update (N37): quit → swap → relaunch — never mid-take, and never over an edit the editor
/// holds unsaved: that's saved first, and the update waits on it (B199).
fn apply_update(w: &mut World) -> Out {
    let Some(version) = crate::derive::update_ready(w).map(String::from) else { return (vec![], Arm::DROPPED) };
    // Never mid-take (law 4) — the button greys; this is the guard behind it.
    if w.take_open() || w.update.applying {
        return (vec![], Arm::DROPPED);
    }
    if let Some(out) = save_first(w, Leaving::Update) {
        return out;
    }
    w.update.applying = true;
    let mut fx = Vec::new();
    // The window in front comes back after the relaunch, at its derived state — never the
    // editor, whose steps live in this process (N63): the one behind it.
    let front = w.session.windows.iter().filter(|(k, _)| !matches!(k, WindowKind::Editor(_))).max_by_key(|(_, d)| d.entered).map(|(k, _)| *k);
    if let Some(window) = front {
        fx.push(Effect::Persist(PersistOp::Restore(Some(Restore { why: RelaunchWhy::Update, window, written: w.wall }))));
    }
    fx.push(Effect::Updater(UpdCmd::Apply(version)));
    (fx, Arm { name: "update", edge: Some(("update-ready", "updating-app")) })
}

/// Quit or Update while editor windows hold edits not saved yet (B199): each is saved first —
/// as its Done, ✕ and ⌘Q save it — and the windows come forward to show it saving, the
/// popover making way; a save already on its way is waited for. What was asked waits on the
/// saves (`Fields::leaving`): it goes on once none is on its way, and is dropped if one
/// doesn't land. `None`: nothing to wait for.
fn save_first(w: &mut World, leaving: Leaving) -> Option<Out> {
    let unsaved = |e: &EditorFields| crate::derive::editor::editable(e) && e.history.as_ref().is_some_and(|h| h.dirty());
    let held: BTreeSet<EditorId> = w.session.fields.editors.iter().filter(|(_, e)| e.save == EditorSave::Saving || unsaved(e)).map(|(id, _)| *id).collect();
    if held.is_empty() {
        return None;
    }
    let mut fx = Vec::new();
    let mut edge = None;
    for id in &held {
        if w.session.fields.editors.get(id).is_some_and(unsaved) {
            let (more, arm) = editor_done(w, *id);
            fx.extend(more);
            edge = edge.or(arm.edge);
        }
    }
    w.session.fields.leaving = Some(leaving);
    if w.session.popover.is_some() {
        dismiss(w);
    }
    for id in held {
        open_window(w, WindowKind::Editor(id));
        fx.push(Effect::Shell(ShellCmd::OpenEditor(id)));
    }
    let name = if leaving == Leaving::Update { "update-waits-on-save" } else { "quit-waits-on-save" };
    Some((fx, Arm { name, edge }))
}

/// What waited on the editors' saves goes on now that they landed (B199): the quit — uploads
/// still draining asked about first — or the update, not waiting for the final video (an
/// upkeep makes it). A take begun meanwhile holds everything that would end the app (law 4):
/// then it's dropped.
fn go_on(w: &mut World, leaving: Option<Leaving>) -> Vec<Effect> {
    let Some(leaving) = leaving.filter(|_| !w.take_open()) else { return vec![] };
    let (fx, _) = match leaving {
        Leaving::Quit => quit(w),
        Leaving::QuitAnyway => quit_now(w, Leaving::QuitAnyway, "quit-anyway"),
        Leaving::Update => apply_update(w),
    };
    fx
}

fn os(w: &mut World, news: OsNews) -> Out {
    let at = w.now;
    let mut fx = Vec::new();
    let recording = w.take.as_ref().is_some_and(|t| t.phase == Phase::Recording);
    match news {
        OsNews::Grants { screen, mic, camera } => {
            w.mac.screen = Fact::known(screen, at);
            w.mac.mic = Fact::known(mic, at);
            w.mac.camera = Fact::known(camera, at);
            // A grant switched off mid-take (N39 `permission-lost`).
            if let Some(out) = guard_take(w) {
                return out;
            }
            // The chapter's chained prompts: the mic prompt the Allow recording click put to
            // macOS was answered (allowed or denied), so the camera follows (N43), and its
            // answer the screen step — that click's later halves (law 11). Only those answers
            // chain: a grants report after a relaunch, or any later one, asks nothing.
            // The chapter runs once the name row ticks, wiring or not (N45).
            let named = w.provisioned() || crate::derive::onboarding::rows_done(w)[3];
            let chapter = w.session.chapter && w.window_visible(WindowKind::Onboarding) && named && !w.row_six_done() && !w.take_open();
            let from = if w.mac.mic_asked && mic != Access::NotAsked {
                w.mac.mic_asked = false;
                if chapter && camera_unasked(w) {
                    w.mac.camera_asked = true;
                    fx.push(Effect::Shell(ShellCmd::RequestPermission(Perm::Camera)));
                    return (fx, Arm { name: "chapter-camera-step", edge: Some(("perm-mic", "perm-camera")) });
                }
                Some("perm-mic")
            } else if w.mac.camera_asked && camera != Access::NotAsked {
                w.mac.camera_asked = false;
                // The bubble starts on (N43) — unless the camera was just denied here.
                if camera != Access::Granted && w.prefs.show_yourself {
                    w.prefs.show_yourself = false;
                    fx.push(persist_prefs(w));
                }
                Some("perm-camera")
            } else {
                None
            };
            if let Some(from) = from
                && chapter
                && !screen
                && !w.mac.screen_asked
            {
                ask_screen(w, &mut fx);
                return (fx, Arm { name: "chapter-screen-step", edge: Some((from, "perm-screen")) });
            }
            (fx, Arm::named("grants"))
        }
        OsNews::ScreenAsked { alert } => {
            w.mac.screen_alert = Some(alert);
            (fx, Arm::named("screen-asked"))
        }
        OsNews::Devices { displays, mics, cameras } => {
            w.mac.displays = Fact::known(displays, at);
            w.mac.mics = Fact::known(mics, at);
            w.mac.cameras = Fact::known(cameras, at);
            // The take's display, camera or microphone went away (N39's auto-pauses).
            if let Some(out) = guard_take(w) {
                return out;
            }
            (fx, Arm::named("devices"))
        }
        OsNews::Disk(free) => {
            w.mac.disk_free = Fact::known(free, at);
            // At the floor the take finalizes itself, never by a surprise ENOSPC (N39).
            if let Some(out) = guard_take(w) {
                return out;
            }
            (fx, Arm::named("disk"))
        }
        OsNews::Session(s) => {
            w.mac.os = Fact::known(s, at);
            if recording && !s.present() {
                let cause = if !s.awake { Cause::Asleep } else { Cause::Locked };
                pause(w, cause);
                fx.push(Effect::Recorder(RecCmd::Pause));
                return (fx, Arm { name: "mac-away", edge: Some(("recording", "mac-away")) });
            }
            (fx, Arm::named("session"))
        }
        OsNews::WillSleep => {
            // macOS waits for the answer (B92): the session is asleep from this instant —
            // a take recording pauses now, before the Mac goes (`mac-away`, N39) — and
            // settle answers once the pause has landed, or at the hold's end.
            let mut s = w.mac.os.get().copied().unwrap_or_default();
            s.awake = false;
            w.mac.os = Fact::known(s, at);
            w.mac.sleep_hold = Some(at);
            if recording {
                pause(w, Cause::Asleep);
                fx.push(Effect::Recorder(RecCmd::Pause));
                return (fx, Arm { name: "mac-away", edge: Some(("recording", "mac-away")) });
            }
            (fx, Arm::named("will-sleep"))
        }
        OsNews::Woke => {
            w.net.reach = Fact::Unknown;
            w.net.not_shroom = None;
            (fx, Arm::named("woke"))
        }
        OsNews::NetPath(online) => {
            w.net.online = Fact::known(online, at);
            if !online {
                w.net.reach = Fact::Unknown;
                w.net.not_shroom = None;
            } else {
                // Back online: the wizard and the uploader resume by themselves, now — no
                // retry button (N18).
                if w.inflight.failures.get(&Op::Upload).is_some_and(|f| f.class == FailClass::Transport) {
                    w.inflight.failures.remove(&Op::Upload);
                }
                // …and so does the join machine (N31: “I'll finish joining the moment you're back”),
                // the stop's page probe (N40) and the page refresh.
                for op in Op::WIZARD.into_iter().chain([Op::Peek, Op::Join, Op::Standing, Op::Roster, Op::Reach, Op::Refresh]) {
                    if w.inflight.failures.get(&op).is_some_and(|f| f.class == FailClass::Transport) {
                        w.inflight.failures.remove(&op);
                    }
                }
                // “You're offline…” is no longer true: the popover falls back to `config`
                // while the page opens by itself (law 1, whole-spec audit #20).
                back_online(w);
            }
            (fx, Arm::named("net-path"))
        }
        OsNews::StatusItemVisible(v) => {
            w.mac.status_item_visible = v;
            (fx, Arm::named("status-item"))
        }
        OsNews::LoginItem(on) => {
            w.mac.login_item = Fact::known(on, at);
            (fx, Arm::named("login-item"))
        }
        OsNews::UtcOffset(min) => {
            w.mac.utc_offset_min = min;
            (fx, Arm::named("utc-offset"))
        }
        OsNews::Terminating { ours } => {
            // macOS's own quit-and-reopen at perm-screen: remember the chapter (bend 9).
            if !ours && onboarding_at(w) == Some(At::PermScreen) {
                w.launch.ending = true;
                // Nothing waits forever: if macOS never ends the process, perm-screen
                // comes back with Reopen shroom offered again.
                w.inflight.ops.insert(Op::Relaunch, at.plus(Op::Relaunch.deadline_ms()));
                let restore = Restore { why: RelaunchWhy::Permission, window: WindowKind::Onboarding, written: w.wall };
                fx.push(Effect::Persist(PersistOp::Restore(Some(restore))));
                return (fx, Arm { name: "macos-relaunch", edge: Some(("perm-screen", "relaunching")) });
            }
            (fx, Arm::named("terminating"))
        }
    }
}

/// Finalize the open take for a stop the person didn't make: it becomes a saved take and
/// asks — never auto-published (N25, N39).
fn finalize_for(w: &mut World, why: Why, fx: &mut Vec<Effect>) {
    if let Some(t) = w.take.as_mut() {
        if matches!(t.phase, Phase::Finalizing(Why::Stop | Why::GetLink | Why::Discard)) {
            return; // a person's Stop or Discard already under way is never downgraded
        }
        t.phase = Phase::Finalizing(why);
        t.since = w.now;
        fx.push(Effect::Recorder(RecCmd::Finalize { why }));
    }
}

/// The open take against the facts as they now stand — the one place an auto-pause or a
/// stop you didn't make is decided (N39), whichever report brought the news (the OS's
/// observers, the recorder's watchdog, an entry's probe):
/// - screen recording switched off with a display recording: the take finalizes and asks
///   (`permission-lost`); a screen grant only returns after a relaunch (law 4);
/// - the take's display, or its camera, gone: `display-lost`, `camera-lost`;
/// - the microphone switched off, or the take's microphone gone: `permission-lost
///   (microphone)`, `mic-lost`;
/// - the disk at the floor: `disk-full-stop`.
///
/// `None` when nothing about the take changed.
fn guard_take(w: &mut World) -> Option<Out> {
    let t = w.take.as_ref()?;
    let (source, phase) = (t.source, t.phase);
    let open = matches!(phase, Phase::Recording | Phase::Pausing(_) | Phase::Paused(_) | Phase::Resuming);
    let mut fx = Vec::new();
    if open && matches!(source, SourceId::Display(_)) && w.mac.screen.is(&false) {
        finalize_for(w, Why::ScreenRevoked, &mut fx);
        return Some((fx, Arm { name: "screen-revoked", edge: Some(("recording", "permission-lost")) }));
    }
    if phase != Phase::Recording {
        return None;
    }
    let t = w.take.as_ref()?;
    let cause = if !crate::derive::source_present(w, t) {
        Some(if matches!(source, SourceId::Display(_)) { Cause::DisplayLost } else { Cause::CameraLost })
    } else if t.mic.is_some() && w.mac.mic.get().is_some_and(|m| *m != Access::Granted) {
        Some(Cause::MicRevoked)
    } else if let (Some(uid), Some(mics)) = (&t.mic, w.mac.mics.get()) {
        (!mics.iter().any(|m| &m.uid == uid)).then_some(Cause::MicLost)
    } else {
        None
    };
    if let Some(cause) = cause {
        pause(w, cause);
        fx.push(Effect::Recorder(RecCmd::Pause));
        let edge = match cause {
            Cause::MicRevoked => ("recording", "mic-permission-lost"),
            Cause::MicLost => ("recording", "mic-lost"),
            Cause::CameraLost => ("recording", "camera-lost"),
            _ => ("recording", "display-lost"),
        };
        return Some((fx, Arm { name: "auto-paused", edge: Some(edge) }));
    }
    let t = w.take.as_ref()?;
    let floor = crate::derive::take_floor_bytes(w.prefs.quality, t.length_ms);
    if w.mac.disk_free.get().is_some_and(|free| *free < floor) {
        finalize_for(w, Why::DiskFull, &mut fx);
        return Some((fx, Arm { name: "disk-floor", edge: Some(("recording", "disk-full-stop")) }));
    }
    None
}

/// The arm-time guard's missing beat: a grant this take needs that reads as never asked
/// (a reset, a cleanup tool) — the microphone first, then the camera when Camera is the
/// source or a camera is picked for the bubble. A Mac with no camera has no camera beat
/// (N43): the camera picker has nothing to show there, so nothing is asked.
fn missing_beat(w: &World, source: SourceId) -> Option<Perm> {
    if !w.prefs.mic_off && w.mac.mic.is(&Access::NotAsked) && w.mac.mics.get().is_some_and(|m| !m.is_empty()) {
        return Some(Perm::Mic);
    }
    let has_camera = w.mac.cameras.get().is_some_and(|c| !c.is_empty());
    let wants_camera = source == SourceId::Camera || (w.prefs.show_yourself && has_camera);
    (wants_camera && w.mac.camera.is(&Access::NotAsked)).then_some(Perm::Camera)
}

fn rec(w: &mut World, news: RecNews) -> Out {
    let now = w.now;
    let mut fx = Vec::new();
    match news {
        RecNews::Started { dir, rec } => {
            w.inflight.ops.remove(&Op::RecStart);
            // A take that died before this one is behind us, said or not (#53).
            w.session.take_lost = None;
            let Some(arming) = w.session.arming.take() else {
                // Nobody asked for this capture any more: keep it as a saved take.
                fx.push(Effect::Recorder(RecCmd::Abandon));
                return (fx, Arm::named("started-unasked"));
            };
            if let (Some(old_dir), Some(old)) = (&arming.restart_of, w.take.as_ref()) {
                if &old.dir == old_dir {
                    fx.push(Effect::Recorder(RecCmd::Discard(old.dir.clone())));
                    fx.push(Effect::Uploader(UpCmd::ForgetRemote(old.rec.clone())));
                }
            }
            // What capture opened: the microphone `config` picked (while macOS allows it), and
            // Camera's camera — the ones the count asked the lane for.
            let mic = w.mac.mic.is(&Access::Granted).then(|| crate::derive::popover_selected_mic(w)).flatten();
            let camera = (arming.source == SourceId::Camera).then(|| crate::derive::selected_camera(w).map(|c| c.uid.clone())).flatten();
            w.take = Some(OpenTake {
                dir,
                rec,
                opened: arming.started,
                source: arming.source,
                phase: Phase::Recording,
                since: now,
                length_ms: 0,
                markers: 0,
                marked_at_pause: false,
                mic,
                camera,
            });
            (fx, Arm { name: "started", edge: Some(("countdown", "recording")) })
        }
        RecNews::Paused { length_ms } => {
            w.inflight.ops.remove(&Op::RecPause);
            let Some(t) = w.take.as_mut() else { return (fx, Arm::named("paused-no-take")) };
            t.length_ms = length_ms;
            if let Phase::Pausing(cause) = t.phase {
                t.phase = Phase::Paused(cause);
                t.since = now;
                // A pause the person made opens asked (DEVIATIONS #10); any other opens
                // without taking the keyboard (law 5, N39).
                w.session.popover = Some(if cause == Cause::User { Door::asked(now) } else { Door::unasked(now) });
            }
            (fx, Arm { name: "paused", edge: Some(("recording", "paused")) })
        }
        RecNews::Resumed => {
            w.inflight.ops.remove(&Op::RecResume);
            if let Some(t) = w.take.as_mut() {
                t.phase = Phase::Recording;
                t.since = now;
            }
            (fx, Arm { name: "resumed", edge: Some(("paused", "recording")) })
        }
        RecNews::Finalized(result) => {
            w.inflight.ops.remove(&Op::RecFinalize);
            let Some(t) = w.take.take() else { return (fx, Arm::named("finalized-no-take")) };
            match (result, t.phase) {
                // Law 3's single gate: only Stop and Get link make a link.
                // The space can't take it (N34): still a link — law 3 — that waits; the take is
                // safe here, and there's no page to open.
                (Ok(_), Phase::Finalizing(Why::Stop | Why::GetLink)) if crate::derive::members::stranded_by_space(w) => {
                    fx.push(Effect::Publish { dir: t.dir.clone(), rec: t.rec.clone() });
                    fx.push(Effect::Takes(TakeCmd::Rescan));
                    w.session.last_stop = Some(StopOutcome::Stranded(t.dir));
                    w.session.popover = Some(Door::asked(now));
                    (fx, Arm { name: "stopped-stranded", edge: Some(("stopping", "stopped-stranded")) })
                }
                (Ok(length_ms), Phase::Finalizing(Why::Stop | Why::GetLink)) => {
                    fx.push(Effect::Publish { dir: t.dir.clone(), rec: t.rec.clone() });
                    // Wake the uploader now: the link exists, its bytes should follow at once.
                    fx.push(Effect::Uploader(UpCmd::Kick));
                    fx.push(Effect::Takes(TakeCmd::Rescan));
                    let _ = length_ms;
                    // The stop's own probe already found — and kept finding (B60) — that the
                    // page isn't the space's: no page to open (N40, whole-spec audit #16) —
                    // the take is safe here. One such answer only asks again.
                    if reached_at(w, t.since) == Some(Reach::NotShroom) && w.net.not_shroom_held() {
                        w.session.last_stop = Some(StopOutcome::Stranded(t.dir));
                        w.session.popover = Some(Door::asked(now));
                        return (fx, Arm { name: "stopped-stranded", edge: Some(("stopping", "stopped-stranded")) });
                    }
                    w.session.page_open = Some(PageOpen { rec: t.rec.clone(), dir: t.dir.clone(), at: w.wall, stopped: t.since });
                    // Offline as macOS says, or the stop's probe couldn't reach the page
                    // (a captive portal, an outage, a slow first handshake): the link waits in
                    // the popover, and the page opens by itself once it answers (N40, #15).
                    if w.online() == Some(false) || reached_at(w, t.since) == Some(Reach::Unreachable) {
                        w.session.last_stop = Some(StopOutcome::Offline(t.rec.clone()));
                        w.session.popover = Some(Door::asked(now));
                        return (fx, Arm { name: "stopped-offline", edge: Some(("stopping", "stopped-offline")) });
                    }
                    (fx, Arm { name: "published", edge: Some(("stopping", "owner-page")) })
                }
                (Ok(length_ms), Phase::Finalizing(why @ (Why::DiskFull | Why::ScreenRevoked))) => {
                    let cause = if why == Why::DiskFull { SavedCause::DiskFull } else { SavedCause::ScreenOff };
                    w.saved.push(SavedTake { dir: t.dir.clone(), rec: t.rec, length_ms, cause, created: w.wall, movies: false });
                    w.session.last_stop = Some(StopOutcome::AutoStopped(t.dir));
                    w.session.popover = Some(Door::unasked(now));
                    (fx, Arm::named("auto-stopped"))
                }
                _ => {
                    // Capture died or finalizing failed: keep the take; it reads as an
                    // interrupted one (DEVIATIONS #6) — never deleted.
                    w.session.take_lost = Some(TakeLost::Pending);
                    fx.push(Effect::Recorder(RecCmd::Abandon));
                    fx.push(Effect::Takes(TakeCmd::Rescan));
                    w.session.popover = Some(Door::unasked(now));
                    (fx, Arm::named("capture-died"))
                }
            }
        }
        RecNews::Failed => {
            w.inflight.ops.remove(&Op::RecStart);
            if let Some(a) = w.session.arming.take() {
                if a.restart_of.is_some() {
                    // Restart's capture failed: back to the old take, intact (N39).
                    w.session.popover = Some(Door::asked(now));
                    return (fx, Arm::named("restart-failed"));
                }
                w.session.popover = Some(Door::asked(now));
                return (fx, Arm::named("start-failed"));
            }
            capture_died(w, &mut fx);
            (fx, Arm::named("capture-died"))
        }
        RecNews::FrameStall => {
            let gone = match w.take.as_ref().map(|t| t.source) {
                Some(SourceId::Display(id)) => !w.mac.displays.get().is_some_and(|ds| ds.iter().any(|d| d.id == id)),
                _ => false,
            };
            if gone {
                pause(w, Cause::DisplayLost);
                fx.push(Effect::Recorder(RecCmd::Pause));
                return (fx, Arm::named("stall-display-lost"));
            }
            capture_died(w, &mut fx);
            (fx, Arm::named("capture-died"))
        }
        RecNews::DiskFloor => {
            if !w.take.as_ref().is_some_and(|t| t.phase == Phase::Recording) {
                return (fx, Arm::named("disk-floor-not-recording"));
            }
            finalize_for(w, Why::DiskFull, &mut fx);
            (fx, Arm { name: "disk-floor", edge: Some(("recording", "disk-full-stop")) })
        }
        RecNews::Abandoned => {
            // The folder is free: the scan brings it back as a saved take (law 2, DEVIATIONS #6).
            if w.session.take_lost == Some(TakeLost::Pending) {
                w.session.take_lost = None;
            }
            fx.push(Effect::Takes(TakeCmd::Rescan));
            (fx, Arm::named("abandoned"))
        }
        RecNews::NothingKept { camera } => {
            // A take that died with nothing a repair could save: no banner will come, so
            // `config` says it (DEVIATIONS #53). A capture nobody asked for says nothing.
            fx.push(Effect::Takes(TakeCmd::Rescan));
            if w.session.take_lost != Some(TakeLost::Pending) {
                return (fx, Arm::named("abandoned"));
            }
            w.session.take_lost = Some(TakeLost::Said { camera });
            if w.session.popover.is_none() {
                w.session.popover = Some(Door::unasked(now));
            }
            (fx, Arm::named("nothing-kept"))
        }
        RecNews::MicLost => {
            // The microphone stopped delivering mid-take: pause, and Resume names the
            // fallback (N39). A take that wasn't recording has nothing to pause.
            if !w.take.as_ref().is_some_and(|t| t.phase == Phase::Recording && t.mic.is_some()) {
                return (fx, Arm::named("mic-lost-not-recording"));
            }
            pause(w, Cause::MicLost);
            fx.push(Effect::Recorder(RecCmd::Pause));
            (fx, Arm { name: "mic-lost", edge: Some(("recording", "mic-lost")) })
        }
        RecNews::Primed(ok) => {
            if ok {
                answered(w, Op::Prime);
                let reconfirm = w.mac.primed.is_some();
                w.mac.primed = Some(w.wall);
                fx.push(Effect::Persist(PersistOp::Primed(w.wall)));
                // The monthly re-confirm, run at `config` (N38): the chapter isn't touched.
                if reconfirm {
                    return (fx, Arm::named("reconfirmed"));
                }
                w.session.chapter = false;
                // Wiring still running: it's the last step (N45).
                let to = if w.provisioned() { "all-set" } else { "wiring" };
                return (fx, Arm { name: "primed", edge: Some(("perm-priming", to)) });
            }
            // Not clean: warm up again after the backoff (never in a loop).
            w.inflight.ops.remove(&Op::Prime);
            note_failure(w, Op::Prime, FailClass::Refused);
            (fx, Arm::named("prime-not-clean"))
        }
        RecNews::Deleted(dir) => {
            w.members.movies.remove(&dir);
            w.members.saving.remove(&dir);
            w.members.save_failed.remove(&dir);
            w.saved.retain(|s| s.dir != dir);
            w.queue.retain(|q| q.dir != dir);
            w.removed.retain(|q| q.dir != dir);
            (fx, Arm::named("deleted"))
        }
        RecNews::SavedToMovies(dir) => {
            w.members.saving.remove(&dir);
            w.members.save_failed.remove(&dir);
            if let Some(s) = w.saved.iter_mut().find(|s| s.dir == dir) {
                s.movies = true;
            }
            // A take the space can't take: “Saved to Movies ✓” — it keeps waiting (N34).
            if w.queue.iter().any(|q| q.dir == dir) {
                w.members.movies.insert(dir.clone());
            }
            // A switch's take: its copy landed, so this Mac's copy goes (DECISIONS B54).
            if w.members.toss.remove(&dir).is_some() {
                fx.push(Effect::Takes(TakeCmd::Delete(dir)));
            }
            (fx, Arm::named("saved-to-movies"))
        }
        RecNews::SaveFailed(dir) => {
            w.members.saving.remove(&dir);
            w.members.save_failed.insert(dir.clone());
            // A switch's take that couldn't be copied: kept, never sent (DEVIATIONS #27).
            if let Some(q) = w.members.toss.remove(&dir) {
                w.removed.push(q);
                fx.push(Effect::Takes(TakeCmd::SetAside(dir)));
            }
            (fx, Arm::named("save-to-movies-failed"))
        }
        RecNews::Length(ms) => {
            if let Some(t) = w.take.as_mut() {
                t.length_ms = ms;
            }
            (fx, Arm::named("length"))
        }
    }
}

fn capture_died(w: &mut World, fx: &mut Vec<Effect>) {
    if w.take.take().is_some() {
        w.session.take_lost = Some(TakeLost::Pending);
        fx.push(Effect::Recorder(RecCmd::Abandon));
        fx.push(Effect::Takes(TakeCmd::Rescan));
        w.session.popover = Some(Door::unasked(w.now));
    }
}

fn cloud(w: &mut World, news: CloudNews) -> Out {
    match news {
        CloudNews::Reached(r) => {
            w.inflight.ops.remove(&Op::Reach);
            w.net.reach = Fact::known(r, w.now);
            // The run of “not shroom” answers since the stop (B60): the page answering as the
            // space ends it; an unreachable answer between two neither counts nor breaks it.
            w.net.not_shroom = match r {
                Reach::NotShroom => Some(crate::world::Streak { count: w.net.not_shroom.map_or(1, |s| s.count.saturating_add(1)), last: w.now }),
                Reach::Shroom => None,
                Reach::Unreachable => w.net.not_shroom,
            };
            if r == Reach::Unreachable {
                // Asked again on a backoff while a stop's page waits (#15).
                note_failure(w, Op::Reach, FailClass::Transport);
            } else {
                w.inflight.failures.remove(&Op::Reach);
                w.net.online = Fact::known(true, w.now);
            }
            reach_answered(w, r)
        }
        CloudNews::PageOpened(rec) => {
            w.inflight.ops.remove(&Op::OpenPage);
            if w.session.page_open.as_ref().is_some_and(|p| p.rec == rec) {
                w.session.page_open = None;
            }
            // The page the offline popover promised is open: its face is behind us (#20).
            if matches!(&w.session.last_stop, Some(StopOutcome::Offline(r)) if *r == rec) {
                w.session.last_stop = None;
            }
            (vec![], Arm::named("page-opened"))
        }
        CloudNews::Refreshed { deployed, ai } => {
            w.inflight.ops.remove(&Op::Refresh);
            w.inflight.failures.remove(&Op::Refresh);
            w.space.page_checked = Some(w.now);
            if ai.is_some() {
                w.space.ai = ai;
            }
            (vec![], Arm::named(if deployed { "page-refreshed" } else { "page-current" }))
        }
        CloudNews::AiSet(on) => {
            answered(w, Op::Ai);
            w.space.ai = Some(on);
            if w.space.ai_turn == Some(AiTurn::TurningOn) {
                w.space.ai_turn = None;
            }
            // The banner's Turn off went through (N75): settled for good on this Mac, and
            // said once, in config's status line.
            if w.space.ai_turn == Some(AiTurn::TurningOff) {
                w.space.ai_turn = None;
                if !on {
                    w.space.ai_turn = Some(AiTurn::Off);
                    return (settle_ai(w), Arm { name: "ai-set", edge: Some(("ai-turning-off", "ai-off")) });
                }
            }
            (vec![], Arm::named("ai-set"))
        }
        CloudNews::Validated { id, result } => {
            answered(w, Op::Validate);
            if w.sweep.paste != Some(Paste::Checking(id)) {
                return (vec![], Arm::named("validated-stale"));
            }
            // The repair mood: the fresh token must be for the space's own account — never
            // a second space by accident (N42).
            if let Some(space_account) = w.space.identity.as_ref().map(|i| i.account_id.clone()) {
                return match result {
                    Ok(accounts) => match accounts.iter().find(|a| a.id == space_account) {
                        Some(account) => {
                            w.sweep.paste = Some(Paste::Saving { id, account: account.clone(), several: accounts.len() > 1 });
                            (vec![], Arm::named("validated"))
                        }
                        None => {
                            w.sweep.paste = Some(Paste::OtherAccount);
                            (vec![], Arm { name: "token-other-account", edge: Some(("token", "token-other-account")) })
                        }
                    },
                    Err(TokenFail::Rejected) => {
                        w.sweep.paste = Some(Paste::Garbled);
                        (vec![], Arm { name: "token-garbled", edge: Some(("token", "token-garbled")) })
                    }
                    Err(TokenFail::WrongPowers) => {
                        w.sweep.paste = Some(Paste::WrongPowers);
                        (vec![], Arm { name: "wrong-powers", edge: Some(("token", "token-wrong-powers")) })
                    }
                };
            }
            match result {
                Ok(mut accounts) if accounts.len() == 1 => {
                    let account = accounts.remove(0);
                    w.sweep.paste = Some(Paste::Saving { id, account, several: false });
                    (vec![], Arm::named("validated"))
                }
                Ok(accounts) if accounts.len() > 1 => {
                    w.session.fields.account = None;
                    w.sweep.paste = Some(Paste::PickAccount { id, accounts });
                    (vec![], Arm { name: "validated-several", edge: Some(("token", "token-pick-account")) })
                }
                // Verify said it isn't a live token at all: a copy that lost characters
                // past the shape check — “copy it once more” (DEVIATIONS #26).
                Err(TokenFail::Rejected) => {
                    w.sweep.paste = Some(Paste::Garbled);
                    (vec![], Arm { name: "token-garbled", edge: Some(("token", "token-garbled")) })
                }
                _ => {
                    w.sweep.paste = Some(Paste::WrongPowers);
                    (vec![], Arm { name: "wrong-powers", edge: Some(("token", "token-wrong-powers")) })
                }
            }
        }
        CloudNews::TokenSaved(id) => {
            answered(w, Op::SaveToken);
            let Some(Paste::Saving { id: saving, account, several }) = w.sweep.paste.clone() else {
                // A save that landed after Disconnect wiped `~/.shroom`: wipe again, so the
                // Mac stays at the start with no token behind it (N36).
                if !w.provisioned() && !w.space.token && w.sweep.paste.is_none() {
                    return (vec![Effect::Persist(PersistOp::Wipe)], Arm::named("token-saved-after-disconnect"));
                }
                return (vec![], Arm::named("token-saved-stale"));
            };
            if saving != id {
                return (vec![], Arm::named("token-saved-stale"));
            }
            // A new token: everything after it is probed afresh.
            w.space.token = true;
            w.sweep = Sweep { token: Fact::known(TokenVerdict::Valid { account, several }, w.now), ..Sweep::default() };
            w.session.fields.settings.add_token = false;
            // Add a token here (N33): this owner-class Mac keeps the token now too.
            if let Some(id) = w.space.identity.as_mut().filter(|i| i.role == Role::OwnerClass) {
                id.role = Role::TokenHolder;
                let id = id.clone();
                return (vec![Effect::Persist(PersistOp::Space(id))], Arm { name: "token-added", edge: Some(("token", "settings-space")) });
            }
            (vec![], Arm { name: "token-saved", edge: Some(("token", "card")) })
        }
        // A run's part underway (N74). The check is one pass: its part is the newest it
        // said, so a pass begun again starts over and ticks nothing it hasn't answered.
        // Wiring keeps its furthest part: every call is idempotent, and a retry re-runs the
        // parts before it in a blink — they stay ticked, the clock counts from the first.
        CloudNews::Part { op, part } => {
            let now = w.now;
            let slot = match op {
                Op::Sweep => &mut w.sweep.check,
                Op::Wire => &mut w.sweep.wiring,
                _ => return (vec![], Arm::DROPPED),
            };
            let monotonic = op == Op::Wire;
            match slot {
                Some(p) if p.part == part => {}
                Some(p) if monotonic && p.part > part => {}
                _ => *slot = Some(Progress { part, since: now }),
            }
            (vec![], Arm::named("part"))
        }
        CloudNews::Swept(ans) => {
            answered(w, Op::Sweep);
            w.sweep.check = None;
            // Every probe pass runs under a stored token; one answering without it was
            // asked before Disconnect, about a space this Mac no longer knows.
            if !w.space.token {
                return (vec![], Arm::named("swept-stale"));
            }
            let now = w.now;
            let was_gated = w.sweep.gates.get().is_some_and(|g| g.email || g.card);
            // The first probe since a rewire said the page was sound (wiring wrote the
            // Worker's fact with its self-check, and nothing has probed it since).
            let after_wire = w.sweep.self_check.is(&true) && w.sweep.worker.at() == w.sweep.self_check.at();
            let whole = ans.token.is_some();
            if let Some(mut v) = ans.token {
                // The account chosen among several stays chosen (N42): a re-proof of the
                // same account keeps row one's fact.
                if let (TokenVerdict::Valid { account, several }, Some(TokenVerdict::Valid { account: was, several: true })) = (&mut v, w.sweep.token.get()) {
                    if account.id == was.id {
                        *several = true;
                    }
                }
                w.sweep.token = Fact::known(v, now);
                // A full pass always answers the token first: this was a whole pass.
                w.sweep.pass = Some(now);
            }
            if let Some(g) = ans.gates {
                w.sweep.gates = Fact::known(g, now);
                // A gate just cleared: the rest of the card step is probed again.
                if was_gated && !g.email && !g.card && ans.subdomain.is_none() {
                    w.sweep.subdomain = Fact::Unknown;
                    w.sweep.foreign_v = Fact::Unknown;
                    w.sweep.label = Fact::Unknown;
                    w.sweep.found = Fact::Unknown;
                    w.sweep.pass = None;
                }
            }
            if let Some(v) = ans.subdomain {
                w.sweep.subdomain = Fact::known(v, now);
            }
            if let Some(v) = ans.foreign_v {
                w.sweep.foreign_v = Fact::known(v, now);
            }
            if let Some(v) = ans.label {
                w.sweep.label = Fact::known(v, now);
            }
            if let Some(v) = ans.bucket {
                w.sweep.bucket = Fact::known(v, now);
            }
            if let Some(v) = ans.worker {
                w.sweep.worker = Fact::known(v, now);
            }
            if let Some(v) = ans.found {
                w.sweep.found = Fact::known(v, now);
            }
            // A whole pass over a space that finds every row sound: the space takes videos
            // again (the degraded banner clears, uploads resume).
            if whole && w.space.token_holder() {
                match settings_view::broken_row(w) {
                    None => {
                        w.space.broken = Fact::known(false, now);
                        w.inflight.failures.remove(&Op::Wire);
                    }
                    Some(at) => {
                        w.space.broken = Fact::known(true, now);
                        // The page still reads broken after a rewire said it was sound: the
                        // next rewire waits its backoff (never a Wire ↔ probe loop).
                        if at == SpaceAt::Wiring && after_wire {
                            note_failure(w, Op::Wire, FailClass::Refused);
                        }
                        let edge = match at {
                            SpaceAt::StorageGone => Some(("repair-mood", "space-storage-gone")),
                            SpaceAt::NameChanged => Some(("repair-mood", "name-changed")),
                            SpaceAt::Token => Some(("repair-mood", "token")),
                            SpaceAt::GateEmail | SpaceAt::GateCard => Some(("repair-mood", "gate-card")),
                            SpaceAt::Wiring => Some(("repair-mood", "wiring")),
                            _ => None,
                        };
                        return (vec![], Arm { name: "swept-broken", edge });
                    }
                }
            }
            (vec![], Arm::named("swept"))
        }
        CloudNews::Claimed { name, result } => {
            answered(w, Op::Claim);
            if !w.space.token {
                return (vec![], Arm::named("claimed-stale"));
            }
            match result {
                Ok(()) => {
                    w.sweep.subdomain = Fact::known(Some(name), w.now);
                    w.sweep.taken = None;
                    (vec![], Arm { name: "claimed", edge: Some(("name", "done")) })
                }
                Err(ClaimFail::Taken) => {
                    w.sweep.taken = Some(name);
                    (vec![], Arm { name: "name-taken", edge: Some(("name", "name-taken")) })
                }
            }
        }
        // Wiring runs under a stored token (the wizard's, or the repair mood's); an answer
        // landing without one was asked before Disconnect — the person left that space, and
        // it is never set up again behind them (N36).
        CloudNews::Wired(_) if !w.space.token => {
            w.inflight.ops.remove(&Op::Wire);
            w.sweep.wiring = None;
            (vec![], Arm::named("wired-stale"))
        }
        CloudNews::Wired(result) => match result {
            Ok(mut identity) => {
                w.sweep.wiring = None;
                let repairing = w.provisioned();
                if repairing {
                    // A rewire in the repair mood keeps its failure history: if the next
                    // probe still finds the page broken, the next rewire waits its backoff
                    // instead of ping-ponging (a whole sound pass clears it).
                    w.inflight.ops.remove(&Op::Wire);
                } else {
                    answered(w, Op::Wire);
                }
                // The account chosen among several is the space's, for good (N42).
                if identity.account_name.is_none() {
                    if let Some(TokenVerdict::Valid { account, several: true }) = w.sweep.token.get() {
                        identity.account_name = Some(account.name.clone());
                    }
                }
                // So is someone else's `v`, if the setup's survey found one (N42) — the
                // name row says it from this, never from the label. A rewire in the repair
                // mood keeps what the setup saw: it runs under the space's own label and
                // never looks at `v`.
                identity.foreign_v = match &w.space.identity {
                    Some(was) => was.foreign_v,
                    None => w.sweep.foreign_v.is(&true),
                };
                w.sweep.self_check = Fact::known(true, w.now);
                // Wiring made the bucket and the page and proved them: those facts are fresh
                // now, and the next entry re-probes the rest.
                w.sweep.bucket = Fact::known(true, w.now);
                w.sweep.worker = Fact::known(WorkerFact::Ours, w.now);
                w.sweep.subdomain = Fact::known(Some(identity.subdomain.clone()), w.now);
                w.sweep.pass = None;
                w.session.fields.settings.fresh = false;
                w.space.identity = Some(identity.clone());
                // The runner writes this Mac's `device.key` before wiring's enroll (§5,
                // §6.11): from here `/me` and the Members tab can ask with it.
                w.space.device_key = true;
                // Provisioning clears the remembered "Set up my space" (bend 1).
                w.prefs.setup_chosen = false;
                // The last step since N45: wired after the chapter ended is all set; wired
                // under the chapter, the chapter goes on (its row just ticks).
                let edge = (!repairing && w.row_six_done()).then_some(("wiring", "all-set"));
                let mut fx = vec![Effect::Persist(PersistOp::Space(identity)), persist_prefs(w)];
                // A space set up here starts with its AI on (N75; Bagrat, 2026-10-25: nothing
                // said in the setup): settled on this Mac, never asked or said about again.
                if !repairing {
                    fx.extend(settle_ai(w));
                }
                (fx, Arm { name: "wired", edge })
            }
            // The fallback classification: a gate closed since the card step.
            Err(WireFail::Gate(g)) => {
                answered(w, Op::Wire);
                w.sweep.wiring = None;
                w.sweep.gates = Fact::known(g, w.now);
                (vec![], Arm::named("wire-gated"))
            }
            Err(WireFail::Refused) => {
                w.inflight.ops.remove(&Op::Wire);
                note_failure(w, Op::Wire, FailClass::Refused);
                (vec![], Arm::named("wire-refused"))
            }
        },
    }
}

/// The newest reachability answer, if it came at or after `since` (a stop's instant).
fn reached_at(w: &World, since: Mono) -> Option<Reach> {
    match &w.net.reach {
        Fact::Known { v, at } if *at >= since => Some(*v),
        _ => None,
    }
}

/// A recording id for a queued take (its take's own, else the dir's name — never empty).
fn rec_of(w: &World, dir: &TakeDir) -> crate::world::RecId {
    w.queue.iter().find(|q| &q.dir == dir).map(|q| q.rec.clone()).unwrap_or_else(|| crate::world::RecId(dir.0.clone()))
}

/// Back online, as macOS says: the offline stop's face is behind us — the page opens by
/// itself once the probe finds it (law 1, N40, whole-spec audit #20).
fn back_online(w: &mut World) {
    if matches!(w.session.last_stop, Some(StopOutcome::Offline(_))) {
        w.session.last_stop = None;
    }
}

/// What a reachability answer means for a stop whose page waits (N40, whole-spec audit
/// #15, #16, #20): the page answers → the offline face is behind us (the page opens);
/// unreachable while macOS calls the Mac online → `stopped-offline`, the link in the
/// popover, re-probed on a backoff within the hour; not the space's page at all →
/// `stopped-stranded`, for every role.
fn reach_answered(w: &mut World, r: Reach) -> Out {
    let now = w.now;
    let Some(page) = w.session.page_open.clone().filter(|p| w.net.reach.at().is_some_and(|at| at >= p.stopped)) else {
        return (vec![], Arm::named("reached"));
    };
    match r {
        Reach::Shroom => {
            back_online(w);
            (vec![], Arm::named("reached"))
        }
        Reach::Unreachable => {
            // Once per stop: the backoff's later answers re-probe quietly.
            let first = w.inflight.failures.get(&Op::Reach).is_none_or(|f| f.tries == 0);
            if first && w.session.last_stop.is_none() && w.take.is_none() {
                w.session.last_stop = Some(StopOutcome::Offline(page.rec));
                w.session.popover = Some(Door::asked(now));
                return (vec![], Arm { name: "stopped-offline", edge: Some(("stopping", "stopped-offline")) });
            }
            (vec![], Arm::named("reached"))
        }
        // One “not shroom” is a freshly wired address still propagating as often as it is a
        // space that's gone (B60): the stop asks again, and strands only on a held verdict.
        Reach::NotShroom if !w.net.not_shroom_held() => (vec![], Arm::named("reached-not-shroom-once")),
        Reach::NotShroom => {
            w.session.page_open = None;
            if w.take.is_none() {
                w.session.last_stop = Some(StopOutcome::Stranded(page.dir));
                w.session.popover = Some(Door::asked(now));
            }
            (vec![], Arm { name: "stopped-stranded", edge: Some(("stopping", "stopped-stranded")) })
        }
    }
}

/// An upload landed after this Mac told the space its uploads are refused: say they move
/// again, once (§4.18).
fn health_ok(w: &mut World, dir: &TakeDir) -> Option<Effect> {
    if !w.members.standing.reported {
        return None;
    }
    let rec = w.queue.iter().find(|q| &q.dir == dir)?.rec.clone();
    w.members.standing.reported = false;
    Some(Effect::Member(MemberCmd::Health { rec, refused: false }))
}

fn upload(w: &mut World, news: UploadNews) -> Out {
    match news {
        UploadNews::Progress { dir, done, total } => {
            if w.space.broken.is(&true) {
                w.space.broken = Fact::known(false, w.now);
            }
            // Bytes are moving: whatever held the uploader back is behind it.
            w.inflight.failures.remove(&Op::Upload);
            let fx = health_ok(w, &dir).into_iter().collect();
            if let Some(q) = w.queue.iter_mut().find(|q| q.dir == dir) {
                q.done = done;
                q.total = total;
                q.blocked = None;
            }
            // Progress keeps the upload's op alive.
            if w.inflight.ops.contains_key(&Op::Upload) {
                w.inflight.ops.insert(Op::Upload, w.now.plus(Op::Upload.deadline_ms()));
            }
            (fx, Arm::named("upload-progress"))
        }
        UploadNews::Live(dir) => {
            if w.space.broken.is(&true) {
                w.space.broken = Fact::known(false, w.now);
            }
            w.inflight.failures.remove(&Op::Upload);
            let fx: Vec<Effect> = health_ok(w, &dir).into_iter().collect();
            w.members.movies.remove(&dir);
            w.queue.retain(|q| q.dir != dir);
            if w.queue.is_empty() {
                w.inflight.ops.remove(&Op::Upload);
            }
            (fx, Arm::named("upload-live"))
        }
        UploadNews::Blocked(dir, b) => {
            if let Some(q) = w.queue.iter_mut().find(|q| q.dir == dir) {
                q.blocked = Some(b);
            }
            w.inflight.ops.remove(&Op::Upload);
            // The next try waits its backoff (5 s doubling, 2 min cap): a space that can't
            // take videos — a resting state until it's repaired — is never hammered.
            let class = if b == Blocked::Offline { FailClass::Transport } else { FailClass::Refused };
            note_failure(w, Op::Upload, class);
            // An auth- or space-class failure on a publish: the space can't take videos
            // (N42) — the banner asks, the takes wait safely here.
            // A token Mac whose space refuses its uploads tells the page so — the owner's
            // “Waiting for your space” (§4.18, whole-spec audit #19) — once, through its device
            // key (a dead token can't write the bucket); the next upload that lands says ok.
            let mut fx = Vec::new();
            if b == Blocked::Space && w.space.token_holder() && w.space.device_key && !w.members.standing.reported {
                w.members.standing.reported = true;
                fx.push(Effect::Member(MemberCmd::Health { rec: rec_of(w, &dir), refused: true }));
            }
            if b == Blocked::Space && !w.space.broken.is(&true) {
                w.space.broken = Fact::known(true, w.now);
                return (fx, Arm { name: "space-degraded", edge: Some(("idle-ready", "space-degraded")) });
            }
            (fx, Arm::named("upload-blocked"))
        }
        UploadNews::Tombstoned(dir) => {
            // Another Mac deleted it: the upload stops, and the take stays here, its
            // maker's to keep or toss (N35, law 2, DEVIATIONS #7).
            if let Some(pos) = w.queue.iter().position(|q| q.dir == dir) {
                let q = w.queue.remove(pos);
                w.removed.push(q);
            }
            (vec![], Arm::named("upload-tombstoned"))
        }
        UploadNews::Idle => {
            let ran = w.inflight.ops.remove(&Op::Upload).is_some();
            // The uploader stopped with takes it could have sent still waiting, and said
            // nothing about them: the next pass waits its backoff — never a Kick loop at
            // network speed (whole-spec audit #2).
            if ran && w.queue.iter().any(|q| q.blocked.is_none() && crate::derive::members::can_send(w, q)) {
                note_failure(w, Op::Upload, FailClass::Refused);
            }
            (vec![], Arm::named("upload-idle"))
        }
        UploadNews::Credentials(minted) => {
            // The scan's word, except where it can only be behind: a credential the
            // uploader minted while the scan was reading the disk is the newer (a scan never
            // lowers an expiry it raced — a refused credential is dropped by `Minted`).
            let mut fresh: std::collections::BTreeMap<_, _> = minted.into_iter().collect();
            for (dir, until) in std::mem::take(&mut w.members.minted) {
                if until > w.wall && w.queue.iter().any(|q| q.dir == dir) {
                    let e = fresh.entry(dir).or_insert(until);
                    *e = (*e).max(until);
                }
            }
            w.members.minted = fresh;
            (vec![], Arm::named("credentials"))
        }
        UploadNews::Minted { dir, until } => {
            match until {
                Some(until) => w.members.minted.insert(dir, until),
                None => w.members.minted.remove(&dir),
            };
            (vec![], Arm::named("credential-minted"))
        }
    }
}

fn failed(w: &mut World, op: Op, class: crate::world::FailClass) -> Out {
    w.inflight.ops.remove(&op);
    note_failure(w, op, class);
    // The page's Edit in shroom waited on this refresh: nothing opens (B221 (c)).
    if op == Op::LibRefresh && matches!(w.session.fields.edit_link, Some(EditLink::Waiting { asked: true, .. })) {
        w.session.fields.edit_link = None;
    }
    let mut fx = Vec::new();
    match op {
        Op::RecStart => {
            w.session.arming = None;
            w.session.popover = Some(Door::asked(w.now));
        }
        Op::RecPause | Op::RecResume | Op::RecFinalize => capture_died(w, &mut fx),
        // The probe's deadline (600 ms) passed: the page can't be reached now (#15).
        Op::Reach => {
            w.net.reach = Fact::known(Reach::Unreachable, w.now);
            let (more, _) = reach_answered(w, Reach::Unreachable);
            fx.extend(more);
        }
        // A lost claim may have landed: the plan re-probes, and the answer routes it
        // (the claim itself is only ever the person's click).
        Op::Claim => w.sweep.pass = None,
        // The relaunch never came: perm-screen again, Reopen shroom offered once more.
        Op::Relaunch => {
            w.launch.ending = false;
            w.inflight.failures.remove(&Op::Relaunch);
            fx.push(Effect::Persist(PersistOp::Restore(None)));
        }
        // The banner's Turn off didn't go through: it says so, Turn off live again (law 12),
        // and it's tried again quietly until it goes through or the person says OK.
        Op::Ai if w.space.ai_turn == Some(AiTurn::TurningOff) => {
            return (fx, Arm { name: "ai-failed", edge: Some(("ai-turning-off", "ai-turn-off-failed")) });
        }
        // A rename or delete that didn't land: the row reads as the space has it again.
        Op::Rename => w.library_ops.renaming.clear(),
        Op::Delete => w.library_ops.deleting.clear(),
        // A keep or a switch that didn't land: it reads as the space has it (N68).
        Op::Keep => w.library_ops.keeping.clear(),
        Op::Plays => w.library_ops.plays = None,
        // The app never quit for the swap: nothing changed — say so once, try later.
        Op::UpdateApply => {
            w.update.applying = false;
            w.update.failed = true;
            w.inflight.failures.remove(&Op::UpdateApply);
            fx.push(Effect::Persist(PersistOp::Restore(None)));
        }
        // The editor (N63): the record or the video that couldn't come, and the save that
        // didn't go through, each say so — their retry is the person's (Try again, Done).
        Op::EditorOpen(id) => {
            w.inflight.failures.remove(&op);
            if let Some(e) = w.session.fields.editors.get_mut(&id).filter(|e| matches!(e.media, EditorMedia::Opening | EditorMedia::Getting { .. })) {
                let came = matches!(e.media, EditorMedia::Getting { .. });
                e.media = EditorMedia::Failed;
                return (fx, Arm { name: "editor-cant-get", edge: came.then_some(("editor-getting", "editor-cant-get")) });
            }
        }
        Op::EditorSave(id) => {
            w.inflight.failures.remove(&op);
            if let Some(e) = w.session.fields.editors.get_mut(&id).filter(|e| e.save == EditorSave::Saving) {
                // A quit or an update that waited on it doesn't go on: the window stays, and
                // says why (B199).
                w.session.fields.leaving = None;
                // The space turned it down — the video is gone, or another Mac made an edit
                // the video: no retry can land, and the window only closes (B199).
                if class == FailClass::Refused {
                    e.save = EditorSave::Refused;
                    return (fx, Arm { name: "editor-save-refused", edge: Some(("editor-saving", "editor-save-refused")) });
                }
                e.save = EditorSave::Failed;
                return (fx, Arm { name: "editor-save-failed", edge: Some(("editor-saving", "editor-save-failed")) });
            }
        }
        // A copy that didn't come out is noted, and the next owed waits its backoff; the
        // page plays the saved list over the original meanwhile (B195 (f)), and an upkeep
        // makes it later (B197 (d)). The window making it says so.
        Op::EditorCopy => {
            let asked: Vec<(RecId, u64)> = w.library_ops.copies.iter().filter(|(_, c)| c.asked).map(|(r, c)| (r.clone(), c.rev)).collect();
            w.library_ops.copies.retain(|_, c| !c.asked);
            if let Some(e) = w.session.fields.editors.values_mut().find(|e| matches!(e.save, EditorSave::Making { .. }) && asked.contains(&(e.rec.clone(), e.rev))) {
                e.save = EditorSave::MakingFailed;
                return (fx, Arm { name: "editor-making-failed", edge: Some(("editor-making", "editor-making-failed")) });
            }
        }
        // An invite the person opened whose peek couldn't get through: its window opens on
        // join-waiting's posture (N31: offline or unanswered) — never over a take (law 4) —
        // and peek keeps trying behind it.
        Op::Peek => {
            let open = !w.take_open() && w.members.invite.as_ref().is_some_and(|i| i.wants_window);
            if open {
                if let Some(inv) = w.members.invite.as_mut() {
                    inv.wants_window = false;
                }
                open_window(w, WindowKind::Onboarding);
            }
        }
        _ => {}
    }
    (fx, Arm::named("failed"))
}

/// The count's end: capture starts on the source the count was for — at its deadline, or at
/// once for the command's start without it (N62).
fn fire(w: &mut World) -> Option<Effect> {
    let a = w.session.arming.as_mut().filter(|a| !a.fired)?;
    a.fired = true;
    let source = a.source;
    let camera = (source == SourceId::Camera).then(|| crate::derive::selected_camera(w).map(|c| c.uid.clone())).flatten();
    Some(Effect::Recorder(RecCmd::Start { source, quality: w.prefs.quality, mic: crate::derive::popover_selected_mic(w), camera }))
}

fn tick(w: &mut World) -> Out {
    let now = w.now;
    let mut fx = Vec::new();
    // The countdown fires at its deadline.
    if w.session.arming.as_ref().is_some_and(|a| !a.fired && now >= a.deadline) {
        fx.extend(fire(w));
    }
    // "Copied" reverts.
    if w.session.copied.as_ref().is_some_and(|(_, at)| now >= at.plus(COPIED_MS)) {
        w.session.copied = None;
    }
    // A one-tap delete's Undo line runs out: it's sent (N68).
    if w.session.fields.library.undo.as_ref().is_some_and(|(_, at)| now >= at.plus(UNDO_MS)) {
        fx.extend(send_undone(w));
    }
    // The settings link's light goes out.
    if w.session.fields.settings.lit.as_ref().is_some_and(|(_, at)| now >= at.plus(LIT_MS)) {
        w.session.fields.settings.lit = None;
    }
    // Deadlines: nothing waits forever.
    let overdue: Vec<Op> = w.inflight.ops.iter().filter(|(_, d)| **d <= now).map(|(op, _)| *op).collect();
    for op in overdue {
        let (more, _) = failed(w, op, crate::world::FailClass::Timeout);
        fx.extend(more);
    }
    // The offline stop's page lapses after an hour of wall time (N40).
    if w.session.page_open.as_ref().is_some_and(|p| w.wall.0 >= p.at.0 + 3_600_000) {
        w.session.page_open = None;
    }
    // The AI's turn on by default, or the banner's Turn off, that didn't go through: tried
    // again at its backoff (N75), with the popover shut too — it has been said.
    if let Some(turn @ (AiTurn::TurningOn | AiTurn::TurningOff)) = w.space.ai_turn {
        if !w.inflight.ops.contains_key(&Op::Ai) && w.inflight.failures.get(&Op::Ai).is_some_and(|f| now >= f.ai_retry_at()) {
            fx.push(Effect::Cloud(CloudCmd::SetAi(turn == AiTurn::TurningOn)));
        }
    }
    (fx, Arm::named("tick"))
}

fn library(w: &mut World, news: LibraryNews) -> Out {
    let now = w.now;
    match news {
        // A listing asked before Disconnect is about a space this Mac no longer knows.
        _ if !w.provisioned() => {
            for op in [Op::LibRefresh, Op::LibCache, Op::Rename, Op::Delete, Op::Cleanup, Op::Suggest, Op::Keep, Op::Plays] {
                w.inflight.ops.remove(&op);
            }
            (vec![], Arm::named("library-stale"))
        }
        LibraryNews::Rows { rows, fresh } => {
            if !fresh {
                answered(w, Op::LibCache);
            }
            if fresh {
                // When this refresh began (its op's deadline, less its span): the writes
                // that landed before then are in its listing; later ones still read over it.
                let began = w.inflight.ops.get(&Op::LibRefresh).map(|d| Mono(d.0.saturating_sub(Op::LibRefresh.deadline_ms()))).unwrap_or(Mono(0));
                // A keep is the suggestions' to forget, not the listing's (N68).
                w.library_ops.landed.retain(|_, (l, at)| *l == Landed::Kept || *at >= began);
                w.library_ops.edited.retain(|_, (_, at)| *at >= began);
                answered(w, Op::LibRefresh);
                w.library_ops.refreshed = Some(now);
            } else if w.library_ops.refreshed.is_some() {
                // The cache is older than what the refresh already said.
                return (vec![], Arm::named("library-cache-stale"));
            }
            w.library = Fact::known(Arc::new(rows), now);
            let fx = edit_link_answered(w, fresh);
            (fx, Arm::named(if fresh { "library-refreshed" } else { "library-cached" }))
        }
        LibraryNews::Renamed { rec, title } => {
            w.library_ops.renaming.remove(&rec);
            w.library_ops.landed.insert(rec.clone(), (Landed::Renamed(title.clone()), now));
            if w.library_ops.renaming.is_empty() {
                answered(w, Op::Rename);
            }
            if let Fact::Known { v, .. } = &mut w.library {
                if let Some(r) = Arc::make_mut(v).iter_mut().find(|r| r.rec == rec) {
                    r.title = title.clone();
                }
            }
            if let Some(q) = w.queue.iter_mut().find(|q| q.rec == rec) {
                q.title = title;
            }
            (vec![], Arm::named("renamed"))
        }
        LibraryNews::Suggested(s) => {
            answered(w, Op::Suggest);
            // A keep that landed reads over a listing begun before it (as a delete does).
            let began = Mono(now.0.saturating_sub(Op::Suggest.deadline_ms()));
            w.library_ops.landed.retain(|_, (l, at)| *l != Landed::Kept || *at >= began);
            w.suggestions = Fact::known(s, now);
            (vec![], Arm::named("suggested"))
        }
        LibraryNews::Kept(rec) => {
            w.library_ops.keeping.remove(&rec);
            w.library_ops.landed.insert(rec, (Landed::Kept, now));
            if w.library_ops.keeping.is_empty() {
                answered(w, Op::Keep);
            }
            (vec![], Arm::named("kept"))
        }
        LibraryNews::Plays(on) => {
            w.library_ops.plays = None;
            answered(w, Op::Plays);
            if let Fact::Known { v, .. } = &mut w.suggestions {
                v.counting = on;
            }
            (vec![], Arm::named("plays-set"))
        }
        LibraryNews::Deleted(recs) => {
            for r in &recs {
                w.library_ops.deleting.remove(r);
                w.library_ops.landed.insert(r.clone(), (Landed::Deleted, now));
            }
            if w.library_ops.deleting.is_empty() {
                answered(w, Op::Delete);
            }
            if let Fact::Known { v, .. } = &mut w.library {
                Arc::make_mut(v).retain(|r| !recs.contains(&r.rec));
            }
            // A suggested one's bytes leave the space's size now, not at the next read: Space
            // used and the money line say what's left (N68).
            if let Fact::Known { v, .. } = &mut w.suggestions {
                let freed: u64 = v.rows.iter().filter(|s| recs.contains(&s.rec)).map(|s| s.bytes).sum();
                v.space_bytes = v.space_bytes.saturating_sub(freed);
                v.rows.retain(|s| !recs.contains(&s.rec));
            }
            // Removed everywhere — this Mac's copy of one still uploading too (the person
            // asked; the tombstone came first — DEVIATIONS #16).
            let mut fx = Vec::new();
            w.queue.retain(|q| {
                let gone = recs.contains(&q.rec);
                if gone {
                    fx.push(Effect::Takes(TakeCmd::Delete(q.dir.clone())));
                }
                !gone
            });
            // …and an editor still open on one of them (B199).
            fx.extend(editor_video_gone(w, &recs));
            (fx, Arm { name: "deleted", edge: Some(("delete-confirm", "library")) })
        }
        // The cleanup sweep answered (B91): nothing on screen moves — the next one is due
        // `CLEANUP_MS` from now.
        LibraryNews::Cleaned { .. } => {
            answered(w, Op::Cleanup);
            w.library_ops.cleaned = Some(now);
            (vec![], Arm::named("cleaned"))
        }
    }
}

fn update_news(w: &mut World, news: UpdateNews) -> Out {
    match news {
        UpdateNews::Checked(offer) => {
            answered(w, Op::UpdateCheck);
            w.update.checked = Some(w.now);
            w.update.tried = Some(w.wall);
            // Versions roll up: only the newest is ever offered.
            w.update.offer = Fact::known(offer, w.now);
            (vec![], Arm::named("update-checked"))
        }
        UpdateNews::Staged(version) => {
            answered(w, Op::UpdateDownload);
            w.update.staged = Some(version);
            (vec![], Arm::named("update-staged"))
        }
    }
}

#[allow(dead_code)]
fn _mono(_: Mono) {}

/// The report lane's answers (B180): each lands only on the sheet that asked for it.
fn report(w: &mut World, news: ReportNews) -> Out {
    match news {
        ReportNews::Gathered(d) => {
            // One gathering serves whichever form waits for it: the sheet, Send feedback, or both.
            let sheet = matches!(w.session.confirm, Some(Confirm::Report { .. }));
            let feedback = w.session.windows.contains_key(&WindowKind::Feedback) && w.session.fields.feedback.gathered.is_none();
            if !sheet && !feedback {
                return (vec![], Arm::DROPPED);
            }
            if feedback {
                w.session.fields.feedback.gathered = Some(d.clone());
            }
            if sheet {
                w.session.fields.settings.report.gathered = Some(d);
            }
            (vec![], Arm::named("report-gathered"))
        }
        ReportNews::Sent(Kind::Feedback) => {
            let f = &mut w.session.fields.feedback;
            if !f.sending {
                return (vec![], Arm::DROPPED);
            }
            f.sending = false;
            f.sent = true;
            (vec![], Arm::named("feedback-sent"))
        }
        ReportNews::NotSent(Kind::Feedback) => {
            let f = &mut w.session.fields.feedback;
            if !f.sending {
                return (vec![], Arm::DROPPED);
            }
            f.sending = false;
            f.failed = true;
            (vec![], Arm::named("feedback-not-sent"))
        }
        ReportNews::Sent(Kind::Problem) => {
            if w.session.confirm != Some(Confirm::Report { sending: true }) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = None;
            w.session.fields.settings.report = ReportFields::default();
            w.session.fields.settings.sent = true;
            (vec![], Arm { name: "report-sent", edge: Some(("report-problem", "settings-general")) })
        }
        ReportNews::NotSent(Kind::Problem) => {
            if w.session.confirm != Some(Confirm::Report { sending: true }) {
                return (vec![], Arm::DROPPED);
            }
            w.session.confirm = Some(Confirm::Report { sending: false });
            w.session.fields.settings.report.failed = true;
            (vec![], Arm::named("report-not-sent"))
        }
    }
}
