---
description: "Index of shroom's slash commands — documentation, not itself a command (plugin.json `commands` lists the real ones, so this file is never loaded as a command)."
---

# commands/

Slash commands exposed by the shroom plugin.

- [`setup.md`](setup.md) — **built (M5b).** The `/shroom:setup` onboarding flow
  (silent local-env check → one consolidated install + library-dir approval →
  Cloudflare login/gates/provisioning). Drives the deterministic backend in
  [`scripts/setup/`](../scripts/setup/); see [`SPEC.md`](../SPEC.md) §8.
- [`record.md`](record.md) — **built (M5c-2).** The `/shroom:record` flow:
  launch the recorder as a harness-tracked background task → pause/resume/stop via
  its control fifo → on stop, **ask the user to name it or auto-name**. A typed
  title publishes the **link instantly** (no whisper wait) and transcription runs
  in the **background**, then the flow enriches the *same* stable URL with chapters
  + transcript; auto-name waits for whisper, then titles from the transcript.
  Drains any pending publish from a prior run (SPEC §6). Orchestration
  around the deterministic recorder ([`scripts/recorder/`](../scripts/recorder/)),
  transcribe, [`write-meta`](../scripts/page/write-meta.mjs), build-page, and
  deploy.

Commands hold **judgment** (what to do, when to ask) and consent (every system
mutation = propose → confirm → run, batched into one approval). The exact,
repeatable mechanism lives in `scripts/`. Keep that boundary (see
[`CLAUDE.md`](../CLAUDE.md)).
