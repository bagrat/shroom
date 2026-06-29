# skills/

Agent skills — the **judgment** layer, which is the actual product (SPEC §7).

- [`title-chapters/`](title-chapters/SKILL.md) — **built (M5c-2).** Authors a
  recording's title / TL;DR / chapters from its `transcript.json` and persists the
  `<id>.md` record via the deterministic
  [`write-meta`](../scripts/page/write-meta.mjs) writer. The judgment half of the
  boundary: the skill decides the text and chapter boundaries; the script writes
  the file. Used by `/shroom:record` post-stop, and standalone to re-title.
- [`cleanup/`](cleanup/SKILL.md) — **built.** Finds stale recordings and reclaims
  space: it judges what looks stale + owns the consent gates; the deterministic
  [`cleanup.mjs`](../scripts/cleanup/cleanup.mjs) scans, prunes local HLS (keeping a
  watchable MP4), deletes locally or from the bucket, and can upload a downloadable
  MP4. The "keep this?" half of the boundary — the script refuses to delete the only
  copy; the skill decides what to even propose.
- [`dashboard/`](dashboard/SKILL.md) — **built.** A visual library: merges the git
  records with local session state into a self-contained static page (thumbnails,
  links, durations, disk footprint) and acts as the entry point for management —
  routing to `cleanup` (prune/delete) and `title-chapters` (re-title). The
  deterministic [`dashboard.mjs`](../scripts/dashboard/dashboard.mjs) builds the
  list + HTML; the skill narrates and routes.

Planned: transcript + semantic search, edit-as-a-sentence, "publish?" (M5c-3+).

Determinism boundary: skills decide; `scripts/` execute. See [`CLAUDE.md`](../CLAUDE.md).
