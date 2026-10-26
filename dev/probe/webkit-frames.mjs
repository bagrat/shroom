// PROBE (never merged): WebKit's measured play, as the heartbeat makes it, under load on a
// hosted Mac. The real page (worker.js's handle over loopback, the unit tests' fake bucket),
// a counter-strip take with sound encoded here, and tests/e2e/frames.mjs's sampler with the
// re-arm off and on, plays interleaved.
//
//   node dev/probe/webkit-frames.mjs <plays per variant> <cpu hogs>
//
// Prints a JSON line per play and a summary per variant.
import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createServer } from "node:http";
import { webkit } from "playwright";
import { handle } from "../../worker/src/worker.js";
import { M1, metaKey, space } from "../../worker/test/space-fixture.js";
import { crc8, MASK, readStrip } from "../../tests/e2e/strip.mjs";
import { frameSampler } from "../../tests/e2e/frames.mjs";

const plays = Number(process.argv[2] ?? 4), hogs = Number(process.argv[3] ?? 0);
if (hogs > 0 && process.env.GITHUB_ACTIONS !== "true") { console.error("load only on a runner"); process.exit(64); }
const here = path.dirname(new URL(import.meta.url).pathname);
const work = fs.mkdtempSync(path.join(os.tmpdir(), "webkit-frames-"));
const W = 1024, H = 768, FPS = 30, SECONDS = 11, FIRST = 300;

function frame(n) {
  const f = Buffer.alloc(W * H);
  for (let y = 0; y < H; y++) f.fill((y + n * 4) & 0xff, y * W, (y + 1) * W);
  const cell = Math.floor(H / 12), word = (((n << 8) | crc8(n)) ^ MASK) >>> 0;
  for (let bit = 0; bit < 32; bit++) {
    const on = (word >>> (31 - bit)) & 1, r = Math.floor(bit / 8), c = bit % 8;
    for (let y = cell + r * cell; y < cell + (r + 1) * cell; y++) f.fill(on ? 255 : 0, y * W + cell + c * cell, y * W + cell + (c + 1) * cell);
  }
  return f;
}
// The strip reads back from what's drawn here.
{ const f = frame(1234), rgba = new Uint8ClampedArray(W * H * 4); for (let i = 0; i < W * H; i++) rgba.set([f[i], f[i], f[i], 255], i * 4);
  if (readStrip(rgba, W, H) !== 1234) throw new Error("the strip doesn't read back"); }

async function encode() {
  const dir = path.join(work, "rec"), v0 = path.join(dir, "v0");
  fs.mkdirSync(v0, { recursive: true });
  const ff = spawn("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "gray", "-s", `${W}x${H}`, "-r", String(FPS), "-i", "pipe:0",
    "-f", "lavfi", "-i", `sine=frequency=1000:beep_factor=4:duration=${SECONDS}`, "-shortest",
    "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", String(FPS), "-c:a", "aac", "-b:a", "96k",
    "-f", "hls", "-hls_time", "2", "-hls_playlist_type", "vod", "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4",
    "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0, stdio: ["pipe", "inherit", "inherit"] });
  const done = new Promise((ok, no) => ff.once("exit", (c) => (c === 0 ? ok() : no(new Error(`ffmpeg ${c}`)))));
  for (let i = 0; i < SECONDS * FPS; i++) if (!ff.stdin.write(frame(FIRST + i))) await new Promise((r) => ff.stdin.once("drain", r));
  ff.stdin.end();
  await done;
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "gray", "-s", `${W}x${H}`, "-i", "pipe:0", "-frames:v", "1", path.join(dir, "poster.jpg")], { input: frame(FIRST) });
  fs.writeFileSync(path.join(dir, "master.m3u8"), `#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=${W}x${H},CODECS="avc1.64001f,mp4a.40.2"\nv0/stream.m3u8\n`);
  return Object.fromEntries(["master.m3u8", "poster.jpg", ...fs.readdirSync(v0).map((n) => `v0/${n}`)].map((f) => [f, new Uint8Array(fs.readFileSync(path.join(dir, f)))]));
}

const ID = "ProbProbProbProb";
const env = space();
await env.BUCKET.put(".shroom/player/hls.min.js", new Uint8Array(fs.readFileSync(path.join(here, "../../worker/vendor/hls.min.js"))));
for (const [f, bytes] of Object.entries(await encode())) await env.BUCKET.put(`rec/${ID}/${f}`, bytes);
await env.BUCKET.put(metaKey(ID), JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: new Date(Date.now() - 60_000).toISOString(), title: "Probe", durationSec: SECONDS,
  media: { hls: `rec/${ID}/master.m3u8`, mp4: null, poster: `rec/${ID}/poster.jpg`, width: W, height: H }, owner: M1 }));
const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  const r = await handle(new Request(`http://127.0.0.1:${server.address().port}${req.url}`, { method: req.method, headers: req.headers, body: chunks.length ? Buffer.concat(chunks) : undefined }), env, Date.now());
  res.writeHead(r.status, Object.fromEntries(r.headers));
  if (r.body) for await (const c of r.body) res.write(c);
  res.end();
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/${ID}`;

const load = [];
for (let i = 0; i < hogs; i++) load.push(spawn("sh", ["-c", "while :; do :; done"], { stdio: "ignore" }));
// The GPU and the media engine busy too, as OBS's rendering and its virtual camera keep them.
if (hogs && process.env.PROBE_VT) for (let i = 0; i < 2; i++) load.push(spawn("sh", ["-c", "while :; do ffmpeg -hide_banner -loglevel quiet -nostdin -f lavfi -i testsrc2=size=1920x1080:rate=60 -t 60 -c:v h264_videotoolbox -b:v 20M -f null -; done"], { stdio: "ignore" }));
await new Promise((r) => setTimeout(r, hogs ? 20_000 : 0));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = { off: [], on: [] };
for (let i = 0; i < plays; i++) {
  for (const variant of ["off", "on"]) {
    const browser = await webkit.launch();
    const page = await browser.newPage();
    await page.addInitScript(frameSampler({ rearm: variant === "on" }));
    // What the element went through, for a play whose callbacks never came.
    await page.addInitScript(`window.__trace = []; document.addEventListener("DOMContentLoaded", () => { const v = document.getElementById("v"); if (!v) return;
      const t0 = performance.now(); for (const e of ["loadstart", "loadedmetadata", "loadeddata", "canplay", "playing", "waiting", "stalled", "ended"])
        v.addEventListener(e, () => __trace.push([e, Math.round(performance.now() - t0), v.readyState, Number(v.currentTime.toFixed(2))])); });`);
    const t0 = Date.now();
    await page.goto(url);
    let s = {};
    while (Date.now() - t0 < 90_000) {
      s = await page.evaluate(() => ({ ended: !!window.__ended, errors: window.__errors }));
      if (s.ended || s.errors.length) break;
      await sleep(500);
    }
    const [counters, unread, undrawnFirst, rearmed, trace, t, el] = await page.evaluate(() => { const v = document.getElementById("v");
      return [window.__samples, window.__unread, !!window.__undrawnFirst, window.__rearmed, window.__trace, window.__t,
        v && { currentTime: Number(v.currentTime.toFixed(2)), paused: v.paused, readyState: v.readyState, networkState: v.networkState, played: v.played.length ? Number(v.played.end(v.played.length - 1).toFixed(2)) : 0 }]; });
    let backwards = 0, prev = null;
    for (const c of counters) { if (c === null) continue; if (prev !== null && c < prev) backwards++; prev = c; }
    const presented = counters.length, readable = counters.filter((c) => c !== null).length;
    const ok = s.ended && s.errors.length === 0 && backwards === 0 && readable === presented && presented > 0;
    const row = { play: i, variant, ok, ended: s.ended, presented, readable, backwards, rearmed, undrawnFirst, mediaTime: t, first: counters.find((c) => c !== null), last: counters.findLast((c) => c !== null), errors: s.errors, unread, load: os.loadavg().map((l) => l.toFixed(1)).join(" "), el, trace: presented > 5 && s.ended ? undefined : trace, wallMs: Date.now() - t0 };
    results[variant].push(row);
    console.log(JSON.stringify(row));
    await browser.close();
  }
}
for (const p of load) p.kill("SIGKILL");
spawn("pkill", ["-f", "h264_videotoolbox"]);
server.close();
const sum = (rows) => ({ plays: rows.length, failed: rows.filter((r) => !r.ok).length, presentedZero: rows.filter((r) => r.presented === 0).length, rearmedPlays: rows.filter((r) => r.rearmed > 0).length,
  presented: rows.map((r) => r.presented).join(",") });
console.log(JSON.stringify({ summary: { hogs, off: sum(results.off), on: sum(results.on) } }));
