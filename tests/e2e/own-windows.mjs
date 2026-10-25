// Which of shroom's own windows are in a take's picture (N82, DECISIONS B256): the take the
// own-windows drive made (`node app/test/take-drive.mjs own-windows`), read here by this Mac's
// own ffmpeg. The drive lifted the Library, an editor and Settings above the pattern in turn,
// then the popover, the agent label and the marker toast, and its report gives the stretch of
// the take each was up for. One frame from the middle of each stretch: the windows light the
// pattern's dark field (a take records them like any app's), the rest leave it dark (a take
// leaves them out). Each frame is also written as a PNG, for a person to look at.
//
//   node tests/e2e/own-windows.mjs <take-dir> <report.json> [<png-dir>]   one JSON line; exit 0 when so
//
// Bars: a window lights at least 5 % of the field (it covers most of the display); what is
// left out lights no more than a clean frame may (tests/e2e/left-out.mjs).

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { fieldLit } from "./field.mjs";
import { CLEAN, canRead } from "./left-out.mjs";

export const SEEN = 0.05;

const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p));

// One frame at `ms` into the take, in gray at 360 rows (and as a PNG at its own size when
// `png` is given); null when it can't be read.
function frameAt(playlist, ms, png) {
  const at = (ms / 1000).toFixed(3);
  const base = ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", playlist, "-ss", at, "-map", "0:v:0", "-frames:v", "1"];
  if (png) spawnSync(tool("ffmpeg"), [...base, "-y", png]);
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", playlist], { encoding: "utf8" });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  if (!w0 || !h0) return null;
  const h = 360, w = 2 * Math.round((h * w0) / h0 / 2);
  const r = spawnSync(tool("ffmpeg"), [...base, "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"], { maxBuffer: 1 << 26 });
  if (r.status !== 0 || r.stdout.length < w * h) return null;
  return { gray: r.stdout.subarray(0, w * h), w, h };
}

export function windowsIn(dir, stretches, pngDir) {
  const playlist = path.join(dir, "stream.m3u8");
  if (!fs.existsSync(playlist)) return { ok: false, error: `no take at ${dir}` };
  if (!Array.isArray(stretches) || stretches.length === 0) return { ok: false, error: "the report gives no stretches" };
  if (pngDir) fs.mkdirSync(pngDir, { recursive: true });
  const read = stretches.map(({ what, recorded, from_ms, to_ms }) => {
    const ms = Math.round((from_ms + to_ms) / 2);
    const png = pngDir ? path.join(pngDir, `${what.replace(/^the /, "").replace(/\W+/g, "-").toLowerCase()}.png`) : null;
    const f = frameAt(playlist, ms, png);
    const lit = f ? fieldLit(f.gray, f.w, f.h) : null;
    const ok = lit !== null && (recorded ? lit >= SEEN : lit <= CLEAN);
    return { what, recorded, atMs: ms, litPct: lit === null ? null : Number((lit * 100).toFixed(2)), ok, png };
  });
  return { ok: read.every((r) => r.ok), barSeenPct: SEEN * 100, barCleanPct: CLEAN * 100, read };
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
