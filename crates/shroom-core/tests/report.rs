//! Report a problem and Copy diagnostics (spec N53, DECISIONS B180): Settings · General's
//! help row, the report-problem sheet, what the shell is asked to do, and the log's cleaning
//! — held to the same cases as the feedback page's (site/test/clean-cases.json).

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::derive;
use shroom_core::effect::{Effect, PublicUrl, ShellCmd};
use shroom_core::event::{Event, Field, Key, ReportNews, Surface};
use shroom_core::report::{self, Diagnostics, Kind, Post};
use shroom_core::scene::{Action, GeneralBody, ReportForm, Scene, SettingsPane, Sheet, Style};
use shroom_core::time::Mono;
use shroom_core::world::*;
use support::*;

trait Drive {
    fn set(&mut self, action: Action) -> &mut Self;
    fn field(&mut self, field: Field, value: &str) -> &mut Self;
}

impl Drive for Run {
    fn set(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Settings, action, rev: 0 })
    }
    fn field(&mut self, field: Field, value: &str) -> &mut Self {
        self.send(Event::Field { field, value: value.into() })
    }
}

fn faces(s: &Scene) -> Vec<&'static str> {
    s.settings.as_ref().map(|w| w.faces.clone()).unwrap_or_default()
}

fn general_open() -> World {
    let mut w = ready();
    w.prefs.settings_tab = Some(SettingsTab::General);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn general(s: &Scene) -> GeneralBody {
    match &s.settings.as_ref().expect("Settings").body.pane {
        SettingsPane::General(g) => g.clone(),
        other => panic!("not General: {other:?}"),
    }
}

fn sheet(s: &Scene) -> Sheet {
    s.settings.as_ref().unwrap().body.sheet.clone().expect("a sheet")
}

fn form(s: &Scene) -> ReportForm {
    s.settings.as_ref().unwrap().body.report.clone().expect("the report's form")
}

fn buttons(sheet: &Sheet) -> Vec<(&str, Style, bool)> {
    sheet.buttons.iter().map(|b| (b.label.as_str(), b.style, b.enabled)).collect()
}

fn gathered() -> Diagnostics {
    Diagnostics {
        shroom: "0.1.18".into(),
        macos: "15.3.1".into(),
        chip: "Apple M2".into(),
        memory: "16 GB".into(),
        setup: "2026-10-12 10:06 UTC key · 10:14 recording allowed · 10:15 wired".into(),
        role: "owns its space".into(),
        log: report::clean_log("  1200 updater staged 0.1.18 (/Users/jane/Applications/shroom.app)\n  413020 uploader 7f3a…: request to https://v.example-owner.workers.dev/x timed out"),
    }
}

/// The sheet asked, its report gathered, and words typed.
fn ready_to_send() -> Run {
    let mut r = Run::new(general_open());
    r.set(Action::ReportProblem);
    r.send(Event::Report(ReportNews::Gathered(gathered())));
    r.field(Field::ReportWords, "The link never finished uploading.");
    r
}

// ── Settings · General's help row ──

/// “Something off?” is a row with two buttons; GitHub stays under it as the public door,
/// with nothing but the version in its issue (N8) — and the mail door left the tab.
#[test]
fn general_carries_the_help_row_and_the_public_door() {
    let r = Run::new(general_open());
    let g = general(&r.scene);
    assert_eq!(g.help_label, "Something off?");
    assert_eq!((g.copy_diagnostics.label.as_str(), g.copy_diagnostics.style, g.copy_diagnostics.enabled), ("Copy diagnostics", Style::Plain, true));
    assert_eq!((g.report.label.as_str(), g.report.style, g.report.enabled), ("Report a problem…", Style::Plain, true));
    assert_eq!(g.help, "Or tell us in public, on GitHub.");
    assert_eq!(g.help_links.iter().map(|b| (b.label.as_str(), b.action.clone())).collect::<Vec<_>>(), [("GitHub", Action::ReportIssue)]);
    assert_eq!(g.sent, None);
}

/// Law 10: the clipboard changes only on Copy diagnostics' click. The button reads Copied
/// for a moment, as Copy link does.
#[test]
fn copy_diagnostics_copies_only_on_its_click_and_says_copied() {
    let mut r = Run::new(general_open());
    assert!(r.effects.is_empty());
    r.set(Action::CopyDiagnostics);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::CopyDiagnostics { role: "owns its space".into() })]);
    assert_eq!(general(&r.scene).copy_diagnostics.label, "Copied");
    let t = r.t;
    r.tick_to(t + 1_600);
    assert_eq!(general(&r.scene).copy_diagnostics.label, "Copy diagnostics", "back after the moment");
}

/// A member's Mac and a Mac with no space name themselves as such — and no more.
#[test]
fn the_report_names_the_macs_role_and_nothing_else_about_its_space() {
    let mut w = general_open();
    w.space.identity.as_mut().unwrap().role = Role::Member;
    let mut r = Run::new(w);
    r.set(Action::CopyDiagnostics);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::CopyDiagnostics { role: "a member’s Mac".into() })]);
}

// ── the sheet ──

/// Report a problem… opens the sheet and asks the shell to gather the report; until it
/// arrives the disclosure says so and Send waits.
#[test]
fn report_a_problem_opens_the_sheet_and_gathers_the_report() {
    let mut r = Run::new(general_open());
    r.set(Action::ReportProblem);
    assert!(r.ids().contains(&"report-problem"), "{:?}", r.ids());
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::GatherReport { role: "owns its space".into() })]);
    let s = sheet(&r.scene);
    assert_eq!(s.id, "report-problem");
    assert_eq!(s.title, "Report a problem");
    assert_eq!(s.lines, ["Tell us what happened. shroom adds its version, your Mac’s model and its recent log — never your videos, links, names or keys. It goes straight to the person who makes shroom."]);
    assert_eq!(buttons(&s), [("Cancel", Style::Plain, true), ("Send", Style::Filled, false)]);
    let f = form(&r.scene);
    assert_eq!(f.words.placeholder, "What were you doing, and what went wrong?");
    assert_eq!(f.reply_label, "Your email, if you’d like an answer (optional)");
    assert_eq!(f.disclosure, "See exactly what’s sent");
    assert_eq!(f.preview, None);
    assert_eq!(f.preparing.as_deref(), Some("Putting it together…"));
    assert_eq!((f.failed.clone(), f.mail.clone()), (None, None));
    assert_eq!((r.scene.ret(Surface::Settings), r.scene.esc(Surface::Settings)), (None, Some(Action::Back)));
}

/// Gathered and typed, Send is live, and the disclosure holds exactly what goes: the words,
/// the reply-to, the Mac's line and the cleaned log.
#[test]
fn the_disclosure_shows_exactly_what_goes() {
    let mut r = ready_to_send();
    assert_eq!(buttons(&sheet(&r.scene)).last().copied(), Some(("Send", Style::Filled, true)));
    r.field(Field::ReportReplyTo, "you@example.com");
    let f = form(&r.scene);
    assert_eq!((f.words.value.as_str(), f.reply_to.value.as_str()), ("The link never finished uploading.", "you@example.com"));
    let preview = f.preview.expect("the report, gathered");
    assert!(preview.starts_with("What happened:\nThe link never finished uploading.\n\nReply to: you@example.com\n\n"), "{preview}");
    assert!(preview.contains("shroom 0.1.18 · macOS 15.3.1 · Apple M2 · 16 GB"), "{preview}");
    assert!(preview.contains("This Mac: owns its space"), "{preview}");
    assert!(preview.contains("~/Applications/shroom.app") && preview.contains("<your space>"), "{preview}");
    assert!(!preview.contains("jane") && !preview.contains("example-owner"), "nothing cleaned reaches the disclosure");
    assert_eq!(f.preparing, None);
}

/// Nothing typed, nothing to send: Send stays grey, and a click on it is dropped.
#[test]
fn send_waits_for_words() {
    let mut r = Run::new(general_open());
    r.set(Action::ReportProblem);
    r.send(Event::Report(ReportNews::Gathered(gathered())));
    r.field(Field::ReportWords, "   ");
    assert!(!sheet(&r.scene).buttons.last().unwrap().enabled);
    r.set(Action::SendReport);
    assert!(r.effects.is_empty());
}

/// Esc ≡ Cancel: back on General, nothing sent.
#[test]
fn cancel_sends_nothing() {
    let mut r = ready_to_send();
    r.key(Surface::Settings, Key::Esc);
    assert_eq!(faces(&r.scene), ["settings-general"]);
    assert!(r.effects.is_empty());
    assert_eq!(r.w.session.fields.settings.report, ReportFields::default(), "the words go with the sheet");
}

/// Send hands the shell the post, once: the words, the reply-to and the gathered report.
/// While it goes, Send reads Sending…, both grey, Esc is dropped, and so is typing.
#[test]
fn send_posts_once_and_holds_still_while_it_goes() {
    let mut r = ready_to_send();
    r.field(Field::ReportReplyTo, " you@example.com ");
    r.set(Action::SendReport);
    assert_eq!(
        r.effects,
        vec![Effect::Shell(ShellCmd::SendReport(Post {
            kind: Kind::Problem,
            words: "The link never finished uploading.".into(),
            reply_to: Some("you@example.com".into()),
            diagnostics: gathered(),
        }))]
    );
    assert_eq!(buttons(&sheet(&r.scene)), [("Cancel", Style::Plain, false), ("Sending…", Style::Filled, false)]);
    r.set(Action::SendReport);
    assert!(r.effects.is_empty(), "a second click sends nothing more");
    r.key(Surface::Settings, Key::Esc);
    assert!(r.ids().contains(&"report-problem"), "Esc doesn't take back a send under way");
    r.field(Field::ReportWords, "changed");
    assert_eq!(form(&r.scene).words.value, "The link never finished uploading.");
}

/// No reply-to: none is sent.
#[test]
fn an_empty_reply_to_sends_none() {
    let mut r = ready_to_send();
    r.set(Action::SendReport);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::SendReport(p)) if p.reply_to.is_none())));
}

/// Taken: the sheet closes, and General says it's sent until the window closes.
#[test]
fn sent_closes_the_sheet_and_general_says_so() {
    let mut r = ready_to_send();
    r.set(Action::SendReport);
    r.send(Event::Report(ReportNews::Sent(Kind::Problem)));
    assert_eq!(faces(&r.scene), ["settings-general"]);
    assert_eq!(general(&r.scene).sent.as_deref(), Some("Sent — thank you. If you left your email, you’ll hear back."));
    assert_eq!(r.w.session.fields.settings.report, ReportFields::default());
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert!(!r.w.session.fields.settings.sent, "the line goes with the window");
}

/// Not taken (offline, refused, switched off): the sheet stays with what was typed, says
/// so, and offers the mail address — a draft with the words and the version. Send tries again.
#[test]
fn not_sent_keeps_the_words_and_offers_the_mail_address() {
    let mut r = ready_to_send();
    r.set(Action::SendReport);
    r.send(Event::Report(ReportNews::NotSent(Kind::Problem)));
    assert!(r.ids().contains(&"report-problem"));
    let f = form(&r.scene);
    assert_eq!(f.words.value, "The link never finished uploading.");
    assert_eq!(f.failed.as_deref(), Some("That didn’t go through — check your connection and try again. Or email it to hi@shroom.am instead."));
    let mail = f.mail.expect("the mail link");
    assert_eq!((mail.label.as_str(), mail.style, mail.enabled), ("email it to hi@shroom.am", Style::Link, true));
    assert_eq!(buttons(&sheet(&r.scene)), [("Cancel", Style::Plain, true), ("Send", Style::Filled, true)]);
    r.set(Action::MailReport);
    assert_eq!(
        r.effects,
        vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::MailReport { version: "0.1.18".into(), words: "The link never finished uploading.".into() }))]
    );
    r.set(Action::SendReport);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::SendReport(_)))), "Send tries again");
    assert_eq!(form(&r.scene).failed, None);
}

/// Law 4: Report a problem asks nothing of a take, so it stays open mid-take.
#[test]
fn reporting_is_open_mid_take() {
    // Opened before the take (law 4: no window opens for one mid-take).
    let mut w = general_open();
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 30_000));
    let mut r = Run::new(w);
    assert!(general(&r.scene).report.enabled && general(&r.scene).copy_diagnostics.enabled);
    r.set(Action::ReportProblem);
    assert!(r.ids().contains(&"report-problem"));
}

/// The sheet's fields clear with the Settings window, and its question with it.
#[test]
fn the_window_closing_takes_the_sheet_and_its_words() {
    let mut r = ready_to_send();
    r.send(Event::WindowClosed(WindowKind::Settings));
    assert_eq!(r.w.session.confirm, None);
    assert_eq!(r.w.session.fields.settings.report, ReportFields::default());
}

/// A report that arrives after its sheet closed is dropped.
#[test]
fn a_late_report_is_dropped() {
    let mut r = Run::new(general_open());
    r.set(Action::ReportProblem);
    r.key(Surface::Settings, Key::Esc);
    r.send(Event::Report(ReportNews::Gathered(gathered())));
    assert_eq!(r.w.session.fields.settings.report.gathered, None);
}

// ── the cleaning ──

/// Every case the feedback page's cleaner holds to, word for word (site/clean.js).
#[test]
fn the_cleaning_is_the_pages() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../../../site/test/clean-cases.json")).unwrap();
    let mut wrong = Vec::new();
    for case in cases["cases"].as_array().unwrap() {
        let (input, want) = (case[0].as_str().unwrap(), case[1].as_str().unwrap());
        let got = report::clean_line(input);
        if got != want {
            wrong.push(format!("{input:?}\n  got  {got:?}\n  want {want:?}"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn a_windows_error_is_cut_short_and_lines_are_bounded() {
    let out = report::clean_line(&format!("  10 webview popover: {}", "x".repeat(600)));
    assert!(out.chars().count() < 200 && out.ends_with('…'), "{out}");
    assert!(report::clean_line(&"y ".repeat(1000)).chars().count() <= report::LINE_CHARS + 1);
}

#[test]
fn the_log_keeps_its_last_lines() {
    let log: String = (0..report::LOG_LINES + 50).map(|i| format!("{i} core step\n\n")).collect();
    let out = report::clean_log(&log);
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), report::LOG_LINES);
    assert_eq!(lines.last().copied(), Some(format!("{} core step", report::LOG_LINES + 49).as_str()));
}

/// The words and the report never hold the canary of a secret: the cleaning keeps a pasted
/// key-shaped run out too.
#[test]
fn a_key_shaped_run_never_reaches_the_report() {
    let line = report::clean_line("  9 cloud: answered 403 for Zm9vYmFyYmF6cXV4MTIzNDU2Nzg5MA==");
    assert!(!line.contains("Zm9v"), "{line}");
    let _ = derive;
}
