# 🍄 shroom

**Like Loom — but the videos are yours.**

shroom is an agent-first, self-hosted screen recorder that lives in [Claude
Code](https://code.claude.com). Record your screen and get a permanent, unlisted
share link with an auto title, chapters, and a searchable transcript — with the
videos living in **your own object storage and git**, not someone else's SaaS.

It ships *code, not a service*: there's nothing to host, operate, or support.

🔗 **[bagrat.github.io/shroom](https://bagrat.github.io/shroom/)**

## Why shroom

- **Yours, end to end** — recordings land in your own Cloudflare R2 and a git
  library you control. Uninstall tomorrow and your videos stay.
- **Auto title, chapters, transcript** — a *local* Whisper pass enriches every
  recording. Nothing is sent to a third party to do it.
- **Permanent unlisted links** — stop recording and you get a stable, unguessable
  URL with a JS-free social unfurl. Share it like any Loom link.
- **Agent-first** — drive it by talking to Claude Code: `/shroom:record`, click the
  menu-bar tray, done.
- **Pennies to run** — you pay only for storage, and R2 egress is free. A 10-minute
  clip costs a fraction of a cent per month to keep.
- **Open source** — MIT-licensed and readable. The recorder, uploader, and player
  are plain scripts you can inspect and fork.

## Requirements

- **macOS** — recording uses a native menu-bar shim (compiled on-device during setup).
- **[Claude Code](https://code.claude.com)**.
- **A Cloudflare account** (the free tier is enough) for R2 storage + Pages hosting.
- Setup installs the rest for you, with consent: `ffmpeg`, `git`, `wrangler`, and
  Node ≥22. `whisper` is optional — it powers titles, chapters, and search, and you
  can add it later.

## Install

```
/plugin marketplace add bagrat/claude-plugins
/plugin install shroom
```

## Set up (one-time)

```
/shroom:setup
```

Checks your local tools, installs anything missing, and provisions your Cloudflare
R2 bucket and Pages site. Every system change — installing a tool, `git init`,
logging in, creating cloud resources — is **proposed, confirmed, then run**; shroom
never silently mutates your machine, and credentials live in `~/.shroom/` (never in
git). It's idempotent, so it's safe to re-run.

## Record

```
/shroom:record
```

Pick quality + devices, then a 🍄 icon appears in your menu bar. Click to start
(3-2-1 countdown), click again to pause, and **Stop** to finalize. Your shareable
link is ready the moment you stop — chapters and the transcript fill in right after.

## How it works

- **git is the database, R2 is the bytes.** Recording metadata is a committed
  `<id>.md`; the video segments live in your bucket (egress is free).
- **Local ffmpeg capture** → HLS/fMP4 segments uploaded **file-by-file** as you
  record. No multipart, no remux server.
- **Static per-video player pages** on Cloudflare Pages, with baked per-video `og:`
  tags so links unfurl in Slack/iMessage/etc. without running JS.
- **Determinism boundary:** deterministic scripts do capture / upload / commit;
  Claude skills do the judgment — titles, chapters, "publish?".

See [`SPEC.md`](SPEC.md) for the full design and the locked decisions.

## License

[MIT](LICENSE).
