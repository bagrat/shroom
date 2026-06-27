---
name: title-chapters
description: Author a shroom recording's title, TL;DR, and chapters from its transcript, then write the <id>.md metadata record. Use after a recording is transcribed, or when the user asks to (re)title / re-chapter a video.
allowed-tools: Read, Bash(node:*)
---

# Title / TL;DR / chapters

You turn a finished recording's **transcript** into the three human-facing pieces
of metadata — **title**, **TL;DR**, **chapters** — and persist them. This is the
judgment half of the determinism boundary (SPEC §7): *you* decide what the title
says and where a chapter falls; the deterministic `write-meta.mjs` script writes
the file. Never hand-write `<id>.md` yourself — always go through the script, so
escaping, key order, and the transcript body stay correct.

## Input

`$ARGUMENTS` is the recording's **session dir** (e.g.
`~/.shroom/recordings/<id>`). You may also be given the `id` and a `library`
override explicitly. From the session dir:

1. Read `<session>/transcript.json` — the normalized transcript
   (`{ language, transcript, durationSec, segments: [{ start, end, text }] }`).
   The `segments` carry the timestamps you need for chapter `t` values.
2. The `id` is in `<session>/events.ndjson` (`session_started.id`) if not given.

**If there is no `transcript.json`** (whisper was skipped — it's optional, SPEC
§8), or it's empty: don't invent content. Author a short, honest title from
whatever context you do have (the user's description of what they recorded, the
duration), set no chapters, and still write the record so the page renders.

## Judgment

- **Title** — one line, ~3–8 words, specific and skimmable. Describe what the
  recording *shows or explains*, not "Screen recording" / "Untitled". No trailing
  period. Match the speaker's own framing where they state it.
- **TL;DR** — 1–2 sentences. What a viewer learns or what was decided. Skip it
  only for a very short clip where it would just restate the title.
- **Chapters** — only when they genuinely help: the recording is long enough
  (roughly ≥ 2 min) **and** moves through distinct topics. A 30-second clip needs
  none. Each chapter is `{ "t": <seconds>, "label": "<short phrase>" }`:
  - `t` is a real segment `start` from the transcript — never invented. The first
    chapter starts at `0`.
  - `label` is a few words, a topic not a sentence. Aim for a handful of chapters,
    not one per paragraph.
- **Ground everything in the transcript.** Don't claim the video covers something
  it doesn't. This metadata gets committed and shipped to the public page.

## Persist (the deterministic step)

Call the writer once. It reads the transcript body + duration + createdAt from the
session itself; you supply only the judgment:

```bash
node "${CLAUDE_PLUGIN_ROOT}/scripts/page/write-meta.mjs" \
  --id "<id>" --session "<session-dir>" \
  --title "<title>" \
  --tldr "<tldr>" \
  --chapters '[{"t":0,"label":"Intro"},{"t":48,"label":"The PUT loop"}]'
```

- Omit `--tldr` / `--chapters` when you decided there are none.
- Pass `--library <dir>` only if you were given an explicit library override; by
  default the script resolves it (creds `library` → `~/shroom`).
- The script prints a JSON summary with `metaPath` — the `<id>.md` it wrote.

When invoked standalone to **re-title** an existing video, point `--session` at
the same dir (or pass `--library` + the fields directly); writing is idempotent
and overwrites in place with a clean diff.

This skill stops at writing `<id>.md`. Rebuilding the page and deploying are the
caller's next steps (the `/shroom:record` flow runs build-page + deploy after).
