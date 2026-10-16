// What one view costs in Worker requests: the real worker.js over loopback HTTP, a real
// recording encoded the way the app encodes it (1080p at Normal's 3,500 kbps, keyframes on
// the segment grid, fMP4 HLS plus a +faststart single-file copy), and real browsers —
// headless Chromium through the pinned hls.js (as the heartbeat forces it) and headless
// WebKit, or the Mac's real Safari with native HLS (the hosted Mac's `view-cost` scenario). Every request that reaches handle() is one the Workers Free plan
// counts (nothing caches in front of a Worker on its own route), so the tally here is the
// tally Cloudflare keeps. Never a visible browser, never the network, never a real space.
//
//   node dev/view-cost.mjs [--secs 300] [--seg 6,10] [--rate 8] [--mp4] [--only hls|mp4]
//                          [--engines chromium,webkit|safari]   (from worker/)
//
// Exits 1 when a view didn't play through, or the copy's view asked for a piece of the stream.
//
// Per engine and segment length: a bounce (the page opened, never played), a full watch
// (played muted to the end at --rate), and a watch with two seeks. --mp4 adds the same
// three with the single-file copy up, which the page then plays (DECISIONS B177); without
// it the page plays the stream.
import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { handle } from "../src/worker.js";
import { M1, metaKey, space } from "../test/space-fixture.js";

const here = path.dirname(new URL(import.meta.url).pathname);
const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const SECS = Number(arg("secs", "300"));
const SEGS = arg("seg", "6,10").split(",").map(Number);
const RATE = Number(arg("rate", "8"));
const MP4 = process.argv.includes("--mp4");
const ONLY = arg("only", "");
const ENGINES = arg("engines", "chromium,webkit").split(",");
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-view-cost-"));

/** A recording as the app uploads it, with `seg`-second segments. */
function encode(seg) {
  const dir = path.join(work, `seg${seg}`), v0 = path.join(dir, "v0");
  fs.mkdirSync(v0, { recursive: true });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1920x1080:rate=30", "-an", "-t", String(SECS),
    "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-b:v", "3500k", "-minrate", "3500k", "-maxrate", "3500k", "-bufsize", "7000k",
    "-x264-params", "nal-hrd=cbr", "-force_key_frames", `expr:gte(t,n_forced*${seg})`, "-sc_threshold", "0",
    "-f", "hls", "-hls_time", String(seg), "-hls_playlist_type", "vod", "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4",
    "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0 });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-i", path.join(v0, "stream.m3u8"), "-c", "copy", "-movflags", "+faststart", path.join(dir, "video.mp4")]);
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1200x675", "-frames:v", "1", path.join(dir, "poster.jpg")]);
  fs.writeFileSync(path.join(dir, "master.m3u8"), "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=3700000,RESOLUTION=1920x1080\nv0/stream.m3u8\n");
  const files = {};
  for (const f of ["master.m3u8", "poster.jpg", "video.mp4", ...fs.readdirSync(v0).map((n) => `v0/${n}`)]) files[f] = new Uint8Array(fs.readFileSync(path.join(dir, f)));
  return files;
}

const kind = (p) => p.endsWith(".m4s") ? "segment" : p.endsWith(".m3u8") ? "playlist" : p.endsWith("init.mp4") ? "init" : p.endsWith("video.mp4") ? "mp4"
  : p.endsWith("poster.jpg") ? "poster" : p.startsWith("/favicon") ? "icon" : p === "/_shroom/hls.js" ? "hls.js" : p.startsWith("/_shroom/status") ? "status" : "page";

async function main() {
  const env = space({});
  await env.BUCKET.put(".shroom/player/hls.min.js", new Uint8Array(fs.readFileSync(path.join(here, "../vendor/hls.min.js"))));
  const ids = {};
  for (const seg of SEGS) {
    const id = `Seg${String(seg).padStart(2, "0")}`.padEnd(16, "x");
    ids[seg] = id;
    for (const [f, bytes] of Object.entries(encode(seg))) await env.BUCKET.put(`rec/${id}/${f}`, bytes);
  }
  /** The record: live, with the single-file copy up or not yet. */
  const record = (id, mp4) => env.BUCKET.put(metaKey(id), JSON.stringify({ schema: 1, id, state: "live", createdAt: new Date(Date.now() - 60_000).toISOString(), title: "A view's cost",
    durationSec: SECS, media: { hls: `rec/${id}/master.m3u8`, mp4: mp4 ? `rec/${id}/video.mp4` : null, poster: `rec/${id}/poster.jpg`, width: 1920, height: 1080 }, owner: M1 }));

  let log = [];
  const server = createServer(async (req, res) => {
    const url = `http://127.0.0.1:${server.address().port}${req.url}`;
    log.push({ path: new URL(url).pathname, range: req.headers.range ?? null });
    const r = await handle(new Request(url, { method: req.method, headers: req.headers }), env, Date.now());
    res.writeHead(r.status, Object.fromEntries(r.headers));
    try { if (r.body) for await (const c of r.body) { if (!res.write(c)) await new Promise((ok) => res.once("drain", ok)); } } catch { /* the browser hung up */ }
    res.end();
  });
  server.on("clientError", () => {});
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const origin = `http://127.0.0.1:${server.address().port}`;

  const rows = [];
  for (const name of ENGINES) {
    const engine = await open(name);
    try {
      for (const seg of SEGS) for (const src of (MP4 && seg === SEGS[0] ? ["hls", "mp4"] : ["hls"]).filter((s) => !ONLY || s === ONLY)) {
        for (const scenario of ["bounce", "watch", "seeks"]) {
          await record(ids[seg], src === "mp4");
          const page = await engine.session();
          let played = true;
          log = [];
          try {
            await page.open(`${origin}/${ids[seg]}`);
            await sleep(scenario === "bounce" ? 15_000 : 1_000);
            if (scenario !== "bounce") {
              await page.js(`(()=>{const v=document.getElementById("v");v.muted=true;v.playbackRate=${RATE};v.play().catch(()=>{});return 1})()`);
              await until(page, `document.getElementById("v").currentTime>0.5`, 30_000);
              await page.js(`(()=>{document.getElementById("v").playbackRate=${RATE};return 1})()`);
              if (scenario === "seeks") {
                await until(page, `document.getElementById("v").currentTime>20`, 120_000);
                await page.js(`(()=>{document.getElementById("v").currentTime=${SECS * 0.6};return 1})()`);
                await until(page, `document.getElementById("v").currentTime>${SECS * 0.6 + 10}`, 120_000);
                await page.js(`(()=>{document.getElementById("v").currentTime=${SECS * 0.3};return 1})()`);
              }
              await until(page, `(v=>v.ended||v.currentTime>=v.duration-0.5)(document.getElementById("v"))`, (SECS / RATE) * 4000 + 60_000);
            }
          } catch (e) {
            played = false;
            console.error(`${name} ${src} ${scenario}: ${String(/** @type {any} */ (e)?.message ?? e).slice(0, 200)}`);
          } finally {
            await page.close();
          }
          const by = {};
          for (const l of log) by[kind(l.path)] = (by[kind(l.path)] ?? 0) + 1;
          // The copy's views ask for no piece of the stream; the stream's played views ask for pieces.
          const ok = played && (src === "mp4" ? by.mp4 > 0 && !by.segment && !by.playlist : scenario === "bounce" || by.segment > 0);
          if (!ok) process.exitCode = 1;
          rows.push({ engine: name, seg, src, scenario, ok, requests: log.length, by });
          console.error(JSON.stringify(rows.at(-1)));
        }
      }
    } finally {
      await engine.stop();
    }
  }
  server.close();
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify({ secs: SECS, rate: RATE, rows }, null, 1));
}

const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));

/** Waits until `expr` is true in the page. @param {{ js: (s: string) => Promise<any> }} page @param {string} expr @param {number} ms */
async function until(page, expr, ms) {
  const end = Date.now() + ms;
  while (!(await page.js(`!!(${expr})`))) {
    if (Date.now() > end) throw new Error(`timed out waiting for ${expr}: ${JSON.stringify(await page.js(`(()=>{const v=document.getElementById("v");return v&&{t:v.currentTime,d:v.duration,paused:v.paused,ended:v.ended,rs:v.readyState,ns:v.networkState,err:v.error&&v.error.code,rate:v.playbackRate}})()`))}`);
    await sleep(500);
  }
}

/** A browser that opens fresh sessions: Chromium through the pinned hls.js (as the heartbeat
 * forces it) and WebKit from Playwright, or the Mac's real Safari through safaridriver's
 * WebDriver (`sudo safaridriver --enable` once; native HLS, as people watch).
 * @param {string} name */
async function open(name) {
  if (name === "safari") {
    const port = 4445, base = `http://127.0.0.1:${port}`;
    const proc = spawn("safaridriver", ["-p", String(port)], { stdio: "ignore" });
    const wd = async (method, p, body) => {
      const r = await fetch(`${base}${p}`, { method, headers: { "content-type": "application/json" }, body: body ? JSON.stringify(body) : undefined });
      const j = await r.json();
      if (!r.ok) throw new Error(`safaridriver ${method} ${p}: ${JSON.stringify(j.value).slice(0, 200)}`);
      return j.value;
    };
    for (let i = 0; ; i++) {
      try { await wd("GET", "/status"); break; } catch (e) { if (i > 40) throw e; await sleep(250); }
    }
    return {
      async session() {
        const { sessionId } = await wd("POST", "/session", { capabilities: { alwaysMatch: { browserName: "safari" } } });
        return {
          open: (url) => wd("POST", `/session/${sessionId}/url`, { url }),
          js: (expr) => wd("POST", `/session/${sessionId}/execute/sync`, { script: `return ${expr}`, args: [] }),
          close: () => wd("DELETE", `/session/${sessionId}`).catch(() => {}),
        };
      },
      async stop() { proc.kill(); },
    };
  }
  const pw = await import("playwright");
  const browser = await pw[name].launch();
  return {
    async session() {
      const context = await browser.newContext();
      if (name === "chromium") await context.addInitScript(`const o = HTMLMediaElement.prototype.canPlayType; HTMLMediaElement.prototype.canPlayType = function (t) { return /mpegurl/i.test(t) ? "" : o.call(this, t); };`);
      const page = await context.newPage();
      return { open: (url) => page.goto(url), js: (expr) => page.evaluate(expr), close: () => context.close() };
    },
    stop: () => browser.close(),
  };
}

main().catch((e) => { console.error(e); process.exit(1); });
