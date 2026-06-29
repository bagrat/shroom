# shroom — working agreement

The **agent-first shroom**: a self-hosted, BYO-storage Loom alternative shipped as a Claude
plugin. Record→publish, setup, and distribution are built and shipping (v0.1.12); see memory
`build-roadmap` to resume and `backlog` for deferred work.

**Read [`SPEC.md`](SPEC.md)** for the carry-over design spec — locked decisions, v1 scope, open
questions. Treat **(locked)** as settled unless Bagrat reopens it.

## How to work here

- **Determinism boundary.** Exact/repeatable work (ffmpeg flags, the uploader, git commits,
  parsing) lives in a **script**; judgment (titles, chapters, links, "keep this?", "publish?")
  lives in a **skill**. Don't blur them.
- **Never silently mutate the machine.** Installing tools, `git init`, anything that changes the
  user's system = propose the exact command, ask, then run.
- **Reviewable changes.** One logical change per commit, phrased so it reverts cleanly.
- **Secrets stay out of git.** Credentials live in `~/.shroom/` (mode 600), never in this repo.

Full reasoning + rejected alternatives: Bagrat's `tent` workspace
(`inbox/shroom-agent-first-loom.md`, `garden/shroom/`).
