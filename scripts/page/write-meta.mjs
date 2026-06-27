#!/usr/bin/env node
// write-meta — the deterministic `<id>.md` writer (SPEC §3, milestone M5c).
//
// The determinism boundary in one file: the title/TL;DR/chapters *skill* decides
// the content (judgment — what the title says, where a chapter falls); THIS script
// writes the file (mechanism — stable serialization, escaping, key order, body).
// It serializes the agent-authored fields plus the recording's transcript into the
// git library as `<library>/<id>.md`, the exact substrate build-page renders from
// (it reads the same file via `--meta`). Re-runnable: re-authoring overwrites the
// record in place with a clean, stable diff.
//
// Usage:
//   node write-meta.mjs --id <id> [--session <dir>] \
//        --title "<title>" [--tldr "<tldr>"] [--chapters '<json>'] \
//        [--library <dir>] [--created-at <iso>] [--duration-sec <n>] [--lang <code>]
//
// Library dir resolution: --library  >  creds `library`  >  ~/shroom.
// From <session> (when given): transcript body + language from transcript.json,
// createdAt (session_started) + durationSec (finalized) from events.ndjson — each
// overridable by the matching flag. Output: a JSON summary on stdout.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { writeMetadataFile } from './lib/metadata.mjs';

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

// Read one field out of ~/.shroom/credentials.json without pulling in the secrets
// loader — we only want the (non-secret) `library` path.
function libraryFromCreds() {
  const p = path.join(os.homedir(), '.shroom', 'credentials.json');
  try { return JSON.parse(fs.readFileSync(p, 'utf8')).library; } catch { return undefined; }
}

// Pull createdAt (first session_started ts) + durationSec (finalized) from a
// recording's events.ndjson. The flags win over these when provided.
function readSessionFacts(sessionDir) {
  const facts = {};
  const p = path.join(sessionDir, 'events.ndjson');
  if (!fs.existsSync(p)) return facts;
  for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    let e;
    try { e = JSON.parse(line); } catch { continue; }
    if (e.event === 'session_started' && facts.createdAt == null) facts.createdAt = e.ts;
    if (e.event === 'finalized' && typeof e.durationSec === 'number') facts.durationSec = e.durationSec;
  }
  return facts;
}

// The skill hands chapters as JSON; be liberal in what we accept (t/time/start,
// label/title) and strict in what we store: { t, label }, rounded, sorted, no
// empties — exactly the shape render.mjs consumes.
function normalizeChapters(raw) {
  let arr;
  try { arr = JSON.parse(raw); } catch { die('--chapters must be a JSON array.'); }
  if (!Array.isArray(arr)) die('--chapters must be a JSON array.');
  return arr
    .map((c) => {
      const t = Number(c?.t ?? c?.time ?? c?.start);
      const label = String(c?.label ?? c?.title ?? '').trim();
      return { t: Number.isFinite(t) ? Math.max(0, Math.round(t)) : 0, label };
    })
    .filter((c) => c.label)
    .sort((a, b) => a.t - b.t);
}

const opts = parseArgs(process.argv.slice(2));
const flag = (k) => (opts[k] !== undefined && opts[k] !== 'true' ? opts[k] : undefined);

const id = flag('id');
if (!id) die('A recording id is required: --id <id>.');
const title = flag('title');
if (!title) die('A title is required: --title "<title>". (The skill authors it.)');

const sessionDir = flag('session') ? path.resolve(flag('session')) : null;
if (sessionDir && !fs.existsSync(sessionDir)) die(`No such session dir: ${sessionDir}`);

const library = path.resolve(flag('library') ?? libraryFromCreds() ?? path.join(os.homedir(), 'shroom'));
fs.mkdirSync(library, { recursive: true });

// Transcript body (+ language/duration fallback) from the normalized transcript.
let transcriptBody = '';
let transcriptDuration;
let transcriptLang;
if (sessionDir) {
  const tp = path.join(sessionDir, 'transcript.json');
  if (fs.existsSync(tp)) {
    try {
      const t = JSON.parse(fs.readFileSync(tp, 'utf8'));
      transcriptBody = typeof t.transcript === 'string' ? t.transcript : '';
      if (typeof t.durationSec === 'number') transcriptDuration = t.durationSec;
      if (typeof t.language === 'string') transcriptLang = t.language;
    } catch { /* a malformed transcript is a soft skip — write the record without a body */ }
  }
}

const facts = sessionDir ? readSessionFacts(sessionDir) : {};

const durationSec = Number(
  flag('duration-sec') ?? facts.durationSec ?? transcriptDuration ?? 0,
);
const createdAt = flag('created-at') ?? facts.createdAt ?? new Date().toISOString();
const lang = flag('lang') ?? transcriptLang;
const chapters = flag('chapters') ? normalizeChapters(flag('chapters')) : [];

const meta = { id, title };
if (flag('tldr')) meta.tldr = flag('tldr');
if (Number.isFinite(durationSec) && durationSec > 0) meta.durationSec = Math.round(durationSec);
meta.createdAt = createdAt;
if (lang) meta.lang = lang;
if (chapters.length) meta.chapters = chapters;

const metaPath = path.join(library, `${id}.md`);
writeMetadataFile(metaPath, { meta, transcript: transcriptBody });

process.stdout.write(
  JSON.stringify({
    event: 'metadata_written',
    id,
    metaPath,
    library,
    chapters: chapters.length,
    durationSec: meta.durationSec ?? 0,
    hasTranscript: Boolean(transcriptBody),
  }) + '\n',
);
