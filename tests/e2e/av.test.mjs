// The A/V reading itself, on made-up recordings with known offsets (node --test tests/e2e).
import { test } from "node:test";
import assert from "node:assert/strict";
import { avOffsets, avTruth, onsets } from "./av.mjs";
import { frameTruth } from "./strip.mjs";

const FPS = 30, RATE = 16_000;

/** A take that starts `phase` s into a wall-clock second: the picture samples a 60 Hz
 * counter on a 30 fps clock, the sound has the beep at each whole second, `late(t)` s late. */
function take(seconds, phase, late) {
  const counters = [];
  for (let i = 0; i < seconds * FPS; i++) counters.push(Math.floor((phase + i / FPS) * 60));
  const samples = new Float32Array(seconds * RATE);
  for (let n = 0; n < samples.length; n++) {
    const wall = phase + n / RATE;
    // The beep heard at output time t was made at wall time t - late.
    const made = wall - late(wall);
    const madeInto = made - Math.floor(made);
    samples[n] = madeInto < 0.1 ? 0.5 * Math.sin(2 * Math.PI * 1000 * made) : 0;
  }
  return { counters, samples };
}

test("reads the sound's lateness against the picture, beep by beep", () => {
  const { counters, samples } = take(12, 0.37, () => 0.025);
  const offsets = avOffsets(counters, FPS, samples, RATE);
  assert.ok(offsets.length >= 10, `beeps placed: ${offsets.length}`);
  for (const o of offsets) assert.ok(Math.abs(o.offsetMs - 25) <= 4, `${o.offsetMs} ms`);
  const t = avTruth(offsets);
  assert.ok(t.driftMs <= 2, `no drift: ${t.driftMs}`);
});

test("sees a sound that drifts", () => {
  const { counters, samples } = take(30, 0.1, (wall) => 0.002 * wall);
  const t = avTruth(avOffsets(counters, FPS, samples, RATE));
  assert.ok(t.driftMs > 30, `drift ${t.driftMs}`);
});

test("finds each onset once, after a quiet stretch", () => {
  const s = new Float32Array(RATE * 3);
  for (const at of [0.5, 1.5, 2.5]) for (let n = 0; n < RATE / 10; n++) s[Math.round(at * RATE) + n] = n % 16 < 8 ? 0.5 : -0.5;
  assert.deepEqual(onsets(s, RATE).map((i) => i / RATE), [0.5, 1.5, 2.5]);
});

test("frame truth: the pattern's 3-then-1 phase is no loss; a hold is a repeat, its catch-up a skip past the ends' one slot of slack", () => {
  const clean = Array.from({ length: 300 }, (_, i) => Math.floor((0.004 + i / FPS) * 60));
  assert.deepEqual([frameTruth(clean).repeated, frameTruth(clean).skipped], [0, 0]);
  const held = clean.slice();
  held[100] = held[99];
  const t = frameTruth(held);
  assert.deepEqual([t.repeated, t.skipped], [1, 0]);
  held[200] = held[199];
  const t2 = frameTruth(held);
  assert.deepEqual([t2.repeated, t2.skipped], [2, 1]);
});

test("frame truth: a hold the pattern itself made (its missed-refresh count moved) isn't capture's", () => {
  const counters = Array.from({ length: 300 }, (_, i) => Math.floor((0.004 + i / FPS) * 60));
  const stalls = counters.map(() => 0);
  counters[100] = counters[99];
  for (let i = 101; i < 300; i++) stalls[i] = 2;
  counters[200] = counters[199];
  const t = frameTruth(counters, 60, FPS, stalls);
  assert.equal(t.repeated, 2);
  assert.equal(t.patternHolds, 1);
  assert.equal(t.repeatedByCapture, 1, "the hold at 200 had no stall of the pattern's beside it");
  assert.equal(t.patternMissedRefreshes, 2);
});
