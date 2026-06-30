# skills/

Agent skills — the **judgment** layer, which is the actual product (SPEC §7).

- [`cleanup/`](cleanup/SKILL.md) — **built.** Finds stale recordings and reclaims
  space: it judges what looks stale + owns the consent gates; the deterministic
  [`cleanup.mjs`](../scripts/cleanup/cleanup.mjs) scans, prunes local HLS (keeping a
  watchable MP4), deletes locally or from the bucket, and can upload a downloadable
  MP4. The "keep this?" half of the boundary — the script refuses to delete the only
  copy; the skill decides what to even propose.
- [`search/`](search/SKILL.md) — **built.** Transcript search over the git-library
  corpus: the deterministic [`search.mjs`](../scripts/search/search.mjs) does lexical
  retrieval (scoring + snippets, no external service); the skill turns a question
  into terms and ranks the candidates semantically, answering with links + chapter
  jump points. The "find where I said X" half of the boundary.

Planned: edit-as-a-sentence, "publish?" (M5c-3+).

Determinism boundary: skills decide; `scripts/` execute. See [`CLAUDE.md`](../CLAUDE.md).
