# skills/

Agent skills — the **judgment** layer, which is the actual product (SPEC §7).

- [`title-chapters/`](title-chapters/SKILL.md) — **built (M5c-2).** Authors a
  recording's title / TL;DR / chapters from its `transcript.json` and persists the
  `<id>.md` record via the deterministic
  [`write-meta`](../scripts/page/write-meta.mjs) writer. The judgment half of the
  boundary: the skill decides the text and chapter boundaries; the script writes
  the file. Used by `/shroom:record` post-stop, and standalone to re-title.

Planned: transcript + semantic search, cross-linking by id, edit-as-a-sentence,
smart retention, "keep this?" / "publish?" (M5c-3+).

Determinism boundary: skills decide; `scripts/` execute. See [`CLAUDE.md`](../CLAUDE.md).
