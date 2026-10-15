// The media-truth test pattern (TESTING-PLAN §4), shown full screen by the dev harness
// while it records. It covers the whole display, so nothing else on screen is ever
// recorded during a test. The strip must match crates/shroom-testpattern exactly:
// a 4×8 grid of cells, a twelfth of the height each, one cell in from the top-left
// corner; 32 bits row-major, most significant first: a 24-bit counter, then CRC-8
// (poly 0x07) of its three bytes, the word XORed with MASK. The counter is display ticks (60 per second) since
// the whole wall-clock second before the pattern started, so captured frames can be checked
// against wall time — and a counter that's a multiple of 60 is a whole wall-clock second,
// the instant the drive's beep (shroom_capture::beep, on the same clock) starts: the
// heartbeat reads A/V offset from the published video that way (DECISIONS B59 d). Each
// frame's counter is the time it is presented: the display's frame time plus one refresh.
//
// A second word, the same encoding, nine cells to the right: how many display refreshes
// this pattern has itself missed so far (its frame callbacks more than one refresh apart).
// A frame the recording holds while this count moved is the pattern's own hold, not
// capture's — the media-truth checks tell the two apart (DECISIONS B59 f).

const ROWS = 4;
const COLS = 8;
const TICKS_PER_SECOND = 60;

export function crc8(counter: number): number {
  let crc = 0;
  for (const byte of [(counter >> 16) & 0xff, (counter >> 8) & 0xff, counter & 0xff]) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) {
      crc = crc & 0x80 ? ((crc << 1) ^ 0x07) & 0xff : (crc << 1) & 0xff;
    }
  }
  return crc;
}

/** XORed over the word so a blank frame can never decode (see crates/shroom-testpattern). */
export const MASK = 0xa5c35a3c;

export function word(counter: number): number {
  return (((counter & 0xffffff) * 256 + crc8(counter & 0xffffff)) ^ MASK) >>> 0;
}

function strip(ctx: CanvasRenderingContext2D, cell: number, col0: number, value: number) {
  for (let bit = 0; bit < ROWS * COLS; bit++) {
    const on = Math.floor(value / 2 ** (31 - bit)) % 2 === 1;
    ctx.fillStyle = on ? "#fff" : "#000";
    const r = Math.floor(bit / COLS);
    const c = bit % COLS;
    ctx.fillRect(cell + (col0 + c) * cell, cell + r * cell, cell, cell);
  }
}

/** Where the missed-refresh word starts, in cells from the first. */
export const STALLS_COL = COLS + 1;

function draw(ctx: CanvasRenderingContext2D, w: number, h: number, counter: number, stalls: number) {
  ctx.fillStyle = "#000";
  ctx.fillRect(0, 0, w, h);

  // A bar sweeping across the lower half keeps the screen changing every tick.
  const x = ((counter * 8) % (w + 200)) - 200;
  ctx.fillStyle = "#4c566a";
  ctx.fillRect(x, h * 0.6, 200, h * 0.3);

  const cell = Math.max(4, Math.floor(h / 12));
  strip(ctx, cell, 0, word(counter));
  strip(ctx, cell, STALLS_COL, word(stalls));
}

function start() {
  const canvas = document.getElementById("pattern") as HTMLCanvasElement | null;
  const ctx = canvas?.getContext("2d", { alpha: false });
  if (!canvas || !ctx) return;
  // Epoch milliseconds of a performance.now() reading, and the whole second the counter
  // counts from (24 bits of ticks last 77 hours).
  const epoch = (t: number) => performance.timeOrigin + t;
  const base = Math.floor(epoch(performance.now()) / 1000) * 1000;
  const refresh = 1000 / TICKS_PER_SECOND;
  let last = 0;
  let stalls = 0;
  const frame = (ts: number) => {
    if (last && ts - last > 1.5 * refresh) stalls += Math.round((ts - last) / refresh) - 1;
    last = ts;
    const dpr = window.devicePixelRatio || 1;
    const w = Math.round(window.innerWidth * dpr);
    const h = Math.round(window.innerHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    const counter = Math.round(((epoch(ts) + 1000 / TICKS_PER_SECOND - base) * TICKS_PER_SECOND) / 1000);
    draw(ctx, w, h, counter, stalls);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

if (typeof document !== "undefined") start();
