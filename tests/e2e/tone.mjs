// The tone capture's reading (TESTING-PLAN §3; DECISIONS B59 e): a continuous sine, read
// block by block (10 ms) against the absolute sample index — its level, its phase, and
// what's left once it's taken out. A sample lost or added anywhere moves the phase of every
// block after it (by 360° × hz / rate per sample: 7.5° at 48 kHz, 8.1° at 44.1 kHz); a
// stretch of silence drops the level.

export class ToneReader {
  constructor(hz, rate) {
    this.block = Math.round(rate / 100);
    this.w = (2 * Math.PI * hz) / rate;
    this.n = 0;
    this.blocks = [];
    this.pending = [];
  }

  /** Feeds samples, in order, in any chunking. */
  push(samples) {
    for (const s of samples) {
      this.pending.push(s);
      if (this.pending.length === this.block) this.#close();
    }
  }

  #close() {
    const x = this.pending, { block, w, n } = this;
    let i = 0, q = 0;
    for (let k = 0; k < block; k++) {
      i += x[k] * Math.cos(w * (n + k));
      q += x[k] * Math.sin(w * (n + k));
    }
    let res = 0;
    for (let k = 0; k < block; k++) {
      const fit = ((2 * i) / block) * Math.cos(w * (n + k)) + ((2 * q) / block) * Math.sin(w * (n + k));
      res += (x[k] - fit) ** 2;
    }
    this.blocks.push([(2 * Math.hypot(i, q)) / block, Math.atan2(q, i), Math.sqrt(res / block)]);
    this.n += block;
    this.pending = [];
  }

  /** Samples read so far. */
  get samples() {
    return this.n + this.pending.length;
  }
}

/** What the blocks say, leaving `edge` blocks out at each end: dropouts (level under half),
 * phase jumps over `jumpDeg` between level blocks, and blocks with more than 5 % of the
 * level left over once the tone is taken out (something else on the line). */
export function toneTruth(blocks, level, { edge = 50, jumpDeg = 3, blockSeconds = 0.01 } = {}) {
  let dropouts = 0, jumps = 0, foreign = 0, worst = 0;
  const firstJumps = [];
  for (let b = Math.max(1, edge); b < blocks.length - edge; b++) {
    const [amp, phase, res] = blocks[b];
    if (amp < level / 2) { dropouts++; continue; }
    if (res > 0.05 * level) foreign++;
    const prev = blocks[b - 1];
    if (prev[0] < level / 2) continue;
    let d = ((phase - prev[1]) * 180) / Math.PI;
    d = ((d + 540) % 360) - 180;
    worst = Math.max(worst, Math.abs(d));
    if (Math.abs(d) > jumpDeg) {
      jumps++;
      if (firstJumps.length < 5) firstJumps.push([Number((b * blockSeconds).toFixed(2)), Number(d.toFixed(1))]);
    }
  }
  return { dropouts, phaseJumps: jumps, firstJumps, worstJumpDeg: Number(worst.toFixed(2)), foreign };
}
