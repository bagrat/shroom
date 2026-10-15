// The recipe's signature (TESTING-PLAN §3, Oracle 2) of a published recording, read back
// from its page's own URLs, against `golden/hls-signature.json` — the golden the reference
// capture holds too (crates/shroom-record/tests/signature.rs computes the same fields).
//
//   signatureOf(base)  base = https://<host>/m/<id>   → the signature
//   compare(sig)       → [] when it matches the golden, else what differs
//
// What depends on the display or the take's length (size, level, bitrate, durations beyond
// the whole-segment rule) is not part of it; a one-take recording has no pause, so it says
// nothing about discontinuities.

import { execFile } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
export const GOLDEN = path.join(here, "golden/hls-signature.json");
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;

function probe(args, input) {
  return new Promise((ok, fail) => execFile(tool("ffprobe"), ["-v", "error", ...args, "-of", "json", input], { timeout: 120_000, maxBuffer: 64 << 20 }, (e, out, err) => {
    if (e) fail(new Error(`ffprobe: ${String(err || e.message).slice(0, 300)}`));
    else ok(JSON.parse(out));
  }));
}

// Top-level ISO-BMFF boxes, walked by size.
function boxes(bytes, names) {
  const found = new Set();
  for (let at = 0; at + 8 <= bytes.length;) {
    const size = bytes.readUInt32BE(at);
    found.add(bytes.subarray(at + 4, at + 8).toString("latin1"));
    if (size < 8) break;
    at += size;
  }
  return names.filter((n) => found.has(n));
}

export async function signatureOf(base) {
  const variant = `${base}/v0/stream.m3u8`;
  const text = await (await fetch(variant)).text();
  const streams = (await probe(["-show_entries", "stream=codec_type,codec_name,profile,pix_fmt,r_frame_rate,time_base,sample_rate,channels"], variant)).streams;
  const pick = (kind, fields) => { const s = streams.find((x) => x.codec_type === kind) ?? {}; return Object.fromEntries(fields.map((f) => [f, s[f] ?? null])); };
  const packets = (await probe(["-select_streams", "v:0", "-show_entries", "packet=flags"], variant)).packets;
  const keys = packets.map((p, i) => [p.flags ?? "", i]).filter(([f]) => f.startsWith("K")).map(([, i]) => i);
  const gop = keys.slice(1).reduce((m, k, i) => Math.max(m, k - keys[i]), 0);

  const takes = [[]];
  for (const l of text.split("\n")) {
    if (l === "#EXT-X-DISCONTINUITY") takes.push([]);
    else if (l.startsWith("#EXTINF:")) takes.at(-1).push(Number(l.slice(8).replace(/,$/, "")));
  }
  const whole = takes.every((t) => t.slice(0, -1).every((d) => Math.abs(d - 6) < 0.05));
  const tag = (t) => text.split("\n").find((l) => l.startsWith(t))?.slice(t.length) ?? null;
  const pauses = takes.length - 1;
  const firstSeg = text.split("\n").find((l) => /^seg_\d{5}\.m4s$/.test(l));
  const init = Buffer.from(await (await fetch(`${base}/v0/init.mp4`)).arrayBuffer());
  const seg = Buffer.from(await (await fetch(`${base}/v0/${firstSeg}`)).arrayBuffer());
  return {
    video: pick("video", ["codec_name", "profile", "pix_fmt", "r_frame_rate", "time_base"]),
    audio: pick("audio", ["codec_name", "sample_rate", "channels", "time_base"]),
    gop_frames: gop,
    playlist: {
      version: tag("#EXT-X-VERSION:"),
      target_duration: tag("#EXT-X-TARGETDURATION:"),
      type: tag("#EXT-X-PLAYLIST-TYPE:"),
      maps: (text.match(/#EXT-X-MAP/g) ?? []).length,
      discontinuities_per_pause: pauses ? (text.match(/#EXT-X-DISCONTINUITY/g) ?? []).length / pauses : null,
      segments_whole_but_each_takes_last: whole,
      ends: text.trimEnd().endsWith("#EXT-X-ENDLIST"),
    },
    fmp4: { init: boxes(init, ["ftyp", "moov", "moof", "mdat"]), segment: boxes(seg, ["styp", "moof", "mdat"]) },
  };
}

// Field by field; a null in `got` (nothing to say — no pause) is skipped.
export function compare(got, want = JSON.parse(fs.readFileSync(GOLDEN, "utf8")), at = "") {
  const out = [];
  for (const [k, w] of Object.entries(want)) {
    const g = got?.[k];
    const where = at ? `${at}.${k}` : k;
    if (g === null) continue;
    if (w && typeof w === "object" && !Array.isArray(w)) out.push(...compare(g, w, where));
    else if (JSON.stringify(g) !== JSON.stringify(w)) out.push(`${where}: ${JSON.stringify(g)} (golden ${JSON.stringify(w)})`);
  }
  return out;
}
