# commands/

Slash commands exposed by the shroom plugin.

- [`setup.md`](setup.md) — **built (M5b).** The `/shroom:setup` onboarding flow
  (silent local-env check → one consolidated install + library-dir approval →
  Cloudflare login/gates/provisioning). Drives the deterministic backend in
  [`scripts/setup/`](../scripts/setup/); see [`SPEC.md`](../SPEC.md) §8.
- `record.md` — **planned (M5c).** Start/stop a recording session, then drive
  title/chapters/publish (the agent orchestration around the deterministic
  recorder in [`scripts/recorder/`](../scripts/recorder/), build-page, and deploy).

Commands hold **judgment** (what to do, when to ask) and consent (every system
mutation = propose → confirm → run, batched into one approval). The exact,
repeatable mechanism lives in `scripts/`. Keep that boundary (see
[`CLAUDE.md`](../CLAUDE.md)).
