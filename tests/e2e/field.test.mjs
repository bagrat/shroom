// The dark-field reader against frames drawn here the way app/src/pattern.ts draws them —
// the same cases crates/shroom-testpattern holds its own reader to.
//
//   node --test tests/e2e/field.test.mjs

import test from "node:test";
import assert from "node:assert/strict";
import { fieldLit } from "./field.mjs";
import { crc8, MASK, readStripGray } from "./strip.mjs";

// The on-screen pattern in gray: black, the bar in its band, both strips.
function onScreen(w, h, counter) {
  const g = new Uint8Array(w * h);
  const cell = Math.max(4, Math.floor(h / 12));
  const word = (((counter * 256 + crc8(counter)) ^ MASK) >>> 0);
  for (let bit = 0; bit < 32; bit++) {
    const on = Math.floor(word / 2 ** (31 - bit)) % 2 === 1;
    const r = Math.floor(bit / 8), c = bit % 8;
    for (let y = cell + r * cell; y < cell + (r + 1) * cell; y++) {
      for (let x = cell + c * cell; x < cell + (c + 1) * cell; x++) g[y * w + x] = on ? 255 : 0;
      // The second strip, nine cells to the right: every cell white is the brightest it gets.
      for (let x = cell + (9 + c) * cell; x < Math.min(w, cell + (10 + c) * cell); x++) g[y * w + x] = 255;
    }
  }
  for (let y = Math.floor(h * 0.6); y < Math.floor(h * 0.9); y++) for (let x = 40; x < Math.min(w, 240); x++) g[y * w + x] = 0x5a;
  return g;
}

// `g` with a box of `value` drawn over it, blended at `alpha`.
function over(g, w, [x0, y0, bw, bh], value, alpha) {
  const out = Uint8Array.from(g);
  for (let y = y0; y < y0 + bh; y++) for (let x = x0; x < x0 + bw; x++) out[y * w + x] = Math.round(out[y * w + x] * (1 - alpha) + value * alpha);
  return out;
}

test("the pattern alone lights none of its dark field", () => {
  for (const [w, h] of [[1920, 1080], [1728, 1117], [1024, 768], [480, 360]]) {
    assert.equal(fieldLit(onScreen(w, h, 4242), w, h), 0, `${w}×${h}`);
    assert.equal(fieldLit(onScreen(w, h, 4242).map((v) => Math.min(255, v + 9)), w, h), 0, `${w}×${h}, black as an encoder leaves it`);
  }
});

test("the display's own rim is not the field", () => {
  // The pattern window's outermost ring at 25 and the next ring in at 5, as a hosted Mac's
  // 1024×768 display at 1× draws it (issue #4).
  for (const [w, h] of [[1920, 1080], [1024, 768], [480, 360]]) {
    const g = onScreen(w, h, 4242);
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const ring = Math.min(x, y, w - 1 - x, h - 1 - y);
      if (ring < 2) g[y * w + x] = ring === 0 ? 25 : 5;
    }
    assert.equal(fieldLit(g, w, h), 0, `${w}×${h}`);
  }
});

test("anything over the pattern lights the field it covers", () => {
  for (const [w, h] of [[1920, 1080], [1024, 768]]) {
    // The count's scrim: the whole display, light, a third opaque.
    assert.ok(fieldLit(over(onScreen(w, h, 7), w, [0, 0, w, h], 236, 0.35), w, h) > 0.99, `${w}×${h}: the scrim`);
    // The popover: 320×360 hanging from the menu bar at the right, in its darkest face.
    assert.ok(fieldLit(over(onScreen(w, h, 7), w, [w - 360, 24, 320, 360], 0x33, 1), w, h) > 0.05, `${w}×${h}: the popover`);
  }
  // The strip still reads under the scrim.
  assert.equal(readStripGray(over(onScreen(1024, 768, 7), 1024, [0, 0, 1024, 768], 236, 0.35), 1024, 768), 7);
});
