// Quality-preset tests: the resolution/bitrate presets, the never-upscale scale
// filter, and the size/cost estimates the picker shows. Run:
// node scripts/recorder/test/quality.test.mjs

import assert from 'node:assert/strict';
import {
  QUALITY, DEFAULT_QUALITY, resolveQuality, scaleFilter, ffmpegBitrate,
  estimate, qualityCatalogue,
} from '../lib/quality.mjs';

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('three presets; default is normal/1080p', () => {
  assert.deepEqual(Object.keys(QUALITY), ['normal', '2k', '4k']);
  assert.equal(DEFAULT_QUALITY, 'normal');
  assert.equal(QUALITY.normal.maxHeight, 1080);
  assert.equal(QUALITY['4k'].maxHeight, 2160);
});

test('resolveQuality falls back to default for unknown keys', () => {
  assert.equal(resolveQuality('2k'), '2k');
  assert.equal(resolveQuality('ultra'), 'normal');
  assert.equal(resolveQuality(undefined), 'normal');
});

test('scaleFilter fits the box, never upscales, keeps even dims', () => {
  const f = scaleFilter('normal');
  assert.match(f, /min\(1920,iw\)/);   // never wider than the box (no upscale)
  assert.match(f, /min\(1080,ih\)/);
  assert.match(f, /force_original_aspect_ratio=decrease/); // preserve aspect
  assert.match(f, /force_divisible_by=2/);                 // yuv420p needs even dims
});

test('ffmpegBitrate formats the preset bitrate', () => {
  assert.equal(ffmpegBitrate('normal'), '3.5M');
  assert.equal(ffmpegBitrate('2k'), '6M');
  assert.equal(ffmpegBitrate('4k'), '12M');
});

test('estimates grow with quality and are sane round numbers', () => {
  const n = estimate('normal');
  const k4 = estimate('4k');
  assert.ok(n.mbPerMin > 20 && n.mbPerMin < 35);       // ~27 MB/min
  assert.ok(k4.mbPerMin > n.mbPerMin * 2);             // 4K is much heavier
  assert.ok(k4.usdPerHourMonth > n.usdPerHourMonth);   // and costs more to store
  assert.ok(n.usdPerHourMonth < 0.1);                  // still pennies (egress free)
});

test('catalogue carries everything the picker surfaces', () => {
  const cat = qualityCatalogue();
  assert.equal(cat.length, 3);
  const four = cat.find((c) => c.key === '4k');
  assert.equal(four.resolution, '3840x2160');
  assert.ok('mbPerMin' in four && 'gbPerHour' in four && 'usdPerHourMonth' in four);
});

(async () => {
  for (const [name, fn] of tests) {
    try { await fn(); passed++; console.log(`ok   ${name}`); }
    catch (e) { console.error(`FAIL ${name}\n     ${e.message}`); }
  }
  console.log(`\n${passed}/${tests.length} tests passed`);
  process.exit(passed === tests.length ? 0 : 1);
})();
