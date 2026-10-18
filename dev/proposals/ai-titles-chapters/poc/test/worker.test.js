import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { nameTake } from '../worker.js';
import { chat } from './fake-ai.js';

let ffmpeg = true;
try {
  execFileSync('ffmpeg', ['-version'], { stdio: 'ignore' });
} catch {
  ffmpeg = false;
}

/** R2's binding, as far as the route uses it. */
function memBucket(entries) {
  const m = new Map(entries);
  return {
    m,
    async get(k, { range } = {}) {
      let v = m.get(k);
      if (v === undefined) return null;
      if (typeof v === 'string') v = new TextEncoder().encode(v);
      if (range) v = v.slice(range.offset, range.offset + range.length);
      return { arrayBuffer: async () => v.slice().buffer };
    },
    async put(k, v) {
      m.set(k, v);
    },
    async list({ prefix, cursor }) {
      const all = [...m.keys()].filter((k) => k.startsWith(prefix)).sort();
      const from = cursor ? Number(cursor) : 0;
      const page = all.slice(from, from + 2);
      return { objects: page.map((key) => ({ key, size: m.get(key).length })), truncated: from + 2 < all.length, cursor: String(from + 2) };
    },
  };
}

test('the route reads the take from the bucket and writes title, transcript and chapters beside it', { skip: !ffmpeg && 'no ffmpeg' }, async () => {
  const dir = mkdtempSync(join(tmpdir(), 'shroom-ai-'));
  execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=320x180:rate=30', '-f', 'lavfi', '-i', 'sine=frequency=300:sample_rate=48000', '-t', '100',
    '-c:v', 'libx264', '-preset', 'ultrafast', '-pix_fmt', 'yuv420p', '-g', '60', '-c:a', 'aac', '-b:a', '128k',
    '-f', 'hls', '-hls_segment_type', 'fmp4', '-hls_time', '6', '-hls_playlist_type', 'vod', '-hls_fmp4_init_filename', 'init.mp4',
    '-hls_segment_filename', join(dir, 'seg_%05d.m4s'), join(dir, 'stream.m3u8')]);
  const bucket = memBucket(readdirSync(dir).filter((f) => f.endsWith('.mp4') || f.endsWith('.m4s')).map((f) => [`rec/t1/v0/${f}`, new Uint8Array(readFileSync(join(dir, f)))]));
  const seen = [];
  const ai = {
    async run(model, input) {
      seen.push(model);
      if (model.includes('whisper')) {
        assert.equal(atob(input.audio).charCodeAt(0), 0xff, 'ADTS, not the fMP4');
        return { segments: [{ words: [{ word: 'Hello.', start: 1, end: 1.5 }, { word: 'Next.', start: 40, end: 40.5 }] }], transcription_info: { language: 'en' } };
      }
      return chat({ title: 'A tone test.', chapters: [{ start: '0:00', title: 'Start' }, { start: '0:40', title: 'Next part' }] });
    },
  };
  const out = await nameTake({ BUCKET: bucket, AI: ai }, 't1');
  assert.equal(out.title, 'A tone test');
  assert.deepEqual(seen.filter((m) => m.includes('whisper')).length, 2, 'two windows for 100 s');
  assert.equal(JSON.parse(bucket.m.get('rec/t1/ai/title.json')).title, 'A tone test');
  assert.match(bucket.m.get('rec/t1/ai/chapters.vtt'), /^WEBVTT\n\n1\n00:00:00.000 --> 00:00:40.000\nStart\n/);
  assert.equal(await nameTake({ BUCKET: bucket, AI: ai }, 'nope'), null);
});
