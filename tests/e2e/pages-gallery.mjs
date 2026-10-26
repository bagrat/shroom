// The page's faces (Region 4's web-page states) as a person sees them: the real worker.js
// served over loopback from an in-memory space (worker/test/space-fixture.js — every secret
// a fixture), a real fMP4 stream behind the live faces, each face opened in headless WebKit,
// light and dark, screenshotted to tests/e2e/gallery-out/, and its said
// lines checked verbatim against spec/machine.json (the page follows the viewer's scheme, its
// player dark under both — both are shot). Never the network, never a real space.
//
//   node tests/e2e/pages-gallery.mjs
//   GALLERY_ONLY=seek-head-live,owner-icons-in-place node tests/e2e/pages-gallery.mjs   (some faces)
//
// Faces: owner-page (uploading, waiting-conn, waiting-space, fully-live), live, on-its-way,
// on-its-way-untitled, on-its-way-stalled, unavailable, removed, embed-face (live and
// removed); and a video the space has named (N66, N67): the owner's page named and waiting
// for the free day, a viewer's with chapters in the seek bar on a laptop and a phone, and its
// captions showing; and the owner's page left open, playing, while the name lands: the
// chapters and captions come in place, with no reload; and while its name is on its way, the
// title shimmering whole with no border, on a laptop and a phone, and the box a click brings up. And the owner editing the chapters (N86):
// Edit chapters shut and open, a rename, an add at the playhead and a remove, a time too near,
// the switch off, a video with none, a phone, and the icon greyed before the video plays. Prints one JSON summary; exits non-zero when a face is missing a line,
// shows another face's, or throws.

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, webkit } from "playwright";
import { handle } from "../../worker/src/worker.js";
import { M1, OWNER_AIR, HEALTH, cookieFor, metaKey, space } from "../../worker/test/space-fixture.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../..");
const out = path.join(here, "gallery-out");
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-pages-gallery-"));
const machine = JSON.parse(fs.readFileSync(path.join(repo, "spec/machine.json"), "utf8"));
const said = (id) => (machine.states[id]?.said ?? []).map((l) => l.text);
const summary = { ok: false, faces: {} };

const iso = (ms) => new Date(ms).toISOString();
const TITLE = "Where the 502s come from";
const media = (id, mp4 = true) => ({ hls: `rec/${id}/master.m3u8`, mp4: mp4 ? `rec/${id}/video.mp4` : null, poster: `rec/${id}/poster.jpg`, width: 640, height: 400 });
const meta = (id, over = {}) => JSON.stringify({ schema: 1, id, state: "live", titled: true, createdAt: iso(Date.now() - 60_000), title: TITLE, durationSec: 8, media: media(id), owner: M1, ...over });
const uploading = (id, done, total, ageMs = 1000, over = {}) => meta(id, { state: "uploading", media: null, progress: { done, total, updatedAt: iso(Date.now() - ageMs) }, ...over });
/** A spec line by its id: the words a face must show. */
const line = (state, id) => machine.states[state]?.said?.find((l) => l.id === id)?.text ?? `(no ${id} in the spec)`;
/** The line that said where the name came from: off the owner's page (Bagrat, 2026-10-25). */
const NAMED_LINE = "Named from what you said.";

// A video the space has named (N66, N67): three minutes, an invented script heard as the
// Worker stores it ([word, start, end] on the recording's clock), the name and chapters the
// naming step wrote, and captions on.
const AI_SECS = 183;
const AI_TITLE = "Sign up screen, invoice bug, trial numbers";
const AI_SAID = [
  [0, "Sign up screen changes", "Hi, this is a quick look at the sign up screen. We moved the email field to the top. The password rules now show while you type. And the button says create account instead of submit."],
  [68, "Invoice page bug details", "Next, the invoice page bug. When a customer pays in two currencies, the total shows the wrong symbol. It only happens in the PDF, not on the page."],
  [114, "Trial results and metrics", "Now the trial numbers. Four hundred people started a trial last week. About one in five turned on billing."],
  [157, "Next steps and wrap up", "Next steps. I will fix the invoice bug tomorrow. Let me know what you think."],
];
/** Each chapter's sentences spread over its span, a word at a time. */
function aiWords() {
  const words = [];
  AI_SAID.forEach(([from, , text], i) => {
    const to = i + 1 < AI_SAID.length ? AI_SAID[i + 1][0] : AI_SECS;
    const ws = text.split(" ");
    const step = Math.min(0.9, (to - from - 2) / ws.length);
    ws.forEach((w, j) => words.push([w, +(from + 0.5 + j * step).toFixed(2), +(from + 0.5 + j * step + step * 0.85).toFixed(2)]));
  });
  return words;
}
const AI_TRANSCRIPT = { v: 1, model: "@cf/openai/whisper-large-v3-turbo", language: "en", duration: AI_SECS, heard_through: AI_SECS, done: true, windows: 4, clock: null, words: aiWords(), tail: [] };
/** A take that went live seconds ago in a space that listens, its name still on its way (N82). */
const naming = (id) => meta(id, { durationSec: AI_SECS, title: "Untitled — Oct 26", titled: false, createdAt: iso(Date.now() - 60_000), publishedAt: iso(Date.now() - 10_000) });
/** The title shimmering with no border, its line the one given (N84). */
const shimmering = (want) => async (page) => {
  const s = await page.evaluate(() => { const t = document.getElementById("t"), cs = getComputedStyle(t); return { naming: t.hasAttribute("data-naming"), line: t.placeholder, border: cs.borderTopColor, sliding: cs.animationName }; });
  return s.naming && s.line === want && /rgba\(0, 0, 0, 0\)|transparent/.test(s.border) && s.sliding === "shroom-shim" ? [] : [`the borderless shimmer saying “${want}” (got ${JSON.stringify(s)})`];
};
const named = (id, over = {}) => meta(id, {
  durationSec: AI_SECS, title: AI_TITLE, titled: false, media: { ...media(id), mp4: null },
  named: { by: "ai", model: "@cf/openai/gpt-oss-120b", at: iso(Date.now()), rev: "0:0", clock: null, words: AI_TRANSCRIPT.words.length, title: AI_TITLE, chapters: AI_SAID.map(([s, t]) => [s, t]), language: "en" },
  captions: { language: "en" }, ...over,
});
/** The seek bar's tip after the pointer rests over `at` seconds, or a finger drags the thumb there. */
async function tipAt(page, at, finger) {
  if (finger) {
    await page.evaluate((at) => {
      const sk = document.querySelector(".seek"), r = sk.getBoundingClientRect(), x = r.left + 7, y = r.top + r.height / 2;
      sk.dispatchEvent(new PointerEvent("pointerdown", { pointerType: "touch", clientX: x, clientY: y, bubbles: true }));
      sk.value = String(at);
      sk.dispatchEvent(new Event("input", { bubbles: true }));
    }, at);
  } else {
    const r = await page.$eval(".seek", (sk) => { const b = sk.getBoundingClientRect(); return { x: b.left, y: b.top + b.height / 2, w: b.width, max: +sk.max }; });
    // Where the head would stand at that time: its travel is inset by its radius (7 px).
    await page.mouse.move(r.x + 7 + ((r.w - 14) * (at + 0.5)) / r.max, r.y);
  }
  await page.waitForTimeout(200);
  return page.evaluate(() => {
    const tip = document.querySelector(".chtip");
    return tip && !tip.hidden ? { time: tip.querySelector(".cht")?.textContent, chapter: tip.lastChild?.textContent, grown: document.querySelectorAll(".chs i.on").length } : null;
  });
}
/** What a named page must do beyond its words: the tip names the chapter under the pointer
 * (or finger), the bar is cut one section a chapter, Captions is a button named by the spec. */
const chaptered = (finger) => async (page) => {
  const missing = [];
  const sections = await page.$$eval(".chs i", (xs) => xs.length);
  if (sections !== AI_SAID.length) missing.push(`${AI_SAID.length} sections in the seek bar (got ${sections})`);
  const tip = await tipAt(page, 122, finger);
  if (tip?.time !== "2:02" || tip?.chapter !== AI_SAID[2][1] || tip?.grown !== 1) missing.push(`the tip “2:02 ${AI_SAID[2][1]}” (got ${JSON.stringify(tip)})`);
  const cc = await page.$eval(".cc", (b) => b.getAttribute("aria-label")).catch(() => null);
  if (cc !== line("live", "live.captions")) missing.push(`the Captions button (got ${cc})`);
  const spoken = await page.$eval(".seek", (sk) => sk.getAttribute("aria-valuetext"));
  if (!spoken.endsWith(`, ${finger ? AI_SAID[2][1] : AI_SAID[0][1]}`)) missing.push(`the bar's spoken value naming the chapter (got “${spoken}”)`);
  return missing;
};

/** Where the seek head is painted, against the player's box (Bagrat, 2026-10-25: at the end its
 * right edge meets the box's right edge; at the start its left edge, the left). Read from the
 * pixels of the bar with the picture hidden: the head is the only near-white there.
 * @returns {Promise<{ left: number, right: number, width: number } | null>} head edges from the box's left */
async function headIn(page) {
  const r = await page.evaluate(() => {
    const p = document.querySelector(".player").getBoundingClientRect(), s = document.querySelector(".seek").getBoundingClientRect();
    document.querySelector("video").style.visibility = "hidden";
    return { x: p.left, w: p.width, y: s.top, h: s.height };
  });
  const png = await page.screenshot({ clip: { x: r.x, y: r.y, width: r.w, height: r.h } });
  await page.evaluate(() => { document.querySelector("video").style.visibility = ""; });
  return page.evaluate(async ({ b64, w }) => {
    const img = new Image(); img.src = "data:image/png;base64," + b64; await img.decode();
    const k = document.createElement("canvas"); k.width = img.naturalWidth; k.height = img.naturalHeight;
    const x = k.getContext("2d"); x.drawImage(img, 0, 0);
    const d = x.getImageData(0, 0, k.width, k.height).data, scale = k.width / w;
    let lo = Infinity, hi = -1;
    for (let i = 0; i < d.length; i += 4) if (d[i] > 215 && d[i + 1] > 215 && d[i + 2] > 215) { const c = (i / 4) % k.width; lo = Math.min(lo, c); hi = Math.max(hi, c); }
    return hi < 0 ? null : { left: lo / scale, right: (hi + 1) / scale, width: w };
  }, { b64: png.toString("base64"), w: r.w });
}
/** The colour of the bar's last 3 px, on its centre line: a fully loaded bar reads loaded to
 * its end (review of B250: it kept a 7 px stub of the unloaded colour). */
async function endOfBar(page) {
  const r = await page.evaluate(() => {
    const p = document.querySelector(".player").getBoundingClientRect(), s = document.querySelector(".seek").getBoundingClientRect(), cs = getComputedStyle(document.querySelector(".player"));
    return { x: p.right - 3, y: s.top + s.height / 2 - 1, buf: cs.getPropertyValue("--player-buf").trim(), track: cs.getPropertyValue("--player-track").trim() };
  });
  const png = await page.screenshot({ clip: { x: r.x, y: r.y, width: 3, height: 2 } });
  const px = await page.evaluate(async (b64) => {
    const img = new Image(); img.src = "data:image/png;base64," + b64; await img.decode();
    const k = document.createElement("canvas"); k.width = img.naturalWidth; k.height = img.naturalHeight;
    const x = k.getContext("2d"); x.drawImage(img, 0, 0); const d = x.getImageData(0, 0, 1, 1).data; return [d[0], d[1], d[2]];
  }, png.toString("base64"));
  const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const near = (a, b) => a.every((v, i) => Math.abs(v - b[i]) <= 14);
  return { px, buffered: near(px, hex(r.buf)), unloaded: near(px, hex(r.track)) };
}
/** The head at 0:00 and at the very end, on a laptop and a phone: inside the box, touching
 * its edge (within a pixel of anti-aliasing); a fully loaded bar reads loaded to its end;
 * with chapters (`starts`, past the first), each boundary on the head's travel, the first
 * and last sections reaching the box's ends. */
const headInside = (starts = null) => async (page) => {
  const missing = [];
  // Loaded first: the player fetches its stream on the first play.
  await page.evaluate(() => { const v = document.querySelector("video"); v.muted = true; return v.play().catch(() => {}); });
  const known = await page.waitForFunction(() => { const v = document.querySelector("video"); return isFinite(v.duration) && v.duration > 0 && v.readyState >= 1; }, null, { timeout: 15000 }).then(() => true, () => false);
  if (!known) return ["the video's length, to seek to its end"];
  for (const [width, height] of [[900, 640], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await page.evaluate(() => { const v = document.querySelector("video"); v.pause(); v.currentTime = 0; });
    await page.waitForTimeout(300);
    const start = await headIn(page);
    if (!start || Math.abs(start.left) > 1) missing.push(`at ${width}px, 0:00: the head's left edge on the box's (got ${JSON.stringify(start)})`);
    const ended = await page.evaluate(() => new Promise((ok) => {
      const v = document.querySelector("video");
      v.muted = true; v.addEventListener("ended", () => ok(true), { once: true });
      v.currentTime = Math.max(0, v.duration - 0.4); v.play().catch(() => ok(false)); setTimeout(() => ok(v.ended), 8000);
    }));
    await page.waitForTimeout(400);
    const end = await headIn(page);
    if (!ended || !end || Math.abs(end.right - end.width) > 1) missing.push(`at ${width}px, the end: the head's right edge on the box's (ended ${ended}, got ${JSON.stringify(end)})`);
    if (starts) {
      const ch = await page.evaluate(() => {
        const xs = [...document.querySelectorAll(".chs i")].map((x) => x.getBoundingClientRect()), p = document.querySelector(".player").getBoundingClientRect(), d = +document.querySelector(".seek").max;
        return { first: xs[0].left - p.left, last: p.right - xs[xs.length - 1].right, gaps: xs.slice(1).map((x, i) => (xs[i].right + x.left) / 2 - p.left), w: p.width, d };
      });
      const want = starts.map((t) => 7 + (t / ch.d) * (ch.w - 14));
      if (Math.abs(ch.first) > 0.5 || Math.abs(ch.last) > 0.5 || ch.gaps.length !== want.length || ch.gaps.some((g, i) => Math.abs(g - want[i]) > 1.5)) missing.push(`at ${width}px: the chapters' boundaries on the head's travel, the ends at the box's (got ${JSON.stringify(ch)}, want ${JSON.stringify(want)})`);
    }
    // Loaded to its end, the head mid-way: the bar's last pixels are the loaded colour.
    const full = await page.evaluate(() => new Promise((ok) => {
      const v = document.querySelector("video"); v.pause(); v.currentTime = v.duration / 2;
      const t = setInterval(() => { const b = v.buffered; if (b.length && b.end(b.length - 1) >= v.duration - 0.2 && b.start(0) <= 0.2) { clearInterval(t); ok(true) } }, 200);
      setTimeout(() => { clearInterval(t); ok(false) }, 20000);
    }));
    await page.waitForTimeout(400);
    const tail = await endOfBar(page);
    if (full && !tail.buffered) missing.push(`at ${width}px: a fully loaded bar loaded to its end (got ${JSON.stringify(tail)})`);
  }
  await page.setViewportSize({ width: 900, height: 640 });
  return missing;
};
/** The owner's title icons, each where it stands: its box, and whether it's greyed. */
const iconsOf = (page) => page.evaluate(() => ["#c", "#th", "#dl a", "#ed"].map((q) => {
  const el = document.querySelector(q); if (!el) return { q, missing: true };
  const b = el.getBoundingClientRect();
  return { q, x: Math.round(b.left * 2) / 2, y: Math.round(b.top * 2) / 2, w: b.width, greyed: el.getAttribute("aria-disabled") === "true", href: el.getAttribute("href") };
}));
/** The owner's chapters panel (owner-page-chapters): its lines by id, and the two with a time or a name. */
const chLine = (id) => line("owner-page-chapters", `owner-page-chapters.${id}`);
const addAt = (t) => chLine("add-a-chapter-at").replace("1:20", t);
const removed = (name) => chLine("removed").replace("Trial results and metrics", name);
/** Opens the panel and waits for its rows. */
async function openChapters(page) {
  await page.click("#chb");
  await page.waitForFunction(() => document.querySelector("#chp .ch-add button")?.textContent, null, { timeout: 8000 }).catch(() => {});
}
/** Lets a save land. */
const settle = (page) => page.waitForTimeout(400);
/** Moves the playhead as a drag on the bar does. */
async function seekTo(page, at) {
  await page.evaluate((at) => { const sk = document.querySelector(".seek"); sk.value = String(at); sk.dispatchEvent(new Event("input", { bubbles: true })); document.querySelector("video").currentTime = at; }, at);
  await page.waitForTimeout(300);
}

// Each face: its id in spec/machine.json, the record behind it, how it's opened, and which of the
// spec's lines it must show (the lines that vary are pinned by the record: 2 of 5 is 40 %).
const FACES = [
  { face: "uploading", id: "OwnrUpldOwnrUpld", owner: true, record: (id) => uploading(id, 2, 5), lines: () => said("uploading") },
  { face: "waiting-conn", id: "OwnrWaitOwnrWait", owner: true, record: (id) => uploading(id, 3, 5, 10 * 60_000), lines: () => said("waiting-conn") },
  { face: "waiting-space", id: "OwnrSpacOwnrSpac", owner: true, blocked: true, record: (id) => uploading(id, 3, 5, 10 * 60_000), lines: () => [said("waiting-space")[0]] },
  // Every file up, the restyled page says nothing: the upload line goes (no “Fully live ✓”).
  { face: "fully-live", id: "OwnrLiveOwnrLive", owner: true, record: (id) => meta(id, { progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }), lines: () => [], absent: ["Fully live ✓"] }, // the line left the page (DEVIATIONS #46)
  // Copy thumbnail (N46): beside Copy link once the video plays; the page draws the still and
  // then the loop from the teaser, stores them, and a click reads “Copied” for a moment.
  { face: "copy-thumbnail", id: "OwnrThmbOwnrThmb", owner: true, record: (id) => meta(id, { media: { ...media(id), teaser: `rec/${id}/teaser.mp4` }, progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }),
    lines: () => [], click: "#th", after: "Copied", pictures: ["jpg"] },
  // Headless WebKit on Linux draws a video as nothing, so the page keeps the still there
  // (it never stores a blank loop); Chromium draws the teaser, and the loop is made too.
  { face: "copy-thumbnail-loop", id: "OwnrLoopOwnrLoop", owner: true, engine: "chromium", record: (id) => meta(id, { media: { ...media(id), teaser: `rec/${id}/teaser.mp4` }, progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }),
    lines: () => [], click: "#th", after: "Copied", wait: (env) => [...env.BUCKET.store.keys()].some((k) => k.startsWith("rec/OwnrLoopOwnrLoop/thumb-") && k.endsWith(".gif")), pictures: ["jpg", "gif"] },
  // The page opened at stop, while the video uploads: the record has no length yet. Once it
  // goes live the page draws the picture — and the picture carries the length (the chip in
  // its corner: light text on the dark pill), learned from the status (Bagrat, 0.1.15).
  { face: "copy-thumbnail-late", id: "OwnrLateOwnrLate", owner: true, engine: "chromium", record: (id) => uploading(id, 2, 5, 1000, { durationSec: null }),
    golive: (id) => meta(id, { media: { ...media(id), teaser: `rec/${id}/teaser.mp4` }, progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }),
    lines: () => [], wait: (env) => [...env.BUCKET.store.keys()].some((k) => k.startsWith("rec/OwnrLateOwnrLate/thumb-") && k.endsWith(".jpg")), pictures: ["jpg"], chip: true },
  // Copy thumbnail waits until its picture is ready (N46, Bagrat: “Yes disabled until ready”):
  // the teaser held back, the button stays greyed and its tooltip, hovered, says it's
  // preparing; once the loop is stored it wakes and says its name. Each tooltip sits 8px
  // below its button (“tooltips need padding between the button”).
  { face: "copy-thumbnail-preparing", id: "OwnrPrepOwnrPrep", owner: true, engine: "chromium", record: (id) => meta(id, { media: { ...media(id), teaser: `rec/${id}/teaser.mp4` }, progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }),
    lines: () => [], hold: "**/teaser.mp4", hover: { disabled: true, tip: said("owner-page").includes("Preparing the thumbnail…") ? "Preparing the thumbnail…" : "(not in the spec)" } },
  { face: "copy-thumbnail-ready", id: "OwnrRedyOwnrRedy", owner: true, engine: "chromium", record: (id) => meta(id, { media: { ...media(id), teaser: `rec/${id}/teaser.mp4` }, progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }),
    lines: () => [], wait: (env) => [...env.BUCKET.store.keys()].some((k) => k.startsWith("rec/OwnrRedyOwnrRedy/thumb-") && k.endsWith(".gif")), hover: { disabled: false, tip: "Copy thumbnail" } },
  { face: "live", id: "LiveLiveLiveLive", record: (id) => meta(id), lines: () => [TITLE] },
  // The viewer's Copy link and Copy thumbnail (N46): Copy thumbnail shows once the owner's
  // page has made a picture for the title, and the copy wraps it; a viewer never stores one.
  { face: "live-copied", id: "LiveCopyLiveCopy", record: (id) => meta(id, { thumb: { title: TITLE, jpg: "thumb-0123456789abcdef.jpg" } }), lines: () => [TITLE], click: "#th", after: "Copied", pictures: [] },
  { face: "live-no-picture", id: "LiveNoThLiveNoTh", record: (id) => meta(id), lines: () => [TITLE], hidden: "#th" },
  { face: "on-its-way", id: "WayxWayxWayxWayx", record: (id) => uploading(id, 1, 4, 1000, { title: "Untitled — Aug 21", titled: false }), lines: () => said("on-its-way") },
  { face: "on-its-way-untitled", id: "NeverHeardOfThis", record: null, lines: () => said("on-its-way-untitled").slice(0, 1), absent: [TITLE] },
  { face: "on-its-way-stalled", id: "StalStalStalStal", record: (id) => uploading(id, 3, 4, 10 * 60_000), lines: () => said("on-its-way-stalled") },
  { face: "unavailable", id: "OffxOffxOffxOffx", record: (id) => meta(id, { media: null }), lines: () => said("unavailable") },
  // The mark's words now show beside its logo, so the face carries both of its lines.
  { face: "removed", id: "GoneGoneGoneGone", record: (id) => JSON.stringify({ schema: 1, id, state: "deleted", title: TITLE }), lines: () => said("removed"), absent: [TITLE] },
  { face: "embed-face", id: "EmbdEmbdEmbdEmbd", embed: true, record: (id) => meta(id), lines: () => [], absent: [TITLE] },
  { face: "embed-face · removed", id: "EmbdGoneEmbdGone", embed: true, record: (id) => JSON.stringify({ schema: 1, id, state: "deleted", title: TITLE }), lines: () => said("removed").slice(0, 1) },
  // Named by the space (N66, N67; DECISIONS B209): the owner's title field holds the name
  // until it's typed over (B230), with no line under it (Bagrat, 2026-10-25: removed)…
  { face: "owner-named", id: "OwnrNamdOwnrNamd", owner: true, ai: true, record: (id) => named(id), lines: () => [`· ${AI_SAID[0][1]}`], absent: [NAMED_LINE],
    act: async (page) => ((await page.$eval("#t", (t) => t.value)) === AI_TITLE ? [] : ["the name in the title field"]) },
  // …and a page left open, playing, while the space names the video (Bagrat's hand test,
  // 2026-10-25: they showed only after a refresh): the name, the chapters in the bar and the
  // Captions button come in place, with no reload, and playback goes on.
  { face: "owner-named-in-place", id: "OwnrInPlOwnrInPl", owner: true, ai: true, listens: true, engine: "chromium",
    record: (id) => meta(id, { durationSec: AI_SECS, title: "Untitled — Oct 25", titled: false, media: { ...media(id), mp4: null }, createdAt: iso(Date.now() - 4 * 60_000) }),
    before: async (page) => {
      await page.evaluate(() => { window.__stayed = true; const v = document.querySelector("video"); v.muted = true; return v.play(); }).catch(() => {});
      await page.waitForFunction(() => document.querySelector("video").currentTime > 0.5, null, { timeout: 15000 }).catch(() => {});
    },
    golive: (id) => named(id), wait: () => true, pictures: [], lines: () => [], absent: [NAMED_LINE], act: async (page, env, f) => {
      const missing = [];
      const came = await page.waitForFunction(() => document.querySelectorAll(".chs i").length > 0 && document.querySelector(".cc"), null, { timeout: 30000, polling: 250 }).then(() => true, () => false);
      if (!came) missing.push("the chapters and Captions in place, without a reload");
      const s = await page.evaluate(() => { const v = document.querySelector("video"); return { stayed: window.__stayed === true, paused: v.paused, t: v.currentTime, title: document.getElementById("t").value, tracks: v.textTracks.length }; });
      if (!s.stayed) missing.push("the same page (it reloaded)");
      if (s.paused || s.t < 1) missing.push(`playback going on (paused ${s.paused} at ${s.t})`);
      if (s.title !== AI_TITLE) missing.push(`the name in the title field (got “${s.title}”)`);
      if (s.tracks !== 1) missing.push(`one captions track (got ${s.tracks})`);
      if (!came) return missing;
      missing.push(...(await chaptered(false)(page)));
      // Captions, pressed, show where words are said.
      await page.click(".cc");
      const cue = await page.waitForFunction(() => {
        const v = document.querySelector("video"), t = v.textTracks[0];
        if (!t || t.mode !== "showing" || !t.cues || !t.cues.length) return false;
        // It plays on: seek, then read the cue on a later look, once the seek has landed.
        if (v.currentTime < 70 || v.currentTime > 72) { v.currentTime = 70; return false; }
        return t.activeCues && t.activeCues.length ? t.activeCues[0].text : false;
      }, null, { timeout: 15000, polling: 250 }).then((h) => h.jsonValue(), () => null);
      if (!(cue && cue.startsWith("Next, the invoice"))) missing.push(`a caption at 1:10 (got ${JSON.stringify(cue)})`);
      // No line under the title once the name is in (Bagrat, 2026-10-25: removed).
      if ((await page.evaluate(() => document.body.innerText)).includes(NAMED_LINE)) missing.push("no named-from line once the name came");
      // A voice cut lands after the name (review of B241): 0:30–1:00 goes, and the page hears
      // what now plays as its edits pull does. The bar comes to what a reload draws: the second
      // chapter from 0:38, named at 0:40.
      await env.BUCKET.put(metaKey(f.id), named(f.id, { edits: { version: 1, source_duration: AI_SECS, in: 0, out: AI_SECS, cuts: [{ start: 30, end: 60, kind: "manual", applied: true }], rev: 2 } }));
      await page.evaluate(() => document.dispatchEvent(new CustomEvent("shroom:keep", { detail: [[0, 30], [60, 183]] })));
      // Each section as long as its chapter on the cut clock: 0:38, 0:46, 0:43 and 0:26.
      const redrawn = await page.waitForFunction(() => {
        const grow = [...document.querySelectorAll(".chs i")].map((x) => Math.round(parseFloat(getComputedStyle(x).flexGrow)));
        return +document.querySelector(".seek").max < 160 && JSON.stringify(grow) === "[38,46,43,26]";
      }, null, { timeout: 10000, polling: 250 }).then(() => true, () => false);
      const at40 = await tipAt(page, 40, false);
      if (!redrawn || at40?.chapter !== AI_SAID[1][1]) missing.push(`the bar redrawn on the cut clock: “${AI_SAID[1][1]}” at 0:40 (got ${JSON.stringify(at40)}, redrawn ${redrawn})`);
      return missing;
    } },
  // …or, the account's free day used up, that the name comes later; never an error.
  { face: "owner-naming-later", id: "OwnrDailyOwnrDly", owner: true, ai: true, record: (id) => meta(id, { durationSec: AI_SECS, title: "Untitled — Oct 18", titled: false, naming: { later: "daily" } }),
    lines: () => [line("owner-page", "owner-page.its-name-and-chapters-come")], absent: [NAMED_LINE] },
  // While the name is on its way (N82, N84): the whole title shimmers with no border, its line
  // in it (the short one on a phone); a click brings up the bordered box, the date its
  // placeholder. A laptop's window (1280) fits the long line; a narrower one takes the short
// one, as a phone does. The landed name is owner-named's.
  { face: "owner-naming-shimmer", id: "OwnrShimOwnrShim", owner: true, ai: true, listens: true, viewport: { width: 1280, height: 760 }, record: (id) => naming(id), lines: () => [], act: shimmering(line("hearing", "hearing.ai-is-naming")) },
  { face: "owner-naming-shimmer-phone", id: "OwnrShPhOwnrShPh", owner: true, ai: true, listens: true, phone: true, record: (id) => naming(id), lines: () => [], act: shimmering(line("hearing", "hearing.naming")) },
  { face: "owner-naming-clicked", id: "OwnrShClOwnrShCl", owner: true, ai: true, listens: true, record: (id) => naming(id), lines: () => [], act: async (page) => {
    await page.click("#t");
    const s = await page.evaluate(() => { const t = document.getElementById("t"), cs = getComputedStyle(t); return { naming: t.hasAttribute("data-naming"), line: t.placeholder, border: cs.borderTopColor, focused: document.activeElement === t }; });
    return s.naming || s.line !== "Untitled — Oct 26" || !s.focused || /rgba\(0, 0, 0, 0\)|transparent/.test(s.border) ? [`the bordered box with the date after a click (got ${JSON.stringify(s)})`] : [];
  } },
  // A viewer's page: the chapters cut into the seek bar, the one under the pointer named in a
  // tip; the one playing beside the time and in the bar's spoken value; Captions in the row.
  { face: "live-chapters", id: "LiveChapLiveChap", ai: true, record: (id) => named(id, { titled: true }), lines: () => [AI_TITLE, `· ${AI_SAID[0][1]}`], absent: [NAMED_LINE], act: chaptered(false) },
  // A phone has no hover: a finger on the bar names the chapter above the thumb. The row has
  // no room beside its five controls for the chapter's name.
  { face: "live-chapters-phone", id: "LivePhonLivePhon", ai: true, phone: true, record: (id) => named(id, { titled: true }), lines: () => [AI_TITLE], act: async (page) => {
    const missing = await chaptered(true)(page);
    if (await page.$eval(".chn", (n) => getComputedStyle(n).display !== "none")) missing.push("no chapter name in a phone's row");
    return missing;
  } },
  // Captions on: the button pressed, the track showing, a cue on screen where words are said.
  { face: "live-captions", id: "LiveCapsLiveCaps", ai: true, engine: "chromium", record: (id) => named(id, { titled: true }), lines: () => [AI_TITLE], act: async (page) => {
    await page.click(".cc");
    const shown = await page.waitForFunction(() => {
      const v = document.querySelector("video"), t = v.textTracks[0];
      if (!t || t.mode !== "showing" || !t.cues || !t.cues.length) return false;
      if (Math.abs(v.currentTime - 70) > 0.5) v.currentTime = 70;
      return t.activeCues && t.activeCues.length ? t.activeCues[0].text : false;
    }, null, { timeout: 15000, polling: 250 }).then((h) => h.jsonValue(), () => null);
    await page.waitForTimeout(600);
    const pressed = await page.$eval(".cc", (b) => b.getAttribute("aria-pressed"));
    return [...(pressed === "true" ? [] : ["Captions pressed"]), ...(shown && shown.startsWith("Next, the invoice") ? [] : [`a caption at 1:10 (got ${JSON.stringify(shown)})`])];
  } },
  // Bagrat, 2026-10-25: the icons beside the title came in one by one and the row flickered.
  // Opened while the video uploads, every one is there, the ones not ready greyed (a greyed
  // Download goes nowhere); once it's fully up they wake where they stand: nothing moves.
  { face: "owner-icons-in-place", id: "OwnrIconOwnrIcon", owner: true, engine: "chromium", record: (id) => uploading(id, 2, 5, 1000, { durationSec: 8 }),
    before: async (page) => {
      await page.evaluate((icons) => { window.__icons = icons; }, await iconsOf(page));
      if (await page.$("#dl a")) await page.click("#dl a", { force: true });
    },
    // The take's own shape (16:10) comes with the go-live; an open page keeps the box it opened with.
    golive: (id) => meta(id, { progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }), wait: () => true, pictures: [], lines: () => [], act: async (page) => {
      const missing = [];
      const before = await page.evaluate(() => window.__icons);
      if (!before) return ["the page left (a greyed Download went somewhere)"];
      for (const i of before) if (i.missing) missing.push(`${i.q} there from the first paint`);
      for (const q of ["#th", "#dl a", "#ed"]) if (!before.find((i) => i.q === q)?.greyed) missing.push(`${q} greyed while it uploads`);
      if (before.find((i) => i.q === "#dl a")?.href) missing.push("a greyed Download that goes nowhere");
      const woke = await page.waitForFunction(() => document.querySelector("#dl a").getAttribute("href") && !document.querySelector("#ed").getAttribute("aria-disabled"), null, { timeout: 20000, polling: 250 }).then(() => true, () => false);
      if (!woke) missing.push("Download and Edit in shroom awake once it's fully up");
      // At the wake, and again once the upload line has gone (the drain gives its room back).
      for (const when of ["awake", "the line gone"]) {
        if (when !== "awake") await page.waitForFunction(() => document.getElementById("ul").hidden, null, { timeout: 15000 }).then(() => page.waitForTimeout(600), () => missing.push("the upload line gone at the drain"));
        const after = await iconsOf(page);
        for (const a of after) {
          const b = before.find((i) => i.q === a.q);
          if (b && (b.x !== a.x || b.y !== a.y || Math.abs(b.w - a.w) > 0.5)) missing.push(`${a.q} where it stood, ${when} (${b.x},${b.y} → ${a.x},${a.y})`);
        }
      }
      return missing;
    } },
  // A page opened while an edit waits for its copy has nothing to draw Copy thumbnail from;
  // when the copy lands under it, the picture is made from the copy and the button wakes, with
  // no reload (2026-10-25: a button that never gets ready is a bug).
  { face: "owner-thumb-after-copy", id: "OwnrCopyOwnrCopy", owner: true, engine: "chromium",
    record: (id) => meta(id, { media: media(id, false), progress: { done: 9, total: 9, updatedAt: iso(Date.now()) }, edits: { version: 1, source_duration: 8, in: 0, out: 8, cuts: [{ start: 2, end: 3, kind: "manual", applied: true }], rev: 3 } }),
    before: async (page, env, f) => {
      const greyed = await page.$eval("#th", (b) => b.getAttribute("aria-disabled"));
      await page.evaluate((g) => { window.__greyed = g; window.__stayed = true; }, greyed);
      for (const [name, from] of [["edit-3.mp4", "teaser.mp4"], ["edit-3-poster.jpg", "poster.jpg"], ["edit-3-teaser.mp4", "teaser.mp4"]]) await env.BUCKET.put(`rec/${f.id}/${name}`, env.BUCKET.store.get(`rec/${f.id}/${from}`));
    },
    golive: (id) => meta(id, { media: media(id, false), progress: { done: 9, total: 9, updatedAt: iso(Date.now()) },
      edits: { version: 1, source_duration: 8, in: 0, out: 8, cuts: [{ start: 2, end: 3, kind: "manual", applied: true }], rev: 3, copy: { rev: 3, file: "edit-3.mp4", duration: 7, poster: "edit-3-poster.jpg", teaser: "edit-3-teaser.mp4" } } }),
    wait: () => true, pictures: [], lines: () => [], act: async (page, env, f) => {
      const missing = [];
      if (await page.evaluate(() => window.__greyed) !== "true") missing.push("Copy thumbnail greyed while the copy is on its way");
      const woke = await page.waitForFunction(() => !document.getElementById("th").getAttribute("aria-disabled"), null, { timeout: 45000, polling: 250 }).then(() => true, () => false);
      if (!woke) missing.push("Copy thumbnail awake once the copy is up, with no reload");
      if (!(await page.evaluate(() => window.__stayed))) missing.push("the same page (it reloaded)");
      const from = env.BUCKET.json(metaKey(f.id))?.thumb?.from;
      if (from !== "edit-3-poster.jpg") missing.push(`a picture drawn from the copy's first frame (got ${JSON.stringify(env.BUCKET.json(metaKey(f.id))?.thumb ?? null)})`);
      return missing;
    } },
  // The edit put back while its copy was on its way (every cut on again, as the strip or the
  // editor does): the page follows the first frame back to the recording's own and wakes.
  { face: "owner-thumb-after-undo", id: "OwnrUndoOwnrUndo", owner: true, engine: "chromium",
    record: (id) => meta(id, { media: media(id, false), progress: { done: 9, total: 9, updatedAt: iso(Date.now()) }, edits: { version: 1, source_duration: 8, in: 0, out: 8, cuts: [{ start: 0, end: 3, kind: "manual", applied: true }], rev: 3 } }),
    before: async (page) => { await page.evaluate((g) => { window.__greyed = g; window.__stayed = true; }, await page.$eval("#th", (b) => b.getAttribute("aria-disabled"))); },
    golive: (id) => meta(id, { media: media(id, false), progress: { done: 9, total: 9, updatedAt: iso(Date.now()) }, edits: { version: 1, source_duration: 8, in: 0, out: 8, cuts: [{ start: 0, end: 3, kind: "manual", applied: false }], rev: 4 } }),
    wait: () => true, pictures: [], lines: () => [], act: async (page, env, f) => {
      const missing = [];
      if (await page.evaluate(() => window.__greyed) !== "true") missing.push("Copy thumbnail greyed while the copy is on its way");
      // What plays changed (the page's own press, or the poll's pull, says so).
      await page.evaluate(() => document.dispatchEvent(new CustomEvent("shroom:keep", { detail: null })));
      const woke = await page.waitForFunction(() => !document.getElementById("th").getAttribute("aria-disabled"), null, { timeout: 45000, polling: 250 }).then(() => true, () => false);
      if (!woke) missing.push("Copy thumbnail awake once the edit is put back, with no reload");
      if (!(await page.evaluate(() => window.__stayed))) missing.push("the same page (it reloaded)");
      const from = env.BUCKET.json(metaKey(f.id))?.thumb?.from;
      if (from !== "poster.jpg") missing.push(`a picture drawn from the recording's own first frame (got ${JSON.stringify(env.BUCKET.json(metaKey(f.id))?.thumb ?? null)})`);
      return missing;
    } },
  // Bagrat, 2026-10-25: the seek head inside the bar, on every face that plays.
  { face: "seek-head-live", id: "SeekLiveSeekLive", record: (id) => meta(id, { media: media(id, false) }), lines: () => [TITLE], act: headInside() },
  { face: "seek-head-live-chromium", id: "SeekChroSeekChro", engine: "chromium", record: (id) => meta(id, { media: media(id, false) }), lines: () => [TITLE], act: headInside() },
  { face: "seek-head-owner", id: "SeekOwnrSeekOwnr", owner: true, record: (id) => meta(id, { media: media(id, false), progress: { done: 9, total: 9, updatedAt: iso(Date.now()) } }), lines: () => [], act: headInside() },
  { face: "seek-head-embed", id: "SeekEmbdSeekEmbd", embed: true, record: (id) => meta(id, { media: media(id, false) }), lines: () => [], absent: [TITLE], act: headInside() },
  { face: "seek-head-chapters", id: "SeekChapSeekChap", ai: true, engine: "chromium", record: (id) => named(id, { titled: true }), lines: () => [AI_TITLE], act: headInside(AI_SAID.slice(1).map(([t]) => t)) },
  // The owner edits the chapters (N86, owner-page-chapters): Edit chapters among the title's
  // icons opens the panel under the player; nothing for chapters shows until it's clicked.
  { face: "owner-chapters-closed", id: "ChapShutChapShut", owner: true, ai: true, engine: "chromium", record: (id) => named(id), lines: () => [], absent: [chLine("show-in-the-seek-bar"), chLine("done")],
    act: async (page) => {
      const missing = [];
      const head = await page.$$eval(".head > *", (xs) => xs.map((x) => x.id || x.className));
      if (head.indexOf("chb") < 0 || (head.includes("ed") && head.indexOf("chb") > head.indexOf("ed"))) missing.push(`Edit chapters before the scissors (got ${head.join(", ")})`);
      if (!(await page.$eval("#chp", (x) => x.hidden))) missing.push("the panel shut");
      await page.hover("#chb");
      await page.waitForTimeout(600);
      const tip = await page.$eval("#chb", (b) => getComputedStyle(b, "::after").content.replace(/^"|"$/g, ""));
      if (tip !== chLine("edit-chapters")) missing.push(`the tooltip “${chLine("edit-chapters")}” (got “${tip}”)`);
      return missing;
    } },
  { face: "owner-chapters-open", id: "ChapOpenChapOpen", owner: true, ai: true, engine: "chromium", full: true, record: (id) => named(id), before: openChapters,
    lines: () => [chLine("chapters"), chLine("show-in-the-seek-bar"), chLine("done"), addAt("0:00")], act: async (page) => {
      const missing = [];
      const rows = await page.$$eval("#chp .ch-row", (xs) => xs.map((x) => ({ at: x.querySelector(".ch-at")?.textContent, name: x.querySelector(".ch-name")?.value, move: !!x.querySelector(".ch-mv"), remove: x.querySelector(".ch-rm")?.getAttribute("aria-label") })));
      const want = AI_SAID.map(([s, t]) => [`${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`, t]);
      if (JSON.stringify(rows.map((r) => [r.at, r.name])) !== JSON.stringify(want)) missing.push(`the four chapters as rows (got ${JSON.stringify(rows)})`);
      if (rows[0]?.move || !rows[1]?.move) missing.push("the arrow on every row but the first");
      if (rows.some((r) => r.remove !== chLine("remove-this-chapter"))) missing.push("✕ named Remove this chapter");
      if ((await page.$eval("#chb", (b) => b.getAttribute("aria-expanded"))) !== "true") missing.push("the icon lit while open");
      return missing;
    } },
  { face: "owner-chapters-edited", saves: true, id: "ChapEditChapEdit", owner: true, ai: true, engine: "chromium", full: true, record: (id) => named(id),
    before: async (page) => {
      await openChapters(page);
      // Rename the second; add one at 1:30; remove the third.
      await page.fill("#chp .ch-row:nth-child(2) .ch-name", "The invoice bug");
      await page.press("#chp .ch-row:nth-child(2) .ch-name", "Enter");
      await settle(page);
      await seekTo(page, 90);
      await page.click("#chp .ch-add button");
      await page.keyboard.type("Questions");
      await page.keyboard.press("Enter");
      await settle(page);
      await page.click("#chp .ch-row:nth-child(4) .ch-rm");
      await settle(page);
    },
    lines: () => [removed(AI_SAID[2][1]), chLine("undo")], act: async (page) => {
      const missing = [];
      const names = await page.$$eval("#chp .ch-row", (xs) => xs.map((x) => x.querySelector(".ch-name")?.value ?? x.textContent));
      const want = [AI_SAID[0][1], "The invoice bug", "Questions", removed(AI_SAID[2][1]) + "Undo", AI_SAID[3][1]];
      if (JSON.stringify(names) !== JSON.stringify(want)) missing.push(`the rows after the edits (got ${JSON.stringify(names)})`);
      const bar = await page.$$eval(".chs i", (xs) => xs.length);
      if (bar !== 4) missing.push(`the bar cut again into 4 (got ${bar})`);
      const tip = await tipAt(page, 95, false);
      if (tip?.chapter !== "Questions") missing.push(`the tip naming the new chapter at 1:35 (got ${JSON.stringify(tip)})`);
      return missing;
    } },
  { face: "owner-chapters-short", saves: true, id: "ChapNearChapNear", owner: true, ai: true, engine: "chromium", full: true, record: (id) => named(id),
    before: async (page) => {
      await openChapters(page);
      await page.click("#chp .ch-row:nth-child(3) button.ch-at");
      await page.fill("#chp .ch-time", "1:10");
      await page.press("#chp .ch-time", "Enter");
      await page.waitForTimeout(300);
    }, lines: () => [chLine("needs-5-seconds")] },
  { face: "owner-chapters-off", saves: true, id: "ChapOffxChapOffx", owner: true, ai: true, engine: "chromium", full: true, record: (id) => named(id),
    before: async (page) => { await openChapters(page); await page.click("#chp .ch-sw"); await settle(page); },
    lines: () => [chLine("off-viewers-see")], act: async (page) => ((await page.$$eval(".player.ch", (xs) => xs.length)) ? ["the bar whole"] : []) },
  { face: "owner-chapters-none", saves: true, id: "ChapNoneChapNone", owner: true, engine: "chromium", full: true, record: (id) => meta(id), before: openChapters,
    lines: () => [chLine("no-chapters-yet"), chLine("add-the-first-chapter")], act: async (page) => {
      await page.click("#chp .ch-add button");
      const ph = await page.$eval("#chp .ch-name", (n) => (document.activeElement === n ? n.placeholder : "")).catch(() => "");
      return ph === chLine("name-this-chapter") ? [] : [`a row at 0:00, its name open (got “${ph}”)`];
    } },
  { face: "owner-chapters-phone", id: "ChapPhonChapPhon", owner: true, ai: true, engine: "chromium", phone: true, full: true, record: (id) => named(id), before: openChapters,
    lines: () => [chLine("chapters"), "In the seek bar"], absent: [chLine("show-in-the-seek-bar")] },
  { face: "owner-chapters-greyed", id: "ChapGreyChapGrey", owner: true, engine: "chromium", record: (id) => uploading(id, 2, 5), lines: () => [], act: async (page) => {
    const missing = [];
    if ((await page.$eval("#chb", (b) => b.getAttribute("aria-disabled"))) !== "true") missing.push("Edit chapters greyed before the video plays");
    await page.click("#chb", { force: true });
    if (!(await page.$eval("#chp", (x) => x.hidden))) missing.push("nothing opens while it's greyed");
    return missing;
  } },
];

/** A real 8 s fMP4 HLS stream and its poster, as the app uploads them. */
function encode() {
  const v0 = path.join(work, "v0");
  fs.mkdirSync(v0);
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x400:rate=30", "-f", "lavfi", "-i", "sine=frequency=440",
    "-t", "8", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-g", "30", "-c:a", "aac", "-f", "hls", "-hls_time", "2", "-hls_playlist_type", "vod",
    "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4", "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0 });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x400", "-frames:v", "1", path.join(work, "poster.jpg")]);
  fs.writeFileSync(path.join(work, "master.m3u8"), "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=640x400\nv0/stream.m3u8\n");
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x400:rate=30", "-t", "4", "-c:v", "libx264", "-pix_fmt", "yuv420p",
    "-movflags", "+faststart", "-an", path.join(work, "teaser.mp4")]);
  const files = { "video.mp4": new TextEncoder().encode("mp4"), "teaser.mp4": new Uint8Array(fs.readFileSync(path.join(work, "teaser.mp4"))) };
  for (const f of ["master.m3u8", "poster.jpg", ...fs.readdirSync(v0).map((n) => `v0/${n}`)]) files[f] = new Uint8Array(fs.readFileSync(path.join(work, f)));
  return files;
}

/** The named video's stream: three minutes, few frames (its seek bar and captions are the point). */
function encodeLong() {
  const dir = path.join(work, "long"), v0 = path.join(dir, "v0");
  fs.mkdirSync(v0, { recursive: true });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=5", "-f", "lavfi", "-i", "sine=frequency=330",
    "-t", String(AI_SECS), "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-g", "10", "-c:a", "aac", "-f", "hls", "-hls_time", "6", "-hls_playlist_type", "vod",
    "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4", "-hls_segment_filename", "seg_%05d.m4s", "stream.m3u8"], { cwd: v0 });
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360", "-frames:v", "1", path.join(dir, "poster.jpg")]);
  fs.writeFileSync(path.join(dir, "master.m3u8"), "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=600000,RESOLUTION=640x360\nv0/stream.m3u8\n");
  const files = {};
  for (const f of ["master.m3u8", "poster.jpg", ...fs.readdirSync(v0).map((n) => `v0/${n}`)]) files[f] = new Uint8Array(fs.readFileSync(path.join(dir, f)));
  return files;
}

/** The brightest pixel where the length sits (the picture's bottom-right corner). */
async function chipIn(browser, bytes) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const b64 = Buffer.from(bytes).toString("base64");
  const r = await page.evaluate(async (b64) => {
    const img = new Image(); img.src = "data:image/jpeg;base64," + b64; await img.decode();
    const W = img.naturalWidth, H = img.naturalHeight, k = document.createElement("canvas"); k.width = W; k.height = H;
    const x = k.getContext("2d"); x.drawImage(img, 0, 0);
    const d = x.getImageData(Math.round(W * 0.88), Math.round(H - W * 0.065), Math.round(W * 0.1), Math.round(W * 0.04)).data;
    let max = 0; for (let i = 0; i < d.length; i += 4) max = Math.max(max, 0.299 * d[i] + 0.587 * d[i + 1] + 0.114 * d[i + 2]);
    return { maxLuma: Math.round(max), width: W };
  }, b64);
  await context.close();
  return r;
}

async function main() {
  const only = process.env.GALLERY_ONLY ? process.env.GALLERY_ONLY.split(",") : null;
  if (only) FACES.splice(0, FACES.length, ...FACES.filter((f) => only.includes(f.face)));
  fs.mkdirSync(out, { recursive: true });
  const files = encode();
  const long = encodeLong();
  const env = space();
  await env.BUCKET.put(".shroom/player/hls.min.js", new Uint8Array(fs.readFileSync(path.join(repo, "worker/vendor/hls.min.js"))));
  for (const f of FACES) {
    for (const [name, bytes] of Object.entries(f.ai ? long : files)) await env.BUCKET.put(`rec/${f.id}/${name}`, bytes);
    if (f.record) await env.BUCKET.put(metaKey(f.id), f.record(f.id));
    if (f.ai) await env.BUCKET.put(`rec/${f.id}/ai/transcript.json`, JSON.stringify(AI_TRANSCRIPT));
  }
  const server = createServer(async (req, res) => {
    const chunks = [];
    for await (const c of req) chunks.push(c);
    const r = await handle(new Request(`http://127.0.0.1:${server.address().port}${req.url}`, { method: req.method, headers: req.headers, body: chunks.length ? Buffer.concat(chunks) : undefined }), env, Date.now());
    res.writeHead(r.status, Object.fromEntries(r.headers));
    if (r.body) for await (const c of r.body) res.write(c);
    res.end();
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await webkit.launch();
  /** @type {import("playwright").Browser | null} */
  let chrome = null;
  try {
    const [cname, cvalue] = cookieFor(M1, OWNER_AIR).split("=");
    for (const f of FACES) {
      const entry = (summary.faces[f.face] = { shots: [] });
      const backgrounds = [];
      if (f.blocked) await env.BUCKET.put(HEALTH, JSON.stringify({ v: 1, uploads: "blocked", at: iso(Date.now()) }));
      // A space with the AI on (its page asks for a name on its way); nothing here runs it.
      if (f.listens) env.AI = { run: async () => { throw new Error("the gallery runs no model"); } };
      for (const scheme of ["light", "dark"]) {
        if (f.golive) {
          await env.BUCKET.put(metaKey(f.id), f.record(f.id));
          for (const k of [...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${f.id}/thumb-`))) env.BUCKET.store.delete(k);
        }
        // A face that saves starts each scheme from its record as written.
        if (f.saves) await env.BUCKET.put(metaKey(f.id), f.record(f.id));
        const engine = f.engine === "chromium" ? (chrome ??= await chromium.launch()) : browser;
        const context = await engine.newContext({ colorScheme: scheme, ...(f.phone ? { viewport: { width: 390, height: 844 }, hasTouch: true } : { viewport: f.viewport ?? { width: 900, height: 640 } }) });
        if (f.owner) await context.addCookies([{ name: cname, value: cvalue, url: origin }]);
        const page = await context.newPage();
        if (f.hold) await page.route(f.hold, () => {});
        const errors = [];
        page.on("pageerror", (e) => errors.push(String(e.message).slice(0, 200)));
        await page.goto(`${origin}/${f.id}${f.embed ? "?embed=1" : ""}`);
        // A held request never lets the network go idle (and Copy thumbnail's 20 s guard would wake it).
        await page.waitForLoadState(f.hold ? "load" : "networkidle").catch(() => {});
        if (!f.record || f.record(f.id).includes('"live"')) await page.waitForTimeout(400);
        let clicked = true;
        if (f.before) await f.before(page, env, f);
        if (f.golive) {
          // Back to uploading for this scheme's pass, then live under the open page.
          await env.BUCKET.put(metaKey(f.id), f.golive(f.id));
          for (let t = 0; !f.wait(env) && t < 150; t++) await page.waitForTimeout(100);
        }
        if (f.click) {
          // The picture is made in the background; the copy waits for the still if it must.
          await page.waitForFunction(() => document.querySelector("#th") && !document.querySelector("#th").hidden && !document.querySelector("#th").getAttribute("aria-disabled"));
          await page.click(f.click);
          clicked = await page.waitForFunction((w) => document.body.innerText.includes(w), f.after, { timeout: 8000 }).then(() => true, () => false);
          for (let t = 0; f.wait && !f.wait(env) && t < 100; t++) await page.waitForTimeout(100);
        }
        const text = await page.evaluate(() => document.body.innerText);
        const missing = f.lines().filter((l) => !text.includes(l));
        if (!clicked) missing.push(f.after);
        if (f.act) missing.push(...(await f.act(page, env, f)));
        if (f.hidden && !(await page.evaluate((q) => document.querySelector(q)?.hidden, f.hidden))) missing.push(`${f.hidden} hidden`);
        if (f.hover) {
          // The still is stored first; then the button is hovered and its tooltip read.
          for (let t = 0; f.wait && !f.wait(env) && t < 150; t++) await page.waitForTimeout(100);
          if (!f.wait) await page.waitForTimeout(1500);
          await page.hover("#th");
          await page.waitForTimeout(600);
          const tip = await page.evaluate(() => {
            const b = document.querySelector("#th"), a = getComputedStyle(b, "::after");
            return { disabled: b.getAttribute("aria-disabled") === "true", text: a.content.replace(/^"|"$/g, ""), shown: a.opacity === "1", gap: parseFloat(a.top) - b.offsetHeight };
          });
          entry.tip = tip;
          if (tip.disabled !== f.hover.disabled) missing.push(f.hover.disabled ? "Copy thumbnail greyed" : "Copy thumbnail awake");
          if (tip.text !== f.hover.tip || !tip.shown) missing.push(`the tooltip “${f.hover.tip}” (got “${tip.text}”, shown ${tip.shown})`);
          if (tip.gap !== 8) missing.push(`8px between the button and its tooltip (got ${tip.gap})`);
          if (f.hover.disabled) {
            // A click while it waits copies nothing.
            // Forced: Playwright waits for an aria-disabled button to wake before clicking it.
            await page.click("#th", { force: true });
            if (await page.waitForFunction(() => document.body.innerText.includes("Copied"), null, { timeout: 1000 }).then(() => true, () => false)) missing.push("no copy while it waits");
          }
        }
        const shown = (f.absent ?? []).filter((l) => text.includes(l));
        backgrounds.push(await page.evaluate(() => getComputedStyle(document.body).backgroundColor));
        const name = `${f.face.replace(/[^a-z0-9-]+/g, "-")}-${scheme}.png`;
        await page.screenshot({ path: path.join(out, name), fullPage: !!f.full });
        entry.shots.push(name);
        entry[scheme] = { missing, shown, errors, owner: await page.evaluate(() => !!document.getElementById("t")) };
        await context.close();
      }
      if (f.blocked) await env.BUCKET.delete(HEALTH);
      if (f.listens) delete env.AI;
      if (f.chip) {
        // The stored still's corner: the length's light text over the dark pill, or not there.
        const key = [...env.BUCKET.store.keys()].find((k) => k.startsWith(`rec/${f.id}/thumb-`) && k.endsWith(".jpg"));
        const chip = key ? await chipIn(browser, env.BUCKET.store.get(key)) : null;
        entry.chip = chip;
        if (!chip || chip.maxLuma < 180) entry.light.missing.push(`the length on the picture (max light in its corner ${chip?.maxLuma})`);
      }
      if (f.click || f.golive) {
        // The pictures the page stored for the title, kept beside the shots: what a paste shows.
        for await (const key of [...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${f.id}/thumb-`))) {
          const ext = key.split(".").pop();
          fs.writeFileSync(path.join(out, `${f.face}-picture.${ext}`), env.BUCKET.store.get(key));
          entry.pictures = [...(entry.pictures ?? []), `${f.face}-picture.${ext}`];
        }
        for (const ext of f.pictures) if (!(entry.pictures ?? []).includes(`${f.face}-picture.${ext}`)) entry.light.missing.push(`a stored .${ext} picture`);
      }
      // The page follows the viewer's scheme (the spec sets none for it; the embed is the
      // player alone, dark under both): both are shot, and the difference is reported, not required.
      entry.schemesDiffer = backgrounds[0] !== backgrounds[1];
      entry.ok = ["light", "dark"].every((s) => entry[s].missing.length === 0 && entry[s].shown.length === 0 && entry[s].errors.length === 0 && entry[s].owner === !!f.owner);
    }
  } finally {
    await browser.close();
    await chrome?.close();
    server.close();
  }
  summary.out = out;
  summary.ok = Object.values(summary.faces).every((f) => f.ok);
}

main().catch((e) => { summary.error = String(e?.stack ?? e).slice(0, 600); }).finally(() => {
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : 1);
});
