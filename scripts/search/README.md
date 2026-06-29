# scripts/search/

Deterministic retrieval for the [`search`](../../skills/search/SKILL.md) skill —
the repeatable index/score half; the skill turns a question into terms and ranks
the candidates semantically (the determinism boundary, [`CLAUDE.md`](../../CLAUDE.md)).

## `search.mjs query --q "<terms>"`

Loads the corpus (every `<id>.md` in the library: title, TL;DR, chapter labels,
transcript body) and scores each record against the query terms — a weighted
term-frequency (title ×4, TL;DR ×2, chapters ×2, transcript ×1) plus a bonus when
the raw query appears verbatim. Returns the top matches with a snippet windowed
around the hit, the public link, matched terms, and any matching chapters (each with
a `time` for jumping):

```json
{ "ok": true, "corpusSize": 12, "results": [
  { "id": "…", "title": "Uploader walkthrough", "link": "…", "score": 14,
    "matchedTerms": ["put"], "snippet": "…each segment up with a PUT…",
    "chapters": [{ "t": 48, "time": "0:48", "label": "The PUT loop" }] } ] }
```

**No external service / embeddings** (v1): retrieval is exact-token lexical, so the
skill supplies synonyms and does the semantic judgment over the snippets. Zero deps.

Tests (`npm test`): tokenization, scoring/ranking, snippet windowing, chapter
matching, and the empty/stopword-only guard.
