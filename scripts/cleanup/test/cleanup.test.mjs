// cleanup tests (deterministic parts only — the network ops are integration):
// the ListObjectsV2 XML parse, the prunable-file classification, and the local
// scan over a synthetic recordings tree.
// Run: node scripts/cleanup/test/cleanup.test.mjs

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseListObjectsV2 } from '../../uploader/lib/s3.mjs';
import { isPrunable, scanSessions, pruneDir } from '../cleanup.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('parseListObjectsV2 pulls keys + sizes and the continuation token', () => {
  const xml = `<?xml version="1.0"?><ListBucketResult>
    <IsTruncated>true</IsTruncated>
    <Contents><Key>abc/init.mp4</Key><Size>1024</Size></Contents>
    <Contents><Key>abc/seg_00001.m4s</Key><Size>2048</Size></Contents>
    <NextContinuationToken>TOK123</NextContinuationToken>
  </ListBucketResult>`;
  const r = parseListObjectsV2(xml);
  assert.deepEqual(r.objects, [
    { key: 'abc/init.mp4', size: 1024 },
    { key: 'abc/seg_00001.m4s', size: 2048 },
  ]);
  assert.equal(r.truncated, true);
  assert.equal(r.nextToken, 'TOK123');
});

test('parseListObjectsV2 on an empty/last page', () => {
  const r = parseListObjectsV2('<ListBucketResult><IsTruncated>false</IsTruncated></ListBucketResult>');
  assert.deepEqual(r.objects, []);
  assert.equal(r.truncated, false);
  assert.equal(r.nextToken, null);
});

test('isPrunable keeps preview.mp4/events but drops HLS + intermediates', () => {
  assert.equal(isPrunable('init.mp4'), true);
  assert.equal(isPrunable('seg_00042.m4s'), true);
  assert.equal(isPrunable('stream.m3u8'), true);
  assert.equal(isPrunable('stream_2.m3u8'), true);
  assert.equal(isPrunable('preview_1.mp4'), true);
  assert.equal(isPrunable('ffmpeg.log'), true);
  // keepers
  assert.equal(isPrunable('preview.mp4'), false);
  assert.equal(isPrunable('events.ndjson'), false);
  assert.equal(isPrunable('index.html'), false);
  assert.equal(isPrunable('poster.jpg'), false);
});

test('scanSessions reads id/state/sizes from a synthetic tree', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-rec-'));
  try {
    const id = 'AbC-1_dEf2'; // base64url id with - and _
    const dir = path.join(root, `20260629-101500-${id}`);
    fs.mkdirSync(dir);
    fs.writeFileSync(path.join(dir, 'events.ndjson'),
      JSON.stringify({ event: 'session_started', id, ts: '2026-06-29T10:15:00.000Z' }) + '\n' +
      JSON.stringify({ event: 'published', playbackUrl: 'https://x.pages.dev/' + id + '/' }) + '\n');
    fs.writeFileSync(path.join(dir, 'init.mp4'), Buffer.alloc(1000));
    fs.writeFileSync(path.join(dir, 'seg_00001.m4s'), Buffer.alloc(2000));
    fs.writeFileSync(path.join(dir, 'stream.m3u8'), Buffer.alloc(50));
    fs.writeFileSync(path.join(dir, 'preview.mp4'), Buffer.alloc(500));

    const [s] = scanSessions(root);
    assert.equal(s.id, id); // from events, not a naive '-' split
    assert.equal(s.published, true);
    assert.equal(s.hasPreviewMp4, true);
    assert.equal(s.hasLocalHls, true);
    assert.equal(s.prunableBytes, 1000 + 2000 + 50); // not preview.mp4
    assert.equal(s.totalBytes, 1000 + 2000 + 50 + 500 + fs.statSync(path.join(dir, 'events.ndjson')).size);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('pruneDir removes HLS + intermediates, keeps preview.mp4 + events', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-prune-'));
  try {
    fs.writeFileSync(path.join(dir, 'init.mp4'), Buffer.alloc(1000));
    fs.writeFileSync(path.join(dir, 'seg_00001.m4s'), Buffer.alloc(2000));
    fs.writeFileSync(path.join(dir, 'stream.m3u8'), Buffer.alloc(50));
    fs.writeFileSync(path.join(dir, 'preview_1.mp4'), Buffer.alloc(300)); // per-take intermediate
    fs.writeFileSync(path.join(dir, 'preview.mp4'), Buffer.alloc(500));   // the keeper
    fs.writeFileSync(path.join(dir, 'events.ndjson'), Buffer.alloc(80));  // the keeper

    const r = pruneDir(dir);
    assert.equal(r.freedBytes, 1000 + 2000 + 50 + 300);
    assert.deepEqual(r.removed.sort(), ['init.mp4', 'preview_1.mp4', 'seg_00001.m4s', 'stream.m3u8']);
    assert.equal(r.keptMp4, true);
    assert.equal(fs.existsSync(path.join(dir, 'preview.mp4')), true);
    assert.equal(fs.existsSync(path.join(dir, 'events.ndjson')), true);
    assert.equal(fs.existsSync(path.join(dir, 'init.mp4')), false);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('pruneDir reports keptMp4:false when there is no preview.mp4', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-prune-'));
  try {
    fs.writeFileSync(path.join(dir, 'seg_00001.m4s'), Buffer.alloc(2000));
    const r = pruneDir(dir);
    assert.equal(r.keptMp4, false);
    assert.deepEqual(r.removed, ['seg_00001.m4s']);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('scanSessions on a missing root returns []', () => {
  assert.deepEqual(scanSessions(path.join(os.tmpdir(), 'shroom-does-not-exist-xyz')), []);
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
