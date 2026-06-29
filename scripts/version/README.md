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

## `post-update.mjs` — "the plugin was just updated; do the follow-ups"

Pairs with `check.mjs` (that *suggests* updating; this runs *after* an update lands).
Compares the installed version against a last-seen marker in
`~/.shroom/version-state.json`, looks up the per-version entries in
[`migrations.json`](migrations.json), and reports the ones newly crossed —
`(lastSeen, installed]`, ascending:

```json
{ "ok": true, "from": "0.1.12", "to": "0.1.13", "firstRun": false,
  "pending": [ { "version": "0.1.13", "whatsNew": "…", "actions": [] } ] }
```

It **advances the marker as it reports**, so each version's entry surfaces exactly
once — nag-proof and idempotent. First ever run baselines to the installed version
(`firstRun: true`) instead of replaying all history. Like `check.mjs` it only
reports and is fully fail-soft (`{ ok:true, pending:[] }`, exit 0, on any problem).
Flags: `--no-advance`, `--manifest`, `--state` (testing).

### `migrations.json` schema

```json
{ "migrations": [ { "version": "0.1.13", "whatsNew": "one changelog line",
                    "actions": [ { "kind": "rebuild-shim", "why": "…", "command": "node …" } ] } ] }
```

- `whatsNew` — a short line the command relays to the user.
- `actions` (optional) — recommended follow-ups. Each carries a `command`; the
  command layer **proposes → asks → runs** it (working agreement — never auto-mutate
  the machine). Omit / leave empty when an update needs no machine-side step (new
  hooks/skills/scripts don't).

Tests: `npm test` (semver precedence, fail-soft, the migration range + advance).
