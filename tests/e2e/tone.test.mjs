// The tone reading itself, on made-up captures (node --test tests/e2e).
import { test } from "node:test";
import assert from "node:assert/strict";
import { ToneReader, toneTruth } from "./tone.mjs";

const HZ = 997, LEVEL = 0.5;

function capture(rate, seconds, edit = (k) => k) {
  const out = new Float32Array(rate * seconds);
  for (let k = 0; k < out.length; k++) out[k] = LEVEL * Math.sin((2 * Math.PI * HZ * edit(k)) / rate);
  return out;
}

const read = (rate, samples) => {
  const r = new ToneReader(HZ, rate);
  for (let at = 0; at < samples.length; at += 1000) r.push(samples.subarray(at, at + 1000));
  return toneTruth(r.blocks, LEVEL);
};

test("a whole tone reads whole", () => {
  for (const rate of [44_100, 48_000]) {
    const t = read(rate, capture(rate, 5));
    assert.deepEqual([t.dropouts, t.phaseJumps, t.foreign], [0, 0, 0], `${rate}: ${JSON.stringify(t)}`);
  }
});

test("one sample lost, or one added, is a jump", () => {
  for (const rate of [44_100, 48_000]) {
    assert.equal(read(rate, capture(rate, 5, (k) => (k < 2 * rate ? k : k + 1))).phaseJumps, 1, `${rate} lost`);
    assert.equal(read(rate, capture(rate, 5, (k) => (k < 2 * rate ? k : k - 1))).phaseJumps, 1, `${rate} added`);
  }
});

test("a stretch of silence is a dropout; another sound on the line is foreign", () => {
  const rate = 48_000;
  const s = capture(rate, 5);
  s.fill(0, 2 * rate, 2 * rate + 4800);
  assert.ok(read(rate, s).dropouts >= 9);
  const t = capture(rate, 5);
  for (let k = 3 * rate; k < 3 * rate + 4800; k++) t[k] += 0.2 * Math.sin((2 * Math.PI * 440 * k) / rate);
  assert.ok(read(rate, t).foreign >= 9);
});
