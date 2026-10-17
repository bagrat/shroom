//! The standing laws as checks over one step: (prev World, prev Scene, event, next World,
//! next Scene, effects). Run at every step of every walkthrough and table test.
#![allow(dead_code)]

/// The independent entry oracle (DECISIONS B89), held at every step beside the laws.
#[path = "entry.rs"]
pub mod entry;

use shroom_core::effect::{Effect, MemberCmd, PersistOp, RecCmd, ShellCmd, TakeCmd, UpdCmd};
use shroom_core::event::{Event, Field, JoinAnswer, LibraryNews, MemberNews, OsNews, RecNews, Surface};
use shroom_core::scene::{Action, Button, Face, PopoverBody, Scene, Sheet, Style};
use shroom_core::world::{Op, Phase, Role, WindowKind, World};

/// What a pasted Cloudflare token starts with — law 13's canary: it never reaches a Scene
/// or an Effect (the core holds only `SecretId` handles).
pub const SECRET_CANARY: &str = "cfat_";

/// Law 6 over one World and one Scene; `None` when it holds: the Dock icon iff a window of
/// shroom's is open — the setup window, the Library or Settings (SM:133, as ratified; the
/// narrowing of DEVIATIONS #38 rejected by Bagrat on 2026-10-13). The icon is held to the
/// World first — which windows show, as the entry oracle reads them from the World's own
/// doors, never through `derive` or a helper of the core's — and only then to the Scene's
/// own windows: a Scene that drops a window and its icon together agrees with itself, and
/// only the World can say it is wrong. On its own so that a test can hand it a Scene
/// `derive` never made — `check` turns every such Scene away at law 1.
pub fn law_6(nw: &World, ns: &Scene) -> Option<String> {
    let open = [WindowKind::Onboarding, WindowKind::Library, WindowKind::Settings, WindowKind::Feedback].into_iter().any(|k| entry::shows(nw, k));
    if ns.dock != open {
        let dock = if ns.dock { "a Dock icon" } else { "no Dock icon" };
        let window = if open { "a window of shroom's is open" } else { "no window of shroom's is open" };
        return Some(format!("law 6: {dock}, and in the World {window}"));
    }
    if ns.dock != (ns.onboarding.is_some() || ns.library.is_some() || ns.settings.is_some() || ns.feedback.is_some()) {
        return Some("law 6: the Dock and the windows disagree".into());
    }
    for (shown, kind) in [
        (ns.onboarding.is_some(), WindowKind::Onboarding),
        (ns.library.is_some(), WindowKind::Library),
        (ns.settings.is_some(), WindowKind::Settings),
        (ns.feedback.is_some(), WindowKind::Feedback),
    ] {
        if shown && !nw.session.windows.contains_key(&kind) {
            return Some("law 6: a window without a door".into());
        }
    }
    None
}

pub fn check(pw: &World, ps: &Scene, ev: &Event, nw: &World, ns: &Scene, fx: &[Effect]) {
    let ctx = || format!("event {ev:?}\n  before {:?}\n  after  {:?}", ps.state_ids(), ns.state_ids());

    // Law 1 — state is derived: derive is a pure function of the World, and what `update`
    // settled is all that was due (settling again starts nothing).
    assert_eq!(&shroom_core::derive(nw), ns, "law 1: derive isn't deterministic — {}", ctx());
    let again = shroom_core::settle::settle(nw);
    assert!(again.is_empty(), "law 1: settle isn't idempotent, still due {again:?} — {}", ctx());

    // Law 2 — the recording is sacred: nothing deletes a take but a person's Discard
    // (or Restart, once the new capture began).
    for e in fx {
        let deletes = matches!(e, Effect::Takes(TakeCmd::Delete(_)) | Effect::Recorder(RecCmd::Discard(_)));
        if deletes {
            let by_person = matches!(ev, Event::Act { action: Action::ConfirmDiscard | Action::ConfirmDiscardSaved | Action::ConfirmDiscardRow, .. });
            let by_restart = matches!(ev, Event::Rec(shroom_core::event::RecNews::Started { .. })) && pw.session.arming.as_ref().is_some_and(|a| a.restart_of.is_some());
            // The Library's Delete: the person confirmed it, the tombstone landed first, and
            // only then does this Mac's copy of a video still uploading go (N35).
            let by_delete = match (ev, e) {
                (Event::Library(LibraryNews::Deleted(recs)), Effect::Takes(TakeCmd::Delete(dir))) => {
                    pw.queue.iter().any(|q| &q.dir == dir && recs.contains(&q.rec) && pw.library_ops.deleting.contains(&q.rec))
                }
                _ => false,
            };
            // Disconnect: only the takes that can't reach the space, and only from a sheet
            // that said so and offered Movies first (N36).
            let by_leaving = match (ev, e) {
                (Event::Act { action: Action::ConfirmDisconnect, .. }, Effect::Takes(TakeCmd::Delete(dir))) => {
                    let said = ps.settings.as_ref().and_then(|s| s.body.sheet.as_ref()).is_some_and(|sh| {
                        sh.lines.iter().any(|l| l.contains("never reached the space")) && sh.buttons.iter().any(|b| b.action == Action::SaveStrandedToMovies)
                    });
                    // …and never a take whose copy to Movies is still under way (B54).
                    said && shroom_core::derive::settings::stranded(pw).iter().any(|q| &q.dir == dir) && !pw.members.saving.contains(dir)
                }
                _ => false,
            };
            // A switch is Leave's twin (N31, N36): at join's 200 for another space, the takes
            // that space can't take go — the switch face said so and offered Movies first —
            // and a take that raced in is copied to Movies in the same breath (DECISIONS B41).
            // The switch's face said so — its stranded line, and Movies first — and the take's
            // copy isn't still under way (DECISIONS B54).
            let by_switch = match (ev, e) {
                (Event::Member(MemberNews::Joined { answer: JoinAnswer::Joined(id), .. }), Effect::Takes(TakeCmd::Delete(dir))) => {
                    let other = pw.space.identity.as_ref().is_some_and(|old| old.host != id.host);
                    let stranded = shroom_core::derive::settings::stranded(pw).iter().any(|q| &q.dir == dir);
                    other && stranded && pw.members.left_behind.contains(dir) && !pw.members.saving.contains(dir)
                }
                _ => false,
            };
            // A switch's take waiting on its copy (one that raced in, or still saving): it
            // goes only once the copy landed (B54).
            let by_saved = match (ev, e) {
                (Event::Rec(shroom_core::event::RecNews::SavedToMovies(d)), Effect::Takes(TakeCmd::Delete(dir))) => d == dir && pw.members.toss.contains_key(dir),
                _ => false,
            };
            assert!(by_person || by_restart || by_delete || by_leaving || by_switch || by_saved, "law 2: a take deleted without a Discard — {}", ctx());
        }
    }

    // Law 2's consent for a switch: Join names takes it may leave behind only from a face
    // that said so (“…never reached … — joining deletes them”) and offered Movies first.
    if matches!(ev, Event::Act { action: Action::Join, .. }) && !nw.members.left_behind.is_empty() {
        let face = serde_json::to_string(&ps.onboarding).unwrap_or_default();
        assert!(face.contains("never reached") && ps.offers(shroom_core::event::Surface::Onboarding, &Action::SaveStrandedToMovies), "law 2: a switch consented for takes its face never named — {}", ctx());
    }

    // Law 3 — only Stop or Get link make a link.
    for e in fx {
        if matches!(e, Effect::Publish { .. }) {
            assert!(
                matches!(pw.take.as_ref().map(|t| t.phase), Some(Phase::Finalizing(shroom_core::world::Why::Stop | shroom_core::world::Why::GetLink))),
                "law 3: published without Stop or Get link — {}",
                ctx()
            );
        }
    }

    // Law 4 — the quiet-take law: with a take open before and after, nothing new appears
    // except the take's own popover (paused, its auto-pause faces, the Discard question).
    if pw.take_open() && nw.take_open() {
        if let Some(p) = &ns.popover {
            assert!(["paused", "discard-confirm", "restart-confirm"].contains(&p.id), "law 4: {} appeared mid-take — {}", p.id, ctx());
        }
        for e in fx {
            assert!(!matches!(e, Effect::Shell(ShellCmd::Quit)), "law 4: quit mid-take — {}", ctx());
            assert!(!matches!(e, Effect::Cloud(shroom_core::effect::CloudCmd::OpenOwnerPage(_))), "law 4: a page opened mid-take — {}", ctx());
        }
        if matches!(ev, Event::QuitRequested(_)) && matches!(pw.take.as_ref().map(|t| t.phase), Some(Phase::Recording)) {
            assert!(fx.is_empty(), "law 4: quit while recording must do nothing — {}", ctx());
        }
        assert!(ns.onboarding.is_none() || ps.onboarding.is_some(), "law 4: a window appeared mid-take — {}", ctx());
        assert!(ns.library.is_none() || ps.library.is_some(), "law 4: the Library appeared mid-take — {}", ctx());
        assert!(ns.settings.is_none() || ps.settings.is_some(), "law 4: Settings appeared mid-take — {}", ctx());
        for e in fx {
            // Nothing that would end, move or replace the app runs mid-take.
            assert!(!matches!(e, Effect::Updater(UpdCmd::Apply(_))), "law 4: an update applied mid-take — {}", ctx());
            assert!(!matches!(e, Effect::Persist(PersistOp::Wipe)), "law 4: disconnected mid-take — {}", ctx());
        }
        // Invites wait out the take (r6 note 12): joining another space never runs mid-take.
        for e in fx {
            assert!(!matches!(e, Effect::Member(MemberCmd::Join(_))), "law 4: a join mid-take — {}", ctx());
        }
        let banner_new = |id: &str| ns.popover.as_ref().and_then(|p| p.banner.as_ref()).is_some_and(|b| b.id == id);
        assert!(!banner_new("update-ready"), "law 4: an update banner mid-take — {}", ctx());
    }
    // An update applying ends the app any moment now: no take is open under it.
    assert!(!(nw.take_open() && nw.update.applying), "law 4: a take open while an update applies — {}", ctx());
    // Greyed for the take: a control that would end, move or replace the app says why.
    if nw.take_open() {
        let window_buttons: Vec<&Button> = ns.settings.iter().flat_map(|s| s.buttons()).collect();
        for b in window_buttons {
            if matches!(b.action, Action::Update | Action::Disconnect | Action::ConfirmDisconnect) {
                assert!(!b.enabled, "law 4: {:?} live mid-take — {}", b.action, ctx());
            }
            if matches!(b.action, Action::Update | Action::Disconnect) {
                assert_eq!(b.note.as_deref(), Some("You can do this once your recording is finished."), "law 4: {:?} greys without its line", b.action);
            }
        }
    }
    // A browser page opens only right after the click that asked for it.
    for e in fx {
        if matches!(e, Effect::Shell(ShellCmd::OpenUrl(_))) {
            assert!(matches!(ev, Event::Act { .. } | Event::Key { .. }), "law 4: a page opened without a click — {}", ctx());
        }
    }

    // Law 5 — an unasked popover never takes the keyboard.
    if let (Some(p), Some(d)) = (&ns.popover, &nw.session.popover) {
        if !d.asked {
            assert!(!p.activates, "law 5: an unasked popover activates — {}", ctx());
        }
    }

    if let (Some(o), Some(d)) = (&ns.onboarding, nw.session.windows.get(&WindowKind::Onboarding)) {
        if !d.asked {
            assert!(!o.activates, "law 5: an unasked window activates — {}", ctx());
        }
    }
    for (activates, kind) in [
        (ns.library.as_ref().map(|l| l.activates), WindowKind::Library),
        (ns.settings.as_ref().map(|s| s.activates), WindowKind::Settings),
        (ns.feedback.as_ref().map(|f| f.activates), WindowKind::Feedback),
    ] {
        if let (Some(a), Some(d)) = (activates, nw.session.windows.get(&kind)) {
            assert!(d.asked || !a, "law 5: an unasked {kind:?} window activates — {}", ctx());
        }
    }

    // Law 6, as Bagrat asked for it on 2026-10-12 (DEVIATIONS #38; the ratified wording was
    // “Dock iff a window is open”): the Dock only while the setup window is open. Once
    // shroom is set up it lives in the menu bar — the Library and Settings never bring the
    // Dock icon, and neither does a popover.
    if let Some(why) = law_6(nw, ns) {
        panic!("{why} — {}", ctx());
    }

    // Law 7 — one banner and one status line at most (by type); banner priority N38.
    if let Some(p) = &ns.popover {
        if let Some(b) = &p.banner {
            if !nw.saved.is_empty() {
                assert_eq!(b.id, "recovered-take", "law 7: a saved take outranks every banner — {}", ctx());
            }
            // N38's order: a saved take · the space can't take videos · recording-off · an update.
            let rank = |id: &str| match id {
                "recovered-take" => 1,
                // Kind 2 — the space can't take videos (N38), in every role's words (N34, N42,
                // N37's needs-update siblings).
                "space-degraded"
                | "space-degraded-no-token-mac"
                | "member-revoked"
                | "space-needs-owner"
                | "space-gone"
                | "space-needs-update-member"
                | "space-needs-update-no-token"
                // DEVIATIONS #21: a removed owner-class Mac's line, in member-revoked's place.
                | "owner-mac-removed" => 2,
                "recording-off" => 3,
                _ => 4,
            };
            let degraded = shroom_core::derive::settings::degraded(nw);
            let higher = !nw.saved.is_empty() && rank(b.id) > 1
                || degraded && rank(b.id) > 2
                || nw.mac.screen.is(&false) && rank(b.id) > 3;
            assert!(!higher, "law 7: {} shown over a higher banner — {}", b.id, ctx());
            if b.tone == shroom_core::scene::Tone::Frost {
                assert!(rank(b.id) == 4, "law 7: only the update is Frost — {}", ctx());
            }
        }
    }
    // The Frost dot is the lowest face: never with a take, never over amber.
    if ns.tray.face == Face::Frost {
        assert!(nw.take.is_none() && nw.saved.is_empty() && !shroom_core::derive::settings::degraded(nw), "law 7: Frost over a higher face — {}", ctx());
        assert!(nw.provisioned(), "law 7: the Frost dot before a space exists — {}", ctx());
    }

    // Laws 8 and 9 — Return never triggers a destructive action; destructive controls are
    // red text or red outline, never filled.
    if let Some(p) = &ns.popover {
        for b in p.buttons() {
            let destructive = matches!(b.action, Action::ConfirmDiscard | Action::ConfirmDiscardSaved | Action::Discard | Action::DiscardSaved | Action::ConfirmRestart);
            if destructive {
                assert!(matches!(b.style, Style::RedText | Style::RedOutline), "law 9: {:?} is styled {:?}", b.action, b.style);
                assert_ne!(p.ret.as_ref(), Some(&b.action), "law 9: Return triggers {:?}", b.action);
            }
        }
        if p.id == "discard-confirm" || p.id == "quit-confirm" || p.id == "restart-confirm" {
            assert_eq!(p.ret, None, "law 9: no Return default on a question");
            assert_eq!(p.esc, Some(Action::Back), "law 9: Esc means Back");
        }
    }
    // Law 8 — a window's Return default is one of its own buttons (or its focused field's).
    if let Some(o) = &ns.onboarding {
        if let Some(r) = &o.ret {
            assert!(o.buttons().iter().any(|b| &b.action == r), "law 8: {}'s Return {r:?} isn't on the window — {}", o.id, ctx());
        }
    }
    if let Some(l) = &ns.library {
        if let Some(r) = &l.ret {
            assert_eq!(r, &Action::CommitRename, "law 8: the Library's Return belongs to the title field — {}", ctx());
            assert!(l.body.rows.iter().any(|row| row.rename.is_some()), "law 8: Return saves a title nobody is typing — {}", ctx());
        }
    }
    if let Some(st) = &ns.settings {
        if let Some(r) = &st.ret {
            assert!(st.buttons().iter().any(|b| &b.action == r), "law 8: Settings' Return {r:?} isn't on the window — {}", ctx());
        }
    }
    // Law 9 — the windows' destructive entry points are red text; their questions are
    // sheets with the destructive answer red-outlined and rightmost, no Return default,
    // and Esc meaning back.
    let mut sheets: Vec<(&Sheet, Option<&Action>, Option<&Action>)> = Vec::new();
    if let Some(l) = &ns.library {
        for row in &l.body.rows {
            for b in row.buttons.iter().chain(&row.hover).chain(&row.menu) {
                if matches!(b.action, Action::DeleteRow(_) | Action::DiscardRow(_)) {
                    assert_eq!(b.style, Style::RedText, "law 9: {:?} isn't red text", b.action);
                }
            }
        }
        sheets.extend(l.body.sheet.iter().map(|s| (s, l.ret.as_ref(), l.esc.as_ref())));
    }
    if let Some(st) = &ns.settings {
        // Remove… is red text on every Members row it's on (N24's pattern, N28).
        for b in st.buttons() {
            if matches!(b.action, Action::Remove(_)) {
                assert_eq!(b.style, Style::RedText, "law 9: {:?} isn't red text", b.action);
            }
        }
        sheets.extend(st.body.sheet.iter().map(|s| (s, st.ret.as_ref(), st.esc.as_ref())));
    }
    for (sheet, ret, esc) in sheets {
        assert_eq!(ret, None, "law 9: {} has a Return default — {}", sheet.id, ctx());
        assert_eq!(esc, Some(&Action::Back), "law 9: {}'s Esc isn't back — {}", sheet.id, ctx());
        // The law's colours are for destructive questions; Report a problem's Send destroys
        // nothing, and is filled as an ordinary answer is (B180).
        if NOT_DESTRUCTIVE.contains(&sheet.id) {
            continue;
        }
        let last = sheet.buttons.last().expect("a sheet answers");
        assert_eq!(last.style, Style::RedOutline, "law 9: {}'s answer isn't red-outlined and rightmost", sheet.id);
        assert!(sheet.buttons.iter().all(|b| b.style != Style::Filled), "law 9: a filled button on {}", sheet.id);
    }

    // Law 16 — in the app's sentences one to nine are words; compact facts (the Library's
    // count, lengths, row facts) keep figures.
    let mut sentences: Vec<&str> = Vec::new();
    if let Some(p) = &ns.popover {
        sentences.extend(p.banner.as_ref().map(|b| b.line.as_str()));
        sentences.extend(p.status_line.as_deref());
        // The popover's own sentences too: config's lines under its fields and Start, the
        // paused popover's context, the questions and the stop faces (gap: “About 7
        // minutes…” once slipped by here).
        match &p.body {
            PopoverBody::Config(c) => {
                sentences.extend([c.room_line.as_deref(), c.mic_line.as_deref(), c.camera_line.as_deref(), c.start.note.as_deref()].into_iter().flatten());
                sentences.push(&c.hint);
            }
            PopoverBody::Paused(b) => {
                sentences.extend([b.context.as_deref(), b.sub.as_deref()].into_iter().flatten());
            }
            PopoverBody::Question(q) => sentences.extend([q.title.as_str(), q.line.as_str()]),
            PopoverBody::Stopped(st) => {
                sentences.extend(st.title.as_deref());
                sentences.push(&st.line);
            }
        }
    }
    for sheet in ns.library.iter().filter_map(|l| l.body.sheet.as_ref()).chain(ns.settings.iter().filter_map(|s| s.body.sheet.as_ref())) {
        sentences.push(&sheet.title);
        sentences.extend(sheet.lines.iter().map(String::as_str));
        sentences.extend(sheet.blocked.as_deref());
    }
    for s in sentences {
        assert!(!figure_counts(s), "law 16: a count under ten as a figure in “{s}” — {}", ctx());
    }

    // Law 10 — nothing auto-copies: the clipboard is written only by a Copy click.
    for e in fx {
        if matches!(e, Effect::Shell(ShellCmd::Copy(_))) {
            // Return is the visible default's own key (law 8): `stopped-offline`'s Copy link
            // is its fixture's Return default (r4), so Return there is that button's press —
            // the button then reads “Copied” (DEVIATIONS #36 asks Bagrat to confirm).
            let by_return = matches!(ev, Event::Key { key: shroom_core::event::Key::Return, .. }) && ps.popover.as_ref().is_some_and(|p| p.ret == Some(Action::CopyLink));
            assert!(matches!(ev, Event::Act { action: Action::CopyLink | Action::CopyRow(_), .. }) || by_return, "law 10: copied without a click — {}", ctx());
        }
        // A picture reaches the clipboard only from its own Copy thumbnail click (N46).
        if let Effect::Shell(ShellCmd::CopyThumb(rec, _)) = e {
            assert!(matches!(ev, Event::Act { action: Action::CopyThumbRow(r), .. } if r == rec), "law 10: a picture copied without its click — {}", ctx());
        }
        // A join link reaches the clipboard only from its own Copy link click.
        if let Effect::Shell(ShellCmd::CopyInvite(id)) = e {
            assert!(matches!(ev, Event::Act { action: Action::CopyInvite(c), .. } if c == id), "law 10: a link copied without its click — {}", ctx());
        }
    }

    // Law 11 — macOS prompts only right after a click that asked (or Return, its key);
    // the chained prompts are the Allow recording click's later halves: the camera on the
    // answer to the mic prompt that click put to macOS, and the screen step on the answer
    // to the last of those (ARCHITECTURE §3.3, N43) — each consumed once; never mid-take.
    use shroom_core::effect::Perm;
    use shroom_core::world::Access;
    let clicked = matches!(ev, Event::Act { .. } | Event::Key { .. });
    let grants = matches!(ev, Event::Os(OsNews::Grants { .. }));
    let mic_answered = pw.mac.mic_asked && !nw.mac.mic_asked && !nw.mac.mic.is(&Access::NotAsked);
    let camera_answered = pw.mac.camera_asked && !nw.mac.camera_asked && !nw.mac.camera.is(&Access::NotAsked);
    if !pw.mac.mic_asked && nw.mac.mic_asked {
        assert!(clicked, "law 11: a pending mic answer without the click that asked — {}", ctx());
    }
    if !pw.mac.camera_asked && nw.mac.camera_asked {
        assert!(clicked || (grants && mic_answered), "law 11: a pending camera answer without the click that asked — {}", ctx());
    }
    for e in fx {
        if let Effect::Shell(ShellCmd::RequestPermission(perm)) = e {
            let chained = grants
                && pw.session.chapter
                && match perm {
                    Perm::Camera => mic_answered,
                    Perm::Screen => mic_answered || camera_answered,
                    Perm::Mic => false,
                };
            assert!(clicked || chained, "law 11: a prompt without a click — {}", ctx());
            assert!(!nw.take_open(), "law 11: a prompt mid-take — {}", ctx());
        }
    }

    // Law 13 — secrets ride files: no secret text in any Scene or Effect.
    let scene = serde_json::to_string(ns).unwrap();
    assert!(!scene.contains(SECRET_CANARY), "law 13: secret text in the Scene — {}", ctx());
    for e in fx {
        assert!(!format!("{e:?}").contains(SECRET_CANARY), "law 13: secret text in an effect — {}", ctx());
    }

    // Law 13 — a join link rides as a stash handle (Region 6): no join code anywhere in the
    // Scene or an effect. A `/join/` path may show only its host part, the code drawn from
    // the stash beside it.
    for (what, text) in [("the Scene", scene.clone())].into_iter().chain(fx.iter().map(|e| ("an effect", format!("{e:?}")))) {
        assert!(!carries_join_code(&text), "law 13: a join code in {what} — {}", ctx());
    }

    // Law 12 — failures are states: every failure ends in a stated face or a quiet retry.
    law12(pw, ps, ev, nw, ns, fx, &ctx);

    // Law 14 — one space per Mac; every Mac of the owner is owner-class; members own their
    // videos, the owner sees all.
    law14(pw, ps, ev, nw, ns, fx, &ctx);

    // Law 15 — product voice: no user-visible string names an internal or an error code
    // (TESTING-PLAN §2.4's lint, over every string the Scene shows).
    for text in user_strings(ns) {
        if let Some(why) = voice(&text) {
            panic!("law 15: “{text}” {why} — {}", ctx());
        }
    }

    let _ = Surface::Popover;
}

/// Law 12, over one step (the machine's law 12, ARCHITECTURE §3.7 item 7): a failure the core
/// hears — an op's error, its deadline, a capture that died — lands somewhere visible (a
/// face or state id not showing before, a line not said before, or the failure's own note
/// read by the Scene) or is a quiet retry (the failure noted, and at its wake something runs
/// again), or the work is re-issued — never a stall. A spinner going away is not a face: the
/// Scene the World would show with the op simply dropped doesn't count as saying anything.
/// As-you-type guards filter and never move the window, never land a rejected character,
/// lowercase silently and never scold; `wizard-offline` resumes by itself, with no retry button.
/// Whether the fixtures name this id a failure-state (law 12's distinct failure faces).
pub fn failure_state(id: &str) -> bool {
    static IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| shroom_spec::Spec::load().states().filter(|s| s.kind == "failure-state").map(|s| s.id.clone()).collect()).iter().any(|s| s == id)
}

/// The background work law 12 lets go quietly once nothing wants it any more: reads, and
/// the uploader — never a write a person asked for.
const BACKGROUND: &[Op] = &[Op::Standing, Op::Roster, Op::Peek, Op::LibCache, Op::LibRefresh, Op::Reach, Op::Sweep, Op::Refresh, Op::UpdateCheck, Op::UpdateDownload, Op::Upload];

/// An op whose failure another op covers, with why.
const COVERED: &[(Op, Op, &str)] = &[(Op::LibCache, Op::LibRefresh, "the remembered list is a head start; the bucket refresh fills the same list")];

/// How far law 12 follows a noted failure's wakes looking for its retry: past the longest
/// background schedule (an update check's four hours, a page refresh's hour).
const RETRY_HORIZON_MS: u64 = 8 * 3_600_000;

fn law12(pw: &World, ps: &Scene, ev: &Event, nw: &World, ns: &Scene, fx: &[Effect], ctx: &dyn Fn() -> String) {
    let quiet = |s: &Scene| {
        let mut s = s.clone();
        s.next_wake = None;
        s
    };
    let ids_before = ps.state_ids();
    let new_face = ns.state_ids().into_iter().find(|i| !ids_before.contains(i));
    let lines_before = user_strings(ps);
    let new_line = user_strings(ns).into_iter().find(|l| !lines_before.contains(l));
    let landed = new_face.is_some() || new_line.is_some();
    let working = nw.inflight.ops.values().any(|d| *d > nw.now) && nw.inflight.ops.len() >= pw.inflight.ops.len();
    let failure = match ev {
        // An op that was running failed, or ran out of time.
        Event::Failed { op, .. } if pw.inflight.ops.contains_key(op) => {
            assert!(nw.inflight.ops.get(op).is_none_or(|d| *d > nw.now), "law 12: {op:?} failed and still waits on a deadline gone by — {}", ctx());
            assert!(lands(pw, ps, nw, ns, fx, *op), "law 12: {op:?} failed into nothing — no face, no line, no note read, no retry — {}", ctx());
            None
        }
        // The deadline sweep: every op past its deadline is gone or re-issued after a Tick.
        Event::Tick => {
            for (op, d) in &nw.inflight.ops {
                assert!(*d > nw.now, "law 12: {op:?} is past its deadline after a Tick — {}", ctx());
            }
            // An op that ran out of time is a failure like any other: it lands too.
            for (op, _) in pw.inflight.ops.iter().filter(|(_, d)| **d <= nw.now) {
                assert!(lands(pw, ps, nw, ns, fx, *op), "law 12: {op:?} timed out into nothing — no face, no line, no note read, no retry — {}", ctx());
            }
            None
        }
        // A capture that died is a stop you didn't make (bend 6): a saved take and its face.
        Event::Rec(RecNews::Failed | RecNews::FrameStall | RecNews::Finalized(Err(_))) if pw.take.is_some() => Some("the capture's failure"),
        Event::Rec(RecNews::SaveFailed(_)) => Some("a copy to Movies that couldn't be made"),
        _ => None,
    };
    if let Some(what) = failure {
        assert!(landed || working || quiet(ps) != quiet(ns), "law 12: {what} changed nothing anyone sees, and nothing runs — {}", ctx());
    }
    // As-you-type guards filter, never scold: typing never moves the window (the one move
    // is name-taken → name, the field no longer holding the taken name).
    if let (Event::Field { .. }, Some(a), Some(b)) = (ev, &ps.onboarding, &ns.onboarding) {
        assert!(a.id == b.id || (a.id == "name-taken" && b.id == "name"), "law 12: typing moved {} to {} — {}", a.id, b.id, ctx());
    }
    // The name guard (N15), recomputed here rather than trusted to the core: a rejected
    // character never lands, uppercase lowercases silently, and the only line typing may
    // add is the quiet fading explanation — and only when a character was rejected.
    if let (Event::Field { field: Field::Name, value }, Some(a)) = (ev, &ps.onboarding) {
        if ["name", "name-taken"].contains(&a.id) {
            let lowered: String = value.chars().flat_map(char::to_lowercase).collect();
            let allowed = |c: &char| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-';
            let kept: String = lowered.chars().filter(allowed).take(63).collect();
            let name = &nw.session.fields.name;
            assert!(name.chars().all(|c| allowed(&c)), "law 12: a rejected character landed in the name “{name}” — {}", ctx());
            assert_eq!(name, &kept, "law 12: the name guard kept “{name}” of “{value}” — {}", ctx());
            let rejected = lowered.chars().any(|c| !allowed(&c));
            if ns.onboarding.as_ref().is_some_and(|b| b.id == a.id) {
                for line in user_strings(ns).into_iter().filter(|l| !lines_before.contains(l)) {
                    let echo = !name.is_empty() && line.contains(name.as_str());
                    let explained = rejected && line == shroom_core::copy::charset();
                    assert!(echo || explained, "law 12: typing “{value}” said “{line}” — {}", ctx());
                }
            }
        }
    }
    // wizard-offline auto-resumes on reconnect: no retry button on it.
    if let Some(o) = ns.onboarding.as_ref().filter(|o| o.id == "wizard-offline") {
        assert!(!o.buttons().iter().any(|b| b.action == Action::TryAgain), "law 12: a retry button on wizard-offline — {}", ctx());
    }
    if let Some(st) = ns.settings.as_ref().filter(|s| s.faces.contains(&"wizard-offline")) {
        assert!(!st.buttons().iter().any(|b| b.action == Action::TryAgain), "law 12: a retry button on wizard-offline — {}", ctx());
    }
}

/// Whether `op`'s failure (its error, or its deadline gone by) over this step lands: a face
/// or line the op merely going away wouldn't show, the failure's note read by the Scene, a
/// noted retry that runs again, or new work answering the noted failure at once.
fn lands(pw: &World, ps: &Scene, nw: &World, ns: &Scene, fx: &[Effect], op: Op) -> bool {
    let quiet = |s: &Scene| {
    let mut s = s.clone();
    s.next_wake = None;
    s
    };
    let (ids_before, lines_before) = (ps.state_ids(), user_strings(ps));
    // What the op merely going away would show (its spinner gone, its button
    // back): not a face. The failure lands only past that.
    let mut dropped = pw.clone();
    dropped.inflight.ops.remove(&op);
    dropped.now = nw.now;
    dropped.wall = nw.wall;
    let gone = shroom_core::derive(&dropped);
    let (gone_ids, gone_lines) = (gone.state_ids(), user_strings(&gone));
    // A failure-state of the fixtures is the failure's own face even when the op
    // going away would derive it too (join-waiting: asked, unanswered, not running).
    let face = ns.state_ids().into_iter().find(|i| !ids_before.contains(i) && (!gone_ids.contains(i) || failure_state(i)));
    let line = user_strings(ns).into_iter().find(|l| !lines_before.contains(l) && !gone_lines.contains(l));
    let landed = face.is_some() || line.is_some();
    let noted = nw.inflight.failures.get(&op).filter(|f| pw.inflight.failures.get(&op) != Some(*f));
    // The note itself is read: the Scene with this failure un-noted reads differently.
    let note_shows = noted.is_some() && {
        let mut un = nw.clone();
        match pw.inflight.failures.get(&op) {
            Some(f) => un.inflight.failures.insert(op, *f),
            None => un.inflight.failures.remove(&op),
        };
        quiet(&shroom_core::derive(&un)) != quiet(ns)
    };
    // A quiet retry: the failure is noted with a wake, and — nothing answering
    // meanwhile — the same op runs again at its backoff or by the horizon (each op keeps
    // its own schedule: the wizard's backoff, an update check's hours, a page refresh's
    // hour). A Tick at the backoff, then one at the horizon, fires everything due by then.
    let retry = noted.is_some_and(|f| {
        ns.next_wake.is_some() && {
            // Everything else still running stays running (its answer isn't this op's).
            let mut later = nw.clone();
            let past = nw.now.plus(RETRY_HORIZON_MS + 1);
            later.inflight.ops.values_mut().for_each(|d| *d = (*d).max(past));
            [f.retry_at(), nw.now.plus(RETRY_HORIZON_MS)].into_iter().any(|at| {
                let wall = shroom_core::Wall(nw.wall.0 + at.0.saturating_sub(nw.now.0) as i64);
                let (more, _) = shroom_core::update(&mut later, shroom_core::Envelope { mono: at, wall, event: Event::Tick });
                more.iter().any(|e| e.op() == Some(op))
            })
        }
    });
    // Re-issued at once, or answered at once with new work (a lost claim re-probes) —
    // only with the failure noted: an un-noted failure re-issued is a retry with no
    // backoff, the same call hammered for as long as it fails.
    let reissued = noted.is_some() && nw.inflight.ops.get(&op).is_some_and(|d| *d > nw.now);
    let answered = noted.is_some() && fx.iter().filter_map(Effect::op).any(|o| !pw.inflight.ops.contains_key(&o));
    // Background work (a read, the uploader) the plan wouldn't ask again even with no
    // backoff is moot, not a stall, when nobody watched it run (a probe for an invite that's
    // now joining) or the Scene already says why it can't (an upload to a space that's gone,
    // under its amber banner). Never a write a person asked for: a rename, a claim, a join,
    // an invite that fails must say so.
    let moot = BACKGROUND.contains(&op) && {
        let mut then = pw.clone();
        then.now = nw.now;
        then.wall = nw.wall;
        let unseen = quiet(&shroom_core::derive(&then)) == quiet(&gone);
        let explained = ns.tray.face == Face::Amber || ns.state_ids().iter().any(|i| failure_state(i));
        let mut fresh = nw.clone();
        fresh.inflight.failures.remove(&op);
        let asked_again = nw.inflight.ops.contains_key(&op) || shroom_core::settle::settle(&fresh).iter().any(|e| e.op() == Some(op));
        (unseen || explained) && !asked_again
    };
    // Covered: another op answers the same surface and answers to law 12 itself (the
    // Library's remembered list is a head start; the bucket refresh fills the same list, and
    // offline the Library's own offline face says so).
    let covered = COVERED.iter().any(|(o, by, _)| *o == op && (nw.inflight.ops.contains_key(by) || nw.inflight.failures.contains_key(by) || nw.online() == Some(false)));
    // The Library's Open page is a link: the browser is its answer, and the lane posts
    // `PageOpened` whatever happens — its deadline only catches a lane that wedged
    // (DECISIONS B57). The stop's page (`page_open`) is retried, and is held to the law.
    let link = op == Op::OpenPage && nw.session.page_open.is_none();
    landed || note_shows || retry || reissued || answered || moot || covered || link
}

/// Law 15 over one Scene: each string that breaks it, named for the row it came from.
pub fn voice_failures(scene: &Scene, row: &str) -> Vec<String> {
    user_strings(scene).into_iter().filter_map(|t| voice(&t).map(|why| format!("{row}: law 15: “{t}” {why}"))).collect()
}

/// Every string the Scene shows a person: labels, lines, notes, titles, placeholders —
/// not ids, actions, kinds, device ids or what the person typed.
pub fn user_strings(scene: &Scene) -> Vec<String> {
    // `preview` is Report a problem's disclosure: the person's words and their log, quoted
    // as they go — data, as a field's `value` is (B180).
    const NOT_COPY: &[&str] = &[
        "id", "faces", "do", "arg", "kind", "key", "camera", "poster", "value", "style", "tone", "face", "click", "glyph", "em", "action", "anchor", "state",
        "preview",
    ];
    fn walk(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => {
                for (k, x) in o {
                    if !NOT_COPY.contains(&k.as_str()) {
                        walk(x, out);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&serde_json::to_value(scene).expect("the Scene serializes"), &mut out);
    // A Library row's state is copy (“Uploading — 40%”); the tray's is a state id.
    if let Some(l) = &scene.library {
        out.extend(l.body.rows.iter().filter_map(|r| r.state.clone()));
    }
    out
}

/// Words that name the machine, never the product (law 15; TESTING-PLAN §2.4): matched as
/// whole words, case-sensitively — “Worker” is the internal, workers.dev inside a link is
/// the link.
pub const INTERNAL_WORDS: &[&str] = &["Worker", "Workers", "R2", "HLS", "S3", "SCK", "TCC", "JSON", "HTTP", "HTTPS", "API", "URL", "SDK", "CLI"];
/// Internals, matched as the start of a word whatever the case (“bucket”, “Buckets”,
/// “credentials”), and phrases matched anywhere.
pub const INTERNAL_STEMS: &[&str] = &[
    "bucket", "creds", "credential", "ffmpeg", "ffprobe", "avfoundation", "screencapturekit", "wrangler", "tauri", "m3u8", "playlist", "manifest",
    "segment", "stderr", "stdout", "exception", "panic", "traceback", "undefined", "errno", "endpoint", "webhook", "daemon", "launchd", "subdomain",
    "sha-256", "sha256", "checksum", "hash", "keychain", "localhost",
];
pub const INTERNAL_PHRASES: &[&str] = &[
    "api token", "api key", "global api", "account token", "user token", "access key", "secret key", "error code", "status code", "error:", "failed with",
    // HTTP's reason phrases are the machine talking.
    "forbidden", "unauthorized", "bad request", "bad gateway", "gateway timeout", "internal server error", "service unavailable", "too many requests",
];

/// Why a user-visible string breaks law 15, if it does.
pub fn voice(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-')).filter(|w| !w.is_empty()).collect();
    for w in &words {
        let bare = w.trim_matches('-');
        if INTERNAL_WORDS.contains(&bare) {
            return Some(format!("names the internal “{bare}”"));
        }
        let lower = bare.to_lowercase();
        if let Some(stem) = INTERNAL_STEMS.iter().find(|s| lower.starts_with(*s)) {
            return Some(format!("names the internal “{stem}”"));
        }
    }
    let lower = text.to_lowercase();
    if let Some(p) = INTERNAL_PHRASES.iter().find(|p| lower.contains(*p)) {
        return Some(format!("says “{p}”"));
    }
    // A quoted span echoes the person's own words (a search, a title): never a code of ours.
    let mut unquoted = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '“' => depth += 1,
            '”' => depth = (depth - 1).max(0),
            _ if depth == 0 => unquoted.push(c),
            _ => {}
        }
    }
    error_code(&unquoted).map(|c| format!("shows the error code {c}"))
}

/// What a number of three or more digits may count or measure in product copy (“100
/// hours”, “140 videos”, “500 MB”): the word after it, singular or plural, whatever the case.
pub const COUNT_NOUNS: &[&str] = &[
    "hour", "minute", "second", "day", "week", "month", "year", "video", "recording", "take", "member", "people", "person", "mac", "link", "invite", "file",
    "frame", "fps", "character", "letter", "word", "time", "view", "gb", "mb", "kb", "tb", "gigabyte", "megabyte", "kilobyte", "ms", "px", "more", "other",
];
/// Words that make the number after them a code whatever follows (“Error 502 occurred”,
/// “status 403”).
pub const CODE_MARKERS: &[&str] = &["error", "code", "status", "http", "got", "returned", "errno", "exit"];

/// An error code: a standalone number of three or more digits (10031, 8000077, “403
/// Forbidden”, “Error 502 occurred”, “Got 500 from the space”), or digits glued to a code's
/// marker (#403, E1001, 0x…) — never a length (4:32), a version (1.3.0), a percentage, a
/// price, a count of something (“100 hours”, “140 videos”: the word after it is one of
/// `COUNT_NOUNS`), or part of a link. After `CODE_MARKERS` a number is a code whatever
/// follows it.
pub fn error_code(text: &str) -> Option<&str> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let bare = |t: &str| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    for (i, raw) in tokens.iter().copied().enumerate() {
        // A link is the link (law 15's own carve-out for the workers.dev host).
        if raw.contains("://") || raw.contains(".workers.dev") || raw.contains('@') {
            continue;
        }
        let t = raw.trim_matches(|c: char| matches!(c, ',' | ';' | '.' | '!' | '?' | '(' | ')' | '“' | '”' | '"' | '\'' | ':'));
        let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
        if digits >= 3 && t.chars().all(|c| c.is_ascii_digit()) {
            let marked = i > 0 && CODE_MARKERS.contains(&bare(tokens[i - 1]).as_str());
            // The number counts the word right after it — not across a stop (“…5000. Try”).
            let ends_clause = raw.ends_with([',', ';', '.', ')', ':', '!', '?']);
            let counts = !ends_clause
                && tokens.get(i + 1).is_some_and(|n| {
                    let w = bare(n);
                    COUNT_NOUNS.iter().any(|c| w == *c || w.strip_suffix('s') == Some(*c) || w.strip_suffix("es") == Some(*c))
                });
            if marked || !counts {
                return Some(raw);
            }
        }
        let marked = t.strip_prefix('#').or_else(|| t.strip_prefix("0x")).or_else(|| t.strip_prefix('E'));
        if marked.is_some_and(|m| m.len() >= 3 && m.chars().all(|c| c.is_ascii_hexdigit()) && m.chars().any(|c| c.is_ascii_digit())) {
            return Some(raw);
        }
    }
    None
}

/// Law 14, over one step.
fn law14(pw: &World, ps: &Scene, ev: &Event, nw: &World, ns: &Scene, fx: &[Effect], ctx: &dyn Fn() -> String) {
    // One space per Mac: moving to another space happens only as the answer to a Join the
    // person asked for (`joining` showed), and nothing of the old space stays behind — its
    // token set aside, its records forgotten (§5's switch).
    let host = |w: &World| w.space.identity.as_ref().map(|i| i.host.clone());
    if pw.provisioned() && nw.provisioned() && host(pw) != host(nw) {
        let joined = matches!(ev, Event::Member(MemberNews::Joined { answer: JoinAnswer::Joined(_), .. }));
        let asked = ps.state_ids().iter().any(|i| ["joining", "join-waiting"].contains(i));
        assert!(joined && asked, "law 14: this Mac changed spaces without a Join — {}", ctx());
        assert!(!nw.space.token, "law 14: the old space's token outlived the switch — {}", ctx());
        assert!(nw.library.get().is_none(), "law 14: the old space's records outlived the switch — {}", ctx());
        // Nothing of the old space rides into the new one: no take queued for it survives.
        for q in &pw.queue {
            assert!(!nw.queue.iter().any(|x| x.dir == q.dir), "law 14: {:?} of the old space queued for the new one — {}", q.dir, ctx());
        }
    }
    // Nothing from the old space is ever orphaned by a switch: no join runs over a space
    // whose uploads still drain or whose recovered take waits (N31) — as the World stands
    // when the join is asked (a step's effects are settled from the World after it: the
    // step that resolves the saved take may ask the join).
    let in_a_space = nw.provisioned() || nw.space.token;
    for e in fx {
        if matches!(e, Effect::Member(MemberCmd::Join(_))) && in_a_space {
            assert_eq!(shroom_core::derive::settings::draining(nw), 0, "law 14: a switch while uploads drain — {}", ctx());
            assert!(nw.saved.is_empty(), "law 14: a switch while a recovered take waits — {}", ctx());
        }
    }
    let role = nw.space.identity.as_ref().map(|i| i.role);
    // The Members tab is owner-class only (N5, N33).
    if let Some(st) = &ns.settings {
        let members_tab = st.body.tabs.iter().any(|t| t.action == Action::SettingsTab(shroom_core::world::SettingsTab::Members));
        let owner_class = matches!(role, Some(Role::TokenHolder | Role::OwnerClass));
        assert_eq!(members_tab, owner_class, "law 14: the Members tab on a {role:?} Mac — {}", ctx());
        let faces = st.faces.clone();
        assert!(owner_class || !faces.contains(&"settings-members"), "law 14: Members on a member's Mac — {}", ctx());
        // Only a Mac holding the token repairs or re-wires the space (N33, N42).
        if !nw.space.token {
            for f in ["settings-space-repair", "repair-mood", "token-repair-mood", "space-storage-gone", "name-changed", "wiring"] {
                assert!(!faces.contains(&f), "law 14: {f} on a Mac without the token — {}", ctx());
            }
        }
    }
    // A member never sees the admin's repair tooling, nor an owner the member faces.
    if let Some(b) = ns.popover.as_ref().and_then(|p| p.banner.as_ref()) {
        let member_faces = ["member-revoked", "space-needs-owner", "space-gone", "space-needs-update-member"];
        match role {
            Some(Role::Member) => assert!(!["space-degraded", "space-degraded-no-token-mac"].contains(&b.id), "law 14: {} on a member's Mac — {}", b.id, ctx()),
            Some(Role::TokenHolder) => assert!(!member_faces.contains(&b.id) && b.id != "space-degraded-no-token-mac", "law 14: {} on the token's Mac — {}", b.id, ctx()),
            Some(Role::OwnerClass) => assert!(!member_faces.contains(&b.id) && b.id != "space-degraded", "law 14: {} on an owner-class Mac — {}", b.id, ctx()),
            None => {}
        }
    }
    // Members own their videos: a member's Library lists only its own (N5).
    if let (Some(Role::Member), Some(l), Some(rows)) = (role, &ns.library, nw.library.get()) {
        let me = nw.space.identity.as_ref().map(|i| i.member_id.as_str());
        for view in &l.body.rows {
            let theirs = rows.iter().any(|r| r.rec.0 == view.key && r.owner.as_deref() != me);
            assert!(!theirs, "law 14: someone else's video {} in a member's Library — {}", view.key, ctx());
        }
    }
}

/// A `/join/` path followed by a code's characters.
pub fn carries_join_code(text: &str) -> bool {
    text.match_indices("/join/").any(|(i, m)| text[i + m.len()..].chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
}

/// “2 videos”, “6 recordings”, “About 7 minutes”: a count from one to nine written as a
/// figure — a lone digit followed by a word. Compact facts keep their figures: lengths
/// (“4:32”), versions (“0.2.1”), percentages and dates (“Aug 3”, a figure that ends the
/// fact), and a numbered name never reaches these sentences.
fn figure_counts(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len()).any(|i| {
        (b'1'..=b'9').contains(&b[i])
            && (i == 0 || !b[i - 1].is_ascii_digit() && b[i - 1] != b'.' && b[i - 1] != b':')
            && i + 2 < b.len()
            && b[i + 1] == b' '
            && s[i + 2..].chars().next().is_some_and(|c| c.is_alphabetic())
    })
}

// ── law 12, invariant 1: every blocking failure is its own state ──

/// Failure-state ids the fixtures don't spell in kebab-case, with why they stand (renaming
/// a spec id is Bagrat's — DECISIONS B57 (g)).
pub const SPEC_NAMED: &[(&str, &str)] = &[("update-failed", "r5 names the status-line fact with a space; the core keeps the spec's id")];

/// The surfaces the core renders (the installer's terminal and the Worker's pages are held
/// to law 12 by their own tests).
pub fn core_surface(surface: &str) -> bool {
    !["terminal", "web-page"].contains(&surface)
}

/// the machine's law 12, invariant 1, over a region's inventory rows: every failure-state of
/// the fixtures these rows reach is a distinct kebab-case id (or named in `SPEC_NAMED`),
/// shows in its row's Scene, renders as itself — its surface, ids aside, unlike any row
/// that doesn't show it — and has its own way on (a fixture transition, a map edge, an
/// edge the build listed, or an enabled button or window on its surface).
pub fn failures_are_states(rows: &[(&'static str, World)], owned: &[&str]) -> Vec<String> {
    let spec = shroom_spec::Spec::load();
    let mut out = Vec::new();
    let scenes: Vec<(&str, Scene)> = rows.iter().map(|(id, w)| (*id, shroom_core::derive(w))).collect();
    let mut seen = std::collections::BTreeSet::new();
    for st in spec.states().filter(|s| s.kind == "failure-state" && core_surface(&s.surface)) {
        let id = st.id.as_str();
        if !owned.contains(&id) || !seen.insert(id) {
            continue;
        }
        if !scenes.iter().any(|(row, _)| *row == id) {
            out.push(format!("{id}: no inventory row reaches it"));
        }
        let kebab = !id.is_empty() && id.split('-').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        if !kebab && !SPEC_NAMED.iter().any(|(n, _)| *n == id) {
            out.push(format!("{id}: not a kebab-case state id"));
        }
        if spec.states().filter(|s| s.id == id).count() != 1 {
            out.push(format!("{id}: the fixtures name it more than once"));
        }
        for (_, scene) in scenes.iter().filter(|(row, _)| *row == id) {
            if !scene.state_ids().contains(&id) {
                out.push(format!("{id}: its row's Scene doesn't show it — {:?}", scene.state_ids()));
                continue;
            }
            let Some((surface, mine)) = surface_showing(scene, id) else {
                out.push(format!("{id}: no surface carries it"));
                continue;
            };
            for (other, theirs) in scenes.iter().filter(|(_, s)| !s.state_ids().contains(&id)) {
                if surface_render(theirs, surface).is_some_and(|r| r == mine) {
                    out.push(format!("{id}: renders exactly as {other} does — a failure needs its own render"));
                }
            }
            let listed = !st.transitions.is_empty() || spec.rules.iter().any(|r| spec.expand(&r.from).contains(&id));
            let way_on = match surface {
                "popover" => scene.popover.as_ref().is_some_and(|p| p.buttons().iter().any(|b| b.enabled)),
                _ => true, // a window closes
            };
            if !listed && !way_on {
                out.push(format!("{id}: no way on — no transition, no edge, nothing enabled"));
            }
        }
    }
    for id in owned.iter().filter(|id| !seen.contains(*id)) {
        out.push(format!("{id}: not a failure-state the core renders"));
    }
    out
}

/// Every failure-state the core renders, by the test file whose inventory owns it: the
/// four region tests each check theirs, and `edges.rs` that the four lists cover the
/// fixtures' every one.
pub const FAILURE_HOMES: &[(&str, &[&str])] = &[
    ("onboarding", &["token-garbled", "token-wrong-powers", "gate-email", "gate-card", "name-taken", "wizard-offline", "wizard-unexpected"]),
    ("region3", &["permission-lost", "disk-full-stop"]),
    ("region5", &["token-other-account", "space-storage-gone", "name-changed", "update-failed"]),
    ("region6", &["join-waiting", "join-expired", "join-used", "join-space-missing", "join-already-here", "join-switch-confirm"]),
];

pub fn failures_owned_by(file: &str) -> &'static [&'static str] {
    FAILURE_HOMES.iter().find(|(f, _)| *f == file).map(|(_, ids)| *ids).unwrap_or(&[])
}

/// The surface whose ids carry `id`, and its render with ids, faces and wakes set aside.
fn surface_showing(scene: &Scene, id: &str) -> Option<(&'static str, serde_json::Value)> {
    ["popover", "onboarding", "library", "settings"].into_iter().find_map(|s| {
        let v = serde_json::to_value(scene).ok()?.get(s)?.clone();
        carries(&v, id).then(|| (s, strip(v)))
    })
}

fn surface_render(scene: &Scene, surface: &str) -> Option<serde_json::Value> {
    let v = serde_json::to_value(scene).ok()?.get(surface)?.clone();
    (!v.is_null()).then(|| strip(v))
}

fn carries(v: &serde_json::Value, id: &str) -> bool {
    match v {
        serde_json::Value::Object(o) => o.iter().any(|(k, x)| match (k.as_str(), x) {
            ("id", serde_json::Value::String(s)) => s == id,
            ("faces", serde_json::Value::Array(a)) => a.iter().any(|f| f.as_str() == Some(id)),
            _ => carries(x, id),
        }),
        serde_json::Value::Array(a) => a.iter().any(|x| carries(x, id)),
        _ => false,
    }
}

fn strip(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(o) => serde_json::Value::Object(o.into_iter().filter(|(k, _)| !["id", "faces"].contains(&k.as_str())).map(|(k, x)| (k, strip(x))).collect()),
        serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(strip).collect()),
        x => x,
    }
}

/// The sheets that ask no destructive question (law 9 is about destruction): their answer
/// may be filled (B180).
const NOT_DESTRUCTIVE: &[&str] = &["report-problem"];
