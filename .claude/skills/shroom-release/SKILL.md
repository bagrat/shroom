---
name: shroom-release
description: Cut a shroom release or an early build from a cloud session. Use when asked to release, ship a version (0.1.x), cut or publish an early build or pre-release, or keep a change out of a release.
---

# Releasing shroom from a session

[`docs/RELEASING.md`](../../../docs/RELEASING.md) is the full road: the one-time setup,
every step of "Cutting a release", "Pre-releases" and "Checking a published release".
This is the short version, and the rules a session keeps (Bagrat, 2026-10-19).

## The rules

- **Every release is Actions → Release → Run workflow, with Publish.** An official
  release runs on `main`; an early build (`x.y.z-early.n`, DECISIONS B220) runs on
  `claude/early-integration` with **Early** ticked too. It builds the branch's head. A
  session dispatches it with its GitHub tools.
- **Never a tag from a session.** A cloud session's tag push is refused (HTTP 403). The
  hand-pushed tag road is Bagrat's alone, so never plan on it, and never ask him to push one.
- **To keep a change out of a release, hold its merge** until the release is out. Never
  merge it and then try to pin an older commit.
- **When the usual way looks blocked, stop and say so.** Don't hand Bagrat a workaround
  to run.
- **A release is his call.** Cut one only on his go (relayed by the orchestrator counts).

## The steps (RELEASING.md "Cutting a release")

1. `tools/bump-version.sh --check`: the version the tree is at. If it's already out,
   move it up (`tools/bump-version.sh <next>`).
2. The notes: `docs/release-notes/<version>.md`, product voice, checked with
   `node tools/release-notes.mjs check <version>`. They're product copy: Bagrat reads them.
3. The heartbeat green on the release's commit: E2E full on `main`.
4. Version and notes go to `main` by a merged PR, never a push.
5. CI green on that commit, then Run workflow with Publish (as above).
6. Check the published release (RELEASING.md "Checking a published release") and report
   up with the release link.

For an early build, first bring `main` into `claude/early-integration` by a merge (never a
rebase), then follow "Pre-releases".
