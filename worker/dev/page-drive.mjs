// The page in real browsers (DECISIONS B43): headless Chromium — through the bucket's
// pinned hls.js, as the heartbeat forces it — and headless WebKit with native HLS, against
// the real worker.js over loopback HTTP and a real fMP4 HLS stream this drive encodes with
// ffmpeg. Never a visible browser, never the network, never a real space.
//
//   npm run drive:page          (from worker/)
//
// What it proves, per engine:
//   player     the poster loads before anything plays; no native controls; the time reads
//              0:00 / 0:20; Tab (WebKit: ⌥Tab, Safari's key for buttons) reaches Play, Space plays (aria-label → Pause) and time
//              moves; Tab to the scrubber, → seeks 5 s; K pauses; the fullscreen button
//              makes the player itself fullscreen and back (Chromium; WebKit headless has
//              no fullscreen, reported but not required)
//   waiting    the recipient's on-its-way line sits in the frame over a breathing bar at
//              the upload's percentage; the stalled line's bar rests
//   owner      the stage is empty while uploading (no recipient sentence); a refused copy
//              keeps "Copy link", a real one says "Copied"; the go-live swaps the player in
//              with its poster and, every file up, the upload line goes; then Download
//              appears when the MP4 lands; a delete swaps
//              to the removed line; unavailable puts its line in the frame
//   mark       the owner flips “made with shroom” off: the switch, its words and the logo
//              leave their own page at once — before the write has answered — and the
//              corner stays empty after a reload; a browser with no cookie and the embed
//              carry none; a write that fails puts them all back; after a Back the switch
//              says what the space says; on again from Settings, the page follows on its
//              next load (DEVIATIONS #41)
//   icon       on a video's page and on the join page the browser finds the three icon
//              lines in the head, each pointing at this space's own address, and
//              /favicon.svg, /favicon.png and /favicon.ico each load as a picture and
//              draw the landing's art (DECISIONS B109) — a headless browser has no tab
//              to draw it in, so the tab itself stays for a person's eyes
//   window     a 4:3 recording in a short (1280×640) window: the player, its controls and
//              the mark's foot fit, the column takes the player's width; the waiting frame
//              fits too; a tall window keeps the 960px column (the canvas's "a short
//              window"). PAGE_DRIVE_SHOTS=<folder> keeps a screenshot of each
//   frame      the whole frame shows: a 16:10, a 16:9 and a portrait recording, each in a
//              wide-short, a tall-narrow and a phone window, before and during a play — the
//              picture's box has the recording's shape, sits whole in the player, and the
//              control bar is below it, never over it
//   copy       with the single-file copy up, the page plays it and asks for no piece of the
//              stream (DECISIONS B177); every check above plays the stream, as before it's up
//   cuts       the owner page's voice cuts (N65, DECISIONS B200): under the player, a row per
//              voice cut between the words around it, an applied one struck through; Put back and Cut it too save at once,
//              the row then saying what it is and what plays reaching the player; a press
//              that doesn't save leaves its row and says so; a recipient's page carries none
//              of it; a page open while the cues are coming keeps its one line (hearing), its spinner
//              turning, then takes the strip in when they land; the hint shows on a take with no
//              voice cuts, and Got it (or a strip seen) puts it away; the line fades when the Mac finds
//              nothing to cut (the hint in its place), and by its own count after five minutes
//   edit       a saved edit (N63, DECISIONS B195: 2–6, 9–12 and 13.5–18 of the 20 s) played
//              at 2× over the stream and over the single-file copy: the position, sampled
//              every frame and every 4 ms, never falls inside a cut (0.05 s past a cut's start
//              allowed), every kept range plays, it stops at the out, and the time reads the
//              edited length, 0:00 / 0:12 then 0:12 / 0:12. With the edit's permanent copy up:
//              the page plays the copy to its end with nothing skipped, its own first frame,
//              asks for nothing of the original, and the original's stream, file and
//              trimmed-off first frame answer 404 (step 3). The copy landing under an open page: its next piece of the
//              original answers 404 and the player moves over to the copy (hls.js; native
//              engines that fetched the whole file have nothing to miss — reported only)
//   baked      a baked recording (B197: media names the edit's own files, no stream) plays its
//              one file to its end, its own first frame and length, asks for nothing of the
//              stream, and what the bake left behind answers 404 while its own files are served
//   keep       shroom:keep (the voice-cut build's event, #59) hands an unedited page the
//              edit's ranges: it then plays them at 2× as a saved edit plays, never a cut
//              moment, the time reading 0:12
//   bytes      a tombstoned recording's playlist answers 404
//
// Prints one JSON summary and exits 0 only when every required check passed.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { chromium, webkit } from "playwright";
import { COPY, VOICE_WAIT_MS, handle } from "../src/worker.js";
import { CODE, INV, M1, OWNER_AIR, OWNER_MAC, cookieFor, metaKey, roster, sha, space } from "../test/space-fixture.js";

const here = path.dirname(new URL(import.meta.url).pathname);
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-page-drive-"));
const summary = { ok: false, engines: {} };
const ids = { live: "LiveLiveLiveLive", wait: "WaitWaitWaitWait", stall: "StalStalStalStal", own: "OwnrOwnrOwnrOwnr", gone: "GoneGoneGoneGone", off: "OffxOffxOffxOffx",
  edit: "EditEditEditEdit", cut: "CutsCutsCutsCuts", land: "LandLandLandLand", bake: "BakeBakeBakeBake", cuts: "VoicVoicVoicVoic" };
const DURATION = 20;
/** The edit the drive plays (N63), as the Mac's editor saves it: 2–6, 9–12 and 13.5–18 of
 * the 20 s recording play — 11.5 s, which reads 0:12. */
const cut = (start, end) => ({ start, end, kind: "manual", confidence: "high", applied: true, removed: "" });
const EDIT = { version: 1, source_duration: DURATION, in: 2, out: 18, cuts: [cut(6, 9), cut(12, 13.5)], rev: 1 };
const KEEP = [[2, 6], [9, 12], [13.5, 18]];
/** What a viewer must never see: the trimmed ends and the cuts. */
const CUT_OUT = [[0, 2], [6, 9], [12, 13.5], [18, DURATION]];
/** The edit's permanent copy, as the Mac puts it up beside the original. */
const EDIT_COPY = { rev: 1, file: "edit-1.mp4", duration: 11.5, poster: "edit-1-poster.jpg" };
/** The shapes the whole-frame check plays (a recording of each, a few seconds long): a
 * MacBook's 16:10 screen, a 16:9 display, a tall portrait window. */
const SHAPES = { "16:10": ["Sh1610Sh1610Sh16", "640x400"], "16:9": ["Sh169xSh169xSh16", "640x360"], portrait: ["ShTallShTallShTa", "360x640"] };
/** The windows it plays them in: wide and short, tall and narrow, a phone. */
const WINDOWS = { "wide-short": [1440, 600], "tall-narrow": [700, 1100], phone: [390, 844] };

/** A real fMP4 HLS stream, its poster and master playlist, as the app uploads them — the
 * picture only. It has no sound on purpose: these engines play through this Mac's real
 * audio output, and a tab that starts playing takes a person's headphones off their phone.
 * Sound is the heartbeat's to prove, where the player is muted. */
function encode(size = "640x400", seconds = DURATION) {
  const dir = path.join(work, `${size}-${seconds}`), v0 = path.join(dir, "v0");
  fs.mkdirSync(v0, { recursive: true });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", `testsrc2=size=${size}:rate=30`, "-an",
    "-t", String(seconds), "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "30", "-f", "hls", "-hls_time", "2", "-hls_playlist_type", "vod",
    "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4", "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0 });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", `testsrc2=size=${size}`, "-frames:v", "1", path.join(dir, "poster.jpg")]);
  fs.writeFileSync(path.join(dir, "master.m3u8"), `#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=${size}\nv0/stream.m3u8\n`);
  const files = {};
  for (const f of ["master.m3u8", "poster.jpg", ...fs.readdirSync(v0).map((n) => `v0/${n}`)]) files[f] = new Uint8Array(fs.readFileSync(path.join(dir, f)));
  // The single-file copy, joined from the stream as the app joins its takes (+faststart).
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-i", path.join(v0, "stream.m3u8"), "-c", "copy", "-movflags", "+faststart", path.join(dir, "video.mp4")]);
  files["video.mp4"] = new Uint8Array(fs.readFileSync(path.join(dir, "video.mp4")));
  return files;
}

/** An edit's permanent copy and its first frame (N63): its own picture, the edit's length. */
function encodeCopy() {
  const dir = path.join(work, "copy");
  fs.mkdirSync(dir, { recursive: true });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=640x400:rate=30", "-an", "-t", String(EDIT_COPY.duration),
    "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "30", "-movflags", "+faststart", path.join(dir, EDIT_COPY.file)]);
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=640x400", "-frames:v", "1", path.join(dir, EDIT_COPY.poster)]);
  return Object.fromEntries([EDIT_COPY.file, EDIT_COPY.poster].map((f) => [f, new Uint8Array(fs.readFileSync(path.join(dir, f)))]));
}

const iso = (ms) => new Date(ms).toISOString();
const meta = (id, over) => JSON.stringify({ schema: 1, id, state: "live", createdAt: iso(Date.now() - 60_000), title: "Where the 502s come from", durationSec: DURATION,
  media: { hls: `rec/${id}/master.m3u8`, mp4: null, poster: `rec/${id}/poster.jpg`, width: 640, height: 400 }, owner: M1, ...over });
const uploading = (id, done, total, ageMs = 1000) => meta(id, { state: "uploading", media: null, progress: { done, total, updatedAt: iso(Date.now() - ageMs) } });

async function main() {
  const files = encode();
  // One invite, made a minute ago: the join page as the person invited sees it.
  const invite = { codeHash: sha(CODE.ani), kind: "member", label: "Ani", inviter: "Bagrat", by: OWNER_MAC.id,
    createdAt: iso(Date.now() - 60_000), expiresAt: iso(Date.now() + 7 * 24 * 3600_000), state: "pending", used: null, endedAt: null };
  const env = space({ roster: roster({ invites: { [INV.ani]: invite } }) });
  const put = (k, v) => env.BUCKET.put(k, v);
  await put(".shroom/player/hls.min.js", new Uint8Array(fs.readFileSync(path.join(here, "../vendor/hls.min.js"))));
  for (const id of Object.values(ids)) for (const [f, bytes] of Object.entries(files)) await put(`rec/${id}/${f}`, bytes);
  for (const [f, bytes] of Object.entries(encodeCopy())) for (const id of [ids.edit, ids.cut, ids.land, ids.bake]) await put(`rec/${id}/${f}`, bytes);
  for (const [id, size] of Object.values(SHAPES)) {
    const [width, height] = size.split("x").map(Number);
    for (const [f, bytes] of Object.entries(encode(size, 4))) await put(`rec/${id}/${f}`, bytes);
    await put(metaKey(id), meta(id, { durationSec: 4, media: { hls: `rec/${id}/master.m3u8`, mp4: null, poster: `rec/${id}/poster.jpg`, width, height } }));
  }

  const server = createServer(async (req, res) => {
    const chunks = [];
    for await (const c of req) chunks.push(c);
    const url = `http://127.0.0.1:${server.address().port}${req.url}`;
    const r = await handle(new Request(url, { method: req.method, headers: req.headers, body: chunks.length ? Buffer.concat(chunks) : undefined }), env, Date.now());
    res.writeHead(r.status, Object.fromEntries(r.headers));
    if (r.body) for await (const c of r.body) res.write(c);
    res.end();
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const origin = `http://127.0.0.1:${server.address().port}`;

  try {
    for (const [engine, forceHlsJs] of [[chromium, true], [webkit, false]]) {
      const name = engine.name();
      // Safari moves focus to buttons with ⌥Tab (Tab alone reaches only fields).
      const tab = name === "webkit" ? "Alt+Tab" : "Tab";
      const out = (summary.engines[name] = { checks: {}, errors: [] });
      const check = (k, v, required = true, detail = undefined) => { out.checks[k] = { ok: !!v, required, ...(v ? {} : { detail }) }; };
      const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
      const browser = await engine.launch();
      try {
        const context = await browser.newContext();
        if (forceHlsJs) await context.addInitScript(`const o = HTMLMediaElement.prototype.canPlayType; HTMLMediaElement.prototype.canPlayType = function (t) { return /mpegurl/i.test(t) ? "" : o.call(this, t); };`);
        const page = await context.newPage();
        page.on("pageerror", (e) => out.errors.push(String(e.message).slice(0, 200)));

        // ── the player ──
        await put(metaKey(ids.live), meta(ids.live, {}));
        const posterLoaded = page.waitForResponse((r) => r.url().endsWith(`/m/${ids.live}/poster.jpg`) && r.status() === 200, { timeout: 10_000 }).then(() => true, () => false);
        await page.goto(`${origin}/${ids.live}`);
        const first = await page.evaluate(() => { const v = document.getElementById("v"); return { controls: v.controls, poster: v.getAttribute("poster"), paused: v.paused, time: document.querySelector(".time").textContent }; });
        check("poster loads before play", (await posterLoaded) && first.paused && first.poster?.endsWith("/poster.jpg"));
        check("no native controls", first.controls === false);
        check("time reads 0:00 / 0:20", first.time === "0:00 / 0:20");
        // Copy link sits ahead of the player in the tab order (N46): Tab goes on to Play.
        let onPlay = false;
        for (let i = 0; i < 6 && !onPlay; i++) {
          await page.keyboard.press(tab);
          onPlay = await page.evaluate(() => !!document.activeElement?.classList.contains("pp"));
        }
        check("Tab reaches Play", onPlay && await page.evaluate((w) => document.activeElement.getAttribute("aria-label") === w, COPY.play));
        await page.keyboard.press("Space");
        const playing = await page.waitForFunction(() => { const v = document.getElementById("v"); return !v.paused && v.currentTime > 1.1; }, null, { timeout: 15_000 }).then(() => true, () => false);
        // The label and the time follow their media events, a task after the state moves.
        const moved = await page.waitForFunction((w) => document.querySelector(".pp").getAttribute("aria-label") === w && /^0:0[1-9] \/ 0:20$/.test(document.querySelector(".time").textContent), COPY.pause, { timeout: 3000 }).then(() => true, () => false);
        const during = await page.evaluate(() => ({ label: document.querySelector(".pp").getAttribute("aria-label"), time: document.querySelector(".time").textContent, t: document.getElementById("v").currentTime }));
        check("Space plays; the label says Pause and the time moves", playing && moved, true, during);
        await page.keyboard.press(tab);
        const onSeek = await page.evaluate(() => document.activeElement?.classList.contains("seek"));
        const before = await page.evaluate(() => document.getElementById("v").currentTime);
        await page.keyboard.press("ArrowRight");
        const after = await page.evaluate(() => document.getElementById("v").currentTime);
        check("Tab to the scrubber, → seeks 5 s", onSeek && after - before >= 4 && after - before <= 6.5);
        // K once the seek has landed: headless WebKit (MSE) plays on again from a pause made
        // while a seek is still under way — 3 runs in 8 at 0.1.24's page.
        await page.waitForFunction(() => !document.getElementById("v").seeking, null, { timeout: 5000 }).catch(() => {});
        await page.keyboard.press("k");
        // The label follows the pause event, a task after `paused` flips.
        const kPaused = await page.waitForFunction((w) => document.getElementById("v").paused && document.querySelector(".pp").getAttribute("aria-label") === w, COPY.play, { timeout: 3000 }).then(() => true, () => false);
        check("K pauses, the label says Play", kPaused, true, await page.evaluate(() => ({ paused: document.getElementById("v").paused, label: document.querySelector(".pp").getAttribute("aria-label"), active: document.activeElement?.className })));
        await page.click(".fs");
        const full = await page.waitForFunction(() => document.fullscreenElement === document.querySelector(".player"), null, { timeout: 3000 }).then(() => true, () => false);
        // The label follows `fullscreenchange`, a task after the element is full screen.
        const exitLabel = full ? await page.waitForFunction((w) => document.querySelector(".fs").getAttribute("aria-label") === w, COPY.exitFullscreen, { timeout: 3000 })
          .then(() => COPY.exitFullscreen, () => page.evaluate(() => document.querySelector(".fs").getAttribute("aria-label"))) : null;
        if (full) await page.click(".fs");
        const back = full ? await page.waitForFunction(() => !document.fullscreenElement, null, { timeout: 3000 }).then(() => true, () => false) : false;
        check("fullscreen is the player itself, and back", full && back && exitLabel === COPY.exitFullscreen, name === "chromium", { full, back, exitLabel });

        // ── the single-file copy (DECISIONS B177) ──
        await put(metaKey(ids.live), meta(ids.live, { media: { hls: `rec/${ids.live}/master.m3u8`, mp4: `rec/${ids.live}/video.mp4`, poster: `rec/${ids.live}/poster.jpg`, width: 640, height: 400 } }));
        const asked = [];
        const onRequest = (/** @type {any} */ r) => asked.push(new URL(r.url()).pathname);
        page.on("request", onRequest);
        await page.goto(`${origin}/${ids.live}`);
        const copyPlays = (await page.evaluate(() => { const v = document.getElementById("v"); v.muted = true; return v.play().then(() => true, () => false); }))
          && (await page.waitForFunction(() => document.getElementById("v").currentTime > 1.1, null, { timeout: 15_000 }).then(() => true, () => false));
        page.off("request", onRequest);
        check("with the copy up, the page plays it and asks for no piece of the stream", copyPlays && asked.some((p) => p.endsWith("/video.mp4")) && !asked.some((p) => /\.(m3u8|m4s)$/.test(p)), true, { copyPlays, asked });
        await put(metaKey(ids.live), meta(ids.live, {}));

        // ── a saved edit (N63, DECISIONS B195) ──
        const stream = (id) => ({ hls: `rec/${id}/master.m3u8`, mp4: null, poster: `rec/${id}/poster.jpg`, width: 640, height: 400 });
        const single = (id) => ({ ...stream(id), mp4: `rec/${id}/video.mp4` });
        /** Opens the page, plays it at 2× from Play, and samples where the video is — every
         * frame, every 4 ms and on every timeupdate — until it stops by itself. `keep`: ranges
         * handed to the page first, as the voice-cut build's `shroom:keep` event does (#59). */
        const playThrough = async (id, ended, keep = null) => {
          const asked = [];
          const onRequest = (/** @type {any} */ r) => asked.push(new URL(r.url()).pathname);
          page.on("request", onRequest);
          await page.goto(`${origin}/${id}`);
          if (keep) await page.evaluate((detail) => document.dispatchEvent(new CustomEvent("shroom:keep", { detail })), keep);
          const before = await page.evaluate(() => ({ time: document.querySelector(".time").textContent, poster: document.getElementById("v").getAttribute("poster") }));
          await page.evaluate(() => {
            const v = document.getElementById("v");
            const w = /** @type {any} */ (window);
            w.seen = [];
            const take = () => { if (!v.paused) w.seen.push(v.currentTime); };
            (function frame() { take(); requestAnimationFrame(frame); })();
            setInterval(take, 4);
            v.addEventListener("timeupdate", take);
          });
          for (let i = 0; i < 3; i++) await page.click(".sp");
          await page.click(".pp");
          const done = await page.waitForFunction((end) => { const v = document.getElementById("v"); return /** @type {any} */ (window).seen.length > 20 && v.paused && (!end || v.ended); }, ended, { timeout: 40_000 }).then(() => true, () => false);
          const seen = /** @type {number[]} */ (await page.evaluate(() => /** @type {any} */ (window).seen));
          const after = await page.evaluate(() => ({ time: document.querySelector(".time").textContent, rate: document.getElementById("v").playbackRate, src: document.getElementById("v").currentSrc }));
          page.off("request", onRequest);
          return { before, done, after, seen, asked, max: seen.length ? Math.max(...seen) : 0, min: seen.length ? Math.min(...seen) : 0 };
        };
        const inCut = (seen) => seen.filter((t) => CUT_OUT.some(([s, e]) => t > s + 0.05 && t < e - 0.001));
        for (const [over, media] of [["the stream", stream], ["the single-file copy", single]]) {
          await put(metaKey(ids.edit), meta(ids.edit, { media: media(ids.edit), edits: EDIT }));
          const run = await playThrough(ids.edit, false);
          const wrong = inCut(run.seen);
          const detail = { n: run.seen.length, min: run.min, max: run.max, wrong: wrong.slice(0, 8), before: run.before, after: run.after };
          check(`an edit over ${over}, at 2×: never a moment of a cut, every kept range plays, it stops at the out`,
            run.done && run.after.rate === 2 && wrong.length === 0 && KEEP.every(([s, e]) => run.seen.some((t) => t >= s && t < e)) && run.max >= 17.8 && run.max <= 18.05, true, detail);
          check(`an edit over ${over}: the time reads the edited length, 0:00 / 0:12, then 0:12 / 0:12`, run.before.time === "0:00 / 0:12" && run.after.time === "0:12 / 0:12", true, detail);
        }

        // ── the edit's permanent copy, up (N63) ──
        await put(metaKey(ids.cut), meta(ids.cut, { media: single(ids.cut), edits: { ...EDIT, copy: EDIT_COPY } }));
        const copyRun = await playThrough(ids.cut, true);
        const jumps = copyRun.seen.filter((t, i) => i > 0 && t - copyRun.seen[i - 1] > 0.5);
        const original = copyRun.asked.filter((p) => /\/(master\.m3u8|stream\.m3u8|[^/]+\.m4s|video\.mp4)$/.test(p));
        check("the edit's copy up: the page plays it to its end with nothing skipped, its own first frame and length, and asks for nothing of the original",
          copyRun.done && jumps.length === 0 && copyRun.max >= 11.3 && copyRun.after.src.endsWith(`/${EDIT_COPY.file}`) && copyRun.before.poster?.endsWith(`/${EDIT_COPY.poster}`)
            && copyRun.before.time === "0:00 / 0:12" && original.length === 0, true, { done: copyRun.done, jumps, max: copyRun.max, before: copyRun.before, after: copyRun.after, original });
        const reach = await page.evaluate(async (id) => {
          const out = {};
          for (const f of ["master.m3u8", "v0/stream.m3u8", "v0/init.mp4", "v0/seg_00000.m4s", "video.mp4", "poster.jpg", "edit-1.mp4", "edit-1-poster.jpg"]) out[f] = (await fetch(`/m/${id}/${f}`, { method: "HEAD" })).status;
          return out;
        }, ids.cut);
        check("the edit's copy up: the original's stream, file and trimmed-off first frame answer 404, the copy and its first frame are served (step 3)",
          same(reach, { "master.m3u8": 404, "v0/stream.m3u8": 404, "v0/init.mp4": 404, "v0/seg_00000.m4s": 404, "video.mp4": 404, "poster.jpg": 404, "edit-1.mp4": 200, "edit-1-poster.jpg": 200 }), true, reach);

        // ── shroom:keep: the page handed new ranges (#59) ──
        await put(metaKey(ids.edit), meta(ids.edit, { media: stream(ids.edit) }));
        const handed = await playThrough(ids.edit, false, KEEP);
        const handedWrong = inCut(handed.seen);
        check("shroom:keep hands an unedited page the ranges: at 2×, never a moment of a cut, every range plays, it stops at the last one's end, the time reads 0:12",
          handed.done && handedWrong.length === 0 && KEEP.every(([s, e]) => handed.seen.some((t) => t >= s && t < e)) && handed.max >= 17.8 && handed.max <= 18.05
            && handed.before.time === "0:00 / 0:12" && handed.after.time === "0:12 / 0:12", true,
          { n: handed.seen.length, min: handed.min, max: handed.max, wrong: handedWrong.slice(0, 8), before: handed.before, after: handed.after });

        // ── a baked recording (B197): the edit became the video ──
        await put(metaKey(ids.bake), meta(ids.bake, { durationSec: EDIT_COPY.duration,
          media: { hls: null, mp4: `rec/${ids.bake}/${EDIT_COPY.file}`, poster: `rec/${ids.bake}/${EDIT_COPY.poster}`, width: 640, height: 400 } }));
        const bakeRun = await playThrough(ids.bake, true);
        const bakeJumps = bakeRun.seen.filter((t, i) => i > 0 && t - bakeRun.seen[i - 1] > 0.5);
        const bakeAsked = bakeRun.asked.filter((p) => /\/(master\.m3u8|stream\.m3u8|[^/]+\.m4s|video\.mp4)$/.test(p));
        check("a baked recording plays its one file to its end, its own first frame and length, and asks for nothing of a stream",
          bakeRun.done && bakeJumps.length === 0 && bakeRun.max >= 11.3 && bakeRun.after.src.endsWith(`/${EDIT_COPY.file}`) && bakeRun.before.poster?.endsWith(`/${EDIT_COPY.poster}`)
            && bakeRun.before.time === "0:00 / 0:12" && bakeAsked.length === 0, true, { done: bakeRun.done, jumps: bakeJumps, max: bakeRun.max, before: bakeRun.before, after: bakeRun.after, asked: bakeAsked });
        const bakeReach = await page.evaluate(async (id) => {
          const out = {};
          for (const f of ["master.m3u8", "v0/stream.m3u8", "v0/seg_00000.m4s", "video.mp4", "poster.jpg", "edit-1.mp4", "edit-1-poster.jpg"]) out[f] = (await fetch(`/m/${id}/${f}`, { method: "HEAD" })).status;
          return out;
        }, ids.bake);
        check("a baked recording: its own files are served, what the bake left behind answers 404",
          same(bakeReach, { "master.m3u8": 404, "v0/stream.m3u8": 404, "v0/seg_00000.m4s": 404, "video.mp4": 404, "poster.jpg": 404, "edit-1.mp4": 200, "edit-1-poster.jpg": 200 }), true, bakeReach);

        // ── the copy lands under an open page (N63, step 3) ──
        // Opened on the original, the edit applied — a recording this browser hasn't played,
        // so nothing of it is in its cache; the copy lands before the first play, so the
        // original's next piece answers 404 and the player moves over to the copy. Required
        // where hls.js plays the stream (it asks piece by piece); a native player may hold it all.
        await put(metaKey(ids.land), meta(ids.land, { media: stream(ids.land), edits: EDIT }));
        await page.goto(`${origin}/${ids.land}`);
        await page.waitForFunction(() => document.getElementById("v").readyState >= 1, null, { timeout: 15_000 }).catch(() => {});
        const byHlsJs = await page.evaluate(() => document.getElementById("v").currentSrc.startsWith("blob:"));
        await put(metaKey(ids.land), meta(ids.land, { media: stream(ids.land), edits: { ...EDIT, copy: EDIT_COPY } }));
        await page.click(".pp");
        const movedOver = await page.waitForFunction((file) => { const v = document.getElementById("v"); return v.currentSrc.endsWith(`/${file}`) && !v.paused && v.currentTime > 0.5; }, EDIT_COPY.file, { timeout: 30_000 }).then(() => true, () => false);
        const landed = await page.evaluate(() => ({ src: document.getElementById("v").currentSrc, t: document.getElementById("v").currentTime, paused: document.getElementById("v").paused, time: document.querySelector(".time").textContent }));
        check("the copy lands under an open page: the player moves over to it and plays on", movedOver, byHlsJs, { byHlsJs, ...landed });
        await page.evaluate(() => document.getElementById("v").pause());

        // ── a short window (the canvas's "The page — a short window") ──
        // A 4:3 recording in a 1280×640 window: the player, its controls and the mark's foot
        // all end inside the window, the title row and the foot share the player's width;
        // the waiting frame fits the same way. A tall window keeps the 960px column.
        const fits = async () => page.evaluate(() => {
          const r = (q) => document.querySelector(q)?.getBoundingClientRect();
          const stage = r(".player") ?? r(".wait"), ctl = r(".ctl"), foot = r(".foot"), main = r("main");
          return { vh: innerHeight, stage: stage && [Math.round(stage.width), Math.round(stage.bottom)], ctl: ctl && Math.round(ctl.bottom), foot: foot && [Math.round(foot.width), Math.round(foot.bottom)], main: main && Math.round(main.width), scroll: document.documentElement.scrollHeight > innerHeight + 1 };
        });
        const fourThree = { media: { hls: `rec/${ids.live}/master.m3u8`, mp4: null, poster: `rec/${ids.live}/poster.jpg`, width: 1024, height: 768 } };
        await put(metaKey(ids.live), meta(ids.live, fourThree));
        await page.setViewportSize({ width: 1280, height: 640 });
        await page.goto(`${origin}/${ids.live}`);
        const short = await fits();
        // For a person comparing with the canvas: PAGE_DRIVE_SHOTS=<folder> keeps each window.
        const shot = (n) => process.env.PAGE_DRIVE_SHOTS ? page.screenshot({ path: path.join(process.env.PAGE_DRIVE_SHOTS, `${name}-${n}.png`) }) : null;
        await shot("short-player");
        check("a short window: the player, its controls and the foot fit; the column takes the player's width",
          short.stage && short.stage[1] <= short.vh && short.ctl <= short.vh && short.foot && short.foot[1] <= short.vh && !short.scroll
            && Math.abs(short.main - short.stage[0]) <= 1 && Math.abs(short.foot[0] - short.stage[0]) <= 1 && short.stage[0] < 960, true, short);
        await put(metaKey(ids.wait), { ...uploading(ids.wait, 1, 4), ...fourThree });
        await page.goto(`${origin}/${ids.wait}`);
        const shortWait = await fits();
        await shot("short-waiting");
        check("a short window: the waiting frame and the foot fit", shortWait.stage && shortWait.stage[1] <= shortWait.vh && shortWait.foot?.[1] <= shortWait.vh && !shortWait.scroll, true, shortWait);
        await page.setViewportSize({ width: 1280, height: 1000 });
        await put(metaKey(ids.live), meta(ids.live, {}));
        await page.goto(`${origin}/${ids.live}`);
        const tall = await fits();
        await shot("tall-player");
        check("a tall window keeps the 960px column", tall.main === 960 && tall.stage?.[0] === 960, true, tall);
        // ── the whole frame (Bagrat's 16:10 take, 0.1.10: the bar hid its bottom row) ──
        // Each shape in each window, before a play and while one runs: the picture's box has
        // the recording's own shape (contain fills it — no bars, nothing cropped), it sits
        // whole inside the player, and the control bar is below it, never over it.
        const frame = () => page.evaluate(() => {
          const v = document.getElementById("v"), r = (e) => e.getBoundingClientRect();
          const box = r(v), player = r(document.querySelector(".player")), ctl = r(document.querySelector(".ctl"));
          return { natural: [v.videoWidth, v.videoHeight], box: [box.width, box.height], fit: getComputedStyle(v).objectFit,
            inside: box.left >= player.left - 0.5 && box.right <= player.right + 0.5 && box.top >= player.top - 0.5 && box.bottom <= player.bottom + 0.5,
            under: ctl.top >= box.bottom - 0.5, ctlShown: getComputedStyle(document.querySelector(".ctl")).opacity };
        });
        const whole = (f, w, h) => f.fit === "contain" && Math.abs(f.box[0] / f.box[1] - w / h) * f.box[1] <= 1.5 && f.inside && f.under;
        for (const [win, [vw, vh]] of Object.entries(WINDOWS)) {
          await page.setViewportSize({ width: vw, height: vh });
          for (const [shape, [id, size]] of Object.entries(SHAPES)) {
            const [w, h] = size.split("x").map(Number);
            await page.goto(`${origin}/${id}`);
            const known = await page.waitForFunction(() => document.getElementById("v").videoWidth > 0, null, { timeout: 15_000 }).then(() => true, () => false);
            const still = await frame();
            await shot(`frame-${shape.replace(":", "x")}-${win}`);
            await page.evaluate(() => document.getElementById("v").play().catch(() => {}));
            await page.waitForFunction(() => document.getElementById("v").currentTime > 0.5, null, { timeout: 10_000 }).catch(() => {});
            const moving = await frame();
            check(`the whole frame: ${shape} in a ${win} window, before and during a play`,
              known && still.natural[0] * h === still.natural[1] * w && whole(still, w, h) && whole(moving, w, h), true, { still, moving });
          }
        }
        await page.setViewportSize({ width: 1280, height: 720 });

        // ── the recipient's waiting faces ──
        await put(metaKey(ids.wait), uploading(ids.wait, 1, 4));
        await page.goto(`${origin}/${ids.wait}`);
        const wait = await page.evaluate(() => { const i = document.querySelector(".wait .bar i"); return { line: document.querySelector(".wait .line")?.textContent, width: i?.style.width, anim: i && getComputedStyle(i).animationName }; });
        check("on its way: the line in the frame, over a breathing bar at 25%", wait.line === COPY.onItsWay && wait.width === "25%" && wait.anim === "shroom-breathe");
        await put(metaKey(ids.stall), uploading(ids.stall, 3, 4, 10 * 60_000));
        await page.goto(`${origin}/${ids.stall}`);
        const stall = await page.evaluate(() => { const i = document.querySelector(".wait .bar i"); return { line: document.querySelector(".wait .line")?.textContent, width: i?.style.width, anim: i && getComputedStyle(i).animationName }; });
        check("stalled: its line, and the bar rests", stall.line === COPY.stalled && stall.width === "75%" && stall.anim === "none");

        // ── the tab icon (DECISIONS B109) ──
        const NAMED = [
          { rel: "icon", type: "image/svg+xml", sizes: "", at: "/favicon.svg", here: true, head: true },
          { rel: "icon", type: "image/png", sizes: "180x180", at: "/favicon.png", here: true, head: true },
          { rel: "apple-touch-icon", type: "", sizes: "", at: "/favicon.png", here: true, head: true },
        ];
        /** The icon as this browser has it: the lines it finds in the page, where each
         * points once the browser has resolved it, and every address loaded as a picture
         * and drawn — how much of a 64-pixel square it paints, and the colour of the
         * tile near its top edge. */
        const icon = () => page.evaluate(async () => {
          const named = [...document.querySelectorAll('link[rel~="icon"], link[rel~="apple-touch-icon"]')].map((l) => {
            const to = new URL(l.href);
            return { rel: l.rel, type: l.type, sizes: l.getAttribute("sizes") ?? "", at: to.pathname + to.search, here: to.origin === location.origin, head: l.parentNode === document.head };
          });
          const drawn = {};
          for (const src of ["/favicon.svg", "/favicon.png", "/favicon.ico"]) {
            try {
              const img = new Image();
              await new Promise((ok, no) => { img.onload = ok; img.onerror = () => no(new Error("not a picture")); img.src = src; });
              const c = document.createElement("canvas");
              c.width = c.height = 64;
              const g = c.getContext("2d");
              g.drawImage(img, 0, 0, 64, 64);
              const px = g.getImageData(0, 0, 64, 64).data;
              let painted = 0;
              for (let i = 3; i < px.length; i += 4) if (px[i] > 200) painted++;
              const top = (5 * 64 + 32) * 4;
              drawn[src] = { w: img.naturalWidth, h: img.naturalHeight, painted, tile: [px[top], px[top + 1], px[top + 2]] };
            } catch (e) { drawn[src] = { error: String(e?.message ?? e).slice(0, 120) }; }
          }
          return { named, drawn, join: !!document.querySelector("main.join #get") };
        });
        /** The landing's art: the dark tile (#2E3440) filling the square but for its round
         * corners; the PNG at its own 180 pixels, the SVG at whatever size it is asked for. */
        const art = (d, px) => !!d && !d.error && d.painted > 3600 && d.w > 0 && d.w === d.h && (!px || d.w === px)
          && Math.abs(d.tile[0] - 0x2e) <= 6 && Math.abs(d.tile[1] - 0x34) <= 6 && Math.abs(d.tile[2] - 0x40) <= 6;
        const draws = (seen) => art(seen.drawn["/favicon.svg"], 0) && art(seen.drawn["/favicon.png"], 180) && art(seen.drawn["/favicon.ico"], 180);
        await page.goto(`${origin}/${ids.live}`);
        const onVideo = await icon();
        check("icon: a video's page names the three, in its head, at this space's own address", same(onVideo.named, NAMED), true, onVideo.named);
        check("icon: from a video's page, /favicon.svg, /favicon.png and /favicon.ico each draw as the picture", draws(onVideo), true, onVideo.drawn);
        await page.goto(`${origin}/join/${CODE.ani}`);
        const onJoin = await icon();
        check("icon: the join page names the three, in its head, at this space's own address", onJoin.join && same(onJoin.named, NAMED), true, { join: onJoin.join, named: onJoin.named });
        check("icon: from the join page, /favicon.svg, /favicon.png and /favicon.ico each draw as the picture", draws(onJoin), true, onJoin.drawn);

        // ── the owner page ──
        const [cname, cvalue] = cookieFor(M1, OWNER_AIR).split("=");
        await context.addCookies([{ name: cname, value: cvalue, url: origin }]);
        await put(metaKey(ids.own), uploading(ids.own, 1, 4));
        await page.goto(`${origin}/${ids.own}`);
        const stage = await page.evaluate(() => ({ w: document.getElementById("w")?.textContent, spin: document.getElementById("b")?.className ?? null, turning: getComputedStyle(document.getElementById("b")).animationName, up: document.getElementById("u").textContent, body: document.body.innerText }));
        check("owner, uploading: the stage is empty, the one line speaks, a spinner turning in front of it", stage.w === "" && stage.spin === "spin" && stage.turning === "shroom-spin" && stage.up === COPY.finishing && !stage.body.includes(COPY.onItsWay), true, stage);
        check("owner, uploading: Edit in shroom waits for the drain (N69)", await page.evaluate(() => { const e = document.getElementById("ed"); return !!e && e.getClientRects().length === 0; }));
        await page.evaluate(() => { navigator.clipboard.writeText = () => Promise.reject(new Error("denied")); document.execCommand = () => false; });
        await page.click("#c");
        await page.waitForTimeout(300);
        check("a refused copy keeps Copy link", (await page.textContent("#c")) === COPY.copyLink);
        await page.evaluate(() => { navigator.clipboard.writeText = () => Promise.resolve(); });
        await page.click("#c");
        check("a real copy says Copied", await page.waitForFunction((w) => document.getElementById("c").textContent === w, COPY.copied, { timeout: 2000 }).then(() => true, () => false));
        await put(metaKey(ids.own), meta(ids.own, {}));
        const swapped = await page.waitForFunction(() => { const v = document.querySelector(".player video"); return v && (v.getAttribute("poster") || "").endsWith("/poster.jpg"); }, null, { timeout: 10_000 }).then(() => true, () => false);
        // Every file up (the record counts none), the line goes: no “Fully live ✓” on the page,
        // and no “Ready ✓” either; in a space that doesn't listen it simply fades (N65 (7)).
        await page.waitForFunction(() => document.getElementById("u").getClientRects().length === 0 && !document.getElementById("b"), null, { timeout: 10_000 }).catch(() => {});
        const afterSwap = await page.evaluate((fully) => ({ up: document.getElementById("u").getClientRects().length > 0, said: document.body.innerText.includes(fully), bar: !!document.getElementById("b"), dl: document.getElementById("dl").textContent, controls: document.querySelector(".player video").controls }), COPY.fullyLive);
        check("the go-live swaps the player in with its poster, and the upload line goes", swapped && !afterSwap.up && !afterSwap.said && !afterSwap.bar && !afterSwap.controls && afterSwap.dl === "", true, afterSwap);
        // Edit in shroom (N69): in at the drain, the last quiet icon, a plain link to shroom's editor.
        const ed = await page.evaluate(() => { const e = document.getElementById("ed"); return e && { shown: e.getClientRects().length > 0, href: e.getAttribute("href"), tip: e.getAttribute("data-tip"), last: e === e.parentElement.lastElementChild }; });
        check("Edit in shroom comes in at the drain, a link to shroom's editor (N69)", !!ed && ed.shown && ed.last && ed.href === `shroom://edit/${new URL(origin).host}/${ids.own}` && ed.tip === COPY.editInShroom, true, ed);
        await put(metaKey(ids.own), meta(ids.own, { media: { hls: `rec/${ids.own}/master.m3u8`, mp4: `rec/${ids.own}/video.mp4`, poster: `rec/${ids.own}/poster.jpg`, width: 640, height: 400 } }));
        check("Download appears when the MP4 lands", await page.waitForFunction((w) => document.querySelector("#dl a")?.textContent === w, COPY.download, { timeout: 20_000 }).then(() => true, () => false));

        // ── the mark (U3, N26; DEVIATIONS #41) ──
        // Switched off, nothing shows in the owner's corner either — the switch, its words
        // and the logo all go (Bagrat on 0.1.4, 2026-10-13); Settings turns it back on.
        const ON = { switch: "on", words: "shown", logo: "shown" }, OFF = { switch: null, words: "gone", logo: "gone" }, NONE = OFF, SEEN = { switch: null, words: "gone", logo: "shown" };
        /** Where the switch stands if it takes room on the screen, and whether its words and
         * the logo do. */
        const foot = (p) => p.evaluate(() => {
          const seen = (e) => !!e && e.getClientRects().length > 0 && getComputedStyle(e).visibility === "visible";
          const b = document.getElementById("mk"), m = document.querySelector("a.mark"), w = document.querySelector(".foot .switch");
          return { switch: seen(b) ? (b.checked ? "on" : "off") : null, words: seen(w) ? "shown" : "gone", logo: seen(m) ? "shown" : "gone" };
        });
        const markWrite = () => page.waitForResponse((r) => r.url().endsWith("/_shroom/mark"), { timeout: 5000 }).then((r) => r.status(), () => 0);
        await put(".shroom/mark.json", JSON.stringify({ on: true }));
        await page.goto(`${origin}/${ids.live}`);
        check("mark: on, the owner's page shows the switch beside the logo", same(await foot(page), ON), true, await foot(page));
        // The write is held back, so what shows next is the page's own doing.
        let release;
        const gate = new Promise((ok) => { release = ok; });
        await page.route("**/_shroom/mark", async (route) => { await gate; await route.continue(); });
        const held = markWrite();
        await page.click("#mk");
        check("mark: the flip takes the switch, its words and the logo away at once, before the write has answered", same(await foot(page), OFF), true, await foot(page));
        release();
        const wrote = await held;
        await page.unroute("**/_shroom/mark");
        check("mark: the write lands, and the corner stays empty", wrote === 200 && same(await foot(page), OFF), true, { wrote, ...(await foot(page)) });
        await page.reload();
        check("mark: after a reload the corner is still empty", same(await foot(page), OFF), true, await foot(page));
        // Someone else's browser: no cookie.
        const elsewhere = await browser.newContext();
        const guest = await elsewhere.newPage();
        await guest.goto(`${origin}/${ids.live}`);
        check("mark: off, a recipient's page carries none", same(await foot(guest), NONE), true, await foot(guest));
        check("a recipient's page has no Edit in shroom (N69)", !(await guest.$("#ed")));
        await guest.goto(`${origin}/${ids.live}?embed=1`);
        check("mark: off, the embed carries none", same(await foot(guest), NONE), true, await foot(guest));
        // Switched back on from elsewhere (Settings); this tab goes away and comes Back.
        await put(".shroom/mark.json", JSON.stringify({ on: true }));
        await page.goto(`${origin}/${ids.wait}`);
        await page.goBack();
        check("mark: after a Back the switch says what the space says", same(await foot(page), ON), true, await foot(page));
        // A write that never arrives.
        await page.route("**/_shroom/mark", (route) => route.abort());
        await page.click("#mk");
        const undone = await page.waitForFunction(() => document.getElementById("mk").checked, null, { timeout: 3000 }).then(() => true, () => false);
        await page.unroute("**/_shroom/mark");
        check("mark: a write that fails puts the switch and the logo back", undone && same(await foot(page), ON), true, await foot(page));
        // Off for real, from the page; then on again from Settings, and the page follows on
        // its next load.
        const answered = markWrite();
        await page.click("#mk");
        const offStatus = await answered;
        const gone = await foot(page);
        await guest.goto(`${origin}/${ids.live}`);
        const guestOff = await foot(guest);
        await put(".shroom/mark.json", JSON.stringify({ on: true }));
        await page.reload();
        check("mark: off from the page, on again from Settings — the page follows", offStatus === 200 && same(gone, OFF) && same(guestOff, NONE) && same(await foot(page), ON), true, { offStatus, gone, guestOff, ...(await foot(page)) });
        await guest.goto(`${origin}/${ids.live}`);
        check("mark: on, a recipient's page carries it again", same(await foot(guest), SEEN), true, await foot(guest));
        await guest.goto(`${origin}/${ids.live}?embed=1`);
        check("mark: on, the embed carries it again", same(await foot(guest), SEEN), true, await foot(guest));
        await elsewhere.close();
        await put(".shroom/mark.json", JSON.stringify({ on: true }));

        await put(metaKey(ids.gone), uploading(ids.gone, 1, 4));
        await page.goto(`${origin}/${ids.gone}`);
        await put(metaKey(ids.gone), JSON.stringify({ schema: 1, id: ids.gone, state: "deleted", title: "x" }));
        check("a delete swaps the page to the not-found line (B141)", await page.waitForFunction((w) => document.querySelector("main").innerText.trim() === w, COPY.notFound, { timeout: 6000 }).then(() => true, () => false));
        const bytes = await page.evaluate(async (id) => (await fetch(`/m/${id}/master.m3u8`)).status, ids.gone);
        check("a tombstoned recording's playlist answers 404", bytes === 404);

        await put(metaKey(ids.off), uploading(ids.off, 1, 4));
        await page.goto(`${origin}/${ids.off}`);
        await put(metaKey(ids.off), meta(ids.off, { media: null }));
        check("unavailable: its line in the frame, the spinner gone", await page.waitForFunction((w) => document.getElementById("w")?.textContent === w && getComputedStyle(document.getElementById("b")).display === "none", COPY.unavailable, { timeout: 6000 }).then(() => true, () => false));

        // ── the voice cuts (N65, DECISIONS B200) ──
        const WORDS = { v1: "Hello, that is shroom. Shroom cut.", v3: "The second one is, um, I forget what it's called." };
        const voiceEdits = () => ({ version: 1, source_duration: DURATION, in: 0, out: DURATION, rev: 1, voice: { planned_at: Date.now() - 5000, cues: 2 },
          cuts: [
            { id: "v1", start: 1.2, end: 3.4, kind: "retake", confidence: "high", applied: true, removed: WORDS.v1, resumes: "Hello, this is shroom." },
            { id: "v2", start: 12, end: 13.1, kind: "cue-only", confidence: "high", applied: true, removed: "Shroom cut.", resumes: "Let's move on to the Library." },
            { id: "v3", start: 9.6, end: 13.1, kind: "line-before-cue", confidence: "low", applied: false, leads: "The settings window has three tabs.", removed: `${WORDS.v3} Shroom cut.`, resumes: "Let's move on to the Library." },
          ] });
        const strip = () => page.evaluate(() => {
          const s = document.getElementById("cuts"), p = document.querySelector(".player");
          if (!s || s.hidden) return null;
          return {
            below: !!p && s.getBoundingClientRect().top >= p.getBoundingClientRect().bottom - 1,
            rows: [...s.querySelectorAll("li")].map((li) => ({ id: li.dataset.cut, cls: li.className, button: li.querySelector("button").textContent, said: li.querySelector("q").textContent, struck: getComputedStyle(li.querySelector("q .w")).textDecorationLine.includes("line-through"),
              around: [...li.querySelectorAll("q .by")].some((e) => getComputedStyle(e).textDecorationLine.includes("line-through")) ? "struck" : "plain", asks: li.querySelector(".why")?.getClientRects().length > 0 })),
            unsaved: !s.querySelector(".unsaved").hidden,
          };
        });
        const row = (id, button) => page.waitForFunction(([i, w]) => document.querySelector(`li[data-cut="${i}"] button`)?.textContent === w, [id, button], { timeout: 3000 }).then(() => true, () => false);
        const pressed = () => page.waitForResponse((r) => r.url().endsWith(`/_shroom/edits/${ids.cuts}`) && r.request().method() === "POST", { timeout: 5000 }).then((r) => r.status(), () => 0);
        const kept = async () => JSON.parse(await (await env.BUCKET.get(metaKey(ids.cuts))).text()).edits;
        await page.setViewportSize({ width: 1280, height: 1000 });
        await put(metaKey(ids.cuts), meta(ids.cuts, { edits: voiceEdits() }));
        await page.goto(`${origin}/${ids.cuts}`);
        await page.evaluate(() => { window.__keeps = []; document.addEventListener("shroom:keep", (e) => window.__keeps.push(e.detail)); });
        const shown = await strip();
        check("cuts: under the player, a row per voice cut — an applied one struck through with Put back, the offered line with Cut it too, each between the words around it, unstruck",
          !!shown?.below && same(shown.rows.map((r) => [r.id, r.cls, r.button, r.struck, r.around, r.said, r.asks]), [
            ["v1", "cut on", COPY.putBack, true, "plain", `“${WORDS.v1} Hello, this is shroom.”`, false],
            ["v3", "cut offer", COPY.cutItToo, false, "plain", `“…The settings window has three tabs. ${WORDS.v3} Let's move on to the Library.”`, true],
          ]), true, shown);
        await shot("owner-cuts");
        await page.emulateMedia({ colorScheme: "dark" });
        await shot("owner-cuts-dark");
        await page.emulateMedia({ colorScheme: "light" });
        let answer = pressed();
        await page.click('li[data-cut="v1"] button');
        const putBack = { status: await answer, row: await row("v1", COPY.cutAgain) };
        const afterBack = await strip();
        check("cuts: Put back saves at once — the row reads Cut again, its words no longer struck; the record and the player follow",
          putBack.status === 200 && putBack.row && afterBack?.rows[0].cls === "cut" && !afterBack.rows[0].struck && (await kept()).cuts.find((c) => c.id === "v1").applied === false
            && same(await page.evaluate(() => window.__keeps.at(-1)), [[0, 12], [13.1, DURATION]]), true, { putBack, afterBack });
        answer = pressed();
        await page.click('li[data-cut="v3"] button');
        const too = { status: await answer, row: await row("v3", COPY.putBack) };
        const afterToo = await strip();
        check("cuts: Cut it too cuts the offered line — its row reads Put back, struck through, its question gone", too.status === 200 && too.row && afterToo?.rows[1].cls === "cut on" && afterToo.rows[1].struck && !afterToo.rows[1].asks, true, { too, afterToo });
        await page.route(`**/_shroom/edits/${ids.cuts}`, (route) => (route.request().method() === "POST" ? route.abort() : route.continue()));
        await page.click('li[data-cut="v1"] button');
        const failed = await page.waitForFunction(() => !document.querySelector("#cuts .unsaved").hidden, null, { timeout: 3000 }).then(() => strip(), () => null);
        await page.unroute(`**/_shroom/edits/${ids.cuts}`);
        await shot("owner-cuts-unsaved");
        check("cuts: a press that doesn't save leaves its row as it was and says so", !!failed?.unsaved && failed.rows[0].cls === "cut" && failed.rows[0].button === COPY.cutAgain && (await kept()).cuts.find((c) => c.id === "v1").applied === false, true, failed);
        const outsider = await browser.newContext();
        const stranger = await outsider.newPage();
        await stranger.goto(`${origin}/${ids.cuts}`);
        const theirs = await stranger.evaluate((w) => ({ strip: !!document.getElementById("cuts"), words: document.documentElement.outerHTML.includes(w) }), WORDS.v1);
        await outsider.close();
        check("cuts: a recipient's page carries no strip and no word that was cut", !theirs.strip && !theirs.words, true, theirs);
        // A space that listens has Cloudflare's AI bound to its page (N66).
        env.AI = {};
        await put(metaKey(ids.cuts), meta(ids.cuts, { publishedAt: iso(Date.now() - 2000) }));
        await page.goto(`${origin}/${ids.cuts}`);
        const coming = await page.evaluate(() => { const s = document.getElementById("cuts"); return !!s && s.hidden; });
        // While the take is heard (hearing): the page's one line stays under the player, its
        // spinner turning, until the cuts land.
        const finishing = () => page.evaluate(() => { const u = document.getElementById("ul"), b = document.getElementById("b"), p = document.querySelector(".player");
          return !!u && !u.hidden && u.getClientRects().length > 0 ? { words: document.getElementById("u").textContent, turning: !!b && b.className === "spin" && getComputedStyle(b).animationName === "shroom-spin",
            below: !!p && u.getBoundingClientRect().top >= p.getBoundingClientRect().bottom - 1 } : null; });
        const lineGone = () => page.waitForFunction(() => { const u = document.getElementById("ul"); return !u || u.hidden; }, null, { timeout: 4000 }).then(() => true, () => false);
        const heard = await finishing();
        await shot("owner-finishing");
        await put(metaKey(ids.cuts), meta(ids.cuts, { publishedAt: iso(Date.now() - 4000), edits: voiceEdits() }));
        const stripLanded = await page.waitForFunction(() => { const s = document.getElementById("cuts"); return !!s && !s.hidden && s.querySelectorAll("li").length === 2; }, null, { timeout: 10_000 }).then(() => true, () => false);
        const goneWithStrip = await lineGone();
        check("cuts: a page open while the cues are coming keeps its one line, its spinner turning, then takes the strip in when they land, the line gone",
          coming && heard?.words === COPY.finishing && heard.turning && heard.below && stripLanded && goneWithStrip, true, { coming, heard, landed: stripLanded, goneWithStrip });
        // The hint (owner-page-cue-hint): a take with no voice cuts, in a space that listens.
        const hintShown = () => page.evaluate(() => { const h = document.querySelector(".cue-hint"); return !!h && !h.hidden && h.getClientRects().length > 0; });
        const forget = () => page.evaluate(() => { try { localStorage.removeItem("shroom:cue-hint"); } catch { /* none kept */ } });
        const noCues = () => ({ ...voiceEdits(), cuts: [] });
        await forget();
        await put(metaKey(ids.cuts), meta(ids.cuts, { edits: noCues() }));
        await page.goto(`${origin}/${ids.cuts}`);
        const shownFirst = await hintShown();
        await shot("owner-cue-hint");
        await page.click(".cue-hint button");
        const goneOnPress = !(await hintShown());
        await page.reload();
        const goneForGood = !(await hintShown());
        // A strip seen counts as a cue used: the hint doesn't come back.
        await forget();
        await put(metaKey(ids.cuts), meta(ids.cuts, { edits: voiceEdits() }));
        await page.goto(`${origin}/${ids.cuts}`);
        await put(metaKey(ids.cuts), meta(ids.cuts, { edits: noCues() }));
        await page.goto(`${origin}/${ids.cuts}`);
        const afterStrip = await hintShown();
        check("cuts: the hint shows under the player on a take with no voice cuts; Got it puts it away for good, as a strip seen does",
          shownFirst && goneOnPress && goneForGood && !afterStrip, true, { shownFirst, goneOnPress, goneForGood, afterStrip });
        // The line ends without cuts: the Mac found none (the hint, while it's due, takes its
        // place), or five minutes passed by the page's own count, whatever the status says.
        await forget();
        await put(metaKey(ids.cuts), meta(ids.cuts, { publishedAt: iso(Date.now() - 2000) }));
        await page.goto(`${origin}/${ids.cuts}`);
        const firstLine = await finishing();
        const hintHeld = !(await hintShown());
        await put(metaKey(ids.cuts), meta(ids.cuts, { publishedAt: iso(Date.now() - 4000), edits: noCues() }));
        const noneGone = await lineGone();
        const hintAfter = await page.waitForFunction(() => { const h = document.querySelector(".cue-hint"); return !!h && !h.hidden; }, null, { timeout: 3000 }).then(() => true, () => false);
        await page.route(`**/_shroom/status/${ids.cuts}`, async (route) => { const res = await route.fetch(); route.fulfill({ response: res, json: { ...(await res.json()), cuts: "coming" } }); });
        await put(metaKey(ids.cuts), meta(ids.cuts, { publishedAt: iso(Date.now() - VOICE_WAIT_MS + 2500) }));
        await page.goto(`${origin}/${ids.cuts}`);
        const late = await finishing();
        const ownCount = await lineGone();
        await page.unroute(`**/_shroom/status/${ids.cuts}`);
        delete env.AI;
        check("cuts: the one line fades when the Mac finds nothing to cut, the hint in its place, and by its own count when its five minutes are up",
          firstLine?.words === COPY.finishing && hintHeld && noneGone && hintAfter && !!late && ownCount, true, { firstLine, hintHeld, noneGone, hintAfter, late, ownCount });
        await context.clearCookies();
      } finally {
        await browser.close();
      }
      out.ok = out.errors.length === 0 && Object.values(out.checks).every((c) => c.ok || !c.required);
    }
    summary.ok = Object.values(summary.engines).every((e) => e.ok);
  } finally {
    server.close();
  }
}

main().catch((e) => { summary.error = String(e?.message ?? e).slice(0, 500); }).finally(() => {
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : 1);
});
