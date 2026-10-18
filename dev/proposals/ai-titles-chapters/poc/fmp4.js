// The audio out of a shroom take's pieces, with no dependency: the init segment's AAC track
// and each piece's audio samples rewrapped as ADTS frames, a stream Whisper reads as it is.
// A piece's start comes from its tfdt, so a window's words carry the take's own clock.

const u32 = (b, o) => ((b[o] << 24) | (b[o + 1] << 16) | (b[o + 2] << 8) | b[o + 3]) >>> 0;
const u64 = (b, o) => u32(b, o) * 2 ** 32 + u32(b, o + 4);
const type = (b, o) => String.fromCharCode(b[o], b[o + 1], b[o + 2], b[o + 3]);

/** The boxes in b[from, to): {type, start, body, end}. */
function boxes(b, from = 0, to = b.length) {
  const out = [];
  for (let o = from; o + 8 <= to; ) {
    let size = u32(b, o), head = 8;
    if (size === 1) (size = u64(b, o + 8)), (head = 16);
    else if (size === 0) size = to - o;
    if (size < head || o + size > to) throw new Error(`bad box at ${o}`);
    out.push({ type: type(b, o + 4), start: o, body: o + head, end: o + size });
    o += size;
  }
  return out;
}
const child = (b, box, t) => boxes(b, box.body, box.end).find((x) => x.type === t);
const children = (b, box, t) => boxes(b, box.body, box.end).filter((x) => x.type === t);
const path = (b, box, ...ts) => ts.reduce((x, t) => x && child(b, x, t), box);

/** An MPEG-4 descriptor's tag, its body's offset and its end. */
function descriptor(b, o) {
  const tag = b[o++];
  let len = 0;
  for (let i = 0; i < 4; i++) {
    const c = b[o++];
    len = (len << 7) | (c & 0x7f);
    if (!(c & 0x80)) break;
  }
  return { tag, body: o, end: o + len };
}

/** The AudioSpecificConfig inside an esds box. */
function audioSpecificConfig(b, esds) {
  let es = descriptor(b, esds.body + 4);
  if (es.tag !== 3) throw new Error('esds without an ES descriptor');
  let o = es.body + 2;
  const flags = b[o++];
  if (flags & 0x80) o += 2;
  if (flags & 0x40) o += 1 + b[o];
  if (flags & 0x20) o += 2;
  const dc = descriptor(b, o);
  if (dc.tag !== 4) throw new Error('esds without a decoder config');
  const dsi = descriptor(b, dc.body + 13);
  if (dsi.tag !== 5) throw new Error('esds without an AudioSpecificConfig');
  const objectType = b[dsi.body] >> 3;
  const freqIndex = ((b[dsi.body] & 7) << 1) | (b[dsi.body + 1] >> 7);
  const channels = (b[dsi.body + 1] >> 3) & 15;
  return { objectType, freqIndex, channels };
}

const RATES = [96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350];

/** The init segment's sound track: its id, clock, AAC config and the movie's defaults. */
export function audioTrack(init) {
  const moov = boxes(init).find((x) => x.type === 'moov');
  if (!moov) throw new Error('no moov');
  for (const trak of children(init, moov, 'trak')) {
    const mdia = child(init, trak, 'mdia');
    const hdlr = mdia && child(init, mdia, 'hdlr');
    if (!hdlr || type(init, hdlr.body + 8) !== 'soun') continue;
    const tkhd = child(init, trak, 'tkhd');
    const trackId = u32(init, tkhd.body + (init[tkhd.body] === 1 ? 20 : 12));
    const mdhd = child(init, mdia, 'mdhd');
    const timescale = u32(init, mdhd.body + (init[mdhd.body] === 1 ? 20 : 12));
    const stsd = path(init, mdia, 'minf', 'stbl', 'stsd');
    const entry = boxes(init, stsd.body + 8, stsd.end)[0];
    if (!entry || entry.type !== 'mp4a') throw new Error(`sound track is ${entry && entry.type}, not AAC`);
    const esds = boxes(init, entry.body + 28, entry.end).find((x) => x.type === 'esds');
    const asc = audioSpecificConfig(init, esds);
    const trex = children(init, child(init, moov, 'mvex') || moov, 'trex').find((x) => u32(init, x.body + 4) === trackId);
    return {
      trackId,
      timescale,
      ...asc,
      sampleRate: RATES[asc.freqIndex],
      defaultDuration: trex ? u32(init, trex.body + 12) : 0,
      defaultSize: trex ? u32(init, trex.body + 16) : 0,
    };
  }
  throw new Error('no sound track');
}

function adtsHeader(track, frameLength) {
  const len = frameLength + 7;
  return [
    0xff,
    0xf1,
    ((track.objectType - 1) << 6) | (track.freqIndex << 2) | (track.channels >> 2),
    ((track.channels & 3) << 6) | (len >> 11),
    (len >> 3) & 0xff,
    ((len & 7) << 5) | 0x1f,
    0xfc,
  ];
}

/**
 * The sound track's samples in a piece's moof boxes: where each sits (an offset into the
 * piece), its size, and the piece's start and length in track ticks. `b` holds the moofs;
 * `at` is where `b` begins in the piece, so a moof read alone (a range) works too.
 */
export function audioSamples(b, track, at = 0) {
  const samples = [];
  let start = null, ticks = 0;
  for (const moof of boxes(b).filter((x) => x.type === 'moof')) {
    for (const traf of children(b, moof, 'traf')) {
      const tfhd = child(b, traf, 'tfhd');
      const tf = u32(b, tfhd.body) & 0xffffff;
      if (u32(b, tfhd.body + 4) !== track.trackId) continue;
      let o = tfhd.body + 8, base = at + moof.start;
      if (tf & 0x1) (base = u64(b, o)), (o += 8);
      if (tf & 0x2) o += 4;
      let dDur = track.defaultDuration, dSize = track.defaultSize;
      if (tf & 0x8) (dDur = u32(b, o)), (o += 4);
      if (tf & 0x10) (dSize = u32(b, o)), (o += 4);
      const tfdt = child(b, traf, 'tfdt');
      if (tfdt && start === null) start = b[tfdt.body] === 1 ? u64(b, tfdt.body + 4) : u32(b, tfdt.body + 4);
      let next = null;
      for (const trun of children(b, traf, 'trun')) {
        const rf = u32(b, trun.body) & 0xffffff;
        const count = u32(b, trun.body + 4);
        let p = trun.body + 8;
        let data = next ?? base;
        if (rf & 0x1) (data = base + (u32(b, p) | 0)), (p += 4);
        if (rf & 0x4) p += 4;
        for (let i = 0; i < count; i++) {
          let dur = dDur, size = dSize;
          if (rf & 0x100) (dur = u32(b, p)), (p += 4);
          if (rf & 0x200) (size = u32(b, p)), (p += 4);
          if (rf & 0x400) p += 4;
          if (rf & 0x800) p += 4;
          samples.push({ at: data, size });
          data += size;
          ticks += dur;
        }
        next = data;
      }
    }
  }
  return { samples, start: start ?? 0, ticks };
}

/** Samples as ADTS frames; `data` holds the piece from offset `from`. */
function adts(track, samples, data, from = 0) {
  const out = new Uint8Array(samples.reduce((n, x) => n + x.size + 7, 0));
  let o = 0;
  for (const x of samples) {
    out.set(adtsHeader(track, x.size), o);
    out.set(data.subarray(x.at - from, x.at - from + x.size), o + 7);
    o += x.size + 7;
  }
  return out;
}

/** One piece's audio: {adts, start, duration} in seconds of the take's clock. */
export function pieceAudio(piece, track) {
  const { samples, start, ticks } = audioSamples(piece, track);
  return { adts: adts(track, samples, piece), start: start / track.timescale, duration: ticks / track.timescale };
}

/**
 * The same, reading only what it needs: `read(offset, length)` returns those bytes of the
 * piece (R2's ranged get). It walks the top-level boxes by their headers, reads each moof
 * whole and, per fragment, the one span its audio samples sit in; the video is never read.
 * A 6 s piece at 3.5 Mbit/s is ~2.6 MB; its audio is ~96 KB.
 */
export async function pieceAudioRanged(read, size, track) {
  const parts = [];
  for (let o = 0; o + 8 <= size; ) {
    const h = await read(o, 16);
    let len = u32(h, 0);
    if (len === 1) len = u64(h, 8);
    else if (len === 0) len = size - o;
    if (type(h, 4) === 'moof') {
      const moof = await read(o, len);
      const s = audioSamples(moof, track, o);
      if (s.samples.length) {
        const from = s.samples[0].at, to = s.samples[s.samples.length - 1].at + s.samples[s.samples.length - 1].size;
        parts.push({ ...s, data: await read(from, to - from), from });
      } else parts.push({ ...s, data: new Uint8Array(0), from: 0 });
    }
    o += len;
  }
  const bytes = parts.map((p) => adts(track, p.samples, p.data, p.from));
  const all = new Uint8Array(bytes.reduce((n, b) => n + b.length, 0));
  let at = 0;
  for (const b of bytes) all.set(b, at), (at += b.length);
  return { adts: all, start: (parts[0]?.start ?? 0) / track.timescale, duration: parts.reduce((n, p) => n + p.ticks, 0) / track.timescale };
}

/** Pieces' audio joined: one ADTS stream, starting where its first piece starts. */
export function joinAudio(parts) {
  const adts = new Uint8Array(parts.reduce((n, p) => n + p.adts.length, 0));
  let at = 0;
  for (const p of parts) adts.set(p.adts, at), (at += p.adts.length);
  const last = parts[parts.length - 1];
  return { adts, start: parts[0].start, end: last.start + last.duration };
}
