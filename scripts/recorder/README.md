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
                [--device "<screen or camera name>"] \
                [--audio none|default|<name>] \
                [--fifo <path>] [--no-upload]

node record.mjs --list-devices    # JSON device catalogue for the picker (no capture)
```

Defaults: a random `--id`, `--out` = `~/.shroom/recordings/<id>/`, video source
`Capture screen 0`, audio off.

`--device` names **any** avfoundation video source — a screen (`Capture screen 0`)
**or** a camera (`FaceTime HD Camera`); resolved **by name** (indices are unstable,
a Continuity Camera connecting shifts them). A camera is recorded *as the source*
(camera-only), not a PiP overlay — PiP is deferred (SPEC §4). `--audio default`
prefers a **built-in** mic and **never** the wireless **iPhone/Continuity mic**
(it drops samples and, sharing one capture session, hitches the video too). The
`/shroom:record` command surfaces `--list-devices` as a one-shot picker so the user
chooses video source + mic before recording.

### Upload (optional, M3)

If S3 storage is configured (`~/.shroom/credentials.json` or `SHROOM_S3_*` env),
the recorder streams each closed segment to the bucket during recording and
publishes the playlist at `/stop` — see [`../uploader/`](../uploader/). It's
fail-safe: enqueue is non-blocking and **recording never waits on the network**.
Until storage is set up (or with `--no-upload`), the recording still renders
locally and instantly (`preview.mp4`) — value before friction (SPEC §8).

Control it via the fifo (or signals):

```bash
echo pause  > <dir>/control.fifo
echo resume > <dir>/control.fifo
echo stop   > <dir>/control.fifo    # or: kill -INT / -TERM <pid>
```

`stop`/`pause` write `q\n` to ffmpeg stdin → valid `moov` + per-take `ENDLIST`,
~200 ms, exit 0. SIGTERM/SIGKILL escalation kicks in only if `q` doesn't take.

### Pause/resume = segment boundary (M2)

A pause is a clean **segment-boundary cut**, not SIGSTOP (which drifts a-v / leaves
dead air — rejected in SPEC §4). Each recording run between pauses is a **take**:

- A pause `q`-stops the current take's ffmpeg; resume spawns a new ffmpeg take with
  `-start_number` set to the next global segment index, so segment filenames stay
  **contiguous across takes** (`seg_00000`, `seg_00001`, … never collide/gap).
- The `init.mp4` is **byte-identical** across takes (validated), so the whole
  session shares one — playback across the join needs only an `#EXT-X-DISCONTINUITY`.
- At finalize, the master `stream.m3u8` is **assembled** from each take's playlist
  (one `EXT-X-MAP`, a `DISCONTINUITY` between takes, one `ENDLIST`), and the
  per-take `preview_<k>.mp4` files are concatenated into one `preview.mp4`.

> **Warmup caveat:** avfoundation takes ~1 s to deliver its first frame, so each
> take (including the first) loses ~1 s of content at its head. Inherent to
> restart-based capture; the SIGSTOP alternative was rejected for worse artifacts.

> **Audio is off by default in M1** to keep testing free of a mic TCC prompt. The
> code path is wired (`--audio default` picks the first mic; `--audio "<name>"`
> picks by name); v1's intended default is screen **+ mic**.

## Control contract (fifo in)

Newline-delimited commands, serialized so they never interleave:

| command | effect |
| --- | --- |
| `pause` | end the current take at a clean segment boundary |
| `resume` | start a new take with contiguous segment numbering |
| `stop` | finalize and exit (the publish act) |

Anything else → `command_ignored`. `pause` while paused / `resume` while
recording are no-ops.

## Event schema (events.ndjson out)

Append-only NDJSON. Every record has `ts` (ISO-8601) and `event`; other fields are
event-specific. This file is also the **durable recovery / pending-publish
artifact** (SPEC §6): it outlives the session and is drained on the next run.

| event | key fields | when |
| --- | --- | --- |
| `session_started` | `id`, `dir`, `video` (`{index,name,kind}` — screen or camera), `audio`, `config` | after device resolution |
| `ffmpeg_command` | `argv`, `cwd` | before take 0 spawn (debug aid) |
| `recording_started` | `pid` | take 0 spawned |
| `take_started` | `take`, `startNumber`, `pid` | a take's ffmpeg spawned |
| `segment_ready` | `index`, `file` | a segment is closed (next began, or sealed at finalize) |
| `paused` | `take`, `nextSegment` | current take ended on `pause` |
| `resumed` | `take`, `startSegment` | a new take began on `resume` |
| `take_ended` | `take`, `exitCode`, `nextSegment` | a take's ffmpeg exited (pause or stop) |
| `command_ignored` | `command` | an unrecognized control command |
| `stop_requested` | `reason` | `stop` received, or a signal |
| `recording_stopped` | `takeCount` | all takes finished |
| `finalized` | `id`, `preview`, `playlist`, `initSegment`, `segments`, `segmentCount`, `durationSec`, `takeCount`, `endlist`, `ok` | session assembled |
| `error` | `phase`, `message` | device resolution / spawn / control failure |

`ok` (on `finalized`) is the success signal: `preview` + `initSegment` present and
≥1 segment in the assembled playlist. The recorder process exits `0` iff `ok`.

## Layout

```
record.mjs            CLI entry + take controller (spawn, pause/resume, stop, watch)
lib/config.mjs        validated recipe constants + segment/take name helpers
lib/devices.mjs       resolve avfoundation devices by name
lib/ffmpeg.mjs        build one take's tee argv (per-take start_number/playlist/preview)
lib/control.mjs       fifo reader (stays open across writer disconnects)
lib/events.mjs        events.ndjson writer (+ stdout echo)
lib/finalize.mjs      assemble master playlist + concat preview; summarize
```

Portable Node core (model B: `shim → node → ffmpeg`). The per-OS native control
shim that owns Screen Recording / tray / hotkey layers in later, unchanged.
