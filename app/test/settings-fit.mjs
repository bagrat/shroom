// Settings fits its window (N78): every scene of every tab the core's gallery
// prints, rendered by the built Settings page in WebKit, takes no more height than the
// window's page — its WindowSpec less macOS's title bar (gallery-kit's windowPage), 688
// under a 720 frame. Members too, with the gallery's people (a longer list may scroll, as
// the Library's does). The cheap half of windows-gallery.mjs's own check, run in CI so a pane
// can't outgrow the window again unnoticed: each scene once, light, at the window's width
// and a short height, so the pane takes its own height rather than the window's.
//
//   npm run build --workspace app      (first: this serves app/dist)
//   node app/test/settings-fit.mjs
//
// The fonts are the machine's, not the Mac's: on a Mac the panes measured within a few
// points of Linux's WebKit (2026-10-25), and the headroom each pane has is printed.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { webkit } from "playwright";
import { dist, gallery, serve, stub, windowPage } from "./gallery-kit.mjs";

if (!existsSync(join(dist, "settings.html"))) {
  console.error("app/dist/settings.html is missing — run `npm run build --workspace app` first.");
  process.exit(2);
}

const PAGE = windowPage("SETTINGS", { width: 600, height: 720 });
const scenes = [...gallery("settings"), ...gallery("members").filter((e) => e.window === "settings")];
const server = await serve();
const browser = await webkit.launch();
const rows = [];
let bad = 0;
for (const entry of scenes) {
  const kind = entry.scene.body.pane.kind;
  const ctx = await browser.newContext({ viewport: { width: PAGE.width, height: 100 } });
  const page = await ctx.newPage();
  await page.addInitScript(stub, entry.scene);
  await page.goto(`${server.origin}/settings.html`);
  await page.waitForSelector(".set > .behind > .pane");
  await page.waitForTimeout(60);
  // Measured from the page's top: a lit row scrolls itself into view.
  const height = await page.evaluate(() => Math.ceil(document.querySelector(".set > .behind > .pane").getBoundingClientRect().bottom + window.scrollY));
  await ctx.close();
  const fits = height <= PAGE.height;
  if (!fits) bad++;
  rows.push({ variant: entry.variant, kind, height, fits });
}
await browser.close();
server.close();

rows.sort((a, b) => b.height - a.height);
for (const r of rows) console.log(`${r.fits ? "ok  " : "FAIL"} ${r.kind.padEnd(7)} ${String(r.height).padStart(4)} of ${PAGE.height}  ${r.variant}`);
const tallest = rows[0];
console.log(`settings ${PAGE.width}×${PAGE.height} (the page under the title bar) · ${rows.length} panes · tallest ${tallest.height} (${tallest.variant}), ${PAGE.height - tallest.height} to spare`);
if (bad) {
  console.error(`${bad} pane(s) taller than the window's page: on a Mac they scroll. Give the rows a home that fits (DESIGN.md, design first).`);
  process.exit(1);
}
