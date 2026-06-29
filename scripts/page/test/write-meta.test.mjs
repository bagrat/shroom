// write-meta tests: the deterministic <id>.md writer. Drives the real CLI in a temp
// HOME/library/session so we exercise library resolution, transcript+events
// ingestion, chapter normalization, and the build-page round-trip (write → --meta
// read). Run: node scripts/page/test/write-meta.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

import { parseMetadata } from '../lib/metadata.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CLI = path.resolve(HERE, '../write-meta.mjs');

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// Run write-meta with an isolated HOME (so creds/library never touch the real one).
function run(args, { home, cwd } = {}) {
  const env = { ...process.env };
  if (home) env.HOME = home;
  const out = execFileSync('node', [CLI, ...args], { env, cwd, encoding: 'utf8' });
  return JSON.parse(out.trim());
}

function tmp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

// A session dir with a normalized transcript + a minimal events.ndjson.
function makeSession({ transcript = 'Hello there.\nThis is a demo.', durationSec = 42, createdAt = '2026-06-27T10:00:00.000Z', language = 'en' } = {}) {
  const dir = tmp('shroom-sess-');
  fs.writeFileSync(path.join(dir, 'transcript.json'), JSON.stringify({ language, transcript, durationSec, segments: [] }));
  const events = [
    { ts: createdAt, event: 'session_started', id: 'ignored', dir },
    { ts: '2026-06-27T10:00:42.000Z', event: 'finalized', durationSec, ok: true },
  ].map((e) => JSON.stringify(e)).join('\n') + '\n';
  fs.writeFileSync(path.join(dir, 'events.ndjson'), events);
  return dir;
}

test('writes <id>.md into an explicit --library, round-tripping through parse', () => {
  const lib = tmp('shroom-lib-');
  const session = makeSession();
  const res = run(['--id', 'AbC123', '--session', session, '--library', lib,
    '--title', 'Fix: the upload bug', '--tldr', 'How file-by-file works.',
    '--chapters', JSON.stringify([{ t: 0, label: 'Intro' }, { t: 12, label: 'The PUT loop' }])]);

  assert.equal(res.event, 'metadata_written');
  assert.equal(res.metaPath, path.join(lib, 'AbC123.md'));
  const { meta, transcript } = parseMetadata(fs.readFileSync(res.metaPath, 'utf8'));
  assert.equal(meta.id, 'AbC123');
  assert.equal(meta.title, 'Fix: the upload bug'); // colon survives quoting
  assert.equal(meta.tldr, 'How file-by-file works.');
  assert.equal(meta.durationSec, 42); // from events.finalized
  assert.equal(meta.createdAt, '2026-06-27T10:00:00.000Z'); // from session_started
  assert.deepEqual(meta.chapters, [{ t: 0, label: 'Intro' }, { t: 12, label: 'The PUT loop' }]);
  assert.match(transcript, /This is a demo\./);
});

test('resolves the library from creds when --library is omitted', () => {
  const home = tmp('shroom-home-');
  const lib = path.join(home, 'my-lib');
  fs.mkdirSync(path.join(home, '.shroom'), { recursive: true });
  fs.writeFileSync(path.join(home, '.shroom', 'credentials.json'), JSON.stringify({ library: lib }));
  const session = makeSession();

  const res = run(['--id', 'x1', '--session', session, '--title', 'Demo'], { home });
  assert.equal(res.metaPath, path.join(lib, 'x1.md'));
  assert.ok(fs.existsSync(res.metaPath));
});

test('falls back to ~/shroom when neither flag nor creds give a library', () => {
  const home = tmp('shroom-home-');
  const res = run(['--id', 'y2', '--title', 'No session'], { home });
  assert.equal(res.metaPath, path.join(home, 'shroom', 'y2.md'));
  assert.equal(res.hasTranscript, false);
});

test('normalizes chapters: time/title aliases, sort, drop empties, round', () => {
  const lib = tmp('shroom-lib-');
  const res = run(['--id', 'z3', '--library', lib, '--title', 'T',
    '--chapters', JSON.stringify([
      { time: 30.6, title: 'Later' },
      { start: 0, label: 'First' },
      { t: 10, label: '   ' }, // empty label → dropped
    ])]);
  assert.equal(res.chapters, 2);
  const { meta } = parseMetadata(fs.readFileSync(res.metaPath, 'utf8'));
  assert.deepEqual(meta.chapters, [{ t: 0, label: 'First' }, { t: 31, label: 'Later' }]);
});

test('flags override session-derived duration/createdAt', () => {
  const lib = tmp('shroom-lib-');
  const session = makeSession({ durationSec: 42 });
  const res = run(['--id', 'o4', '--session', session, '--library', lib, '--title', 'T',
    '--duration-sec', '99', '--created-at', '2020-01-01T00:00:00.000Z']);
  const { meta } = parseMetadata(fs.readFileSync(res.metaPath, 'utf8'));
  assert.equal(meta.durationSec, 99);
  assert.equal(meta.createdAt, '2020-01-01T00:00:00.000Z');
});

test('requires an id, and a title unless an existing record supplies one', () => {
  const lib = tmp('shroom-lib-');
  assert.throws(() => run(['--library', lib, '--title', 'T'])); // no id
  assert.throws(() => run(['--id', 'a', '--library', lib])); // no title, no existing file
});

test('enrichment: omitting --title inherits the existing title, adds chapters+transcript', () => {
  const lib = tmp('shroom-lib-');
  // First write: the user's instant title, no transcript yet (manual-name path).
  run(['--id', 'e5', '--library', lib, '--title', 'My own title']);
  let parsed = parseMetadata(fs.readFileSync(path.join(lib, 'e5.md'), 'utf8'));
  assert.equal(parsed.meta.title, 'My own title');
  assert.equal(parsed.transcript, '');

  // Background enrichment: transcript now exists; add chapters, keep the title.
  const session = makeSession({ transcript: 'Body text here.' });
  const res = run(['--id', 'e5', '--library', lib, '--session', session,
    '--chapters', JSON.stringify([{ t: 0, label: 'Start' }])]);
  assert.equal(res.chapters, 1);
  parsed = parseMetadata(fs.readFileSync(res.metaPath, 'utf8'));
  assert.equal(parsed.meta.title, 'My own title'); // preserved, not clobbered
  assert.deepEqual(parsed.meta.chapters, [{ t: 0, label: 'Start' }]);
  assert.match(parsed.transcript, /Body text here\./);
});

test('--mp4 sets the download flag, and it survives an enrich re-publish', () => {
  const lib = tmp('shroom-lib-');
  // cleanup skill marks the record after uploading video.mp4.
  run(['--id', 'm6', '--library', lib, '--title', 'Has a download', '--mp4']);
  let parsed = parseMetadata(fs.readFileSync(path.join(lib, 'm6.md'), 'utf8'));
  assert.equal(parsed.meta.mp4, true);

  // A later enrich pass rebuilds meta from flags WITHOUT --mp4 — must not drop it.
  const session = makeSession({ transcript: 'Body.' });
  run(['--id', 'm6', '--library', lib, '--session', session,
    '--chapters', JSON.stringify([{ t: 0, label: 'Start' }])]);
  parsed = parseMetadata(fs.readFileSync(path.join(lib, 'm6.md'), 'utf8'));
  assert.equal(parsed.meta.mp4, true); // inherited, not clobbered
  assert.equal(parsed.meta.title, 'Has a download');
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
