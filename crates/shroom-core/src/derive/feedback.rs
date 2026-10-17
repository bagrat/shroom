//! The Send feedback window (N60): the words, the reply-to, the log box and “See exactly
//! what's sent”, then Cancel and Send — and, once shroom.am took it, the sent face with Done.
//! With the box ticked, Send waits for the report the shell gathers; unticked, the report is
//! the version alone and Send waits for nothing but words.

use crate::copy;
use crate::report::{self, Diagnostics, Kind};
use crate::scene::{Action, Button, FeedbackBody, FeedbackSent, Style, TextField, Toggle, Window};
use crate::world::{FeedbackFields, WindowKind, World};

pub fn derive(w: &World) -> Option<Window<FeedbackBody>> {
    let door = w.session.windows.get(&WindowKind::Feedback)?;
    if !w.window_visible(WindowKind::Feedback) || !w.provisioned() {
        return None;
    }
    let f = &w.session.fields.feedback;
    let sending = f.sending;
    let what = goes(w, f);
    let send = if sending {
        Button::new(Action::SendFeedback, copy::sending(), Style::Filled).enabled(false)
    } else {
        Button::new(Action::SendFeedback, copy::send(), Style::Filled).enabled(what.is_some() && !f.words.trim().is_empty())
    };
    let failed = f.failed && !sending;
    let sent = f.sent.then(|| FeedbackSent {
        title: copy::feedback_sent(),
        line: copy::feedback_sent_line(),
        done: Button::new(Action::CloseFeedback, copy::done(), Style::Filled),
    });
    let body = FeedbackBody {
        title: copy::send_feedback(),
        line: copy::feedback_line(),
        words: TextField { placeholder: copy::feedback_placeholder(), value: f.words.clone(), enabled: !sending, edits: f.words_edits },
        reply_label: copy::report_reply_label(),
        reply_to: TextField { placeholder: String::new(), value: f.reply_to.clone(), enabled: !sending, edits: f.reply_edits },
        log: Toggle { action: Action::FeedbackLog(f.without_log), label: copy::include_log(), on: !f.without_log, enabled: !sending },
        log_note: copy::include_log_note(),
        disclosure: copy::report_disclosure(),
        preview: what.as_ref().map(|d| report::preview(Kind::Feedback, &f.words, Some(&f.reply_to), d)),
        preparing: what.is_none().then(copy::report_preparing),
        failed: failed.then(copy::report_failed),
        mail: failed.then(|| Button::new(Action::MailFeedback, copy::report_mail(), Style::Link)),
        buttons: vec![Button::new(Action::CloseFeedback, copy::cancel(), Style::Plain).enabled(!sending), send],
        sent,
    };
    // Return types a new line in the words (⌘Return is the webview's Send); on the sent
    // face it's Done. Esc is Cancel, but not while it goes.
    let ret = f.sent.then_some(Action::CloseFeedback);
    let esc = (!sending).then_some(Action::CloseFeedback);
    Some(Window { id: "send-feedback", faces: vec![], body, ret, esc, activates: door.asked })
}

/// What goes with the words: the gathered report with the box ticked (none until it's
/// gathered), the version alone without it.
pub fn goes(w: &World, f: &FeedbackFields) -> Option<Diagnostics> {
    if f.without_log { Some(Diagnostics::version_only(&w.launch.version)) } else { f.gathered.clone() }
}
