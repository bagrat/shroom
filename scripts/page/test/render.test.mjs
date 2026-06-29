// Renderer tests: token substitution, escaping (the security-critical bit — agent/
// user text must never break out of attributes, markup, or the JSON island),
// duration formatting, chapters, and that the real template produces valid output.
// Run: node scripts/page/test/render.test.mjs

import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { renderPage, formatDuration } from '../lib/render.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const TEMPLATE = fs.readFileSync(path.resolve(HERE, '../../../templates/player.html'), 'utf8');

const URLS = {
  hlsUrl: 'https://pub-x.r2.dev/vid1/stream.m3u8',
  pageUrl: 'https://shroom.pages.dev/vid1/',
  posterUrl: 'https://shroom.pages.dev/vid1/poster.jpg',
  hlsJsUrl: '/hls.min.js',
};

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('formatDuration: m:ss and h:mm:ss', () => {
  assert.equal(formatDuration(0), '0:00');
  assert.equal(formatDuration(5), '0:05');
  assert.equal(formatDuration(83), '1:23');
  assert.equal(formatDuration(3661), '1:01:01');
  assert.equal(formatDuration(undefined), '0:00');
});

test('substitutes title, duration, urls into the real template', () => {
  const html = renderPage({
    template: TEMPLATE,
    meta: { title: 'Demo of the thing', durationSec: 83, tldr: 'A short walkthrough.' },
    urls: URLS,
  });
  assert.ok(html.includes('<title>Demo of the thing</title>'));
  assert.ok(html.includes('content="Demo of the thing"')); // og:title
  assert.ok(html.includes('content="A short walkthrough."')); // og:description
  assert.ok(html.includes('content="https://shroom.pages.dev/vid1/"')); // og:url
  assert.ok(html.includes('poster="https://shroom.pages.dev/vid1/poster.jpg"'));
  assert.ok(html.includes('>1:23</span>')); // duration label
  assert.ok(html.includes('content="83"')); // og:video:duration
  assert.ok(!html.includes('{{'), 'no unsubstituted tokens remain');
});

test('HLS url + hls.js path land in the JSON data island, not raw markup', () => {
  const html = renderPage({ template: TEMPLATE, meta: { title: 'x', durationSec: 10 }, urls: URLS });
  const m = html.match(/<script id="shroom-data" type="application\/json">(.*?)<\/script>/s);
  assert.ok(m, 'data island present');
  const data = JSON.parse(m[1]);
  assert.equal(data.hlsUrl, URLS.hlsUrl);
  assert.equal(data.hlsJsUrl, '/hls.min.js');
  assert.equal(data.durationSec, 10);
});

test('data island carries pageUrl + chapters for the timeline/share', () => {
  const html = renderPage({
    template: TEMPLATE,
    meta: { title: 'x', durationSec: 120, chapters: [{ t: 0, label: 'Intro' }, { t: 60, label: 'Mid' }] },
    urls: URLS,
  });
  const data = JSON.parse(html.match(/type="application\/json">(.*?)<\/script>/s)[1]);
  assert.equal(data.pageUrl, URLS.pageUrl);
  assert.deepEqual(data.chapters, [{ t: 0, label: 'Intro' }, { t: 60, label: 'Mid' }]);
  // The static scaffolding the client wires up is present.
  assert.ok(html.includes('id="shroom-timeline"'));
  assert.ok(html.includes('data-copy="embed"'));
});

test('escapes HTML in title — no attribute/markup breakout', () => {
  const html = renderPage({
    template: TEMPLATE,
    meta: { title: '"><script>alert(1)</script>', durationSec: 1 },
    urls: URLS,
  });
  assert.ok(!html.includes('<script>alert(1)</script>'), 'raw script must not appear');
  assert.ok(html.includes('&lt;script&gt;alert(1)&lt;/script&gt;'));
  // The title also feeds og:title as an attribute — the quote must be escaped.
  assert.ok(html.includes('content="&quot;&gt;'));
});

test('escapes a </script> in the JSON island so it cannot close the block early', () => {
  const html = renderPage({
    template: TEMPLATE,
    meta: { title: 'ok', durationSec: 1 },
    urls: { ...URLS, hlsUrl: 'https://x/</script><script>evil()</script>/stream.m3u8' },
  });
  const island = html.match(/type="application\/json">(.*?)<\/script>/s)[1];
  assert.ok(!island.includes('</script>'), 'no literal </script> inside the island');
  assert.ok(island.includes('\\u003c'));
  // And it still parses back to the original string.
  assert.equal(JSON.parse(island).hlsUrl, 'https://x/</script><script>evil()</script>/stream.m3u8');
});

test('renders chapters as seekable buttons; omits the list when empty', () => {
  const withCh = renderPage({
    template: TEMPLATE,
    meta: { title: 't', durationSec: 120, chapters: [{ t: 0, label: 'Intro' }, { t: 65, label: 'The fix' }] },
    urls: URLS,
  });
  assert.ok(withCh.includes('data-seek="0"'));
  assert.ok(withCh.includes('data-seek="65"'));
  assert.ok(withCh.includes('>1:05</span>'));
  assert.ok(withCh.includes('Intro') && withCh.includes('The fix'));

  const noCh = renderPage({ template: TEMPLATE, meta: { title: 't', durationSec: 1 }, urls: URLS });
  assert.ok(!noCh.includes('class="chapters"'));
});

test('Download button renders only when mp4 flag + download URL are present', () => {
  const urls = { ...URLS, downloadUrl: 'https://pub-x.r2.dev/vid1/video.mp4' };
  const on = renderPage({ template: TEMPLATE, meta: { title: 't', durationSec: 5, mp4: true }, urls });
  assert.ok(on.includes('class="download"'));
  assert.ok(on.includes('href="https://pub-x.r2.dev/vid1/video.mp4"'));
  assert.ok(on.includes('download>'));

  const offNoFlag = renderPage({ template: TEMPLATE, meta: { title: 't', durationSec: 5 }, urls });
  assert.ok(!offNoFlag.includes('class="download"'));
  const offNoUrl = renderPage({ template: TEMPLATE, meta: { title: 't', durationSec: 5, mp4: true }, urls: URLS });
  assert.ok(!offNoUrl.includes('class="download"'));
  assert.ok(!offNoFlag.includes('{{')); // token still substitutes to empty
});

test('missing title falls back to "Untitled recording"', () => {
  const html = renderPage({ template: TEMPLATE, meta: { durationSec: 1 }, urls: URLS });
  assert.ok(html.includes('<title>Untitled recording</title>'));
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
