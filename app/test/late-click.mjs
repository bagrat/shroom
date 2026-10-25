// A click on a control that something late just moved does nothing (spec N82): the popover
// and Settings, rendered by the built pages in WebKit against the stubbed bridge, each handed
// a second Scene that puts something in above the controls — the update's ready banner over
// config, Update and What's new in General's Updates row, where its line was. A pointer click on a control that
// moved or appeared sends nothing within the half-second; the same click a moment later
// does. What doesn't move stays live; the keyboard is never held back; what the person's own
// click changed (a picker's menu) is theirs.
//
//   npm run build --workspace app      (first: this serves app/dist)
//   node app/test/late-click.mjs

import { existsSync } from "node:fs";
import { join } from "node:path";
import { webkit } from "playwright";
import { dist, gallery, serve, stub } from "./gallery-kit.mjs";

for (const f of ["popover.html", "settings.html"]) {
  if (!existsSync(join(dist, f))) {
    console.error(`app/dist/${f} is missing — run \`npm run build --workspace app\` first.`);
    process.exit(2);
  }
}

// The half-second (N82), and a moment past it.
const STEADY_MS = 500;
const PAST = STEADY_MS + 150;

const results = [];
const check = (name, ok, detail = "") => results.push({ name, ok: !!ok, detail });
const region3 = gallery("region3");
const settings = gallery("settings");
const sceneOf = (list, variant) => {
  const e = list.find((x) => x.variant === variant);
  if (!e) throw new Error(`the core's gallery has no ${variant}`);
  return e.scene;
};
const config = sceneOf(region3, "config");
const ready = sceneOf(region3, "update-ready");
const upToDate = sceneOf(settings, "general-up-to-date");
const updateReady = sceneOf(settings, "general-update-ready");

const server = await serve();
const browser = await webkit.launch();

async function open(surface, scene, size) {
  const ctx = await browser.newContext({ viewport: size });
  const page = await ctx.newPage();
  await page.addInitScript(stub, scene);
  await page.goto(`${server.origin}/${surface}.html`);
  await page.waitForSelector(surface === "popover" ? "main.pop" : ".set > .behind > .pane");
  // The first paint is long settled, as a popover that has been open a moment is.
  await page.waitForTimeout(PAST);
  return { ctx, page };
}
const acts = (page) => page.evaluate(() => window.__calls.filter((c) => c.cmd === "act").map((c) => c.args.action.do));
const clear = (page) => page.evaluate(() => (window.__calls = []));
/** The Scene changes by itself: the core's next slice, as the runner emits it. */
const land = async (page, scene) => {
  await page.evaluate((s) => window.__emit(s), scene);
  // Rendered (the next frames), well inside the half-second.
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
};
/** A pointer click at a control's centre, where it is now — as a hand that was already there. */
async function clickAt(locator) {
  const b = await locator.boundingBox();
  await locator.page().mouse.click(b.x + b.width / 2, b.y + b.height / 2);
}

// ── The popover: the ready banner lands over config ──
{
  const { ctx, page } = await open("popover", config, { width: 320, height: 720 });
  const update = page.locator(".banner").getByRole("button", { name: "Update", exact: true });
  const source = page.locator(".prow").first();
  const before = await source.boundingBox();
  await land(page, ready);
  const after = await source.boundingBox();
  check("popover: the banner moves the rows down (the case itself)", after.y > before.y + 40, `${before.y} → ${after.y}`);

  await clearAndClick(page, update);
  check("popover: Update, just landed under the pointer, sends nothing", (await acts(page)).length === 0, (await acts(page)).join(","));
  await clearAndClick(page, source);
  check("popover: a picker row the banner just moved opens nothing", (await page.locator(".menu").count()) === 0);
  check("popover: …and sends nothing", (await acts(page)).length === 0);

  await page.waitForTimeout(PAST);
  await clearAndClick(page, update);
  check("popover: the same Update a moment later sends update", (await acts(page)).join(",") === "update", (await acts(page)).join(","));
  await ctx.close();
}

// A click that took nothing (on a line, between controls) is no change of the person's: the
// banner landing right after it still holds back the click that follows.
{
  const { ctx, page } = await open("popover", config, { width: 320, height: 720 });
  await clickAt(page.locator(".hint"));
  await land(page, ready);
  await clearAndClick(page, page.locator(".banner").getByRole("button", { name: "Update", exact: true }));
  check("popover: a click on a line just before the banner doesn't excuse it", (await acts(page)).length === 0, (await acts(page)).join(","));
  await ctx.close();
}

// The keyboard is never held back: focus doesn't move with the layout.
{
  const { ctx, page } = await open("popover", config, { width: 320, height: 720 });
  await land(page, ready);
  await clear(page);
  await page.locator(".banner").getByRole("button", { name: "Update", exact: true }).focus();
  await page.keyboard.press("Space");
  check("popover: Space on a just-landed Update is taken (the keyboard isn't held back)", (await acts(page)).join(",") === "update", (await acts(page)).join(","));
  await ctx.close();
}

// What doesn't move stays live: a Scene that changes nothing on screen holds nothing back.
{
  const { ctx, page } = await open("popover", ready, { width: 320, height: 720 });
  await land(page, ready);
  await clearAndClick(page, page.locator(".banner").getByRole("button", { name: "Update", exact: true }));
  check("popover: a Scene that moves nothing leaves Update live", (await acts(page)).join(",") === "update", (await acts(page)).join(","));
  await ctx.close();
}

// The person's own click is theirs: a picker's menu opens and its row is picked at once.
{
  const { ctx, page } = await open("popover", config, { width: 320, height: 720 });
  await clear(page);
  await clickAt(page.locator(".prow").nth(1));
  const item = page.locator(".menu button:not([aria-checked='true']):not(:disabled)").first();
  await item.waitFor();
  await clickAt(item);
  check("popover: a menu the person just opened takes its pick at once", (await acts(page)).length === 1, (await acts(page)).join(","));
  await ctx.close();
}

// ── Settings · General: Update and What's new land in the Updates row, where its line was ──
{
  const { ctx, page } = await open("settings", upToDate, { width: 600, height: 688 });
  const copy = page.getByRole("button", { name: upToDate.body.pane.copy_diagnostics.label, exact: true });
  await land(page, updateReady);
  const update = page.getByRole("button", { name: updateReady.body.pane.update.label, exact: true });
  await clearAndClick(page, update);
  check("settings: Update, just landed under the pointer, sends nothing", (await acts(page)).length === 0, (await acts(page)).join(","));
  await clearAndClick(page, copy);
  check("settings: Copy diagnostics, which didn't move, is live at once", (await acts(page)).join(",") === upToDate.body.pane.copy_diagnostics.action.do, (await acts(page)).join(","));
  await page.waitForTimeout(PAST);
  await clearAndClick(page, update);
  check("settings: the same Update a moment later sends update", (await acts(page)).join(",") === "update", (await acts(page)).join(","));
  await ctx.close();
}

async function clearAndClick(page, locator) {
  await clear(page);
  await clickAt(locator);
}

await browser.close();
server.close();

let failed = 0;
for (const r of results) {
  if (!r.ok) failed++;
  console.log(`${r.ok ? "ok  " : "FAIL"} ${r.name}${r.ok || !r.detail ? "" : ` — ${r.detail}`}`);
}
console.log(`late clicks (N82): ${results.length - failed} of ${results.length}`);
if (failed) process.exit(1);
