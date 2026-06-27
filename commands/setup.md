---
description: One-time shroom setup — check local tools, install what's missing, and provision Cloudflare R2 + Pages so record → link works.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Bash(node:*), Bash(git:*), Bash(brew:*), Bash(npm:*), Bash(/bin/bash:*), Bash(wrangler:*), Bash(open:*)
---

You are running `/shroom:setup` — the one-time onboarding flow (SPEC §8). Your job
is **judgment and consent**; the exact mechanism lives in
`${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs` (the determinism boundary). Drive
that script; don't reimplement it.

**Golden rule (working agreement):** never silently mutate the machine. Every
system change — installing tools, `git init`, `wrangler login`, creating cloud
resources — is **propose the exact command → ask → run**, and you batch these into
**one approval, not N**. The whole flow is **idempotent**: it's safe to re-run, so
on a partial setup just run it again and it picks up where it left off.

The optional `$ARGUMENTS` is a preferred library directory.

## Phase 1 — silent local-env check (no questions yet)

Run the probe and read its JSON. This only *reads*; it never installs.

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" probe --json
```

Parse `{ ready, missingRequired, missingOptional, plan }`. Do **not** narrate the
results tool-by-tool — go straight to Phase 2 with whatever's missing.

## Phase 2 — one consolidated AskUserQuestion

Ask **two questions in a single `AskUserQuestion` call** (SPEC §8 step 2), so the
user makes both decisions at once:

1. **Install the missing tools?** Only if `plan.steps` is non-empty. Show the exact
   `plan.combinedCommand` in the option description so the approval is informed.
   One toggle covers *all* of them (and the Homebrew bootstrap, if `needsBrew`).
   Required-vs-optional: note that skipping a *required* tool blocks recording;
   `whisper` (optional) only powers titles/chapters/search and can come later.
2. **Where should the library live?** Default `~/shroom`; offer a free-text
   override. If `$ARGUMENTS` was given, make that the default instead.

Handle every combination gracefully — "pick a dir but skip install", or "nothing
to install" (skip question 1 entirely).

Then, as **one** batched action the user already approved:
- If they approved install, run `plan.combinedCommand` (a single Bash call), then
  **re-run the probe** to confirm the required tools are now present. If something
  required is still missing, stop and report it — don't proceed to Cloudflare.
- `git init` the chosen library dir if it isn't already a repo (`git -C <dir>
  rev-parse` to check). This folds into the same approval.

## Phase 3 — Cloudflare (explain before the browser opens)

Trust beat for the audience: **before** running anything, tell the user you'll ask
Cloudflare for **narrow scopes** — R2 + Pages + account read — and why. Then:

1. **Check the existing session first** (probe capability, don't assume):
   `wrangler whoami`. If it returns an account, **skip login silently** (returning
   user). If not, run `wrangler login` — say plainly that it opens a browser for
   OAuth, no token paste (SPEC §9).
2. **Provision** over the script:
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" provision --json
   ```
   (Pass `--bucket` / `--pages-project` only if the user wants non-default names.)

   This attempts the real gated ops and **branches on the result** (SPEC §8) —
   you don't detect account state, you react to it. Read the JSON:
   - `ok: true` → bucket, public URL, and Pages project are ready. Continue to
     Phase 4.
   - `ok: false` with `needsDashboard: true` → a **human-only gate**. Surface the
     relevant ones **together** with deep-links (you have `accountId`), one browser
     trip, not a ping-pong:
       - `email_unverified` → verify email at `https://dash.cloudflare.com`.
       - `r2_not_enabled` → enable R2 (ToS + card) at
         `https://dash.cloudflare.com/<accountId>/r2/overview`.
       - `needs_payment` → add a payment method (same R2 activation flow).
     After the user says they've done it, **auto-poll**: re-run `provision` to
     retry (don't make them type "done"). Repeat until `ok: true`.
   - `ok: false` with `not_logged_in` / `insufficient_scope` → re-run
     `wrangler login` (asking for the missing scope), then retry provision.
   - any other `ok: false` → surface `message` and stop; don't guess.

## Phase 4 — S3 upload token + finish

`provision` reports `s3Token`:
- `"written"` → done. The uploader can PUT segments.
- `"deferred"` → minting the R2 **S3 API token** programmatically isn't wired yet
  (it's the one step wrangler has no command for; it lands in the live-account
  session). Until then, guide the user to create an **R2 API token** at
  `https://dash.cloudflare.com/<accountId>/r2/api-tokens` (Object Read & Write),
  and write its Access Key ID + Secret into `~/.shroom/credentials.json` as
  `accessKeyId` / `secretAccessKey`. Be explicit this is the interim path.

Finish by confirming what's ready and where the creds live
(`~/.shroom/credentials.json`, mode 600 — secrets never touch the git repo), and
tell the user the next step is `/shroom:record`. Keep it short.
