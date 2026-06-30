# scripts/cleanup/

Deterministic backend for the [`cleanup`](../../skills/cleanup/SKILL.md) skill —
listing + removal mechanism only; the skill owns the judgment ("which is stale?
keep it?") and the consent gates (the determinism boundary, [`CLAUDE.md`](../../CLAUDE.md)).

## `cleanup.mjs`

| subcommand | what it does |
| --- | --- |
| `scan [--verify]` | List local recordings (`~/.shroom/recordings/*`) with id, age, published state, sizes, and what's reclaimable. `--verify` also HEADs each remote playlist. |
| `prune-local --session <dir>` | Drop the bulky local HLS (init + `seg_*.m4s` + per-take intermediates) but **keep `preview.mp4`**. Refuses unless the remote copy is confirmed (or `--force`) — never deletes the only copy. |
| `delete-local --session <dir>` | Remove a whole local session dir. |
| `delete-remote --id <id>` | Delete every `<id>/*` object from the bucket (SigV4 `DELETE`). **Breaks the public link.** |
| `upload-mp4 --session <dir>` | Upload `preview.mp4` → `<id>/video.mp4`; print the public `downloadUrl` (for the player's Download button). |
| `archive-local --session <dir>` | The record flow's automatic post-stop step: `upload-mp4` **and** `prune-local` in one call. Each half is best-effort and independently gated (upload needs storage; prune needs the remote HLS confirmed), so a local-only recording keeps every byte. Returns `{ mp4, prune }`. |

All print JSON; mutating ops act on exactly one target — the skill passes it after
the user confirms, or the record flow hands over the just-recorded session for
`archive-local`. There is no "delete everything" sweep. Mutating paths are guarded to
stay under `~/.shroom/recordings`. The remote ops reuse the byte-verified SigV4
signer + `deletePrefix`/`putObject`/`headObject` in
[`../uploader/lib/s3.mjs`](../uploader/lib/s3.mjs) (which gained `deleteObject`,
`listObjects`, `deletePrefix`, `parseListObjectsV2`).

Tests (`npm test`): the ListObjectsV2 XML parse, prunable-file classification, and
the local scan over a synthetic tree. The networked ops are integration (need real
storage).
