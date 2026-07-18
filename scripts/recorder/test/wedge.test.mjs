// Wedge-classification tests: turning ffmpeg's log into the right abort reason. Run:
// node scripts/recorder/test/wedge.test.mjs

import assert from 'node:assert/strict';
import { classifyWedge } from '../lib/wedge.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// The genuine grant-inactive log: the device never opens (no "Input #0, avfoundation"),
// just the configuration failure.
const GRANT_INACTIVE = `
[AVFoundation indev] Configuration of video device failed, falling back to default.
[in#0/avfoundation] Stream #0: not enough frames to estimate rate; consider increasing probesize
`;

// The bug-report log (recording aLsyYJD6Gd2KaORy): the device DID open — config-failed and
// "not supported" print as benign pixel-format fallback, then "Input #0, avfoundation"
// enumerates the stream — but no frames ever flowed.
const NO_FRAMES = `
[AVFoundation indev] Configuration of video device failed, falling back to default.
[in#0] Selected pixel format (yuv420p) is not supported by the input device.
[in#0] Overriding selected pixel format to use uyvy422 instead.
[in#0/avfoundation] Stream #0: not enough frames to estimate rate; consider increasing probesize
Input #0, avfoundation, from '3:none':
  Stream #0:0: Video: rawvideo (UYVY / 0x59565955), uyvy422, 3840x2160, 1000k tbr, 1000k tbn
Input #1, f32le: Stream #1:0: Audio: pcm_f32le, 48000 Hz, mono
`;

// The audio deadlock: both inputs opened fine, ffmpeg then blocked on the mic.
const AUDIO_DEADLOCK = `
Input #0, avfoundation, from '3:none':
  Stream #0:0: Video: rawvideo, uyvy422, 3840x2160
Input #1, f32le: Stream #1:0: Audio: pcm_f32le, 48000 Hz, mono
Press [q] to stop, [?] for help
`;

test('device never opened + config failure → screen_grant_inactive', () => {
  assert.equal(classifyWedge(GRANT_INACTIVE).reason, 'screen_grant_inactive');
});

test('bug aLsyYJD6Gd2KaORy: device opened, no frames → capture_no_frames, NOT grant_inactive', () => {
  const w = classifyWedge(NO_FRAMES);
  assert.equal(w.reason, 'capture_no_frames');
  assert.notEqual(w.reason, 'screen_grant_inactive'); // the false positive we fixed
});

test('the benign config-failed lines alone do not mean grant-inactive once the device opened', () => {
  // Same fallback lines as the grant-inactive log, but with the device-open marker.
  const opened = GRANT_INACTIVE + "\nInput #0, avfoundation, from '3:none':\n";
  assert.notEqual(classifyWedge(opened).reason, 'screen_grant_inactive');
});

test('both inputs opened, no no-frames marker → capture_wedged (audio deadlock)', () => {
  assert.equal(classifyWedge(AUDIO_DEADLOCK).reason, 'capture_wedged');
});

test('empty / missing log defaults to capture_wedged', () => {
  assert.equal(classifyWedge('').reason, 'capture_wedged');
  assert.equal(classifyWedge(undefined).reason, 'capture_wedged');
});

test('every reason carries a human message', () => {
  for (const log of [GRANT_INACTIVE, NO_FRAMES, AUDIO_DEADLOCK, '']) {
    assert.ok(classifyWedge(log).message.length > 0);
  }
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
