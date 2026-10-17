// Renders each board beside it to a PNG at its own size: node render.mjs
import { chromium } from "playwright";
import { readdirSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 1 });
for (const f of readdirSync(here).filter((n) => n.endsWith(".dc.html"))) {
  const m = /"width":(\d+),"height":(\d+)/.exec((await import("node:fs")).readFileSync(join(here, f), "utf8"));
  await page.setViewportSize({ width: +m[1], height: +m[2] });
  await page.goto(pathToFileURL(join(here, f)).href);
  await page.screenshot({ path: join(here, f.replace(".dc.html", ".png")) });
  console.log(f);
}
await browser.close();
