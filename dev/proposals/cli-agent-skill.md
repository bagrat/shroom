# shroom on the command line, for coding agents (a proposal and a POC)

**Status: built in full (2026-10-18), on Bagrat's ask the same day to build it now and
decide the release later. Provisional: spec N62; DECISIONS B215–B219.** Branch
`claude/cli-agent-skill`, PR #58, open for review and not to merge until he says
(merging deploys `/cli`). What changed from the POC to the build: §11.

**Bagrat's calls, 2026-10-18** (relayed by the orchestrator, from options he picked):

- An agent may start and stop recordings only once a Settings switch is on. It is off by
  default.
- The command may make invites.
- The command is always installed: by the install line, inside the bundle, and with
  Homebrew.
- Rename and delete keep refusing while a take is open (law 4).
- The card's words and place were left to this session.

He also asked the threads to decide the rest themselves; those calls are in §9.

**The ask (Bagrat, 2026-10-18, paraphrased).** A quick win: shroom usable from the command
line whether the app is running or not, so someone who already works with a coding agent
can ask it to use shroom. A served skill the agent reads, then `shroom --help`, then it lists
videos and so on. In the app: "here's a prompt for your agent". Anything shroom does
should be doable through the CLI.

**In one line.** A `shroom` binary inside the app bundle that reads nothing secret, asks the
space's own page under this Mac's device key, and asks the running app to record. Every
answer is JSON on request, with fixed error codes. A one-page guide at
**shroom.am/cli**, in Agent Skills form, tells an agent how to use it and when to ask
first. In the app, Settings → General gets a "Your coding agent" card with
`Use shroom on my Mac: read https://shroom.am/cli and follow it.`, [ Copy ], and the switch
“Let it start and stop recordings” (off by default).

---

## 1. What was found

**The POC's lessons** (`history/POC-LEARNINGS.md`, the archive's `archived-poc/`):

- *Keep* the determinism boundary: the mechanism is exact, the judgment is the agent's.
  The POC's scripts answered `{ok, reason, hint}` with fixed reason codes, and the agent
  branched on them.
- *Keep* "one target per destructive call, no sweep", and refuse to drop the only copy
  without `--force` (`archived-poc/scripts/cleanup/README.md`).
- *Keep* one call that carries everything (`--prep`), and fail soft on paths that aren't
  core.
- *Avoid* capture from a CLI child. TCC charges the grant to the process that asks, so a
  child would prompt as "Terminal". A screen grant needs a relaunch, and the Sequoia
  consent names the GUI app. **Recording must go through shroom.app.**
- *Avoid* secrets on argv, in chat or in logs (law 13), and an agent that is itself the
  state machine (the 500-line skill that couldn't be tested).
- *Avoid* compound shell. Each prompt to approve one was friction, and that is part of
  why the BYO-agent skill was retired (B132, DV #50). The demo hit this again (§7).
- B132 kept `status.json` public "for a future doctor". `shroom status` is a first reader
  of it.

**Conventions** (clig.dev; the Agent Skills spec at agentskills.io; 2026 write-ups on CLI
vs MCP for agents):

- `--json` gives one object on stdout. Messages go to stderr. Exit codes are distinct.
  Nothing prompts when not on a TTY. A destructive act needs `--yes`, with `--dry-run`
  beside it.
- A skill is `SKILL.md` with `name` (≤ 64, lowercase-hyphen) and `description` (≤ 1024,
  saying what it does and when to use it), under 500 lines. Claude Code, Codex, Cursor,
  Gemini CLI, Copilot and others read the format.
- **No agent loads a skill from a URL.** They load from `~/.claude/skills`,
  `~/.agents/skills` or a plugin. So the zero-install door is a prompt that says "read
  shroom.am/cli". The install door (`npx skills add …`, or the draft
  `/.well-known/agent-skills/index.json`) can come later.
- For a coding agent, a CLI costs about 200 tokens a call. An MCP server costs tens of
  thousands of tokens just to load its tools. CLI first; MCP only for shell-less clients.

The competitor comparison is in shroom-private (`strategy/research/`), not here.

## 2. How it works

```
 coding agent ──runs──▶ shroom (CLI, in the bundle)
                          │  reads ~/.shroom: space.json, device.key, status.json, library.json
                          │  (never token.json)
                          ├──▶ the space's page  POST /_shroom/api/library · /mint (manage) · /gone · /invites · GET /members
                          │      └─ a manage credential ──▶ the bucket (rename / delete, 15 min, memory only)
                          └──▶ shroom.app over ~/.shroom/control.sock  (record start · pause · resume · stop)
```

- **One door: the device key** (WORKER-INTERFACE §4.10, §6.8). This is the path every
  member's Mac already uses for its Library. The token Mac has a device key too: it
  enrolls at wiring. The CLI therefore **never reads the Cloudflare token**, and owners
  and members take the same code path. A member sees their own videos and an owner's Mac
  sees everyone's: the Worker scopes it, not the CLI.
- **Works whether the app runs or not.** List, info, link, open, rename, delete, cleanup
  and members need only the files and the network.
- **Reads only.** The app is the only writer of `~/.shroom`, so the CLI never races it. It
  seeds the list from `library.json` so unchanged records aren't read again, and never
  writes that file. The app's Library catches up on its next refresh.
- **Deletes the way the Library does.** It mints a `manage` credential, writes the
  tombstone first, deletes the bytes, then seals with `/gone` (B125). The test checks all
  three on the real `worker.js`.
- **Quiet while recording (law 4).** A rename, delete, invite or removal refuses with
  `take_open` while `status.json` says a take is open.
- **Recording goes through the app** (`app/src-tauri/src/runtime/control.rs`, built).
  - **The socket.** The CLI writes one JSON line to `~/.shroom/control.sock` and reads one
    line back. The folder is 700 and the socket 600, and the app checks every peer's uid
    (`getpeereid`).
  - **What the core does with an ask.** It answers `Event::Cli` with `Effect::Cli`
    (`crates/shroom-core/src/apply.rs:cli`):
    - **refused** with `recording_off` while General's switch is off;
    - **start** is the popover's Start, exactly as if the popover were open: its
      countdown, its permission beat, its grants re-read first;
    - **pause** is the tray's pause;
    - **resume and stop** are the paused popover's;
    - **between two steps** (pausing, resuming, stopping) it answers `busy`;
    - **a stop** answers its link.
  - **Codes, not words.** The app answers with a code, and the command says the words.
  - **When the app isn't running**, `record start` opens it (`open -g -b am.shroom`) and
    waits up to 15 s.
  - **The demo** uses a stand-in with the same answers,
    `crates/shroom-cli/examples/fake_app.rs`.
  - **On a real Mac** (`dev/mac/scenarios/cli-socket.sh`, the hosted Mac, 2026-10-18, all
    7 checks pass): the dev app made the socket at 600; the command's four asks were
    refused as `recording_off` with the switch off; with it on, start reached the core and
    was answered `not_set_up` (that throwaway home has no space).
  - **Not yet shown:** a take started through the socket in a wired space. That needs the
    test account's e2e space (the heartbeat's setup).
  - Rejected: the `shroom://` URL scheme. Any web page can fire it, and nothing comes
    back.
  - Rejected: localhost HTTP. It needs a token file, and it is reachable from more than
    this user.
  - Rejected: XPC with signature checks. It is the strongest option, but heavy for a
    self-signed app. Worth revisiting if the socket is ever thought too weak.

## 3. The command surface

| Command | Does | POC |
|---|---|---|
| `shroom status` | app running? recording? set up? the space's host and role | built |
| `shroom list [--search W] [--mine] [--limit N]` | videos, newest first | built |
| `shroom info <video>` | title, date, length, link | built |
| `shroom link <video>` | the link alone | built |
| `shroom open <video>` | opens it in the browser | built |
| `shroom rename <video> <title>` | rename, live on the next page load | built |
| `shroom delete <id-or-link>… [--dry-run] [--yes]` | delete for good; without `--yes`, exit 7 and nothing changes | built |
| `shroom cleanup-candidates [--older-than D]` | old videos, oldest first; `views`/`last_viewed`/`score` reserved (null) | built (age only) |
| `shroom members` | owner, their Macs, members, open invites (owner-class) | built |
| `shroom invite <name>` · `shroom invite cancel <id>` | a member invite link (7 days, one use) · cancel it | built |
| `shroom members remove <member-id> [--dry-run] [--yes]` | they leave the space; their videos stay | built |
| `shroom record start\|pause\|resume\|stop` | asks the app, once the switch is on; `stop` returns the new link | built (both sides) |
| `shroom guide` | prints the guide (the same text as shroom.am/cli, built in) | built |
| `shroom save <video>` (Save to Movies), `shroom doctor` | later | — |

- **`<video>`** is an id, a link to this space, or title words in any order. Several
  matches give `ambiguous` with the matches listed. **A delete takes only ids or links,
  never title words.**
- **Errors.** With `--json`, a failure is
  `{"ok":false,"error":<code>,"message":<product voice>}`. The codes and exit statuses are
  a contract, added to and never renamed:

  | Exit | Code | Meaning |
  |---|---|---|
  | 1 | `failed` | anything else |
  | 2 | `usage` | the command line was wrong |
  | 3 | `not_set_up` | not set up, or not finished joining |
  | 4 | `not_found` / `ambiguous` | no such video, or several match |
  | 5 | `offline` | the space can't be reached, or is busy |
  | 6 | `refused` | the space said no |
  | 7 | `needs_confirmation` | a delete without `--yes` |
  | 8 | `app_unreachable` | the app is needed and didn't answer |
  | 9 | `take_open` | a recording is open |
  | 10 | `recording_off` | General's switch is off: the agent may not record |
  | 11 | `partial` | a batch: some items went through, some didn't (every result is in the answer) |

- **The cleanup hook.** The sibling cleanup-score proposal (view analytics) is the source
  for `views`, `last_viewed` and `score`. The guide tells an agent that `null` means
  *not known*, never zero (the first demo run read it as "none viewed").

  Once that proposal's `POST /_shroom/api/cleanup` lands (draft #57, its §6), `cleanup()`
  in `crates/shroom-cli/src/main.rs` changes as follows:
  - It asks the route under the device key, with `{"scope": "mine" | "everyone"}` (by
    role, as `list` does), `--formula` passed through, and pages on `next`.
  - It keeps the items whose `candidate` is true, ranked by `score`.
  - Its fields map as: `plays` → `views`, `lastPlayedAt` → `last_viewed`, `score` →
    `score`, and the route's `reason.code` (`never-played` · `stopped-early` ·
    `not-lately` · `big-and-quiet`) → `reasons`.
  - `signals` becomes the formula's name, and `bytes` and `usdPerMonth` ride along.
  - On a 404 (a page from before the route) it keeps today's age-only list, with the
    view fields `null`.
  - An agent's reading of the JSON doesn't change shape.

  It isn't wired now because the route isn't merged, and whether to count plays at all is
  still Bagrat's call.

## 3b. Many at once (batch — built)

**Bagrat, 2026-10-18:** a team lead asks their agent to invite the engineering team. The
agent works out who (or asks), the lead confirms, and one shroom call returns a name and a
link for each person. The agent then mails each person themselves with its own mail tool,
and shroom sends nothing. He asked for the other batch cases worth having too.

**The shape** (`crates/shroom-cli/src/batch.rs`), the same for every batch command:

- **Items:** each argument is one item (`--batch`), or `--from <file>` (a JSON array, or
  one item per line; `-` reads stdin, for agents that can pipe).
- **Results:** an answer per item, in the order given, each with a stable `status`, plus
  `counts`.
- **Safe to run again:** nothing is done twice. A re-run after a lost answer or a partial
  failure reports what's already done (`already_invited`, `not_found`, `unchanged`) and
  does the rest.
- **`partial`** (exit 11) when any item failed. The answer still carries every result, so
  the agent can say which failed and run it again for those.
- **`--dry-run`** for every batch that changes anything. A delete or a removal still needs
  one `--yes` for the whole list.
- **Caps:** at most 100 items a call, and 20 invites (a space holds at most 20 pending
  invites, WORKER-INTERFACE §4.12). Over the cap is refused before anything is made.

| They ask | Command | Each item says |
|---|---|---|
| Invite the engineering team | `shroom invite --batch "Ann Lee" "Sam Ng" …` → the list to confirm, then `--yes` | `invited` + `link`, `already_member`, `already_invited` (+ `--renew` → `renewed`, a new link), `failed` |
| Delete these old ones | `shroom delete <id> <id> … [--dry-run] [--yes]` | `deleted`, `not_found` (a re-run), `failed` |
| Give these better titles (say, from AI titles) | `shroom rename --batch "<id>=<title>" …` | `renamed`, `unchanged`, `would_rename` |
| The links to these, for a message | `shroom links "<words>" "<words>" …` | `found` + `link`, `ambiguous` (+ its matches), `not_found` |
| What did I record this week? (a digest) | `shroom list --mine --since 7d` | — |
| Who's in, who's still invited? (reconcile against a team list) | `shroom members --json` | — |
| Remove these people | `shroom members remove <id> <id> … [--dry-run] [--yes]` | `removed`, `not_found`, `failed` |
| Cancel the invites nobody used (reconciling) | `shroom invite cancel <id> <id> …` | `cancelled`, `not_found`, `failed` (already used: they're in) |

**Calls (decided X because Y):**

- **Decided: a batch invite asks first, like a delete.** Without `--yes` it makes nothing
  and answers `needs_confirmation` with each name's plan.
  - **Why:** the first run of the batch demo. Told to invite "the engineering team", the
    agent looked the team up in the directory and skipped the list the guide told it to
    show. It invited and emailed everyone, including someone on leave. When told, it
    cancelled that invite at once, but the email had gone.
  - **The lesson** is the old POC's: a rule written for the agent isn't a guard; the
    mechanism must be.
  - **Since the fix,** the same demo shows the list and asks before anything is made.
  - **A single `shroom invite "<name>"` doesn't ask:** the person named that one person.
- **Decided: a person already invited gets `already_invited` and no link, unless
  `--renew`.** The space keeps only an invite code's hash, so a sent link can't be shown
  again. Renewing silently would break a link that may already be in someone's inbox.
- **Decided: names only.** The guide tells the agent to keep email addresses itself and
  never pass them to shroom. A label is admin-private in the roster, and an address there
  would be a person's data shroom has no use for.
- **Decided: a name given twice in one call is asked once** (case aside), and a name that
  is already a member's is `already_member`. A team list often repeats people who already
  joined.
- **Decided: the mailing stays the agent's.** shroom never sends mail (no send-as, and
  nothing we operate). Each person gets their own link in their own message, and the
  guide holds the one-link-per-person rule.
- **Decided: a delete's unknown id is `not_found`, not fatal.** That keeps a re-run after
  a partial delete safe. Title words are still refused for deletes.
- **Not built: a scheduled weekly digest.** `list --mine --since 7d --json` is the data;
  the agent's own scheduler (or the person asking each week) does the sending. A
  shroom-side schedule would be a service we operate.

## 4. Getting it on PATH (always installed — built)

- **In the bundle.** The command ships inside it as a second external binary, beside the
  encoder: `Contents/MacOS/shroom-cli` (`tauri.release.conf.json`).
  - `tools/release-build.sh` builds it for the bundle's macOS floor, and the bundler signs
    it.
  - The build checks it is there, signed, answers `--version`, and refuses the dev-only
    `--home` (a release build compiles the dev options out).
  - It updates with the app, so it never drifts from it.
- **The install line** (`installer/install.sh`, `link_cli`):
  - It links `~/.local/bin/shroom` to the bundle's binary: inside the home folder, as the
    script promises.
  - When that folder isn't on PATH, it adds one marked line to the login profile
    (`~/.zprofile` for zsh, `~/.bash_profile` for bash), for new Terminal windows.
  - It never touches a file of someone else's named `shroom`, and it never fails an
    install.
  - `--uninstall` takes back exactly the link and the marked line (`unlink_cli`).
  - The script's opening comment is ratified copy (`source-view`), so it is unchanged.
    The command is named in its "how it works" note instead. A proposed header line for
    step 3, for Bagrat: "Place it in ~/Applications — no admin password, nothing outside
    your home folder — with its `shroom` command beside it in ~/.local/bin."
  - Tests in `installer/test/install.test.mjs` (they run on the Mac).
- **Homebrew.** The cask links the same binary into brew's bin:
  `binary "#{Dir.home}/Applications/shroom.app/Contents/MacOS/shroom-cli", target: "shroom"`.
  `cask.test.mjs` holds it equal to the install line's path. Not yet tried with a real
  `brew install`, which needs a Mac.
- **The guide's fallbacks.** A Terminal opened before the install doesn't see the new
  PATH, so the guide tells the agent to try `~/.local/bin/shroom`, then the bundle's
  path.

## 5. The guide, served (shroom.am/cli)

- **The source is [`site/cli.md`](../../site/cli.md)**, a valid `SKILL.md`. It is served
  like `/agent`: markdown to an agent or curl, a page to a browser, and always markdown at
  `/cli.md`. The binary embeds the same file as `shroom guide`, so the two can't
  disagree.
- **It holds:**
  - how to find the command;
  - `--json` always, one command per call;
  - a "they ask → run" table;
  - the rules: do only what they asked; dry-run, ask, then `--yes` on a clear yes; never
    delete because something is a candidate; record only when asked right now; links are
    theirs to share; no secrets;
  - one row per error code.
- **`/agent` gains one line:** "If shroom is already on their Mac and they asked you to do
  something with it — their videos, a link, a recording — read https://shroom.am/cli and
  follow that instead." `/` answers an agent with `/agent`, so an agent that only knows
  "shroom.am" finds `/cli` too.
- **`site/test/cli.test.mjs`** holds the guide to the binary:
  - the frontmatter is valid;
  - every `shroom <verb>` and `--option` it names exists in `main.rs`;
  - its error table matches `out.rs` exactly;
  - the ask-first rules are present;
  - no file name of `~/.shroom` and no dev flag appears.
- **Later:** `/.well-known/agent-skills/index.json`, so `npx skills add https://shroom.am`
  installs it once the draft RFC settles. Also `shroom agents install --target claude`,
  which writes `~/.claude/skills/shroom/SKILL.md`, on an explicit ask only.

## 6. In the app: "Your coding agent" (designed, then built)

- **The board** is on the canvas: **“Settings — General: your coding agent (N62,
  provisional; as built on the CLI branch)”**, under the cleanup boards in the Library and
  Settings column. It shows two states: as the window opens, and after Copy with the
  switch on.
- **The card sits under the help line** (my call), so “Something off?” keeps its line
  beneath it. It holds:
  - the label **Your coding agent**;
  - **Read it first** → shroom.am/cli;
  - the prompt in a code well, with [ Copy ] → “Copied” (law 10);
  - the line “Paste it into your coding agent. It can find, share and tidy your videos,
    and asks you before it deletes anything.”;
  - under a hairline, the switch **Let it start and stop recordings**, off by default
    (`prefs.json` `agent_recording`).
- **Rejected:**
  - a line in the Library's footer: the Library is for videos, and Settings is where a
    setting lives;
  - the card above the login row: it pushed the everyday rows down.
- **Built:**
  - `General.svelte` and `settings.css`;
  - the core's `GeneralBody.agent` (`derive/settings.rs`);
  - its six said-lines, as provisional rows of `settings-general` in `spec/machine.json`;
  - `windows-gallery.mjs`: the card, its Copy, Read it first and the switch, in both
    themes (6,579 checks pass).

## 7. The demo (cloud, real agent)

`bash dev/cli/demo.sh <out>` runs **Claude Code headless** (`claude -p`, six turns) with
only `shroom`, `curl` and `sleep` allowed, against:

- the real `worker.js` over an in-memory space (`worker/dev/serve.mjs`), seeded by
  `crates/shroom-cli/examples/demo_space.rs` with seven invented videos;
- shroom.am's real Worker over a fresh site build (`dev/cli/site.mjs`), serving `/cli`;
- the control-socket stand-in.

`python3 dev/cli/transcript.py <out>/turn-*.jsonl` prints the run as a transcript.

The final run, as the agent did it:

1. It read `/cli`, ran `shroom --version`, `cleanup-candidates --json` and
   `link "login bug repro" --json`. It answered with the link and a table of three
   candidates, said views aren't counted yet, and deleted nothing.
2. Asked to delete "the old demo for the prospect", it dry-ran it, showed the title, date
   and length, and asked "Delete it for good?".
3. On "Yes", it ran `delete <id> --yes --json`.
4. Asked to invite Sam Lee, it made the link and handed it over once. It said it hadn't
   sent it anywhere and that anyone holding the link can join, and it named the invite's
   id for a cancel.
5. Asked to record, it got `recording_off`. It told the person where the switch is and
   tried nothing else.
6. After "I turned it on", it recorded, stopped, returned the new link, and listed the
   three newest videos.

The first runs found two snags, both fixed:

- Title words had to be contiguous, so "login bug repro" missed. They now match in any
  order.
- The agent chained `||` and `;`, and its tool stopped each chain for approval. The guide
  now says one command per call.

### The batch demo

`bash dev/cli/demo-batch.sh <out>` uses the same setup over five turns and adds two
stand-ins: `directory`, the person's team directory (invented people), and `send-mail`,
the person's mail tool, which only writes `outbox.jsonl`. The final run:

1. Asked to invite the engineering team and email each person their own invite, it read
   `/cli`, looked the team up, and ran `invite --batch` with all five names and no
   `--yes`. shroom made nothing and answered `needs_confirmation`: four would get a link,
   and one was already a member. The agent showed the four and asked.
2. On "all of them except Dana, she's on leave", it ran `invite --batch` for the other
   three with `--yes`, then mailed each of them their own link, one message each. The
   outbox holds exactly those three messages.
3. Asked for a digest of the last ten days, it ran `list --mine --since 10d --json` and
   listed the two videos with their links.
4. Asked to delete everything older than 200 days, it ran `cleanup-candidates
   --older-than 200`, dry-ran deleting both, and asked "Delete these two for good?".
5. On "yes", it deleted both in one call.

The first run is why a batch invite asks first (§3b). It skipped showing the list,
invited and mailed all five, and cancelled the invite of the one on leave only after her
email had gone.

## 8. In the spec (provisional)

- **N62** is in `spec/machine.json` as a provisional ledger row: the command, its two
  doors, the switch, the card, and shroom.am/cli. It amends N36 and N59.
- **`settings-general`** gains the card's six said-lines, each `provisional` and citing
  N62.
- **No new states or edges.** Each recording ask takes the edge its click takes:
  - `config → countdown` for start;
  - `paused → recording` for resume;
  - `paused → stopping` for stop;
  - the tray's arm for pause.
- **Laws:**
  - 3: a stop makes the link on the person's ask, relayed by their agent.
  - 4: changes wait mid-take.
  - 10: nothing is copied but by Copy's click; a stop's link is returned, not copied.
  - 11: the command never prompts; it refuses with a code.
  - 13: no secret on argv or in output. The one exception is an invite link, by Bagrat's
    call, and the guide tells an agent to hand it over once.

## 9. Calls made on my own judgment (decided X because Y)

- **Decided: the device key is the only door**, because every Mac has one and it keeps the
  Cloudflare token out of the command entirely. A token Mac that never enrolled gets
  `not_set_up` ("open shroom once").
- **Decided: a Unix socket for the app**, because it is this user's only and answers
  back. Rejected:
  - the URL scheme: any page can fire it, and nothing comes back;
  - localhost HTTP: reachable beyond this user;
  - XPC with signature checks: heavy for a self-signed app.
- **Decided: the switch gates all four recording asks**, not only start, because a stop
  makes a link (law 3) and pause/resume act on someone's live screen.
- **Decided: start is the popover's Start, the countdown included**, because the
  countdown and the tray make an agent's recording visible.
- **Decided: a member's removal asks first, like a delete** (`--dry-run`, `--yes`),
  because it can't be undone from the command.
- **Decided: the app answers in codes and the command says the words**, because the
  core's said-lines are the spec's, and the command's words live with the command and its
  guide, as the installer's do.
- **Decided: the install line edits the login profile only when `~/.local/bin` isn't on
  PATH, with a marked line `--uninstall` removes**, because "always installed" must work
  in a new Terminal without admin rights.
- **Decided: no MCP server now**, because a CLI covers coding agents at a fraction of the
  tokens. A thin `shroom mcp serve` over the same binary can come later for shell-less
  clients.
- **Decided: no auto-install of the skill** into `~/.claude/skills`, because the prompt
  door needs nothing installed. `/.well-known/agent-skills/` can come once the draft RFC
  settles.

**Left for Bagrat when he reviews:**

- the card's words;
- whether invites should also be cancellable from the card (today it's Members);
- the profile edit on install.

## 10. What's in the PR

- **`spec/machine.json`:** N62 (provisional), and the card's said-lines; `MACHINE.md`
  regenerated.
- **`crates/shroom-core`:**
  - `Event::Cli` and `Effect::Cli`;
  - `Prefs.agent_recording`;
  - General's `AgentCard` and its actions;
  - tests in `tests/cli.rs`.
- **`app/`:**
  - `runtime/control.rs` (the socket);
  - the clipboard and URL wiring;
  - `General.svelte`, `settings.css` and `types.ts`;
  - the windows gallery's checks.
- **`crates/shroom-cli/`:**
  - the binary, with `batch.rs` for many at once;
  - tests: unit tests, plus `tests/space.rs`, end to end on the real `worker.js` (list,
    find, rename, delete, invite, cancel, remove, a team invited in one call and run
    again, many videos at once);
  - dev-only examples `demo_space.rs` and `fake_app.rs`.
- **Packaging:**
  - `tools/release-build.sh` and `tauri.release.conf.json` (the binary in the bundle);
  - `installer/install.sh` (`link_cli` and `unlink_cli`) and its tests;
  - the cask's `binary` stanza and its test.
- **`site/`:**
  - `cli.md` (the guide);
  - the `/cli` and `/cli.md` routes;
  - `/agent`'s pointer;
  - `test/cli.test.mjs` and the `worker.test.mjs` cases.
- **`dev/cli/`:** `demo.sh`, `demo-batch.sh` with its stand-ins (`directory`,
  `send-mail`), `site.mjs` and `transcript.py`.

**Merging this PR deploys `/cli` to shroom.am** (site.yml). It stays a draft until
Bagrat says so.

## 11. From the POC to the build (2026-10-18)

Bagrat asked for the POC to become the real build the same day. What that added:

- **The spec first.** N62 written for the build, with the command line as a surface of the
  machine and the command's start as an edge (`idle-ready → countdown`); DECISIONS B215–B219
  (the door, the socket, PATH, many at once, the guide); spec tests committed before the
  code (`crates/shroom-core/tests/cli.rs`: every ask in every moment answered once as its
  click; `crates/shroom-cli/tests/flavor.rs`, `cleanup.rs`, `voice.rs`).
- **The command follows the app it ships in.** Its folder and the app it opens come from
  its bundle's identifier, so a dev build's command reads `~/.shroom-dev`. Its binary is
  `shroom-cli`, as in the bundle, so it never shares a name with the app's binary.
- **Cleanup candidates** read the page's own suggestions (the cleanup proposal's route)
  where the page makes them, and age alone where it doesn't.
- **Every line it says** is in `crates/shroom-cli/src/say.rs`, for ratification.
- **The dev build carries the command**, so the full e2e installs it with the real install
  line and runs a command leg on the test account (`tests/e2e/cli-leg.mjs`).
- **The cask** links the command only for a release whose manifest says it carries it: the
  tap renders `main`'s cask for the latest release, and an unconditional `binary` line
  would have broken `brew install` until the first such release.
- **A code review** found the rest, each fixed with a test first: dry runs that changed
  things (`invite`, `rename`, `invite cancel`), an invite id that could become a path
  (`../members/<id>`), a removal that failed when run again, `--from` items of the wrong
  shape dropped quietly, the invite cap counting people already in, a stop's edge misnamed,
  the space's refusals in the wrong words.
- **Two things outside the command, met on the way:** the release build's floor probe
  read the string `library` as a macOS 26 selector (it joins its known names), and the
  Settings drive still expected ⌘Q to quit with Settings in front, which B145 changed.
