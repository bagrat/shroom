# scripts/dashboard/

Deterministic backend for the [`dashboard`](../../skills/dashboard/SKILL.md) skill —
the listing + HTML; the skill owns the overview narration and routes management
actions (the determinism boundary, [`CLAUDE.md`](../../CLAUDE.md)).

## `dashboard.mjs`

Merges the two substrates (SPEC §3): the **git library** (`<id>.md` records — the
canonical title / duration / chapters / link list, via
[`page/lib/metadata`](../page/lib/metadata.mjs)) and the **local recordings dir**
(disk footprint + per-session state, via [`cleanup`](../cleanup/cleanup.mjs)'s
`scanSessions`). Links resolve from creds `pagesBaseUrl`; thumbnails come from each
recording's `poster.jpg` under `~/.shroom/site/<id>/`.

| subcommand | what it does |
| --- | --- |
| `data` | The merged list as JSON: per-item `id`, `title`, `link`, `durationSec`, `chapters`, `mp4`, `createdAt`, `inLibrary`, `local` (or null); plus totals + `prunableBytes`. |
| `build [--out <dir>]` | Render a self-contained static page (cards + thumbnails copied in) to `~/.shroom/dashboard/index.html`; print its path. Open it to browse. |

A recording shows up if it's in the library **or** has a local session — so
locally-only (not-yet-published) takes and library records whose local copy was
pruned both appear. All interpolated text is HTML-escaped.

Tests (`npm test`): the library+local merge (keying, sort, link fallback) and the
HTML render escaping. The filesystem/creds gathering is integration.
