# shroom setup

The deterministic backend for **`/shroom:setup`** (SPEC §8). The *judgment* —
what to ask, when to install, how to phrase the Cloudflare gates — lives in the
setup [command](../../commands/) (M5b-3). This is the exact, repeatable mechanism
it calls (the determinism boundary).

> **Status: M5b-2.** Local-env half (probe + install plan) and the Cloudflare
> provisioning core (error catalogue, `whoami` → bucket → public access → Pages
> project, credentials writer) are built and offline-tested. The first *live*
> `wrangler login` + real provisioning — and minting the R2 S3 API token (the one
> step wrangler has no command for) — land in a live-account session. The
> orchestrating `/shroom:setup` command is M5b-3.

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

## Cloudflare provisioning (SPEC §8 sub-sequence, §9)

`provision` runs the Cloudflare sub-sequence over wrangler (the same spawn+tee
seam deploy uses) and merges the results into `~/.shroom/credentials.json`:

1. **`whoami`** → account id + email (and "are we logged in at all"). The OAuth
   session itself comes from `wrangler login`, which the *command* runs (it's
   interactive / opens a browser); this re-probes after.
2. **`r2 bucket create`** — `already_exists` counts as success (setup is
   idempotent). A cold account fails *here* with a specific state.
3. **`r2 bucket dev-url enable`** → the managed `*.r2.dev` public origin
   (`publicBaseUrl`, zero DNS).
4. **`pages project create`** → the `*.pages.dev` site base (`pagesBaseUrl`).

**Probe capability, not state** (SPEC §8): we don't try to detect whether the
account is verified / R2-enabled / has a card — we attempt the real op and
**branch on the classified failure**. The states are catalogued in
`lib/wrangler-errors.mjs` so `not_logged_in` vs `email_unverified` vs
`r2_not_enabled` vs `needs_payment` vs `insufficient_scope` each route to the
right next step (re-login, a dashboard gate, a retry). These matchers are
best-effort until validated against real wrangler output — the live session
tightens them (the SPEC §11 build task).

**The deferred piece:** minting the **R2 S3 API token** (access key id + secret
the uploader needs) — wrangler has *no command* for it, so it's an injected
`mintR2Token` seam whose real implementation (a Cloudflare API call) is wired in
the live session. Without it, `provision` still completes and writes the creds
*without* S3 keys, reporting the token as `deferred` rather than fabricating one.

## Credentials file (`~/.shroom/credentials.json`, mode 600)

One file, two kinds of fields, so setup writes once and each consumer loads its
slice: **secrets** (`endpoint`/`region`/`bucket`/`accessKeyId`/`secretAccessKey`
→ uploader) and **public** (`publicBaseUrl`/`pagesBaseUrl`/`pagesProject`/
`hlsJsUrl` → page + deploy), plus `accountId` and `library` (the git library dir
where `<id>.md` records live → `/shroom:record` + write-meta). The `endpoint` is *derived* from
`accountId` (`https://<id>.r2.cloudflarestorage.com`). Writes are **merge, not
clobber** — a re-run or a later top-up (e.g. the token arriving) preserves
untouched fields. Secrets stay out of git (working agreement).

## Usage

```bash
node setup.mjs probe [--json]
node setup.mjs provision [--bucket N] [--pages-project N] [--branch N] [--wrangler BIN] [--json]
node setup.mjs set-library --dir <path> [--json]
```

`probe` prints a per-tool ✓/✗/○ summary + proposed install commands; `--json`
emits `{ results, ready, missingRequired, missingOptional, plan }`. `provision`
prints a summary (or `--json` result) and merges the creds; ndjson `cf_*` events
go to stderr. `set-library` persists the chosen library dir into the creds (a pure
write — the command does the `git init`). Exit `0` on success, `1` otherwise.

## Layout

```
setup.mjs              CLI: `probe`, `provision`, `set-library`
lib/env-probe.mjs      tool catalogue + probe (run + PATH-lookup seams)
lib/install-plan.mjs   missing tools → consolidated, batched install commands
lib/wrangler-errors.mjs the error-shape catalogue (classify → next step)
lib/cloudflare.mjs     provisioning orchestration over the runWrangler seam
lib/credentials.mjs    read/merge/write ~/.shroom/credentials.json (mode 600)
test/setup.test.mjs    offline behaviour tests (fake run / lookup / wrangler seams)
```

## Tests

```bash
node test/setup.test.mjs
```

Runs fully offline — the `run` (version command), `lookupPath` (PATH scan), and
`runWrangler` (Cloudflare) seams are all injected with fakes, plus a temp `HOME`
for the credentials writer, so no real binary is spawned, no real PATH is read,
and nothing touches the real `~/.shroom`. Bare-machine verification of the
install commands (clean Mac / VM, the install-path-testing call) and the first
real `wrangler login` + provisioning (live-account session) are deferred — the
seams exist precisely so that can wait.
