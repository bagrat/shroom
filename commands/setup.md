---
description: One-time shroom setup — check local tools, install what's missing, and provision Cloudflare R2 + Pages so record → link works.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Bash(node:*), Bash(git:*), Bash(brew:*), Bash(npm:*), Bash(/bin/bash:*), Bash(/bin/sh:*), Bash(xcode-select:*), Bash(wrangler:*), Bash(open:*)
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
   They're **all required** — whisper included, since titles / chapters / transcript
   search depend on it — so skipping leaves setup not-ready. Don't offer a "skip
   whisper" choice; it's a single install-or-not decision now.
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
- Record the chosen dir so `/shroom:record` finds it without re-asking:
  `node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" set-library --dir <dir>`
  (a pure creds write — no machine mutation).
- **Vendor `hls.min.js` now** so the first publish's deploy doesn't stall on it
  (the player lazy-loads it on non-Safari browsers; deploy refuses to ship without
  it): `node "${CLAUDE_PLUGIN_ROOT}/scripts/page/vendor/fetch-hls.mjs"`. It's a
  pinned + SHA-256-verified fetch (a network action — fold it into this same
  approval); idempotent, a no-op if already vendored.
- **Compile the macOS control shim** (the menu-bar "tray" that `/shroom:record`
  launches — it owns Screen-Recording permission and the start/stop controls).
  We ship readable Swift source and build it on-device, never a precompiled blob:
  `/bin/sh "${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build.sh"` → `build/shroom-shim`
  (`swiftc -O` + an ad-hoc `codesign` for a stable TCC grant). It needs the Xcode
  **Command Line Tools**; if `build.sh` exits with *"swiftc not found"*, that's the
  one missing piece — propose **`xcode-select --install`** (a separate GUI installer,
  so it can't fold into the batched command; ask, let the user run it, then re-run
  `build.sh`). Idempotent — safe to re-run; recompiling is how an updated shim gets
  re-signed. (macOS only; skip on other platforms — recording needs the shim there.)

## Phase 3 — Cloudflare (one step at a time; explain before any browser opens)

**Go one step at a time, and use `AskUserQuestion` for every consent or decision
gate** — never bury a yes/no inside a paragraph, and never stack an explanation,
numbered instructions, and several asks into a single message (that wall of text is
exactly what overwhelms people here). Every `open <url>` is an outward action — explain
it, get a yes via `AskUserQuestion`, *then* open. Never pop a browser unannounced.

Two distinct credentials are in play (live-verified): **Pages** rides the wrangler
**OAuth** login; **R2 cannot** — there is *no* R2 OAuth scope, so `r2 bucket create`
over OAuth returns `Authentication error [code: 10000]` even on a verified, R2-enabled
account. R2 needs an **R2 API token you create in the dashboard**. So:

1. **Log in for Pages (narrow scopes).** Trust beat: tell the user you'll request
   only `account:read user:read pages:write` — *not* wrangler's ~27-scope default.
   - Check the session first: `wrangler whoami` (it exits 0 even when logged out, so
     read the text, not just the code — "not authenticated" means log in).
   - If logged out: `wrangler login --scopes account:read user:read pages:write`
     (opens a browser, no token paste, SPEC §9). Afterward wrangler prints a yellow
     "missing some expected OAuth scopes" warning — **expected and harmless** with a
     narrow login; tell the user so it doesn't alarm them.
2. **Give a short heads-up before sending them anywhere.** In **one brief message**,
   tell the user what the R2 setup involves, so nothing ambushes them mid-flow:
   - They'll **create an R2 API token by hand** in the Cloudflare dashboard — there's
     no way to do it through the login; that's just how Cloudflare works.
   - Turning on R2 **requires a credit card** on the Cloudflare account. R2 has a
     generous free tier, but the card is mandatory to enable the service.
   - If they signed up with **email** (not Google), Cloudflare may make them **verify
     their email** first — the token page stays blocked until they do.
   - Setup will switch on the bucket's **public `*.r2.dev` URL** so videos can play
     back, which makes the video bytes **readable by anyone who has the link** (the
     links are long and unguessable, but public). This is required for playback.
3. **Ask, with `AskUserQuestion`, whether to open the dashboard now** — e.g. *"Open the
   Cloudflare dashboard to create the R2 token?"* → *Open it (recommended) / Skip
   Cloudflare for now*. Choosing to proceed is their consent for the public-access
   behavior you just explained — don't ask for it again separately.
4. **Only after they say yes: open the page, then give the steps.** Open
   `https://dash.cloudflare.com/<accountId>/r2/api-tokens`, then a short numbered list:
   1. Click **Create Account API token**
   2. Permission: **Admin Read & Write** (so it can create the bucket and enable public access)
   3. **Apply to all buckets**
   4. On the confirmation screen, copy the three values: **Token value**, **Access Key
      ID**, **Secret Access Key**
   Have them **paste the three values right here in the session** — no temp files, that's
   just extra friction for a personal tool.
5. **Provision**, passing the token + keys:
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/setup/setup.mjs" provision --json \
     --r2-token <TOKEN> --r2-access-key-id <AKID> --r2-secret-access-key <SECRET>
   ```
   (Add `--bucket` / `--pages-project` only for non-default names.) R2 ops run with
   the token; Pages runs on OAuth; wrangler runs under the persisted Node ≥22 so it
   works without changing the user's default node. **Branch on the result** (SPEC §8):
   - `ok: true` → bucket, public URL, Pages project, and S3 keys are ready → Phase 4.
   - `ok: false`, `needsDashboard: true` → a human gate; surface together with
     deep-links (ask before opening), then **auto-poll** (re-run `provision`, don't
     make them type "done"):
       - `r2_token_required` → no/invalid R2 token; (re)create it at the api-tokens page.
       - `email_unverified` → verify email at `https://dash.cloudflare.com`.
       - `r2_not_enabled` / `needs_payment` → enable R2 (ToS + card) at
         `https://dash.cloudflare.com/<accountId>/r2/overview`.
   - `ok: false`, `not_logged_in` / `insufficient_scope` → re-run the narrow
     `wrangler login`, then retry.
   - any other `ok: false` → surface `message` and stop; don't guess.

## Phase 4 — finish

`provision` reports `s3Token: "written"` once the dashboard token's Access Key ID +
Secret are in the creds — the uploader can now PUT segments. (`"deferred"` only if the
keys weren't passed; loop back and capture them.)

Finish by confirming what's ready and where the creds live
(`~/.shroom/credentials.json`, mode 600 — secrets never touch the git repo), and
tell the user the next step is `/shroom:record`. Keep it short.
