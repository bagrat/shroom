// Renders shroom's five menu-bar faces (the spec's N24/N25/N37/N38) from SVG:
// plain · red dot (recording) · pause bars (paused) · amber dot (something needs you) ·
// Frost dot (something new). The plain and paused faces are monochrome *template* images
// (macOS tints them for light and dark menu bars); a colored dot can't live in a template
// image, so the dot faces come in a light-menu-bar and a dark-menu-bar variant.
//
//   node tools/render-tray-icons.mjs        → app/src-tauri/icons/tray/*.png (@1x 18 px, @2x 36 px)

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

const out = path.resolve("app/src-tauri/icons/tray");
fs.mkdirSync(out, { recursive: true });

// The POC logo's cap and stem, scaled into an 18-unit box; spots knocked out of the cap.
const mushroom = (ink, badge) => `
  <defs><mask id="m"><rect width="18" height="18" fill="#fff"/>
    <g fill="#000">
      <circle cx="4.6" cy="7.6" r="0.75"/><circle cx="7.5" cy="4.6" r="1.25"/>
      <circle cx="11.6" cy="5.4" r="0.8"/><circle cx="10.1" cy="7.6" r="0.6"/>
    </g>
    ${badge ? `<circle cx="14.2" cy="13.8" r="4.6" fill="#000"/>` : ""}
  </mask></defs>
  <g mask="url(#m)" fill="${ink}">
    <path d="M1.6,8.4 C1.6,4.0 5.0,1.2 9,1.2 C13.0,1.2 16.4,4.0 16.4,8.4 C14.2,10.6 3.8,10.6 1.6,8.4 Z"/>
    <path d="M6.9,10.6 C6.8,13.0 6.2,14.8 6.2,15.6 C6.2,16.6 7.4,17.1 9,17.1 C10.6,17.1 11.8,16.6 11.8,15.6 C11.8,14.8 11.2,13.0 11.1,10.6 Z"/>
  </g>`;

const dot = (color) => `<circle cx="14.2" cy="13.8" r="3.4" fill="${color}"/>`;
const pause = (ink) => `<g fill="${ink}"><rect x="11.9" y="10.9" width="1.7" height="5.8" rx="0.5"/><rect x="14.8" y="10.9" width="1.7" height="5.8" rx="0.5"/></g>`;

// Nord Aurora red/yellow and Frost, tuned for small sizes on both menu bars.
const faces = {
  "plain-template": mushroom("#000", false),
  "paused-template": mushroom("#000", true) + pause("#000"),
  "recording-light": mushroom("#000", true) + dot("#d1414b"),
  "recording-dark": mushroom("#fff", true) + dot("#e25c66"),
  "attention-light": mushroom("#000", true) + dot("#d99a1e"),
  "attention-dark": mushroom("#fff", true) + dot("#ebcb8b"),
  "update-light": mushroom("#000", true) + dot("#4c7fae"),
  "update-dark": mushroom("#fff", true) + dot("#88c0d0"),
};

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 1 });
for (const [name, body] of Object.entries(faces)) {
  for (const [suffix, px] of [["", 18], ["@2x", 36]]) {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18" width="${px}" height="${px}">${body}</svg>`;
    await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
    await page.locator("svg").screenshot({ path: path.join(out, `${name}${suffix}.png`), omitBackground: true });
  }
}
await browser.close();
console.log(`rendered ${Object.keys(faces).length * 2} images into ${path.relative(process.cwd(), out)}`);
