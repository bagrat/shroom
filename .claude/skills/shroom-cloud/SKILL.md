---
name: shroom-cloud
description: Run, test and show shroom v1 in a Claude Code cloud session (Linux, driven from a phone). Use when Bagrat asks to run the tests, check a change works, see a window, a state or the page ("show me the popover", "what does the removed page look like"), drive the page, run the live Cloudflare tests, check a fix on a real Mac and get a video of it, or asks "what can I test here vs on the Mac".
---

# shroom in a cloud session

Bagrat is usually on his **phone**. He prompts; you run everything. Never hand him a
command to run. Show results as short summaries, and **send visual output as files**
(SendUserFile, `display: "render"`): screenshots of a window's states, a face of the
page. Keep replies short enough to read on a phone.

This skill is dev tooling. The CLAUDE.md product-voice rule covers what the **app, the
page and the installer** say to people. It doesn't stop you naming scripts and tools to
Bagrat here. CLAUDE.md's build rules do hold here: the spec wins, a test is never
edited to pass, and nothing is pushed to `main` — a cloud session's work
goes on its own `claude/…` branch and reaches `main` by a PR, merged under CLAUDE.md's
*Merge when sure*.

Run everything from the repo root. This is v1: the Rust crates, the Worker, the app's
webviews. For the POC's recipes (fake recordings, wrangler login, whisper) see
`git show v0.1.5:archived-poc/dev/cloud/SKILL.md` in the private bagrat/shroom-archive (the POC left the tree 2026-10-13).

**How sure the lists below are.** They were decided on a Mac — each crate's dependency
tree resolved for Linux, the tests read for macOS-only tools, every command run on
macOS with those tools made to fail — so they are *expected* to run here, and the
first cloud session confirms. When a listed command fails for a platform reason, that
is a finding, not noise: fix `dev/cloud/setup.sh` if a tool is missing, or correct this
list if the test can't run on Linux, and commit.

## 0. Check the session is ready

The SessionStart hook (`.claude/hooks/session-start.sh`) prints one status line into
your context: node, rust, ffmpeg, node_modules, playwright, live-test-token. If
something you need shows `no`, run `bash dev/cloud/setup.sh` (idempotent). Two slow
things start **in the background** at session start:

- **Playwright's browsers** — `playwright` may show `installing`. Before a gallery or
  the page drive, run `bash dev/cloud/playwright.sh wait` (it starts the install if
  needed; ~2 min cold). On `failed`, read `~/.shroom-dev/cloud/playwright-install.log`
  and retry with `playwright.sh start`.
- **The crates' sources** — the first `cargo` command of a cold session may sit on
  "Blocking waiting for file lock" or "Updating git repository" for a couple of
  minutes: the workspace pins two macOS-only git dependencies that cargo fetches on
  every platform. Let it finish; it happens once.

If `cargo` isn't found, run `. dev/cloud/path.sh` in that command (Rust may sit in
`~/.cargo/bin`). Don't install things outside `setup.sh`. If a new tool is needed,
**add it to `dev/cloud/setup.sh`**, run it, and commit, so every later session gets it.
That file *is* the environment's setup script: the claude.ai Setup-script field is a
fixed launcher for it (`dev/cloud/README.md`) and the SessionStart hook re-runs it, so
a repo edit reaches the next session even on a warm cache. Never send Bagrat to the
environment settings to add a tool, whatever generic setup-script advice says.

## 1. Tests

```bash
# The crates — everything but the app shell and the tests that record
cargo test -p shroom-core -p shroom-spec -p shroom-testpattern -p shroom-cloudfake -p shroom-storage -p shroom-cloud -p shroom-cli
cargo test --lib -p shroom-encode -p shroom-record -p shroom-update -p shroom-capture
cargo test -p shroom-upload --lib --test faults --test inits --test library --test progress
cargo check --workspace --exclude shroom-app --all-targets   # the Mac-only tests still compile

# The Worker
npm test --workspace worker                   # vitest over a fake bucket
npm run -s test:workerd --workspace worker    # the real worker.js under workerd
npm run -s typecheck --workspace worker

# The app's webviews (no Tauri here): types, then the build
npm run -s check --workspace app
npm run -s build --workspace app

# The landing page, the agent page, the media readers, the spec's readable form
node --test site/test/landing.test.mjs site/test/agent.test.mjs site/test/cli.test.mjs tests/e2e/av.test.mjs tests/e2e/tone.test.mjs
python3 tools/render-machine-md.py --check
```

What a clean run looked like on the Mac at `b4c326d`: the three `cargo test` lines 305,
41 and 31 passed (the second is 34 here — `shroom-capture` keeps one of its eight
tests off macOS), vitest 374, workerd 11, the node
tests 19, the fixtures 425 lines (that check retired with B133: `render-machine-md.py --check` now).

- **Never `cargo test --workspace` here.** `shroom-app` doesn't build on Linux, and the
  tests left out above record through the Mac's encoder or call `ditto`: they fail on
  the platform, not on the change. Name them as owed to the Mac (§4) instead.
- A live test with no token prints `skipped` and passes; that isn't a pass (§3).
- Report as "N passed" or the failing test names plus the assertion. Don't paste the
  whole output.

## 2. Show it

Every window and every face of the page can be rendered here and sent to the phone.

```bash
bash dev/cloud/playwright.sh wait             # the browsers are ready
npm run -s build --workspace app              # the galleries serve app/dist

# The app's windows, every state, light and dark → app/test/gallery-out/
npm run -s test:gallery --workspace app

# The page's faces, light and dark → tests/e2e/gallery-out/
npm run -s gallery:pages --workspace app

# The page driven on the real worker.js over loopback, in Chromium and WebKit:
# play, seek, the waiting faces, the owner's view, a delete. Prints a JSON summary.
npm run -s drive:page --workspace worker
```

- **Which file.** In `app/test/gallery-out/`: `popover-<variant>-<light|dark>.png`,
  `library-…`, `settings-…`, `countdown-…`, and the setup and invite window's states
  under their own names (`welcome-…`, `token-…`, `name-…`, `all-set-…`, `join-…`). In
  `tests/e2e/gallery-out/`: `<face>-<light|dark>.png` — `live`, `uploading`,
  `on-its-way`, `removed`, `unavailable`, `embed-face`, … `ls` the folder and pick; the
  variants come from `cargo run -q -p shroom-core --example gallery` (add `-- library`,
  `-- settings`, `-- members` or `-- region3`), which prints each state's scene as JSON.
- Send the few he asked about, not the folder (there are ~280 app screenshots). Both
  folders are ignored by git. Never commit them.
- **What a screenshot here is.** The real webview code rendered from the real state,
  in Playwright's WebKit — not the Mac app's window. The fonts are Linux's, so text is
  set a little differently and a gallery's size or position checks may fail where the
  Mac's pass; the screenshots are written either way. Say which checks failed and that
  the Mac decides them. The tray, the bubble and the camera preview are native and
  can't be shown here.
- **Does it play?** `drive:page` is the answer: it reports per engine. If an engine
  can't decode here, say so and name the check; don't call the page broken on that.
- **A space to poke at.** `sleep 600 | node worker/dev/serve.mjs --port 8787` (in the
  background; it serves until its input closes) is the real Worker over an in-memory
  space: `curl -s http://127.0.0.1:8787/v.myshroom.workers.dev/_shroom/ping`. Its
  header lists the `/_dev/…` knobs.

## 3. Cloudflare, live

v1 has **no login here**: no wrangler, no OAuth, nothing to deploy from a cloud
session. Everything above runs against fakes (`crates/shroom-cloudfake`, the Worker's
fake bucket) and never reaches the network.

Both live reaches run on the dedicated **test account** (TESTING-PLAN D3), in GitHub
workflows through the `cf-e2e` environment's secrets — a cloud session holds no token for
it:

- The **full e2e** (§6) on a hosted Mac — start it, never its pieces, from here.
- The **test-account suite** (`tests/e2e-live`: the run token's powers, the gates, the
  name, wiring, and the fake's contract with Cloudflare) in `e2e.yml`, started with
  **Run workflow** on a branch (a session dispatches it with its GitHub tools). Each of
  its tests takes a ticket and waits its turn on the account, as the full e2e does (§6).

Main-account testing (`SHROOM_LIVE=1`, `shroom-test-…` names on Bagrat's own account) is
retired, with its token.

**The environment does carry Bagrat's account token**: `CLOUDFLARE_API_TOKEN` and
`CLOUDFLARE_ACCOUNT_ID`, on the account that hosts shroom.am (the token "Claude Cloud";
Bagrat, 2026-10-14). Cloud sessions may use it for **shroom.am's ops and stats** and
nothing else. That covers the `shroom-site` Worker, its zone, and reading Analytics
Engine and Web Analytics (`dev/stats/collect.mjs`). It is
never a test account: no app, e2e or test run points at it. It is never echoed, logged,
written to a file or committed. Pass it through the environment (`$CLOUDFLARE_API_TOKEN`
in a header), never on a command line as literal text. A change to the account's
settings (enabling a product, a token's powers, DNS) is still Bagrat's to make or approve.

**Our own fetches of shroom.am carry a tag** (DECISIONS B228), so the stats leave them out:
`?ref=test` on anything a session asks of it by hand or in a live drive
(`curl -fsSL 'https://shroom.am/install?ref=test' | …`, `https://shroom.am/?ref=test`), and
`SHROOM_REF=test` in the environment of an installer run against the public releases (its
download is counted on the way). GitHub Actions jobs use `ci` instead. An untagged fetch counts as a person's.

- Never ask Bagrat to paste a token, key or secret into the chat.
- **Never print the environment** (`env`, `printenv`, `set`, `export -p`, a dump of
  `process.env`): a secret may be in it. To see one variable, name it: `echo "$PATH"`.
- **Never from here:** the checkpoint (`docs/CHECKPOINT.md`, on the official release), or
  anything else that reaches his own space, his page or his links; a release or a
  stand-in release; the heartbeat (it records — §4).

## 4. What needs the Mac

None of these run here. **The order** (Bagrat, 2026-10-14 — rely on a Mac as little as
possible; `dev/proposals/heartbeat-in-vm.md` §3): everything that *can* run here does;
only a change to code on this list needs a Mac, and then **the hosted Mac of §5** — its
**E2E full** run is the heartbeat of record. A Tart clone on Bagrat's Mac
(`tests/l5/heartbeat-vm.sh` and the other `tests/l5/` lanes) is the last resort, for what
the hosted Mac can't settle (a timing bar, a 4K display, a long soak) — name it and why,
and let Bagrat start it. **Never** ask him to run a dev build on his own Mac: it carries
only the official release, and every launcher refuses there (`tests/test-machine.mjs`).
Never fake a check.

- **Capture**: the screen, camera and microphone (ScreenCaptureKit, the native mic),
  the permission prompts, the tray, the bubble, the countdown on a real screen.
- **Any recording at all**: the encoder is the Mac's (`h264_videotoolbox`), so no take
  can be made here. That is the tests left out of §1 — `shroom-encode`'s
  `real_ffmpeg` and `unfurl`, `shroom-record`'s `synthetic`, `signature`,
  `interruptions` and `av_sync`, `shroom-devtool`'s `crash`, `shroom-upload`'s
  `init_probe` — and `cargo test --workspace` as a whole.
- **The app**: `shroom-app` (Tauri and AppKit), its dev build (`build:dev`), the drives
  of the real app (`app/test/*-drive.*`, `worker/dev/region6-drive.mjs`), signing.
- **The updater and the installer**: `shroom-update`'s `release` test,
  `installer/test/install.test.mjs` (it calls `ditto`, `plutil`, `codesign`),
  `tools/package-release.sh`, `tools/build-ffmpeg.sh`.
- **The heartbeat** (`tests/e2e/heartbeat.mjs`), the kills and soaks, the two-Mac
  drive, the clean-Mac VM (`tests/l5/`), and Bagrat's checkpoint.
- **Playback in Safari** and the feel of anything: timing, sound by ear.

When a change touches both, run everything here first, then name the exact Mac-only
checks still owed.

## 5. A Mac in the loop

A GitHub-hosted Mac (macOS 26, Apple silicon, three cores, a 1024×768 screen) builds
this branch's dev app, grants it its permissions and runs it with the screen recorded.
`dev/mac/loop.sh` asks, waits and brings the evidence back. Use it when a fix touches
something §4 lists and Bagrat should see it work.

```bash
git push origin HEAD                          # the Mac tries what is on origin
bash dev/mac/loop.sh run selftest settings    # ~4 min warm, ~7 cold; prints the table and the videos
```

- **What comes back**, under `${TMPDIR:-/tmp}/shroom-mac/latest/evidence`: `summary.md`
  (the table), and per scenario `video.mp4`, `output.log` (the drive's own report),
  `windows.log`. Send Bagrat the scenario's video (SendUserFile, `display: "render"`)
  with its lines of the table. The drive's exit code is the verdict — read
  `output.log` for which check said no before calling anything fixed or broken.
- **The job's colour is not the verdict.** The Mac job is green whenever the Mac ran
  what was asked, a scenario's no included; it is red only when the Mac couldn't (the
  build, the grants, the SSH door). Always read the table — `loop.sh` ends with a
  `verdict:` line from it — and never call a scenario passed from a green run.
- **Scenarios** are `dev/mac/scenarios/<name>.sh`: `ls` them. Each wraps one of §4's
  drives. A new check is a new file on your branch — a few lines run from the repo's
  root, with `MAC_APP` (the bundle) and `MAC_OUT` (a folder for what it writes). Start
  the app with `open`, never by its binary: a child of the job's shell borrows the
  runner's own grants, and passes falsely.
- **When a script can't show it**, keep the Mac: `loop.sh run --hold 20 <scenario>`
  returns once its SSH door is open. Then `loop.sh ssh '<command>'` (run from the
  repo's root there), `loop.sh shot` (the screen as a PNG — send it), `loop.sh get
  <path>`, and `loop.sh release` when done: it fetches the evidence, with a video of
  the whole hold. Drive with `open`, `cliclick` and System Events keystrokes and
  clicks. A `tell application "X"` to any other app raises a consent prompt and hangs
  two minutes; answer it with `osascript -e 'tell application "System Events" to tell
  process "UserNotificationCenter" to click button "Allow" of window 1'`. The dev app's
  `--home` takes a folder under the temp dir: `"$(getconf DARWIN_USER_TEMP_DIR)x"`.
- **If the workflow can't be started** (`gh workflow run` refuses the session):
  `loop.sh run --by-push …` asks through a commit to `dev/mac/request` instead. That
  works on a session's own `claude/…` branch. Never on `main`: the ask is a commit and
  a push there, and `loop.sh` refuses it.
- **A camera.** The runner has none of its own; `bash dev/mac/camera.sh up`, first in a
  scenario, gives it one: OBS's virtual camera playing a test pattern (installed,
  approved in System Settings by a throwaway admin, started — ~2 min). The `camera`
  scenario runs it and the dev app's `--camera-check record` on it (proven 2026-10-13,
  run 38463184750); `take-camera` makes a real take with Camera as the source and keeps
  it — `camera-take.mp4` and `camera-take.png` in its evidence (B121; run 38465588089).
  A scenario that drives the bubble or Camera as the source starts the same way. OBS's
  camera offers only 1920×1080 at 60 fps (DECISIONS B120), and
  with OBS rendering on three cores it delivers ~20 fps: the camera's path, the
  bubble and `camera-lost` (quit OBS) can be read there, its timing can't.
- **The Mac's sound.** The `system-audio` scenario (`tests/e2e/system-audio-check.mjs`,
  DECISIONS B172) records two takes of the pattern, with the beep as the voice and the
  Mac's sound mixed in. In take 0, two other apps play: the dev app's `--play-out` burst
  and Apple's `afplay`. Both must be heard against the picture, and the app's own tone
  must be in neither take. Take 1 must be the beep alone. Start any "other app" from the
  script, never from shroom: ScreenCaptureKit's `excludesCurrentProcessAudio` leaves out
  shroom's own children too (probed 2026-10-16), so a player shroom starts reads as silence.
- **A Retina screen.** The runner's own is 1024 × 768 at 1×. `dev/mac/hidpi.m` makes a
  virtual display of 1280 × 800 points at 2× the main one for as long as it runs;
  `editor-2x` builds it with `clang` and runs the editor's drive there (proven 2026-10-25,
  run 40244400746). A scenario that must look at 2× starts the same way.
- **What it can't settle** — say so, don't stretch it. First-run permission prompts:
  the grants are written, never asked for. A real camera's picture and rate. Timing
  bars — repeated and skipped frames, A/V offset: `record` ends with 2 there, and a
  Mac decides. How anything looks at speed: a window's first paint takes two to four
  seconds, so a drive that closes one sooner shows it blank (its checks read the
  app's state, not the picture). Sound by ear, a second display, a notch. Real
  Cloudflare: the drives run against the fake. Those stay §4's, on Bagrat's Mac.
- **No VM inside it.** The runner is itself a VM ("Apple M1 (Virtual)") with no
  hypervisor: `kern.hv_support` is 0, Virtualization.framework says unsupported, and
  Tart's boot ends "Virtualization is not available on this hardware" (probed
  2026-10-13; GitHub's docs say the same). The L5 VM lanes stay on Bagrat's Mac. For more
  at once, ask more runners: each is already a fresh Mac. The plan, noted and not built:
  `dev/proposals/test-lanes.md`.
- **By hand, and the report.** `dev/mac/MANUAL.md` lists what no script proves yet: the
  checks to make by hand on a held Mac (with `dev/mac/hand.sh`), what only Bagrat's Mac
  settles, and what is red on a runner and why. After a run, build its report page —
  every scenario's verdict, failing checks and video — with `dev/mac/report.mjs` and
  publish it as an artifact (MANUAL §4); send Bagrat the link.
- **It is public.** The workflow's log and its evidence can be read by anyone signed
  in to GitHub, for three days. It records only the runner's own screen. Never put a
  token, a key or anyone's real content into a scenario, a request or a held Mac.
- **How sure this is.** Proven from a Mac session and from a cloud session on
  2026-10-13, hold and all: `run --by-push`, `--hold` (node's WebSocket through the
  session's HTTP proxy, `dev/mac/ws-proxy.mjs`), `shot` and `release`. The download is
  fetched from a storage host, so the environment needs Full network access; `--hold`
  needs `ssh` and `ssh-keygen`, which `dev/cloud/setup.sh` installs. When a step fails
  for a platform reason, that is a finding: fix `loop.sh` or `setup.sh`, or correct
  this section, and commit.

## 6. The full e2e, before a PR merges

Bagrat works from his phone: a bug comes in, you fix it, run the tests above and the Mac
scenarios its area touches, and **end with the full e2e and its video** — what he watches to
see nothing else broke. `.github/workflows/e2e-full.yml` (DECISIONS B127) runs it on a hosted
Mac against the dedicated Cloudflare test account (TESTING-PLAN D3): the branch's app
installed by the real one-line installer, setup through its own wizard with a token of the
product's two powers, one take, the page in WebKit and Chromium, every frame and the sound,
the owner's title, a member invited, joining, recording and removed, delete — then the
run's own space deleted. About seven minutes.

```bash
# On your own claude/… branch, pushed: a commit to dev/e2e/request starts it (never on main)
printf '# A session asks .github/workflows/e2e-full.yml for its own branch by committing here — never on main.\nasked=%s\n' \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > dev/e2e/request
git add dev/e2e/request && git commit -m "e2e: <why>" && git push origin HEAD
```

- **Watching it.** Find the run with the GitHub tools (`actions_list`, workflow
  `e2e-full.yml`, your branch), then poll it in the background — the public API answers
  without a token: `curl -s https://api.github.com/repos/bagrat/shroom/actions/runs/<id>`
  until `status` is `completed`. A PR from the branch also gets an `e2e (test account)`
  status, which wakes a session subscribed to it. Don't ask Bagrat whether he approved.
- **"Cancelled … — ask again"** (that status, `error`, B129): the run was cancelled before
  it said its verdict. With no concurrency group (B131) a newer request no longer takes a
  waiting one's place, so that is a cancel by hand — ask again if the branch still needs it.
- **The verdict** is the job's colour (and that status). Read `results.tsv` and
  `e2e-full/output.log` for which check said no. A **warning** "Timing bars missed on the
  runner" is a pass: the path held, and repeated/skipped frames or the A/V offset on three
  busy virtual cores are a Mac's to decide (the heartbeat's bars are never loosened).
- **The evidence** is the artifact `e2e-evidence-<run>-<attempt>` (`actions_list`
  → `list_workflow_run_artifacts`, `actions_get` → `download_workflow_run_artifact`, then
  `curl` the URL into a fresh scratch folder). Send Bagrat, with SendUserFile
  (`display: "render"`): `e2e-full/video-captioned.mp4` (the Mac's screen, each step
  captioned: setup, the measured take under the test pattern, the member leg's second
  mushroom, delete, then the show pass — the popover, the bubble, Start, the count, Stop —
  and its link playing in a browser; `video.mp4` is the same uncaptioned) and
  `e2e-full/page/play-webkit.mp4` (the measured take's page playing in WebKit; `owner.mp4`
  and `removed.png` are there too). A run whose recording has no captions (its step warns)
  can be captioned here: `CAPTIONS_VIDEO_START_MS=<the first chapter's from − 2500> node
  dev/e2e/captions.mjs <the folder>`. `wizard.json`, `heartbeat.json` and
  `run.log` (the app's log to the end of setup) say why when it's red.
- **The test account is shared** — Bagrat's Mac runs the heartbeat on it too, and runs go
  side by side: each run has its own space (`v-<run>` over `shroom-space-<run>`, the run id
  in `~/.shroom-dev/e2e-space-id`) and its close deletes only that space and its token;
  a crashed run's leftovers are reaped after 45 min by the age in their id (DECISIONS
  B131). There is no lock and no queue: two requests a minute apart run together.
- **Never** from here: a run on `main`, a token or the account id in a scenario, a log or the
  chat. The evidence is public for three days; it shows only the runner's screen and the
  test account's throwaway space.
