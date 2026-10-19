// The heartbeat (TESTING-PLAN §5), through the product path: record (the test pattern) →
// Stop → the page is live → a recipient's browsers play it → the title set via the owner
// cookie → a member joins, records and is removed (B68) → delete through the app's Library
// → the removed face. On the dedicated test account (D3), swept before and after.
//
//   node tests/e2e/heartbeat.mjs [seconds=10] [--e2e]
//
// With --e2e (the pre-merge e2e run, TESTING-PLAN D3): on the dedicated test account, in
// the space the app's own wizard wired there (~/.shroom-dev/space.json) — no dev space is
// brought up or taken down (the run's close sweeps the account), and every app instance
// starts with --e2e, its client pinned to that account. SHROOM_INSTALLED names the
// installed dev app (default ~/Applications/shroom-dev.app). E2E_PAGE_VIDEO_DIR, when set,
// keeps what the browsers saw for a person to watch: the page playing in WebKit
// (`play-webkit.webm` — a pass of its own after the measured plays, which a recording
// would disturb), the owner titling it (`owner.webm`) and the removed face (`removed.png`).
// They are headless, so the screen's own recording never shows them.
//
// Preflight (Oracle 2, TESTING-PLAN §3): ffmpeg and ffprobe are here; the installed dev app
// (~/Applications/shroom-dev.app) verifies, its designated requirement is its identifier +
// certificate leaf (never a per-build cdhash — E1: a rebuild keeps the grants), and it is the
// fresh build (target/debug/bundle/macos/shroom-dev.app, same requirement, same binary).
//
// Media truth (Oracle 3, TESTING-PLAN §4), read from what was published: every frame's
// counter decoded back from the page's own stream (repeated and skipped frames, each ≤ 0.5 %
// — the plan's bar is over a 10-minute take: `heartbeat.mjs 600`); A/V offset, each beep
// against where the published picture shows that second (≤ 40 ms; no drift ≤ 15 ms on a
// take of 30 s or more) — the pattern's counter and the drive's beep share the wall clock
// (DECISIONS B59 d); the recipe's signature against tests/e2e/golden/hls-signature.json; the
// beep's 1 Hz cadence (b4 #17: the pattern's sound, never the room); sound in WebKit's
// player. From the drive's own report: Stop → the keyed page's first 200 ≤ 1 s (the app's
// own key, minted ahead while the take ran — B59 g), the encoder's finalize at the pause and
// Stop's finalize ≤ 200 ms each, capture ≤ ~1 core.
//
// Needs the dev app installed with its grants — built with the harness (`npm run build:dev`
// in app/, i.e. `--features dev-harness`) — and target/debug/shroom-dev built. The drive
// records only the test pattern: the display it covers (checked on the Scene before
// Return) and its 1 Hz beep in place of the microphone. The owner's one-time key reaches
// this script through a mode-600 file in its own throwaway folder — never argv, never
// printed.
//
// Prints one JSON summary. Exit 0: everything held. Exit 2: the product path held but a
// media-truth bar was missed (`bars` says which, with the figures; `load` is the Mac's load
// average — a bar missed under build load is re-measured idle, never loosened). Exit 1: the
// path broke.

import { execFileSync, spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { webkit, chromium } from "playwright";
import { readStrip, readStripGray, crc8, frameTruth, STALLS_COL } from "./strip.mjs";
import { signatureOf, compare } from "./signature.mjs";
import { avOffsets, avTruth, f32 } from "./av.mjs";
import { COPY } from "../../worker/src/worker.js";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("heartbeat.mjs");

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const e2e = process.argv.includes("--e2e");
const seconds = Number(process.argv.slice(2).find((a) => !a.startsWith("--")) ?? 10);
// A bad duration would leave the app recording (and the pattern covering the screen)
// with no end: refuse it before anything starts.
if (!Number.isFinite(seconds) || seconds < 2 || seconds > 600) {
  console.error("usage: node tests/e2e/heartbeat.mjs [seconds, 2–600]");
  process.exit(64);
}
const BAR = { framesPct: 0.5, keyedPageMs: 1000, finalizeMs: 200, cores: 1.0, avMs: 40, avDriftMs: 15 };
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-heartbeat-"));
const devtool = path.join(repo, "target/debug/shroom-dev");
const installed = process.env.SHROOM_INSTALLED ?? path.join(os.homedir(), "Applications/shroom-dev.app");
// What every app instance this script starts is told: on the e2e run, the test account only.
const pinned = e2e ? ["--e2e"] : [];
// What the headless browsers saw, kept for a person (E2E_PAGE_VIDEO_DIR).
const pageVideos = process.env.E2E_PAGE_VIDEO_DIR || null;
const SEEN = { width: 1024, height: 640 };
const seen = pageVideos ? { viewport: SEEN, recordVideo: { dir: pageVideos, size: SEEN } } : {};
async function keep(page, name) {
  if (!pageVideos || !page.video()) return;
  await page.close();
  await page.video().saveAs(path.join(pageVideos, name));
  await page.video().delete();
}
const built = path.join(repo, "target/debug/bundle/macos/shroom-dev.app");
const summary = { ok: false, pathOk: false, steps: {}, bars: {}, load: os.loadavg().map((l) => Number(l.toFixed(2))) };
const t0 = Date.now();
const step = (name, data) => { summary.steps[name] = { at: ((Date.now() - t0) / 1000).toFixed(1), ...data }; };
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// The beep's onsets in a published recording's audio, in seconds: where each silence ends.
function beepOnsets(url) {
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-nostats", "-i", url, "-map", "0:a:0", "-af", "silencedetect=noise=-30dB:d=0.3", "-f", "null", "-"], { encoding: "utf8", timeout: 120_000 });
  return [...(r.stderr ?? "").matchAll(/silence_end: ([\d.]+)/g)].map((m) => Number(m[1]));
}

// Why ffprobe read no size: its own answer, and what the page serves right then — the
// master playlist and the first variant it names. A miss with every fetch 200 is ffprobe's;
// a non-200 is the stream's (the run of 2026-10-19 on 1d0f347 left only "no size").
async function whyNoSize(url, probe, ms) {
  // Each fetch has 10 s: a diagnosis must never hang the run it explains.
  const get = (u) => fetch(u, { signal: AbortSignal.timeout(10_000) });
  const status = async (u) => { try { return (await get(u)).status; } catch (e) { return `unreachable (${e.cause?.code ?? e.name ?? e.message})`; } };
  let variant = null, variantStatus = null;
  try {
    const r = await get(url);
    const name = r.ok ? (await r.text()).split("\n").find((l) => l && !l.startsWith("#")) : null;
    if (name) { variant = new URL(name.trim(), url).href; variantStatus = await status(variant); }
  } catch { /* said below as the master's own status */ }
  const said = (probe.stderr ?? "").trim().replace(/\s+/g, " ").slice(0, 300);
  return [`ffprobe exit ${probe.status ?? "none"}${probe.signal ? `, ${probe.signal}` : ""}${probe.error ? `, ${probe.error.code ?? probe.error.message}` : ""} after ${ms} ms`,
    `said ${said ? JSON.stringify(said) : "nothing"}`, `master ${await status(url)}`, variant ? `variant ${variantStatus}` : "no variant named"].join("; ");
}

// Every frame's counter, decoded back from the page's own stream (TESTING-PLAN §4) at a
// height the strip reads well at, keeping the picture's shape — frame by frame as ffmpeg
// decodes them (a 10-minute take's frames are never held whole).
async function publishedCounters(url) {
  const asked = Date.now();
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", url], { encoding: "utf8", timeout: 60_000 });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  if (!w0 || !h0) throw new Error(`the published stream has no size — ${await whyNoSize(url, size, Date.now() - asked)}`);
  const h = 270, w = 2 * Math.round((h * w0) / h0 / 2);
  return new Promise((resolve, reject) => {
    const ff = spawn(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-i", url, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"]);
    const counters = [], stalls = [];
    let carry = Buffer.alloc(0), err = "";
    ff.stderr.on("data", (d) => { err += d; });
    ff.stdout.on("data", (chunk) => {
      const buf = carry.length ? Buffer.concat([carry, chunk]) : chunk;
      let at = 0;
      for (; at + w * h <= buf.length; at += w * h) {
        const frame = buf.subarray(at, at + w * h);
        counters.push(readStripGray(frame, w, h));
        stalls.push(readStripGray(frame, w, h, STALLS_COL));
      }
      carry = Buffer.from(buf.subarray(at));
    });
    ff.on("close", (code) => (code === 0 ? resolve({ counters, stalls, size: [w0, h0] }) : reject(new Error(`decoding the published stream: ${err.slice(0, 300)}`))));
  });
}

// The published sound, mono at 16 kHz, and where each stream starts.
const AV_RATE = 16_000;
function publishedSound(url) {
  const start = (stream) => Number(spawnSync(tool("ffprobe"), ["-v", "error", "-select_streams", stream, "-show_entries", "stream=start_time", "-of", "csv=p=0", url], { encoding: "utf8", timeout: 60_000 }).stdout.trim().split("\n")[0]) || 0;
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-i", url, "-map", "0:a:0", "-ac", "1", "-ar", String(AV_RATE), "-f", "f32le", "-"], { maxBuffer: 1 << 30, timeout: 300_000 });
  if (r.status !== 0) throw new Error(`decoding the published sound: ${String(r.stderr).slice(0, 300)}`);
  return { samples: f32(r.stdout), videoStart: start("v:0"), audioStart: start("a:0") };
}

// `codesign -d -r-`: the designated requirement.
const designated = (app) => {
  const r = spawnSync("codesign", ["-d", "-r-", app], { encoding: "utf8" });
  return (`${r.stdout}${r.stderr}`.match(/designated => (.*)/) ?? [])[1] ?? null;
};
const sha = (file) => createHash("sha256").update(fs.readFileSync(file)).digest("hex");

function preflight() {
  const pre = {};
  for (const t of ["ffmpeg", "ffprobe"]) if (!tool(t)) throw new Error(`${t} isn't installed (brew install ffmpeg)`);
  const verify = spawnSync("codesign", ["--verify", "--deep", "--strict", installed], { encoding: "utf8" });
  if (verify.status !== 0) throw new Error(`the installed dev app doesn't verify: ${verify.stderr.trim().slice(0, 200)}`);
  pre.dr = designated(installed);
  if (!/^identifier "am\.shroom\.dev" and certificate leaf = H"[0-9a-f]{40}"$/.test(pre.dr ?? "")) {
    throw new Error(`the dev app's designated requirement isn't its identifier + certificate leaf: ${pre.dr}`);
  }
  if (!fs.existsSync(built)) throw new Error("no fresh build to compare with (npm run build:dev in app/)");
  pre.sameRequirementAsBuild = designated(built) === pre.dr;
  pre.freshInstall = sha(path.join(installed, "Contents/MacOS/shroom")) === sha(path.join(built, "Contents/MacOS/shroom"));
  if (!pre.sameRequirementAsBuild) throw new Error("the fresh build is signed with another requirement — its grants wouldn't carry over (E1)");
  if (!pre.freshInstall) throw new Error("the installed dev app isn't the fresh build — install it first (ditto)");
  return pre;
}

const DRIVE_PROC = `${path.basename(installed)}/Contents/MacOS/shroom`;
let spaceUp = false;
let spaceHost = null;
let driving = null;
let member = null;
let memberHome = null;
try {
  step("preflight", preflight());

  // 1. A fenced space for the dev app — or, on the e2e run, the one its wizard wired.
  const up = e2e
    ? { host: JSON.parse(fs.readFileSync(path.join(os.homedir(), ".shroom-dev/space.json"), "utf8")).host }
    : JSON.parse(execFileSync(devtool, ["dev-space", "up", path.join(repo, "worker/src/worker.js"), path.join(repo, "node_modules/hls.js/dist/hls.min.js")], { encoding: "utf8" }).trim());
  spaceUp = !e2e;
  spaceHost = up.host;
  // A page just deployed takes a few seconds to answer as itself on its new address; wiring
  // waits for that before it's done, so the app starts once it does (else its first probe
  // reads the page as not the space's, and the stop strands — seen 2026-10-02).
  // Three answers in a row: a fresh workers.dev address can still answer “no Worker here”
  // now and then for a while after its first good answer (a stop's probe once read that).
  const pinged = Date.now();
  let inARow = 0;
  while (inARow < 3 && Date.now() - pinged < 90_000) {
    const r = await fetch(`https://${up.host}/_shroom/ping`).catch(() => null);
    const ok = !!r?.ok && (await r.json().catch(() => ({}))).shroom === true;
    inARow = ok ? inARow + 1 : 0;
    await sleep(ok ? 1000 : 500);
  }
  if (inARow < 3) throw new Error("the dev space's page never answered as a shroom space");
  step("space", { host: up.host, answeredAfterMs: Date.now() - pinged });

  // 2. The real app, driven like a person, under its own grants (LaunchServices). It
  // records, stops, waits for the video to be fully uploaded, leaves the owner's key beside its
  // report, and holds there until this script asks for the delete.
  spawnSync("pkill", ["-f", DRIVE_PROC]);
  const report = path.join(work, "drive.json");
  driving = spawn("open", ["-W", "-n", installed, "--args", ...pinned, "--drive", report, String(seconds)], { stdio: "ignore" });
  const exited = new Promise((ok) => driving.once("exit", ok));
  let drive = null;
  for (const until = Date.now() + (seconds + 300) * 1000; Date.now() < until;) {
    try { drive = JSON.parse(fs.readFileSync(report, "utf8")); } catch { drive = null; }
    if (drive && (drive.phase !== "recording" || drive.error)) break;
    if (driving.exitCode !== null) break;
    await sleep(250);
  }
  if (!drive) throw new Error("the drive left no report");
  if (drive.error) throw new Error(`drive: ${drive.error}`);
  if (drive.phase !== "live") throw new Error(`the drive stopped at ${drive.phase}`);
  step("record", {
    steps: Object.fromEntries(drive.steps.map(([k, ms]) => [k, ms])), stopToLinkMs: drive.stop_to_published_ms, stopToUploadedMs: drive.stop_to_uploaded_ms,
    stopToOpenMs: drive.stop_to_open_ms, stopToPage200Ms: drive.stop_to_page_200_ms, stopToKeyedPageMs: drive.stop_to_keyed_page_ms,
    keyedPageIsOwner: drive.keyed_page_is_owner, keyedStepsMs: drive.keyed_steps_ms, takeEndMs: drive.take_end_ms, finalizeMs: drive.finalize_ms, capture: drive.capture,
    takeFrames: drive.take_frames,
  });
  const origin = `https://${drive.host}`;
  const pageUrl = `${origin}/${drive.rec}`;

  // 3. The page answers.
  const pageStart = Date.now();
  let html = "";
  while (Date.now() - pageStart < 60_000) {
    const res = await fetch(pageUrl).catch(() => null);
    if (res?.ok) { html = await res.text(); if (html.includes("<video")) break; }
    await sleep(1000);
  }
  if (!html.includes("<video")) throw new Error("the page never showed the player");
  step("page", { url: pageUrl, title: (html.match(/<title>([^<]*)<\/title>/) || [])[1], noindex: html.includes('content="noindex"') });

  // 4. A recipient plays it: native HLS in WebKit, the page's hls.js path in Chromium.
  const plays = [];
  for (const [engine, forceHlsJs] of [[webkit, false], [chromium, true]]) {
    const browser = await engine.launch();
    const page = await browser.newPage();
    await page.addInitScript(`${crc8.toString()}\n${readStrip.toString()}
      window.__samples = []; window.__errors = []; window.__unread = [];
      ${forceHlsJs ? `const orig = HTMLMediaElement.prototype.canPlayType; HTMLMediaElement.prototype.canPlayType = function (t) { return /mpegurl/i.test(t) ? "" : orig.call(this, t); };` : ""}
      document.addEventListener("DOMContentLoaded", () => {
        const v = document.getElementById("v"); if (!v) return;
        const c = document.createElement("canvas"); const ctx = c.getContext("2d", { willReadFrequently: true });
        v.muted = true; v.addEventListener("error", () => __errors.push("media " + (v.error && v.error.code)));
        v.addEventListener("ended", () => { window.__ended = true; });
        const onFrame = (_, meta) => { const h = 270, w = Math.round(v.videoWidth / v.videoHeight * h) || 480;
          c.width = w; c.height = h; ctx.drawImage(v, 0, 0, w, h);
          const px = ctx.getImageData(0, 0, w, h).data, n = readStrip(px, w, h);
          // An unreadable frame, described: was anything drawn at all (the canvas starts each
          // frame transparent), how bright, and how ready the element said it was.
          let drawn = true;
          if (n === null) { let a = 0, l = 0; for (let i = 0; i < px.length; i += 4) { a += px[i + 3]; l += px[i] + px[i + 1] + px[i + 2]; }
            drawn = a > 0;
            if (__unread.length < 5) __unread.push({ at: __samples.length, drawn, luma: Math.round(l / (px.length / 4) / 3), readyState: v.readyState, presentedFrames: meta.presentedFrames, mediaTime: meta.mediaTime }); }
          // WebKit's native player can call back for the first frame before it can be drawn:
          // nothing lands on the canvas, and the next callback brings that same frame, readable
          // (DECISIONS B128). That one call — the first, nothing drawn — is no frame seen; every
          // other one counts.
          if (n === null && !drawn && __samples.length === 0 && !window.__undrawnFirst) window.__undrawnFirst = true;
          else __samples.push(n);
          window.__t = meta.mediaTime; v.requestVideoFrameCallback(onFrame); };
        v.requestVideoFrameCallback(onFrame);
        const go = () => { v.playbackRate = 2; v.play().catch(e => __errors.push("play " + e.message)); };
        if (v.readyState > 0) go(); else v.addEventListener("loadedmetadata", go, { once: true });
      });`);
    await page.goto(pageUrl);
    const start = Date.now();
    let s = {};
    let sought = false;
    while (Date.now() - start < 90_000) {
      // A long take plays its first and last 15 s: the jump forward is no frame backwards.
      if (seconds > 60 && !sought && s.t > 15) {
        await page.evaluate(() => { const v = document.getElementById("v"); v.currentTime = Math.max(0, v.duration - 15); });
        sought = true;
      }
      s = await page.evaluate(() => { const v = document.getElementById("v");
        return { ended: !!window.__ended, errors: window.__errors, t: window.__t, n: window.__samples.length, hlsjs: typeof window.Hls === "function",
          audioBytes: v && typeof v.webkitAudioDecodedByteCount === "number" ? v.webkitAudioDecodedByteCount : null,
          audioTracks: v && v.audioTracks ? v.audioTracks.length : null,
          audioEnabled: v && v.audioTracks && v.audioTracks.length ? !!v.audioTracks[0].enabled : null }; });
      if (s.ended || s.errors.length) break;
      await sleep(500);
    }
    const [counters, unread, undrawnFirst] = await page.evaluate(() => [window.__samples, window.__unread, !!window.__undrawnFirst]);
    let backwards = 0, prev = null;
    for (const c of counters) { if (c === null) continue; if (prev !== null && c < prev) backwards++; prev = c; }
    // Sound, as the engine reports it — never nothing: WebKit's native player reports an
    // enabled audio track (its decoded-byte count isn't exposed there); an engine that counts
    // decoded audio bytes must count some. A null in WebKit fails.
    const heard = engine === webkit
      ? (s.audioBytes !== null ? s.audioBytes > 0 : s.audioTracks > 0 && s.audioEnabled === true)
      : s.audioBytes === null || s.audioBytes > 0;
    plays.push({ engine: engine.name(), player: s.hlsjs ? "hls.js" : "native", ended: s.ended, mediaTime: s.t, audioBytes: s.audioBytes, audioTracks: s.audioTracks, audioEnabled: s.audioEnabled, heard,
      presented: counters.length, readable: counters.filter((c) => c !== null).length, backwards, errors: s.errors,
      // Where a presented frame didn't read, and its neighbours' counters: what to look at.
      unreadableAt: counters.flatMap((c, i) => (c === null ? [[i, counters[i - 1] ?? null, counters[i + 1] ?? null]] : [])).slice(0, 5), unread, undrawnFirst });
    await browser.close();
  }
  step("play", { plays });
  // For a person (E2E_PAGE_VIDEO_DIR): the page playing the take in WebKit, at its own pace —
  // a pass of its own, never measured.
  if (pageVideos) {
    const b = await webkit.launch();
    try {
      const p = await b.newPage(seen);
      await p.goto(pageUrl);
      await p.evaluate(() => { const v = document.getElementById("v"); if (v) { v.muted = true; v.play().catch(() => {}); } });
      await sleep(Math.min(seconds, 30) * 1000 + 2500);
      await keep(p, "play-webkit.webm");
    } catch { /* a clip for a person, never the verdict */ } finally {
      await b.close();
    }
  }
  const playsOk = plays.every((p) => p.ended && p.errors.length === 0 && p.backwards === 0 && p.readable === p.presented && p.presented > 0 && p.heard);

  // 5. Media truth from what was published: every frame's counter, and the recipe's shape.
  const media = `${origin}/m/${drive.rec}`;
  const decoded = await publishedCounters(`${media}/master.m3u8`);
  const truth = frameTruth(decoded.counters, 60, 30, decoded.stalls);
  const sigDiff = compare(await signatureOf(media));
  step("media", { size: decoded.size, ...truth, signature: sigDiff.length ? sigDiff : "matches the golden" });
  const mediaOk = truth.frames > 0 && truth.unreadable === 0 && truth.backwards === 0 && sigDiff.length === 0;

  // 6. What was published sounds like the pattern, never the room: its audio is the 1 Hz
  // beep — an onset every second (b4 #17).
  const beeps = beepOnsets(`${media}/master.m3u8`);
  const gaps = beeps.slice(1).map((t, i) => t - beeps[i]);
  const onBeat = gaps.filter((g) => Math.abs(g - 1) <= 0.1).length;
  step("beep", { onsets: beeps.length, gaps: gaps.map((g) => Number(g.toFixed(3))), onBeat });
  const beepOk = beeps.length >= Math.floor(seconds) - 2 && gaps.length > 0 && onBeat >= Math.ceil(gaps.length * 0.8);

  // 6b. A/V through the real capture path: each beep against where the published picture
  // shows that second (the pattern counts from a whole second on the beep's clock).
  const heard = publishedSound(`${media}/master.m3u8`);
  const offsets = avOffsets(decoded.counters, 30, heard.samples, AV_RATE, { videoStart: heard.videoStart, audioStart: heard.audioStart });
  const av = avTruth(offsets);
  step("av", { ...av, starts: [heard.videoStart, heard.audioStart], offsetsMs: offsets.length <= 40 ? offsets.map((o) => o.offsetMs) : undefined });
  const avOk = av.beeps >= Math.floor(seconds) - 3;

  // 7. The owner titles it: the app's one-time key (left beside the drive's report) makes
  // this browser the owner's; the page's own title field saves it (the browser's POST, with
  // its cookie and Origin); a fresh recipient sees the new title.
  const title = `Heartbeat ${new Date().toISOString().slice(11, 19)}`;
  const k = fs.readFileSync(path.join(work, "owner-key"), "utf8").trim();
  const owner = await chromium.launch();
  let titled;
  try {
    const page = await owner.newPage(seen);
    await page.goto(`${pageUrl}?k=${encodeURIComponent(k)}`);
    const landed = page.url();
    const cookie = (await page.context().cookies()).some((c) => c.name === "shroom_owner" && c.httpOnly && c.secure);
    await page.waitForSelector("#t", { timeout: 10_000 });
    const posted = page.waitForResponse((r) => r.url() === `${origin}/_shroom/title/${drive.rec}` && r.request().method() === "POST", { timeout: 15_000 });
    await page.fill("#t", title);
    await page.press("#t", "Enter");
    const res = await posted;
    titled = { landedClean: landed === pageUrl, cookie, status: res.status(), origin: (await res.request().allHeaders()).origin === origin };
    // A moment on the titled page, for whoever watches.
    if (pageVideos) await sleep(1500);
    await keep(page, "owner.webm");
  } finally {
    await owner.close();
  }
  const recipient = await webkit.launch();
  try {
    const page = await recipient.newPage();
    await page.goto(pageUrl);
    titled.recipientSees = (await page.title()) === title;
  } finally {
    await recipient.close();
  }
  step("title", { title, ...titled });
  const titleOk = titled.landedClean && titled.cookie && titled.status === 200 && titled.origin && titled.recipientSees;

  // 7b. The member leg (DECISIONS B68), live on the same fenced space: the owner invites
  // in Members (its drive makes the link and leaves it in a mode-600 hand-off file); the
  // file is moved into a second instance's throwaway home, as the installer hands an
  // invite over (never an argv, a pipe or a log); that instance — the installed dev app,
  // `open -n` — joins, records the pattern into the space (minted by the live page); a
  // recipient plays the member's video; the owner removes the member in Members; the
  // member's next look reads member-revoked from the live page.
  const readReport = () => { try { return JSON.parse(fs.readFileSync(report, "utf8")); } catch { return null; } };
  const ownerAt = async (phase, secs) => {
    for (const until = Date.now() + secs * 1000; Date.now() < until;) {
      const r = readReport();
      if (r?.error) { const last = r.member?.at(-1) ?? {}; throw new Error(`drive (member leg): ${r.error}: ${last.error ?? ""} ${JSON.stringify(last.facts ?? {})}`.slice(0, 1500)); }
      if (r?.phase === phase) return r;
      if (driving.exitCode !== null) throw new Error(`the owner's drive ended before ${phase}`);
      await sleep(250);
    }
    throw new Error(`the owner's drive never reached ${phase}`);
  };
  memberHome = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-hb-member-"));
  fs.chmodSync(memberHome, 0o700);
  fs.writeFileSync(path.join(work, "member-now"), "");
  const invited = await ownerAt("invited", 180);
  fs.renameSync(path.join(work, "invite.handoff"), path.join(memberHome, "invite"));
  fs.chmodSync(path.join(memberHome, "invite"), 0o600);
  const memberOut = path.join(work, "member.json");
  member = spawn("open", ["-W", "-n", "--stderr", path.join(work, "member.stderr.log"), installed, "--args", ...pinned, "--home", memberHome, "--drive", "hb-member", memberOut, "--invite"], { stdio: "ignore" });
  const memberExited = new Promise((ok) => member.once("exit", ok));
  let recorded = null;
  for (const until = Date.now() + 420_000; Date.now() < until && !recorded;) {
    try { recorded = JSON.parse(fs.readFileSync(path.join(work, "member-recorded"), "utf8")); } catch { recorded = null; }
    if (!recorded && member.exitCode !== null) break;
    await sleep(250);
  }
  if (!recorded) {
    const m = (() => { try { return JSON.parse(fs.readFileSync(memberOut, "utf8")); } catch { return null; } })();
    throw new Error(`the member never recorded: ${m?.error ?? "no report"} ${JSON.stringify((m?.checks ?? []).filter((c) => !c[1]))}`.slice(0, 800));
  }
  const memberPage = await fetch(`${origin}/${recorded.rec}`).catch(() => null);
  const memberHtml = memberPage?.ok ? await memberPage.text() : "";
  const memberPlaylist = (await fetch(`${origin}/m/${recorded.rec}/master.m3u8`).catch(() => null))?.status ?? null;
  fs.writeFileSync(path.join(work, "remove-now"), "");
  const removedOwner = await ownerAt("removed", 180);
  fs.writeFileSync(path.join(work, "go-learn"), "");
  await Promise.race([memberExited, sleep(240_000)]);
  const m = (() => { try { return JSON.parse(fs.readFileSync(memberOut, "utf8")); } catch { return { ok: false, error: "no report" }; } })();
  const memberLeg = {
    invite: invited.member?.[0]?.checks?.length ?? 0,
    sameSpace: recorded.host === drive.host,
    page: memberPage?.status ?? null, player: memberHtml.includes("<video"), playlist: memberPlaylist,
    removal: removedOwner.member?.[1] ? { ok: removedOwner.member[1].ok, checks: removedOwner.member[1].checks } : null,
    member: { ok: m.ok, error: m.error, checks: m.checks?.length, failed: (m.checks ?? []).filter((c) => !c[1]), states: m.states?.slice(-4) },
  };
  step("member", memberLeg);
  const memberOk = memberLeg.sameSpace && memberLeg.page === 200 && memberLeg.player && memberLeg.playlist === 200 && memberLeg.removal?.ok === true && m.ok === true;

  // 8. Delete, through the app's Library (the drive does it like a person), then the page
  // shows the removed face and the bytes are gone for everyone.
  fs.writeFileSync(path.join(work, "delete-now"), "");
  await Promise.race([exited, sleep(120_000)]);
  const final = JSON.parse(fs.readFileSync(report, "utf8"));
  if (final.error) throw new Error(`drive (delete): ${final.error}`);
  const removedRes = await fetch(pageUrl);
  const removedHtml = await removedRes.text();
  if (pageVideos) {
    const b = await webkit.launch();
    try {
      const p = await b.newPage({ viewport: SEEN });
      await p.goto(pageUrl);
      await p.screenshot({ path: path.join(pageVideos, "removed.png") });
    } catch { /* a still for a person, never the verdict */ } finally {
      await b.close();
    }
  }
  const removed = {
    library: final.deleted === true && final.phase === "deleted",
    // A removed video reads as a plain not found (B141): the not-found line, a 404, no player.
    face: removedRes.status === 404 && removedHtml.includes(COPY.notFound) && !removedHtml.includes("<video"),
    playlist: (await fetch(`${media}/master.m3u8`)).status,
    download: (await fetch(`${origin}/d/${drive.rec}`)).status,
  };
  step("delete", { steps: final.steps.slice(-6).map(([k]) => k), ...removed });
  const deleteOk = removed.library && removed.face && removed.playlist === 404 && removed.download === 404;

  summary.pathOk = playsOk && mediaOk && beepOk && avOk && titleOk && memberOk && deleteOk && drive.keyed_page_is_owner === true;

  // The bars (TESTING-PLAN §4), each with its figure.
  const bar = (name, value, limit) => { summary.bars[name] = { value, bar: limit, ok: value !== null && value !== undefined && value <= limit }; };
  // Capture's: a hold the pattern itself made (its own missed refreshes, drawn beside its
  // counter) was on the screen twice and is kept faithfully (DECISIONS B59 f); `media` has
  // the raw figures too.
  bar("repeatedFramesPct", truth.repeatedByCapturePct, BAR.framesPct);
  bar("skippedFramesPct", truth.skippedByCapturePct, BAR.framesPct);
  bar("stopToKeyedPageMs", drive.stop_to_keyed_page_ms, BAR.keyedPageMs);
  bar("takeEndMs", drive.take_end_ms, BAR.finalizeMs);
  bar("finalizeMs", drive.finalize_ms, BAR.finalizeMs);
  bar("captureCores", drive.capture ? Number(drive.capture.cores.toFixed(3)) : null, BAR.cores);
  bar("avOffsetMaxMs", av.maxAbsMs, BAR.avMs);
  // Thirds of a take mean something from 30 s on.
  if (seconds >= 30) bar("avDriftMs", av.driftMs, BAR.avDriftMs);
  summary.ok = summary.pathOk && Object.values(summary.bars).every((b) => b.ok);
} catch (e) {
  summary.error = String(e.message ?? e).slice(0, 1500);
} finally {
  // A drive still waiting (this script failed before the delete) ends here: no app is left
  // holding a take or a window.
  if (driving && driving.exitCode === null) spawnSync("pkill", ["-f", DRIVE_PROC]);
  if (member && member.exitCode === null) spawnSync("pkill", ["-f", DRIVE_PROC]);
  // The member's throwaway home (its device key, its take of the pattern) goes.
  if (memberHome) fs.rmSync(memberHome, { recursive: true, force: true });
  if (spaceUp) {
    try { execFileSync(devtool, ["dev-space", "down"]); step("teardown", {}); }
    catch (e) { summary.teardownError = String(e.message).slice(0, 300); }
    // The device key the drive enrolled for this space goes with it, and the Library's cache
    // of the space (its rows and posters).
    const deviceKey = path.join(os.homedir(), ".shroom-dev/device.key");
    try { if (JSON.parse(fs.readFileSync(deviceKey, "utf8")).host === spaceHost) fs.rmSync(deviceKey); } catch { /* none, or another space's */ }
    for (const f of ["library.json", "posters"]) fs.rmSync(path.join(os.homedir(), ".shroom-dev", f), { recursive: true, force: true });
  }
  fs.rmSync(work, { recursive: true, force: true });
  // The dev app's test takes (the pattern and its beep): never kept.
  const takes = path.join(os.homedir(), ".shroom-dev/takes");
  for (const d of fs.existsSync(takes) ? fs.readdirSync(takes, { withFileTypes: true }).filter((e) => e.isDirectory()) : []) {
    fs.rmSync(path.join(takes, d.name), { recursive: true, force: true });
  }
  summary.seconds = Number(((Date.now() - t0) / 1000).toFixed(1));
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : summary.pathOk ? 2 : 1);
}
