# The machine as one JSON, the canvas as its view

Bagrat's call, 2026-10-14 (DECISIONS B133), from a design discussion.
This file is the migration plan. The tools from the discussion sit beside it in
[`machine-json/`](machine-json/).

## What changes

Today the machine lives in three hand-kept places, and nothing makes them agree:

- **The text**, `spec/UX-STATEMACHINE.md`.
- **The map**, `spec/UX-MAP.html` → `spec/map.json`.
- **The fixtures**, `spec/r1–r6.json` and `spec/meta.json`, extracted from the text once by
  agents. `tools/check-fixtures.py` checks their lines against the text.

The tests read the JSON, and alias tables in four places bridge the names.

After the migration:

- **`spec/machine.json` is the machine, and the spec.** It holds every state, edge, rule,
  said-line and law. Sessions edit it; Bagrat never does by hand.
- **The canvas is its view.** A generated column on the
  [shroom UI](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg) canvas holds an overview
  board, then one flow board per region, top to bottom (it is read on a phone). Each
  state is a node that embeds its real screen board live, and the arrows and labels come
  from the JSON. The screen boards stay hand-designed.
- **A readable `spec/MACHINE.md` is generated** from the JSON, for reading and for PR
  review. Nobody edits it.

**Two lanes**:

| Change | Path |
|---|---|
| Behaviour or words (a state, an edge, a rule, a said-line) | A comment on the canvas → a session edits `machine.json` in a PR → after merge, the canvas is regenerated → code |
| Look only (layout, colour, spacing, where a line sits) | Edit the screen board on the canvas (design first, as today) → code. No JSON change |

Where the lanes meet:

- Each said-line on a screen board carries its id (`data-said="paused.title"`).
- Regeneration patches those texts from the JSON.
- A line retyped on the canvas is reported as drift, never silently overwritten. It is
  either proposed into the JSON or reverted.

**The probe** proved the canvas can do this: a throwaway "shroom machine probe"
artifact (not public).

- The screens embed live (`<dc-import>`, scaled).
- A look-only edit on a screen board shows in the flow board at once.
- It reads on a phone.

The state → board mapping there is a guess. In the real thing it is a field of each state.

## Built from the implementation, not the spec

Bagrat: build the machine from what the app actually does, unless the two are exactly
level. The audit (2026-10-14, [`machine-json/`](machine-json/)) says they are close, not
level.

**The checks are green**:

- 425/425 said-lines verbatim in the spec.
- `cargo test -p shroom-core --tests`: 248 passed.

**States**:

- The code can show 113 ids, the fixtures have 138, the map has 83. Only 53 appear in all
  three under the same name.
- 3 states exist only in the code: `restart-confirm` (#44), `wizard-unexpected` (#25) and
  `owner-mac-removed` (#21).
- 15 map states go by another name. The four alias tables (`support::map_names`,
  `edges.rs::starts`, `region6.rs::alias`, `enumerate.rs::names`) miss 4 of them.
- The id `stopping` is duplicated in r3 and r4.

**Said-lines**:

| Result | Count |
|---|---|
| Shipped verbatim | 301 |
| Template values filled | 84 |
| Composed from parts | 22 |
| Flagged | 18 |

About 6 of the flagged are real changes, all logged:

- "Camera only" (#43).
- "Apple silicon", without "and Intel".
- The masked invite link (#22).
- The two-line "Getting shroom…" (#13).
- The agent's parting line (#50).

About 32 shipped lines are not in the spec. All are logged except 2 singular forms of
logged plurals.

**Edges**:

- **The code has no transition table.** 82 moves are named in `apply.rs` (`Arm.edge`);
  every other move falls out of `derive`.
- Enumerating the onboarding and Settings · Space windows gave 188 distinct moves. Only 46
  are drawn edges; **142 rest on prose rules**:

  | Rule | Moves |
  |---|---|
  | N42 offline | 59 |
  | N19/N21 ✕-close | 36 |
  | #25 | 20 |
  | N18 | 9 |
  | N13 | 7 |
  | N14 | 4 |
  | N36 | 3 |
  | N17 | 3 |
  | Checking beat | 1 |

- Two map edges are unbuilt or changed: `landing→r4demo` (#29) and `owner-page→fully-live`
  (#46a).
- Deviation rows #33–35 and #37 are cited but never written.

So the machine is assembled. The spec supplies its states, words and prose, the code
overrides wherever they differ, and the edges come from running the code, because they
can't be read off it.

## Rules, not arrows, for moves that hold across many states

Bagrat's pick is (b). A move that holds for a whole group of states is one **rule** in the
JSON, not dozens of edges. Example: "✕ closes the window from any wizard step", or "any
step can go offline and resumes".

- The canvas draws a rule as a labelled band around its group's states.
- The tests expand every rule into its moves and check each one.

Nothing is lost, and the boards stay readable.

## The shape (a starting point, to settle in phase 1)

```jsonc
{
  "laws": [...], "ledger": [...], "notation": {...},          // today's meta.json
  "regions": [{ "id": 3, "title": "the permissions chapter and the tray" }],
  "groups": [{ "id": "wizard-steps", "region": 2, "states": ["account", "token", "card", "name", "wiring"] }],
  "states": {
    "paused": {
      "region": 3, "surface": "popover", "status": "ratified",          // or "provisional"
      "board": "Paused",                                                 // its screen board, or null
      "summary": "...", "entry": "...",
      "said": [{ "id": "paused.title", "text": "Paused — 4:32", "role": "title", "copy": "ratified", "note": "the length varies" }],
      "actions": [...],
      "cites": ["N39"]
    }
  },
  "edges": [{ "from": "paused", "to": "restart-confirm", "kind": "flow", "label": "Restart", "trigger": "...", "status": "provisional", "cites": ["#44"] }],
  "rules": [{ "id": "close-wizard", "group": "wizard-steps", "to": "idle-unprovisioned", "kind": "side", "label": "✕ closes the window", "cites": ["N19", "N21"] }],
  "ports": [...]
}
```

Rules for the shape:

- **One id per state**, used by the code, the tests and the canvas alike. The alias tables
  go.
- **Every departure from the old spec is marked.** It keeps a `provisional` status and
  `cites` its deviation row, so ratifying stays Bagrat's.

## Phases (one PR each; each keeps CI green)

1. **Assemble `spec/machine.json`.**
   - Write a one-off script that merges `r*.json`, `map.json` and `meta.json`, unifies the
     ids and resolves `stopping`.
   - Overlay the implementation: the 3 code-only states, the shipped copy wherever a
     logged deviation changed it, and the edges and rules from enumerating every window.
     The edges come from `machine-json/enumerate-moves.patch` run over all suites, not
     just two windows.
   - Ship the merge script and a report listing every place the JSON differs from the old
     spec, each with its citation. Bagrat reads the report, not the JSON.
   - Nothing reads the new file yet.
2. **Point the tests at it.**
   - `crates/shroom-spec` loads `machine.json`, and the Worker, site and installer tests
     read their said-lines from it.
   - The edge checks become: every `Arm.edge` and every enumerated move is an edge or
     falls under a rule. That closes the 142-move gap for good.
   - The alias tables are deleted.
   - A new check: every shipped copy string is a said-line or listed as not-said.
3. **Retire the old sources.**
   - `spec/MACHINE.md` is generated from the JSON (`tools/render-machine-md.py`), and CI
     fails if it is stale.
   - `UX-STATEMACHINE.md`, `UX-MAP.html`, `r*.json`, `map.json`, `meta.json`,
     `check-fixtures.py` and `extract-map.py` move to `history/` or go.
   - Update CLAUDE.md, `docs/DESIGN.md`, `docs/TESTING-PLAN.md` and `docs/ARCHITECTURE.md`:
     *the spec* now means `machine.json`.
4. **Generate the canvas.**
   - `tools/render-machine-canvas.mjs`, grown from `machine-json/probe-canvas.mjs` (elkjs,
     a pinned devDependency), writes the overview and region boards. Rules are drawn as
     bands.
   - Add a "The machine" column to shroom UI.
   - Tag the said-lines on the screen boards with `data-said`, and report drift.
   - `docs/DESIGN.md` gains the two lanes and the regeneration steps (after every merge
     that touches `machine.json`).
5. **The change loop.**
   - A PR that touches `machine.json` carries a readable change list
     (`tools/machine-diff.py`) and a before/after picture of the regions it touches.
   - After it merges, the session regenerates the canvas.

## Risks

- **A large canvas.** It has 83+ nodes with live embeds. The probe has 6 regions and 40
  embedded screens; watch how it pans on a phone.
- **History cites old paths.** `history/` cites the old files' lines. They are frozen and
  read at their commit, so that's fine.
- **The deviations workflow carries on.** A provisional state or edge in the JSON cites its
  row, as code does today.
