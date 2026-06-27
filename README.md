# shroom

An **agent-first, self-hosted Loom alternative**. Record your screen; get a
permanent, unlisted share link with auto title, chapters, and a searchable
transcript — with your videos living in **your own object storage and your own
git**, not someone else's SaaS.

Ship *code, not a service*: there's nothing to host, operate, or support. It
installs as a Claude plugin (BYO storage — Cloudflare R2 by default, any
S3-compatible bucket otherwise).

> **Status: greenfield.** Nothing is built yet. Start from [`SPEC.md`](SPEC.md) —
> the carry-over design spec distilled from the original design thread. The full
> reasoning (and rejected alternatives) lives in Bagrat's `tent` workspace at
> `inbox/shroom-agent-first-loom.md`.

## The shape (see SPEC.md for detail)

- **git = the database**, **R2 = the bytes** (egress free → flat ~$15/TB stored).
- **Local ffmpeg capture** → HLS/fMP4 segments uploaded **file-by-file** during
  recording (no multipart, no remux server).
- **Static per-video player pages** on Cloudflare Pages (per-video `og:` tags for
  link unfurling).
- **The agent is the product**: titles, chapters, transcripts, semantic search,
  edit-as-a-sentence — the stuff Loom charges for.
- **Determinism boundary:** scripts do capture/upload/commit; skills do the
  judgment (titles, chapters, "publish?").
