//! `settle(&World)`: a flat, ordered list of pure "due" reactions — what should be
//! happening that isn't. It reads the World and never writes it; `update` marks what it
//! returns as in flight, so settling again right after is empty (the idempotence test).

use crate::derive::onboarding::{self, At, GATE_POLL_MS};
use crate::derive::settings::{self as settings_view, SpaceAt};
use crate::effect::{CloudCmd, EditCmd, Effect, LibCmd, MemberCmd, RecCmd, ShellCmd, SweepPart, UpCmd, UpdCmd};
use crate::time::Mono;
use crate::world::{Op, Paste, Reach, SettingsTab, WindowKind, World};

pub fn settle(w: &World) -> Vec<Effect> {
    let mut fx = Vec::new();
    due_validate(w, &mut fx);
    due_save(w, &mut fx);
    due_sweep(w, &mut fx);
    due_gate_poll(w, &mut fx);
    due_wire(w, &mut fx);
    due_prime(w, &mut fx);
    due_reconfirm(w, &mut fx);
    due_reach(w, &mut fx);
    due_page(w, &mut fx);
    due_upload(w, &mut fx);
    due_space_sweep(w, &mut fx);
    due_self_check(w, &mut fx);
    due_space_gate_poll(w, &mut fx);
    due_space_wire(w, &mut fx);
    due_page_refresh(w, &mut fx);
    due_library(w, &mut fx);
    due_update_check(w, &mut fx);
    due_update_download(w, &mut fx);
    due_peek(w, &mut fx);
    due_join(w, &mut fx);
    due_probe(w, &mut fx);
    due_roster(w, &mut fx);
    due_keep_awake(w, &mut fx);
    due_cleanup(w, &mut fx);
    due_sleep_ack(w, &mut fx);
    due_edit_copy(w, &mut fx);
    due_edit_upkeep(w, &mut fx);
    fx
}

/// A saved edit's permanent copy (N63, B195 (f)): owed copies are cut one at a time, never
/// over a take — the Mac's encoder and its bandwidth are the take's — never offline, never
/// while the upkeep holds the editor's lane, and a copy that didn't come out holds the next
/// back by its backoff.
fn due_edit_copy(w: &World, fx: &mut Vec<Effect>) {
    if w.take_open() || w.online() == Some(false) || !w.provisioned() || !idle(w, Op::EditorCopy) || w.inflight.ops.contains_key(&Op::EditorUpkeep) {
        return;
    }
    if let Some((rec, c)) = w.library_ops.copies.iter().find(|(_, c)| !c.asked) {
        fx.push(Effect::Editor(EditCmd::Copy { rec: rec.clone(), rev: c.rev }));
    }
}

/// The editor's upkeep (B197 (d)): on every Mac, once a launch and then on the cleanup
/// sweep's cadence, this Mac's originals past their two days are baked and the copies its
/// edits are missing made — the lane decides which. Never over a take (the encoder and the
/// bandwidth are the take's), never offline, never while the editor is open (it may be on
/// one of them) or while the editor's lane runs anything else; one that couldn't run waits
/// for the next turn.
fn due_edit_upkeep(w: &World, fx: &mut Vec<Effect>) {
    let Some(at) = crate::derive::next_upkeep(w) else { return };
    let lane_busy = [Op::EditorOpen, Op::EditorSave, Op::EditorCopy, Op::EditorUpkeep].iter().any(|op| w.inflight.ops.contains_key(op));
    let owed = w.library_ops.copies.values().any(|c| !c.asked);
    if w.now >= at && !w.take_open() && w.online() != Some(false) && w.session.fields.editor.is_none() && !lane_busy && !owed {
        fx.push(Effect::Editor(EditCmd::Upkeep));
    }
}

/// The token Mac's cleanup sweep (DECISIONS B91; ARCHITECTURE §5.5's GC, §16 #49): on the
/// self-check's slow cadence, over the token's own pair, fail-soft — never offline, never
/// over a take (its bandwidth is the take's), never while a paste is being checked.
fn due_cleanup(w: &World, fx: &mut Vec<Effect>) {
    let Some(at) = crate::derive::next_cleanup(w) else { return };
    let pasting = matches!(w.sweep.paste, Some(Paste::Checking(_) | Paste::Saving { .. } | Paste::PickAccount { .. }));
    if w.now >= at && !pasting && !w.take_open() && w.online() != Some(false) && !w.inflight.ops.contains_key(&Op::Cleanup) {
        fx.push(Effect::Library(LibCmd::Cleanup));
    }
}

/// macOS waits on shroom's answer to its will-sleep (ARCHITECTURE §3.3's power row,
/// DECISIONS B92): the answer goes once the take is safe — paused (the `mac-away` pause
/// the will-sleep asked for has landed), or no take at all — and at most `SLEEP_HOLD_MS`
/// after macOS asked, whatever happened. shroom never holds sleep beyond the pause.
fn due_sleep_ack(w: &World, fx: &mut Vec<Effect>) {
    let Some(asked) = w.mac.sleep_hold else { return };
    let safe = w.take.as_ref().is_none_or(|t| matches!(t.phase, crate::world::Phase::Paused(_)));
    if safe || w.now >= asked.plus(crate::derive::SLEEP_HOLD_MS) {
        fx.push(Effect::Shell(ShellCmd::AllowSleep));
    }
}

// ── Region 6: the join machine, member life, the Members face ──

/// An arrived invite is peeked (§6.2 steps 2–3): at once when the person opened it, again
/// on every entry of its window (law 1: a tray click reruns peek) — never mid-take (law 4:
/// invites wait), never offline (the window rests on its offline line).
fn due_peek(w: &World, fx: &mut Vec<Effect>) {
    let Some(inv) = &w.members.invite else { return };
    // A join replayed at launch (`join.json`) never heard peek: asked once, for the plan's
    // name and address — its answer never changes where the join stands.
    let replay_unpeeked = inv.joining && inv.peek.get().is_none();
    if (inv.joining && !replay_unpeeked) || inv.refused.is_some() || w.take_open() || w.online() == Some(false) || !idle(w, Op::Peek) {
        return;
    }
    let entered = w.session.windows.get(&WindowKind::Onboarding).filter(|_| w.window_visible(WindowKind::Onboarding)).map(|d| d.entered);
    let stale = |t: Mono| inv.peek.at().is_none_or(|at| at < t);
    // An invite the person opened is peeked until its window opens (an answer that lands
    // mid-take opens nothing: the take ends, and it's asked again).
    if inv.wants_window || entered.is_some_and(stale) {
        fx.push(Effect::Member(MemberCmd::Peek(inv.link)));
    }
}

/// Join was clicked: the call runs — and replays the identical body after its backoff —
/// until it answers for good. Never mid-take (law 4), and never over an old space that
/// still drains or waits on a recovered take (N31).
fn due_join(w: &World, fx: &mut Vec<Effect>) {
    let Some(inv) = &w.members.invite else { return };
    if !inv.joining || inv.refused.is_some() || w.take_open() || w.online() == Some(false) || !idle(w, Op::Join) {
        return;
    }
    let in_a_space = w.provisioned() || w.space.token;
    if in_a_space && (settings_view::draining(w) > 0 || !w.saved.is_empty() || settings_view::saving_stranded(w)) {
        return;
    }
    fx.push(Effect::Member(MemberCmd::Join(inv.link)));
}

/// `/me` (N34, §4.7) with this Mac's device key: law 1's re-probe on every entry of a Mac
/// without the token (config, Settings, the Library); on the token's Mac, the Space tab's
/// owner facts (the mark, the only-holder line). Also C7's second look after one space-side
/// answer, and an invite for this Mac's own space (§6.2 step 4) or one that would switch
/// a token Mac (the come-back line, U9).
fn due_probe(w: &World, fx: &mut Vec<Effect>) {
    if !w.space.device_key || !w.provisioned() || w.online() == Some(false) || !idle(w, Op::Standing) {
        return;
    }
    let st = &w.members.standing;
    let since = |t: Mono| st.at.is_none_or(|at| at < t);
    let second_look = st.strike.is_some_and(|s| !s.confirmed() && w.now >= s.first.plus(crate::world::STRIKE_GAP_MS));
    let invite = w.members.invite.as_ref().filter(|i| !i.joining && i.refused.is_none()).and_then(|i| match (i.peek.get(), i.peek.at()) {
        (Some(crate::event::PeekAnswer::Pending { host, .. }), Some(at)) => {
            let same = w.space.identity.as_ref().is_some_and(|id| &id.host == host);
            (same || w.space.token_holder()).then_some(at)
        }
        _ => None,
    });
    let visible = |k: WindowKind| w.session.windows.get(&k).filter(|_| w.window_visible(k)).map(|d| d.entered);
    let entry = if w.space.token_holder() {
        visible(WindowKind::Settings).filter(|_| settings_view::tab(w) == SettingsTab::Space)
    } else {
        [w.session.popover.map(|d| d.entered), visible(WindowKind::Settings), visible(WindowKind::Library)].into_iter().flatten().max()
    };
    if second_look || invite.is_some_and(since) || entry.is_some_and(since) {
        fx.push(Effect::Member(MemberCmd::Probe));
    }
}

/// The Members face reads its rows on every entry (law 1) — and the Library its labels,
/// on owner-class Macs (N33).
fn due_roster(w: &World, fx: &mut Vec<Effect>) {
    if !w.space.device_key || !w.space.owner_class() || w.online() == Some(false) || !idle(w, Op::Roster) {
        return;
    }
    let visible = |k: WindowKind| w.session.windows.get(&k).filter(|_| w.window_visible(k)).map(|d| d.entered);
    let members = visible(WindowKind::Settings).filter(|_| settings_view::tab(w) == SettingsTab::Members);
    let entry = [members, visible(WindowKind::Library)].into_iter().flatten().max();
    if entry.is_some_and(|t| w.members.roster.at().is_none_or(|at| at < t)) {
        fx.push(Effect::Member(MemberCmd::Roster));
    }
}

/// N41: uploads hold off idle sleep, never lid sleep. Held while a published take is
/// actually moving — a space, the network not known down, and a queued take that nothing
/// holds back — and let go the moment none is: a take the space refused (or any, while the
/// space can't take videos) never holds the Mac awake, and neither does waiting — offline,
/// or on the backoff after an upload that couldn't get through on a network macOS still
/// calls up (a captive portal, a dead link, an outage): nothing moves until a try gets
/// through, and an idle Mac waiting on nothing should sleep (DECISIONS B34). The next
/// progress clears the take's block and the hold comes back. Asked once per change: the
/// World remembers what was asked.
fn due_keep_awake(w: &World, fx: &mut Vec<Effect>) {
    let want = can_drain(w);
    if want != w.mac.awake_held {
        fx.push(Effect::Shell(ShellCmd::PowerAssertion(want)));
    }
}

/// Uploads moving now: a queued take nothing holds back, on a network not known down — on
/// a space that stopped minting for this Mac, only a take already holding its credential
/// (N32).
pub fn can_drain(w: &World) -> bool {
    w.provisioned() && w.online() != Some(false) && !w.space.broken.is(&true) && w.queue.iter().any(|q| q.blocked.is_none() && crate::derive::members::can_send(w, q))
}

/// Where Settings · Space stands, if it shows (its probes run only while it does).
fn space_at(w: &World) -> Option<SpaceAt> {
    (w.window_visible(WindowKind::Settings) && settings_view::tab(w) == SettingsTab::Space && w.space.token_holder())
        .then(|| settings_view::space_at(w))
}

/// Settings · Space re-probes the space on every entry (law 1, N36): the same probe pass
/// as the wizard's, when no whole pass has answered since the window (or tab) was entered.
fn due_space_sweep(w: &World, fx: &mut Vec<Effect>) {
    let Some(door) = w.session.windows.get(&WindowKind::Settings) else { return };
    // A paste being checked or saved holds the pass (its answer re-probes); one already
    // turned down never does — the next entry still re-probes (law 1).
    let pasting = matches!(w.sweep.paste, Some(Paste::Checking(_) | Paste::Saving { .. } | Paste::PickAccount { .. }));
    if space_at(w).is_none() || pasting || w.online() == Some(false) || !idle(w, Op::Sweep) {
        return;
    }
    if w.sweep.pass.is_none_or(|t| t < door.entered) {
        fx.push(Effect::Cloud(CloudCmd::Sweep(SweepPart::Full)));
    }
}

/// The repair mood's gate: watched like the wizard's.
fn due_space_gate_poll(w: &World, fx: &mut Vec<Effect>) {
    if !matches!(space_at(w), Some(SpaceAt::GateEmail | SpaceAt::GateCard)) || w.online() == Some(false) || !idle(w, Op::Sweep) {
        return;
    }
    if w.sweep.gates.at().is_some_and(|t| w.now >= t.plus(GATE_POLL_MS)) {
        fx.push(Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates)));
    }
}

/// The periodic self-check (N42's “the periodic self-check”, ARCHITECTURE §3.3's settle
/// table, row 9, DECISIONS B51): a token Mac re-proves its whole space at launch and every
/// six hours of awake time — and at once after a stop found its page isn't the space's —
/// whether or not Settings is open. A whole pass sets or clears `space.broken` (`space-degraded`'s cause),
/// as the Settings pass does. Fail-soft: a pass that couldn't run waits as long as a check.
fn due_self_check(w: &World, fx: &mut Vec<Effect>) {
    let Some(at) = crate::derive::next_self_check(w) else { return };
    let pasting = matches!(w.sweep.paste, Some(Paste::Checking(_) | Paste::Saving { .. } | Paste::PickAccount { .. }));
    if w.now >= at && !pasting && w.online() != Some(false) && !w.inflight.ops.contains_key(&Op::Sweep) {
        fx.push(Effect::Cloud(CloudCmd::Sweep(SweepPart::Full)));
    }
}

/// The page follows the app (§6.10, DECISIONS B50): once a launch, on a token Mac — so after
/// every update — the space's page is pinged and, only when it answers older than this
/// app's, this app's page goes up. Fail-soft: a refresh that couldn't run tries again in an
/// hour, or the moment the network is back.
fn due_page_refresh(w: &World, fx: &mut Vec<Effect>) {
    let Some(at) = crate::derive::next_refresh(w) else { return };
    if w.now >= at && w.online() != Some(false) && !w.inflight.ops.contains_key(&Op::Refresh) {
        fx.push(Effect::Cloud(CloudCmd::Refresh));
    }
}

/// The page or its key is gone: wiring runs again under the space's own label — the
/// click that wired it was the consent, every call idempotent (N42). Never over a Worker
/// that isn't shroom's (B24): that rests on the calm line (DEVIATIONS #11's gap).
fn due_space_wire(w: &World, fx: &mut Vec<Effect>) {
    let Some(id) = &w.space.identity else { return };
    let foreign = w.sweep.worker.is(&crate::world::WorkerFact::Foreign);
    if space_at(w) == Some(SpaceAt::Wiring) && !foreign && w.online() != Some(false) && idle(w, Op::Wire) {
        fx.push(Effect::Cloud(CloudCmd::Wire { label: id.label.clone() }));
    }
}

/// The Library opens on the local cache at once; the bucket refresh runs behind it on
/// every entry (law 1) — when none has answered since the window was entered.
fn due_library(w: &World, fx: &mut Vec<Effect>) {
    let Some(door) = w.session.windows.get(&WindowKind::Library) else { return };
    // What this Mac remembers, whatever the network (N35's offline face shows it): read
    // once, the first time the window shows with nothing known yet.
    if w.window_visible(WindowKind::Library) && w.provisioned() && w.library.get().is_none() && !w.inflight.ops.contains_key(&Op::LibCache) && !w.inflight.failures.contains_key(&Op::LibCache) {
        fx.push(Effect::Library(LibCmd::LoadCache));
    }
    if !w.window_visible(WindowKind::Library) || !w.provisioned() || w.online() == Some(false) || !idle(w, Op::LibRefresh) {
        return;
    }
    if w.library_ops.refreshed.is_none_or(|t| t < door.entered) {
        fx.push(Effect::Library(LibCmd::Refresh));
    }
}

/// The background update check: at launch, then every few hours, and when the popover
/// opens a while after the last (B154); fail-soft (N37).
fn due_update_check(w: &World, fx: &mut Vec<Effect>) {
    let Some(at) = crate::derive::next_check(w) else { return };
    if (w.now >= at || crate::derive::popover_check_due(w)) && w.online() != Some(false) && !w.inflight.ops.contains_key(&Op::UpdateCheck) {
        fx.push(Effect::Updater(UpdCmd::Check));
    }
}

/// The newest offer downloads in the background, verified as the installer verifies.
fn due_update_download(w: &World, fx: &mut Vec<Effect>) {
    let Some((at, version)) = crate::derive::next_download(w) else { return };
    if w.now >= at && w.online() != Some(false) && !w.inflight.ops.contains_key(&Op::UpdateDownload) {
        fx.push(Effect::Updater(UpdCmd::Download(version)));
    }
}

/// Where the onboarding window stands, if it shows (the plan's probes run only while it
/// does — closing the window mid-wire is safe, and re-entry resumes).
fn plan_at(w: &World) -> Option<At> {
    w.window_visible(WindowKind::Onboarding).then(|| onboarding::at(w))
}

/// A pasted token's shape passed: verify it and probe it by use (SM:476-487).
fn due_validate(w: &World, fx: &mut Vec<Effect>) {
    if let Some(Paste::Checking(id)) = &w.sweep.paste {
        if w.online() != Some(false) && idle(w, Op::Validate) {
            fx.push(Effect::Cloud(CloudCmd::Validate(*id)));
        }
    }
}

/// A verified token is written to `~/.shroom` before any wiring begins (law 13).
fn due_save(w: &World, fx: &mut Vec<Effect>) {
    if let Some(Paste::Saving { id, account, .. }) = &w.sweep.paste {
        if idle(w, Op::SaveToken) {
            fx.push(Effect::Cloud(CloudCmd::SaveToken(*id, account.id.clone())));
        }
    }
}

/// Every entry re-probes (law 1, SM:429-435): with a stored token and the plan showing,
/// the probe pass runs when no whole pass has answered since the window's entry — right
/// after the token is saved, on every return to the window, after a gate clears, and after
/// a claim whose answer was lost. A pass that stopped early (a dead token, a gate up) is a
/// whole answer: nothing re-fires until the next entry or the gate's poll.
fn due_sweep(w: &World, fx: &mut Vec<Effect>) {
    let Some(door) = w.session.windows.get(&WindowKind::Onboarding) else { return };
    if plan_at(w).is_none() || w.provisioned() || !w.space.token || w.sweep.paste.is_some() {
        return;
    }
    if w.online() == Some(false) || !idle(w, Op::Sweep) {
        return;
    }
    if w.sweep.pass.is_none_or(|t| t < door.entered) {
        fx.push(Effect::Cloud(CloudCmd::Sweep(SweepPart::Full)));
    }
}

/// A gate shows: watch it, every few seconds, until it clears (N14 — the spinner is the
/// only progress surface).
fn due_gate_poll(w: &World, fx: &mut Vec<Effect>) {
    if !matches!(plan_at(w), Some(At::GateEmail | At::GateCard)) || w.online() == Some(false) || !idle(w, Op::Sweep) {
        return;
    }
    if w.sweep.gates.at().is_some_and(|t| w.now >= t.plus(GATE_POLL_MS)) {
        fx.push(Effect::Cloud(CloudCmd::Sweep(SweepPart::Gates)));
    }
}

/// Everything before the Wiring row is proved: wire (idempotent calls, the self-check),
/// under the survey's label — never one it found someone else's (B24).
fn due_wire(w: &World, fx: &mut Vec<Effect>) {
    let Some(label) = w.sweep.label() else { return };
    // Since N45 wiring also runs out of sight under the permissions chapter, from the
    // moment the name row ticks — never on the way out of a relaunch.
    let under_chapter = matches!(plan_at(w), Some(At::Done | At::PermMic | At::PermCamera | At::PermScreen | At::Priming))
        && !w.provisioned()
        && !w.launch.ending
        && onboarding::rows_done(w)[3];
    if (under_chapter || matches!(plan_at(w), Some(At::Wiring | At::Found(_)))) && w.online() != Some(false) && idle(w, Op::Wire) {
        fx.push(Effect::Cloud(CloudCmd::Wire { label: label.to_string() }));
    }
}

/// The screen grant is live in the chapter: the ~1 s warm-up, so Apple's second consent
/// fires now, at setup, never mid-recording (N20). Never with a take open (law 11).
fn due_prime(w: &World, fx: &mut Vec<Effect>) {
    if plan_at(w) == Some(At::Priming) && !w.take_open() && idle(w, Op::Prime) {
        fx.push(Effect::Recorder(RecCmd::Prime));
    }
}

/// How old the last warm-up may be before `config` runs it again: Apple's screen-recording
/// re-confirm comes about monthly, so it's asked a few days early (DEVIATIONS #8, N38).
pub const RECONFIRM_MS: i64 = 25 * 24 * 3_600_000;

/// The monthly re-confirm, moved out of the take (N38): when `config` shows — no take, no
/// count, no question or stop on it — the screen grant is on, and the last warm-up is 25
/// days old, `perm-priming`'s ~1 s warm-up runs, so macOS asks over the popover, never
/// inside a take (law 11).
fn due_reconfirm(w: &World, fx: &mut Vec<Effect>) {
    let Some(primed) = w.mac.primed else { return };
    let config = w.session.popover.is_some() && w.take.is_none() && w.session.arming.is_none() && w.session.confirm.is_none() && w.session.last_stop.is_none();
    let due = w.wall.0 - primed.0 >= RECONFIRM_MS;
    if config && due && w.mac.screen.is(&true) && plan_at(w) != Some(At::Priming) && idle(w, Op::Prime) {
        fx.push(Effect::Recorder(RecCmd::Prime));
    }
}

fn idle(w: &World, op: Op) -> bool {
    !w.inflight.ops.contains_key(&op) && w.inflight.failures.get(&op).is_none_or(|f| w.now >= f.retry_at())
}

/// A stopped video's page waits on knowing the link answers (N40): probe reachability
/// when the last answer is older than the stop — and, while the page couldn't be reached
/// on a network macOS calls up, again on a backoff within the hour, so the page still
/// opens by itself once it answers (whole-spec audit #15).
fn due_reach(w: &World, fx: &mut Vec<Effect>) {
    let Some(page) = &w.session.page_open else { return };
    if w.online() == Some(false) || !idle(w, Op::Reach) || w.wall.0 >= page.at.0 + 3_600_000 {
        return;
    }
    let fresh = w.net.reach.at().is_some_and(|at| at >= page.stopped);
    // A “not shroom” answer that hasn't held yet is asked again, a few seconds on (B60).
    let unsure = w.net.not_shroom_reprobe().is_some_and(|t| w.now >= t);
    if !fresh || w.net.reach.is(&Reach::Unreachable) || unsure {
        fx.push(Effect::Cloud(CloudCmd::Reach));
    }
}

/// Open the owner page once: online, the link answers as this space, no take open
/// (law 4), within the hour (N40).
fn due_page(w: &World, fx: &mut Vec<Effect>) {
    let Some(page) = &w.session.page_open else { return };
    let answered = matches!(&w.net.reach, crate::time::Fact::Known { v: Reach::Shroom, at } if *at >= page.stopped);
    if answered && !w.take_open() && w.wall.0 < page.at.0 + 3_600_000 && idle(w, Op::OpenPage) {
        fx.push(Effect::Cloud(CloudCmd::OpenOwnerPage(page.rec.clone())));
    }
}

/// Uploads ride along whenever there's something to send and a way to send it.
fn due_upload(w: &World, fx: &mut Vec<Effect>) {
    let work = !w.queue.is_empty() || w.take.is_some();
    // A space that can't take this Mac's videos at all is never asked (N34): they wait here
    // — all but a take already holding its credential, which can still finish (N32), even
    // after a transient failure (its backoff is `idle`'s; whole-spec audit #22).
    let refused = crate::derive::members::stranded_by_space(w) && !w.queue.iter().any(|q| crate::derive::members::can_send(w, q));
    if work && !refused && w.provisioned() && w.online() != Some(false) && idle(w, Op::Upload) {
        fx.push(Effect::Uploader(UpCmd::Kick));
    }
}
