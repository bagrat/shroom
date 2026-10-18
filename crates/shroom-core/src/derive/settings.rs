//! The Settings window (N36): General · Space · Members (owner-class). The Space tab is
//! Region 2's plan as the space's settings surface — every row re-probed on entry (law 1),
//! ticked with its fact in the inspection mood, or the first broken row current with
//! Region 2's own state beneath it in the repair mood (N42). No second repair UI: the
//! nests are the wizard's.

use crate::copy;
use crate::scene::{
    Action, Button, Choice, GeneralBody, Glyph, Nest, PlanRow, PlanView, ReportForm, SettingsBody, SettingsPane, Sheet, SpaceBody, Style,
    TextField, Toggle, Window,
};
use crate::time::Mono;
use crate::world::{Access, Confirm, CopyTarget, FailClass, Op, Paste, Queued, Role, SettingsTab, TakeDir, TokenVerdict, UpdateOffer, WindowKind, WorkerFact, World};

use super::onboarding::GATE_POLL_MS;

/// Where Settings · Space stands — derived, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceAt {
    /// Every row ticked.
    Ready,
    /// The space was reported broken and this entry's re-probe hasn't answered.
    Checking,
    /// The token died (revoked or expired): `token~repair-mood`.
    Token,
    /// A Mac without the token asked for one here: the wizard's token row.
    AddToken,
    /// A fresh paste is being verified or saved.
    TokenChecking,
    Garbled,
    WrongPowers,
    OtherAccount,
    GateEmail,
    GateCard,
    NameChanged,
    /// The page or its key is gone: wiring runs again.
    Wiring,
    StorageGone,
    /// Row six: the screen grant was revoked.
    ScreenOff,
    PermScreen,
    Relaunching,
}

impl SpaceAt {
    /// The row this sits under; `None` when every row is ticked (or the re-probe runs).
    pub fn row(self) -> Option<u8> {
        Some(match self {
            SpaceAt::Ready | SpaceAt::Checking => return None,
            SpaceAt::Token | SpaceAt::AddToken | SpaceAt::TokenChecking | SpaceAt::Garbled | SpaceAt::WrongPowers | SpaceAt::OtherAccount => 2,
            SpaceAt::GateEmail | SpaceAt::GateCard => 3,
            SpaceAt::NameChanged => 4,
            SpaceAt::Wiring | SpaceAt::StorageGone => 5,
            SpaceAt::ScreenOff | SpaceAt::PermScreen | SpaceAt::Relaunching => 6,
        })
    }

    /// The repair mood: a row of the space itself is broken (row six is per Mac). Adding
    /// a token on a Mac without one isn't a repair.
    pub fn repair(self) -> bool {
        (matches!(self.row(), Some(2..=5)) || self == SpaceAt::Checking) && self != SpaceAt::AddToken
    }

    fn network_bound(self) -> bool {
        matches!(self, SpaceAt::Checking | SpaceAt::TokenChecking | SpaceAt::GateEmail | SpaceAt::GateCard | SpaceAt::Wiring)
    }

    /// Region 2's state ids for the nest (the repair mood's machine is the wizard's).
    fn ids(self) -> &'static [&'static str] {
        match self {
            SpaceAt::Ready | SpaceAt::Checking | SpaceAt::ScreenOff => &[],
            SpaceAt::Token => &["token", "token-repair-mood"],
            SpaceAt::AddToken => &["token"],
            SpaceAt::TokenChecking => &["token"],
            SpaceAt::Garbled => &["token-garbled"],
            SpaceAt::WrongPowers => &["token-wrong-powers"],
            SpaceAt::OtherAccount => &["token-other-account"],
            SpaceAt::GateEmail => &["gate-email"],
            SpaceAt::GateCard => &["gate-card"],
            SpaceAt::NameChanged => &["name-changed"],
            SpaceAt::Wiring => &["wiring"],
            SpaceAt::StorageGone => &["space-storage-gone"],
            SpaceAt::PermScreen => &["perm-screen"],
            SpaceAt::Relaunching => &["relaunching"],
        }
    }
}

/// What the facts say broke on one row of the space (rows two to five), on a Mac that
/// holds the token. Unknown facts are not broken: the re-probe decides.
fn row_break(w: &World, row: u8) -> Option<SpaceAt> {
    let id = w.space.identity.as_ref()?;
    if !w.space.token_holder() {
        return None;
    }
    match row {
        2 => matches!(w.sweep.token.get(), Some(TokenVerdict::Revoked | TokenVerdict::WrongPowers)).then_some(SpaceAt::Token),
        3 => match w.sweep.gates.get() {
            Some(g) if g.email => Some(SpaceAt::GateEmail),
            Some(g) if g.card => Some(SpaceAt::GateCard),
            _ => None,
        },
        4 => w.sweep.subdomain.get().is_some_and(|sub| sub.as_deref() != Some(id.subdomain.as_str())).then_some(SpaceAt::NameChanged),
        5 => {
            if w.sweep.bucket.is(&false) {
                Some(SpaceAt::StorageGone)
            } else {
                matches!(w.sweep.worker.get(), Some(WorkerFact::Absent | WorkerFact::KeyGone | WorkerFact::Foreign)).then_some(SpaceAt::Wiring)
            }
        }
        _ => None,
    }
}

/// The first row of the space the facts say is broken (rows two to five).
pub fn broken_row(w: &World) -> Option<SpaceAt> {
    (2..=5).find_map(|r| row_break(w, r))
}

/// Whether a row is un-ticked: whatever broke (N42's repair mood), and row six whenever
/// this Mac's screen grant is off (live on every Mac, N36).
fn unticked(w: &World, row: u8) -> bool {
    match row {
        6 => w.mac.screen.is(&false) || w.mac.primed.is_none(),
        r => row_break(w, r).is_some(),
    }
}

/// `space-degraded`'s cause on this Mac: the space can't take videos — a publish said so,
/// or a whole re-probe found a row broken; an upload going through, or a whole re-probe
/// finding every row sound, clears it (law 1: the face clears when its cause does). Only a
/// token-holding Mac reads it (members and owner-class Macs without the token read Region
/// 6's lines).
pub fn degraded(w: &World) -> bool {
    w.space.token_holder() && w.space.broken.is(&true)
}

/// Takes that can't drain — the space refused them, or it can't take videos at all: they
/// never block an exit (N36), and leaving deletes them from this Mac.
pub fn stranded(w: &World) -> Vec<&Queued> {
    w.queue.iter().filter(|q| stranded_take(w, q)).collect()
}

/// The one predicate for “this take can't reach the space” — the Library's “Waiting for
/// your space” row, the exit sheets' count and the banners all read it (whole-spec audit
/// b4 #8): the space refused it, the space can't take videos at all (`space-degraded`'s
/// cause), or this Mac may no longer send it (N32).
pub fn stranded_take(w: &World, q: &Queued) -> bool {
    q.stranded() || w.space.broken.is(&true) || !super::members::can_send(w, q)
}

/// A stranded take's copy to Movies is under way: the exits wait for it (law 2, B54).
pub fn saving_stranded(w: &World) -> bool {
    stranded(w).iter().any(|q| w.members.saving.contains(&q.dir))
}

/// Uploads still draining: they block the exit until they're done (N31's rule).
pub fn draining(w: &World) -> usize {
    w.queue.len() - stranded(w).len()
}

/// This entry's re-probe hasn't answered (law 1).
fn probing(w: &World) -> bool {
    let entered = w.session.windows.get(&WindowKind::Settings).map(|d| d.entered).unwrap_or(Mono(0));
    w.sweep.pass.is_none_or(|t| t < entered)
}

pub fn space_at(w: &World) -> SpaceAt {
    // “Add a token here” on a Mac without the token: the wizard's token row (N33, §6.2).
    if !w.space.token && w.session.fields.settings.add_token {
        return match &w.sweep.paste {
            Some(Paste::Garbled) => SpaceAt::Garbled,
            Some(Paste::WrongPowers) => SpaceAt::WrongPowers,
            Some(Paste::OtherAccount) => SpaceAt::OtherAccount,
            Some(Paste::Checking(_) | Paste::Saving { .. } | Paste::PickAccount { .. }) => SpaceAt::TokenChecking,
            None => SpaceAt::AddToken,
        };
    }
    if let Some(b) = broken_row(w) {
        // Set it up fresh was clicked: wiring runs (or waits its retry) — the calm line,
        // never the storage-gone question again under a live button (#10).
        if b == SpaceAt::StorageGone && w.session.fields.settings.fresh {
            return SpaceAt::Wiring;
        }
        if b == SpaceAt::Token {
            return match &w.sweep.paste {
                Some(Paste::Garbled) => SpaceAt::Garbled,
                Some(Paste::WrongPowers) => SpaceAt::WrongPowers,
                Some(Paste::OtherAccount) => SpaceAt::OtherAccount,
                Some(Paste::Checking(_) | Paste::Saving { .. } | Paste::PickAccount { .. }) => SpaceAt::TokenChecking,
                None => SpaceAt::Token,
            };
        }
        return b;
    }
    if w.space.token_holder() && w.space.broken.is(&true) && probing(w) {
        return SpaceAt::Checking;
    }
    if w.mac.screen.is(&false) || w.mac.primed.is_none() {
        return match (w.session.chapter, w.launch.ending) {
            (true, true) => SpaceAt::Relaunching,
            (true, false) => SpaceAt::PermScreen,
            _ => SpaceAt::ScreenOff,
        };
    }
    SpaceAt::Ready
}

/// The tab showing: the last one used, General the first time (N36). Members only on an
/// owner-class Mac (law 14).
pub fn tab(w: &World) -> SettingsTab {
    match w.prefs.settings_tab {
        Some(SettingsTab::Members) if w.space.owner_class() => SettingsTab::Members,
        Some(SettingsTab::Members) | None => SettingsTab::General,
        Some(t) => t,
    }
}

/// What a Mac without the token is in its Space tab (Region 6): owner-class or a member.
fn role_face(w: &World) -> Option<&'static str> {
    match w.space.role()? {
        Role::Member => Some("settings-space-member"),
        Role::OwnerClass => Some("settings-space-no-token"),
        Role::TokenHolder if !w.space.token => Some("settings-space-no-token"),
        Role::TokenHolder => None,
    }
}

/// The wizard's “something unexpected” face in the Space tab (DEVIATIONS #25): the call
/// this row waits on failed in a way nothing else classifies.
fn unexpected(w: &World, at: SpaceAt) -> bool {
    let ops: &[Op] = match at {
        SpaceAt::TokenChecking => &[Op::Validate, Op::SaveToken],
        SpaceAt::Checking | SpaceAt::GateEmail | SpaceAt::GateCard => &[Op::Sweep],
        SpaceAt::Wiring => &[Op::Wire],
        _ => &[],
    };
    super::onboarding::unexpected_in(w, ops)
}

/// `wizard-offline` in the Space tab: a network-bound row while offline, or after a
/// transport failure no wizard call has answered since.
fn offline(w: &World, at: SpaceAt) -> bool {
    at.network_bound()
        && (w.online() == Some(false)
            || Op::WIZARD.iter().any(|op| w.inflight.failures.get(op).is_some_and(|f| f.class == FailClass::Transport)))
}

pub fn derive(w: &World) -> Option<Window<SettingsBody>> {
    let door = w.session.windows.get(&WindowKind::Settings)?;
    if !w.window_visible(WindowKind::Settings) || !w.provisioned() {
        return None;
    }
    let tab = tab(w);
    let mut tabs_shown = vec![(SettingsTab::General, copy::tab_general()), (SettingsTab::Space, copy::tab_space())];
    // The third tab, owner-class Macs only (N36, law 14).
    if w.space.owner_class() {
        tabs_shown.push((SettingsTab::Members, copy::tab_members()));
    }
    let tabs = tabs_shown.into_iter().map(|(t, label)| Choice { action: Action::SettingsTab(t), label, selected: t == tab, enabled: true, detail: None }).collect();
    let mut faces: Vec<&'static str> = Vec::new();
    let mut ret = None;
    let pane = match tab {
        SettingsTab::General => {
            faces.push("settings-general");
            SettingsPane::General(general(w))
        }
        SettingsTab::Members => {
            faces.extend(super::members::faces(w));
            // The composer's field owns Return (law 8): Return ≡ Invite.
            ret = Some(Action::Invite);
            SettingsPane::Members(super::members::pane(w))
        }
        SettingsTab::Space => {
            faces.push("settings-space");
            let at = space_at(w);
            let off = offline(w, at);
            if let Some(face) = role_face(w) {
                faces.push(face);
            } else if at.repair() {
                faces.extend(["settings-space-repair", "repair-mood"]);
            } else {
                faces.extend(["settings-space-inspection", "re-entry-as-settings"]);
            }
            if off {
                faces.push("wizard-offline");
            } else if unexpected(w, at) {
                faces.push("wizard-unexpected");
            } else {
                faces.extend(at.ids());
                ret = space_ret(at);
            }
            SettingsPane::Space(space(w, at, off))
        }
    };
    let (sheet, report) = match report_sheet(w) {
        Some((sheet, form)) => (Some(sheet), Some(form)),
        None => (sheet(w), None),
    };
    // Esc in the name invites carry, while it's being changed, puts it back (B158). A
    // sheet's Esc is Back — a report on its way drops it, as an uninstall under way does.
    let changing = tab == SettingsTab::Members && w.session.fields.settings.from.is_some();
    let (ret, esc) = if sheet.is_some() { (None, Some(Action::Back)) } else { (ret, changing.then_some(Action::KeepFrom)) };
    Some(Window { id: "settings", faces, body: SettingsBody { tabs, pane, sheet, report }, ret, esc, activates: door.asked })
}

/// Return in the Space tab: the nest's own default, as the wizard's state gives it.
fn space_ret(at: SpaceAt) -> Option<Action> {
    match at {
        SpaceAt::Token | SpaceAt::AddToken | SpaceAt::TokenChecking | SpaceAt::Garbled | SpaceAt::WrongPowers | SpaceAt::OtherAccount => {
            Some(Action::CreateToken)
        }
        SpaceAt::GateCard => Some(Action::AddCard),
        SpaceAt::StorageGone => Some(Action::SetUpFresh),
        _ => None,
    }
}

/// A button that would end, move or replace the app greys mid-take, and says why (law 4).
fn not_mid_take(w: &World, b: Button) -> Button {
    if w.take_open() {
        Button { enabled: false, note: Some(copy::after_take()), ..b }
    } else {
        b
    }
}

/// The version an offer names, ready to apply: downloaded and verified (N37).
pub fn ready_version(w: &World) -> Option<&str> {
    match w.update.offer.get() {
        Some(Some(UpdateOffer::Ready { version })) if w.update.staged.as_deref() == Some(version.as_str()) => Some(version),
        _ => None,
    }
}

fn general(w: &World) -> GeneralBody {
    let on = w.mac.login_item.is(&true);
    let running = w.launch.version.as_str();
    let (update_line, update, whats_new) = match (ready_version(w), w.update.offer.get()) {
        (Some(_), _) if w.update.applying => (copy::updating(), None, None),
        (Some(v), _) => (
            copy::update_ready(v),
            Some(not_mid_take(w, Button::new(Action::Update, copy::update(), Style::Filled))),
            Some(Button::new(Action::WhatsNew, copy::whats_new(), Style::Link)),
        ),
        (None, Some(Some(UpdateOffer::NeedsMacos { version, macos }))) => (copy::needs_macos(version, *macos, running), None, None),
        // “Up to date” only when a check said there's nothing newer (whole-spec audit b4 #7).
        (None, Some(None)) if !w.update.failed => (copy::up_to_date(running), None, None),
        // An update that didn't finish says so until a check answers again (N37's line).
        _ if w.update.failed => (copy::update_failed(), None, None),
        _ => (copy::running_version(running), None, None),
    };
    GeneralBody {
        login: Toggle { action: Action::SetLoginItem(!on), label: copy::login_box(), on, enabled: true },
        updates_label: copy::updates(),
        update_line,
        update,
        whats_new,
        // Reporting asks nothing of a take: open mid-take too (B180).
        help_label: copy::help_label(),
        copy_diagnostics: Button::new(
            Action::CopyDiagnostics,
            if matches!(w.session.copied, Some((CopyTarget::Diagnostics, _))) { copy::copied() } else { copy::copy_diagnostics() },
            Style::Plain,
        ),
        report: Button::new(Action::ReportProblem, copy::report_a_problem(), Style::Plain),
        sent: w.session.fields.settings.sent.then(copy::report_sent),
        help: copy::help_line(),
        help_links: vec![Button::new(Action::ReportIssue, copy::github(), Style::Link)],
        uninstall: not_mid_take(w, Button::new(Action::Uninstall, copy::uninstall_ellipsis(), Style::Quiet)),
    }
}

/// Row one's fact: the account's name — as the re-probe proved it, else as the space
/// remembers it (N42).
fn account_name(w: &World) -> Option<String> {
    match w.sweep.token.get() {
        Some(TokenVerdict::Valid { account, .. }) => Some(account.name.clone()),
        _ => w.space.identity.as_ref().and_then(|i| i.account_name.clone()),
    }
}

fn mic_denied(w: &World) -> bool {
    matches!(w.mac.mic.get(), Some(Access::Denied | Access::Restricted))
}

/// The Space tab on a Mac without the token (Region 6): the space's row and row six, and
/// where the space is looked after.
fn space_elsewhere(w: &World, at: SpaceAt, offline: bool) -> PlanView {
    let id = w.space.identity.as_ref().expect("Settings shows only with a space");
    let name = (id.role == Role::Member).then_some(id.owner_name.as_str());
    let six = if matches!(at.row(), Some(6)) {
        PlanRow::new(Glyph::Current, copy::row_six_off(), &[])
    } else if unticked(w, 6) {
        PlanRow::new(Glyph::Ahead, copy::row_ahead(6), copy::row_ahead_marks(6))
    } else {
        let fact = if mic_denied(w) { copy::mic_off_fact() } else { copy::on_this_mac() };
        PlanRow::new(Glyph::Done, copy::space_done(6, Some(&fact)), &[])
    };
    // The space no longer takes this Mac's videos (N34): its row loses its tick, and the
    // popover's own line stands in for the one below (DECISIONS B157).
    let cut_off = cut_off_line(w);
    let space = match cut_off {
        Some(_) => PlanRow::new(Glyph::Ahead, copy::space_row_off(name, &id.host), &[]),
        None => PlanRow::new(Glyph::Done, copy::space_row_done(name, &id.host), &[]),
    };
    let rows = vec![space, six];
    let nest = if offline {
        Nest::Offline { line: copy::offline_wizard() }
    } else if let (Some(line), SpaceAt::Ready) = (cut_off, at) {
        Nest::NoToken { line, button: None }
    } else if at == SpaceAt::Ready {
        match name {
            Some(n) => Nest::NoToken { line: copy::member_space_line(n), button: None },
            None => Nest::NoToken {
                line: copy::repairs_elsewhere(),
                button: Some(Button::new(Action::AddToken, copy::add_token_here(), Style::Plain)),
            },
        }
    } else if at == SpaceAt::AddToken {
        token_nest(w, copy::token_line(), false, None)
    } else {
        nest(w, at)
    };
    // Row six is this two-row plan's second (a row's place among the rows shown).
    let current = matches!(at.row(), Some(6)).then_some(2);
    PlanView { header: copy::space_header(name), rows, current, nest }
}

/// The member-life line when the space can't take this Mac's videos at all (N34): removed,
/// or the space gone — the banner's own words. `None` while it can.
fn cut_off_line(w: &World) -> Option<String> {
    use super::members::{Life, life};
    let name = w.space.identity.as_ref().map(|i| i.owner_name.as_str()).unwrap_or_default();
    match (w.space.role()?, life(w)?) {
        (Role::Member, Life::Revoked) => Some(copy::member_revoked(name)),
        (Role::Member, Life::Gone) => Some(copy::space_gone(name)),
        (Role::OwnerClass, Life::Revoked) => Some(copy::owner_mac_removed()),
        _ => None,
    }
}

fn space(w: &World, at: SpaceAt, offline: bool) -> SpaceBody {
    let id = w.space.identity.as_ref().expect("Settings shows only with a space");
    // The mark is the owner's (N40), once the space said where it stands (U3).
    let mark_known = w.members.standing.mark.filter(|_| w.space.owner_class());
    let mark = mark_known.map(|on| Toggle { action: Action::SetMark(!on), label: copy::mark_label(), on, enabled: !w.inflight.ops.contains_key(&Op::Mark) });
    let mark_line = mark_known.map(|_| copy::mark_line());
    // Naming (N66): only the Mac that holds the setup key can re-wire the page, once its
    // page said where it stands; greyed while a re-wiring runs and mid-take (law 4).
    let ai_known = w.space.ai.filter(|_| w.space.token_holder());
    let ai = ai_known.map(|on| Toggle { action: Action::SetAi(!on), label: copy::ai_label(), on, enabled: !w.inflight.ops.contains_key(&Op::Ai) && !w.take_open() });
    let ai_line = ai_known.map(|_| if super::ai_fell_through(w, crate::world::WindowKind::Settings) { copy::ai_failed() } else { copy::ai_cost() });
    let exit = if id.role == Role::Member { copy::leave_space_ellipsis() } else { copy::disconnect_ellipsis() };
    let exits = vec![not_mid_take(w, Button::new(Action::Disconnect, exit, Style::Quiet))];
    let mic = mic_denied(w).then(|| Button::new(Action::OpenMicSettings, copy::turn_on_mic(), Style::Quiet));
    if role_face(w).is_some() {
        return SpaceBody { plan: space_elsewhere(w, at, offline), mic, mark, mark_line, ai, ai_line, exits };
    }
    let current = at.row();
    let rows = (1..=6u8)
        .map(|r| {
            if Some(r) == current {
                if r == 6 {
                    PlanRow::new(Glyph::Current, copy::row_six_off(), &[])
                } else {
                    PlanRow::new(Glyph::Current, copy::row_current(r), &[])
                }
            } else if unticked(w, r) {
                // Broken too, behind the first broken row: un-ticked, as the wizard draws
                // a row still to do (N42).
                PlanRow::new(Glyph::Ahead, copy::row_ahead(r), copy::row_ahead_marks(r))
            } else {
                let fact = match r {
                    1 => account_name(w).map(|n| copy::account_fact(&n)),
                    2 => Some(copy::on_this_mac()),
                    4 => Some(copy::name_fact(&id.host, id.foreign_v)),
                    6 if mic_denied(w) => Some(copy::mic_off_fact()),
                    6 => Some(copy::on_this_mac()),
                    _ => None,
                };
                PlanRow::new(Glyph::Done, copy::space_done(r, fact.as_deref()), &[])
            }
        })
        .collect();
    let nest = if offline {
        Nest::Offline { line: copy::offline_wizard() }
    } else if unexpected(w, at) {
        super::onboarding::unexpected_nest()
    } else {
        nest(w, at)
    };
    SpaceBody { plan: PlanView { header: copy::plan_header(), rows, current, nest }, mic, mark, mark_line, ai, ai_line, exits }
}

fn token_nest(w: &World, line: String, checking: bool, error: Option<String>) -> Nest {
    Nest::Token {
        line,
        create: Button::new(Action::CreateToken, copy::create_token(), Style::Filled),
        placeholder: copy::paste_token(),
        held: super::onboarding::paste_held(w),
        checking,
        error,
        quiet: None,
    }
}

fn nest(w: &World, at: SpaceAt) -> Nest {
    let host = w.space.identity.as_ref().map(|i| i.host.clone()).unwrap_or_default();
    // A Mac adding a token reads the wizard's own line; the repair mood says what broke.
    let token_line = || if w.space.token { copy::token_repair() } else { copy::token_line() };
    match at {
        SpaceAt::Ready => Nest::Ready { line: copy::nothing_to_redo() },
        SpaceAt::Checking => Nest::Checking,
        SpaceAt::Token => token_nest(w, copy::token_repair(), false, None),
        SpaceAt::AddToken => token_nest(w, copy::token_line(), false, None),
        SpaceAt::TokenChecking => token_nest(w, token_line(), true, None),
        SpaceAt::Garbled => token_nest(w, token_line(), false, Some(copy::token_garbled())),
        SpaceAt::WrongPowers => token_nest(w, token_line(), false, Some(copy::token_wrong_powers())),
        SpaceAt::OtherAccount => token_nest(w, token_line(), false, Some(copy::token_other_account(&host))),
        SpaceAt::GateEmail => Nest::Gate { line: copy::gate_email(), button: None, quiet: None },
        SpaceAt::GateCard => super::onboarding::card_gate(w),
        SpaceAt::NameChanged => Nest::NameChanged { line: copy::name_changed() },
        SpaceAt::Wiring => Nest::Calm { line: copy::wiring() },
        SpaceAt::StorageGone => Nest::StorageGone {
            line: copy::storage_gone(),
            buttons: vec![
                Button::new(Action::SetUpFresh, copy::set_it_up_fresh(), Style::Filled),
                not_mid_take(w, Button::new(Action::Disconnect, copy::disconnect_ellipsis(), Style::Quiet)),
            ],
        },
        // Law 11: no prompt mid-take — the button greys.
        SpaceAt::ScreenOff => Nest::ScreenOff { button: not_mid_take(w, Button::new(Action::TurnBackOn, copy::turn_back_on(), Style::Filled)) },
        SpaceAt::PermScreen => super::onboarding::perm_screen(w),
        SpaceAt::Relaunching => Nest::Relaunching,
    }
}

/// The Report a problem sheet and its form (B180): Send waits for words and for the report
/// the shell gathers; while it goes, Send reads Sending… and both grey; a send that didn't
/// go through keeps the words and offers the mail address.
fn report_sheet(w: &World) -> Option<(Sheet, ReportForm)> {
    let Some(Confirm::Report { sending }) = w.session.confirm else { return None };
    if tab(w) != SettingsTab::General {
        return None;
    }
    let f = &w.session.fields.settings.report;
    let ready = f.gathered.is_some() && !f.words.trim().is_empty();
    let send = if sending {
        Button::new(Action::SendReport, copy::sending(), Style::Filled).enabled(false)
    } else {
        Button::new(Action::SendReport, copy::send(), Style::Filled).enabled(ready)
    };
    let sheet = Sheet {
        id: "report-problem",
        faces: vec![],
        title: copy::report_title(),
        lines: vec![copy::report_line()],
        blocked: None,
        buttons: vec![Button::new(Action::Back, copy::cancel(), Style::Plain).enabled(!sending), send],
    };
    let failed = f.failed && !sending;
    let form = ReportForm {
        words: TextField { placeholder: copy::report_placeholder(), value: f.words.clone(), enabled: !sending, edits: f.words_edits },
        reply_label: copy::report_reply_label(),
        reply_to: TextField { placeholder: String::new(), value: f.reply_to.clone(), enabled: !sending, edits: f.reply_edits },
        disclosure: copy::report_disclosure(),
        preview: f.gathered.as_ref().map(|d| crate::report::preview(crate::report::Kind::Problem, &f.words, Some(&f.reply_to), d)),
        preparing: f.gathered.is_none().then(copy::report_preparing),
        failed: failed.then(copy::report_failed),
        mail: failed.then(|| Button::new(Action::MailReport, copy::report_mail(), Style::Link)),
    };
    Some((sheet, form))
}

/// The Disconnect sheet (owner, holds the token) under N23's law: no Return default, Esc ≡
/// Cancel, Disconnect red-outlined and rightmost — greyed while uploads drain or a
/// recovered take waits (N31's rule), and mid-take (law 4).
fn sheet(w: &World) -> Option<Sheet> {
    if let (Some(Confirm::Remove(r)), SettingsTab::Members) = (&w.session.confirm, tab(w)) {
        return super::members::remove_sheet(w, r);
    }
    if let (Some(Confirm::Uninstall { going }), SettingsTab::General) = (&w.session.confirm, tab(w)) {
        return Some(uninstall_sheet(w, *going));
    }
    if w.session.confirm != Some(Confirm::Exit) || tab(w) != SettingsTab::Space {
        return None;
    }
    let role = w.space.role();
    let member = role == Some(Role::Member);
    let token = w.space.token_holder();
    let draining = draining(w);
    let blocked = if draining > 0 {
        Some(if member { copy::leave_uploading(draining as u64) } else { copy::disconnect_uploading(draining as u64) })
    } else if !w.saved.is_empty() {
        Some(copy::disconnect_saved_waits())
    } else {
        None
    };
    let name = w.space.identity.as_ref().map(|i| i.owner_name.clone()).unwrap_or_default();
    let (id, title, mut lines, answer) = if member {
        ("leave-confirm", copy::leave_title(&name), vec![copy::leave_line(&name)], copy::leave())
    } else if token {
        ("disconnect-confirm", copy::disconnect_title(), vec![copy::disconnect_line()], copy::disconnect())
    } else {
        ("disconnect-confirm-no-token", copy::disconnect_title(), vec![copy::disconnect_owner_class_line()], copy::disconnect())
    };
    if token && only_token_holder(w) {
        lines.push(copy::disconnect_only_holder());
    }
    let mut buttons = vec![Button::new(Action::Back, copy::cancel(), Style::Plain)];
    // Stranded takes don't block (they can't drain): one line, and Movies first (N36).
    let takes = stranded(w);
    let n = takes.len();
    if n > 0 {
        lines.push(copy::stranded_line(n as u64));
        let dirs: Vec<&TakeDir> = takes.iter().map(|q| &q.dir).collect();
        buttons.push(super::members::save_button(w, Action::SaveStrandedToMovies, &dirs, copy::save_them_to_movies(n as u64), Style::Plain));
    }
    // A copy to Movies under way holds the exit until it lands (law 2, B54).
    let enabled = blocked.is_none() && !w.take_open() && !saving_stranded(w);
    buttons.push(not_mid_take(w, Button::new(Action::ConfirmDisconnect, answer, Style::RedOutline)).enabled(enabled));
    Some(Sheet { id, faces: vec![], title, lines, blocked, buttons })
}

/// The Uninstall sheet (B135), under N23's law and Disconnect's guards: greyed while
/// uploads drain or a recovered take waits, stranded takes named with Movies first. Once
/// answered, Uninstalling… — both buttons grey until shroom has quit.
pub fn uninstall_sheet(w: &World, going: bool) -> Sheet {
    let member = w.space.identity.as_ref().filter(|i| i.role == Role::Member);
    let (stays, back) = match member {
        Some(i) => (copy::uninstall_owners_space_stays(&i.owner_name), copy::uninstall_back_member()),
        None => (copy::uninstall_space_stays(), copy::uninstall_back_owner()),
    };
    // A Mac with no space (asked over the welcome, B160): only the line that's true of it.
    let mut lines = if w.provisioned() { vec![stays, copy::uninstall_lets_go(), back] } else { vec![copy::uninstall_lets_go()] };
    let draining = draining(w);
    let blocked = if draining > 0 {
        Some(copy::uninstall_uploading(draining as u64))
    } else if !w.saved.is_empty() {
        Some(copy::disconnect_saved_waits())
    } else {
        None
    };
    if going {
        let buttons = vec![
            Button::new(Action::Back, copy::cancel(), Style::Plain).enabled(false),
            Button::new(Action::ConfirmUninstall, copy::uninstalling(), Style::RedOutline).enabled(false),
        ];
        return Sheet { id: "uninstall-confirm", faces: vec![], title: copy::uninstall_title(), lines, blocked: None, buttons };
    }
    let mut buttons = vec![Button::new(Action::Back, copy::cancel(), Style::Plain)];
    let takes = stranded(w);
    let n = takes.len();
    if n > 0 {
        lines.push(copy::uninstall_stranded(n as u64));
        let dirs: Vec<&TakeDir> = takes.iter().map(|q| &q.dir).collect();
        buttons.push(super::members::save_button(w, Action::SaveStrandedToMovies, &dirs, copy::save_them_to_movies(n as u64), Style::Plain));
    }
    let enabled = blocked.is_none() && !w.take_open() && !saving_stranded(w);
    buttons.push(not_mid_take(w, Button::new(Action::ConfirmUninstall, copy::uninstall(), Style::RedOutline)).enabled(enabled));
    Sheet { id: "uninstall-confirm", faces: vec![], title: copy::uninstall_title(), lines, blocked, buttons }
}

/// Whether this Mac is its space's only token-holder: as `/me` said (U9) — and until it
/// says, the line is said (it's true for any token Mac whose space exists, §6.2).
fn only_token_holder(w: &World) -> bool {
    w.space.token_holder() && w.members.standing.only_holder != Some(false)
}

/// The instants Settings would read differently on its own: the gate's next poll.
pub fn wakes(w: &World, out: &mut Vec<Mono>) {
    if w.window_visible(WindowKind::Settings) && tab(w) == SettingsTab::Space && matches!(space_at(w), SpaceAt::GateEmail | SpaceAt::GateCard) {
        if let Some(t) = w.sweep.gates.at() {
            out.push(t.plus(GATE_POLL_MS));
        }
    }
}
