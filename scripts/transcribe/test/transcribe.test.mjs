// Offline tests for the transcription core: timestamp formatting, whisper-JSON
// normalization, and the runTranscribe orchestration over a FAKE runWhisper seam
// (tests never spawn whisper). Audio/output live in a temp dir. Run:
//   node scripts/transcribe/test/transcribe.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  formatTimestamp,
  parseWhisperJson,
  runTranscribe,
  TRANSCRIPT_FILE,
} from '../lib/transcribe.mjs';

let passed = 0;
function test(name, fn) {
  return Promise.resolve()
    .then(fn)
    .then(() => { passed++; })
    .catch((e) => { console.error(`✗ ${name}\n  ${e.stack || e}`); process.exitCode = 1; });
}

// A fresh temp dir with a stub preview.mp4 (whisper is faked, so contents don't matter).
function tmpSession() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-transcribe-'));
  fs.writeFileSync(path.join(dir, 'preview.mp4'), 'fake-bytes');
  return dir;
}

// A fake whisper that returns a fixed JSON object (or a failure), and records calls.
function fakeWhisper(result, calls = []) {
  return async (args) => { calls.push(args); return result; };
}

const SAMPLE = {
  language: 'en',
  text: 'full text ignored when segments present',
  segments: [
    { start: 0, end: 2.5, text: ' Hello there. ' },
    { start: 2.5, end: 5.004, text: 'Second line.' },
    { start: 5, end: 6, text: '   ' }, // whitespace-only → dropped
  ],
};

// --- formatTimestamp ---
await test('formatTimestamp: under an hour is M:SS', () => {
  assert.equal(formatTimestamp(0), '0:00');
  assert.equal(formatTimestamp(5), '0:05');
  assert.equal(formatTimestamp(65), '1:05');
  assert.equal(formatTimestamp(599), '9:59');
});
await test('formatTimestamp: an hour or more is H:MM:SS', () => {
  assert.equal(formatTimestamp(3600), '1:00:00');
  assert.equal(formatTimestamp(3661), '1:01:01');
});
await test('formatTimestamp: floors fractions and clamps negatives', () => {
  assert.equal(formatTimestamp(5.9), '0:05');
  assert.equal(formatTimestamp(-10), '0:00');
});

// --- parseWhisperJson ---
await test('parseWhisperJson: trims, drops empty, rounds, rebuilds transcript', () => {
  const r = parseWhisperJson(SAMPLE);
  assert.equal(r.language, 'en');
  assert.deepEqual(r.segments, [
    { start: 0, end: 2.5, text: 'Hello there.' },
    { start: 2.5, end: 5, text: 'Second line.' }, // 5.004 → 5
  ]);
  assert.equal(r.transcript, 'Hello there.\nSecond line.');
  assert.equal(r.durationSec, 5); // last kept segment's end
});
await test('parseWhisperJson: accepts a raw JSON string', () => {
  const r = parseWhisperJson(JSON.stringify(SAMPLE));
  assert.equal(r.segments.length, 2);
});
await test('parseWhisperJson: falls back to top-level text when no segments', () => {
  const r = parseWhisperJson({ language: 'en', text: '  only text  ', segments: [] });
  assert.equal(r.transcript, 'only text');
  assert.equal(r.durationSec, 0);
  assert.deepEqual(r.segments, []);
});
await test('parseWhisperJson: tolerates a missing/garbage shape', () => {
  assert.deepEqual(parseWhisperJson(null), { language: null, transcript: '', durationSec: 0, segments: [] });
  assert.deepEqual(parseWhisperJson({}), { language: null, transcript: '', durationSec: 0, segments: [] });
});

// --- runTranscribe ---
await test('runTranscribe: success writes transcript.json + emits transcribed', async () => {
  const dir = tmpSession();
  const events = [];
  const r = await runTranscribe({
    sessionDir: dir,
    runWhisper: fakeWhisper({ ok: true, raw: SAMPLE }),
    log: (e, f) => events.push([e, f]),
  });
  assert.equal(r.ok, true);
  assert.equal(r.segments.length, 2);
  const written = JSON.parse(fs.readFileSync(path.join(dir, TRANSCRIPT_FILE), 'utf8'));
  assert.equal(written.transcript, 'Hello there.\nSecond line.');
  assert.equal(r.transcriptPath, path.join(dir, TRANSCRIPT_FILE));
  assert.deepEqual(events.map((e) => e[0]), ['transcribe_started', 'transcribed']);
});

await test('runTranscribe: passes resolved audio + model to the seam', async () => {
  const dir = tmpSession();
  const calls = [];
  await runTranscribe({ sessionDir: dir, model: 'small', runWhisper: fakeWhisper({ ok: true, raw: SAMPLE }, calls) });
  assert.equal(calls[0].audioPath, path.join(dir, 'preview.mp4'));
  assert.equal(calls[0].outDir, dir);
  assert.equal(calls[0].model, 'small');
});

await test('runTranscribe: no audio is a soft skip', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-noaudio-'));
  const events = [];
  const r = await runTranscribe({ sessionDir: dir, runWhisper: fakeWhisper({ ok: true, raw: SAMPLE }), log: (e, f) => events.push([e, f]) });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'no_audio');
  assert.equal(events[0][0], 'transcribe_skipped');
  assert.ok(!fs.existsSync(path.join(dir, TRANSCRIPT_FILE)));
});

await test('runTranscribe: whisper failure is a soft skip', async () => {
  const dir = tmpSession();
  const r = await runTranscribe({ sessionDir: dir, runWhisper: fakeWhisper({ ok: false, code: 1, stderr: 'boom' }) });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'whisper_failed');
  assert.ok(!fs.existsSync(path.join(dir, TRANSCRIPT_FILE)));
});

await test('runTranscribe: a thrown seam (missing binary) is a soft skip', async () => {
  const dir = tmpSession();
  const r = await runTranscribe({ sessionDir: dir, runWhisper: async () => { throw new Error('ENOENT'); } });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'whisper_error');
});

await test('runTranscribe: empty transcript is a soft skip (no file written)', async () => {
  const dir = tmpSession();
  const r = await runTranscribe({ sessionDir: dir, runWhisper: fakeWhisper({ ok: true, raw: { segments: [], text: '' } }) });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'empty_transcript');
  assert.ok(!fs.existsSync(path.join(dir, TRANSCRIPT_FILE)));
});

await test('runTranscribe: explicit --audio outside a session writes alongside it', async () => {
  const dir = tmpSession();
  const audio = path.join(dir, 'preview.mp4');
  const r = await runTranscribe({ audioPath: audio, runWhisper: fakeWhisper({ ok: true, raw: SAMPLE }) });
  assert.equal(r.ok, true);
  assert.equal(r.transcriptPath, path.join(dir, TRANSCRIPT_FILE));
});

console.log(`\n${passed} passed`);
