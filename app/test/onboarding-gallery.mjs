// The onboarding gallery: every onboarding inventory row — and Region 6's invite window
// (member-welcome, joining, the member plan, every join-* face, join-switch-confirm) — rendered
// by the built window in WebKit against a stubbed Tauri bridge, light and dark.
//
//   cargo run -q -p shroom-core --example gallery [-- members]   (run for you)
//   npm run build --workspace app                   (run it first: this serves app/dist)
//   node app/test/onboarding-gallery.mjs
//
// For each entry: every line in the Scene is visible text, and the page shows no text the
// Scene didn't give it (webviews compose no sentences); every Scene button is there,
// enabled as the Scene says, and no others; radios, the login box, placeholders and the
// name's value read the Scene; the glyphs take their colours; the nest sits under the
// current row (below the plan when none); the spinner shows where the Scene implies one.
// Then the interactions: a paste goes to paste_secret and leaves the field empty, typing
// the name goes to field(), Return in the name field is Claim (never key("return")).
// Screenshots land in app/test/gallery-out/ (gitignored).

import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { webkit } from "playwright";
import { collect, dist, gallery as load, hex, isButton, isChoice, norm, out, serve, stub, texts as said } from "./gallery-kit.mjs";

if (!existsSync(join(dist, "onboarding.html"))) {
  console.error("app/dist/onboarding.html is missing — run `npm run build --workspace app` first.");
  process.exit(2);
}
mkdirSync(out, { recursive: true });

// Region 6's entries for this window ride along (`-- members`, window "onboarding").
const gallery = [...load(), ...load("members").filter((e) => e.window === "onboarding")];
const server = await serve();
const url = `${server.origin}/onboarding.html`;

const TEXT = new Set(["text", "paragraphs", "label", "header", "line", "lines", "question", "error", "hint", "taken", "quiet", "miss", "prefix", "suffix", "note", "title", "door"]);
const texts = (node) => said(node, TEXT);

// ── the run ──

const results = [];
const check = (name, ok, detail = "") => results.push({ name, ok: !!ok, detail });

// A paste shroom holds shows as a row of dots — the Scene's `held`, drawn (DECISIONS B104).
// One fixed row whichever paste it was: nothing of the paste, not even its length.
const DOTS = /^•{6,}$/;
const masks = new Set();

const browser = await webkit.launch();

async function open(entry, colorScheme = "light") {
  const ctx = await browser.newContext({ colorScheme, viewport: { width: 520, height: 600 }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript(stub, entry.scene);
  await page.goto(url);
  await page.waitForSelector("main.ob");
  await page.waitForTimeout(50);
  return { ctx, page, errors };
}

const calls = (page) => page.evaluate(() => window.__calls);

for (const entry of gallery) {
  const s = entry.scene;
  const nest = s.body.kind === "plan" ? s.body.nest : null;
  for (const scheme of ["light", "dark"]) {
    const tag = `${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open(entry, scheme);

    // Every line visible; nothing else said — but the brand mark heading the window (the
    // mushroom and its “shroom”, DEVIATIONS #45), which says that one word and no other.
    const mark = norm(await page.locator("main.ob > .wordmark:first-child").innerText());
    check(`${tag}: the header is the brand mark alone`, mark === "shroom" && (await page.locator(".wordmark").count()) === 1, JSON.stringify(mark));
    const full = norm(await page.locator("body").innerText());
    const body = full.startsWith(mark) ? norm(full.slice(mark.length)) : full;
    const want = [...new Set(texts(s).map(norm))].sort((a, b) => b.length - a.length);
    const missing = want.filter((t) => !body.includes(t));
    check(`${tag}: every line is visible`, missing.length === 0, missing.join(" | "));
    let rest = body;
    for (const t of want) rest = rest.split(t).join(" ");
    check(`${tag}: no text the Scene didn't give`, norm(rest) === "", norm(rest));

    // Buttons: present, enabled as said, and no others.
    const buttons = collect(s, isButton);
    for (const b of buttons) {
      const loc = page.getByRole("button", { name: b.label, exact: true });
      const n = await loc.count();
      const enabled = n ? await loc.first().isEnabled() : null;
      check(`${tag}: button “${b.label}” ${b.enabled ? "enabled" : "disabled"}`, n === 1 && enabled === b.enabled, `count ${n}, enabled ${enabled}`);
    }
    const shown = await page.locator("button:visible").count();
    check(`${tag}: ${buttons.length} buttons, no others`, shown === buttons.length, `shown ${shown}`);

    // Radios, the login box, placeholders, the name's value.
    for (const c of collect(s, isChoice)) {
      const r = page.getByRole("radio", { name: c.label, exact: true });
      check(`${tag}: radio “${c.label}” ${c.selected ? "on" : "off"}`, (await r.count()) === 1 && (await r.isChecked()) === c.selected);
    }
    if (s.body.kind === "invite") {
      const b = s.body;
      const box = page.getByRole("checkbox");
      check(`${tag}: login box ${b.login ? (b.login.on ? "checked" : "unchecked") : "absent"}`, b.login ? (await box.count()) === 1 && (await box.isChecked()) === b.login.on : (await box.count()) === 0);
      check(`${tag}: paste door ${b.door ? "open" : "absent"}`, (await page.locator(".paste-door input.paste").count()) === (b.door ? 1 : 0));
      if (b.door) check(`${tag}: the door's line labels its field`, (await page.locator(".paste-door input.paste").getAttribute("aria-label")) === b.door);
      if (b.title) check(`${tag}: the title is the hero`, (await page.locator(".invite > h1").innerText()) === b.title);
      const order = await page.locator(".invite .acts button").allInnerTexts();
      check(`${tag}: buttons keep the Scene's order`, order.join("|") === b.buttons.map((x) => x.label).join("|"), order.join("|"));
    }
    if (s.body.kind === "welcome") {
      const box = page.getByRole("checkbox", { name: s.body.login.label, exact: true });
      check(`${tag}: login box ${s.body.login.on ? "checked" : "unchecked"}`, (await box.isChecked()) === s.body.login.on);
      check(`${tag}: no paste field (B159)`, (await page.locator("input.paste").count()) === 0);
      check(`${tag}: ${s.body.uninstall.label} quiet at the foot (B160)`, (await page.locator(".welcome .foot > .b.quiet").innerText()) === s.body.uninstall.label);
      check(`${tag}: sheet ${s.body.sheet ? "over the welcome" : "absent"}`, (await page.locator(".sheet").count()) === (s.body.sheet ? 1 : 0));
    }
    if (nest?.kind === "token") {
      check(`${tag}: token placeholder`, await page.locator(`input.paste[placeholder="${nest.placeholder}"]`).isVisible());
      const shown = await page.locator("input.paste").inputValue();
      if (nest.held) masks.add(shown);
      check(`${tag}: the token field ${nest.held ? "shows its dots" : "is empty"}`, nest.held ? DOTS.test(shown) : shown === "", JSON.stringify(shown));
    } else if (nest?.kind === "join-refused") {
      // A refusal after Join (N31): the paste door where a new link is the way on.
      check(`${tag}: the paste door ${nest.door ? "under the row" : "absent"}`, (await page.locator(".under .paste-door input.paste").count()) === (nest.door ? 1 : 0) && (await page.locator("input.paste").count()) === (nest.door ? 1 : 0));
    } else if (s.body.kind === "plan") {
      check(`${tag}: no paste field`, (await page.locator("input.paste").count()) === 0);
    }
    if (nest?.kind === "name") {
      check(`${tag}: name shows the Scene's value`, (await page.locator("input.name").inputValue()) === nest.value);
      // The field's own edge: the URL's name field (its prefix and suffix sit outside it).
      const red = await page.locator("input.name").evaluate((e) => getComputedStyle(e).borderTopColor);
      const redVar = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--red"));
      check(`${tag}: red edges iff taken`, (red === hex(redVar)) === !!nest.taken, red);
    }

    // The spinner where the Scene implies one; at rest offline.
    const spinning = nest && (["checking", "gate", "calm", "priming"].includes(nest.kind) || (nest.kind === "token" && nest.checking));
    // A row shroom does out of sight (wiring under the permissions, N45) turns its own.
    const working = s.body.kind === "plan" ? s.body.rows.filter((r) => r.glyph === "working").length : 0;
    check(`${tag}: spinner ${spinning ? "turns" : "absent"}`, (await page.locator(".spin:not(.rest)").count()) === (spinning ? 1 : 0) + working);
    // At rest: wizard-offline / join-waiting's nest, or an arrival waiting on its space.
    const resting = nest?.kind === "offline" || nest?.kind === "unexpected" || (s.body.kind === "invite" && s.body.spinner);
    check(`${tag}: resting spinner iff offline or waiting`, (await page.locator(".spin.rest").count()) === (resting ? 1 : 0));

    // The plan: glyph colours, the nest's place.
    if (s.body.kind === "plan") {
      const vars = await page.evaluate(() => {
        const cs = getComputedStyle(document.documentElement);
        // A row still ahead is the canvas's dim dot (DEVIATIONS #45).
        return { current: cs.getPropertyValue("--frost"), done: cs.getPropertyValue("--green"), ahead: cs.getPropertyValue("--dim"), working: cs.getPropertyValue("--dim") };
      });
      const glyphs = await page.locator("li.row .glyph").evaluateAll((els) => els.map((e) => getComputedStyle(e).color));
      const bad = s.body.rows.map((r, i) => (glyphs[i] === hex(vars[r.glyph]) ? null : `${i + 1}:${r.glyph}=${glyphs[i]}`)).filter(Boolean);
      // Six rows for the owner's plan; two for a member's (N30).
      check(`${tag}: glyph colours`, bad.length === 0 && glyphs.length === s.body.rows.length && [2, 6].includes(glyphs.length), bad.join(" "));
      // `current` counts the owner's six rows; a member's two-row plan hangs the nest under
      // the row it marks current (N30: “Row two becomes current”).
      const cur = s.body.current === null ? null : s.body.rows.length === 6 ? s.body.current : s.body.rows.findIndex((r) => r.glyph === "current") + 1;
      // A nest that draws nothing under its row (the row's mark is the spinner; macOS's own
      // prompt is up) has no row of its own (Plan.svelte's `silent`, CHECKPOINT §10, 3.2).
      const silent = cur !== null && (["perm-mic", "perm-camera", "relaunching"].includes(nest.kind) || nest.kind === "checking");
      const placed = cur === null
        ? await page.locator(".plan > .below > .nest").count()
        : await page.locator(`ol.rows > li.row:nth-of-type(${cur}) + li.under > .nest`).count();
      check(`${tag}: nest ${cur === null ? "below the plan" : silent ? `none under row ${cur}` : `under row ${cur}`}`, placed === (silent ? 0 : 1));
    }

    // The window is 520 wide (the runner's inner size): nothing scrolls it sideways.
    const wide = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth }));
    check(`${tag}: fits the window's width`, wide.sw <= wide.cw, JSON.stringify(wide));
    // Emphasis is the Scene's runs, rendered as given — and none the Scene didn't give.
    const runs = collect(s, (o) => Array.isArray(o.runs)).flatMap((o) => o.runs);
    const marked = (k) => runs.filter((r) => r.em === k).map((r) => norm(r.text)).sort();
    const got = await page.evaluate(() => ({
      strong: [...document.querySelectorAll("main strong")].map((e) => e.textContent.replace(/\s+/g, " ").trim()).sort(),
      em: [...document.querySelectorAll("main em")].map((e) => e.textContent.replace(/\s+/g, " ").trim()).sort(),
    }));
    check(`${tag}: emphasis is the Scene's runs`, JSON.stringify(got) === JSON.stringify({ strong: marked("strong"), em: marked("em") }), JSON.stringify(got));
    check(`${tag}: no page errors`, errors.length === 0, errors.join(" | "));
    check(`${tag}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === "onboarding");
    check(`${tag}: rendering calls nothing`, (await calls(page)).length === 0, JSON.stringify(await calls(page)));
    // Nothing runs off the window's 600px height (the window doesn't scroll by design).
    const tall = await page.evaluate(() => Math.ceil(document.querySelector("main.ob").getBoundingClientRect().bottom));
    check(`${tag}: fits the window's height (${tall} ≤ 600)`, tall <= 600);
    await page.screenshot({ path: join(out, `${entry.variant}-${scheme}.png`), fullPage: true });
    await ctx.close();
  }
}

check("token: one row of dots, whichever paste is held", masks.size === 1, JSON.stringify([...masks]));

// ── interactions ──

const by = (variant) => gallery.find((e) => e.variant === variant);
const SECRET = "cfut_ExampleTokenValue0123456789abcdefABCDEF";

async function pasteInto(page, selector, text) {
  return page.evaluate(
    ([sel, t]) => {
      const input = document.querySelector(sel);
      input.focus();
      const dt = new DataTransfer();
      dt.setData("text/plain", t);
      const ev = new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true });
      input.dispatchEvent(ev);
      // Read synchronously: the field is cleared inside the handler, not later.
      return { value: input.value, prevented: ev.defaultPrevented };
    },
    [selector, text],
  );
}

{
  const { ctx, page } = await open(by("token"));
  const r = await pasteInto(page, "input.paste", SECRET);
  const c = await calls(page);
  check("token: a paste calls paste_secret(token, text)", c.some((x) => x.cmd === "paste_secret" && x.args.field === "token" && x.args.text === SECRET), JSON.stringify(c));
  check("token: the field is empty right after the paste", r.value === "" && r.prevented, JSON.stringify(r));
  check("token: the pasted text is nowhere in the page", !(await page.content()).includes(SECRET));
  await page.locator("input.paste").focus();
  await page.keyboard.type("abc");
  check("token: typing lands nothing (paste-only)", (await page.locator("input.paste").inputValue()) === "");
  await page.keyboard.press("Enter");
  check("token: Return in the paste field is the field's", !(await calls(page)).some((x) => x.cmd === "key" || x.cmd === "act"));
  await page.locator("h1").click();
  await page.keyboard.press("Enter");
  check("token: Return elsewhere goes to key(return)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "return" && x.args.surface === "onboarding"));
  await ctx.close();
}
{
  // The Scene says when a paste is held (DECISIONS B104): the field then shows its dots —
  // the same row for any paste, nothing of it — and is empty again when the paste is refused.
  const { ctx, page } = await open(by("token"));
  const value = () => page.locator("input.paste").inputValue();
  const seen = async (text) => (await page.content()).includes(text) || (await page.evaluate((t) => [...document.querySelectorAll("input")].some((i) => i.value.includes(t)), text));
  await page.locator("h1").click();
  await page.evaluate((t) => {
    const dt = new DataTransfer();
    dt.setData("text/plain", t);
    document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
  }, SECRET);
  check("token held: a paste anywhere in the window goes to the runner; the field waits for the Scene", (await calls(page)).some((x) => x.cmd === "paste_secret" && x.args.text === SECRET) && (await value()) === "");
  await page.evaluate((s) => window.__emit(s), by("token-checking").scene);
  await page.waitForFunction(() => document.querySelector("input.paste").value !== "", null, { timeout: 2000 }).catch(() => {});
  const dots = await value();
  check("token held: the field shows its dots", DOTS.test(dots) && masks.has(dots), JSON.stringify(dots));
  check("token held: the dots are not the paste, nor as long as it", dots.length !== SECRET.length && !(await seen(SECRET)));
  // A longer token pasted over a held one: to the runner, and the same dots.
  const LONGER = `${SECRET}0123456789abcdefghijklmnopqrstuvwxyz`;
  const r = await pasteInto(page, "input.paste", LONGER);
  check("token held: another paste goes to the runner and the dots stay as they were", r.prevented && r.value === dots && (await calls(page)).some((x) => x.cmd === "paste_secret" && x.args.text === LONGER), JSON.stringify(r));
  await page.locator("input.paste").focus();
  await page.keyboard.type("abc");
  await page.keyboard.press("Backspace");
  check("token held: typing changes nothing", (await value()) === dots);
  await page.evaluate((s) => window.__emit(s), by("token-saving").scene);
  await page.waitForTimeout(50);
  check("token accepted: the dots stay while it's put away", (await value()) === dots);
  check("token held: nothing of either paste is anywhere in the page", !(await seen(SECRET)) && !(await seen(LONGER)));
  // Refused: empty again, with its line.
  const refused = by("token-wrong-powers").scene;
  await page.evaluate((s) => window.__emit(s), refused);
  await page.waitForFunction(() => document.querySelector("input.paste").value === "", null, { timeout: 2000 }).catch(() => {});
  check("token refused: the field is empty again, with its line", (await value()) === "" && norm(await page.locator(".nest").innerText()).includes(norm(refused.body.nest.error)));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("welcome"));
  const strong = await page.locator("main strong").allInnerTexts();
  const em = await page.locator("main em").allInnerTexts();
  check("welcome: “shroom” bold, “your own” italic (the ratified emphasis)", strong.join("|") === "shroom" && em.join("|") === "your own", `${strong} / ${em}`);
  await ctx.close();
}
// The other webviews hear only their own slice (one "scene" event, targeted per label).
for (const [html, label] of [["popover.html", "popover"], ["countdown.html", "countdown"]]) {
  const ctx = await browser.newContext({ viewport: { width: 520, height: 600 } });
  const page = await ctx.newPage();
  await page.addInitScript(stub, null);
  await page.goto(url.replace("onboarding.html", html));
  await page.waitForFunction(() => window.__listenTarget() !== null, null, { timeout: 5000 }).catch(() => {});
  check(`${label}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === label);
  await ctx.close();
}
{
  const { ctx, page } = await open(by("welcome"));
  await page.keyboard.press("Enter");
  check("welcome: Return goes to key(return)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "return"));
  await page.keyboard.press("Escape");
  check("welcome: Esc goes to key(esc)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "esc"));
  await page.getByRole("checkbox").click();
  const c = await calls(page);
  check("welcome: the login box sends its action", c.some((x) => x.cmd === "act" && x.args.action.do === "set-login-box" && x.args.action.arg === false));
  check("welcome: the box keeps the Scene's state until the Scene changes", await page.getByRole("checkbox").isChecked());
  check("welcome: no invite line (B159)", (await page.getByRole("button", { name: "Have an invite link?" }).count()) === 0);
  // A link pasted anywhere in the welcome goes to the invite field's paste (B159).
  await page.evaluate(() => {
    const dt = new DataTransfer();
    dt.setData("text/plain", "https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp");
    window.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt }));
  });
  check("welcome: a paste anywhere is the invite's", (await calls(page)).some((x) => x.cmd === "paste_secret" && x.args.field === "invite"), JSON.stringify((await calls(page)).slice(-2)));
  await page.getByRole("button", { name: "Uninstall shroom…" }).click();
  check("welcome: Uninstall shroom… sends its action (B160)", (await calls(page)).some((x) => x.cmd === "act" && x.args.action.do === "uninstall"));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("welcome-uninstall"));
  check("welcome-uninstall: the sheet is over the welcome", await page.locator('.sheet[data-sheet="uninstall-confirm"]').isVisible());
  check("welcome-uninstall: the foot is inert under it", (await page.locator(".welcome .foot[inert]").count()) === 1);
  await page.keyboard.press("Escape");
  check("welcome-uninstall: Esc goes to key(esc)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "esc"));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("name"));
  check("name: the field has the keyboard", await page.locator("input.name").evaluate((e) => e === document.activeElement));
  await page.locator("input.name").pressSequentially("my");
  const c = await calls(page);
  check("name: typing calls field(name, …) on every input", c.filter((x) => x.cmd === "field" && x.args.field === "name").map((x) => x.args.value).join(",") === "m,my", JSON.stringify(c));
  await page.keyboard.press("Enter");
  check("name: Return with Claim disabled does nothing", !(await calls(page)).some((x) => x.cmd === "act" || x.cmd === "key"));
  // The Scene answers with its filtered value — and how many edits it has counted (B107:
  // an answer says which edit it answers); the input shows it.
  const typed = by("name-typed").scene;
  const answer = (edits) => ({ ...typed, body: { ...typed.body, nest: { ...typed.body.nest, value: "my", edits } } });
  await page.evaluate((s) => window.__emit(s), answer(2));
  await page.waitForTimeout(30);
  check("name: the input shows the Scene's value back", (await page.locator("input.name").inputValue()) === "my");
  await page.locator("input.name").pressSequentially("X");
  await page.evaluate((s) => window.__emit(s), answer(3));
  await page.waitForTimeout(30);
  check("name: a character the Scene refused is taken back out", (await page.locator("input.name").inputValue()) === "my");
  await ctx.close();
}
{
  const { ctx, page } = await open(by("name-typed"));
  await page.locator("input.name").focus();
  await page.keyboard.press("Enter");
  const c = await calls(page);
  check("name: Return in the field is Claim", c.some((x) => x.cmd === "act" && x.args.surface === "onboarding" && x.args.action.do === "claim"), JSON.stringify(c));
  check("name: Return in the field is not key(return)", !c.some((x) => x.cmd === "key"));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("token-pick-account"));
  await page.getByRole("radio", { name: "Acme Engineering" }).click();
  check("pick-account: a radio sends its pick", (await calls(page)).some((x) => x.cmd === "act" && x.args.action.do === "pick-account" && x.args.action.arg === "acme"));
  check("pick-account: the radios keep the Scene's pick", await page.getByRole("radio", { name: "Bagrat's Account" }).isChecked());
  await page.getByRole("button", { name: "Use this account" }).click();
  check("pick-account: Use this account acts", (await calls(page)).some((x) => x.cmd === "act" && x.args.action.do === "use-account"));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("all-set"));
  await page.getByRole("button", { name: "Record now" }).click();
  check("all-set: Record now acts", (await calls(page)).some((x) => x.cmd === "act" && x.args.action.do === "record-now"));
  const order = await page.locator(".nest button").allInnerTexts();
  check("all-set: buttons keep the Scene's order", order.join("|") === "Close|Invite someone|Record now", order.join("|"));
  await ctx.close();
}

// ── Region 6: the invite window ──

const canon = (a) => JSON.stringify(a, Object.keys(a ?? {}).sort());
const acted = (c, action) => c.some((x) => x.cmd === "act" && x.args.surface === "onboarding" && canon(x.args.action) === canon(action));
{
  const { ctx, page } = await open(by("member-welcome"));
  await page.keyboard.press("Enter");
  check("member-welcome: Return goes to key(return) (the Scene's ret is Join)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "return") && by("member-welcome").scene.ret?.do === "join");
  await page.getByRole("button", { name: "Join", exact: true }).click();
  await page.getByRole("button", { name: "Later", exact: true }).click();
  let c = await calls(page);
  check("member-welcome: Join and Later send their actions", acted(c, { do: "join" }) && acted(c, { do: "later" }), JSON.stringify(c));
  await page.getByRole("checkbox").click();
  c = await calls(page);
  check("member-welcome: the login box sends its action and keeps the Scene's state", acted(c, { do: "set-login-box", arg: false }) && (await page.getByRole("checkbox").isChecked()));
  const xs = await page.locator(".invite .acts button").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().left));
  check("member-welcome: the filled Join first, as its twin the welcome", xs[0] < xs[1] && (await page.locator(".invite .acts button").first().getAttribute("class")).includes("filled"));
  check("member-welcome: no paste field (the app already knows it's an invite)", (await page.locator("input.paste").count()) === 0);
  await ctx.close();
}
{
  const { ctx, page } = await open(by("join-switch-draining"));
  const join = page.getByRole("button", { name: "Join Bagrat's space", exact: true });
  check("join-switch-confirm draining: Join greyed", !(await join.isEnabled()));
  await join.click({ force: true }).catch(() => {});
  await page.getByRole("button", { name: "Stay", exact: true }).click();
  const c = await calls(page);
  check("join-switch-confirm draining: greyed Join sends nothing; Stay does", !acted(c, { do: "join" }) && acted(c, { do: "stay" }), JSON.stringify(c));
  await page.keyboard.press("Enter");
  check("join-switch-confirm draining: no Return default in the Scene", by("join-switch-draining").scene.ret === null);
  await ctx.close();
}
// N31: Join's refusal renders under the joining row, the plan kept; dead at arrival
// (DEVIATIONS #20) the whole window's face, no plan.
for (const variant of ["join-expired", "join-used", "join-space-missing"]) {
  const e = by(variant);
  const { ctx, page } = await open(e);
  check(`${variant}: the plan stays — its header`, (await page.locator(".plan h1").innerText()) === e.scene.body.header);
  const items = await page.locator(".plan ol.rows > li").evaluateAll((els) => els.map((x) => x.className.split(" ")[0] + ":" + x.textContent.replace(/\s+/g, " ").trim().slice(0, 60)));
  check(`${variant}: the line is nested right under “→ Joining Bagrat's space”`, items[0].startsWith("row:→") && items[0].includes("Joining Bagrat") && items[1].startsWith("under:") && items[1].includes(e.scene.body.nest.line.slice(0, 20)), items.join(" | "));
  check(`${variant}: row six follows, still ahead`, items[2]?.startsWith("row:•") && items[2].includes("Letting macOS"), items.join(" | "));
  await ctx.close();
}
for (const variant of ["join-expired-arrival", "join-used-arrival", "join-space-missing-arrival"]) {
  const { ctx, page } = await open(by(variant));
  check(`${variant}: dead at arrival — the whole window's face, no plan`, (await page.locator(".plan").count()) === 0 && (await page.locator(".invite").count()) === 1);
  await ctx.close();
}
{
  const { ctx, page } = await open(by("join-switch-midtake"));
  const boxes = await page.locator(".invite .acts button").evaluateAll((els) => els.map((e) => [e.innerText, e.getBoundingClientRect().top, e.getBoundingClientRect().left]));
  check("join-switch-confirm mid-take: Join then Stay on one line", boxes.length === 2 && boxes[0][0] === "Join Bagrat's space" && boxes[1][0] === "Stay" && Math.abs(boxes[0][1] - boxes[1][1]) < 1 && boxes[0][2] < boxes[1][2], JSON.stringify(boxes));
  const acts = await page.locator(".invite .acts").evaluate((e) => e.getBoundingClientRect().bottom);
  const note = page.locator(".invite > p.note");
  check("join-switch-confirm mid-take: law 4's note once, under the row", (await note.count()) === 1 && (await note.innerText()) === "You can do this once your recording is finished." && (await note.evaluate((e) => e.getBoundingClientRect().top)) >= acts - 0.5);
  check("join-switch-confirm mid-take: no note between the buttons", (await page.locator(".invite .acts p").count()) === 0);
  await ctx.close();
}
for (const variant of ["join-expired", "join-used", "join-expired-arrival", "join-used-arrival"]) {
  const { ctx, page } = await open(by(variant));
  const LINK = "https://v.myshroom.workers.dev/join/q8w3ztNEWLINK";
  check(`${variant}: the door's field has the keyboard`, await page.locator(".paste-door input.paste").evaluate((e) => e === document.activeElement));
  const r = await pasteInto(page, ".paste-door input.paste", LINK);
  let c = await calls(page);
  check(`${variant}: a paste in the door calls paste_secret(invite, text)`, c.length === 1 && c[0].cmd === "paste_secret" && c[0].args.field === "invite" && c[0].args.text === LINK, JSON.stringify(c.map((x) => x.cmd)));
  check(`${variant}: the field is empty right after the paste`, r.value === "" && r.prevented);
  check(`${variant}: the pasted link is nowhere in the page`, !(await page.content()).includes("NEWLINK"));
  await page.evaluate(() => (window.__calls = []));
  // A paste anywhere in the window lands in the door.
  await page.locator(".door-line").first().click();
  await page.evaluate((t) => {
    const dt = new DataTransfer();
    dt.setData("text/plain", t);
    document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
  }, LINK);
  c = await calls(page);
  check(`${variant}: a paste anywhere in the window goes to the door`, c.some((x) => x.cmd === "paste_secret" && x.args.field === "invite"), JSON.stringify(c.map((x) => x.cmd)));
  await ctx.close();
}
for (const variant of ["join-space-missing", "join-space-missing-arrival"]) {
  const { ctx, page } = await open(by(variant));
  await page.evaluate((t) => {
    const dt = new DataTransfer();
    dt.setData("text/plain", t);
    document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
  }, "https://v.myshroom.workers.dev/join/abc");
  check(`${variant}: no door, so a paste lands nowhere`, (await calls(page)).length === 0);
  await ctx.close();
}
{
  const { ctx, page } = await open(by("join-already-here"));
  await page.getByRole("button", { name: "Close", exact: true }).click();
  check("join-already-here: Close sends its action", acted(await calls(page), { do: "close-window" }));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("all-set-member"));
  const order = await page.locator(".nest button").allInnerTexts();
  check("all-set · member: two doors — members don't invite (N5)", order.join("|") === "Close|Record now", order.join("|"));
  await ctx.close();
}
{
  const { ctx, page } = await open(by("done-member"));
  await page.getByRole("button", { name: "Allow recording", exact: true }).click();
  check("done · member: Allow recording sends its action", acted(await calls(page), { do: "allow-recording" }));
  await ctx.close();
}

await browser.close();
server.close();

const failed = results.filter((r) => !r.ok);
for (const f of failed) console.log(`FAIL ${f.name}${f.detail ? ` — ${f.detail}` : ""}`);
console.log(`\n${gallery.length} states × 2 schemes · ${results.length - failed.length}/${results.length} checks passed · screenshots in ${out}`);
process.exit(failed.length ? 1 : 0);
