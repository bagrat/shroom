---
name: cleanup
description: Find and remove stale shroom recordings to reclaim disk + storage — list local recordings with sizes, drop heavy local HLS while keeping a watchable MP4, delete a recording locally or from the bucket, or add a downloadable MP4 to a player. Use when the user wants to clean up, free space, prune, or delete recordings.
allowed-tools: AskUserQuestion, Read, Bash(node:*)
---

# Cleanup — prune & delete recordings

You help the user reclaim space without ever destroying something they still want.
This is the **judgment** half (SPEC §7): *you* decide what looks stale and confirm
intent; the deterministic `cleanup.mjs` does the actual listing and removal. Never
hand-delete files yourself — always go through the script, so the safety checks (it
refuses to drop the only copy of a recording) and the path guards apply.

## The mental model — three very different "deletes"

Be precise with the user; these are not interchangeable:

- **Prune local** (safe, recommended) — drop the bulky local HLS bytes (init +
  segments + per-take intermediates) **but keep `preview.mp4`**, the one watchable
  file. The shareable link keeps working (it serves from the bucket). This is the
  default way to reclaim disk on already-published recordings.
- **Delete local** — remove the whole local session dir, `preview.mp4` and all. The
  link still works; you just no longer have a local copy.
- **Delete remote** — delete the bytes from the bucket. **This breaks the public
  link** — anyone who has it gets a 404. Outward and irreversible: confirm hard, and
  never do it as part of a "free up space" batch without singling it out.

## Step 1 — scan

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/cleanup/cleanup.mjs" scan --verify --json
```

Parse `{ sessions: [...], totalBytes, prunableBytes }`. Each session has `id`,
`name`, `dir`, `ageDays`, `published`, `playbackUrl`, `totalBytes`, `prunableBytes`,
`hasPreviewMp4`, `hasLocalHls`, and (with `--verify`) `remoteConfirmed`. `--verify`
HEADs each remote playlist so you know which are safely backed up; drop it (faster,
local-only) if storage isn't configured.

Show a short, scannable summary — newest first, sizes human-readable (MB/GB), age in
days, and whether each is published + backed up. Lead with the **total reclaimable**
(`prunableBytes`) so the user sees the win.

## Step 2 — propose, by safety tier

Use `AskUserQuestion`. Default your recommendation to the **safe** action:

- **Published + `remoteConfirmed` + has local HLS** → offer **prune local** (keeps the
  MP4 and the link). This is the bread-and-butter cleanup; batch these.
- **Unpublished / `remoteConfirmed: false`** → do **not** prune (it may be the only
  copy). Offer to finish publishing, delete-local if it's junk, or leave it.
- **"I want this gone for good"** → spell out delete-local vs delete-remote and which
  consequences apply; get an explicit yes for delete-remote since it kills the link.

Never select a destructive option for the user. One confirmation per destructive op
(or one batched confirmation for a set of equivalent safe prunes).

## Step 3 — execute (one target at a time)

After the user confirms, loop over the chosen sessions:

```
# safe: reclaim disk, keep preview.mp4 + the link (refuses if remote unconfirmed)
node "${CLAUDE_PLUGIN_ROOT}/scripts/cleanup/cleanup.mjs" prune-local --session "<dir>" --json

# remove the whole local dir
node "${CLAUDE_PLUGIN_ROOT}/scripts/cleanup/cleanup.mjs" delete-local --session "<dir>" --json

# delete bucket bytes — BREAKS THE LINK (confirm first)
node "${CLAUDE_PLUGIN_ROOT}/scripts/cleanup/cleanup.mjs" delete-remote --id "<id>" --json
```

- `prune-local` refuses with `reason: "remote_not_confirmed"` unless the upload is
  verified (or `--force`). Don't pass `--force` unless the user knowingly accepts
  losing the only copy.
- Report freed bytes after each. Keep the tally; tell the user the total reclaimed.

## Add a downloadable MP4 (optional)

The player streams HLS; some viewers want a plain file to download. To offer one:

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/cleanup/cleanup.mjs" upload-mp4 --session "<dir>" --json
```

It uploads `preview.mp4` → `<id>/video.mp4` and returns `downloadUrl`. Then make the
**Download** button appear on the page: mark the record and re-publish —

```
node "${CLAUDE_PLUGIN_ROOT}/scripts/page/write-meta.mjs" --id "<id>" --session "<dir>" --mp4
node "${CLAUDE_PLUGIN_ROOT}/scripts/page/publish.mjs" --session "<dir>"
```

(`--mp4` sets the page's download flag; the re-publish re-renders + re-deploys the
same stable URL.) Don't `upload-mp4` after a `delete-local` — there's no `preview.mp4`
left to upload.

## Safety rules (non-negotiable)

- Confirm before any delete; **delete-remote always gets its own explicit yes**.
- Prefer **prune local** when the goal is just disk space.
- Trust the script's refusals — they exist so you never delete the last copy.
