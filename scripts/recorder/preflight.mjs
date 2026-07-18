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
//   devices        — the full picker payload (video/audio/qualities/lastProfile)
//   prep           — with --prep, on a configured machine: the ONE native pre-record
//                    step (prime mic + Screen Recording + capture consent) PLUS the
//                    staged recording identity (id + session dir). null otherwise. This
//                    is what lets /shroom:record do every programmatic thing in a single
//                    call BEFORE the picker, then just launch the tray after it.
//
// Every read-only branch is fail-soft: a slow network, missing state, old Node — any
// problem degrades that one field, never the whole payload; those never mutate the
// machine beyond advancing their own marker files. `--prep` is the deliberate exception:
// it prompts for permissions (idempotent — silent once granted) and is likewise
// fail-soft (a not-yet-compiled app reports appMissing so the caller routes to setup).
//
// NOT surfaced here on purpose: the published-link recovery (SPEC §6). Announcing/opening
// a PRIOR recording's link in the middle of starting a NEW one is intrusive, so it's kept
// out of the record hot path — `scanPublished` + `pendingPublish` stay exported below for
// a future status/dashboard surface to use, but preflight neither computes nor advances
// their marker.
//
// Flags (all optional; most exist for tests):
//   --prep                 also run the native pre-record prep + stage the recording
//                          identity (see `prep` above); off by default so the read-only
//                          checks never prompt for permissions
//   --no-version           skip the network version check (→ version:null)
//   --version-timeout <ms> version-fetch timeout (default 2000 — keep the hot path snappy)
//   --recordings <dir>     override the recordings base (default ~/.shroom/recordings)
//   --verify               live-check the stored R2 keys in the setup status (adds a HEAD)

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { computeStatus } from '../setup/lib/status.mjs';
import { shroomDir } from '../setup/lib/credentials.mjs';
import { checkVersion } from '../version/check.mjs';
import { runPostUpdate } from '../version/post-update.mjs';
import { buildPreflight } from './lib/devices.mjs';

const MAX_PENDING = 5;         // cap what we surface — a fresh run only cares about recent
const SCAN_DIRS = 20;          // newest-N recording dirs to inspect (bounds IO)

function parseArgs(argv) {
  const a = { version: true, timeout: 2000, verify: false, prep: false };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--prep') a.prep = true;
    else if (k === '--no-version') a.version = false;
    else if (k === '--version-timeout') a.timeout = Number(argv[++i]) || 2000;
    else if (k === '--recordings') a.recordings = argv[++i];
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
export function pendingPublish({ base, stateFile = path.join(shroomDir(), 'publish-surfaced.json'), advance = true }) {
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

// The compiled shim binary, resolved through any skills-dir symlink so it points at the
// real repo checkout where build/ lives (same realpath trick as the entry guard below).
function shimBinary() {
  const here = path.dirname(fs.realpathSync(fileURLToPath(import.meta.url)));
  return path.join(here, '..', 'shim', 'macos', 'build', 'shroom.app', 'Contents', 'MacOS', 'shroom');
}

// YYYYMMDD-HHMMSS in LOCAL time — the session-dir prefix, matching the `date
// +%Y%m%d-%H%M%S` the command used to shell out for (so dirs still sort + eyeball nicely).
function stamp(d = new Date()) {
  const p = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

// The single native pre-record step, run only on a configured machine (ready + --prep):
// one throwaway shim launch primes Microphone + Screen Recording as "shroom" and, when
// screen is already granted, the capture consent. It also mints the recording id and its
// session dir HERE — in the deterministic core — so the command needs no id/timestamp
// shell calls and can hand the tray a ready path after the picker. Fail-soft: a not-yet-
// compiled app reports appMissing (→ caller routes to /shroom:setup); any other hiccup
// still returns id + sessionDir (screen/mic null) so the tray's launch-time safety nets
// carry the record. The dir is only NAMED here — the shim/recorder mkdir it at launch.
export function runPrep({ base }) {
  const id = crypto.randomBytes(12).toString('base64url');
  const sessionDir = path.join(base, `${stamp()}-${id}`);
  let screen = null, mic = null, appMissing = false;
  try {
    const out = execFileSync(shimBinary(), ['--prep'], { encoding: 'utf8' });
    const j = JSON.parse((out.trim().split('\n').pop()) || '{}');
    screen = j.screen ?? null;
    mic = j.mic ?? null;
  } catch (e) {
    if (e && e.code === 'ENOENT') appMissing = true;   // not compiled yet → needs setup
    // else best-effort: leave screen/mic null; the tray primes again at launch.
  }
  return { ok: !appMissing, appMissing, screen, mic, id, sessionDir };
}

export async function runPreflight(args = {}) {
  const base = args.recordings || path.join(os.homedir(), '.shroom', 'recordings');

  // Fire every check at once; each is independently fail-soft.
  const [setup, version, devices] = await Promise.all([
    computeStatus({ verify: args.verify }).catch((e) => ({ ok: false, ready: false, error: 'status_failed', detail: String(e?.message || e) })),
    args.version === false
      ? Promise.resolve(null)
      : checkVersion({ timeout: args.timeout }).catch(() => null),
    buildPreflight().catch((e) => ({ error: 'devices_failed', detail: String(e?.message || e), video: [], audio: [], defaultAudioName: null, qualities: [], lastProfile: null })),
  ]);

  // Cheap + synchronous; keep it out of the await group.
  let postUpdate = null;
  try { postUpdate = runPostUpdate({ advance: true }); } catch { postUpdate = null; }

  const ready = Boolean(setup && setup.ready);

  // The one native pre-record step + staged recording identity — only when asked
  // (--prep) AND configured (no point prompting for permissions on a machine that's
  // about to be routed to setup). Fail-soft so a prep hiccup never sinks the payload.
  let prep = null;
  if (args.prep && ready) {
    try { prep = runPrep({ base }); } catch { prep = null; }
  }

  return {
    ok: true,
    ready,
    setup,
    version,
    postUpdate,
    devices,
    prep,
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
