# scripts/version/

Version awareness — the deterministic half; the **command** decides whether/how to
surface anything (the determinism boundary, [`CLAUDE.md`](../../CLAUDE.md)).

## `check.mjs` — "is a newer version published?"

Reads the installed plugin version (from `<root>/.claude-plugin/plugin.json`),
fetches the latest published version (`plugin.json` on shroom's `main` — the
marketplace tracks the repo unpinned, so main is what `/plugin marketplace update`
delivers), semver-compares, and prints a verdict:

```json
{ "ok": true, "local": "0.1.12", "latest": "0.1.13", "updateAvailable": true, "source": "…" }
```

It **never updates anything** — it only reports. **Fail-soft is the contract:**
offline, slow network, a moved file, old Node without `fetch` — any problem yields
`{ ok:true, updateAvailable:false, error:… }` and exit 0, so a check can never block
or slow a record/setup. Flags: `--url`, `--local`, `--timeout` (all for testing).

Used by `/shroom:record` and `/shroom:setup` at the top of a fresh invocation: if
`updateAvailable`, they mention it in one line and carry on; otherwise stay silent.

Tests: `npm test` (semver precedence + fail-soft).
