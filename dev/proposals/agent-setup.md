# Agent-driven setup: `shroom setup …` (a proposal)

**Status: approved by Bagrat 2026-10-30, with the Law 13 deviation (§4), and the board
("Agent setup", images in `agent-setup/`) approved the same day; built as spec N88, DECISIONS
B277 and DEVIATIONS #59.** What the build settled differently from this draft: the
installer's lines are unchanged (Installed is the agent's cue to run `shroom setup`); an
agent's setup never registers the login item (its box was never shown); and a Settings
repair (`space-degraded`) stays the window's for now — `shroom setup` drives the setup
window's states only.

**The ask (Bagrat, 2026-10-30, paraphrased).** Besides the wizard, which stays for people
without an agent, an agent-led setup: the same setup with no window, the agent driving it
by a script. Sign-in with Cloudflare can't make the token (RESEARCH R7.7a, B170), so the
token still comes from the person.

**His rulings, 2026-10-30:**

1. **No window.** An agent setup runs with only the mushroom in the menu bar; a click on
   it opens the normal window at the current step.
2. **The token by the clipboard, read by the command.** `shroom setup token` reads the
   clipboard itself, the way `pbpaste` does (one plain command, no pipe), and also takes
   the token on stdin (`pbpaste | shroom setup token`). Never in argv, never in output.
   The whole setup becomes `shroom setup …` commands an agent can run anytime: to start,
   resume or repair.
3. **Permissions one per run**, in the wizard's order (microphone, camera, then screen
   recording in System Settings with its quit and reopen). Each run says what to click;
   the agent runs it again until all three are answered. A denied mic or camera never
   blocks.
4. **The page is short; the words are in the command.** shroom.am/agent gets a "set it up
   too" part: install, then `shroom setup --json` and follow each answer until it says
   done. Written for any coding agent with a terminal on the Mac, naming none.

---

## 1. The flow, as the agent runs it

Every answer is one JSON object: `{"ok", "at", "say", "do", "next"}`. `at` is the wizard's
own state id (`account`, `token`, `gate-card`, … `all-set`), `say` is the line for the
person, `do` is what the person does (or `null`), `next` is the command to run after.
The agent says `say`, waits for the person when `do` is set, then runs `next`.

| # | The agent runs | shroom does | The person does |
|---|---|---|---|
| 0 | `curl -fsSL https://shroom.am/install \| sh -s -- --agent` (asked first, as today) | installs, opens as the mushroom only | says yes |
| 1 | `shroom setup --json` | answers `account` | says whether they have a Cloudflare account |
| 2 | `shroom setup account new` or `… account have` | `new` opens Cloudflare's sign-up | signs up, confirms their email |
| 3 | `shroom setup token open` | opens the pre-filled token page | reviews, creates, presses Copy, says "copied" |
| 4 | `shroom setup token` | reads the clipboard, checks the token, saves it, clears the clipboard; runs the probe pass | — |
| 4a | `shroom setup account pick <n>` | only when the token reaches several accounts | picks one |
| 5 | `shroom setup wait` | waits (≤ 60 s a run) while a gate is shut: email, then card | confirms the email / adds the card |
| 5a | `shroom setup card open` | opens Cloudflare's billing page | adds the card |
| 6 | `shroom setup name '<name>'` | claims it (`name-taken` → ask again); skipped when the account has one | chooses the link name |
| 7 | `shroom setup permissions` (×3–4) | asks the next permission; screen recording opens System Settings, then quits and reopens as the mushroom | clicks Allow / flips the switch / Quit & Reopen |
| 8 | `shroom setup wait` | until wiring's last part ticks | — |
| 9 | — | answers `all-set` with the link name | — |

Steps 5 and 8 are skipped when nothing waits. Any step can be run again: each command
re-reads where the setup stands (the plan is derived, never stored), so a command run
at the wrong moment just answers where things are and what's `next`.

**Anytime.** The same commands resume a setup after a quit or a day away, and repair a
broken space on the token Mac (`space-degraded`: the token revoked → `token`, the card
removed → `gate-card`, the page gone → wiring). The window and the commands drive the
same core, so someone can switch midway either way.

## 2. The human gates that remain

None of these can be taken over; the agent only says what to do and waits.

- **The Cloudflare account and its email** (sign-up, the confirmation mail).
- **The token**: review the pre-filled page, create, press Copy.
- **The card** that switches storage on (no billing API).
- **The link name**: the person's choice; the agent asks.
- **macOS's three permissions** and the quit-and-reopen for screen recording.

## 3. What it saves, and what it can't

**Saves:** reading the wizard and finding the next button; leaving the chat; questions
answered as they come ("why a card?"); resuming or repairing by asking the agent
("shroom says my space needs me").

**Can't save:** any of §2. The same three browser trips (sign-up, token, card) and the
permissions stay, and each `shroom setup` run may cost an approval in the agent. In
time it's about the wizard's ten minutes; the gain is that the person stays in one
conversation.

## 4. What changes

**Spec (`spec/machine.json`, provisional until ratified).**
- Region 1: an `--agent` install launches with no welcome: `first-launch` → `idle-unprovisioned`
  (new edge), the tray click still opens the welcome, now at the current step.
- Regions 2–3: every setup state gains an **agent face**, a `say` line (role `agent-line`)
  and a `do` line where the person acts, in the window's one voice. Drafts below.
- A standing note: the commands' state is the window's state; a running window follows.

**The app.**
- Control socket ops `setup.status`, `setup.account`, `setup.token` (the secret on the
  socket line, once), `setup.pick`, `setup.name`, `setup.open` (signup / token page /
  billing), `setup.permission`, `setup.wait`. Each is the window's own click or paste,
  posted to the core; the answer is a state id and codes, never words (as `record.*`).
- `--relaunched=install-agent`: no welcome window. After the screen-recording relaunch, a
  setup the commands drove comes back as the mushroom only.
- The status seam (`status.json`) is unchanged; it already carries `setup`.

**The CLI (`crates/shroom-cli`).** `shroom setup` and its verbs; the words (`say`, `do`)
come from the spec's agent faces. `token` reads `NSPasteboard` in the command's own process
(or stdin when piped), checks the shape locally, sends it on the socket, clears the
clipboard only if it still holds that same value, and prints nothing of it. `setup` opens
the app (tray only) when it isn't running.

**The installer.** `--agent`: the same install, launched with `--relaunched=install-agent`;
its last line says to run `shroom setup`.

**The pages.** shroom.am/agent: the "set it up too" part (draft below). shroom.am/cli: one
row, "Set shroom up / pick up the setup → `shroom setup --json`, and follow it".

**A deviation to ratify.** Law 13 says secrets ride files only. The token rides the control
socket once (600, peer uid checked, never logged), and nothing else ever does; on disk it
lands only as the wizard's own `token.json`. It's tighter than a hand-off file, which would
hold an unchecked token on disk.

## 5. Draft copy (for the canvas)

The agent page, under "If you're an agent", after the install:

> If they also want you to set it up, install with `sh -s -- --agent` at the end instead.
> Then run `shroom setup --json` and follow each answer: say its `say` line to them, wait
> while they do its `do`, then run its `next`. Stop when `at` is `all-set`. They click
> every button in Cloudflare and macOS; you never do. The token goes from Cloudflare's
> Copy button straight to `shroom setup token`: never run `pbpaste` yourself, and never
> ask for the token in this chat. If they'd rather use shroom's window, they click the
> mushroom in the menu bar.

Agent faces, a few (the rest follow the window's lines):

| `at` | `say` | `do` |
|---|---|---|
| `account` | "Your videos will live in your own Cloudflare account. Do you have one already?" | answer |
| `account-signing-up` | "Cloudflare's sign-up is open in your browser. Finish there and confirm your email from their message, then tell me." | sign up |
| `token` | "Next, one access token for shroom. The page is pre-filled — two narrow permissions. Create it, press Copy, and tell me — not the token itself." | create, Copy |
| `token-garbled` | "That copy lost some characters — press Copy once more and tell me." | Copy |
| `gate-card` | "One step on Cloudflare's side: adding a card switches storage on. The free tier stays free — the card is their rule, not a charge." | add the card |
| `name` | "What name should your links carry? It's part of every link: v.yourname.workers.dev." | answer |
| `perm-mic` | "macOS is asking for your microphone — click Allow." | Allow |
| `perm-screen` | "System Settings is open at Screen Recording: flip the switch next to shroom, then choose Quit & Reopen." | flip, reopen |
| `all-set` | "You're set: v.myshroom.workers.dev. Click the mushroom in the menu bar to record." | — |

## 6. Tests (spec tests first, committed separately)

- **Spec:** every Region 2–3 setup state has an agent face; the new Region 1 edge.
- **CLI, against the cloud fake (`shroom-cloudfake`), in the cloud:** a full setup driven
  only by `shroom setup …` — account, token, both gates, name, taken name, several accounts,
  space-found, offline, wiring; and resume after a relaunch.
- **The secret:** a canary token through clipboard and stdin never in the command's output,
  argv, the run log or a Scene; the clipboard cleared only when unchanged.
- **App:** each `setup.*` op is the click it names (the onboarding drive, window shut);
  `install-agent` opens no window; the mushroom opens the window at the current step.
- **Pages / installer:** `site/test/agent.test.mjs` for the new part; `install.test.mjs`
  for `--agent`.
- **The Mac (hosted):** the permissions run and the screen-recording relaunch.

## 7. Order of the build

1. This proposal → his go. 2. The canvas: an "Agent setup" board (the conversation, step by
step) → his look. 3. Spec + spec tests. 4. App ops and `install-agent`. 5. The CLI.
6. Installer and pages. 7. Hosted-Mac check of the permissions.
