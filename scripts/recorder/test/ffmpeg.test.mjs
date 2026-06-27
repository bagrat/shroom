// ffmpeg-args tests: the validated tee recipe is assembled correctly, and the
// audio path carries the aresample sync filter (the fix for avfoundation's mic
// clock dropping ~6% of samples). Run: node scripts/recorder/test/ffmpeg.test.mjs

import assert from 'node:assert/strict';
import { buildFfmpegArgs } from '../lib/ffmpeg.mjs';
import { CONFIG } from '../lib/config.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// Find the value following a flag in the argv.
const after = (args, flag) => args[args.indexOf(flag) + 1];

test('with audio: input spec is video:audio and the sync filter is present', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audioIndex: 2 });
  assert.equal(after(args, '-i'), '3:2');
  // The fix: -af aresample=async=1 must be in the audio path, before the tee.
  assert.equal(after(args, '-af'), CONFIG.audioFilter);
  assert.match(CONFIG.audioFilter, /aresample=async=/);
  assert.ok(args.indexOf('-af') < args.lastIndexOf('-f'), '-af must come before the tee output');
  assert.ok(args.includes('-c:a') && args.includes('0:a'));
});

test('no audio: no -af, no -c:a, no audio map', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audioIndex: 'none' });
  assert.equal(after(args, '-i'), '3:none');
  assert.ok(!args.includes('-af'));
  assert.ok(!args.includes('-c:a'));
  assert.ok(!args.includes('0:a'));
});

test('a camera index works the same as a screen index', () => {
  const args = buildFfmpegArgs({ videoIndex: 1, audioIndex: 2 }); // camera as source
  assert.equal(after(args, '-i'), '1:2');
});

test('forced output rate + keyframes at each segment boundary (the HLS fix)', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audioIndex: 'none' });
  assert.equal(after(args, '-r'), String(CONFIG.framerate));
  assert.equal(after(args, '-force_key_frames'), `expr:gte(t,n_forced*${CONFIG.segmentSeconds})`);
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
