// dashboard tests (deterministic parts): the library+local merge and the static
// HTML render (escaping). The filesystem/creds gathering is integration.
// Run: node scripts/dashboard/test/dashboard.test.mjs

import assert from 'node:assert/strict';
import { buildDashboardItems, renderDashboard } from '../dashboard.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

const LIBRARY = {
  vidA: { id: 'vidA', title: 'Alpha', durationSec: 120, createdAt: '2026-06-28T10:00:00.000Z', chapters: [{ t: 0, label: 'x' }], mp4: true },
};
const SESSIONS = [
  { id: 'vidA', dir: '/r/vidA', totalBytes: 1000, prunableBytes: 800, hasPreviewMp4: true, hasLocalHls: true, published: true, createdAt: '2026-06-28T10:00:00.000Z' },
  { id: 'vidB', dir: '/r/vidB', totalBytes: 50, prunableBytes: 0, hasPreviewMp4: true, hasLocalHls: false, published: false, createdAt: '2026-06-29T09:00:00.000Z', playbackUrl: null },
];

test('merges library records with local sessions, keyed by id', () => {
  const items = buildDashboardItems({ library: LIBRARY, sessions: SESSIONS, pagesBaseUrl: 'https://s.pages.dev' });
  assert.equal(items.length, 2);
  const a = items.find((i) => i.id === 'vidA');
  const b = items.find((i) => i.id === 'vidB');
  assert.equal(a.inLibrary, true);
  assert.equal(a.title, 'Alpha');
  assert.equal(a.chapters, 1);
  assert.equal(a.mp4, true);
  assert.equal(a.link, 'https://s.pages.dev/vidA/'); // in library + site configured → live
  assert.equal(a.live, true);
  assert.equal(a.local.totalBytes, 1000);
  // vidB has no library record → still listed, from local state.
  assert.equal(b.inLibrary, false);
  assert.equal(b.title, 'Untitled recording');
  assert.equal(b.local.published, false);
});

test('an unpublished local-only take gets no live link (would 404)', () => {
  const items = buildDashboardItems({ library: {}, sessions: SESSIONS, pagesBaseUrl: 'https://s.pages.dev' });
  const b = items.find((i) => i.id === 'vidB'); // not in library, published:false, no playbackUrl
  assert.equal(b.live, false);
  assert.equal(b.link, null);
  // but a locally-published one (published:true) is live.
  const a = items.find((i) => i.id === 'vidA');
  assert.equal(a.link, 'https://s.pages.dev/vidA/');
});

test('sorts newest first by createdAt', () => {
  const items = buildDashboardItems({ library: LIBRARY, sessions: SESSIONS, pagesBaseUrl: 'https://s.pages.dev' });
  assert.deepEqual(items.map((i) => i.id), ['vidB', 'vidA']); // 06-29 before 06-28
});

test('without pagesBaseUrl, falls back to a session playbackUrl', () => {
  const items = buildDashboardItems({
    library: {},
    sessions: [{ id: 'v', dir: '/r/v', totalBytes: 1, prunableBytes: 0, createdAt: '2026-01-01T00:00:00Z', playbackUrl: 'https://x/v/' }],
    pagesBaseUrl: '',
  });
  assert.equal(items[0].link, 'https://x/v/');
});

test('renderDashboard escapes titles and lists a count', () => {
  const html = renderDashboard([
    { id: 'v', title: '<script>alert(1)</script>', durationSec: 65, createdAt: '2026-06-28T00:00:00Z', chapters: 0, mp4: false, link: null, local: null, thumb: null },
  ]);
  assert.ok(!html.includes('<script>alert(1)</script>'));
  assert.ok(html.includes('&lt;script&gt;'));
  assert.ok(html.includes('1 recording'));
  assert.ok(html.includes('>1:05<')); // duration chip
});

test('renderDashboard on an empty library shows the empty state', () => {
  const html = renderDashboard([]);
  assert.ok(html.includes('No recordings yet'));
  assert.ok(html.includes('0 recordings'));
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
