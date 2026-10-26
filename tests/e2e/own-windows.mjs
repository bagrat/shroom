// Which of shroom's own windows are in a take's picture (N83, DECISIONS B264): the take the
// own-windows drive made (`node app/test/take-drive.mjs own-windows`), read here by this Mac's
// own ffmpeg. The drive lifted the Library, an editor and Settings above the pattern in turn,
// then the popover, the agent label and the marker toast, and its report gives the pattern's
// counter just before each went up and just after it went. Each stretch is found in the take's
// own frames by that counter (the take's clock and the drive's don't line up), and the frames
// in its middle are read: the windows light the pattern's dark field (a take records them like
// any app's), the rest leave it dark (a take leaves them out). The middle frame of each is also
// written as a PNG, for a person to look at.
//
//   node tests/e2e/own-windows.mjs <take-dir> <report.json> [<png-dir>]   one JSON line; exit 0 when so
//
// Bars: a window lights at least 5 % of the field (it covers most of the display); what is
// left out lights no more than a clean frame may (tests/e2e/left-out.mjs). A window covers the
// counter strip, so inside its stretch the counter reads nothing; the frames around it do.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { fieldLit } from "./field.mjs";
import { CLEAN, canRead } from "./left-out.mjs";
import { readStripGray } from "./strip.mjs";

export const SEEN = 0.05;
// How many frames either side of a stretch's middle are read with it.
const AROUND = 3;

const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p));

// Where a stretch is in the take's frames: after the last frame whose counter is at most
// `before`, and before the first after that whose counter is at least `after` — exclusive
// indices, or null when the take doesn't hold it.
export function findStretch(counters, before, after) {
  let start = -1;
  for (let i = 0; i < counters.length; i++) if (counters[i] !== null && counters[i] <= before) start = i;
  if (start < 0) return null;
  for (let i = start + 1; i < counters.length; i++) if (counters[i] !== null && counters[i] >= after) return i - start > 1 ? [start, i] : null;
  return null;
}

// Every frame of the take in gray at 360 rows: each one's counter (or null) and how much of the
// field it lights.
function frames(playlist) {
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", playlist], { encoding: "utf8" });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  if (!w0 || !h0) return { error: `the take's size couldn't be read: ${(size.stderr ?? "").trim().slice(0, 200)}` };
  const h = 360, w = 2 * Math.round((h * w0) / h0 / 2);
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", playlist, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"], { maxBuffer: 1 << 30 });
  if (r.status !== 0) return { error: `the take didn't decode: ${String(r.stderr ?? "").trim().slice(0, 300)}` };
  const counters = [], lit = [];
  for (let at = 0; at + w * h <= r.stdout.length; at += w * h) {
    const frame = r.stdout.subarray(at, at + w * h);
    counters.push(readStripGray(frame, w, h));
    lit.push(fieldLit(frame, w, h));
  }
  return { counters, lit };
}

// The take's frame number `n` as a PNG at its own size.
function png(playlist, n, file) {
  spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", playlist, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `select=eq(n\\,${n})`, "-frames:v", "1", "-y", file]);
}

export function windowsIn(dir, stretches, pngDir) {
  const playlist = path.join(dir, "stream.m3u8");
  if (!fs.existsSync(playlist)) return { ok: false, error: `no take at ${dir}` };
  if (!Array.isArray(stretches) || stretches.length === 0) return { ok: false, error: "the report gives no stretches" };
  const f = frames(playlist);
  if (f.error) return { ok: false, error: f.error };
  if (pngDir) fs.mkdirSync(pngDir, { recursive: true });
  const read = stretches.map(({ what, recorded, before, after }) => {
    const at = findStretch(f.counters, before, after);
    if (!at) return { what, recorded, before, after, found: false, ok: false };
    const mid = Math.floor((at[0] + at[1]) / 2);
    const from = Math.max(at[0] + 1, mid - AROUND), to = Math.min(at[1] - 1, mid + AROUND);
    const lits = f.lit.slice(from, to + 1);
    const ok = lits.every((l) => (recorded ? l >= SEEN : l <= CLEAN));
    const file = pngDir ? path.join(pngDir, `${what.replace(/^the /, "").replace(/\W+/g, "-").toLowerCase()}.png`) : null;
    if (file) png(playlist, mid, file);
    return { what, recorded, frames: at, read: [from, to], litPct: lits.map((l) => Number((l * 100).toFixed(2))), ok, png: file };
  });
  return { ok: read.every((r) => r.ok), frames: f.counters.length, barSeenPct: SEEN * 100, barCleanPct: CLEAN * 100, read };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [dir, report, pngs] = process.argv.slice(2);
  if (!dir || !report) { console.error("usage: node tests/e2e/own-windows.mjs <take-dir> <report.json> [<png-dir>]"); process.exit(64); }
  if (!canRead()) { console.error("no ffmpeg and ffprobe here to decode the take with"); process.exit(2); }
  const r = JSON.parse(fs.readFileSync(report, "utf8"));
  const p = windowsIn(path.resolve(dir), r.facts?.stretches, pngs && path.resolve(pngs));
  console.log(JSON.stringify(p));
  process.exit(p.ok ? 0 : 1);
}
