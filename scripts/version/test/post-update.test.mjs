// post-update runner tests: first-run baselines without replaying history; a
// bump reports exactly the migrations in (lastSeen, installed]; the marker
// advances so they fire once; and everything is fail-soft.
// Run: node scripts/version/test/post-update.test.mjs

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.resolve(HERE, '..', 'post-update.mjs');
const INSTALLED = JSON.parse(
  fs.readFileSync(path.resolve(HERE, '..', '..', '..', '.claude-plugin', 'plugin.json'), 'utf8'),
).version; // 0.1.12 at time of writing

let tmp;
const setup = () => (tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-pu-')));
const cleanup = () => fs.rmSync(tmp, { recursive: true, force: true });

// A fixture manifest with entries straddling the installed version.
function writeManifest(versions) {
  const file = path.join(tmp, 'm.json');
  fs.writeFileSync(file, JSON.stringify({ migrations: versions.map((v) => ({ version: v, whatsNew: `notes ${v}`, actions: [] })) }));
  return file;
}
// Offset the installed version's patch, so fixtures track the real version and don't
// go stale on every bump (post-update reads installed from the real plugin.json).
const patchOffset = (delta) => {
  const p = INSTALLED.split('.').map(Number);
  p[2] += delta;
  return p.join('.');
};
const run = (args) => JSON.parse(execFileSync(process.execPath, [SCRIPT, ...args], { encoding: 'utf8' }));

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('first run baselines to installed and reports nothing', () => {
  const state = path.join(tmp, 's.json');
  const res = run(['--state', state, '--manifest', writeManifest(['0.0.1'])]);
  assert.equal(res.firstRun, true);
  assert.deepEqual(res.pending, []);
  assert.equal(JSON.parse(fs.readFileSync(state, 'utf8')).lastSeenVersion, INSTALLED);
});

test('a bump reports migrations in (from, installed], ascending', () => {
  const state = path.join(tmp, 's2.json');
  const from = patchOffset(-3);      // == lastSeen, excluded (not > from)
  const lo = patchOffset(-2);        // in range
  const mid = patchOffset(-1);       // in range
  const above = patchOffset(+1);     // > installed, excluded
  fs.writeFileSync(state, JSON.stringify({ lastSeenVersion: from }));
  const manifest = writeManifest([INSTALLED, lo, above, mid, from]);
  const res = run(['--state', state, '--manifest', manifest]);
  // `from` excluded (== from, not > from); `above` excluded (> installed)
  assert.deepEqual(res.pending.map((m) => m.version), [lo, mid, INSTALLED]);
  assert.equal(res.pending[0].whatsNew, `notes ${lo}`);
});

test('the marker advances, so a re-run reports nothing', () => {
  const state = path.join(tmp, 's3.json');
  fs.writeFileSync(state, JSON.stringify({ lastSeenVersion: '0.1.10' }));
  const manifest = writeManifest(['0.1.11', '0.1.12']);
  run(['--state', state, '--manifest', manifest]); // advances to installed
  const second = run(['--state', state, '--manifest', manifest]);
  assert.deepEqual(second.pending, []);
});

test('--no-advance leaves the marker untouched (dry-run)', () => {
  const state = path.join(tmp, 's4.json');
  fs.writeFileSync(state, JSON.stringify({ lastSeenVersion: '0.1.10' }));
  const manifest = writeManifest(['0.1.11']);
  const res = run(['--state', state, '--manifest', manifest, '--no-advance']);
  assert.deepEqual(res.pending.map((m) => m.version), ['0.1.11']);
  assert.equal(JSON.parse(fs.readFileSync(state, 'utf8')).lastSeenVersion, '0.1.10');
});

test('same version is a no-op', () => {
  const state = path.join(tmp, 's5.json');
  fs.writeFileSync(state, JSON.stringify({ lastSeenVersion: INSTALLED }));
  const res = run(['--state', state, '--manifest', writeManifest(['0.1.11'])]);
  assert.deepEqual(res.pending, []);
});

test('a garbage manifest fails soft', () => {
  const state = path.join(tmp, 's6.json');
  fs.writeFileSync(state, JSON.stringify({ lastSeenVersion: '0.1.10' }));
  const bad = path.join(tmp, 'bad.json');
  fs.writeFileSync(bad, 'not json{');
  const res = run(['--state', state, '--manifest', bad]);
  assert.equal(res.ok, true);
  assert.deepEqual(res.pending, []);
});

(async () => {
  setup();
  try {
    for (const [name, fn] of tests) {
      try { await fn(); passed++; console.log(`ok   ${name}`); }
      catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
    }
  } finally {
    cleanup();
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
