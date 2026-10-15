// The counter strip reader (crates/shroom-testpattern, ported): a 4×8 grid of cells,
// a twelfth of the frame height each, one cell in from the top-left; 32 bits row-major,
// MSB first: 24-bit counter, then CRC-8 (poly 0x07), the word XORed with MASK so blank
// frames never decode. Returns null when the CRC fails. `col0` reads the word that many cells
// to the right: the pattern draws a second one at STALLS_COL, the refreshes it has itself
// missed so far (DECISIONS B59 f).

export const STALLS_COL = 9;

export function crc8(counter) {
  let crc = 0;
  for (const byte of [(counter >> 16) & 0xff, (counter >> 8) & 0xff, counter & 0xff]) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = crc & 0x80 ? ((crc << 1) ^ 0x07) & 0xff : (crc << 1) & 0xff;
  }
  return crc;
}

// `rgba` is ImageData.data for a width×height frame.
export const MASK = 0xa5c35a3c;

export function readStrip(rgba, width, height, col0 = 0) {
  const cell = Math.max(4, Math.floor(height / 12));
  const inset = Math.floor(cell / 4);
  let bits = [];
  for (let bit = 0; bit < 32; bit++) {
    const r = Math.floor(bit / 8), c = col0 + (bit % 8);
    const x0 = cell + c * cell + inset, y0 = cell + r * cell + inset;
    const x1 = cell + (c + 1) * cell - inset, y1 = cell + (r + 1) * cell - inset;
    let sum = 0, n = 0;
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
      const i = (y * width + x) * 4;
      sum += (rgba[i] + rgba[i + 1] + rgba[i + 2]) / 3; n++;
    }
    bits.push(n && sum / n >= 128 ? 1 : 0);
  }
  let w = 0;
  for (let i = 0; i < 32; i++) w = w * 2 + bits[i];
  w = (w ^ 0xa5c35a3c) >>> 0;
  const counter = Math.floor(w / 256);
  const check = w % 256;
  return crc8(counter) === check ? counter : null;
}

// The same strip from a one-channel (gray) frame, as ffmpeg's `-pix_fmt gray` gives it.
export function readStripGray(gray, width, height, col0 = 0) {
  const cell = Math.max(4, Math.floor(height / 12));
  const inset = Math.floor(cell / 4);
  let w = 0;
  for (let bit = 0; bit < 32; bit++) {
    const r = Math.floor(bit / 8), c = col0 + (bit % 8);
    const x0 = cell + c * cell + inset, y0 = cell + r * cell + inset;
    const x1 = cell + (c + 1) * cell - inset, y1 = cell + (r + 1) * cell - inset;
    let sum = 0, n = 0;
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) { sum += gray[y * width + x]; n++; }
    w = w * 2 + (n && sum / n >= 128 ? 1 : 0);
  }
  w = (w ^ MASK) >>> 0;
  const counter = Math.floor(w / 256);
  return crc8(counter) === w % 256 ? counter : null;
}

// What a run of counters says against TESTING-PLAN §4's frame bar, for a pattern that counts
// `ticks` per second captured at `fps`. A repeated frame shows the previous frame's counter
// (a hold). A skipped frame is a 1/fps slot the video never showed: the slots the counter
// spans, less the distinct frames that showed them (less one slot: the ends' phase) — so the
// two clocks' phase (a step of 3 ticks, then 1) nets out, and a hold followed by the
// catch-up counts once each way.
//
// `stalls` (optional, per frame: the pattern's own missed-refresh count, or null) says which
// holds were the pattern's: a repeated frame whose neighbours (the frame before it and the
// one after) show that count moving is a hold the screen itself made — the screen showed
// the same picture twice and the recording kept it faithfully — and its catch-up skip with
// it. `repeatedByCapture` / `skippedByCapture` are the rest: what capture itself held or lost.
export function frameTruth(counters, ticks = 60, fps = 30, stalls = null) {
  const step = ticks / fps;
  let repeated = 0, backwards = 0, prev = null;
  const at = counters.map((c, i) => [c, i]).filter(([c]) => c !== null);
  for (const [c] of at) {
    if (prev !== null) {
      if (c === prev) repeated++;
      else if (c < prev) backwards++;
    }
    prev = c;
  }
  const frames = counters.length;
  let skipped = 0, pace = null;
  if (at.length > 1) {
    const [[first, i0], [last, i1]] = [at[0], at.at(-1)];
    const slots = Math.round((last - first) / step) + 1;
    const shown = i1 - i0 + 1 - repeated;
    // Each end's frame catches the pattern at its own phase (a tick early or late): the span
    // can read one slot long with nothing lost. One slot is that slack, never a loss.
    skipped = Math.max(0, slots - shown - 1);
    pace = Number((((last - first) / ticks) / ((i1 - i0) / fps)).toFixed(4));
  }
  const pct = (n) => Number(((n / Math.max(frames, 1)) * 100).toFixed(3));
  const out = { frames, unreadable: frames - at.length, repeated, skipped, backwards, repeatedPct: pct(repeated), skippedPct: pct(skipped), pace };
  if (stalls) {
    let byPattern = 0;
    for (let i = 1; i < counters.length; i++) {
      if (counters[i] === null || counters[i] !== counters[i - 1]) continue;
      const before = stalls[i - 1], after = stalls[Math.min(i + 1, stalls.length - 1)];
      if (before !== null && after !== null && after > before) byPattern++;
    }
    const capRepeated = repeated - byPattern;
    // Each pattern hold's catch-up is its own skip; what's left is capture's.
    const capSkipped = Math.max(0, skipped - byPattern);
    Object.assign(out, {
      patternHolds: byPattern, patternMissedRefreshes: stalls.filter((x) => x !== null).at(-1) - (stalls.find((x) => x !== null) ?? 0),
      repeatedByCapture: capRepeated, skippedByCapture: capSkipped, repeatedByCapturePct: pct(capRepeated), skippedByCapturePct: pct(capSkipped),
    });
  }
  return out;
}
