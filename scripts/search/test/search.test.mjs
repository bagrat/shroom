// search tests (deterministic retrieval): tokenization, weighted scoring + ranking,
// snippet extraction, chapter matching, and the empty-query guard.
// Run: node scripts/search/test/search.test.mjs

import assert from 'node:assert/strict';
import { tokenize, searchCorpus } from '../search.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

const CORPUS = [
  { id: 'a', title: 'Uploader walkthrough', tldr: 'How file-by-file upload works.',
    chapters: [{ t: 0, label: 'Intro' }, { t: 48, label: 'The PUT loop' }],
    transcript: 'So the uploader streams each segment up with a PUT. The PUT is idempotent.', createdAt: '2026-06-28T00:00:00Z' },
  { id: 'b', title: 'Permissions on macOS', tldr: 'TCC and screen recording consent.',
    chapters: [{ t: 0, label: 'TCC basics' }],
    transcript: 'The shim becomes its own responsible process so the prompt reads shroom.', createdAt: '2026-06-27T00:00:00Z' },
  { id: 'c', title: 'Random demo', tldr: '', chapters: [],
    transcript: 'Just a quick hello, nothing about uploads here.', createdAt: '2026-06-29T00:00:00Z' },
];

test('tokenize lowercases and splits on non-alphanumerics', () => {
  assert.deepEqual(tokenize('The PUT-loop, idempotent!'), ['the', 'put', 'loop', 'idempotent']);
});

test('ranks the title/chapter match above an incidental body mention', () => {
  const r = searchCorpus(CORPUS, 'uploader PUT', { limit: 5 });
  assert.equal(r[0].id, 'a'); // title + chapter + body hits
  assert.ok(r[0].matchedTerms.includes('put'));
});

test('surfaces chapters whose label matches a term', () => {
  const r = searchCorpus(CORPUS, 'PUT loop');
  const a = r.find((x) => x.id === 'a');
  assert.ok(a.chapters.some((c) => c.label === 'The PUT loop' && c.time === '0:48'));
});

test('a stopword-only query returns nothing', () => {
  assert.deepEqual(searchCorpus(CORPUS, 'the and of'), []);
});

test('non-matching query returns no results', () => {
  assert.deepEqual(searchCorpus(CORPUS, 'kubernetes'), []);
});

test('snippet windows around the hit', () => {
  const r = searchCorpus(CORPUS, 'responsible');
  assert.equal(r[0].id, 'b');
  assert.match(r[0].snippet, /responsible process/);
});

test('phrase bonus lifts an exact-phrase record', () => {
  const r = searchCorpus(CORPUS, 'file-by-file upload');
  assert.equal(r[0].id, 'a'); // exact phrase in tldr
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
