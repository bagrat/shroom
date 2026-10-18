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
//   bytes      a tombstoned recording's playlist answers 404
//
// Prints one JSON summary and exits 0 only when every required check passed.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { chromium, webkit } from "playwright";
import { COPY, handle } from "../src/worker.js";
import { CODE, INV, M1, OWNER_AIR, OWNER_MAC, cookieFor, metaKey, roster, sha, space } from "../test/space-fixture.js";

const here = path.dirname(new URL(import.meta.url).pathname);
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-page-drive-"));
const summary = { ok: false, engines: {} };
const ids = { live: "LiveLiveLiveLive", wait: "WaitWaitWaitWait", stall: "StalStalStalStal", own: "OwnrOwnrOwnrOwnr", gone: "GoneGoneGoneGone", off: "OffxOffxOffxOffx" };
const DURATION = 20;
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
        await page.keyboard.press("k");
        // The label follows the pause event, a task after `paused` flips.
        const kPaused = await page.waitForFunction((w) => document.getElementById("v").paused && document.querySelector(".pp").getAttribute("aria-label") === w, COPY.play, { timeout: 3000 }).then(() => true, () => false);
        check("K pauses, the label says Play", kPaused, true, await page.evaluate(() => ({ paused: document.getElementById("v").paused, label: document.querySelector(".pp").getAttribute("aria-label"), active: document.activeElement?.className })));
        await page.click(".fs");
        const full = await page.waitForFunction(() => document.fullscreenElement === document.querySelector(".player"), null, { timeout: 3000 }).then(() => true, () => false);
        const exitLabel = full ? await page.evaluate(() => document.querySelector(".fs").getAttribute("aria-label")) : null;
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
        const stage = await page.evaluate(() => ({ w: document.getElementById("w")?.textContent, bar: !!document.getElementById("b"), up: document.getElementById("u").textContent, body: document.body.innerText }));
        check("owner, uploading: the stage is empty, the upload line speaks", stage.w === "" && stage.bar && stage.up.startsWith("Uploading — 25%") && !stage.body.includes(COPY.onItsWay));
        await page.evaluate(() => { navigator.clipboard.writeText = () => Promise.reject(new Error("denied")); document.execCommand = () => false; });
        await page.click("#c");
        await page.waitForTimeout(300);
        check("a refused copy keeps Copy link", (await page.textContent("#c")) === COPY.copyLink);
        await page.evaluate(() => { navigator.clipboard.writeText = () => Promise.resolve(); });
        await page.click("#c");
        check("a real copy says Copied", await page.waitForFunction((w) => document.getElementById("c").textContent === w, COPY.copied, { timeout: 2000 }).then(() => true, () => false));
        await put(metaKey(ids.own), meta(ids.own, {}));
        const swapped = await page.waitForFunction(() => { const v = document.querySelector(".player video"); return v && (v.getAttribute("poster") || "").endsWith("/poster.jpg"); }, null, { timeout: 10_000 }).then(() => true, () => false);
        // Every file up (the record counts none), the upload line goes: no “Fully live ✓” on the
        // page — “Ready ✓” for a few seconds first, then nothing (B142).
        await page.waitForFunction(() => document.getElementById("u").getClientRects().length === 0 && !document.getElementById("b"), null, { timeout: 10_000 }).catch(() => {});
        const afterSwap = await page.evaluate((fully) => ({ up: document.getElementById("u").getClientRects().length > 0, said: document.body.innerText.includes(fully), bar: !!document.getElementById("b"), dl: document.getElementById("dl").textContent, controls: document.querySelector(".player video").controls }), COPY.fullyLive);
        check("the go-live swaps the player in with its poster, and the upload line goes", swapped && !afterSwap.up && !afterSwap.said && !afterSwap.bar && !afterSwap.controls && afterSwap.dl === "", true, afterSwap);
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
        check("unavailable: its line in the frame, the bar gone", await page.waitForFunction((w) => document.getElementById("w")?.textContent === w && getComputedStyle(document.getElementById("b").parentNode).display === "none", COPY.unavailable, { timeout: 6000 }).then(() => true, () => false));
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
