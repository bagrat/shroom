// Record-preflight tests: the pending-publish recovery scan + its fire-once
// "surfaced" marker (the new logic in the one-shot preflight). The other aggregated
// checks — setup status, version, post-update, device catalogue — are covered by
// their own suites; this file exercises only what preflight.mjs adds.
// Run: node scripts/recorder/test/preflight.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { scanPublished, pendingPublish } from '../preflight.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// Build a throwaway recordings dir with one session per spec. Each spec:
//   { dir, lines: [event objects] } → written as events.ndjson.
function makeRecordings(specs) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-preflight-'));
  for (const s of specs) {
    const d = path.join(base, s.dir);
    fs.mkdirSync(d, { recursive: true });
    fs.writeFileSync(path.join(d, 'events.ndjson'), s.lines.map((l) => JSON.stringify(l)).join('\n') + '\n');
    // Nudge mtime so newest-first ordering is deterministic by dir index.
    const t = 1_700_000_000_000 + (s.order ?? 0) * 1000;
    fs.utimesSync(d, new Date(t), new Date(t));
  }
  return base;
}

const publishedLine = (id, url) => ({ event: 'published', id, playbackUrl: url, committed: true });

test('scanPublished: finds only sessions with a terminal playbackUrl', () => {
  const base = makeRecordings([
    { dir: '20260101-000001-aaa', order: 1, lines: [{ event: 'session_started', id: 'aaa' }, publishedLine('aaa', 'https://x/aaa')] },
    // uploader-style published (no playbackUrl) must NOT count as "live".
    { dir: '20260101-000002-bbb', order: 2, lines: [{ event: 'published', id: 'bbb', playlistKey: 'k', ok: true }] },
    // no published event at all → interrupted publish, not surfaced here.
    { dir: '20260101-000003-ccc', order: 3, lines: [{ event: 'session_started', id: 'ccc' }] },
  ]);
  const found = scanPublished(base);
  assert.deepEqual(found.map((f) => f.id), ['aaa']);
  assert.equal(found[0].playbackUrl, 'https://x/aaa');
  fs.rmSync(base, { recursive: true, force: true });
});

test('scanPublished: newest-first; id falls back to the dir suffix', () => {
  const base = makeRecordings([
    { dir: '20260101-000001-old', order: 1, lines: [publishedLine(undefined, 'https://x/old')] },
    { dir: '20260101-000002-new', order: 2, lines: [publishedLine(undefined, 'https://x/new')] },
  ]);
  const found = scanPublished(base);
  assert.deepEqual(found.map((f) => f.id), ['new', 'old']); // id parsed from "<ts>-<id>"
  fs.rmSync(base, { recursive: true, force: true });
});

test('scanPublished: missing dir → [] (fail-soft)', () => {
  assert.deepEqual(scanPublished('/no/such/dir/at/all'), []);
});

test('pendingPublish: first run baselines silently (never replays history)', () => {
  const base = makeRecordings([
    { dir: '20260101-000001-aaa', order: 1, lines: [publishedLine('aaa', 'https://x/aaa')] },
  ]);
  const stateFile = path.join(base, 'surfaced.json');
  const r = pendingPublish({ base, stateFile, advance: true });
  assert.equal(r.firstRun, true);
  assert.deepEqual(r.candidates, []);              // reported nothing…
  assert.ok(fs.existsSync(stateFile));             // …but recorded the baseline
  assert.deepEqual(JSON.parse(fs.readFileSync(stateFile, 'utf8')).surfaced, ['aaa']);
  fs.rmSync(base, { recursive: true, force: true });
});

test('pendingPublish: a new publish after baseline is reported exactly once', () => {
  const base = makeRecordings([
    { dir: '20260101-000001-aaa', order: 1, lines: [publishedLine('aaa', 'https://x/aaa')] },
  ]);
  const stateFile = path.join(base, 'surfaced.json');
  pendingPublish({ base, stateFile, advance: true });           // baseline {aaa}

  // A new recording lands and publishes.
  const d = path.join(base, '20260101-000002-bbb');
  fs.mkdirSync(d);
  fs.writeFileSync(path.join(d, 'events.ndjson'), JSON.stringify(publishedLine('bbb', 'https://x/bbb')) + '\n');
  fs.utimesSync(d, new Date(1_700_000_002_000), new Date(1_700_000_002_000));

  const first = pendingPublish({ base, stateFile, advance: true });
  assert.deepEqual(first.candidates.map((c) => c.id), ['bbb']); // surfaced once
  const second = pendingPublish({ base, stateFile, advance: true });
  assert.deepEqual(second.candidates, []);                      // never again
  fs.rmSync(base, { recursive: true, force: true });
});

test('pendingPublish: --no-advance reports without persisting (dry-run stays repeatable)', () => {
  const base = makeRecordings([
    { dir: '20260101-000001-aaa', order: 1, lines: [publishedLine('aaa', 'https://x/aaa')] },
  ]);
  const stateFile = path.join(base, 'surfaced.json');
  fs.writeFileSync(stateFile, JSON.stringify({ surfaced: [] }) + '\n'); // exists → not firstRun
  const a = pendingPublish({ base, stateFile, advance: false });
  const b = pendingPublish({ base, stateFile, advance: false });
  assert.deepEqual(a.candidates.map((c) => c.id), ['aaa']);
  assert.deepEqual(b.candidates.map((c) => c.id), ['aaa']); // unchanged — marker untouched
  fs.rmSync(base, { recursive: true, force: true });
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
