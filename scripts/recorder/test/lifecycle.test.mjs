// Lifecycle tests for the arm/start decouple (S1): the recorder must LAUNCH
// without capturing and only spin up ffmpeg on an explicit `start` (the consent
// boundary — see record.mjs header). Driven end-to-end through the real fifo +
// events.ndjson, with a FAKE ffmpeg on PATH so the test is hermetic (no real
// screen capture, runs anywhere). Run:
//   node scripts/recorder/test/lifecycle.test.mjs

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const RECORD = path.resolve(HERE, '../record.mjs');

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// A fake `ffmpeg`: answers -list_devices with a tiny catalogue, and in capture
// mode writes just enough artifacts for finalize (init + one segment + per-take
// playlist + preview) then waits for `q` on stdin (clean stop) like real ffmpeg.
// CommonJS (run directly as a bare \`ffmpeg\` file with no .mjs extension, so node
// parses it as CJS — \`import\` would throw and crash the fake).
const FAKE_FFMPEG = `#!/usr/bin/env node
const fs = require('fs');
const argv = process.argv.slice(2);
const joined = argv.join(' ');
if (joined.includes('-list_devices')) {
  process.stderr.write([
    '[AVFoundation indev @ 0x0] AVFoundation video devices:',
    '[AVFoundation indev @ 0x0] [0] FaceTime HD Camera',
    '[AVFoundation indev @ 0x0] [1] Capture screen 0',
    '[AVFoundation indev @ 0x0] AVFoundation audio devices:',
    '[AVFoundation indev @ 0x0] [0] MacBook Pro Microphone',
    '',
  ].join('\\n'));
  process.exit(1);
}
if (joined.includes('concat')) { // finalize multi-take path (unused here)
  fs.writeFileSync(argv[argv.length - 1], 'PREVIEW'); process.exit(0);
}
// Two-input capture: open + drain the audio fifo like real ffmpeg would, so node's
// fifo write end unblocks (else the mic buffer just fills — no deadlock, but the fifo
// never connects). No audio arg → no-op (the video-only tests).
const af = argv.find((a) => /audio_\\d+\\.pcm$/.test(a));
if (af) { try { fs.createReadStream(af).on('data', () => {}).on('error', () => {}); } catch (e) {} }
const k = (joined.match(/stream_(\\d+)\\.m3u8/) || [, '0'])[1];
const start = Number((joined.match(/start_number=(\\d+)/) || [, '0'])[1]);
const seg = 'seg_' + String(start).padStart(5, '0') + '.m4s';
fs.writeFileSync('init.mp4', 'INIT');
fs.writeFileSync(seg, 'SEG');
fs.writeFileSync('stream_' + k + '.m3u8',
  '#EXTM3U\\n#EXT-X-MAP:URI="init.mp4"\\n#EXTINF:6.000000,\\n' + seg + '\\n#EXT-X-ENDLIST\\n');
fs.writeFileSync('preview_' + k + '.mp4', 'PREVIEW');
let buf = '';
process.stdin.on('data', (d) => { buf += d.toString(); if (buf.includes('q')) process.exit(0); });
process.on('SIGTERM', () => process.exit(0));
process.on('SIGINT', () => process.exit(0));
setInterval(() => {}, 1000);
`;

// A fake mic tap (the shim's `--mic-tap` mode): `--probe` prints the rate; otherwise
// it streams f32le bytes on stdout until SIGTERM — enough to exercise record.mjs's
// tap supervision hermetically (no real mic, runs anywhere).
const FAKE_MIC = `#!/usr/bin/env node
const argv = process.argv.slice(2);
process.stdout.on('error', () => {}); // ignore EPIPE when the reader goes away
if (argv.includes('--probe')) { process.stdout.write('rate=48000\\n'); process.exit(0); }
const buf = Buffer.alloc(4096);
const iv = setInterval(() => { try { process.stdout.write(buf); } catch (e) {} }, 20);
const stop = () => { clearInterval(iv); process.exit(0); };
process.on('SIGTERM', stop);
process.on('SIGINT', stop);
setInterval(() => {}, 1000);
`;

// One temp session: a fake-ffmpeg bin on PATH, a session dir, the launched
// recorder. Returns helpers to read events, send fifo commands, and await exit.
// audio: the --audio value; withMic: install a fake mic tap and point --mic-cmd at it.
function launch(extraArgs = [], { audio = 'none', withMic = false } = {}) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-life-'));
  const bin = path.join(tmp, 'bin');
  fs.mkdirSync(bin);
  const fake = path.join(bin, 'ffmpeg');
  fs.writeFileSync(fake, FAKE_FFMPEG, { mode: 0o755 });
  let micArgs = [];
  if (withMic) {
    const fakeMic = path.join(bin, 'mic');
    fs.writeFileSync(fakeMic, FAKE_MIC, { mode: 0o755 });
    micArgs = ['--mic-cmd', fakeMic];
  }
  const dir = path.join(tmp, 'session');

  const child = spawn('node', [
    RECORD, '--out', dir, '--device', 'Capture screen 0', '--audio', audio,
    '--no-upload', ...micArgs, ...extraArgs,
  ], { env: { ...process.env, PATH: bin + ':' + process.env.PATH } });

  // Capture the recorder's stdout too — on discard the session dir (and its
  // events.ndjson) is deleted, so the streamed events are the only record left.
  let out = '';
  child.stdout.on('data', (d) => { out += d.toString(); });
  const parseLines = (text) => text.split('\n').filter(Boolean)
    .map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);

  let exitCode = 'running';
  const exited = new Promise((res) => child.on('exit', (c) => { exitCode = c; res(c); }));
  const events = () => {
    const p = path.join(dir, 'events.ndjson');
    if (!fs.existsSync(p)) return [];
    return parseLines(fs.readFileSync(p, 'utf8'));
  };
  const has = (name) => events().some((e) => e.event === name);
  const hasOut = (name) => parseLines(out).some((e) => e.event === name); // from stdout
  const send = (cmd) => fs.writeFileSync(path.join(dir, 'control.fifo'), cmd + '\n');
  const waitFor = async (name, ms = 5000) => {
    const t0 = Date.now();
    while (Date.now() - t0 < ms) {
      if (has(name)) return true;
      await new Promise((r) => setTimeout(r, 50));
    }
    throw new Error(`timed out waiting for "${name}" event; saw: ${events().map((e) => e.event).join(',')}`);
  };
  return { tmp, dir, child, exited, events, has, hasOut, send, waitFor, getExit: () => exitCode };
}

test('launch ARMS but does not capture — no ffmpeg until `start`', async () => {
  const s = launch();
  await s.waitFor('armed');
  // The whole point: armed, but nothing recording yet.
  assert.equal(s.has('recording_started'), false, 'must NOT have started capture at launch');
  assert.equal(s.has('take_started'), false, 'no ffmpeg take should be spawned while armed');

  s.send('start');
  await s.waitFor('recording_started');
  assert.equal(s.has('take_started'), true, 'start spawns take 0');

  s.send('stop');
  await s.exited;
  assert.equal(s.getExit(), 0, 'clean exit after a real recording');
  const fin = s.events().find((e) => e.event === 'finalized');
  assert.ok(fin && fin.ok === true, 'finalized ok after start→stop');
  assert.equal(s.has('aborted'), false, 'a real recording is finalized, not aborted');
});

test('stop while ARMED aborts cleanly — no finalize, no half-built page', async () => {
  const s = launch();
  await s.waitFor('armed');
  s.send('stop');
  await s.exited;
  assert.equal(s.getExit(), 0, 'abort exits 0');
  assert.equal(s.has('aborted'), true, 'emits aborted');
  assert.equal(s.has('recording_started'), false, 'never started');
  assert.equal(s.has('finalized'), false, 'no finalize for a session that never captured');
});

test('pause/resume before start are ignored (no take leaks out of armed)', async () => {
  const s = launch();
  await s.waitFor('armed');
  s.send('pause');
  s.send('resume');
  // Give the command chain a moment, then confirm still armed, nothing recorded.
  await new Promise((r) => setTimeout(r, 300));
  assert.equal(s.has('recording_started'), false, 'pause/resume must not start capture');
  assert.equal(s.has('take_started'), false);
  s.send('stop');
  await s.exited;
  assert.equal(s.has('aborted'), true);
});

test('cancel discards — stops, no finalize/publish, deletes the session dir', async () => {
  const s = launch();
  await s.waitFor('armed');
  s.send('start');
  await s.waitFor('recording_started');
  s.send('cancel');
  await s.exited;
  assert.equal(s.getExit(), 0, 'discard exits 0');
  assert.equal(s.hasOut('discarded'), true, 'emits discarded (read from stdout)');
  assert.equal(s.hasOut('finalized'), false, 'discard never finalizes/publishes');
  assert.equal(fs.existsSync(s.dir), false, 'the session scratch dir is deleted');
});

test('cancel while still armed discards cleanly (nothing captured)', async () => {
  const s = launch();
  await s.waitFor('armed');
  s.send('cancel');
  await s.exited;
  assert.equal(s.getExit(), 0);
  assert.equal(s.hasOut('discarded'), true);
  assert.equal(fs.existsSync(s.dir), false, 'armed-then-cancel also cleans up');
});

test('--autostart begins capture at launch (test/headless escape hatch)', async () => {
  const s = launch(['--autostart']);
  await s.waitFor('recording_started');
  assert.equal(s.has('take_started'), true, 'autostart spawns take 0 with no fifo `start`');
  s.send('stop');
  await s.exited;
  assert.equal(s.getExit(), 0);
});

test('audio path: native mic tap is probed + wired as a 2nd input, clean start→stop', async () => {
  const s = launch(['--autostart'], { audio: 'default', withMic: true });
  await s.waitFor('recording_started');
  const enabled = s.events().find((e) => e.event === 'audio_enabled');
  assert.ok(enabled && enabled.rate === 48000, 'mic rate probed + audio enabled');
  const ts = s.events().find((e) => e.event === 'take_started');
  assert.equal(ts.audio, true, 'take_started marks audio on');
  // The recipe is the two-input one: video-only input 0 + f32le fifo input 1, -map 1:a.
  const cmd = s.events().find((e) => e.event === 'ffmpeg_command');
  const argv = cmd.argv.join(' ');
  assert.match(argv, /-f f32le -ar 48000 -ac 1 -i \S*audio_0\.pcm/, 'f32le fifo is the 2nd input');
  assert.match(argv, /-map 1:a/, 'audio mapped from input 1');
  assert.match(argv, /:none/, 'video input carries no avfoundation audio');
  assert.ok(!argv.includes('aresample'), 'no aresample band-aid');

  s.send('stop');
  await s.exited;
  assert.equal(s.getExit(), 0, 'clean exit with the audio path');
  const fin = s.events().find((e) => e.event === 'finalized');
  assert.ok(fin && fin.ok === true, 'finalized ok with audio');
  assert.equal(fs.existsSync(path.join(s.dir, 'audio_0.pcm')), false, 'the take fifo is cleaned up');
});

test('audio requested but no mic tap given → records silently, never crashes', async () => {
  // --audio default but NO --mic-cmd: the recording must still render (video-only),
  // with a logged skip. No avfoundation-audio fallback — that path is the bug.
  const s = launch(['--autostart'], { audio: 'default', withMic: false });
  await s.waitFor('recording_started');
  const skipped = s.events().find((e) => e.event === 'audio_skipped');
  assert.ok(skipped && skipped.reason === 'no_mic_tap', 'logs the skip reason');
  const ts = s.events().find((e) => e.event === 'take_started');
  assert.equal(ts.audio, false, 'take is video-only when no tap is available');
  const cmd = s.events().find((e) => e.event === 'ffmpeg_command');
  assert.ok(!cmd.argv.join(' ').includes('f32le'), 'no phantom audio input');
  s.send('stop');
  await s.exited;
  assert.equal(s.getExit(), 0, 'renders and exits cleanly regardless');
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.stack || e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
