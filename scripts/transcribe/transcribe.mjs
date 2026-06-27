#!/usr/bin/env node
// transcribe — turn a finished recording into a timestamped transcript (SPEC §7,
// milestone M5c). Runs whisper over the session's preview.mp4 and writes a
// normalized transcript.json the title/TL;DR/chapters skill reads. whisper is
// OPTIONAL: if it's missing or the recording has no speech this exits 0 with a
// skip reason — the recording still renders locally without it (SPEC §8).
//
// Usage:
//   node transcribe.mjs --session <dir> [--audio <file>] [--model base]
//                       [--whisper <bin>] [--json]
// Defaults: --audio <session>/preview.mp4, --model base.
// Output: ndjson events on stdout; appended to <session>/events.ndjson if --session.
//         With --json, prints the parsed transcript object on stdout instead.

import fs from 'node:fs';
import path from 'node:path';

import { runTranscribe, DEFAULT_MODEL } from './lib/transcribe.mjs';
import { spawnWhisper } from './lib/whisper.mjs';

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
  }
  return o;
}

function die(msg, code = 2) {
  process.stderr.write(msg + '\n');
  process.exit(code);
}

const opts = parseArgs(process.argv.slice(2));
const flag = (k) => (opts[k] !== undefined && opts[k] !== 'true' ? opts[k] : undefined);

const sessionDir = flag('session') ? path.resolve(flag('session')) : null;
const audioPath = flag('audio') ? path.resolve(flag('audio')) : null;
if (!sessionDir && !audioPath) die('Need a recording to transcribe: --session <dir> or --audio <file>.');
if (sessionDir && !fs.existsSync(sessionDir)) die(`No such session dir: ${sessionDir}`);

// Mirror the other CLIs: emit each event to stdout AND append to the session's
// events.ndjson (the durable artifact the next /shroom run drains, SPEC §6).
const eventsFile = sessionDir ? path.join(sessionDir, 'events.ndjson') : null;
const asJson = opts.json === 'true';
const log = (event, fields = {}) => {
  const line = JSON.stringify({ ts: new Date().toISOString(), event, ...fields }) + '\n';
  if (!asJson) process.stdout.write(line);
  if (eventsFile) {
    try { fs.appendFileSync(eventsFile, line); } catch { /* best-effort durability */ }
  }
};

const runWhisper = (a) => spawnWhisper({ ...a, bin: flag('whisper') ?? 'whisper' });

const result = await runTranscribe({
  sessionDir,
  audioPath,
  model: flag('model') ?? DEFAULT_MODEL,
  runWhisper,
  log,
});

if (asJson) process.stdout.write(JSON.stringify(result) + '\n');
// A missing transcript is a soft skip, not a failure: exit 0 so the record flow
// continues (page/deploy don't require a transcript).
process.exit(0);
