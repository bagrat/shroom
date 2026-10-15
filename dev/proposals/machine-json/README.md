# The machine-json tools

For the migration in [`../machine-json.md`](../machine-json.md). None of them run in CI.

## Phase 1: assembling `spec/machine.json`

- **[`REPORT.md`](REPORT.md)** — what the JSON changes from the old spec, Bagrat's to-ratify items first.
  Generated; read this rather than the JSON.
- **`record-moves.patch`** — applied to `crates/shroom-core/tests`, every test logs each step
  (`SHROOM_MOVES=<file> cargo test -p shroom-core --tests`) and the three enumerations print
  how they listed each move (`cargo test -p shroom-core --test enumerate -- --nocapture | grep ^MOVE`).
  Never committed applied.
- **`reduce-moves.py <log> <MOVE lines> moves.json`** folds that to **`moves.json`**: every distinct
  move the core made, by surface (the popover and tray, a banner, each window), with its event,
  the arm that ran and the tests that made it. Ids are the code's.
- **`assemble.py`** writes `spec/machine.json` and `assemble-notes.json`. It reads:
  - **`ids.py`** — one id per state: every old name and the id it becomes, the map's ports,
    and the two states the fixtures wrote twice;
  - **`overlay.py`** — the implementation over the spec: the code-only states, shipped copy
    under its deviation rows, DECISIONS' copy table by state, the edges the build changed;
  - **`rules.py`** — the groups and the rules (Bagrat's (b)), with the matcher that holds a
    move to a rule.
  Every recorded move must be an edge or fall under a rule; one that isn't becomes a
  `provisional` edge citing its row.
- **`report.py`** writes `REPORT.md` from the JSON and the notes.

Retired with phase 3: `spec/machine.json` is edited directly now, and the old sources are in
`history/spec/`. `assemble.py` refuses to run (it would overwrite every edit since) unless
given `--rerun-phase-1`.

## From the discussion

- **`probe-canvas.mjs`** generates the probe's overview and region boards from `spec/`.
  Run it as `node probe-canvas.mjs <root> <sha>`, where `<root>/project/` already holds the
  screen boards read from the canvas and a `canvas.json`. It needs `elkjs` (0.9.3).
  The state → board map in it is a guess (and `machine.json`'s `board` field starts from it).
- **`edges.py`** and **`said.py`** are the audit: the code's named edges against the map,
  the fixtures and the alias tables, and every said-line against the shipped app, Worker
  and site text.
- **`enumerate-moves.patch`** is a one-line patch to `crates/shroom-core/tests/enumerate.rs`.
  It prints every move the core makes while the test walks a window.
  **`enumerated_moves.json`** is its output for the onboarding and Settings · Space windows:
  188 moves, 46 drawn, 142 by rule. `record-moves.patch` supersedes it.
