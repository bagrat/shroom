# shroom recorder core

The **deterministic hands** of shroom (SPEC §4). It owns the entire ffmpeg
lifecycle and nothing else: no judgment, no LLM in the real-time path. The agent
orchestrates *around* it (title / chapters / "publish?"); the button/stop path
works even with no agent session live.

```
  control.fifo  ──▶  record.mjs  ──▶  events.ndjson
   (commands in)     (ffmpeg owner)    (events out)
```

One ffmpeg encode is `tee`'d to two outputs (the validated recipe):

- **`stream.m3u8` + `init.mp4` + `seg_NNNNN.m4s`** — HLS / fMP4, the upload
  artifact (M3). Segments stream up incrementally; the playlist is the publish act.
- **`preview.mp4`** — progressive faststart MP4 for instant local `file://`
  playback with zero JS / zero server (SPEC §8 "value before friction").

## Run it

```bash
node record.mjs [--id <id>] [--out <dir>] \
                [--device "Capture screen 0"] \
                [--audio none|default|<name>] \
                [--fifo <path>]
```

Defaults: a random `--id`, `--out` = `~/.shroom/recordings/<id>/`, screen device
resolved **by name** (indices are unstable), audio off.

Stop cleanly:

```bash
echo stop > <dir>/control.fifo      # or: kill -INT / -TERM <pid>
```

`stop` writes `q\n` to ffmpeg stdin → valid `moov` + `ENDLIST`, ~200 ms, exit 0.
SIGTERM/SIGKILL escalation kicks in only if `q` doesn't take.

> **Audio is off by default in M1** to keep testing free of a mic TCC prompt. The
> code path is wired (`--audio default` picks the first mic; `--audio "<name>"`
> picks by name); v1's intended default is screen **+ mic**.

## Control contract (fifo in)

Newline-delimited commands. v1/M1 understands:

| command | effect |
| --- | --- |
| `stop` | finalize and exit (the publish act) |

`pause` / `resume` are reserved for **M2** (segment-boundary pause) — currently
logged as `command_ignored`.

## Event schema (events.ndjson out)

Append-only NDJSON. Every record has `ts` (ISO-8601) and `event`; other fields are
event-specific. This file is also the **durable recovery / pending-publish
artifact** (SPEC §6): it outlives the session and is drained on the next run.

| event | key fields | when |
| --- | --- | --- |
| `session_started` | `id`, `dir`, `screen`, `audio`, `config` | after device resolution |
| `ffmpeg_command` | `argv`, `cwd` | just before spawn (debug aid) |
| `recording_started` | `pid` | ffmpeg spawned |
| `segment_ready` | `index`, `file` | a segment is closed (N+1 began, or sealed at stop) |
| `command_ignored` | `command` | an as-yet-unsupported control command (e.g. `pause`) |
| `stop_requested` | `reason` | `stop` received, or a signal |
| `recording_stopped` | `exitCode` | ffmpeg process exited |
| `finalized` | `id`, `preview`, `playlist`, `initSegment`, `segments`, `segmentIndices`, `segmentCount`, `durationSec`, `endlist`, `ok`, `ffmpegExit` | session summarized |
| `error` | `phase`, `message` | device resolution / spawn / control failure |

`ok` (on `finalized`) is the success signal: `preview` + `initSegment` present,
`endlist` sealed, and ≥1 segment. The recorder process exits `0` iff `ok`.

## Layout

```
record.mjs            CLI entry + orchestration (spawn, stop, watch, finalize)
lib/config.mjs        validated recipe constants + segName()
lib/devices.mjs       resolve avfoundation devices by name
lib/ffmpeg.mjs        build the tee argv
lib/control.mjs       fifo reader (stays open across writer disconnects)
lib/events.mjs        events.ndjson writer (+ stdout echo)
lib/finalize.mjs      summarize a finished session dir
```

Portable Node core (model B: `shim → node → ffmpeg`). The per-OS native control
shim that owns Screen Recording / tray / hotkey layers in later, unchanged.
