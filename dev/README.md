# dev/ — developing & diagnosing shroom

Tooling and notes for working **on** shroom (not shipped functionality). The plugin
itself lives in `commands/`, `skills/`, `scripts/`, `templates/`.

## The dev setup (Claude desktop app)

shroom loads as a **personal skills-dir plugin**: `~/.claude/skills/shroom` is a
symlink to this repo, so it loads **in place** (no install/cache/version-bump) in
every desktop window, reflecting the checked-out branch. Confirm with `/plugin`
(shows `shroom@skills-dir`) or `claude plugin list`. No marketplace shroom is
installed, so no `/shroom:*` collision. Disable: `claude plugin disable
shroom@skills-dir` or `rm ~/.claude/skills/shroom`.

**Two modes = which folder you open** (the plugin is loaded either way):

- **Develop** — open this repo as the project. You get `CLAUDE.md`, `SPEC.md`,
  project memory, the tests, and git **plus** the plugin live. Fix bugs here.
- **Clean-user test** — open `~/shroom-dev` (or any non-repo folder). Neutral root;
  catches CWD-dependent bugs. **Don't fix from here** — it has no working agreement,
  SPEC, or memory. Capture the bug and fix in a Develop window. (See
  `~/shroom-dev/README.md` for the bug-report template.)

After editing: `/reload-plugins` (skill text is live immediately; command
frontmatter, `plugin.json`, and new files need the reload or an app restart).

## Grabbing logs from a test run

Dev and Develop sessions **share global state** — `~/.shroom/` and the `~/shroom`
library are absolute `$HOME` paths, identical for both. So a Develop session can
read a test run's artifacts directly. They're scattered, so:

```
node dev/collect-logs.mjs            # newest recording — full report
node dev/collect-logs.mjs --list     # list recent recordings
node dev/collect-logs.mjs <id>       # a specific one
node dev/collect-logs.mjs <id> --tail 80
```

It's **read-only** and **never prints credentials** (keys show as "present"; only
public URLs are shown). It bundles: the event timeline, the `ffmpeg.log` tail, the
on-disk file inventory, the transcript summary, the library `<id>.md`, the built
site dir, the version marker, and a secret-safe creds summary.

## Diagnostics map — where everything lives

| Artifact | Path | Written by |
| --- | --- | --- |
| Per-recording session dir | `~/.shroom/recordings/<YYYYMMDD-HHMMSS>-<id>/` | recorder/shim |
| Event timeline | `…/<session>/events.ndjson` | recorder, uploader, publish, deploy |
| ffmpeg invocation + stderr | `…/<session>/ffmpeg.log` | recorder |
| HLS bytes (local) | `…/<session>/init.mp4`, `seg_*.m4s`, `stream.m3u8` | recorder |
| Watchable preview | `…/<session>/preview.mp4` | finalize |
| Transcript | `…/<session>/transcript.json` | transcribe |
| Library record (committed) | `<library>/<id>.md` (`~/shroom` by default) | write-meta / publish |
| Built page (pre-deploy) | `~/.shroom/site/<id>/index.html` + `poster.jpg` | build-page |
| Credentials (secrets, mode 600) | `~/.shroom/credentials.json` | setup |
| Post-update marker | `~/.shroom/version-state.json` | version/post-update.mjs |
| Shim build (gitignored) | `scripts/shim/macos/build/shroom.app/…` | setup init-library |

The Claude session transcript of a test window (the agent's actions, not plugin
state) lives under `~/.claude/projects/<project-hash>/` if you ever need it.

## Test suites

```
for t in scripts/*/test/*.test.mjs; do node "$t"; done
claude plugin validate .
```

## Manual test walkthrough

[`TEST-PLAN.md`](TEST-PLAN.md) — the guided Tier B + C walkthrough (the green-set
features, end to end), sequenced for a fresh machine. When the user asks to "walk
through the test step by step," drive it one step at a time and record each outcome.
The test itself is done by *using* the plugin in a clean-user window; failures are
handed off here per the diagnostics flow above.
