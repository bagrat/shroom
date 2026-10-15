# history/

The decided record: what the build was designed from and how it got here. **Frozen** —
read it for the reasons behind a decision, never as a description of today's code. Live
documents (the spec, the architecture as built, the decisions index, the status, how to
test and release) stay at the repository root; CLAUDE.md maps them.

| Path | What it was |
|---|---|
| `RESEARCH.md` | The pre-build research R1–R11 and the hands-on probe results that chose the stack. Cited as *R* |
| `POC-LEARNINGS.md` | Every hard-won finding from the agent-first POC (v0.1.16): capture, audio, macOS permissions, upload, Cloudflare. **Still worth reading before touching any of those areas.** Cited as *PL* |
| `UX-WALKTHROUGH.md` | The walkthrough the spec was derived from |
| `design/ARCHITECTURE.md` | The architecture as designed (2026-10-02) with its later "as built" notes, rejected alternatives and probe evidence — replaced by the root `ARCHITECTURE.md` |
| `design/WORKER-INTERFACE.md` | The Worker interface as designed (three competing designs, judged) with its amendments — replaced by the root `WORKER-INTERFACE.md` |
| `decisions/` | Decisions B1–B120 in full — the evidence, the proof, the rejected alternatives. The root `DECISIONS.md` indexes them and carries B121 on in full |
| `build-status/` | BUILD-STATUS.md's milestone record (tests, heartbeat, incidents, media truth, spec coverage) and the long waiting items in full |
| `build-log/` | Batch logs moved out of BUILD-STATUS.md |
| `proposals/` | Proposals that were built (or settled) |
| `audits/` | The 2026-10-13 repo audit |
| `spec/` | The spec before B133: `UX-STATEMACHINE.md` (the text, cited as *SM*), `UX-MAP.html` (its drawing), the fixtures drawn from them (`r1–r6.json`, `map.json`, `meta.json`) and their two scripts (`check-fixtures.py`, `extract-map.py`, which read the old `spec/` layout). Replaced 2026-10-14 by `spec/machine.json`, assembled from them and the implementation (`dev/proposals/machine-json/`) |

Line citations into these files (`R:2216`, `RQ:395`, `PL:124`) are as of the move
(2026-10-14) and stay valid: nothing here is edited. The POC's code itself is at tag
`v0.1.5`, under `archived-poc/`.
