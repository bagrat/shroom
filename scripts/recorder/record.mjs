#!/usr/bin/env node
// shroom recorder core (M1) — deterministic hands for the whole ffmpeg lifecycle.
//
// Contract (the determinism boundary, SPEC §4):
//   IN  : a control fifo  (newline commands: `stop`; `pause`/`resume` land in M2)
//   OUT : events.ndjson   (session_started, recording_started, segment_ready,
//                          stop_requested, recording_stopped, finalized, error)
//
// There is nothing for an LLM to decide in real time here — recording is pure
// mechanism. The agent orchestrates *around* this (title/chapters/publish).
//
// Usage:
//   node record.mjs [--id <id>] [--out <dir>] [--device "Capture screen 0"]
//                   [--audio none|default|<name>] [--fifo <path>]
//
// Stop it with:  echo stop > <dir>/control.fifo   (or SIGINT/SIGTERM)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';

import { CONFIG, segName } from './lib/config.mjs';
import { resolveDevices } from './lib/devices.mjs';
import { buildFfmpegArgs } from './lib/ffmpeg.mjs';
import { watchControl } from './lib/control.mjs';
import { createEventLog } from './lib/events.mjs';
import { summarize } from './lib/finalize.mjs';

function parseArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const val = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
      opts[key] = val;
    }
  }
  return opts;
}

// Unguessable-ish id for now; the bucket key/prefix scheme is deferred (SPEC §11).
function genId() {
  return crypto.randomBytes(12).toString('base64url');
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  const id = opts.id ?? genId();
  const dir = path.resolve(
    opts.out ?? path.join(os.homedir(), '.shroom', 'recordings', id),
  );
  fs.mkdirSync(dir, { recursive: true });

  const log = createEventLog(path.join(dir, CONFIG.files.events));

  // --- control fifo ---
  const fifoPath = opts.fifo ?? path.join(dir, CONFIG.files.control);
  if (!fs.existsSync(fifoPath)) execFileSync('mkfifo', [fifoPath]);

  // --- resolve capture devices by name ---
  let devices;
  try {
    devices = await resolveDevices({
      screenName: opts.device ?? 'Capture screen 0',
      audio: opts.audio ?? 'none',
    });
  } catch (e) {
    log.emit('error', { phase: 'device_resolution', message: e.message });
    log.close();
    try { fs.unlinkSync(fifoPath); } catch {}
    process.exit(1);
  }

  log.emit('session_started', {
    id,
    dir,
    screen: devices.screen,
    audio: devices.audioName ? { index: devices.audioIndex, name: devices.audioName } : null,
    config: {
      framerate: CONFIG.framerate,
      segmentSeconds: CONFIG.segmentSeconds,
      videoBitrate: CONFIG.videoBitrate,
    },
  });

  // --- spawn ffmpeg (cwd = session dir, so output filenames stay relative) ---
  const args = buildFfmpegArgs({
    screenIndex: devices.screen.index,
    audioIndex: devices.audioIndex,
  });
  log.emit('ffmpeg_command', { argv: ['ffmpeg', ...args], cwd: dir });

  const ff = spawn('ffmpeg', args, { cwd: dir, stdio: ['pipe', 'ignore', 'pipe'] });
  const ffLog = fs.createWriteStream(path.join(dir, CONFIG.files.ffmpegLog));
  ff.stderr.pipe(ffLog);
  log.emit('recording_started', { pid: ff.pid });

  // --- segment watcher ---
  // A segment N is COMPLETE once segment N+1 begins (HLS opens the next file). We
  // announce completed segments as `segment_ready`; the M3 uploader consumes these.
  const emitted = new Set();
  let highest = -1;
  const emitSegment = (i) => {
    if (emitted.has(i)) return;
    emitted.add(i);
    log.emit('segment_ready', { index: i, file: segName(i) });
  };
  const watcher = fs.watch(dir, (_type, filename) => {
    if (!filename) return;
    const m = filename.match(CONFIG.files.segmentGlob);
    if (!m) return;
    const n = Number(m[1]);
    if (n > highest) {
      for (let i = 0; i < n; i++) emitSegment(i); // everything before n is now closed
      highest = n;
    }
  });

  // --- stop handling (single-shot, with escalation) ---
  let stopping = false;
  const requestStop = (reason) => {
    if (stopping) return;
    stopping = true;
    log.emit('stop_requested', { reason });
    // Primary: clean shutdown via `q` on ffmpeg stdin (valid moov + ENDLIST, exit 0).
    try { ff.stdin.write('q\n'); } catch {}
    // Fallbacks if ffmpeg doesn't exit promptly.
    setTimeout(() => { if (!ff.killed) try { ff.kill('SIGTERM'); } catch {} }, 5000).unref();
    setTimeout(() => { if (!ff.killed) try { ff.kill('SIGKILL'); } catch {} }, 10000).unref();
  };

  const control = watchControl(fifoPath);
  control.on('command', (cmd) => {
    if (cmd === 'stop') requestStop('control:stop');
    else log.emit('command_ignored', { command: cmd }); // pause/resume = M2
  });
  control.on('error', (e) => log.emit('error', { phase: 'control', message: e.message }));

  process.on('SIGINT', () => requestStop('signal:SIGINT'));
  process.on('SIGTERM', () => requestStop('signal:SIGTERM'));

  // --- finalize on ffmpeg exit ---
  ff.on('close', (code) => {
    try { watcher.close(); } catch {}
    control.close?.();
    const summary = summarize(dir);
    // Announce any segments not seen by the watcher (notably the last one, sealed at stop).
    for (const i of summary.segmentIndices) emitSegment(i);

    log.emit('recording_stopped', { exitCode: code });
    log.emit('finalized', { id, ...summary, ffmpegExit: code });
    log.close();
    try { fs.unlinkSync(fifoPath); } catch {}
    process.exit(summary.ok ? 0 : 1);
  });

  ff.on('error', (e) => {
    log.emit('error', { phase: 'ffmpeg_spawn', message: e.message });
    log.close();
    try { fs.unlinkSync(fifoPath); } catch {}
    process.exit(1);
  });
}

main().catch((e) => {
  process.stderr.write(`recorder fatal: ${e?.stack || e}\n`);
  process.exit(1);
});
