---
name: search
description: Search across your shroom recordings by what was said in them — find the recording (and chapter) that covers a topic, answer a question from the transcripts, or locate where something was discussed. Use when the user asks to find/search their recordings, or which video covered a topic.
allowed-tools: Read, Bash(node:*), Bash(open:*)
---

# Search the transcripts

You answer "which of my recordings talked about X?" over the local git-library
corpus. The deterministic `search.mjs` does the **retrieval** (lexical scoring +
snippets over every `<id>.md`); **you do the semantic part** (SPEC §7): turn the
question into good search terms, then judge which candidate actually answers it.
No external service — it's all the user's own files on their machine.

## Step 1 — pick terms, retrieve

From the user's natural-language question, choose the **content words** to search
for (drop filler; include obvious synonyms/variants — the retrieval is exact-token,
so *you* supply "auth", "authentication", "login" if they might all be meant). Then:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/search/search.mjs" query --q "<terms>" --limit 6
```

Read `{ corpusSize, results: [...] }`. Each result has `id`, `title`, `link`,
`score`, `matchedTerms`, `snippet`, and `chapters` (matching chapters, each with a
`time` like `0:48`). `corpusSize` is how many recordings exist to search.

- **No results?** Broaden: retry with synonyms or fewer/again-chosen terms before
  concluding nothing matches. If `corpusSize` is 0, say there's nothing recorded yet.

## Step 2 — rank semantically + answer

The `score` is only a lexical prior — **don't just echo the top score**. Read the
snippets and decide which recording(s) genuinely address the question; a lower-scored
hit with an on-topic snippet can beat a high-scored incidental keyword match. Then
answer like a person who watched them:

- Lead with the **answer / the best recording**: its title as a link, one line on
  why it's the match (quote the relevant snippet briefly).
- If a matching **chapter** is listed, point to it ("→ jump to *The PUT loop* at
  0:48") so they can click it on the page.
- List a couple of runner-up recordings if they're plausibly relevant; don't pad.
- Offer to `open "<link>"` the best one.

Ground every claim in a snippet — don't assert a recording covers something the
retrieval didn't surface. Be honest when it's a weak/uncertain match.
