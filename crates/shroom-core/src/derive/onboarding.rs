//! The onboarding window: the welcome, then the plan — six rows marked from the facts in
//! the order token verify → gates → bucket → Worker → subdomain, row six per Mac (Regions
//! 1–2, and Region 3's permissions chapter). No step is stored: where the plan stands is
//! computed here each time (law 1).

use crate::copy;
use crate::scene::{
    Action, Button, Choice, Glyph, Nest, OnboardingBody, Part, PlanRow, PlanView, Rich, Style, Toggle, WelcomeBody, Window,
};
use crate::time::Mono;
use crate::world::{Access, Confirm, FailClass, Fork, LaunchKind, Op, Paste, Role, TokenVerdict, WindowKind, World};

/// How long a quiet filter line shows before it fades (N15's filter-not-scold).
pub const HINT_MS: u64 = 3_000;
/// The gates' auto-poll interval while one shows (N14).
pub const GATE_POLL_MS: u64 = 5_000;
/// How long the card watch runs before its quiet line (N74).
pub const CARD_LONG_MS: u64 = 45_000;
/// How long switching the name on runs before wiring's quiet line (N74).
pub const NAME_LONG_MS: u64 = 180_000;
/// Wiring's part that switches the name on: the long one (N74).
const NAME_PART: u8 = 3;

/// Where the onboarding window stands — derived, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum At {
    Welcome,
    Account,
    SigningUp,
    Token,
    /// A paste is being verified or saved: the token row, its spinner on the field.
    TokenChecking,
    Garbled,
    WrongPowers,
    PickAccount,
    /// The probe pass under the card row hasn't answered.
    Card,
    GateEmail,
    GateCard,
    Name,
    NameTaken,
    Wiring,
    Found(u32),
    Done,
    PermMic,
    /// macOS's own camera prompt, chained from the mic's answer (N43).
    PermCamera,
    PermScreen,
    Relaunching,
    Priming,
    AllSet,
    // ── Region 6: the invite window (N29–N31) ──
    MemberWelcome,
    Joining,
    /// A join — or an arrival — waiting on its space: offline, or not answering.
    JoinWaiting,
    JoinExpired,
    JoinUsed,
    JoinSpaceMissing,
    AlreadyHere,
    SwitchConfirm,
}

impl At {
    /// The plan row this sits under (1–6); `None` on the welcome and at all-set.
    pub fn row(&self) -> Option<u8> {
        Some(match self {
            At::Welcome | At::AllSet => return None,
            At::MemberWelcome | At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing | At::AlreadyHere | At::SwitchConfirm => return None,
            At::Joining | At::JoinWaiting => 1,
            At::Account | At::SigningUp => 1,
            At::Token | At::TokenChecking | At::Garbled | At::WrongPowers | At::PickAccount => 2,
            At::Card | At::GateEmail | At::GateCard => 3,
            At::Name | At::NameTaken => 4,
            At::Wiring | At::Found(_) => 5,
            At::Done | At::PermMic | At::PermCamera | At::PermScreen | At::Relaunching | At::Priming => 6,
        })
    }

    /// The step waits on the network (N18: `wizard-offline` can interpose here).
    pub fn network_bound(&self) -> bool {
        matches!(
            self,
            At::TokenChecking | At::Card | At::GateEmail | At::GateCard | At::Name | At::NameTaken | At::Wiring | At::Found(_)
        )
    }

    pub fn id(&self) -> &'static str {
        match self {
            At::Welcome => "welcome",
            At::Account => "account",
            At::SigningUp => "account-signing-up",
            At::Token | At::TokenChecking => "token",
            At::Garbled => "token-garbled",
            At::WrongPowers => "token-wrong-powers",
            At::PickAccount => "token-pick-account",
            At::Card => "card",
            At::GateEmail => "gate-email",
            At::GateCard => "gate-card",
            At::Name => "name",
            At::NameTaken => "name-taken",
            At::Wiring => "wiring",
            At::Found(_) => "space-found",
            At::Done => "done",
            At::PermMic => "perm-mic",
            At::PermCamera => "perm-camera",
            At::PermScreen => "perm-screen",
            At::Relaunching => "relaunching",
            At::Priming => "perm-priming",
            At::AllSet => "all-set",
            At::MemberWelcome => "member-welcome",
            At::Joining => "joining",
            At::JoinWaiting => "join-waiting",
            At::JoinExpired => "join-expired",
            At::JoinUsed => "join-used",
            At::JoinSpaceMissing => "join-space-missing",
            At::AlreadyHere => "join-already-here",
            At::SwitchConfirm => "join-switch-confirm",
        }
    }
}

/// The welcome until "Set up my space" (or a saved token, or a provisioned space).
fn welcome(w: &World) -> bool {
    !w.provisioned() && !w.prefs.setup_chosen && !w.space.token && w.sweep.paste.is_none()
}

/// The six rows, done or not, from the facts.
pub fn rows_done(w: &World) -> [bool; 6] {
    let prov = w.provisioned();
    // A stored token was proved when it was saved; it stays ticked while this entry's
    // re-probe runs (the spinner sits on the card row, never a blank plan) and un-ticks
    // only when the re-probe finds it dead (`token`'s re-entry, SM:429-435).
    let dead = matches!(w.sweep.token.get(), Some(TokenVerdict::WrongPowers | TokenVerdict::Revoked));
    let token = prov || (w.space.token && !dead);
    // The chapter is entered only once the name row has ticked (N45): after a permission
    // relaunch the card and name rows stay ticked while the re-probe hasn't answered them
    // — the token's rule — and un-tick only on what it finds.
    let held = w.session.chapter;
    let gates_clear = w.sweep.gates.get().map_or(held, |g| !g.email && !g.card);
    // The card step's pass also surveys the name and the label (B26): both answered.
    let surveyed = |answered: bool| answered || held;
    let card = prov || (token && gates_clear && surveyed(w.sweep.subdomain.get().is_some()) && surveyed(w.sweep.label.get().is_some()));
    let name = prov || (card && w.sweep.subdomain.get().map_or(held, |s| s.is_some()));
    // space-found: rows three to five tick in order (whole-spec audit #9) — the wiring row's
    // nest then says what was found while this Mac joins it.
    let found = name && matches!(w.sweep.found.get(), Some(Some(_)));
    [
        // Box 1: self-reported at proceed; a stored token re-proves it (N14).
        prov || w.space.token || w.session.fork == Fork::Affirmed,
        token,
        card,
        name,
        prov || found,
        w.row_six_done(),
    ]
}

pub fn at(w: &World) -> At {
    // An invite this Mac holds is what the window is about (Region 6) — until it's joined.
    if let Some(a) = super::members::invite_at(w) {
        return a;
    }
    if welcome(w) {
        return At::Welcome;
    }
    let done = rows_done(w);
    // A space already there: its line under the ticked wiring row while this Mac joins it
    // (the re-place and the enroll); row six becomes current once it has (N42).
    if let (false, true, Some(Some(n))) = (w.provisioned(), done[4], w.sweep.found.get()) {
        return At::Found(*n);
    }
    // The permissions come before the wiring (N45): once the name row ticks, wiring runs
    // out of sight under the chapter, and is the last step only if it's still going when
    // the chapter ends.
    let row = match done.iter().position(|d| !d) {
        Some(4) if !done[5] => 5,
        Some(r) => r,
        None => return At::AllSet,
    };
    match row {
        0 if w.session.fork == Fork::SigningUp => At::SigningUp,
        0 => At::Account,
        1 => match &w.sweep.paste {
            Some(Paste::Garbled) => At::Garbled,
            Some(Paste::WrongPowers) => At::WrongPowers,
            Some(Paste::PickAccount { .. }) => At::PickAccount,
            Some(Paste::Checking(_) | Paste::Saving { .. }) => At::TokenChecking,
            // Only the repair mood tells a token for another account apart (Settings · Space).
            Some(Paste::OtherAccount) | None => At::Token,
        },
        2 => match w.sweep.gates.get() {
            Some(g) if g.email => At::GateEmail,
            Some(g) if g.card => At::GateCard,
            _ => At::Card,
        },
        3 if !w.session.fields.name.is_empty() && w.sweep.taken.as_deref() == Some(w.session.fields.name.as_str()) => At::NameTaken,
        3 => At::Name,
        4 => At::Wiring,
        _ if !w.session.chapter => At::Done,
        _ if w.mac.mic.is(&Access::NotAsked) || w.mac.mic.get().is_none() => At::PermMic,
        // Only while the prompt the mic's answer chained is up: a relaunch never parks here.
        _ if w.mac.camera_asked && w.mac.camera.is(&Access::NotAsked) => At::PermCamera,
        _ if !w.mac.screen.is(&true) && w.launch.ending => At::Relaunching,
        _ if !w.mac.screen.is(&true) => At::PermScreen,
        _ => At::Priming,
    }
}

/// `wizard-offline`: a network-bound step while offline, or after a transport failure no
/// success has answered since (ARCHITECTURE §3.3) — any wizard call answering clears the
/// transport failures before it (`apply::answered`), and every one of them is retried by
/// a backoff or a re-probe, so the face never outlives the network's return.
pub fn offline(w: &World, at: &At) -> bool {
    at.network_bound()
        && (w.online() == Some(false)
            || Op::WIZARD.iter().any(|op| w.inflight.failures.get(op).is_some_and(|f| f.class == FailClass::Transport)))
}

/// The wizard calls a face waits on, by where it stands: the ones whose failure the
/// “something unexpected” face is about.
pub(crate) fn ops_of(at: &At) -> &'static [Op] {
    match at {
        At::TokenChecking => &[Op::Validate, Op::SaveToken],
        At::Card | At::GateEmail | At::GateCard => &[Op::Sweep],
        At::Name | At::NameTaken => &[Op::Claim],
        At::Wiring | At::Found(_) => &[Op::Wire],
        _ => &[],
    }
}

/// A failure nothing else classifies — refused, unclassified, or a call that panicked — on
/// the call this face waits on, that no answer has cleared since (DEVIATIONS #25): the
/// wizard says so instead of spinning forever. A transport failure is `wizard-offline`'s; a
/// call that ran out of time (or Cloudflare busy) just tries again, calmly.
pub fn unexpected_in(w: &World, ops: &[Op]) -> bool {
    ops.iter().any(|op| w.inflight.failures.get(op).is_some_and(|f| matches!(f.class, FailClass::Refused | FailClass::Unclassified | FailClass::Panicked)))
}

fn fresh(at: Option<Mono>, now: Mono) -> bool {
    at.is_some_and(|t| now < t.plus(HINT_MS))
}

/// Someone else's `v` is in the account, so the space's pages took another label (N42):
/// what the survey **found** — the space's own record of it once provisioned (kept from
/// its setup), else this pass's. Never the label alone: a build that spares `v` passes it
/// by without a look, and the row would then say “v was taken” of a `v` nobody saw.
fn clash(w: &World) -> bool {
    match &w.space.identity {
        Some(id) => id.foreign_v,
        None => w.sweep.foreign_v.is(&true),
    }
}

/// The link host as the plan states it: the space's own once provisioned, else the
/// claimed or found name under the survey's label.
pub fn host(w: &World) -> Option<String> {
    if let Some(id) = &w.space.identity {
        return Some(id.host.clone());
    }
    let sub = w.sweep.subdomain.get()?.as_ref()?;
    Some(format!("{}.{sub}.workers.dev", w.sweep.label()?))
}

pub(crate) fn row_fact(w: &World, row: u8) -> Option<String> {
    match row {
        // The account chosen among several (N42) — the space's, for good: kept with the
        // space once provisioned, from the token's verdict before.
        1 => match (&w.space.identity, w.sweep.token.get()) {
            (Some(id), _) => id.account_name.clone(),
            (None, Some(TokenVerdict::Valid { account, several: true })) => Some(account.name.clone()),
            _ => None,
        },
        4 => host(w).map(|h| copy::name_fact(&h, clash(w))),
        6 => matches!(w.mac.mic.get(), Some(Access::Denied | Access::Restricted)).then(copy::mic_off_fact),
        _ => None,
    }
}

pub fn derive(w: &World) -> Option<Window<OnboardingBody>> {
    // An agent's setup runs the window's plan with the window shut (N88): no window to show.
    if w.setup_headless() {
        return None;
    }
    slice(w)
}

/// The setup window's slice, whether it shows or not: what an agent's `shroom setup` ask is
/// held to (N88), as a click is held to what the window offers.
pub fn slice(w: &World) -> Option<Window<OnboardingBody>> {
    let door = w.session.windows.get(&WindowKind::Onboarding)?;
    if !w.window_visible(WindowKind::Onboarding) {
        return None;
    }
    let at = at(w);
    let offline = offline(w, &at);
    let unexpected = !offline && unexpected_in(w, ops_of(&at));
    let mut faces = Vec::new();
    if unexpected {
        faces.push("wizard-unexpected");
    }
    // Only the window the installer's launch opened is the first launch's.
    if at == At::Welcome && matches!(w.launch.kind, LaunchKind::Install { .. }) && door.opened == w.launch.booted {
        faces.push("first-launch");
    }
    let invite_face = matches!(at, At::MemberWelcome | At::AlreadyHere | At::SwitchConfirm)
        || matches!(at, At::JoinWaiting | At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing) && !super::members::uses_plan(w, &at);
    let own_mac = super::members::own_mac_invite(w);
    match at {
        At::MemberWelcome if own_mac => faces.push("member-welcome-own-mac"),
        At::Joining if own_mac => faces.push("joining-own-mac"),
        At::SwitchConfirm => faces.extend(super::members::switch_faces(w)),
        At::Done if w.space.role().is_some_and(|r| r != Role::TokenHolder) => faces.push("member-done"),
        At::AllSet if w.space.role() == Some(Role::Member) => faces.push("member-all-set"),
        At::AllSet if w.space.role() == Some(Role::OwnerClass) => faces.push("member-all-set-own-mac"),
        _ => {}
    }
    let body = if at == At::Welcome {
        OnboardingBody::Welcome(welcome_body(w))
    } else if invite_face {
        OnboardingBody::Invite(super::members::invite_body(w, &at))
    } else if super::members::uses_plan(w, &at) {
        OnboardingBody::Plan(super::members::plan(w, &at))
    } else {
        if clash(w) {
            faces.push("name-v-clash");
        }
        OnboardingBody::Plan(plan(w, &at, offline))
    };
    // The face replaces the nest under the current row — but the name row keeps its field
    // and Claim (the claim is only ever the person's click), its line said beneath.
    let replaced = unexpected && !matches!(at, At::Name | At::NameTaken);
    let ret = if offline || replaced { None } else { ret(w, &at) };
    let id = if offline {
        "wizard-offline"
    } else if replaced {
        "wizard-unexpected"
    } else {
        at.id()
    };
    // The Uninstall sheet over the welcome (B160): its face, and Esc ≡ Cancel (N23).
    let sheet = matches!(&body, OnboardingBody::Welcome(b) if b.sheet.is_some());
    if sheet {
        faces.push("uninstall-confirm");
    }
    Some(Window { id, faces, body, ret, esc: sheet.then_some(Action::Back), activates: door.asked })
}

/// What Return does: the surface's one default (law 8), as the fixtures give it.
fn ret(w: &World, at: &At) -> Option<Action> {
    match at {
        // The open invite door is a focused field, and a focused field's Return belongs to
        // it (law 8) — it validates on paste and has no Continue, so Return does nothing.
        // Under the Uninstall sheet, nothing (N23's law: no Return default).
        At::Welcome if w.session.confirm.is_some() => None,
        At::Welcome => Some(Action::SetUp),
        At::Account => Some(Action::HaveOne),
        At::SigningUp => Some(Action::SignedUp),
        At::Token | At::TokenChecking | At::Garbled | At::WrongPowers => Some(Action::CreateToken),
        At::PickAccount => Some(Action::UseAccount),
        At::GateCard => Some(Action::AddCard),
        At::Name | At::NameTaken => Some(Action::Claim),
        At::Done => Some(Action::AllowRecording),
        At::AllSet => Some(Action::RecordNow),
        // Greyed while the invite's page is behind (open question 15): no default.
        At::MemberWelcome => super::members::welcome_join_enabled(w).then_some(Action::Join),
        // Greyed while the old space still holds something, or mid-take: no default.
        At::SwitchConfirm => super::members::switch_join_enabled(w).then_some(Action::Join),
        _ => None,
    }
}

fn welcome_body(w: &World) -> WelcomeBody {
    let f = &w.session.fields;
    WelcomeBody {
        paragraphs: vec![Rich::new(copy::welcome_what(), copy::WELCOME_WHAT_MARKS), Rich::new(copy::welcome_steps(), &[])],
        login: Toggle { action: Action::SetLoginBox(f.login_off), label: copy::login_box(), on: !f.login_off, enabled: true },
        buttons: vec![Button::new(Action::SetUp, copy::set_up(), Style::Filled), Button::new(Action::Later, copy::later(), Style::Plain)],
        uninstall: Button::new(Action::Uninstall, copy::uninstall_ellipsis(), Style::Quiet),
        sheet: match w.session.confirm {
            Some(Confirm::Uninstall { going }) => Some(super::settings::uninstall_sheet(w, going)),
            _ => None,
        },
    }
}

/// The rows as the setup window draws them (N45): the permissions before the wiring. The
/// machine keeps their numbers — row five is wiring, row six the permissions.
pub const DRAWN: [u8; 6] = [1, 2, 3, 4, 6, 5];

fn plan(w: &World, at: &At, offline: bool) -> PlanView {
    let done = rows_done(w);
    let current = at.row();
    // Wiring runs out of sight while the permissions are the step (N45).
    let working = done[3] && !done[4] && !done[5];
    let rows = DRAWN
        .iter()
        .map(|&r| {
            if done[r as usize - 1] {
                PlanRow::new(Glyph::Done, copy::row_done(r, row_fact(w, r).as_deref()), &[])
            } else if Some(r) == current {
                PlanRow::new(Glyph::Current, copy::row_current(r), &[])
            } else if r == 5 && working {
                // The part underway, named, with its clock (N74).
                let line = w.sweep.wiring.map_or_else(copy::row_working, |p| copy::row_working_part(p.part));
                PlanRow { clock: w.sweep.wiring.and_then(|p| clock(p.since, w.now)), ..PlanRow::new(Glyph::Working, line, &[]) }
            } else {
                PlanRow::new(Glyph::Ahead, copy::row_ahead(r), copy::row_ahead_marks(r))
            }
        })
        .collect();
    // `current` is a row's place among the rows drawn.
    let current = current.and_then(|c| DRAWN.iter().position(|&r| r == c)).map(|i| i as u8 + 1);
    let nest = if offline {
        Nest::Offline { line: copy::offline_wizard() }
    } else if unexpected_in(w, ops_of(at)) && !matches!(at, At::Name | At::NameTaken) {
        unexpected_nest()
    } else {
        nest(w, at)
    };
    PlanView { header: copy::plan_header(), rows, current, nest }
}

/// The “something unexpected” nest (DEVIATIONS #25): the spinner rests, its amber line,
/// and [ Try again ] — the backoff retries meanwhile.
pub(crate) fn unexpected_nest() -> Nest {
    Nest::Unexpected { line: copy::unexpected_wizard(), button: Button::new(Action::TryAgain, copy::try_again(), Style::Plain) }
}

/// The token field holds a paste: one was taken and hasn't been refused (D:B104).
pub(crate) fn paste_held(w: &World) -> bool {
    w.sweep.paste.as_ref().is_some_and(Paste::held)
}

fn token_nest(w: &World, checking: bool, error: Option<String>) -> Nest {
    Nest::Token {
        line: copy::token_line(),
        create: Button::new(Action::CreateToken, copy::create_token(), Style::Filled),
        placeholder: copy::paste_token(),
        held: paste_held(w),
        checking,
        error,
        // Cloudflare makes no token before the email is confirmed, and shroom can't see
        // that without one: on the sign-up path it says so, until a paste lands (N45).
        quiet: (w.session.fields.signed_up && w.sweep.paste.is_none()).then(copy::token_email_first),
    }
}

/// The card gate (N14), and the quiet line under its button: where it goes until it's
/// clicked, then that the page is open (N45).
pub(crate) fn card_gate(w: &World) -> Nest {
    let opened = w.session.fields.billing_at;
    Nest::Gate {
        line: copy::gate_card(),
        button: Some(Button::new(Action::AddCard, copy::add_card(), Style::Filled)),
        quiet: Some(if w.session.fields.billing_opened { copy::card_opened() } else { copy::card_where() }),
        // Once the billing page is open, the watch is a part of its own with its clock, and
        // its quiet line from 45 s (N74).
        watch: opened.map(|t| Part { glyph: Glyph::Working, line: copy::card_waiting(), clock: clock(t, w.now) }),
        long: opened.filter(|t| w.now.0.saturating_sub(t.0) >= CARD_LONG_MS).map(|_| copy::card_long()),
    }
}

/// A spinning row's running clock (N74): from its first second, as a length (“0:48”).
fn clock(since: Mono, now: Mono) -> Option<String> {
    let ms = now.0.saturating_sub(since.0);
    (ms >= 1_000).then(|| copy::length(ms))
}

/// A run's parts as they stand (N74): the ones before the part underway done, it spinning
/// with its clock, the rest to come — nothing ticked that the run hasn't answered.
fn parts(count: u8, at: Option<crate::world::Progress>, now: Mono, line: fn(u8, Glyph) -> String) -> Vec<Part> {
    let current = at.map_or(1, |p| p.part);
    (1..=count)
        .map(|n| {
            let glyph = match n.cmp(&current) {
                std::cmp::Ordering::Less => Glyph::Done,
                std::cmp::Ordering::Equal => Glyph::Working,
                std::cmp::Ordering::Greater => Glyph::Ahead,
            };
            let clock = if glyph == Glyph::Working { at.and_then(|p| clock(p.since, now)) } else { None };
            Part { glyph, line: line(n, glyph), clock }
        })
        .collect()
}

/// The card step's check (N74): the token, the email gate, storage, the look around.
fn check_nest(w: &World) -> Nest {
    Nest::Check { parts: parts(4, w.sweep.check, w.now, copy::check_part) }
}

/// The last step (N74): the calm line, wiring's parts, and the name's quiet line from 3 min.
fn wiring_nest(w: &World) -> Nest {
    let at = w.sweep.wiring;
    let long = at.filter(|p| p.part == NAME_PART && w.now.0.saturating_sub(p.since.0) >= NAME_LONG_MS).map(|_| copy::wiring_long());
    Nest::Calm { line: copy::wiring_last(), parts: parts(5, at, w.now, copy::wire_part), long }
}

pub(crate) fn nest(w: &World, at: &At) -> Nest {
    let f = &w.session.fields;
    match at {
        At::Welcome => Nest::Checking,
        At::Account => Nest::Account {
            question: copy::have_account(),
            buttons: vec![
                Button::new(Action::NotYet, copy::not_yet(), Style::Plain),
                Button::new(Action::HaveOne, copy::have_one(), Style::Filled),
            ],
        },
        At::SigningUp => Nest::SigningUp {
            line: copy::signup_open(),
            buttons: vec![
                Button::new(Action::SignupAgain, copy::signup_again(), Style::Plain),
                Button::new(Action::SignedUp, copy::signed_up(), Style::Filled),
            ],
        },
        At::Token => token_nest(w, false, None),
        At::TokenChecking => token_nest(w, true, None),
        At::Card => check_nest(w),
        At::Garbled => token_nest(w, false, Some(copy::token_garbled())),
        At::WrongPowers => token_nest(w, false, Some(copy::token_wrong_powers())),
        At::PickAccount => {
            let Some(Paste::PickAccount { accounts, .. }) = &w.sweep.paste else { return Nest::Checking };
            let chosen = picked(w).unwrap_or_default();
            Nest::PickAccount {
                line: copy::pick_account(),
                accounts: accounts
                    .iter()
                    .map(|a| Choice { action: Action::PickAccount(a.id.clone()), label: a.name.clone(), selected: a.id == chosen, enabled: true, detail: None })
                    .collect(),
                choose: Button::new(Action::UseAccount, copy::use_account(), Style::Filled),
            }
        }
        At::GateEmail => Nest::Gate { line: copy::gate_email(), button: None, quiet: None, watch: None, long: None },
        At::GateCard => card_gate(w),
        At::Name | At::NameTaken => Nest::Name {
            prefix: copy::name_prefix(w.sweep.label().unwrap_or("v")),
            value: f.name.clone(),
            edits: f.name_edits,
            suffix: copy::name_suffix(),
            line: copy::keeper(),
            claim: Button::new(Action::Claim, copy::claim(), Style::Filled)
                .enabled(copy::name_ok(&f.name) && !w.inflight.ops.contains_key(&Op::Claim)),
            hint: fresh(f.name_hint, w.now).then(copy::charset),
            taken: (*at == At::NameTaken).then(copy::name_taken),
            unexpected: unexpected_in(w, &[Op::Claim]).then(copy::unexpected_wizard),
        },
        // The last step, the permissions done first (N45).
        At::Wiring => wiring_nest(w),
        At::Found(n) => Nest::Found { line: copy::found(&host(w).unwrap_or_default(), *n) },
        At::Done => Nest::Done {
            lines: vec![copy::done_ask(), copy::done_flip()],
            allow: Button::new(Action::AllowRecording, copy::allow_recording(), Style::Filled),
        },
        At::PermMic => Nest::PermMic,
        At::PermCamera => Nest::PermCamera,
        At::PermScreen => perm_screen(w),
        At::Relaunching => Nest::Relaunching,
        At::Priming => Nest::Priming { line: copy::priming() },
        At::AllSet => Nest::AllSet {
            lines: vec![copy::all_set(), copy::menu_bar()],
            buttons: {
                let mut b = vec![Button::new(Action::CloseWindow, copy::close(), Style::Plain)];
                // Members don't invite (N5): two doors; every owner-class Mac has three.
                if w.space.role() != Some(Role::Member) {
                    b.push(Button::new(Action::InviteSomeone, copy::invite_someone(), Style::Plain));
                }
                b.push(Button::new(Action::RecordNow, copy::record_now(), Style::Filled));
                b
            },
        },
        // The invite window's own faces have their own bodies (members::invite_body, plan).
        At::MemberWelcome | At::Joining | At::JoinWaiting | At::JoinExpired | At::JoinUsed | At::JoinSpaceMissing | At::AlreadyHere | At::SwitchConfirm => {
            Nest::Checking
        }
    }
}

/// The account picked in `token-pick-account`: the person's radio, else the first.
pub fn picked(w: &World) -> Option<String> {
    let Some(Paste::PickAccount { accounts, .. }) = &w.sweep.paste else { return None };
    w.session
        .fields
        .account
        .clone()
        .filter(|id| accounts.iter().any(|a| &a.id == id))
        .or_else(|| accounts.first().map(|a| a.id.clone()))
}

/// The instants this window would read differently on its own: a hint fading, the next
/// gate poll.
pub fn wakes(w: &World, out: &mut Vec<Mono>) {
    let f = &w.session.fields;
    out.extend(f.name_hint.iter().map(|t| t.plus(HINT_MS)));
    if w.session.windows.contains_key(&WindowKind::Onboarding) && matches!(at(w), At::GateEmail | At::GateCard) {
        if let Some(t) = w.sweep.gates.at() {
            out.push(t.plus(GATE_POLL_MS));
        }
    }
    // A running clock's next second (N74): the check's part, the card watch, wiring's part
    // (the last step, or the working row under the chapter).
    if w.window_visible(WindowKind::Onboarding) {
        let at = at(w);
        let since = match at {
            At::Card => w.sweep.check.map(|p| p.since),
            At::GateCard => f.billing_at,
            _ => w.sweep.wiring.map(|p| p.since),
        };
        if let Some(s) = since {
            out.push(Mono(s.0 + (w.now.0.saturating_sub(s.0) / 1_000 + 1) * 1_000));
        }
    }
}

/// perm-screen, one surface at a time (N43): while macOS's own alert is up its line points
/// there, with a quiet door to the pane for a closed alert; else shroom opened the pane.
pub(crate) fn perm_screen(w: &World) -> Nest {
    let alert = w.mac.screen_alert == Some(true);
    Nest::PermScreen {
        line: if alert { copy::perm_screen_asking() } else { copy::perm_screen() },
        quiet: copy::flipped_already(),
        reopen: Button::new(Action::ReopenShroom, copy::reopen(), Style::Quiet),
        settings: alert.then(|| crate::scene::QuietDoor {
            quiet: copy::closed_its_window(),
            button: Button::new(Action::OpenScreenSettings, copy::open_system_settings(), Style::Quiet),
        }),
    }
}
