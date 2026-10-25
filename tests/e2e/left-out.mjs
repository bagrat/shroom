// What is in a take's picture (DECISIONS B114): every frame of a take the left-out drive
// made (`node app/test/take-drive.mjs left-out`), decoded here by this Mac's own ffmpeg —
// not the one that made it — and held to one thing: nothing but the pattern. The drive
// lifted shroom's popover, its count, the agent label and the marker toast above the pattern
// and showed each while the take ran (a capture that leaves nothing out saw them there); a take that didn't leave one out
// has it over the pattern's dark field in those frames.
//
//   node tests/e2e/left-out.mjs <take-dir>     one JSON line; exit 0 when the picture is clean
//   node tests/e2e/left-out.mjs <take-dir> --the-count-stays-in
//       the reader's own proof, for a take of `take-drive.mjs not-left-out` (the count taken
//       off the list of what a take leaves out): exit 0 when the count IS found in it
//
// Bars: every frame's counter reads (the pattern is in every frame), and no frame has more
// than 0.2 % of the dark field lit. Over the pattern the count's scrim lights all of it,
// and the popover 2–4 % on the clean VM's 4:3 display (most of it hangs over the strips
// there; more on a wide one). The take is the pattern and nothing else; whoever ran the
// drive deletes it.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { fieldLit } from "./field.mjs";
import { readStripGray } from "./strip.mjs";

// The most of the dark field a clean frame may have lit.
export const CLEAN = 0.002;
// The least of it the count's scrim lights: it covers the display (its digit is dark).
export const SCRIM = 0.9;

const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p));

// Whether this Mac can decode a take to raw frames: a whole ffmpeg, and ffprobe beside it.
// (The app's own bundled encoder can't: it has no raw-video writer.)
export function canRead() {
  return Boolean(tool("ffmpeg") && tool("ffprobe"));
}

export function pictureOf(dir) {
  const playlist = path.join(dir, "stream.m3u8");
  if (!fs.existsSync(playlist)) return { ok: false, error: `no take at ${dir}` };
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", playlist], { encoding: "utf8" });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  if (!w0 || !h0) return { ok: false, error: `the take's size couldn't be read: ${(size.stderr ?? "").trim().slice(0, 200)}` };
  const h = 360, w = 2 * Math.round((h * w0) / h0 / 2);
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", playlist, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"], { maxBuffer: 1 << 30 });
  if (r.status !== 0) return { ok: false, error: `the take didn't decode: ${String(r.stderr ?? "").trim().slice(0, 300)}` };
  let frames = 0, unreadable = 0, most = 0, underAScrim = 0;
  const lit = [];
  for (let at = 0; at + w * h <= r.stdout.length; at += w * h) {
    const frame = r.stdout.subarray(at, at + w * h);
    if (readStripGray(frame, w, h) === null) unreadable++;
    const f = fieldLit(frame, w, h);
    most = Math.max(most, f);
    if (f > CLEAN) lit.push({ frame: frames, litPct: Number((f * 100).toFixed(2)) });
    if (f >= SCRIM) underAScrim++;
    frames++;
  }
  return {
    ok: frames > 0 && unreadable === 0 && lit.length === 0,
    size: [w0, h0], frames, unreadable, mostLitPct: Number((most * 100).toFixed(3)), barPct: CLEAN * 100,
    framesWithSomethingOverThePattern: lit.length, framesUnderTheCount: underAScrim, first: lit.slice(0, 8),
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) { console.error("usage: node tests/e2e/left-out.mjs <take-dir> [--the-count-stays-in]"); process.exit(64); }
  if (!canRead()) { console.error("no ffmpeg and ffprobe here to decode the take with"); process.exit(2); }
  const p = pictureOf(path.resolve(process.argv[2]));
  console.log(JSON.stringify(p));
  process.exit((process.argv.includes("--the-count-stays-in") ? p.framesUnderTheCount > 0 : p.ok) ? 0 : 1);
}
