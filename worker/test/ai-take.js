// A take as shroom's encoder writes it (crates/shroom-encode/src/args.rs: ffmpeg's HLS muxer,
// fragmented MP4, six-second pieces, AAC beside H.264), built box by box so the tests can say
// which bytes are sound — and a fake Whisper that hears exactly the frames it is sent.
//
// Every sound frame's four bytes name its piece and its place in it ([0xAA, piece, n >> 8,
// n & 255]), so the fake knows the recording time of all it hears; a picture sample is 0x11s
// throughout, so a test can tell whether a picture byte was ever sent. Sound runs at 48 kHz in
// frames of 1,000 ticks (48 a second, so six seconds is 288 frames exactly).

const enc = new TextEncoder();
/** @param {(number[] | Uint8Array)[]} parts */
export function cat(parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) { out.set(p, o); o += p.length; }
  return out;
}
const u16 = (n) => [(n >> 8) & 255, n & 255];
const u32 = (n) => [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];
const u64 = (n) => [...u32(Math.floor(n / 2 ** 32)), ...u32(n >>> 0)];
const zeros = (n) => new Uint8Array(n);
const box = (type, ...parts) => { const body = cat(parts); return cat([u32(8 + body.length), enc.encode(type), body]); };
const full = (type, version, flags, ...parts) => box(type, [version, (flags >> 16) & 255, (flags >> 8) & 255, flags & 255], ...parts);
/** An MPEG-4 descriptor, its length in four bytes as ffmpeg writes it. */
const desc = (tag, ...parts) => { const body = cat(parts); return cat([[tag, 0x80, 0x80, 0x80, body.length], body]); };

export const RATE = 48000;
export const FRAME_TICKS = 1000;
export const FRAMES_PER_PIECE = 288; // 6 s
export const PIECE_SECS = (FRAMES_PER_PIECE * FRAME_TICKS) / RATE;
const VIDEO_SCALE = 15360;
const VIDEO_TICKS = 512; // 30 fps
const VIDEO_SIZE = 3000;

/** The init segment: picture (track 1) and, unless `sound` is false, AAC-LC (track 2). */
export function initMp4({ sound = true, channels = 1 } = {}) {
  const tkhd = (id) => full("tkhd", 0, 3, u32(0), u32(0), u32(id), u32(0), u32(0), zeros(60));
  const mdhd = (scale) => full("mdhd", 0, 0, u32(0), u32(0), u32(scale), u32(0), u16(0x55c4), u16(0));
  const hdlr = (kind) => full("hdlr", 0, 0, u32(0), enc.encode(kind), zeros(12), enc.encode("Handler\0"));
  // AAC-LC (2), 48 kHz (index 3), `channels`: 00010 0011 cccc 000.
  const asc = [0x11, 0x80 | (channels << 3)];
  const esds = full("esds", 0, 0, desc(3, u16(1), [0], desc(4, [0x40, 0x15, 0, 0, 0], u32(128000), u32(128000), desc(5, asc)), desc(6, [2])));
  const mp4a = box("mp4a", zeros(6), u16(1), zeros(8), u16(channels), u16(16), u16(0), u16(0), u32(RATE * 65536), esds);
  const video = box("trak", tkhd(1), box("mdia", mdhd(VIDEO_SCALE), hdlr("vide"), box("minf", box("stbl", full("stsd", 0, 0, u32(1), box("avc1", zeros(78)))))));
  const audio = box("trak", tkhd(2), box("mdia", mdhd(RATE), hdlr("soun"), box("minf", box("stbl", full("stsd", 0, 0, u32(1), mp4a)))));
  const trex = (id, dur, size) => full("trex", 0, 0, u32(id), u32(1), u32(dur), u32(size), u32(0));
  const mvex = box("mvex", trex(1, VIDEO_TICKS, VIDEO_SIZE), ...(sound ? [trex(2, FRAME_TICKS, 4)] : []));
  return cat([box("ftyp", enc.encode("iso6"), u32(512), enc.encode("iso6mp41")), box("moov", full("mvhd", 0, 0, u32(0), u32(0), u32(1000), u32(0), zeros(80)), video, ...(sound ? [audio] : []), mvex)]);
}

/**
 * Piece `p` of a take: `fragments` moof+mdat pairs, each the picture's samples and then the
 * sound's. `legacy`: no default-base-is-moof — the sound's run follows the picture's data
 * with no offset of its own (ISO/IEC 14496-12's other way to say where a run starts).
 */
export function pieceMp4(p, { sound = true, fragments = 2, frames = FRAMES_PER_PIECE, legacy = false } = {}) {
  const parts = [box("styp", enc.encode("msdh"), u32(0), enc.encode("msdhmsix"))];
  const perFrag = Math.ceil(frames / fragments);
  const videoPerFrag = Math.round((perFrag * FRAME_TICKS * VIDEO_SCALE) / RATE / VIDEO_TICKS);
  for (let f = 0, first = 0; first < frames; f++, first += perFrag) {
    const n = Math.min(perFrag, frames - first);
    const video = new Uint8Array(videoPerFrag * VIDEO_SIZE).fill(0x11);
    const audio = cat(Array.from({ length: sound ? n : 0 }, (_, i) => [0xaa, p, ((first + i) >> 8) & 255, (first + i) & 255]));
    const moof = (vOff, aOff) => box("moof", full("mfhd", 0, 0, u32(p * 100 + f + 1)),
      box("traf", full("tfhd", 0, legacy ? 0 : 0x020000, u32(1)), full("tfdt", 1, 0, u64(0)), full("trun", 0, 0x1 | 0x4, u32(videoPerFrag), u32(vOff), u32(0x02000000))),
      ...(sound ? [box("traf", full("tfhd", 0, legacy ? 0 : 0x020000, u32(2)), full("tfdt", 1, 0, u64((p * frames + first) * FRAME_TICKS)),
        legacy
          ? full("trun", 0, 0x100 | 0x200, u32(n), ...Array.from({ length: n }, () => [...u32(FRAME_TICKS), ...u32(4)]))
          : full("trun", 0, 0x1 | 0x100 | 0x200, u32(n), u32(aOff), ...Array.from({ length: n }, () => [...u32(FRAME_TICKS), ...u32(4)])))] : []));
    const size = moof(0, 0).length;
    parts.push(moof(size + 8, size + 8 + video.length), box("mdat", video, audio));
  }
  return cat(parts);
}

/** A take's objects under rec/<id>/v0/, as a recording Mac uploads them. */
export function takeObjects(id, pieces, opts = {}) {
  /** @type {Record<string, Uint8Array>} */
  const out = { [`rec/${id}/v0/init.mp4`]: initMp4(opts) };
  for (let p = 0; p < pieces; p++) out[`rec/${id}/v0/seg_${String(p).padStart(5, "0")}.m4s`] = pieceMp4(p, opts);
  return out;
}

/** Words on the recording's clock, `rate` s a word, a pause of `gap` s after each sentence. */
export function script(sentences, { rate = 0.4, gap = 0.6, from = 0.5 } = {}) {
  /** @type {{ w: string, s: number, e: number }[]} */
  const words = [];
  let t = from;
  for (const sentence of sentences) {
    for (const w of sentence.split(/\s+/).filter(Boolean)) { words.push({ w, s: t, e: t + rate - 0.05 }); t += rate; }
    t += gap;
  }
  return words;
}

/** The ADTS frames in `bytes`, each frame's four payload bytes. */
export function adtsFrames(bytes) {
  const out = [];
  for (let o = 0; o + 7 <= bytes.length;) {
    if (bytes[o] !== 0xff || (bytes[o + 1] & 0xf6) !== 0xf0) throw new Error(`no ADTS sync at ${o}`);
    const len = ((bytes[o + 3] & 3) << 11) | (bytes[o + 4] << 3) | (bytes[o + 5] >> 5);
    out.push({ header: bytes.subarray(o, o + 7), payload: bytes.subarray(o + 7, o + len) });
    o += len;
  }
  return out;
}

/**
 * A fake `env.AI`. Whisper (turbo) answers what is said in the frames it was sent: words
 * whose middle it heard, timed from the clip's first frame, a word cut by the clip's end
 * half-heard (as Whisper does). `name(input, model)` answers the naming calls; `fail(model,
 * input)` may throw first, as a refused call does.
 */
export function fakeAI(words, { name, fail } = {}) {
  /** @type {{ model: string, input: any, heard?: { from: number, to: number, frames: any[] } }[]} */
  const calls = [];
  return {
    calls,
    async run(model, input) {
      const call = { model, input };
      calls.push(call);
      if (fail) await fail(model, input);
      if (model.includes("whisper")) {
        if (!(input.audio?.body instanceof ReadableStream) || input.audio.contentType !== "audio/aac") throw new Error("8001: Invalid input");
        const frames = adtsFrames(new Uint8Array(await new Response(input.audio.body).arrayBuffer()));
        const at = (f) => f.payload[1] * PIECE_SECS + ((f.payload[2] << 8) | f.payload[3]) * (FRAME_TICKS / RATE);
        if (!frames.length) return { text: "", segments: [] };
        const from = at(frames[0]), to = at(frames[frames.length - 1]) + FRAME_TICKS / RATE;
        call.heard = { from, to, frames };
        const heard = [];
        for (const x of words) {
          if (x.e <= from || x.s >= to) continue;
          if (x.s < from) continue; // its start went unheard
          if (x.e > to) { heard.push({ word: ` ${x.w.slice(0, 2)}-`, start: x.s - from, end: to - from }); continue; }
          heard.push({ word: ` ${x.w}`, start: +(x.s - from).toFixed(2), end: +(x.e - from).toFixed(2) });
        }
        return { text: heard.map((x) => x.word).join(""), transcription_info: { language: "en", duration: to - from }, segments: [{ start: 0, end: to - from, text: "", words: heard }] };
      }
      if (!name) throw new Error("no naming answer");
      return name(input, model);
    },
  };
}

/** A chat answer in the OpenAI shape gpt-oss gives on Workers AI. */
export const chat = (obj) => ({ choices: [{ message: { role: "assistant", content: JSON.stringify(obj) } }], usage: { prompt_tokens: 900, completion_tokens: 80 } });
/** The `{response}` shape other models give. */
export const responseOf = (obj) => ({ response: obj });
