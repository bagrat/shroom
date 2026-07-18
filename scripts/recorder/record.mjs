#!/usr/bin/env node
// shroom recorder core (M1 + M2) — deterministic hands for the whole ffmpeg
// lifecycle, including pause/resume as a segment boundary.
//
// Contract (the determinism boundary, SPEC §4):
//   IN  : a control fifo  (newline commands: `start`, `pause`, `resume`, `stop`,
//                          `cancel`)
//   OUT : events.ndjson   (session_started, armed, take_started, segment_ready,
//                          paused, resumed, take_ended, stop_requested,
//                          recording_started, recording_stopped, aborted,
//                          cancel_requested, discarded, finalized, error)
//
// LAUNCH ≠ CAPTURE. The recorder launches into an `armed` state — devices
// resolved, fifo + events + uploader ready — but spins up NO ffmpeg until it
// receives `start`. This is the consent boundary: a human knowingly begins the
// screen capture (in v1, by clicking the Mac tray shim, which writes `start`).
// The recorder mechanism is neutral about *who* writes the fifo; "start is
// user-only" is enforced upstream — the agent (`/shroom:record`) launches the
// recorder but does NOT write `start`, it tells the user to click the tray.
// (`--autostart` writes `start` to self at launch — for tests/headless only,
// never the consent flow.)
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
//                   [--autostart]             # begin capture at launch (tests/headless ONLY)
//   node record.mjs --preflight               # JSON for the picker: devices + quality presets + last profile
//
// --device names ANY avfoundation video source — a screen ("Capture screen 0") or a
// camera ("FaceTime HD Camera"); camera-as-source, not PiP (SPEC §4). --audio
// "default" prefers a built-in mic and never the iPhone/Continuity mic. --quality
// picks a resolution/bitrate preset (lib/quality.mjs; default normal = 1080p).
//
// Control:  echo start  > <dir>/control.fifo     (begin capture — user, from the tray)
//           echo pause  > <dir>/control.fifo
//           echo resume > <dir>/control.fifo
//           echo stop   > <dir>/control.fifo     (publish; or SIGINT/SIGTERM)
//           echo cancel > <dir>/control.fifo     (discard: stop, no publish, delete)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { PassThrough } from 'node:stream';

import { CONFIG, segName } from './lib/config.mjs';
import { resolveDevices, buildPreflight } from './lib/devices.mjs';
import { buildFfmpegArgs } from './lib/ffmpeg.mjs';
import { QUALITY, resolveQuality, ffmpegBitrate, DEFAULT_QUALITY } from './lib/quality.mjs';
import { watchControl } from './lib/control.mjs';
import { createEventLog } from './lib/events.mjs';
import { finalizeSession, maxSegmentIndex } from './lib/finalize.mjs';
import { loadStorageConfig, isConfigured } from '../uploader/lib/storage-config.mjs';
import { Uploader } from '../uploader/lib/uploader.mjs';
import { transcribeHead, segmentsForSeconds, HEAD_SECONDS } from './lib/head-transcribe.mjs';

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

// The MIC TAP is the native mic capture path (the shim binary in `--mic-tap` mode):
// it streams clean mono f32le PCM that ffmpeg reads as a 2nd input, replacing
// ffmpeg's buggy avfoundation audio (see lib/ffmpeg.mjs). The shim owns AVFoundation
// + the mic grant, so it passes its own path as `--mic-cmd`. No tap given (or it
// can't be probed) → the recording still renders locally, just without audio (a
// logged skip, never a crash — SPEC §8). A raw `node record.mjs` dev run supplies
// `--mic-cmd <built shroom>` to get audio.
const MIC_TAP_WATCHDOG_MS = 12000; // first capture must produce init.mp4 within this

function resolveMicCmd(explicit) {
  return explicit && fs.existsSync(explicit) ? explicit : null;
}

// Ask the mic tap for its device's native sample rate so ffmpeg's -ar matches
// exactly (no resample). Best-effort: any failure/timeout resolves null → audio off.
function probeMicRate(cmd, device, timeoutMs = 8000) {
  return new Promise((resolve) => {
    const a = ['--mic-tap', '--probe', ...(device ? ['--mic-device', device] : [])];
    let p;
    try { p = spawn(cmd, a, { stdio: ['ignore', 'pipe', 'pipe'] }); }
    catch { return resolve(null); }
    let out = '';
    let done = false;
    const finish = (rate) => { if (done) return; done = true; try { p.kill('SIGKILL'); } catch {} resolve(rate); };
    const timer = setTimeout(() => finish(null), timeoutMs);
    timer.unref();
    p.stdout.on('data', (d) => {
      out += d;
      const m = out.match(/rate=(\d+)/);
      if (m) { clearTimeout(timer); finish(Number(m[1])); }
    });
    p.on('error', () => { clearTimeout(timer); finish(null); });
    p.on('close', () => { clearTimeout(timer); const m = out.match(/rate=(\d+)/); finish(m ? Number(m[1]) : null); });
  });
}

// Preflight JSON for the picker (the command asks the user; this only reads). The
// payload — devices, quality catalogue, last profile with device availability — is
// built by lib/devices so the one-shot record preflight can share it verbatim.
async function preflightJson() {
  process.stdout.write(JSON.stringify(await buildPreflight()) + '\n');
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
    if (typeof fetch !== 'function') {
      // We're on a Node too old to have global fetch (<18) — every PUT/HEAD would
      // throw immediately. Don't enable uploads: skip cleanly so the recording still
      // renders locally and the process exits, rather than storming retries forever.
      // The launcher pins a current Node (run-node); this guards a stray old one.
      log.emit('upload_skipped', { reason: 'no_fetch' });
    } else if (isConfigured(cfg)) {
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

  // Mid-record head transcription (Thread 2): the instant we've captured enough to
  // cover the head, transcribe it in the BACKGROUND so a good auto-title is ready at
  // /stop with no wait. Fire once, best-effort, never blocking capture — read-only
  // over the closed segments (see head-transcribe.mjs). Off with --no-head-transcribe.
  let headFired = false;
  let headPromise = null; // the in-flight head job, retained so /stop can let it finish
                          // writing head-transcript.json before we exit (else whisper
                          // is orphaned mid-parse and only the raw head.json survives).
  const HEAD_AT = segmentsForSeconds(HEAD_SECONDS); // segment index that means ~HEAD_SECONDS is closed
  const HEAD_STOP_GRACE_MS = Number(opts['head-grace-ms']) || 4000; // max /stop wait for it
  const maybeHeadTranscribe = (i) => {
    if (headFired || opts['no-head-transcribe'] === 'true' || i < HEAD_AT) return;
    headFired = true;
    headPromise = transcribeHead({ dir, maxSeconds: HEAD_SECONDS, log: (event, fields) => log.emit(event, fields) })
      .catch((e) => log.emit('head_transcribe_skipped', { reason: 'error', message: e.message }));
  };

  const emitSegment = (i) => {
    if (i < 0 || emitted.has(i)) return;
    emitted.add(i);
    log.emit('segment_ready', { index: i, file: segName(i) });
    // Stream the closed segment up opportunistically (non-blocking, retried).
    if (uploader) uploader.enqueue(segName(i));
    maybeHeadTranscribe(i);
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
  // 'armed'  : launched, everything ready, NO ffmpeg yet — waiting for `start`.
  // 'recording' | 'paused' | 'stopping' follow once capture begins.
  let state = 'armed';
  let nextSegment = 0; // start_number for the next take
  const takes = []; // take indices that have started
  let current = null; // { k, ff, exited, mic, audioBuf, fifo, fifoStream }

  // --- native mic tap ---
  // Audio is requested when a mic was resolved. The tap (the shim binary in
  // `--mic-tap` mode) streams mono f32le PCM; ffmpeg reads it as a 2nd input. We
  // probe the device's rate ONCE at the first start (not at arm — the mic stays
  // untouched until the user begins), then reuse it for every resume take.
  const audioOn = devices.audioName != null;
  const micCmd = resolveMicCmd(opts['mic-cmd']);
  let audioTap = null;   // { device, rate } once probed & available; null = record silent
  let audioProbed = false;

  async function ensureAudioProbed() {
    if (audioProbed) return;
    audioProbed = true;
    if (!audioOn) return;
    if (!micCmd) { log.emit('audio_skipped', { reason: 'no_mic_tap' }); return; }
    const rate = await probeMicRate(micCmd, devices.audioName);
    if (!rate) { log.emit('audio_skipped', { reason: 'mic_probe_failed' }); return; }
    audioTap = { device: devices.audioName, rate };
    log.emit('audio_enabled', { device: devices.audioName, rate });
  }

  // Start one take's mic tap: a fresh per-take fifo, the tap child, and a ~16MB
  // in-memory buffer between them. NODE-IN-THE-MIDDLE is load-bearing: if the tap
  // wrote straight to the fifo, ffmpeg's ~1s avfoundation video warmup wouldn't drain
  // it, the tap would block on a full pipe, deliver zero audio, and ffmpeg would block
  // forever on its audio input. Buffering here means the mic never blocks.
  function startMicTap(k) {
    const fifo = path.join(dir, `audio_${k}.pcm`);
    try { if (fs.existsSync(fifo)) fs.unlinkSync(fifo); } catch {}
    execFileSync('mkfifo', [fifo]);
    const a = ['--mic-tap', ...(audioTap.device ? ['--mic-device', audioTap.device] : [])];
    const mic = spawn(micCmd, a, { stdio: ['ignore', 'pipe', 'pipe'] });
    mic.stderr.pipe(fs.createWriteStream(path.join(dir, `mictap_${k}.log`)));
    mic.on('error', (e) => log.emit('error', { phase: 'mic_tap_spawn', take: k, message: e.message }));
    const audioBuf = new PassThrough({ highWaterMark: 1 << 24 });
    mic.stdout.pipe(audioBuf);
    return { mic, audioBuf, fifo, fifoStream: null };
  }

  // Open the fifo's write end AFTER ffmpeg is up (it opens the read end), then flush
  // the buffered PCM into it. Opening for write blocks until a reader exists, so this
  // must follow the ffmpeg spawn.
  function connectMicFifo(mt, k) {
    const fifoStream = fs.createWriteStream(mt.fifo);
    fifoStream.on('error', (e) => log.emit('error', { phase: 'mic_fifo', take: k, message: e.message }));
    mt.audioBuf.pipe(fifoStream);
    mt.fifoStream = fifoStream;
  }

  // Tear a take's mic tap down. Called AFTER ffmpeg has finalized (see stop ordering).
  function teardownMic(t) {
    try { t.mic?.kill('SIGTERM'); } catch {}
    try { t.audioBuf?.destroy(); } catch {}
    try { t.fifoStream?.destroy(); } catch {}
    try { if (t.fifo && fs.existsSync(t.fifo)) fs.unlinkSync(t.fifo); } catch {}
  }

  function spawnTake(k) {
    const mt = audioTap ? startMicTap(k) : null;
    const audio = mt ? { fifo: mt.fifo, rate: audioTap.rate } : null;
    const args = buildFfmpegArgs({
      videoIndex: devices.video.index,
      audio,
      startNumber: nextSegment,
      take: k,
      quality,
    });
    if (k === 0) log.emit('ffmpeg_command', { argv: ['ffmpeg', ...args], cwd: dir });
    const ff = spawn('ffmpeg', args, { cwd: dir, stdio: ['pipe', 'ignore', 'pipe'] });
    ff.stderr.pipe(fs.createWriteStream(path.join(dir, `ffmpeg_${k}.log`)));
    if (mt) connectMicFifo(mt, k); // ffmpeg is now reading — open the write end + flush
    const exited = new Promise((res) => ff.on('close', (code) => res(code)));
    ff.on('error', (e) => log.emit('error', { phase: 'ffmpeg_spawn', take: k, message: e.message }));
    takes.push(k);
    log.emit('take_started', { take: k, startNumber: nextSegment, pid: ff.pid, audio: Boolean(mt) });
    return { k, ff, exited, mic: mt?.mic ?? null, audioBuf: mt?.audioBuf ?? null, fifo: mt?.fifo ?? null, fifoStream: mt?.fifoStream ?? null };
  }

  // Cleanly end the current take's ffmpeg (q → SIGTERM → SIGKILL), then — STOP
  // ORDERING — stop the mic tap. ffmpeg must finalize FIRST; killing the mic before
  // it drains would EOF the audio early and leave a video-only tail. Finally advance
  // nextSegment past whatever this take wrote.
  async function endCurrentTake() {
    if (!current) return;
    const t = current;
    try { t.ff.stdin.write('q\n'); } catch {}
    const term = setTimeout(() => { try { t.ff.kill('SIGTERM'); } catch {} }, 5000);
    const kill = setTimeout(() => { try { t.ff.kill('SIGKILL'); } catch {} }, 10000);
    term.unref(); kill.unref();
    const code = await t.exited;
    clearTimeout(term); clearTimeout(kill);
    teardownMic(t); // ffmpeg is done — now the mic can stop
    nextSegment = maxSegmentIndex(dir) + 1;
    log.emit('take_ended', { take: t.k, exitCode: code, nextSegment });
    current = null;
    return code;
  }

  // A wedge (first take never produced init.mp4) has two very different causes, and
  // the fix differs — so read ffmpeg's own log to tell them apart:
  //   • screen access isn't active THIS launch — a freshly (re)granted Screen
  //     Recording permission only takes effect on the NEXT launch. ffmpeg logs
  //     "Configuration of video device failed" / "not supported by the input device"
  //     and never opens the screen. Actionable: the user just records again.
  //   • otherwise it's the two-input audio deadlock (ffmpeg blocked on the mic input).
  // The command maps these reasons to product-voice guidance; the slugs are internal.
  function classifyWedge(k) {
    try {
      const t = fs.readFileSync(path.join(dir, `ffmpeg_${k}.log`), 'utf8');
      if (/Configuration of video device failed|not supported by the input device/i.test(t)) {
        return { reason: 'screen_grant_inactive', message: 'screen capture did not start — its permission takes effect on the next launch' };
      }
    } catch { /* no log yet → treat as the audio deadlock */ }
    return { reason: 'capture_wedged', message: 'no init segment within watchdog window' };
  }

  // Two-live-input deadlock guard (audio path only): if the first take never
  // produces init.mp4, ffmpeg is wedged waiting on the audio it can't get — q won't
  // help, so SIGKILL everything, report it, and exit rather than hang holding the
  // mic + screen. Video-only capture can't deadlock, so we only arm this with audio.
  function armMicWatchdog(t) {
    if (!t.mic) return;
    const timer = setTimeout(() => {
      if (t !== current) return; // already moved on / stopped
      if (fs.existsSync(path.join(dir, CONFIG.files.initSegment))) return; // capture is live
      const w = classifyWedge(t.k);
      log.emit('error', { phase: w.reason, take: t.k, message: w.message });
      try { t.ff.kill('SIGKILL'); } catch {}
      teardownMic(t);
      try { watcher.close(); } catch {}
      control.close?.();
      log.emit('aborted', { reason: w.reason });
      log.close();
      try { fs.unlinkSync(fifoPath); } catch {}
      process.exit(1);
    }, MIC_TAP_WATCHDOG_MS);
    timer.unref();
  }

  // Serialize control commands so pause/resume/stop never interleave.
  let chain = Promise.resolve();
  const enqueue = (fn) => {
    chain = chain.then(fn).catch((e) => log.emit('error', { phase: 'command', message: e.message }));
    return chain;
  };

  // Begin capture — only valid from `armed`. This is the act `start` triggers:
  // the user clicking the tray (the recorder doesn't do it itself, the agent
  // doesn't write it — see the header contract).
  async function doStart() {
    if (state !== 'armed') return;
    await ensureAudioProbed(); // resolve the mic rate once, here (fail-safe → silent)
    current = spawnTake(0);
    state = 'recording';
    log.emit('recording_started', { pid: current.ff.pid, audio: Boolean(current.mic) });
    armMicWatchdog(current);
  }

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
    const neverStarted = takes.length === 0; // stopped while still `armed`
    state = 'stopping';
    log.emit('stop_requested', { reason });
    await endCurrentTake(); // no-op when nothing was ever captured
    try { watcher.close(); } catch {}
    control.close?.();

    // Stopped before any capture began: clean teardown, no finalize, no publish,
    // no half-built page. (This is the cancel-before-start case; mid-recording
    // `cancel` — discard a real take — is reserved for later.)
    if (neverStarted) {
      log.emit('aborted', { reason: 'stopped_before_start' });
      log.close();
      try { fs.unlinkSync(fifoPath); } catch {}
      process.exit(0);
    }

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
      // Publish is best-effort: the local recording is already valid (SPEC §8), so a
      // network/storage failure must degrade to a local-only result, never block the
      // exit or crash. Catch anything finalize throws and report it as not-published.
      try {
        const pub = await uploader.finalizePublish({ segments: summary.segments });
        log.emit('upload_finalized', {
          published: pub.published,
          confirmed: pub.confirmed.length,
          failed: pub.failed,
        });
      } catch (e) {
        log.emit('upload_finalized', { published: false, error: e.message });
      }
    }

    // Let an in-flight head transcription finish so head-transcript.json (the /stop
    // title suggestion) lands before we exit — otherwise process.exit orphans whisper
    // mid-parse and only the raw head.json survives. Bounded: a short grace, then
    // degrade to the auto-name fallback (the command handles an absent head
    // transcript). Usually finalize + publish already covered the wait.
    if (headPromise) {
      let timer;
      const grace = new Promise((r) => { timer = setTimeout(() => r('timeout'), HEAD_STOP_GRACE_MS); timer.unref(); });
      const outcome = await Promise.race([headPromise.then(() => 'done'), grace]);
      clearTimeout(timer);
      if (outcome === 'timeout') log.emit('head_transcribe_timeout', { graceMs: HEAD_STOP_GRACE_MS });
    }

    log.close();
    try { fs.unlinkSync(fifoPath); } catch {}
    process.exit(summary.ok ? 0 : 1);
  }

  // Discard: stop ffmpeg, do NOT finalize or publish, and delete the session's
  // scratch — the user threw this recording away (the tray's "Discard"). Distinct
  // from `stop`, which is the publish act. We delete our OWN session dir only, and
  // only when it looks like ours (holds events.ndjson), so a stray --out can't
  // nuke an unrelated directory.
  async function doCancel(reason) {
    if (state === 'stopping') return;
    state = 'stopping';
    log.emit('cancel_requested', { reason });
    await endCurrentTake(); // q-stop ffmpeg if running; no-op while armed
    try { watcher.close(); } catch {}
    control.close?.();
    log.emit('discarded', { id, reason }); // echoed to stdout BEFORE we delete
    log.close();
    try {
      if (fs.existsSync(path.join(dir, CONFIG.files.events))) {
        fs.rmSync(dir, { recursive: true, force: true });
      } else {
        try { fs.unlinkSync(fifoPath); } catch {}
      }
    } catch {}
    process.exit(0);
  }

  // --- control wiring ---
  const control = watchControl(fifoPath);
  control.on('command', (cmd) => {
    switch (cmd) {
      case 'start': return void enqueue(doStart);
      case 'pause': return void enqueue(doPause);
      case 'resume': return void enqueue(doResume);
      case 'stop': return void enqueue(() => doStop('control:stop'));
      case 'cancel': return void enqueue(() => doCancel('control:cancel'));
      default: return void log.emit('command_ignored', { command: cmd });
    }
  });
  control.on('error', (e) => log.emit('error', { phase: 'control', message: e.message }));

  // Last-resort safety net: if we exit for any reason with a mic tap still running,
  // SIGKILL it so it never orphans holding the microphone (the graceful paths stop it
  // in order via endCurrentTake; this only fires on an abnormal exit).
  process.on('exit', () => { try { current?.mic?.kill('SIGKILL'); } catch {} });

  process.on('SIGINT', () => enqueue(() => doStop('signal:SIGINT')));
  process.on('SIGTERM', () => enqueue(() => doStop('signal:SIGTERM')));

  // --- armed: ready, waiting for `start` ---
  // No ffmpeg yet (LAUNCH ≠ CAPTURE — see header). The open control socket keeps
  // the event loop alive; capture begins only when `start` arrives on the fifo.
  // The agent surfaces device errors before this point, so by `armed` the only
  // thing left is the user's deliberate go.
  log.emit('armed', { fifo: fifoPath });

  // Test/headless escape hatch ONLY — never the consent flow (see header).
  if (opts.autostart === 'true') enqueue(doStart);
}

main().catch((e) => {
  process.stderr.write(`recorder fatal: ${e?.stack || e}\n`);
  process.exit(1);
});
