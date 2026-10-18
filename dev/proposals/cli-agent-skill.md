# shroom on the command line, for coding agents (a proposal and a POC)

**Status: proposal + POC, for Bagrat (2026-10-18). Nothing here is ratified.** Branch
`claude/cli-agent-skill`, draft PR. New features, product copy and spec rows are his call.

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
`Use shroom on my Mac: read https://shroom.am/cli and follow it.` and [ Copy ].

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
                          ├──▶ the space's page  POST /_shroom/api/library · /mint (manage) · /gone · GET /members
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
- **Quiet while recording (law 4).** A rename or delete refuses with `take_open` while
  `status.json` says a take is open.
- **Recording goes through the app.** The CLI writes one JSON line to
  `~/.shroom/control.sock` and reads one line back. The folder is 700, and the app would
  check the peer's uid. If the app isn't running, `record start` opens it (`open -g -b
  am.shroom`) and waits up to 15 s. The ops are exactly the popover's: start (with its
  countdown), pause, resume, stop. **The app side is not built**: the app runs only on a
  Mac. The demo uses a stand-in, `crates/shroom-cli/examples/fake_app.rs`.
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
| `shroom record start\|pause\|resume\|stop` | asks the app; `stop` returns the new link | CLI side built; app side proposed |
| `shroom guide` | prints the guide (the same text as shroom.am/cli, built in) | built |
| `shroom invite [--name N]` · `shroom invite cancel <id>` · `shroom members remove <id>` | invites and members | designed, not built (§9 Q3) |
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

- **The cleanup hook.** The sibling cleanup-score proposal (view analytics) is the source
  for `views`, `last_viewed` and `score`. When it lands, `cleanup()` in
  `crates/shroom-cli/src/main.rs` ranks by `score` and `signals` becomes its name. The
  JSON shape doesn't change. The guide tells an agent that `null` means *not known*,
  never zero (the first demo run read it as "none viewed").

## 4. Getting it on PATH

The CLI ships **inside the bundle**, at `shroom.app/Contents/Resources/bin/shroom`, so it
is signed with the app and updated with it. It never drifts from the app.

1. **No install at all.** The guide tells the agent to try `shroom`, then the full bundle
   path. Nothing touches the machine.
2. **Homebrew.** The cask (B175) gains `binary "#{appdir}/shroom.app/Contents/Resources/bin/shroom"`.
   It is linked into brew's bin, with no sudo.
3. **The install line.** `install.sh --cli`, or a step in the agent's install flow, links
   it into `~/.local/bin`, only when asked ("never silently mutate the machine").
4. **Later, a Settings switch** ("Use shroom from Terminal"), as VS Code and Docker do.

Recommendation: (1) and (2) at once. (3) only on an explicit flag. (4) waits for demand.

Packaging is not built: `tools/release-build.sh` would build and sign `shroom-cli` into
`Resources/bin`. There is one release-only snag: the CLI's debug-only flags (`--home`,
`--space-base`, `SHROOM_HOME`) are compiled out of a release build, as the app's dev
flags are fenced to the dev flavor.

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

## 6. In the app: a prompt for your agent (design first)

The board is on the canvas: **“Settings — General, with a prompt for your coding agent
(a proposal, for Bagrat)”**, below the cleanup boards in the Library and Settings column.

- A new card, **Your coding agent**:
  - the prompt in a code well with [ Copy ], which reads "Copied ✓" after the click (law
    10);
  - **Read it first**, linking shroom.am/cli;
  - the line "Paste it into your coding agent. It can find, share and tidy your videos,
    and asks you before it deletes or records."
- It mirrors the landing's Coding agent tab (N59).
- The board also draws the Help row as the app has it today (B180), which the old
  "Settings — General" board predates.
- No code yet. It waits for his look at the board.

## 7. The demo (cloud, real agent)

`bash dev/cli/demo.sh <out>` runs **Claude Code headless** (`claude -p`, four turns) with
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
2. Asked to delete "the old demo for the prospect", it ran `delete <id> --dry-run --json`,
   showed the title, date and length, and asked "Delete it for good?".
3. On "Yes", it ran `delete <id> --yes --json`.
4. It ran `record start`, `sleep 5`, `record stop`, then `list --limit 3`, and returned the
   new link.

The run took 14 steps over 4 turns (about 35 s). The first run found two snags, both
fixed:

- Title words had to be contiguous, so "login bug repro" missed. They now match in any
  order.
- The agent chained `||` and `;`, and its tool stopped each chain for approval. The guide
  now says one command per call.

## 8. Spec sketch (provisional, for `spec/machine.json` once ratified)

- **N62 (provisional) — the command line.**
  - `shroom` acts for the person, from a terminal or their agent.
  - Reads and Library writes (rename, delete) go through the space's page under the
    Mac's device key, app running or not.
  - Recording goes through the running app's control socket, and is opened if needed;
    each op equals its popover action, countdown included.
  - It never reads the token.
  - A delete needs `--yes`; without it, nothing changes (exit 7).
  - Writes refuse mid-take (law 4).
  - Its messages are said-lines (law 15); its JSON codes are a contract.
  - Edges:
    - `cli.record.start` → the same edge as popover Start (`idle → countdown`);
    - `cli.record.stop` → popover Stop (`recording|paused → stopping`; its link is
      returned to the caller instead of copied: law 10 holds, nothing auto-copies);
    - `cli.delete` → Library Delete… (the tombstone first).
- **N63 (provisional) — "Your coding agent"** in Settings → General: the prompt, [ Copy ] →
  "Copied ✓", "Read it first" → shroom.am/cli, the line under it.
- **Landing / shroom.am:** `/cli` beside `/agent` (N59's agent door), and `/agent`'s
  one-line pointer.
- **Laws touched:**
  - 3 ("only you make a link"): `record stop` makes a link on the person's ask, relayed
    by their agent.
  - 10: nothing auto-copies; the CLI prints, never copies.
  - 11 ("no dialog to an empty chair"): the CLI never prompts; it refuses with a code.
  - 13: no secret on argv or in output, except the invite link (Q3).

## 9. Open questions for Bagrat

1. **Can an agent start a recording with no click?** The POC lets it: `record start`
   shows the 3-2-1 countdown, then records. The POC in the archive only armed the
   recorder, and a human click started it. Options:
   - (a) as built;
   - (b) `record start` opens the popover, ready, and the person clicks Start;
   - (c) a Settings switch "Let my coding agent start recordings", off by default.

   Recommendation: (a), because the countdown and the tray make it visible. (c) is the
   fallback if it feels too much.
2. **Mid-take writes.** Rename and delete refuse while recording (law 4, conservative).
   Is a rename of a *past* video mid-take fine?
3. **Invites from the CLI.** An invite link is a secret until used. Printing it puts it in
   the agent's transcript, which goes to the agent's provider. Options:
   - (a) don't offer invites in the CLI;
   - (b) print it only with `--show-link`;
   - (c) the app shows the link, and the CLI only opens Settings → Members.

   Recommendation: (c) for v1.
4. **The card's words and place.** "Your coding agent" in Settings → General, or a line in
   the Library's footer, or both?
5. **On PATH.** The bundle plus the Homebrew `binary` stanza now, and `install.sh --cli`
   only on a flag. OK?
6. **MCP.** Not proposed for now: the CLI covers coding agents. A thin `shroom mcp serve`
   over the same binary could come later for Claude Desktop and ChatGPT.

## 10. What's in the PR

- `crates/shroom-cli/`: the binary (`src/main.rs`, `out.rs`, `home.rs`, `space.rs`,
  `app.rs`).
  - Tests: 10 unit tests, plus `tests/space.rs`, which runs end to end on the real
    `worker.js` (skipped without node).
  - Examples: `demo_space.rs` and `fake_app.rs`, dev only.
- `site/cli.md`, the `/cli` and `/cli.md` routes (`site/worker.js`, `site/build.mjs`,
  `site/wrangler.jsonc`), and `/agent`'s pointer line. Tests: `site/test/cli.test.mjs`,
  plus cases in `worker.test.mjs`.
- `dev/cli/`: `demo.sh`, `site.mjs`, `transcript.py`.
- **Not in it:**
  - the app's control socket;
  - the Settings card's code;
  - packaging into the bundle;
  - the cask stanza;
  - spec rows.

  All wait on §9.

**Merging this PR deploys `/cli` to shroom.am** (site.yml). It stays a draft until
Bagrat says so.
