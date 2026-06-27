# scripts/page — per-video static page generator (M4)

Turns a finished recording into a **static HTML player page**, generated from
**one** template (`templates/player.html`). This is the deterministic half of
SPEC §6 — pure file generation, **zero network**. The only cloud-touching step
(`wrangler pages deploy`) lands in M5; everything here is buildable and testable
offline, and re-runnable / idempotent.

## Why static, per-video (not one param-driven page)

Link unfurling. Slack / Twitter / iMessage crawlers don't run JS, so each video
needs its **own** page with baked `og:` meta tags. One template in source → N
generated pages, each re-derivable from its metadata record. Bonus: instant load
(values baked in) and privacy-friendly (only published fields ship).

## The two inputs (SPEC §3 substrate split)

| Source | Provides |
| --- | --- |
| `--session <dir>` (`~/.shroom/recordings/<id>`) | `preview.mp4` → the poster; `events.ndjson` → `id` + `durationSec` fallback |
| `--meta <id.md>` (the git library record) | `title` / `tldr` / `chapters` the agent authored (a skill, M5) |

Both are optional-ish: with no metadata file you still get a valid page
(title falls back to *"Untitled recording"*); with no session you must pass `--id`.

## Usage

```sh
node scripts/page/build-page.mjs \
  --session ~/.shroom/recordings/<id> \
  --meta    ~/shroom/<id>.md \
  --out     ~/.shroom/site \
  --public-base https://pub-xxxx.r2.dev \
  --pages-base  https://<project>.pages.dev
```

Writes `<out>/<id>/index.html` (+ `poster.jpg`) and prints a `page_built` JSON
summary (the playback URL, the HLS URL, whether config was complete). The public
bases also live in `~/.shroom/credentials.json` (set at setup, M5) or `SHROOM_*`
env vars, so in normal use the two `--*-base` flags are unnecessary.

## URL model

- **HLS bytes** (uploaded by `scripts/uploader`): `<public-base>/<id>/stream.m3u8`
  — the bucket's public origin (`*.r2.dev`).
- **Player page**: `<pages-base>/<id>/` — the Pages site. This is the playback /
  share link, baked into `og:url`.
- **hls.js**: a single shared copy at the site root (`/hls.min.js` by default,
  `--hlsjs-url` to override). Safari plays HLS natively and never loads it; other
  browsers fetch it lazily. See [`vendor/`](vendor/README.md) — it is **not**
  committed; you vendor a pinned, SHA-256-verified copy explicitly.

## Layout

```
build-page.mjs        CLI: load inputs → render → write site bundle → emit summary
lib/render.mjs        PURE: template + metadata + urls → HTML (all values escaped)
lib/metadata.mjs      parse/serialize the `<id>.md` frontmatter (the substrate record)
lib/page-config.mjs   public-URL config (publicBaseUrl / pagesBaseUrl / hlsJsUrl)
lib/poster.mjs        ffmpeg: preview.mp4 → poster.jpg (best-effort; never blocks)
vendor/fetch-hls.mjs  pinned + integrity-checked hls.js fetch (explicit, not auto-run)
test/                 render + metadata unit tests
```

## Test

```sh
node scripts/page/test/render.test.mjs
node scripts/page/test/metadata.test.mjs
```

Render tests cover token substitution, duration formatting, chapters, and the
**escaping** that stops agent/user text from breaking out of an attribute, the
markup, or the JSON data island (including a `</script>` injection attempt).
