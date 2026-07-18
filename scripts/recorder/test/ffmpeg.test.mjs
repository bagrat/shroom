// ffmpeg-args tests: the validated TWO-INPUT tee recipe is assembled correctly —
// video-only from avfoundation (input 0) + clean native mic PCM on a fifo (input 1),
// mapped `0:v` + `1:a`. The old avfoundation-audio path (and its aresample band-aid)
// is gone. Run: node scripts/recorder/test/ffmpeg.test.mjs

import assert from 'node:assert/strict';
import { buildFfmpegArgs } from '../lib/ffmpeg.mjs';
import { CONFIG } from '../lib/config.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// Find the value following the FIRST occurrence of a flag in the argv.
const after = (args, flag) => args[args.indexOf(flag) + 1];
const AUDIO = { fifo: 'audio_0.pcm', rate: 48000 };

test('video input is always video-only (`<index>:none`) — audio never rides avfoundation', () => {
  const withAudio = buildFfmpegArgs({ videoIndex: 3, audio: AUDIO });
  assert.equal(after(withAudio, '-i'), '3:none');
  const noAudio = buildFfmpegArgs({ videoIndex: 3, audio: null });
  assert.equal(after(noAudio, '-i'), '3:none');
});

test('with audio: a second f32le fifo input at the mic rate, mapped 1:a, no aresample', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audio: AUDIO });
  // The mic PCM input: -f f32le -ar <rate> -ac 1 -i <fifo>.
  assert.ok(args.includes('f32le'), 'raw float PCM input format present');
  assert.equal(after(args, '-ar'), String(AUDIO.rate));
  assert.equal(after(args, '-ac'), '1');
  // The fifo is the SECOND -i.
  const inputs = args.map((a, i) => (a === '-i' ? args[i + 1] : null)).filter(Boolean);
  assert.deepEqual(inputs, ['3:none', AUDIO.fifo]);
  // Audio encoded + mapped from input 1; band-aid filter gone.
  assert.ok(args.includes('-c:a') && args.includes('1:a'));
  assert.ok(!args.includes('-af'), 'no aresample band-aid');
  assert.ok(!args.some((a) => /aresample/.test(a)));
});

test('no audio: no second input, no -c:a, no audio map', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audio: null });
  assert.ok(!args.includes('f32le'));
  assert.ok(!args.includes('-c:a'));
  assert.ok(!args.includes('1:a'));
  const inputs = args.map((a, i) => (a === '-i' ? args[i + 1] : null)).filter(Boolean);
  assert.deepEqual(inputs, ['3:none']); // video only
});

test('optional itsoffset shifts the audio input when set', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audio: { ...AUDIO, itsoffset: -0.05 } });
  assert.equal(after(args, '-itsoffset'), '-0.05');
  // itsoffset must precede the audio input it applies to.
  assert.ok(args.indexOf('-itsoffset') < args.lastIndexOf('-i'));
});

test('a camera index works the same as a screen index', () => {
  const args = buildFfmpegArgs({ videoIndex: 1, audio: AUDIO }); // camera as source
  assert.equal(after(args, '-i'), '1:none');
});

test('forced output rate + keyframes at each segment boundary (the HLS fix)', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audio: null });
  assert.equal(after(args, '-r'), String(CONFIG.framerate));
  assert.equal(after(args, '-force_key_frames'), `expr:gte(t,n_forced*${CONFIG.segmentSeconds})`);
});

test('default quality caps to 1080p (matches the original) — present without audio', () => {
  const args = buildFfmpegArgs({ videoIndex: 3, audio: null });
  const vf = after(args, '-vf');
  assert.match(vf, /min\(1920,iw\).*min\(1080,ih\)/);
  assert.match(vf, /force_divisible_by=2/); // even dims for yuv420p
  assert.equal(after(args, '-b:v'), '3.5M');
});

test('quality preset drives the scale box + bitrate', () => {
  const k2 = buildFfmpegArgs({ videoIndex: 3, audio: null, quality: '2k' });
  assert.match(after(k2, '-vf'), /min\(2560,iw\).*min\(1440,ih\)/);
  assert.equal(after(k2, '-b:v'), '6M');

  const k4 = buildFfmpegArgs({ videoIndex: 3, audio: null, quality: '4k' });
  assert.match(after(k4, '-vf'), /min\(3840,iw\).*min\(2160,ih\)/);
  assert.equal(after(k4, '-b:v'), '12M');

  // An unknown quality falls back to the default (normal), never crashes.
  const bad = buildFfmpegArgs({ videoIndex: 3, audio: null, quality: 'ultra' });
  assert.equal(after(bad, '-b:v'), '3.5M');
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
