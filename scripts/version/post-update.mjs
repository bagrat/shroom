#!/usr/bin/env node
// post-update runner — the deterministic half of "the plugin was just updated;
// do the per-version follow-ups" (green-set item 4). It detects a version bump
// (installed plugin.json version vs a last-seen marker in ~/.shroom/), looks up
// the per-version migration entries shipped in migrations.json, and reports the
// ones newly crossed. It ADVANCES the marker as it reports, so each version's
// migration is surfaced exactly once, ever — nag-proof and idempotent.
//
// It only ever REPORTS. Any action a migration recommends (rebuild the shim,
// re-verify storage, …) is a machine mutation, so the command proposes → asks →
// runs it per the working agreement; the runner never mutates beyond writing its
// own marker file. "What's new" is judgment too — the command surfaces it.
//
// Fail-soft is the contract: a missing/corrupt manifest or state, an unwritable
// ~/.shroom, old Node — any problem yields { ok:true, pending:[] } and exit 0, so
// a post-update check can never block or slow a record/setup.
//
// Output (JSON):
//   { ok, from, to, firstRun, pending: [ { version, whatsNew, actions } ] }
//
// Flags:
//   --no-advance      report without moving the marker (testing / dry-run)
//   --manifest <p>    override the migrations manifest path (testing)
//   --state <p>       override the marker file path (testing)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { localVersion, compareSemver } from './check.mjs';
import { shroomDir } from '../setup/lib/credentials.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));

function parseArgs(argv) {
  const a = { advance: true };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === '--no-advance') a.advance = false;
    else if (k === '--manifest') a.manifest = argv[++i];
    else if (k === '--state') a.state = argv[++i];
  }
  return a;
}

function statePath(override) {
  return override || path.join(shroomDir(), 'version-state.json');
}

function readState(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return {};
  }
}

// Best-effort marker write; never throws (fail-soft). The dir is created 700 to
// match the rest of ~/.shroom even though the marker itself isn't a secret.
function writeState(file, obj) {
  try {
    fs.mkdirSync(path.dirname(file), { recursive: true, mode: 0o700 });
    fs.writeFileSync(file, JSON.stringify(obj, null, 2) + '\n');
    return true;
  } catch {
    return false;
  }
}

function loadManifest(override) {
  const file = override || path.join(HERE, 'migrations.json');
  try {
    const json = JSON.parse(fs.readFileSync(file, 'utf8'));
    return Array.isArray(json.migrations) ? json.migrations : [];
  } catch {
    return [];
  }
}

function run(args) {
  let to;
  try {
    to = localVersion();
  } catch (e) {
    return { ok: true, from: null, to: null, firstRun: false, pending: [], error: 'no_local_version', detail: e.message };
  }

  const sfile = statePath(args.state);
  const from = readState(sfile).lastSeenVersion || null;

  // First time we've ever looked: baseline to the installed version instead of
  // replaying the entire history of migrations. Record the baseline and report
  // nothing.
  if (!from) {
    if (args.advance) writeState(sfile, { lastSeenVersion: to, updatedAt: new Date().toISOString() });
    return { ok: true, from: null, to, firstRun: true, pending: [] };
  }

  // No forward movement (same version, or a downgrade) → nothing to do.
  if (compareSemver(to, from) <= 0) {
    return { ok: true, from, to, firstRun: false, pending: [] };
  }

  // Migrations strictly newer than the last-seen version, up to and including the
  // installed one, in ascending order.
  const pending = loadManifest(args.manifest)
    .filter((m) => m && m.version && compareSemver(m.version, from) > 0 && compareSemver(m.version, to) <= 0)
    .sort((a, b) => compareSemver(a.version, b.version))
    .map((m) => ({ version: m.version, whatsNew: m.whatsNew || '', actions: Array.isArray(m.actions) ? m.actions : [] }));

  // Advance the marker as we report, so these fire exactly once.
  if (args.advance) writeState(sfile, { lastSeenVersion: to, updatedAt: new Date().toISOString() });

  return { ok: true, from, to, firstRun: false, pending };
}

export { run as runPostUpdate, statePath, loadManifest };

// argv[1] may be a symlink (e.g. a skills-dir symlink); resolve it so it matches
// import.meta.url, which Node resolves through symlinks — else main() is skipped.
const entryPath = process.argv[1] && fs.realpathSync(process.argv[1]);
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  let out;
  try {
    out = run(parseArgs(process.argv.slice(2)));
  } catch (e) {
    out = { ok: true, from: null, to: null, firstRun: false, pending: [], error: 'unexpected', detail: e.message };
  }
  process.stdout.write(JSON.stringify(out) + '\n');
  process.exit(0);
}
