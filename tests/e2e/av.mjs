// A/V offset read back from a recording (TESTING-PLAN §4, DECISIONS B58 d / B59 d): the
// test pattern's counter is display ticks (60 a second) from a whole wall-clock second, and
// the beep starts on every whole wall-clock second — so each beep's onset is compared with
// where the picture shows that second. The picture's clock around a beep is averaged over
// the second around it (a single sample-and-hold frame reads ±1 frame). Positive: the sound
// is late.

const mean = (a) => a.reduce((s, x) => s + x, 0) / a.length;

/** Where each beep starts, as sample indices: the first loud sample after a quiet stretch. */
export function onsets(samples, rate, { level = 0.2, quiet = 0.25 } = {}) {
  const out = [];
  let still = Infinity;
  for (let i = 0; i < samples.length; i++) {
    if (Math.abs(samples[i]) > level) {
      if (still > quiet * rate) out.push(i);
      still = 0;
    } else {
      still++;
    }
  }
  return out;
}

/**
 * Each beep against the picture. `counters[i]` is frame i's counter (or null), shown at
 * `videoStart + i / fps`; `samples` is mono audio at `rate` starting at `audioStart`.
 */
export function avOffsets(counters, fps, samples, rate, { videoStart = 0, audioStart = 0, ticks = 60 } = {}) {
  const frames = [];
  counters.forEach((c, i) => { if (c !== null) frames.push([videoStart + i / fps, c / ticks]); });
  const out = [];
  let from = 0;
  for (const i of onsets(samples, rate)) {
    const at = audioStart + i / rate;
    while (from < frames.length && frames[from][0] < at - 0.5) from++;
    const near = [];
    for (let j = from; j < frames.length && frames[j][0] <= at + 0.5; j++) near.push(frames[j][1] - frames[j][0]);
    // A second's worth of the picture, or the beep can't be placed (the take's ends).
    if (near.length < (fps * 2) / 3) continue;
    const ahead = mean(near);
    const second = Math.round(at + ahead);
    out.push({ at: Number(at.toFixed(3)), offsetMs: Number(((at - (second - ahead)) * 1000).toFixed(1)) });
  }
  return out;
}

/** The bar's figures: the worst offset, and the drift (the last third's mean against the first's). */
export function avTruth(offsets) {
  if (offsets.length === 0) return { beeps: 0, maxAbsMs: null, meanMs: null, driftMs: null };
  const ms = offsets.map((o) => o.offsetMs);
  const third = Math.max(1, Math.floor(ms.length / 3));
  return {
    beeps: ms.length,
    maxAbsMs: Math.max(...ms.map(Math.abs)),
    meanMs: Number(mean(ms).toFixed(1)),
    driftMs: Number(Math.abs(mean(ms.slice(-third)) - mean(ms.slice(0, third))).toFixed(1)),
  };
}

/** Float samples from ffmpeg's f32le output (a Buffer of any alignment). */
export function f32(buffer) {
  const out = new Float32Array(Math.floor(buffer.length / 4));
  for (let i = 0; i < out.length; i++) out[i] = buffer.readFloatLE(i * 4);
  return out;
}
