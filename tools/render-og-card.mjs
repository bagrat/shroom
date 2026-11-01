// Renders shroom.am's link preview card — the canvas board "shroom.am — the link preview
// card": one 1200 × 630 picture every page names as its og:image. The words are the
// landing's heading (B140), held to site/index.html by site/test/og.test.mjs; the sub-line
// left the card with the landing's (Bagrat, 2026-10-25).
//
//   node tools/render-og-card.mjs   → site/og.png
//
// Run where the Inter font is installed (a cloud session has it): it stands in for the Mac's
// system font, so the picture comes out the same wherever it is rendered.

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

export const HEADING = "Record. Share. Pay-as-you-go.";
export const FOOT = "Your videos, in your own Cloudflare account";

const html = `<!doctype html><html><head><meta charset="utf-8"><style>
  body { margin: 0; }
  .card { width: 1200px; height: 630px; box-sizing: border-box; padding: 72px 80px; background: #2b2f36; color: #eceff4;
    font-family: Inter, -apple-system, system-ui, sans-serif; display: flex; flex-direction: column; justify-content: space-between; }
  .brand { display: flex; align-items: center; gap: 18px; font-size: 44px; font-weight: 600; }
  .brand svg { width: 64px; height: 69px; display: block; }
  h1 { margin: 0; white-space: nowrap; font-size: 70px; font-weight: 700; line-height: 1.05; letter-spacing: -0.015em; }
  .foot { display: flex; justify-content: space-between; align-items: center; font-size: 28px; color: #c3cad6; }
  .foot b { font: 400 28px ui-monospace, "DejaVu Sans Mono", monospace; color: #eceff4; }
</style></head><body><div class="card">
  <div class="brand"><svg viewBox="18 20 84 90" aria-hidden="true">
    <path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"/>
    <path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"/>
    <circle cx="50" cy="44" r="6" fill="#ECEFF4"/><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"/><circle cx="33" cy="58" r="3.4" fill="#ECEFF4"/>
    <circle cx="64" cy="58" r="2.6" fill="#ECEFF4"/><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"/>
  </svg>shroom</div>
  <div><h1>${HEADING}</h1></div>
  <div class="foot"><span>${FOOT}</span><b>shroom.am</b></div>
</div></body></html>`;

if (process.argv[1] && path.resolve(process.argv[1]) === new URL(import.meta.url).pathname) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
  await page.setContent(html);
  await page.evaluate(() => document.fonts.ready);
  fs.writeFileSync(path.resolve("site/og.png"), await page.screenshot({ clip: { x: 0, y: 0, width: 1200, height: 630 } }));
  await browser.close();
  console.log("site/og.png");
}
