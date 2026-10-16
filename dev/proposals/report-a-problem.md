# Report a problem and feedback: one endpoint on shroom.am, mailing only Bagrat

*Proposal, 2026-10-16. No product code until Bagrat has seen the boards. The words on the
boards are placeholders. The copy, and the six questions, are Bagrat's.*

> **Direction (Bagrat, 2026-10-16, relayed by the orchestrator).** He didn't much like a mail
> draft opening for the person to send. He liked a small endpoint on the shroom.am site Worker
> that takes a report and emails it to his own inbox. This proposal is rebuilt around that.
> The first draft's mail-first design survives only as a fallback, and among the rejected
> options below.
>
> **Ruled (Bagrat, 2026-10-16, relayed by the orchestrator):** he approved the plan, both
> halves: the endpoint and /feedback first, the app's button for 0.1.18. That ruling covers
> the exception below. The six questions are his; the in-app entry is in Settings only for now.
>
> **The exception.** Until now, shroom has run nothing that receives anything from a
> user. This endpoint would be the first: a small receiving service on shroom.am. It stores
> nothing, and it only ever sends to his inbox. It is still an **exception to “never a SaaS
> we operate”** (PRINCIPLES) and to the privacy page's “It sends nothing to us”. Both lines
> would change with it. N8 had parked “the tiny opt-in proxy” with the telemetry track; this
> unparks a narrower one: only on a person's Send, and only words and a cleaned log.

**Asked** (Bagrat, 2026-10-16): a general “Report a problem”. The person says what went wrong,
and it reaches him with the log. He also asked for “Copy diagnostics” in Settings: the
version, macOS, the chip, the setup timeline and the last ~200 lines of the log, cleaned.
Then a feedback page for testers.

## Boards

On the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg):

- **“Settings — Report a problem and Copy diagnostics, sent through shroom.am”**, in the
  Library and Settings column ([`ReportProblem.png`](report-a-problem/ReportProblem.png)):
  - A: the Settings row, with Copy diagnostics' “Copied”;
  - B: the sheet, empty;
  - C: the sheet filled, with what's sent shown;
  - D: Sending…;
  - E: Sent;
  - F: couldn't send, with the mail link as the fallback.
- **“Report a problem — what reaches Bagrat's inbox, the shroom.am/report endpoint, and what
  cleaning removes”**, same column ([`ReportArrives.png`](report-a-problem/ReportArrives.png)).
- **“shroom.am/feedback — the testers' questions, a log, and Send; and after Send”**, in the
  Index and invite pages column ([`FeedbackPage.png`](report-a-problem/FeedbackPage.png)).
  The six questions are placeholders until their words are in.

## Today

- Settings · General says “Something off? Tell us on GitHub — or write to hi@shroom.am.”
  (`settings-general.something-off-tell-us`; N8, N48). GitHub opens a public issue with only
  `shroom <version>` in its body (`runtime/os.rs`). The mail link opens a blank draft.
- No log goes anywhere, and there's no export. The privacy page says the app “sends nothing to
  us: no telemetry, no analytics, no crash reports. Its logs stay on your Mac.”
- **`~/.shroom/logs/run.log`** (folder 700, file 600) rolls to `run.log.1` at 5 MB. Each line
  starts with the core's milliseconds since launch. It holds no wall clock and no version.
  The trace ring (the last 512 steps, kinds only) is written into it on a panic, a breach or
  a rescued take (`runtime/ring.rs`).

### Is the log free of values? Checked, 2026-10-16

All 190 `rt.log` call sites were read. **No secret is ever logged.** A paste logs only its
handle and shape (`paste_secret`), and the boot lines carry kinds only. But the log isn't
kinds and names only. These are values a report must not send raw:

| What | Where | Example |
|---|---|---|
| A space's host | `member.rs` (`{host}` in about 12 lines) | `v.example-owner.workers.dev` |
| A path in the home folder | `updater.rs:151` (`app.display()`); the `{e}` of file errors | `/Users/<you>/Applications/shroom.app` |
| Error text from macOS, the network or S3 | every `{e}` (recorder, uploader, library, takes…) | may hold a URL, a bucket name or a path |
| A webview's own error message | `commands.rs:47`, up to 500 characters | whatever the script error said |
| URLs, in quiet runs only | `os.rs:58`, `mod.rs:1205` (“shell would open …”) | a test harness, never a release |

Recording ids are already cut to four characters (`short`). Titles, emails and keys never
appear.

So **every line is cleaned before anyone sees it**, by the rules in the board's table:

- the home folder becomes `~`;
- the space's host becomes `<your space>`;
- other URLs become `<a web address>`;
- emails become `<an email>`;
- a run of 16 or more hex or base64 characters becomes `<an id>`;
- a webview message is cut to 160 characters, then cleaned the same way.

The rules run in three places, each tested against one shared fixture of cases:

- in the app (Rust, `shroom-core`);
- in the feedback page (the same rules in JS);
- once more in the Worker, in case a sender skipped them.

## The endpoint: `POST shroom.am/report`

- **Body:** JSON with these fields; anything else is refused.
  - `kind`: `problem` or `feedback`;
  - `words`: the description;
  - `answers`: the feedback page's questions, numbered;
  - `reply_to`: optional; an email the person typed so a reply can reach them;
  - `from`: `app` or `page`;
  - `versions`: shroom, macOS, the chip and the memory;
  - `setup`: the timeline;
  - `log`: up to 200 cleaned lines.
- **Checks, in order. Any that fails answers 4xx and sends nothing.**
  1. The size: 64 KB at most. Two hundred lines is about 24 KB.
  2. The shape: known fields only.
  3. The rate. This uses the Workers rate-limit binding
     ([docs](https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/)),
     GA, with a period of 10 or 60 s and counts kept per Cloudflare location. It allows a few
     posts a minute per IP, plus a ceiling for all senders together.
  4. Turnstile, for the web form only. The app can't run it without loading Cloudflare's
     challenge script in a webview. Its posts rely on the other checks.
  5. An off switch: a variable `REPORTS=off` answers 503, and the app and the page then
     point to hi@shroom.am.
- **Sending:** the `send_email` binding
  ([Email Routing: send emails from Workers](https://developers.cloudflare.com/email-routing/email-workers/send-email-workers/)).
  - It sends one message from `report-sender@shroom.am` to Bagrat's inbox, with Reply-To set to
    `reply_to` when one was given.
  - The cleaned log goes as an attached `report.txt`.
  - **His address is a Worker secret (`REPORT_TO`), never in the repo.** The binding is
    declared with no address. In production, such a binding can send only to addresses
    verified in Email Routing, so the secret can't aim it anywhere else.
- **Kept: nothing.** No KV, R2 or D1 is bound. Request logs stay off (`observability: false`,
  as today). The rate-limit counters are Cloudflare's own and short-lived. A report lives
  only in the inbox it was sent to.
- **Probed here, 2026-10-16:** in Miniflare 5 (the repo's), a `send-email` binding sends an
  `EmailMessage` built from `cloudflare:email`. One with `allowedDestinationAddresses`
  refuses any other address (“email to other@example.com not allowed”). Not probed: a live
  send, which needs Bagrat's account.
- **What Bagrat sets up once, in his account** (a credential and an account change, so his
  to do or approve):
  - Email Routing is already on for shroom.am (hi@shroom.am routes through it). Check that
    his inbox is a *verified destination*.
  - `wrangler secret put REPORT_TO` for the shroom-site Worker.
  - A Turnstile widget for shroom.am: its site key goes in the page, its secret in
    `TURNSTILE_SECRET`.
  - Check that the deploy token (Workers Scripts: Edit, Workers Routes: Edit) can deploy a
    Worker with a `send_email` binding and a `ratelimits` binding. If it can't, the token
    gains the one permission wrangler names.

## The entry points

1. **shroom.am/feedback** (the web form; site only, so no app release is needed).
   - The testers' six questions come first.
   - Then an optional log. In 0.1.18 or newer it's pasted from Copy diagnostics. On an older
     shroom it's chosen as a file: `~/.shroom/logs/run.log`, where ⌘⇧. shows hidden
     folders in the picker.
   - The log is cleaned in the browser, with “See exactly what's sent”, before anything leaves.
   - Then an optional reply-to, and Send, with Turnstile running invisibly.
   - After Send, a thank-you page. A refusal keeps the form filled and says so in one line.
2. **Report a problem… in the app** (Settings · General).
   - “Something off?” becomes a row with **Copy diagnostics** and **Report a problem…**.
     “Or tell us in public, on GitHub.” stays below as the public option.
   - The sheet has the description, an optional reply-to, “See exactly what's sent”, Save full
     report…, Cancel and **Send**. Send is the default (⌘Return) and stays grey until
     something is typed.
   - Send posts to `shroom.am/report`. Success closes the sheet with a line in Settings.
   - On a failure (offline, refused, switched off), the sheet keeps its text and says so.
     Send tries again, and a link opens a mail draft to hi@shroom.am as the fallback. That's
     the only place mail survives.
   - Dev builds never post to shroom.am: a quiet run logs “would send”. A dev build posts only
     to a local stand-in, as a dev build's space does.
   - **Law 10:** nothing is copied without a click.
   - **Law 4:** the sheet stays available mid-take. Sending doesn't touch the take.
3. **Later, optionally:** “Send feedback…” in the app, posting the page's questions as
   `kind: feedback`.

To build the app half, the log needs **one more boot line**:
`boot: shroom <version> · <UTC wall clock>`. Without it, the setup timeline has no dates.

## The options

| | Privacy | Friction | What Bagrat receives | Build |
|---|---|---|---|---|
| **A. Endpoint on the shroom.am Worker → his inbox** (chosen direction) | Good. The person sees every word first, nothing is stored, and only his inbox receives it. **But it is a receiving service of ours: an exception for him to rule on** | Lowest: one Send | Everything: the words, the answers, the Mac, the full cleaned log, a reply-to | M |
| B. Mail draft to hi@shroom.am (`mailto:`) | Good | A mail app must open. **It's a dead end on Macs with none** (webmail users) | Only a short log tail. `mailto` can't attach files (RFC 6068), and long links fail: old reports put the limit near 1,000 characters for Apple Mail ([cocoa-dev, 2002](https://lists.apple.com/archives/cocoa-dev/2002/May/msg01177.html)) and 4,096 for Gmail ([codeforboston/maple#1964](https://github.com/codeforboston/maple/issues/1964)) | S |
| C. Save a report file, then attach it to an email | Good | Highest: save, find, attach | Everything | S |
| D. Copy diagnostics to the clipboard | Good | Low, but it's only half a report: it still has to be sent somewhere | Everything, wherever it's pasted | XS |
| E. The owner's own Worker | Fine | — | **Nothing.** Reports would land in the owner's space, not with the developer. A member's would land in someone else's account | M |
| F. A public GitHub issue carrying the log | Weakest: public forever | Needs a GitHub account | Everything, in public | XS |

**Kept:**
- **A** for both entry points;
- **D** as Copy diagnostics, which also feeds the web form;
- **C** as Save full report…;
- **B** as the fallback when A can't be reached;
- **F** only as the person's own door, with nothing prefilled but the version.

## The spec change (provisional, ledger N53)

- **`settings-general`:** `something-off-tell-us` is replaced by:
  - the row label `something-off`;
  - the buttons `copy-diagnostics` and `report-a-problem`;
  - the line “Or tell us in public, on GitHub.”;
  - `copied` and the status line `report-sent`.
- **A new state, `report-problem`** (the sheet over Settings · General).
  - Its said-lines: `title`, `line`, `what-happened` (a placeholder), `reply-to`,
    `see-whats-sent`, `save-full-report`, `cancel`, `send`, `sending` and `couldnt-send`.
  - Its edges:
    - Send goes to `report-sending`;
    - from there, accepted goes back to `settings-general` with `report-sent`;
    - refused or offline goes back to `report-problem` with `couldnt-send`;
    - Cancel and Esc go back.
- **The site:** `/feedback` and its thank-you page join the site's surfaces, with the
  privacy-page change.
- **N8** gains the third way: a report through shroom.am, beside the issue link and the
  address. **N48** is unchanged.
- **PRINCIPLES and the privacy page:** a line on the exception, worded by Bagrat after he
  rules.

The said-lines, the states and the site surfaces go into `machine.json` with the build, after
Bagrat's go. This PR adds only the provisional ledger row.

## As built in the app (b)

The sheet follows the board, with two changes from building it:

- **Save full report… left the sheet.** The app has no save panel today, and Copy
  diagnostics carries the same report. A failed send offers the mail draft, and the row's
  Copy diagnostics stays behind the sheet.
- **Copy diagnostics reads “Copied” for a moment,** as Copy link does, instead of a
  separate line beside it.

Two of the core's law checks now say what they cover:
- law 9's colours apply to sheets that destroy something (`NOT_DESTRUCTIVE` in
  `tests/common/laws.rs` and in the windows gallery). Send destroys nothing, and is filled
  as an ordinary answer is.
- law 15's scan leaves out the disclosure's `preview`: it quotes the person's words and
  log as they go, as data, like a field's `value`.

The app's screenshots are `app-sheet-dark.png` and `app-sheet-failed-light.png`, from the
windows gallery.

## Estimates

- **(a) The endpoint and /feedback (site only):** about **1 day** of session work.
  - The Worker route: checks, the shared cleaner in JS with its fixture, MIME, and
    `send_email`;
  - the page and its thank-you page, with Turnstile;
  - tests in `site/test`, with the Miniflare probe as an integration test;
  - the privacy-page change.

  Then Bagrat's one-time setup (the secret, the Turnstile widget, the verified destination),
  and a deploy through `site.yml`.
- **(b) The in-app sheet:** about **1 day**, for 0.1.18.
  - The report builder and the Rust cleaner, with spec tests first (½ day);
  - the sheet, the Settings row and the gallery states, plus the shell: the POST, the save
    panel, the clipboard, the boot line, and the dev-build fence (½ day).

  It depends on (a) being live.
