// The play counter on a real Cloudflare account (POC, history/proposals/cleanup-suggestions.md):
// this branch's worker.js deployed as its own Worker over its own bucket, both named
// `shroom-poc-views`, a short recording put in it, then played in headless Chromium and
// WebKit through the workers.dev link; the plays record and POST /_shroom/api/cleanup are
// read back. Everything it made is torn down at the end (--keep leaves it up).
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node dev/views-live.mjs [--keep]   (from worker/)
//
// Only ever the dedicated test account (TESTING-PLAN D3): it refuses an account whose name
// isn't the test account's, and touches nothing it didn't name. The token is read from the
// environment and sent only in a header.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { chromium, webkit } from "playwright";
import { M1, OWNER_MAC, cookieFor, roster } from "../test/space-fixture.js";

const TOKEN = process.env.SHROOM_E2E_TOKEN, ACCOUNT = process.env.SHROOM_E2E_ACCOUNT_ID;
if (!TOKEN || !ACCOUNT) { console.error("needs SHROOM_E2E_TOKEN and SHROOM_E2E_ACCOUNT_ID"); process.exit(2); }
const NAME = "shroom-poc-views";
const KEEP = process.argv.includes("--keep");
const ID = "LiveViewsVideo00";
const API = `https://api.cloudflare.com/client/v4/accounts/${ACCOUNT}`;
const here = path.dirname(new URL(import.meta.url).pathname);

async function cf(method, p, body, headers = {}) {
  const r = await fetch(`${API}${p}`, { method, body, headers: { authorization: `Bearer ${TOKEN}`, ...headers } });
  const text = await r.text();
  let json = null;
  try { json = JSON.parse(text); } catch { /* an object's body */ }
  if (!r.ok && !(method === "DELETE" && r.status === 404)) throw new Error(`${method} ${p}: ${r.status} ${JSON.stringify(json?.errors ?? text.slice(0, 200))}`);
  return json ?? text;
}
/** A bucket made a moment after one of its name was deleted answers 10073 for a while: wait it out. */
async function putObject(key, bytes, type) {
  for (let i = 0; ; i++) {
    try { return await cf("PUT", `/r2/buckets/${NAME}/objects/${key.split("/").map(encodeURIComponent).join("/")}`, bytes, { "content-type": type }); }
    catch (e) { if (i >= 20 || !/10073/.test(String(e))) throw e; await new Promise((ok) => setTimeout(ok, 3000)); }
  }
}
const getObject = async (key) => { try { const t = await cf("GET", `/r2/buckets/${NAME}/objects/${key.split("/").map(encodeURIComponent).join("/")}`); return typeof t === "string" ? JSON.parse(t) : t; } catch (e) { console.error(String(e)); return null; } };

const account = await cf("GET", "");
if (!/^test@shroom\.am/i.test(account.result.name)) { console.error(`refusing: not the test account (${account.result.name})`); process.exit(2); }
console.error(`account: ${account.result.name}`);

const made = [];
try {
  // The bucket and the Worker, both ours by name.
  await cf("POST", "/r2/buckets", JSON.stringify({ name: NAME }), { "content-type": "application/json" }).catch((e) => { if (!/already exists|10004/.test(String(e))) throw e; });
  made.push(`bucket ${NAME}`);
  const form = new FormData();
  form.append("metadata", new Blob([JSON.stringify({ main_module: "worker.js", compatibility_date: "2026-09-01", bindings: [{ type: "r2_bucket", name: "BUCKET", bucket_name: NAME }] })], { type: "application/json" }));
  form.append("worker.js", new Blob([fs.readFileSync(path.join(here, "../src/worker.js"))], { type: "application/javascript+module" }), "worker.js");
  await cf("PUT", `/workers/scripts/${NAME}`, form);
  made.push(`Worker ${NAME}`);
  await cf("POST", `/workers/scripts/${NAME}/subdomain`, JSON.stringify({ enabled: true }), { "content-type": "application/json" });
  const sub = (await cf("GET", "/workers/subdomain")).result.subdomain;
  const origin = `https://${NAME}.${sub}.workers.dev`;
  console.error(`page: ${origin}`);

  // The space as wiring leaves it, and one recording.
  const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-views-live-"));
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30", "-an", "-t", "12", "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-movflags", "+faststart", path.join(work, "video.mp4")]);
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-i", path.join(work, "video.mp4"), "-c", "copy", "-f", "hls", "-hls_time", "6", "-hls_playlist_type", "vod", "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4", "-hls_segment_filename", path.join(work, "seg_%05d.m4s"), path.join(work, "stream.m3u8")]);
  await putObject(".shroom/cookie.key", Buffer.alloc(32, 7).toString("base64url"), "text/plain");
  await putObject(".shroom/registry/roster.json", JSON.stringify(roster()), "application/json");
  for (const f of fs.readdirSync(work).filter((f) => f !== "video.mp4")) await putObject(`rec/${ID}/v0/${f}`, fs.readFileSync(path.join(work, f)), f.endsWith(".m3u8") ? "application/vnd.apple.mpegurl" : "video/mp4");
  await putObject(`rec/${ID}/video.mp4`, fs.readFileSync(path.join(work, "video.mp4")), "video/mp4");
  await putObject(`rec/${ID}/master.m3u8`, "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\nv0/stream.m3u8\n", "application/vnd.apple.mpegurl");
  await putObject(`meta/${ID}.json`, JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: new Date(Date.now() - 40 * 86_400_000).toISOString(), title: "Counting plays, live",
    durationSec: 12, media: { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, poster: null, width: 640, height: 360 }, owner: M1 }), "application/json");
  fs.rmSync(work, { recursive: true, force: true });
  for (let i = 0; i < 30; i++) { const r = await fetch(`${origin}/_shroom/ping`).catch(() => null); if (r?.ok) break; await new Promise((ok) => setTimeout(ok, 2000)); }

  // Two viewers play it through; the owner plays it too.
  for (const engine of [chromium, webkit]) {
    const b = await engine.launch();
    const page = await b.newPage();
    await page.goto(`${origin}/${ID}`);
    await page.evaluate(`(()=>{const v=document.getElementById("v");v.muted=true;v.playbackRate=2;v.play();return 1})()`);
    await page.waitForFunction(`document.getElementById("v").ended`, null, { timeout: 60_000 });
    await page.goto("about:blank");
    await new Promise((ok) => setTimeout(ok, 1500));
    await b.close();
  }
  const b = await chromium.launch();
  const ctx = await b.newContext();
  const [cname, cvalue] = cookieFor(M1, OWNER_MAC).split(/=(.*)/s);
  await ctx.addCookies([{ name: cname, value: cvalue, url: origin }]);
  const page = await ctx.newPage();
  await page.goto(`${origin}/${ID}`);
  const ownerPage = (await page.content()).includes("shroomCount(");
  await b.close();

  await new Promise((ok) => setTimeout(ok, 2000));
  const rec = await getObject(`.shroom/views/${ID}.json`);
  const cleanup = await (await fetch(`${origin}/_shroom/api/cleanup`, { method: "POST", headers: { authorization: OWNER_MAC.bearer, "content-type": "application/json" }, body: JSON.stringify({ scope: "everyone" }) })).json();
  const ok = rec?.plays === 2 && rec?.ends === 2 && rec.watch[9] === 2 && !ownerPage && cleanup?.items?.[0]?.plays === 2;
  console.log(JSON.stringify({ ok, record: rec && { ...rec, today: { d: rec.today.d, viewers: rec.today.h.length } }, ownerPageCarriesBeacon: ownerPage, cleanup: cleanup.items?.map(({ id, plays, viewers, medianWatched, candidate, reason }) => ({ id, plays, viewers, medianWatched, candidate, reason: reason.code })) }, null, 1));
  if (!ok) process.exitCode = 1;
} finally {
  if (KEEP) console.error(`kept: ${made.join(", ")}`);
  else {
    // Ours only: the Worker, then every object in our bucket, then the bucket.
    await cf("DELETE", `/workers/scripts/${NAME}`).catch((e) => console.error(String(e)));
    for (;;) {
      const list = await cf("GET", `/r2/buckets/${NAME}/objects?per_page=1000`).catch(() => null);
      const keys = (list?.result ?? []).map((o) => o.key);
      if (!keys.length) break;
      for (const k of keys) await cf("DELETE", `/r2/buckets/${NAME}/objects/${encodeURIComponent(k)}`);
    }
    await cf("DELETE", `/r2/buckets/${NAME}`).catch((e) => console.error(String(e)));
    console.error(`torn down: ${made.join(", ")}`);
  }
}
