// Deterministic transcription core (SPEC §7, milestone M5c) — the bridge between a
// finished recording and the agent's judgment layer. whisper does the speech→text;
// THIS module is the deterministic half: pick the audio, drive whisper over an
// injected seam, then normalize its JSON into a small, stable shape the
// title/TL;DR/chapters skill consumes:
//
//   { language, transcript, durationSec, segments: [{ start, end, text }] }
//
// Nothing here decides anything an LLM should (titles, chapter boundaries) — it
// only produces the timestamped substrate. whisper is OPTIONAL (SPEC §8); when it's
// missing the recording still rendered locally, so callers treat a failed/absent
// transcript as a soft skip, not a hard error.

import fs from 'node:fs';
import path from 'node:path';

// whisper's default-ish model. Fast, English-leaning, good enough for titles and
// chapters; overridable per call / via the CLI.
export const DEFAULT_MODEL = 'base';

// Where the normalized transcript lands inside a session dir. The agent reads this
// to author <id>.md (metadata + transcript body); the plain text becomes the body.
export const TRANSCRIPT_FILE = 'transcript.json';

// Whole seconds → a compact clock string: "M:SS", or "H:MM:SS" past an hour. Used
// for chapter / transcript display downstream (kept here so one definition serves
// every consumer).
export function formatTimestamp(totalSeconds) {
  const s = Math.max(0, Math.floor(Number(totalSeconds) || 0));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

// Normalize one whisper JSON object into our stable shape. whisper emits
// { text, language, segments: [{ start, end, text, ... }] }; we trim each segment,
// drop empties, round times, and rebuild the transcript from the kept segments so
// the text and the timeline can never disagree (falling back to top-level `text`).
export function parseWhisperJson(raw) {
  const obj = typeof raw === 'string' ? JSON.parse(raw) : (raw ?? {});
  const segments = [];
  for (const s of Array.isArray(obj.segments) ? obj.segments : []) {
    const text = typeof s?.text === 'string' ? s.text.trim() : '';
    if (!text) continue;
    const start = round2(s.start);
    const end = round2(s.end);
    segments.push({ start, end, text });
  }
  const transcript = segments.length
    ? segments.map((s) => s.text).join('\n')
    : (typeof obj.text === 'string' ? obj.text.trim() : '');
  const durationSec = segments.length ? segments[segments.length - 1].end : 0;
  return {
    language: typeof obj.language === 'string' ? obj.language : null,
    transcript,
    durationSec,
    segments,
  };
}

// Drive transcription end-to-end over an injected `runWhisper` seam (so the core is
// pure and offline-testable; the real seam shells out to whisper). Resolves the
// audio (preview.mp4 by default — one progressive file, simplest whisper input),
// runs whisper, normalizes the result, and writes TRANSCRIPT_FILE next to it.
//
// Returns { ok, ... } — `ok:false` with a `reason` is a SOFT skip (no audio,
// whisper missing/failed, empty transcript); callers continue without a transcript.
export async function runTranscribe({
  sessionDir,
  audioPath,
  outDir,
  model = DEFAULT_MODEL,
  runWhisper,
  log = () => {},
}) {
  const audio = audioPath ?? (sessionDir ? path.join(sessionDir, 'preview.mp4') : null);
  if (!audio || !fs.existsSync(audio)) {
    log('transcribe_skipped', { reason: 'no_audio', audio: audio ?? null });
    return { ok: false, reason: 'no_audio' };
  }

  const dest = outDir ?? sessionDir ?? path.dirname(audio);
  log('transcribe_started', { audio, model });

  let res;
  try {
    res = await runWhisper({ audioPath: audio, outDir: dest, model });
  } catch (e) {
    log('transcribe_skipped', { reason: 'whisper_error', message: e.message });
    return { ok: false, reason: 'whisper_error', message: e.message };
  }

  if (!res || !res.ok) {
    const reason = res?.reason ?? 'whisper_failed';
    log('transcribe_skipped', { reason, code: res?.code, message: res?.stderr });
    return { ok: false, reason, code: res?.code, message: res?.stderr };
  }

  const parsed = parseWhisperJson(res.raw);
  if (!parsed.transcript) {
    log('transcribe_skipped', { reason: 'empty_transcript' });
    return { ok: false, reason: 'empty_transcript' };
  }

  const transcriptPath = path.join(dest, TRANSCRIPT_FILE);
  fs.writeFileSync(transcriptPath, JSON.stringify(parsed, null, 2) + '\n');
  log('transcribed', {
    language: parsed.language,
    segmentCount: parsed.segments.length,
    durationSec: parsed.durationSec,
    chars: parsed.transcript.length,
    transcriptPath,
  });
  return { ok: true, ...parsed, transcriptPath };
}

function round2(n) {
  const v = Number(n);
  return Number.isFinite(v) ? Math.round(v * 100) / 100 : 0;
}
