# templates/

Source templates rendered into generated outputs.

- **`player.html`** — the single per-video player page (SPEC §6). One template →
  N static HTML pages, each with per-video `og:` tags baked in (so Slack/Twitter/
  iMessage unfurl without running JS). Rendered at finalize by
  [`scripts/page`](../scripts/page/README.md) via deterministic token
  substitution (`{{TITLE}}`, `{{DATA_JSON}}`, …); all interpolated values are
  escaped. Safari plays HLS natively; other browsers lazy-load bundled hls.js.
