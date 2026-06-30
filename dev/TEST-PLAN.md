# shroom Tier B + C test walkthrough

A guided manual test of the green-set features, sequenced for a **fresh machine**
(nothing configured — setup is the first real test). Exercised by *using* the plugin
in a clean-user test window (`~/shroom-dev` open in the desktop app). The 🔢 tags map
to the green-set items being verified.

## Agent: how to drive this

When the user asks to walk through the test (or "step by step"):

- Go **one step at a time, in order**. Present the step's **action** + what to look
  for, then **stop and wait** for the user's observed result before continuing.
- After each step, record the outcome (✅ pass / ❌ fail + a note) so progress is
  visible. Keep a running tally; don't lose earlier results across turns.
- On ❌ (or anything surprising): **do not fix it from a test window** — it has no
  working agreement / SPEC / memory. Gather evidence with
  `node /Users/bagrat/shroom-claude-plugin/dev/collect-logs.mjs <id>` and produce a
  handoff block for the dev/fix (repo) window (see `dev/README.md`). Then ask whether
  to continue the walkthrough or pause.
- Steps ⑤–⑨ depend on a recording existing; keep the order.
- After a fix lands, remind the user to `/reload-plugins` in the test window before
  re-testing the failed step.

---

## Phase 0 — record gate (before setup)

- **① Record gate · item 2** — On a fresh machine (no setup yet), run
  `/shroom:record`. **Expect:** it does **not** launch the recorder — it explains
  recording needs a one-time `/shroom:setup` (~5–10 min) and **asks** whether to
  start setup now. Say **no** → it stops cleanly (no device picker, no shim). Saying
  yes would jump straight into Phase 1.

## Phase 1 — setup

- **② Fresh setup · item 9 (first-timer path)** — `/shroom:setup ~/<new-library-name>`
  **Expect:** opens with the **welcome (what shroom is + cost)** — *no plan yet*; then one
  line ("let me check what you've got") + a quick read-only preflight; then it shows the
  plan **once, pre-marked** (all ⬜ for a first-timer — never a blank copy *before* the
  check) and walks you through — tools present, then Cloudflare (OAuth valid, card on
  file), the one manual **R2 token** recreation, provisions a new bucket + Pages project,
  creates the new library.
- **③ Re-run pre-marking · item 9 (the payoff)** — run `/shroom:setup` **again**.
  **Expect:** welcome, then the quick preflight, then the plan shown **once** with steps
  1–5 ✅ (no redundant blank plan first) and jumps to "already set up — bucket `…`, site
  `…`," no re-walk.
- **④ Version/post-update preflight · items 3, 4** — runs silently during the Step 0
  preflight (and atop record).
  **Expect:** no update suggestion (on latest), no "what's new" (item 4 only fires
  after a real version bump — a fresh install just baselines). Nothing shown = correct.

## Phase 2 — record + player

- **⑤ Record · item 5 (capture)** — `/shroom:record`, capture **30–60s with 2–3
  distinct topic shifts** (so chapters generate), Stop, let it publish.
  **Expect:** instant local preview → title prompt → a permanent link. **Note the id.**
- **⑥ Player polish · item 6** — open the live link in a browser.
  **Expect:** clicking the **chapter timeline** chunks seeks + the fill advances;
  **Copy embed** yields a working iframe (or append `?embed=1` for the chrome-less
  player); arrow keys seek, space play/pause, `f` fullscreen.

## Phase 3 — management skills (need ⑤'s recording)

- **⑦ Dashboard · item 7** — *removed.* The static-page dashboard was pulled (its
  `main()` never fired through the skills-dir symlink — same entry-point guard bug now
  fixed for search/cleanup). To be **redesigned as a local server** that renders the
  library; deferred — see backlog `dashboard-as-server`. Skip this step.
- **⑧ Search · item 8** — "which recording covered `<a topic you said>`."
  **Expect:** returns the recording with a snippet + a **chapter jump-time**.
- **⑨ Cleanup + MP4 · item 5** — "add a downloadable MP4," reload the page (Download
  button appears); then "clean up old recordings."
  **Expect:** scans, shows tiers, **asks before** anything; `prune-local` keeps the
  MP4 + the link; **`delete-remote` needs its own explicit yes**, after which the link
  404s.

---

## ⏸ Parked — resume here (2026-06-29)

Stopped after the dashboard work; **⑧ and ⑨ remain**, and there are **no recordings left**, so
both need a fresh capture first. In a new **repo** session (or just the test window for the runs):

1. **Repo state:** the symlink entry-point guard fix (search/cleanup/version) + the dashboard
   removal are **edited but NOT committed** on `green-set-overhaul`. Decide whether to commit
   before/after finishing the test.
2. **Test window** (`~/shroom-dev`): run **`/reload-plugins`** (picks up the script fix, drops the
   removed dashboard skill).
3. **Re-confirm ④** — version preflight was likely a false pass (it silently no-opped through the
   symlink before the fix). On a fresh `/shroom:record` Step 0, check it actually runs now.
4. **Re-capture ⑤** — record 30–60s with 2–3 topic shifts; note the id.
5. **⑧ Search** — "which recording covered `<topic you said>`" → expect snippet + chapter jump-time.
6. **⑨ Cleanup + MP4** — "add a downloadable MP4" (Download button on reload), then "clean up old
   recordings" (asks before anything; `delete-remote` needs its own explicit yes → link 404s).

## Results

| Step | Item | Status | Notes / handoff |
| --- | --- | --- | --- |
| ① Record gate | 2 | ✅ | confirmed before this session |
| ② Fresh setup | 9 | ✅ | confirmed before this session |
| ③ Re-run pre-marking | 9 | ✅ | confirmed before this session |
| ④ Version/post-update | 3,4 | ✅ | confirmed before this session |
| ⑤ Record | 5 | ✅ | host e2e PASS on v0.1.13 (2026-06-30). First run hit the `--node node`→Node<18→"fetch is not defined" upload storm; fixed (run-node launch + graceful upload degrade). Re-run clean. |
| ⑥ Player | 6 | ✅ | confirmed before this session |
| ⑦ Dashboard | 7 | ➖ | feature removed; redesign as a server (backlog `dashboard-as-server`) |
| ⑧ Search | 8 | ⬜ | |
| ⑨ Cleanup + MP4 | 5 | ⬜ | |
