# templates/

Source templates rendered into generated outputs.

- **`player.html`** — the single per-video player page (SPEC §6). One template →
  N static HTML pages, each with per-video `og:` tags baked in (so Slack/Twitter/
  iMessage unfurl without running JS). Rendered at finalize by
  [`scripts/page`](../scripts/page/README.md) via deterministic token
  substitution (`{{TITLE}}`, `{{DATA_JSON}}`, …); all interpolated values are
  escaped. Safari plays HLS natively; other browsers lazy-load bundled hls.js.

  Beyond the native video controls it adds, from the data island: a **seekable
  chapter timeline** (proportional chunks that fill + highlight as the playhead
  crosses them), a **Copy link / Copy embed** toolbar (the embed is a Loom-style
  responsive iframe pointing at `?embed=1`, a chrome-less mode that shows just the
  player), a **Download MP4** button when the record's `mp4` flag is set, and
  keyboard shortcuts (← / → seek, space play/pause, `f` fullscreen). The share
  toolbar reveals only when a public page URL exists, so a local-only render stays
  clean.
