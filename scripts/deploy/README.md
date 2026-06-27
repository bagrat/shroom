# shroom deploy

The **deploy step** (SPEC §6) — the one cloud action for the *page* side of
publish. The bytes (HLS segments) go to the bucket via the [uploader](../uploader/);
this puts the per-video static pages [build-page](../page/) generated live on
**Cloudflare Pages** and surfaces the shareable playback URL.

Deterministic mechanism → a script (the determinism boundary). The agent decides
*when* to publish; this just does it, repeatably.

## What it does

1. **Place hls.min.js once at the site root** (`/hls.min.js`), shared by every
   per-video page — so the 20k-file Pages limit counts it once, not per video. It
   is **copied** from the explicitly-vendored, SHA-256-verified file
   (`scripts/page/vendor/hls.min.js`), never fetched here (no silent network side
   effects). If it isn't vendored yet, deploy stops and prints the exact
   `fetch-hls.mjs` command.
2. **`wrangler pages deploy <siteDir>`** ships the whole site bundle to the
   project's production branch. wrangler is OAuth-authed by the `wrangler login`
   session from setup — **no token paste** (SPEC §9).
3. **Emit `published`** with the shareable playback URL (`<pagesBase>/<id>/`).
   This is the durable SPEC §6 go-live signal, drained by the next `/shroom` run.

The playback base is the configured `pagesBaseUrl` (set at setup); if it isn't
set yet, it's derived from wrangler's deployment URL
(`https://<hash>.<project>.pages.dev` → `https://<project>.pages.dev`).

## Usage

```bash
node deploy.mjs --project <pages-project> [--id <id>] [--site <dir>] \
                [--branch <name>] [--session <dir>] [--pages-base <url>] \
                [--wrangler <bin>] [--force-hlsjs]
```

Defaults: `--site ~/.shroom/site`, `--branch main`, `--pages-base` from
page-config. With `--session <dir>` the emitted events are also appended to that
session's `events.ndjson` (the durable pending-publish artifact). Idempotent —
re-running just re-deploys; unchanged pages are no-ops.

## Events

- `hlsjs_placed` — the shared player was copied to the site root.
- `deployed` — `{ deploymentUrl, projectName, branch, siteDir }`.
- `published` — `{ id, playbackUrl, deploymentUrl }` (SPEC §6 go-live).
- `deploy_failed` — `{ reason, ... }` on a guard failure or a non-zero wrangler.

> Note: the uploader also emits a `published` event when the HLS **playlist**
> goes live in the bucket (the *bytes* go-live). Publish is two-phase — bytes,
> then page; only this step's `published` carries a `playbackUrl`, so that's the
> one a drain consumer keys on.

## Layout

```
deploy.mjs            CLI / recovery entry point (mirrors uploader/upload.mjs)
lib/deploy.mjs        URL parsing, hls.js placement, the deploy orchestration
lib/wrangler.mjs      the wrangler seam (spawn + tee) — injected as runWrangler
test/deploy.test.mjs  behaviour tests against a fake wrangler
```

## Tests

```bash
node test/deploy.test.mjs
```

Runs offline with no wrangler binary, no Cloudflare account, and no network — the
wrangler call is an injected seam. **The first real `wrangler pages deploy`** is
deferred to a live-account session (with setup, M5); cataloguing wrangler's actual
error shapes (project-not-found, not-logged-in, scope errors) is the deploy half
of the SPEC §11 error-shape catalogue and lands then.
