# shroom transcribe

The **transcription step** (SPEC §7) — the deterministic bridge from a finished
recording to the agent's judgment layer. whisper does the speech→text; this turns
its output into a small, stable `transcript.json` that the title / TL;DR / chapters
skill reads to author each video's `<id>.md`.

Deterministic mechanism → a script (the determinism boundary). The *agent* decides
the title, the chapter boundaries, what the TL;DR says; this just produces the
timestamped substrate, repeatably.

## What it does

1. **Pick the audio** — the session's `preview.mp4` by default (one progressive
   file, the simplest whisper input). Override with `--audio`.
2. **Run whisper** over an injected seam (`runWhisper`):
   `whisper <audio> --model <m> --output_format json --output_dir <dir>`. whisper
   is OAuth-free and fully local (SPEC §7).
3. **Normalize** whisper's JSON into a stable shape — trim each segment, drop
   empties, round times, and rebuild the transcript from the kept segments so the
   text and the timeline can't disagree:

   ```json
   { "language": "en", "transcript": "…", "durationSec": 42.5,
     "segments": [ { "start": 0, "end": 2.5, "text": "Hello there." } ] }
   ```
4. **Write** it to `<session>/transcript.json` and emit a `transcribed` event.

**whisper is optional (SPEC §8).** No audio, no whisper binary, a whisper failure,
or an empty transcript are all **soft skips** — a `transcribe_skipped` event and a
`{ ok:false, reason }` result, exit 0. The recording still rendered locally and the
page/deploy steps don't require a transcript, so the record flow continues without
one (the title/chapters skill just falls back to defaults).

## Usage

```bash
node transcribe.mjs --session <dir> [--audio <file>] [--model base] \
                    [--whisper <bin>] [--json]
```

Defaults: `--audio <session>/preview.mp4`, `--model base`. With `--session` the
events are also appended to that session's `events.ndjson` (the durable artifact
the next `/shroom` run drains, SPEC §6). `--json` prints the parsed transcript
object on stdout instead of the event stream.

## Events

- `transcribe_started` — `{ audio, model }`.
- `transcribed` — `{ language, segmentCount, durationSec, chars, transcriptPath }`.
- `transcribe_skipped` — `{ reason }` (`no_audio` / `whisper_error` /
  `whisper_failed` / `empty_transcript`) — a soft skip, not a failure.

## Layout

```
transcribe.mjs            CLI / recovery entry point (mirrors uploader/upload.mjs)
lib/transcribe.mjs        audio pick, normalization, the transcription core
lib/whisper.mjs           the whisper seam (spawn + tee + read JSON) — runWhisper
test/transcribe.test.mjs  behaviour tests against a fake whisper
```

## Tests

```bash
node test/transcribe.test.mjs
```

Runs offline with no whisper binary and no audio decode — the whisper call is an
injected seam fed fixed JSON. A real end-to-end transcribe over a genuine
recording lands when `/shroom:record` is wired (M5c).
