# Send feedback from inside the app

*Proposal, 2026-10-17. No product code until Bagrat has picked a design. The new words on
the boards are copy proposals, for him.*

**Asked** (Bagrat, 2026-10-17): make sending feedback easier from inside the app. Feedback is
most useful from people who have installed shroom. It is easier to send from the app than
from the web form, and the log can go along without anyone attaching it by hand. This covers
the backlog's "Report a problem, more reachable". He had suggested putting it beside Library,
Settings and Quit in the popover.

## Today

- **Settings · General → Report a problem…** (N53, B180): a sheet with the words, an optional
  reply-to, "See exactly what's sent" and Send. It always carries the cleaned report: the
  version, macOS, the chip and memory, the setup timeline and the last 200 log lines. It
  posts to `shroom.am/report` as kind `problem`.
- **shroom.am/feedback**: the testers' five questions. A log is optional and is cleaned in
  the browser. It posts as kind `feedback`.
- The endpoint already takes `kind: "feedback"` with `from: "app"`. The log is an optional
  field. Only the message's "Sent from" line assumes the app's door is Report a problem.

## Boards

On the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg), in the Library
and Settings column, at its foot:

- **A: a fourth icon in the popover, and its own window** (the pick).
  [`FeedbackA.png`](in-app-feedback/FeedbackA.png). A speech bubble before Library · Settings
  · Quit opens a small Send feedback window. It has the words, the optional email, an
  **Include shroom's recent log** box (ticked) with a line on what goes and what never goes,
  and "See exactly what's sent". After Send, the window says it was sent and offers Done. Two
  doors share one form: Settings' Report a problem… stays as built, kind `problem` with the
  log always included. The popover's door sends kind `feedback`.
- **B: one door, one window, two kinds.** [`FeedbackB.png`](in-app-feedback/FeedbackB.png).
  The entry is a line of words at the popover's foot: "Something to tell us? Send feedback".
  The window has a Feedback · A problem switch. Settings' Report a problem… opens the same
  window on A problem, and its sheet goes.
- **C: inside the popover, no window.** [`FeedbackC.png`](in-app-feedback/FeedbackC.png). The
  icon turns the popover itself into a short form, the way the quit question does. After
  Send, the popover is ready again and shows the thank-you line.

- **A's icon, eight options** ([`FeedbackIcons.png`](in-app-feedback/FeedbackIcons.png)): Bagrat chose A on 2026-10-17 and asked to see more icons. Each option is drawn in the header in dark and light, at real size and at 3×. The pick is 2, the bubble with lines: at 16 px it reads as a message rather than a blank shape, and it doesn't look like Library's frame. The envelope reads as mail and the paper plane as share, which a recording app already means.

`make-boards.py` writes the three boards, `make-icons.py` the icon sheet, and `render.mjs` renders their PNGs.

## Why A

- **Reachable where people are.** The popover is the one surface everyone opens. An icon
  beside its three siblings is what Bagrat suggested, and it costs no height.
- **Room to write, and it survives a click.** A window doesn't close when the person clicks
  into another app. C's form in the popover is narrow, and it hangs on the popover staying
  open (or on a kept draft).
- **The log goes by default, in plain sight.** The box is ticked, and it says what goes and
  what never goes. "See exactly what's sent" shows the report word for word. Unticking it
  leaves the words and the version. The person still sends nothing until they press Send
  (law 10).
- **Report a problem is left as it is.** It is ratified, built and tested. A keeps one form
  component and tags each door's kind. B merges the two but changes ratified copy and the
  Settings flow, for one less entry.
- **The same pipe.** `POST shroom.am/report`, kind `feedback`, `from: "app"`. The inbox
  subject reads "Feedback, from the app". The only Worker change is "Sent from" naming the
  door. Nothing new is stored.

The icon is only in the popover's ready face, like its neighbours, so it never shows mid-take.

## Copy proposals (for Bagrat)

| Where | Words | New? |
|---|---|---|
| Icon's name (hover, VoiceOver) and window title | Send feedback | Same as the site's header label (N55) |
| Window line | An idea, something that bugged you, something that broke — it goes straight to the person who makes shroom. | New |
| Placeholder | What's on your mind? | New |
| Email label | Your email, if you'd like an answer (optional) | Ratified (Report a problem) |
| Log box | Include shroom's recent log | New |
| Under the box | Its version, your Mac's model and its recent log, cleaned — never your videos, links, names or keys. | New, drawn from Report a problem's ratified line |
| Disclosure | See exactly what's sent | Ratified |
| Sending, failed | Sending… · That didn't go through — … | Ratified |
| Sent | Sent — thank you. / If you left your email, you'll hear back. / Done | Ratified line split in two; Done new |
| B only | Feedback · A problem; Something to tell us? Send feedback | New |
| C only | It goes straight to the person who makes shroom.; Your email, for an answer (optional); Back | New, shortened |

## Once picked

Spec first: a `send-feedback` window state with its said-lines, its entry from `config`'s
header, and the ledger row. Then tests, the app, and the Worker's "Sent from" line.
