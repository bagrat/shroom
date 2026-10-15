# shroom — working agreement

shroom is a least-resistance, self-hosted (BYO-storage) screen-recording tool — "Loom UX
without Loom pricing, never a SaaS we operate." The repo is **in the v1 build** (since
2026-10-01): the agent-first POC shipped (v0.1.16) and was archived, the rethink chose the
stack (Tauri v2 shell + Rust capture core) and designed the UX, and the build is under way.

## The lay of the land

At the root: this file, [`README.md`](README.md) and the licences. The spec
lives in `spec/`; the working docs live in `docs/`.

- **[`spec/machine.json`](spec/machine.json)** — **the spec** (DECISIONS B133). Every state, edge,
  rule and said-line of v1, the decisions ledger, the 16 standing laws and the scaffold
  flags; whatever isn't ratified is `provisional` and cites its row. Sessions edit it in a PR;
  Bagrat never does by hand. Read it as [`spec/MACHINE.md`](spec/MACHINE.md), generated from it
  (`tools/render-machine-md.py`; CI fails when it's stale). The old text, `UX-STATEMACHINE.md`,
  and its map are frozen in `history/spec/` — `SM:<line>` citations mean that file.
- **[`docs/DESIGN.md`](docs/DESIGN.md)** — **the look.** The
  [shroom UI](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg) design canvas, with every
  surface drawn as a board. It covers how a session opens, edits and publishes it, and
  which code builds each board.
- **[`docs/TESTING-PLAN.md`](docs/TESTING-PLAN.md)** — how the build proves itself: the three
  oracles, the heartbeat, the layers, the tripwires, and the pre-approved machine changes.
  Beside it, [`docs/RELEASING.md`](docs/RELEASING.md) (cutting and checking a release) and
  [`docs/CHECKPOINT.md`](docs/CHECKPOINT.md) (Bagrat's from-scratch sitting).
- **[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)** and **[`docs/WORKER-INTERFACE.md`](docs/WORKER-INTERFACE.md)**
  — how it's built: the crates, the app, the seams, and the Worker's routes and stored
  layout, as they are today.
- **[`docs/DECISIONS.md`](docs/DECISIONS.md)** — every in-frame decision: B1–B120 one line each
  (their full rows in `history/decisions/`), B121 on in full; then
  [the deviations](docs/DECISIONS.md#deviations-from-the-spec) — where the build departs from
  the spec, for ratifying, cited as DEVIATIONS #n; then the copy table the Worker's tests read.
  **[`docs/BUILD-STATUS.md`](docs/BUILD-STATUS.md)** — where the build is and what waits on Bagrat.
- **[`docs/PRINCIPLES.md`](docs/PRINCIPLES.md)** — the job, the north star and the
  non-negotiable principles every call is weighed against. Bagrat's living brainstorm
  behind them is private (see *Public and private*); propose changes, don't rewrite them unasked.
- **[`history/`](history/README.md)** — the decided record, frozen: the research behind the
  stack decision ([`history/RESEARCH.md`](history/RESEARCH.md) R1–R11), the designs as first written,
  every decision row in full, the build logs, built proposals, and audits. Read it for the
  why; never edit it. **[`history/POC-LEARNINGS.md`](history/POC-LEARNINGS.md)** is every
  hard-won finding from the POC: the validated capture recipe, the ffmpeg-avfoundation
  audio bug + native-audio fix, the full macOS TCC/permissions saga (and the
  ScreenCaptureKit endgame), the HLS upload/publish architecture, the Cloudflare/wrangler
  empirical map, distribution, testing, and an honest agent-first retro. **Read the
  relevant section before designing or debugging anything in its territory — the walls
  documented there were expensive.**
- **The POC's code** — the frozen v0.1.16 reference implementation, removed from the tree
  2026-10-13. It lives in the private **bagrat/shroom-archive** (this repo's history up to
  2026-10-16) at tag `v0.1.5`, under `archived-poc/` (its own CLAUDE.md, SPEC.md,
  TESTING.md, per-package READMEs): attach it (`add_repo`), then
  `git show v0.1.5:archived-poc/<path>` there. Every
  `archived-poc/…` citation in the docs means that tree. Mine it for answers; never restore it.
- **Cloud sessions** — the `shroom-cloud` skill. The environment's setup script **is the
  repo file [`dev/cloud/setup.sh`](dev/cloud/setup.sh)**: the claude.ai Setup-script
  field only launches it, and the SessionStart hook re-runs it every session. A missing
  tool is added there and committed — never by asking Bagrat to edit the environment.

## Build rules (v1)

- **Code lives on `main`**, beside the design record: the local `build/v1` branch was
  merged into it and retired (2026-10-13). It changes only by merged PRs (see *Merge when sure*).
- **The spec wins.** Spec tests are written before each region's code and committed
  separately. When the spec is wrong or unbuildable, write the change to the deviations in
  `docs/DECISIONS.md` (what, why, evidence) for Bagrat to ratify — never edit a test to make it pass.
- **Design first.** Any new or changed UI/UX is proposed on the design canvas before any
  code changes. That means a screen, a state's look, a control,
  or where a line sits. Edit or add its boards, tell Bagrat which, and build once he's seen
  them (or has said to go ahead). The code matches the canvas. A surprise found while
  building goes back to the canvas first. How to read, edit and publish the canvas, and
  which code builds each board: [`docs/DESIGN.md`](docs/DESIGN.md).
- **In-frame design calls** (encode leg, updater, overlay home, scap pinning, min-OS,
  Intel, layout) are decided with a quick probe and logged in `docs/DECISIONS.md` with the
  rejected alternatives. Stack-level changes are Bagrat's.
- **The heartbeat stays green**, and the tripwires in TESTING-PLAN §7 stop the run.
- **Machine changes**: only the ones pre-approved in TESTING-PLAN §9 (M1–M8); anything
  else is proposed and asked first.
- **Agent runs are measured and tuned.** After each multi-agent run, add its row to
  [`dev/AGENT-RUNS.md`](dev/AGENT-RUNS.md) (`dev/agent-run-times.py` prints the numbers)
  and change one thing by what they say — speed first, then cost. Read its "Rules that
  protect a run" before writing a workflow script.
- **Don't push `main` without Bagrat's go**: a push to `main` that touches the site
  deploys shroom.am (`.github/workflows/site.yml` → the `shroom-site` Worker, from
  `site/`) — pushing is a user-facing act. GitHub Pages and the plugin marketplace that
  used to ride on `main` are decommissioned (see `history/proposals/shroom-am-worker.md`).
  Nothing is pushed to `main` directly. A PR merged under *Merge when sure* counts as his
  standing go.
- **Test in the cloud first; never on Bagrat's Mac.** It carries only the official
  release. A change is tested in a cloud session and CI. Only Mac-only code (capture,
  record/encode, the app shell, updater/installer, signing) goes to the hosted Mac, and a
  Tart clone on his Mac is the last resort (TESTING-PLAN §5,
  `dev/proposals/heartbeat-in-vm.md` §3). Every dev-build launcher refuses on his Mac.
- **Spawning side work.** A spawn-task chip only for work that needs a local Mac session.
  When a cloud session is enough, ask Bagrat in the chat and, on his yes, start the cloud
  session yourself.

## How sessions work together

- **One orchestrator.** Bagrat talks mostly to one orchestrator session. It watches every
  live shroom session and talks to them (`send_message`). About every 30 min it sends him a
  recap he can act on without scrolling back. When a session needs him, it sends one push
  notification per need. Several asks come in the order to do them: blocking or
  time-sensitive first, then reviews, then decisions, then checks by hand.
- **Substantial work gets its own session.** The orchestrator spawns it and tracks it
  rather than doing it.
- **Titles stay true.** A session renames itself (`set_session_title` on its own id) when
  its topic changes or gets more concrete, or the title no longer fits.
- **References carry context and links.** A PR, issue or numbered reference (DEVIATIONS
  #49, B125, CHECKPOINT §5, run ids…) gets a few words in italic parentheses:
  [#36](https://github.com/bagrat/shroom-archive/pull/36) *(retire main-account testing)*.
  Make everything that can be followed a link: PRs, issues, commits, CI runs, files on
  GitHub. Session links (`https://claude.ai/code/<session_id>`) go in chat and in
  shroom-private only, never in this repo (see *Public and private*).
- **Spawned sessions attach the repo.** Every `create_session` passes `source_url`
  `https://github.com/bagrat/shroom` and an `outcome_branch` (`claude/<topic>`).
  Without them the new session lands in the app's "Other" section. It also passes
  `permission_mode: "auto"` and `extra_allowed_tools` with
  `mcp__claude-code-remote__send_message`. Left out, the mode is inherited from the
  orchestrator's at that moment, and a child in default mode stalls silently on its first
  report up, waiting for an approval nobody sees. A session whose work is private material
  passes `https://github.com/bagrat/shroom-private` instead and attaches this repo when it
  needs code (that repo's CLAUDE.md says how its cloud setup runs).
- **Blocked children are surfaced.** Each recap checks every live child's `status_bucket`.
  One that reads blocked or waiting on a permission gets a push to Bagrat at once, with its
  session link and the exact thing to approve.
- **Report up; don't wait to be polled.** A session the orchestrator spawned or watches
  messages it (`send_message`) when something that matters changes. That covers finishing,
  opening or merging a PR, failing, getting blocked, or needing Bagrat: a question, a
  decision, a review or merge, or an approval on his Mac. Send one short message with
  context and links, and nothing for routine progress. The orchestrator's id comes from the
  session that spawned it. A session started by hand reports to the orchestrator Bagrat
  names.
- **Merge when sure.** A session merges its own PR without waiting when all of these hold:
  CI is green on the head, it's mergeable, the change is in scope, and nothing in it is
  Bagrat's to decide (a ruling, product copy, a stack change, anything this file reserves
  for him). Otherwise it asks him through the orchestrator.
- **No separate "go" for in-scope work.** A session commits and pushes to its own
  `claude/<topic>` branch, opens its PR, and merges under *Merge when sure* without
  Bagrat's "go" in that session — this file is that go. When something *is* his call (a
  ruling, product copy, a stack change, a machine change, a release, a credential), his
  answer relayed by the orchestrator counts as his: the orchestrator quotes his exact
  words and where he said them (in chat), and the answer settles only the question he was
  asked. What lands in this repo paraphrases it with a date ("Bagrat, 2026-10-15: keep
  the pasted key").
- **Archive when done.** After its PR merges, a session is archived (by the orchestrator,
  or by itself with `archive_session`) only when nothing is left: no open follow-ups, no
  pending runs, no question waiting on Bagrat. Otherwise it stays open and says what's
  left.

## Public and private

This repo is public, and so is everything around it: PR titles and bodies, issues,
comments, commit messages, branch names and release notes.

- **Personal, strategy, pricing, marketing and launch material lives in the private
  bagrat/shroom-private, never here.** That includes PR bodies, issues, commit messages
  and branch names. It covers Bagrat's brainstorm and motivation, the rethink interview,
  the analytics and after-v1 plans, the demo plan and script, his schedule and meetings,
  and details of his Mac or home. Sessions that need it attach it (`add_repo`
  bagrat/shroom-private). Text flows private → public only, and a public doc never
  links into the private repo.
- **No session links, no quoted chat.** No `claude.ai/code` session link or session id
  anywhere in the repo, its PRs, issues or commits. Bagrat's rulings are paraphrased with
  their date, never quoted from chat. Quotes and links go in the chat and in shroom-private.
- **Attribution: no `Claude-Session:` trailer and no session link.** The harness's
  attribution reminder asks for both on commits and PR bodies; in this repo, leave them
  out. A commit ends with the `Co-Authored-By:` line only. A PR body ends with
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)` and nothing after it.
- **Sample data is invented.** Emails in copy, tests and docs use `example.com` (or the
  role addresses on shroom.am); names are made up.
- **The tripwire.** `tools/public-tripwire.sh` (CI's `public` job) fails on session
  links, `Claude-Session:` trailers, personal email addresses and the names of the files
  that moved private, in the tree, the PR's commit messages and its body. Run it before
  you push.
- **PR numbers restarted at #1** on 2026-10-16, when the repo was republished. The
  history up to then is in the private bagrat/shroom-archive: a `#N` or a PR link in a
  doc dated before that means shroom-archive's.

## Principles that carry over from the POC (proven, keep them)

- **Never silently mutate the machine.** Installing tools, `git init`, logins, cloud
  resources — propose the exact command, ask, then run. (The POC allowed itself exactly one
  scoped, documented exception; treat any new exception as a design decision, not a habit.)
- **Secrets stay out of git.** Credentials live in `~/.shroom/` (mode 600), never in the
  repo, never on a command line.
- **Product voice in everything a user sees.** Warm, brief, product concepts only — no
  internal names, no infra nouns, no narration of mechanism. Write the said-line, not the
  mechanism.
- **Mechanism deterministic, judgment above.** The POC's determinism boundary (exact,
  testable scripts under an orchestration layer) is why it stayed debuggable; v2 keeps the
  discipline whatever the stack — core engine exact and seam-testable, AI/judgment optional
  garnish at the edges.
- **Reviewable changes.** One logical change per commit, phrased to revert cleanly.
- **OSS trust posture.** Readable source over opaque blobs, narrow permission scopes,
  no paid certs assumed.
