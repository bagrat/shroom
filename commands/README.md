# commands/

Slash commands exposed by the shroom plugin.

- [`setup.md`](setup.md) — **built (M5b).** The `/shroom:setup` onboarding flow
  (silent local-env check → one consolidated install + library-dir approval →
  Cloudflare login/gates/provisioning). Drives the deterministic backend in
  [`scripts/setup/`](../scripts/setup/); see [`SPEC.md`](../SPEC.md) §8.
- [`record.md`](record.md) — **built (M5c-2).** The `/shroom:record` flow:
  launch the recorder as a harness-tracked background task → pause/resume/stop via
  its control fifo → on completion, transcribe → the `title-chapters` skill
  authors `<id>.md` → build-page → deploy → present the link (and drain any
  pending publish from a prior run, SPEC §6). Orchestration around the
  deterministic recorder ([`scripts/recorder/`](../scripts/recorder/)),
  transcribe, [`write-meta`](../scripts/page/write-meta.mjs), build-page, and
  deploy. Titles are authored **automatically** post-stop — no edit-before-publish
  gate yet (editing-as-a-sentence is M5c-3).

Commands hold **judgment** (what to do, when to ask) and consent (every system
mutation = propose → confirm → run, batched into one approval). The exact,
repeatable mechanism lives in `scripts/`. Keep that boundary (see
[`CLAUDE.md`](../CLAUDE.md)).
