---
name: dashboard
description: Show a visual library of all shroom recordings — thumbnails, links, durations, and local disk footprint — and act as the entry point for managing them (open, re-title, clean up). Use when the user wants to see, browse, list, or manage their recordings / library / dashboard.
allowed-tools: AskUserQuestion, Skill, Read, Bash(node:*), Bash(open:*)
---

# Dashboard — browse & manage the library

You give the user one place to *see* everything they've recorded and route them to
the right management action. The deterministic `dashboard.mjs` does the listing and
HTML; you provide the overview and the judgment about what to do next.

## Step 1 — build + open

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/dashboard/dashboard.mjs" build
```

It merges the git library (`<id>.md` records — titles, durations, chapters, links)
with the local recordings (disk footprint + per-session state) and writes a
self-contained static page (thumbnails copied in) to `~/.shroom/dashboard/index.html`.
Read the JSON (`{ path, count, library }`), then **offer to open it** — it's an
outward-ish action, so one line then `open "<path>"`:

```
open "$HOME/.shroom/dashboard/index.html"
```

If `count` is 0, say the library's empty and point them at `/shroom:record` instead
of opening a blank page.

## Step 2 — summarize + offer actions

You can also read the structured list directly:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/dashboard/dashboard.mjs" data --json
```

Each item has `id`, `title`, `link`, `durationSec`, `chapters`, `mp4`, `createdAt`,
and `local` (`{ dir, totalBytes, prunableBytes, hasPreviewMp4, published }` or null).
The top-level carries `prunableBytes` — the disk you could reclaim right now.

Give a short overview (how many, total on disk, how much reclaimable) and offer the
natural next actions — don't do them silently:

- **Reclaim disk / delete a recording** → hand off to the **`cleanup` skill** (it
  owns the prune/delete consent gates). If `prunableBytes` is sizeable, mention it.
- **Re-title / re-chapter one** → hand off to the **`title-chapters` skill** with
  that recording's session dir (or `--library` + `--id`).
- **Open a recording** → `open "<link>"` for its public page.
- **Add a downloadable MP4** → that's the cleanup skill's `upload-mp4` flow.

Keep it a launchpad: show the picture, then route. Don't reimplement cleanup or
titling here — invoke those skills so their safety rules apply.
