---
description: One-time shroom setup — check local tools, install what's missing, and provision Cloudflare R2 + Pages so record → link works.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Write, Bash(node:*), Bash(git:*), Bash(brew:*), Bash(npm:*), Bash(/bin/bash:*), Bash(/bin/sh:*), Bash(xcode-select:*), Bash(wrangler:*), Bash(open:*)
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

First, a best-effort **version check** (silent on failure, never blocks): run
`node "${CLAUDE_PLUGIN_ROOT}/scripts/version/check.mjs"` and read its JSON. If
`updateAvailable` is true, note in **one line** that a newer shroom (`<latest>`) is
out and they may want to update from the `/plugin` menu + `/reload-plugins` before
continuing — but don't insist; setup works fine as-is. On any error or
`updateAvailable: false`, say nothing.

Also a **post-update check** (best-effort): run
`node "${CLAUDE_PLUGIN_ROOT}/scripts/version/post-update.mjs"` and read its JSON. For
each `pending` entry relay its `whatsNew` in one line; if an entry carries `actions`,
**propose → ask → run** each (show its `command`, get a yes) — never auto-run. The
check records the version itself, so it won't repeat. Empty / any error → say nothing.

Then a **silent** env check — it only *reads*, never installs:

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

**First, check what's already set up — never re-ask for credentials that already
work.** Re-running setup (or running it after a plugin update) must be a no-op when
Cloudflare is already provisioned; the R2 token especially is a manual, annoying thing
to recreate. Run:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" status --verify --json
```

`--verify` live-checks the stored R2 keys (a cheap signed HEAD). Branch on the result:

- **`ready: true`** (storage + pages configured and `storage.verified` is not `false`)
  → everything's already provisioned and the keys still work. Say so plainly — e.g.
  *"Cloudflare's already set up: bucket `<storage.bucket>`, site `<pages.project>` —
  nothing to redo"* — mark steps 2–5 ✅, and jump to **Step 6**. Only walk the steps
  again if the user **explicitly** wants to switch account/bucket.
- **`storage.verified: false`** (`verifyReason: "invalid_keys"`) → the stored R2 token
  was revoked/expired. Keep everything else; go straight to **Step 4** to recreate just
  the token, then **Step 5** (provision). Don't redo login or the dashboard card.
- **`storage.configured: false`** → storage isn't set up; walk Steps 2–5 below.
- **`pages.configured: false`** but storage is fine → only the site project is missing.
  Jump to **Step 5** but run provision **`--pages-only`** (no `--r2-creds-file`): it
  skips the R2 steps and creates just the Pages site over the OAuth session, so you
  **never ask for an R2 token** for storage that already works. Just confirm the user's
  still logged in (Step 2's `wrangler whoami`) first.
- **`verifyReason: "no_fetch"` or `verified: null`** → couldn't live-check (old Node or
  offline). Fall back to presence: if storage + pages are `configured`, treat as done;
  otherwise walk the steps. Don't re-ask for keys just because verification was skipped.

If you do need to (re)provision, two distinct credentials are in play (live-verified):
**Pages** (where each video's page
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
  paste). NOTE: `offline_access` is **not** a valid `--scopes` value (wrangler errors);
  wrangler appends it to the OAuth request itself, so a refresh token is issued regardless.
  Afterward wrangler prints a yellow "missing some expected OAuth scopes" warning — about
  the other ~24 default scopes we skip; **expected and harmless**.
- **Provision (step 5) runs wrangler non-interactively**, so the OAuth session must still be
  present then — it can't pop a browser to re-auth. The Pages project is created via the
  Cloudflare REST API with the OAuth token directly (it carries `pages:write`), *not* through
  `wrangler pages project create` — that command demands a `CLOUDFLARE_API_TOKEN` when
  non-interactive and refuses the OAuth session no matter how fresh it is. Provision reads the
  **freshest** wrangler OAuth token across all candidate config paths (by `expiration_time`) —
  a stale leftover at one path used to shadow the refreshed one and cause a `10000`
  Authentication error even right after login (wrangler 4.x `whoami` refreshes but doesn't
  rewrite the orphan file). If provision still returns an auth error / `not_logged_in`, the
  session is genuinely gone: re-run `wrangler login` and retry.
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
  - `false` → `AskUserQuestion`. Question: "Cloudflare emailed a verification link to the
    address you signed up with — click it, then continue. (Check spam if it's not there.)"
    **Do NOT name a specific email address** — never use the ambient/session user email
    (that's the operator's personal email, not the Cloudflare account's). If you must show
    one, use *only* the address from `wrangler whoami`; otherwise name none. Options,
    **self-explanatory, no jargon**: *"I've verified — continue"* and *"No email yet — open
    Cloudflare"* (the latter opens `https://dash.cloudflare.com`, which shows a verify
    banner with a Resend button — don't label an option just "Resend"; it reads as
    nonsense). On "continue", **re-run `check-verified`**; loop until `true`. **Do not
    proceed to step 3 while unverified.**
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
3. On the confirmation screen, copy the three values: **Token value**, **Access Key ID**,
   **Secret Access Key**

Have them **paste the three values right here in the session**. Reprint (step 4 ✅).

**Step 5 — Provision.** Provision runs wrangler **non-interactively**. Its first call is
`wrangler whoami`, which refreshes the OAuth token via the refresh token; provision then reads
the **freshest** token across all wrangler config paths (by `expiration_time`, so a stale
leftover can't shadow it) — the bucket/public-URL/Pages calls all run under it without manual
re-login. The Pages project is created via the Cloudflare REST API with that OAuth
token (it carries `pages:write`); we never shell `wrangler pages project create`, which
rejects the OAuth session non-interactively. Only if provision returns `not_logged_in` (the
refresh token is gone) do you re-run `wrangler login --scopes account:read user:read
pages:write` and retry.

Then, keep the secrets off the command line: **write the three pasted values to a creds file
with the Write tool** (a file write — they never appear in a shell command, consent prompt,
or shell history) as JSON to `~/.shroom/r2-provision.json`:
`{"r2Token":"…","r2AccessKeyId":"…","r2SecretAccessKey":"…"}`. Then run:
```
node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" provision --json \
  --r2-creds-file ~/.shroom/r2-provision.json
```
`provision` reads it and **deletes it** when done (kept only across dashboard-gate retries).
(Add `--bucket` / `--pages-project` only for non-default names.)

**Pages-only shortcut** — if the earlier `status` showed storage already configured + the
keys verified and *only Pages* missing, skip the token entirely: run
`provision --json --pages-only` (no `--r2-creds-file`). It does just the OAuth Pages
step and leaves the stored bucket/keys untouched (`s3Token: "unchanged"`).

The full run creates the bucket,
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
