#!/usr/bin/env node
// One-shot record preflight — runs every read-only pre-record check IN PARALLEL and
// prints ONE JSON payload, so `/shroom:record` issues a single command instead of
// 4-5 sequential round-trips before it can put the user in front of the picker. The
// win is latency: the model waits on one tool call, and the slow part (the version
// network fetch) overlaps the device enumeration instead of following it.
//
// Aggregates:
//   ready          — setup gate (offline; false → the command routes to /shroom:setup)
//   setup          — the full status object (library/storage/pages) for messaging
//   version        — update-available check (network, best-effort, fail-soft)
//   postUpdate     — per-version "what's new" since last-seen (advances its own
//                    marker; each version fires once)
//   pendingPublish — recordings published in a PRIOR run whose live link the user
//                    may not have seen yet (fire-once; advances a "surfaced" marker)
//   devices        — the full picker payload (video/audio/qualities/lastProfile)
//
// Every branch is fail-soft: a slow network, missing state, old Node — any problem
// degrades that one field, never the whole payload. Like post-update, it NEVER
// mutates the machine beyond advancing its own marker files.
//
// Flags (all optional; most exist for tests):
//   --no-version           skip the network version check (→ version:null)
//   --version-timeout <ms> version-fetch timeout (default 2000 — keep the hot path snappy)
//   --recordings <dir>     override the recordings base (default ~/.shroom/recordings)
//   --surfaced-state <p>   override the pending-publish "surfaced" marker path
//   --no-advance           report pendingPublish without advancing its marker (dry-run)
//   --verify               live-check the stored R2 keys in the setup status (adds a HEAD)

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { computeStatus } from '../setup/lib/status.mjs';
import { shroomDir } from '../setup/lib/credentials.mjs';
import { checkVersion } from '../version/check.mjs';
import { runPostUpdate } from '../version/post-update.mjs';
import { buildPreflight } from './lib/devices.mjs';

const MAX_PENDING = 5;         // cap what we surface — a fresh run only cares about recent
const SCAN_DIRS = 20;          // newest-N recording dirs to inspect (bounds IO)

function parseArgs(argv) {
  const a = { advance: true, version: true, timeout: 2000, verify: false };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--no-advance') a.advance = false;
    else if (k === '--no-version') a.version = false;
    else if (k === '--version-timeout') a.timeout = Number(argv[++i]) || 2000;
    else if (k === '--recordings') a.recordings = argv[++i];
    else if (k === '--surfaced-state') a.surfacedState = argv[++i];
    else if (k === '--verify') a.verify = true;
  }
  return a;
}

// Scan the recordings dir (newest-first) for sessions that reached a terminal
// `published` event carrying a live `playbackUrl`. Returns the recent ones in
// descending order — pure, so it's testable without a home dir. Only the terminal
// publish event carries `playbackUrl` (the uploader's own `published` doesn't), so
// that field is exactly the "the link is live" signal.
export function scanPublished(base, limit = SCAN_DIRS) {
  let dirs;
  try {
    dirs = fs.readdirSync(base, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => ({ name: d.name, mtime: fs.statSync(path.join(base, d.name)).mtimeMs }))
      .sort((a, b) => b.mtime - a.mtime)
      .slice(0, limit);
  } catch { return []; }

  const found = [];
  for (const d of dirs) {
    const ev = path.join(base, d.name, 'events.ndjson');
    let text;
    try { text = fs.readFileSync(ev, 'utf8'); } catch { continue; }
    let hit = null;
    for (const line of text.split('\n')) {
      if (!line.trim()) continue;
      let e; try { e = JSON.parse(line); } catch { continue; }
      if (e.event === 'published' && e.playbackUrl) {
        // id from the event, else parsed from the "<timestamp>-<id>" dir name.
        const id = e.id || d.name.replace(/^\d{8}-\d{6}-/, '');
        hit = { id, dir: path.join(base, d.name), playbackUrl: e.playbackUrl };
      }
    }
    if (hit) found.push(hit);
  }
  return found;
}

function readSurfaced(file) {
  try {
    const j = JSON.parse(fs.readFileSync(file, 'utf8'));
    return Array.isArray(j.surfaced) ? j.surfaced : [];
  } catch { return []; }
}

function writeSurfaced(file, ids) {
  try {
    fs.mkdirSync(path.dirname(file), { recursive: true, mode: 0o700 });
    fs.writeFileSync(file, JSON.stringify({ surfaced: ids, updatedAt: new Date().toISOString() }, null, 2) + '\n');
    return true;
  } catch { return false; }
}

// pendingPublish: published recordings the user hasn't been shown yet. Fire-once via
// a "surfaced" marker (same pattern as post-update's version marker). First ever run
// baselines silently — it records what's already published and reports nothing, so a
// fresh install never replays the whole history.
export function pendingPublish({ base, stateFile, advance }) {
  const published = scanPublished(base);
  const firstRun = !fs.existsSync(stateFile);
  const surfaced = readSurfaced(stateFile);
  const allIds = published.map((p) => p.id);

  const candidates = firstRun ? [] : published.filter((p) => !surfaced.includes(p.id)).slice(0, MAX_PENDING);

  if (advance) {
    // Advance the marker to cover everything currently published, so each id is
    // surfaced at most once, ever.
    const merged = Array.from(new Set([...surfaced, ...allIds]));
    writeSurfaced(stateFile, merged);
  }
  return { candidates, firstRun };
}

export async function runPreflight(args = {}) {
  const base = args.recordings || path.join(os.homedir(), '.shroom', 'recordings');
  const stateFile = args.surfacedState || path.join(shroomDir(), 'publish-surfaced.json');

  // Fire every check at once; each is independently fail-soft.
  const [setup, version, devices] = await Promise.all([
    computeStatus({ verify: args.verify }).catch((e) => ({ ok: false, ready: false, error: 'status_failed', detail: String(e?.message || e) })),
    args.version === false
      ? Promise.resolve(null)
      : checkVersion({ timeout: args.timeout }).catch(() => null),
    buildPreflight().catch((e) => ({ error: 'devices_failed', detail: String(e?.message || e), video: [], audio: [], defaultAudioName: null, qualities: [], lastProfile: null })),
  ]);

  // These two are cheap + synchronous; keep them out of the await group.
  let postUpdate = null;
  try { postUpdate = runPostUpdate({ advance: true }); } catch { postUpdate = null; }
  let pending = { candidates: [], firstRun: false };
  try { pending = pendingPublish({ base, stateFile, advance: args.advance !== false }); } catch { /* fail-soft */ }

  return {
    ok: true,
    ready: Boolean(setup && setup.ready),
    setup,
    version,
    postUpdate,
    pendingPublish: pending.candidates,
    devices,
  };
}

// argv[1] may be a symlink (skills-dir symlink); resolve it so it matches
// import.meta.url — else main() is skipped when invoked through the symlink.
const entryPath = process.argv[1] && fs.realpathSync(process.argv[1]);
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const args = parseArgs(process.argv.slice(2));
  runPreflight(args).then((out) => {
    process.stdout.write(JSON.stringify(out) + '\n');
    process.exit(0);
  }).catch((e) => {
    // Absolute last resort — even a total failure prints valid, minimal JSON so the
    // command never chokes on parse.
    process.stdout.write(JSON.stringify({ ok: false, ready: false, error: 'preflight_failed', detail: String(e?.message || e) }) + '\n');
    process.exit(0);
  });
}
