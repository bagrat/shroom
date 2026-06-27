// Behaviour tests for the uploader against an in-process mock S3: live enqueue,
// idempotency, retry/backoff, the publish-last gate, and gap-aware resume.
// Run: node scripts/uploader/test/uploader.test.mjs

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import { Uploader, PLAYLIST, INIT } from '../lib/uploader.mjs';
import { startMockS3 } from './mock-s3.mjs';

const ID = 'TestId123';
const CREDS = { region: 'auto', bucket: 'mybucket', accessKeyId: 'AK', secretAccessKey: 'SK' };

function makeSessionDir({ segments = 2, withPlaylist = true } = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-up-'));
  fs.writeFileSync(path.join(dir, INIT), Buffer.from('INIT'));
  const segs = [];
  for (let i = 0; i < segments; i++) {
    const f = `seg_${String(i).padStart(5, '0')}.m4s`;
    fs.writeFileSync(path.join(dir, f), Buffer.from(`SEG-${i}-`.repeat(8)));
    segs.push(f);
  }
  // local-only files that must NOT be uploaded
  fs.writeFileSync(path.join(dir, 'preview.mp4'), Buffer.from('PREVIEW'));
  fs.writeFileSync(path.join(dir, 'preview_0.mp4'), Buffer.from('PREVIEW0'));
  fs.writeFileSync(path.join(dir, 'events.ndjson'), Buffer.from('{}'));
  if (withPlaylist) fs.writeFileSync(path.join(dir, PLAYLIST), Buffer.from('#EXTM3U\n#EXT-X-ENDLIST\n'));
  return { dir, segs };
}

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('live enqueue + finalizePublish uploads init+segments+playlist', async () => {
  const mock = await startMockS3();
  const { dir, segs } = makeSessionDir({ segments: 3 });
  const up = new Uploader({ ...CREDS, endpoint: mock.endpoint }, { id: ID, dir });
  up.enqueue(INIT);
  for (const s of segs) up.enqueue(s);
  const r = await up.finalizePublish({ segments: segs });

  assert.equal(r.published, true);
  for (const f of [INIT, ...segs, PLAYLIST]) assert.ok(mock.store.has(`${ID}/${f}`), `missing ${f}`);
  // local-only files never uploaded
  assert.ok(!mock.store.has(`${ID}/preview.mp4`));
  assert.ok(!mock.store.has(`${ID}/preview_0.mp4`));
  assert.ok(!mock.store.has(`${ID}/events.ndjson`));
  await mock.close();
});

test('idempotent: enqueuing a segment twice uploads it once', async () => {
  const mock = await startMockS3();
  const { dir, segs } = makeSessionDir({ segments: 1 });
  const up = new Uploader({ ...CREDS, endpoint: mock.endpoint }, { id: ID, dir });
  up.enqueue(segs[0]);
  up.enqueue(segs[0]);
  await up.settle();
  const puts = mock.puts.filter((k) => k === `${ID}/${segs[0]}`);
  assert.equal(puts.length, 1);
  await mock.close();
});

test('retry/backoff: recovers after injected 503s', async () => {
  const { dir, segs } = makeSessionDir({ segments: 1 });
  const key = `${ID}/${segs[0]}`;
  const mock = await startMockS3({ failuresByKey: { [key]: 2 } });
  const up = new Uploader({ ...CREDS, endpoint: mock.endpoint }, {
    id: ID, dir, baseDelayMs: 5, maxDelayMs: 20,
  });
  up.enqueue(segs[0]);
  await up.settle();
  assert.ok(up.confirmed.has(segs[0]), 'segment should be confirmed after retries');
  assert.ok(mock.store.has(key));
  await mock.close();
});

test('publish gate: a permanently-failing segment blocks the playlist', async () => {
  const { dir, segs } = makeSessionDir({ segments: 2 });
  const badKey = `${ID}/${segs[1]}`;
  const mock = await startMockS3({ failuresByKey: { [badKey]: 999 } });
  const up = new Uploader({ ...CREDS, endpoint: mock.endpoint }, {
    id: ID, dir, maxAttempts: 2, baseDelayMs: 2, maxDelayMs: 4,
  });
  const r = await up.finalizePublish({ segments: segs });
  assert.equal(r.published, false);
  assert.ok(!mock.store.has(`${ID}/${PLAYLIST}`), 'playlist must NOT be published');
  assert.deepEqual(r.failed, [segs[1]]);
  await mock.close();
});

test('syncDir resume: only the gap is uploaded, then publish', async () => {
  const mock = await startMockS3();
  const { dir, segs } = makeSessionDir({ segments: 3 });
  // Pre-seed the bucket as if a prior run uploaded init + seg 0.
  const seed = new Uploader({ ...CREDS, endpoint: mock.endpoint }, { id: ID, dir });
  await seed.ensureUploaded([INIT, segs[0]]);
  mock.puts.length = 0; // reset the PUT log

  const up = new Uploader({ ...CREDS, endpoint: mock.endpoint }, { id: ID, dir });
  const r = await up.syncDir();

  assert.equal(r.published, true);
  // Only the gap (seg 1, seg 2) + the playlist should have been PUT this run.
  assert.deepEqual(mock.puts.sort(), [`${ID}/${segs[1]}`, `${ID}/${segs[2]}`, `${ID}/${PLAYLIST}`].sort());
  for (const f of [INIT, ...segs, PLAYLIST]) assert.ok(mock.store.has(`${ID}/${f}`));
  await mock.close();
});

(async () => {
  for (const [name, fn] of tests) {
    try {
      await fn();
      passed++;
      console.log(`ok   ${name}`);
    } catch (e) {
      console.error(`FAIL ${name}\n     ${e.message}`);
    }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
