---
description: Record your screen → instant local preview, then a permanent unlisted link with auto title, chapters, and a searchable transcript.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Skill, Read, Bash(node:*), Bash(open:*), Bash(ls:*), Bash(${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build/shroom.app/Contents/MacOS/shroom:*)
---

You are running `/shroom:record`. Your job is **orchestration and judgment**: you
launch the native **shim** (the menu-bar "tray") and drive the transcribe, title,
page, and deploy scripts — you never reimplement them. The **shim owns the actual
screen capture and its controls**, and a *human* drives it: the agent can only
*launch* the recorder (`armed`, no ffmpeg yet); the user's click in the menu bar is
what **starts** the capture (the consent boundary, SPEC §4). So you never write the
control fifo and you never start capture yourself — you launch, step back, and come
back once the recording is stopped to title, page, and deploy it. Keep that boundary
(CLAUDE.md, SPEC §4/§7). The shim is **macOS-only**; on first use it must be
compiled by `/shroom:setup`.

`$ARGUMENTS`, if given, is a library-dir override for this recording's `<id>.md`.

## Step 0 — what triggered this turn?

You launch the **shim** and (later) transcription as **harness-tracked background
tasks**, so a turn may begin because one of them just finished. The shim stays alive
while the user records and **quits on its own** once the recording is **stopped**
(finalized + published) **or discarded** — either way its background task completes
and re-invokes you. Branch first:

- **The shim task just completed** → the user finished with the tray. Go to **Step 4
  (name + instant publish)** for the session dir you launched in Step 2. (Step 4
  handles the discarded case — a Discard deletes the session, so there's nothing to
  publish.)
- **A transcription task just completed** → go to **Step 5 (enrich)** for its
  session: the link is already live; you're adding chapters + transcript.
- **Otherwise (a fresh `/shroom:record`)** → first drain any **pending publish**
  from an earlier run (SPEC §6 recovery): glance at the recent recordings,

  ```
  ls -t ~/.shroom/recordings 2>/dev/null
  ```

  (single command, no pipe — eyeball the most recent few yourself) and for each
  check `events.ndjson` for a `published` event **carrying a
  `playbackUrl`** you haven't surfaced yet — if found, tell the user "your last
  recording is live: <url>" and `open` it. Then go to **Step 1 (pick devices)**.

## Step 1 — settings: quality + devices

Preflight first (this only reads, no capture):

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/recorder/record.mjs" --preflight
```

Parse `{ video, audio, defaultAudioName, qualities, lastProfile }`:
- `qualities[]` — `{ key, label, resolution, mbPerMin, refMinutes, refSizeMB,
  refCentsPerMonth }` for `normal` (1080p), `2k`, `4k`. The `ref*` fields describe a
  relatable 10-minute clip: its size and what it costs to keep stored per month.
- `lastProfile` — `{ quality, video, audio }` from the most recent recording, or
  `null` on the first ever run.

### If `lastProfile` exists — offer to reuse it

State it plainly and ask (single `AskUserQuestion`, **Use these** / **Change**):

> Last time: **<quality label>**, video **<lastProfile.video>**, mic
> **<lastProfile.audio>**.

- **Use these** → carry them straight to Step 2 (if `lastProfile.quality` is
  `null` — an older recording — default to `normal`).
- **Change** → fall through to the full picker below.

### The full picker (first run, or on "Change")

Ask **three questions in one `AskUserQuestion`** (one bulk, before recording):

1. **Quality** — one option per `qualities[]` entry. Put the **size + cost** in each
   description, framed around a 10-minute clip so it's relatable, e.g. *"2K (1440p) —
   a 10-min video ≈ 460 MB, ~0.7¢/month to keep stored"*. Note storage is the only
   cost (egress is free), and it's pennies — so pick on quality vs. file size, not
   price. Default to `normal`.
2. **Video source** — the `video` devices, each labelled by `kind` (screen /
   camera). A screen is the usual pick; a camera records camera-as-source (not a PiP
   overlay — deferred). Default to the first screen.
3. **Microphone** — the `audio` devices plus a **"No mic"** option. Pre-select the
   `recommended` one (`defaultAudioName`, the built-in mic) and **steer away from any
   iPhone/Continuity mic** — it drops audio and hitches the video.

Carry the chosen **names** (not indices — they shift) and the quality **key** into
Step 2. The recorder records the choice into its `session_started`, so it becomes
next time's `lastProfile` automatically — no separate save step.

## Step 2 — launch the shim (harness-tracked background task)

You **launch** the recorder; the **user** starts the capture from the menu bar. First
confirm the menu-bar binary is built — it's compiled on-device by `/shroom:setup` and
lives at `${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build/shroom.app/Contents/MacOS/shroom` (gitignored):

```
ls "${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build/shroom.app/Contents/MacOS/shroom"
```

If it's missing, tell the user to run `/shroom:setup` first (it compiles it) and
stop — don't try to record without it.

### Prime permissions (foreground, before the tray)

Run the permissions primer **and wait for it** — a throwaway launch that requests
**Microphone** and registers **Screen Recording** as the **“shroom”** principal (so
prompts read "shroom", never "Terminal"), then exits with one line of JSON:
`{"screen":"granted|prompted","mic":"granted|denied"}`:

```
"${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build/shroom.app/Contents/MacOS/shroom" --permissions
```

The **mic** prompt resolves inline while it runs (a one-line heads-up beforehand is
kind). Then branch on the JSON — **you** own the Screen-Recording gate (the primer no
longer pops a dialog of its own; that stacked confusingly over the system prompt):

- **`screen: "granted"`** → already enabled; go straight to launching the tray.
- **`screen: "prompted"`** → it's the user's **first** record. Screen Recording can't
  be granted from its prompt — the user toggles it in System Settings, and it only
  takes effect on the **next** launch (the tray you're about to start). So:
  1. Open the pane: `open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"`.
  2. Tell the user: turn **ON “shroom”** under Screen Recording.
  3. **Gate with `AskUserQuestion`** ("Enabled Screen Recording for shroom?" — e.g.
     *"Yes, it's on"* / *"Open the settings again"*). Only launch the tray once they
     confirm — otherwise the first capture starts blind.
- **`mic: "denied"`** → fine, they can record without a mic; don't block on it.

Always call it **shroom** to the user — never say "shim" in anything you surface.

Mint the recording's **id** — the unguessable storage/URL key — yourself, so you can
name the session dir after it and the URL key is fixed up front:

```
node -e "console.log(require('crypto').randomBytes(12).toString('base64url'))"
```

Then the **session dir** is `~/.shroom/recordings/<YYYYMMDD-HHMMSS>-<id>` (timestamp
so it sorts/eyeballs nicely, `id` so the dir cross-references the link) — **remember
it**; that's where the control fifo + `events.ndjson` live and what you read in Step
4. Launch it **in the background** so the user stays free to chat and the harness
re-invokes you when the session ends (SPEC §6 — do not block on a long tail), passing
the id + the picker's choices straight through after `--`. Give the background task a
**user-facing label without "shim"** (e.g. *"shroom recorder (menu-bar tray)"*) —
that label is shown to the user when the task completes:

```
"${CLAUDE_PLUGIN_ROOT}/scripts/shim/macos/build/shroom.app/Contents/MacOS/shroom" \
  --recorder "${CLAUDE_PLUGIN_ROOT}/scripts/recorder/record.mjs" \
  --node node \
  -- --out "$HOME/.shroom/recordings/<YYYYMMDD-HHMMSS>-<id>" --id <id> \
     --quality <key> --device "<video name>" --audio "<mic name>"
```

(`--node node` — the recorder only needs Node ≥18, so the user's default node is
fine; the Node ≥22 in creds `nodeBinDir` is only for wrangler.) Everything after
`--` flows unchanged to `record.mjs`; the shim derives the fifo + log from `--out`.

Then tell the user **how to drive the tray** (the menu-bar app, not you, controls capture):

- A shroom icon (**🍄**) appears in the menu bar. **Click it to start** — a 3-2-1
  countdown (cancelable: click again during it to abort), then it records (**●**).
- **Click while recording** → pauses immediately (**❚❚**) and opens a menu.
- The menu offers **Resume**, **Stop** (finalize + publish), **Restart** (throw
  this take away and re-arm a fresh one), and **Discard** (throw it away entirely).

Don't poll or tail in a loop — **end your turn after launching**. It quits on its own
(after the user's Stop or Discard), completing the background task and bringing you back.

## Step 3 — the user stops (or discards)

You do nothing here — the user drives the tray. When they **Stop**, the recorder
finalizes (valid moov, assembled playlist + `preview.mp4`), publishes the bytes if
storage is configured, and exits; the shim then quits, completing the background
task and re-invoking you → **Step 4**. If they **Discard** instead, the session is
deleted (no publish) and the shim still quits — Step 4 detects the missing session
and reports it. If storage isn't set up, the recorder emits `upload_skipped`
(`storage_not_configured`) — fine: it still renders **locally and instantly** (SPEC
§8); note it in Step 4 and offer `/shroom:setup` for the shareable link.

## Step 4 — name it, then publish instantly

The link should be in the user's hands **right after stop**, not gated behind
transcription (whisper can take a while). The page is re-derivable and the URL is
stable, so publish a good page now and let it get richer later.

Work **quietly** here — no "let me check…" narration, and don't probe with a shell
one-liner (a compound `if`/`[ -d ]` trips an approval prompt). Just **Read**
`<dir>/events.ndjson` with the Read tool (not `cat`) and branch on what's there:

- **Read fails / the file is gone** → the user **discarded** the take. Say it was
  discarded, nothing was published, and stop (don't hunt for an older session).
- **only an `aborted` event** (quit while still `armed`, before clicking Start) →
  nothing was captured; say so and stop.
- **`finalized` with `ok: true`** → good: read the `id` from `session_started` (the
  storage/URL key), `open <dir>/preview.mp4` for the instant local preview (SPEC §8),
  and go straight to titling. (Any other `finalized` / an `error` → surface it and stop.)

Then **ask the user how to title it** (`AskUserQuestion`).

Phrase the question so a typed title is a **one-step** answer, not a second turn:
make the question itself *"Title this recording — type your own below, or pick
auto-name"*. `AskUserQuestion` requires **at least two** options, so list these two
— the always-present free-text **"Other"** field is what keeps a typed title
one-step (it's there no matter how many options you list):

- **Auto-name it** — say plainly this reads the recording's transcript with
  whisper, so it takes a few seconds (longer for a long recording). You get a title
  *and* chapters from it (Path B).
- **I'll type my own title** — the affordance for a user-chosen title (Path A).

The **fast path** is the user typing their title **straight into the free-text
("Other") field**: treat that text as the title, take **Path A**, and publish
**immediately**, transcribing in the background for search + chapters. Only if the
user *selects* "I'll type my own title" **without** typing anything do you ask them
for it — one extra turn, and only in that case (never a forced round-trip for a user
who just types).

### Path A — the user gave a title (instant link)

1. Write the record with their title (no transcript needed yet):
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/page/write-meta.mjs" --id <id> --session <dir> --title "<their title>"
   ```
   (Pass `--library <dir>` if `$ARGUMENTS` gave an override.) Capture `metaPath`.
2. **Publish now** (see *Publish* below — it builds, deploys, and commits the
   record in one step) → hand over the link.
3. **Kick off transcription in the background** and end your turn — don't wait:
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/transcribe/transcribe.mjs" --session <dir>
   ```
   Its completion re-invokes you at **Step 5 (enrich)**.

### Path B — auto-name (the user opted to wait)

1. Transcribe in the foreground (the user is waiting):
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/transcribe/transcribe.mjs" --session <dir>
   ```
   If it **soft-skips** (`transcribe_skipped` — no whisper/audio), auto-naming
   isn't possible: tell the user, ask them to type a title, and continue as Path A
   (but there's no transcript to background, so skip Step 5).
2. Invoke the **`title-chapters` skill** (author-from-scratch) with the session
   dir → it authors title + TL;DR + chapters and writes `<id>.md`. Capture
   `metaPath`.
3. **Build + deploy** (below) → hand over the link. No Step 5 needed — the
   transcript is already baked in.

### Publish (both paths)

One command builds the page, deploys it (if Cloudflare is provisioned), **and
commits the `<id>.md` to the git library** — all deterministic, so it's a single
step:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/page/publish.mjs" --session <dir> --meta <metaPath> --title "<title>"
```

It reads `pagesProject` + `library` from `~/.shroom/credentials.json` itself. Read
the terminal event:

- **`published`** → present its `playbackUrl` as the shareable link and `open` it.
  That URL **is** the publish (SPEC §6 — record → link). `committed: true` means the
  record is already saved to the user's library (see *the record is committed
  automatically* below).
- **`publish_local`** → Cloudflare isn't provisioned: say it rendered locally (give
  the `preview` path) and that `/shroom:setup` unlocks the shareable link.

(`--title` is only used for the commit message; pass the user's title when you have
it, omit it on an enrich re-publish that didn't change the title.)

## Step 5 — enrich in the background (Path A only)

Triggered when the background transcription completes. The link is already live;
you're upgrading the same URL in place.

1. Confirm `<dir>/transcript.json` exists (if transcription soft-skipped, there's
   nothing to add — stop quietly).
2. Invoke the **`title-chapters` skill** in **enrich mode**: it adds TL;DR +
   chapters from the transcript and **preserves the user's title** (the writer
   inherits it — the skill omits `--title`). Capture `metaPath`.
3. **Re-publish** the same way (*Publish* above) — omit `--title` so the commit
   message isn't rewritten; the no-op-commit case is handled. The stable URL now
   carries chapters, the transcript, and richer `og:` tags.
4. Tell the user briefly: "added chapters + a searchable transcript to <link>."

## The record is committed automatically

`publish.mjs` commits the `<id>.md` to the git library as its last step (SPEC §3 —
the record is the thing worth keeping). It's automatic and best-effort: a missing or
non-git library is reported (`commit_skipped`) but never fails the publish, and a
re-publish with no metadata change is a no-op (`commit_noop`) — the record stays
committed. Don't ask the user to confirm a commit; it's part of the normal record
lifecycle into their own library.

Keep the final message short: the link (or local path), the title, and — when the
event reported `committed: true` — that it's saved to their library. If you instead
saw `commit_skipped` with `not_a_git_repo`, mention the library isn't under git yet
(`/shroom:setup` initializes it).
