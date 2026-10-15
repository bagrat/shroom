// The pre-merge run's screen recording, captioned (DECISIONS B127): each step of
// tests/e2e/full.mjs (its chapters.json, wall clock) shown at the foot of the picture, so a
// person on a phone knows what they're watching. Writes video-captioned.mp4 beside it.
//
//   node dev/e2e/captions.mjs <scenario folder>
//
// Each caption is drawn as an image by Playwright's Chromium and laid over its stretch of the
// video by ffmpeg's overlay — Homebrew's ffmpeg has no text filter. The recording starts when
// mac.sh's watch starts ffmpeg, which is when the file is born; CAPTIONS_VIDEO_START_MS says
// otherwise (a copy whose birth isn't the recording's). A caption is for a person: if this
// fails, the uncaptioned video is still there.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const dir = process.argv[2];
const video = path.join(dir ?? "", "video.mp4");
const chaptersFile = path.join(dir ?? "", "chapters.json");
if (!dir || !fs.existsSync(video) || !fs.existsSync(chaptersFile)) {
  console.error("captions: needs <folder> with video.mp4 and chapters.json");
  process.exit(64);
}
const start = Number(process.env.CAPTIONS_VIDEO_START_MS) || fs.statSync(video).birthtimeMs;
const chapters = JSON.parse(fs.readFileSync(chaptersFile, "utf8"));
const probe = spawnSync("ffprobe", ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width", "-of", "csv=p=0", video], { encoding: "utf8" });
const width = Number(probe.stdout.trim()) || 1024;

// Each caption as a PNG: white words on a dark band, as wide as the picture.
const pngs = fs.mkdtempSync(path.join(dir, ".captions-"));
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width, height: 200 }, deviceScaleFactor: 1 });
  for (const [i, c] of chapters.entries()) {
    await page.setContent(`<body style="margin:0;background:transparent">
      <div id="c" style="box-sizing:border-box;width:${width}px;padding:10px 24px;background:rgba(0,0,0,.72);color:#fff;
        font:500 21px/1.35 -apple-system,'Helvetica Neue',Arial,sans-serif;text-align:center"></div></body>`);
    await page.$eval("#c", (el, text) => { el.textContent = text; }, c.text);
    await (await page.$("#c")).screenshot({ path: path.join(pngs, `${i}.png`), omitBackground: true });
  }
} finally {
  await browser.close();
}

// Every caption laid over the picture's foot for its own stretch of time.
const inputs = chapters.flatMap((_, i) => ["-i", path.join(pngs, `${i}.png`)]);
let last = "0:v";
const graph = chapters
  .map((c, i) => {
    const from = Math.max(0, (c.from - start) / 1000);
    const to = Math.max(from + 1, (c.to - start) / 1000);
    const out = `v${i}`;
    const step = `[${last}][${i + 1}:v]overlay=x=0:y=H-h-24:enable='between(t,${from.toFixed(2)},${to.toFixed(2)})'[${out}]`;
    last = out;
    return step;
  })
  .join(";");
const args = ["-hide_banner", "-nostdin", "-v", "error", "-y", "-i", video, ...inputs];
args.push(...(chapters.length ? ["-filter_complex", graph, "-map", `[${last}]`] : ["-map", "0:v"]));
args.push("-c:v", "libx264", "-preset", "veryfast", "-crf", "26", "-pix_fmt", "yuv420p", "-movflags", "+faststart", path.join(dir, "video-captioned.mp4"));
const r = spawnSync("ffmpeg", args, { encoding: "utf8" });
fs.rmSync(pngs, { recursive: true, force: true });
if (r.status !== 0) {
  console.error(`captions: ffmpeg said ${String(r.stderr).slice(0, 400)}`);
  process.exit(1);
}
console.log(`captions: ${chapters.length} laid into video-captioned.mp4`);
