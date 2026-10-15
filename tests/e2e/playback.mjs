// Plays a recording in real browser engines and reads the counter strip from every
// frame the player actually presents (requestVideoFrameCallback → canvas → strip).
//
//   node tests/e2e/playback.mjs <recording dir> [webkit|chromium ...]
//
// Native HLS where the engine offers it (WebKit; Chromium too since 2025), hls.js otherwise
// (Firefox) — "<engine>+hlsjs" forces hls.js to keep that path covered.
// Prints one JSON line per engine: frames presented, readable, out-of-order steps, the
// largest jump (the pause), media duration reached, errors.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { webkit, chromium } from "playwright";
import { readStrip, crc8 } from "./strip.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(process.argv[2] ?? ".");
// Engines: webkit, chromium; a "+hlsjs" suffix forces hls.js even where HLS is native.
const engines = process.argv.slice(3).length ? process.argv.slice(3) : ["webkit", "chromium", "chromium+hlsjs"];
const hlsJs = path.resolve(here, "../../node_modules/hls.js/dist/hls.min.js");

const TYPES = { ".m3u8": "application/vnd.apple.mpegurl", ".m4s": "video/iso.segment", ".mp4": "video/mp4", ".js": "text/javascript", ".html": "text/html" };
const PAGE = `<!doctype html><meta charset="utf-8"><body style="margin:0;background:#000">
<video id="v" muted playsinline style="width:640px"></video><canvas id="c" style="display:none"></canvas>
<script src="/hls.min.js"></script>
<script>${crc8.toString()}
${readStrip.toString()}</script>
<script>
  const v = document.getElementById("v"), c = document.getElementById("c");
  window.samples = []; window.errors = []; window.done = false; window.mode = "";
  v.addEventListener("error", () => errors.push("media error " + (v.error && v.error.code)));
  v.addEventListener("ended", () => { window.done = true; });
  const src = "/stream.m3u8";
  const forceHlsJs = new URLSearchParams(location.search).has("hlsjs");
  if (!forceHlsJs && v.canPlayType("application/vnd.apple.mpegurl")) { mode = "native"; v.src = src; }
  else { mode = "hls.js"; const h = new Hls({ enableWorker: false }); h.on(Hls.Events.ERROR, (_, d) => { if (d.fatal) errors.push("hls " + d.details); }); h.loadSource(src); h.attachMedia(v); }
  const ctx = c.getContext("2d", { willReadFrequently: true });
  // Read the strip in the page from a small canvas (cells are a twelfth of the height,
  // so 270 rows still gives 22-pixel cells); only counters leave the page.
  const onFrame = (_, meta) => {
    const h = 270, w = Math.round((v.videoWidth / v.videoHeight) * h) || 480;
    if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
    ctx.drawImage(v, 0, 0, w, h);
    samples.push({ t: meta.mediaTime, c: readStrip(ctx.getImageData(0, 0, w, h).data, w, h) });
    v.requestVideoFrameCallback(onFrame);
  };
  v.requestVideoFrameCallback(onFrame);
  v.playbackRate = 2;
  v.play().catch(e => errors.push("play: " + e.message));
</script>`;

const server = http.createServer((req, res) => {
  const url = decodeURIComponent(new URL(req.url, "http://x").pathname);
  if (url === "/") { res.writeHead(200, { "content-type": "text/html" }); return res.end(PAGE); }
  const file = url === "/hls.min.js" ? hlsJs : path.join(root, url);
  if (!file.startsWith(root) && file !== hlsJs) { res.writeHead(403); return res.end(); }
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { "content-type": TYPES[path.extname(file)] ?? "application/octet-stream" });
    res.end(data);
  });
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}/`;

for (const name of engines) {
  const [engine, flag] = name.split("+");
  const browser = await { webkit, chromium }[engine].launch();
  const page = await browser.newPage();
  const counters = [];
  let lastT = 0;
  await page.goto(flag === "hlsjs" ? base + "?hlsjs=1" : base);
  const started = Date.now();
  while (Date.now() - started < 60_000) {
    const batch = await page.evaluate(() => { const s = window.samples; window.samples = []; return { s, done: window.done, errors: window.errors, mode: window.mode }; });
    for (const f of batch.s) { counters.push(f.c); lastT = Math.max(lastT, f.t); }
    if (batch.done || batch.errors.length) break;
    await new Promise((r) => setTimeout(r, 200));
  }
  const final = await page.evaluate(() => ({ errors: window.errors, mode: window.mode, duration: document.getElementById("v").duration, done: window.done }));
  let backwards = 0, maxJump = 0, prev = null;
  for (const c of counters) {
    if (c === null) continue;
    if (prev !== null) { if (c < prev) backwards++; maxJump = Math.max(maxJump, c - prev); }
    prev = c;
  }
  console.log(JSON.stringify({ engine: name, mode: final.mode, ended: final.done, duration: final.duration, mediaTimeReached: Number(lastT.toFixed(3)),
    presented: counters.length, readable: counters.filter((c) => c !== null).length, backwards, maxJump, errors: final.errors }));
  await browser.close();
}
server.close();
