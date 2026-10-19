<!-- Generated from spec/machine.json by tools/render-machine-md.py. Don't edit: change the JSON and re-run. -->

# The machine

The machine: every state, edge, rule, said-line and law of shroom v1 — the spec (DECISIONS B133). Sessions edit it; MACHINE.md and the canvas's machine boards are generated from it.

162 states · 338 edges · 26 rules · 679 said-lines · 16 laws · 81 ledger rows.

Status: **ratified** is Bagrat's; **provisional** is designed or built and awaits him (each cites its row: `#n` is a DEVIATIONS row of docs/DECISIONS.md, `Bn` a decision, `Nn`/`Un`/`Pn` a ledger row below). `SM:n` in older citations is a line of the retired spec text, kept in history/spec/UX-STATEMACHINE.md.

## Contents

- [Region 1 — acquisition → first launch](#region-1)
- [Region 2 — the cloud setup wizard](#region-2)
- [Region 3 — the permissions chapter and the tray](#region-3)
- [Region 4 — stop → the link → the recipient](#region-4)
- [Region 5 — the tail: library, settings, updates](#region-5)
- [Region 6 — members and the join machine](#region-6)
- [Rules](#rules) · [Groups](#groups) · [Laws](#laws) · [The ledger](#the-ledger) · [Surfaces](#surfaces) · [Absent from v1](#absent-from-v1) · [Scaffold flags](#scaffold-flags) · [Notes](#notes) · [Events](#events)

## Notation

States are kebab-case, stable, scaffold-grade names; events are written in bold, [guards] in brackets, and said-lines in quotes. Every region carries its failure classification and resume semantics — the POC's wizard discipline generalized: attempt → classify → route to the fix → auto-poll → resume (POC §6) — and state is derived from probes, never stored as a step counter (UX-STATEMACHINE.md L92-98). Status keys used across the fixture: the ledger's When column carries a date for ratified rows and “· autonomous — awaiting ratification” for N29–N42 (designed 2026-09-30, L22-29); in the inventory R = ratified by Bagrat and D = designed in the autonomous sitting, awaiting ratification (= provisional), with faces that are row facts or copy variants folded into their state per N18 (L1666-1669); said-lines are drafts unless their section is marked copy ratified (L6-9). Fixture provenance: ledger entries are the table rows at L38-90 in table order (P17 at L38 … N42 at L90; N17 sits after N19), title = the Question cell and decision = the Decision cell, both verbatim markdown source; law rules are the markdown source with line wraps joined by single spaces; inventory entries are rows L1673-1709 in order, states split on top-level “ · ” with backticks dropped (non-scaffold descriptive faces kept verbatim, e.g. “source view”, “delete sheet”, “Add another Mac”), status given as “normalized | verbatim cell”; surfaces are rows L1715-1720 with may_do = the Lives | Opens | Rules cells joined by “ | ”; absent-list items (L1724-1734, split on “ · ”) and scaffold flags (L1740-1762, markdown source) each carry a two-space-separated [Lnnn] source suffix; amends/amended_by entries are “ID (relation)” strings covering every explicit amends / amended / refined / reversed / replaces / extends / settles / resolves marker in the ledger, recorded on both sides (non-ledger targets such as “Region 4 beat 6” kept as text). Spec observations for the test author: N31 says it replaces “the merged expired/used row (N18)”, but N18's ledger roster as written lists no such row (the only other mention is the N31 section heading, L1530); P1, P5, P8, P11, P13 (absent list) and P12 (N33) are cited but have no row in this ledger (only P17 does); the state inventory carries no row for countdown's or config's sub-faces beyond those listed, and “stopping” (R) has no ledger citation.

<a id="region-1"></a>

## Region 1 — acquisition → first launch

*Region 1 — acquisition → first launch*

### `landing`

web-page · web-page · **ratified** · board: Landing · cites P17, N6, N7, N11, U6, U3, B140, N55, N58, N59, N61, N70

shroom.am, a static page with one job: take a curious visitor to a pasted command, with no false notes. Two columns on a laptop (N59). Left: the shroom mark and wordmark, large; the headline on one line and its line under it; one thin price bar, 0 to 30 hours to scale, as wide as the headline, $0 for the first tenth and $1.50 a month at its end; then seven terms with green ticks. Right: one box, its top level with the wordmark's baseline and its bottom with the last term's: the hero, shroom recorded with shroom (“2 min”, shroom.am's own file, playing in place: a still of the editor after the take's cuts with a play button and the length, the video under it, no caption; N58, N61, N70), a small Setup walkthrough button under it that opens “How to set up and use shroom” over the page (N70), a cut, and Install under that (Terminal, Homebrew and Coding agent tabs, Terminal first; each command with [ Copy ]; Read it first; the system-requirements line). No header: the page's foot, at the bottom of the first screen under the left column, holds Send feedback, GitHub, Questions and limits, Privacy and Terms, then the visits line. On a phone it stacks, terms before the video and install last: the brand, the headline and its line, the bar (its labels short: “$0 / 3h video”, “$1.50 / 30h”), the terms, the box (the hero, Setup walkthrough, then install), the foot, its links wrapping.

**Entry.** A visitor arrives at shroom.am through one of three entry arrows. (1) Direct: HN, README or word of mouth; a cold visitor who needs the pitch. (2) The recipient loop: they watched a shroom video and clicked the player page's mark (Region 4; whether the mark exists = U3); a warm visitor who already saw the product work. (3) An agent fetching the BYO-agent skill (Region 6's overlay). Also reached by going back from the source view.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Record. Share. Pay-as-you-go.” | title | ratified | Hero punchline, first line, bold. Bagrat's own wording (B140, 2026-10-14), replacing “Screen recordings without the monthly seat.” |
| “No monthly seat, sometimes completely free” | line | ratified | Hero punchline, second line, with no closing period, as he wrote it (B140). |
| “$0 · about three hours of video” | other | ratified | The price bar's left label on a laptop: one thin bar from 0 to 30 hours, true to scale, its first tenth lit (the free part). '$0' is bold, in frost. It replaces the first of the two price tiles, which read as two plans. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “$0 /month — about three hours of video” · N59 |
| “$1.50 /month · about thirty hours” | other | ratified | The price bar's right label on a laptop, under its end. '$1.50 /month' is bold. Words, not numerals, on the landing; the welcome deliberately uses numerals (Law 16, N12). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “$1.50 /month — about thirty hours” · N59 |
| “$0 / 3h video” | other | ratified | The price bar's left label on a phone. '$0' is bold, in frost. Numerals here, against Law 16's words-on-the-landing, by his own wording: it must fit on one line beside the right label. Bagrat's phone note, 2026-10-17 (N59): the long label wrapped at 390 wide. · N59 |
| “$1.50 / 30h” | other | ratified | The price bar's right label on a phone. '$1.50' is bold ($1.50, not his “$1.5”, to match the laptop's label). Bagrat's phone note, 2026-10-17 (N59): the long label wrapped at 390 wide. · N59 |
| “Your videos live in your own Cloudflare account” | line | ratified | Term 1 of 7, under the price bar; green tick. 'your own Cloudflare account' is bold. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Your videos live in your own Cloudflare account — we never hold them.” · N59 |
| “You pay only Cloudflare, for the storage” | line | ratified | Term 2 of 7. 'only Cloudflare' is bold. Drops “No account with us, no subscription.” from the page (the search description keeps it). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “You only ever pay Cloudflare. No account with us, no subscription.” · N59 |
| “Invite your team with a link” | line | ratified | Term 3 of 7. 'Invite your team' is bold. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Your team just accepts an invite and records — no Cloudflare setup for them.” · N59 |
| “No price per person — the bill counts video, not people” | line | ratified | Term 4 of 7, new: the bill doesn't grow with the team. 'No price per person' is bold. Draft (a) of three on the board; Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · N59 |
| “Delete old videos and pay less” | line | ratified | Term 5 of 7. 'Delete old videos' is bold. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Delete old videos and pay less — storage is the whole bill.” · N59 |
| “About ten minutes of guided setup, once” | line | ratified | Term 6 of 7. 'ten minutes' is bold. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “You set Cloudflare up once, in a guided setup — about ten minutes.” · N59 |
| “Open source — every line on GitHub” | line | ratified | Term 7 of 7, moved from the trust strip, which is gone. 'Open source' is bold. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Open source — Every line on GitHub — the app, the installer, the player.” · N59 |
| “Install in one line” | title | ratified | The install block's heading, bold, inside the box under the video, the three tabs at its right. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · N59 |
| “Terminal” | button | ratified | The install block's first tab, chosen when the page opens: the install line with its Copy. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Or without it:” · N52, N59 |
| “Homebrew” | button | ratified | The install block's second tab: the Homebrew line with its Copy (N52's line, now a tab). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “With Homebrew:” · N52, N59 |
| “Coding agent” | button | ratified | The install block's third tab: the agent line with its Copy, then Read it first and the ask-first line. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Or ask your coding agent” · N59 |
| “curl -fsSL https://shroom.am/install \| sh” | other | ratified | The one install command, set as code with its Copy: the Terminal tab, the one shown when the page opens (N59). |
| “brew install --cask bagrat/shroom/shroom && open ~/Applications/shroom.app” | other | ratified | The Homebrew tab's line (N49, N52), set as code with its Copy. The app is opened by its path: Launch Services may not know it by name the moment brew returns (B175). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · N49, N52, N59 |
| “Copy” | button | ratified | The Copy icon inside each command's end (law 10: a click reads “Copied” beside a tick). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · N59 |
| “Read it first” | line | ratified | Under the install line (and the agent line): a link alone. It opens the same address as readable source (/install), or the page the agent reads (/agent). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Read it first — the same address opens as readable source.” · N59 |
| “Your agent asks you before it runs anything.” | line | ratified | Beside Read it first under the agent line, in the Coding agent tab. The agent page's frame in a browser keeps its own line (landing.this-is-what-your). Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “Read it first — the same address shows what your agent will read. Your agent asks you before it runs anything.” · N59 |
| “For macOS 14 or newer, Apple silicon.” | line | ratified | The sentence shape is ratified. The floor ('14') and the Intel stance are scaffold-time facts (SCK matrix): fill in the facts, don't reword. · was “For macOS 14 or newer, Apple silicon and Intel.” · B7, B44, copy table, B134 |
| “Send feedback” | link | ratified | The first link of the page's foot (it was the header's, beside GitHub); it opens shroom.am/feedback (N53). Ratified by Bagrat, 2026-10-17 (N55); moved by N59. · N55, N59 |
| “Send feedback · GitHub · Questions and limits · Privacy · Terms” | line | ratified | The page's foot: one row of links under the left column, at the bottom of the first screen (after the content when it runs longer); the visits line under it. The header is gone, so Send feedback and GitHub live here. “Trouble, questions” is gone: it opened /feedback, as Send feedback does. The other shroom.am pages keep their header and footer. Bagrat approved the board “shroom.am redesign — B8-box”, 2026-10-17 (N59). · was “shroom · Trouble, questions” · N55, N59 |
| “shroom — Record. Share. Pay-as-you-go.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The landing's browser title and link-preview title · copy table |
| “Screen recordings that live in your own Cloudflare account. No account with us, no subscription.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The landing's search description · copy table |
| “This is what your coding agent reads when you ask it to install shroom — word for word.” | line | ratified | one case the spec didn't print, so it varies by case — the copy table: The landing's agent door (`landing`, Install block, below the install line) and the agent page's frame in a browser. **Ratified by Bagrat** (2026-10-14) · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Terminal / Homebrew / Coding agent | other |  | `landing` | The install block's tabs choose which command shows; Terminal is chosen when the page opens. No address changes (N59). |
| Copy | secondary |  | `landing` | Copies the command of the tab that's showing. Law 10: it reads “Copied” beside a tick for a moment. |
| Read it first | text-link |  | `source-view` | Opens the same URL (shroom.am/install) as readable source: one URL, two audiences. In the Coding agent tab it opens shroom.am/agent. |
| GitHub link | text-link |  | `landing` | Opens GitHub, outside shroom. The page's foot, after Send feedback (N59). |
| Send feedback | text-link |  | `landing` | Opens shroom.am/feedback, the feedback page (N53), which the spec draws no state for. The page's foot, first (N55, moved by N59). |
| watch the hero | other |  | `landing` | The hero plays in place on the landing: the still fades and the video plays under it; the page never navigates away (N58, N61, N70). |
| Setup walkthrough | secondary |  | `landing` | Opens “How to set up and use shroom” over the page and plays it (the hero pauses); Esc, Close or a click outside it closes it. The page never navigates away (N70). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `source-view` | side | Read it first | ratified |  |
| `installing` | flow | run the install command | ratified |  |

### `source-view`

web-page · web-page · **ratified** · a face of `landing` · cites N11, P17

"Read it first" opens shroom.am/install, the same URL the command fetches, as plain text: the install script itself. The header comment block is ratified copy (N11). Inline comments stay technical but honest. Bound at scaffold: /install never redirects, and the temp download lands under ~.

**Entry.** Read it first is clicked on the landing: a text view of the script at the same URL.

| Said | Role | Copy | Note |
|---|---|---|---|
| “# shroom installer<br>#<br># You're reading exactly what will run — this address serves one file,<br># and this is it. Four things, in order:<br>#<br>#   1. Download the shroom app for your Mac.<br>#   2. Check it against its published fingerprint — if anything is off,<br>#      nothing gets installed.<br>#   3. Place it in ~/Applications — no admin password, nothing outside<br>#      your home folder.<br>#   4. Open it.<br>#<br># Run it again anytime — that's also the update: same four steps, newer<br># shroom. If shroom is in the middle of a recording, the script stops<br># and changes nothing.<br>#<br># Run it with --uninstall to remove shroom from this Mac.<br>#<br># Trouble, questions: https://github.com/bagrat/shroom/issues/new<br># No GitHub? hi@shroom.am works too.” | other | ratified | The ratified header comment block of the install script, verbatim with its line breaks and leading spaces. In the file it directly follows '#!/bin/sh'. Inline comments elsewhere aren't ratified copy. The --uninstall line is B135's (Bagrat, 2026-10-14, on the UninstallTerminal board). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `landing` | side | back | ratified |  |

### `installing`

state · terminal · **ratified** · cites P17, N9, N10, N11, N18, N20

The install script. Internally it runs the version probe (before fetch), then fetch, checksum verify, place in ~/Applications, launch, handoff. The user experiences it as one stream of about 5 s. The pasted one-liner keeps -fsSL and fetches only the tiny script. The app download inside the script shows curl's native progress bar (N10). The script always ends by opening the app (no quarantine ⇒ no prompt). No Gatekeeper, no admin password, nothing signing-shaped on the machine. The menu-bar handoff happens later, at all-set (N20).

**Entry.** The user runs `curl -fsSL https://shroom.am/install | sh` in Terminal: from the landing's install block, or as a re-run after a net drop, a checksum failure or a finished take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Getting shroom… verified ✓” | terminal-output | ratified | Stream line 1; the ellipsis is the single character U+2026. curl's native progress bar shows during the app download (N10).; prints as “Getting shroom…”, curl's own bar, then “verified ✓” on the line after (DEVIATIONS #13); the words and their order are unchanged |
| “Installed — opening shroom now.” | terminal-output | ratified | Stream line 2. |
| “shroom is in ~/Applications but didn't open — open it from there.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, macOS won't open the copy it just placed (`install-didnt-open`) · copy table |
| “Couldn't place shroom on this Mac — check there's room for it, then run the same command again.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, this Mac can't take it (no room for the download or the app, a locked `~/Applications`, a move that fails) · copy table |
| “Nothing was changed.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, this Mac can't take it (no room for the download or the app, a locked `~/Applications`, a move that fails) · copy table |
| “This looks like it's running somewhere restricted, so nothing was installed — run the same command in Terminal.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, somewhere restricted (a coding agent's sandbox that says so, or a write “not permitted”) · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `install-net-drop` | fail | fetch exit | ratified |  |
| `install-checksum-fail` | fail | hash ≠ manifest | ratified |  |
| `install-unsupported-os` | fail | version probe, before fetch | ratified |  |
| `updating` | auto | bundle present | ratified |  |
| `install-refused-midtake` | fail | app status seam | ratified |  |
| `first-launch` | flow | app placed and launched | ratified |  |

### `install-net-drop`

failure-state · terminal · **ratified** · cites N18

The app download didn't finish. The script tells the user to check the connection and run the same command again. Re-running is idempotent.

**Entry.** Classified by the fetch's exit: the in-script app download failed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Couldn't finish the download — check your connection and run the same command again.” | terminal-output | draft | From the N18 table's 'Said-line draft' column. The section's copy ratification covers only the stream and the source header. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `installing` | retry | re-run — idempotent | ratified |  |

### `install-checksum-fail`

failure-state · terminal · **ratified** · cites N18, N8

The downloaded app didn't match its published fingerprint, so nothing was installed. The message offers N8's two issue doors. A mismatch is never installed.

**Entry.** Classified by the downloaded app's hash ≠ the published manifest.

| Said | Role | Copy | Note |
|---|---|---|---|
| “The download didn't check out, so nothing was installed. Run it once more — if it repeats, tell us: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too.” | terminal-output | draft | From the 'Said-line draft' column. The spec sets the URL as code; the backticks are stripped here. The two doors are per N8, and the issue URL may prefill title and body through query params (an N8 scaffold nicety). |
| “This release isn't all there on our side, so nothing was installed. Tell us and we'll fix it: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, a release that's ours to fix (its manifest or file answers 4xx, a malformed manifest, a zip with no shroom.app) · copy table |
| “shroom's signature didn't verify, so nothing was installed. Run the same command in Terminal — if it repeats, tell us: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, a signature that doesn't verify or isn't the release's own signer (B125) · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `installing` | retry | run it once more | ratified |  |

### `install-unsupported-os`

failure-state · terminal · **ratified** · cites N18

This Mac's macOS is below the floor: an honest stop. The version probe runs before any fetch.

**Entry.** The version probe, run before the fetch, finds macOS older than the supported floor.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom needs macOS N or newer — this Mac is on M.” | terminal-output | draft | From the 'Said-line draft' column. N = the supported macOS floor (a scaffold-time fact); M = this Mac's macOS version. |
| “shroom doesn't run on this kind of Mac yet — Apple silicon only, for now.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, no build for this kind of Mac · copy table |
| “shroom runs on a Mac — run this in Terminal on yours.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, run on a machine that isn't a Mac (a Linux box, a server, a coding agent's sandbox) · copy table |

### `updating`

state · terminal · **ratified** · cites B156, N18, N41, N9

Not a failure: the update lane. A shroom bundle is already present and no take is open, so the script quits shroom (if it's running), swaps the bundle and relaunches it. The relaunch is tray-only and silent (dock rule), and the disk-backed upload queue rides through. The script is the manual update path, forever. Designed 2026-09-30 (N41): if the command carries --join, the relaunched app opens the invite.

**Entry.** A shroom bundle is present and the app's machine-readable status seam reports no open take (not recording, not paused). The process is never sniffed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Updated shroom to 1.2.0 — opening it now.” | terminal-output | draft | From the 'Said-line draft' column. '1.2.0' is an example: the newly installed version varies. |
| “shroom 1.2.0 is up to date — opening it now.” | terminal-output | ratified | Said before anything is downloaded, when the copy in ~/Applications is the release's version and its signature still holds (B156). An open shroom is brought forward, never quit (B44) — unless an invite is coming: then it's quit and reopened to read it. '1.2.0' varies. Ratified by Bagrat, 2026-10-15 (10a and 10c approved; 10b to do the same as 10a). · B156 |
| “Repaired shroom 1.2.0 — opening it now.” | terminal-output | ratified | The latest version is here but doesn't check out (broken, or changed since it was signed): downloaded again and swapped in, as an update. '1.2.0' varies. Ratified by Bagrat, 2026-10-15 (10a and 10c approved). · B156 |
| “shroom didn't close — quit it from the menu bar, then run this again.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, an update the app won't close for · copy table |
| “Nothing was changed.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The installer, an update the app won't close for · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | quit → swap → relaunch (N9); never swap under a running process | ratified |  |
| `member-welcome` | flow | the install command carries --join: the relaunched app opens the invite — member-welcome, or join-already-here / join-switch-confirm when this Mac already has a space (N41 + Region 6 member lane, designed 2026-09-30, awaiting ratification) | provisional |  |
| `join-already-here` | fail | the install command carries --join: the relaunched app opens the invite — member-welcome, or join-already-here / join-switch-confirm when this Mac already has a space (N41 + Region 6 member lane, designed 2026-09-30, awaiting ratification) | provisional |  |
| `join-switch-confirm` | fail | the install command carries --join: the relaunched app opens the invite — member-welcome, or join-already-here / join-switch-confirm when this Mac already has a space (N41 + Region 6 member lane, designed 2026-09-30, awaiting ratification) | provisional |  |

### `install-refused-midtake`

failure-state · terminal · **ratified** · cites N9, N18

shroom is mid-take (recording or paused, since a paused take is live). The script politely refuses and touches nothing: the sacred-recording law reaches into the installer.

**Entry.** A shroom bundle is present and the app's machine-readable status seam (the agent overlay's seam) reports a take open, recording or paused. The process is never sniffed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom is in the middle of a recording — finish that take, then run this again.” | terminal-output | draft | Refusal line 1. The update-path blockquote breaks the refusal after 'again.'; the N18 table gives it as one line: "shroom is in the middle of a recording — finish that take, then run this again. Nothing was changed." The copy is draft; the section ratifies only the stream and the source header. |
| “Nothing was changed.” | terminal-output | draft | Refusal line 2 (see the note on line 1). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `installing` | retry | after the take | ratified |  |

### `first-launch`

state · window · **ratified** · cites U1, N41

The installer's launch presents shroom as a normal macOS app, with a dock icon and a centered welcome window: no hunting in the menu bar on day one. The tray mushroom appears from the first launch onward regardless. Standing rule: the dock icon shows iff a shroom window is open. When the last window closes, the app retreats to the tray and the dock icon goes away. Launches shroom does itself (the login item, the update relaunch) are silent and tray-only. A launch the user does opens the door, like a tray click (N41, designed 2026-09-30).

**Entry.** The installer placed and launched the app on a Mac with no space: a first launch.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `welcome` | flow | first launch only | ratified |  |
| `member-welcome` | flow | the install command carried --join on a fresh Mac: the app opens at member-welcome instead of welcome (Region 6 member lane, N30, designed 2026-09-30, awaiting ratification) | provisional |  |

### `welcome`

state · window · **ratified** · board: Welcome · cites U1, U4, N12, N21, N30, N41, B159, B160

The centered welcome window (U1 + U4). It says what shroom is and what it costs, names the two steps ahead, shows the pre-checked login box, and offers [ Set up my space ] [ Later ]. It is the same ratified window on first launch and on any later day, from idle-unprovisioned's tray click. Set up my space morphs this same window into the wizard: one continuous onboarding surface, never a second window. Closing it ≡ Later, so there is no trap. There is no invite line (B159, which replaces N30's door): an invite link pasted anywhere in the window (⌘V) goes to member-welcome, a Cloudflare token goes to the wizard at `token`, and anything else does nothing. Quiet at its foot, where the line was, is Uninstall shroom… (B160): a Mac with no space has no Settings, so the welcome is where it uninstalls.

**Entry.** Any of: a first launch on a Mac with no space (the installer's launch); the tray click in idle-unprovisioned (N21); or, designed 2026-09-30 (N41), a launch the user did themselves while unprovisioned, which acts as a tray click.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom records your screen and hands you a link to share — no account with us, no subscription. Your videos live in your own Cloudflare storage: free for about three hours of video, and pennies after that — about 30 hours for $1.50 a month.” | line | ratified | First paragraph. The source wraps are joined with spaces. 'shroom' is bold and 'your own' is italic. 'storage' (the landing says 'account') and the numerals ('30 hours') are deliberate two-register choices: don't make them uniform (N12). |
| “Two steps before your first video: set up your Cloudflare, and let macOS allow recording. About ten minutes, once.” | line | ratified | Second paragraph. 'set up' is N12's copy correction (it was 'connect'). |
| “Start shroom when you log in” | field-label | ratified | Checkbox label. The checkbox is visible and checked by default (☑). |
| “Set up my space” | button | ratified | The filled button (bold), placed first: the ratified welcome pair puts its filled action first (Law 8's exception). |
| “Later” | button | ratified | Second button, not filled. |
| “Uninstall shroom…” | link | ratified | Quiet text at the foot, left, where “Have an invite link?” was (B160; Bagrat, 2026-10-15). Opens uninstall-confirm over this window. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Set up my space | filled-default | ↵ | `account` | Morphs this same window into the wizard. Registers the login item iff the box is still checked (U4/N12). Return is the default per Law 8 (one default per surface; this pair's filled action comes first, a deliberate local exception to check at ratification). Law 8 comes from N38, which is awaiting ratification. |
| Later | secondary |  | `idle-unprovisioned` | Registers the login item iff the box is still checked (U4/N12). |
| ✕ | close |  | `idle-unprovisioned` | Close ≡ Later, exactly, with no special case (N12). It registers the login item iff the box is still checked. |
| Start shroom when you log in | other |  | `welcome` | Checkbox, visible and checked by default. Applied only when the panel is dismissed (either button or ✕), changeable in settings, never registered silently. |
| ⌘V (an invite link or a token, pasted anywhere) | other |  | `member-welcome` | No door shows (B159). A join link goes to member-welcome; a Cloudflare token opens the wizard at `token` with the paste already landed; anything else does nothing. |
| Uninstall shroom… | text-link |  | `uninstall-confirm` | The Uninstall sheet over the welcome (B160). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Later — or ✕-close (login item iff box checked) | ratified |  |
| `account` | flow | Set up my space — the window morphs | ratified |  |
| `token` | flow | a Cloudflare token pasted anywhere in the welcome (⌘V; no door shows, B159): the wizard opens at `token` with the paste already landed, the recovery door reached early (N30) | ratified |  |
| `member-welcome` | flow | a join link pasted anywhere in the welcome (⌘V; no door shows, B159) | provisional |  |
| `uninstall-confirm` | side | Uninstall shroom…, quiet at the welcome's foot: a Mac with no space has no Settings (B160) | ratified | B160 |

### `idle-unprovisioned`

state · tray · **ratified** · board: TrayFaces · cites U1, N21, N41

The only pre-setup state (U1). There is no menu: the tray click opens the welcome window, the same ratified window any day later, and the dock icon shows while it is open. Right-click opens a one-item native menu, Quit shroom, the only way to quit before a space exists. Before a space exists there is no capture, no library and no local recording. The icon is the plain mushroom, the same as idle-ready's. The former Record-routed context line was retired along with the menu.

**Entry.** No space exists on this Mac and no welcome window is open. Ways in: the welcome was dismissed with Later or ✕; shroom launched itself before setup (the login item), silently and tray-only; Settings' Disconnect this Mac… or Leave this space… sent the app back here (Region 5; ~/.shroom secrets wiped, the login item stays as it was); or member-welcome's Later, with the invite remembered (Region 6, N30, designed 2026-09-30).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Quit shroom” | menu-item | ratified | The one item of the right-click native menu (N21). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| tray click | other |  | `welcome` | The click is the door (N21). The dock icon shows while the window is open. |
| right-click | other |  | `idle-unprovisioned` | Opens a one-item native menu: Quit shroom. The only way to quit before a space exists. |
| Quit shroom | other |  | `closed` | shroom quits. Native menu item. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `welcome` | flow | tray click — the same window, any day (N21) | ratified |  |
| `idle-ready` | flow | Cloudflare: wired | ratified | N13, B134 |
| `member-welcome` | flow | tray click while a remembered invite still lives (defined in Region 6, N30, designed 2026-09-30) | provisional |  |
| `join-expired` | fail | tray click after the remembered invite has died: the window's join-expired face, shown once; the plain welcome thereafter (defined in Region 6, N30, designed 2026-09-30) | provisional |  |

### `uninstall-sh`

state · terminal · **provisional** · board: UninstallTerminal · cites B135, B156, N9

The install line asked to uninstall, for a Mac whose shroom won't open: `curl -fsSL https://shroom.am/install | sh -s -- --uninstall`. The same one file, never a second address. It refuses mid-take (the installer's own refusal) and while recordings never reached the space (changing nothing), quits a running shroom the way an update does, resets its screen, microphone and camera permissions while the app is still there, then removes the app, ~/.shroom and the system's caches and preferences for it. The space is never touched.

**Entry.** The install script run with --uninstall.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Removed shroom from this Mac — the app, its settings and its permissions.” | terminal-output | ratified |  |
| “Every video and link is just as it was — only this Mac changed.” | terminal-output | ratified | The second line after a removal. One line for an owner and a member alike: the script can't tell which this Mac was, and after Leave this space there is no space on it at all (CHECKPOINT §10 11c; was “Your space and every video in it are just as they were.”). Ratified by Bagrat, 2026-10-15. · B156 |
| “shroom isn't on this Mac — nothing to remove.” | terminal-output | ratified | No app in ~/Applications and no ~/.shroom. |
| “Two recordings on this Mac never reached your space — open shroom to send them or save them to Movies, then run this again.” | terminal-output | ratified | Nothing is changed. The count is in words (law 16) and varies. |
| “One recording on this Mac never reached your space — open shroom to send it or save it to Movies, then run this again.” | terminal-output | provisional | The one-take case of the line above. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `install-refused-midtake` | fail | mid-take: the app's status seam reports a take open | provisional | B135 |
| `landing` | flow | removed (the space untouched) | provisional | B135 |

#### Region 1's amendments

- **N6** (amends U6 (placement of the member story)): The landing's separate "Record together" section dissolves into the hero's team point: "Your team just accepts an invite and records — no Cloudflare setup for them." The claim stands, promoted into the hero.
- **N12** (amends U4 (login-item mechanics) and the welcome copy): ✕-close ≡ Later exactly, with no special case. Any dismissal registers the login item iff the box is still checked. Copy corrected to "set up your Cloudflare" (it was "connect"). The storage/account and numeral/word choices stand as deliberate two-register choices.
- **N20** (amends the installer's passive menu-bar handoff): The menu-bar handoff moves to `all-set`, where it is earned. The installer just opens the app, with no menu-bar hunting on day one.
- **N21** (amends U1's tray menu (Record routing to the wizard) and N1's Copy last link): In idle-unprovisioned the native tray menu is gone: the tray click opens the `welcome` window directly, and right-click gives a one-item native menu, Quit shroom. The Record-routed context line (“Your first video is one setup away — about five minutes, once.”) is retired.
- **N30** (amends the ratified welcome window (Region 1, copy ratified 2026-07-25 / N12)): Designed 2026-09-30, awaiting ratification. Adds one quiet line under the welcome's buttons, "Have an invite link?". It opens a classified paste door ("Paste your invite link"): a join link goes to `member-welcome`, a Cloudflare token goes to the wizard at `token`, and anything else gets "That doesn't look like an invite link."
- **N41** (amends Region 1's "every launch is silent" (and refines N9)): Designed 2026-09-30, awaiting ratification. Silence applies only to launches shroom does itself (the login item, the update relaunch). A launch the user does (Spotlight, Finder, the Dock, a fresh install onto a Mac that still remembers its space) acts exactly as a tray click. The `updating` lane stays silent, except that an install command carrying --join opens the invite.

#### Region 1's open notes

- Hero point count, to check at ratification: the spec calls the hero a 'six-point explanation' and the team line 'point six' (lines 54, 205, 1658). The ratified hero draws only five dashed points, and 'tiles between points three and four' matches five-point numbering. Either a point is missing or the count is wrong.
- Law 8's welcome exception is flagged in the spec: 'A deliberate local exception — check it at ratification.' The welcome pair's filled action (Set up my space) comes first rather than rightmost. Its Return default is derived from Law 8 (N38, designed 2026-09-30, awaiting ratification); Region 1 itself never names a Return key (Law 8 is lines 136-143).
- Copy status of the Terminal failure lines: the `installing` heading ratifies copy only for 'stream + source header'. The N18 table's column is headed 'Said-line draft', so install-net-drop, install-checksum-fail, install-unsupported-os, updating and install-refused-midtake lines are marked draft, including the mid-take refusal quoted in the N9 update-path block (lines 233, 254-263).
- The mid-take refusal appears twice: as two lines in the update-path blockquote (254-255) and as one line in the N18 table (263). The fixture uses the two-line form.
- The invite door on the welcome (N30) is designed, not ratified, and amends the ratified welcome. The spec says the join page's Open shroom deep link is the main road, and this paste door is its fallback (and the agent persona's) (lines 334-341).
- N41 amends Region 1's 'every launch is silent' and is awaiting ratification. A launch the user does (Spotlight, Finder, the Dock, a fresh install onto a Mac that remembers its space) acts as a tray click. This is also the rescue when a crowded menu bar hides the mushroom behind the notch (lines 300-306).
- Open question: N41 and Law 5 say an update relaunch restores whatever window was open before it. Region 1 says the script's `updating` lane relaunches tray-only and silent. Whether the restore applies to the script lane is not stated (lines 89, 128-129, 248-249, 304).
- Retired copy that must NOT appear in idle-unprovisioned: the former Record-routed context line “Your first video is one setup away — about five minutes, once.” (retired with the menu by N21, lines 352-353).
- Region 6 extends Region 1 states (lines 1462-1474). In the member lane, `installing` forwards --join to the app at launch. On a fresh Mac the app opens at `member-welcome` instead of `welcome`. If shroom is already installed, the `updating` lane runs and the relaunched app opens the invite (`member-welcome`, `join-already-here` or `join-switch-confirm`). Mid-take gives `install-refused-midtake` and the link stays unused. These are designed, not ratified.
- Law 10 ('copying is always a visible button that reads “Copied”') is a standing law, but Region 1 doesn't draw a Copied face for the landing's [ Copy ] button. No 'Copied' said-line was invented.
- The landing's page chrome is given only as '🍄 shroom wordmark left · GitHub link right.' The 🍄 is likely the mock stand-in for the logo (spec header, lines 18-20). The GitHub link's wording is not specified.
- The demo id 'b4kq9d' is illustrative ('real space; id illustrative').
- Personas: the dev walks Region 1 alone. The guided persona's agent overlays narration on the same machine; Region 6 defines the overlay, not a second machine (lines 169-170). Entry arrow 3 into the landing is the agent fetching the BYO-agent skill.
- The welcome's two-register choices are ratified as-is: 'storage' on the welcome vs 'account' on the landing, and numerals ('30 hours') on the welcome vs words ('about thirty hours') on the landing tile. Tests must not normalize them (lines 331-333, Law 16).
- There are no Fat-marker status notes in Region 1. Deliberately absent: brew and .dmg lanes (post-v1), any local recording before setup (U1), and the min-OS floor and Intel stance (scaffold-time facts) (lines 377-381).
- All apostrophes in Region 1's said-lines are straight ASCII (') in the spec; quote marks around lines are curly “ ” and are stripped. Tests should match the spec's straight apostrophes.

#### Region 1's scaffold flags

- The system-requirements sentence's shape is ratified. The macOS floor and the Intel stance are scaffold-time facts (SCK matrix): fill the facts into the ratified shape without rewording, and state them plainly on the page once known. The same floor is the N in install-unsupported-os.
- /install never redirects: architecture dispatch lives inside the script ("this address serves one file, and this is it"). The same URL serves the source view, so one URL serves two audiences.
- The temporary download lands under ~ ("nothing outside your home folder"), and the app is placed in ~/Applications with no admin password.
- The pasted one-liner keeps -fsSL and fetches only the tiny script. The app download inside the script uses curl's native progress bar (N10): no hand-rolled ticker or cursor-control code in the source people are told to read first.
- The script always ends by opening the app (no quarantine ⇒ no prompt). No Gatekeeper, no admin password, nothing signing-shaped on the machine.
- The OS version probe runs before the fetch (install-unsupported-os). A checksum mismatch is never installed. A re-run after a net drop is idempotent.
- Mid-take classification reads the app's machine-readable status seam (the agent overlay's seam, the same door) and never sniffs the process. Recording and paused both refuse, and nothing is touched.
- Update lane: quit → swap → relaunch, tray-only and silent. The disk-backed upload queue rides through. Never swap under a running process (a signature mismatch kills it on the next load; verify the exact behavior at scaffold).
- The issue link in failure copy may prefill title and body through query params, with everything visible in the URL (N8 scaffold nicety).
- The dock icon shows iff a shroom window is open. The tray mushroom appears from the first launch onward regardless. Popovers never bring the dock icon (Law 6).
- Login item (U4/N12): the checkbox is visible and checked by default. The item is registered at panel dismissal (Set up my space, Later or ✕) iff the box is still checked. Never silent; changeable in settings.
- Set up my space morphs the same window into the wizard: never a second window.
- Invite door paste field (N30, designed): validates the moment a paste lands, with no Continue button. It classifies the paste: a join link, a Cloudflare token, or anything else. The join link's shape is v.<name>.workers.dev/join/<code> (N28), and both the bare and the https:// forms are accepted (N29).
- Before a space exists there is no capture, no library and no local recording (U1). There is no .dmg lane in v1 (P17); Homebrew installs beside the install line (N49).
- The source view has no backticked scaffold-grade name. The inventory calls it just 'source view' (folded under `landing`), so the fixture uses that literal id.
- The mermaid diagram uses underscore names (install_net_drop, install_checksum_fail, install_unsupported_os, install_refused_midtake, first_launch, idle_unprovisioned). The spec's scaffold-grade names are kebab-case, and the fixtures use the kebab forms.

<a id="region-2"></a>

## Region 2 — the cloud setup wizard

*Region 2 — the cloud setup wizard (ratified 2026-07-20; per-screen copy ratified 2026-07-25; N42 additions designed 2026-09-30, awaiting ratification). UX-STATEMACHINE.md lines 385-713.*

### `account`

state · window · **ratified** · board: WizardAccount · cites U1, N13, N14, N17, N19, N45

The wizard's first screen: the plan panel (six rows, row one current) over a clean fork asking whether the user already has a Cloudflare account. The plan itself pre-warns both gates (box 1 = the sign-in gate, the card row = the card gate). The plan panel persists over every wizard state; its rows are recorded here, as the spec quotes them at this screen.

**Entry.** Wizard entered (welcome → Set up my space, or idle-unprovisioned → tray click → welcome → Set up my space) and nothing is probeable yet: no valid stored token. Also the re-entry point after a relaunch during the signup trip (the virgin fork).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space — set up once, yours forever:” | title | ratified | Plan panel heading; the panel spans the whole onboarding and appears on every wizard state. |
| “→ A Cloudflare account (free — they'll want a verified email or a Google sign-in)” | line | ratified | Plan row 1, as rendered at account (current, → in Frost). Glyph varies by derived state: → current · ✓ done (green) · • still ahead. Parentheticals drop off rows that are behind you. |
| “• One access token for shroom (a pre-filled page — review, create, paste)” | line | ratified | Plan row 2 (still ahead at account). |
| “• A credit card for storage (their rule, not a charge — free up to about three hours of video)” | line | ratified | Plan row 3 (still ahead). Spec soft-wraps this row across two blockquote lines; joined with a single space. The visible card row is the card-gate pre-warn (N14). |
| “• Your link name (part of every link you share — v.yourname.workers.dev)” | line | ratified | Plan row 4. Spec italicises 'yourname' (markdown *yourname*); asterisks stripped. When this row is current (name), the parenthetical sheds only its example. |
| “• Wiring it up (shroom does this part)” | line | ratified | Plan row 5. N45: drawn last, after the permissions row. |
| “• Letting macOS allow recording (microphone, camera and screen — a one-time yes each)” | line | provisional | Plan row 6 (N17) — per-Mac where rows 1-5 are per-space; it is the permissions chapter. N43: the camera joins the chapter. N45: drawn fifth, before the wiring row. |
| “Do you already have a Cloudflare account?” | line | ratified | The fork question in the current row's nest. |
| “Not yet — take me there” | button | ratified |  |
| “I have one” | button | ratified | Filled default, rightmost per macOS. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Not yet — take me there | secondary |  | `account-signing-up` | Opens Cloudflare's signup in the browser. |
| I have one | filled-default | ↵ | `token` | Explicitly 'the filled default, rightmost per macOS' (line 444) — the dev-majority path. Box 1 ticks at proceed — the plan's one self-reported tick. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `account-signing-up` | flow | Not yet — opens signup | ratified |  |
| `token` | flow | I have one — box 1 ticks at proceed | ratified |  |

### `account-signing-up`

state · window · **ratified** · cites N19, N14, N45

After 'Not yet' opens Cloudflare's signup, the account row's nest acknowledges the browser trip (reverses N13's no-morph). Account creation stays self-reported.

**Entry.** User chose Not yet — take me there at account; Cloudflare's signup is open in the browser.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Cloudflare's signup is open in your browser. Finish there, then confirm your email from the message they send — they make no token until you do. I'll wait.” | line | ratified | N45 (replaces “…finish there, I'll wait.”). Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |
| “Take me there again” | button | ratified |  |
| “I signed up” | button | ratified | Filled continue. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Take me there again | secondary |  |  | Reopens the signup tab — no transition. |
| I signed up | filled-default | ↵ | `token` | Box 1 still ticks at proceed (the wizard cannot observe the dashboard session). Spec says 'filled continue'; return_default inferred from Law 8 (one filled button per surface) — not stated in Region 2. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `account` | flow | Relaunch mid-trip — re-enters at the virgin fork (nothing probeable before a token exists) | ratified |  |
| `token` | flow | I signed up — box 1 ticks at proceed | ratified |  |

### `token`

state · window · **ratified** · board: Wizard · cites N14, N18, N42, N45

Row two current. Instruction line, the filled Create token in Cloudflare button (the only browser trigger, serving first visit and every retry), then the paste field. Never auto-opens the browser. Paste validates the moment it lands — no Continue button. Pipeline: shape/prefix check → verify endpoint → probe by use. On pass the token is written to ~/.shroom/ (mode 600) before wiring begins.

**Entry.** Box 1 ticked (I have one / I signed up), or a re-entry/repair re-probe finds no valid stored token, or a door into the wizard at token (Reconnect here on an owner-class Mac without the token).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space runs on one access token. The page comes pre-filled — review what it grants, two narrow permissions and nothing more, create the token, and paste it here.” | line | ratified | Soft-wrapped in spec; joined with a single space. |
| “Create token in Cloudflare” | button | ratified | Filled. |
| “Paste your token” | placeholder | ratified | Paste field placeholder. |
| “Cloudflare won't make it yet? Confirm your email first — their message is in your inbox.” | line | ratified | N45: quiet, under the paste field — only on the sign-up path (reached by I signed up), until a paste lands. Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Create token in Cloudflare | filled-default | ↵ |  | Opens the pre-filled token page on dash.cloudflare.com (name + Workers Scripts Write, Workers R2 Storage Write); no state transition. return_default inferred from Law 8 (the surface's one filled button); spec silent on Return while the paste field holds focus. |
| Paste your token | field |  |  | Validates the moment a paste lands; no Continue button exists. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token-garbled` | fail | shape check | ratified |  |
| `token-wrong-powers` | fail | probe by use | ratified |  |
| `token-pick-account` | side | several accounts | provisional |  |
| `token-other-account` | fail | repair mood — a token for another account | provisional |  |
| `card` | flow | paste validates — probe pass runs | ratified |  |
| `wizard-offline` | fail | network drop during verify/probe (any step — N18) | ratified |  |
| `settings-space` | flow | the token saved: this Mac holds it now (Add a token here) | provisional | N33 |

### `token-garbled`

failure-state · window · **ratified** · cites N18, N14

Garbled or truncated paste caught by the pre-network shape/prefix check; rendered under the token row.

**Entry.** A pasted value fails the shape/prefix check (pre-network).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Tokens sometimes lose characters in the copy — copy it once more.” | line | draft | Appears only in the failure-classification table, which carries no copy-ratified mark (see open notes). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Paste your token | field |  | `token` | A new paste re-runs the pipeline; resume is immediate. |
| Create token in Cloudflare | filled-default | ↵ |  | Still on screen from token's render — the button serves every retry (lines 466-467); not restated in the failure row. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | retry | copy once more | ratified |  |

### `token-wrong-powers`

failure-state · window · **ratified** · cites N18, N14

A real token that lacks the needed powers, caught by probe-by-use (never by echo); the line points back at the create button already on screen.

**Entry.** Verify passes but one harmless probe per permission fails.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This token can't manage your video space — create a fresh one and paste that.” | line | draft | Only in the failure-classification table; N14 says the wrong-powers line points back at the create button. See open notes on copy status. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Create token in Cloudflare | filled-default | ↵ |  | "the create button is already on screen" (line 616). |
| Paste your token | field |  | `token` | Resume on new paste. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | retry | on new paste | ratified |  |

### `token-repair-mood`

face · window · **provisional** · a face of `token` · cites N42

The token state as the repair mood renders it when the token was revoked or expired: a redrafted line replaces the normal instruction, with the create button and paste field. Id assigned by extractor (spec gives no scaffold name for this face).

**Entry.** Repair mood re-probe finds the stored token revoked or expired; the 'One access token for shroom' row un-ticks and becomes current.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Cloudflare stopped accepting shroom's token — create a fresh one and paste it here. Nothing else needs redoing.” | line | provisional | Replaces the draft “…changed its mind about our token…”. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Create token in Cloudflare | filled-default | ↵ |  | Opens the pre-filled token page. return_default inferred from Law 8. |
| Paste your token | field |  | `token` | The paste field; runs token's pipeline. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | paste — handled by token's own pipeline (this face is token itself); what follows a valid repair paste is not drawn in the spec | provisional |  |
| `token-other-account` | fail | a fresh token pasted for a different Cloudflare account | provisional |  |

### `token-pick-account`

state · window · **provisional** · board: WizardPickAccount · cites N42

Not a failure: the verified token reaches two or more Cloudflare accounts, so the nest asks which one holds the videos before anything is created. The choice is the space's, for good. One account → this never appears.

**Entry.** Verify lists more than one account for the pasted token.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This token works for more than one Cloudflare account — which one should hold your videos?” | line | provisional | Soft-wrapped in spec; joined. |
| “Use this account” | button | provisional | Filled. |
| “✓ A Cloudflare account — Bagrat's Account” | status | provisional | Row one's fact after the choice; the account name varies. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| ◉ Bagrat's Account · ○ Acme Engineering | field |  |  | Radio list of the accounts the token reaches; names are example data. |
| Use this account | filled-default | ↵ | `card` | return_default inferred from Law 8 (one filled button per surface) — not stated in Region 2. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | Use account | ratified | N14, ARCHITECTURE §3.7, B134 |
| `card` | flow | Use this account | provisional |  |

### `token-other-account`

failure-state · window · **provisional** · cites N42

Repair mood only: a pasted token belongs to a different Cloudflare account than the space's. Never builds a second space by accident.

**Entry.** In the repair mood, the pasted token's account isn't the space's.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This token is for a different Cloudflare account than your space — create it in the account that holds v.myshroom.workers.dev.” | line | provisional | The link name varies (spec renders it in backticks — monospace formatting stripped). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Paste your token | field |  | `token` | Resume on new paste. |
| Create token in Cloudflare | filled-default | ↵ |  | Not restated for this face; inherited from token's render (the button serves every retry, lines 466-467). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | on new paste | provisional |  |

### `card`

state · window · **ratified** · board: WizardCard · cites N14

The visible gate step under the plan's card row. One probe pass runs at paste-validate (write-class probe, R2-enabled probe, subdomain GET); its gates surface here, email first then card. Existing-R2 accounts tick the row silently, seeing neither gate.

**Entry.** Paste validated and token persisted (or account chosen at token-pick-account); probe pass ran.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `gate-email` | fail | 403 8000077 | ratified |  |
| `gate-card` | fail | needs-payment class | ratified |  |
| `name` | flow | cleared — no subdomain yet (10007) | ratified |  |
| `wizard-offline` | fail | network drop during the probe pass (any step — N18) | ratified |  |
| `done` | flow | cleared — subdomain exists: the name row ticks itself, wiring starts underneath (N45) | ratified | N45 |

### `gate-email`

failure-state · window · **ratified** · cites N14, N18

Cloudflare's email-verification gate under the card row; auto-polls with a spinner (the only progress surface, no refresh affordance). Mostly greets fresh email signups.

**Entry.** Write-class probe returns 403 8000077 (numeric match). Also reachable from wiring's fallback classification if the gate closes between the card step and wiring.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Cloudflare wants your email verified before it opens storage. Check your inbox for their message — I'll notice the moment it's done and carry on.” | line | ratified | Soft-wrapped in spec; joined. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `card` | auto | auto-poll clears | ratified |  |
| `wizard-offline` | fail | network drop (any step — N18) | ratified |  |

### `gate-card`

failure-state · window · **ratified** · cites N14, N18, N42, N45

The card gate under the card row. The button opens Cloudflare's own billing page (shroom never sees payment data); a spinner marks the auto-poll watch. Also the repair mood's state when the card is removed / storage switched off.

**Entry.** R2-enabled probe returns the needs-payment class (after the email gate, if both). Repair mood: card removed, storage switched off. Also wiring's fallback classification.

| Said | Role | Copy | Note |
|---|---|---|---|
| “One step left on Cloudflare's side: adding a card switches storage on. The free tier stays free — the card is their rule, not a charge.” | line | ratified | The same ratified line in the repair mood. |
| “Add credit card” | button | ratified | Filled. |
| “Add credit card opens your Cloudflare billing page. I'll notice the card the moment it's there and carry on.” | line | ratified | N45: quiet, under the button, until it's clicked. Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |
| “Billing is open in your browser — add the card there. I'll carry on by myself.” | line | ratified | N45: quiet, under the button, once it was clicked. Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Add credit card | filled-default | ↵ |  | Opens Cloudflare's billing page (card or PayPal); no transition — the spinner watches. return_default inferred from Law 8. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `card` | auto | auto-poll clears | ratified |  |
| `wizard-offline` | fail | network drop (any step — N18) | ratified |  |

### `name`

state · window · **ratified** · cites N15, U8, N18

Row four current. The row is the ask; the URL is the field — the name is typed in place inside https://v.___.workers.dev/…. Explicit claim only (filled, Enter-equivalent, disabled until the local charset check passes); no availability pre-check. The charset guard filters, never scolds (stateless). Both endings tick the row with the fact — the plan's only outcome-bearing tick.

**Entry.** Guard: the account has no workers.dev subdomain (10007) after the card row clears. Skip path: an existing subdomain ticks the row itself and the wizard never stops here.

| Said | Role | Copy | Note |
|---|---|---|---|
| “https://v.” | field-label | ratified | URL prefix before the in-place name field (shown as `https://v.` in spec). With the “v” clash it reads with the label shroom instead. |
| “.workers.dev/…” | field-label | ratified | URL suffix after the field. Spec mock shows the typed value 'myshroom' in the field. |
| “This one's a keeper: choose a name you'll be glad to see in a year.” | line | ratified |  |
| “Claim this name” | button | ratified | Filled. |
| “Lowercase letters, numbers, and dashes.” | line | ratified | Quiet line that appears, then fades, when a character outside the set is typed (it never lands); uppercase lowercases silently. Stateless — no transition. |
| “✓ Your link name — v.myshroom.workers.dev” | status | ratified | Row fact after claim or on the skip path (pre-existing subdomain); shown on the plan from then on. The name varies; backticks in spec are monospace formatting. |
| “→ Your link name (part of every link you share)” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, the name row while current · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| https://v.[ name ].workers.dev/… (name field) | field |  |  | Return in the focused field belongs to Claim this name (Law 8). |
| Claim this name | filled-default | ↵ | `wiring` | Enter-equivalent; the only trigger (never a typing debounce); disabled until the charset check passes. A 10031 answer goes to name-taken. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `name-taken` | fail | 10031 at claim | ratified |  |
| `wizard-offline` | fail | network drop (any step — N18) | ratified |  |
| `done` | flow | Claim this name — the row ticks with the fact, wiring starts underneath (N45) | ratified | N45 |

### `name-taken`

failure-state · window · **ratified** · board: WizardName · cites N15, N18

The claimed name is taken: field edges red, the value stays put for editing, inline retry.

**Entry.** Claim returns 10031.

| Said | Role | Copy | Note |
|---|---|---|---|
| “That name's taken — try another.” | line | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| name field (value stays, red edges) | field |  |  | Return belongs to Claim this name. |
| Claim this name | filled-default | ↵ | `name` | Inline retry. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `name` | retry | inline retry | ratified |  |

### `name-v-clash`

face · window · **provisional** · a face of `name` · cites N42, U8

Not a state — a derived label. If the account already holds a Worker named v that isn't shroom's, wiring never touches it and the space's pages take the label shroom; the URL field shows the right label from the start and the row's fact says it once. Id assigned by extractor.

**Entry.** Probe pass (before the name is claimed) finds a non-shroom Worker named v in the account.

| Said | Role | Copy | Note |
|---|---|---|---|
| “✓ Your link name — shroom.myname.workers.dev (“v” was already taken in your Cloudflare)” | status | provisional | Row fact; soft-wrapped in spec, joined with a single space; inner “v” quotes are part of the line. The name varies. |

### `wiring`

state · window · **ratified** · board: WizardWiring · cites N16, N18, N3, N4, R6, N45

The setup's last step since N45, reached only when the permissions chapter finished before wiring did: a spinner and one calm line while the ~6 idempotent REST calls (private bucket, the v Worker, the Worker secret, member #1 enrollment, the subdomain's propagation — minutes on a new name) and the silent self-check finish. Wiring itself starts the moment the link name ticks and runs under the chapter, out of sight. Closing the window mid-wire is safe. In the repair mood, wiring runs again when the page or its key is gone (the click was the consent).

**Entry.** The permissions chapter is done (row six ticked) and wiring hasn't finished. Repair mood: the page (Worker) or its key is gone.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Setting up your space… switching your name on takes about a minute, one time only.” | line | ratified | The calm line of the repair mood's re-wire (Settings · Space). Since N45 the setup's last step says the line below. |
| “Almost there — switching your name on takes a few minutes the first time, and only once.” | line | ratified | N45: the setup's last step, waiting for wiring. Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `gate-email` | fail | fallback classification — the email gate closes between the card step and wiring | ratified |  |
| `gate-card` | fail | fallback classification — the card gate closes between the card step and wiring | ratified |  |
| `wizard-offline` | fail | transport failure | ratified |  |
| `space-found` | side | a space is already there | provisional |  |
| `all-set` | flow | silent self-check passes — the last step (N45) | ratified | N45 |

### `wizard-offline`

failure-state · window · **ratified** · board: WizardOffline · cites N18, N16

One offline state for the whole wizard, rendered under whichever row is current: the spinner rests, one amber line, auto-resume on reconnect. No retry affordance.

**Entry.** Transport-level failure at any wizard step.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Everything so far is saved — I'll pick up right here when you're back online.” | line | ratified | Amber line. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `wiring` | auto | auto-resume on reconnect | ratified |  |

### `wizard-unexpected`

failure-state · window · **provisional** · cites #25

The wizard's one face for a failure nothing else classifies: the call a wizard face waits on (the paste's check, the token's save, the probe pass, wiring; Settings · Space's repair mood the same) fails Refused, Unclassified, or panicked, and no answer has cleared it. The spinner rests under the current row with one amber line and Try again; the backoff keeps retrying meanwhile. Under the name row the line sits beneath the field and Claim stays live.

**Entry.** A wizard call failed in a way nothing else classifies, and no answer has cleared it. Cloudflare busy (429, 5xx) and wiring's propagation wait running out are timeouts — a calm retry, never this face.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Something unexpected got in the way — everything so far is saved. I'll keep trying.” | line | provisional | amber, under the current row |
| “Try again” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Try again | secondary |  |  | every wizard call asked again now, the window re-probed |

### `space-found`

state · window · **provisional** · cites N42, U10

A pasted token's probes find a space already there (a Mac replaced, or the owner adding a token-holding Mac): rows three to five tick in order and the wiring row's nest says this instead of the calm line; then row six becomes current — permissions are per-Mac.

**Entry.** Probes find an existing space for the pasted token's account.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Found your space — v.myshroom.workers.dev, 14 videos. This Mac is joined.” | line | provisional | Link name and video count vary (count kept as figures per Law 16). |
| “Found your space — v.myshroom.workers.dev, 1 video. This Mac is joined.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `space-found`, one video · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `done` | flow | row six becomes current | provisional |  |

### `done`

state · window · **ratified** · board: WizardPerms · cites N17, U5, N45

The resting beat that opens the permissions chapter: four ticks, the permissions row current (drawn fifth, N45), its nest previewing the choreography, and the wiring row beneath it working out of sight. Nothing auto-fires (no dialog to an empty chair). perm-preamble is fused in. Row six ticks only at the end of permissions; the menu-bar handoff lives at all-set.

**Entry.** The link name row ticks (a name claimed, or the card step found one) — wiring starts underneath (N45) — or space-found ticked rows three to five.

| Said | Role | Copy | Note |
|---|---|---|---|
| “macOS will ask three times: your microphone and your camera — one click each — then screen recording, which goes through System Settings.” | line | provisional | Spec gives this and the next line as one quote across a line break; the N17 ledger renders them as two lines. N43: the camera joins the chapter. |
| “Flip the switch next to shroom there. When macOS offers to quit and reopen the app, say yes — I'll be right where you left off.” | line | ratified |  |
| “Allow recording” | button | ratified | Filled. |
| “✓ A Cloudflare account” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, rows behind you · copy table |
| “✓ One access token for shroom” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, rows behind you · copy table |
| “✓ A credit card for storage” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, rows behind you · copy table |
| “✓ Wiring it up” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, rows behind you · copy table |
| “✓ Letting macOS allow recording” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, rows behind you · copy table |
| “• Wiring it up (shroom is doing this part now)” | line | ratified | N45: the wiring row while wiring runs under the permissions chapter — its dot a small spinner. Ratified by Bagrat, 2026-10-15 (on the canvas's setup-fixes boards). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Allow recording | filled-default | ↵ | `perm-mic` | Starts Region 3's chapter in the same window (U5). return_default inferred from Law 8 (one filled button per surface) — not stated in Region 2. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-mic` | flow | Allow recording | ratified |  |

### `space-degraded`

banner · banner · **provisional** · board: SpaceRepair · cites N42, N38, N21

The wizard's afterlife on the owner's token-holding Mac: any auth- or space-class failure surfaces as the config popover's banner under the amber dot. Start recording stays live (capture is local; takes wait and drain after repair). Members never see it (their side is space-needs-owner).

**Entry.** An auth- or space-class failure on a publish or the periodic self-check, any time after setup (diagram: from idle-ready).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space needs you — new videos wait safely on this Mac.” | banner | provisional |  |
| “Reconnect your space” | button | provisional | Filled. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Reconnect your space | filled-default |  | `repair-mood` | Filled, but a banner's own filled button is never the Return default (Law 8). Opens the Settings window on Space in the repair mood. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `repair-mood` | flow | Reconnect your space | provisional |  |

### `space-degraded-no-token-mac`

banner · banner · **provisional** · a face of `space-degraded` · cites N42, N33

The different banner an owner-class Mac without the token reads for the same break. Id assigned by extractor.

**Entry.** space-degraded's cause, on an owner-class Mac that does not hold the Cloudflare token.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space needs a fix — open shroom on the Mac you set it up on, or reconnect here with a fresh token.” | banner | provisional |  |
| “Reconnect here” | button | provisional | Not bolded in spec. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Reconnect here | secondary |  | `token` | The wizard at token, making this Mac a token-holder too. Spec does not mark it filled; banner buttons are never the Return default (Law 8). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | Reconnect here | provisional |  |

### `repair-mood`

state · window · **provisional** · board: SpaceRepair · cites N42, N36

The Settings window's Space tab in the repair mood: the plan re-probed from scratch (no stored step), whatever broke un-ticked, the first broken row current with Region 2's own state beneath it. No second repair UI exists. Id from the diagram node repair_mood.

**Entry.** Reconnect your space from the space-degraded banner.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | the token died (revoked or expired) — renders as token~repair-mood | provisional |  |
| `gate-card` | fail | storage switched off (card removed) | provisional |  |
| `wiring` | flow | the page or its key is gone — rewire | provisional |  |
| `space-storage-gone` | fail | the bucket is gone | provisional |  |
| `name-changed` | fail | the name changed | provisional |  |

### `space-storage-gone`

failure-state · window · **provisional** · cites N42

Repair mood only: the bucket is missing; the Wiring it up row un-ticks. Offers fresh storage or disconnecting this Mac.

**Entry.** Repair-mood re-probe finds the space's bucket missing.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space's storage is gone from Cloudflare — the videos that were in it can't come back. Set up fresh storage?” | line | provisional |  |
| “Set it up fresh” | button | provisional | Bold (filled) in the repair table (652); plain in the failure table (623). |
| “Disconnect this Mac…” | link | provisional | Quiet text opening the Disconnect sheet (Region 5). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Set it up fresh | filled-default | ↵ |  | Resume 'on choice'; destination not drawn in the spec. return_default inferred from Law 8. |
| Disconnect this Mac… | text-link |  | `disconnect-confirm` | Region 5 describes it as quiet text; sheet under N23's law. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `wiring` | flow | Set up fresh | provisional | N42, B52 |

### `name-changed`

failure-state · window · **provisional** · cites N42

Repair mood only: the account's workers.dev name differs from the space's, so shared links stopped opening; changing it back in Cloudflare restores them. Re-probed when the window regains focus.

**Entry.** Repair-mood re-probe finds the account's workers.dev name differs from the space's.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your link name changed in Cloudflare, so links you've shared stopped opening. Changing it back brings them back.” | line | provisional |  |

### `re-entry-as-settings`

state · window · **provisional** · cites N36, N21

Re-entry after success: the all-ticked panel is the space's settings surface — since N36 the Settings window's Space tab in its inspection mood, with Disconnect this Mac… at its foot. Name taken from the section heading (not backticked).

**Entry.** Re-entry through the gear in the config popover's footer and all probes pass.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space is ready — nothing to redo.” | line | draft | Only in the unmarked failure-classification table (and Region 5's D — N36 inspection mood). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Disconnect this Mac… | text-link |  | `disconnect-confirm` |  |

#### Region 2's amendments

- **N42** (amends U8): When the account already holds a non-shroom Worker named v, the space's pages take the label shroom instead. The name row's fact says it once. This applies to that case only; it is a derived label, not a state.
- **N42** (amends Region 2 entry arrow 4 (space-degraded, 2026-07-20: “opens at `token` only”)): Reconnect your space now opens the repair mood in Settings → Space. The plan is re-probed and the first broken row is current with its own Region 2 state (token · gate-card · wiring · space-storage-gone · name-changed · token-other-account).
- **N42** (amends repair token line draft (“…changed its mind about our token…”)): Redrafted as “Cloudflare stopped accepting shroom's token — create a fresh one and paste it here. Nothing else needs redoing.”
- **N19** (amends N13 (deliberate no-morph after “Not yet”)): account-signing-up acknowledges the browser trip (Take me there again / I signed up). Signup stays self-reported, and box 1 still ticks at proceed.
- **N14** (amends N13 (box-1 wording) and the 2026-07-20 invisible post-paste-sweep surfacing): The card content moves to its own plan row, which becomes the gate pre-warn. Gates surface visibly under the card row as gate-email then gate-card. The sweep survives as the mechanism underneath.
- **N17** (amends perm-preamble as a separate beat; the five-row plan): The plan gains a sixth row (permissions). perm-preamble fuses into done, which rests with row six current. Region 3 opens at perm-mic.
- **N18** (amends N16 (wiring-only offline)): wizard-offline generalizes the offline handling into one state for the whole wizard. It renders under whichever row is current and auto-resumes on reconnect by derived state.
- **N21** (amends Region 2 entry arrow 2 (tray menu + Record-routed context line)): The tray menu is retired. idle-unprovisioned's tray click opens welcome directly. Re-entry after success goes through the gear in config's footer.
- **N36** (amends Re-entry-as-settings surface): The all-ticked panel now lives in the Settings window's Space tab (inspection mood, with the repair mood and Disconnect this Mac… at its foot).

#### Region 2's open notes

- Copy status of token-garbled / token-wrong-powers: their lines appear only in the Failure classification table, which has no copy-ratified mark. The token screen's copy was ratified 2026-07-25, and N14 says “the wrong-powers line points back at” the create button. Marked draft to be safe; confirm whether leg 5 ratified them.
- “Your space is ready — nothing to redo.” (re-entry after success) appears only in the unmarked failure table and in Region 5's inspection mood (D — N36). Marked draft. The table row says “all ✅”, but the plan glyph for done is ✓.
- space-found's line (“Found your space — v.myshroom.workers.dev, 14 videos. This Mac is joined.”) differs from U10's ledger wording (“Found your space — 14 videos. This Mac is joined.”): N42 inserts the link name. N42 says it “gives U10's recovery line its place” but does not mark this an amendment (the spec header says amending rows say **amends** and name it), so it is not listed under amendments. Check at ratification.
- space-found entry differs between prose and diagram. Prose: “when a pasted token's probes find a space already there”, rendered in the wiring row's nest with rows three to five ticking in order. The diagram draws it as wiring --> space_found.
- space-storage-gone: Set it up fresh resumes “on choice (N42)”, but its destination is not drawn and the diagram has no outgoing edge from space_storage_gone. The button is bold in the repair table (line 652) and plain in the failure table (line 623).
- name-changed has no outgoing edge in the diagram. The prose says only “re-probe on return” / “re-probed when the window regains focus”.
- Things the diagram omits that the prose states: wizard-offline as a node (drawn only as a note, “can interpose at any step”); account-signing-up's relaunch → account; wiring's fallback to gate-email/gate-card; Reconnect here → token on an owner-class Mac without the token; Take me there again (no transition). The wizard-offline transitions on token, card, gate-email, gate-card, name and wiring were added from N18's “network drop, any step”. The spec does not list the steps, so tests should cover every step that makes a network call.
- Return defaults inferred from Law 8 (the surface's one filled button), with no explicit statement in Region 2: I signed up (account-signing-up), Create token in Cloudflare (token and its failure faces), Use this account (token-pick-account), Add credit card (gate-card), Allow recording (done), Set it up fresh (space-storage-gone). Only I have one (“the filled default”, line 444) and Claim this name (“Enter equivalent”, line 538; Law 8 names it) are explicit. Law 8 itself comes from N38, which is autonomous and awaiting ratification. The spec does not say what Return does while token's paste field has focus.
- token-other-account: the repair table names only the line. The Create token in Cloudflare button and the paste field are inherited from token's render, not restated for this face.
- The diagram's idle_ready --> space_degraded edge starts at idle-ready, which is Region 3's state. Where the banner sits in the popover (priority 2 of the N38 banner order) is defined outside this range (line 771).
- space-degraded as a state dates from 2026-07-20 (its heading: “state 2026-07-20; surface designed 2026-09-30 — N42”); only its banner surface, copy and the repair mood are N42. The inventory marks the whole row D — N42, so it is recorded as provisional.
- Plan row renders outside the first screen are not quoted, apart from the outcome-bearing facts (“✓ Your link name — …”, “✓ A Cloudflare account — Bagrat's Account”). Rows behind you drop their parentheticals, but the exact strings are not given, so tests should not assert them verbatim.
- Cross-region doors into token that this range does not define: welcome's “Have an invite link?” paste door (N30: a token → the wizard at token), and Settings Space's “Add a token here” for owner-class Macs without the token (Region 5, around line 1190). The inspection → repair-mood re-probe is described in Region 5 (line 1184).
- Region 2 has no “Fat-marker status” notes and no “check at ratification” markers. Every N42 item here (token-pick-account, the v clash, space-found, the space-degraded surface + repair mood, token-other-account, space-storage-gone, name-changed, the redrafted token line, entry arrow 4) is autonomous and awaiting Bagrat's ratification.
- The region heading says only “ratified 2026-07-20” (structure). Copy is ratified per subsection on 2026-07-25 (the plan, account, token, card, name, wiring, done).

#### Region 2's scaffold flags

- Extractor-assigned ids. These have no backticked scaffold name in the spec: name~v-clash, token~repair-mood and space-degraded~no-token-mac (faces); repair-mood (from the diagram node repair_mood); re-entry-as-settings (from the ### heading). Rename if the scaffold names them differently.
- The wizard is a window, never a popover: it has to survive the browser round-trip. Dock icon shows only while a window is open.
- No step counter. Every entry (first run, after a crash, a week later) re-probes in this order: token verify → gates → bucket → Worker → subdomain, and marks the plan from the answers. It never reprints a blank plan. All wiring calls are idempotent.
- Box 1 ticks at proceed and is self-reported: the plan's one exception. On re-entry a valid stored token re-proves it. The gates are probed, never assumed.
- Plan glyphs: → current (Frost) · ✓ done (green) · • still ahead. Nothing in the plan looks clickable, so no checkboxes. Parentheticals drop off rows that are behind you. At name, the current row sheds only its example.
- Action buttons inside nested step content sit flush-left across the wizard.
- Row six is per-Mac and rows 1-5 are per-space. A second Mac redoes only row six. The plan spans the whole onboarding.
- token never auto-opens the browser. Create token in Cloudflare is the only browser trigger, for the first visit and every retry. It opens the pre-filled dash.cloudflare.com token page (name + Workers Scripts Write, Workers R2 Storage Write).
- A paste validates the moment it lands; there is no Continue button. Pipeline order: shape/prefix check (pre-network; verify-endpoint routing by prefix) → verify endpoint → probe by use (one harmless call per permission, never by echo).
- On pass, the token goes to ~/.shroom/ (mode 600) BEFORE wiring begins. Never argv, chat or logs.
- Probe pass at paste-validate: write-class probe (403 8000077 → gate-email), R2-enabled probe (needs-payment → gate-card), subdomain GET (10007 → name). Gates surface email first, then card. Existing-R2 accounts tick the card row silently.
- Gates auto-poll under a spinner. The spinner is the only progress surface: no refresh affordance, no polling dots.
- gate-card: the card is entered on Cloudflare's own billing page (card or PayPal). shroom never sees payment data.
- wiring keeps the same gate classifications as a fallback. A gate that closes between the card step and wiring routes the same way there.
- name: claim is explicit only, never on a typing debounce. Claim this name is filled and Enter-equivalent, disabled until the local charset check passes. No availability pre-check: taken is discovered at claim. Uppercase lowercases silently. Out-of-set characters never land, and a hint appears then fades. Exact charset/length edges are scaffold-time facts. The subdomain is created by PUT under the same token.
- The as-you-type charset guard is stateless: no transition happens (N18 nuance).
- “v” clash: detected in the probe pass before the name is claimed. Wiring never touches the foreign v Worker. The label shroom is derived; this is not a state.
- Wiring: about 6 REST calls. The bucket is private from birth (r2.dev never enabled) and the v Worker is the only read path. Places the Worker's narrow R2-scoped parent token as a Worker secret. Silently enrolls the owner as member #1. The ~40 s propagation wait rides the calm line. Ends with a silent self-check: derive the S3 keys, then write/read/delete one tiny object through the real path.
- wiring has zero affordances: no cancel, no button, no stage narration. Closing the window mid-wire is safe; re-entry re-probes and resumes.
- Copy never names internals: 10007, verify-by-prefix, empty-bindings echo, the propagation poll, key derivation.
- Every failure row is a first-class state with its own render and transitions, rendered under the plan's current row.
- space-degraded classification: any auth- or space-class failure on a publish or on the periodic self-check → the config popover banner under the amber dot.
- Repair mood = the same machine. The plan is re-probed from scratch and the first broken row is current with Region 2's own state. No second repair UI. Re-wiring needs no extra confirmation: the click was the consent.
- Start recording stays live while degraded. Takes wait and drain after the repair. The stop sequence opens the page when the page still answers (N40).
- Members never see repair tooling. Their side of the same break is space-needs-owner (N34).
- token-other-account must never build a second space by accident.
- name-changed is re-probed when the window regains focus.
- Relaunching during the signup trip re-enters at the virgin fork (account).
- Typography: Region 2 copy uses straight ASCII apostrophes (') throughout (no ’; verified: 67 straight, 0 curly in lines 385-713). Ellipses are the single character …, dashes are em dashes —; the only curly quotes inside a said-line are the “v” in the v-clash row fact. Tests should match the spec's characters exactly.

<a id="region-3"></a>

## Region 3 — the permissions chapter and the tray

*Region 3 — permissions + the first recording (UX-STATEMACHINE.md lines 714-906; heading verbatim: structure ratified 2026-07-20; permissions chapter copy ratified 2026-07-25 — N17/N20; tray + `config` ratified 2026-08-21 — N21; `countdown` — N22; `discard-confirm` — N23; `paused` + tray faces — N24; `recovered-take` — N25. Not in the heading: the autonomous-sitting additions N38, N39, N41 and N42 are designed 2026-09-30 and awaiting ratification, so they are provisional.)*

### `perm-mic`

state · window · **ratified** · board: WizardPerms · cites U5, N17, N20, N43

The permissions chapter's first beat, in the onboarding window. macOS's microphone prompt appears and is answered inline. The mic resolves fully before anything else, so dialogs never stack (POC §4). The camera is asked next, in its own beat (N43). A denied mic never blocks.

**Entry.** At Region 2's `done` the user clicked Allow recording (N17: perm-preamble fused into done, so the chapter opens here). The microphone grant is unresolved (state is derived from probes, Law 1). The arm-time guard re-probes grants at every record arm and replays exactly the missing beat.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom records your voice with your video.” | other | ratified | Info.plist microphone usage string, shown inside macOS's own mic prompt. N20 ratifies 'the mic usage string' by name. |
| “shroom shows and records your camera only when you choose it.” | other | ratified | Info.plist camera usage string. Since N43 it appears in the chapter's `perm-camera` beat; the first camera pick asks only if setup didn't (a Mac set up before N43, or one that had no camera then). |
| “→ Letting macOS allow recording (microphone, camera and screen — a one-time yes each)” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The plan, row six while current. N43: the camera joins the chapter. · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| macOS's microphone prompt — allow or deny | other |  | `perm-camera` | Apple's own dialog. The spec gives no button copy of ours. Either answer resolves the mic. Then the camera (N43), or straight to perm-screen on a Mac with no camera, or one whose camera is already answered. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-camera` | flow | mic resolved — allow or deny | provisional | N43 |
| `perm-screen` | flow | mic resolved — no camera to ask | provisional | N43 |

### `perm-camera`

state · window · **provisional** · board: WizardPerms · cites N43, U5, #43

The chapter's second beat (N43): macOS's own camera prompt, answered inline, the moment the microphone's answer lands. Nothing of ours shows. A denied camera never blocks: the camera picker starts at Camera off and Camera as the source greys Start, as N39's Camera denied says (DEVIATIONS #43). Skipped on a Mac with no camera.

**Entry.** The microphone prompt the Allow recording click put to macOS was answered (allowed or denied), the camera grant is unasked, and the Mac has a camera.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom shows and records your camera only when you choose it.” | other | ratified | Info.plist camera usage string, inside macOS's own camera prompt (N43 moves the prompt here). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| macOS's camera prompt — allow or deny | other |  | `perm-screen` | Apple's own dialog. Either answer resolves the camera, and its answer chains the screen step — the Allow recording click's last half (law 11). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-screen` | flow | camera resolved — allow or deny | provisional | N43 |
| `perm-priming` | flow | camera resolved — screen recording already allowed | provisional | N43 |

### `perm-screen`

state · window · **ratified** · board: WizardPerms · cites U5, N18, N20, N42, N43

One surface at a time (N43). shroom asks macOS for screen recording. On the first ask macOS raises its own alert, whose Open System Settings opens the exact pane: shroom opens nothing itself and its line points at that alert, with a quiet door to the pane for when the alert was closed. When macOS doesn't ask (every later ask), shroom opens the exact pane itself and the ratified line holds. The switch takes effect on next launch, through macOS's own quit-and-reopen offer (the `relaunching` beat). Onboarding resumes at this chapter from derived state, with no stored counter. N42 adds a quiet 'Reopen shroom' door underneath, for when macOS's offer was declined or never came.

**Entry.** The mic and the camera are resolved (allowed or denied; the camera skipped on a Mac without one) and the screen-recording grant is not live (derived). It is re-entered after `relaunching` ('resumes right here'). It is also reached from config's `recording-off` banner: Turn it back on opens the Settings window, Space tab, with row six current at perm-screen (N38).

| Said | Role | Copy | Note |
|---|---|---|---|
| “macOS handles screen recording in System Settings — it's open. Flip the switch next to shroom, and when macOS offers to quit and reopen it, say yes. I'll be right where you left me.” | line | ratified | N20 ratifies 'perm-screen's flip-the-switch line' by name. The text ends 'left me.', not 'left off.' (the 'left off' wording belongs to done's preview, N17). The source uses straight apostrophes. Since N43, the face when macOS doesn't ask: shroom opened the pane itself. |
| “macOS is asking about screen recording now — choose Open System Settings in its window, then flip the switch next to shroom. When macOS offers to quit and reopen it, say yes. I'll be right where you left me.” | line | ratified | N43: the face while macOS's own alert is up (the first ask). shroom opens nothing itself. Ratified by Bagrat, 2026-10-14, on the canvas. |
| “Flipped it already?” | line | provisional | N42, quiet, beneath the main line |
| “Reopen shroom” | button | provisional | N42 |
| “Closed macOS's window?” | line | ratified | N43, quiet, under the alert face's line only Ratified by Bagrat, 2026-10-14, on the canvas. |
| “Open System Settings” | button | ratified | N43, quiet: opens the exact pane; the face becomes the pane's (the ratified line) Ratified by Bagrat, 2026-10-14, on the canvas. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Reopen shroom | secondary |  | `relaunching` | Quiet, beneath the main line (N42). shroom relaunches itself into `relaunching`. The spec names no filled action and no Return default on this surface. |
| flip the switch in System Settings and accept macOS's quit-and-reopen offer | other |  | `relaunching` | Done in macOS's own UI, not ours |
| Open System Settings | secondary |  | `perm-screen` | N43: the alert face's quiet door, for when macOS's alert was closed or denied (shroom stays in the pane's list either way). Opens the exact pane; the line becomes the ratified one. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `relaunching` | flow | switch flipped — say yes to quit and reopen | ratified |  |
| `perm-priming` | flow | grant live | ratified |  |

### `relaunching`

state · window · **ratified** · cites N18, N20, N42

A named beat (N18 spirit, named by N20): shroom quits and reopens so the screen grant takes effect. Onboarding then resumes at the permissions chapter from derived state. The region gives this beat no render and no said-line.

**Entry.** The screen-recording switch was flipped and the user accepted macOS's quit-and-reopen offer, or the user clicked Reopen shroom (N42).

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-screen` | auto | resumes right here — derived state | ratified |  |

### `perm-priming`

state · window · **ratified** · board: WizardPerms · cites U5, N20, N38, N45

An invisible warm-up of about 1 s, the moment the screen grant is live. Apple's second (~30-day) consent fires now, at setup, never mid-recording (POC §4; probe: the dialog is real on Tahoe and non-fatal). Since N38 (provisional), the same warm-up also runs when `config` opens and Apple's periodic re-confirm is due. macOS then asks over the popover, never inside a take. The popover may close as the dialog takes focus, and the next click finds it again.

**Entry.** Onboarding: the screen-recording grant has just become live. Everyday (N38): `config` opens while Apple's periodic re-confirm is due.

| Said | Role | Copy | Note |
|---|---|---|---|
| “If macOS double-checks about recording, approve it — that's the last one for a good while.” | line | ratified | N20 ratifies 'priming's double-check line' by name |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `all-set` | flow | warm-up clean — row six ticks | ratified |  |
| `member-all-set` | flow | row two ticks | provisional |  |
| `wiring` | flow | warm-up clean — row six ticks, wiring still running (N45) | ratified | N45 |

### `all-set`

state · window · **ratified** · board: AllSet · cites N20, N12, N21, N42, N45

The onboarding window's final face, a resting state with three doors (N20; it replaces the passive handoff). All six plan rows are green, with a closing block below the plan. Record now closes the window and opens `config` as a tray popover, so the eye lands on the menu bar. Invite someone morphs this same window into Region 6's members surface. Close, and ✕ (exactly the same as Close), is the retreat: the window closes, the dock icon goes away, and tray-only life begins.

**Entry.** `perm-priming` finished clean and row six ticked, and wiring is done (else `wiring` is the last step, N45). All six rows are green. If the mic was denied, row six ticks with the fact 'microphone off for now' (N42).

| Said | Role | Copy | Note |
|---|---|---|---|
| “You're all set — ready to record.” | line | ratified | first line of the closing block below the plan |
| “shroom lives in your menu bar now — look for the little mushroom.” | line | ratified |  |
| “Close” | button | ratified |  |
| “Invite someone” | button | ratified | N20: 'Invite someone', not 'someone else' |
| “Record now” | button | ratified | filled |
| “✓ Letting macOS allow recording — microphone off for now” | status | provisional | N42: row six's tick fact when the mic was denied. The spec writes it in the denied-mic bullet with the ✓ glyph included. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | secondary |  | `idle-ready` | leftmost. The window closes, the dock icon goes away, tray-only life. |
| Invite someone | secondary |  | `invite-born` | morphs this same window into the Region 6 members surface |
| Record now | filled-default | ↵ | `config` | Filled and rightmost. Closes the window and opens config at the tray. Return comes from Law 8 (one default per surface, and Return triggers it). The region's row doesn't restate it. |
| ✕ | close |  | `idle-ready` | ≡ Close exactly (N12's precedent) |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | Close — dock retreats | ratified |  |
| `config` | flow | Record now — the window closes, the popover opens at the tray | ratified |  |
| `invite-born` | side | Invite someone | ratified |  |

### `idle-ready`

state · tray · **ratified** · board: TrayFaces · cites N21, N25, N38, N41

Tray-only life with no menu. The tray click opens `config` directly. Right-click opens a one-item native menu, Quit shroom. The icon is the plain mushroom, identical to idle-unprovisioned's, unless something needs you (amber dot) or an update waits (Frost dot). The five faces are N38's.

**Entry.** The space is provisioned and no take is open. The onboarding window is closed, or the user retreated from all-set, or a discard ended a take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Quit shroom” | menu-item | ratified | the right-click one-item native menu (N21). It exists only in the idle states (N39). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| tray click | other |  | `config` | N21: the click is the door. A launch you did yourself counts as a click (law 5, N41). |
| Quit shroom (right-click menu) | other |  | `quit-confirm` | Quits at once when nothing is draining. While uploads drain, it asks the same way as the footer Quit (N41). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Confirm disconnect | ratified | N36, B134 |
| `space-degraded` | fail | auth-class failure → classify | provisional |  |
| `config` | flow | tray click | ratified |  |
| `recording` | flow | the recorder: started | ratified | N39, B134 |
| `recovered-take` | flow | launch finds an interrupted take — amber dot (N25) | ratified |  |
| `quit-confirm` | flow | right-click Quit shroom while uploads drain (N41; table only, not drawn) | provisional |  |
| `stopping` | flow | Get link row | ratified | N35, B134 |
| `stopped-offline` | flow | Cloudflare: reached | ratified | N40, B134 |
| `stopped-stranded` | flow | Cloudflare: reached | ratified | N40, N34, B134 |
| `settings-general` | flow | shroom://settings/agent | ratified | N62 |
| `countdown` | flow | shroom record start | ratified | N62 |

### `config`

state · popover · **ratified** · board: Main · cites N21, N22, N37, N38, N39, N41, U5, N44, #43, N56, N60, N69

A same-themed custom popover under the tray (Loom-style). It is never a native menu and never a window, so the dock stays clear. Click-away or Esc dismisses it with nothing lost. It is pre-filled with last-used values and never skipped: a confirm, not a form. Rows run top to bottom, each a picker (an icon, the pick's name, a chevron; its label is the row's accessible name, DEVIATIONS #45): Record (one source picker: displays by human name first, then Camera only), the camera (every camera here, then Camera off — a camera turns the bubble on with it, Camera off turns it off; DEVIATIONS #43), Microphone (every microphone, then No mic), Quality, then Start recording (filled, full width) with a quiet line beneath. The footer holds four icons with tooltips: Send feedback, Library, Settings, Quit (N60). The grammar is N38's (provisional): a banner slot at the top for asks, a status line above the footer for facts, and disk-low under Start. When it opens, the grant probe runs (`recording-off`), and a due monthly re-confirm runs perm-priming's warm-up. The device rules are N39's (provisional): never auto-pick an iPhone (mic or camera); a click picks the display it came from as the source, over a remembered display but never over a hand pick in this take's Record picker or Camera only (N56); a gone last-used display falls back silently to the menu-bar display; same-named displays are labeled by position; Camera as the source previews here; with a camera picked for the bubble, picking another display moves the bubble there. If the camera is denied, the camera picker shows Camera off and Camera as the source greys Start. Since N44 both pickers start with System default: it follows macOS's default input and its preferred camera (never an iPhone by itself), and a take keeps the device it started with. The owner page's Edit in shroom, for a video this Mac can't edit (N69), opens it by itself under the mushroom — the app you were in keeps the keyboard — with one line in the status line's place, said once until the popover closes.

**Entry.** Not mid-take, and the tray clicked in idle-ready (or a launch you did, or all-set's Record now, or a cancelled countdown, which returns here still armed). Popover banners stack one at a time, highest first: saved take (recovered-take / disk-full-stop), the space can't take videos (other regions), recording-off, update-ready (Region 5).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Record” | field-label | ratified |  |
| “LG UltraFine” | menu-item | ratified | An example only. Displays appear by their human name from macOS, which varies by hardware (§5.2 hard requirement). |
| “Built-in Display” | menu-item | ratified | An example of a display's human name, which varies |
| “Camera only” | menu-item | ratified | Unquoted in the spec, but named as the picker's option below the displays · was “Camera” · #43 |
| “Camera” | field-label | ratified | the camera picker's accessible name, not shown on the row; “Show yourself” named a switch until the camera became a pick, and Bagrat renamed it Camera on 2026-10-16 · was “Show yourself” · #43, #45, B181 |
| “Microphone” | field-label | ratified |  |
| “Quality” | field-label | ratified |  |
| “Normal” | other | ratified | Segmented control option, bolded in the spec. '(1080p)' is the spec's gloss, and whether it renders isn't stated. |
| “2K” | other | ratified | segmented control option |
| “4K” | other | ratified | segmented control option |
| “Start recording” | button | ratified | filled, full width |
| “Starts after 3-2-1 — click anywhere to cancel.” | line | ratified | Quiet, beneath Start. '3-2-1' uses ASCII hyphens. |
| “Library” | tooltip | ratified | footer icon tooltip |
| “Settings” | tooltip | ratified | footer gear tooltip, the door to the Region-5 inventory |
| “Quit shroom” | tooltip | ratified | footer icon tooltip |
| “allowed in System Settings” | line | ratified | A quoted fragment: with the mic denied, config shows the mic off with this quiet hint. Its full rendered wording isn't given. It sits in the permissions-chapter bullets, under that subsection's 'copy ratified 2026-07-25' heading, with no later (N38/N42) marker. N20's enumeration doesn't name it, so confirm. |
| “Uploading — 40%. Your link already works.” | status | provisional | Status line (N38). The percent varies. The same wording is ratified on the owner page (N26), but its status-line placement here is provisional. |
| “Uploading two videos — 40%.” | status | provisional | Status line (N38) with several uploads. The count is in words (Law 16) and varies, as does the percent. The spec wraps it across a line break. |
| “Waiting for connection — your links already work.” | status | provisional | status line (N38) |
| “Updated to 1.3.0 ✓” | status | provisional | One-shot status line (N37/N38). The version varies. |
| “The update didn't finish — shroom will try again later.” | status | provisional | one-shot status line (N37/N38) |
| “No microphone — your video will be silent.” | line | provisional | N39: on a Mac with no microphone |
| “LG UltraFine (left)” | menu-item | provisional | N39: two displays with the same name get their place. The name varies, and they are numbered only when position can't tell them apart. |
| “LG UltraFine (right)” | menu-item | provisional | N39, as above |
| “Camera is off for shroom —” | line | provisional | N39: shown when the camera is denied. The camera picker goes back to Camera off (DEVIATIONS #43), and Camera as the source greys Start. The line is followed by the Open System Settings button. |
| “Open System Settings” | button | provisional | N39 camera-denied door |
| “System default” | menu-item | provisional | N44: the first item of the microphone's and the camera's menus, the device it means in grey beneath (the device's own name, so it varies by Mac) · N44 |
| “Camera off” | menu-item | ratified | the camera picker's last pick; ratified by Bagrat in the asking · #43 |
| “No mic” | menu-item | ratified | the microphone's last pick; ratified by Bagrat in the asking · #43 |
| “You can record once this Mac has joined Bagrat's space.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The popover's Start while a switch's join runs · copy table |
| “LG UltraFine 1” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Displays of one name that place can't tell apart · copy table |
| “LG UltraFine 2” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Displays of one name that place can't tell apart · copy table |
| “LG UltraFine 3” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Displays of one name that place can't tell apart · copy table |
| “Your camera didn't send a picture, so nothing was recorded — try again.” | status | ratified | A Camera only take whose capture died with nothing a repair could save: said once in config's status line, until the popover closes. Ratified by Bagrat (2026-10-14, wording approved); DEVIATIONS #53. · #53 |
| “Your screen didn't send a picture, so nothing was recorded — try again.” | status | ratified | A display take whose capture died with nothing a repair could save: said once in config's status line, until the popover closes. Ratified by Bagrat (2026-10-14, wording approved); DEVIATIONS #53. · #53 |
| “This Mac isn't set up for that video's space.” | status | provisional | The owner page's Edit in shroom (N69), on a Mac set up for another space: the popover opens by itself under the mushroom, the app you were in keeping the keyboard; said once in the status line, first in its order, until the popover closes. Approved as drawn (Bagrat, 2026-10-18). · N69 |
| “Only whoever recorded this video, or the space's owner, can edit it.” | status | provisional | The owner page's Edit in shroom (N69), on a member's Mac, for someone else's video: the popover opens by itself under the mushroom, the app you were in keeping the keyboard; said once in the status line, first in its order, until the popover closes. Approved as drawn (Bagrat, 2026-10-18). · N69 |
| “That video isn't in your space anymore.” | status | provisional | The owner page's Edit in shroom (N69), for a video deleted since its page opened: the popover opens by itself under the mushroom, the app you were in keeping the keyboard; said once in the status line, first in its order, until the popover closes. Approved as drawn (Bagrat, 2026-10-18). · N69 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Record | field |  |  | One source picker. Default: the display the mushroom was clicked on (N56); a display picked here by hand holds until that take ends, and Camera only stays Camera only. Without a click (a popover that opened by itself, Record now): last-used, else the display with the menu bar. Choosing Camera only keeps the camera picker as the pick of the camera the take records, without Camera off, and a first pick of Camera only triggers the inline camera prompt (N22, DEVIATIONS #43). |
| Camera | field |  |  | The camera picker, no longer a switch (DEVIATIONS #43); “Camera” is its accessible name, not shown (DEVIATIONS #45; “Show yourself” until B181). System default first, every camera here, then Camera off: a camera turns the bubble on, live from the moment the popover opens; Camera off turns it off and keeps the pick. The first-ever pick is System default, the bubble on (N43, N44); last-used after that. With Camera only as the source it picks the camera the take records, without Camera off. |
| Microphone | field |  |  | By name, prefers built-in, never auto-picks an iPhone |
| Quality | other |  |  | segmented Normal / 2K / 4K |
| Start recording | filled-default | ↵ | `countdown` | Return = Start recording (Law 8, N38): 'click the mushroom, press Return'. Greys under about a minute of disk (disk-low). |
| Send feedback | other |  | `send-feedback` | The header's first icon, a waving hand, before Library (N60; Bagrat's pick, 2026-10-17). |
| Library | other |  | `library` | footer icon, the Region 5 window |
| Settings | other |  | `settings-general` | footer gear |
| Quit shroom | other |  | `quit-confirm` | Footer icon. Quits at once when nothing is draining. Asks quit-confirm while uploads drain (N41). |
| click away or Esc | other |  | `idle-ready` | dismisses, nothing lost |
| Open System Settings | secondary |  |  | only in the camera-denied variant (N39) |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | retry | click away or Esc — nothing lost | ratified |  |
| `recording-off` | fail | screen recording switched off | provisional |  |
| `countdown` | flow | Start recording | ratified |  |
| `discard-confirm` | flow | Discard row | ratified | N25, B134 |
| `quit-confirm` | side | Quit shroom while uploads drain | provisional |  |
| `stopping` | flow | Get link | ratified | N25, B134 |
| `library` | side | footer icons | provisional |  |
| `settings-general` | side | footer icons | provisional |  |
| `send-feedback` | side | footer icons | ratified | N60 |
| `ai-offer` | auto | config opens on the Mac that holds the setup key while the space's AI is off, no higher banner shows and the banner has never been answered on this Mac (N66) | ratified | N66 |

### `disk-low`

face · popover · **provisional** · a face of `config` · cites N38, N39

A line under Start, in amber, because it is about this take (N38/N39): neither a banner nor the status line. Below about a minute of room, Start greys.

**Entry.** `config` is open and free disk space at the chosen quality is low. Thresholds are a scaffold flag.

| Said | Role | Copy | Note |
|---|---|---|---|
| “About 20 minutes of room left on this Mac.” | line | provisional | Amber, under Start. The minutes vary with free space at the chosen quality. |
| “This Mac is out of room for recording — free up some space first.” | line | provisional | Under about a minute of room. Start greys. |
| “About one minute of room left on this Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `disk-low`, one to nine minutes · copy table |
| “About seven minutes of room left on this Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `disk-low`, one to nine minutes · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Start recording | filled-default | ↵ | `countdown` | Still config's Return default while live (Law 8) in the 'About 20 minutes' variant. Greyed under about a minute of room, when neither click nor Return starts. |

### `recording-off`

banner · banner · **provisional** · a face of `config` · board: DevicesOff · cites N38, N39

The arm-time guard made visible (N38). The grant probe runs when `config` opens, so a missing screen grant shows as a banner before Start, never as a surprise after it. Amber, banner priority 3. Display sources grey out, and Camera still records.

**Entry.** `config` opens and the probe finds screen recording switched off for shroom. This includes the next config after a screen `permission-lost`.

| Said | Role | Copy | Note |
|---|---|---|---|
| “macOS has screen recording switched off for shroom.” | banner | provisional |  |
| “Turn it back on” | button | provisional | filled |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Turn it back on | filled-default |  | `perm-screen` | Filled, but a banner's own filled button is never the Return default (Law 8). Opens the Settings window, Space tab, with row six current at perm-screen. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-screen` | flow | Turn it back on — Settings window, Space tab, row six current | provisional |  |

### `bubble`

overlay · overlay · **ratified** · cites N22, N39, #43

The corner bubble, born at `config`, not at the count (N22). Live from the moment the popover opens, so you see and place yourself before Start. It stays through the count and the take, and Camera off in the camera picker hides it live (DEVIATIONS #43). Draggable, not resizable. It uses the un-excluded preview-window trick with zero compositing. It never takes focus, and it is the only thing of ours ever in a take. Discard takes it away. If the bubble's camera disappears, the take is never interrupted; the bubble simply goes.

**Entry.** A camera is picked in the camera picker (not Camera off) and the source is a display (with Camera only as the source there is no bubble), from config's opening through countdown, recording and paused.

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| drag | other |  |  | draggable, not resizable |

### `countdown`

overlay · overlay · **ratified** · board: Countdown · cites N22, N7, N39, N56

A light scrim on the chosen display only, with a large 3 · 2 · 1 and one quiet line beneath. Silent, and other displays are untouched. The scrim lifts the instant capture starts, so the count is never in the take (N7). There is no recording flash of ours; macOS's own indicator marks the capture. The bubble, if on, simply stays. With Camera as the source, the count plays on the display holding the camera preview: the one the mushroom was clicked on (N39, N56). Cancel by any click, Esc, or the line itself.

**Entry.** Start recording in `config`. Also Restart in `paused` or Restart instead in `discard-confirm`, which re-arm with the count already running.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Click anywhere to cancel.” | line | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| click anywhere | other |  | `config` | Cancel: back to config, still armed, one click from retry. If the count was Restart's, it returns to paused with the take intact (N39). |
| Esc | other |  | `config` | Cancel. The countdown takes focus so Esc works. |
| Click anywhere to cancel. | text-link |  | `config` | the line itself cancels |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | a deadline passes | ratified | N39, #6, B134 |
| `config` | retry | any click, Esc, or the line — cancel, still armed | ratified |  |
| `recording` | flow | 3-2-1 elapsed — the scrim lifts | ratified |  |
| `paused` | flow | cancelling Restart's count (any click, Esc) — take intact (N39; not drawn in the diagram) | ratified |  |

### `recording`

state · tray · **ratified** · board: TrayFaces · cites N24, N39, N41, N25, N54, N62, N64

The tray is the whole surface and there is no popover. The icon wears the red dot. A click is an instant pause, and the popover opens only after the pause is confirmed (never catch a menu in the last frame, POC §8). No timer in the menu bar. Network is never the recording's problem: capture writes locally regardless, and upload is opportunistic in the background. A right-click mid-take is the same as a left click (N39). ⌘Q and the Dock's Quit are declined silently (law 4).

**Entry.** `countdown` elapsed and capture began, or Resume from `paused`. Or capture began at once: the shroom command's start without the count (N62).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your coding agent is recording your screen” | label | ratified | The agent label: a small label at the top right of the display being recorded, while a take the shroom command started records (N62, B216). Left out of the video, as the countdown and the bubble's controls are; it takes no clicks. Not with --no-label; never on a take the person started, nor on one of the camera only; paused, it goes. Ratified by Bagrat, 2026-10-18, relayed by the orchestrator: the board as drawn. · N62 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| tray click | other |  | `paused` | Instant pause. The popover opens after the pause is confirmed. |
| right-click | other |  | `paused` | same as a left click mid-take (N39) |
| ⌥⌘P | other |  | `paused` | the tray click's pause, from the keyboard; the popover opens after the pause is confirmed (N54) |
| ⌥⌘M | other |  | `recording` | drops a marker at this moment of the take; nothing on screen answers, the take goes on (N64) |
| ⌘Q / Dock Quit | other |  |  | declined silently while recording (law 4) |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | the recorder: frame stall | ratified | #6, B134 |
| `paused` | flow | tray click or ⌥⌘P — pause confirmed first | ratified | N54 |
| `display-lost` | fail | display unplugged — auto-pause | ratified |  |
| `mic-lost` | fail | mic · camera · sleep · lock | provisional |  |
| `camera-lost` | fail | mic · camera · sleep · lock | provisional |  |
| `mac-away` | fail | mic · camera · sleep · lock | provisional |  |
| `permission-lost` | fail | screen permission switched off — finalized and asked, like a full disk (N39; the diagram draws this edge into disk_full_stop) | provisional |  |
| `mic-permission-lost` | fail | mic · camera · sleep · lock | provisional |  |
| `disk-full-stop` | fail | out of room | provisional |  |
| `recovered-take` | fail | the take survives, truncated but valid | ratified |  |
| `stopping` | flow | tray click | ratified | N39, B134 |

### `paused`

state · popover · **ratified** · board: Paused · cites N24, N38, N39, N41, B143, N54, N64, B199

The same custom popover, with the icon wearing the pause bars. Top to bottom: the header with the length, Resume (the one filled action, Return default), Finish & Share and Restart as plain rows (Finish & Share was Stop, with a line beneath it; B143), and Discard… as red text at the bottom beside a grey no-Quit note. Quit is absent by law. Drop a marker sits between Finish & Share and Restart (N64, reversing B143's hiding): it lands a marker at the pause point, and the row then reads Marker dropped, greyed until the take resumes. Markers show in the editor (N63). Restart re-arms with the countdown already running. The count is Restart's undo (N39, provisional): the old take is discarded only the instant the new capture begins.

**Entry.** Tray click while recording, after the pause is confirmed (a right-click counts the same, N39). Also: Back (or click-away / Esc) from discard-confirm; a cancelled Restart count; a Quit request while paused, which opens this popover (law 4).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Paused — 4:32” | title | ratified | header; the length varies |
| “Resume” | button | ratified | filled |
| “Restart” | button | ratified |  |
| “Finish & Share” | button | ratified | was “Stop” (B143) |
| “Drop a marker” | button | ratified | N64; between Finish & Share and Restart; greyed in these words while a window's question is open, a marker not dropped (B199) |
| “Marker dropped” | button | ratified | N64; the same row, greyed, once a marker is dropped at this pause — only then (B199) |
| “Discard…” | link | ratified | red text at the bottom, never a button |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Resume | filled-default | ↵ | `recording` | Return = Resume (Law 8, N38) |
| ⌥⌘P | other |  | `recording` | Resume from the keyboard, wherever focus is; nothing while Resume is greyed (N54) |
| Finish & Share | secondary |  | `stopping` | deliberate but visible; a plain row like Restart, with no line beneath (B143) |
| Restart | secondary |  | `restart-confirm` | Restart asks first (DEVIATIONS #44, B134); its count is still Restart's undo (N39). The old take dies only when the new capture begins. |
| Drop a marker | secondary |  | `paused` | A plain row between Finish & Share and Restart (N64). Lands a marker at the pause point; the row then reads Marker dropped and greys until Resume. No state change. A question open in a window (the Library's delete sheet, a Settings sheet) greys it, its words unchanged (B199). |
| Discard… | red-text |  | `discard-confirm` |  |
| Esc | other |  |  | N38's keyboard bullet says Esc dismisses the popover. The spec says nothing more for paused. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording` | flow | Resume or ⌥⌘P | ratified | N54 |
| `discard-confirm` | flow | Discard… | ratified |  |
| `restart-confirm` | flow | Restart | ratified | #44, B134 |
| `stopping` | flow | Finish & Share — the link is made and its page opens | ratified | B143 |

### `discard-confirm`

state · popover · **ratified** · board: DiscardConfirm · cites N23, N24, U1

The paused popover morphs into the question; no window, no dock icon. Discard is red-outlined and rightmost, never filled. There is no Enter/Return default, so a stray Return can never destroy. Click-away or Esc is the same as Back. Discard deletes the local take and any bytes the opportunistic upload already placed in the bucket. It is a clean delete with no tombstone, because the link was never handed out. The POC's 'Keep' option was removed (U1, a 2026-07-20 POC delta), so the panel is three-way.

**Entry.** Discard… from `paused` (including its auto-pause faces). Also Discard… from `recovered-take` (the saved-take variant).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Toss this recording?” | title | ratified | bold |
| “4:32 recorded so far. There's no undo.” | line | ratified | the length varies |
| “Back” | button | ratified |  |
| “Restart instead” | button | ratified |  |
| “Discard” | button | ratified | red outline, rightmost, never filled |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Back | secondary |  | `paused` | leftmost; separated from the other two (· · ·) |
| Restart instead | secondary |  | `countdown` | re-arms with the count already running |
| Discard | destructive |  | `idle-ready` | No Return default (N23, Law 9). The bubble goes with it. Clean delete of the local take and any uploaded bytes. |
| click away or Esc | other |  | `paused` | ≡ Back |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | Discard — clean delete, the bubble goes | ratified |  |
| `config` | flow | Back | ratified | N25, B134 |
| `countdown` | flow | Restart instead — count already running | ratified |  |
| `paused` | retry | Back — click away, Esc | ratified |  |

### `restart-confirm`

state · popover · **ratified** · cites #44, N24, N39, B134

Restart's question, in the paused popover: Restart replaced the take with one click, one row from Stop. Its Restart starts the count, which stays cancellable and is still Restart's undo: a cancelled count gives the old take back, paused. The Discard question's Restart instead is an answer already and starts the count at once.

**Entry.** Restart on the paused popover, any face of it.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Start over?” | title | ratified |  |
| “This replaces the 4:32 you've recorded so far.” | line | ratified | the length varies |
| “Back” | button | ratified |  |
| “Restart” | button | ratified | red outline, rightmost |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Back | secondary |  | `paused` | Esc is Back |
| Restart | destructive |  | `countdown` | no Return default (law 9) |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `countdown` | flow | Confirm restart | ratified | #44, N39, B134 |
| `paused` | flow | Back | ratified | #44, B134 |

### `discard-confirm-saved-take`

face · popover · **ratified** · a face of `discard-confirm` · cites N25, N23

N23's question with its second line changed, used when discarding a saved take from the recovered-take banner. Same clean delete. The spec gives this variant no scaffold-grade name of its own.

**Entry.** Discard… on the `recovered-take` banner in config.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Toss this recording?” | title | ratified | Implied unchanged per N25 ('N23's question, one word changed'); not re-quoted for this variant. |
| “4:32 was saved. There's no undo.” | line | ratified | the saved length varies |
| “Discard” | button | ratified | red outline, never filled (implied by N23's question) |
| “There's no undo.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library, Discard… on a “Removed from the space” row · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Discard | destructive |  |  | Clean delete of the saved take. The destination isn't stated, and the diagram draws only discard_confirm → idle_ready. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Confirm discard row | ratified | #15, B134 |

### `display-lost`

face · popover · **ratified** · a face of `paused` · board: AutoPause · cites N24, N39, B143

The recorded display was unplugged, so the take auto-pauses. This is the paused popover with one context line above the actions. Resume is greyed until the display is back. The take up to the unplug is intact (sacred law). Since N39 (provisional), the popover opens by itself at once and never takes key focus.

**Entry.** Mid-take, the recorded display disconnects.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your display disconnected — recording paused.” | line | ratified | context line above the actions (N24) |
| “Resume” | button | ratified | greyed until the display is back |
| “Finish & Share” | button | ratified | was “Stop” (B143) |
| “Discard…” | link | ratified | red text |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Resume | filled-default |  | `recording` | Greyed until the display is back. The unasked popover never takes key focus (law 5, N39). |
| Finish & Share | secondary |  | `stopping` |  |
| Discard… | red-text |  | `discard-confirm` |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording` | flow | Resume, once the display is back | ratified |  |
| `discard-confirm` | flow | Discard… | ratified |  |
| `stopping` | flow | Stop | ratified |  |

### `mic-lost`

face · popover · **provisional** · a face of `paused` · board: AutoPause · cites N39

The microphone disappears mid-take (AirPods out of range, a USB mic pulled), so the take auto-pauses. A context line appears plus a quiet line naming the mic Resume will use. Resume stays live. The popover opens by itself at once, without key focus.

**Entry.** Mid-take, the selected microphone disconnects.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your microphone disconnected — recording paused.” | line | provisional |  |
| “Resume uses MacBook Pro Microphone.” | line | provisional | Quiet. Names the built-in mic, so the name varies by Mac. |
| “Resume continues without your microphone.” | line | ratified | replaces the line above on a Mac with no other microphone, so which of the two shows varies by Mac. Was “Resume continues without sound.” until the Mac's own sound kept recording (Bagrat, 2026-10-16, N47) |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Resume | filled-default | ↵ | `recording` | Stays live and switches to the built-in mic (or continues without your microphone, the Mac's sound still recorded — N47). It stays paused's Return default, but the unasked popover never takes key focus, so a Return typed into another app can't Resume. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording` | flow | Resume | provisional |  |

### `camera-lost`

face · popover · **provisional** · a face of `paused` · board: AutoPause · cites N39

Camera is the source and it disappears, so the take auto-pauses. Resume is greyed until the camera is back. If the bubble's camera disappears instead, the take is never interrupted; the bubble simply goes.

**Entry.** Mid-take with Camera as the source, and the camera disconnects.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your camera disconnected — recording paused.” | line | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Resume | filled-default |  | `recording` | Greyed until the camera is back. Unasked popover, no key focus. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording` | flow | Resume, once the camera is back | provisional |  |

### `mac-away`

face · popover · **provisional** · a face of `paused` · board: AutoPause · cites N39

The take auto-pauses at the instant of sleep, lid closed, screen locked, or another user switched in. The popover opens by itself at unlock or wake, with its context line and without key focus.

**Entry.** Mid-take, the Mac sleeps, the lid closes, the screen locks, or a user switch happens.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your Mac went to sleep — recording paused.” | line | provisional |  |
| “Your screen locked — recording paused.” | line | provisional |  |

### `permission-lost`

failure-state · popover · **provisional** · cites N39

The screen half: macOS switches off screen recording mid-take (System Settings, a cleanup tool). A screen grant only returns after a relaunch, and nothing relaunches mid-take (law 4). So this is a stop you didn't make, not a pause. The take finalizes and asks, like disk-full-stop, and is never auto-published. The popover opens by itself without key focus. `recording-off` then greets the next config. The microphone half is an auto-pause and has its own face entry.

**Entry.** Mid-take, the screen-recording grant is revoked.

| Said | Role | Copy | Note |
|---|---|---|---|
| “macOS switched off screen recording, so the recording stopped at 7:15 — everything so far is saved.” | line | provisional | the stop time varies |
| “Get link” | button | provisional | filled |
| “Discard…” | link | provisional | red text |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Get link | filled-default | ↵ | `stopping` | The face's one filled action, so Return triggers it once the popover has focus (Law 8; line 854 says an unasked popover's lack of key focus is what stops a stray Return from triggering Get link). The filled button leads (Law 8's local exception for single-action faces). |
| Discard… | red-text |  |  | destination not stated or drawn |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording-off` | flow | the next config opens (banner greets it) | provisional |  |
| `discard-confirm` | flow | Discard saved | ratified | N25, B134 |
| `stopping` | flow | Get link | provisional |  |

### `mic-permission-lost`

face · popover · **provisional** · a face of `paused` · board: AutoPause · cites N39

The microphone half of `permission-lost`: macOS switches off the microphone mid-take, so the take auto-pauses. Resume continues without your microphone. The popover opens by itself without key focus.

**Entry.** Mid-take, the microphone grant is revoked.

| Said | Role | Copy | Note |
|---|---|---|---|
| “macOS switched off the microphone — recording paused.” | line | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Resume | filled-default | ↵ | `recording` | Continues without your microphone (the Mac's sound still recorded — N47). Spec text: '(Resume continues without sound)', unquoted here. It is paused's Return default, but there is no key focus while unasked. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `recording` | flow | Resume — continues without your microphone | provisional |  |

### `disk-full-stop`

failure-state · popover · **provisional** · board: StoppedForYou · cites N39, N25, N38

At the disk floor the take finalizes itself, so it never dies by a surprise ENOSPC. It is never auto-published (N25's principle: you didn't choose to stop). The popover opens by itself without key focus and asks. Dismissed unanswered, it becomes a recovered take under the amber dot (banner kind 1).

**Entry.** Mid-take, free disk space reaches the floor.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This Mac ran out of room, so the recording stopped at 12:04 — everything so far is saved.” | line | provisional | the stop time varies |
| “Get link” | button | provisional | filled |
| “Discard…” | link | provisional | red text |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Get link | filled-default | ↵ | `stopping` | The face's one filled action, so Return triggers it once the popover has focus (Law 8; line 854: an unasked popover never takes key focus, 'so a Return typed into another app can't … Get link'). The filled button leads (Law 8's local exception). |
| Discard… | red-text |  |  | destination not stated or drawn |
| dismiss unanswered | other |  | `recovered-take` |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `discard-confirm` | flow | Discard saved | ratified | N25, B134 |
| `recovered-take` | flow | dismissed unanswered | provisional |  |
| `stopping` | flow | Get link | provisional |  |

### `recovered-take`

banner · banner · **ratified** · a face of `config` · board: SavedTake · cites N25, N38, N41

After a crash or forced shutdown mid-take, §5.5's valid-but-truncated file survives. The next launch is silent and the tray wears the amber dot. The next click opens `config` with this banner on top, and config stays usable beneath it. Never auto-published. Get link runs the normal stop sequence from the saved file. Discard… confirms with N23's question (saved-take variant) and does a clean delete. It persists until resolved, across launches. Two interrupted takes surface oldest first, and a new take can start meanwhile. N38 places it at banner priority 1 (amber).

**Entry.** A take was interrupted (crash, forced shutdown, system-initiated termination), or disk-full-stop was dismissed unanswered. The next tray click opens config.

| Said | Role | Copy | Note |
|---|---|---|---|
| “We saved your interrupted recording (4:32).” | banner | ratified | the saved length varies |
| “Get link” | button | ratified | filled |
| “Discard…” | link | ratified | red text |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Get link | filled-default |  | `stopping` | Filled and leads (Law 8's saved-take-banner exception). A banner's own filled button is never the Return default; Return in config stays Start recording. |
| Discard… | red-text |  | `discard-confirm-saved-take` |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `discard-confirm` | flow | Discard… — “4:32 was saved.” | ratified |  |
| `discard-confirm-saved-take` | flow | Discard… (the diagram draws recovered_take → discard_confirm) | ratified |  |
| `stopping` | flow | Get link — the normal stop sequence | ratified |  |

### `quit-confirm`

state · popover · **provisional** · board: QuitConfirm · cites N41

Quit with uploads draining: the popover morphs into a question. No Return default. Nothing is destroyed, so nothing is red. The right-click Quit asks the same way. With nothing draining, Quit quits at once. A system shutdown never asks: the queue is on disk and resumes at the next launch.

**Entry.** Quit shroom (config footer, or the idle right-click menu) while uploads are draining.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Quit with two videos still uploading?” | title | provisional | Bold. The count is in words (Law 16) and varies. |
| “They'll finish the next time shroom opens — until then, their links say “on its way.”” | line | provisional | Keeps the inner curly quotes around 'on its way.'; the outer closing quote is stripped. |
| “Back” | button | provisional |  |
| “Quit anyway” | button | provisional | rightmost; not red, not filled |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Back | secondary |  | `config` | leftmost, separated (···) |
| Quit anyway | secondary |  |  | Quits shroom. Uploads resume at the next launch. No Return default, and not red because nothing is destroyed. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | tray click | ratified | law 9, N21, B134 |
| `config` | flow | Back | provisional |  |

### `plain`

tray-face · tray · **ratified** · a face of `idle-ready` · cites N21, N24, N38

The plain mushroom, identical to idle-unprovisioned's. It shows when no take is open, nothing needs you, and no update waits. Face priority: the recording faces, then amber, then Frost, then plain.

**Entry.** Idle, with no banner of kinds 1–4 waiting.

### `tray-red-dot`

tray-face · tray · **ratified** · a face of `recording` · cites N24, N38

Recording. No popover, and the click is the pause. No timer in the menu bar.

**Entry.** A take is recording.

### `tray-pause-bars`

tray-face · tray · **ratified** · a face of `paused` · cites N24, N38

Paused. The icon wears pause bars while the take is paused.

**Entry.** A take is paused.

### `tray-amber-dot`

tray-face · tray · **ratified** · a face of `idle-ready` · cites N25, N38

Something needs you: a banner of kinds 1–3 waits (a saved take, the space can't take videos, recording-off). Born ratified for the saved take (N25). Its 'something needs you' generalization is N38's (provisional). No window, no notification.

**Entry.** Idle, and a recovered take / unanswered disk-full-stop, a space-can't-take-videos banner, or recording-off waits.

### `tray-frost-dot`

tray-face · tray · **provisional** · a face of `idle-ready` · cites N24, N25, N37, N38

Something new from us: an update waits (update-ready, Region 5). This is the lowest non-plain face. The Frost dot is the update's own tray face, the lowest in priority: the recording faces and the amber dot both override it. Amber stays 'something needs you'; Frost says 'something new from us'.

**Entry.** Idle, an update is ready, and no amber-kind banner waits. update-ready holds, there is no recording or paused take, and no amber cause. Never shown before a space exists or when the release is past this Mac's macOS.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | tray click | provisional |  |

#### Region 3's amendments

- **N17** (amends Region 3's original perm-preamble beat): perm-preamble fuses into Region 2's `done`, which previews the choreography with Allow recording. Region 3 opens at perm-mic.
- **N20** (amends the passive end-of-onboarding handoff): all-set becomes the window's final face, a resting state with three doors (Close · Invite someone · Record now). relaunching is named as a beat.
- **N21** (amends N1 (tray Copy last link), U9's tray upload mirror, U7's tray update row): No native menus. The tray click opens config directly, and right-click is a one-item Quit shroom menu. Copy last link is removed (N1 amended). The orphaned rows move to popover slots (settled by N24, N37 and N38).
- **U1** (amends the POC's four-way discard panel (POC delta, 2026-07-20)): 'Keep' (keep locally without publishing) is removed because U1 removed local keeps, so discard-confirm is three-way. N23 later ratified the three-way panel's copy.
- **N37** (amends settles N25's open badge question): Autonomous, awaiting ratification. The update gets its own Frost dot, the lowest tray face. Amber stays 'something needs you'.
- **N38** (amends extends N24's three tray faces plus N25's amber fourth; settles N21's popover slots and, with N37, N25's badge question): Autonomous, awaiting ratification. Five tray faces: plain · red dot · pause bars · amber (something needs you) · Frost (update). The popover has one banner (ask) and one status line (fact). recording-off appears before Start. The monthly re-confirm moves to config. Return = Start / Resume.
- **N39** (amends extends ratified N24 (paused's Restart, display-lost)): Autonomous, awaiting ratification. The count is Restart's undo. The auto-pause family (mic-lost, camera-lost, mac-away, permission-lost) and disk-full-stop open their popover unasked without key focus. disk-low sits under Start. Device rules at config.
- **N41** (amends Region 1's 'every launch is silent' (touches recovered-take's 'next launch is silent')): Autonomous, awaiting ratification. A launch you did is a tray click, and shroom's own launches stay silent. quit-confirm applies while uploads drain. Nothing ends the app mid-take.
- **N42** (amends extends ratified N20 (perm-screen, all-set row six)): Autonomous, awaiting ratification. Adds 'Flipped it already?' [ Reopen shroom ] under perm-screen for a declined relaunch. Row six ticks with 'microphone off for now' when the mic was denied.
- **U5** (amends the walkthrough's first-arm permission recommendation): Permissions happen at the end of onboarding in the same window (mic inline → Settings switch → guided relaunch → priming). The first Record is pure, and the arm-time re-probe survives as a guard.

#### Region 3's open notes

- Law 8's local exception is marked 'check it at ratification': single-action faces whose one filled button LEADS (the saved-take banners). In this region that means recovered-take's Get link before Discard…, and by extension disk-full-stop and screen permission-lost (lines 141-143).
- Return on disk-full-stop / screen permission-lost: Law 8 makes the one filled action the Return default, and line 854 says only the missing key focus stops a stray Return from triggering Get link, so Get link is modeled as the Return default once focused. Law 8's 'never the Return default' applies to banners, and these are popovers. Confirm.
- The camera usage string sits in the copy-ratified chapter table, but N20 enumerates only 'the mic usage string' as ratified. It is marked ratified here on the heading's authority; confirm.
- The quiet hint 'allowed in System Settings' (denied mic at config) is a quoted fragment with no full rendered wording. It sits under the permissions subsection's 'copy ratified' heading, so it is marked ratified on that authority (consistent with the camera string), but N20 doesn't enumerate it; confirm, and pin the full wording.
- The diagram draws the screen-permission edge as recording → disk_full_stop, but the table gives screen permission-lost its own line ('macOS switched off screen recording, so the recording stopped at 7:15 …'). It is modeled here as its own failure-state that behaves like disk-full-stop. If it is literally disk_full_stop, it would also inherit 'dismissed unanswered → recovered-take', which the table doesn't state for permission-lost.
- Discard… from disk-full-stop and from screen permission-lost has no stated or drawn destination (presumably discard-confirm).
- discard-confirm reached from recovered-take: only the line change and the clean delete are stated. The Back and Discard destinations and whether Restart instead appears are unstated, and the diagram draws Back → paused only.
- The diagram draws idle_ready → recovered_take ('launch finds an interrupted take'), but recovered-take renders as a banner atop config, greeted by the next tray click.
- Not drawn in the diagram, though stated in the tables: Reopen shroom → relaunching (N42); a cancelled Restart count → paused with the take intact (N39); right-click Quit → quit-confirm from idle-ready (N41); config → Library / Settings via the footer; recording-off and disk-low; crash mid-take → recovered-take; recording-off → perm-screen via the Settings window.
- mac-away gives lines only for sleep and lock. Lines for lid closed and user switch are unstated, as are its actions (presumably the paused popover's).
- Screen permission-lost's microphone half says '(Resume continues without sound)' unquoted. mic-lost's quoted equivalent is “Resume continues without sound.” It's unclear whether a line renders.
- `relaunching` has no render or said-line in the region.
- Click-away on the paused popover is unspecified. N38's keyboard bullet says only that Esc dismisses.
- display-lost: N24 says 'the same popover with the context line', but the edge row lists only Resume (greyed) · Stop · Discard…. Whether Drop a marker and Restart remain is unstated.
- quit-confirm reached by right-click Quit from idle-ready: the Back destination is unstated (the diagram draws config → quit_confirm → config). Esc / click-away on quit-confirm is also unstated beyond Law 9's 'Esc always means back'.
- recovered-take says 'next launch is silent', but N41 / law 5 make a user-made launch a tray click. Presumably such a launch opens config with the banner. Check against N41's amendment of Region 1.
- The config heading is copy ratified (N21), but the popover grammar (banner slot, status line, disk-low under Start, Return = Start) is N38/N39 and provisional. The status-line string 'Uploading — 40%. Your link already works.' matches ratified owner-page copy (N26), but is provisional in this placement. 'Fully live ✓' is gone from the popover as from the page (DEVIATIONS #46, #52): the upload line just goes when the last upload finishes. Since 2026-10-18 the owner page says “Finishing your video — the link already works, share it anytime.” instead (N65 (7)); the popover keeps its own line.
- all-set says 'all six rows green', while N42 lets row six tick with 'microphone off for now'.
- The member all-set variant (two doors, Close · Record now) lives in Region 6 (N30), not here.
- The bubble is not a state in the inventory. It is included as the overlay the region defines (N22; the surfaces table, line 1720). 'discard-confirm (saved take)' and 'permission-lost (microphone)' are fixture ids for variants that the spec doesn't name separately.
- Law 4: a Quit request while paused opens the paused popover (its no-Quit note), and is declined silently while recording.
- No in-range 'Fat-marker status' notes or 'check at ratification' markers exist beyond Law 8's exception (cited from the Standing laws).

#### Region 3's scaffold flags

- Info.plist usage strings ARE said-lines: the mic and camera usage strings must ship verbatim in Info.plist.
- Mic resolves fully before the screen step, so system dialogs never stack (POC §4).
- perm-screen opens the exact System Settings pane. The grant takes effect only on the next launch, and resumption after relaunch is derived from probes with no stored step counter.
- Reopen shroom (N42) needs a self-relaunch path that lands in `relaunching` / the permissions chapter.
- perm-priming: an invisible warm-up of about 1 s that triggers Apple's second (~30-day) consent at setup (probe: dialog real on Tahoe, non-fatal).
- Arm-time guard: grants are re-probed at every record arm and when config opens. The missing beat is replayed, and a missing screen grant surfaces as `recording-off` before Start.
- Monthly re-confirm: config must know when Apple's periodic re-confirm is due and run perm-priming's warm-up then. The popover may close as the dialog takes focus.
- The camera bubble: the un-excluded preview-window trick with zero compositing. Draggable, not resizable, never takes focus. Live from config's opening through the take.
- Never auto-pick an iPhone (Continuity devices), for the microphone or the camera (POC §2).
- Quality presets Normal (1080p) / 2K / 4K come from POC §2.
- Device fallbacks: a click's display is the source unless a display was picked by hand for this take (N56); a gone last-used display falls back silently to the menu-bar display. Same-named displays are labeled by position and numbered only when position can't tell them apart. Camera as the source previews at config on the clicked display, and the count plays on that display.
- Countdown scrim lifts the instant capture starts, so the count is never in the take. The countdown takes focus so Esc works without extra macOS permission.
- Recording: click = instant pause, and the popover opens only after the pause is confirmed (POC §8). Right-click mid-take = left click.
- Capture writes locally regardless of network. Upload is opportunistic in the background.
- Markers: metadata-only at the pause point in v1, and hidden from the UI until the editor ships (B143).
- Restart's undo: the old take is discarded only the instant the new capture begins. A cancelled count leaves it intact.
- Discard deletes the local take AND any bytes already uploaded to the bucket: a clean delete, no tombstone.
- Non-activating popovers: an unasked (auto-pause / auto-stop) popover never takes key focus.
- Thresholds: disk-low (~20 minutes example), the Start-greys floor (~1 minute), and the disk-full-stop floor, each at each quality.
- disk-full-stop finalizes at the floor, never by a surprise ENOSPC.
- A screen grant only returns after a relaunch, so screen permission-lost mid-take must finalize rather than pause.
- mac-away detection: sleep, lid closed, screen lock, and user switch pause at that instant. The popover opens at unlock or wake.
- recovered-take relies on §5.5's valid-but-truncated file surviving a crash or forced shutdown. Interrupted takes surface oldest first and persist across launches.
- quit-confirm: the upload queue is on disk and resumes at the next launch. A system shutdown never asks.
- stopping: finalize measured at 102 ms.
- Launch provenance: tell a user-made launch (treated as a tray click) from a login-item or update relaunch (silent).
- Popover anchor fallback to the top-right when the status item is occluded (e.g. behind the notch); over a hidden menu bar, where the item rests (B161).
- Tray faces have no scaffold-grade names in the spec. The fixture ids use the spec's words (plain, red dot, pause bars, amber dot, Frost dot), and the scaffold should pin real names.

<a id="region-4"></a>

## Region 4 — stop → the link → the recipient

*Region 4 — stop → the link → the recipient (UX-STATEMACHINE.md lines 907-1074; ratified 2026-07-20; copy ratified 2026-08-21 — owner page N26, recipient faces N27; N40 edges designed 2026-09-30, awaiting ratification)*

### `stopping`

state · other · **ratified** · cites N24, N25, N39, N40, P9, POC §5, §5.3

The stop beat: the take is finalized (measured 102 ms) and the video's id exists, so its page exists (P9). Stop means published, with no publish prompt (§5.3). Three ways out: the owner page opens (the normal case), stopped-offline, or stopped-stranded. The two N40 exits are designed, not ratified. `stopping` is the scaffold-grade name (Region 3, line 863, and the inventory, line 1696); the Region 4 diagram draws the same beat as `finalizing`. A beat: finalize (measured 102 ms), then Region 4, the link moment.

**Entry.** A deliberate Stop from the paused popover (Region 3 paused → stopping), or Get link on a saved take (recovered-take's banner, disk-full-stop), which runs the normal stop sequence from the saved file. Stop in `paused`, or Get link from `recovered-take`, `disk-full-stop` or screen `permission-lost`.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | the recorder: finalized | ratified | N25, B134 |
| `config` | flow | a deadline passes | ratified | #6, B134 |
| `permission-lost` | fail | the recorder: finalized | ratified | N39, B134 |
| `disk-full-stop` | fail | the recorder: finalized | ratified | N39, B134 |
| `owner-page` | flow | finalize, 102 ms — the page opens | ratified |  |
| `stopped-offline` | fail | offline at stop | provisional |  |
| `stopped-stranded` | fail | the space can't take it | provisional |  |

### `owner-page`

state · web-page · **ratified** · board: OwnerPage · cites U2, N26, N1, U9, U3, N35, N40, R11, N46, N69, N66, N67

The real shareable page, opened in the browser at stop. It is never a preview or a processing screen. From top to bottom: the editable title field, which is the only chrome (no header, no wordmark, no name, per U3); the player (first frame, play, scrub bar, `0:00 / 4:32`, fullscreen); [ Copy link ] and, once the video plays, [ Copy thumbnail ] beside it (N46), the page's copy doors, with the upload line and a thin bar beside it; then, bottom-right, the owner switch, “made with shroom”, and the mark. Download appears only once the single-file MP4 exists, and it is second-class (R11). Only the editable title and the switch are owner-only. The address bar shows the clean link because the key died at the 302. The clipboard is never touched (N1). Rename at any time later is a metadata write, live worldwide on the next load (R4). It works from this page in owner-mode or from the Library. Owner-mode by role is N40 and still provisional: the cookie names a member, a member's own pages give them the title only, and owner-class browsers get full owner-mode on every page of the space. When the space names its videos (N66), an untitled video takes the name made from what was said; a line under the title says so until the title is edited, or that the name comes a little later while the free day is used up; and the player gains the chapters and the Captions button the recipient sees (N67). Edit in shroom (N69): the scissors, the last of the quiet icons beside the title, after Download, only on the owner page and only once the video is fully up (as Edit… in the Library); it opens the video in shroom's editor on this Mac.

**Entry.** Stop finalized and the space's page is reachable and can fill. The app opens `v.<name>.workers.dev/<id>?k=<one-time-key>`. The Worker validates k, sets a long-lived owner cookie that names the member, and 302-redirects to the clean URL. The page also opens later on any visit from a browser carrying a qualifying cookie (owner-mode persists per-browser, rendered server-side), when stopped-offline reconnects, and through the Library's Open page (the owner-keyed door, N35).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Untitled — Aug 21” | placeholder | ratified | The title field's placeholder. The date varies (the day's date). Left untouched, the placeholder becomes the title, so a shared link is never blank. |
| “0:00 / 4:32” | other | ratified | The player's time readout (position / length). The values vary. The spec writes it in backticks, not as quoted copy. |
| “Copy link” | button | ratified |  |
| “Copied” | status | ratified | Copy link's label for a moment after the click. |
| “Copy thumbnail” | button | ratified | Beside Copy link, once the video plays (N46). Greyed until the picture for the current title is ready (the loop, or the still when no loop can be made). Reads “Copied” for a moment after the click, like Copy link. |
| “Preparing the thumbnail…” | tooltip | ratified | N46: Copy thumbnail's tooltip while it waits for its picture (wording approved by Bagrat, 2026-10-15). |
| “Download” | button | ratified | Present only once the single-file MP4 exists; second-class (R11). The spec gives no button styling beyond “second-class”; the action is recorded as secondary on that basis. |
| “made with shroom” | field-label | ratified | The owner switch's label, beside the mark. The same three words are the hover tooltip recipients get. |
| “Edit in shroom” | tooltip | provisional | The scissors' tooltip, and its name for screen readers (N69). Approved as drawn (Bagrat, 2026-10-18). · N69 |
| “Named from what you said. Click the title to change it.” | line | provisional | N67. Under the title while the space's name stands and nobody has typed one; gone once the title is edited. Proposed 2026-10-18 (AiOwnerNamed); awaiting Bagrat. · N67 |
| “Its name and chapters come a little later, once your free hours are back.” | line | provisional | N66 (4), N67. Under the title while the take waits for the next free day (after 00:00 UTC); nothing reads as an error. Proposed 2026-10-18 (AiNamingLater); awaiting Bagrat. · N66, N67 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Untitled — Aug 21 | field | ↵ | `owner-page` | Saves the title in place. The title field, focused on arrival. Enter or click-away saves. Left untouched, the placeholder becomes the title. Clearing the title and leaving the field brings the previous title back (N35). Per Law 8, a focused field's Return belongs to its field. |
| Copy link | filled-default |  | `owner-page` | The label reads “Copied” for a moment. The page's one copy door (N1); copies the clean link. No copy affordance ever emits a keyed URL. It is filled, but Return belongs to the focused title field, and the spec names no Return default for this button. |
| Copy thumbnail | secondary |  | `owner-page` | N46. Copies the video's picture wrapped in the clean link, with the plain link as text beside it; reads “Copied” for a moment. Shown once the video plays, and greyed (no copy) until the picture for the current title is stored: the loop, or the still when no loop can be made. The picture is made in the background after each title save, and a video with none yet gets it made when its page opens. |
| Download | secondary |  |  | Appears only when the single-file MP4 exists; second-class (R11). |
| Edit in shroom | secondary |  | `editor` | N69. The scissors, the last of the quiet icons beside the title: the shroom://edit/<host>/<id> link, with nothing secret in it. Shown only on the owner page (the space's owners on every video, a member on their own), once the video is fully up, as Edit… in the Library. The browser's own “Open shroom?” is the only step it adds; the Mac decides by its own state: the editor, or the popover's one line. Where shroom isn't on the Mac, the page says nothing (only a Mac running shroom makes a browser the owner's). |
| made with shroom | switch |  |  | The owner switch bottom-right, beside the mark. Shown to owner-class only (N40). Flipping it is the space-global mark setting: one metadata write, which the Worker reads at render (U3). |
| the mark (small shroom icon) | other |  | `landing` | Bottom-right; hover “made with shroom”; links home. |
| play | other |  |  | Player control. |
| scrub bar | other |  |  | Player control. |
| fullscreen | other |  |  | Player control. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `uploading` | flow | (unlabeled in the diagram) the page opens while the upload queue drains | ratified |  |
| `waiting-conn` | fail | the Mac drops offline | ratified |  |
| `fully-live` | auto | queue drains — the page refreshes itself | provisional | #46 |
| `live` | flow | the link, always clean — content ready | ratified |  |
| `on-its-way` | flow | the link beat the upload | ratified |  |
| `owner-page-cuts` | auto | the take's voice cuts land — the page shows them | provisional | N65 |
| `owner-page-cue-hint` | auto | a take with no voice cuts, in a space that listens — the page shows how to cut a slip | provisional | N65 |
| `hearing` | auto | opened while the take is heard — the line, its spinner turning | provisional | N65 |
| `editor` | side | Edit in shroom | provisional | N69 |
| `editor-getting` | side | Edit in shroom | provisional | N69 |
| `config` | side | Edit in shroom | provisional | N69 |

### `uploading`

face · web-page · **ratified** · a face of `owner-page` · board: OwnerPageFinishing · cites N21, N26, N65, U9

The owner page's one line while the take is finishing (N65 (7)): from go-live, under the player, after a small spinner that turns while the take is finishing (Bagrat, 2026-10-18 (relayed by the orchestrator): a small spinner in front of the words, no bar); no percent. In a space that listens it stays after the upload while the take is heard (`hearing`); elsewhere it goes when the upload drains (`fully-live`). It replaced “Uploading — 40%. Your link already works.” on the page (Bagrat, 2026-10-18 (relayed by the orchestrator): one indicator on the owner page, from the upload until the voice cuts land, “Finishing” his pick of its first word). The config popover's status line keeps its own words (U9 / N21).

**Entry.** The owner page is open and this video's upload is in progress.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Finishing your video — the link already works, share it anytime.” | status | ratified | Under the player, after a small spinner that turns while the take is finishing (Bagrat, 2026-10-18 (relayed by the orchestrator): a small spinner in front of the words, no bar); no percent. Bagrat, 2026-10-18 (relayed by the orchestrator): one indicator on the owner page, from the upload until the voice cuts land, “Finishing” his pick of its first word. The same line stays while the take is heard (hearing); a stalled upload's line (waiting-conn) keeps its spinner resting. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `hearing` | auto | the upload drains in a space that listens while the take is heard — the line stays, its spinner turning | provisional | N65 |
| `waiting-conn` | flow | offline | ratified |  |
| `fully-live` | flow | queue drained | ratified |  |

### `waiting-conn`

face · web-page · **ratified** · a face of `owner-page` · board: OwnerPagePending · cites N26, N40

The owner page's upload line when the Mac dropped offline after the page opened. The face is in the ratified diagram (inventory: “waiting”, R — N26). Its wording is an N40 line variant, so the copy is provisional. Its spinner rests, dimmed, until the bytes move again (N65 (7), 2026-10-18).

**Entry.** The Mac went offline after the owner page opened, while the upload was still draining.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Waiting for connection — your link already works.” | status | provisional | N40 line variant. The config popover's status-line counterpart (Region 3, line 777) reads “Waiting for connection — your links already work.” |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `owner-page` | auto | back online — the video follows | ratified |  |
| `uploading` | flow | back online | ratified |  |
| `on-its-way` | side | recipients meanwhile | ratified |  |

### `waiting-space`

face · web-page · **provisional** · a face of `owner-page` · cites N40

The owner page's upload line when the space still serves pages but can't take uploads yet. The page opens as usual and its upload line says so. The spec never gives this face a scaffold name: `waiting-space` is a descriptive placeholder.

**Entry.** At stop, or later: the space still serves pages but can't take uploads yet (a dead token, storage switched off). Members see the named-owner variant.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Waiting for your space — your link already works.” | status | provisional |  |
| “Waiting for Bagrat's space — your link already works.” | status | provisional | The members' variant. The owner's name varies. |

### `fully-live`

face · web-page · **ratified** · a face of `owner-page` · board: OwnerPageLive · cites #46, #52, B142, N26, N65, U9

The owner page once the queue has drained and nothing more is being finished: the line goes. On a page that was open, it fades, spinner and all; in a space that listens it first stays while the take is heard (`hearing`). A page opened after shows no line. No “Ready ✓” since 2026-10-18 (Bagrat, 2026-10-18 (relayed by the orchestrator): one indicator on the owner page, from the upload until the voice cuts land, “Finishing” his pick of its first word: one line, nothing after it; B142's “Ready ✓” retired on the page). The config popover's status line just goes (DEVIATIONS #46, #52).

**Entry.** This video's upload queue has drained.

### `owner-page-member`

face · web-page · **provisional** · a face of `owner-page` · cites N40, U3, N5

Owner-mode by role (N40): on a member's own pages, the member's owner-mode is the editable title only. The mark switch is space-global (U3), so it belongs to owner-class Macs. The member sees no switch. When the space can't take uploads, the member's upload line names the owner. The spec doesn't name this variant: `owner-page-member` is a descriptive placeholder.

**Entry.** The browser's owner cookie names a member who made this video and is not owner-class. The Worker checks the video's maker against the cookie.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Waiting for Bagrat's space — your link already works.” | status | provisional | The members' form of the waiting-space line. The owner's name varies. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Untitled — Aug 21 | field | ↵ | `owner-page-member` | Saves the title in place. The title, the member's only owner-mode control. It behaves like the owner page's title field. |

### `stopped-offline`

state · popover · **provisional** · board: StoppedOffline · cites N40, N1, N41

An offline Mac can't load the page, so the paused popover morphs in place: no window, no dock icon. It shows the link, a Copy link button, and the promise that the page opens by itself when the connection returns. Copy link is explicit (N1). Recipients meanwhile get on-its-way, without a title until the details arrive. When the connection returns, the owner page opens, keyed, exactly as a live stop would. If a take is running by then, it waits for that take to end (the quiet-take law, N41). It fires once, only for the latest stop, and only while it's still “now”: several offline stops open one page, the newest. The page opens by itself only if you're back within the hour and shroom is still running. Otherwise, including after a quit or restart, the Library row carries the link and a login launch stays silent. Click-away dismisses the popover, and the page still opens later.

**Entry.** During finalize, the short reachability probe of the space's page (capped well under a second) can't reach it. This also covers Get link while offline from a saved take's banner (recovered-take) or from disk-full-stop.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your link is ready.” | title | provisional |  |
| “You're offline — it's already safe to share. The page opens by itself the moment you're back.” | line | provisional | Carries the meaning of beat 6's superseded ratified line. |
| “v.myshroom.workers.dev/b4kq9d” | other | provisional | The link, shown in the popover (backticked in the spec). The name and the id vary. |
| “Copy link” | button | provisional |  |
| “Copied” | status | provisional | Comes from Law 10 (line 147: copying is always a visible button that reads “Copied”). The region text doesn't restate it for this popover. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | filled-default | ↵ | `stopped-offline` | The label reads “Copied”. The only filled button on the surface. Return-triggering is inferred from Law 8 (one default per surface, and Return triggers it); the region text doesn't say. Copying is explicit (N1). |
| click-away | close |  | `closed` | The page still opens later; the Library row carries the link meanwhile. The spec doesn't name the state the popover lands in. Esc presumably dismisses (line 787 “Esc dismisses”; Law 9 “Esc always means back”), but this popover doesn't restate it. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `owner-page` | auto | back online — opens itself | provisional |  |

### `stopped-stranded`

state · popover · **provisional** · cites N40, N34, N42, N38

The space can't take this video. The popover says the recording is safe, gives the matching banner's reason line (N34 / Region 2), and offers [ Save to Movies ]. Owners also get [ Reconnect your space ]. The page never opens, because it could never fill. The take waits in the Library as “Waiting for your space” until it drains, is saved, or is discarded. Under Law 8, this single-action face's one filled button leads rather than sitting rightmost.

**Entry.** At stop, the space can't take this video: member-revoked, space-gone, or an owner's space whose page is gone.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your recording is safe on this Mac.” | title | provisional |  |
| “Your access to Bagrat's space was turned off. Recordings on this Mac are safe — ask Bagrat if that's a surprise.” | line | provisional | The reason line when the cause is member-revoked. The region only says “the matching banner's reason line (N34 / Region 2)”; this is the member-revoked banner's line from line 1578. The owner's name varies. |
| “Bagrat's space isn't there anymore. Recordings on this Mac are safe — save any you want to keep.” | line | provisional | The reason line when the cause is space-gone, from the space-gone banner (line 1580). The owner's name varies. |
| “Save to Movies” | button | provisional |  |
| “Reconnect your space” | button | provisional | Owners only. |
| “Saving to Movies…” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Save to Movies while its copy runs (the banners, the exit sheets, the switch, the Library's rows, `stopped-stranded`) · copy table |
| “Couldn't save to Movies — try again” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Save to Movies that couldn't make its file · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Save to Movies | filled-default | ↵ | `stopped-stranded` | The single-file copy lands in ~/Movies and Finder shows it; the take keeps waiting until discarded. The filled button leads (leftmost), per Law 8's single-action exception, which is to be checked at ratification. Return-triggering is inferred from Law 8 (one default per surface, and Return triggers it); the region text doesn't say. The after-click face (“Saved to Movies ✓”) is specified only for the banners (lines 1582-1586). |
| Reconnect your space | secondary |  | `settings-space-repair` | Owners only. |

### `live`

web-page · web-page · **ratified** · board: RecipientLive · cites N27, U3, P1, R11, R5, N46, N67

The recipient's normal page: the owner page minus the owner's two controls. It shows the title as plain text, the minimal player (first frame well under 1 s per R6.2, custom controls), Copy link and Copy thumbnail (N46), Download (quiet and second-class, once the single-file MP4 exists), and the mark with its hover label. Nothing else: no comments, no sign-in, no name, no shroom account anywhere. Every page is `noindex`, with no passwords and no expiry; captions only where the owner turned naming on (N67). Unfurls follow R5 honesty. When the space names its videos (N66), the seek bar splits into the video’s chapters, named on hover and above the thumb on a touch, and a Captions button joins the player’s row (N67).

**Entry.** Someone opens the clean shared link without a qualifying owner cookie, and the video's content is ready and served (the normal case).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Untitled — Aug 21” | title | ratified | The video's title as plain text. This is the untouched default, with the date varying; any title the owner typed replaces it. |
| “Copy link” | button | ratified | N46 (settling the question this face carried): the same as the owner page's — the clean link. |
| “Copied” | status | ratified | Copy link's and Copy thumbnail's label for a moment after the click. |
| “Copy thumbnail” | button | ratified | N46: the newest picture made for the current title, wrapped in the link. Hidden until such a picture exists. A viewer stores nothing. |
| “Download” | button | ratified | Quiet and second-class; only once the single-file MP4 exists. |
| “made with shroom” | tooltip | ratified | The mark's hover label. |
| “Play” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's controls, named for screen readers and keyboards (icons on screen, never text) · copy table |
| “Pause” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's controls, named for screen readers and keyboards (icons on screen, never text) · copy table |
| “Seek” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's controls, named for screen readers and keyboards (icons on screen, never text) · copy table |
| “Full screen” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's controls, named for screen readers and keyboards (icons on screen, never text) · copy table |
| “Exit full screen” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's controls, named for screen readers and keyboards (icons on screen, never text) · copy table |
| “Playback speed” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The restyled player's speed and sound controls, named the same way (the speed button shows its rate, and its name reads “Playback speed, 1.5×”) · copy table |
| “Mute” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The restyled player's speed and sound controls, named the same way (the speed button shows its rate, and its name reads “Playback speed, 1.5×”) · copy table |
| “Unmute” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The restyled player's speed and sound controls, named the same way (the speed button shows its rate, and its name reads “Playback speed, 1.5×”) · copy table |
| “Captions” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The player's captions button, named for screen readers and keyboards (an icon on screen, never text); only where the video has captions (N67) · copy table, N67 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | secondary |  | `live` | N46. Copies the clean link; reads “Copied” for a moment. |
| Copy thumbnail | secondary |  | `live` | N46. The newest stored picture for the current title wrapped in the link, with the plain link as text; shown only once such a picture exists. |
| play | other |  |  | Minimal player, custom controls. |
| scrub bar | other |  |  |  |
| fullscreen | other |  |  |  |
| Download | secondary |  |  | Quiet and second-class; only once the single-file MP4 exists (R11). |
| the mark (small shroom icon) | other |  | `landing` | Bottom-right; hover “made with shroom”; links home. |
| Captions | other |  |  | N67. Only where the video has captions: turns them on and off, off by default and remembered on that browser. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `unavailable` | flow | storage switched off or mid-repair (N40) | provisional |  |

### `on-its-way`

web-page · web-page · **ratified** · board: RecipientOnItsWay · cites N27, U9

The link beat the upload. The title stays (the untouched placeholder if the owner never typed). Where the player will be, one line sits over a thin bar: the owner's upload seen from the other side (U9). The page refreshes itself; there is nothing for the viewer to do. Recipients also get this face while an offline stop's video hasn't arrived.

**Entry.** The shared link is opened before the upload has finished (the link beat the upload).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Untitled — Aug 21” | title | ratified | The title stays: the owner's title, or this untouched placeholder (the date varies) if the owner never typed. |
| “This video is on its way. The page refreshes itself — nothing to do.” | line | ratified | Shown where the player will be, over a thin bar. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `live` | auto | self-refresh — content ready | ratified |  |
| `on-its-way-stalled` | auto | no progress for a while | provisional |  |

### `on-its-way-untitled`

face · web-page · **provisional** · a face of `on-its-way` · cites N40, R10

Unknown ids and videos that aren't described yet render on-its-way without a title. This is indistinguishable by design: a bogus id leaks nothing (R10). It is also exactly what an offline stop's link shows before its details arrive. The dated placeholder appears once the details land. The spec doesn't name this face: `on-its-way-untitled` is a descriptive placeholder.

**Entry.** The page is opened for an id the space has never heard of, or for a video whose details haven't arrived yet (for example, an offline stop's link).

| Said | Role | Copy | Note |
|---|---|---|---|
| “This video is on its way. The page refreshes itself — nothing to do.” | line | ratified | on-its-way's ratified line, shown here without a title. The untitled rendering itself is N40. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `on-its-way` | flow | the details land — the dated placeholder appears | provisional |  |

### `on-its-way-stalled`

face · web-page · **provisional** · a face of `on-its-way` · cites N40

The upload has made no progress for a while, for example because the sender's Mac is asleep or offline. The line changes and the bar rests. The page promises nothing it can't know, and a mistyped link lands here too, by design. The id comes from the diagram's `on_its_way_stalled`; the inventory writes it as “`on-its-way`, stalled”.

**Entry.** on-its-way with no upload progress for the stall window (threshold to pin at scaffold). Unknown or mistyped ids also end up here.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This video isn't here yet — the page keeps checking and will play it the moment it arrives.” | line | provisional | The bar rests beneath. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `live` | flow | the sender's Mac is back | provisional |  |

### `unavailable`

web-page · web-page · **provisional** · cites N40

The space can't serve the bytes right now. The title stays if it's known, and the page says nothing about why: it isn't the viewer's business.

**Entry.** The space can't serve the bytes right now: its storage is switched off, or its page is mid-repair.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This video can't play right now — try again a little later.” | line | provisional |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `live` | flow | served again | provisional |  |

### `removed`

web-page · web-page · **ratified** · board: RecipientRemoved · cites N27, P2, B141

A removed video's link answers as a plain not-found page, HTTP 404, the embed the same: no title, no player, and nothing saying a video was ever there (B141, provisional: Bagrat wants to discuss it further). The mark stays, and the address lives forever. Before B141: the P2 tombstone, “This video was removed by its owner.”

**Entry.** The video was deleted (the tombstone, forever). Deleting happens in the Library, never from the page.

| Said | Role | Copy | Note |
|---|---|---|---|
| “There's no video at this link.” | line | provisional | The not-found line, 404 (B141). It replaces “This video was removed by its owner.” (ratified, P2). An id with no record still reads as on its way (P9), so the two differ; the words never say the video existed. |
| “made with shroom” | tooltip | ratified | The mark's hover label (line 1040); the mark stays on this face (line 1019). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| the mark (small shroom icon) | other |  | `landing` | The mark stays; links home. |

### `embed-face`

web-page · web-page · **provisional** · cites N40, §5.4, R4, R5

`…/<id>?embed=1`: the player alone, with no title and the mark small in its corner. It has the same on-its-way, unavailable and removed faces. This is what Notion's embed and any iframe get. The spec calls it “the embed face” but gives no backticked name: `embed-face` is a descriptive placeholder.

**Entry.** The page is requested with `?embed=1`, for example from Notion's embed or any iframe.

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| the mark (small, in its corner) | other |  | `landing` | Small mark in the corner. The spec doesn't restate its hover label for the embed. |

### `owner-page-cuts`

face · web-page · **provisional** · a face of `owner-page` · board: OwnerPageCuts · cites N65, N63, N66, N40, B21

Voice cuts (N65): when the speaker said “shroom cut” in a take, the owner page shows what was cut, under the player. A heading, a quiet note that only the owner sees it, then one row per cut: its time on the video, and what was said, struck through while it is cut, between a few words of what plays around it (up to six either side, never struck), with Put back (and, once put back, Cut again). A cue with nothing said again after it cuts the line before it too (OwnerPageCutUnsure): the row strikes both, the line and “shroom cut”, under an amber note that it isn't sure how far back the slip went, so check; its Put back brings the line back and keeps “shroom cut” out, and Cut again cuts the line again. Owner-mode as the title has it (N40): the video's maker and owner-class browsers. Viewers see only the cut video, never the strip or the words cut. A press saves at once and the player follows; a press that didn't save puts the row back as it was and says so on one line (OwnerPageCutRows). A cut whose edges were moved in the editor (N63) shows how long it is and where it now ends in place of its words, which would no longer be what's cut; Put back and Cut again work on it as before.

**Entry.** The record's edit list holds voice cuts (N63's list; kinds retake, cue-only and line-before-cue, landed by the Mac once the take is heard) and the browser is in owner-mode for this video. The strip shows on load, or, on a page already open, once the page's own poll sees the cuts land.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Cut where you said “shroom cut”” | heading | provisional | The strip's heading, with the scissors. Proposed 2026-10-18 (OwnerPageCuts); awaiting Bagrat. |
| “Only you see this.” | note | provisional | Quiet, at the heading's right: viewers never see the strip. Proposed 2026-10-18; awaiting Bagrat. |
| “Put back” | button | provisional | On a cut row: the slip plays again; on the unsure cut, the line before the cue plays again and “shroom cut” stays out (OwnerPageCutUnsure; Bagrat's pick, 2026-10-18). Proposed 2026-10-18; awaiting Bagrat. |
| “Cut again” | button | provisional | On a row that was put back. Proposed 2026-10-18; awaiting Bagrat. |
| “Not sure how far back the slip went — check this one.” | note | ratified | Above the unsure cut's row, in amber: the line before a cue with nothing said again after it, cut with the cue. Bagrat, 2026-10-18 (relayed by the orchestrator), after a take of his own on the early build: it should say it wasn't clear how far back the slip went, and ask to check. His pick of three ways on OwnerPageCutUnsure, 2026-10-18 (relayed by the orchestrator). |
| “Couldn't save that. Check your connection, then try again.” | status | provisional | Under the rows, in amber, after a press that didn't save; the row is back as it was. Proposed 2026-10-18; awaiting Bagrat. |
| “4.6 s, to 0:09” | line | ratified | In place of a cut's words once its edges were moved in the editor (N63: a cut's edges dragged, its `adjusted`): how long it is now and where it ends; the numbers vary. The words would no longer be what's cut. Approved as drawn, Bagrat 2026-10-18 (OwnerPageCutAdjusted). |
| “changed in the editor” | note | ratified | After the length, quieter: why the row shows no words. Approved as drawn, Bagrat 2026-10-18 (OwnerPageCutAdjusted). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Put back | secondary |  | `owner-page-cuts` | Saves at once: the cut no longer applies (the record's edit list, under If-Match, B21), the row's words are no longer struck, the button becomes Cut again, and the player follows. On the unsure cut it's the line's cut that no longer applies: “shroom cut” has a cut of its own, which stays, struck. |
| Cut again | secondary |  | `owner-page-cuts` | The same save, the other way. |

### `owner-page-cue-hint`

face · web-page · **provisional** · a face of `owner-page` · board: OwnerPageCueHint · cites N65, N66, N40, B203

The voice-cut hint (N65): in a space that listens, the owner page of a take with no voice cuts shows one quiet line under the player, where the strip would be, with the scissors: how to cut a slip by saying so. Got it puts it away for good in that browser, and so does a first take with voice cuts; never beside the strip, never while the cuts are on their way. Owner-mode as the title has it (N40); viewers never see it (OwnerPageCueHint).

**Entry.** The browser is in owner-mode for this video, the space listens (N66), the record has no voice cuts and none are on their way, and this browser hasn't put the hint away or shown a strip of voice cuts.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Slipped? Say “shroom cut”, then say the line again.” | note | provisional | Under the player, where the strip would be, with the scissors. OK'd by Bagrat 2026-10-18 (relayed by the orchestrator), with whichever cue word wins: “shroom cut”, his pick the same day (B203). |
| “Got it” | button | provisional | At the line's right: the hint goes, for good in this browser. Proposed 2026-10-18; awaiting Bagrat. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Got it | secondary |  | `owner-page` | The hint goes and stays gone in this browser (its own storage; where that can't be kept, for this page view). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `owner-page` | flow | Got it — the hint goes, for good in this browser | provisional | N65 |

### `hearing`

face · web-page · **provisional** · a face of `owner-page` · board: OwnerPageFinishing · cites N65, N66, N40

While the take is heard (N65 (7), N66): in a space that listens, once the upload has drained, the owner page's one line stays: the same words, its spinner turning, until the take's voice cuts land (the strip takes its place), the Mac finds none (it fades, and the hint shows in its place while that is due), or five minutes have passed since the take went live, by the page's own count. It never speaks of cuts: the same listening names the video and makes its captions and chapters. Edit comes in on the title row as the upload drains, beside it (OwnerPageFinishing, OwnerPageFinishingEnds). Owner-mode as the title has it (N40); viewers never see it.

**Entry.** The browser is in owner-mode for this video, the space listens (N66), the upload has drained, the take went live less than five minutes ago, its words don't wait for the next free day, and the Mac hasn't been through them yet: the status says the voice cuts are coming.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Finishing your video — the link already works, share it anytime.” | status | ratified | The upload's line (uploading), kept after the upload while the take is heard, its spinner turning. Bagrat, 2026-10-18 (relayed by the orchestrator): one indicator on the owner page, from the upload until the voice cuts land, “Finishing” his pick of its first word. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `owner-page-cuts` | auto | the take's voice cuts land — the strip takes the line's place | provisional | N65 |
| `owner-page-cue-hint` | auto | nothing to cut, or five minutes pass, and the hint is due — the hint takes the line's place | provisional | N65 |
| `fully-live` | auto | nothing to cut, or five minutes pass — the line fades | provisional | N65 |

#### Region 4's amendments

- **N40** (amends Region 4 beat 6 — the offline stop (ratified 2026-07-20; offline line kept by reference in N26)): An offline Mac can't load the page, so the paused popover becomes stopped-offline: the link, Copy link, and a page that opens by itself on reconnect, after any running take. The ratified offline line keeps its meaning in the popover's new wording. Recipients see on-its-way meanwhile. Awaiting ratification.
- **N40** (amends U2 — owner-cookie auth): The cookie names a member, not the space, and the Worker checks the video's maker against it. Owner-mode is by role: a member's own pages give the title only, and owner-class browsers get owner-mode, the mark switch included, on every page of the space. Awaiting ratification.
- **N40** (amends N27 — the recipient page, three faces (extends)): Adds the on-its-way-stalled face, the unavailable face, and the embed face (`?embed=1`), plus untitled on-its-way for unknown or not-yet-described ids. Awaiting ratification.
- **N21** (amends N1 — clipboard at stop): The tray's Copy last link is removed. The page's Copy link and the Library row are the two copy doors (beat 4). Ratified 2026-08-21.
- **N21** (amends U9 — where fully live lands): The tray's upload mirror becomes the config popover's status line (beat 5). Ratified 2026-08-21.
- **N33** (amends U10 — second-Mac join (as reflected in Region 4's cookie section)): Another browser or Mac re-keys by opening the page from that machine's Library: Open page is the owner-keyed door (N35). A new Mac joins through Add another Mac, owner-class. Awaiting ratification.

#### Region 4's open notes

- Law 8's deliberate local exception, marked “check it at ratification” in the spec: stopped-stranded's one filled button (Save to Movies) leads instead of sitting rightmost (lines 139-143).
- The region text never says whether Return triggers a button in stopped-offline (Copy link) or stopped-stranded (Save to Movies). The fixture sets return_default=true for both, inferred from Law 8 (“One default per surface, and Return triggers it”). Confirm at ratification; if wrong, those tests flip.
- On the owner page, Copy link is filled, but the title field arrives focused and Enter saves the title. The fixture gives Return to the field (Law 8: a focused field's Return belongs to its field) and marks Copy link return_default=false.
- Settled (N46, 2026-10-15): the recipient's live face shows Copy link — and Copy thumbnail — beside Download. Bagrat ruled yes: both buttons, with Download there too. (It used to read: “Does the recipient's live face show Copy link? … Check at ratification.”)
- The owner page section is marked copy ratified (N26, 2026-08-21), but it holds 2026-09-30 additions. “Clearing the title and leaving the field brings the previous title back” (line 936) cites N35. Owner-mode by role (941-944) and the upload-line faces (946-949) are N40. These are treated as provisional where they carry copy.
- Unfurl honesty (line 1024-1025): Linear was corrected on 2026-09-30 (R5.1) inside ratified region text, with no ledger row. The Notion line now points at the N40 embed face. Confirm at ratification.
- stopped-stranded's reason line is “the matching banner's reason line (N34 / Region 2)”. The member-revoked and space-gone lines map to N34's banners (lines 1578 and 1580) and are in the fixture. For “an owner's space whose page is gone”, the spec doesn't say which Region 2 line applies — the space-degraded banner (line 640: “Your space needs you — new videos wait safely on this Mac.”) or the owner-class-without-token banner (lines 659-661) — so the fixture asserts no reason line for that case. Pin it.
- Classification overlap: stopped-offline takes “anything that can't reach” the space's page (line 961), while stopped-stranded covers “an owner's space whose page is gone” (line 975). An online Mac whose space page is gone fails the reachability probe; the spec doesn't say which state wins.
- Transitions the spec doesn't draw. There is no exit from waiting-space (back to uploading or fully-live once the space is fixed). stopped-stranded has no exits (the take waits in the Library as “Waiting for your space” until it drains, is saved, or is discarded, line 982-983), and its click-away and Esc behavior is unstated. stopped-offline's click-away destination state isn't named. removed is entered only from [*]: no live → removed or on-its-way → removed. fully-live has no outgoing transitions.
- Line 1017 says a mistyped link lands in the stalled face, while line 1027 says an unknown id renders on-its-way without a title. The diagram draws on_its_way → on_its_way_stalled but nothing from the untitled face; presumably the untitled face inherits its parent's stall transition. Confirm.
- Embed face: “the same on its way, unavailable and removed faces” (line 1032), but those lines aren't restated for the embed. Whether on-its-way-stalled applies to the embed isn't said, and neither is the embed mark's hover label.
- on-its-way-stalled: whether the title stays isn't stated. on-its-way keeps the title, and unavailable keeps it “if known”.
- Beat 6's ratified line “You're offline — your link is ready and safe to share. The video follows the moment you're back online.” (line 921) is superseded. Its meaning now lives in stopped-offline's new wording, so tests should not assert the old line on any surface.
- Save to Movies after the click: the “Saved to Movies ✓” label and the quiet Discard… are specified only for the N34 banners (lines 1582-1586), not for the stopped-stranded popover.
- Character fidelity: UX-STATEMACHINE.md uses straight ASCII apostrophes (') throughout, with zero U+2019 in the file (verified). The fixtures keep the file's characters, while em dashes (—), ellipses (…) and ✓ are kept as written.
- The page's upload line is singular (“your link already works.”), while the config popover's status line (Region 3, line 777) is plural (“Waiting for connection — your links already work.”). This is probably deliberate, but it is a two-surface difference tests must respect.
- No Fat-marker status notes sit inside Region 4: the region was closed by N27.

#### Region 4's scaffold flags

- Fixture ids that aren't backticked scaffold names in the spec: owner-page, uploading, waiting-conn, fully-live, on-its-way-stalled and unavailable are kebab-cased from the Region 4 mermaid's snake_case names, using the same mapping as stopped_offline ↔ `stopped-offline`. waiting-space, owner-page-member, on-its-way-untitled and embed-face are descriptive placeholders for faces the spec describes but never names. Pin the scaffold names.
- One beat, two names: the Region 4 diagram draws it as `finalizing`, while Region 3 (line 863, bold-backticked) and the inventory name it `stopping` (finalize, measured 102 ms). The fixture uses `stopping`; the diagram node should be renamed to match.
- The reachability probe at stop: a short probe of the space's page, capped well under a second so a live stop never waits on it. Anything that can't reach the page takes stopped-offline.
- The member cookie: the stop opens `v.<name>.workers.dev/<id>?k=<one-time-key>`. The Worker validates k and sets a long-lived cookie that names the member (“likely HttpOnly” — verify). It then 302-redirects to the clean URL, so the key never rests in the address bar. Owner-mode is rendered server-side. The Worker checks the video's maker against the cookie, and owner-class sees all. The cookie bears metadata powers only (rename, mark toggle), never storage credentials.
- No copy affordance ever emits a keyed URL. Copy link (page, popover, Library) always yields the clean link.
- Thresholds to pin: the stall window for on-its-way-stalled, and the upload no-progress window.
- The deferred-page window after an offline stop: the page opens by itself only within the hour and only while shroom is still running. Only the newest of several offline stops opens. It waits for any running take to end.
- Media artifacts the pages depend on. Download appears only once the single-file MP4 exists (R11). A small teaser MP4 lets iMessage and Discord play inline. The player's first frame comes well under 1 s (R6.2), with custom controls.
- The embed route is `…/<id>?embed=1`: player only, with a small mark and the same on-its-way, unavailable and removed faces.
- Every page is `noindex` (P1). No passwords or expiry in v1. No captions surface, but the VTT-grade foundation stays as background machinery.
- The schema carries a sender-name field from day one, with no name UI in v1 (U3).
- The mark switch is one space-global metadata write, which the Worker reads at render. Rename is a metadata write, live worldwide on the next load (R4: strongly consistent; no redeploy).
- Docs carry the one honest cache note on rename: chat apps keep their unfurl cards for a while — already-posted cards keep the old title, new shares pick up the new one (R4.2).
- Unknown ids render on-its-way without a title, indistinguishable from a not-yet-described video (R10: a bogus id leaks nothing).
- Tombstone check before every upload resume (N35), so another Mac's delete stops this Mac's upload. This affects removed and uploading.
- The page's interface stays backwards-compatible across app versions (N37).

<a id="region-5"></a>

## Region 5 — the tail: library, settings, updates

*Region 5 — the tail: library, settings, updates, custom domain (UX-STATEMACHINE.md lines 1075-1281; structure ratified 2026-07-20; everything below designed 2026-09-30 in the autonomous sitting — N35, N36, N37 — awaiting Bagrat's ratification)*

### `library`

state · window · **provisional** · board: Library · cites N35, N5, N33, N25, N26, N28, N34, N41, U2, §5.6, R10, P13, N46, N68, N63

The Library window, thin by design (§5.6), built for two jobs: re-share an old video and clean up. It is a list, newest first. Each row shows a small poster, the title, then date · length; in Everyone, other members' rows add their admin-private label (N28; removed members keep it). Copy link is always visible; Copy thumbnail sits under it in the row's menu (N46), and Edit… after Open page (N63). Open page and Delete… show on hover or selection. State words appear only when a video isn't live (no word means live). A recovered take pins to the top under the amber dot, mirroring N25's banner. The full macOS idiom applies: click a title to rename it, double-click to open, ⌘C copies, the Delete key deletes, right-click opens a menu, ⌘/⇧-click multi-selects. Mine · Everyone shows only on owner-class Macs once someone else has a video here, and Mine is the default. Search is local and instant, over titles only. The window opens on the local cache, and the bucket refresh (R10 delta sync) runs behind it, sliding new rows in, including videos from your other Macs. Footer: the count only, with no sizes or prices (P13 parked). Scope: members see their own videos, owner-class Macs see the whole space. If another Mac deletes a video while this Mac is uploading it, the tombstone check at resume stops the upload and this row turns to 'Removed from the space' with Save to Movies · Discard…. Mocked row shape: '<title> — Aug 21 · 4:32' (+ ' · Mher' in Everyone).

**Entry.** A space exists, so config is reachable: idle-ready, not mid-take. The user clicks the Library (folder) icon in config's footer. One instance: clicking the icon again brings the open window forward. A window opened before a take stays open through it (N41's quiet-take law covers only what shroom would newly show). It is also restored at its derived state after an update relaunch if it was open when Update ran. The dock icon shows while it is open (Region 1 dock rule).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Library” | title | provisional | Window head in the mock (bold, unquoted). |
| “Search titles” | placeholder | provisional | Search field. Filters as you type over the titles this Mac knows. |
| “Mine” | other | provisional | Segment of the 'Mine · Everyone' control (italic in the mock). Only on owner-class Macs once someone else has a video here. Default. |
| “Everyone” | other | provisional | Second segment. Rows here add the member's label. |
| “Copy link” | button | provisional | On every row, always visible, outline style. |
| “Copied” | status | provisional | Copy link reads this for a moment after the click (N26's pattern, Law 10). The word itself is ratified on the owner page (N26). |
| “Copy thumbnail” | menu-item | ratified | In a row's menu (right-click), under Copy link (N46). The row's visible button stays Copy link alone. |
| “Open page” | link | provisional | Appears on hover or selection. |
| “Delete…” | link | provisional | Red text. Appears on hover or selection. |
| “Uploading — 40%” | status | provisional | Row state word with a thin bar under the row. The percentage varies. |
| “Waiting for connection” | status | provisional | Row state word. |
| “Waiting for your space” | status | provisional | Amber. Shown in the degraded and member faces (N34), with [ Save to Movies ] while the take can't go anywhere. |
| “Save to Movies” | button | provisional | Shown next to 'Waiting for your space' while the take can't go anywhere, and on a 'Removed from the space' row. |
| “Interrupted recording — 4:32 saved” | status | provisional | Recovered-take row, pinned to the top under the amber dot and mirroring N25's banner. The length varies. |
| “Get link” | button | provisional | Filled. On the recovered-take row. |
| “Discard…” | link | provisional | On the recovered-take row, and on a 'Removed from the space' row. |
| “Removed from the space” | status | provisional | Row state on the Mac whose upload was stopped by another Mac's delete (tombstone found at resume). |
| “Copy link” | menu-item | provisional | Right-click menu item 1 of 4. |
| “Open page” | menu-item | provisional | Right-click menu item 2 of 4. |
| “Rename” | menu-item | provisional | Right-click menu item 3 of 4. |
| “Delete…” | menu-item | provisional | Right-click menu item 4 of 4. |
| “14 videos” | status | provisional | Footer count, the only footer content. The number varies and stays a figure (Law 16, compact facts). The singular form is not given. |
| “Uploading” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library, a row uploading from another Mac · copy table |
| “1 video” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library footer, one video · copy table |
| “Aug 21” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library's dates · copy table |
| “Aug 21 · 4:32” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library's dates · copy table |
| “Six videos might be worth deleting — they take 11 GB.” | line | ratified | N68's line above the rows, when the suggestions hold at least one video and Suggest videos to clean up is on. The count varies (a word up to nine, Law 16), and so does the size, the suggestions' total. One suggestion reads 'One video might be worth deleting — it takes 3.2 GB.' |
| “Review…” | button | ratified | Opens Suggested to clean up (N68). |
| “Not now” | tooltip | ratified | The ✕ at the line's end, named for VoiceOver. Hides the line until the suggestions change (N68). |
| “Edit…” | menu-item | ratified | N63; a live row's menu, after Open page |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | secondary |  | `library` | Outline, always visible. Also ⌘C on a selected row and right-click → Copy link. Reads 'Copied' for a moment. Stays live offline and when the space can't be reached. |
| Copy thumbnail | other |  | `library` | N46. Right-click → Copy thumbnail, under Copy link: the same clipboard as the page's (the picture wrapped in the link, and the plain link). Live rows only. |
| Open page | text-link |  | `owner-page` | On hover or selection. Also double-click a row and right-click → Open page. Opens the owner-keyed URL (U2): the page arrives in owner-mode and the default browser gets the cookie, which is also the door for a new browser or a new Mac. |
| Edit… | other |  | `editor` | N63. Right-click → Edit…, after Open page. Live rows only, in this Mac's scope. Greys out offline and while a take is open (law 4), as Rename does. One editor at a time: Edit… while it is open brings it forward, saying why when it holds another video (N69's line). |
| Delete… | red-text |  | `delete-confirm` | On hover or selection. Also the Delete key, right-click → Delete…, and ⌘/⇧-click several rows then Delete… once. Greys out offline. |
| Rename (click a title) | field | ↵ | `library` | Renames in place. Enter or click-away saves (the field's own Return, Law 8). Esc cancels. An emptied title brings the previous one back, never blank. A metadata write, live on the next load (Region 4 Rename). Greys out offline. |
| Search titles | field |  |  | Filters as you type, local and instant. Return behaviour not stated. |
| Mine · Everyone | other |  |  | Segmented scope control, owner-class only, once someone else records here. |
| Save to Movies | secondary |  |  | Stranded-take rescue (N34): copies to ~/Movies. Live even when the space can't be reached. |
| Get link | filled-default |  | `stopping` | Recovered-take row. Its one filled button leads (Law 8's single-action pattern, mirroring the saved-take banner). Return is not stated for the Library window. |
| Discard… (recovered take) | red-text |  | `discard-confirm` | N25's confirm, not the delete sheet: no link was ever handed out, so no tombstone. Red per Law 9 (the row text doesn't restate the colour). |
| Discard… (Removed from the space row) | red-text |  |  | The take on this Mac is still its maker's to keep or toss (law 2). Which confirm it opens is not stated. |
| ✕ (close window) | close |  |  | Ordinary window: closing it takes the dock icon away unless another window is open (Region 1 dock rule: dock iff a window is open; ✕ ≡ the retreat, line 1717). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `discard-confirm-saved-take` | flow | Discard… on the recovered-take row | provisional |  |
| `stopping` | flow | Get link on the pinned recovered-take row (N25's normal stop sequence from the saved file) | provisional |  |
| `owner-page` | flow | Open page / double-click a row / right-click Open page | provisional |  |
| `library-empty` | flow | no videos in this Mac's scope | provisional |  |
| `library-no-match` | flow | search text matches no title | provisional |  |
| `library-offline` | flow | Mac goes offline | provisional |  |
| `library-unreachable` | flow | member-revoked or space-gone (N34) | provisional |  |
| `delete-confirm` | side | Delete… · Delete key | provisional |  |
| `library-cleanup` | flow | Review… | ratified | N68 |
| `editor` | side | Edit… | ratified | N63 |
| `editor-getting` | side | Edit… | ratified | N63 |

### `library-empty`

face · window · **provisional** · a face of `library` · cites N35

Empty Library: one line pointing at the mushroom.

**Entry.** Library open and there are no videos in this Mac's scope.

| Said | Role | Copy | Note |
|---|---|---|---|
| “No videos yet — click the mushroom in your menu bar to record your first.” | line | provisional |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | a video exists (derived, so the face clears by itself — Law 1) | provisional |  |

### `library-no-match`

face · window · **provisional** · a face of `library` · cites N35

The search matches no title.

**Entry.** Library open, search text non-empty, and no known title matches.

| Said | Role | Copy | Note |
|---|---|---|---|
| “No titles match “502”.” | line | provisional | “502” is the typed search text, shown inside curly quotes. It varies. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Search titles | field |  |  | Editing or clearing the search returns to the list. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | search text changed so titles match, or cleared | provisional |  |

### `library-offline`

face · window · **provisional** · a face of `library` · cites N35

Offline: the list this Mac remembers, with one quiet line on top. Copy link stays live because an address never changes. Rename and Delete… grey out.

**Entry.** Library open while the Mac is offline.

| Said | Role | Copy | Note |
|---|---|---|---|
| “You're offline — showing what this Mac remembers. Renaming and deleting wait until you're back.” | line | provisional | A quiet line at the top of the window. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | secondary |  |  | Stays live. |
| Rename (click a title) | field |  |  | Greyed. |
| Delete… | red-text |  |  | Greyed. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | back online (derived — Law 1) | provisional |  |

### `library-unreachable`

face · window · **provisional** · a face of `library` · cites N35, N34

Member-revoked or space-gone: the list this Mac remembers, and only Copy link and Save to Movies stay live.

**Entry.** Library open on a member Mac in member-revoked or space-gone (N34).

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | secondary |  |  | Live. |
| Save to Movies | secondary |  |  | Live. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | the cause clears (derived — Law 1, N34) | provisional |  |

### `editor`

state · window · **ratified** · board: EditorCuts · cites N63, B195, B197, B199, N69

The editor (N63): a window of its own, titled “Edit — <the video's title>”, one at a time. Top to bottom: the video, playing the edit (cuts skipped, held to the trim); the play button and the time, edited position / edited length, with “was <the recording's length>” once the edit changes it; on the right the hint and Undo · Redo; then the timeline, the whole recording's sound as a waveform with the playhead: two yellow ends that drag to trim, a drag across the sound that selects a part with Cut <its length> under it, every cut red and hatched (clicking one offers Put back <its length>), every suggestion not yet taken dashed amber (clicking one offers Cut <its length>, as a selection does); either edge of a cut or a suggestion drags to move it, the pointer ↔ over it, its length shown live under it and the picture at the edge as it goes, stopping at the next cut or suggestion, the trim's end, a tenth of a second from its other edge, or where a second would be all that's left, one step for Undo (B195 (g)), and every marker (N64) a flag at its time. The footer: the line that the link stays the same, Revert to original and Done. The waveform fills in once the sound is read; until then the timeline is empty and the ends wait. Every change can be undone, Put back and Revert included; the changes stay in the window until Done saves them. While an earlier edit's original is still kept, the time it has left sits by Revert to original; once it is gone, Revert to original only undoes what changed since the window opened. While it is open, no edit becomes a video behind it on this Mac: the upkeep, and on the Mac holding the token the cleanup sweep, wait until the window closes (B199). The owner page's Edit in shroom (N69) opens this same window; while it holds another video, that press, or Edit… in the Library, brings it forward as it is, one at a time, the footer line saying why until the next change.

**Entry.** Edit… on a live row in the Library (N63), when the video is on this Mac: its take was recorded here and is still here, or it came from the space before. One instance: Edit… again brings the open editor forward, on the video it holds. A take started while it is open leaves it open (law 4 covers only what shroom would newly show). The dock icon shows while it is open.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Edit — Setting up shroom in two minutes” | window-title | ratified | “Edit — ” and the video's title |
| “1:16 / 3:39” | line | ratified | the edited position / the edited length; varies |
| “was 4:32” | line | ratified | the recording's own length; only when the edit changes the length |
| “Drag the yellow ends to trim. Drag across the sound to cut a part.” | line | ratified |  |
| “Play” | label | ratified | the play button's name on hover and to VoiceOver; “Pause” while playing |
| “Pause” | label | ratified |  |
| “Undo” | label | ratified | the button's name on hover and to VoiceOver; also ⌘Z |
| “Redo” | label | ratified | also ⇧⌘Z |
| “Cut 12.0 s” | button | ratified | under a selected part, and under a dashed suggestion that was clicked, its length then (Bagrat, 2026-10-19: a suggestion's chip says what a selection's does); the length varies |
| “Put back 12.0 s” | button | ratified | under a cut that was clicked; the length varies |
| “Your link stays the same. Viewers see the edit once it's saved.” | line | ratified |  |
| “You're editing another video — press Done, then Edit again.” | line | provisional | In the footer line's place when the owner page's Edit in shroom (N69), or Edit… in the Library, asks for another video while the editor holds this one: the window comes forward as it is, and the line goes with the next change. Approved as drawn (Bagrat, 2026-10-18); Edit… says it too since 2026-10-19, when it was found saying nothing. · N69 |
| “The original is kept one more day” | line | ratified | by Revert, only while an earlier edit's original is kept (B197); never more than it has left (B199): “two more days” from 47 hours, “one more day” from a day, “<n> more hours” from 2 hours (whole hours, rounded down; two to nine in words, figures from 10, law 16), “one more hour” from an hour, and “less than an hour” under one |
| “Revert to original” | button | ratified |  |
| “Done” | button | ratified | filled; Return |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Play / Pause | other |  |  | Space too. Plays the edit: the cuts are skipped and it holds to the trim. |
| drag a yellow end | other |  | `editor` | trims; a click on the timeline moves the playhead |
| drag across the sound | other |  | `editor` | selects a part; Cut <length> cuts it (the Delete key too) |
| Cut | secondary |  | `editor` | red, and skipped as it plays |
| Put back | secondary |  | `editor` | after a click on a cut |
| Cut | secondary |  | `editor` | after a click on a dashed suggestion, its chip reading as a selection's (Bagrat, 2026-10-19): it's cut, red |
| drag a cut's edge | other |  | `editor` | either edge of a red cut or a dashed suggestion (↔ under the pointer): it moves that edge, its length live; it stops at the next cut or suggestion, the trim's end, a tenth of a second from its other edge, or where a second is all that's left. A suggestion stays offered. One step for Undo (B195 (g)) |
| Undo · Redo | other |  | `editor` | ⌘Z · ⇧⌘Z; every change made in the window |
| Revert to original | secondary |  | `editor` | puts back every cut and both ends; undoable |
| Done | filled-default | ↵ | `editor-saving` | Return. With nothing changed it closes the window at once. ✕ is Done. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `editor-saving` | flow | Done | ratified | N63, B199 |
| `library` | flow | Done | ratified | N63 |

### `editor-getting`

state · window · **ratified** · board: EditorStates · cites N63, B195

The editor while the video comes from the space: it isn't on this Mac (another Mac recorded it, or its take is gone). The time reads 0:00 / the recording's length; where the timeline goes, “Getting the video from your space…” over a bar that fills; Done waits.

**Entry.** Edit… on a live row whose video isn't on this Mac.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Getting the video from your space…” | line | ratified |  |
| “Your link stays the same. Viewers see the edit once it's saved.” | line | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Done | filled-default |  |  | greyed until the video is here |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `editor` | flow | the video is here and its sound read | ratified | N63 |
| `editor-cant-get` | flow | the space didn't hand the video over | ratified | N63 |

### `editor-cant-get`

state · window · **ratified** · board: EditorStates · cites N63, B195

The editor when the video couldn't come from the space (offline, or the space didn't answer): one line and Try again where the timeline goes; Close closes the window.

**Entry.** The fetch in editor-getting failed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Couldn't get the video. Check your connection, then try again.” | line | ratified |  |
| “Try again” | button | ratified |  |
| “Close” | button | ratified | filled; Return |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Try again | secondary |  | `editor-getting` |  |
| Close | filled-default | ↵ |  | closes the window; ✕ and Esc too |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `editor-getting` | flow | Try again | ratified | N63 |
| `library` | flow | Close | ratified | N63 |

### `editor-saving`

state · window · **ratified** · board: EditorStates · cites N63, B195, B199

Done pressed with changes: the footer says the edit is saving and Done reads Saving…, greyed. Saved, the share page plays the edit from its next load, and the window goes on to make the final video (or closes, when nothing is left out any more). A quit or an update asked meanwhile waits on it: it goes on once the edit is saved, not waiting for the final video, and is dropped if the save doesn't land (B199).

**Entry.** Done (or ✕) in editor with changes, or Done again in editor-save-failed. Also: Quit or Update while the edit holds changes not saved yet (B199).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Saving your edit…” | line | ratified |  |
| “Saving…” | button | ratified | Done, greyed while it saves |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | saved, and nothing is left out any more (the edit was reverted): the editor closes | ratified | N63 |
| `editor-save-failed` | flow | the save didn't go through | ratified | N63 |
| `editor-making` | flow | saved with something left out: the final video is made | ratified | N63, B197 |
| `editor-save-refused` | flow | the space turned the save down: the video is gone, or another Mac made an edit the video meanwhile | ratified | N63, B199 |

### `editor-save-failed`

state · window · **ratified** · board: EditorStates · cites N63, B195, B199

The save didn't go through (offline, or the space didn't answer): the window stays open with every change in it, the footer line says so in amber, and Done tries again. A save the space turns down is editor-save-refused's (B199).

**Entry.** A save from editor-saving failed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Couldn't save your edit. Check your connection, then press Done again.” | line | ratified | amber, in the footer's place |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Done | filled-default | ↵ | `editor-saving` | tries the save again |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `editor-saving` | flow | Done | ratified | N63, B199 |

### `editor-save-refused`

state · window · **ratified** · board: EditorStates · cites N63, B199

The space turned the save down: the video is gone (deleted from another Mac), or another Mac made an edit the video while this window was open, so the edit was made on a video that no longer plays. Nothing can be saved over it: in the save-failed line's place, one amber line says so; Revert to original is gone, the changes can't be stepped, and Close stands in Done's place. Close, Return, Esc and ✕ close the window, and nothing is saved.

**Entry.** A save from editor-saving the space turned down: the video is gone, or another Mac made an edit the video while the editor was open.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This video changed since you opened it, so this edit can't be saved.” | line | ratified | amber, in the save-failed line's place |
| “Close” | button | ratified | filled, in Done's place; Return |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | filled-default | ↵ | `library` | closes the window, and nothing is saved; ✕ and Esc too |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Close | ratified | N63, B199 |

### `editor-making`

state · window · **ratified** · board: EditorAfterSave · cites N63, B197

Saved: the final video is being made on this Mac and put up beside the original. Where the timeline was, “Making the final video… 40%” over a bar that fills; the footer says the link already plays the edit and that the window can be closed. Close (or ✕) closes it and shroom finishes in the background; the question that would follow is then answered Keep for 2 days.

**Entry.** The save landed in editor-saving with something left out.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Making the final video… 40%” | line | ratified | the share grows |
| “The link already plays your edit. You can close this window: shroom finishes it.” | line | ratified |  |
| “Close” | button | ratified | filled; Return |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | filled-default | ↵ | `library` | the final video is still made; the original is then kept for 2 days |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `editor-keep-original` | flow | the final video is up, named in the video's record | ratified | N63, B197, B199 |
| `editor-making-failed` | flow | it couldn't be made or put up — or it was dropped, never named: the video's record moved on first, a newer save landing (B199) | ratified | N63, B197, B199 |
| `library` | flow | Close | ratified | N63, B197 |

### `editor-making-failed`

state · window · **ratified** · board: EditorAfterSave · cites N63, B197, B199

The final video couldn't be made or put up (offline, the space didn't answer, the encoder failed), or a newer save landed first and it was dropped (B199): one amber line and Close. The link keeps playing the edit over the original; shroom tries again later.

**Entry.** Making the final video failed in editor-making. Also: it was made but dropped, never named — the video's record moved on first, a newer save landing from this Mac or another (B199).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Couldn't make the final video. The link plays your edit anyway, and shroom tries again later.” | line | ratified | amber |
| “Close” | button | ratified | filled; Return |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | filled-default | ↵ | `library` |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Close | ratified | N63, B197 |

### `editor-keep-original`

state · window · **ratified** · board: EditorAfterSave · cites N63, B197

The final video is up: the window asks whether to delete the original. One card: “Delete the original?”, a line that the edit is the video now and the original is only kept for reverting, then Keep for 2 days and Yes, delete, which leads (filled, as Bagrat asked) but is never the Return default: Return does nothing here (law 9; DEVIATIONS #55). Yes, delete makes the edit the video for good and deletes the original now; Keep for 2 days keeps it, and shroom deletes it two days on. ✕ or Esc is Keep for 2 days.

**Entry.** The final video landed while the editor was open.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Delete the original?” | title | ratified |  |
| “Your edit is the video now. The original is only kept so you can revert to it.” | line | ratified |  |
| “Keep for 2 days” | button | ratified |  |
| “Yes, delete” | button | ratified | filled and leading; never the Return default (law 9; DEVIATIONS #55) |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Yes, delete | filled-default |  | `library` | the edit becomes the video; the original's files are deleted. Filled, as Bagrat asked, but a click only: Return does nothing (law 9; DEVIATIONS #55) |
| Keep for 2 days | secondary |  | `library` | deleted two days on, by whichever of the space's Macs is open first (B197) |
| Esc / ✕ | other |  | `library` | ≡ Keep for 2 days |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Yes, delete · Keep for 2 days | ratified | N63, B197 |

### `delete-confirm`

sheet · sheet · **provisional** · a face of `library` · board: DeleteSheet · cites N35, N23, P2, R10, N5

Delete confirmation sheet under N23's law: no Return default, Esc ≡ Cancel, and the destructive button red-outlined, rightmost, never filled. A video still uploading gets the same sheet, and its upload stops. Behind the sheet the P2 tombstone is written first, then the bytes are deleted (R10's crash-safe order). When the upload runs on another Mac (a member's, or your other Mac), that Mac checks for the tombstone before every resume and stops, and its Library row turns to 'Removed from the space' with Save to Movies · Discard…. A recovered take's Discard… does not use this sheet.

**Entry.** Online, and Delete… is invoked on a published video in this Mac's scope (own video, or any video on an owner-class Mac): via the red text, the Delete key, the right-click menu, or a multi-selection. Not for a recovered take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Delete “Where the 502s come from”?” | sheet-title | provisional | Bold. The video's title varies and is shown in curly quotes. |
| “It's removed everywhere. The link stays, and says the video is gone.” | sheet-body | provisional |  |
| “Cancel” | button | provisional |  |
| “Delete” | button | provisional | Red-outlined, rightmost, never filled. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `library` | Esc ≡ Cancel. No Return default anywhere on this sheet (N23, Law 9). |
| Delete | destructive |  | `library` | Red-outlined, rightmost, never filled. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `removed` | fail | Delete — the link stays, says the video is gone | provisional |  |
| `library` | flow | Cancel or Esc | provisional |  |

### `delete-confirm-other-member`

face · sheet · **provisional** · a face of `delete-confirm` · cites N35, N5

Owner-class Mac deleting another member's video: the sheet gains one more line naming whose video it is.

**Entry.** Owner-class Mac. The video being deleted was made by another member.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This is Mher's video.” | sheet-body | provisional | Added to the parent sheet's lines. 'Mher' is the member's admin-private label and varies. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `library` | Inherited from the parent sheet. |
| Delete | destructive |  | `library` | Inherited from the parent sheet. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Cancel or Esc | provisional |  |

### `delete-confirm-several`

face · sheet · **provisional** · a face of `delete-confirm` · cites N35

Several rows selected (the cleanup job): one sheet for all of them, with plural copy.

**Entry.** Several rows selected with ⌘/⇧-click, then Delete… once.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Delete six videos?” | sheet-title | provisional | Bold. The count varies and is written as a word ('six', Law 16 — one to nine are words). The wording above nine is not given. |
| “They're removed everywhere. Their links stay, and say the videos are gone.” | sheet-body | provisional |  |
| “Delete 14 videos?” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Library delete sheet, ten or more · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `library` | From the parent sheet's pair (line 1128). |
| Delete | destructive |  | `library` | From the parent sheet's pair. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | Cancel or Esc | provisional |  |

### `settings`

state · window · **provisional** · cites N36, N28, N12, N5, N33, N42, N38

One standard macOS settings window with toolbar tabs General · Space · Members. Members appears only on owner-class Macs (N5, N33). The window opens on the last tab used (General the first time). One instance. ✕ ≡ Close (N12). Resolves N28's open placement.

**Entry.** A space exists, so config is reachable: not mid-take. The window opens from any of these doors: (a) the gear in config's footer; (b) all-set's Invite someone, which morphs the onboarding window into Settings on Members; (c) space-degraded's Reconnect your space, which opens Space in the repair mood (Region 2); (d) recording-off's Turn it back on, which opens Space with row six current at perm-screen (Region 3). It is also restored at its derived state after an update relaunch. A window opened before a take stays open.

| Said | Role | Copy | Note |
|---|---|---|---|
| “General” | other | provisional | Toolbar tab. |
| “Space” | other | provisional | Toolbar tab. |
| “Members” | other | provisional | Toolbar tab, owner-class Macs only. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| General | other |  | `settings-general` | Toolbar tab. |
| Space | other |  | `settings-space` | Toolbar tab. |
| Members | other |  | `settings-members` | Toolbar tab, owner-class only. |
| ✕ | close |  |  | ≡ Close (N12). The window closes; the dock icon goes unless another window (e.g. Library) is still open (dock iff a window is open). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `settings-general` | flow | first open ever | provisional |  |
| `settings-space` | flow | open (any later time) — the last tab used | provisional |  |
| `settings-space-repair` | flow | space-degraded → Reconnect your space | provisional |  |
| `settings-members` | flow | open (any later time) — the last tab used | provisional |  |

### `settings-general`

face · window · **provisional** · a face of `settings` · board: SettingsGeneral · cites N36, U4, N8, N37, U7, B135, B180, N53, N62, N68

The General tab: U4's login checkbox at home at last (it reads the system's truth: if the login item is removed in System Settings, the box shows unchecked), the update row mirroring config's banner, and the help row: Copy diagnostics and Report a problem… (B180), with GitHub's new-issue link under it as the public door (N8). Past this Mac's macOS, General carries the only update line: no banner, no dot.

**Entry.** Settings window on the General tab (the default on first open). The shroom command's settings link (N62) opens it too, the switch's row lit for a moment.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Start shroom when you log in” | field-label | provisional | Checkbox label (☑). Verbatim the ratified welcome checkbox (Region 1, copy ratified 2026-07-25, line 324). Its Settings home is N36's and not yet ratified. |
| “Updates” | field-label | provisional | Row label of the update row. |
| “shroom 1.2.0 — up to date.” | status | provisional | The version varies. |
| “shroom 1.3.0 is ready.” | status | provisional | Mirrors config's update-ready banner. The version varies. |
| “Update” | button | provisional | Filled. |
| “What's new” | link | provisional |  |
| “Something off?” | field-label | ratified | Row label of the help row: Copy diagnostics and Report a problem… sit at its end (B180). Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Copied” | button | ratified | Copy diagnostics reads Copied for a moment after its click, as Copy link does (B180). Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Or tell us in public, on GitHub.” | line | ratified | Under the card. 'GitHub' is the new-issue link prefilled with the version (N8), with no log in it. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Sent — thank you. If you left your email, you’ll hear back.” | status | ratified | After a report went through (B180); until the window closes. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “shroom 1.4.0 needs macOS 15 — this Mac stays on 1.3.2.” | status | provisional | Past this Mac's macOS: the update row reads this, with no banner and no dot. The versions and the macOS number vary. |
| “You can do this once your recording is finished.” | line | provisional | Law 4: mid-take, Update greys with this line (only possible if the window was open before the take). |
| “shroom 1.2.0.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Settings · General, nothing known about a newer version (no check answered yet, or one couldn't run) · copy table |
| “Uninstall shroom…” | link | ratified | Quiet text at the tab's foot. Greys mid-take (law 4). |
| “Your coding agent” | field-label | ratified | The card's label, under the help row (N62). Ratified by Bagrat, 2026-10-18: as built; he'll tweak the card's words once he uses it. · N62 |
| “Use shroom on my Mac: read https://shroom.am/cli and follow it.” | line | ratified | The prompt itself, in a code well; what Copy puts on the clipboard (N62). Ratified by Bagrat, 2026-10-18: as built; he'll tweak the card's words once he uses it. · N62 |
| “Copy” | button | ratified | Copies the prompt; reads Copied for a moment after its click, as Copy diagnostics does (law 10). Ratified by Bagrat, 2026-10-18: as built; he'll tweak the card's words once he uses it. · N62 |
| “Read it first” | link | ratified | Opens shroom.am/cli in the browser: what the agent will read, word for word. Ratified by Bagrat, 2026-10-18: as built; he'll tweak the card's words once he uses it. · N62 |
| “Paste it into your coding agent. It can find, share and tidy your videos, and asks you before it deletes anything.” | line | ratified | Under the prompt. Ratified by Bagrat, 2026-10-18: as built; he'll tweak the card's words once he uses it. · N62 |
| “Let it start and stop recordings” | field-label | ratified | A switch, off by default (Bagrat, 2026-10-18). While it's off, the app refuses the command's recording asks, and the refusal's link (`shroom://settings/agent`) opens General here with this row in view and lit for a moment, never flipped. Ratified by Bagrat, 2026-10-18, relayed by the orchestrator. · N62 |
| “Your videos” | field-label | ratified | The group's heading (N68). |
| “Count plays” | field-label | ratified | Switch, on by default. The space's: owner-class Macs change it; elsewhere it shows greyed (N68). |
| “How often each video is played and how far, kept in your Cloudflare account. No cookies, nothing about who watched.” | line | ratified |  |
| “Couldn't change that just now — try again in a moment.” | line | ratified | Under Count plays, when the space didn't take the switch (law 12); the switch reads as the space has it. |
| “Suggest videos to clean up” | field-label | ratified | Switch, on by default; this Mac's (N68). |
| “In the Library, videos that might be worth deleting: rarely watched lately, and taking real room.” | line | ratified |  |
| “Space used” | field-label | ratified |  |
| “34 GB — about 36¢ a month past Cloudflare's free 10 GB” | status | ratified | Past the free 10 GB; the size and the cents vary (B212). |
| “8.2 GB — inside Cloudflare's free 10 GB” | status | ratified | Inside the free 10 GB; the size varies (B212). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Start shroom when you log in | other |  |  | Checkbox. Reflects and sets the system login item, and reads the system's truth. |
| Update | filled-default |  | `updating-app` | Return behaviour in Settings is not stated. Greys mid-take with “You can do this once your recording is finished.” (Law 4). |
| What's new | text-link |  |  | Opens the release notes in the browser. |
| Copy diagnostics | secondary |  |  | Puts the report, without a description, on the clipboard: the version, macOS, the chip and memory, the setup timeline and the last 200 log lines, cleaned (B180). Law 10: only on this click. |
| Report a problem… | secondary |  | `report-problem` | Opens the report sheet (B180). |
| GitHub | text-link |  |  | The new-issue link, prefilled with the version and nothing else (N8, B180). |
| Uninstall shroom… | text-link |  | `uninstall-confirm` | Opens the Uninstall sheet. Greys mid-take (law 4). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `settings-space` | flow | the Space tab | provisional | N36 |
| `updating-app` | flow | Update (an update is ready, not mid-take) | provisional |  |
| `uninstall-confirm` | side | Uninstall shroom…, not mid-take | provisional | B135 |
| `report-problem` | side | Report a problem… | provisional | B180 |

### `settings-space`

face · window · **provisional** · a face of `settings` · board: SettingsSpace · cites N36, N42, U3, N29, N31, law 4, B159, N66

The Space tab: one tab with a mood per kind of Mac (inspection · repair · owner-class-without-token · member). Beneath the rows, by role: the mark switch (owner-class only — a space setting, U3) and the exits as quiet text (Disconnect this Mac… for owners, Leave this space… for members). Row six is live on every Mac. A revoked grant un-ticks it (“screen recording is off”), with Turn it back on → the permissions chapter at exactly the missing beat, in this window. A denied microphone ticks the row with the fact, plus a quiet Turn on the microphone that opens System Settings' Microphone pane (a denied mic never blocks). Mid-take both exits grey (law 4). After Disconnect or Leave, the ~/.shroom secrets are wiped and the app returns to idle-unprovisioned; the login item stays as it was. No invite door shows (B159): an invite link pasted anywhere in the tab (⌘V) opens its invite, as the page's Open shroom does. On the Mac that holds the setup key, a second switch turns the space's AI on for new videos — titles, chapters, captions and voice cuts (N66) — with one line on what it does and costs; turning it on or off re-wires the page.

**Entry.** Settings window on the Space tab. Which mood shows is derived from the kind of Mac and the probe results.

| Said | Role | Copy | Note |
|---|---|---|---|
| ““made with shroom” on your pages” | field-label | provisional | Label of the mark switch, owner-class only. The spec writes the curly quotes around 'made with shroom' inline, with the rest of the label unquoted, so it is ambiguous whether the quotes render. Kept as written. |
| “The small mark on every video page. You can flip it right on a page, too.” | line | provisional | Description beside the switch. |
| “Disconnect this Mac…” | link | provisional | Quiet text, owners only. |
| “Leave this space…” | link | provisional | Quiet text, members only. |
| “→ Letting macOS allow recording — screen recording is off” | status | provisional | Row six, un-ticked by a revoked grant (→ = the current-row glyph). |
| “Turn it back on” | button | provisional | Filled. Same label as the recording-off banner (N38). |
| “— microphone off for now” | status | provisional | Row-six fact when the mic is denied (the row stays ticked). The full row reads “✓ Letting macOS allow recording — microphone off for now” (Region 3, line 730). |
| “Turn on the microphone” | button | provisional | Quiet. |
| “You can do this once your recording is finished.” | line | provisional | Mid-take, both exits grey with this line (law 4). |
| “AI for new videos” | field-label | ratified | N66. Label of the AI switch, one switch for every AI feature (voice cuts, N65, included), on the Mac that holds the setup key only (it re-wires the page). Bagrat approved the board AiSettings as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Titles, chapters and captions from what’s said, and your “shroom cut” slips cut — on your own Cloudflare account. Free for about 2½ hours of new video a day; more is about 4¢ per video hour on Cloudflare’s $5 plan.” | line | ratified | N66. Description beside the AI switch: the one cost line, also in ai-confirm. Bagrat approved the board AiSettings as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Couldn't change that just now — try again in a moment.” | line | provisional | N66. In place of the AI switch's line when a re-wiring didn't go through, until the next try or the next visit (law 12); the switch is as the page has it. · N66 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| made with shroom switch | switch |  |  | Owner-class only. A space-global setting (U3), also flippable on a page. |
| AI for new videos switch | switch |  |  | N66. Only on the Mac that holds the setup key; one switch for every AI feature, voice cuts included. On re-wires the page with Cloudflare's AI and its schedule; off takes them away. Greys while the re-wire runs and mid-take (law 4). |
| ⌘V (an invite link, pasted anywhere in the tab) | other |  | `join-switch-confirm` | No door shows (B159). A link to this Mac's own space goes to join-already-here instead. Into the join machine (Region 6). Return behaviour not stated. |
| Turn it back on | filled-default |  | `perm-screen` | The permissions chapter at exactly the missing beat, in this window. Only shown when the screen grant is revoked. Return behaviour in Settings is not stated. |
| Turn on the microphone | secondary |  |  | Quiet. Opens System Settings' Microphone pane. |
| Disconnect this Mac… | text-link |  | `disconnect-confirm` | An owner-class Mac without the token gets disconnect-confirm-no-token. Quiet text, owners. Greys mid-take (law 4). Which sheet opens depends on whether this Mac holds the token. |
| Leave this space… | text-link |  | `leave-confirm` | Quiet text, members. Greys mid-take (law 4). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-screen` | flow | Turn it back on (screen grant revoked) | provisional |  |
| `settings-space-inspection` | flow | setup Mac (holds the token), all probes pass | provisional |  |
| `settings-space-repair` | flow | token-holding Mac, a re-probe un-ticks a row (or space-degraded → Reconnect your space) | provisional |  |
| `settings-space-no-token` | flow | owner-class Mac without the token (joined via Add another Mac) | provisional |  |
| `settings-space-member` | flow | member's Mac | provisional |  |
| `disconnect-confirm` | side | Disconnect… · Leave… | provisional |  |
| `disconnect-confirm-no-token` | flow | Disconnect this Mac… (owner-class, no token) | provisional |  |
| `leave-confirm` | flow | Leave this space… (member) | provisional |  |
| `join-already-here` | fail | paste a join link for this Mac's own space | provisional |  |
| `join-switch-confirm` | fail | paste a join link for a different space | provisional |  |

### `settings-space-inspection`

face · window · **provisional** · a face of `settings-space` · cites N36, B139

The inspection mood on the setup Mac (the one holding the token) — Region 2's re-entry-as-settings made concrete. It shows the plan with all six rows ticked, facts on the rows that have them, then the 'nothing to redo' line.

**Entry.** This Mac holds the Cloudflare token and every probe passes.

| Said | Role | Copy | Note |
|---|---|---|---|
| “✓ Cloudflare account — you@example.com” | status | provisional | Row 1 fact. The account name varies. Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. The fact is the account's email, lowercased, when the account still has Cloudflare's own name for it (“<email>'s Account”); any other account name shows as it is. |
| “✓ Access token valid — on this Mac” | status | provisional | Row 2 fact. Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “✓ Cloudflare billing set up” | status | provisional | Row 3, no fact. Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “✓ Your link name — v.myshroom.workers.dev” | status | provisional | Row 4 fact. The spec sets the link name in markdown code backticks (not rendered). The name varies. The tick-with-fact shape matches N15's ratified name-row ending (“✓ Your link name — v.name.workers.dev”). |
| “✓ Wired up” | status | provisional | Row 5, no fact. Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “✓ macOS permissions granted — on this Mac” | status | provisional | Row 6 fact. Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “Your space is ready — nothing to redo.” | line | provisional | Same text as Region 2's re-entry-after-success line (line 625, not copy-ratified there). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `settings-space-repair` | flow | a re-probe un-ticks a row | provisional |  |

### `settings-space-repair`

face · window · **provisional** · a face of `settings-space` · cites N42, N36

The repair mood (N42). A re-probe that un-ticks a row makes the first broken row current, with Region 2's own state beneath it. It is the same machine — there is no second repair UI. Region 5 adds no said-lines of its own; the nests are Region 2's states and lines.

**Entry.** A token-holding Mac where a re-probe un-ticks a row. Typically reached from space-degraded's banner → Reconnect your space.

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | the token died (revoked or expired) | provisional |  |
| `token-other-account` | fail | a fresh token pasted for a different account (from token) | provisional |  |
| `gate-card` | fail | storage switched off / card removed | provisional |  |
| `wiring` | flow | the page or its key is gone — rewire | provisional |  |
| `space-storage-gone` | fail | the bucket is gone | provisional |  |
| `name-changed` | fail | the link name changed | provisional |  |
| `settings-space-inspection` | flow | all probes pass after the fix (derived — Law 1) | provisional |  |

### `settings-space-no-token`

face · window · **provisional** · a face of `settings-space` · cites N36, N33, N42, B139, B157

An owner-class Mac without the token (joined through Add another Mac). It shows two ticked rows and a quiet line saying repairs happen where the token lives, with Add a token here → the wizard at token (the degraded banner's own door, N42). Once this Mac is removed from its space (owner-mac-removed), the space's row loses its tick and the banner's own line replaces the repairs line and its button (B157).

**Entry.** Owner-class Mac that does not hold the Cloudflare token.

| Said | Role | Copy | Note |
|---|---|---|---|
| “✓ Your space — v.myshroom.workers.dev” | status | provisional | The spec sets the link name in backticks (not rendered). The name varies. |
| “✓ macOS permissions granted — on this Mac” | status | provisional | Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “Repairs happen where your Cloudflare token lives — on the Mac you set your space up on, or here if you add a fresh one.” | line | provisional | Quiet line. |
| “Add a token here” | button | provisional |  |
| “• Your space — v.myshroom.workers.dev” | status | provisional | This Mac removed from the space (owner-mac-removed): the space's row, un-ticked (B157). The link name varies. One case's line: it varies by case. |
| “This Mac was removed from your space. Recordings on this Mac are safe — add it again from Members on your other Mac.” | line | provisional | This Mac removed: owner-mac-removed's banner line, in place of the repairs line and Add a token here — a removed owner Mac has nothing to reconnect to (DEVIATIONS #21, B157). One case's line: it varies by case. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Add a token here | secondary |  | `token` | Not filled in the spec. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | Add a token here | provisional |  |

### `settings-space-member`

face · window · **provisional** · a face of `settings-space` · cites N36, N5, B139, B157

A member's Mac: two ticked rows and one line of N5 truth. There is no repair tooling here — a member has nothing to fix. Once the space can't take this Mac's videos (member-revoked, space-gone), the space's row loses its tick and the banner's own line stands in for N5's (B157).

**Entry.** This Mac is a member (not owner-class).

| Said | Role | Copy | Note |
|---|---|---|---|
| “✓ Bagrat's space — v.myshroom.workers.dev” | status | provisional | The owner's name (macOS short form, N29) and the link name vary. The link name is in backticks in the spec (not rendered). |
| “✓ macOS permissions granted — on this Mac” | status | provisional | Settings → Space's own done-state words (B139, Bagrat, 2026-10-14); a row still to do keeps the wizard's. |
| “Your videos live there, where Bagrat can see them too.” | line | provisional | The owner's name varies. |
| “• Bagrat's space — v.myshroom.workers.dev” | status | provisional | Removed or the space gone (N34): the space's row, un-ticked — the same words (B157). The owner's name and the link name vary. One case's line: it varies by case. |
| “Your access to Bagrat's space was turned off. Recordings on this Mac are safe — ask Bagrat if that's a surprise.” | line | provisional | Removed: member-revoked's banner line, in place of “Your videos live there…” (B157). The owner's name varies. One case's line: it varies by case. |
| “Bagrat's space isn't there anymore. Recordings on this Mac are safe — save any you want to keep.” | line | provisional | The space gone: space-gone's banner line, in place of “Your videos live there…” (B157). The owner's name varies. One case's line: it varies by case. |

### `disconnect-confirm`

sheet · sheet · **provisional** · a face of `settings-space` · board: DisconnectSheet · cites N36, N23, N31

Disconnect confirmation for a token-holding owner Mac, under N23's law. The space's only token-holder gets one extra line. The sheet is blocked while uploads drain or a recovered take waits. After Disconnect: the ~/.shroom secrets are wiped, the app goes to idle-unprovisioned (the tray click opens the welcome), and the login item stays as it was.

**Entry.** Owner Mac holding the token. Disconnect this Mac… clicked, not mid-take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Disconnect this Mac?” | sheet-title | provisional | Bold. |
| “Your space and every video in it stay exactly as they are. This Mac forgets your space and goes back to the start.” | sheet-body | provisional |  |
| “To manage your space again, set it up with a fresh token — everything will be there.” | sheet-body | provisional | Added only when this Mac is its space's only token-holder. |
| “Two videos are still uploading — you can disconnect once they're done.” | line | provisional | Blocked state while uploads drain: 'the button greys, one line'. The count word varies. The line for a waiting recovered take is not given. |
| “Cancel” | button | provisional |  |
| “Disconnect” | button | provisional | Red-outlined, rightmost, never filled (N23's law). |
| “One recording never reached the space — leaving deletes it from this Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Disconnect sheet, one stranded take · copy table |
| “One video is still uploading — you can disconnect once it's done.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Disconnect sheet, one upload draining · copy table |
| “Get the link for your saved recording first, or discard it.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Disconnect sheet, a recovered take waiting · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `settings-space` | Esc ≡ Cancel. No Return default (N23, Law 9). |
| Disconnect | destructive |  | `idle-unprovisioned` | Greys while uploads drain or a recovered take waits (N31's rule). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Disconnect (~/.shroom secrets wiped; login item unchanged) | provisional |  |
| `settings-space` | flow | Cancel or Esc | provisional |  |

### `disconnect-confirm-no-token`

sheet · sheet · **provisional** · a face of `settings-space` · cites N36, N23, N33, N31

Disconnect confirmation for an owner-class Mac without the token, under N23's law. The Mac can be re-added from Members. The sheet is blocked while uploads drain or a recovered take waits.

**Entry.** Owner-class Mac without the token. Disconnect this Mac… clicked, not mid-take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Disconnect this Mac?” | sheet-title | provisional | Bold. |
| “Your space and every video stay as they are — add this Mac again anytime from Members.” | sheet-body | provisional |  |
| “Two videos are still uploading — you can disconnect once they're done.” | line | provisional | Blocked state while uploads drain. The count word varies. |
| “Cancel” | button | provisional |  |
| “Disconnect” | button | provisional | Red-outlined, rightmost, never filled. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `settings-space` | Esc ≡ Cancel. No Return default. |
| Disconnect | destructive |  | `idle-unprovisioned` | Greys while uploads drain or a recovered take waits. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Disconnect (~/.shroom secrets wiped; login item unchanged) | provisional |  |
| `settings-space` | flow | Cancel or Esc | provisional |  |

### `leave-confirm`

sheet · sheet · **provisional** · a face of `settings-space` · cites N36, N23, N31, N34

Leave confirmation for a member Mac, under N23's law. Stranded takes don't block (they can't drain): the sheet adds a line and Save them to Movies first, so revoked or space-gone members can always leave. The sheet is blocked while uploads drain or a recovered take waits. After Leave: the ~/.shroom secrets are wiped and the app returns to idle-unprovisioned.

**Entry.** Member Mac. Leave this space… clicked, not mid-take. Also reachable via the 'Leave this space…' text on the member-revoked and space-gone banners (Region 6, N34, lines 1578-1580).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Leave Bagrat's space?” | sheet-title | provisional | Bold. The owner's name varies. |
| “Your videos stay there, where Bagrat can still see them — delete any you want gone first. This Mac goes back to the start.” | sheet-body | provisional |  |
| “Two recordings never reached the space — leaving deletes them from this Mac.” | sheet-body | provisional | Added only when stranded takes exist. The count word varies. |
| “Save them to Movies first” | button | provisional | Shown with the stranded-takes line. |
| “Cancel” | button | provisional |  |
| “Leave” | button | provisional | Red-outlined, rightmost, never filled. |
| “Two videos are still uploading — you can leave once they're done.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Leave sheet, uploads draining · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `settings-space` | Esc ≡ Cancel. No Return default. |
| Save them to Movies first | secondary |  |  | Stranded-take rescue (N34) before leaving. Whether the sheet stays open afterwards is not stated. |
| Leave | destructive |  | `idle-unprovisioned` | Greys while uploads drain or a recovered take waits (the blocked line is given only with 'disconnect' wording). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Leave (~/.shroom secrets wiped; stranded takes deleted from this Mac; login item unchanged) | provisional |  |
| `settings-space` | flow | Cancel or Esc | provisional |  |

### `settings-members`

face · window · **provisional** · a face of `settings` · cites N36, N28, N33

The third tab: N28's Members face (Region 6 invite-born, copy ratified) with N33's You row. Region 5 only places it here (N36). Its content and copy belong to Region 6 and are not extracted here.

**Entry.** Owner-class Mac only. Reached by the Members tab, or by all-set's Invite someone (the window morphs, field focused).

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `invite-born` | flow | this tab is the invite-born face | provisional |  |

### `update-ready`

state · banner · **provisional** · cites N37, U7, N38, N9, N41

U7 notify-then-apply. A newer release, checked quietly in the background, is downloaded and verified. config grows a slim banner — the lowest of the four banner kinds (N38) — and the tray shows the Frost dot. What's new opens the release notes in the browser, and Settings → General mirrors the row. The banner never shows mid-take (config is unreachable mid-take, so it simply waits). There is no nag: the dot and banner sit quietly until applied, and versions roll up so only the newest is offered. On a token-holding Mac, applying an update also refreshes the space's page code. The install script stays the manual update path forever.

**Entry.** The background check found a newer release. It runs at launch, every few hours, and when the popover opens fifteen minutes or more after the last check (B154), against the project's public releases (no shroom server). The release is downloaded and verified against the published fingerprint. A space exists (config exists — before that, no banner and no dot). This Mac's macOS supports the release. It shows in config only when no higher banner (saved take, space can't take videos, recording-off) is present.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom 1.3.0 is ready.” | banner | provisional | The version varies (always the newest). |
| “Update” | button | provisional | Filled. |
| “What's new” | link | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Update | filled-default |  | `updating-app` | A banner's own filled button is never the Return default (Law 8). Return stays Start recording in config. |
| What's new | text-link |  |  | Opens the release notes in the browser. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `updating-app` | flow | Update | provisional |  |

### `updating-app`

face · banner · **provisional** · a face of `update-ready` · cites N37, N9, N41

After Update, the banner reads the updating line. Then quit → swap → relaunch, using N9's semantics: silent and tray-only, with the upload queue riding through. Any Library, Settings or onboarding window open at that moment reopens after the relaunch at its derived state.

**Entry.** Update clicked in config's banner or Settings → General, not mid-take.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Updating — back in a few seconds.” | banner | provisional |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `updated` | flow | swap succeeds → relaunch → the popover opens by itself (B155) | provisional | B155 |
| `update-failed` | fail | swap refused | provisional |  |

### `updated`

face · status-line · **provisional** · a face of `config` · cites N37, N38, B155

A one-shot status-line fact after a successful update relaunch. The relaunch opens the popover by itself to show it (B155); it appears once. The id comes from the said-line: the spec gives this outcome no name.

**Entry.** A successful update relaunch: the popover opens by itself, unasked, under the mushroom (B155). If it's closed before it's seen, the next config open.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Updated to 1.3.0 ✓” | status | provisional | Once only. The version varies. It ranks after the upload truth in the status line (N38). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | shown once | provisional |  |

### `update-failed`

failure-state · status-line · **provisional** · a face of `config` · cites N37, N38, B155

A failed swap leaves the running version in place, and config's status line says so. Failure never breaks anything. A fingerprint mismatch is not this state: it is discarded silently, retried later, and never installed.

**Entry.** An update's swap failed. The running version is unchanged. After a failed relaunch the popover opens by itself to say so, as after a successful one (B155).

| Said | Role | Copy | Note |
|---|---|---|---|
| “The update didn't finish — shroom will try again later.” | status | provisional | A one-shot status line (N38). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `update-ready` | flow | retried later (automatic) | provisional |  |

### `space-needs-update-member`

banner · banner · **provisional** · cites N37, N34

When a member's app needs a newer page than its space has, the member reads a sibling line to space-needs-owner. The page is refreshed only from a token-holding Mac. The page's interface stays backwards-compatible, so this should be rare.

**Entry.** Member Mac. This app version needs newer page code than the space currently runs.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat's space needs an update — it happens when Bagrat updates shroom.” | banner | provisional | The owner's name varies (twice). The spec calls it 'space-needs-owner's sibling line'. space-needs-owner is an amber config banner (N34), so a banner surface is implied, not stated. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | the owner updates shroom on a token-holding Mac (derived — Law 1) | provisional |  |

### `space-needs-update-no-token`

banner · other · **provisional** · cites N37, N33

The same condition seen from an owner-class Mac without the token. It points at the token Mac, or at adding a token here.

**Entry.** Owner-class Mac without the token. This app version needs newer page code than the space has.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space needs an update — open shroom on the Mac you set it up on, or add a fresh token here.” | line | provisional | The surface is not stated (it parallels Region 2's owner-class degraded banner, lines 659-661). |
| “Add a token here” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Add a token here | secondary |  | `token` | The destination is not restated here. The same-label button in the Space tab goes to the wizard at token (line 1190-1191). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `token` | flow | Add a token here | provisional |  |

### `uninstall-confirm`

sheet · sheet · **provisional** · a face of `settings-general` · board: UninstallSheet · cites B135, B156, N23, N31, N36, B160

Uninstall shroom from this Mac, asked first under N23's law. The space is never touched: no bucket, Worker, video, link or member entry (this Mac's included, as Disconnect leaves it). The sheet is blocked while uploads drain or a recovered take waits, and offers Save them to Movies first when recordings never reached the space. After Uninstall: the button reads Uninstalling…, shroom switches its login item off, resets its screen, microphone and camera permissions, and quits; a helper it leaves behind waits for it to exit, then removes the app, ~/.shroom and the system's caches and preferences for it. Copies saved to Movies stay.

**Entry.** Settings → General, Uninstall shroom… clicked, not mid-take, on any Mac with a space, owner or member. A Mac with none asks over the welcome (B160): Uninstall shroom… at its foot, the same sheet with only “This Mac lets go of shroom…” (it has no space to keep).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Uninstall shroom?” | sheet-title | ratified | Bold. |
| “Your space, every video in it and every link stay exactly as they are.” | sheet-body | ratified | An owner-class Mac. Not on a Mac with no space (the welcome's sheet, B160). |
| “Your videos and every link stay exactly as they are, in Bagrat's space.” | sheet-body | ratified | A member's Mac: their videos lead, the space is the owner's — the name varies (CHECKPOINT §10 11c; Bagrat asked for the emphasis on the member's videos staying as they are, not on the owner's space). Member line ratified by Bagrat, 2026-10-15. · B156 |
| “This Mac lets go of shroom: the app, its settings, and its permission to record your screen, microphone and camera.” | sheet-body | ratified |  |
| “To come back, install shroom again and set it up with a fresh token — everything will be there.” | sheet-body | ratified | An owner-class Mac. |
| “To come back, install shroom again and ask for a new invite.” | sheet-body | ratified | A member's Mac. |
| “Two recordings never reached the space — uninstalling deletes them from this Mac.” | line | ratified | With Save them to Movies first. The count is in words (law 16) and varies; one: “One recording never reached the space — uninstalling deletes it from this Mac.” |
| “One recording never reached the space — uninstalling deletes it from this Mac.” | line | provisional | The one-take case of the line above. |
| “Two videos are still uploading — you can uninstall once they're done.” | line | ratified | Uninstall greys. The count varies. |
| “One video is still uploading — you can uninstall once it's done.” | line | provisional | The one-video case of the line above. |
| “Get the link for your saved recording first, or discard it.” | line | ratified | A recovered take waits: Uninstall greys. |
| “Cancel” | button | ratified |  |
| “Uninstall” | button | ratified | Red-outlined, rightmost, never filled (N23's law). |
| “Uninstalling…” | button | ratified | The Uninstall button once clicked, greyed with Cancel, until shroom quits. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `settings-general` | Esc ≡ Cancel. No Return default (N23, law 9). |
| Save them to Movies first | secondary |  |  | Only with recordings that never reached the space; as on the Leave sheet. |
| Uninstall | destructive |  | `landing` | Greys while uploads drain or a recovered take waits (N31's rule). Then shroom removes itself: nothing is left on screen. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `settings-general` | flow | Cancel or Esc | provisional | B135 |
| `landing` | flow | Uninstall (shroom removes itself; the space untouched) | provisional | B135 |

### `report-problem`

sheet · sheet · **provisional** · a face of `settings-general` · board: ReportProblem · cites B180, N53, N8, N48

Report a problem (B180, N53): a description, an optional email for an answer, and “See exactly what’s sent” — the whole report as it goes: the words, the version, macOS, the chip and memory, the setup timeline and the last 1,000 lines of shroom’s log, each line cleaned (home paths, the space’s host, web addresses, emails and long ids replaced). Send posts it to shroom.am/report, which emails it to the person who makes shroom and keeps nothing. Sending, the button reads Sending… and both grey; accepted, the sheet closes and General says so; refused or offline, the sheet stays with what was typed, says so, and offers the mail address. A dev build never reaches shroom.am.

**Entry.** Settings → General, Report a problem… clicked. Available mid-take too: it touches no take (law 4 holds — no window opens for it).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Report a problem” | sheet-title | ratified | Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Tell us what happened. shroom adds its version, your Mac’s model and its recent log — never your videos, links, names or keys. It goes straight to the person who makes shroom.” | sheet-body | ratified | Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “What were you doing, and what went wrong?” | placeholder | ratified | The description box; Return types a new line, ⌘Return sends. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Your email, if you’d like an answer (optional)” | field-label | ratified | Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “See exactly what’s sent” | disclosure | ratified | Opens the whole report, word for word, cleaned. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Putting it together…” | line | ratified | Under the disclosure until the report is gathered; Send waits for it. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “Sending…” | button | ratified | Send's label while it posts; Cancel greys with it. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |
| “That didn’t go through — check your connection and try again. Or email it to hi@shroom.am instead.” | line | ratified | Offline, refused, rate-limited or switched off: the sheet keeps what was typed. “email it to hi@shroom.am” is a link: a mail draft with the description and the version. Ratified by Bagrat, 2026-10-16, relayed by the orchestrator. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `settings-general` | Esc ≡ Cancel. Greys while sending. |
| Send | filled-default |  | `settings-general` | ⌘Return; plain Return types a new line. Grey until something is typed and the report is gathered. |
| email it to hi@shroom.am | text-link |  |  | Only once a send failed: a mail draft to hi@shroom.am with the description and the version (N48). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `settings-general` | flow | Cancel or Esc (not while sending); or Send, and shroom.am took it: General then says it’s sent | provisional | B180 |

### `send-feedback`

state · window · **ratified** · board: FeedbackA · cites N60, N53, B180, N48, N12, N41

Send feedback (N60): a small window of its own, opened from the popover's first header icon. One line on where it goes, the words, an optional email for an answer, **Include shroom’s recent log** — ticked, with a line on what goes and what never goes — and “See exactly what’s sent”, which shows the whole report word for word: the words, the reply-to and, with the box ticked, N53's gathered report (the version, macOS, the chip and memory, the setup timeline and the last 1,000 lines of shroom's log, cleaned); unticked, the words, the reply-to and the version only. Send posts it to shroom.am/report as kind feedback from the app, which emails it to the person who makes shroom and keeps nothing. Sending, the button reads Sending… and Cancel greys; accepted, the window says it's sent and offers Done; refused or offline, it keeps what was typed, says so, and offers the mail address. A dev build never reaches shroom.am.

**Entry.** The ready popover's first header icon, Send feedback, clicked (not mid-take: the header lives in config only). One instance: clicking it again brings the open window forward, as typed. A window opened before a take stays open through it (law 4 covers only what shroom would newly show). The dock icon shows while it is open (Region 1 dock rule).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Send feedback” | window-title | ratified | The window's title, and the header icon's name on hover and to VoiceOver: the site's header label (N55, Bagrat's pick). Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “An idea, something that bugged you, something that broke — it goes straight to the person who makes shroom.” | line | ratified | Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “What’s on your mind?” | placeholder | ratified | The words box; Return types a new line, ⌘Return sends. Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “Your email, if you’d like an answer (optional)” | field-label | ratified | The same words as Report a problem's, ratified by Bagrat, 2026-10-16. |
| “Include shroom’s recent log” | checkbox | ratified | Ticked when the window opens (Bagrat, 2026-10-17). Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “Its version, your Mac’s model and its recent log, cleaned — never your videos, links, names or keys.” | line | ratified | Under the box. Drawn from Report a problem's ratified line. Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “See exactly what’s sent” | disclosure | ratified | Opens the whole report as it would go, box ticked or not. The same words as Report a problem's, ratified by Bagrat, 2026-10-16. |
| “Putting it together…” | line | ratified | Under the disclosure until the report is gathered; Send waits for it only while the box is ticked. The same words as Report a problem's, ratified by Bagrat, 2026-10-16. |
| “Sending…” | button | ratified | Send's label while it posts; Cancel greys with it. The same words as Report a problem's, ratified by Bagrat, 2026-10-16. |
| “That didn’t go through — check your connection and try again. Or email it to hi@shroom.am instead.” | line | ratified | Offline, refused, rate-limited or switched off: the window keeps what was typed. “email it to hi@shroom.am” is a link: a mail draft with the words and the version (N48). The same words as Report a problem's, ratified by Bagrat, 2026-10-16. |
| “Sent — thank you.” | status | ratified | The sent face's heading. Report a problem's ratified line, split in two. Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “If you left your email, you’ll hear back.” | line | ratified | Under it. Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |
| “Done” | button | ratified | The sent face's one button, filled; Return or Esc. Ratified by Bagrat, 2026-10-17, relayed by the orchestrator. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Include shroom’s recent log | field |  |  | A checkbox, ticked when the window opens. Unticked, the report carries the words, the reply-to and the version only, and the disclosure shows exactly that. |
| Cancel | secondary |  |  | Closes the window, the words dropped. Esc ≡ Cancel; ✕ ≡ Cancel (N12). Greys while sending. |
| Send | filled-default |  |  | ⌘Return; plain Return types a new line. Grey until something is typed and, with the box ticked, the report is gathered. |
| email it to hi@shroom.am | text-link |  |  | Only once a send failed: a mail draft to hi@shroom.am with the words and the version (N48). |
| Done | filled-default | ↵ |  | The sent face only: closes the window. Esc and ✕ too. |

### `ai-offer`

banner · banner · **ratified** · a face of `config` · board: AiBanner · cites N66

The AI offer (N66; Bagrat, 2026-10-18: off by default, but visible): a frost banner in the popover, like the update's, on the Mac that holds the setup key while the space's AI is off. It says what shroom can do — name videos, add chapters, cut slips — never what it costs: Turn on… opens the short confirm that says so. The lowest banner: a saved take, the space, recording off and an update come first. Not now hides it for good on this Mac (Bagrat, 2026-10-18): from then on only the switch in Settings · Space turns AI on. Members' Macs and the space's other Macs never show it, nor does a Mac once AI has been on there.

**Entry.** `config` opens on the Mac that holds the setup key while the space's AI is off and no higher banner shows, and the banner has never been answered on this Mac.

| Said | Role | Copy | Note |
|---|---|---|---|
| “shroom can name your videos, add chapters and cut your slips for you.” | banner | ratified | N66. The banner's line (AiBanner). Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Turn on…” | button | ratified | N66. Filled; a banner's own filled button is never the Return default (law 8). Opens ai-confirm. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Not now” | link | ratified | N66. Quiet, left of Turn on…. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Turn on… | filled-default |  | `ai-confirm` | The confirm says what it costs. |
| Not now | text-link |  | `config` | Hidden for good on this Mac; the switch in Settings · Space stays. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `ai-confirm` | flow | Turn on… | ratified | N66 |
| `config` | flow | Not now | ratified | N66 |

### `ai-confirm`

state · popover · **ratified** · board: AiConfirm · cites N66

Turn on…'s short confirm (N66): the popover morphs into a question that says what turning AI on does and what it costs, once, before it costs anything. Like every question in the popover it has no Return default: Turn on is a click. Back and Esc go back to config with the banner still there.

**Entry.** Turn on… in the AI banner.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Turn on AI for new videos?” | title | ratified | N66. Bold. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Titles, chapters and captions from what’s said, and your “shroom cut” slips cut — on your own Cloudflare account. Free for about 2½ hours of new video a day; more is about 4¢ per video hour on Cloudflare’s $5 plan.” | line | ratified | N66. The one cost line, also under the switch in Settings · Space. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Back” | button | ratified | N66. Leftmost. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Turn on” | button | ratified | N66. Filled, rightmost; no Return default (a click, as every popover question). Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Back | secondary |  | `config` | Esc too; the banner stays. |
| Turn on | filled-default |  | `ai-turning-on` | Re-wires the space's page with Cloudflare's AI and its schedule, as the switch in Settings · Space does. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `ai-turning-on` | flow | Turn on | ratified | N66 |
| `config` | flow | Back | ratified | N66 |

### `ai-turning-on`

face · banner · **ratified** · a face of `ai-offer` · board: AiBannerStates · cites N66

While the space's page is put up again with Cloudflare's AI (a few seconds), the banner says so and holds no buttons.

**Entry.** Turn on in ai-confirm, until the page answers.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Turning on — a few seconds.” | banner | ratified | N66. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `ai-on` | auto | the page answers that its AI is on | ratified | N66 |
| `ai-offer-failed` | fail | the re-wiring didn't go through (law 12) | ratified | N66 |

### `ai-offer-failed`

face · banner · **ratified** · a face of `ai-offer` · board: AiBannerStates · cites N66, law 12

The re-wiring didn't go through (law 12): the banner says so in place of its line, for this visit to the popover, and Turn on… is live again.

**Entry.** ai-turning-on, and the space's page couldn't be put up again with the AI.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Couldn't change that just now — try again in a moment.” | banner | ratified | N66. The words of the switch's own failure line. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Turn on…” | button | ratified | N66. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |
| “Not now” | link | ratified | N66. Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Turn on… | filled-default |  | `ai-confirm` |  |
| Not now | text-link |  | `config` | Hidden for good on this Mac, as ai-offer's. |

### `ai-on`

face · status-line · **ratified** · a face of `config` · board: AiBannerStates · cites N66, N38

A one-shot status-line fact once the banner's Turn on has gone through, where the update's “Updated ✓” goes: said once, then the line is behind us.

**Entry.** The page answers that its AI is on after the banner's Turn on: in the popover then, or the next time it opens.

| Said | Role | Copy | Note |
|---|---|---|---|
| “AI is on for new videos ✓” | status | ratified | N66. Once only; it ranks after the upload truth (N38). Bagrat approved the board as drawn, 2026-10-18, with the price said per video hour; he may tweak the words later. · N66 |

### `library-cleanup`

face · window · **ratified** · a face of `library` · board: CleanupView · cites N68

Suggested to clean up (N68): the Library's own view of the videos worth deleting, costliest per remaining play first. A line says what deleting all of them frees and saves; each row shows the poster, the title, then when it was made · its size · why it's suggested, with Open page, Keep and Delete (red text). Delete is one tap: the row leaves and an Undo line stays a few seconds before the delete is sent (a sealed delete can't be taken back, B125). Delete all… opens the several-videos sheet. Keep sets the record's keep and the row leaves for good. Members see their own; owner-class Macs follow Mine · Everyone.

**Entry.** Review… on the Library's suggestion line.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Suggested to clean up” | title | ratified |  |
| “‹ Library” | button | ratified | Back to the Library. |
| “These might be worth deleting: rarely watched lately, and taking real room. Deleting all six frees 11 GB — about 17¢ a month off your Cloudflare bill.” | line | ratified | Past the free 10 GB. The count varies (a word up to nine, Law 16), and so do the size and the cents. Two read 'Deleting both', one 'Deleting it'. |
| “These might be worth deleting: rarely watched lately, and taking real room. Deleting all six frees 5 GB. Your space is inside Cloudflare's free 10 GB, so this saves nothing yet.” | line | ratified | Inside the free 10 GB (B212). The count and the size: each varies. |
| “These might be worth deleting: rarely watched lately, and taking real room. Deleting all six frees 3 GB and brings your space back inside Cloudflare's free 10 GB.” | line | ratified | When deleting them all would take the space back under 10 GB (B212). The count and the size: each varies. |
| “These might be worth deleting: rarely watched lately, and taking real room. Deleting all six frees 11 GB.” | line | ratified | On a member's Mac: the bill is the space owner's, so no money is said. The count and the size: each varies. |
| “5¢ a month” | line | ratified | A row's cost, after its size, only when the space is past Cloudflare's free 10 GB and never on a member's Mac (N68 (8)). The cents vary: the row's size at the list price, $0.015 a GB-month, rounded, at least 1¢ (B212). |
| “Watched 4 times, the last 10 months ago” | line | ratified | A row's reason (B211); the counts and times vary. Counts are figures here: a row fact (Law 16). Shapes: 'Watched once, 5 months ago' · 'Watched twice, the last 2 months ago' · 'Watched twice, stopped early, the last 4 months ago' (most stopped in the first quarter) · 'Nobody has watched it'. Times: days under 60, months under 24, then years. |
| “Nobody has watched it” | line | ratified | The reason when counting never saw a play. |
| “Open page” | button | ratified |  |
| “Keep” | button | ratified | Takes the video off the list for good. |
| “Delete” | button | ratified | Red text, never a button (Law 9); one tap, then Undo. |
| “Deleted “Old onboarding take”.” | line | ratified | The Undo line, a few seconds; the delete is sent when it goes. The title varies. |
| “Undo” | button | ratified |  |
| “Delete all six…” | button | ratified | Red text; opens delete-confirm-several. The count varies, a word up to nine; two reads 'Delete both…'. |
| “Keep takes a video off this list for good. Plays are counted in your own Cloudflare account.” | line | ratified |  |
| “Nothing to clean up right now.” | line | ratified | Once the last suggestion is kept or deleted. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| ‹ Library | secondary |  | `library` |  |
| Delete all six… | destructive |  | `delete-confirm-several` |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `library` | flow | ‹ Library | ratified | N68 |
| `delete-confirm-several` | flow | Delete all… | ratified | N68 |

#### Region 5's amendments

- **N36** (amends N28): Resolves N28's open placement ('how Members sits beside the Space panel is the settings leg's call'): Members is the Settings window's third tab, owner-class only, and all-set's Invite someone morphs the onboarding window into this window on Members ('N28's morph, now with a home', line 1163-1164). N28's row records it as 'Settled by N36'. Not marked **amends** in the ledger.
- **N36** (amends U4): U4's 'changeable in settings' gets its home: the General checkbox, which reads the system's login-item truth. A refinement, not marked **amends**.
- **N36** (amends Region 2 Re-entry-as-settings): The all-✅ re-entry panel now lives as the Settings window's Space tab (inspection mood), alongside the repair mood and the exits (lines 665-669, 1178-1183).
- **N37** (amends N25): Settles N25's open badge question: the update does not share the amber dot. It gets its own lowest-priority Frost dot, and amber stays 'something needs you'. N25's row records it as 'Settled by N37/N38'.
- **N37** (amends N24): Extends N24's tray faces with the Frost dot (five faces with N38's amber). N24's row records it as 'Extended by N37/N38'.
- **N37** (amends U7): Makes U7's tray badge concrete as the Frost dot plus a slim config banner (lowest of the banners) with [ Update ] · What's new, mirrored in Settings → General. Apply follows N9's quit/swap/relaunch. A refinement, not marked **amends**.

#### Region 5's open notes

- Region 5 has no mermaid diagram. Transitions here come from prose. The repair-mood edges are drawn in Region 2's diagram (lines 699-707) and are reproduced on 'settings · Space · repair'.
- Copy status: every said-line in lines 1075-1281 is designed 2026-09-30 (N35–N37, awaiting ratification), so all are marked provisional. Some text is verbatim copy ratified elsewhere: 'Start shroom when you log in' (welcome, line 324), 'Copied' (N26), the N15 name-row tick shape, and 'Turn it back on' (N38's recording-off, itself provisional). Their use here is still N36/N35's and unratified.
- Ids: only `library`, `settings` and `update-ready` are backticked scaffold-grade names in Region 5. All other ids (faces, tabs, moods, sheets, 'updated', 'update failed', 'space needs an update · …') are built from the spec's own labels and need real scaffold-grade names. Inventory row 1703's 'updating' clashes with Region 1's `updating` (Terminal lane, N9/N18), so it is filed as 'update-ready · updating'.
- Not in the inventory, but defined in Region 5 prose: the success status line “Updated to 1.3.0 ✓” (also in N38, line 779) and the two 'space needs an update' lines (member / owner-class without token). The inventory's Region 5 rows (1701-1703) otherwise all map to entries above.
- Law 8 'check it at ratification' (line 143): single-action faces lead with their filled button. The Library's recovered-take row mirrors N25's saved-take banner ([ Get link ] then Discard…), so it falls under this local exception.
- Return defaults in the Library and Settings windows are not stated. Law 8 says one default per surface, but no default is named for the Settings tabs, the Library, or the filled buttons there ('Update' in General, 'Turn it back on', 'Get link' on a row). All are recorded as return_default=false. Confirm at ratification.
- Disconnect this Mac… / Leave this space… are written as 'quiet text', while Law 9 says destructive entry points are red text. Check whether these exits count as destructive entry points.
- Exit-sheet gaps: (a) 'the button greys' while uploads drain is ambiguous between the tab's exit text and the sheet's Disconnect/Leave button; recorded on the sheets. (b) The blocked line is given only with the verb 'disconnect'; the Leave wording and the recovered-take-waiting line are not given (compare join-switch-confirm's “Get the link for your saved recording first, or discard it.”, line 1555-1556). (c) The stranded-takes line + [ Save them to Movies first ] is stated generically ('the sheet adds') but says 'leaving'. It is attached only to the member Leave sheet; whether the owner Disconnect sheets carry it is unstated.
- Library gaps: the Discard… on a 'Removed from the space' row names no confirm. The singular form of the footer count ('14 videos') is not given. The mocked row separator (title — date · length) is from a mock, and whether the em dash renders is unconfirmed. The mark-switch label “made with shroom” on your pages is ambiguous about whether the inner curly quotes render.
- Settings paste door: Region 5 names it only as N29's paste door (any Mac). Whether it also classifies a pasted token (as the welcome's N30 door does → the wizard at token) is not stated; not modelled here.
- Update gaps: after a failed swap ('shroom will try again later') it isn't said whether the Frost dot and banner persist until the retry. 'Every few hours' check cadence is unpinned (not in the Thresholds flag). Before a space exists there is no dot and no banner — the install command is the update path for a never-set-up app. Past this Mac's macOS: no banner, no dot, General line only.
- Fat-marker status: no 'Fat-marker status' notes appear in lines 1075-1281. The region's structure is ratified (2026-07-20), and N35–N37 await Bagrat's ratification.
- Custom domain (N2): post-v1.0, so v1 has no state or said-line for it. When it lands it lives in Settings → Space as a token-Mac row. Docs state the upgrade honestly per R6.3 (never pitched as a reliability rescue). It is listed under 'Deliberately absent from v1' (line 1729).
- Rejected alternatives recorded in-region (do not build): a web library on the Worker; a thumbnail grid; per-row sizes (P13); the mark switch in General; a Quit row in Settings; an uninstall button (docs: Disconnect this Mac, then drag shroom to the Trash) — reversed by B135 (Bagrat, 2026-10-14): Settings → General carries Uninstall shroom… (uninstall-confirm), and the install line takes --uninstall (uninstall-sh, Region 1)
- Settings → Members content (invite-born, You row, Add another Mac, member-remove-confirm) belongs to Region 6 and is not extracted here. Region 5 only places it as the third tab (owner-class only).
- Markdown stripped from said-lines: bold markers and code backticks around link names (e.g. `v.myshroom.workers.dev`); line-wrapped quotes are joined with single spaces. Apostrophes in this region are straight (U+0027; no U+2019 in 1075-1281), ellipses are U+2026 (no '...' in range), and checks are ✓ U+2713.

#### Region 5's scaffold flags

- Tombstone check before every upload resume (N35): another Mac's delete must stop this Mac's upload, and this Mac's Library row turns to 'Removed from the space' with Save to Movies · Discard…. The copy is bound to it.
- The page's interface stays backwards-compatible across app versions (N37), so members ahead of or behind the owner keep working.
- The page refuses a downgrade (N37, N42): a token-holding Mac on an older app, repairing or updating, never replaces newer page code.
- Swap-under-running-process behavior (N9, carried): Update reuses N9's quit → swap → relaunch semantics. Verify the exact signature-mismatch behavior at scaffold.
- Launch provenance (N41): an update relaunch stays silent but must restore whatever Library, Settings or onboarding window was open, at its derived state. This requires telling an update relaunch apart from a launch the user did.
- Fingerprint verification of downloaded updates exactly as the installer does. A mismatch is discarded silently and retried, never installed (the installer's law). Region-local; not in the machine-wide scaffold-flag list (1736-1762).

<a id="region-6"></a>

## Region 6 — members and the join machine

*Region 6 — members, the join machine, and the guided overlay (UX-STATEMACHINE.md lines 1282-1663)*

### `invite-born`

state · window · **ratified** · board: MembersInvited · cites N28, N33, N36, N29, N5, N3, N26, N24, N12, B137, B158

The Members face, the Settings window's third tab (N36). It shows a heading, the one line of N5 truth, the member list (You, the owner's other Macs as sub-rows once there are two Macs (B137), joined members, pending invites) and the composer (a name field + Invite), which is always present. Invite mints a one-person, seven-day join link. The link appears under the new row with a filled Copy link. Copy is ratified (N28). The You row's Add another Mac and the owner-Mac sub-rows are provisional (N33).

**Entry.** Owner-class Mac only (N5, N33). Reached by the gear → Settings → Members, or by all-set's Invite someone morphing the same window (the name field arrives focused). Diagram: [*] → invite_born ('owner — Invite someone, or Add another Mac').

| Said | Role | Copy | Note |
|---|---|---|---|
| “Members” | title | ratified |  |
| “Everyone here records to your space — their videos live in your storage, and you can see and delete any of them.” | line | ratified | The one line of N5 truth, said once, here. |
| “You” | other | ratified | Member #1's row. It carries no fact (N28). |
| “Add another Mac” | link | provisional | Quiet Frost text beside You, not a button (N33). |
| “Mher — joined Aug 3” | status | ratified | A joined member's row: the admin-private label + the join date. Both vary. |
| “Ani — invited just now” | status | ratified | A pending invite's row right after Invite. The label is what the admin typed. |
| “invited Aug 21 · 5 days left” | status | ratified | The pending-row fact later in the invite's life. Date and days left vary (figures per Law 16). |
| “Remove…” | link | ratified | Red text on every row but You (N24's pattern). |
| “v.myshroom.workers.dev/join/” | other | ratified | The new link's pill under the new row: v.<name>.workers.dev/join/<code>. Name and code vary. · was “v.myshroom.workers.dev/join/7xk2pm” · #22, B134 |
| “Copy link” | button | ratified |  |
| “Copied” | status | ratified | Copy link reads “Copied” briefly (N26/N28, Law 10). |
| “Send Ani this link — it's good for one person, for seven days.” | line | ratified | Under the new link. The invitee's label varies. |
| “Who's this for?” | placeholder | ratified | The composer's name field. The typed name is the admin-private label, forever. |
| “Invite” | button | ratified |  |
| “You'll get a link to send them — over any chat.” | line | ratified | Beneath the composer. |
| “Invites will say they're from Bagrat.” | line | ratified | Under the composer, always: the name new invites carry, set in bold, macOS's short form until it's changed (B158). The name varies. |
| “Change” | link | ratified | Beside that line: the name becomes a field, the line's head its label (B158). |
| “Invites will say they're from” | field-label | ratified | The field's label while the name is being changed (B158). One case's line: it varies by case. |
| “Save” | link | ratified | Beside the field: keeps the name (Return in the field too); greyed while the field is empty. Esc puts the name back (B158). One case's line: it varies by case. |
| “New link” | button | ratified | Offered on an expired invite's row. A row face, not a state (N18 nuance). |
| “You · MacBook Air — joined Sep 2 · Remove…” | status | provisional | One of the owner's other Macs as a sub-row under You (N33). The Mac's sharing name and the date vary. Shown only once the owner has a second Mac: with one Mac there are no sub-rows, and a second record of the same Mac (same name) doesn't count as a second Mac (B137, Bagrat, 2026-10-14: hidden until a second Mac). |
| “keeps your Cloudflare token” | status | provisional | Row fact on the sub-row of an owner Mac that holds the token. That row has no Remove… (N33). Like every owner-Mac sub-row, it shows only once there is a second Mac (B137). |
| “Sona — invited Aug 10” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Members, an expired invite's row · copy table |
| “You · MacBook Air — joined Sep 2” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: Members, another of the owner's Macs · copy table |
| “Couldn't make a link just now — try again in a moment.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Members face's composer line, after a link that didn't come back (Invite, Add another Mac, New link) · copy table |
| “Couldn't remove that just now — try again in a moment.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The Members face's composer line, after a Remove… that didn't go through · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Invite | filled-default | ↵ | `invite-born` | Return in the name field ≡ Invite (the field owns Return, Law 8). Mints the link, which appears under the new pending row. Drops to outline while a new link is on screen. |
| Copy link | filled-default |  | `invite-born` | Filled while a new link is on screen: the moment's one action. Reads “Copied” briefly. The spec does not bind Return to it. |
| Remove… | red-text |  | `member-remove-confirm` | On every row but You. Absent on the sub-row of a Mac that holds the token. |
| New link | secondary |  | `invite-born` | Expired invite rows only. Nothing is blocked. Not marked filled. |
| Add another Mac | text-link |  | `add-another-mac` | Opens the nest under You (N33). |
| ✕ | close |  |  | ≡ Close (N12). The face has no back-chrome. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `add-another-mac` | side | You — Add another Mac | provisional |  |
| `member-remove-confirm` | side | Remove… | provisional |  |
| `join-landing` | flow | the link, sent over any chat, opened | provisional |  |

### `add-another-mac`

face · window · **provisional** · a face of `invite-born` · cites N33

The You row's nest for adding one of the owner's own Macs. It holds an owner-class join link, one-time use and good for one hour, with a filled Copy link. The joined Mac later appears as a sub-row under You, labelled by its own sharing name.

**Entry.** Owner-class Mac on the Members face; the owner clicks the quiet Add another Mac beside You. Members never have this door in v1.

| Said | Role | Copy | Note |
|---|---|---|---|
| “v.myshroom.workers.dev/join/” | other | ratified | The own-Mac link pill. Name and code vary. · was “v.myshroom.workers.dev/join/q8w3zt” · #22, B134 |
| “Copy link” | button | provisional | Same label as the ratified Copy link on the Members face. This nest is N33. |
| “Open this link on your other Mac — it's good for one hour.” | line | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | filled-default |  |  | Filled (bold in the spec). Reads “Copied” per Law 10. The spec does not bind Return to it. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `join-landing-own-mac` | flow | the link is opened on the other Mac | provisional |  |

### `member-remove-confirm`

sheet · sheet · **provisional** · a face of `invite-born` · cites N32, N33, N23, N24

A sheet over the Members face that confirms a Remove…. It has three faces by row kind: a member, a pending invite, your other Mac. Esc ≡ the left button. There is no Return default. The destructive button is red-outlined, rightmost and never filled. A removal is never bundled with deleting videos.

**Entry.** Remove… clicked on any Members row except You (and never on a Mac that keeps the Cloudflare token).

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Esc | other |  | `invite-born` | ≡ the left button (Cancel / Keep it). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `invite-born` | flow | Cancel / Keep it / Esc: nothing changes | provisional |  |

### `member-remove-confirm-member`

face · sheet · **provisional** · a face of `member-remove-confirm` · cites N32

Removing a joined member. They can't record to the space anymore, but a video already on its way can still finish. Their videos stay in the Library with their label.

**Entry.** Remove… on a joined member's row.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Remove Mher?” | sheet-title | provisional | The member's admin-private label varies. |
| “Mher won't be able to record to your space anymore — a video already on its way can still finish. Mher's videos stay; you'll find them in your Library.” | sheet-body | provisional | Bound to per-recording minting. Per-member creds would force “for up to a week”. |
| “Cancel” | button | provisional |  |
| “Remove” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `invite-born` | Left button; ≡ Esc. |
| Remove | destructive |  | `invite-born` | Red outline, rightmost, never filled; no Return default. The member's row goes; their videos keep their label in the Library. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `invite-born` | flow | Cancel / Esc | provisional |  |

### `member-remove-confirm-invite`

face · sheet · **provisional** · a face of `member-remove-confirm` · cites N32

Cancelling a pending invite: the link stops working right away.

**Entry.** Remove… on a pending invite's row.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Cancel Ani's invite?” | sheet-title | provisional | The invitee's label varies. |
| “The link stops working right away.” | sheet-body | provisional |  |
| “Keep it” | button | provisional |  |
| “Cancel invite” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Keep it | secondary |  | `invite-born` | Left button; ≡ Esc. |
| Cancel invite | destructive |  | `invite-born` | Red outline, rightmost, never filled; no Return default. The link then lands on join-expired. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `invite-born` | flow | Keep it / Esc | provisional |  |

### `member-remove-confirm-own-mac`

face · sheet · **provisional** · a face of `member-remove-confirm` · cites N32, N33

Removing one of the owner's other Macs revokes its device key. That Mac stops recording to the space and its videos stay. Not offered for a Mac that keeps the Cloudflare token, or for this Mac itself (that is Disconnect, N36).

**Entry.** Remove… on an owner-Mac sub-row under You that does not keep the Cloudflare token and is not the current Mac.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Remove MacBook Air?” | sheet-title | provisional | The Mac's sharing name varies. |
| “That Mac stops recording to your space. Its videos stay.” | sheet-body | provisional |  |
| “Cancel” | button | provisional |  |
| “Remove” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Cancel | secondary |  | `invite-born` | Left button; ≡ Esc. |
| Remove | destructive |  | `invite-born` | Red outline, rightmost, never filled; no Return default. Revokes that Mac's device key. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `invite-born` | flow | Cancel / Esc | provisional |  |

### `join-landing`

web-page · web-page · **provisional** · board: JoinLanding · cites N29, N3, N6, U3

The page a join link opens, served by the space's own Worker with no shroom.am dependency. It has no header, its title is the hero line (so a chat unfurl says it) and it is noindex. It holds a Get-shroom block with the --join install command, an Open shroom deep link with a paste fallback, the one-person/seven-days line, and the mark bottom-right. Opening the page never uses the invite; only Join in the app does.

**Entry.** A join link (v.<name>.workers.dev/join/<code>) opened in a browser on a Mac; the invite is valid, unused and unexpired; no owner cookie riding along.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat invited you to record with shroom.” | title | provisional | Also the page's title, used by the unfurl. The inviter's name varies: the inviting Mac account's name in macOS's short, locale-aware form (N29: 'Bagrat' for 'Bagrat Aznauryan'). |
| “shroom records your screen and hands you a link to share. Nothing to sign up for — Bagrat set up a space for you.” | line | provisional | The inviter's name varies (macOS short form, N29). |
| “Get shroom” | other | provisional | Block heading. |
| “Paste this into Terminal (⌘ space, type “Terminal”) and press Return — it installs shroom and opens it, already knowing it's you.” | line | provisional | The Terminal hand-hold (flag 2, kept). The inner quotes are curly in the spec. |
| “curl -fsSL https://shroom.am/install \| sh -s -- --join v.myshroom.workers.dev/join/7xk2pm” | other | provisional | The install command with the join link built in. The link varies. |
| “Copy” | button | provisional |  |
| “Read it first — the same address opens as readable source.” | line | ratified | The landing's ratified line, verbatim (N6). |
| “For macOS 14 or newer, Apple silicon.” | line | ratified | The landing's ratified line, verbatim. Only the sentence's shape is ratified: the OS floor and Intel stance are scaffold-time facts, to fill in without rewording (line 214). · was “For macOS 14 or newer, Apple silicon and Intel.” · B7, B44, copy table, B134 |
| “Use Homebrew? This does the same:” | line | ratified | Under the install command, above a divider of its own; quieter than the install line (N51). · N51 |
| “brew install --cask bagrat/shroom/shroom && open -a ~/Applications/shroom.app "https://v.myshroom.workers.dev/join/7xk2pm"” | other | ratified | The Homebrew join line (N50, N51): the link varies, and is always whole, https:// and all — the app takes a web address only as a join link. Its Copy is outlined, not filled. · N50, N51 |
| “Already have shroom?” | line | provisional | Below a divider. |
| “Open shroom” | button | provisional | The shroom://join/… deep link. |
| “If it doesn't open, paste this page's address into shroom.” | line | provisional | The fallback for the deep link. |
| “This invite is for one person and works for seven days.” | line | provisional |  |
| “made with shroom” | tooltip | ratified | Bottom-right label beside the mark (U3/N26). The page's only shroom chrome; links home. |
| “This invite link doesn't work right now — try again a little later.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The join page, while the space's records can't be read (no roster yet, an unreadable one, a storage error) · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy | secondary |  |  | Copies the install command. Not marked filled. Reads “Copied” per Law 10. |
| Open shroom | secondary |  | `member-welcome` | The shroom://join/… deep link. The browser's own “open shroom?” prompt is the only step it adds. Not marked filled. |
| made with shroom (the mark) | text-link |  |  | Links home (shroom.am). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `join-landing-owner-view` | flow | the space owner's cookie rides along | provisional |  |
| `join-landing-elsewhere` | side | not a Mac | provisional |  |
| `join-installing` | flow | curl with the link built in | provisional |  |
| `member-welcome` | flow | Open shroom — already installed | provisional |  |
| `join-expired` | fail | past its days · cancelled | provisional |  |
| `join-expired-page` | flow | past its days, cancelled, unknown (diagram: join_expired) | provisional |  |
| `join-used` | fail | already used | provisional |  |
| `join-used-page` | flow | already used (diagram: join_used) | provisional |  |

### `join-landing-owner-view`

face · web-page · **provisional** · a face of `join-landing` · cites N29

The same join page with one quiet note on top, for an owner checking their own link. Opening it here doesn't use it up.

**Entry.** The join link is opened with the space owner's cookie riding along (they clicked their own link to check it).

| Said | Role | Copy | Note |
|---|---|---|---|
| “This is the page Ani will see — opening it here doesn't use it up.” | line | provisional | One quiet note on top of an otherwise identical page. The invitee's admin-private label varies. |
| “This is the page your other Mac will see — opening it here doesn't use it up.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The join page's owner view of an own-Mac link · copy table |

### `join-landing-elsewhere`

face · web-page · **provisional** · a face of `join-landing` · cites N29

The join page on something that isn't a Mac (phone, tablet, Windows, Linux). The hero stays. The Get-shroom block and the divider give way to one line + Copy link. The one-person line stays.

**Entry.** The join link is opened on something that is not a Mac.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat invited you to record with shroom.” | title | provisional | “The hero stays.” The inviter's name varies (macOS short form, N29). |
| “shroom runs on a Mac — open this link on yours to get started.” | line | provisional |  |
| “Copy link” | button | provisional |  |
| “This invite is for one person and works for seven days.” | line | provisional | “The one-person line stays.” |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy link | secondary |  |  | Copies the join link. Fill not specified. Reads “Copied” per Law 10. |

### `join-landing-own-mac`

face · web-page · **provisional** · a face of `join-landing` · cites N33, N29

The join page for an own-Mac link (from Add another Mac). Same shape, different words: owner-class, one Mac, one hour.

**Entry.** An own-Mac link (from Add another Mac) is opened while valid and unused, within its hour.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Add this Mac to your shroom space.” | title | provisional |  |
| “Same videos, same members — v.myshroom.workers.dev, on this Mac too.” | line | provisional | The space address varies (v.<name>.workers.dev). The spec's example is kept. |
| “This link is for one Mac and works for one hour.” | line | provisional | Replaces the one-person/seven-days line. The Get-shroom block is the same as join-landing's. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Copy | secondary |  |  | The same Get-shroom block as join-landing. |
| Open shroom | secondary |  | `member-welcome-own-mac` | The same block; deep link. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `join-installing` | flow | curl with the own-Mac link built in | provisional |  |
| `member-welcome-own-mac` | flow | Open shroom (already installed) | provisional |  |
| `join-expired-page` | flow | the hour has passed | provisional |  |

### `join-installing`

face · terminal · **provisional** · a face of `installing` · cites N3, N29, N33, N9, N41

Region 1's install script verbatim, run with --join <link>; it passes the link to the app at launch. The join link rides argv and shell history once. This is the conscious scoped exception: one-time use + a 7-day TTL (one hour for own-Mac links). The admin token never gets this exception.

**Entry.** The user runs the join page's install command (curl … | sh -s -- --join <link>).

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `updating` | flow | shroom already installed: quit → swap → relaunch (N9); the relaunched app opens the invite | provisional |  |
| `install-refused-midtake` | fail | mid-take: nothing changed; the link is still unused | provisional |  |
| `member-welcome` | flow | the script forwards the link | provisional |  |
| `join-already-here` | fail | already installed and this Mac is already in that space (after the updating lane) | provisional |  |
| `join-switch-confirm` | fail | already installed and this Mac has another space (after the updating lane) | provisional |  |

### `member-welcome`

state · window · **provisional** · board: MemberWelcome · cites N30, U4, N12, N5, N41

The welcome window's twin, which already knows it's an invite. It shows the hero, the N5 truth mirrored for the invitee (the consent point), the permissions step, the U4 login box, and [ Join ] [ Later ] in the welcome's order (filled first). No Cloudflare, no card, no prices.

**Entry.** The app holds a live invite and this Mac has no space. It arrives by: the install script passing --join (fresh Mac, or a relaunch after the updating lane); the join page's Open shroom deep link; a join link macOS hands to shroom as it opens (`open -a shroom <link>`, the Homebrew join line — N50); the paste door (the welcome's “Have an invite link?” or Settings → Space); or a tray click in idle-unprovisioned while a remembered invite lives. Not while a take is open (Law 4: invites wait).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat invited you to record with shroom.” | title | provisional | The inviter's name varies: the inviting Mac account's name in macOS's short, locale-aware form (N29). |
| “shroom records your screen and hands you a link to share. Your videos will live in Bagrat's space, where Bagrat can see them too — nothing to sign up for, nothing to pay.” | line | provisional | The N5 truth mirrored for the invitee, said once. The inviter's name varies. |
| “One step before your first video: let macOS allow recording. About a minute, once.” | line | provisional |  |
| “Start shroom when you log in” | field-label | ratified | The welcome's ratified U4 checkbox label, reused. Pre-checked. |
| “Join” | button | provisional |  |
| “Later” | button | ratified | The welcome's ratified Later label, reused. |
| “Bagrat's space needs an update — it happens when Bagrat updates shroom.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The invite window, a space whose page is behind · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Join | filled-default | ↵ | `joining` | Filled, first (leftmost), mirroring the ratified welcome's order (Law 8's local exception; check at ratification). |
| Later | secondary |  | `idle-unprovisioned` | The invite is remembered (~/.shroom, mode 600). The login item registers iff the box is still checked. |
| ✕ | close |  | `idle-unprovisioned` | ≡ Later exactly (N12). |
| Start shroom when you log in | switch |  |  | Checkbox, checked by default. Applied at any dismissal (U4/N12). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Later: the invite is remembered | provisional |  |
| `joining` | flow | Join | provisional |  |
| `join-already-here` | fail | already in that space | provisional |  |
| `join-switch-confirm` | side | in another space | provisional |  |

### `member-welcome-own-mac`

face · window · **provisional** · a face of `member-welcome` · cites N33, N30

member-welcome for an own-Mac link. It adds this Mac to the owner's own space as owner-class.

**Entry.** The invite the app holds is an own-Mac link (from Add another Mac) and this Mac has no space.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Add this Mac to your space.” | title | provisional | Differs from the page's hero (“…your shroom space.”). |
| “Same videos, same members — v.myshroom.workers.dev, on this Mac too.” | line | provisional | The space address varies (v.<name>.workers.dev). The spec's example is kept. |
| “One step before your first video here: let macOS allow recording. About a minute, once.” | line | provisional |  |
| “Start shroom when you log in” | field-label | ratified | ☑: the ratified welcome checkbox label. |
| “Join” | button | provisional |  |
| “Later” | button | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Join | filled-default | ↵ | `joining-own-mac` | Filled, first. |
| Later | secondary |  | `idle-unprovisioned` |  |
| ✕ | close |  | `idle-unprovisioned` | ≡ Later. |
| Start shroom when you log in | switch |  |  | Checked by default. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Later / ✕ | provisional |  |
| `joining-own-mac` | flow | Join | provisional |  |

### `joining`

state · window · **provisional** · board: Joining · cites N30, N16, N13, N17, N4

Join morphs the window into a two-row plan. Row one is current, with a spinner and one calm line, while the app validates the link with the Worker and the Mac's member identity lands locally (creds mint later, per recording). Zero affordances (N16's posture).

**Entry.** Join in member-welcome, or Join Bagrat's space in join-switch-confirm; auto-resume from join-waiting.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat's space, on this Mac:” | title | provisional | Plan header. The inviter's name varies (macOS short form, N29). |
| “→ Joining Bagrat's space (shroom does this part)” | line | provisional | Row one, current. → is the plan's current-row glyph (N13). |
| “• Letting macOS allow recording (microphone, camera and screen — a one-time yes each)” | line | provisional | Row six verbatim (N17), still ahead (• glyph). N43: the camera joins the chapter. |
| “Joining — a few seconds.” | status | provisional | With a spinner. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `done` | flow | the join lane answers | ratified | N33, B134 |
| `all-set` | flow | the join lane answers | ratified | N33, B134 |
| `member-welcome` | flow | the join lane answers | provisional | #30 |
| `member-done` | flow | ticked with the fact — rests | provisional |  |
| `join-waiting` | fail | offline or unanswered | provisional |  |
| `join-expired` | fail | expired or cancelled (diagram) | provisional |  |
| `join-used` | fail | already used (diagram) | provisional |  |
| `join-space-missing` | fail | no space there | provisional |  |
| `join-already-here` | fail | the join lane answers | provisional | N31 |

### `joining-own-mac`

face · window · **provisional** · a face of `joining` · cites N30, N33

The own-Mac plan. The header and the row speak of “your space”, and the row ticks with the space address.

**Entry.** Join on member-welcome's own-Mac variant.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your space, on this Mac:” | title | provisional | Own-Mac plan header. |
| “Joining your space” | line | provisional | Own-Mac row one. The spec doesn't say whether “(shroom does this part)” follows. |
| “✓ Your space — v.myshroom.workers.dev” | status | provisional | The row's tick with the fact once joined. The space address varies. |
| “→ Joining your space (shroom does this part)” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `joining · own-Mac variant`, row one · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `member-done` | flow | joined: the row ticks with the fact (own-Mac wording) | provisional |  |

### `member-done`

face · window · **provisional** · a face of `done` · cites N30, N17, N15

The member plan's resting beat. Row one is ticked with the fact; row two (row six verbatim) is current, with done's ratified nest verbatim and Allow recording. Nothing fires by itself: no macOS dialog to an empty chair (N17).

**Entry.** joining succeeded: the member is confirmed and the identity has landed locally. Diagram node member_done.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat's space, on this Mac:” | title | provisional | Plan header. The inviter's name varies. |
| “✓ Bagrat's space — v.myshroom.workers.dev” | status | provisional | Row one ticked with the fact (N15's outcome-bearing tick). The inviter's name and the space address vary. |
| “Letting macOS allow recording (microphone, camera and screen — a one-time yes each)” | line | provisional | Row two (= row six, N17), now current. N43: the camera joins the chapter. |
| “macOS will ask three times: your microphone and your camera — one click each — then screen recording, which goes through System Settings.” | line | provisional | done's ratified nest, verbatim (lines 599-600). N43: the camera joins the chapter. |
| “Flip the switch next to shroom there. When macOS offers to quit and reopen the app, say yes — I'll be right where you left off.” | line | ratified | done's ratified nest, verbatim (lines 601-602). |
| “Allow recording” | button | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Allow recording | filled-default | ↵ | `perm-mic` | Filled. Return comes from Law 8 (one default per surface); it is not restated for this face. Starts Region 3's chapter verbatim: perm-mic → perm-screen → relaunching → perm-priming. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `perm-mic` | flow | Allow recording | provisional |  |

### `member-all-set`

face · window · **provisional** · a face of `all-set` · board: AllSet · cites N30, N20, N5, N12

The member's final window face. All rows are green, with all-set's ratified closing block and two doors: Close · Record now (members don't invite, N5).

**Entry.** The permissions chapter ends on a member Mac and row two ticks (diagram: permissions → member_all_set).

| Said | Role | Copy | Note |
|---|---|---|---|
| “You're all set — ready to record.” | line | ratified | all-set's ratified line (N20), reused. |
| “shroom lives in your menu bar now — look for the little mushroom.” | line | ratified | all-set's ratified line (N20), reused. |
| “Close” | button | ratified |  |
| “Record now” | button | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | secondary |  | `idle-ready` | The retreat to tray-only life. |
| Record now | filled-default | ↵ | `config` | Filled, rightmost. Closes the window and opens config at the tray. Return per Law 8. |
| ✕ | close |  | `idle-ready` | ≡ Close (N20/N12, the window rule). Not restated for the member face. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | Close (diagram) | provisional |  |
| `config` | flow | Record now · Close | provisional |  |

### `member-all-set-own-mac`

face · window · **provisional** · a face of `all-set` · cites N30, N33, N20

The own-Mac all-set keeps all three doors (owner-class): Close · Invite someone · Record now.

**Entry.** The permissions chapter ends on a Mac joined through an own-Mac link.

| Said | Role | Copy | Note |
|---|---|---|---|
| “✓ Your space — v.myshroom.workers.dev” | status | provisional | The plan's ticked row (own-Mac). The space address varies. |
| “You're all set — ready to record.” | line | ratified |  |
| “shroom lives in your menu bar now — look for the little mushroom.” | line | ratified |  |
| “Close” | button | ratified |  |
| “Invite someone” | button | ratified |  |
| “Record now” | button | ratified |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | secondary |  | `idle-ready` |  |
| Invite someone | secondary |  | `invite-born` | Morphs this window into the Members face, field focused. |
| Record now | filled-default | ↵ | `config` | Return per Law 8. |
| ✕ | close |  | `idle-ready` | ≡ Close. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-ready` | flow | Close / ✕ | provisional |  |
| `config` | flow | Record now | provisional |  |
| `invite-born` | flow | Invite someone | provisional |  |

### `join-waiting`

failure-state · window · **provisional** · cites N31, N18

Joining can't reach the space: the Mac is offline, or the space answers with a transient error. The spinner rests, one line says why, and there is no button. It resumes by itself (N18's wizard-offline posture).

**Entry.** During joining: a transport failure (offline) or the space answering with a transient error (online but unanswered).

| Said | Role | Copy | Note |
|---|---|---|---|
| “You're offline — I'll finish joining the moment you're back.” | line | provisional | Face when offline. |
| “Bagrat's space isn't answering right now — I'll keep trying.” | line | provisional | Face when online but unanswered. The inviter's name varies. |
| “This invite's space isn't answering right now — I'll keep trying.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-waiting`, no inviter known (peek never got through) · copy table |
| “Your space isn't answering right now — I'll keep trying.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-waiting`, an own-Mac link whose space isn't answering · copy table |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `joining` | auto | auto-resume | provisional |  |

### `join-expired`

failure-state · window · **provisional** · board: JoinLine · cites N31, N29, N30

In the app: the Worker says the invite is expired, cancelled or unknown. One line + the paste door for a new link. Reads the same as the page's face.

**Entry.** During joining, the Worker answers expired, cancelled or unknown. Also a tray click in idle-unprovisioned after a remembered invite has died: shown once, then later tray clicks from idle-unprovisioned show the plain welcome.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This invite has expired — ask Bagrat for a fresh one.” | line | provisional | The inviter's name varies. |
| “Have a new link? Paste it here.” | field-label | provisional | The paste door. |
| “This link has expired — make a fresh one from Members on your other Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: An own-Mac link past its hour, in the app · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Have a new link? Paste it here. | field |  | `member-welcome` | Paste door. Resume = a new link (the paste door's arrivals land on member-welcome, line 1503). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `member-welcome` | flow | a new link pasted (Resume: a new link) | provisional |  |

### `join-expired-page`

face · web-page · **provisional** · a face of `join-expired` · cites N29, N33

The join page's expired face: the line alone, no install block. An unknown code gets the same face without a name. An own-Mac link has its own wording.

**Entry.** The join link is opened in a browser and the invite is past its days or was cancelled from Members; or the code is unknown; or an own-Mac link's hour has passed.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This invite has expired — ask Bagrat for a fresh one.” | line | provisional | Expired or cancelled. The inviter's name varies. |
| “This invite link doesn't work — ask whoever sent it for a fresh one.” | line | provisional | Unknown code: the wording without a name. |
| “This link has expired — make a fresh one from Members on your other Mac.” | line | provisional | The own-Mac link's expired face (N33). |

### `join-used`

failure-state · window · **provisional** · board: JoinLine · cites N31, N29, N3

In the app: the Worker says the invite was already used. This is the honest face for a forwarded or stolen link. Plus the paste door.

**Entry.** During joining, the Worker answers already used.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This invite was already used. If that wasn't you, ask Bagrat for a fresh one.” | line | provisional | The inviter's name varies. |
| “Have a new link? Paste it here.” | field-label | provisional | “+ the paste door”, worded as in join-expired's row. |
| “This link was already used — make a fresh one from Members on your other Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-used` in the app, an own-Mac link · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Have a new link? Paste it here. | field |  | `member-welcome` | Paste door. Resume = a new link. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `member-welcome` | flow | a new link pasted (Resume: a new link) | provisional |  |

### `join-used-page`

face · web-page · **provisional** · a face of `join-used` · cites N29, N3

The join page's used face (N3's “fails loudly for the real invitee”). It has a “You're in” variant when the joiner's own cookie rides along.

**Entry.** The join link is opened in a browser after it was used. The “You're in” wording shows when the joiner's own cookie rides along.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This invite was already used. If that wasn't you, ask Bagrat for a fresh one.” | line | provisional | The inviter's name varies. |
| “You're in — this invite already did its job. The mushroom in your menu bar is all you need.” | line | provisional | Variant for the joiner's own cookie. |
| “This link was already used — make a fresh one from Members on your other Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The join page, an own-Mac link already used (to anyone but the Mac it let in) · copy table |

### `join-space-missing`

failure-state · window · **provisional** · cites N31

The link's address answers as no space at all (no shroom Worker there). One line; no resume.

**Entry.** During joining, the link's address answers as no space at all.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This invite's space isn't there anymore — ask Bagrat about it.” | line | provisional | The inviter's name varies. |
| “This invite's space isn't there anymore — ask whoever sent it about it.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-space-missing` at arrival, no inviter known · copy table |

### `join-already-here`

failure-state · window · **provisional** · cites N31, N33

A whole-window face: this Mac is already in the invite's space, so there is nothing to do. The link stays unused so it can go to someone else. An own-Mac link opened on a Mac already in that space gets the own-space wording.

**Entry.** An invite arrives (any door) for the space this Mac is already in. Not after a removal: a re-invite then re-joins the same space (this Mac's identity is dead).

| Said | Role | Copy | Note |
|---|---|---|---|
| “This Mac is already in Bagrat's space — nothing to do. The invite is still unused, so it can go to someone else.” | line | provisional | The inviter's name varies. |
| “Close” | button | provisional |  |
| “This Mac is already in your space — open the link on the Mac you want to add.” | line | provisional | Own-space wording (an own-Mac link). |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Close | secondary |  |  | Closes the window; the link stays unused. Not marked filled; Return not specified. |

### `join-switch-confirm`

failure-state · window · **provisional** · board: SwitchConfirm · cites N31, N30, N36, N41

The invite window asks before anything moves. There is one space per Mac, so joining moves this Mac out of its current space; the videos there stay. A space's only token-holder gets one extra line. A half-set-up space counts too: an invite that arrives mid-wizard gets one different line.

**Entry.** An invite arrives and this Mac is in a different space, including a half-set-up one (token saved, nothing wired yet). Invites wait while a take is open (Law 4).

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat invited you to record with shroom.” | title | provisional | The inviter's name varies. |
| “This Mac records to v.mine.workers.dev now. Joining moves it to Bagrat's space — your videos in v.mine.workers.dev stay right where they are.” | line | provisional | The current space's address varies (example v.mine.workers.dev). The inviter's name varies. |
| “You can come back to your own space anytime by setting it up again with a fresh token — everything will be there.” | line | provisional | One more line, shown only when this Mac is its space's only token-holder. |
| “You've started setting up your own space. Joining Bagrat's instead sets that aside — your Cloudflare stays as it is.” | line | provisional | The one different line for a half-set-up space (the invite arrives mid-wizard). |
| “Join Bagrat's space” | button | provisional | The inviter's name varies. |
| “Stay” | button | provisional |  |
| “You can do this once your recording is finished.” | status | provisional | Law 4 (N41): 'joining another space' never runs mid-take. If a take is open while this window is up, Join greys with this line. |
| “Two recordings never reached v.mine.workers.dev — joining deletes them from this Mac.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-switch-confirm`, takes the old space can't take · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Join Bagrat's space | filled-default | ↵ | `joining` | Filled, first (leftmost): a twin of the welcome (Law 8's local exception; check at ratification). Return per Law 8. Greyed mid-take with Law 4's line. |
| Stay | secondary |  |  | The invite stays unused; the window closes. |
| ✕ | close |  |  | ≡ Stay. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `idle-unprovisioned` | flow | Stay | ratified | N31, B134 |
| `idle-ready` | flow | Stay | ratified | N31, B134 |
| `paused` | flow | Stay | ratified | N31, B134 |
| `joining` | flow | Join Bagrat's space | provisional |  |
| `join-switch-confirm-uploads` | flow | uploads still draining | provisional |  |
| `join-switch-confirm-saved-take` | flow | a recovered take waits (N25) | provisional |  |

### `join-switch-confirm-uploads`

face · window · **provisional** · a face of `join-switch-confirm` · cites N31

Join is greyed while uploads drain. It enables itself when the queue empties.

**Entry.** join-switch-confirm while uploads from the current space are still draining.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Two videos are still uploading — you can switch once they're done.” | line | provisional | The count varies (words for one to nine, Law 16). |
| “One video is still uploading — you can switch once it's done.” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: `join-switch-confirm`, one upload draining · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Join Bagrat's space | filled-default |  | `joining` | Greyed until the queue empties. |
| Stay | secondary |  |  |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `join-switch-confirm` | fail | the queue empties: Join enables itself | provisional |  |

### `join-switch-confirm-saved-take`

face · window · **provisional** · a face of `join-switch-confirm` · cites N31, N25

A recovered take waiting (N25) blocks the switch the same way, so nothing from the old space is ever orphaned.

**Entry.** join-switch-confirm while a recovered take waits.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Get the link for your saved recording first, or discard it.” | line | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Join Bagrat's space | filled-default |  | `joining` | Greyed (“blocks the same way”). |
| Stay | secondary |  |  |  |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `join-switch-confirm` | fail | the saved take is resolved (Get link or discarded) | provisional |  |

### `member-revoked`

banner · banner · **provisional** · a face of `config` · cites N34, N38, N32

A member-only banner on config under the amber dot: this member's access was turned off. Start recording is greyed. Recordings on this Mac are safe, and Save to Movies rescues waiting takes. The banner is derived and clears by itself when the cause clears.

**Entry.** A member Mac whose mint door refuses with the removed class (the owner removed this member). Diagram: idle_ready → member_revoked ('removed from the space').

| Said | Role | Copy | Note |
|---|---|---|---|
| “Your access to Bagrat's space was turned off. Recordings on this Mac are safe — ask Bagrat if that's a surprise.” | banner | provisional | The inviter's name varies. |
| “Save to Movies” | button | provisional | Only while takes wait. Save to Movies puts the take's single-file copy in ~/Movies under its title, and Finder shows it. The take keeps waiting until it is discarded from the Library. |
| “Save three to Movies” | button | provisional | When several takes wait. The count varies. |
| “Saved to Movies ✓” | status | provisional | The button after saving, with a quiet Discard… beside it. |
| “Discard…” | link | provisional | Quiet, beside “Saved to Movies ✓”. |
| “shroom recording — Aug 21” | other | provisional | The saved file's name when the take has no title. The date varies. |
| “Leave this space…” | link | provisional |  |
| “Save two to Movies” | line | provisional | one case the spec didn't print, so it varies by case — the copy table: The member banners, two takes waiting · copy table |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Save to Movies | other |  |  | Fill not marked. A banner's own button is never the Return default (Law 8). |
| Discard… | red-text |  |  | Appears after saving. A destructive entry point (Law 9). Its confirmation is not drawn here. |
| Leave this space… | text-link |  |  | Destination not drawn here. Settings → Space's Leave this space… is a sheet under N23's law (N36). |
| Start recording | filled-default |  |  | Greyed: a new video would have nowhere to go. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | the cause clears and the banner clears by itself (e.g. re-invited: the new link re-joins the same space and waiting takes drain) | provisional |  |

### `space-needs-owner`

banner · banner · **provisional** · a face of `config` · cites N34, N38

A member-only banner under the amber dot: the space needs a fix on the owner's side. Start stays live. Videos wait on this Mac and upload later by themselves.

**Entry.** A member Mac whose mint door fails with a space-side class (the space's own key revoked, storage switched off, billing). Diagram: idle_ready → space_needs_owner.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat's space needs a fix on Bagrat's side. Your videos wait safely on this Mac and upload by themselves once it's done.” | banner | provisional | The inviter's name varies. |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Start recording | filled-default | ↵ |  | Live: videos drain later. Config's Return default (Law 8). |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | the space is fixed: the banner clears by itself and waiting videos upload | provisional |  |

### `space-gone`

banner · banner · **provisional** · a face of `config` · cites N34, N38

A member-only banner under the amber dot: the space's address answers as no space at all. Start is greyed. Save to Movies rescues waiting takes, and Leave this space… is offered.

**Entry.** A member Mac whose space's address answers as no space at all. Diagram: idle_ready → space_gone.

| Said | Role | Copy | Note |
|---|---|---|---|
| “Bagrat's space isn't there anymore. Recordings on this Mac are safe — save any you want to keep.” | banner | provisional | The inviter's name varies. |
| “Save to Movies” | button | provisional | Only while takes wait. Save to Movies puts the take's single-file copy in ~/Movies under its title, and Finder shows it. The take keeps waiting until it is discarded from the Library. |
| “Save three to Movies” | button | provisional | When several takes wait. The count varies. |
| “Saved to Movies ✓” | status | provisional | After saving, with a quiet Discard… beside it. |
| “Discard…” | link | provisional |  |
| “shroom recording — Aug 21” | other | provisional | The saved file's name when the take has no title. The date varies. |
| “Leave this space…” | link | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Save to Movies | other |  |  | Fill not marked. Never the Return default (Law 8). |
| Discard… | red-text |  |  | After saving. A destructive entry point (Law 9). |
| Leave this space… | text-link |  |  | Destination not drawn here (N36's Leave sheet). |
| Start recording | filled-default |  |  | Greyed. |

| → | Kind | Label | Status | Cites |
|---|---|---|---|---|
| `config` | flow | the space comes back: the banner clears by itself and waiting takes upload | provisional |  |

### `owner-mac-removed`

banner · banner · **provisional** · a face of `config` · cites #21

A removed owner-class Mac (its /me or mint answers 401, or 403 removed): config's banner under the amber dot, Start greyed, a stop stranded. N34's three faces are member faces; this is the owner's own Mac.

**Entry.** An owner-class Mac's /me or mint answers that it was removed from the space.

| Said | Role | Copy | Note |
|---|---|---|---|
| “This Mac was removed from your space. Recordings on this Mac are safe — add it again from Members on your other Mac.” | line | provisional |  |
| “Save to Movies” | button | provisional | while takes wait |
| “Disconnect this Mac…” | button | provisional |  |

| Action | Kind | Return | Goes to | Note |
|---|---|---|---|---|
| Save to Movies | secondary |  |  |  |
| Disconnect this Mac… | secondary |  | `disconnect-confirm-no-token` |  |
| Start recording | filled-default |  |  | Greyed: a new video would have nowhere to go. |

#### Region 6's amendments

- **N29** (amends N28): The inviter's name on the join page uses macOS's short, locale-aware form of the account name ('Bagrat' for 'Bagrat Aznauryan'). This amends N28's 'full name', which its own copy never showed.
- **N30** (amends N12 (Region 1's ratified welcome)): Adds one quiet line under the welcome's buttons, “Have an invite link?”, which opens the classified paste door (a join link → member-welcome; a token → the wizard at token).
- **N31** (amends N18): Replaces the merged expired/used row with first-class join states: join-waiting, join-expired, join-used, join-space-missing, join-already-here, join-switch-confirm.
- **N33** (amends U10): The second Mac joins through Add another Mac on the You row (owner-class, one hour, one-time), not as a plain member, which would silently regress P12.
- **N33** (amends N28): The You row gains the quiet Add another Mac, and the owner's other Macs appear as sub-rows (“You · MacBook Air — joined Sep 2 · Remove…”). A Mac that holds the token shows “keeps your Cloudflare token” and has no Remove….
- **N33** (amends N3): Deliberately extends N3's argv/shell-history exception to own-Mac links: they carry owner powers, but last one hour and work once.
- **N36** (amends N28): Settles N28's open placement: Members is the Settings window's third tab, owner-class only; ✕ ≡ Close.
- **N40** (amends U2): The owner cookie names a member and owner-mode follows role. A member's page gives the title only; the space-global mark switch belongs to owner-class Macs (member life, 1569-1571).
- **N41** (amends Region 1's “every launch is silent” (and refines N9)): An install command carrying --join opens the invite even in the otherwise silent updating lane. In the member lane, an already-installed Mac relaunches into member-welcome / join-already-here / join-switch-confirm.
- **N6** (amends U6): U6's “Record together” section dissolved into the landing hero's sixth point; the claim stands, promoted (ratified 2026-07-25).

#### Region 6's open notes

- Status split per the region heading and inventory (1282, 1704-1709). Ratified: the member model (N3/N4/N5, 2026-07-20) and the Members face copy (N28, 2026-08-21). Provisional: the You row's Add another Mac and the owner-Mac sub-rows (N33), and everything from join-landing through member life (N29–N34, designed 2026-09-30, awaiting ratification). The guided-agent overlay is structure-ratified (2026-07-20) with draft copy.
- Check at ratification (Law 8, lines 141-143): member-welcome and join-switch-confirm put the filled action first, following the ratified welcome's order. This is a deliberate local exception to macOS's default-rightmost order. Law 8 also names the saved-take banners as leading with their one filled button; whether member-revoked/space-gone's Save to Movies is filled is not stated.
- Return is unspecified in several places. On invite-born while a new link is on screen, Copy link is filled but the field-owned Return ≡ Invite (Law 8). Add another Mac's filled Copy link, join-already-here's lone Close, and the web page's Copy/Open shroom have no stated Return. Return on done · member's Allow recording and all-set · member's Record now comes only from Law 8's general rule. The spec names Return explicitly only for config/paused, the name field, and the no-default confirms.
- join-expired in the app gives only the named wording for 'expired, cancelled, or unknown' (1539). The page has a separate wording without a name for an unknown code (1454). N31 says the join failures read the same in the app as on the page (79, 1532-1534), so the in-app wording for an unknown code is ambiguous. The in-app wording of join failures for own-Mac links is not given either; only the page's own-Mac expired face is (1460-1461).
- join-switch-confirm for a half-set-up space gets 'one different line' (1561-1563); which line it replaces is not stated (presumably 'This Mac records to…'). join-landing-elsewhere says 'the hero stays' (1453); whether the sub-hero paragraph and the mark stay is not stated.
- Undrawn exits: join-space-missing has no actions and no resume ('—'). Where join-already-here's Close and join-switch-confirm's Stay land is not drawn (the window closes; the Mac's idle state). The destination of Leave this space… in member-revoked/space-gone is not drawn (Settings → Space has a sheet of the same name, N36). Save to Movies' Discard… confirm is not drawn here (deleting is the Library's job, N35). The diagram has no edges out of join-expired/join-used (the table's resume is 'a new link' via the paste door) or out of the guided overlay.
- Fixture conventions used here. Faces without a backticked name are named '<parent> · <the spec's face name>'. The diagram nodes member_done / member_all_set / permissions map to 'done · member' / 'all-set · member' / Region 3's chapter (perm-mic → perm-screen → relaunching → perm-priming). The page renders of join-expired/join-used are faces of the in-app states ('· page'), because the diagram has one node for each and N31 says they read the same. Copy variants that swap a single line are folded into their parent with a condition note: offline/unanswered, unknown code, own-Mac expired, You're in, own space, only token-holder, half-set-up, Law 4's mid-take greyed line, and Save three / Saved ✓. Markdown ** and backticks are stripped from said-lines, plan glyphs (→ • ✓) are kept as written, and the spec's apostrophes are straight (') throughout this region (verified: zero U+2019 in 1282-1663 and in the reused lines 590-605/715-725).
- Cross-region edges drawn in Region 6's diagram whose source state lives elsewhere: idle-unprovisioned → member-welcome (tray click while the invite lives, 1497-1498, 1616); idle-unprovisioned tray click after the invite died → join-expired once, then the plain welcome on later clicks (1498-1499; an edge out of idle-unprovisioned, not out of join-expired); all-set's Invite someone → invite-born (N20, 68, 1605); the end of the permissions chapter → all-set · member (row two ticks, 1628); idle-ready → member-revoked / space-needs-owner / space-gone (1631-1633). The paste doors that land on member-welcome: the welcome's 'Have an invite link?' → field 'Paste your invite link' (a token → the wizard at token; anything else → 'That doesn't look like an invite link.', 334-341), and Settings → Space's 'Have an invite link? Paste it here.' (1200).
- Lines restated in this region that belong to other regions: U10's final form (1597-1598) restates Region 2's space-found line “Found your space — v.myshroom.workers.dev, 14 videos. This Mac is joined.” (D, N42, line 588); U6 (1658-1659) restates the landing hero's point six “Your team just accepts an invite and records — no Cloudflare setup for them.” (ratified, N6, line 202).
- Membership facts without a state of their own. When a member leaves, the owner sees their row simply go; their videos stay, with their label, in Everyone (1564-1565). Removed members' videos keep their label in the Library; removal and deletion are never bundled (1369-1371). If re-invited after removal, the new link re-joins the same space (not join-already-here) and waiting takes drain (1588-1589). A member never sees repair tooling (1590).
- A member's second Mac is a second invite: a separate row and a separate library. Members have no Add another Mac in v1, which is named as a post-v1 candidate (1400-1401, 1733-1734). Owner-class Macs other than the token-holder can do everything except repair or re-wire the space (1405-1407).
- Secrets and argv: Later stores the remembered invite under ~/.shroom, mode 600 (1496-1497). The join link rides argv/shell history once: one-time use, a 7-day TTL, and one hour for own-Mac links. The admin token never gets this exception (1465-1468, 1389-1392, Law 13).
- Quiet-take law: invites and deep links wait while a take is open (Law 4, 116-120). A mid-take install goes to install-refused-midtake with the link still unused (1474). Joining another space is among the acts that never run mid-take; their buttons grey with “You can do this once your recording is finished.” (120-123).
- N29's three leg-19 flags were resolved as recommended: the link is the code (one artifact, three doors; bare and https:// forms accepted everywhere a link is pasted, 1413-1420); the Terminal hand-hold stays (flag 2, 1444); the inviter's name uses the short form. The name comes from the Mac account in macOS's short, locale-aware form, used silently, with no field anywhere; a wrong name is a post-v1 fix (1351-1353).
- Join page facts: it is served by the space's own Worker with no shroom.am dependency. Its title is the hero line (for the unfurl) and it is noindex (1424-1426). It shows no prices, no Cloudflare and no card (1443). The members surface's N5 line is said once, with nothing about prices (1348-1350).
- No Fat-marker status notes appear in this region.

#### Region 6's scaffold flags

- Instant member cut-off = rotating the parent token (an admin operation). Whether the UI offers it is a scaffold flag. An instant member cut-off control is listed as deliberately absent from v1 (parent-token rotation UI, N3).
- Per-recording credential minting at upload start. member-remove-confirm's “a video already on its way can still finish” is true only if creds mint per recording. Per-member creds would force the line to become “for up to a week”: the copy is bound to the mechanism.
- Own-Mac invites: one hour, one-time, owner-class, labelled silently by the joining Mac's sharing name. Every owner Mac records as member #1 with its own device key, which is what Remove revokes. A Mac that holds the Cloudflare token mints from that token and can't be stopped by a device key (no Remove…; Disconnect, or delete the token in Cloudflare).
- Deep link registration (shroom://join/…) at first launch. The browser's own “open shroom?” prompt is the only step it adds (N29). [ Open shroom ] on the join page is this deep link; the paste line is its fallback.
- The member cookie: owner-mode keys on the member, not the space (N40). The Worker checks maker vs cookie, and owner-class sees all. A member's owner-mode on a page is the title only; the space-global mark switch belongs to owner-class Macs.
- R10's index/layout work is v1-urgent. The index must be safe for many writers from day one: per-recording objects (rec/<id>/ + meta/<id>.json), no per-member prefix, no shared mutable index. Named consequence, not in the spec's flags list.
- The deterministic status seam (engineering requirement, not in the spec's flags list): the app exposes setup, permission and join state as machine-readable status, like the POC's --prep JSON. The guided agent narrates state and never infers from screenshots.

## Rules

A move that holds for a whole group of states, written once (B133). The tests expand each into its moves.

- **offline, then back where the facts stand** (`offline`, fail, **ratified**, N18, N42): @network-bound → wizard-offline (and back), on any. wizard-offline interposes on any network-bound step and resumes where the facts stand.
- **a call answering proves the network is back** (`offline-answer`, auto, **ratified**, ARCHITECTURE §3.3, B134): wizard-offline → *, on Cloud. A wizard call that was out when the network dropped answers after all: the face re-derives from its answer.
- **an unclassified failure, and back** (`unexpected`, fail, **ratified**, #25, B134): * → wizard-unexpected (and back), on any. The one face for a failure nothing else classifies, from any wizard face, and back when an answer clears it or Try again.
- **✕ closes the window** (`close-window`, side, **ratified**, N19, N21, B145): * → @idle, closed, config, on WindowClosed, Act(CloseWindow), QuitRequested. ✕ closes the window at any row — Later's twin: nothing is lost, and the next door re-derives it. ⌘Q with one of shroom's windows in front and no take open does the same: that window closes, and shroom stays in the menu bar (B145). The Dock's Quit, the popover's and the right-click menu's still quit.
- **every paste meets the shape check, then verify** (`token-paste`, auto, **ratified**, N14, ARCHITECTURE §3.7): @token-row → @token-row, on SecretPasted(Token), answer. Any new paste starts the token row over: the shape check first, then verify; the row's spinner while a paste is checked or saved (the Checking beat).
- **the rows mark themselves from the facts** (`rows-mark-themselves`, auto, **ratified**, N13, law 1): @wizard-rows → @wizard-rows, on answer. An answer can tick several rows: the plan's rows mark themselves from the facts each answer brings (token → gates → bucket → page → name).
- **the chapter's step follows the grants** (`chapter-derived`, auto, **ratified**, N17, law 1): @chapter → @chapter, on answer. Row six's step is derived from the grants and the warm-up, never stored: a grant already given skips its step, one switched off steps back.
- **Turn it back on** (`turn-back-on`, flow, **ratified**, N36, B134): * → perm-screen, on Act(TurnBackOn). Row six is live on every Mac: Turn it back on runs the chapter's screen step wherever recording-off shows.
- **the Space tab re-probes every row** (`space-reprobe`, auto, **ratified**, N42, B134): @repair → @repair, on answer, WindowFocused. Settings · Space re-probes every row on entry; the first broken row is current with Region 2's state beneath it, or every row ticks (the inspection).
- **click-away or Esc closes it** (`popover-away`, side, **ratified**, law 9, N21): @popovers → @idle, closed, paused, recording, on away. Any popover closes on click-away or Esc; nothing is lost, and the tray stands where the facts are.
- **paused shows its cause** (`paused-faces`, auto, **ratified**, N24, N39): @paused-faces → @paused-faces, on answer, click. The auto-pause faces are faces of paused, re-derived from the cause and the Mac's presence; the tray click on the pause bars opens the popover at its cause; a Mac away shows nothing until it's back.
- **Quit with uploads draining asks first** (`quit-asks`, side, **ratified**, N21, B145): @idle, closed, config, discard-confirm, quit-confirm, stopped-offline, stopped-stranded, permission-lost, disk-full-stop → quit-confirm, on QuitRequested. Quit shroom — the right-click menu or the popover's — while uploads drain asks first, over whatever idle popover stands. ⌘Q with a shroom window in front closes that window instead (B145).
- **the take finalizes itself** (`take-stops-itself`, fail, **ratified**, N39, #6): @paused-faces, recording, discard-confirm, restart-confirm → stopping, permission-lost, disk-full-stop, on answer. Mid-take — a paused or asking take too — the screen grant revoked or the disk at its floor finalizes the take: the stop beat, then its own popover.
- **a stop's popover holds until dismissed or answered** (`stop-popover-holds`, side, **ratified**, N34, N39, N40, B134): @stop-popovers → config, @idle, closed, on any. A stop's popover is the stop's own: it holds until dismissed or answered (back online, the page opened); the next door is config.
- **the tray click is the door** (`tray-door`, flow, **ratified**, law 5, N21, #5): @idle, closed → config, @idle, closed, @wizard-rows, @chapter, welcome, on click. The tray click is the door: idle → config, or the setup window at the row the facts derive (a Mac whose chapter isn't done opens at row six).
- **a relaunch re-derives where you were** (`relaunch`, auto, **ratified**, law 1): * → *, on Booted, Reopened. Every entry re-probes: a launch or a relaunch lands on derive(probes), never on a remembered step.
- **Disconnect… or Leave…** (`exit-sheet`, side, **ratified**, N36, B134): @space-tab, config → @exit-sheets, on Act(Disconnect). The Space tab's exit is there in every mood: Disconnect… (an owner-class Mac) or Leave… (a member) opens its sheet — from a banner's button too (a removed Mac, a member's).
- **Back keeps the space** (`exit-sheet-back`, side, **ratified**, N36, law 9, B134): @exit-sheets → @space-tab, on Act(Back), Key(Esc), away. Back, or Esc, closes the sheet; the Space tab re-probes where it was.
- **Settings opens where it should** (`settings-door`, flow, **ratified**, N36, N42, law 1, B134): config, stopped-stranded → @settings-tabs, @space-tab, on Act(OpenSettings), Act(ReconnectSpace), Act(AddToken). The gear opens Settings on General the first time, then the tab last used; Reconnect your space and Add a token here open the Space tab, which re-probes and lands on the row the facts derive.
- **the tabs** (`settings-tabs`, flow, **ratified**, N36, law 1, B134): @settings-tabs, @space-tab → @settings-tabs, @space-tab, on Act(SettingsTab). Clicking a tab is an entry: General, Space (which re-probes, law 1) and Members (owner-class only).
- **the settings link opens Settings at its spot** (`settings-link`, flow, **ratified**, N62, law 4, B160): @idle, @popovers, @settings-tabs, @space-tab → settings-general, @idle, on SettingsLink. The link the shroom command's recording refusal carries (`shroom://settings/agent`, N62) opens Settings on General with the switch “Let it start and stop recordings” in view and lit for a moment: from the tray at rest, over an idle popover (it closes, as a click away closes it), and from another tab. It flips nothing. A sheet open in Settings stays, and Settings comes forward as it stands. Mid-take it opens nothing (law 4), and a Mac with no space has no Settings (B160).
- **an invite opens its window** (`invite-arrives`, flow, **ratified**, N30, N31, #2, #20, B134): @join-doors → @join-arrival, on Member, Failed(Peek), SecretPasted(Invite), click, Scanned. An invite that arrives — the deep link, a paste, the installer's file, or one remembered at the tray click — opens the invite window at the face peek's answer derives: the welcome, waiting, a dead link's line, already here, or the switch's question.
- **a face comes and goes with its cause** (`faces`, auto, **ratified**, N18, law 1, N34): * → * (a state and its own faces), on any. Faces are render properties of their state: a state and its faces, and one state's faces, re-derive into one another as their causes come and go.
- **one banner at a time, the highest** (`banners`, auto, **ratified**, N38, law 1): @banners, none → @banners, none, on any. config's banner shows while its cause stands and clears by itself when it goes; a higher banner hides a lower one (one at a time).
- **its video deleted, the editor closes** (`editor-video-deleted`, side, **ratified**, N63, B199): @editor-faces → library, on Act(ConfirmDelete), Library(Deleted). Deleting the video the editor holds — Delete… confirmed in the Library, or this Mac's delete landing — closes the editor: what its open asked for stops, a save on its way answers no one, and nothing is saved (B199). Another Mac's delete reaches it as a save the space turns down (editor-save-refused).
- **Quit or Update saves an unsaved edit first** (`quit-saves-the-edit`, side, **ratified**, N63, B199): editor, editor-save-failed → editor-saving, on QuitRequested, Act(Quit), Act(QuitAnyway), Act(Update). Quit — the Dock's, the popover's, the right-click menu's or macOS's, and Quit anyway once uploads are asked about — and Update, while the editor holds changes not saved yet, save them first, as ✕ and ⌘Q do, and bring the editor forward; a save already on its way is waited for. Once it lands the quit or the update goes on, without waiting for the final video (an upkeep makes it later). A save that doesn't land holds it: shroom stays, and the editor says why. A take begun meanwhile drops it (law 4) (B199).

## Groups

- **@network-bound** — the wizard's network-bound steps (N18): `token`, `card`, `gate-email`, `gate-card`, `name`, `name-taken`, `wiring`, `space-found`, `repair-mood`, `token-repair-mood`, `re-entry-as-settings`
- **@token-row** — the token row and its faces (N14): `token`, `token-repair-mood`, `token-garbled`, `token-wrong-powers`, `token-other-account`, `token-pick-account`
- **@wizard-rows** — the plan's rows, account to done (N13): `account`, `account-signing-up`, `token`, `token-garbled`, `token-wrong-powers`, `token-pick-account`, `card`, `gate-email`, `gate-card`, `name`, `name-taken`, `wiring`, `space-found`, `done`
- **@chapter** — the permissions chapter (row six) (N17): `done`, `perm-mic`, `perm-screen`, `relaunching`, `perm-priming`, `all-set`
- **@repair** — Settings · Space's re-probe and the rows it can land on (N42): `re-entry-as-settings`, `repair-mood`, `token-repair-mood`, `token`, `token-garbled`, `token-wrong-powers`, `token-other-account`, `gate-email`, `gate-card`, `name-changed`, `wiring`, `space-storage-gone`, `perm-screen`, `relaunching`, `settings-space-no-token`, `settings-space-member`
- **@popovers** — the tray's popovers (N21): `config`, `paused`, `discard-confirm`, `restart-confirm`, `quit-confirm`, `stopped-offline`, `stopped-stranded`, `permission-lost`, `disk-full-stop`
- **@paused-faces** — paused and its auto-pause faces (N24, N39): `paused`, `display-lost`, `mic-lost`, `camera-lost`, `mac-away`, `mic-permission-lost`
- **@auto-pause** — the auto-pause faces (the map's “mic · camera · sleep · lock”) (N39): `mac-away`, `mic-lost`, `camera-lost`, `mic-permission-lost`
- **@stop-popovers** — a stop's own popovers (N34, N39, N40): `stopped-offline`, `stopped-stranded`, `permission-lost`, `disk-full-stop`
- **@idle** — the tray at rest (law 5): `idle-ready`, `idle-unprovisioned`
- **@space-tab** — the Space tab, every mood and the Region 2 state beneath it (N36, N42): `settings-space`, `settings-space-inspection`, `settings-space-repair`, `settings-space-no-token`, `settings-space-member`, `re-entry-as-settings`, `repair-mood`, `token-repair-mood`, `token`, `token-garbled`, `token-wrong-powers`, `token-other-account`, `gate-email`, `gate-card`, `name-changed`, `wiring`, `wizard-offline`, `wizard-unexpected`, `space-storage-gone`, `perm-screen`, `relaunching`
- **@exit-sheets** — the Space tab's exit sheets (N36): `disconnect-confirm`, `disconnect-confirm-no-token`, `leave-confirm`
- **@settings-tabs** — the Settings window's tabs (N36): `settings-general`, `settings-members`, `invite-born`, `add-another-mac`
- **@join-doors** — where an invite can arrive (N30, N31): `idle-ready`, `idle-unprovisioned`, `config`, `welcome`, `first-launch`, `join-waiting`, `join-expired`, `join-used`, `join-space-missing`
- **@join-arrival** — the invite window's faces on arrival (N30, N31, #20): `member-welcome`, `join-waiting`, `join-expired`, `join-used`, `join-space-missing`, `join-already-here`, `join-switch-confirm`, `join-switch-confirm-uploads`, `join-switch-confirm-saved-take`, `joining`
- **@banners** — config's banners (one at a time, N38) (N38): `space-degraded`, `space-degraded-no-token-mac`, `recording-off`, `recovered-take`, `update-ready`, `updating-app`, `space-needs-update-member`, `space-needs-update-no-token`, `member-revoked`, `space-needs-owner`, `space-gone`, `owner-mac-removed`
- **@editor-faces** — the editor's faces (N63, B199): `editor`, `editor-getting`, `editor-cant-get`, `editor-saving`, `editor-save-failed`, `editor-save-refused`, `editor-making`, `editor-making-failed`, `editor-keep-original`

## Laws

1. **State is derived, never a step counter** — every entry re-probes reality (POC §6/§8, Region 2's backbone). A face clears by itself when its cause does (N34).
   - Persisted state under ~/.shroom holds no wizard step counter or step index; the current state id is a pure function of probe results: derive(probes) called twice with identical probes returns the identical state id.
   - Every entry (launch, relaunch, reconnect, window reopen, tray click) runs the probes before choosing a state; no transition target is read from a stored 'last step'.
   - Kill + relaunch in any state S lands in derive(probes), not in S by memory — e.g. a relaunch mid account-signing-up re-enters at the virgin fork (N19); wizard-offline on reconnect routes by derived state (N18).
   - Every derived face (member-revoked, space-needs-owner, space-gone, space-degraded, recording-off) disappears with no user action once its cause probe clears.
2. **The recording is sacred** — capture is local whatever the network or the space does; a crash leaves a saved take (N25); a full disk finalizes (N39); a space that went away leaves Save to Movies (N34). Nothing shroom does ever loses a take the user didn't discard.
   - No network, space, or upload failure event causes a transition out of recording or paused; capture bytes keep landing locally whatever those probes say.
   - Local take bytes are removed only by an explicit user act — discard-confirm → Discard, the saved-take banner's Discard… confirmed, or Restart (whose old take dies only when the new capture begins, N39); no crash, disk-full, auto-pause, offline, or space-loss transition deletes them.
   - After a simulated crash mid-take, the next launch has the take saved and the recovered-take banner present in config with the amber tray face.
   - A full disk mid-take enters disk-full-stop with the take finalized, never discarded or auto-published; left unanswered it remains a saved take (disk_full_stop → recovered_take, L898).
   - When a take can't reach the space (stopped-stranded, member-revoked, space-gone), Save to Movies is offered; it copies the single file to ~/Movies and the take keeps waiting until discarded (N34).
3. **Only you make a link** — a link comes into being by Finish & Share (the stop) or Get link, never by a crash, a full disk, or an auto-pause (N25, N39). Stop = published (§5.3) — the one place that's said is the paused popover's “Finish & Share”, by its name (B143: it was Stop, with a quiet line beneath).
   - A link/page id comes into being only on Stop or Get link events; crash recovery, disk-full-stop, and every auto-pause (display-lost, mic-lost, camera-lost, mac-away, permission-lost) create no link and open no page.
   - disk-full-stop and a lost screen grant finalize and ask; neither transitions to a page-open or upload-published state without a user Stop/Get link.
   - No surface says “Your link opens right away.”: Bagrat took the quiet line off the paused popover's stop, now “Finish & Share” (B143), so it reads like the rows beside it.
4. **The quiet-take law** *(N41)* — while a take is open (recording *or* paused), shroom puts nothing new on screen: invites, deep links, windows, the deferred page of an offline stop, update banners — all wait for the take to end. Only the bubble and macOS's own indicator can ever appear in a take; the paused popover exists only after the pause is confirmed (POC §8). **And nothing that would end, move or replace the app runs mid-take** — Update, Disconnect, Leave, joining another space, Quit (⌘Q or the Dock, reachable when a window was already open) — their buttons grey with “You can do this once your recording is finished.”; a Quit request is declined silently while recording and opens the paused popover (its “no Quit while a take is open” note) while paused.
   - Let take_open = state ∈ {recording, paused, and paused's morphs/auto-pause faces}. While take_open, no window, banner, invite, deep-link handling, deferred offline-stop page, or update banner becomes newly visible; the only shroom surfaces that may newly appear are the bubble overlay, the paused popover (only after the pause is confirmed) and its morphs (discard-confirm), and the auto-pause popovers (display-lost, mic-lost, camera-lost, mac-away, permission-lost, disk-full-stop).
   - Items that arrive while take_open (invite deep link, offline-stop page, update banner, windows) are queued and surface only after the take ends.
   - While take_open, Update, Disconnect, Leave, join-another-space and Quit controls are disabled and show “You can do this once your recording is finished.”
   - A Quit request (⌘Q or Dock) while recording causes no state change and no visible surface; while paused it opens/keeps the paused popover showing its “no Quit while a take is open” note; the process keeps running in both.
   - The paused popover is never visible before the capture engine confirms the pause (POC §8).
   - An install/update run while take_open lands in install-refused-midtake and touches nothing (N9).
5. **The click is the door** (N21) — no menus; the mushroom opens the one surface the moment needs. **A launch you did yourself is a click** (N41); launches shroom does itself stay silent: the login item opens nothing, and an update relaunch restores the window that was open before it and opens the popover by itself, under the mushroom, to say how the update went (B155). When the mushroom can't be seen, the popover drops from the top-right of the screen with the pointer; while the menu bar is hidden (a full-screen app, or set to hide) it opens where the mushroom rests (B161). A popover opens unasked only for a pause or stop you didn't make (N39) or after an update relaunch (B155) — and **an unasked popover never takes the keyboard**: a Return typed into another app stays there.
   - Left-click on the tray never shows a native menu in any state; in idle-unprovisioned it opens welcome (or member-welcome while an invite remembered under Later still lives, N30), in idle-ready it opens config, in recording it pauses (popover only after the pause is confirmed).
   - A user-initiated launch (Spotlight, Finder, Dock, a fresh install onto a Mac that remembers its space, the reopen event) yields exactly the transition a tray click yields in the same state.
   - A login-item launch opens no surface. An update relaunch restores only the window that was open before it, and opens the popover unasked with the update's one-shot status line (B155). Sole other exception: an install command carrying --join opens the invite (N41).
   - The popover an update relaunch opens is non-activating like every unasked popover, but a click anywhere else closes it, as it closes a popover you opened; the other unasked popovers stay until touched (B155, DEVIATIONS #39).
   - Popovers opened unasked (the auto-pause/auto-stop family) are non-activating: shroom's key-window/focus state is unchanged and a Return typed into the frontmost other app is not delivered to shroom.
   - When the status item is occluded, the popover is anchored at the top-right of the screen that has the pointer; a status item parked over a hidden menu bar anchors it where the item rests (B161).
   - Right-click yields the one-item native menu “Quit shroom” only in idle states; mid-take a right-click behaves as a click (surfaces table, N39).
6. **Dock iff a window is open** (Region 1). Popovers never bring the dock icon.
   - For every state: dock_icon_visible == (number of open shroom windows > 0).
   - Opening or closing any popover, banner, or overlay (the countdown scrim, the bubble) never changes dock_icon_visible.
   - Popover-only states (config, paused, discard-confirm, quit-confirm, stopped-offline, stopped-stranded, auto-pause faces) with no window open have no dock icon.
   - Confirmed by Bagrat at the 0.1.10 checkpoint (B138): the Library, Settings and the setup window each bring the Dock icon while open, after setup too; with every window closed it goes, so a set-up shroom that opens or comes back from an update with no window shows none.
7. **Asks are banners, facts are the status line** (N38) — one of each at most; the amber dot means *something needs you*, the Frost dot *something new from us*.
   - At any moment the popover shows at most one banner and at most one status line.
   - With several banner causes active, the shown banner is the highest of: a saved take > the space can't take videos > recording-off > an update (N38).
   - Banners carry the ask (an action); the status line carries facts only (upload truth, then one-shot lines) and no action button.
   - Tray face is exactly one of {plain, red dot, pause bars, amber, Frost}: red dot iff recording, pause bars iff paused, amber = something needs you (a saved take, the space can't take videos, recording off), Frost = an update is ready and is the lowest face — shown only when no other non-plain face applies (N37, N38).
8. **One default per surface, and Return triggers it** (N38) — Start in `config`, Resume in `paused`; *click the mushroom, press Return* is the whole path to a recording. A banner's own filled button is never the Return default, and a focused field's Return belongs to its field (Invite in Members, Claim in `name`). Button order follows macOS — the default rightmost — except the ratified welcome pair, whose filled action comes first; its twins (`member-welcome`, `join-switch-confirm`) follow it, and so do the single-action faces (`stopped-stranded`, the saved-take banners), whose one filled button leads. *(A deliberate local exception — check it at ratification.)*
   - Each state has at most one default action, and Return activates exactly it: config → Start, paused → Resume.
   - From idle-ready with Start enabled, the event sequence tray-click, Return reaches countdown with no other input.
   - A banner's filled button (e.g. the saved-take banner's Get link, the update banner's Update) is never the Return target; Return in config with a banner showing triggers Start when Start is enabled, and nothing when a banner greys Start (member-revoked, space-gone).
   - With a text field focused, Return triggers that field's action: Members composer → Invite; name → Claim this name (and only once the charset check passes).
   - In every face that has a default/filled action among several buttons, the default is rightmost, except welcome, member-welcome, join-switch-confirm, stopped-stranded and the saved-take banners, whose filled action comes first.
9. **Destruction is never one stray keystroke away** (N23, N24) — destructive entry points are red text, never buttons; destructive questions are red-outlined, rightmost, never filled, with **no Return default**; Esc always means *back*.
   - Every destructive entry point (Discard…, Remove…, Delete…) renders as red text, never as a button.
   - Every destructive question (discard-confirm, the saved-take Discard… confirm, member-remove-confirm, the Library delete sheet, the Disconnect/Leave sheets) renders its destructive button red-outlined, rightmost, never filled.
   - In every destructive-question state, Return causes no transition (no Return default).
   - In every confirm/popover state, Esc maps to back (e.g. discard-confirm Esc → paused; click-away ≡ Back) and never to the destructive action.
10. **Nothing auto-copies** (N1) — copying is always a visible button that reads “Copied”.
   - No transition writes the system clipboard unless it is a user-invoked copy (a visible Copy link button click; ⌘C in the Library, N35).
   - Entering the owner page after Stop/Get link, invite-born, stopped-offline, or any other state leaves the clipboard unchanged.
   - Every Copy link button reads “Copied” immediately after its click, then reverts.
11. **No dialog to an empty chair** (N17) — a macOS prompt appears only right after a click that asked for it; the monthly re-confirm is pulled forward to `config` so it never lands inside a take (N38).
   - Every macOS permission prompt (microphone, screen, camera) is requested only inside the handler of a user click that asked for it (e.g. Allow recording, a camera picked in the popover, Camera as the source).
   - done never fires a permission dialog on entry (N17).
   - No macOS prompt — including the monthly screen-recording re-confirm — appears while a take is open; the re-confirm is surfaced at config (the recording-off banner) before Start.
12. **Failures are states** (N18) with scaffold-grade names and their own renders; guards that act as you type filter and never scold (N15).
   - Every blocking failure in the spec maps to a distinct kebab-case state id with its own render and its own set of enabled transitions (e.g. install-net-drop, install-checksum-fail, install-unsupported-os, install-refused-midtake, token-garbled, token-wrong-powers, gate-email, gate-card, name-taken, wizard-offline, the join failures).
   - As-you-type guards (the name charset filter) cause no state transition: rejected characters never land, uppercase lowercases silently, and no scolding/error line renders (only the quiet fading explanation).
   - wizard-offline auto-resumes on reconnect with no retry button.
13. **Secrets ride files** under `~/.shroom`, mode 600 — never argv, chat, or logs. One scoped exception: a join link — a member's or an own-Mac one — once, in an install command (N3, N29, N33).
   - Every file under ~/.shroom that holds a secret has mode 0600.
   - No spawned argv, log line, or chat-facing text contains a token, temp credential, or device key — sole exception: a join link (member or own-Mac) passed once via --join in the install command.
   - Join links accepted on argv are one-time-use; member links expire after seven days, own-Mac links after one hour (N3, N33).
   - Stored secrets (the admin token, member/device credentials, an invite remembered under Later — N30) exist only as files under ~/.shroom, never in argv, chat, or logs.
14. **One space per Mac** (N31); **every Mac of the owner is owner-class** (N33); members own their videos, the owner sees all (N5).
   - A Mac is joined to at most one space at any time; moving to another space happens only through join-switch-confirm (blocked while uploads drain or a recovered take waits).
   - Every Mac joined via Add another Mac, and every Mac holding the token, has owner-class role: its Library can show all members' videos and it sees the Members tab.
   - A member Mac lists and manages only videos whose maker is that member; owner-class Macs see and can delete all.
15. **Product voice** — *your space · an invite · members · your link*; internals never surface (no error codes, no bucket, no Worker, no creds).
   - No user-visible string (window, popover, banner, sheet, page, Terminal output, tooltip) contains an error code (e.g. 10031) or the internals “bucket”, “Worker”, or “creds”/credentials (“Cloudflare” and “token” are product-facing in the wizard's ratified copy and are not banned; the workers.dev hostname inside a link is the link, not the internal noun — match “Worker” case-sensitively as a word).
   - User-facing copy names things with the product vocabulary: your space · an invite · members · your link.
   - Scope: the no-internals check covers said-lines, including the installer's header comment block (ratifiable copy, N11); the installer's inline source comments are exempt (technical but honest, N11).
16. **Numbers** — in the app's sentences, one to nine are words (“seven days”, “two videos are still uploading”, N28); compact facts keep figures (row facts like “5 days left”, counts like “14 videos”, lengths like “4:32”). The price facts are the deliberate two-register pair: numerals in the welcome, words on the landing's tiles (N12).
   - In the app's sentences, a standalone count from one to nine is spelled as a word (“seven days”, “two videos are still uploading”).
   - Compact facts keep figures: row facts (“5 days left”), counts (“14 videos”), lengths (“4:32”).
   - Price facts: numerals in the welcome, words on the landing's tiles (N12) — the deliberate two-register pair.

## The ledger

| Id | Question | Decision | Status | Date |
|---|---|---|---|---|
| P17 | Landing page in v1's definition of done? | **Yes — full page, curl-only.** One install path at launch: the readable script at `shroom.am/install`. brew formula + .dmg are post-v1 additions, absent from the v1 machine. | ratified | 2026-07-20 |
| U1 | Record before cloud setup? | **No — setup-first gate.** `Later` parks the app in `idle-unprovisioned`; Record routes to the wizard. No local-recording or local-library states exist in v1. | ratified | 2026-07-20 |
| U4 | Start at login? | **Pre-checked visible checkbox in the welcome panel.** Applied on panel dismissal (either button), changeable in settings, never registered silently. | ratified | 2026-07-20 |
| U8 | First hostname label (the Worker's name, visible in every link) | **`v`** — every link reads `v.<name>.workers.dev/<id>`; a custom domain (upgrade tier) hides the label entirely. Permanent once links exist. *Amended by N42 for one case: a clashing “v” becomes the label `shroom`.* | ratified | 2026-07-20 |
| U5 | Permission timing | **End of onboarding, same window** — mic inline → Settings switch → guided relaunch → priming warm-up. First Record is pure. Arm-time re-probe survives as a guard, not a step. (Reversed the walkthrough's first-arm rec — that rec was premised on the value-first U1 shape, which U1's decision rejected.) | ratified | 2026-07-20 |
| U2 | The title moment at stop | **On-page focused title, owner-cookie auth.** Stop opens `…/<id>?k=<one-time-key>` → the Worker validates, sets a long-lived owner cookie (space-scoped, likely HttpOnly), 302s to the clean URL — the address bar never holds the key. Owner-mode persists per-browser: opening your own link from Slack months later still edits. Shared links are always clean. *Amended by N40: the cookie names a member, and owner-mode is by role.* | ratified | 2026-07-20 |
| N1 | Clipboard at stop *(new question, leg 4)* | **Never auto-copied.** Copying is explicit — [ Copy link ] on the page, Copy last link in the tray. *Tray door removed by N21 — the page and the Library row are the two doors.* | ratified | 2026-07-20 |
| U9 | Where "fully live" lands | **The page itself is the signal.** The self-refreshing page carries live upload progress while the queue drains; the tray mirrors it. **No macOS notifications in v1** — zero notification-permission surface. *Since N21 the tray's mirror is the config popover's status line.* | ratified | 2026-07-20 |
| U3 | Sender identity on the page | **Anonymous.** Schema carries a sender-name field from day one; no name UI in v1. The mark: a small shroom icon bottom-right, tooltip “made with shroom”, linking home — and in owner-mode it's toggleable **right on the page**, the setting applying space-globally. | ratified | 2026-07-20 |
| U7 | Update posture | **Notify-then-apply.** A tray badge announces the update; the user applies with one click. (Bagrat, against the auto-apply recommendation — consent over convenience.) *The apply row lives in the config popover since N21.* | ratified | 2026-07-20 |
| N2 | Custom-domain timing *(new question, leg 5)* | **Post-v1.0.** v1 is workers.dev-only; the Worker design keeps the attach door open; docs state the upgrade honestly. | ratified | 2026-07-20 |
| U10 | Second-Mac join flow | **Dissolved into the member model (N3):** the second Mac is a member you invite from the first — 30 s, no dashboard. Admin re-paste survives as the recovery door (“Found your space — 14 videos. This Mac is joined.”). *Amended by N33: the second Mac joins through Add another Mac, owner-class — not as a plain member.* | ratified | 2026-07-20 |
| N3 | Member model *(Bagrat reframe, leg 5)* | **Ratified for v1.** The space is born multi-member: the pasted token is infra/admin setup and retires after it; recording always runs on member creds; the owner is silently member #1. Members join with zero Cloudflare anything — the C persona lands in v1. Join code may embed in the install command (zero-paste join; conscious argv/history exception — one-time-use, 7-day TTL). | ratified | 2026-07-20 |
| N4 | Member write path | **Worker-minted temp creds.** One cred shape for all: prefix-scoped ≤7-day S3 temp creds (probed); owner self-mints locally from the admin token, members mint via one authenticated Worker endpoint. **Minting is automatic and invisible for every role** (Bagrat's ratification condition). Bytes always direct to R2 — never proxied. | ratified | 2026-07-20 |
| N5 | In-space authority | **Own videos + admin sees all.** Members see/manage only their own; the admin (whose bucket and bill) sees the whole library, deletes anything, manages members. | ratified | 2026-07-20 |
| U6 | Is the guided persona claimed publicly? | **Claim it, second billing.** Landing leads with the dev story; a “Record together” section states the member story honestly — true without asterisks now (the card cliff exists only for the admin). *Placement amended by N6: the section dissolved into hero point six — the claim stands, promoted.* | ratified | 2026-07-20 |
| N6 | Landing message architecture *(fat-marker leg 1)* | **The hero carries the whole message:** a punchline in shroom's own words (“Screen recordings without the monthly seat. / Pay as you go — sometimes completely free.” — re-ratified 2026-10-13: no competitor's name on the page) + six-point explanation with eyeball-number tiles. Install block above the demo (the command is small and hides below a full-width player; recipient-loop visitors already saw the demo half work). Trust strip = open source · your-storage-your-bill · built-to-outlive-us (telemetry line cut). | ratified | 2026-07-25 |
| N7 | Demo video shape *(fat-marker leg 1)* | **Self-recorded, record-only, single take, no editing.** The controls can't appear in their own take (config/countdown precede capture; the tray click *is* pause; the page opens post-take) — so the demo narrates its own start, shows the being-recorded life + one real task, and ends “I click stop — the page you're watching is what opened.” Setup never appears in the demo; the VM full-loop variant rejected as a stunt. | ratified | 2026-07-25 |
| N8 | Issue-reporting second door *(fat-marker leg 2)* | **Two doors in failure copy: the new-issue deep link + `issues@shroom.am`.** The alias is one Cloudflare Email Routing rule on the domain — zero operated service, works for members without GitHub accounts. The tiny opt-in proxy behind PRINCIPLES' opt-in telemetry stays parked with the post-v1 telemetry track. Scaffold nicety: the issue URL may prefill title/body via query params — everything visible in the URL. | ratified | 2026-07-25 |
| N9 | Update-while-running semantics *(fat-marker leg 2)* | **Quit → swap → relaunch; mid-take refuses.** Running but not mid-take: the script quits shroom, replaces the bundle, relaunches — tray-only, silent per the dock rule; the disk-backed upload queue rides through. Mid-take — recording **or paused** (a paused take is live) — polite refusal, nothing touched: the sacred-recording law reaches into the installer. Never swap under a running process (signature-mismatch kill on next load — verify exact behavior at scaffold). Classification via the app's machine-readable status seam, not process-sniffing. *Refined by N41: a fresh install onto a Mac that remembers its space opens the door; this update lane stays silent.* | ratified | 2026-07-25 |
| N10 | Install progress posture *(fat-marker leg 2)* | **curl's native bar for the app fetch.** The pasted one-liner keeps `-fsSL` — it fetches only the tiny script. The in-script app download shows curl's own progress bar: native terminal idiom, zero custom code in the readable source. Stillness rejected (indistinguishable from a hang on slow links); hand-rolled ticker rejected (cursor-control code in read-it-first source). | ratified | 2026-07-25 |
| N11 | Installer source comments as copy *(fat-marker leg 2)* | **A said-line surface.** “Read it first” makes the source a first-class audience page — the one page a skeptic reads slowly. The header comment block is ratifiable copy (plain-speech what-it-does); inline comments stay technical but honest. One screenful mocked and ratified with this screen. | ratified | 2026-07-25 |
| N12 | Welcome dismissal semantics *(fat-marker leg 3)* | **✕-close ≡ Later, exactly — no special case.** Any dismissal (either button or the close control) registers the login item iff the visible box is still checked; U4 refined. Copy correction: “**set up** your Cloudflare” (was “connect”). “Storage” here vs the landing's “account”, and numerals vs words, stand as deliberate two-register choices. | ratified | 2026-07-25 |
| N13 | Wizard plan panel + `account` screen *(fat-marker leg 4)* | **Plan glyphs, not checkboxes:** → current · ✓ done · • still ahead — checkboxes read as something the user must click. Box 1 reworded: the Google sign-in alternative named in-panel (promoted from docs-only), “Adding it costs nothing”, and the free ceiling made concrete (“up to about six hours of video” — the welcome register). Box 3 gains the example `v.yourname.workers.dev`. **I have one** = filled rightmost default; after “Not yet” opens signup the screen deliberately doesn't morph — self-report by proceeding stands. *Box-1 wording amended by N14 — the card content moved to its own row. The no-morph is reversed by N19 (`account-signing-up`).* | ratified | 2026-07-25 |
| N14 | The card step + token-screen posture *(fat-marker leg 5)* | **The plan grows a fifth row and the gates move into it.** Box 1 ticks at proceed — the plan's one self-reported tick (nothing is probeable pre-token; the paste proves it moments later). New row: “A credit card for storage (their rule, not a charge — free up to about six hours of video)”; box 1 slims to the sign-in gate — the visible card row *is* the pre-warn now. Card step: email check first (`gate-email`), then card presence (`gate-card`, button **Add credit card**); auto-poll spinner (polling dots rejected — read as a carousel); both clear → row ticks. Token screen never auto-opens the browser: read → **[ Create token in Cloudflare ]** (the only trigger, serves every retry) → paste; the wrong-powers line points back at it. | ratified | 2026-07-25 |
| N15 | The `name` step surface *(fat-marker leg 6)* | **The row is the ask; the URL is the field.** No instruction line — typing happens inside `https://v.___.workers.dev/…`; the current row's parenthetical sheds only its example (the live URL replaces it). Explicit **[ Claim this name ]**, Enter-equivalent — permanent names never auto-claim; disabled until the charset check passes; no availability pre-check (taken discovered at claim: `10031` → red field edges, value stays, inline retry). The charset guard filters, never scolds: stray characters never land, a quiet fading line explains; uppercase lowercases silently. **Both endings tick the row with the fact** (“✓ Your link name — v.*name*.workers.dev”) — the plan's only outcome-bearing tick. Action buttons inside nested step content sit flush-left, wizard-wide. | ratified | 2026-07-25 |
| N16 | The `wiring` surface *(fat-marker leg 7)* | **Zero affordances — a spinner and one calm line.** No cancel, no button, no stage narration (the ~6 calls, secret placement, member-#1 enrollment, and propagation all hide behind the line); closing the window mid-wire is safe by the backbone. Offline: the spinner rests, “Everything so far is saved — I'll pick up right here when you're back online.”, auto-resume on reconnect — no retry affordance. The name row shows its N15 outcome throughout. *Offline generalized wizard-wide by N18 (`wizard-offline`).* | ratified | 2026-07-25 |
| N18 | Non-happy paths are states *(fat-marker, map session)* | **Every blocking failure is a first-class state with a scaffold-grade name** — a distinct render plus a distinct set of enabled transitions; easier to build and reason about (Bagrat's call). Roster: `install-net-drop` · `install-checksum-fail` · `install-unsupported-os` · `install-refused-midtake` · `updating` (the not-a-failure lane) · `token-garbled` · `token-wrong-powers` · `gate-email` / `gate-card` (already states) · `name-taken` · `wizard-offline`. **`wizard-offline` generalizes N16's wiring-only offline** — one state for the whole wizard, rendered under whichever row is current, auto-resume on reconnect (re-probe routes by derived state). Nuance: as-you-type guards (the N15 charset filter) stay stateless — no transition happens. | ratified | 2026-07-25 |
| N19 | `account-signing-up` *(fat-marker, map session)* | **The return from the browser is acknowledged, never a dead screen** — reverses N13's deliberate no-morph. After “Not yet” the nest becomes: “Cloudflare's signup is open in your browser — finish there, I'll wait.” + [ Take me there again ] · [ **I signed up** ] (filled). Signup stays self-reported — box 1 ticks at proceed; “Take me there again” reopens the tab, no transition; a relaunch mid-trip re-enters at the virgin fork (nothing probeable before a token exists). | ratified | 2026-07-25 |
| N17 | The `done` screen + the sixth row *(fat-marker legs 8–9; ratified after two Bagrat corrections)* | **The plan grows a SIXTH row and spans the whole onboarding:** “Letting macOS allow recording (microphone and screen — a one-time yes each)” — visible from the wizard's first screen; per-Mac where rows 1–5 are per-space. **`done` = the resting beat** (never auto-fires a permission dialog): five ticks, row six current, its nest **previews the permissions choreography** — “macOS will ask twice…” / “Flip the switch next to shroom there… say yes — I'll be right where you left off.” + [ **Allow recording** ]. “Updates never ask again.” removed (Bagrat); the payoff line retired — the name row already states the link fact. **`perm-preamble` fuses into `done`**; Region 3 opens at `perm-mic`. | ratified | 2026-07-25 |
| N20 | The permissions chapter + `all-set` *(fat-marker leg 10)* | **Chapter copy ratified as drafted** (the mic usage string · perm-screen's flip-the-switch line · priming's double-check line); `relaunching` named as a beat (N18 spirit). **`all-set` = the window's final face — a resting state with three doors** (replaces the passive handoff): all six rows green, closing block below the plan — “You're all set — ready to record.” · “shroom lives in your menu bar now — look for the little mushroom.” · [ Close ] [ Invite someone ] [ **Record now** ] (filled). Record now closes the window and opens `config` at the tray (teaching by gesture, not sentence); Invite someone morphs into the members surface (Region 6 `invite-born`); Close — and ✕, ≡ Close exactly per N12 — is the retreat to tray-only life. “Invite someone”, not “someone else”. | ratified | 2026-07-25 |
| N21 | The tray surface *(fat-marker leg 11 — the tray quartet's idle pair, merged with `config`)* | **Loom-style custom popover, never a native menu — and the tray click is the door itself.** `idle-unprovisioned`: click → the `welcome` window, immediately (the same ratified window, any day later). `idle-ready`: click → `config`, directly — a same-themed popover, pre-filled, one click from Start, footer of three icons with hover tooltips: **Library · Settings** (the gear — the door to the Region-5 inventory) **· Quit shroom**. Both native menus die with their rows: **Copy last link removed** (N1 amended — the page and the Library row are the doors); the wizard's Record-routed context line retires (welcome already says it); quit before setup = right-click → a one-item native menu, Quit shroom (both states). Orphaned tray rows get popover homes at their own legs: U9's upload mirror → a status line above the footer; U7's update row → a banner row; `space-degraded`'s Reconnect your space → the popover's degraded face. Config's Record row is **one source picker** — displays by human name first, Camera below (Camera greys Show yourself). Recording-life click semantics untouched (click = pause); the paused surface follows the same direction at its leg. *The slots “at their own legs” were settled by N24 (paused), N37 (updates) and N38 (the grammar).* | ratified | 2026-08-21 |
| N22 | `countdown` + the bubble's birth *(fat-marker leg 12)* | **Countdown ratified as rendered; cancel by any of three — a click anywhere, Esc, or the quiet line itself.** Bagrat kept click-anywhere (the click-through variant rejected: a cancelled count is one click from retry). The overlay: a light scrim on the chosen display only, a large 3 · 2 · 1, one quiet line beneath — “Click anywhere to cancel.” (the config line, repeated where the decision is made); silent; other displays untouched; the scrim lifts the instant capture starts, so the count is never in the take (N7); no recording flash of ours — macOS's own indicator marks the capture. **The bubble is born at `config`, not at the count** (Bagrat): with Show yourself on, the camera preview appears the moment the popover opens — you see and place yourself before Start; it stays through the count into the take, and the switch hides it live. Consequence: the first-ever Show yourself defaults **off**, and flipping it on is the camera-permission moment — the inline-at-first-pick rule now has two first picks (the switch, or Camera as the source); last-used thereafter. | ratified | 2026-08-21 |
| N23 | `discard-confirm` *(fat-marker leg 13)* | **Ratified as rendered — the paused popover morphs into the question; no window, no dock icon.** “Toss this recording?” · “4:32 recorded so far. There's no undo.” (new second line — the length gives the decision its weight, the no-undo is the truth) · [ Back ] ··· [ Restart instead ] [ Discard ] (red outline, rightmost, never filled). Click-away or Esc ≡ Back; **no Enter default** — a stray Return can never destroy. Restart instead re-arms with the count already running; Discard → `idle-ready`, the bubble goes with it. **What Discard deletes:** the local take *and* any bytes the opportunistic upload already placed in the bucket — a clean delete with no tombstone, since the link was never handed out (the page never opened, nothing copied). Ordering: the `paused` popover — promoted to its own leg by N21 — comes next. | ratified | 2026-08-21 |
| N24 | `paused` + the tray faces *(fat-marker leg 14)* | **Ratified as rendered.** The take's popover (opens only after the click's pause is confirmed — POC §8): header “Paused — 4:32” · [ **Resume** ] (the one filled action) · [ Drop a marker ] [ Restart ] · [ Stop ] with one quiet line beneath, “Your link opens right away.” — the single place stop = published (§5.3) gets said · “Discard…” as red text at the bottom, never a button a stray click finds · a grey “no Quit while a take is open” note (kept). **Three tray faces:** plain (idle) · red dot (recording — no popover, the click *is* the pause) · pause bars (paused). **No timer in the menu bar** — the length lives in the popover header. `display-lost` ratified by reference: the same popover with the context line “Your display disconnected — recording paused.” and Resume greyed until the display is back. Drop a marker lands at the pause point, metadata-only in v1. *Extended by N37/N38: five tray faces (amber, Frost).* | ratified | 2026-08-21 |
| N25 | `recovered-take` *(fat-marker leg 15)* | **Ratified as rendered — a banner at the top of `config`, greeted by the next tray click; launch stays silent.** A **fourth tray face, the amber dot** — something waits for you (no window, no notification: U9 and the dock rule hold). The banner: “We saved your interrupted recording (4:32).” · [ **Get link** ] · “Discard…”; config stays usable beneath it. Get link runs the normal stop sequence from the saved file — the page opens exactly as a deliberate stop would (Region 4). Discard… confirms with N23's question, one word changed — “4:32 was saved. There's no undo.” — and the same clean delete. The banner persists until resolved, across launches; a new take can start meanwhile; two interrupted takes surface oldest first. Named tradeoff: someone who never clicks the mushroom won't learn of it — the take is safe meanwhile. The update badge (U7) and the degraded face may share the amber dot — decided at their legs. *Settled by N37/N38: amber = something needs you (saved take, space, recording off); the update has its own Frost dot.* | ratified | 2026-08-21 |
| N26 | The owner page at stop *(fat-marker leg 16)* | **Ratified as rendered.** The title field is the page's only chrome — focused on arrival, Enter or click-away saves, an untouched placeholder “Untitled — Aug 21” (the day's date) becomes the title so a shared link is never blank; no header, no wordmark, no name (U3). Player: first frame, play, scrub, time, fullscreen. [ **Copy link** ] reads “Copied” for a moment — the page's one copy door (N1); beside it the upload truth, “Uploading — 40%. Your link already works.” with a thin bar → “Fully live ✓”. Download appears only when the single-file MP4 exists (R11). Bottom-right: the owner switch beside the mark, its label “made with shroom” doubling as the tooltip recipients get — flipping it is the space-global setting (U3). Offline line kept by reference. The address bar shows the clean link — the key died at the 302. | ratified | 2026-08-21 |
| N27 | The recipient page, three faces *(fat-marker leg 17)* | **Ratified as rendered, copy verbatim from Region 4.** `live` = the owner page minus the owner's two controls: the title as plain text, the player, Download quiet and second-class once the MP4 exists, the mark with its hover label — no comments, no sign-in, no name. `on-its-way` keeps the title (the untouched placeholder if never typed) and puts the line where the player will be, over a thin bar — the owner's upload seen from the other side; self-refresh. `removed` = the tombstone line alone — no title, no player — the mark stays, the address lives forever. `noindex` on every face; unfurl honesty per R5 unchanged. Closes Region 4. *Extended by N40: a stalled face, an unavailable face, the embed face.* | ratified | 2026-08-21 |
| N28 | `invite-born` — the Members face *(fat-marker leg 18)* | **Ratified as rendered.** The Members face of the window — the gear → Settings → Members, or `all-set`'s Invite someone morphing the same window, the field focused. Head “Members” + one line of N5 truth: “Everyone here records to your space — their videos live in your storage, and you can see and delete any of them.” The list: You (no fact — member #1 shows, silently) · joined members with their date · pending invites. **The composer is the ask** — a name field (“Who's this for?”) + [ Invite ], always present, no Invite-someone button on this face; beneath it “You'll get a link to send them — over any chat.” The typed name is the admin-private label for the member, forever — members never type a name (U3 holds). **The born state appears under the new row:** the link pill `v.<name>.workers.dev/join/<code>` · [ **Copy link** ] (filled — the moment's one action; reads “Copied” briefly per N26; Invite drops to outline meanwhile) · “Send Ani this link — it's good for one person, for seven days.” Row facts: “invited just now” → “invited Aug 21 · 5 days left” → expired: [ New link ] (a row face, not a state — nothing is blocked). “Remove…” = red text on every row but You (N24's pattern), confirming at its own beat, `member-remove-confirm`, with the ≤7-day honesty. Flags resolved with the ratification: the inviter's name on the join page = the Mac account's full name, used silently, no field; small numbers as words; the face has no back-chrome (✕ ≡ Close) — how Members sits beside the Space panel is the settings leg's call. *Settled by N36: Members is the Settings window's third tab. Amended by N29 (the name's short form) and N33 (the You row).* | ratified | 2026-08-21 |
| N29 | `join-landing` *(leg 19)* | **The leg-19 proposal, adopted with all three flags as recommended.** The join link *is* the code — one artifact, three doors (the page, `--join <link>` in the install command, the in-app paste door); both the bare and the `https://` forms accepted. The Terminal hand-hold line stays. Page faces: `join-landing` · an owner-view note when the owner's cookie rides along (“This is the page Ani will see — opening it here doesn't use it up.”) · `join-landing-elsewhere` for phones and non-Macs (“shroom runs on a Mac — open this link on yours to get started.” + Copy link) · `join-expired` (expired, cancelled, or an unknown code — the nameless wording) · `join-used` (with a “You're in” variant for the joiner's own cookie). Opening the page never uses the invite; only Join in the app does. **The inviter's name renders in macOS's short form of the account name** (“Bagrat” for “Bagrat Aznauryan”, locale-aware) — **amends** N28's “full name”, which its own copy never showed. | autonomous-awaiting-ratification | 2026-09-30 |
| N30 | `member-welcome` → `joining` → member `all-set` | **The welcome's twin, already knowing it's an invite.** “Bagrat invited you to record with shroom.” + the N5 truth mirrored for the invitee, once, before anything exists (“…where Bagrat can see them too — nothing to sign up for, nothing to pay.”) + the U4 box + [ **Join** ] [ Later ] (twin's button order). Later remembers the invite (mode 600); the tray click reopens it while it lives. Join morphs into a **two-row plan** — “Joining Bagrat's space (shroom does this part)” → ticked with the fact, then row six verbatim — resting at `done`'s nest (N17). Member `all-set` has two doors, Close · Record now (members don't invite). Own-Mac variants throughout (N33). **Amends** the ratified welcome: one quiet line under its buttons, “Have an invite link?”, opens the classified paste door (a join link → here; a token → the wizard at `token`). *Amended by B159 (Bagrat, 2026-10-15): the “Have an invite link?” line and its field are gone from the welcome; a link pasted anywhere in it still opens its invite.* | autonomous-awaiting-ratification | 2026-09-30 |
| N31 | Join failures as states | **Replaces the merged expired/used row** (N18): `join-waiting` (offline or unanswered — auto-resume, no button) · `join-expired` · `join-used` (both with a paste door for a new link) · `join-space-missing` · `join-already-here` (the link stays unused — pass it on) · `join-switch-confirm` (one space per Mac: an explicit move, blocked while uploads drain or a recovered take waits, with a way-back line for a space's only token-holder). The join failures read the same in-app as on the page. | autonomous-awaiting-ratification | 2026-09-30 |
| N32 | `member-remove-confirm` | **A sheet with three faces, no Return default:** “Remove Mher?” / “Mher won't be able to record to your space anymore — a video already on its way can still finish. Mher's videos stay; you'll find them in your Library.” · “Cancel Ani's invite?” / “The link stops working right away.” [ Keep it ] [ Cancel invite ] · “Remove MacBook Air?” / “That Mac stops recording to your space. Its videos stay.” Removal and deletion are never bundled. The ≤7-day honesty rides “already on its way” — **bound to per-recording minting**; per-member creds would force “for up to a week”. | autonomous-awaiting-ratification | 2026-09-30 |
| N33 | The owner's other Macs *(**amends** U10 and N28's You row)* | **Add another Mac — a quiet action on the You row, owner-class, one Mac, one hour.** A plain member invite would have shown the second Mac only its own videos — a silent P12 regression. Own-Mac links carry owner powers, so they die young (one hour, one-time). The joined Mac is labeled by its own name, silently, as a sub-row under You. Owner-class Macs see all and manage members through the Worker; only a token-holding Mac repairs or re-wires. Every surface of the join machine has an own-Mac variant. **Extends N3's argv exception to own-Mac links, deliberately** — owner powers, but one hour and one-time: a link in shell history is dead long before anyone reads it, and a stolen live one shows up as an unexpected Mac under You. Every owner Mac records as member #1 (Mine spans all your Macs) with its own device key — Remove revokes the device; the You row itself is never removable, and neither is a Mac that keeps the Cloudflare token (a device key can't stop it — Disconnect on that Mac, or delete the token in Cloudflare). | autonomous-awaiting-ratification | 2026-09-30 |
| N34 | Member life + the stranded-take rescue | **Three member-only banners under the amber dot, all derived (they clear by themselves):** `member-revoked` (Start greyed) · `space-needs-owner` (Start live — videos drain later) · `space-gone` (Start greyed). **Save to Movies** is the rescue for any take the space can't take: the single-file copy lands in `~/Movies` and Finder shows it; the take keeps waiting until discarded. The sacred-recording law, carried to its end — no take is ever lost to a space that went away. | autonomous-awaiting-ratification | 2026-09-30 |
| N35 | The Library window | **A list, newest first, tuned to its two jobs — re-share and clean up.** Rows: small poster · title · date · length (· the member's label in *Everyone*); **Copy link always visible**, Open page · Delete… on hover; state words only when not live (silence = live); a recovered take pins to the top under the amber dot. The whole macOS idiom (click-title rename, double-click opens, ⌘C, Delete key, multi-select delete). *Mine · Everyone* for owner-class Macs once others record. Search over titles, local and instant. Delete is a sheet under N23's law carrying the tombstone truth; opens on the cache, refreshes behind. Footer = the count only — P13 stays parked. | autonomous-awaiting-ratification | 2026-09-30 |
| N36 | The Settings window *(resolves N28's open placement)* | **One window, toolbar tabs General · Space · Members** (Members owner-class only). General: the login box (reading the system's truth), the update row, N8's help doors. Space: one tab, a mood per kind of Mac — inspection (the six ticked rows with facts), repair (N42), owner-class-without-token, member — plus the mark switch (owner-class only), the invite paste door, and the exits: **Disconnect this Mac…** / **Leave this space…**, sheets under N23's law, blocked while uploads drain. Row six is live everywhere, with [ Turn it back on ]. *Amended by B159 (2026-10-15): Space shows no invite paste door; a link pasted anywhere in the tab still opens its invite.* | autonomous-awaiting-ratification | 2026-09-30 |
| N37 | Updates *(settles N25's badge question)* | **The Frost dot — its own, lowest tray face; amber stays “something needs you”.** A slim `config` banner, “shroom 1.3.0 is ready.” [ Update ] · What's new → “Updating — back in a few seconds.” → quit/swap/relaunch (N9) → “Updated to 1.3.0 ✓” once. Never mid-take; a failed swap leaves the running version; a bad fingerprint is never installed; too-old macOS gets a line in General, no dot; versions roll up, no nag. The page (Worker) follows the app on token-holding Macs only; its interface stays backwards-compatible. | autonomous-awaiting-ratification | 2026-09-30 |
| N38 | The popover's grammar + the tray faces, final *(settles N21's slots and N25's badge question with N37)* | **Two slots, never more: a banner at the top is an ask, the status line above the footer is a fact.** One banner at a time, highest first — a saved take · the space can't take videos · `recording-off` · an update. Status line: the upload truth, then one-shot lines. Disk-low sits under Start (it's about this take). **Five tray faces:** plain · red dot · pause bars · amber (something needs you) · Frost (update). The arm-time guard made visible — `recording-off` banner before Start; Apple's monthly re-confirm pulled forward to `config`. Return = the filled action (Start, Resume); destructive questions keep no Return default. | autonomous-awaiting-ratification | 2026-09-30 |
| N39 | The auto-pause family, Restart's undo, disk, devices | **Every pause or stop the user didn't make opens its popover by itself once they're back — without taking the keyboard** (the only unasked popovers): `display-lost` (ratified) + `mic-lost` (Resume switches to the built-in mic) + `camera-lost` (camera as source only — a lost bubble never interrupts) + `mac-away` (sleep, lid, lock, user switch) + `permission-lost` (the mic pauses; a lost *screen* grant can't come back mid-take, so it finalizes and asks like a full disk). **The count is Restart's undo** — the old take dies only when the new capture begins. `disk-low` under Start; `disk-full-stop` finalizes and asks, never auto-publishes. Never auto-pick an iPhone, mic or camera; a lost display falls back to the menu-bar one; Camera-as-source previews at `config`. | autonomous-awaiting-ratification | 2026-09-30 |
| N40 | The stop's edges *(**amends** Region 4 beat 6)* | **An offline Mac can't load the page — so `stopped-offline` puts the link in the popover** (“Your link is ready.” + Copy link) and the page opens by itself on reconnect (after any running take). **`stopped-stranded`** when the space can't take the video — no page, Save to Movies. A space that serves but can't take uploads still opens the page, its upload line saying so (“Waiting for your space — your link already works.”). Owner-mode by role: the cookie names a member; members' pages give the title only, owner-class gets the switch everywhere. Recipients get an *on its way, stalled* face when the upload has made no progress for a while. | autonomous-awaiting-ratification | 2026-09-30 |
| N41 | Launch & presence laws *(**amends** Region 1's “every launch is silent”)* | **The quiet-take law** — nothing new on screen while a take is open (invites, deep links, windows, deferred pages, banners all wait). **A launch you did is a tray click** (Spotlight, Finder, Dock, a fresh install onto a Mac that still remembers its space — the `updating` lane stays silent per N9, except that an install command carrying `--join` opens the invite, because the user asked); launches shroom does itself stay silent — though an update relaunch restores whatever window was open — also the rescue for a mushroom hidden behind the notch; the popover falls back to the top-right when the mushroom can't be seen. **`quit-confirm`** when uploads are draining (no red, no Return default); shutdown never asks. Uploads hold off idle sleep, never lid sleep. | autonomous-awaiting-ratification | 2026-09-30 |
| N42 | Wizard edges + the repair mood | **`token-pick-account`** when a token reaches several accounts · the **“v” clash** falls back to the label `shroom`, said once in the name row (**amends** U8 for that case) · **Reopen shroom** under `perm-screen` for a declined relaunch · row six ticks with “microphone off for now” · **`space-found`** gives U10's recovery line its place · **`space-degraded` = one banner → the Settings Space tab in the repair mood**, the first broken row current with Region 2's own state (`token` · `gate-card` · re-`wiring` · `space-storage-gone` · `name-changed`); the token line redrafted (“Cloudflare stopped accepting shroom's token…”). Start stays live while degraded. | autonomous-awaiting-ratification | 2026-09-30 |
| N43 | The permissions chapter: one surface at a time, and the camera at setup | **Camera is asked at setup, in the chapter, not at the first camera pick** (Bagrat, 2026-10-14): `perm-mic` → **`perm-camera`** (macOS's own camera prompt, answered inline; skipped on a Mac with no camera) → `perm-screen`. **`perm-screen` shows one surface at a time:** shroom asks macOS, which raises its own screen-recording alert only on the first ask (probe, macOS 26.6: `CGRequestScreenCaptureAccess` alerts once, its Open System Settings opens the exact pane, and only that ask puts shroom in the pane's list). While macOS's alert is up shroom opens nothing and says so — “macOS is asking about screen recording now — choose Open System Settings in its window…” with a quiet “Closed macOS's window?” [ Open System Settings ]. When macOS doesn't ask (every later ask, Turn it back on), shroom opens the pane itself and the ratified line holds. **Amends** N17's `done` preview (three asks) and row six's “(microphone, camera and screen — a one-time yes each)”, and N22's inline-at-first-pick rule (the first pick asks only when setup didn't). **Show yourself starts on** (Bagrat, 2026-10-14: on by default): the first popover shows the bubble on the camera shroom picks, unless the camera was denied in the chapter, which turns it off — **amends** N22's “first-ever default off”. Bagrat approved perm-screen's new lines as drawn on the canvas. | ratified | 2026-10-14 |
| N44 | System default, for the microphone and the camera | **Both pickers offer “System default” first, and it's what a new Mac starts on** (Bagrat, 2026-10-14: the pickers account for the default devices too, so the input follows macOS when, say, AirPods join). It follows what macOS has as the default input (the microphone) and its preferred camera, shown in grey beneath — AirPods joining moves it. N39 holds: an iPhone over Continuity is never taken by itself, so a default that is one falls back to the built-in device. A device picked by name stays picked. **A take keeps the device it started with**: a default that changes mid-take is followed from the next take (B149). | autonomous-awaiting-ratification | 2026-10-14 |
| N45 | The setup window after the first stranger run: the email first, the card watched, the token box still, and the permissions before the wiring | **Bagrat, on the canvas (2026-10-15: all four boards approved, to be built), after CHECKPOINT §10's first two-VM run.** (1) Cloudflare makes no token until the account's email is confirmed, and shroom can't see that before it has one: the signup trip says to confirm the email first, and the token step reached through “I signed up” carries one quiet reminder under the field until a paste lands. (2) The card gate keeps its line and Add credit card; one quiet line under the button says where it goes and that shroom is watching — another once the button was clicked. (3) While a paste is checked the field goes still: disabled, dimmed, no caret; any answer but a pass gives it back live and empty. (4) **The two last rows swap**: “Letting macOS allow recording” is drawn fifth and “Wiring it up” last. Wiring starts the moment the link name ticks and runs out of sight under the permissions chapter (its row's dot a small spinner, its aside “shroom is doing this part now”); the chapter's `done` follows the name row at once. Permissions finished first: the last step is `wiring`, with “Almost there — switching your name on takes a few minutes the first time, and only once.”; wiring finished first: the chapter's end is `all-set`. A new name still switching on is this wait, never `wizard-offline` (DEVIATIONS #54). Rows keep their numbers in the machine (row five is wiring, row six the permissions); only the drawn order changes. A space found is joined first, as before (its line under the wiring row, then `done`). Settings · Space keeps its order (its microphone door hangs under the last row). | ratified | 2026-10-15 |
| N46 | Copy thumbnail: a link that looks embedded where links don't embed | **A second copy beside Copy link, on the owner page and in the Library row's menu, reading “Copy thumbnail”** (Bagrat, 2026-10-15, after the boards “Copy as a thumbnail — designs”, “B kept current” and “B + loop, kept current”: he chose B, kept current, with the loop, in both places, as a Copy thumbnail that also acts as the link, and said to build it). It puts the video's picture on the clipboard wrapped in the link, and the plain link as text beside it, so a paste into Basecamp or an email shows a picture that opens the page, and a paste where only text goes is the link. **The picture is B + loop, kept current:** a 3 s silent loop from the start of the video with the title on a soft band at its foot and the length in a corner; before the loop can be made, the same as a still. Untitled, it has no band. A new picture is made for each title by whoever sets it, after the title saves; an earlier paste keeps the picture it was given. A video from before this gets its picture on its first copy. Copying stays explicit (N1); nothing on the picture names the sender (U3). **The viewer's page too** (Bagrat, 2026-10-15, asked whether the recipient's live face shows Copy link: yes, both buttons, with Download there too): `live` shows Copy link and Copy thumbnail beside Download (still only once the single-file MP4 exists). A viewer stores nothing: their Copy thumbnail wraps the newest picture made for the current title, or, when none exists yet, the video's first frame. **Copy thumbnail waits until its picture is ready** (Bagrat, 2026-10-15: after trying 0.1.15 he asked for the button to stay disabled until its picture is ready, then approved the board “Copy thumbnail waits until the moving picture is ready” and its wording, asking for a gap between each tooltip and its button). On the owner page the button is greyed and doesn't copy until the loop for the current title is stored; hovering or focusing it says “Preparing the thumbnail…”. A new title greys it again until its loop is stored. When no loop can be made (the upload finished with no teaser, or drawing it failed), it wakes with the still. On the viewer's page it is hidden until a picture made for the current title exists (it no longer falls back to the first frame). The Library row's menu keeps it enabled: an older video's picture is long made. Every tooltip on the page (Copy link, Copy thumbnail, Download, the preparing line) sits a clear gap below its button. | ratified | 2026-10-15 |
| N47 | The Mac's own sound in a take | **A take of a display records the Mac's sound with the voice, on by default, with no switch and no setting** (Bagrat, 2026-10-16: on by default, with no setting and no UI, to keep the work minimal). Whatever the Mac plays (a video, a call, an app's sounds) is mixed with the microphone into the take's one sound track, in step with the picture. shroom's own sounds are never in it. With the microphone off, or after it is lost and the take resumes without it, the Mac's sound is still recorded. No screen, popover or control changes; one line does: mic-lost's “Resume continues without sound.” becomes “Resume continues without your microphone.”, as Bagrat chose the same day. A take of the camera alone records the microphone only; with the microphone off, the Mac's sound is kept (Bagrat, 2026-10-16, answering both). How it is captured and mixed: DECISIONS B172. | ratified | 2026-10-16 |
| N48 | The storage figures and the one contact address | **The storage figures count what a recording keeps, and shroom has one address, `hi@shroom.am`** (Bagrat, 2026-10-16: correct the copy now, optimise storage later; one contact address everywhere). A recording keeps its stream and its MP4 copy, about 3.2 GB an hour at Normal, so the free 10 GB holds about three hours and $1.50 a month (100 GB at $0.015/GB-month) about thirty. The landing's tiles read “$0 /month — about three hours of video” and “$1.50 /month — about thirty hours”; the welcome reads “free for about three hours of video, and pennies after that — about 30 hours for $1.50 a month.”; the card row reads “free up to about three hours of video”. Every line that named `issues@shroom.am` (the installer's header and failure lines, the app's “Something off?” line and its mail door) names `hi@shroom.am`. | ratified | 2026-10-16 |
| N49 | Install via Homebrew | **shroom installs via Homebrew in v1, beside the install line** (Bagrat, 2026-10-16: a required item). `brew install --cask bagrat/shroom/shroom`, from shroom's own tap: Homebrew's own cask list takes only apps Apple notarized, and the release is self-signed (B125). The cask does what the install line does: the release's zip, checked against its fingerprint; placed in `~/Applications`; taken only from the release's signer, and opened by macOS without asking because nothing marks it as downloaded. The app keeps itself up to date, so Homebrew leaves updates to it. No screen, line or control of the app changes. Showing it on the landing page would be a design change, put on the canvas first. How it is built and kept current: DECISIONS B175. | ratified | 2026-10-16 |
| N50 | A join link macOS hands to shroom | **A join link macOS hands to shroom as it opens is an arrival, like the page's Open shroom** (Bagrat, 2026-10-16). `open -a shroom "https://<space>/join/<code>"` (the Homebrew join line) makes macOS give shroom the link with the open; shroom takes a web address only when it reads as a join link and ignores any other. It lands where Open shroom's `shroom://join/…` lands: kept in the owner-only invite file, then the invite window (member-welcome, or join-already-here / join-switch-confirm) — never a silent join — and it waits while a take is open (Law 4). shroom claims no web addresses, so no website can send it one, and the link is never on the app's command line. Built and proved: DECISIONS B176. | ratified | 2026-10-16 |
| N51 | The invite page offers the Homebrew join line | **The invite page shows the Homebrew join line under the install command** (Bagrat, 2026-10-16: the board approved, placement and words as drawn). “Use Homebrew? This does the same:” then `brew install --cask bagrat/shroom/shroom && open -a ~/Applications/shroom.app "https://<space>/join/<code>"` with its own Copy, quieter than the install command's. It goes live only with the app that takes the link (N50, 0.1.17). Built: DECISIONS B178. | ratified | 2026-10-16 |
| N52 | shroom.am puts Homebrew first | **The landing's install block offers Homebrew first and the install line under it**, as the README does (Bagrat, 2026-10-16: build it, he looks at the screenshot after). “With Homebrew:” `brew install --cask bagrat/shroom/shroom && open ~/Applications/shroom.app` with the filled Copy; “Or without it:” the install line with an outlined one, then its Read it first; the agent door below as before. The two lead-ins are placeholders for his words. Built: DECISIONS B179. | provisional | 2026-10-16 |
| N53 | Report a problem and feedback, through shroom.am | **Bagrat asked for it, 2026-10-16; the same day he chose a report endpoint on the shroom.am site Worker over a mail draft, and approved the plan: the endpoint and the page first, the app's button for 0.1.18.** **One endpoint, `POST shroom.am/report`**, takes a report (kind problem or feedback, the words, optional answers, an optional reply-to, the versions, the setup timeline, and up to 200 cleaned log lines), checks its size, shape and rate (and Turnstile from the web form), and emails it through the `send_email` binding to one verified inbox whose address is a Worker secret. It stores nothing. **Two doors:** shroom.am/feedback (the testers' questions, an optional log pasted or chosen and cleaned in the browser), and in the app Settings · General's “Something off?” row with **Copy diagnostics** and **Report a problem…**, a sheet with “See exactly what's sent” and **Send** (a mail draft to hi@shroom.am only as the fallback when sending fails). Cleaning: home paths, the space's host, other web addresses, emails and long ids replaced, the same rules in the app, the page and the Worker. **The exception to “never a SaaS we operate”** (a receiving service of ours, storing nothing) is ruled by Bagrat, 2026-10-16; the privacy page's words change with his OK. Proposal: dev/proposals/report-a-problem.md; DECISIONS B180. The said-lines, the `report-problem` states and the site surfaces land with the build. | provisional | 2026-10-16 |
| N54 | ⌥⌘P pauses a take and resumes it | **One global shortcut, ⌥⌘P, held only while a take is open** (Bagrat asked for it, 2026-10-16, to pause between beats of a take; the same day he chose ⌥⌘P, fixed rather than settable for now, and pause and resume only, no Stop shortcut). While recording it is the tray click: an instant pause, the paused popover opening once the pause is confirmed. While paused it is Resume: the popover closes and capture resumes, with no count. It does nothing while a question is open (Discard…, Restart) or while Resume is greyed. Registered with the system's hot-key service, which needs no new permission (no Accessibility, no Input Monitoring), and let go the moment the take closes, so ⌥⌘P belongs to other apps the rest of the time. Nothing on screen names it in v1. Built: DECISIONS B182. | provisional | 2026-10-16 |
| N55 | shroom.am's header offers Send feedback, and the footer's Trouble, questions opens the feedback page | **Bagrat asked for both, 2026-10-17, and chose the label “Send feedback” from “Send feedback”, “Got feedback?” and “Feedback”.** Every shroom.am page's header carries a second link, **Send feedback**, beside GitHub and in its style, opening shroom.am/feedback (N53's page). The footer keeps its words, “shroom · Trouble, questions”, and its link now opens shroom.am/feedback instead of a new GitHub issue. The installer's header block and its lines keep the issue address (N11 and N8 are unchanged). Canvas board: “shroom.am — Send feedback in the header, and the footer's Trouble, questions opens shroom.am/feedback”. Built: DECISIONS B183. | ratified | 2026-10-17 |
| N56 | The display you click the mushroom on is the one you record | **Bagrat found it with an external display, 2026-10-17** (“Displays have separate Spaces” on, so every display has its own menu bar and mushroom): he clicked the mushroom on the MacBook's display, and the bubble and the count came up on the external one. **A click on the mushroom opens `config` with the display it was clicked on as the source** — the popover already hangs there, and now the bubble, the count and the take go there too. It wins over a remembered display (Bagrat, 2026-10-17: the click wins). **A hand pick holds:** a display picked in the Record picker stays the source, whichever display the popover is reopened from, until that take ends (Bagrat, 2026-10-17). **Camera only stays Camera only** — a source you chose — but its preview, and with it the count, moves to the clicked display (Bagrat, 2026-10-17). It is the rule for every click; with one display (or a mirror set) nothing changes. A launch you did is a click (N41) and takes the display it came from too; a popover that opens by itself (a pause you didn't make, an update relaunch) and Record now from `all-set` keep last-used, else the main display. Mid-take a click is still the pause and moves nothing. Built: DECISIONS B185. | ratified | 2026-10-17 |
| N57 | The popover and the windows it opens stay on the mushroom's display | **Bagrat found both on a MacBook beside an external display, 2026-10-17.** (1) After an update relaunch, the popover shroom opened by itself sat at the MacBook screen's top-left corner, not under the mushroom: the just-made mushroom was read before it was laid out, at its screen's very left edge, and was taken as at rest. **A popover hangs under the mushroom, on the mushroom's display, however it opened** (a click, a launch you did, an update relaunch, a pause or stop you didn't make); only a mushroom that can't be seen sends it to the corner (N41, B161). A mushroom read at its screen's left edge isn't laid out yet: a status item never rests there, since the left of a menu bar is the Apple menu's and the app's. The opening waits for it as for any mushroom still on its way in. (2) Settings and the Library, opened from the popover on the MacBook, came up on the external display. **A window opens on the mushroom's display**, centred there — the display the popover hangs on — unless it is already on that display, where it stays where the person left it. That covers the setup window, the Library and Settings. Built: DECISIONS B186. Ratified by Bagrat, 2026-10-17. | ratified | 2026-10-17 |
| N58 | The demo plays on the landing, second, with a short caption | **Bagrat confirmed the design on the canvas, 2026-10-17** (the board “shroom.am — the demo, B1b at position 2”). (1) **The demo is the page's second block**: after the points, the price tiles and the team line, and right before Install, so the install line sits under the demo. (2) **It plays in place.** The block shows a still of the take (the real setup window from 7:00 and the camera bubble from 0:16, mirrored, cut from the video onto the landing's own ground) with a play button and the length. The video itself loads quietly behind the still as the block comes near the screen; a click fades the still and the video plays there. The landing never sends the visitor to the player page. (3) **The caption is “Recorded with shroom.”**, with no link and no address shown. Built: DECISIONS B190. Ratified by Bagrat, 2026-10-17. | ratified | 2026-10-17 |
| N59 | shroom.am, redesigned: two columns, the price as one bar, install in tabs | **Bagrat approved the canvas board “shroom.am redesign — B8-box”, 2026-10-17**, after eight rounds from his ask for a page with less text that shows the demo and install without scrolling. (1) **Two columns on a laptop.** Left: the shroom mark and wordmark, large, at the top; then the headline “Record. Share. Pay-as-you-go.” on one line with its line under it; then one thin price bar, 0 to 30 hours to scale, as wide as the headline, its first tenth lit, labelled “$0 · about three hours of video” and “$1.50 /month · about thirty hours” (it replaces the two price tiles, which read as two plans); then seven terms with green ticks, in his order: “Your videos live in your own Cloudflare account”, “You pay only Cloudflare, for the storage”, “Invite your team with a link”, “No price per person — the bill counts video, not people”, “Delete old videos and pay less”, “About ten minutes of guided setup, once”, “Open source — every line on GitHub”. Spacing: brand to headline 70 px, headline's line to the bar 48, bar to terms 48. Right: one box, corners 10 px, its top level with the wordmark's baseline and its bottom with the last term's: the demo (N58), then Install. (2) **Install is one command at a time, in tabs: Terminal (chosen), Homebrew, Coding agent**, amending N52's Homebrew-first stack; under each, “Read it first” alone, and in the agent's tab “Your agent asks you before it runs anything.”; the system-requirements line under all three. (3) **No header and no trust strip.** The page's foot, under the left column at the bottom of the first screen (after the content when it runs longer), holds Send feedback · GitHub · Questions and limits · Privacy · Terms, then the visits line; “Trouble, questions” goes (it opened /feedback, as Send feedback does), and so do “Your storage, your bill” (term 2 says it) and “Built to outlive us”. The other shroom.am pages keep their header and footer. **The demo's caption, “Recorded with shroom.”, goes** (his later note, the same day), with no link to the take in its place: the video is to be replaced by a generated one; the still and play-in-place stay. (4) **On a phone** it stacks, and by his phone notes the same day the terms come before the demo and install goes last: the brand, the headline, the bar, the terms, the box (the demo, then install), the foot, its links wrapping to fit. The bar's labels are short there, “$0 / 3h video” and “$1.50 / 30h”, because the long ones wrapped; the laptop keeps the long ones. Built: DECISIONS B191. | ratified | 2026-10-17 |
| N60 | Send feedback from the popover, in its own window, the log going along | **Bagrat asked for it, 2026-10-17**: feedback is worth most from people who have installed shroom, it is easier to send from inside the app than from the web form, and the log should go without anyone attaching it by hand (it makes N53's in-app door easier to reach). **The same day he chose design A** of three on the canvas (“Send feedback — A”) and **the log box ticked by default**. **The popover's header gains a fourth icon, first: Send feedback · Library · Settings · Quit**, named “Send feedback” on hover and to VoiceOver; its glyph a waving hand, his pick from 28 on the canvas (“Send feedback — A: the icon” boards, #23), 2026-10-17. It opens **its own small window, Send feedback**: one line on where it goes, the words, the optional email for an answer, **Include shroom’s recent log** (ticked) with a line on what goes and what never goes, and N53's “See exactly what’s sent”. Unticked, the report carries the words and the version only. **Send** posts to N53's `shroom.am/report` as kind feedback, from the app; Sending… and a send that didn't go through are N53's, word for word, mail link included. **Sent**, the window says so and offers Done. Settings · General's Report a problem… stays as it is (a problem, the log always). The icon sits in the ready popover only, so it never shows mid-take (law 4). Bagrat OK'd the window's words the same day (“its recent log” for “its last 200 lines” among them). Built: DECISIONS B193. Ratified by Bagrat, 2026-10-17. | ratified | 2026-10-17 |
| N61 | The landing's video becomes “How to set up and use shroom”, served by shroom.am | **Bagrat approved the new video and asked for it to go up, 2026-10-18.** (1) **The video in the landing's box is now “How to set up and use shroom”** (four minutes): a walkthrough drawn from the design boards, with his own narration, replacing the 15-minute take N58 placed there. It is not a shroom recording, so nothing on the page or in the README says it was recorded with shroom. (2) **Its still** is a frame of the video itself: the setup window at “You're all set — ready to record.”, on the landing's dark ground, the same size as before. The play button reads “Play how to set up and use shroom, 4 minutes” to VoiceOver; the length shown is “4 mins”, in minutes rather than to the second (his note the same day). (3) **shroom.am serves it** as its own file (`/how-to.mp4`), no longer from the maker's shroom space; it still loads only as its block nears the screen, and plays in place on a click. (4) **The word “demo” leaves the page** where it meant this video. (5) **While it plays, its controls hide** when the pointer is idle for a few seconds or leaves the video, and come back on a move, a tap or the keyboard (his ask, the same day): the browser's own controls, which do this in Chromium and WebKit and keep keyboard and VoiceOver access. The layout of N59 is unchanged. Built: DECISIONS B194. Ratified by Bagrat, 2026-10-18. | ratified | 2026-10-18 |
| N65 | Voice cuts: say “shroom cut”, say the line again, and the slip is cut | **Bagrat asked for it, 2026-10-18** (relayed by the orchestrator), and tried it with his own voice the same day: the cut worked. (1) **The cue.** A speaker who slips says “shroom cut” on its own and says the line again. When the space listens (N66: the space's own Cloudflare account hears each take as it uploads), the Mac that made the take reads the take's transcript once it is whole and finds each cue. “shroom” is matched by sound, so the way it was heard doesn't matter, and it must be followed by “cut”. A mention (“just say shroom cut”, “the shroom cut feature”) is left alone; a looser hearing (“Troom cut”) counts only when it stands as a sentence of its own. (2) **The cut.** The words after the cue are lined up against the words before it to find where the line was started again; the cut runs from that restart through the cue, its edges set in the quiet on either side, found in the take on the Mac. A retake that says enough of the slip again is cut at once. A cue that breaks off a sentence (it stops without a full stop, or on a word no sentence ends on: “the”, “and”, “to”, “will”, or “is” after a noun, as in “…and your link is”) takes the sentence with it, back to its start, when it began within 20 s; the finished sentence before it stays, even when the speaker ran straight on from it. A line said again in other words is weighed by meaning (Bagrat, 2026-10-18, relayed by the orchestrator: the mechanism deterministic, the judgment above it): the Mac lists the places the redone stretch might begin, within 20 s before the cue: the starts of up to three sentences, and the pauses of 0.4 s or more inside the cue's own sentence that leave at least three of its words before them (a finished sentence before it is redone whole or not at all); the space's own Cloudflare account (Clef) scores whether what was said after the cue says again everything from each; the cut runs from the farthest start scored 0.4 or more, and only ever reaches farther back than the words alone would. When it can't answer (the free day used up, a busy model), the words alone decide. When nothing was said again after a cue, how far back the slip went isn't clear: the line before the cue is cut with it, and the strip asks the owner to check (Bagrat, 2026-10-18, after a take of his own on the early build: cut the likely slip at once instead of offering it). (3) **Where it lands.** The cuts go into the record's edit list (N63) once per take, so the share page, the embed and the download follow them, as do the captions and chapters made from the same transcript (N66). The words cut stay out of what viewers get. Only a take stopped while the space listens is cut; nothing already shared changes. (4) **No switch of its own.** The space's listening (N66) is the one switch: the cue is a deliberate command, and a take without it is untouched. (5) **The owner page shows them** (`owner-page-cuts`), under the player, in owner-mode only: each cut's time and what was said, struck through, between a few words of what plays around it, with Put back; the line cut before a cue with nothing said again after it shows “shroom cut” struck as well, under a note asking the owner to check it, and its Put back brings the line back while “shroom cut” stays out (both his picks on the board, 2026-10-18). A press saves at once; one that doesn't save puts the row back and says so. They are reviewed in both places (Bagrat, 2026-10-18, relayed by the orchestrator: “Both”): the strip on the owner page for a quick check, and the editor's timeline (N63) for fine-tuning, the cuts as red parts, a cut put back dashed, to be cut again; both read and write the one edit list, so a Put back in either shows in the other. Viewers see only the cut video. (6) **The hint** (`owner-page-cue-hint`; Bagrat OK'd its line 2026-10-18, relayed by the orchestrator): in a space that listens, the owner page of a take with no voice cuts says how, under the player, until Got it or a first take with voice cuts. (7) **Finishing** (`uploading`, `hearing`; Bagrat, 2026-10-18 (relayed by the orchestrator): one indicator on the owner page, from the upload until the voice cuts land, “Finishing” his pick of its first word): the owner page's upload line becomes one line from go-live, “Finishing your video — the link already works, share it anytime.”, a small spinner turning in front of it and no bar (Bagrat, 2026-10-18 (relayed by the orchestrator): a small spinner in front of the words, no bar), resting beside a stalled upload's line; in a space that listens it stays after the upload, its spinner turning, while the take is heard, five minutes at most, and it fades when the cuts land (the strip takes its place), when there are none (the hint, while it's due), or when the five minutes pass. It never speaks of cuts. No “Ready ✓” after it (B142's, retired on the page) and no second line. The words are proposals until he OKs them. Built: DECISIONS B200, B201, B202, B203. | provisional | 2026-10-18 |
| N62 | shroom on the command line, for a person or their coding agent | **Proposed 2026-10-18 (dev/proposals/cli-agent-skill.md). Bagrat's calls the same day, relayed by the orchestrator:** an agent may start and stop recordings only once a Settings switch is on, off by default; a refused recording says so, says that switch turns it on, and gives a link that opens Settings right at it; the command may make invites; the command is always installed; it does things many at once, so an agent can invite a team or tidy many videos in one step; and it is built in full now, its release his to time. (1) **The `shroom` command** ships inside the app (`Contents/MacOS/shroom-cli`) and follows the app it ships in: that app's folder (a dev build's command reads `~/.shroom-dev`) and that app to open. The install line links it into `~/.local/bin` (and puts that folder on the login profile's PATH when it isn't there), the cask into Homebrew's bin; `--uninstall` takes both back. (2) **It works whether the app runs or not:** list, info, links, open, rename, delete, cleanup candidates, members, invites — through the space's own page under this Mac's device key, never the Cloudflare token. A delete, a member's removal or a batch invite changes nothing without `--yes`; a rename, delete, invite or removal waits while a take is open (law 4). (3) **Many at once:** a batch takes its items as arguments or from a file and answers each with its own status; it is safe to run again (nothing is done twice), and says so when only some went through. At most 20 invites or 100 items a call. Each person invited gets their own link: shroom sends nothing. (4) **Cleanup candidates** are the page's own suggestions where the page makes them, and otherwise the videos older than a number of days. (5) **Recording goes through the running app** (its control socket in shroom's folder, this user only): start is the popover's Start, countdown and permission beat included, the popover open or not; pause is the tray's; Resume and Stop are the paused popover's; a stop answers with its link (law 3: this stop made it; law 10: nothing is copied). The app answers with a code, and the command says the words. (6) **Settings → General gains “Your coding agent”**: the prompt to paste, Copy (“Copied”, law 10), Read it first (shroom.am/cli), the line under them, and the switch “Let it start and stop recordings”, off by default; while it's off the app refuses every recording ask. **The refusal is three lines:** recording from a coding agent is off; the switch in Settings, under General, turns it on; and the link `shroom://settings/agent` (with `--json`, also its own field, `settings_link`; exit 10 as before). Bagrat approved its words and the switch's label as drawn, 2026-10-18. Clicked, the link opens shroom if it isn't running, and Settings on General with the switch's row in view and lit for a moment; the switch stays as it was — only the person flips it. Mid-take it opens nothing (law 4); a sheet open in Settings stays, and Settings comes forward as it stands. Settings needs a space (B160), so a Mac without one is told to set shroom up first, before the switch comes up. (7) **shroom.am/cli** is the guide an agent reads (a skill's SKILL.md), and `shroom guide` prints the same; /agent points there. (8) **Without the count, and the label** (Bagrat, 2026-10-18): `record start --no-countdown` starts the take at once, capture beginning as the count's end would; the app's own Start keeps its count. A take the command starts shows a small label at the top right of the display it records — “Your coding agent is recording your screen” — while it records (paused, it goes; a take of the camera only has none), left out of the video and taking no clicks; `--no-label` leaves it off. The tray shows every take, whatever the flags. Bagrat approved the label's board as drawn — its look, its words and the label on by default — 2026-10-18. Built: DECISIONS B215–B219. Ratified by Bagrat, 2026-10-18: the Settings card's words as built (he'll tweak them once he uses it), the command's words and the guide as built. | ratified | 2026-10-18 |
| N68 | The page counts plays in the owner's own account, and the Library suggests videos to clean up | **Bagrat asked for it, 2026-10-18**: shroom is pay-as-you-go, so it should say which videos are worth deleting, from how old they are, how often and how far they've been watched and when last. **The same day he chose**: plays counted **on by default**, with the switch in Settings; **plays and unique-ish viewers** both counted; the score is **fading and cost together**; deleting from the suggestions is **one tap with Undo**; and the rest to the build's judgment (B210–B214). (1) **The page counts plays.** A viewer's page and the embed send one beacon at the first play and one each time the tab hides, saying how much was watched; the Worker keeps per-video counts in the space's own bucket — plays, unique-ish viewers (a day-only hash under a salt replaced daily), how far each was watched in tenths, plays per day, last played. No cookie, nothing stored on the viewer's device, nothing kept that says who watched. The maker's own page never counts. A video that isn't live gets no record; a deleted one's record goes with its seal. (2) **Settings → General gains a group, Your videos**: **Count plays** (on; the space's switch, owner-class only, as the mark's is), **Suggest videos to clean up** (on; this Mac's), and **Space used** (the space's size, and what it costs past Cloudflare's free 10 GB). (3) **A video is suggested** when counting has watched it at least 30 days, it is at least 30 days old and nobody has played it in 30, its plays have faded (each counts half as much every 30 days, weighted by how far it was watched: 0.3 + 0.7 × its median share) below a quarter of one fresh full watch, and it takes at least a cent a month to keep at list price (about 0.67 GB). Never one marked Keep. The costliest per remaining play comes first. (4) **The Library says so in one line** above the rows, with Review… and ✕ (not now, until the list changes). (5) **Suggested to clean up** is the Library's own view: what deleting all of them frees and saves, then each video with when it was made, its size and why ("Watched 4 times, the last 10 months ago"), with Open page, Keep and Delete. **Delete is one tap**: the row goes and an Undo line stays a few seconds before the delete is sent; Delete all… asks once with the several-videos sheet. Keep takes a video off the list for good (the record's `keep`). (6) **The money line** gives sizes always, and cents only past the free 10 GB; inside it, the saving reads as nothing yet, and a clean-up that brings the space back inside it says so. (7) **shroom.am/privacy's line on viewers changes**; its new words, from the board "shroom.am/privacy — the viewers' line once plays are counted", Bagrat approved on 2026-10-18 with the sentence on the one-day viewer stand-in, and the page carries them (site/pages/privacy.html). (8) **Each suggested row says what it costs a month**, only when the space is past the free 10 GB and never on a member's Mac (Bagrat, 2026-10-18): its size at the list price (B212). Built: DECISIONS B210–B214. **Bagrat ratified it as built, 2026-10-18**: its words and rules, and B211's 1¢ size gate. | ratified | 2026-10-18 |
| N66 | The space names its videos, adds chapters and writes captions, through the owner's own Cloudflare | **B171's plan, built** (Bagrat, 2026-10-18, relayed by the orchestrator: build it now, decide the release later; the build's calls are made on judgment and listed in DECISIONS B205–B208 for him to overturn). (1) **Off until the owner turns it on, on the Mac that holds the setup key, and visible** (Bagrat, 2026-10-18: off by default, not to spend the AI allowance, but very visible; one switch for every AI feature): a banner in the popover, like the update's, says what shroom can do — name videos, add chapters, cut slips — and Turn on… opens a short confirm that says what it costs, once; the switch in Settings · Space, “AI for new videos”, is its home, and covers voice cuts (N65) too. Not now hides the banner for good on that Mac (Bagrat, 2026-10-18: from then on only the switch turns AI on). Turning it on re-wires the space's page with Cloudflare's AI and a schedule that runs each minute; the owner's key needs no new power (B171's live check 2). Turning it off takes them away; names, chapters and captions already made stay. Only new videos (Bagrat, 2026-10-18: no offer to name earlier ones). Members' Macs never show it and never see a cost. (2) **Each take is heard as it uploads**: every Mac tells the space when another minute of a take is up and once more after Stop, and the space hears that minute and keeps its words beside the take, on the recording's own clock. (3) **The title comes early, the rest after Stop** (Bagrat, 2026-10-18: the title must be there when the page opens): once the first minute is heard while the take records, a title from it alone, so the page opens named; after Stop the space names the take again from all of it and places its chapters, from the words that play: the title while nobody has typed one (a typed title always wins and is never written over), chapters only on a video of 90 s or more with two or more, and captions from the same words, never a second hearing. (4) **The free day**: every Cloudflare account gets 10,000 neurons a day (about 2½ hours of new video; Bagrat, 2026-10-18); when they run out, the rest waits and is finished after 00:00 UTC by the space's schedule, with no error anywhere; on a paid account it simply runs (about 4¢ per video hour). (5) **Edits never re-hear** (the editor, N63, when it lands): chapters and captions follow the saved edit on every view; the name and chapters are made again only when an edit removes at least a fifth of what was said or a whole chapter. **The turn-on, ratified** (Bagrat, 2026-10-18): he approved the banner, its confirm, its turning-on, failed and on faces and the switch in Settings · Space as drawn (AiBanner, AiConfirm, AiBannerStates, AiSettings), with the price said “per video hour” so it reads as hours of video, not of the day; he may tweak the words later. **The landing's line, agreed** (Bagrat, 2026-10-18), goes on shroom.am with the homepage's redesign, not before: “AI titles, chapters and captions are free for about 2½ hours of new video a day. Recording more? About 4¢ per video hour on Cloudflare’s $5 plan.” **The privacy page's AI point, approved** (Bagrat, 2026-10-18), as drafted on the board AiPrivacy. The rest is provisional: its words are drafts until he OKs them. | provisional | 2026-10-18 |
| N67 | Chapters in the seek bar, captions, and the named title on the page | **Chapters live in the page's seek bar** (Bagrat's ruling, 2026-10-18, relayed by the orchestrator; he approved the canvas boards “The page — chapters in the seek bar, named on hover” and “The page on a phone — chapters in the seek bar, named while a finger is on it” the same day). The bar splits into one section per chapter, 2 px apart. Hovering it, playing or paused, shows the time and the chapter's name at that point, and the section under the pointer grows a little. On a phone a finger on the bar shows the same above the thumb, while it drags and for a moment after a tap, and the thumb grows while the finger is on it. The chapter now playing is named after the time, cut short when long, where the player's row has room for it; a phone's row has none beside its five controls, so there the finger's tip names it (a change from the phone board as approved, found while building and put back on the canvas; Bagrat kept it so, 2026-10-18, over a line under the controls or no sound button on phones). The bar's spoken value names it everywhere. There is no chapter list. A video with fewer than two chapters keeps today's whole bar. **Captions**, when the space has them: a Captions button in the player's row turns them on; off by default, and remembered on that browser. **The named title** is the page's title, everywhere a title shows (the page, its tab, unfurls, the download's name, the Library). On the owner's page a line under the title says it was named from what was said, until the title is edited; while the free day is used up, it says the name comes a little later instead. Built: DECISIONS B209. Provisional: the words are drafts until he OKs them. | provisional | 2026-10-18 |
| N63 | The editor: trim, cut, undo, and edits the link plays | **Bagrat asked for it, 2026-10-18** (relayed by the orchestrator): a real editor, apart from AI, that the spoken-cue layer can plug into later. **Edit… in a Library row's menu** opens the **editor**, a window of its own (one at a time), on a live video in this Mac's scope, renamable here: the video plays at the top; under it the play button and the time, edited position / edited length, with “was 4:32” once the edit changes the length; then the **timeline**: the whole recording's sound as a waveform with the playhead, **yellow ends that drag to trim**, and **a drag across the sound that selects a part**, with **Cut 12.0 s** under it. **A cut shows red** and the preview skips it as it plays; clicking a cut offers **Put back**. **Undo and Redo** (also ⌘Z and ⇧⌘Z) step through every change made in the window, Put back and Revert included. **Revert to original** puts back every cut and both ends. **Done** (Return) saves the edit and closes the window; ✕ is Done. The footer says the link stays the same. Saving is a small edit list kept in the video's record; **the share page and the embed apply it as they play** (they skip the cuts, hold to the trim, and show the edited length), so **the link never changes** and nothing is re-encoded first. Until then the original stays in the space, so Revert works from any Mac. Then the window stays to make **the final video**: the Mac cuts the edit with its own encoder and puts it up beside the original under the same link (“Making the final video… 40%”; the window can be closed and shroom finishes it); once it is up, Download and the page use it. **Then it asks “Delete the original?”** (Bagrat, 2026-10-18: encourage deleting it soon; no 30-day keep): **Yes, delete** leads, filled — a click, never Return (law 9; DEVIATIONS #55, ratified 2026-10-18) — and the edit becomes the video for good; **Keep for 2 days** keeps the original so the edit can still be reverted, and shroom deletes it after two days, whether or not this Mac is open then; closing the window there means Keep for 2 days. While the original is kept, the editor says by Revert how long it has left; once it is gone, the edit is final and Revert to original only undoes what changed since the window opened. A video that isn't on this Mac comes from the space first (“Getting the video from your space…”); if that fails the window says so and offers Try again. While it saves, Done reads Saving…; if the save doesn't go through, the window stays open and says so, and Done tries again. Edit… greys offline and while a take is open (law 4), as Rename does. Built: DECISIONS B195, B197. **Ratified by Bagrat as built, 2026-10-18**, with B195–B199 and the build’s words. | ratified | 2026-10-18 |
| N64 | Markers: Drop a marker comes back while paused, and ⌥⌘M drops one while recording | **Bagrat asked for markers dropped while recording, 2026-10-18** (relayed by the orchestrator), as the POC once had. (1) **Drop a marker comes back to the paused popover**, between Finish & Share and Restart, reversing B143's hiding now that the editor (N63) shows markers. It lands a marker at the pause point; once dropped, the row reads **Marker dropped** and greys until the take resumes (a second marker at the same point says nothing). (2) **⌥⌘M drops a marker while recording**, at that moment of the take, held only while a take is open, exactly as N54's ⌥⌘P (no new permission, let go when the take closes, nothing on screen names it in v1, and nothing on screen answers it: the take goes on). (3) **Markers show as flags on the editor's timeline**, at their times on the recording. They never reach viewers in v1. Built: DECISIONS B196. **Ratified by Bagrat as built, 2026-10-18**, with B195–B199 and the build’s words. | ratified | 2026-10-18 |
| N69 | Edit in shroom: the owner's page opens the video in shroom's editor | **Bagrat asked for an Edit button on the video's page that opens it in the editor, 2026-10-18** (relayed by the orchestrator), and approved the boards “The page — Edit in shroom” and “Editor — opened from the page's Edit” as drawn, with their lines, option A and the tooltip “Edit in shroom” (2026-10-18). (1) **The owner page gains Edit in shroom**: the scissors, the last of the quiet icons beside the title, after Download, its tooltip “Edit in shroom”. It shows only on the owner page (the space's owners on every video, a member on their own) and only once the video is fully up, as Edit… in the Library; a recipient's page and the embed never have it. (2) **It opens `shroom://edit/<the space's address>/<the video>`**, with nothing secret in it: the browser asks first (its own “Open shroom?”), and the Mac decides by its own state. (3) **The Mac's answers.** A video this Mac can edit (the owner's Macs: every video in the space; a member's Mac: their own) opens in the editor as Edit… opens it. The editor already open comes forward as it is, one at a time; holding another video, its footer says “You're editing another video — press Done, then Edit again.” While a take is open the press waits (law 4), and the editor opens once the take ends. Otherwise the popover opens by itself under the mushroom with one line, said once: “This Mac isn't set up for that video's space.”, “Only whoever recorded this video, or the space's owner, can edit it.” or “That video isn't in your space anymore.” A Mac not set up at all opens its welcome window, as a click on the mushroom does. (4) **Where shroom isn't on the Mac, the page says nothing** (option A): only a Mac running shroom makes a browser the owner's, so the button shows only where it can work. Built: DECISIONS B221. | provisional | 2026-10-18 |
| N70 | The landing's hero: shroom, recorded with shroom, with the walkthrough behind a button | **Bagrat approved the board “shroom.am — the voice-cut hero takes the box”, 2026-10-18, and asked for it to go live, 2026-10-19.** (1) **The landing's box holds the hero**: a two-minute take of shroom recorded with shroom itself, his voice cut included (“shroom cut”, cut and put back on screen), music under it and the how-to's end card. It is shroom.am's own file (`/hero.mp4`), loads only as its block nears the screen and plays in place on a click, as N61's video did. Its still is the editor after the take's cuts with his camera bubble and the cut row; the play button reads “Play shroom, recorded with shroom, 2 minutes” to VoiceOver and the length shown is “2 min”. No caption. (2) **“How to set up and use shroom” moves behind a small Setup walkthrough button** under the hero, then a 3 px cut through the box, then Install. The button opens it in a lightbox over the page (the whole screen on a phone); Esc, Close or a click outside closes and stops it, and the hero pauses while it plays. (3) **The hidden /next**, where the page waited, sends anyone who kept the link to `/`. The layout of N59 is otherwise unchanged. Built: DECISIONS B222. Ratified by Bagrat, 2026-10-19. | ratified | 2026-10-19 |

## Surfaces

- **The tray mushroom** — Lives: always | Opens: — | Rules: five faces (N38); the click is the door (N21); right-click → Quit shroom (idle only — mid-take it's a click, N39)  [UX-STATEMACHINE.md L1715]
- **The popover** — Lives: under the mushroom, on its display, however it opened (N57) | Opens: on a click, a launch you did, all-set's Record now, or a pause or stop you didn't make (without the keyboard) | Rules: `config` · `paused` · the confirms that morph from them; one banner + one status line (N38); never brings the dock icon  [UX-STATEMACHINE.md L1716]
- **The window** — Lives: centered on the mushroom's display (N57), one at a time per kind | Opens: the editor (Edit…, N63), onboarding, Library, Settings, Send feedback (N60) | Rules: the dock icon while open; ✕ ≡ the retreat (N12); survives a browser round-trip  [UX-STATEMACHINE.md L1717]
- **The page** — Lives: the space's own address | Opens: Stop, Get link, Open page, a shared link, an invite | Rules: owner-mode by cookie, per member (N40); `noindex`; the mark bottom-right  [UX-STATEMACHINE.md L1718]
- **Terminal** — Lives: the user's own | Opens: the install command | Rules: the readable script; refuses mid-take; the update path forever  [UX-STATEMACHINE.md L1719]
- **The overlays** — Lives: the chosen display | Opens: the countdown scrim; the bubble from `config` through the take; the agent label while a take the shroom command started records (N62) | Rules: never windows, never the dock icon; the bubble never takes focus, while the countdown does — you just started it, and its Esc must work without any extra macOS permission; the bubble is the only thing of ours ever in a take (the label, like the countdown, is left out of it)  [UX-STATEMACHINE.md L1720]
- **The command line** — Lives: the person's terminal, or their coding agent's | Opens: — (a link in the browser when asked); its recording refusal carries a link that opens Settings at General's switch, for the person to click | Rules: the space's own page under this Mac's device key, never the token; nothing undoable without --yes; waits while a take is open (law 4); recording only through the running app, and only with General's switch on (N62)

## Absent from v1

- Local recording before setup and any local library (U1)  [L1724]
- the .dmg (P17; brew joined v1 by N49)  [L1724]
- macOS notifications (U9)  [L1724-1725]
- global hotkeys, save ⌥⌘P's pause and resume and ⌥⌘M's marker during a take (N54, N64) (the reopen door covers a hidden mushroom, N41)  [L1725-1726]
- the tray's Copy last link and any auto-copy (N21, N1)  [L1726]
- transcript search (P8; captions left this list with N67)  [L1727]
- an editor beyond trim and cut: titles, overlays, transitions, a second track (P11; trim, cut and markers joined v1 with N63 and N64)  [L1726-1727]
- a sender name or avatar (U3)  [L1727-1728]
- passwords or expiry on links (P1)  [L1728]
- storage sizes and cost figures (P13)  [L1728]
- custom domains (N2)  [L1728-1729]
- view tracking  [L1729]
- more than one space per Mac (N31)  [L1729]
- deleting from the page (delete lives in the Library)  [L1729-1730]
- an uninstall button (N36)  [L1730]
- window/region capture (§5.2; system audio left this list with N47)  [L1730-1731]
- OAuth sign-in (P5)  [L1731]
- a second repair UI (N42)  [L1731]
- an instant member cut-off control (parent-token rotation stays a scaffold flag)  [L1731-1732]
- deleting a whole space from the app (Cloudflare's dashboard does it; the docs say how)  [L1732-1733]
- a member's own Add another Mac (a member's second Mac is a second invite, N33)  [L1733-1734]

## Scaffold flags

- **Per-recording credential minting** — N32's removal copy is true only if creds mint per recording at upload start; per-member creds would force “for up to a week”.  [L1740-1741]
- **The member cookie** — owner-mode keys on the member, not the space (N40); the Worker checks maker vs cookie, owner-class sees all.  [L1742-1743]
- **Launch provenance** — tell a launch you did from a login-item or update relaunch (N41); treat the reopen event as a tray click in every state.  [L1744-1745]
- **The popover's anchor fallback** when the status item is occluded (N41).  [L1746]
- **The reachability probe at stop** — capped well under a second, so a live stop never waits on it (N40).  [L1747-1748]
- **Thresholds** — the stall window for *on its way, stalled* (N40); disk-low and the disk floor at each quality (N39); the upload no-progress window.  [L1749-1750]
- **Deep link registration** (`shroom://join/…`) at first launch, and the browser's own “open shroom?” prompt as the only step it adds (N29).  [L1751-1752]
- **Own-Mac invites** — one hour, owner-class, labelled by the Mac's sharing name (N33).  [L1753]
- **The page's interface stays backwards-compatible** across app versions (N37).  [L1754]
- **The page refuses a downgrade** — a token-holding Mac on an older app, repairing or updating, never replaces newer page code (N37, N42).  [L1755-1756]
- **Label fallback chain** — if `shroom` is taken too, the next free of `shroom-v`, … (N42).  [L1757]
- **Tombstone check before every upload resume**, so another Mac's delete stops this Mac's upload (N35).  [L1758-1759]
- **Non-activating popovers** for everything shroom opens unasked (law 5, N39).  [L1760]
- Carried from before: the min-OS floor and Intel stance (Region 1) · the name charset edges (N15) · swap-under-running-process behavior (N9) · parent-token rotation UI (N3).  [L1761-1762]

## Notes

The map's notes: lines about a state that aren't a state.

- **launch-door** (Region 1, about `idle-unprovisioned`): a launch you did is a click — N41 · D Opening shroom yourself — Spotlight, Finder, the Dock, a fresh install onto a Mac that still remembers its space — acts exactly as a tray click, in every state (mid-take: it's the pause). The update lane stays silent (N9). And the quiet-take law: while a take is open nothing new appears, and nothing that would end or move the app runs — Update, Disconnect, Leave, Quit grey out with “You can do this once your recording is finished.” Launches shroom does itself stay silent — the login item — except the update relaunch, which opens the popover to say how it went (B155). Also the rescue when a crowded menu bar hides the mushroom behind the notch; the popover then drops from the top-right.
- **charset** (Region 2, about `name`): charset guard — not a state (N18 nuance) “Lowercase letters, numbers, and dashes.” a stray character never lands — the line appears, then fades; uppercase lowercases silently. No transition happens, so this stays a guard.
- **settingsnote** (Region 2, about `done`): Re-entry after success is Settings → Space (N36): the six ticked rows with their facts — “Your space is ready — nothing to redo.” — and Disconnect this Mac… at its foot. See Region 5.
- **mic-denied** (Region 3, about `perm-mic`): A denied mic never blocks — silent screencasts are legitimate. config later shows the mic off with a quiet “allowed in System Settings” hint.
- **camera-note** (Region 3): Camera is never asked at onboarding — it prompts inline at the first camera-mode pick. Don't ask for hardware the user may never use.
- **arm-guard** (Region 3): Arm-time guard, not a step: every record arm re-probes grants and replays exactly the missing beat. Docs carry Apple's ~monthly re-confirm honestly — their policy.
- **statusnote** (Region 3, about `config`): the popover's grammar — N38 · D Two slots, never more. The banner (top) is an ask — one at a time, highest first: a saved take · the space can't take videos · recording switched off · an update (Frost, slimmest). The status line (above the footer) is a fact — “Uploading — 40%. Your link already works.” · “Uploading two videos — 40%.” · “Waiting for connection — your links already work.” · “Updated to 1.3.0 ✓” once. Disk-low sits under Start. Return = Start recording.
- **devices** (Region 3): devices at config — N39 · D Never auto-pick an iPhone — mic or camera. A last-used display that's gone falls back to the one with the menu bar. No microphone: “No microphone — your video will be silent.” Camera as the source previews at config, like the bubble. With a camera picked for the bubble, picking another display moves the bubble there. Same-named displays get their place: “LG UltraFine (left)” / “(right)”. Camera denied: the camera picker goes back to Camera off (DEVIATIONS #43), Camera-as-source greys Start — “Camera is off for shroom —” [ Open System Settings ]. The count is Restart's undo: the old take dies only when the new capture begins.
- **library-faces** (Region 5, about `library`): library faces empty: “No videos yet — click the mushroom in your menu bar to record your first.” · no match: “No titles match “502”.” · offline: “You're offline — showing what this Mac remembers. Renaming and deleting wait until you're back.” (Copy link stays live) · space unreachable (N34): the remembered list; only Copy link and Save to Movies live.
- **space-moods** (Region 5, about `settings-space`): Space, other Macs Owner-class, no token: “✓ Your space — v.myshroom.workers.dev” · “✓ Letting macOS allow recording — on this Mac” · “Repairs happen where your Cloudflare token lives — on the Mac you set your space up on, or here if you add a fresh one.” [ Add a token here ] — Member: “✓ Bagrat's space — v.myshroom.workers.dev” · row six · “Your videos live there, where Bagrat can see them too.” · Leave this space… — the mark switch is owner-class only.
- **invite-row-faces** (Region 6, about `invite-born`): row faces — not states (N18 nuance) “invited just now” → “invited Aug 21 · 5 days left” → expired: the row offers New link — nothing is blocked, so no transition happens.
- **join-owner-view** (Region 6, about `join-landing`): owner view — the owner's cookie rides along “This is the page Ani will see — opening it here doesn't use it up.” one quiet note on top of the same page — the owner who clicks their own link to check it never wonders if they spent it.
- **save-movies** (Region 6, about `member-revoked`): Save to Movies — the stranded-take rescue (N34) The take's single-file copy lands in ~/Movies under its title and Finder shows it. The take keeps waiting — it still uploads if the space returns — until discarded. Several: “Save three to Movies”. After saving, the banner reads “Saved to Movies ✓” with a quiet Discard…. No take is ever lost to a space that went away.

## Events

How a rule names the events it moves on. A pattern is an event's kind as the core's `Event` names it: `TrayClicked`, `WindowClosed`, `Act(Restart)`, `Cloud(Swept)`, `Failed(Wire)`; a bare `Cloud`, `Rec`, `Os` or `Failed` matches every kind under it. The words below stand for several.

- **answer**: Cloud, Rec, Failed, Tick, Os, Member, Upload, Join, Scanned
- **away**: Act(Dismiss), Key(Esc), PopoverDismissed
- **click**: TrayClicked
- **any**: *
