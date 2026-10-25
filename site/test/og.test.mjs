// Link previews: every page shroom.am serves as HTML names the same card (site/og.png, drawn
// by tools/render-og-card.mjs from the canvas board "shroom.am — the link preview card") and
// a description. Run: node --test site/test/og.test.mjs
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { build } from "../build.mjs";

const ROOT = new URL("../..", import.meta.url).pathname;
const out = build(join(mkdtempSync(join(tmpdir(), "shroom-og-")), "public"));
const pages = {
  "/": readFileSync(join(out, "index.html"), "utf8"),
  "/agent": readFileSync(join(out, "_/agent.html"), "utf8"),
  "/whats-new": readFileSync(join(out, "_/whats-new/latest.html"), "utf8"),
  "/faq": readFileSync(join(out, "_/faq.html"), "utf8"),
  "/privacy": readFileSync(join(out, "_/privacy.html"), "utf8"),
  "/terms": readFileSync(join(out, "_/terms.html"), "utf8"),
};
const meta = (html, attr, key) => html.match(new RegExp(`<meta ${attr}="${key}" content="([^"]*)">`))?.[1];

describe("link previews", () => {
  for (const [route, html] of Object.entries(pages)) {
    it(`${route} names the card and a description`, () => {
      assert.equal(meta(html, "property", "og:image"), "https://shroom.am/og.png", "an absolute address: previews don't resolve relative ones");
      assert.equal(meta(html, "property", "og:image:width"), "1200");
      assert.equal(meta(html, "property", "og:image:height"), "630");
      assert.ok(meta(html, "property", "og:image:alt"), "the card says what it shows");
      assert.equal(meta(html, "name", "twitter:card"), "summary_large_image");
      const description = meta(html, "property", "og:description");
      assert.ok(description, "og:description");
      assert.equal(description, meta(html, "name", "description"), "the preview says what a search result says");
    });
  }

  it("ships the card, 1200 × 630", () => {
    const png = readFileSync(join(out, "og.png"));
    assert.equal(png.subarray(1, 4).toString(), "PNG");
    assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [1200, 630]);
  });

  it("draws the landing's own heading on the card, and the sub-line the landing used to carry", () => {
    const tool = readFileSync(join(ROOT, "tools/render-og-card.mjs"), "utf8");
    const landing = pages["/"];
    const h1 = landing.match(/<h1>([^<]*)<\/h1>/)?.[1];
    assert.equal(tool.match(/export const HEADING = "([^"]*)"/)?.[1], h1);
    // The landing dropped its line under the headline (Bagrat, 2026-10-25); the card, its own
    // approved board, keeps it (B140's words) until he says otherwise.
    assert.equal(tool.match(/export const SUB = "([^"]*)"/)?.[1], "No monthly seat, sometimes completely free");
  });
});
