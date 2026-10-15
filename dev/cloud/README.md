# dev/cloud — working on shroom from a Claude Code cloud session

For running v1's Linux-runnable parts from a phone. You prompt, and Claude runs
everything through the `shroom-cloud` skill (`.claude/skills/shroom-cloud/`): the
crates' tests, the Worker's, the webviews' build, the UI galleries and the page's faces
as screenshots. Capture, the app itself, the encoder and the installer need the Mac —
the skill's §4 has the list, and its §5 the hosted Mac that runs much of it for you
(`dev/mac/`: a GitHub-hosted Mac builds the branch's dev app, runs it with the screen
recorded, and hands back a video).

The POC's cloud tooling (fake recordings, wrangler login, whisper) went with the POC: it is
in the private bagrat/shroom-archive at tag `v0.1.5`, `git show v0.1.5:archived-poc/dev/cloud/README.md`.

## One-time environment config (claude.ai/code → environment → Edit)

**Setup script** — the whole field:

```bash
#!/bin/bash
for d in "$PWD" /home/user/shroom; do
  [ -f "$d/dev/cloud/setup.sh" ] && SHROOM_CLOUD_HEAVY=setup exec bash "$d/dev/cloud/setup.sh"
done
exit 0
```

**Network access:** Full — crates.io and GitHub (the workspace pins two git
dependencies), npm, Playwright's browser downloads, apt.

**Environment variables:** none. A cloud session never holds a Cloudflare token: the live
tests run in GitHub workflows (`e2e.yml`, `e2e-full.yml`) on the dedicated test account,
through the `cf-e2e` environment's secrets. `SHROOM_TEST_TOKEN_B64`, the main-account test
token's variable, retired with main-account testing (TESTING-PLAN D3); if your environment
still sets it, delete it there — the hook unsets it either way.
- `SHROOM_CLOUD_HEAVY` is set inline in the setup script above, not here, because
  changing an env var doesn't rebuild the environment cache, but editing the setup
  script does. It covers the slow two: Playwright's browsers with their system
  libraries, and the crates' sources. Values: `defer` (default): both start in the
  background at session start and never block the session. `setup`: do them in the
  setup script, so they're baked into the environment cache (that step takes a few
  minutes, but only when the cache rebuilds). `0`: neither.

## Files

| File | What |
| --- | --- |
| `setup.sh` | Idempotent provisioning: Node ≥ 22 (checked), ffmpeg + ffprobe, a C linker, ssh + ssh-keygen (`dev/mac/loop.sh --hold`), Rust ≥ `Cargo.toml`'s `rust-version`, `npm ci` when `node_modules` is missing or older than the lockfile; the slow two only in `setup` mode. Run by the UI setup script *and* the SessionStart hook, so edits here reach new sessions even while the environment cache is warm. Linux only — on a Mac it says so and leaves. |
| `playwright.sh` | The workspace's Playwright browsers (WebKit, Chromium) and their system libraries, installed off the critical path: `start` (background), `status`, `wait`, `install`. Ready only once both engines have started headless. |
| `path.sh` | Sourced by the others: finds Rust in `~/.cargo/bin` and Node 22 in `/opt/node22/bin` when a fresh shell's PATH lacks them. |
| `../../.claude/hooks/session-start.sh` | Cloud-only hook (`CLAUDE_CODE_REMOTE=true`; exits at once anywhere else): runs `setup.sh`, persists `path.sh`'s findings for the session, starts the background installs, writes the test token from `SHROOM_TEST_TOKEN_B64` (one line, or it is ignored) and unsets the variable for later commands, prints a status line. |
| `../../.claude/skills/shroom-cloud/SKILL.md` | The recipes: the session check, every test that runs here, the galleries and the page as screenshots, the fenced live tests, what needs the Mac. |

No fake recorder, no mock storage and no screenshot helper live here, on purpose: v1
already has them as its own tests and drives — `crates/shroom-cloudfake` (Cloudflare
and storage in-process), `worker/dev/serve.mjs` (the real Worker on loopback),
`app/test/*-gallery.mjs` and `tests/e2e/pages-gallery.mjs` (screenshots into
`gallery-out/` folders), `worker/dev/page-drive.mjs` (the page in two engines).

## What runs here, and how sure that is

Decided on a Mac, not on Linux: each crate's dependency tree was resolved for
`x86_64-unknown-linux-gnu` (`cargo tree --target`), the tests were read for macOS-only
tools, and every listed command was run on macOS behind stand-ins that make
VideoToolbox, `ditto`, `plutil`, `codesign` and friends fail the way their absence
would. So the skill's lists are **expected to run on Linux — the first cloud session
confirms.** When a step fails there, the rule is the same as for a missing tool: fix
`setup.sh` (or correct the skill's list), and commit, so every later session gets it.

- **Runs:** every crate but the app shell — whole for `shroom-core`, `-spec`,
  `-testpattern`, `-cloudfake`, `-storage`, `-cloud`; unit tests for `-encode`,
  `-record`, `-update`, `-capture`; `-upload` but for one test. The Worker (vitest and
  under workerd). The webviews' type check and build. The landing page's test, the
  agent skill's copy test, the media readers, the spec fixtures. The galleries and the
  page drive, on Playwright's Linux WebKit and Chromium.
- **Doesn't:** `shroom-app` (on Linux Tauri wants GTK and WebKitGTK, and the shell
  itself is written against AppKit and the panel plugin — `objc2-app-kit` and
  `tauri-nspanel` are in its tree on every platform); any test that records
  (`crates/shroom-encode` asks ffmpeg for `h264_videotoolbox`, which only a Mac's
  ffmpeg has); the updater's and the installer's tests (`ditto`, `plutil`, `codesign`,
  `clang -arch`); the status reader's (`plutil`); the heartbeat, the drives of the
  real app, the VM.
