// The editor's waveform: the recording's sound as the board draws it — bars every 4 px,
// centred, from the peaks the Mac read (shroom_encode::peaks: one u8 per 20 ms, the peak
// |sample| over every channel, 255 at full scale). Drawn on a canvas at the display's
// pixel density, so the bars stay crisp on a Retina screen.

/** The peaks' rate: `shroom_encode::peaks::PER_SECOND`. */
export const PER_SECOND = 50;
/** The board's bars: one every 4 px, 2.8 px wide, 6 px of air in the track's height. */
const STEP = 4;
const BAR = 2.8;
const AIR = 6;
/** A quiet microphone still shows its shape: the bars scale to the loudest moment, by no
 * more than this much (silence never swells into noise). */
const MAX_GAIN = 4;

/** Each bar's level (0..1): the loudest 20 ms under it. */
export function levels(peaks: Uint8Array, duration: number, width: number): Float32Array {
  const n = Math.max(0, Math.floor(width / STEP));
  const out = new Float32Array(n);
  if (!(duration > 0) || !n) return out;
  const per = (STEP / width) * duration * PER_SECOND;
  for (let i = 0; i < n; i++) {
    const a = Math.floor(i * per);
    const b = Math.max(a + 1, Math.floor((i + 1) * per));
    let m = 0;
    for (let j = a; j < b && j < peaks.length; j++) if (peaks[j] > m) m = peaks[j];
    out[i] = m / 255;
  }
  return out;
}

/** Draws the bars into `canvas`, sized to its box at the device's pixel density. */
export function draw(canvas: HTMLCanvasElement, peaks: Uint8Array | null, duration: number, colour: string) {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  const W = Math.round(w * dpr);
  const H = Math.round(h * dpr);
  if (canvas.width !== W) canvas.width = W;
  if (canvas.height !== H) canvas.height = H;
  const g = canvas.getContext("2d");
  if (!g) return;
  g.clearRect(0, 0, W, H);
  if (!peaks?.length || !(duration > 0) || !w || !h) return;
  const lv = levels(peaks, duration, w);
  const loudest = lv.reduce((m, v) => Math.max(m, v), 0);
  const gain = loudest > 0 ? Math.min(MAX_GAIN, 1 / loudest) : 1;
  const bw = Math.max(1, Math.round(BAR * dpr));
  const room = H - AIR * dpr;
  const least = Math.round(2 * dpr);
  g.fillStyle = colour;
  for (let i = 0; i < lv.length; i++) {
    // Whole device pixels, an even height: every bar sits square on the pixel grid.
    const bh = Math.max(least, Math.round((Math.min(1, lv[i] * gain) * room) / 2) * 2);
    g.fillRect(Math.round(i * STEP * dpr), Math.round((H - bh) / 2), bw, bh);
  }
}
