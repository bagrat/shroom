# Agent runs — speed, cost, and what to tune

shroom is built largely by multi-agent runs (workflow scripts: an agent per step or per
lane, reviewers after). This file is where their speed is tracked and their settings are
tuned. **The aim, in Bagrat's order: speed first, then cost** — and never by quietly
skipping a check.

**The standing note.** After every multi-agent run:

1. Measure it: `python3 dev/agent-run-times.py <the run's transcript dir>`.
2. Add its row to [the log](#the-log) — what the run was, how long, the model's share, and
   what was set differently from the run before.
3. If the numbers point at something, change **one** thing for the next run (a setting, the
   script's shape, a brief) and write it in the row, so the next measurement says whether
   it helped.
4. Keep [the settings table](#settings-in-force) true.

A change that didn't help is still a row: it stops the next session trying it again.

## Where the time goes

First measured 2026-10-13, over the night's six runs (23 finished agents, the bug-fix batch
after the from-scratch preview test). The split is read from each agent's transcript: the
wait before the model's message is the model's, the wait before a command's result is the
command's.

- **About half and half.** Model 8.1 h, commands 6.6 h: the model's share was 55%.
- **Fix and review agents are mostly model time**: 60–85%. They take 80–250 turns at 7–17
  seconds a turn. What drives the seconds per turn (context size, effort, how much is
  written) is not yet measured.
- **Merge, suite and integration steps are mostly waiting**: 60–75% commands. The Rust
  suite is about 6.5 minutes, each VM lane 5–7, an app build a few more — and every step
  re-ran them.
- **Commands of five minutes or more are few**: six at most in one agent, none in most.

## Levers for speed, in the order the numbers suggest

1. **Fewer and faster model turns** — the larger half.
   - Fast mode (`/fast`: the same model, faster output, a higher price). Whether workflow
     agents inherit it is unverified: measure one step with it and one without.
   - Effort per step. A workflow call takes `effort`; so far every agent inherited the
     session's. Mechanical steps (a merge without conflicts, running the suites, bringing
     documents in line) can run lower; fixes that need a root cause, and reviews, stay high.
   - Narrow briefs: one concern per agent. A step given nine loose ends carries all nine in
     its context for every turn.
   - One command that does five things beats five round trips.
2. **Don't re-run everything at every step.** Per step: the tests near the change. The
   full suites and both VM lanes: once in the closing step and once in the review. A break
   is then found later, which is the price.
3. **More in parallel.** Lanes that touch different parts run at once in separate worktrees
   (the first night: three lanes in under an hour). What holds the rest in a line: one
   build directory for the app (the disk is tight), two VM slots, one screen. The hosted
   Mac (`dev/mac`, the cloud skill's §5) lifts the last two for verification: every branch
   can have its own Mac.
4. **The subagent prompt cache.** The docs give a subagent's cache five minutes by default
   (the main session's is longer). After a longer command the agent reads its whole context
   again, cold. With so few long commands this is small for speed; it is a cost item.
   `subagentPromptCacheTtl: "1h"` (or `CLAUDE_CODE_SUBAGENT_PROMPT_CACHE_TTL=1h`) removes
   it. Not set yet; Bagrat's to switch.
5. **The auto-compact window is not a speed lever.** A smaller window makes a turn slightly
   quicker, but each compaction is a pause, and the agent re-reads what it forgot. It is a
   mild cost lever (cost scales with context size), paid for in lost detail.

## Settings in force

| Setting | Value | Since | Note |
| --- | --- | --- | --- |
| Auto-compact window, Opus 5.5 | 350k | 2026-10-13, Bagrat (`/autocompact`; 300k and 500k tried the same night) | Saved per model in the user's settings. Whether subagents take it is not documented. |
| Subagent prompt cache | the default (five minutes) | — | Candidate: `1h`, see lever 4. |
| Effort in workflow scripts | inherited from the session, except one-turn mechanical steps (`effort: 'low'`, `model: 'haiku'`) | 2026-10-13, the agent-page runs | Measured there: 30 such agents under a minute together. Still a candidate on longer mechanical steps (a merge, running the suites), see lever 1. |
| Fast mode | not recorded | — | Record it with the next run. |

## What the docs say about compaction

Looked up 2026-10-13 (code.claude.com/docs: sub-agents, model-config, settings-reference,
prompt-caching, context-window, costs, changelog). "Not documented" means exactly that.

- Subagents compact on their own, under the same conditions as the main conversation.
- `/autocompact <N>` sets the token count at which compaction runs: 100K to 1M, capped at
  the model's window. `CLAUDE_CODE_AUTO_COMPACT_WINDOW` beats the `--autocompact` flag,
  which beats the saved value.
- A window per subagent: the changelog (v2.1.296) names `autoCompactWindow` in an agent
  definition's frontmatter; the reference table doesn't list it yet. Nothing is documented
  for workflow-script agents. `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` applies to subagents.
- A compaction is one more request: the same history plus an instruction to summarise. A
  warm cache makes that read cheap; after a gap longer than the cache's lifetime it is
  read at full price. The summary's size is not documented. Afterwards up to five recently
  changed files are read again, and the cache rebuilds from the shorter history.
- No break-even figure is documented. It pays when what is dropped is no longer needed:
  compact at a natural break, not mid-step.

## Rules that protect a run

Each of these cost time on 2026-10-13 when it was broken.

- **Never message a running workflow agent.** It doesn't reach the agent: it starts a
  second copy in the same tree. There is no way to steer a running agent — every rule goes
  in the brief before launch. A rule found mid-run is enforced from outside, or waits.
- **Agents run commands in the foreground.** A background job's completion notice can also
  start a second copy.
- **A resumed run replays only the unchanged start of its script.** Once one call's brief
  differs, everything after it runs again. Put finished lanes into the script as constants
  rather than relying on the replay.
- **One agent per working tree at a time.** Parallel work goes in separate worktrees, and
  only one tree builds the app.
- **Never share a Cargo target dir between worktrees.** Cargo names a workspace crate's
  artifacts by a hash of its path *relative to the workspace root*, so every worktree's
  `libshroom_core-<hash>` is the same file: lanes overwrite each other's builds, and a build
  whose own sources look older takes another tree's artifact as fresh. A test then runs, or
  a check fails, against code that isn't in the tree (2026-10-18: a macOS check linked another
  lane's half-changed library). Each worktree builds in its own target dir; the cost is one
  dependency build per lane, a few minutes. Before trusting a result from a tree that ever
  shared one, touch its sources.
- **To change the rest of a running run**, wait until the tree is clean between two steps,
  stop the run, edit the script, start it again.

## The log

Wall-clock is first line to last; agent-minutes add up every agent, so parallel lanes make
them larger than the clock.

| Date | Run | Shape | Clock | Agent-min | Model's share | Set differently | What it showed |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-10-13 | bug-fix batch, first start | 3 fix lanes in parallel, reviews behind each | 1 h 03 | 157 | 68% | — (baseline) | Stopped by the lead after messaging its agents; one agent's unreported run was lost. |
| 2026-10-13 | page icon | 1 fix, 1 review | 0 h 19 | 19 | 86% | — | Small page-only change: almost all model time. |
| 2026-10-13 | bug-fix batch, restarted | finish + reviews, then the Dock fix and its review | 2 h 54 | 133 | 67% | finished lanes hardcoded | — |
| 2026-10-13 | popover lane | 1 fix, 2 reviews in parallel, refix, recheck | 3 h 19 | 228 | 63% | own worktree, VMs for proof | Two rounds: the first review failed it. |
| 2026-10-13 | integrate lanes | 6 steps in a line, main tree | 3 h 22 | 202 | 42% | every step re-ran suites | Waiting dominates once steps re-run the suites. |
| 2026-10-13 | merge popover and close | 7 steps in a line, main tree (merge, launch, capture proof, loose ends, close, review, refix) | 12 h 51 | 771 | 28% | as above, plus VM lanes per step | Waiting was 72%: every step re-ran the suites and the VM lanes (5–10 min each; one step ran fourteen such commands and took 195 min). The steps also grew: each found and chased things beyond its brief. For the next run of this kind: the full suites and lanes once, at the close and in review; a time box per step in the brief. |
| 2026-10-13 | agent page: review | 8 read-only reviewers in parallel, words only | 0 h 19 | 69 | 98% | first run with no build and no suite | All model time: 11–34 turns an agent at 17–45 s a turn, most of it writing the report. |
| 2026-10-13 | agent page: drafts and judges | 3 drafters, then 3 judges beside 30 one-turn summaries and a grader | 0 h 27 | 80 | 99% | the 30 summaries on `model: 'haiku'`, `effort: 'low'`, `agentType: 'Explore'` | A lowered step costs next to nothing: 30 agents, one turn each, under a minute together. The clock is the drafters and judges. |
| 2026-10-13 | agent page: verify | 3 verifiers beside 36 summaries and a grader | 0 h 13 | 42 | 99% | as above | One grader reading 36 outputs in a single turn: 4 minutes. |
| 2026-10-13 | agent page: one choice | 32 summaries, a grader, one last reader | 0 h 07 | 13 | 99% | as above | The measurement couldn't separate the two wordings: more repetitions, not more agents, is what it needed. |
| 2026-10-13 | canvas design pass, built | 9 subagents from the main session (Agent tool, no workflow): 1 inventory, 2 frame drawers, 3 build lanes in their own worktrees (setup window · Library and Settings · page and site), 2 audits, 1 frame updater; the main session did the core and the popover | 1 h 42 | 195 | 61% | build lanes in worktrees sharing one cargo target; briefs named their own files, theme tokens and checks | Drawing, audits and frame updates are all model time and short (4–13 min each). The build lanes are where the clock goes: the setup lane 75 min, 46 of them commands — its whole gallery run (5 min a run, the popover section included, which wasn't its own) again and again, and a shared target dir another lane's half-merged core kept breaking. The lanes also had to be told mid-run about shared files (the theme, the gallery); a lane's base should be the branch's head at spawn, not `main`. For the next run of this kind: give a build lane a way to run only its own gallery section, and a private cargo target. |
| 2026-10-14 | docs rewrite (ARCHITECTURE, WORKER-INTERFACE) | 2 general-purpose subagents in parallel, no reviewers; the lead checked cited § numbers | 0 h 17 | 32 | not measured (Agent tool, no journal) | a hard size cap (60 KB) and "keep every § number" in the brief | Both landed just under the cap on one pass; 88 and 75 tool calls. Most turns went to checking the old text against the code. |
| 2026-10-16 | public snapshot: scrub quotes, links and citations | 3 general-purpose subagents in parallel, one per file set (DECISIONS · the spec · history, dev and other docs), no reviewers; the lead ran the tripwire and the suites | 0 h 05 | 13 | not measured (Agent tool, no journal) | each brief named the files it owned, the exact grep that must come back empty and the tests to run | All three came back clean on their own grep in under five minutes; the cross-file leftovers (`?ref=hn` in the site, the migration notes' copies of spec quotes) were found by the lead's whole-tree tripwire, so a whole-tree check after parallel file-scoped lanes is the step to keep. |
| 2026-10-18 | editor (#61): wave 1 | 2 build lanes in parallel in worktrees (workflow): the core's editor track; the media track (peaks, the cut, the window's protocol) | 0 h 41 | 73 | 84% | briefs pointing at one shared contract file (the edit list's shape, the ops, the effects) instead of each restating it | Model-bound: 233 and 159 turns at 9–10 s a turn; commands only 12 of 73 min. |
| 2026-10-18 | editor (#61): wave 2 | the same two lanes again for the after-save flow (B197) | 0 h 42 | 65 | 67% | — | The core lane's commands rose to 18 of its 42 min: it ran the core suite (about 40 s plus builds) after each change. |
| 2026-10-18 | editor (#61): tracks from the main session | 3 Explore mappers (3–7 min), then 3 lanes by the Agent tool: the webview (28 min), the Worker and page (75 min), the app shell (185 min over two rounds) | about 4 h | about 300 | not measured (Agent tool, no journal) | the Mac-only lane type-checked by a cross-compile from Linux (stub headers) before each hosted-Mac run | The app shell was the long pole: each hosted-Mac round is 15–20 min, and the cross-check caught every compile error before a round was spent. |
| 2026-10-18 | editor (#61): adversarial review | 5 finders by area (Worker, upload, core, edit, privacy), then one verifier per finding, pipelined (workflow) | 2 h 07 | 234 | 89% | pipelined: an area's verifiers start as its finder ends | 30 findings: 20 held up, 3 were refuted, and 7 had already been fixed by a build lane that landed while the review read an older snapshot, so a quarter of the verifiers' time went on fixed code. Finders took 11–26 min, verifiers 2–8 min each. |

Next to try, one at a time: suites once per run instead of per step; `effort` lowered on
the closing step; fast mode on one comparable step; the one-hour subagent cache. For
words-only runs (reviews, drafts): they are all model time and the turns are long because
the reports are — ask for shorter reports (findings only, no walkthrough) and see what
the clock does.

## The first night, per agent

```
agent                         total  model   cmds turns s/turn  commands of 4 min or more
fix:favicon                     16m    14m     2m    93    9.0
fix:rename                      56m    43m    12m   201   13.0
fix:mark                        29m    18m    11m   112    9.7
review:mark:truth               10m     9m     2m    51   10.2
fix:wizard (finish)             20m    10m    10m    81    7.3
review:rename:truth             22m    14m     8m    60   13.5
review:favicon:truth            12m    10m     1m    54   11.4
review:wizard:truth             15m    13m     2m    95    8.1
fix:dock                        25m    18m     7m   105   10.4
review:dock:macos               39m    24m    15m   117   12.3
fix:surfaces                    73m    50m    22m   235   12.8
review:surfaces:macos           30m    22m     7m    83   16.2
review:surfaces:spec            36m    23m    13m    81   17.0
refix:surfaces:1                47m    28m    19m    96   17.3
recheck:surfaces:1              44m    21m    23m    82   15.7
integrate                       27m     8m    19m    94    5.1  5
polish:page                     13m    10m     3m    57   10.3
polish:typed                    33m    14m    19m    44   18.7  5
polish:lanes                    62m    30m    33m   141   12.6  7 5
version+suites                  31m    11m    20m    75    8.5  5 5
review:integration              35m    13m    23m   100    7.7  5 5
merge:popover                  103m    28m    75m   155   10.6  6 6 5 5 5 4
```

## How to measure

```bash
python3 dev/agent-run-times.py <the run's transcript dir>
```

The Workflow tool prints the transcript dir when a run starts (`…/subagents/workflows/wf_…`).
The script reads the run's `journal.jsonl` and each agent's transcript, and changes nothing.
Several dirs give a total across them.
