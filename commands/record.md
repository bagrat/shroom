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

## Step 0 — drain any pending publish (recovery, SPEC §6)

A recording launched earlier may have finished while no session was live. Before
anything else, glance at the most recent recordings:

```
ls -t ~/.shroom/recordings 2>/dev/null | head -3
```

For each, check `events.ndjson` for a `published` event **carrying a
`playbackUrl`** (the deploy one, not the uploader's URL-less publish). If you find
one you haven't already surfaced this session, tell the user "your last recording
is live: <url>" and `open` it. This is also exactly what happens when the
background recorder you launched **re-invokes you on completion** — if this turn
was triggered by a finished recorder, skip straight to **Step 3 (publish)** for
that session instead of starting a new one.

## Step 1 — start recording (harness-tracked background task)

First-run note: macOS will prompt for **Screen Recording** and **Microphone**
permission the first time — tell the user to approve them; the recorder will fail
device resolution otherwise.

Launch the recorder **in the background** so the user stays free to chat and the
harness re-invokes you when it finishes (SPEC §6 — do not block on a long tail):

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/recorder/record.mjs" --audio default
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

## Step 2 — stop

When the user asks to stop, write `stop` to the fifo. The recorder finalizes
(valid moov, assembled playlist + `preview.mp4`), publishes the bytes if storage
is configured, and exits — which completes the background task and re-invokes you.
Proceed to Step 3 with that session's `dir` and `id`.

## Step 3 — publish (automatic, no edit gate)

Read `<dir>/events.ndjson`: confirm `finalized` with `ok: true` (else surface the
`error`/failure and stop). Then, in order:

1. **Show it immediately.** `open <dir>/preview.mp4` — the local instant preview,
   zero cloud needed (SPEC §8).
2. **Transcribe** (optional, soft-skips without whisper/audio):
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/transcribe/transcribe.mjs" --session <dir>
   ```
3. **Title / TL;DR / chapters** — invoke the **`title-chapters` skill** with the
   session dir. It reads `transcript.json`, authors the metadata, and writes
   `<id>.md` via the deterministic writer. Author it **automatically** — there is
   **no edit-before-publish gate** in this version (editing-as-a-sentence comes
   later). Capture the `metaPath` it reports. If `$ARGUMENTS` gave a library
   override, pass it through to the skill.
4. **Build the page**:
   ```
   node "${CLAUDE_PLUGIN_ROOT}/scripts/page/build-page.mjs" --session <dir> --meta <metaPath>
   ```
5. **Deploy** — only if Cloudflare is provisioned. Read `pagesProject` from
   `~/.shroom/credentials.json`:
   - **present** →
     ```
     node "${CLAUDE_PLUGIN_ROOT}/scripts/deploy/deploy.mjs" --project <pagesProject> --session <dir>
     ```
     Read the `published` event's `playbackUrl`, present the shareable link, and
     `open` it. That URL **is** the publish (SPEC §6 — record → link, no separate
     publish step).
   - **absent** → say the recording is rendered locally (give the `preview.mp4`
     path) and that `/shroom:setup` unlocks the shareable link. Stop here.

## Step 4 — commit the record (propose → confirm → run)

The `<id>.md` lives in the git library (SPEC §3). Committing is a system change,
so propose it, don't do it silently. Offer one command and run it only on the
user's yes:

```
git -C <library> add <id>.md && git -C <library> commit -m "Add recording: <title>"
```

Keep the final message short: the link (or local path), the title, and that the
transcript is committed to their library.
