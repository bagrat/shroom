# hooks/

Plugin lifecycle hooks (auto-discovered from `hooks/hooks.json`).

- **`not-configured-nudge.mjs`** — a `SessionStart` hook. shroom can be installed
  long before it's configured, and there's no `onInstall` event, so this is the
  first-use detection pattern: if `~/.shroom/credentials.json` is absent it injects
  a one-line note suggesting `/shroom:setup`. It **only suggests, never acts**
  (working agreement), stays silent once configured, fires only on a fresh session
  entry (`startup`/`resume`, never `clear`/`compact`), and is fail-soft (any error
  exits 0 with no output) since it runs at the start of every session.

Hooks are the imperative seam; judgment about *whether/how* to surface the nudge is
left to the agent reading the injected context (it's phrased as guidance, not a
command). See [`CLAUDE.md`](../CLAUDE.md) for the determinism boundary.
