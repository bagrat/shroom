# commands/

Slash commands exposed by the shroom plugin. **Skeleton — populated in later milestones.**

Planned:

- `record.md` — start/stop a recording session, then drive title/chapters/publish
  (the agent orchestration around the deterministic recorder in
  [`scripts/recorder/`](../scripts/recorder/)).
- `setup.md` — the `/shroom:setup` onboarding flow (local-env check → tool install
  → Cloudflare provisioning). See [`SPEC.md`](../SPEC.md) §8.

Commands hold **judgment** (what to do, when to ask). The exact, repeatable
mechanism lives in `scripts/`. Keep that boundary (see [`CLAUDE.md`](../CLAUDE.md)).
