#!/usr/bin/env node
// Standalone uploader / crash-resume CLI. Diffs a finished (or interrupted) session
// dir against the bucket and uploads the gap, then publishes the playlist. Idempotent
// — safe to re-run. The recorder calls the same Uploader inline during recording;
// this CLI is the manual / recovery entry point.
//
// Usage:  node upload.mjs <session-dir> [--id <id>]
// Creds:  ~/.shroom/credentials.json  or  SHROOM_S3_* env vars.

import fs from 'node:fs';
import path from 'node:path';
import { loadStorageConfig, isConfigured, missingFields } from './lib/storage-config.mjs';
import { Uploader } from './lib/uploader.mjs';

function readIdFromEvents(dir) {
  const p = path.join(dir, 'events.ndjson');
  if (!fs.existsSync(p)) return null;
  for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
    if (!line.trim()) continue;
    try {
      const e = JSON.parse(line);
      if (e.event === 'session_started' && e.id) return e.id;
    } catch { /* skip malformed line */ }
  }
  return null;
}

const args = process.argv.slice(2);
const dir = path.resolve(args.find((a) => !a.startsWith('--')) ?? '.');
const idIdx = args.indexOf('--id');
const id = (idIdx >= 0 ? args[idIdx + 1] : null) ?? readIdFromEvents(dir);

if (!fs.existsSync(dir)) {
  console.error(`No such directory: ${dir}`);
  process.exit(2);
}
if (!id) {
  console.error('Could not determine recording id (pass --id, or run in a dir with events.ndjson).');
  process.exit(2);
}

const cfg = loadStorageConfig();
if (!isConfigured(cfg)) {
  console.error(`Storage not configured. Missing: ${missingFields(cfg).join(', ')}.`);
  console.error('Provide ~/.shroom/credentials.json or SHROOM_S3_* env vars.');
  process.exit(3);
}

const log = (event, fields) =>
  process.stdout.write(JSON.stringify({ ts: new Date().toISOString(), event, ...fields }) + '\n');

const up = new Uploader(cfg, { id, dir, log });
const r = await up.syncDir();
log('sync_complete', r);
process.exit(r.failed.length === 0 ? 0 : 1);
