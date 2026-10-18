// The landing's video hides its controls while it plays and the pointer is idle or gone
// (Bagrat, 2026-10-18; DECISIONS B194). The page leaves this to the browser's own controls,
// which do it in Chromium and WebKit and keep keyboard and VoiceOver access; this holds it, in
// both, on the built site. The video plays at 1/16 speed so its own picture barely moves: what
// changes along the bottom of the stage is the control bar.
// Needs Playwright's browsers (a cloud session has them). Run: node --test site/test/controls.browser.mjs
import { createServer } from "node:http";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, describe, it } from "node:test";
import assert from "node:assert/strict";
import { chromium, webkit } from "playwright";
import { build } from "../build.mjs";
import worker from "../worker.js";

const out = build(join(mkdtempSync(join(tmpdir(), "shroom-site-")), "public"));
const ASSETS = {
  /** @param {Request} r */
  async fetch(r) {
    try {
      return new Response(readFileSync(join(out, decodeURIComponent(new URL(r.url).pathname))));
    } catch {
      return new Response("", { status: 404 });
    }
  },
};
// The landing's files as shroom.am serves them: /how-to.mp4 through the Worker, the rest as they are.
const server = createServer(async (req, res) => {
  const path = (req.url ?? "/").split("?")[0];
  const answer = path === "/how-to.mp4" || path === "/"
    ? await worker.fetch(new Request(`https://shroom.am${path}`, { headers: { accept: "text/html", ...(req.headers.range ? { range: req.headers.range } : {}) } }), { ASSETS })
    : await ASSETS.fetch(new Request(`https://shroom.am${path}`));
  res.writeHead(answer.status, Object.fromEntries(answer.headers));
  res.end(Buffer.from(await answer.arrayBuffer()));
});
await new Promise((ok) => server.listen(0, "127.0.0.1", () => ok(undefined)));
const address = server.address();
const base = `http://127.0.0.1:${typeof address === "object" && address ? address.port : 0}`;
after(() => server.close());

/** How far apart two screenshots' bottom strips are: the mean difference per channel, 0–255.
 * @param {import("playwright").Page} page @param {Buffer} a @param {Buffer} b */
async function apart(page, a, b) {
  return page.evaluate(async ([a, b]) => {
    const pixels = async (/** @type {string} */ src) => {
      const img = new Image();
      img.src = `data:image/png;base64,${src}`;
      await img.decode();
      const c = document.createElement("canvas");
      c.width = img.width; c.height = img.height;
      const g = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));
      g.drawImage(img, 0, 0);
      const h = Math.round(img.height * 0.2); // the bottom fifth, where the control bar sits
      return g.getImageData(0, img.height - h, img.width, h).data;
    };
    const [p, q] = [await pixels(a), await pixels(b)];
    let sum = 0;
    for (let i = 0; i < p.length; i += 4) sum += Math.abs(p[i] - q[i]) + Math.abs(p[i + 1] - q[i + 1]) + Math.abs(p[i + 2] - q[i + 2]);
    return sum / (p.length / 4) / 3;
  }, [a.toString("base64"), b.toString("base64")]);
}

for (const [name, engine] of /** @type {const} */ ([["Chromium", chromium], ["WebKit", webkit]])) {
  describe(`the landing's video controls, in ${name}`, () => {
    it("show while the pointer moves, and hide once it's idle or gone", async () => {
      const browser = await engine.launch();
      try {
        const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
        await page.goto(`${base}/`);
        await page.click(".still");
        await page.waitForFunction(() => { const v = document.querySelector("video"); return !!v && !v.paused && v.currentTime > 0; });
        await page.evaluate(() => { /** @type {HTMLVideoElement} */ (document.querySelector("video")).playbackRate = 0.0625; });
        const stage = page.locator(".stage");
        const box = /** @type {{ x: number, y: number, width: number, height: number }} */ (await stage.boundingBox());
        // The bar shows on the pointer's move (any first frame may still be fading in).
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.move(box.x + box.width / 2 + 12, box.y + box.height / 2 + 6);
        await page.waitForTimeout(600);
        const moving = await stage.screenshot();
        await page.waitForTimeout(5000);
        const idle = await stage.screenshot();
        await page.mouse.move(box.x + box.width / 2 + 24, box.y + box.height / 2 + 12);
        await page.waitForTimeout(600);
        const back = await stage.screenshot();
        await page.mouse.move(5, 5);
        await page.waitForTimeout(3000);
        const gone = await stage.screenshot();
        assert.equal(await page.evaluate(() => /** @type {HTMLVideoElement} */ (document.querySelector("video")).paused), false, "still playing");
        const hid = await apart(page, moving, idle), returned = await apart(page, moving, back), left = await apart(page, idle, gone);
        assert.ok(hid > 4, `the bar hides when the pointer is idle (${hid.toFixed(1)})`);
        assert.ok(returned < hid / 2, `and comes back when it moves (${returned.toFixed(1)} vs ${hid.toFixed(1)})`);
        assert.ok(left < hid / 2, `and stays hidden once the pointer leaves (${left.toFixed(1)} vs ${hid.toFixed(1)})`);
      } finally {
        await browser.close();
      }
    });
  });
}
