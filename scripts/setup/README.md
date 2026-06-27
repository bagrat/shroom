# shroom setup

The deterministic backend for **`/shroom:setup`** (SPEC §8). The *judgment* —
what to ask, when to install, how to phrase the Cloudflare gates — lives in the
setup [command](../../commands/) (M5b-3). This is the exact, repeatable mechanism
it calls (the determinism boundary).

> **Status: M5b-1.** The local-env half (probe + install plan) is built and
> offline-tested. Cloudflare provisioning (`wrangler login` → bucket → public
> access → Pages project → R2 S3 token → `~/.shroom/credentials.json`) is M5b-2.

## What it does (so far)

**Silent local-env check** (SPEC §8 step 1) — checks for the tools shroom needs
and only reports; it never installs. Runs first so the command prompts only about
what's actually missing.

| tool | required | why |
|------|----------|-----|
| `git` | yes | the video library is a git repo (SPEC §3) |
| `ffmpeg` | yes | screen + mic capture and HLS segmenting (SPEC §4) |
| `wrangler` | yes | Cloudflare login, R2 + Pages provisioning, deploy (SPEC §8) |
| `whisper` | no | local transcription → titles / chapters / search (SPEC §7) |

**Consolidated install plan** (SPEC §8 step 2) — collapses everything missing
into the fewest exact commands (batched per package manager: one `brew install`,
one `npm install -g`), so the command can surface it as **one** "propose →
confirm → run" approval, not N. If a brew-managed tool is missing and Homebrew
itself is absent, the plan prepends the official Homebrew bootstrap so the chain
stays one approval. It **builds** the commands; it never runs them.

## Detection notes

- **Version-bearing tools** (git/ffmpeg/wrangler) are detected by running their
  `--version` command — a `0` exit (or a parsed version even on non-zero) proves
  presence and yields the version.
- **Presence-only tools** (whisper) are detected by a pure **PATH lookup**, never
  executed: `whisper` imports torch on every invocation, so a cold `whisper --help`
  can blow a timeout and flap to "absent". We only execute a tool when we need to
  parse its version.

## Usage

```bash
node setup.mjs probe [--json]
```

`probe` prints a per-tool ✓/✗/○ summary and the proposed install commands.
`--json` emits `{ results, ready, missingRequired, missingOptional, plan }` — the
machine-readable form the setup command consumes to drive its single
`AskUserQuestion`. Exit `0` when all required tools are present, `1` otherwise.

## Layout

```
setup.mjs            CLI: `probe` (more subcommands in M5b-2)
lib/env-probe.mjs    the tool catalogue + probe (run + PATH-lookup seams)
lib/install-plan.mjs missing tools → consolidated, batched install commands
test/setup.test.mjs  offline behaviour tests (fake run + lookup seams)
```

## Tests

```bash
node test/setup.test.mjs
```

Runs fully offline — both the `run` (version command) and `lookupPath` (PATH
scan) seams are injected with fakes, so no real binary is spawned and the real
PATH is never read. Bare-machine verification of the actual install commands is
deferred to a clean Mac / VM (the install-path-testing call); the probe is a
mockable seam precisely so that can wait.
