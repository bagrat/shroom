#!/usr/bin/env node
// dev/collect-logs — read-only diagnostics collector for shroom dev/test runs.
//
// When you hit a bug while testing (in a clean-user window, or anywhere), the
// evidence is scattered across ~/.shroom and the git library. This gathers it
// for ONE recording into a single report the Develop session can read: the event
// log, the ffmpeg tail, the file inventory, the library record, the deploy/site
// state, the version + dashboard markers, and a SECRET-SAFE creds summary.
//
// It only ever READS. It NEVER prints credentials — just which fields are set and
// the public URLs (bucket name / account id are reported as present/absent).
//
// Usage:
//   node dev/collect-logs.mjs                # newest recording
//   node dev/collect-logs.mjs latest         # same
//   node dev/collect-logs.mjs <id>           # a specific recording id
//   node dev/collect-logs.mjs --list         # list recent recordings, then stop
//   node dev/collect-logs.mjs <id> --tail 80 # more ffmpeg.log lines (default 40)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { scanSessions } from '../scripts/cleanup/cleanup.mjs';
import { parseMetadata } from '../scripts/page/lib/metadata.mjs';
import { readCreds, credsPath } from '../scripts/setup/lib/credentials.mjs';

const HOME = os.homedir();
const SHROOM = path.join(HOME, '.shroom');
const SITE_ROOT = path.join(SHROOM, 'site');

const args = process.argv.slice(2);
const wantList = args.includes('--list');
const tailN = (() => {
  const i = args.indexOf('--tail');
  return i >= 0 ? Number(args[i + 1]) || 40 : 40;
})();
const target = args.find((a) => !a.startsWith('--') && a !== String(tailN)) || 'latest';

const out = (s = '') => process.stdout.write(s + '\n');
const hr = (t) => out(`\n=== ${t} ===`);
const human = (n) => {
  n = Number(n) || 0;
  if (n < 1024) return n + ' B';
  const u = ['KB', 'MB', 'GB']; let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return `${n.toFixed(1)} ${u[i]}`;
};
const exists = (p) => { try { return fs.existsSync(p); } catch { return false; } };
const sizeOf = (p) => { try { return fs.statSync(p).size; } catch { return 0; } };
const readSafe = (p) => { try { return fs.readFileSync(p, 'utf8'); } catch { return null; } };

const sessions = scanSessions();

if (wantList || sessions.length === 0) {
  hr(`recordings (${sessions.length})`);
  for (const s of sessions) {
    out(`  ${s.id}  ${s.createdAt || '?'}  published=${s.published}  ${human(s.totalBytes)}  ${s.name}`);
  }
  if (wantList) process.exit(0);
  if (sessions.length === 0) { out('\n(no recordings under ~/.shroom/recordings)'); process.exit(0); }
}

const rec = target === 'latest' ? sessions[0] : sessions.find((s) => s.id === target);
if (!rec) {
  out(`No recording matches "${target}". Try --list.`);
  process.exit(1);
}

const dir = rec.dir;
out(`# shroom diagnostics for ${rec.id}`);
out(`session dir : ${dir}`);
out(`created     : ${rec.createdAt}`);
out(`published   : ${rec.published}${rec.playbackUrl ? '  ' + rec.playbackUrl : ''}`);

// --- event log (small ndjson; print in full) ---
hr('events.ndjson');
out(readSafe(path.join(dir, 'events.ndjson')) || '(none)');

// --- ffmpeg log tail ---
hr(`ffmpeg.log (last ${tailN})`);
const flog = readSafe(path.join(dir, 'ffmpeg.log'));
out(flog ? flog.trim().split('\n').slice(-tailN).join('\n') : '(none)');

// --- file inventory ---
hr('files on disk');
let segCount = 0;
try { segCount = fs.readdirSync(dir).filter((f) => /^seg_\d+\.m4s$/.test(f)).length; } catch {}
for (const f of ['init.mp4', 'stream.m3u8', 'preview.mp4', 'transcript.json', 'index.html', 'poster.jpg']) {
  out(`  ${exists(path.join(dir, f)) ? '✓' : '·'} ${f}${exists(path.join(dir, f)) ? '  ' + human(sizeOf(path.join(dir, f))) : ''}`);
}
out(`  seg_*.m4s × ${segCount}`);

// --- transcript summary ---
const tj = readSafe(path.join(dir, 'transcript.json'));
if (tj) {
  hr('transcript.json');
  try {
    const t = JSON.parse(tj);
    out(`  language=${t.language}  durationSec=${t.durationSec}  segments=${Array.isArray(t.segments) ? t.segments.length : 0}  chars=${(t.transcript || '').length}`);
  } catch { out('  (unparseable)'); }
}

// --- library record (<id>.md) ---
hr('library record');
const libraryDir = readCreds(credsPath()).library || path.join(HOME, 'shroom');
const mdPath = path.join(libraryDir, `${rec.id}.md`);
const md = readSafe(mdPath);
if (md) {
  const { meta, transcript } = parseMetadata(md);
  out(`  ${mdPath}`);
  out(`  meta: ${JSON.stringify(meta)}`);
  out(`  transcript body: ${transcript ? transcript.length + ' chars' : 'empty'}`);
} else {
  out(`  (no ${rec.id}.md in ${libraryDir} — not committed yet)`);
}

// --- deployed page / site dir ---
hr('site dir (~/.shroom/site/<id>)');
const siteDir = path.join(SITE_ROOT, rec.id);
if (exists(siteDir)) {
  for (const f of fs.readdirSync(siteDir)) out(`  ${f}  ${human(sizeOf(path.join(siteDir, f)))}`);
} else out('  (none built)');

// --- global markers ---
hr('global state');
const vs = readSafe(path.join(SHROOM, 'version-state.json'));
out(`  version-state: ${vs ? vs.trim() : '(none)'}`);
const dash = path.join(SHROOM, 'dashboard', 'index.html');
out(`  dashboard: ${exists(dash) ? 'built ' + new Date(fs.statSync(dash).mtime).toISOString() : '(none)'}`);

// --- creds summary (SECRET-SAFE: presence + public values only) ---
hr('creds summary (no secrets)');
const c = readCreds(credsPath());
out(`  library      : ${c.library || '(unset)'}`);
out(`  bucket       : ${c.bucket ? 'set' : '(unset)'}`);
out(`  accountId    : ${c.accountId ? 'set' : '(unset)'}`);
out(`  storage keys : ${c.accessKeyId && c.secretAccessKey ? 'present' : 'absent'}`);
out(`  publicBaseUrl: ${c.publicBaseUrl || '(unset)'}`);
out(`  pagesProject : ${c.pagesProject || '(unset)'}`);
out(`  pagesBaseUrl : ${c.pagesBaseUrl || '(unset)'}`);
out(`  nodeBinDir   : ${c.nodeBinDir || '(unset)'}`);
