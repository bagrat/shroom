import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { audioTrack, pieceAudio, pieceAudioRanged, joinAudio } from '../fmp4.js';

let ffmpeg = true;
try {
  execFileSync('ffmpeg', ['-version'], { stdio: 'ignore' });
} catch {
  ffmpeg = false;
}

test('a take shaped like shroom\'s: the AAC track, each piece\'s audio on the take\'s clock', { skip: !ffmpeg && 'no ffmpeg' }, async () => {
  const dir = mkdtempSync(join(tmpdir(), 'shroom-ai-'));
  execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=320x180:rate=30', '-f', 'lavfi', '-i', 'sine=frequency=440:sample_rate=48000', '-t', '20',
    '-c:v', 'libx264', '-preset', 'ultrafast', '-pix_fmt', 'yuv420p', '-g', '60', '-c:a', 'aac', '-b:a', '128k', '-ac', '2',
    '-f', 'hls', '-hls_segment_type', 'fmp4', '-hls_time', '6', '-hls_playlist_type', 'vod', '-hls_fmp4_init_filename', 'init.mp4',
    '-hls_segment_filename', join(dir, 'seg_%05d.m4s'), join(dir, 'stream.m3u8')]);
  const track = audioTrack(readFileSync(join(dir, 'init.mp4')));
  assert.equal(track.objectType, 2, 'AAC-LC');
  assert.equal(track.sampleRate, 48000);
  assert.equal(track.channels, 2);
  const parts = readdirSync(dir).filter((f) => f.endsWith('.m4s')).sort().map((f) => pieceAudio(readFileSync(join(dir, f)), track));
  assert.ok(parts.length >= 3);
  for (let i = 1; i < parts.length; i++) assert.ok(Math.abs(parts[i].start - (parts[i - 1].start + parts[i - 1].duration)) < 0.03, 'pieces are back to back');
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.m4s'))) {
    const piece = readFileSync(join(dir, f));
    let read = 0;
    const ranged = await pieceAudioRanged(async (o, n) => ((read += Math.min(n, piece.length - o)), new Uint8Array(piece.subarray(o, o + n))), piece.length, track);
    const whole = pieceAudio(piece, track);
    assert.deepEqual(ranged.adts, whole.adts, `${f}: ranged reads give the same audio`);
    assert.equal(ranged.start, whole.start);
    assert.ok(read < piece.length / 2, `${f}: read ${read} of ${piece.length} bytes`);
  }
  const all = joinAudio(parts);
  assert.ok(Math.abs(all.end - 20) < 0.1, `ends at ${all.end}`);
  for (let o = 0; o < all.adts.length; ) {
    assert.equal(all.adts[o], 0xff);
    assert.equal(all.adts[o + 1] & 0xf0, 0xf0);
    o += ((all.adts[o + 3] & 3) << 11) | (all.adts[o + 4] << 3) | (all.adts[o + 5] >> 5);
  }
  writeFileSync(join(dir, 'all.aac'), all.adts);
  const secs = Number(execFileSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', join(dir, 'all.aac')]).toString());
  assert.ok(Math.abs(secs - 20) < 0.5, `ffmpeg hears ${secs} s`);
});
