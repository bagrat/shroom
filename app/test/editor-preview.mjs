// The editor (N63) seen without the core: the built window (app/dist/editor.html) in WebKit
// against the stubbed Tauri bridge, handed the hand-written faces of editor-fixture.mjs, its
// media from a loopback stand-in for the shell's `shroom-media:` protocol — a 75 s test
// video with sound made here by the system's ffmpeg (a test pattern and a speech-like tone
// with pauses), and its peaks (50 a second, u8) read from ffmpeg's PCM. Every face is
// screenshotted light and dark into app/test/gallery-out/ (gitignored), and the window is
// driven: a drag across the sound shows the Cut chip and the chip sends the cut, a cut
// clicked offers Put back, a yellow end dragged sends the trim, ⌘Z / ⇧⌘Z / Delete / Esc /
// Return, the buttons send the Scene's actions, Space plays and the playhead moves, and
// while it plays the video never lands inside a cut or past the trim.
//
//   npm run build --workspace app       (run it first: this serves app/dist)
//   node app/test/editor-preview.mjs
//
// Exits non-zero on a failed check. windows-gallery.mjs (npm run test:gallery) runs the
// pieces exported here — editorLooks over the core's own faces (`-- editor`), editorDrives
// over these — and holds the fixture to the core's shape and words; this file alone is the
// quicker look.

import { execFileSync } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { webkit } from "playwright";
import { dist, norm, out, serve, stub, windowPage } from "./gallery-kit.mjs";
import { editorScenes, REC } from "./editor-fixture.mjs";

/** The editor's page: its WindowSpec in runtime/presenter.rs, less the title bar. */
export const EDITOR_SIZE = windowPage("EDITOR", { width: 1100, height: 720 });
/** The window's minimum (the presenter's min size, less the title bar). */
export const EDITOR_MIN = { width: 900, height: 640 - 32 };
const PER_SECOND = 50;

/** A test video with sound and its peaks, in `dir`: { video, peaks, duration }. */
export function makeMedia(dir, duration = 75) {
  const video = join(dir, "video.mp4");
  const pcm = join(dir, "sound.f32");
  const peaks = join(dir, "peaks.u8");
  // Speech-like: a voiced tone whose pitch drifts, syllables (~4.6 a second) louder and
  // softer by turns, phrases with pauses between them, and six seconds of silence at 0:20.
  const sound =
    "aevalsrc='0.8*sin(2*PI*(170+50*sin(2*PI*0.7*t))*t)*abs(sin(2*PI*2.3*t))*(0.2+0.8*abs(sin(2*PI*0.47*t+2*sin(2*PI*0.11*t))))" +
    `*gt(sin(2*PI*0.21*t+1)\\,-0.4)*not(between(t\\,20\\,26))':s=48000:d=${duration}`;
  execFileSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", `testsrc2=size=640x360:rate=25:duration=${duration}`, "-f", "lavfi", "-i", sound,
    "-c:v", "libx264", "-preset", "veryfast", "-crf", "32", "-pix_fmt", "yuv420p", "-g", "25", "-c:a", "aac", "-b:a", "64k", "-movflags", "+faststart", "-shortest", video]);
  const channels = Number(execFileSync("ffprobe", ["-v", "error", "-select_streams", "a:0", "-show_entries", "stream=channels", "-of", "csv=p=0", video], { encoding: "utf8" }).trim()) || 1;
  execFileSync("ffmpeg", ["-v", "error", "-y", "-i", video, "-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ar", "48000", pcm]);
  // As shroom_encode::peaks reads them: per 20 ms, the peak |sample| over every channel.
  const raw = readFileSync(pcm);
  const samples = new Float32Array(raw.buffer, raw.byteOffset, Math.floor(raw.length / 4));
  const per = (48000 / PER_SECOND) * channels;
  const values = new Uint8Array(Math.ceil(samples.length / per));
  for (let i = 0; i < values.length; i++) {
    let p = 0;
    for (let j = i * per; j < Math.min(samples.length, (i + 1) * per); j++) p = Math.max(p, Math.abs(samples[j]));
    values[i] = Math.round(255 * Math.min(1, p));
  }
  writeFileSync(peaks, values);
  rmSync(pcm);
  return { video, peaks, duration };
}

/** The shell's `shroom-media://localhost/<rec>/{video,peaks}`, on loopback: the video with
 * byte ranges (as WebKit asks for them), the peaks whole; anything else 404. */
export async function serveMedia({ video, peaks }, rec = REC) {
  const v = readFileSync(video);
  const p = readFileSync(peaks);
  const server = createServer((req, res) => {
    const path = new URL(req.url, "http://x").pathname;
    // The page's origin isn't the media's (in the app too: tauri: vs shroom-media:).
    const cors = { "access-control-allow-origin": "*" };
    if (path === `/${rec}/peaks`) {
      res.writeHead(200, { ...cors, "content-type": "application/octet-stream", "content-length": p.length }).end(p);
      return;
    }
    if (path !== `/${rec}/video`) {
      res.writeHead(404, cors).end();
      return;
    }
    const m = /^bytes=(\d*)-(\d*)$/.exec(req.headers.range ?? "");
    if (!m) {
      res.writeHead(200, { ...cors, "content-type": "video/mp4", "accept-ranges": "bytes", "content-length": v.length }).end(v);
      return;
    }
    const start = m[1] ? Number(m[1]) : Math.max(0, v.length - Number(m[2]));
    const end = m[1] && m[2] ? Math.min(Number(m[2]), v.length - 1) : v.length - 1;
    res
      .writeHead(206, { ...cors, "content-type": "video/mp4", "accept-ranges": "bytes", "content-range": `bytes ${start}-${end}/${v.length}`, "content-length": end - start + 1 })
      .end(v.subarray(start, end + 1));
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  return { origin: `http://127.0.0.1:${server.address().port}`, close: () => server.close() };
}

/** The editor window on `scene`, its media from `mediaOrigin`. */
export async function openEditor(browser, origin, mediaOrigin, scene, { colorScheme = "light", size = EDITOR_SIZE } = {}) {
  const ctx = await browser.newContext({ colorScheme, viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript(stub, scene);
  await page.addInitScript((m) => (window.__shroomMediaBase = m), mediaOrigin);
  await page.goto(`${origin}/editor.html`);
  await page.waitForSelector("main.ed");
  if (scene.body.media.state === "ready") {
    await page.waitForFunction(() => (document.querySelector("video")?.readyState ?? 0) >= 2);
  }
  // The track, with its waveform, unless the final video's bar stands in its place.
  if (scene.body.media.state === "ready" && scene.body.after?.state !== "making") {
    // The peaks are drawn: some pixel of the waveform is painted.
    await page.waitForFunction(() => {
      const c = document.querySelector("canvas.wave");
      if (!c?.width) return false;
      const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
      for (let i = 3; i < d.length; i += 4) if (d[i]) return true;
      return false;
    });
  }
  await page.waitForTimeout(60);
  return { ctx, page, errors };
}

const calls = (page) => page.evaluate(() => window.__calls);
const clear = (page) => page.evaluate(() => (window.__calls = []));
const steps = (c) => c.filter((x) => x.cmd === "edit").map((x) => x.args.step);
const canon = (a) => JSON.stringify(a, Object.keys(a ?? {}).sort());
const acted = (c, action) => c.some((x) => x.cmd === "act" && x.args.surface === "editor" && canon(x.args.action) === canon(action));
const keyed = (c, k) => c.some((x) => x.cmd === "key" && x.args.surface === "editor" && x.args.key === k);
const near = (a, b, tol) => Math.abs(a - b) <= tol;

/** A point on the track at source time `t` (of `duration`). */
async function at(page, t, duration) {
  const r = await page.locator(".track").boundingBox();
  return { x: r.x + (t / duration) * r.width, y: r.y + r.height / 2 };
}
async function drag(page, from, to, duration) {
  const a = await at(page, from, duration);
  const b = await at(page, to, duration);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move((a.x + b.x) / 2, a.y, { steps: 4 });
  await page.mouse.move(b.x, b.y, { steps: 4 });
  await page.mouse.up();
}
async function click(page, t, duration) {
  const a = await at(page, t, duration);
  await page.mouse.click(a.x, a.y);
}
const timeText = (page) => page.locator(".time").innerText();
const current = (page) => page.evaluate(() => document.querySelector("video").currentTime);

/** Each face, light and dark: its lines, its buttons as the Scene says, nothing sideways,
 * the waveform crisp, no errors, nothing sent; a screenshot. On the edited face also the
 * two chips, as the board draws them. */
export async function editorLooks({ browser, origin, mediaOrigin, entries, check, shots = out }) {
  for (const entry of entries) {
    const w = entry.scene;
    const b = w.body;
    for (const scheme of ["light", "dark"]) {
      const tag = `editor/${entry.variant} [${scheme}]`;
      const { ctx, page, errors } = await openEditor(browser, origin, mediaOrigin, w, { colorScheme: scheme });
      check(`${tag}: the face is the Scene's`, (await page.locator("main.ed").getAttribute("data-state")) === w.id);
      check(`${tag}: titled as the Scene says`, (await page.title()) === b.title);
      const body = norm(await page.locator("body").innerText());
      // After a save (B197): the final video being made where the track was, its failure in
      // the footer, then the question in the footer's place; Revert is gone.
      const after = b.after ?? null;
      const asking = after?.state === "asking";
      const lines = asking
        ? [after.title, after.line]
        : [b.footer, ...(["ready", "opening"].includes(b.media.state) && !after ? [b.hint] : []), ...(b.media.line ? [b.media.line] : []), ...(after?.state === "making" ? [after.line] : []), ...(b.original_kept && !after ? [b.original_kept] : [])];
      const missing = lines.filter((l) => !body.includes(norm(l)));
      check(`${tag}: every line is visible`, missing.length === 0, missing.join(" | "));
      const want = asking ? [after.keep, after.delete] : [...(b.media.state === "failed" ? [b.media.retry] : []), ...(b.media.state === "failed" || after || !b.revert ? [] : [b.revert]), b.done];
      const shown = (await page.locator("button:visible").evaluateAll((els) => els.map((e) => e.innerText.trim()).filter(Boolean))).map(norm);
      check(`${tag}: the Scene's buttons, no others with words`, shown.sort().join("|") === want.map((x) => x.label).sort().join("|"), shown.join("|"));
      for (const x of want) {
        const loc = page.getByRole("button", { name: x.label, exact: true });
        const ok = (await loc.count()) === 1 && (await loc.isEnabled()) === x.enabled && (await loc.getAttribute("class")).split(/\s+/).includes(x.style);
        check(`${tag}: “${x.label}” ${x.enabled ? "enabled" : "greyed"}, ${x.style}`, ok);
      }
      if (asking) {
        check(`${tag}: the question takes the footer's place`, (await page.locator(".foot").count()) === 0 && (await page.locator(".ask").count()) === 1);
        const order = await page.locator(".ask button").allInnerTexts();
        check(`${tag}: Keep for 2 days, then Yes, delete — filled, leading`, order.map(norm).join("|") === [after.keep.label, after.delete.label].join("|") && after.delete.style === "filled", order.join("|"));
        check(`${tag}: Return isn't Yes, delete (law 9)`, w.ret === null);
      } else {
        const amber = await page.locator(".foot .line").evaluate((e) => getComputedStyle(e).color === getComputedStyle(document.documentElement).getPropertyValue("--amber").trim() || e.classList.contains("amber"));
        check(`${tag}: the footer line's tone`, amber === (b.footer_tone === "amber"));
      }
      if (after?.state === "making") {
        const pct = await page.locator(".note.making .fill span").evaluate((e) => e.style.width);
        check(`${tag}: the final video's bar fills as the Scene says, where the track was`, pct === `${after.pct}%` && (await page.locator(".track").count()) === 0, pct);
      }
      if (b.original_kept && !after && b.revert) {
        const kept = await page.locator(".foot .kept").evaluate((e) => ({ text: e.innerText, size: getComputedStyle(e).fontSize }));
        const next = await page.locator(".foot .kept + button").innerText();
        check(`${tag}: the time the original has left, small, by Revert`, kept.text === b.original_kept && kept.size === "12px" && norm(next) === b.revert.label, JSON.stringify(kept));
      }
      const wide = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1 || document.documentElement.scrollHeight > window.innerHeight + 1);
      check(`${tag}: nothing scrolls`, !wide);
      // The system font, as every window has it (theme.css's --font): a browser's default
      // here would be a serif in a Mac's webview.
      const fonts = await page.evaluate(() => [".ed", ".time", ".hint", ".foot .line", ".foot button", ".ask h2", ".ask button"].map((q) => document.querySelector(q)).filter(Boolean).map((e) => getComputedStyle(e).fontFamily));
      check(`${tag}: its words in the system font`, fonts.length >= 3 && fonts.every((f) => f.includes("-apple-system")), fonts.join(" | "));
      check(`${tag}: no page errors`, errors.length === 0, errors.join(" | "));
      check(`${tag}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === "editor");
      const c = await calls(page);
      check(`${tag}: rendering calls nothing`, c.length === 0, JSON.stringify(c));
      if (b.media.state === "ready" && after?.state !== "making") {
        const cv = await page.locator("canvas.wave").evaluate((e) => ({ w: e.width, h: e.height, cw: e.clientWidth, ch: e.clientHeight }));
        check(`${tag}: the waveform is drawn at the display's density`, cv.w === Math.round(cv.cw * 2) && cv.h === Math.round(cv.ch * 2), JSON.stringify(cv));
        const len = b.edited_duration;
        const t = await timeText(page);
        check(`${tag}: the time reads edited position / edited length`, t === `0:00 / ${clockOf(len, true)}`, t);
        const was = await page.locator(".was").count();
        check(`${tag}: “was” only when the edit changes the length`, (was === 1) === (clockOf(len, true) !== clockOf(b.source_duration, true)));
        check(`${tag}: a flag for every marker`, (await page.locator(".marker").count()) === b.markers.length);
        check(`${tag}: a hatched cut for every applied cut`, (await page.locator(".cut").count()) === b.list.cuts.filter((x) => x.applied).length);
      }
      if (b.media.state === "getting") {
        const pct = await page.locator(".fill span").evaluate((e) => e.style.width);
        check(`${tag}: the bar fills as the Scene says`, pct === `${b.media.pct}%`, pct);
        check(`${tag}: no video until it's here`, (await page.locator("video").count()) === 0);
      }
      await page.screenshot({ path: join(shots, `${entry.variant}-${scheme}.png`), animations: "disabled" });
      // The chips, on the edited face (the fixture's, or the core's own).
      if (entry.variant === "editor-cut" || entry.variant === "editor-ready") {
        const d = b.source_duration;
        const cut = b.list.cuts[0];
        await click(page, (cut.start + cut.end) / 2, d);
        await page.waitForSelector(".chip");
        await page.screenshot({ path: join(shots, `editor-chip-put-back-${scheme}.png`), animations: "disabled" });
        await drag(page, 46, 58, d);
        await page.waitForSelector(".chip .cutb");
        await page.screenshot({ path: join(shots, `editor-chip-cut-${scheme}.png`), animations: "disabled" });
      }
      await ctx.close();
    }
  }
}

/** The page's clock (app/src/editor/clock.ts is held to it by its own test). */
function clockOf(seconds, length = false) {
  const n = Math.max(0, Number(seconds) || 0);
  const s = length ? Math.round(n) : Math.floor(n);
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60;
  const two = (v) => String(v).padStart(2, "0");
  return h ? `${h}:${two(m)}:${two(x)}` : `${m}:${two(x)}`;
}

/** The interactions, on the edited face and the others. */
export async function editorDrives({ browser, origin, mediaOrigin, entries, check }) {
  const face = (variant) => entries.find((e) => e.variant === variant).scene;
  const cutFace = face("editor-cut");
  const b = cutFace.body;
  const d = b.source_duration;
  const [cut] = b.list.cuts;
  const lastEnd = b.keep[b.keep.length - 1][1];
  const { ctx, page, errors } = await openEditor(browser, origin, mediaOrigin, cutFace);

  // A drag across the sound: the selection and its chip, and the chip sends the cut.
  await drag(page, 46, 56, d);
  const label = norm(await page.locator(".chip button").innerText());
  const sel = Number(/^Cut (\d+\.\d) s$/.exec(label)?.[1]);
  check("editor: a drag across the sound selects a part, with Cut <length> under it", (await page.locator(".sel").count()) === 1 && near(sel, 10, 0.3), label);
  await clear(page);
  await page.locator(".chip button").click();
  let s = steps(await calls(page));
  check("editor: the chip sends the cut", s.length === 1 && s[0].op === "cut" && near(s[0].start, 46, 0.3) && near(s[0].end, 56, 0.3), JSON.stringify(s));
  check("editor: … and the selection goes", (await page.locator(".sel, .chip").count()) === 0);

  // Esc drops a selection (and sends nothing); Delete cuts it.
  await drag(page, 10, 14, d);
  await clear(page);
  await page.keyboard.press("Escape");
  check("editor: Esc drops the selection, nothing sent", (await page.locator(".sel").count()) === 0 && (await calls(page)).length === 0);
  await drag(page, 10, 14, d);
  await clear(page);
  await page.keyboard.press("Delete");
  s = steps(await calls(page));
  check("editor: Delete cuts the selection", s.length === 1 && s[0].op === "cut" && near(s[0].start, 10, 0.3) && near(s[0].end, 14, 0.3), JSON.stringify(s));
  await clear(page);
  await page.keyboard.press("Backspace");
  check("editor: Delete with nothing selected sends nothing", (await calls(page)).length === 0);

  // A selection is held inside the trim.
  await drag(page, 1, 8, d);
  const held = await page.locator(".chip button").innerText();
  check("editor: a selection stops at the trim's in", norm(held) === `Cut ${(8 - b.list.in).toFixed(1)} s` || near(Number(/(\d+\.\d)/.exec(held)?.[1]), 8 - b.list.in, 0.3), held);
  await page.keyboard.press("Escape");

  // A click on a cut picks it: Put back <length>, which sends put-back.
  await click(page, (cut.start + cut.end) / 2, d);
  const back = norm(await page.locator(".chip button").innerText());
  check("editor: a cut clicked offers Put back <its length>", back === `Put back ${(cut.end - cut.start).toFixed(1)} s`, back);
  await clear(page);
  await page.locator(".chip button").click();
  s = steps(await calls(page));
  check("editor: Put back sends put-back for that cut", s.length === 1 && s[0].op === "put-back" && s[0].index === 0, JSON.stringify(s));
  await clear(page);
  await page.locator(".badge").first().dispatchEvent("pointerdown", { button: 0, pointerId: 1 });
  check("editor: the scissors pick their cut too", norm(await page.locator(".chip button").innerText()) === back);
  await page.keyboard.press("Escape");

  // A suggestion left unapplied: dashed, and clicked it offers Cut <its length>, applying it.
  const offer = b.list.cuts.findIndex((x) => !x.applied);
  if (offer >= 0) {
    const o = b.list.cuts[offer];
    check("editor: a suggestion shows dashed, not cut", (await page.locator(".offer").count()) === b.list.cuts.filter((x) => !x.applied).length);
    await click(page, (o.start + o.end) / 2, d);
    const apply = norm(await page.locator(".chip button").innerText());
    await clear(page);
    await page.locator(".chip button").click();
    s = steps(await calls(page));
    check("editor: a suggestion clicked offers Cut it too, which applies it", apply === (b.labels.cut_it_too ?? `Cut ${(o.end - o.start).toFixed(1)} s`) && s.length === 1 && s[0].op === "apply" && s[0].index === offer, `${apply} ${JSON.stringify(s)}`);
  }

  // A cut's edge dragged (B195 (g)): ↔ over it, the chip says the cut's length as it goes,
  // nothing is sent until it's let go, then one resize for that cut.
  const edgeEl = page.locator(`[data-edge="0-end"]`);
  const eb = await edgeEl.boundingBox();
  const pointer = await edgeEl.evaluate((el) => getComputedStyle(el).cursor);
  check("editor: a cut's edge shows the ↔ pointer", pointer === "ew-resize", pointer);
  const edgeTo = await at(page, cut.end + 4, d);
  await clear(page);
  await page.mouse.move(eb.x + eb.width / 2, eb.y + eb.height / 2);
  await page.mouse.down();
  await page.mouse.move(edgeTo.x, edgeTo.y, { steps: 6 });
  const live = norm(await page.locator(".chip button").innerText());
  const mid = steps(await calls(page));
  await page.mouse.up();
  s = steps(await calls(page));
  check("editor: a cut's edge dragged shows its length live and sends nothing yet", mid.length === 0 && near(Number(/(\d+\.\d)/.exec(live)?.[1]), cut.end + 4 - cut.start, 0.4), `${live} ${JSON.stringify(mid)}`);
  check("editor: … let go, it sends one resize for that cut", s.length === 1 && s[0].op === "resize" && s[0].index === 0 && s[0].start === cut.start && near(s[0].end, cut.end + 4, 0.4), JSON.stringify(s));
  await page.keyboard.press("Escape");
  await page.waitForTimeout(1100);

  // A yellow end dragged: the trim, sent once on release; the ends hold where they were
  // let go until the core answers.
  const inBox = await page.locator(".end.in").boundingBox();
  const to = await at(page, 8, d);
  await clear(page);
  await page.mouse.move(inBox.x + inBox.width / 2, inBox.y + inBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x - 6, to.y, { steps: 6 });
  const during = steps(await calls(page));
  const shown = await timeText(page);
  await page.mouse.up();
  s = steps(await calls(page));
  check("editor: dragging an end sends nothing until it's let go", during.length === 0, JSON.stringify(during));
  check("editor: the time reads the dragged trim's length while it's dragged", shown.endsWith(` / ${clockOf(b.edited_duration - (8 - b.list.in), true)}`) || shown.endsWith(` / ${clockOf(b.edited_duration - (8 - b.list.in) + 1, true)}`), shown);
  check("editor: a dragged end sends the trim", s.length === 1 && s[0].op === "trim" && near(s[0].in, 8, 0.3) && s[0].out === b.list.out, JSON.stringify(s));
  const letGo = (await page.locator(".end.in").boundingBox()).x;
  check("editor: … and stays where it was let go while the core answers", near(letGo, (await at(page, 8, d)).x - 12, 2), `${letGo} vs ${(await at(page, 8, d)).x - 12}`);
  const out1 = await page.locator(".end.out").boundingBox();
  const outTo = await at(page, 60, d);
  await clear(page);
  await page.mouse.move(out1.x + out1.width / 2, out1.y + out1.height / 2);
  await page.mouse.down();
  await page.mouse.move(outTo.x + 6, outTo.y, { steps: 6 });
  await page.mouse.up();
  s = steps(await calls(page));
  check("editor: the out end sends the trim too", s.length === 1 && s[0].op === "trim" && near(s[0].out, 60, 0.3), JSON.stringify(s));
  // The core answers with the slice it had (it refused, say): the ends go back.
  await page.evaluate((w) => window.__emit(w), { ...cutFace, body: { ...b, can_redo: true } });
  await page.waitForTimeout(1200);
  const inNow = await page.locator(".end.in").boundingBox();
  check("editor: a trim the core didn't take goes back to the slice's ends", near(inNow.x, inBox.x, 1.5), `${inNow.x} vs ${inBox.x}`);

  // Undo · Redo: the keys and the buttons, as the slice offers them.
  await clear(page);
  await page.keyboard.press("Meta+z");
  await page.keyboard.press("Meta+Shift+z");
  s = steps(await calls(page));
  check("editor: ⌘Z is undo, ⇧⌘Z redo (both offered)", s.length === 2 && s[0].op === "undo" && s[1].op === "redo", JSON.stringify(s));
  await page.evaluate((w) => window.__emit(w), cutFace);
  await page.waitForTimeout(30);
  await clear(page);
  await page.keyboard.press("Meta+Shift+z");
  check("editor: ⇧⌘Z with Redo greyed sends nothing", (await calls(page)).length === 0);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  check("editor: the Undo button sends undo", steps(await calls(page)).map((x) => x.op).join() === "undo");
  check("editor: Redo is greyed when the slice says so", await page.getByRole("button", { name: "Redo", exact: true }).isDisabled());

  // The Scene's buttons and Return.
  await clear(page);
  await page.getByRole("button", { name: b.revert.label, exact: true }).click();
  await page.getByRole("button", { name: b.done.label, exact: true }).click();
  let c = await calls(page);
  check("editor: Revert to original and Done send the Scene's actions", acted(c, b.revert.action) && acted(c, b.done.action), JSON.stringify(c));
  await clear(page);
  await page.keyboard.press("Enter");
  check("editor: Return is the Scene's ret", keyed(await calls(page), "return"));

  // A click on the sound moves the playhead; in a cut it lands where the edit resumes.
  await click(page, 12, d);
  const clicked = { t: await current(page), text: await timeText(page) };
  check("editor: a click moves the playhead", near(clicked.t, 12, 0.3) && clicked.text.startsWith(`${clockOf(clicked.t - b.list.in)} / `), JSON.stringify(clicked));
  await click(page, 20, d);
  check("editor: … and stays off a cut's scissors", (await page.locator(".chip").count()) === 0);
  await page.locator(".track").click({ position: { x: 3, y: 33 } });
  check("editor: a click before the trim lands on its in", near(await current(page), b.list.in, 0.05), String(await current(page)));

  // Space plays: the playhead moves, the time with it; Space again pauses.
  const head0 = (await page.locator(".playhead").boundingBox()).x;
  await page.keyboard.press(" ");
  await page.waitForTimeout(1500);
  const head1 = (await page.locator(".playhead").boundingBox()).x;
  const playing = await page.evaluate(() => !document.querySelector("video").paused);
  check("editor: Space plays and the playhead moves", playing && head1 > head0 + 5, `${head0} → ${head1}`);
  check("editor: the play button reads Pause while playing", (await page.locator(".play").getAttribute("aria-label")) === b.labels.pause);
  check("editor: the time runs", (await timeText(page)) !== `0:00 / ${clockOf(b.edited_duration, true)}`, await timeText(page));
  await page.keyboard.press(" ");
  await page.waitForTimeout(100);
  check("editor: Space again pauses", await page.evaluate(() => document.querySelector("video").paused));

  // Playing across a cut: never a moment inside it.
  await click(page, cut.start - 2.5, d);
  await page.evaluate(() => {
    const v = document.querySelector("video");
    window.__seen = [];
    const look = () => {
      window.__seen.push(v.currentTime);
      if (!v.paused) requestAnimationFrame(look);
    };
    v.addEventListener("play", () => requestAnimationFrame(look), { once: true });
    v.addEventListener("timeupdate", () => window.__seen.push(v.currentTime));
  });
  await page.keyboard.press(" ");
  await page.waitForTimeout(4500);
  await page.keyboard.press(" ");
  const seen = await page.evaluate(() => window.__seen);
  const inside = seen.filter((t) => t > cut.start + 0.001 && t < cut.end - 0.001);
  check("editor: playing, the video never lands inside a cut", seen.length > 20 && inside.length === 0, `${inside.length} of ${seen.length} inside: ${inside.slice(0, 5).join(", ")}`);
  check("editor: … and plays on past it", Math.max(...seen) > cut.end + 1, String(Math.max(...seen)));
  const tt = await timeText(page);
  check("editor: … the time reads the edited position", tt.startsWith(clockOf(cut.start - b.list.in + (Math.max(...seen) - cut.end) - 0.5)) || tt.startsWith(clockOf(cut.start - b.list.in + (Math.max(...seen) - cut.end))), tt);

  // Playing to the trim's out: it pauses there, reading the whole length.
  await click(page, lastEnd - 2, d);
  await page.evaluate(() => (window.__seen = []));
  await page.keyboard.press(" ");
  await page.waitForTimeout(3200);
  const end = await page.evaluate(() => ({ paused: document.querySelector("video").paused, t: document.querySelector("video").currentTime, seen: Math.max(...window.__seen) }));
  check("editor: playing stops at the trim's out", end.paused && end.t <= lastEnd && end.seen <= lastEnd + 0.001, JSON.stringify(end));
  const endText = await timeText(page);
  check("editor: … and reads the whole length", endText === `${clockOf(b.edited_duration, true)} / ${clockOf(b.edited_duration, true)}`, endText);
  await page.keyboard.press(" ");
  await page.waitForTimeout(400);
  const again = await current(page);
  check("editor: Play at the end starts the edit over", again >= b.list.in && again < b.list.in + 1.5, String(again));
  await page.keyboard.press(" ");

  // Done stops the preview: playing when the window moves on to saving (the same video, no
  // longer editable), it pauses. And a window put away lets go of its video: it is only
  // hidden, so nothing may play on behind it.
  const isPlaying = () => page.evaluate(() => !!document.querySelector("video") && !document.querySelector("video").paused);
  await page.keyboard.press(" ");
  await page.waitForTimeout(400);
  const playingAtDone = await isPlaying();
  await page.evaluate((w) => window.__emit(w), { ...cutFace, id: "editor-saving", body: { ...cutFace.body, editable: false, can_undo: false, can_redo: false } });
  await page.waitForTimeout(200);
  check("editor: Done stops the preview (nothing plays on while it saves)", playingAtDone && !(await isPlaying()), String(playingAtDone));
  await page.evaluate((w) => window.__emit(w), cutFace);
  await page.keyboard.press(" ");
  await page.waitForTimeout(400);
  const playingAtClose = await isPlaying();
  await page.evaluate(() => window.__emit(null));
  await page.waitForTimeout(200);
  const left = await page.evaluate(() => [...document.querySelectorAll("video")].map((v) => ({ paused: v.paused, src: v.currentSrc })));
  check("editor: put away, the window lets go of its video, nothing playing behind it", playingAtClose && left.length === 0, `${playingAtClose} ${JSON.stringify(left)}`);
  // Opened again after that: it starts paused, the button reads Play, and Play plays
  // (early.1 opened every editor stuck on Pause, the button and Space doing nothing).
  const fresh = async () => {
    await page.waitForFunction(() => (document.querySelector("video")?.readyState ?? 0) >= 1, null, { timeout: 5000 }).catch(() => {});
    return page.evaluate(() => ({ paused: document.querySelector("video")?.paused ?? null, label: document.querySelector(".play")?.getAttribute("aria-label") }));
  };
  const playsAgain = async () => {
    await page.keyboard.press(" ");
    await page.waitForTimeout(400);
    const ok = (await isPlaying()) && (await page.locator(".play").getAttribute("aria-label")) === b.labels.pause;
    await page.keyboard.press(" ");
    await page.waitForTimeout(150);
    return ok;
  };
  await page.evaluate((w) => window.__emit(w), cutFace);
  const reopened = await fresh();
  check("editor: opened again after it was put away playing, it starts paused, reading Play", reopened.paused === true && reopened.label === b.labels.play, JSON.stringify(reopened));
  check("editor: … and Play plays, after it was put away", await playsAgain());
  // A new file in its place while playing (opened afresh, or its edit baked under the same
  // name): the same.
  await page.keyboard.press(" ");
  await page.waitForTimeout(400);
  const playingAtSwap = await isPlaying();
  await page.evaluate((w) => window.__emit(w), { ...cutFace, body: { ...cutFace.body, media: { state: "opening" } } });
  await page.waitForTimeout(150);
  await page.evaluate((w) => window.__emit(w), cutFace);
  const swapped = await fresh();
  check("editor: a new file in its place while playing: it starts paused, reading Play", playingAtSwap && swapped.paused === true && swapped.label === b.labels.play, `${playingAtSwap} ${JSON.stringify(swapped)}`);
  check("editor: … and Play plays, with the new file", await playsAgain());
  await page.waitForTimeout(500);

  // A narrower window: the waveform redrawn to its new width, still crisp; nothing sideways.
  await page.setViewportSize(EDITOR_MIN);
  await page.waitForTimeout(150);
  const cv = await page.locator("canvas.wave").evaluate((e) => ({ w: e.width, cw: e.clientWidth }));
  check("editor: a resize redraws the waveform at its new width", cv.w === Math.round(cv.cw * 2) && cv.cw === EDITOR_MIN.width - 48, JSON.stringify(cv));
  const over = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth + 1 || document.documentElement.scrollHeight > window.innerHeight + 1);
  check("editor: at the minimum size nothing scrolls", !over);
  check("editor: no page errors while driven", errors.length === 0, errors.join(" | "));
  await ctx.close();

  // Saving: nothing edits, Done waits.
  {
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, face("editor-saving"));
    await drag(page, 46, 56, d);
    await clear(page);
    await page.keyboard.press("Meta+z");
    await page.keyboard.press("Delete");
    check("editor-saving: no selecting, no undo, no cut", (await page.locator(".sel, .chip").count()) === 0 && steps(await calls(page)).length === 0);
    check("editor-saving: the ends don't drag", (await page.locator(".tl.editable").count()) === 0);
    await page.keyboard.press("Enter");
    check("editor-saving: Return sends nothing (no ret)", !keyed(await calls(page), "return"));
    await ctx.close();
  }
  // Couldn't get the video: Try again, Close, Return and Esc are the Scene's.
  {
    const w = face("editor-cant-get");
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, w);
    await clear(page);
    await page.getByRole("button", { name: w.body.media.retry.label, exact: true }).click();
    await page.getByRole("button", { name: w.body.done.label, exact: true }).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
    c = await calls(page);
    check("editor-cant-get: Try again and Close send their actions; Return and Esc the Scene's", acted(c, w.body.media.retry.action) && acted(c, w.body.done.action) && keyed(c, "return") && keyed(c, "esc"), JSON.stringify(c));
    check("editor-cant-get: no Revert to original", (await page.getByRole("button", { name: w.body.revert.label }).count()) === 0);
    await ctx.close();
  }
  // A save the space turned down (B199): Close, Return and Esc are the Scene's; no Revert to
  // original, and nothing edits.
  {
    const w = face("editor-save-refused");
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, w);
    await clear(page);
    await drag(page, 46, 56, d);
    await page.keyboard.press("Meta+z");
    await page.keyboard.press("Delete");
    check("editor-save-refused: no selecting, no undo, no cut", (await page.locator(".sel, .chip").count()) === 0 && steps(await calls(page)).length === 0);
    await page.getByRole("button", { name: w.body.done.label, exact: true }).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
    c = await calls(page);
    check("editor-save-refused: Close, Return and Esc are the Scene's", acted(c, w.body.done.action) && keyed(c, "return") && keyed(c, "esc"), JSON.stringify(c));
    check("editor-save-refused: no Revert to original", (await page.getByRole("button", { name: "Revert to original" }).count()) === 0);
    await ctx.close();
  }
  // The final video being made: Close (and Return, and Esc) are the Scene's; nothing edits.
  {
    const w = face("editor-making");
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, w);
    await clear(page);
    await page.getByRole("button", { name: w.body.done.label, exact: true }).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
    c = await calls(page);
    check("editor-making: Close, Return and Esc are the Scene's", acted(c, w.body.done.action) && keyed(c, "return") && keyed(c, "esc"), JSON.stringify(c));
    check("editor-making: no Revert to original, no Undo, no hint", (await page.getByRole("button", { name: w.body.revert.label }).count()) === 0 && (await page.locator(".hint, .icon").count()) === 0);
    await ctx.close();
  }
  // “Delete the original?”: Yes, delete and Keep send theirs; Return sends nothing (law 9),
  // Esc is the Scene's (Keep for 2 days).
  {
    const w = face("editor-keep-original");
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, w);
    await clear(page);
    await page.keyboard.press("Enter");
    check("editor-keep-original: Return does nothing", (await calls(page)).length === 0, JSON.stringify(await calls(page)));
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: w.body.after.delete.label, exact: true }).click();
    await page.getByRole("button", { name: w.body.after.keep.label, exact: true }).click();
    c = await calls(page);
    check("editor-keep-original: Esc, Yes, delete and Keep for 2 days send the Scene's", keyed(c, "esc") && acted(c, w.body.after.delete.action) && acted(c, w.body.after.keep.action), JSON.stringify(c));
    await ctx.close();
  }
  // Getting: Space does nothing, the play button waits.
  {
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, face("editor-getting"));
    await page.keyboard.press(" ");
    check("editor-getting: the play button waits for the video", await page.locator(".play").isDisabled());
    check("editor-getting: the time reads 0:00 / the length", (await timeText(page)) === `0:00 / ${clockOf(face("editor-getting").body.source_duration, true)}`);
    await ctx.close();
  }
  // A new slice while open: a new edit is drawn without reopening.
  {
    const { ctx, page } = await openEditor(browser, origin, mediaOrigin, face("editor-untouched"));
    check("editor: untouched, no cut and no “was”", (await page.locator(".cut, .was").count()) === 0);
    await page.evaluate((w) => window.__emit(w), cutFace);
    await page.waitForTimeout(60);
    check("editor: a new slice draws its cut and its length", (await page.locator(".cut").count()) === 1 && (await page.locator(".was").count()) === 1);
    await ctx.close();
  }
}

async function main() {
  if (!existsSync(join(dist, "editor.html"))) {
    console.error("app/dist/editor.html is missing — run `npm run build --workspace app` first.");
    process.exit(2);
  }
  mkdirSync(out, { recursive: true });
  const dir = mkdtempSync(join(tmpdir(), "shroom-editor-"));
  const results = [];
  const check = (name, ok, detail = "") => results.push({ name, ok: !!ok, detail });
  try {
    const media = makeMedia(dir);
    const entries = editorScenes(media.duration);
    const server = await serve();
    const mediaServer = await serveMedia(media);
    const browser = await webkit.launch();
    const env = { browser, origin: server.origin, mediaOrigin: mediaServer.origin, entries, check };
    await editorLooks(env);
    await editorDrives(env);
    await browser.close();
    server.close();
    mediaServer.close();
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  const failed = results.filter((r) => !r.ok);
  for (const f of failed) console.log(`FAIL ${f.name}${f.detail ? ` — ${f.detail}` : ""}`);
  console.log(`\neditor: ${results.length - failed.length}/${results.length} checks passed · screenshots in ${out}`);
  process.exit(failed.length ? 1 : 0);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
