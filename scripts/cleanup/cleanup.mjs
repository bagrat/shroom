#!/usr/bin/env node
// cleanup — the deterministic backend for the `cleanup` skill (green-set item 5).
//
// The judgment ("which of these is stale? keep it? really delete?") lives in the
// skill; this script is the exact, repeatable mechanism it calls. Every mutating
// op is explicit and single-target (one session / one id) — there is no "delete
// everything" sweep here; the skill loops after the user confirms each.
//
// Subcommands (all take --json):
//   scan [--verify]        List local recordings with state + sizes + age. --verify
//                          additionally HEADs each one's remote playlist.
//   prune-local --session  Drop the heavy local HLS bytes (init + segments + the
//                          per-take intermediates) but KEEP preview.mp4 — the one
//                          watchable file. Refuses unless the remote copy is
//                          confirmed present (or --force), so it never deletes the
//                          only copy of a recording.
//   delete-local --session Remove a whole local session dir.
//   delete-remote --id     Delete every `<id>/*` object from the bucket (SigV4).
//   upload-mp4 --session   Upload preview.mp4 → `<id>/video.mp4` and print the
//                          public download URL (for the player's Download button).
//   archive-local --session  The record flow's automatic post-stop step: upload the
//                          watchable MP4 (so the Download button has a target) AND
//                          reclaim disk by pruning local HLS — each half best-effort
//                          and individually gated (see below). Composes upload-mp4 +
//                          prune-local in one call so the flow launches one task.
//
// Mechanism only — never decides on its own what to remove. Mutating subcommands
// act on exactly the target passed in (the skill after the user's yes, or — for
// archive-local — the just-recorded session the record flow hands over).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { loadStorageConfig, isConfigured } from '../uploader/lib/storage-config.mjs';
import { headObject, deletePrefix, putObject } from '../uploader/lib/s3.mjs';
import { readCreds, credsPath } from '../setup/lib/credentials.mjs';

const HOME = os.homedir();
export const RECORDINGS_ROOT = path.join(HOME, '.shroom', 'recordings');

// HLS / intermediate artifacts that are safe to drop once the remote copy is up.
// preview.mp4 (the final watchable file), events.ndjson, and the page bits stay.
const PRUNE_EXACT = new Set(['init.mp4', 'stream.m3u8', 'ffmpeg.log', 'preview_concat.txt']);
const PRUNE_RE = [/^seg_\d+\.m4s$/, /^stream_\d+\.m3u8$/, /^preview_\d+\.mp4$/];
const isPrunable = (f) => PRUNE_EXACT.has(f) || PRUNE_RE.some((re) => re.test(f));

function parseArgs(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) continue;
    const key = argv[i].slice(2);
    o[key] = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : true;
  }
  return o;
}

function dirSize(dir, filter = () => true) {
  let bytes = 0;
  let entries;
  try { entries = fs.readdirSync(dir); } catch { return 0; }
  for (const f of entries) {
    try {
      const st = fs.statSync(path.join(dir, f));
      if (st.isFile() && filter(f)) bytes += st.size;
    } catch { /* vanished mid-scan */ }
  }
  return bytes;
}

// Read the id + createdAt + published state from a session's events.ndjson.
// Falls back to the dir-name suffix for the id (`YYYYMMDD-HHMMSS-<id>` → slice 16),
// which is robust even though the base64url id itself can contain '-'/'_'.
function sessionMeta(dir, name) {
  let id = name.length > 16 ? name.slice(16) : name;
  let createdAt = null;
  let published = false;
  let playbackUrl = null;
  try {
    for (const line of fs.readFileSync(path.join(dir, 'events.ndjson'), 'utf8').split('\n')) {
      if (!line.trim()) continue;
      let e;
      try { e = JSON.parse(line); } catch { continue; }
      if (e.event === 'session_started') {
        if (e.id) id = e.id;
        if (e.ts) createdAt = e.ts;
      }
      if (e.event === 'published' && e.playbackUrl) { published = true; playbackUrl = e.playbackUrl; }
    }
  } catch { /* no events file — use fallbacks */ }
  if (!createdAt) {
    try { createdAt = fs.statSync(dir).mtime.toISOString(); } catch { /* ignore */ }
  }
  return { id, createdAt, published, playbackUrl };
}

export function scanSessions(root = RECORDINGS_ROOT) {
  let names;
  try { names = fs.readdirSync(root); } catch { return []; }
  const sessions = [];
  for (const name of names) {
    const dir = path.join(root, name);
    let st;
    try { st = fs.statSync(dir); } catch { continue; }
    if (!st.isDirectory()) continue;
    const { id, createdAt, published, playbackUrl } = sessionMeta(dir, name);
    const totalBytes = dirSize(dir);
    const prunableBytes = dirSize(dir, isPrunable);
    const hasPreviewMp4 = fs.existsSync(path.join(dir, 'preview.mp4'));
    const hasLocalHls = prunableBytes > 0;
    const ageDays = createdAt ? Math.floor((Date.now() - Date.parse(createdAt)) / 86400000) : null;
    sessions.push({
      id, name, dir, createdAt, ageDays,
      published, playbackUrl,
      totalBytes, prunableBytes, hasLocalHls, hasPreviewMp4,
    });
  }
  // Newest first (by createdAt, else name which is timestamp-prefixed).
  sessions.sort((a, b) => (Date.parse(b.createdAt || 0) || 0) - (Date.parse(a.createdAt || 0) || 0) || (a.name < b.name ? 1 : -1));
  return sessions;
}

function client() {
  const cfg = loadStorageConfig();
  if (!isConfigured(cfg)) return null;
  return cfg;
}

async function remotePlaylistPresent(cl, id) {
  const { exists } = await headObject(cl, `${id}/stream.m3u8`);
  return exists;
}

// --- subcommands -----------------------------------------------------------

async function cmdScan(opts) {
  const sessions = scanSessions();
  if (opts.verify) {
    const cl = client();
    for (const s of sessions) {
      s.remoteConfirmed = cl ? await remotePlaylistPresent(cl, s.id).catch(() => null) : null;
    }
  }
  return {
    ok: true,
    root: RECORDINGS_ROOT,
    count: sessions.length,
    totalBytes: sessions.reduce((n, s) => n + s.totalBytes, 0),
    prunableBytes: sessions.reduce((n, s) => n + s.prunableBytes, 0),
    sessions,
  };
}

function resolveSession(opts) {
  const dir = path.resolve(String(opts.session));
  // Safety: only ever touch a session dir strictly UNDER the recordings root —
  // never the root itself (so a stray --session <root> can't wipe everything).
  if (dir === RECORDINGS_ROOT || !(dir + path.sep).startsWith(RECORDINGS_ROOT + path.sep)) {
    throw new Error(`refusing: ${dir} is not a session dir under ${RECORDINGS_ROOT}`);
  }
  if (!fs.existsSync(dir)) throw new Error(`no such session dir: ${dir}`);
  const name = path.basename(dir);
  return { dir, name, ...sessionMeta(dir, name) };
}

// Drop the prunable HLS/intermediate files from a session dir, keeping preview.mp4
// and the page bits. Pure local fs op — callers apply the safety guards (the dir is
// under the recordings root, the remote copy is confirmed) BEFORE invoking.
export function pruneDir(dir) {
  const removed = [];
  let freedBytes = 0;
  for (const f of fs.readdirSync(dir)) {
    if (!isPrunable(f)) continue;
    const p = path.join(dir, f);
    try {
      const sz = fs.statSync(p).size;
      fs.rmSync(p);
      removed.push(f); freedBytes += sz;
    } catch { /* skip */ }
  }
  return { freedBytes, removed, keptMp4: fs.existsSync(path.join(dir, 'preview.mp4')) };
}

async function cmdPruneLocal(opts) {
  const s = resolveSession(opts);
  if (!opts.force) {
    const cl = client();
    if (!cl) return { ok: false, reason: 'storage_not_configured', hint: 'remote copy unverifiable; pass --force only if you are sure' };
    const present = await remotePlaylistPresent(cl, s.id).catch(() => false);
    if (!present) return { ok: false, reason: 'remote_not_confirmed', id: s.id, hint: 'the recording is not (fully) uploaded; refusing to drop the only copy. --force to override' };
  }
  return { ok: true, id: s.id, dir: s.dir, ...pruneDir(s.dir) };
}

async function cmdDeleteLocal(opts) {
  const s = resolveSession(opts);
  const freed = dirSize(s.dir);
  fs.rmSync(s.dir, { recursive: true, force: true });
  return { ok: true, id: s.id, dir: s.dir, freedBytes: freed };
}

async function cmdDeleteRemote(opts) {
  const cl = client();
  if (!cl) return { ok: false, reason: 'storage_not_configured' };
  const id = String(opts.id);
  const res = await deletePrefix(cl, `${id}/`);
  return { ok: res.failed.length === 0, id, ...res };
}

async function cmdUploadMp4(opts) {
  const cl = client();
  if (!cl) return { ok: false, reason: 'storage_not_configured' };
  const s = resolveSession(opts);
  const mp4 = path.join(s.dir, 'preview.mp4');
  if (!fs.existsSync(mp4)) return { ok: false, reason: 'no_preview_mp4', dir: s.dir };
  const body = fs.readFileSync(mp4);
  const key = `${s.id}/video.mp4`;
  const put = await putObject(cl, key, body);
  if (!put.ok) return { ok: false, reason: 'upload_failed', status: put.status, key };
  const base = readCreds(credsPath()).publicBaseUrl;
  const downloadUrl = base ? `${base.replace(/\/+$/, '')}/${key}` : null;
  return { ok: true, id: s.id, key, bytes: body.length, downloadUrl };
}

// The record flow's automatic post-stop step. Two best-effort, independently gated
// halves: (1) upload preview.mp4 → `<id>/video.mp4` so the player's Download button
// has a target (needs storage); (2) prune local HLS, keeping preview.mp4 (needs the
// remote HLS confirmed). A local-only or not-yet-uploaded recording keeps every
// byte. Never fatal — `ok` reflects the session resolving, not the two halves.
async function cmdArchiveLocal(opts) {
  const s = resolveSession(opts);

  let mp4;
  if (!client()) {
    mp4 = { uploaded: false, reason: 'storage_not_configured' };
  } else {
    const r = await cmdUploadMp4(opts);
    mp4 = r.ok
      ? { uploaded: true, key: r.key, bytes: r.bytes, downloadUrl: r.downloadUrl }
      : { uploaded: false, reason: r.reason, status: r.status };
  }

  const p = await cmdPruneLocal(opts);
  const prune = p.ok
    ? { pruned: true, freedBytes: p.freedBytes, removed: p.removed, keptMp4: p.keptMp4 }
    : { pruned: false, reason: p.reason };

  return { ok: true, id: s.id, dir: s.dir, mp4, prune };
}

const COMMANDS = {
  scan: cmdScan,
  'prune-local': cmdPruneLocal,
  'delete-local': cmdDeleteLocal,
  'delete-remote': cmdDeleteRemote,
  'upload-mp4': cmdUploadMp4,
  'archive-local': cmdArchiveLocal,
};

async function main() {
  const [sub, ...rest] = process.argv.slice(2);
  const fn = COMMANDS[sub];
  if (!fn) {
    process.stderr.write(`unknown subcommand: ${sub || '(none)'}\nexpected: ${Object.keys(COMMANDS).join(', ')}\n`);
    process.exit(2);
  }
  const opts = parseArgs(rest);
  const out = await fn(opts);
  process.stdout.write(JSON.stringify(out, null, opts.json === true || opts.json === undefined ? 2 : 0) + '\n');
  process.exit(out.ok === false ? 1 : 0);
}

// argv[1] may be a symlink (e.g. a skills-dir symlink); resolve it so it matches
// import.meta.url, which Node resolves through symlinks — else main() is skipped.
const entryPath = process.argv[1] && fs.realpathSync(process.argv[1]);
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  main().catch((e) => {
    process.stdout.write(JSON.stringify({ ok: false, reason: 'error', detail: e.message }) + '\n');
    process.exit(1);
  });
}

export { isPrunable };
