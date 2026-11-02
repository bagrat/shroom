// The player's controls take a phone's taps (Bagrat, 2026-11-02: on a phone, none of the
// player's buttons did anything). Headless WebKit as an iPhone and Chromium as an Android
// phone (touch, a coarse pointer), against the real worker.js over loopback: on a recipient's
// page, with chapters and captions and without, a tap on each control's centre and near
// each of its edges reaches that control and nothing else — never the seek bar drawn above
// them, which a coarse pointer makes taller (AI_STYLE). The media is never played, so no
// stream is needed. Seconds.
//
//   node worker/dev/phone-drive.mjs
import { createServer } from "node:http";
import { chromium, devices, webkit } from "playwright";
import { handle } from "../src/worker.js";
import { M1, metaKey, roster, space } from "../test/space-fixture.js";

const ids = { plain: "PlainPlainPlainP", named: "NamedNamedNamedN" };
const env = space({ roster: roster({}) });
const meta = (id, over = {}) => JSON.stringify({ schema: 1, id, state: "live", createdAt: new Date(Date.now() - 86_400_000).toISOString(), title: "A take", durationSec: 300,
  media: { hls: `rec/${id}/master.m3u8`, mp4: null, poster: null, width: 1280, height: 800 }, owner: M1, ...over });
await env.BUCKET.put(metaKey(ids.plain), meta(ids.plain));
await env.BUCKET.put(metaKey(ids.named), meta(ids.named, { named: { by: "ai", title: "A take", chapters: [[0, "The start"], [120, "The middle"]] }, captions: { language: "en" } }));

const server = createServer(async (req, res) => {
  const r = await handle(new Request(`http://127.0.0.1:${server.address().port}${req.url}`, { method: req.method, headers: req.headers }), env, Date.now());
  res.writeHead(r.status, Object.fromEntries(r.headers));
  if (r.body) for await (const c of r.body) res.write(c);
  res.end();
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const origin = `http://127.0.0.1:${server.address().port}`;

const CONTROLS = [".pp", ".cc", ".sp", ".mu", ".fs"];
const failures = [];
for (const [engine, device] of [[webkit, devices["iPhone 13"]], [chromium, devices["Pixel 7"]]]) {
  const browser = await engine.launch();
  try {
    const page = await (await browser.newContext({ ...device })).newPage();
    for (const [face, id] of Object.entries(ids)) {
      await page.goto(`${origin}/${id}`);
      // Where each click lands, stopped there: the controls' own handlers would play, mute or
      // go fullscreen, and only where a tap lands is asked.
      await page.evaluate(() => { window.__hit = []; window.addEventListener("click", (e) => { window.__hit.push(e.target.closest("button,input")?.className ?? e.target.tagName); e.stopPropagation(); e.preventDefault(); }, true); });
      const chapters = await page.evaluate(() => document.querySelector(".player").classList.contains("ch"));
      if (face === "named" && !chapters) failures.push(`${engine.name()} ${face}: the page drew no chapters`);
      for (const sel of CONTROLS) {
        const box = await page.locator(sel).boundingBox().catch(() => null);
        if (!box) { if (sel !== ".cc" || face === "named") failures.push(`${engine.name()} ${face}: no ${sel}`); continue; }
        const cls = sel.slice(1);
        for (const [name, x, y] of [["centre", 0.5, 0.5], ["top", 0.5, 0.1], ["bottom", 0.5, 0.9], ["left", 0.1, 0.5], ["right", 0.9, 0.5]]) {
          await page.evaluate(() => { window.__hit = []; });
          await page.touchscreen.tap(box.x + box.width * x, box.y + box.height * y);
          const hit = await page.evaluate(() => window.__hit);
          if (hit.length !== 1 || hit[0] !== cls) failures.push(`${engine.name()} ${face}: a tap at ${sel}'s ${name} reached ${JSON.stringify(hit)}`);
        }
      }
    }
  } finally {
    await browser.close();
  }
}
server.close();
console.log(failures.length ? `The player's controls on a phone — NO:\n${failures.map((f) => `- ${f}`).join("\n")}` : "The player's controls on a phone take every tap: OK");
process.exit(failures.length ? 1 : 0);
