//! Send feedback (spec N60): the popover header's first icon, its own window, the log box
//! (ticked when it opens), what the shell is asked to post, and the sent face.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::{Effect, PublicUrl, ShellCmd};
use shroom_core::event::{Event, Field, Key, ReportNews, Surface};
use shroom_core::report::{self, Diagnostics, Kind, Post};
use shroom_core::scene::{Action, FeedbackBody, PopoverBody, Style};
use shroom_core::time::Mono;
use shroom_core::world::*;
use support::*;

trait Drive {
    fn tap(&mut self, action: Action) -> &mut Self;
    fn field(&mut self, field: Field, value: &str) -> &mut Self;
}

impl Drive for Run {
    fn tap(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Feedback, action, rev: 0 })
    }
    fn field(&mut self, field: Field, value: &str) -> &mut Self {
        self.send(Event::Field { field, value: value.into() })
    }
}

fn body(r: &Run) -> FeedbackBody {
    r.scene.feedback.as_ref().expect("the Send feedback window").body.clone()
}

fn buttons(b: &FeedbackBody) -> Vec<(&str, Style, bool)> {
    b.buttons.iter().map(|b| (b.label.as_str(), b.style, b.enabled)).collect()
}

fn gathered() -> Diagnostics {
    Diagnostics {
        shroom: "0.1.18".into(),
        macos: "15.3.1".into(),
        chip: "Apple M2".into(),
        memory: "16 GB".into(),
        setup: "2026-10-12 10:06 UTC key · 10:14 recording allowed · 10:15 wired".into(),
        role: "owns its space".into(),
        log: report::clean_log("  1200 updater staged 0.1.18 (/Users/jane/Applications/shroom.app)"),
    }
}

fn ready_at(version: &str) -> World {
    let mut w = ready();
    w.launch.version = version.into();
    w
}

/// The popover's Send feedback clicked: the window open, nothing gathered yet.
fn opened() -> Run {
    let mut r = Run::new(ready_at("0.1.18"));
    r.click();
    r.act(Action::OpenFeedback);
    r
}

/// The window open, its report gathered, words typed.
fn ready_to_send() -> Run {
    let mut r = opened();
    r.send(Event::Report(ReportNews::Gathered(gathered())));
    r.field(Field::FeedbackWords, "Copy the link the moment I stop.");
    r
}

// ── the door ──

/// The header holds four icons: Send feedback first, then Library, Settings and Quit (N60).
#[test]
fn the_header_offers_send_feedback_first() {
    let mut r = Run::new(ready());
    r.click();
    let Some(PopoverBody::Config(c)) = r.scene.popover.as_ref().map(|p| &p.body) else { panic!("config") };
    let footer: Vec<_> = c.footer.iter().map(|b| (b.label.as_str(), b.action.clone(), b.style)).collect();
    assert_eq!(
        footer,
        [
            ("Send feedback", Action::OpenFeedback, Style::Quiet),
            ("Library", Action::OpenLibrary, Style::Quiet),
            ("Settings", Action::OpenSettings, Style::Quiet),
            ("Quit shroom", Action::Quit, Style::Quiet),
        ]
    );
}

/// Its click closes the popover and opens the window, asked for, and asks the shell to
/// gather the report for the log box and the disclosure.
#[test]
fn the_icon_opens_the_window_and_gathers_the_report() {
    let r = opened();
    assert!(r.w.session.popover.is_none(), "the popover closes");
    assert_eq!(r.arm.edge, Some(("config", "send-feedback")));
    assert_eq!(
        r.effects,
        vec![Effect::Shell(ShellCmd::GatherReport { role: "owns its space".into() }), Effect::Shell(ShellCmd::OpenFeedback)]
    );
    let win = r.scene.feedback.as_ref().expect("the window");
    assert_eq!(win.id, "send-feedback");
    assert!(win.activates, "the person asked for it");
    assert!(r.scene.dock, "law 6: a window of shroom's is open");
    let b = body(&r);
    assert_eq!(b.title, "Send feedback");
    assert_eq!(b.line, "An idea, something that bugged you, something that broke — it goes straight to the person who makes shroom.");
    assert_eq!((b.words.placeholder.as_str(), b.words.value.as_str()), ("What’s on your mind?", ""));
    assert_eq!(b.reply_label, "Your email, if you’d like an answer (optional)");
    assert_eq!((b.log.label.as_str(), b.log.on, b.log.enabled), ("Include shroom’s recent log", true, true), "ticked when it opens");
    assert_eq!(b.log_note, "Its version, your Mac’s model and its recent log, cleaned — never your videos, links, names or keys.");
    assert_eq!(b.disclosure, "See exactly what’s sent");
    assert_eq!((b.preview.as_deref(), b.preparing.as_deref()), (None, Some("Putting it together…")));
    assert_eq!(buttons(&b), [("Cancel", Style::Plain, true), ("Send", Style::Filled, false)]);
    assert_eq!(b.sent, None);
}

/// The icon lives in the ready popover only: never mid-take (law 4).
#[test]
fn there_is_no_door_mid_take() {
    let mut w = ready();
    w.take = Some(take(Phase::Recording, 30_000));
    w.session.popover = Some(Door::asked(Mono(100)));
    let mut r = Run::new(w);
    assert!(!r.scene.offers(Surface::Popover, &Action::OpenFeedback));
    r.act(Action::OpenFeedback);
    assert!(r.scene.feedback.is_none());
}

/// A second click on the icon brings the open window forward as typed — nothing is
/// gathered again and nothing is cleared.
#[test]
fn the_icon_again_keeps_what_was_typed() {
    let mut r = ready_to_send();
    r.click();
    r.act(Action::OpenFeedback);
    assert_eq!(r.effects, vec![Effect::Shell(ShellCmd::OpenFeedback)]);
    assert_eq!(body(&r).words.value, "Copy the link the moment I stop.");
}

// ── the form ──

/// With the box ticked, Send waits for words and for the report; the disclosure then shows
/// everything that goes.
#[test]
fn send_waits_for_words_and_the_report() {
    let mut r = opened();
    r.field(Field::FeedbackWords, "Copy the link the moment I stop.");
    assert_eq!(buttons(&body(&r))[1], ("Send", Style::Filled, false), "the report isn't gathered yet");
    r.send(Event::Report(ReportNews::Gathered(gathered())));
    let b = body(&r);
    assert_eq!(buttons(&b)[1], ("Send", Style::Filled, true));
    assert_eq!(b.preparing, None);
    let preview = b.preview.expect("what goes");
    assert!(preview.starts_with("Feedback:\nCopy the link the moment I stop.\n\nshroom 0.1.18 · macOS 15.3.1 · Apple M2 · 16 GB"), "{preview}");
    assert!(preview.contains("(~/Applications/shroom.app)"), "the log, cleaned: {preview}");
}

/// Send posts once, as feedback: the words, the reply-to and the gathered report.
#[test]
fn send_posts_feedback_with_the_log() {
    let mut r = ready_to_send();
    r.field(Field::FeedbackReplyTo, " you@example.com ");
    r.tap(Action::SendFeedback);
    assert_eq!(
        r.effects,
        vec![Effect::Shell(ShellCmd::SendReport(Post {
            kind: Kind::Feedback,
            words: "Copy the link the moment I stop.".into(),
            reply_to: Some("you@example.com".into()),
            diagnostics: gathered(),
        }))]
    );
}

/// Unticked, the report is the version alone: Send needs only words, and the disclosure
/// shows exactly that. Ticked again, the gathered report comes back.
#[test]
fn unticked_it_sends_the_words_and_the_version_only() {
    let mut r = opened();
    r.tap(Action::FeedbackLog(false));
    r.field(Field::FeedbackWords, "Love it.");
    let b = body(&r);
    assert!(!b.log.on);
    assert_eq!(b.preparing, None, "nothing to wait for");
    assert_eq!(b.preview.as_deref(), Some("Feedback:\nLove it.\n\nshroom 0.1.18"));
    assert_eq!(buttons(&b)[1], ("Send", Style::Filled, true));
    r.tap(Action::SendFeedback);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::SendReport(p)) if p.kind == Kind::Feedback && p.diagnostics == Diagnostics::version_only("0.1.18"))));
    let mut r = opened();
    r.tap(Action::FeedbackLog(false)).tap(Action::FeedbackLog(true));
    assert_eq!(body(&r).preparing.as_deref(), Some("Putting it together…"));
}

/// While it goes, Send reads Sending…, everything greys, Esc and typing are dropped.
#[test]
fn it_holds_still_while_it_goes() {
    let mut r = ready_to_send();
    r.tap(Action::SendFeedback);
    let b = body(&r);
    assert_eq!(buttons(&b), [("Cancel", Style::Plain, false), ("Sending…", Style::Filled, false)]);
    assert!(!b.words.enabled && !b.reply_to.enabled && !b.log.enabled);
    r.tap(Action::SendFeedback);
    assert!(r.effects.is_empty(), "a second click sends nothing more");
    r.key(Surface::Feedback, Key::Esc);
    assert!(r.scene.feedback.is_some(), "Esc doesn't take back a send under way");
    r.field(Field::FeedbackWords, "changed");
    assert_eq!(body(&r).words.value, "Copy the link the moment I stop.");
}

/// Taken: the window says so and offers Done, its Return; Done closes it.
#[test]
fn sent_says_so_and_done_closes() {
    let mut r = ready_to_send();
    r.tap(Action::SendFeedback);
    r.send(Event::Report(ReportNews::Sent(Kind::Feedback)));
    let sent = body(&r).sent.expect("the sent face");
    assert_eq!((sent.title.as_str(), sent.line.as_str()), ("Sent — thank you.", "If you left your email, you’ll hear back."));
    assert_eq!((sent.done.label.as_str(), sent.done.style), ("Done", Style::Filled));
    assert_eq!(r.scene.ret(Surface::Feedback), Some(Action::CloseFeedback));
    r.key(Surface::Feedback, Key::Return);
    assert!(r.scene.feedback.is_none());
    assert_eq!(r.w.session.fields.feedback, FeedbackFields::default());
    assert!(!r.scene.dock, "law 6: no window, no Dock");
}

/// Not taken: the words stay, the line says so and offers the mail address — a draft with
/// the words and the version. Send tries again.
#[test]
fn not_sent_keeps_the_words_and_offers_the_mail_address() {
    let mut r = ready_to_send();
    r.tap(Action::SendFeedback);
    r.send(Event::Report(ReportNews::NotSent(Kind::Feedback)));
    let b = body(&r);
    assert_eq!(b.words.value, "Copy the link the moment I stop.");
    assert_eq!(b.failed.as_deref(), Some("That didn’t go through — check your connection and try again. Or email it to hi@shroom.am instead."));
    assert_eq!(b.mail.as_ref().map(|m| (m.label.as_str(), m.style)), Some(("email it to hi@shroom.am", Style::Link)));
    assert_eq!(buttons(&b), [("Cancel", Style::Plain, true), ("Send", Style::Filled, true)]);
    r.tap(Action::MailFeedback);
    assert_eq!(
        r.effects,
        vec![Effect::Shell(ShellCmd::OpenUrl(PublicUrl::MailReport { version: "0.1.18".into(), words: "Copy the link the moment I stop.".into() }))]
    );
    r.tap(Action::SendFeedback);
    assert!(r.has(|e| matches!(e, Effect::Shell(ShellCmd::SendReport(_)))), "Send tries again");
    assert_eq!(body(&r).failed, None);
}

/// Cancel and Esc close the window and drop the words; nothing is sent.
#[test]
fn cancel_and_esc_send_nothing() {
    let mut r = ready_to_send();
    r.key(Surface::Feedback, Key::Esc);
    assert!(r.scene.feedback.is_none());
    assert!(r.effects.is_empty());
    let mut r = ready_to_send();
    r.tap(Action::CloseFeedback);
    assert!(r.scene.feedback.is_none());
    r.click();
    r.act(Action::OpenFeedback);
    assert_eq!(body(&r).words.value, "", "a new window starts empty");
}

/// ✕ mid-send: the window goes; the answer finds no one waiting.
#[test]
fn closed_mid_send_the_answer_is_dropped() {
    let mut r = ready_to_send();
    r.tap(Action::SendFeedback);
    r.send(Event::WindowClosed(WindowKind::Feedback));
    assert!(r.scene.feedback.is_none());
    r.send(Event::Report(ReportNews::Sent(Kind::Feedback)));
    assert!(r.scene.feedback.is_none());
    assert_eq!(r.w.session.fields.feedback, FeedbackFields::default());
}

/// Each door hears only its own answer: Report a problem's never lands here, nor this one there.
#[test]
fn each_form_hears_only_its_own_answer() {
    let mut r = ready_to_send();
    r.tap(Action::SendFeedback);
    r.send(Event::Report(ReportNews::Sent(Kind::Problem)));
    assert!(body(&r).sent.is_none());
    assert!(r.w.session.fields.feedback.sending);
}

/// A window opened before a take stays open through it (law 4 covers only what shroom
/// would newly show).
#[test]
fn an_open_window_stays_through_a_take() {
    let mut w = ready();
    w.session.windows.insert(WindowKind::Feedback, Door::asked(Mono(100)));
    w.take = Some(take(Phase::Recording, 30_000));
    let r = Run::new(w);
    assert!(r.scene.feedback.is_some());
}
