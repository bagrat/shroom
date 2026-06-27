#!/usr/bin/env node
// shroom recorder core (M1 + M2) — deterministic hands for the whole ffmpeg
// lifecycle, including pause/resume as a segment boundary.
//
// Contract (the determinism boundary, SPEC §4):
//   IN  : a control fifo  (newline commands: `pause`, `resume`, `stop`)
//   OUT : events.ndjson   (session_started, take_started, segment_ready,
//                          paused, resumed, take_ended, stop_requested,
//                          recording_stopped, finalized, error)
//
// A "take" is one ffmpeg run between pauses. Pause = clean q-stop at a segment
// boundary; resume = a new take with contiguous segment numbering (SPEC §4/§5).
// Segments stream straight to disk; the master playlist + preview are assembled
// only at finalize. Nothing here is for an LLM to decide in real time — the agent
// orchestrates *around* this (title/chapters/publish).
//
// Usage:
//   node record.mjs [--id <id>] [--out <dir>] [--device "<screen or camera name>"]
//                   [--audio none|default|<name>] [--quality normal|2k|4k] [--fifo <path>]
//   node record.mjs --preflight               # JSON for the picker: devices + quality presets + last profile
//
// --device names ANY avfoundation video source — a screen ("Capture screen 0") or a
// camera ("FaceTime HD Camera"); camera-as-source, not PiP (SPEC §4). --audio
// "default" prefers a built-in mic and never the iPhone/Continuity mic. --quality
// picks a resolution/bitrate preset (lib/quality.mjs; default normal = 1080p).
//
// Control:  echo pause  > <dir>/control.fifo
//           echo resume > <dir>/control.fifo
//           echo stop   > <dir>/control.fifo     (or SIGINT/SIGTERM)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';

import { CONFIG, segName } from './lib/config.mjs';
import { resolveDevices, listDevices, pickDefaultAudio } from './lib/devices.mjs';
import { buildFfmpegArgs } from './lib/ffmpeg.mjs';
import { QUALITY, resolveQuality, ffmpegBitrate, qualityCatalogue, DEFAULT_QUALITY } from './lib/quality.mjs';
import { watchControl } from './lib/control.mjs';
import { createEventLog } from './lib/events.mjs';
import { finalizeSession, maxSegmentIndex } from './lib/finalize.mjs';
import { loadStorageConfig, isConfigured } from '../uploader/lib/storage-config.mjs';
import { Uploader } from '../uploader/lib/uploader.mjs';

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

// The newest prior recording's settings (quality + video + mic), read from its
// session_started event — the "use last settings?" the command offers. No separate
// profile file: events.ndjson already durably records each recording's choices.
function readLastProfile() {
  const base = path.join(os.homedir(), '.shroom', 'recordings');
  let dirs;
  try {
    dirs = fs.readdirSync(base, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => ({ name: d.name, mtime: fs.statSync(path.join(base, d.name)).mtimeMs }))
      .sort((a, b) => b.mtime - a.mtime);
  } catch { return null; }
  for (const d of dirs) {
    const ev = path.join(base, d.name, 'events.ndjson');
    if (!fs.existsSync(ev)) continue;
    for (const line of fs.readFileSync(ev, 'utf8').split('\n')) {
      if (!line.trim()) continue;
      let e; try { e = JSON.parse(line); } catch { continue; }
      if (e.event === 'session_started') {
        return { quality: e.config?.quality ?? null, video: e.video?.name ?? null, audio: e.audio?.name ?? null };
      }
    }
  }
  return null;
}

// Preflight JSON for the picker (the command asks the user; this only reads):
// devices (video tagged screen/camera, mic with a recommended non-Continuity
// default), the quality catalogue with size/cost estimates, and the last profile.
async function preflightJson() {
  const { video, audio } = await listDevices();
  const def = pickDefaultAudio(audio);
  process.stdout.write(JSON.stringify({
    video,
    audio: audio.map((d) => ({ ...d, recommended: def ? d.index === def.index : false })),
    defaultAudioName: def?.name ?? null,
    qualities: qualityCatalogue(),
    lastProfile: readLastProfile(),
  }) + '\n');
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));

  // Accept --preflight (full picker JSON) and --list-devices (back-compat alias).
  if (opts.preflight === 'true' || opts['list-devices'] === 'true') {
    await preflightJson();
    return;
  }

  const id = opts.id ?? genId();
  const quality = resolveQuality(opts.quality ?? DEFAULT_QUALITY);
  const dir = path.resolve(
    opts.out ?? path.join(os.homedir(), '.shroom', 'recordings', id),
  );
  fs.mkdirSync(dir, { recursive: true });

  const log = createEventLog(path.join(dir, CONFIG.files.events));

  const fifoPath = opts.fifo ?? path.join(dir, CONFIG.files.control);
  if (!fs.existsSync(fifoPath)) execFileSync('mkfifo', [fifoPath]);

  let devices;
  try {
    devices = await resolveDevices({
      videoName: opts.device ?? 'Capture screen 0',
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
    video: devices.video, // { index, name, kind } — a screen or a camera
    audio: devices.audioName ? { index: devices.audioIndex, name: devices.audioName } : null,
    config: {
      framerate: CONFIG.framerate,
      segmentSeconds: CONFIG.segmentSeconds,
      quality, // the picker key (normal | 2k | 4k) — also the "use last settings?" record
      resolution: `${QUALITY[quality].maxWidth}x${QUALITY[quality].maxHeight}`,
      videoBitrate: ffmpegBitrate(quality),
    },
  });

  // --- uploader (optional; off when storage isn't configured or --no-upload) ---
  // The recorder owns the upload (SPEC §3), but it's fail-safe: enqueue is
  // non-blocking and the recording never waits on the network (SPEC §5). Until
  // storage is set up, the recording still renders locally (SPEC §8 value-first).
  let uploader = null;
  if (opts['no-upload'] !== 'true') {
    const cfg = loadStorageConfig();
    if (isConfigured(cfg)) {
      uploader = new Uploader(cfg, { id, dir, log: (event, fields) => log.emit(event, fields) });
      log.emit('upload_enabled', { endpoint: cfg.endpoint, bucket: cfg.bucket });
    } else {
      log.emit('upload_skipped', { reason: 'storage_not_configured' });
    }
  }

  // --- segment watcher (global across takes) ---
  // Segment N is COMPLETE once N+1 begins. On resume, the next take's first segment
  // appearing also closes the previous take's last one; finalize sweeps any remainder.
  const emitted = new Set();
  let highest = -1;
  const emitSegment = (i) => {
    if (i < 0 || emitted.has(i)) return;
    emitted.add(i);
    log.emit('segment_ready', { index: i, file: segName(i) });
    // Stream the closed segment up opportunistically (non-blocking, retried).
    if (uploader) uploader.enqueue(segName(i));
  };
  const watcher = fs.watch(dir, (_type, filename) => {
    if (!filename) return;
    const m = filename.match(CONFIG.files.segmentGlob);
    if (!m) return;
    const n = Number(m[1]);
    if (n > highest) {
      for (let i = 0; i < n; i++) emitSegment(i);
      highest = n;
    }
  });

  // --- take management ---
  let state = 'recording'; // 'recording' | 'paused' | 'stopping'
  let nextSegment = 0; // start_number for the next take
  const takes = []; // take indices that have started
  let current = null; // { k, ff, exited }

  function spawnTake(k) {
    const args = buildFfmpegArgs({
      videoIndex: devices.video.index,
      audioIndex: devices.audioIndex,
      startNumber: nextSegment,
      take: k,
      quality,
    });
    if (k === 0) log.emit('ffmpeg_command', { argv: ['ffmpeg', ...args], cwd: dir });
    const ff = spawn('ffmpeg', args, { cwd: dir, stdio: ['pipe', 'ignore', 'pipe'] });
    ff.stderr.pipe(fs.createWriteStream(path.join(dir, `ffmpeg_${k}.log`)));
    const exited = new Promise((res) => ff.on('close', (code) => res(code)));
    ff.on('error', (e) => log.emit('error', { phase: 'ffmpeg_spawn', take: k, message: e.message }));
    takes.push(k);
    log.emit('take_started', { take: k, startNumber: nextSegment, pid: ff.pid });
    return { k, ff, exited };
  }

  // Cleanly end the current take's ffmpeg (q → SIGTERM → SIGKILL), then advance
  // nextSegment past whatever it wrote.
  async function endCurrentTake() {
    if (!current) return;
    const t = current;
    try { t.ff.stdin.write('q\n'); } catch {}
    const term = setTimeout(() => { try { t.ff.kill('SIGTERM'); } catch {} }, 5000);
    const kill = setTimeout(() => { try { t.ff.kill('SIGKILL'); } catch {} }, 10000);
    term.unref(); kill.unref();
    const code = await t.exited;
    clearTimeout(term); clearTimeout(kill);
    nextSegment = maxSegmentIndex(dir) + 1;
    log.emit('take_ended', { take: t.k, exitCode: code, nextSegment });
    current = null;
    return code;
  }

  // Serialize control commands so pause/resume/stop never interleave.
  let chain = Promise.resolve();
  const enqueue = (fn) => {
    chain = chain.then(fn).catch((e) => log.emit('error', { phase: 'command', message: e.message }));
    return chain;
  };

  async function doPause() {
    if (state !== 'recording') return;
    state = 'paused';
    await endCurrentTake();
    log.emit('paused', { take: takes[takes.length - 1], nextSegment });
  }

  async function doResume() {
    if (state !== 'paused') return;
    const k = takes.length;
    current = spawnTake(k);
    state = 'recording';
    log.emit('resumed', { take: k, startSegment: nextSegment });
  }

  async function doStop(reason) {
    if (state === 'stopping') return;
    state = 'stopping';
    log.emit('stop_requested', { reason });
    await endCurrentTake();
    try { watcher.close(); } catch {}
    control.close?.();

    const summary = await finalizeSession(dir, takes);
    for (const file of summary.segments) {
      const m = file.match(CONFIG.files.segmentGlob);
      if (m) emitSegment(Number(m[1]));
    }
    log.emit('recording_stopped', { takeCount: takes.length });
    log.emit('finalized', { id, ...summary });

    // Publish: upload any remaining segments + init, then the playlist last (the
    // go-live act). The segments mostly streamed up during recording, so /stop is
    // near-instant. Only attempted on a valid local recording.
    if (uploader && summary.ok) {
      const pub = await uploader.finalizePublish({ segments: summary.segments });
      log.emit('upload_finalized', {
        published: pub.published,
        confirmed: pub.confirmed.length,
        failed: pub.failed,
      });
    }

    log.close();
    try { fs.unlinkSync(fifoPath); } catch {}
    process.exit(summary.ok ? 0 : 1);
  }

  // --- control wiring ---
  const control = watchControl(fifoPath);
  control.on('command', (cmd) => {
    switch (cmd) {
      case 'pause': return void enqueue(doPause);
      case 'resume': return void enqueue(doResume);
      case 'stop': return void enqueue(() => doStop('control:stop'));
      default: return void log.emit('command_ignored', { command: cmd });
    }
  });
  control.on('error', (e) => log.emit('error', { phase: 'control', message: e.message }));

  process.on('SIGINT', () => enqueue(() => doStop('signal:SIGINT')));
  process.on('SIGTERM', () => enqueue(() => doStop('signal:SIGTERM')));

  // --- start take 0 ---
  current = spawnTake(0);
  log.emit('recording_started', { pid: current.ff.pid });
}

main().catch((e) => {
  process.stderr.write(`recorder fatal: ${e?.stack || e}\n`);
  process.exit(1);
});
