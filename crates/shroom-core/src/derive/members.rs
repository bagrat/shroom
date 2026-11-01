//! Region 6 (N28–N34): the invite window's faces, member life's banners, the Members tab,
//! and the facts every other surface reads about this Mac's place in its space — derived
//! from what the space's Worker said (WORKER-INTERFACE §4), never stored.

use crate::copy;
use crate::event::PeekAnswer;
use crate::scene::{Action, Banner, Button, Em, FromName, Glyph, InviteBody, LinkPill, MemberRowView, MembersBody, Nest, PlanRow, PlanView, Rich, Sheet, Style, TextField, Toggle, Tone};
use crate::time::{Mono, Wall};
use crate::world::{Born, CopyTarget, InviteKind, JoinRefusal, Op, PendingInvite, Removal, Role, STRIKE_GAP_MS, TakeDir, WindowKind, World};

use super::onboarding::At;

const DAY_MS: i64 = 86_400_000;

/// What this Mac's own space said about it that a face answers (N34, §6.7). Never on the
/// token's Mac: it re-proves its space by the sweep, and repairs it (N42).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Life {
    /// 401, or 403 `removed`: this identity is dead here.
    Revoked,
    /// The space's own side refused, twice at least 30 s apart (C7).
    NeedsOwner,
    /// The address answers as no space at all, twice at least 30 s apart.
    Gone,
    /// The page lacks a route this app calls.
    Behind,
}

/// This Mac's member life, if a face answers it.
pub fn life(w: &World) -> Option<Life> {
    if !matches!(w.space.role(), Some(Role::Member | Role::OwnerClass)) {
        return None;
    }
    let st = &w.members.standing;
    if st.revoked {
        return Some(Life::Revoked);
    }
    if let Some(s) = st.strike.filter(|s| s.confirmed()) {
        return Some(if s.gone { Life::Gone } else { Life::NeedsOwner });
    }
    st.behind.then_some(Life::Behind)
}

/// The space can't take this Mac's videos at all (N34): the takes here can only go to
/// Movies, and nothing is sent.
pub fn stranded_by_space(w: &World) -> bool {
    matches!(life(w), Some(Life::Revoked | Life::Gone))
}

/// An upload credential is used only while it has more than this left; nearer its end the
/// uploader mints it again (a credential lives an hour: WORKER-INTERFACE §3.7).
pub const CRED_LEFT_MS: i64 = 15 * 60_000;

/// Whether this queued take can still go up from this Mac: any, while the space takes this
/// Mac's videos; once it doesn't (removed, or gone), only one already holding an upload
/// credential with more than [`CRED_LEFT_MS`] left — N32's video already on its way, for
/// what is left of its hour (whole-spec audit #22). The rest wait here, stranded.
pub fn can_send(w: &World, q: &crate::world::Queued) -> bool {
    !stranded_by_space(w) || w.members.minted.get(&q.dir).is_some_and(|until| until.0 - w.wall.0 > CRED_LEFT_MS)
}

/// A new video would have nowhere to go: Start greys (N34's table; a removed owner Mac, as
/// WORKER-INTERFACE open question 18 recommends) — and while a switch's join runs, a new
/// video would belong to a space this Mac is leaving (law 14, DECISIONS B41 b).
pub fn start_greyed(w: &World) -> bool {
    switching(w) || matches!((w.space.role(), life(w)), (Some(Role::Member), Some(Life::Revoked | Life::Gone)) | (Some(Role::OwnerClass), Some(Life::Revoked)))
}

/// A switch's Join was clicked and hasn't answered for good: this Mac is on its way out of
/// its space.
pub fn switching(w: &World) -> bool {
    w.provisioned() && w.members.invite.as_ref().is_some_and(|i| i.joining)
}

/// Start's line while a switch's join runs.
pub fn start_note(w: &World) -> Option<String> {
    switching(w).then(|| copy::record_after_switch(inviter(w).unwrap_or_default()))
}

/// The space owner's name, as this Mac knows it (N29's short form).
fn owner_name(w: &World) -> String {
    w.space.identity.as_ref().map(|i| i.owner_name.clone()).unwrap_or_default()
}

/// Kind 2 of N38's banners, in this Mac's role's words (N34, §6.7) — `None` on the token's
/// Mac (it reads `space-degraded`).
pub fn banner(w: &World) -> Option<Banner> {
    let life = life(w)?;
    let name = owner_name(w);
    let amber = |id: &'static str, line: String, buttons: Vec<Button>| Banner { id, tone: Tone::Amber, line, detail: None, buttons, parts: vec![], long: None };
    Some(match (w.space.role()?, life) {
        (Role::Member, Life::Revoked) => amber("member-revoked", copy::member_revoked(&name), stranded_buttons(w, copy::leave_space_ellipsis())),
        (Role::Member, Life::NeedsOwner) => amber("space-needs-owner", copy::needs_owner(&name), vec![]),
        (Role::Member, Life::Gone) => amber("space-gone", copy::space_gone(&name), stranded_buttons(w, copy::leave_space_ellipsis())),
        (Role::Member, Life::Behind) => amber("space-needs-update-member", copy::needs_update_member(&name), vec![]),
        (Role::OwnerClass, Life::Revoked) => amber("owner-mac-removed", copy::owner_mac_removed(), stranded_buttons(w, copy::disconnect_ellipsis())),
        (Role::OwnerClass, Life::NeedsOwner | Life::Gone) => {
            amber("space-degraded-no-token-mac", copy::degraded_no_token(), vec![Button::new(Action::AddToken, copy::reconnect_here(), Style::Plain)])
        }
        (Role::OwnerClass, Life::Behind) => amber(
            "space-needs-update-no-token",
            copy::needs_update_owner_class(),
            vec![Button::new(Action::AddToken, copy::add_token_here(), Style::Plain)],
        ),
        (Role::TokenHolder, _) => return None,
    })
}

/// While takes wait: [ Save to Movies ] (one button for them all), then “Saved to Movies ✓”
/// with a quiet Discard… beside it; and the way out.
fn stranded_buttons(w: &World, exit: String) -> Vec<Button> {
    let stranded = super::settings::stranded(w);
    let unsaved: Vec<&TakeDir> = stranded.iter().filter(|q| !w.members.movies.contains(&q.dir)).map(|q| &q.dir).collect();
    let mut out = Vec::new();
    if !unsaved.is_empty() {
        out.push(save_button(w, Action::SaveStrandedToMovies, &unsaved, copy::save_n_to_movies(unsaved.len() as u64), Style::Plain));
    } else if let Some(first) = stranded.first() {
        out.push(Button::new(Action::SaveStrandedToMovies, copy::saved_to_movies(), Style::Plain).enabled(false));
        out.push(Button::new(Action::DiscardRow(first.dir.clone()), copy::discard_ellipsis(), Style::RedText));
    }
    out.push(Button::new(Action::Disconnect, exit, Style::Link));
    out
}

/// A Save to Movies button for these takes, as their copy stands: waiting while it's under
/// way (greyed — law 2 holds every exit meanwhile), asking again after one that couldn't
/// be made, else its own words (DECISIONS B54).
pub fn save_button(w: &World, action: Action, dirs: &[&TakeDir], label: String, style: Style) -> Button {
    if dirs.iter().any(|d| w.members.saving.contains(*d)) {
        Button::new(action, copy::saving_to_movies(), style).enabled(false)
    } else if dirs.iter().any(|d| w.members.save_failed.contains(*d)) {
        Button::new(action, copy::save_to_movies_again(), style)
    } else {
        Button::new(action, label, style)
    }
}

/// The name Save to Movies gives a take's file: its title — as the Library shows it — or
/// “shroom recording — Aug 21” in this Mac's calendar when it has none (N34).
pub fn movies_name(w: &World, dir: &TakeDir) -> String {
    let q = w.queue.iter().chain(&w.removed).chain(w.members.toss.values()).find(|q| &q.dir == dir);
    if let Some(q) = q {
        let title = super::library::title_of(w, &q.rec).unwrap_or_else(|| q.title.clone());
        if !title.trim().is_empty() {
            return title;
        }
    }
    let when = q.map(|q| q.stopped).or_else(|| w.saved.iter().find(|s| &s.dir == dir).map(|s| s.created)).unwrap_or(w.wall);
    copy::movies_untitled(&copy::day(when, w.mac.utc_offset_min))
}

/// A stranded stop's reason line: the matching banner's (N34) — and when the stop's own
/// probe found the page isn't the space's before any debounce confirmed it (whole-spec
/// audit #16), the line that page-gone reads in this Mac's role: the member's `space-gone`,
/// an owner-class Mac's degraded line, the token Mac's `space-degraded` (Region 2).
pub fn stranded_reason(w: &World) -> Option<String> {
    let name = owner_name(w);
    match (w.space.role()?, life(w)) {
        (Role::Member, Some(Life::Revoked)) => Some(copy::member_revoked(&name)),
        (Role::Member, Some(Life::Gone) | None) => Some(copy::space_gone(&name)),
        (Role::OwnerClass, Some(Life::Revoked)) => Some(copy::owner_mac_removed()),
        (Role::OwnerClass, Some(Life::Gone) | None) => Some(copy::degraded_no_token()),
        (Role::TokenHolder, _) if !w.space.token => Some(copy::degraded_no_token()),
        (Role::TokenHolder, _) => Some(copy::space_degraded()),
        _ => None,
    }
}

/// A stranded stop's owners' button (r4: “owners: [ Reconnect your space ]”): on the token
/// Mac, the repair mood (Settings · Space); on another owner-class Mac whose space's page is
/// gone, the token row here (N33). A removed owner Mac has nothing to reconnect to
/// (DEVIATIONS #21); members have no such door.
pub fn reconnect_action(w: &World) -> Option<Action> {
    match (w.space.role()?, life(w)) {
        (Role::TokenHolder, _) if w.space.token => Some(Action::ReconnectSpace),
        (Role::TokenHolder | Role::OwnerClass, Some(Life::Gone) | None) => Some(Action::AddToken),
        _ => None,
    }
}

// ── the invite window (N29–N31) ──

/// The pending invite's facts, as peek gave them.
fn pending(w: &World) -> Option<(&str, InviteKind, &str)> {
    match w.members.invite.as_ref()?.peek.get()? {
        PeekAnswer::Pending { host, kind, inviter, .. } => Some((host.as_str(), *kind, inviter.as_str())),
        _ => None,
    }
}

/// The inviter's name, if peek gave one.
fn inviter(w: &World) -> Option<&str> {
    match w.members.invite.as_ref()?.peek.get()? {
        PeekAnswer::Pending { inviter, .. } | PeekAnswer::Used { inviter, .. } => Some(inviter),
        PeekAnswer::Expired { inviter, .. } => inviter.as_deref(),
        PeekAnswer::SpaceMissing { .. } => None,
    }
}

fn kind(w: &World) -> Option<InviteKind> {
    match w.members.invite.as_ref()?.peek.get()? {
        PeekAnswer::Pending { kind, .. } | PeekAnswer::Expired { kind, .. } | PeekAnswer::Used { kind, .. } => Some(*kind),
        PeekAnswer::SpaceMissing { .. } => None,
    }
}

pub fn own_mac_invite(w: &World) -> bool {
    kind(w) == Some(InviteKind::OwnMac)
}

/// Where the invite window stands, when an invite is what it's about.
pub fn invite_at(w: &World) -> Option<At> {
    let inv = w.members.invite.as_ref()?;
    if let Some(r) = inv.refused {
        return Some(match r {
            JoinRefusal::Expired => At::JoinExpired,
            JoinRefusal::Used => At::JoinUsed,
            JoinRefusal::AlreadyHere => At::AlreadyHere,
            JoinRefusal::SpaceMissing => At::JoinSpaceMissing,
        });
    }
    if inv.joining {
        if w.inflight.ops.contains_key(&Op::Join) {
            return Some(At::Joining);
        }
        // A switch parked on what the old space still holds says so (N31) — never that the
        // space isn't answering.
        let in_a_space = w.provisioned() || w.space.token;
        return Some(if in_a_space && switch_blocked(w).is_some() { At::SwitchConfirm } else { At::JoinWaiting });
    }
    // Peek hasn't answered yet — checking, offline, or it couldn't get through: the waiting
    // posture (DEVIATIONS #20), with no default, never a face of the owner's wizard (B41 j).
    let Some(peek) = inv.peek.get() else { return Some(At::JoinWaiting) };
    Some(match peek {
        PeekAnswer::Expired { .. } => At::JoinExpired,
        PeekAnswer::Used { .. } => At::JoinUsed,
        PeekAnswer::SpaceMissing { .. } => At::JoinSpaceMissing,
        PeekAnswer::Pending { host, .. } => match &w.space.identity {
            // A page that's behind rests here too, Join greyed on the needs-update line.
            None if !w.space.token => At::MemberWelcome,
            // A half-set-up space counts (N31).
            None => At::SwitchConfirm,
            Some(id) if id.host != *host => At::SwitchConfirm,
            // The same space: `/me` says whether this Mac's identity still lives (§6.2 step 4).
            Some(_) => {
                let st = &w.members.standing;
                let asked = inv.peek.at().unwrap_or(Mono(0));
                // The space's own side faulted since the peek, and C7 confirmed it: the
                // identity wasn't refused, so this Mac is still here (B41 l).
                let faulted = st.at.is_some_and(|t| t >= asked) && st.strike.is_some_and(|s| s.confirmed());
                if st.revoked {
                    At::MemberWelcome
                } else if !w.space.device_key || st.fine.is_some_and(|t| t >= asked) || faulted {
                    At::AlreadyHere
                } else {
                    At::JoinWaiting
                }
            }
        },
    })
}

/// The switch's Join waits for what the old space still holds (N31): uploads draining, a
/// recovered take waiting.
fn switch_blocked(w: &World) -> Option<String> {
    let draining = super::settings::draining(w);
    if draining > 0 {
        Some(copy::switch_uploading(draining as u64))
    } else if !w.saved.is_empty() {
        Some(copy::switch_saved_waits())
    } else {
        None
    }
}

/// Join on the switch: greyed while the old space still holds something, and mid-take with
/// law 4's line.
pub fn switch_join_enabled(w: &World) -> bool {
    switch_blocked(w).is_none() && !w.take_open() && !behind(w) && !super::settings::saving_stranded(w)
}

/// The invite's space has a page older than this app needs (U3, open question 15).
pub fn behind(w: &World) -> bool {
    w.members.invite.as_ref().is_some_and(|i| i.behind)
}

/// member-welcome's Join: greyed while the invite's page is behind.
pub fn welcome_join_enabled(w: &World) -> bool {
    !behind(w)
}

/// The needs-update line in the invite window (open question 15), in the words of the role
/// the invite would give this Mac.
fn behind_line(w: &World, name: &str) -> Option<String> {
    behind(w).then(|| if own_mac_invite(w) { copy::needs_update_owner_class() } else { copy::needs_update_member(name) })
}

/// The join couldn't get through: worded with the inviter's name when peek gave one.
fn not_answering(w: &World, name: &str) -> String {
    // An own-Mac link is the owner's own space: never their name on their own screen (B38).
    if own_mac_invite(w) {
        copy::own_space_not_answering()
    } else if name.is_empty() {
        copy::join_not_answering_unnamed()
    } else {
        copy::join_not_answering(name)
    }
}

/// The switch's faces (N31).
pub fn switch_faces(w: &World) -> Vec<&'static str> {
    if super::settings::draining(w) > 0 {
        vec!["join-switch-confirm-uploads"]
    } else if !w.saved.is_empty() {
        vec!["join-switch-confirm-saved-take"]
    } else {
        vec![]
    }
}

/// The invite window's faces that aren't a plan.
pub fn invite_body(w: &World, at: &At) -> InviteBody {
    let name = inviter(w).unwrap_or_default().to_string();
    let mut b = InviteBody { title: None, space: None, lines: vec![], login: None, buttons: vec![], door: None, spinner: false };
    match at {
        At::MemberWelcome => {
            let (host, kind, _) = pending(w).unwrap_or(("", InviteKind::Member, ""));
            if kind == InviteKind::OwnMac {
                b.title = Some(copy::add_this_mac());
                b.lines = vec![copy::same_videos(host), copy::member_step_here()];
            } else {
                b.title = Some(copy::invited_you(&name));
                b.space = Some(copy::their_space(host));
                b.lines = vec![copy::member_truth(&name), copy::member_step()];
            }
            let off = w.session.fields.login_off;
            b.lines.extend(behind_line(w, &name));
            b.login = Some(Toggle { action: Action::SetLoginBox(off), label: copy::login_box(), on: !off, enabled: true });
            b.buttons = vec![
                Button::new(Action::Join, copy::join(), Style::Filled).enabled(welcome_join_enabled(w)),
                Button::new(Action::Later, copy::later(), Style::Plain),
            ];
        }
        At::JoinWaiting => {
            b.spinner = true;
            // A space-side answer to this Mac's own space, short of C7's second look, reads
            // as not answering too.
            let struck = w.members.standing.strike.is_some() && w.members.invite.as_ref().is_some_and(|i| !i.joining);
            if w.online() == Some(false) {
                b.lines = vec![copy::join_offline()];
            } else if struck || [Op::Peek, Op::Join, Op::Standing].iter().any(|op| w.inflight.failures.contains_key(op)) {
                b.lines = vec![not_answering(w, &name)];
            }
        }
        At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing => {
            if let Some((line, door)) = refusal(w, at) {
                b.lines = vec![line];
                b.door = door;
            }
        }
        At::AlreadyHere => {
            // Its wording follows this Mac's role, not the invite's kind (U5).
            b.lines = vec![if w.space.owner_class() { copy::already_here_own() } else { copy::already_here(&name) }];
            b.buttons = vec![Button::new(Action::CloseWindow, copy::close(), Style::Plain)];
        }
        At::SwitchConfirm => {
            b.title = Some(copy::invited_you(&name));
            b.space = pending(w).map(|(host, _, _)| copy::their_space(host));
            match &w.space.identity {
                Some(id) => {
                    b.lines.push(copy::switch_line(&id.host, &name));
                    // Said unless the old space says another Mac keeps a token too (U9).
                    if w.space.token_holder() && w.members.standing.only_holder != Some(false) {
                        b.lines.push(copy::switch_only_holder());
                    }
                }
                None => b.lines.push(copy::switch_half_set_up(&name)),
            }
            b.lines.extend(switch_blocked(w));
            b.lines.extend(behind_line(w, &name));
            // Takes the old space can't take don't block (they can't drain): Leave's rule —
            // one line, and Movies first (N36, B41 b).
            let takes = super::settings::stranded(w);
            let stranded = takes.len() as u64;
            let dirs: Vec<&TakeDir> = takes.iter().map(|q| &q.dir).collect();
            let host = w.space.identity.as_ref().map(|i| i.host.as_str()).unwrap_or_default();
            if stranded > 0 {
                b.lines.push(copy::switch_stranded(stranded, host));
            }
            let join = Button::new(Action::Join, copy::join_space(&name), Style::Filled);
            let join = if w.take_open() { Button { enabled: false, note: Some(copy::after_take()), ..join } } else { join.enabled(switch_join_enabled(w)) };
            b.buttons = vec![join, Button::new(Action::Stay, copy::stay(), Style::Plain)];
            if stranded > 0 {
                b.buttons.push(save_button(w, Action::SaveStrandedToMovies, &dirs, copy::save_them_to_movies(stranded), Style::Plain));
            }
        }
        _ => {}
    }
    b
}

/// A dead invite's line, and its paste door where a new link is the way on (N31): the same
/// words whether peek knew it at arrival or Join's answer said it.
fn refusal(w: &World, at: &At) -> Option<(String, Option<String>)> {
    let name = inviter(w).unwrap_or_default().to_string();
    Some(match at {
        At::JoinExpired => (
            if kind(w) == Some(InviteKind::OwnMac) {
                copy::own_mac_expired()
            } else if name.is_empty() {
                copy::invite_unknown()
            } else {
                copy::invite_expired(&name)
            },
            Some(copy::new_link_door()),
        ),
        // An own-Mac link: the page's own-Mac words, never “ask Bagrat” on Bagrat's Mac (B54).
        At::JoinUsed => (if kind(w) == Some(InviteKind::OwnMac) { copy::own_mac_used() } else { copy::invite_used(&name) }, Some(copy::new_link_door())),
        At::JoinSpaceMissing => (if name.is_empty() { copy::space_missing_unnamed() } else { copy::space_missing(&name) }, None),
        _ => return None,
    })
}

/// Join was clicked and the space answered it for good, short of the 200 — expired, used,
/// its space missing: the failure renders under the joining row (N31's table). Already-here
/// is the whole window's face; a link peek knew dead at arrival never had a plan
/// (DEVIATIONS #20).
fn refused_after_join(w: &World) -> bool {
    w.members.invite.as_ref().is_some_and(|i| i.refused.is_some_and(|r| r != JoinRefusal::AlreadyHere))
}

/// The plan is about the invite (not this Mac's own space): its join runs, or answered
/// for good short of the 200.
fn about_invite(w: &World) -> bool {
    w.members.invite.as_ref().is_some_and(|i| i.joining) || refused_after_join(w)
}

/// The plan as a joining Mac sees it — the space's row, then row six (N29): about the
/// invite's space while the join runs (and when Join's answer refused it), about this Mac's
/// own space after.
pub fn uses_plan(w: &World, at: &At) -> bool {
    match at {
        At::Joining | At::JoinWaiting => w.members.invite.as_ref().is_some_and(|i| i.joining),
        At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing => refused_after_join(w),
        At::Welcome | At::MemberWelcome | At::AlreadyHere | At::SwitchConfirm => false,
        _ => w.space.role().is_some_and(|r| r != Role::TokenHolder),
    }
}

/// The space the member plan is about: its owner's name (`None` on the owner's own Macs —
/// “Your space”) and its address.
fn plan_space(w: &World) -> (Option<String>, String) {
    if let Some(inv) = w.members.invite.as_ref().filter(|_| about_invite(w)) {
        let host = inv.host.clone().unwrap_or_default();
        // Peek's facts, whatever it answered (a replayed join may hear `used` — its own).
        return ((kind(w) != Some(InviteKind::OwnMac)).then(|| inviter(w).unwrap_or_default().to_string()).filter(|n| !n.is_empty()), host);
    }
    match &w.space.identity {
        Some(id) => ((id.role == Role::Member).then(|| id.owner_name.clone()), id.host.clone()),
        None => (None, String::new()),
    }
}

pub fn plan(w: &World, at: &At) -> PlanView {
    let (name, host) = plan_space(w);
    let joined = w.provisioned() && !about_invite(w);
    let first = if joined {
        PlanRow::new(Glyph::Done, copy::space_row_done(name.as_deref(), &host), &[])
    } else {
        PlanRow::new(Glyph::Current, copy::joining_row(name.as_deref()), &[])
    };
    let six = if joined && w.row_six_done() {
        PlanRow::new(Glyph::Done, copy::row_done(6, super::onboarding::row_fact(w, 6).as_deref()), &[])
    } else if joined {
        PlanRow::new(Glyph::Current, copy::row_current(6), &[])
    } else {
        PlanRow::new(Glyph::Ahead, copy::row_ahead(6), copy::row_ahead_marks(6))
    };
    let nest = match at {
        At::Joining => Nest::Calm { line: copy::joining_line(), parts: vec![], long: None },
        At::JoinWaiting => Nest::Offline { line: if w.online() == Some(false) { copy::join_offline() } else { not_answering(w, inviter(w).unwrap_or_default()) } },
        At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing => match refusal(w, at) {
            Some((line, door)) => Nest::JoinRefused { line, door },
            None => Nest::Checking,
        },
        _ => super::onboarding::nest(w, at),
    };
    // The row's place in the rows shown (two): the owner's row six is this plan's second.
    let current = if joined { at.row().map(|r| if r == 6 { 2 } else { 1 }) } else { Some(1) };
    PlanView { header: copy::space_header(name.as_deref()), rows: vec![first, six], current, nest }
}

// ── the Members tab (N28, N33) ──

fn day(w: &World, t: Wall) -> String {
    copy::day(t, w.mac.utc_offset_min)
}

fn expired(w: &World, inv: &PendingInvite) -> bool {
    w.wall >= inv.expires
}

fn pill(w: &World, born: &Born, line: String) -> LinkPill {
    let host = w.space.identity.as_ref().map(|i| i.host.as_str()).unwrap_or_default();
    let copied = matches!(&w.session.copied, Some((CopyTarget::Invite(c), _)) if *c == born.link);
    LinkPill {
        prefix: copy::link_prefix(host),
        link: born.link,
        copy: Button::new(Action::CopyInvite(born.link), if copied { copy::copied() } else { copy::copy_link() }, Style::Filled),
        line,
    }
}

fn remove(row: crate::scene::RowRef) -> Button {
    Button::new(Action::Remove(row), copy::remove_ellipsis(), Style::RedText)
}

pub fn pane(w: &World) -> MembersBody {
    use crate::scene::RowRef;
    let empty = crate::world::Roster::default();
    let roster = w.members.roster.get().unwrap_or(&empty);
    let f = &w.session.fields.settings;
    let born = f.born.as_ref();
    // The owner's Mac rows wait for a second Mac (Bagrat, 2026-10-14, B137): with one, the
    // You row says it all. Macs are counted by name, so a second record of this same Mac
    // (an earlier enroll's) isn't a second Mac.
    let names: std::collections::BTreeSet<&str> = roster.macs.iter().map(|m| m.name.as_str()).collect();
    let macs = roster
        .macs
        .iter()
        .filter(|m| names.len() > 1 && !m.this_mac)
        .map(|m| MemberRowView {
            key: m.id.clone(),
            line: copy::your_mac(&m.name, &day(w, m.joined)),
            // A Mac that keeps the token mints from it: a device key can't stop it (N33).
            fact: m.token.then(copy::keeps_token),
            buttons: if m.token { vec![] } else { vec![remove(RowRef::Mac(m.id.clone()))] },
            born: None,
        })
        .collect();
    let mut rows: Vec<MemberRowView> = roster
        .members
        .iter()
        .map(|m| MemberRowView {
            key: m.id.clone(),
            line: copy::joined_row(&m.label, &day(w, m.joined)),
            fact: None,
            buttons: vec![remove(RowRef::Member(m.id.clone()))],
            born: None,
        })
        .collect();
    let renewing = w.inflight.ops.contains_key(&Op::Invite);
    // A pending own-Mac link has no row in v1 (§6.4).
    for inv in roster.invites.iter().filter(|i| !i.own_mac) {
        let label = inv.label.clone().unwrap_or_default();
        let this = born.filter(|b| b.invite == inv.id);
        let (line, mut buttons) = if this.is_some() {
            (copy::invited_now(&label), vec![])
        } else if expired(w, inv) {
            // A row face, nothing blocked (N18).
            (copy::invited_expired(&label, &day(w, inv.created)), vec![Button::new(Action::NewLink(inv.id.clone()), copy::new_link(), Style::Plain).enabled(!renewing)])
        } else {
            let left = (inv.expires.0 - w.wall.0 + DAY_MS - 1).div_euclid(DAY_MS).max(1) as u64;
            (copy::invited_row(&label, &day(w, inv.created), left), vec![])
        };
        buttons.push(remove(RowRef::Invite(inv.id.clone())));
        rows.push(MemberRowView { key: inv.id.clone(), line, fact: None, buttons, born: this.map(|b| pill(w, b, copy::send_link(&label))) });
    }
    let typed = f.invite_name.trim();
    MembersBody {
        title: copy::members_title(),
        line: copy::members_truth(),
        you: copy::you(),
        add_mac: Button::new(Action::AddMac, copy::add_another_mac(), Style::Link).enabled(!renewing),
        mac_link: born.filter(|b| b.own_mac).map(|b| pill(w, b, copy::own_mac_link_line())),
        macs,
        rows,
        name: TextField { placeholder: copy::who_for(), value: f.invite_name.clone(), enabled: true, edits: f.invite_edits },
        // The composer is the ask; it drops to outline while a new link — a member's or an
        // own Mac's — is on screen.
        // Either link's filled Copy link is then the face's one default (law 8).
        invite: Button::new(Action::Invite, copy::invite(), if born.is_some() { Style::Plain } else { Style::Filled })
            .enabled(!typed.is_empty() && !renewing),
        invite_line: match fell_through(w) {
            Some(Op::Remove) => copy::remove_failed(),
            Some(_) => copy::invite_failed(),
            None => copy::invite_line(),
        },
        from: from_name(w),
    }
}

/// Who invites say they're from (B158): the name this Mac's space keeps — macOS's short
/// form until it's changed — under the composer, so it's read before a link goes out.
fn from_name(w: &World) -> FromName {
    match &w.session.fields.settings.from {
        Some(typed) => FromName::Changing {
            label: copy::invites_from_label(),
            field: TextField { placeholder: copy::invites_from_label(), value: typed.clone(), enabled: true, edits: w.session.fields.settings.from_edits },
            save: Button::new(Action::SaveFrom, copy::save(), Style::Link).enabled(!typed.trim().is_empty()),
        },
        None => {
            let name = owner_name(w);
            FromName::Shown { line: Rich::new(copy::invites_from(&name), &[(name.as_str(), Em::Strong)]), change: Button::new(Action::ChangeFrom, copy::change(), Style::Link) }
        }
    }
}

/// The Members face's last write that failed and hasn't been answered since (law 12: a
/// failure is a face — DEVIATIONS #28): an invite's link or a removal, while nothing of the
/// same kind runs, and only for the visit to this window it was asked in.
fn fell_through(w: &World) -> Option<Op> {
    let entered = w.session.windows.get(&WindowKind::Settings).map(|d| d.entered)?;
    [Op::Invite, Op::Remove]
        .into_iter()
        .filter(|op| !w.inflight.ops.contains_key(op))
        .filter_map(|op| w.inflight.failures.get(&op).filter(|f| f.at >= entered).map(|f| (f.at, op)))
        .max()
        .map(|(_, op)| op)
}

/// The Members faces showing.
pub fn faces(w: &World) -> Vec<&'static str> {
    let mut faces = vec!["settings-members", "invite-born"];
    if w.session.fields.settings.born.as_ref().is_some_and(|b| b.own_mac) {
        faces.push("add-another-mac");
    }
    faces
}

/// member-remove-confirm (N23's law: no Return default, Esc ≡ the left button).
pub fn remove_sheet(w: &World, r: &Removal) -> Option<Sheet> {
    let roster = w.members.roster.get()?;
    let back = |label: String| Button::new(Action::Back, label, Style::Plain);
    let confirm = |label: String| Button::new(Action::ConfirmRemove, label, Style::RedOutline);
    let (face, title, line, buttons) = match r {
        Removal::Member(id) => {
            let m = roster.members.iter().find(|m| &m.id == id)?;
            ("member-remove-confirm-member", copy::remove_title(&m.label), copy::remove_member_line(&m.label), vec![back(copy::cancel()), confirm(copy::remove())])
        }
        Removal::Invite(id) => {
            let i = roster.invites.iter().find(|i| &i.id == id)?;
            let label = i.label.clone().unwrap_or_default();
            ("member-remove-confirm-invite", copy::cancel_invite_title(&label), copy::cancel_invite_line(), vec![back(copy::keep_it()), confirm(copy::cancel_invite())])
        }
        Removal::Mac(id) => {
            let m = roster.macs.iter().find(|m| &m.id == id)?;
            ("member-remove-confirm-own-mac", copy::remove_title(&m.name), copy::remove_mac_line(), vec![back(copy::cancel()), confirm(copy::remove())])
        }
    };
    Some(Sheet { id: "member-remove-confirm", faces: vec![face], title, lines: vec![line], blocked: None, buttons })
}

/// The member label of a video someone else made here, as the roster knows it.
pub fn label_of(w: &World, owner: &str) -> Option<String> {
    w.members.roster.get().and_then(|r| r.labels.get(owner).cloned())
}

/// The instants these faces would read differently on their own: C7's second look, and a
/// pending invite running out while Members shows.
pub fn wakes(w: &World, out: &mut Vec<Mono>) {
    if let Some(s) = w.members.standing.strike.filter(|s| !s.confirmed()) {
        out.push(s.first.plus(STRIKE_GAP_MS));
    }
    if w.window_visible(crate::world::WindowKind::Settings) && super::settings::tab(w) == crate::world::SettingsTab::Members {
        if let Some(r) = w.members.roster.get() {
            let soonest = r.invites.iter().filter(|i| !expired(w, i)).map(|i| i.expires.0 - w.wall.0).min();
            out.extend(soonest.map(|ms| w.now.plus(ms.max(0) as u64)));
        }
    }
}
