# Report a problem: a description and a cleaned log, sent by the person's own mail

*Proposal, 2026-10-16. No product code until Bagrat has seen the boards. The words on the
boards are placeholders; the copy is Bagrat's.*

**Asked** (Bagrat, 2026-10-16): a general "Report a problem". The person presses a button,
says what went wrong, and the report reaches him with the log. He also liked a simpler
"Copy diagnostics" button in Settings, carrying the version, macOS, the chip, the setup
timeline and the last ~200 lines of the log, cleaned.

**Boards** on the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg), in
the Library and Settings column under the setup boards:

- **"Settings — Report a problem and Copy diagnostics"** ([`ReportProblem.png`](report-a-problem/ReportProblem.png)):
  A, Settings · General with the new row; B, the sheet, empty; C, the sheet filled, with
  what's sent shown; D, Copy diagnostics' "Copied"; E, after Email it; F, a Mac with no mail app.
- **"Report a problem — the mail draft, the saved report, and what cleaning removes"**
  ([`ReportArrives.png`](report-a-problem/ReportArrives.png)): G, the draft as it opens;
  H, the saved report; the cleaning table.

## Today

- Settings · General's line "Something off? Tell us on GitHub — or write to hi@shroom.am."
  (`settings-general.something-off-tell-us`; N8, N48). "GitHub" opens a new public issue
  with only `shroom <version>` in its body (`runtime/os.rs` `PublicUrl::NewIssue`); the address
  opens a blank `mailto:hi@shroom.am`. No log goes anywhere, and there's no export.
- **`~/.shroom/logs/run.log`** (folder 700, file 600; rolls to `run.log.1` at 5 MB). Each line
  is the core's milliseconds since launch, then the line. There's no wall clock and no version.
  The core's trace ring (the last 512 steps, kinds and names only) is written into it on a
  panic, a breach, or a rescued take (`runtime/ring.rs`).

### Is the log free of values? Checked, 2026-10-16

All 190 `rt.log` call sites were read. **No secret is ever logged.** A paste logs only
its handle and shape (`paste_secret`), and the boot lines carry kinds only. But "kinds and
names only" isn't quite true. Some lines carry values a report must not send raw:

| What | Where | Example |
|---|---|---|
| A space's host | `member.rs` (`{host}` in about 12 lines) | `v.example-owner.workers.dev` |
| A path in the home folder | `updater.rs:151` (`app.display()`); `{e}` of file errors | `/Users/<you>/Applications/shroom.app` |
| Error text from macOS, the network or S3 | every `{e}` (recorder, uploader, library, takes…) | may hold a URL, a bucket name or a path |
| A webview's own error message | `commands.rs:47`, up to 500 characters | whatever the script error said |
| URLs, in quiet runs only | `os.rs:58`, `mod.rs:1205` ("shell would open …") | a test harness, never a release |

Recording ids are already cut to four characters (`short`). Titles, emails and keys never
appear. So a report **cleans every line** before anyone sees it. Each rule is on the board's
table and is pure, testable code in `shroom-core`:

- the home folder becomes `~`;
- the space's host becomes `<your space>`;
- any other URL becomes `<a web address>`;
- an email address becomes `<an email>`;
- a run of 16 or more hex or base64 characters becomes `<an id>`;
- a webview message is cut to 160 characters, then cleaned the same way.

## The options

shroom is never a SaaS we operate, so nothing below has a collector of ours.

| | Privacy | Friction | What Bagrat receives | Build |
|---|---|---|---|---|
| **1. Mail draft to hi@shroom.am** (`mailto:` with a body) | Good: the person sees the draft and sends it themselves | Low with a mail app; **a dead end without one** (many Macs run Gmail in a browser and never set up Mail) | The description, version and Mac, and a **short** log tail. `mailto` can't attach files (RFC 6068), and long links fail. Old reports put Apple's limit near 1,000 characters and Gmail's near 4,096 ([cocoa-dev, 2002](https://lists.apple.com/archives/cocoa-dev/2002/May/msg01177.html); [codeforboston/maple#1964](https://github.com/codeforboston/maple/issues/1964)). Nothing current or authoritative turned up, so a probe on the hosted Mac picks the budget | S |
| **2. Save a report file and show it in Finder** | Good: the person picks where it goes and attaches it | Higher: save, then attach | Everything: the full 200 lines | S |
| **3. Copy diagnostics to the clipboard** | Good: one click, pasted anywhere | Low, and it works with no mail app (webmail, chat, a GitHub issue) | Everything, wherever it's pasted | XS |
| **4. The owner's own Worker** | Fine | — | **Nothing.** Reports would land in the owner's space, not with the developer. A member's report would go to someone else's account. Rejected | M |
| **5. A public GitHub issue with the log** | Weakest: public forever, and cleaning can still miss something | Needs a GitHub account | Everything, in public. Keep it only as the door the person chooses, with **no log prefilled**. The person can paste one | XS (today's link stays) |

## Recommended: 1 + 2 + 3 in one sheet, and Copy diagnostics beside it

- **Where:** Settings · General. "Something off?" becomes a row with two buttons, **Copy
  diagnostics** and **Report a problem…**. Under it, "Or tell us in public, on GitHub."
  The popover and the tray stay as they are: the popover reaches it through its Settings button.
- **The sheet** (like Uninstall's): a line saying what's added and what never is, then the
  description box, then **"See exactly what's sent"**. That opens the whole cleaned text, word
  for word, as OSS trust calls for. The buttons are **Save full report…** (a link), **Copy**,
  Cancel and **Email it** (the default; ⌘Return, since Return types a new line). Email it
  is grey until something is typed.
- **Email it** opens a draft to hi@shroom.am: the subject `shroom <version> — a problem`, then
  the description, the Mac line and as many of the last log lines as fit the budget
  (about 1,800 characters encoded, until the probe says otherwise). A last line says the full
  report can be attached if asked. The sheet closes, and Settings says the draft is in the
  mail app. **When macOS has no mail app for `mailto:`** (`LSCopyDefaultApplicationURLForURL`
  answers none), nothing opens. The sheet stays, and its line points to Copy. Nothing is copied
  without a click (law 10).
- **Save full report…** opens the save panel, so no file appears unasked. The default name is
  `shroom report <date>.txt` in Downloads. The file is then shown in Finder.
- **Copy diagnostics** puts the same report on the clipboard, without a description, and says
  "Copied" for a moment, like the page does.
- **The report** (one builder, three exits):
  - the version, the macOS version, the chip and the memory;
  - the setup timeline: the date and the hours from first step to wired, from the log, when
    it's still there;
  - this Mac's role, as counts only: owner or member, and how many Macs;
  - the last 200 lines of `run.log` (into `run.log.1` when the current file is shorter),
    cleaned.
- **Law 4 (the quiet take):** Report a problem stays available mid-take. It is read-only, and
  the mail app opening doesn't touch the take.

To build it, the log needs **one more boot line**: `boot: shroom <version> · <UTC wall clock>`.
Without it, the timeline has no dates and a report can't say which version wrote which lines.

## The spec change (provisional, ledger N53)

- `settings-general`:
  - `settings-general.something-off-tell-us` is replaced by the row label
    `settings-general.something-off`, the buttons `copy-diagnostics` and `report-a-problem`,
    and the line "Or tell us in public, on GitHub.";
  - two status lines, `draft-ready` and `no-mail-app`, and the moment `copied`.
- A new state, **`report-problem`**: the sheet over Settings · General. Its said-lines are
  `title`, `line`, `what-happened` (a placeholder), `see-whats-sent`, `save-full-report`,
  `copy`, `cancel` and `email-it`. Its edges:
  - Email it with a mail app goes back to `settings-general` with `draft-ready`;
  - Email it with no mail app stays, showing `no-mail-app`;
  - Cancel and Esc go back;
  - Copy stays, showing "Copied".
- N8's "two doors" stays: hi@shroom.am is still the mail door, and GitHub the public one.

The said-lines and edges go into `machine.json` with the build, after Bagrat's go. This PR
adds only the provisional ledger row.

## Estimate

About **1½ days** of session work:

- the report builder and cleaner in `shroom-core`, with spec tests first (½ day);
- the sheet and the Settings row in Svelte, and the gallery states (½ day);
- the shell (`mailto`, the no-mail-app check, the save panel, the clipboard, the boot line),
  plus the hosted-Mac probe that sets the mail budget with Mail and with Gmail as the
  default (½ day).

Then one release.
