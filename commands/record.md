---
description: Record your screen → instant local preview, then a permanent unlisted link with auto title, chapters, and a searchable transcript.
argument-hint: "[library-dir]"
allowed-tools: AskUserQuestion, Skill, Read, Bash(node:*), Bash(echo:*), Bash(open:*), Bash(cat:*), Bash(ls:*), Bash(git:*)
---

You are running `/shroom:record`. Your job is **orchestration and judgment**: you
drive the deterministic recorder, transcribe, title, page, and deploy scripts —
you never reimplement them. The recorder owns ffmpeg; the agent owns the session
*around* it (start, "stop?", title/chapters, "publish"). Keep that boundary
(CLAUDE.md, SPEC §4/§7).

`$ARGUMENTS`, if given, is a library-dir override for this recording's `<id>.md`.

## Step 0 — what triggered this turn?

You launch the recorder and (later) transcription as **harness-tracked background
tasks**, so a turn may begin because one of them just finished. Branch first:

- **A recorder task just completed** → that recording stopped. Go to **Step 4
  (name + instant publish)** for its session.
- **A transcription task just completed** → go to **Step 5 (enrich)** for its
  session: the link is already live; you're adding chapters + transcript.
- **Otherwise (a fresh `/shroom:record`)** → first drain any **pending publish**
  from an earlier run (SPEC §6 recovery): glance at the recent recordings,

  ```
  ls -t ~/.shroom/recordings 2>/dev/null | head -3
  ```

  and for each check `events.ndjson` for a `published` event **carrying a
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

## Step 2 — start recording (harness-tracked background task)

First-run note: macOS will prompt for **Screen Recording** and **Microphone**
permission the first time — tell the user to approve them; the recorder will fail
device resolution otherwise.

Launch the recorder **in the background** with the chosen settings so the user stays
free to chat and the harness re-invokes you when it finishes (SPEC §6 — do not
block on a long tail):

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/recorder/record.mjs" --quality <key> --device "<video name>" --audio "<mic name>"
```

The recorder echoes its events to stdout. Read the **`session_started`** line and
capture its `id` and `dir` — that's this recording's storage key and where the
control fifo + `events.ndjson` live. If instead you see an `error` event (e.g.
device resolution), surface it and stop.

If storage isn't set up yet you'll see `upload_skipped`
(`storage_not_configured`) — that's fine: the recording still renders **locally
and instantly** (value before friction, SPEC §8). Note it; offer `/shroom:setup`
later for the shareable link. Don't block recording on it.

Then tell the user it's recording and how to control it — they say it in chat, you
translate to the fifo:

- **pause** → `echo pause  > <dir>/control.fifo`
- **resume** → `echo resume > <dir>/control.fifo`
- **stop** → `echo stop   > <dir>/control.fifo`

`stop` is the finalize act. Don't poll or tail in a loop — end your turn after
starting; the user's "stop" (or the background task completing) brings you back.

## Step 3 — stop

When the user asks to stop, write `stop` to the fifo. The recorder finalizes
(valid moov, assembled playlist + `preview.mp4`), publishes the bytes if storage
is configured, and exits — which completes the background task and re-invokes you.
Proceed to Step 4 with that session's `dir` and `id`.

## Step 4 — name it, then publish instantly

The link should be in the user's hands **right after stop**, not gated behind
transcription (whisper can take a while). The page is re-derivable and the URL is
stable, so publish a good page now and let it get richer later.

Read `<dir>/events.ndjson`: confirm `finalized` with `ok: true` (else surface the
`error`/failure and stop). Then `open <dir>/preview.mp4` — the instant local
preview (SPEC §8). Now **ask the user how to title it** (`AskUserQuestion`).

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
2. **Build + deploy now** (see *Publish* below) → hand over the link.
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

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/page/build-page.mjs" --session <dir> --meta <metaPath>
```

Then deploy **only if Cloudflare is provisioned** — read `pagesProject` from
`~/.shroom/credentials.json`:

- **present** →
  ```
  node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy/deploy.mjs" --project <pagesProject> --session <dir>
  ```
  Read the `published` event's `playbackUrl`, present the shareable link, and
  `open` it. That URL **is** the publish (SPEC §6 — record → link).
- **absent** → say it rendered locally (give the `preview.mp4` path) and that
  `/shroom:setup` unlocks the shareable link.

## Step 5 — enrich in the background (Path A only)

Triggered when the background transcription completes. The link is already live;
you're upgrading the same URL in place.

1. Confirm `<dir>/transcript.json` exists (if transcription soft-skipped, there's
   nothing to add — stop quietly).
2. Invoke the **`title-chapters` skill** in **enrich mode**: it adds TL;DR +
   chapters from the transcript and **preserves the user's title** (the writer
   inherits it — the skill omits `--title`). Capture `metaPath`.
3. **Re-build + re-deploy** the same way (*Publish* above). The stable URL now
   carries chapters, the transcript, and richer `og:` tags.
4. Tell the user briefly: "added chapters + a searchable transcript to <link>."

## Step 6 — commit the record (propose → confirm → run)

Once the record is complete (after Step 5 in Path A, after publish in Path B), the
`<id>.md` in the git library is the thing worth keeping (SPEC §3). Committing is a
system change — propose it, don't do it silently. Offer one command, run it only
on the user's yes:

```
git -C <library> add <id>.md && git -C <library> commit -m "Add recording: <title>"
```

Keep the final message short: the link (or local path), the title, and that the
transcript is committed to their library.
