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

## Phase 0 — before setup

- **① Nudge · item 2** — Open a **new** test window (don't run setup yet).
  **Expect:** the assistant proactively mentions you can run `/shroom:setup`, once.
  Silent once configured.

## Phase 1 — setup

- **② Fresh setup · item 9 (first-timer path)** — `/shroom:setup ~/<new-library-name>`
  **Expect:** opens with the **welcome + plan (all steps ⬜)** *first*; then one line
  ("let me check what you've got") + a quick read-only preflight; then it **re-renders
  the plan still all ⬜** (nothing done yet) and walks you through — tools present, then
  Cloudflare (OAuth valid, card on file), the one manual **R2 token** recreation,
  provisions a new bucket + Pages project, creates the new library.
- **③ Re-run pre-marking · item 9 (the payoff)** — run `/shroom:setup` **again**.
  **Expect:** welcome + plan, then after the quick preflight it **re-renders the plan
  with steps 1–5 ✅** and jumps to "already set up — bucket `…`, site `…`," no re-walk.
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

- **⑦ Dashboard · item 7** — "show my library."
  **Expect:** builds + opens a card page — thumbnail, duration, a **live** link, disk
  footprint.
- **⑧ Search · item 8** — "which recording covered `<a topic you said>`."
  **Expect:** returns the recording with a snippet + a **chapter jump-time**.
- **⑨ Cleanup + MP4 · item 5** — "add a downloadable MP4," reload the page (Download
  button appears); then "clean up old recordings."
  **Expect:** scans, shows tiers, **asks before** anything; `prune-local` keeps the
  MP4 + the link; **`delete-remote` needs its own explicit yes**, after which the link
  404s.

---

## Results

| Step | Item | Status | Notes / handoff |
| --- | --- | --- | --- |
| ① Nudge | 2 | ⬜ | |
| ② Fresh setup | 9 | ⬜ | |
| ③ Re-run pre-marking | 9 | ⬜ | |
| ④ Version/post-update | 3,4 | ⬜ | |
| ⑤ Record | 5 | ⬜ | |
| ⑥ Player | 6 | ⬜ | |
| ⑦ Dashboard | 7 | ⬜ | |
| ⑧ Search | 8 | ⬜ | |
| ⑨ Cleanup + MP4 | 5 | ⬜ | |
