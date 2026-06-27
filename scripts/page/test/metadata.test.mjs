// Metadata tests: frontmatter round-trips losslessly (the agent writes, the
// renderer reads — both must agree), JSON-valued fields (chapters) survive, key
// order is stable for clean git diffs, and tricky string values are quoted.
// Run: node scripts/page/test/metadata.test.mjs

import assert from 'node:assert/strict';
import { parseMetadata, serializeMetadata } from '../lib/metadata.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('round-trips a full record losslessly', () => {
  const rec = {
    meta: {
      id: 'AbC123',
      title: 'Walkthrough: the new uploader',
      tldr: 'How file-by-file upload works.',
      durationSec: 312,
      createdAt: '2026-06-27T10:00:00.000Z',
      chapters: [{ t: 0, label: 'Intro' }, { t: 45, label: 'The PUT loop' }],
    },
    transcript: 'So the first thing we do is...\nThen the segment streams up.',
  };
  const out = parseMetadata(serializeMetadata(rec));
  assert.deepEqual(out.meta, rec.meta);
  assert.equal(out.transcript, rec.transcript);
});

test('serializes keys in stable order regardless of input order', () => {
  const a = serializeMetadata({ meta: { chapters: [], id: 'x', title: 'T', durationSec: 1 } });
  const b = serializeMetadata({ meta: { id: 'x', durationSec: 1, title: 'T', chapters: [] } });
  assert.equal(a, b);
  // id before title before durationSec before chapters
  assert.ok(a.indexOf('id:') < a.indexOf('title:'));
  assert.ok(a.indexOf('title:') < a.indexOf('durationSec:'));
  assert.ok(a.indexOf('durationSec:') < a.indexOf('chapters:'));
});

test('quotes strings that would otherwise be mis-parsed', () => {
  const rec = { meta: { id: 'x', title: 'Fix: the bug', tldr: '[draft] notes' }, transcript: '' };
  const out = parseMetadata(serializeMetadata(rec));
  assert.equal(out.meta.title, 'Fix: the bug'); // colon → must be quoted + survive
  assert.equal(out.meta.tldr, '[draft] notes'); // leading [ → must not parse as JSON
});

test('parses a hand-written doc and tolerates a missing body', () => {
  const doc = '---\nid: v9\ntitle: Quick demo\ndurationSec: 7\n---\n';
  const { meta, transcript } = parseMetadata(doc);
  assert.equal(meta.id, 'v9');
  assert.equal(meta.title, 'Quick demo');
  assert.equal(meta.durationSec, 7);
  assert.equal(transcript, '');
});

test('a doc with no frontmatter is treated as pure transcript', () => {
  const { meta, transcript } = parseMetadata('just some text\nno frontmatter');
  assert.deepEqual(meta, {});
  assert.equal(transcript, 'just some text\nno frontmatter');
});

test('unknown extra keys are preserved (after the known ones)', () => {
  const out = serializeMetadata({ meta: { id: 'x', title: 'T', source: 'screen', durationSec: 2 } });
  assert.ok(out.includes('source: screen'));
  assert.ok(out.indexOf('durationSec:') < out.indexOf('source:'));
  assert.equal(parseMetadata(out).meta.source, 'screen');
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
