// Device-catalogue tests: the pure parse/classify/select logic behind the
// pre-record picker (no ffmpeg spawn). Covers the real-world bug that the
// auto-selected mic was the wireless iPhone Continuity mic (audio dropouts +
// video hitches) — `pickDefaultAudio` must prefer the built-in mic instead.
// Run: node scripts/recorder/test/devices.test.mjs

import assert from 'node:assert/strict';
import {
  parseDeviceList,
  classifyVideoKind,
  isContinuityMic,
  pickDefaultAudio,
} from '../lib/devices.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// A realistic `ffmpeg -list_devices` stderr dump (the shape we parse).
const LISTING = `[AVFoundation indev @ 0x14] AVFoundation video devices:
[AVFoundation indev @ 0x14] [0] iPhone Camera
[AVFoundation indev @ 0x14] [1] FaceTime HD Camera
[AVFoundation indev @ 0x14] [2] iPhone Desk View Camera
[AVFoundation indev @ 0x14] [3] Capture screen 0
[AVFoundation indev @ 0x14] AVFoundation audio devices:
[AVFoundation indev @ 0x14] [0] iPhone Microphone
[AVFoundation indev @ 0x14] [1] BlackHole 2ch
[AVFoundation indev @ 0x14] [2] MacBook Pro Microphone
[AVFoundation indev @ 0x14] [3] LoomAudioDevice`;

test('parses video + audio devices and tags screen vs camera', () => {
  const { video, audio } = parseDeviceList(LISTING);
  assert.deepEqual(video, [
    { index: 0, name: 'iPhone Camera', kind: 'camera' },
    { index: 1, name: 'FaceTime HD Camera', kind: 'camera' },
    { index: 2, name: 'iPhone Desk View Camera', kind: 'camera' },
    { index: 3, name: 'Capture screen 0', kind: 'screen' },
  ]);
  assert.deepEqual(audio.map((d) => d.index), [0, 1, 2, 3]);
  assert.equal(audio[2].name, 'MacBook Pro Microphone');
});

test('classifyVideoKind: only "Capture screen N" is a screen', () => {
  assert.equal(classifyVideoKind('Capture screen 0'), 'screen');
  assert.equal(classifyVideoKind('Capture screen 1'), 'screen');
  assert.equal(classifyVideoKind('FaceTime HD Camera'), 'camera');
  assert.equal(classifyVideoKind('iPhone Camera'), 'camera');
});

test('isContinuityMic flags iPhone/iPad, not the built-in', () => {
  assert.ok(isContinuityMic('iPhone Microphone'));
  assert.ok(isContinuityMic('iPad Microphone'));
  assert.ok(!isContinuityMic('MacBook Pro Microphone'));
  assert.ok(!isContinuityMic('BlackHole 2ch'));
});

test('pickDefaultAudio prefers the built-in mic over the iPhone (the bug)', () => {
  const { audio } = parseDeviceList(LISTING);
  // Before: audio[0] (iPhone Microphone) was auto-picked → dropouts. Now: built-in.
  assert.equal(pickDefaultAudio(audio).name, 'MacBook Pro Microphone');
});

test('pickDefaultAudio: no built-in → first non-Continuity → else first', () => {
  assert.equal(
    pickDefaultAudio([{ index: 0, name: 'iPhone Microphone' }, { index: 1, name: 'BlackHole 2ch' }]).name,
    'BlackHole 2ch',
  );
  assert.equal(
    pickDefaultAudio([{ index: 0, name: 'iPhone Microphone' }]).name,
    'iPhone Microphone', // only option — better than nothing
  );
  assert.equal(pickDefaultAudio([]), null);
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
