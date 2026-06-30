// Tests for mid-record head transcription (Thread 2). The deterministic part —
// building a closed head playlist from the first complete segments — is exercised
// over a synthetic segment tree; the full pipeline runs over FAKE ffmpeg + whisper
// seams (no real decode/transcribe), asserting it writes head-transcript.json and
// cleans up its temp files. Run:
//   node scripts/recorder/test/head-transcribe.test.mjs

import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  segmentsForSeconds,
  buildHeadPlaylist,
  transcribeHead,
  HEAD_TRANSCRIPT_FILE,
} from '../lib/head-transcribe.mjs';
import { segName, CONFIG } from '../lib/config.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// A session dir with `n` non-empty segments (+ init unless withInit=false).
function tmpDirWithSegments(n, { withInit = true } = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-head-'));
  if (withInit) fs.writeFileSync(path.join(dir, CONFIG.files.initSegment), 'init');
  for (let i = 0; i < n; i++) fs.writeFileSync(path.join(dir, segName(i)), `seg${i}`);
  return dir;
}

// A fake ffmpeg that writes the requested wav (last arg) under cwd and exits 0.
function fakeFfmpeg() {
  return (_cmd, args, opts) => {
    const ee = new EventEmitter();
    const outName = args[args.length - 1];
    process.nextTick(() => {
      try { fs.writeFileSync(path.join(opts.cwd, outName), 'fake-wav'); ee.emit('close', 0); }
      catch (e) { ee.emit('error', e); }
    });
    return ee;
  };
}

const SAMPLE = { language: 'en', text: '', segments: [{ start: 0, end: 3, text: ' Intro to the thing. ' }] };

test('segmentsForSeconds rounds up to whole segments', () => {
  assert.equal(segmentsForSeconds(60), 10); // 60 / 6
  assert.equal(segmentsForSeconds(66), 11);
  assert.equal(segmentsForSeconds(6), 1);
  assert.equal(segmentsForSeconds(1), 1);
});

test('buildHeadPlaylist lists exactly the first `need` segments + init + ENDLIST', () => {
  const dir = tmpDirWithSegments(13); // need=10, so 11+ present → uses seg_0..seg_9
  try {
    const pl = buildHeadPlaylist(dir, { maxSeconds: 60 });
    assert.ok(pl, 'expected a playlist path');
    const body = fs.readFileSync(pl, 'utf8');
    assert.match(body, new RegExp(`#EXT-X-MAP:URI="${CONFIG.files.initSegment}"`));
    assert.match(body, /#EXT-X-ENDLIST/);
    // first 10 segments present, the 11th (seg_00010) not
    for (let i = 0; i < 10; i++) assert.ok(body.includes(segName(i)), `missing ${segName(i)}`);
    assert.ok(!body.includes(segName(10)), 'should not include seg_00010');
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

test('buildHeadPlaylist returns null until need+1 segments are closed', () => {
  // need=10 → need 11 present. 10 present is NOT enough (the 10th may still be open).
  const dir = tmpDirWithSegments(10);
  try { assert.equal(buildHeadPlaylist(dir, { maxSeconds: 60 }), null); }
  finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

test('buildHeadPlaylist returns null with no init segment', () => {
  const dir = tmpDirWithSegments(13, { withInit: false });
  try { assert.equal(buildHeadPlaylist(dir, { maxSeconds: 60 }), null); }
  finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

test('transcribeHead writes head-transcript.json and cleans temp files', async () => {
  const dir = tmpDirWithSegments(13);
  try {
    const events = [];
    const r = await transcribeHead({
      dir,
      spawnFn: fakeFfmpeg(),
      runWhisper: async () => ({ ok: true, raw: SAMPLE }),
      log: (e, f) => events.push([e, f]),
    });
    assert.equal(r.ok, true);
    const written = JSON.parse(fs.readFileSync(path.join(dir, HEAD_TRANSCRIPT_FILE), 'utf8'));
    assert.equal(written.transcript, 'Intro to the thing.');
    assert.ok(!fs.existsSync(path.join(dir, 'head.m3u8')), 'temp playlist not cleaned');
    assert.ok(!fs.existsSync(path.join(dir, 'head.wav')), 'temp wav not cleaned');
    assert.ok(events.some((e) => e[0] === 'head_transcribed'));
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

test('transcribeHead soft-skips when there are not enough segments', async () => {
  const dir = tmpDirWithSegments(3);
  try {
    const r = await transcribeHead({ dir, spawnFn: fakeFfmpeg(), runWhisper: async () => ({ ok: true, raw: SAMPLE }) });
    assert.equal(r.ok, false);
    assert.equal(r.reason, 'not_enough_segments');
    assert.ok(!fs.existsSync(path.join(dir, HEAD_TRANSCRIPT_FILE)));
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.stack || e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
