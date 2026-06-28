---
description: One-time shroom setup — check local tools, install what's missing, and provision Cloudflare R2 + Pages so record → link works.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Bash(node:*), Bash(git:*), Bash(brew:*), Bash(npm:*), Bash(/bin/bash:*), Bash(/bin/sh:*), Bash(xcode-select:*), Bash(wrangler:*), Bash(open:*)
---

You are running `/shroom:setup` — the one-time onboarding flow (SPEC §8). Treat the
person in front of you as **non-technical**: this should feel like a friendly guided
setup, not a sequence of scary technical prompts. Your job is **judgment and consent**;
the exact mechanism lives in `${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs` (the
determinism boundary) — drive that script, don't reimplement it.

**Golden rule (working agreement):** never silently mutate the machine. Every system
change — installing tools, `git init`, `wrangler login`, creating cloud resources — is
**propose → ask → run**, batched into **one approval where possible, not N**. The whole
flow is **idempotent**: safe to re-run, so on a partial setup just run it again and it
picks up where it left off.

The optional `$ARGUMENTS` is a preferred library directory.

## Tone & format — as important as the mechanics

- **Open with the welcome + the whole plan**, so the user sees the full journey before
  anything happens. Don't dribble out steps without showing where they lead.
- Keep **every message short and scannable** — numbered or bulleted, never a wall of
  prose, never alarming jargon. Explain *why* in half a sentence, not a paragraph.
- Use **`AskUserQuestion`** for every consent/decision gate — never bury a yes/no in
  text, never stack an explanation + instructions + several asks into one message.
- **Re-render the progress checklist** at the top of each step so the user always knows
  what's done (✅), what's happening now (▶️), and what's left (⬜). Reprint *just* the
  list — no extra narration around it.
- Every `open <url>` is an outward action: explain it in one line, get a yes, then open.

## Step 0 — Welcome + the plan (show this first)

Lead with a short welcome and the full plan. Use these real numbers (default 1080p
quality); keep it tight, roughly:

> **Welcome to shroom 🍄 — let's get you set up.** (~5–10 min, mostly installs.)
>
> shroom is **local-first**: your recordings library is a **git repo on your Mac**
> (default `~/shroom`), and the **video files upload to Cloudflare R2** (object storage)
> so every recording gets a permanent link you can share with anyone.
>
> **Cost:** **watching is always free** — Cloudflare R2 charges *nothing* for bandwidth,
> however many people view. You only pay for storage, ~**$0.015/GB-month**. At 1080p a
> recording is ~**27 MB/min** (~**1.6 GB/hour**), so the **free tier (10 GB) holds ~6
> hours** of recordings; past that it's pennies (≈100 hours kept online ≈ **$2.40/month**).
> To switch the free tier on, Cloudflare does require a **credit card** on file.
>
> **The plan:**
> 1. ⬜ Install local tools + create your library (`~/shroom`)
> 2. ⬜ Log in / sign up at Cloudflare
> 3. ⬜ Add a card + turn on R2 storage (activates the free tier)
> 4. ⬜ Create a storage access token (I'll guide you; you paste 3 values back)
> 5. ⬜ I set up your bucket + video site and save credentials to `~/.shroom`
> 6. ⬜ Done — record anytime with `/shroom:record`

Then a single `AskUserQuestion` to begin. After each step, reprint the plan with that
step ✅ and the next ▶️.

## Step 1 — Install local tools + create the library

First, a **silent** env check — it only *reads*, never installs:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" probe --json
```

Parse `{ ready, missingRequired, missingOptional, plan }`. Don't narrate it tool-by-tool.
Then **one consolidated `AskUserQuestion`** with two decisions:

1. **Install the missing tools?** Only if `plan.steps` is non-empty. Show the exact
   `plan.combinedCommand` in the option description. One toggle covers *all* of them (and
   the Homebrew bootstrap, if `needsBrew`). They're **all required** — whisper included,
   since titles / chapters / transcript search depend on it — so it's a single
   install-or-not choice; don't offer a "skip whisper" option.
2. **Where should the library live?** Default `~/shroom` (or `$ARGUMENTS` if given), with
   a free-text override.

Then, as **one** batched action the user approved:
- If approved, run `plan.combinedCommand` (single Bash call), then **re-run the probe** to
  confirm required tools are present. If something required is still missing, stop and say
  so plainly — don't proceed to Cloudflare.
- Build the library + local helpers in **one script call** — don't hand-assemble shell
  (that trips a scary "can't analyze this command" prompt and isn't the determinism
  boundary):
  ```
  node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" init-library --dir <dir> --json
  ```
  It creates the dir, `git init`s it if needed, records it in the creds, vendors
  `hls.min.js`, and compiles the macOS control shim (the menu-bar "tray" `/shroom:record`
  launches; it owns Screen-Recording permission) — all idempotent. Read the JSON:
  - `ok: false` → surface `stage` + `message` and stop.
  - `shim: "needs-xcode-clt"` → the shim needs Xcode **Command Line Tools**; propose
    **`xcode-select --install`** (a separate GUI installer — ask, let them run it), then
    **re-run `init-library`** (idempotent; only the shim is left to build).
  - `shim: "built"` (or `"skipped"` off macOS) → done.

Reprint the plan (step 1 ✅).

## Steps 2–5 — Cloudflare

Two distinct credentials are in play (live-verified): **Pages** (where each video's page
is hosted) rides the wrangler **OAuth** login; **R2** (the storage) **cannot** — there's
no R2 OAuth scope, so `r2 bucket create` over OAuth returns `Authentication error
[code: 10000]` even on a verified, R2-enabled account. R2 needs an **access token you
create in the dashboard**. Walk it as separate, brief steps:

**Step 2 — Log in / sign up at Cloudflare.**
- Check the session: `wrangler whoami` (it exits 0 even when logged out — read the text,
  not the code; "not authenticated" means log in).
- If logged out, one-line trust note then log in: you request only
  `account:read user:read pages:write`, *not* wrangler's ~27-scope default —
  `wrangler login --scopes account:read user:read pages:write` (opens a browser, no token
  paste). Afterward wrangler prints a yellow "missing some expected OAuth scopes" warning
  — tell them up front it's **expected and harmless** with a narrow login.
- **Then check email verification — right here, before step 3.** Cloudflare blocks the R2
  page *and* token creation until the account email is verified (email signups only;
  Google SSO is pre-verified). Don't let a later step discover it — that's the scary
  "verification required" wall. Run:
  ```
  node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" check-verified --json
  ```
  (it auto-detects the account; pass `--account <id>` if you already have it). Branch on
  `verified`:
  - `true` → carry on.
  - `false` → `AskUserQuestion`: tell them to open the verification email Cloudflare sent
    to their address and click the link; options *I've verified — continue / Resend (open
    `https://dash.cloudflare.com`)*. On continue, **re-run `check-verified`**; loop until
    `true`. **Do not proceed to step 3 while unverified.**
  - `null` → couldn't determine; don't block — proceed, and provision catches
    `email_unverified` later as a fallback.

  Reprint (step 2 ✅).

**Step 3 — Add a card + turn on R2.** One-line why (free tier needs a card on file), then
`AskUserQuestion` to open `https://dash.cloudflare.com/<accountId>/r2/overview`; they
accept the terms + add a card to enable R2. Reprint (step 3 ✅).

**Step 4 — Create the storage access token.** `AskUserQuestion` to open
`https://dash.cloudflare.com/<accountId>/r2/api-tokens`, then a short numbered list:
1. Click **Create Account API token**
2. Permission: **Admin Read & Write** (so it can create the bucket + enable public access)
3. **Apply to all buckets**
4. On the confirmation screen, copy the three values: **Token value**, **Access Key ID**,
   **Secret Access Key**

Have them **paste the three values right here in the session** — no temp files. Reprint
(step 4 ✅).

**Step 5 — Provision.** Pass the token + keys:
```
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" provision --json \
  --r2-token <TOKEN> --r2-access-key-id <AKID> --r2-secret-access-key <SECRET>
```
(Add `--bucket` / `--pages-project` only for non-default names.) This creates the bucket,
enables its public `*.r2.dev` URL (so videos play back — anyone with a link can watch;
links are long and unguessable, but public), creates the Pages project, and writes the S3
keys. **Branch on the result** (SPEC §8):
- `ok: true` → bucket, public URL, Pages project, S3 keys ready → step 6.
- `ok: false`, `needsDashboard: true` → a human gate; surface it with the deep-link (ask
  before opening), then **auto-poll** (re-run `provision`; don't make them type "done"):
    - `r2_token_required` → no/invalid token; (re)create it at the api-tokens page (step 4).
    - `email_unverified` → verify email at `https://dash.cloudflare.com`.
    - `r2_not_enabled` / `needs_payment` → enable R2 (terms + card) at
      `https://dash.cloudflare.com/<accountId>/r2/overview` (step 3).
- `ok: false`, `not_logged_in` / `insufficient_scope` → re-run the narrow `wrangler login`.
- any other `ok: false` → surface `message` and stop; don't guess.

Reprint (step 5 ✅).

## Step 6 — Done

`provision` reports `s3Token: "written"` once the token's Access Key ID + Secret are in
the creds — the uploader can now PUT segments. (`"deferred"` only if keys weren't passed;
loop back and capture them.)

Reprint the plan with **all six ✅**, then in one or two lines: confirm what's ready, note
the creds live in `~/.shroom/credentials.json` (mode 600 — secrets never touch the git
repo), and tell them the next step is **`/shroom:record`**. Keep it short and warm.
