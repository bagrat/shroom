# shroom — working agreement

This is the **agent-first shroom**: a self-hosted, BYO-storage Loom alternative
shipped as a Claude plugin. It is **greenfield** — nothing is built yet.

**Start by reading [`SPEC.md`](SPEC.md).** It is the carry-over design spec
(distilled 2026-06-27); it holds the locked decisions, the v1 scope, and the
open questions. Treat decisions marked **(locked)** as settled unless Bagrat
reopens them; treat §11 as the live to-do.

## How to work here

- **Determinism boundary.** Anything exact or repeatable — ffmpeg flags, the
  file-by-file uploader, git commits, parsing — lives in a **script**. Judgment —
  titles, chapters, links, "keep this?", "publish?" — lives in a **skill**. Don't
  blur them.
- **Never silently mutate the machine.** Installing tools, `git init`, anything
  that changes the user's system = propose the exact command, ask, then run.
- **Reviewable changes.** One logical change per commit, phrased so it can be
  reverted cleanly.
- **Secrets stay out of git.** Credentials live in `~/.shroom/` (mode 600), never
  in this repo.

Provenance for the full reasoning (and rejected alternatives): Bagrat's `tent`
workspace, `inbox/shroom-agent-first-loom.md` and `garden/shroom/`.
