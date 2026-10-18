// The play counter in real browsers (POC, history/proposals/cleanup-suggestions.md): the real
// worker.js over loopback HTTP, a short recording encoded as the app encodes it, and
// headless Chromium and WebKit. Per engine: a bounce (opened, never played — no play), a
// half watch then leaving the page (a play, ended near half), and a full watch then
// leaving (a play, ended at the end). Then the owner's own page, played through: no play.
// Never a visible browser, never the network, never a real space.
//
//   node dev/views-drive.mjs [--secs 30] [--engines chromium,webkit]   (from worker/)
//
// Prints the video's plays record after each engine; exits 1 when a count is off.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { chromium, webkit } from "playwright";
import { handle } from "../src/worker.js";
import { M1, OWNER_MAC, cookieFor, ledger, metaKey, space } from "../test/space-fixture.js";

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? process.argv[i + 1] : d; };
const SECS = Number(arg("secs", "30"));
const ENGINES = arg("engines", "chromium,webkit").split(",");
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-views-"));
const ID = "ViewsDriveVideo0";

const v0 = path.join(work, "v0");
fs.mkdirSync(v0);
execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=30", "-an", "-t", String(SECS),
  "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-force_key_frames", "expr:gte(t,n_forced*6)",
  "-f", "hls", "-hls_time", "6", "-hls_playlist_type", "vod", "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4",
  "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0 });
execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-i", path.join(v0, "stream.m3u8"), "-c", "copy", "-movflags", "+faststart", path.join(work, "video.mp4")]);

const env = space({});
for (const f of fs.readdirSync(v0)) await env.BUCKET.put(`rec/${ID}/v0/${f}`, new Uint8Array(fs.readFileSync(path.join(v0, f))));
await env.BUCKET.put(`rec/${ID}/master.m3u8`, "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1280x720\nv0/stream.m3u8\n");
await env.BUCKET.put(`rec/${ID}/video.mp4`, new Uint8Array(fs.readFileSync(path.join(work, "video.mp4"))));
await env.BUCKET.put(metaKey(ID), JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: new Date(Date.now() - 86_400_000).toISOString(), title: "Counting plays",
  durationSec: SECS, media: { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, poster: null, width: 1280, height: 720 }, owner: M1 }));
await ledger(env, ID, M1, OWNER_MAC.id);

const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  const url = `http://127.0.0.1:${server.address().port}${req.url}`;
  const init = { method: req.method, headers: req.headers };
  if (chunks.length) init.body = Buffer.concat(chunks);
  const r = await handle(new Request(url, init), env, Date.now());
  res.writeHead(r.status, Object.fromEntries(r.headers));
  try { if (r.body) for await (const c of r.body) res.write(c); } catch { /* the browser hung up */ }
  res.end();
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const origin = `http://127.0.0.1:${server.address().port}`;
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const record = () => env.BUCKET.json(`.shroom/views/${ID}.json`);

/** @param {import("playwright").Page} page @param {string} expr */
async function until(page, expr, ms = 60_000) {
  const end = Date.now() + ms;
  while (!(await page.evaluate(`!!(${expr})`))) { if (Date.now() > end) throw new Error(`timed out: ${expr}`); await sleep(200); }
}

let failed = false;
const expect = (what, ok) => { console.error(`${ok ? "ok  " : "FAIL"} ${what}`); if (!ok) failed = true; };
const before = () => record() ?? { plays: 0, ends: 0, watch: Array(10).fill(0) };

for (const name of ENGINES) {
  const browser = await (name === "webkit" ? webkit : chromium).launch({ executablePath: name === "chromium" ? process.env.CHROMIUM_PATH : undefined });
  try {
    // A bounce: the page opened, never played.
    let was = before();
    let page = await browser.newPage();
    await page.goto(`${origin}/${ID}`);
    await sleep(1500);
    await page.goto("about:blank");
    await sleep(500);
    expect(`${name}: a bounce isn't a play`, (record()?.plays ?? 0) === was.plays);

    for (const [label, upTo, tenth] of [["half", 0.5, [4, 5]], ["full", 1, [9]]]) {
      was = before();
      await page.goto(`${origin}/${ID}`);
      await page.evaluate(`(()=>{const v=document.getElementById("v");v.muted=true;v.playbackRate=4;v.play();return 1})()`);
      await until(page, upTo === 1 ? `document.getElementById("v").ended` : `document.getElementById("v").currentTime>=${SECS * upTo}`);
      await page.evaluate(`document.getElementById("v").pause()`);
      await page.goto("about:blank"); // pagehide: the end beacon
      await sleep(800);
      const r = record();
      const landed = r && r.watch.findIndex((n, i) => n > (was.watch[i] ?? 0));
      expect(`${name}: a ${label} watch is one play`, r?.plays === was.plays + 1);
      expect(`${name}: and its end lands in tenth ${tenth.join(" or ")} (got ${landed})`, r?.ends === was.ends + 1 && tenth.includes(landed));
    }

    // The owner's own page: played through, never counted.
    was = before();
    const ctx = await browser.newContext();
    const [cname, cvalue] = cookieFor(M1, OWNER_MAC).split(/=(.*)/s);
    await ctx.addCookies([{ name: cname, value: cvalue, url: origin }]);
    page = await ctx.newPage();
    await page.goto(`${origin}/${ID}`);
    await page.evaluate(`(()=>{const v=document.getElementById("v");v.muted=true;v.playbackRate=4;v.play();return 1})()`);
    await until(page, `document.getElementById("v").ended`);
    await page.goto("about:blank");
    await sleep(800);
    expect(`${name}: the owner's own watch isn't a play`, (record()?.plays ?? 0) === was.plays);
    await ctx.close();
    console.log(JSON.stringify({ engine: name, record: { ...record(), today: undefined } }));
  } finally {
    await browser.close();
  }
}
server.close();
fs.rmSync(work, { recursive: true, force: true });
process.exit(failed ? 1 : 0);
