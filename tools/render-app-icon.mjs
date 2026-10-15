// Renders shroom's app icon from SVG — the canvas's: the mushroom on a dark Nord rounded
// square (DEVIATIONS #45), on macOS's icon grid (an 824-unit square, radius 185, centered on
// a 1024 canvas, with its soft shadow inside the margin).
//
//   node tools/render-app-icon.mjs   → app/src-tauri/icons/{32x32,64x64,128x128,128x128@2x,icon}.png
//                                      and icon.icns (PNG entries, 16 to 1024)

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

const out = path.resolve("app/src-tauri/icons");

const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs><filter id="s" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="18" stdDeviation="20" flood-color="#000" flood-opacity="0.32"/></filter></defs>
  <rect x="100" y="100" width="824" height="824" rx="185" fill="#2e3440" filter="url(#s)"/>
  <g transform="translate(512 527) scale(6.1) translate(-60 -65)">
    <path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"/>
    <path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"/>
    <circle cx="50" cy="44" r="6" fill="#ECEFF4"/><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"/><circle cx="33" cy="58" r="3.4" fill="#ECEFF4"/>
    <circle cx="64" cy="58" r="2.6" fill="#ECEFF4"/><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"/>
  </g>
</svg>`;

const browser = await chromium.launch();
const page = await browser.newPage();
const png = async (px) => {
  await page.setViewportSize({ width: px, height: px });
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg.replace("<svg ", `<svg width="${px}" height="${px}" `)}</body></html>`);
  return page.screenshot({ omitBackground: true, clip: { x: 0, y: 0, width: px, height: px } });
};

const sizes = [16, 32, 64, 128, 256, 512, 1024];
const at = {};
for (const px of sizes) at[px] = await png(px);
await browser.close();

fs.writeFileSync(path.join(out, "32x32.png"), at[32]);
fs.writeFileSync(path.join(out, "64x64.png"), at[64]);
fs.writeFileSync(path.join(out, "128x128.png"), at[128]);
fs.writeFileSync(path.join(out, "128x128@2x.png"), at[256]);
fs.writeFileSync(path.join(out, "icon.png"), at[512]);

// icns: 'icns' + total length, then (type, length, PNG) entries.
const entries = [["icp4", 16], ["icp5", 32], ["icp6", 64], ["ic07", 128], ["ic08", 256], ["ic09", 512], ["ic10", 1024], ["ic11", 32], ["ic12", 64], ["ic13", 256], ["ic14", 512]];
const parts = entries.map(([type, px]) => {
  const head = Buffer.alloc(8);
  head.write(type, 0, "ascii");
  head.writeUInt32BE(8 + at[px].length, 4);
  return Buffer.concat([head, at[px]]);
});
const body = Buffer.concat(parts);
const head = Buffer.alloc(8);
head.write("icns", 0, "ascii");
head.writeUInt32BE(8 + body.length, 4);
fs.writeFileSync(path.join(out, "icon.icns"), Buffer.concat([head, body]));
console.log(`icons → ${out}: ${sizes.join(", ")} px; icon.icns ${8 + body.length} bytes`);
