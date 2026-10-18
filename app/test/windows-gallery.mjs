// The Library and Settings gallery: every library and settings scene the core's gallery
// prints — Region 6's too (the Members tab and its sheets, Settings · Space on every kind of
// Mac, Mine · Everyone) — rendered by the built windows in WebKit against a stubbed Tauri
// bridge, at the windows' sizes, light and dark; then the popover's faces — Region 6's
// banners and Region 3's whole recording machine (config and its variants, disk-low,
// recording-off, the paused faces, the questions, the stops) — and the countdown. Then the
// editor (N63): every face the core prints (`-- editor`) over a test video as long as its
// recording, then driven over editor-fixture.mjs's faces and their own video (the gestures
// are placed on that edit), the fixture held to the core's shape and words.
//
//   cargo run -q -p shroom-core --example gallery -- library | settings | members   (run for you)
//   npm run build --workspace app                   (run it first: this serves app/dist)
//   node app/test/windows-gallery.mjs
//
// For each entry: every said-line in the Scene is visible (a row's hover-only buttons show
// on hover or selection), and the page shows no text the Scene didn't give it; every Scene
// button is there, enabled and styled as the Scene says, and no others; the search field,
// the title being renamed, the login box, the tabs and the plan read the Scene; a sheet's
// answer is rightmost and red-outlined, no button holds the keyboard, Return does nothing
// and Esc sends key(esc) (the Scene's esc); nothing scrolls sideways — at the window's size
// and at the Library's minimum width. Then the gestures and keys: each sends the Scene's
// action, and Copy link reads the Scene's “Copied”. Screenshots land in
// app/test/gallery-out/ (gitignored).

import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { webkit } from "playwright";
import { collect, dist, gallery, hex, isButton, norm, out, repo, serve, stub, texts, windowPage } from "./gallery-kit.mjs";
import { editorDrives, editorLooks, makeMedia, serveMedia } from "./editor-preview.mjs";
import { editorScenes, REC } from "./editor-fixture.mjs";

for (const f of ["library.html", "settings.html", "feedback.html", "popover.html", "countdown.html", "editor.html"]) {
  if (!existsSync(join(dist, f))) {
    console.error(`app/dist/${f} is missing — run \`npm run build --workspace app\` first.`);
    process.exit(2);
  }
}
mkdirSync(out, { recursive: true });

// The windows' inner sizes, as the runner builds them (runtime/presenter.rs's WindowSpecs —
// read from there so the gallery and the runner stay in step; the fallbacks are the sizes
// they were written against). The Library resizes down to its minimum; Settings is fixed,
// tall enough for its tallest pane (checked below).
function spec(label, fallback) {
  try {
    const src = readFileSync(join(repo, "app/src-tauri/src/runtime/presenter.rs"), "utf8");
    const m = src.match(new RegExp(`label: ${label},[^}]*?width: ([\\d.]+), height: ([\\d.]+)`));
    if (m) return { width: Number(m[1]), height: Number(m[2]) };
  } catch {}
  return fallback;
}
export const SIZES = {
  library: { ...spec("LIBRARY", { width: 680, height: 560 }), min: { width: 520, height: 360 } },
  settings: spec("SETTINGS", { width: 600, height: 620 }),
  // Send feedback (N60): its page under the title bar.
  feedback: windowPage("FEEDBACK", { width: 520, height: 600 }),
  // The popover is 320 wide and fits its content's height (commands.rs's popover_fit, up to 720).
  popover: { width: 320, height: 720 },
  // The countdown covers its display; a laptop's is drawn here.
  countdown: { width: 1512, height: 982 },
};

// Region 6's entries ride along, each in its own window (`-- members`).
const members = gallery("members");
const library = [...gallery("library"), ...members.filter((e) => e.window === "library")];
const settings = [...gallery("settings"), ...members.filter((e) => e.window === "settings")];
// Region 3's faces (`-- region3`): every popover state of the recording machine, and the count.
const region3 = gallery("region3");
const popovers = [...members, ...region3].filter((e) => e.window === "popover");
const countdowns = region3.filter((e) => e.window === "countdown");
// Send feedback (`-- feedback`): the form in each of its faces, and the sent face (N60).
const feedbacks = gallery("feedback");
// The editor (`-- editor`, N63): its faces over one recording.
const editors = gallery("editor");
const server = await serve();

const TEXT = new Set([
  "title", "label", "header", "line", "lines", "question", "error", "hint", "taken", "quiet", "note",
  "notice", "empty", "footer", "meta", "state", "blocked", "updates_label", "update_line", "help",
  // The Library's greyed-button notes, shown once for the window (never inside a row).
  "window_notes",
  // Region 6: the member's label on a row, Settings · Space's beneath-the-rows lines, the
  // Members tab's You row, a row's fact, the composer's line, a born link's address.
  "member", "mark_line", "you", "fact", "invite_line", "prefix",
  // A line in runs (Rich): who invites say they're from (B158).
  "text",
  // Report a problem (B180): General's help row and its sent line; the sheet's field label,
  // its disclosure, the line while it's gathered, and a failed send's line. The disclosure's
  // text (`preview`) is the person's own report, shown only when opened.
  "help_label", "sent", "reply_label", "disclosure", "preparing", "failed",
  // Send feedback (N60): the line under the log box.
  "log_note",
]);

/** The sheets that ask no destructive question: law 9's colours aren't theirs (B180, as
 * the core's laws.rs NOT_DESTRUCTIVE). */
const NOT_DESTRUCTIVE = ["report-problem"];

const heights = [];
const results = [];
const check = (name, ok, detail = "") => results.push({ name, ok: !!ok, detail });
// A paste shroom holds shows as a row of dots — the Scene's `held`, drawn (DECISIONS B104).
const DOTS = /^•{6,}$/;
const browser = await webkit.launch();

async function open(surface, scene, { colorScheme = "light", size = SIZES[surface] } = {}) {
  const ctx = await browser.newContext({ colorScheme, viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript(stub, scene);
  await page.goto(`${server.origin}/${surface}.html`);
  await page.waitForSelector({ library: ".lib", settings: ".set", feedback: "main.fb", popover: "main.pop", countdown: "#digit:not(:empty)" }[surface]);
  await page.waitForTimeout(60);
  return { ctx, page, errors };
}
const calls = (page) => page.evaluate(() => window.__calls);
const clear = (page) => page.evaluate(() => (window.__calls = []));
const vars = (page) =>
  page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    const v = (n) => cs.getPropertyValue(n).trim();
    return { red: v("--red"), accent: v("--accent"), frost: v("--frost"), green: v("--green"), muted: v("--muted"), dim: v("--dim"), amber: v("--amber") };
  });
const canon = (a) => JSON.stringify(a, Object.keys(a ?? {}).sort());
const sameAction = (a, b) => canon(a) === canon(b);
const acted = (c, surface, action) => c.some((x) => x.cmd === "act" && x.args.surface === surface && sameAction(x.args.action, action));

/** A button's look against its Style (law 9's red outline, red text, the filled ask). */
async function styled(loc, style, v) {
  const s = await loc.evaluate((e) => {
    const cs = getComputedStyle(e);
    return { cls: e.className, color: cs.color, border: cs.borderTopColor, bw: cs.borderTopWidth, bg: cs.backgroundColor };
  });
  if (!s.cls.split(/\s+/).includes(style)) return `class ${s.cls}`;
  if (style === "red-text" && s.color !== hex(v.red)) return `colour ${s.color}`;
  if (style === "red-outline" && (s.border !== hex(v.red) || s.color !== hex(v.red) || s.bw === "0px" || s.bg !== "rgba(0, 0, 0, 0)")) return JSON.stringify(s);
  if (style === "filled" && s.bg !== hex(v.accent)) return `background ${s.bg}`;
  return null;
}

/** Every common check: lines, no extra text, sideways scroll, errors, the listener, quiet render. */
async function common(tag, page, errors, surface, scene, { hidden = [], allow = [] } = {}) {
  const body = norm(await page.locator("body").innerText());
  const visible = [...new Set(texts(scene, TEXT).map(norm))].filter((t) => t && !hidden.includes(t));
  const missing = visible.filter((t) => !body.includes(t));
  check(`${tag}: every line is visible`, missing.length === 0, missing.join(" | "));
  let rest = body;
  for (const t of [...new Set([...visible, ...hidden])].sort((a, b) => b.length - a.length)) rest = rest.split(t).join(" ");
  check(`${tag}: no text the Scene didn't give`, norm(rest) === "", norm(rest));
  const wide = await page.evaluate(() => {
    const els = [document.documentElement, ...document.querySelectorAll(".list, .pane, .sheet, .rows")];
    return els.filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => `${e.className || e.tagName} ${e.scrollWidth}>${e.clientWidth}`);
  });
  check(`${tag}: nothing scrolls sideways`, wide.length === 0, wide.join(" "));
  check(`${tag}: no page errors`, errors.length === 0, errors.join(" | "));
  check(`${tag}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === surface);
  const c = (await calls(page)).filter((x) => !allow.includes(x.cmd));
  check(`${tag}: rendering calls nothing`, c.length === 0, JSON.stringify(c));
}

/** A sheet: its order, its rightmost red-outlined answer, nobody holding the keyboard, Return
 * does nothing, Esc goes to key(esc) as the Scene's esc. Leaves the page's calls cleared. */
async function sheetChecks(tag, page, surface, w, v) {
  const sh = w.body.sheet;
  const labels = await page.locator(".sheet .sheet-acts button").allInnerTexts();
  check(`${tag}: sheet buttons in the Scene's order`, labels.join("|") === sh.buttons.map((b) => b.label).join("|"), labels.join("|"));
  const last = sh.buttons[sh.buttons.length - 1];
  const destructive = !NOT_DESTRUCTIVE.includes(sh.id);
  if (destructive) check(`${tag}: the answer is red-outlined (law 9)`, last.style === "red-outline");
  const xs = await page.locator(".sheet .sheet-acts button").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().right));
  check(`${tag}: the answer is rightmost`, xs.length && xs[xs.length - 1] === Math.max(...xs), JSON.stringify(xs));
  const tops = await page.locator(".sheet .sheet-acts button").evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
  check(`${tag}: the buttons sit on one line (Cancel beside the answer)`, new Set(tops).size === 1, JSON.stringify(tops));
  check(`${tag}: nothing but buttons in the button row`, (await page.locator(".sheet .sheet-acts > :not(button)").count()) === 0);
  const notes = [...new Set(sh.buttons.map((b) => b.note).filter((n) => n && n !== sh.blocked))];
  check(`${tag}: a greyed answer's note sits with the lines`, JSON.stringify(await page.locator(".sheet > p.note").allInnerTexts()) === JSON.stringify(notes));
  const bad = await styled(page.locator(".sheet .sheet-acts button").last(), last.style, v);
  check(destructive ? `${tag}: the answer looks red-outlined, never filled` : `${tag}: the answer looks as the Scene says`, !bad, bad ?? "");
  check(`${tag}: no button holds the keyboard`, await page.evaluate(() => !(document.activeElement instanceof HTMLButtonElement)));
  check(`${tag}: the window behind is inert`, (await page.locator(".behind[inert]").count()) === 1);
  check(`${tag}: no Return default in the Scene`, w.ret === null);
  await clear(page);
  await page.keyboard.press("Enter");
  // Even with the answer focused: Return never clicks it.
  await page.locator(".sheet .sheet-acts button").last().focus().catch(() => {});
  await page.keyboard.press("Enter");
  const c1 = await calls(page);
  check(`${tag}: Return does nothing`, !c1.some((x) => x.cmd === "act" || (x.cmd === "key" && x.args.key === "return")), JSON.stringify(c1));
  await clear(page);
  await page.keyboard.press("Escape");
  const c2 = await calls(page);
  check(`${tag}: Esc sends the Scene's esc`, w.esc !== null && c2.some((x) => x.cmd === "key" && x.args.surface === surface && x.args.key === "esc"), JSON.stringify(c2));
  await clear(page);
}

/** What in a row is cut short (ellipsized or hidden): its title, its date · length, its
 * state word. innerText still holds clipped text, so this measures. */
const unclipped = (row) =>
  row.evaluate((li) =>
    [...li.querySelectorAll(".title, .line2, .line2 .meta, .state")]
      .filter((e) => e.scrollWidth > e.clientWidth + 1 || e.getBoundingClientRect().right > li.getBoundingClientRect().right + 0.5)
      .map((e) => `${e.className} “${e.textContent}” ${e.scrollWidth}>${e.clientWidth}`),
  );

// ── the Library ──

for (const entry of library) {
  const w = entry.scene;
  const b = w.body;
  const hoverLabels = b.rows.filter((r) => !r.selected).flatMap((r) => r.hover.map((h) => norm(h.label)));
  const menuLabels = b.rows.flatMap((r) => r.menu.map((m) => norm(m.label)));
  // Hidden until hover or a right-click: allowed, not required. A title being renamed lives in its field.
  const shownElsewhere = new Set([...b.rows.filter((r) => r.selected).flatMap((r) => r.hover.map((h) => norm(h.label))), ...b.rows.flatMap((r) => r.buttons.map((x) => norm(x.label)))]);
  const hidden = [...new Set([...hoverLabels, ...menuLabels])].filter((l) => !shownElsewhere.has(l) && !(b.sheet?.buttons ?? []).some((x) => norm(x.label) === l));
  const renamed = b.rows.filter((r) => r.rename && r.title).map((r) => norm(r.title));
  const rowNotes = [...new Set(b.rows.flatMap((r) => [...r.buttons, ...r.hover]).map((x) => x.note).filter(Boolean))];
  const sceneForText = { ...w, body: { ...b, window_notes: rowNotes, rows: b.rows.map((r) => ({ ...r, title: r.rename ? null : r.title, hover: r.selected ? r.hover : [], menu: [] })) } };

  for (const scheme of ["light", "dark"]) {
    const tag = `library/${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open("library", w, { colorScheme: scheme });
    const v = await vars(page);
    await common(tag, page, errors, "library", sceneForText, { hidden: [...hidden, ...renamed.filter((t) => !shownElsewhere.has(t))] });

    // The search field reads the Scene.
    const s = page.locator('input[data-field="library-search"]');
    check(`${tag}: search placeholder and value`, (await s.getAttribute("placeholder")) === b.search.placeholder && (await s.inputValue()) === b.search.value);
    check(`${tag}: search ${b.search.enabled ? "enabled" : "disabled"}`, (await s.isEnabled()) === b.search.enabled);
    // The empty / no-match line stands in place of the list.
    check(`${tag}: ${b.empty ? "the empty line, no rows" : "rows, no empty line"}`, (await page.locator(".list > .empty").count()) === (b.empty ? 1 : 0) && (await page.locator("li.lrow").count()) === b.rows.length);
    check(`${tag}: footer ${b.footer ? "shows the count" : "absent"}`, b.footer ? (await page.locator("footer.foot").innerText()) === b.footer : (await page.locator("footer.foot").count()) === 0);
    check(`${tag}: notice ${b.notice ? "on top" : "absent"}`, (await page.locator(".toolbar + .notice").count()) === (b.notice ? 1 : 0));
    // A greyed button's note shows once for the window, never inside a row (a row's room is
    // its title's and its state word's).
    const notesShown = await page.locator(".behind > .notes").allInnerTexts();
    check(`${tag}: row notes once, under the toolbar`, JSON.stringify(notesShown) === JSON.stringify(rowNotes), JSON.stringify(notesShown));
    check(`${tag}: no note inside a row`, (await page.locator("li.lrow .note").count()) === 0);
    // Rows keep the Scene's order.
    const keys = await page.locator("li.lrow").evaluateAll((els) => els.map((e) => e.dataset.key));
    check(`${tag}: rows in the Scene's order`, keys.join("|") === b.rows.map((r) => r.key).join("|"));

    // *Mine · Everyone*: the Scene's choices, the selected one marked; absent when empty.
    const seg = await page.locator(".toolbar .scope [role=radio]").evaluateAll((els) => els.map((e) => ({ label: e.textContent.trim(), on: e.getAttribute("aria-checked") === "true", en: !e.disabled })));
    check(`${tag}: scope ${b.scope.length ? "is the Scene's" : "absent"}`, JSON.stringify(seg) === JSON.stringify(b.scope.map((c) => ({ label: c.label, on: c.selected, en: c.enabled }))), JSON.stringify(seg));
    for (const r of b.rows) {
      const tagged = await page.locator(`li.lrow[data-key="${r.key}"] .line2 .member`).allInnerTexts();
      check(`${tag}: row ${r.key} ${r.member ? `carries “${r.member}”` : "carries no member's label"}`, JSON.stringify(tagged) === JSON.stringify(r.member ? [r.member] : []), JSON.stringify(tagged));
    }
    let expected = (b.sheet ? b.sheet.buttons.length : 0) + b.scope.length;
    for (const r of b.rows) {
      const row = page.locator(`li.lrow[data-key="${r.key}"]`);
      const rt = `${tag}: row ${r.key}`;
      check(`${rt} ${r.selected ? "selected" : "not selected"}`, (await row.getAttribute("aria-selected")) === String(r.selected));
      check(`${rt} pinned dot iff pinned`, (await row.locator(".dot").count()) === (r.pinned ? 1 : 0));
      if (r.title !== null && !r.rename) {
        const t = row.locator(".title");
        const clipped = await t.evaluate((e) => e.scrollWidth > e.clientWidth + 1);
        check(`${rt} title shown whole`, (await t.innerText()) === r.title && !clipped);
      }
      if (r.state) {
        const st = row.locator(".state");
        const colour = await st.evaluate((e) => getComputedStyle(e).color);
        check(`${rt} state word in its tone`, r.tone ? colour === hex(v[r.tone]) : colour !== hex(v.amber), colour);
      }
      const tag2 = row.locator(".line2 .member");
      if (r.member) {
        const cut = await tag2.evaluate((e) => e.scrollWidth > e.clientWidth + 1 || e.getBoundingClientRect().right > e.closest("li").getBoundingClientRect().right);
        check(`${rt} the member's label unclipped`, !cut);
      }
      const clipped = await unclipped(row);
      check(`${rt} title, date · length and state word unclipped`, clipped.length === 0, clipped.join(" "));
      if (r.rename) {
        const f = row.locator('input[data-field="title"]');
        check(`${rt} title field shows the Scene's value`, (await f.inputValue()) === r.rename.value);
        check(`${rt} title field has the keyboard (activates)`, (await f.evaluate((e) => e === document.activeElement)) === w.activates);
        check(`${rt} Return/Esc are the Scene's (ret saves, esc cancels)`, w.ret?.do === "commit-rename" && w.esc?.do === "cancel-rename");
      }
      if (r.progress !== null) {
        const pct = await row.locator(".bar > span").evaluate((e) => parseFloat(e.style.width));
        check(`${rt} the bar reads ${r.progress}%`, pct === r.progress, String(pct));
      } else check(`${rt} no bar`, (await row.locator(".bar").count()) === 0);
      for (const x of r.buttons) {
        const loc = row.locator(".tail .always button", { hasText: x.label });
        const n = await loc.count();
        check(`${rt} button “${x.label}” ${x.enabled ? "enabled" : "disabled"}`, n === 1 && (await loc.isEnabled()) === x.enabled && (await loc.isVisible()), `count ${n}`);
        if (n === 1) {
          const bad = await styled(loc, x.style, v);
          check(`${rt} “${x.label}” styled ${x.style}`, !bad, bad ?? "");
        }
      }
      expected += b.sheet ? 0 : r.buttons.length + (r.selected ? r.hover.length : 0);
      const hov = row.locator(".tail .hover button");
      check(`${rt} hover buttons ${r.selected ? "shown (selected)" : "hidden until hover"}`, (await hov.count()) === r.hover.length && (r.hover.length === 0 || (await hov.first().isVisible()) === r.selected));
    }
    if (b.sheet) expected += b.rows.reduce((n, r) => n + r.buttons.length + (r.selected ? r.hover.length : 0), 0);
    const shown = await page.locator("button:visible").count();
    check(`${tag}: ${expected} buttons showing, no others`, shown === expected, `shown ${shown}`);

    // Hover each row: its hover buttons appear, enabled and styled as said.
    if (!b.sheet) {
      for (const r of b.rows.filter((r) => r.hover.length)) {
        const row = page.locator(`li.lrow[data-key="${r.key}"]`);
        await row.hover();
        for (const x of r.hover) {
          const loc = row.locator(".tail .hover button", { hasText: x.label });
          const bad = (await loc.count()) === 1 ? await styled(loc, x.style, v) : "missing";
          check(`${tag}: row ${r.key} on hover “${x.label}” ${x.enabled ? "enabled" : "disabled"}, ${x.style}`, (await loc.isVisible()) && (await loc.isEnabled()) === x.enabled && !bad, bad ?? "");
        }
      }
      await page.mouse.move(1, 1);
    }

    await page.screenshot({ path: join(out, `library-${entry.variant}-${scheme}.png`), animations: "disabled" });
    if (b.sheet) {
      check(`${tag}: the sheet's title`, (await page.locator(".sheet h2").innerText()) === b.sheet.title);
      await sheetChecks(tag, page, "library", w, v);
    }
    await ctx.close();
  }

  // The Library's minimum width: still nothing sideways, every title whole or ellipsized in place.
  {
    const { ctx, page, errors } = await open("library", w, { size: SIZES.library.min });
    const wide = await page.evaluate(() =>
      [document.documentElement, ...document.querySelectorAll(".list, .rows, .lrow")].filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => e.className || e.tagName),
    );
    check(`library/${entry.variant} [min width]: nothing scrolls sideways`, wide.length === 0, wide.join(" "));
    const btns = await page.locator(".tail .always button:visible").evaluateAll((els) => els.filter((e) => e.getBoundingClientRect().right > window.innerWidth).length);
    check(`library/${entry.variant} [min width]: every button inside the window`, btns === 0, String(btns));
    for (const r of b.rows) {
      const cut = await unclipped(page.locator(`li.lrow[data-key="${r.key}"]`));
      check(`library/${entry.variant} [min width]: row ${r.key} title, date · length and state word unclipped`, cut.length === 0, cut.join(" "));
    }
    check(`library/${entry.variant} [min width]: no page errors`, errors.length === 0, errors.join(" | "));
    await page.screenshot({ path: join(out, `library-${entry.variant}-min.png`), animations: "disabled" });
    await ctx.close();
  }
}

// ── the Library: gestures and keys ──

const lib = (variant) => library.find((e) => e.variant === variant).scene;
{
  const w = lib("library");
  const { ctx, page } = await open("library", w);
  const row = (k) => page.locator(`li.lrow[data-key="${k}"]`);
  await row("r502").locator(".meta").click();
  check("library: a click selects (the Scene's click)", acted(await calls(page), "library", { do: "select-row", arg: "r502" }));
  await clear(page);
  await row("r19").locator(".meta").click({ modifiers: ["Meta"] });
  check("library: ⌘-click toggles", acted(await calls(page), "library", { do: "toggle-row", arg: "r19" }) && !(await calls(page)).some((x) => x.args?.action?.do === "select-row"));
  await clear(page);
  await row("onb").locator(".meta").click({ modifiers: ["Shift"] });
  check("library: ⇧-click extends", acted(await calls(page), "library", { do: "extend-row", arg: "onb" }));
  await clear(page);
  await row("r502").locator(".title").click();
  await page.waitForTimeout(500);
  let c = await calls(page);
  check("library: a click on the title renames (after the double-click interval)", acted(c, "library", { do: "start-rename", arg: "r502" }), JSON.stringify(c));
  await clear(page);
  await row("r19").locator(".title").dblclick();
  await page.waitForTimeout(500);
  c = await calls(page);
  check("library: a double-click opens the page, never renames", acted(c, "library", { do: "open-page", arg: "r19" }) && !c.some((x) => x.args?.action?.do === "start-rename"), JSON.stringify(c));
  await clear(page);
  await row("r502").getByRole("button", { name: "Copy link", exact: true }).click();
  c = await calls(page);
  check("library: Copy link sends its action, selects nothing", acted(c, "library", { do: "copy-row", arg: "r502" }) && c.length === 1, JSON.stringify(c));
  // The runner answers with the Scene where it reads “Copied” (N26's pattern).
  await page.evaluate((s) => window.__emit(s), lib("library-every-row"));
  await page.waitForTimeout(30);
  check("library: Copy link reads the Scene's “Copied”", (await row("r502").locator(".tail .always button").innerText()) === "Copied");
  const widths = await row("r19").locator(".tail .always button").evaluate((e) => e.getBoundingClientRect().width);
  const copiedW = await row("r502").locator(".tail .always button").evaluate((e) => e.getBoundingClientRect().width);
  check("library: “Copied” doesn't shift the row", Math.abs(widths - copiedW) < 0.5, `${widths} vs ${copiedW}`);
  await page.evaluate((s) => window.__emit(s), w);
  await page.waitForTimeout(30);
  await clear(page);
  await row("r19").hover();
  await row("r19").getByRole("button", { name: "Open page" }).click();
  await row("r19").getByRole("button", { name: "Delete…" }).click();
  c = await calls(page);
  check("library: Open page and Delete… send their actions", acted(c, "library", { do: "open-page", arg: "r19" }) && acted(c, "library", { do: "delete-row", arg: "r19" }), JSON.stringify(c));
  await clear(page);
  // Right-click: the Scene's menu, in its order.
  await row("onb").locator(".meta").click({ button: "right" });
  const items = await page.locator(".menu [role=menuitem]").allInnerTexts();
  check("library: right-click shows the Scene's menu", items.join("|") === w.body.rows[2].menu.map((m) => m.label).join("|"), items.join("|"));
  check("library: right-click sends nothing by itself", (await calls(page)).length === 0);
  await page.keyboard.press("Escape");
  check("library: Esc closes the menu, nothing else", (await page.locator(".menu").count()) === 0 && (await calls(page)).length === 0);
  await row("onb").locator(".meta").click({ button: "right" });
  await page.locator(".menu [role=menuitem]", { hasText: "Rename" }).click();
  check("library: a menu pick sends its action and closes", acted(await calls(page), "library", { do: "start-rename", arg: "onb" }) && (await page.locator(".menu").count()) === 0);
  await clear(page);
  // Copy thumbnail sits under Copy link in a live row's menu (N46) and sends its own action.
  await row("onb").locator(".meta").click({ button: "right" });
  await page.screenshot({ path: join(out, "library-row-menu.png"), animations: "disabled" });
  const thumbItem = page.locator(".menu [role=menuitem]", { hasText: "Copy thumbnail" });
  check("library: Copy thumbnail is in a live row's menu, under Copy link", (await thumbItem.count()) === 1 && items.indexOf("Copy thumbnail") === items.findIndex((t) => /^Cop(y link|ied)$/.test(t)) + 1, items.join("|"));
  await thumbItem.click();
  check("library: Copy thumbnail sends its action", acted(await calls(page), "library", { do: "copy-thumb-row", arg: "onb" }), JSON.stringify(await calls(page)));
  await clear(page);
  // Search: every keystroke to field(); its Esc clears; its Return is its own.
  const s = page.locator('input[data-field="library-search"]');
  await s.pressSequentially("50");
  c = await calls(page);
  check("library: typing in search goes to field(library-search)", c.filter((x) => x.cmd === "field" && x.args.field === "library-search").map((x) => x.args.value).join(",") === "5,50", JSON.stringify(c));
  await clear(page);
  await page.keyboard.press("Enter");
  check("library: Return in search does nothing", (await calls(page)).length === 0);
  await page.keyboard.press("Escape");
  c = await calls(page);
  check("library: Esc in search clears it (field, not key)", c.some((x) => x.cmd === "field" && x.args.field === "library-search" && x.args.value === "") && !c.some((x) => x.cmd === "key"), JSON.stringify(c));
  // The Scene answers — the three edits above counted (“5”, “50”, the Esc), its own value
  // in the field (DECISIONS B107: an answer says which edit it answers); the field shows
  // that value back.
  const answered = lib("library-no-match");
  await page.evaluate((s) => window.__emit(s), { ...answered, body: { ...answered.body, search: { ...answered.body.search, edits: 3 } } });
  await page.waitForTimeout(30);
  check("library: search shows the Scene's value back", (await s.inputValue()) === "zzz");
  await clear(page);
  // No Return default, no Esc default, in the plain window.
  await page.evaluate((s) => window.__emit(s), w);
  await page.waitForTimeout(30);
  await page.locator(".toolbar h1").click();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  check("library: no Return or Esc default (the Scene names none)", (await calls(page)).length === 0, JSON.stringify(await calls(page)));
  await ctx.close();
}
{
  // Keys on the selection: ⌘C copies, Delete asks.
  const w = lib("library-every-row");
  const { ctx, page } = await open("library", w);
  await page.locator(".toolbar h1").click();
  await page.keyboard.press("Meta+c");
  check("library: ⌘C on the selection is its Copy link", acted(await calls(page), "library", { do: "copy-row", arg: "r19" }));
  await clear(page);
  await page.keyboard.press("Backspace");
  check("library: Delete on the selection asks (delete-selected)", acted(await calls(page), "library", { do: "delete-selected" }));
  await clear(page);
  const pinned = page.locator('li.lrow.pinned');
  await pinned.locator(".meta").click();
  check("library: a recovered take's row has no click (the Scene gives none)", (await calls(page)).length === 0);
  await pinned.getByRole("button", { name: "Get link" }).click();
  await pinned.getByRole("button", { name: "Discard…" }).click();
  const c = await calls(page);
  check("library: Get link / Discard… on the recovered take", acted(c, "library", { do: "get-link-row", arg: "20261001-090000-cd34" }) && acted(c, "library", { do: "discard-row", arg: "20261001-090000-cd34" }), JSON.stringify(c));
  await clear(page);
  await page.locator('li.lrow[data-key="upspace"]').getByRole("button", { name: "Save to Movies" }).click();
  check("library: Save to Movies sends its action", acted(await calls(page), "library", { do: "save-row-to-movies", arg: "qspace" }));
  await ctx.close();
}
{
  const w = lib("library-renaming");
  const { ctx, page } = await open("library", w);
  const f = page.locator('input[data-field="title"]');
  check("library: the title field opens selected, ready to retype", await f.evaluate((e) => e.selectionStart === 0 && e.selectionEnd === e.value.length));
  await f.press("ArrowRight");
  await f.pressSequentially("!");
  let c = await calls(page);
  check("library: typing a title goes to field(title)", c.some((x) => x.cmd === "field" && x.args.field === "title" && x.args.value === "Where the 502s come from!"), JSON.stringify(c));
  await clear(page);
  await f.press("Enter");
  c = await calls(page);
  check("library: Return in the title is the Scene's ret (key return)", c.length === 1 && c[0].cmd === "key" && c[0].args.key === "return" && c[0].args.surface === "library", JSON.stringify(c));
  await clear(page);
  await f.press("Escape");
  c = await calls(page);
  check("library: Esc in the title is the Scene's esc (key esc)", c.some((x) => x.cmd === "key" && x.args.key === "esc") && !c.some((x) => x.cmd === "field"), JSON.stringify(c));
  await clear(page);
  await page.locator(".toolbar h1").click();
  c = await calls(page);
  check("library: a click away saves (the Scene's ret)", c.some((x) => x.cmd === "key" && x.args.key === "return"), JSON.stringify(c));
  await ctx.close();
}
{
  const w = lib("library-offline");
  const { ctx, page } = await open("library", w);
  const row = page.locator('li.lrow[data-key="r502"]');
  await row.locator(".title").click();
  await page.waitForTimeout(500);
  check("library offline: a title click doesn't rename (the Scene gives none)", !(await calls(page)).some((x) => x.args?.action?.do === "start-rename"));
  await clear(page);
  await row.locator(".meta").click({ button: "right" });
  check("library offline: Rename and Delete… greyed in the menu", !(await page.locator(".menu [role=menuitem]", { hasText: "Rename" }).isEnabled()) && !(await page.locator(".menu [role=menuitem]", { hasText: "Delete…" }).isEnabled()));
  check("library offline: Copy link stays live", await page.locator(".menu [role=menuitem]", { hasText: "Copy link" }).isEnabled());
  await ctx.close();
}
for (const variant of ["library-delete-sheet", "library-delete-sheet-several"]) {
  const w = lib(variant);
  const { ctx, page } = await open("library", w);
  await page.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  const c = await calls(page);
  check(`${variant}: Cancel is back, Delete confirms`, acted(c, "library", { do: "back" }) && acted(c, "library", { do: "confirm-delete" }), JSON.stringify(c));
  await clear(page);
  await page.keyboard.press("Meta+c");
  await page.keyboard.press("Backspace");
  check(`${variant}: the window behind takes no keys`, (await calls(page)).length === 0, JSON.stringify(await calls(page)));
  await ctx.close();
}

// ── Settings ──

for (const entry of settings) {
  const w = entry.scene;
  const b = w.body;
  for (const scheme of ["light", "dark"]) {
    const tag = `settings/${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open("settings", w, { colorScheme: scheme });
    const v = await vars(page);
    await common(tag, page, errors, "settings", w);

    // The toolbar: the Scene's tabs, the selected one marked.
    const tabs = await page.locator("[role=tab]").evaluateAll((els) => els.map((e) => ({ label: e.textContent.trim(), on: e.getAttribute("aria-selected") === "true", en: !e.disabled })));
    check(`${tag}: tabs are the Scene's`, JSON.stringify(tabs) === JSON.stringify(b.tabs.map((t) => ({ label: t.label, on: t.selected, en: t.enabled }))), JSON.stringify(tabs));

    // Buttons: present (as many as the Scene has of each label), enabled and styled as said; no others.
    const buttons = collect(b, isButton);
    const byLabel = new Map();
    for (const x of buttons) byLabel.set(x.label, [...(byLabel.get(x.label) ?? []), x]);
    for (const [label, xs] of byLabel) {
      const loc = page.locator("button:not([role=tab])", { hasText: label }).filter({ hasText: new RegExp(`^${label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`) });
      const n = await loc.count();
      const en = n ? await loc.evaluateAll((els) => els.map((e) => !e.disabled)) : [];
      check(`${tag}: button “${label}” ×${xs.length}, ${xs.map((x) => (x.enabled ? "enabled" : "disabled")).join("/")}`, n === xs.length && JSON.stringify(en) === JSON.stringify(xs.map((x) => x.enabled)), `count ${n} ${JSON.stringify(en)}`);
      for (let i = 0; i < Math.min(n, xs.length); i++) {
        const bad = await styled(loc.nth(i), xs[i].style, v);
        check(`${tag}: “${label}” styled ${xs[i].style}`, !bad, bad ?? "");
      }
    }
    const shown = await page.locator("button:visible:not([role=tab])").count();
    check(`${tag}: ${buttons.length} buttons, no others`, shown === buttons.length, `shown ${shown}`);

    if (b.pane.kind === "general") {
      const g = b.pane;
      const box = page.getByRole("checkbox", { name: g.login.label, exact: true });
      check(`${tag}: login box ${g.login.on ? "checked" : "unchecked"}`, (await box.isChecked()) === g.login.on && (await box.isEnabled()) === g.login.enabled);
      check(`${tag}: the help line is one line with its doors in it`, norm(await page.locator(".help").innerText()) === norm(g.help));
    } else if (b.pane.kind === "members") {
      const m = b.pane;
      check(`${tag}: the head and its one line of truth`, (await page.locator(".members > h1").innerText()) === m.title && (await page.locator(".members > .truth").innerText()) === m.line);
      const you = page.locator(".mrows > li.you");
      check(`${tag}: You first, with no fact, Add another Mac beside it`, (await page.locator(".mrows > li").first().evaluate((e) => e.classList.contains("you"))) && (await you.locator(".mline").innerText()) === m.you && (await you.locator(".fact").count()) === 0 && (await you.getByRole("button", { name: m.add_mac.label, exact: true }).count()) === 1);
      const keys = await page.locator(".mrows > li.mrow:not(.you)").evaluateAll((els) => els.map((e) => e.dataset.key));
      check(`${tag}: the other Macs, then members and invites, in the Scene's order`, keys.join("|") === [...m.macs, ...m.rows].map((r) => r.key).join("|"), keys.join("|"));
      check(`${tag}: the other Macs sit under You as sub-rows`, (await page.locator(".mrows > li.mrow.sub").evaluateAll((els) => els.map((e) => e.dataset.key))).join("|") === m.macs.map((r) => r.key).join("|"));
      for (const r of [...m.macs, ...m.rows]) {
        const row = page.locator(`.mrows > li.mrow[data-key="${r.key}"]`);
        check(`${tag}: row ${r.key} reads its line${r.fact ? " and its fact" : ""}`, (await row.locator(".mline").innerText()) === r.line && (r.fact ? (await row.locator(".fact").innerText()) === r.fact : (await row.locator(".fact").count()) === 0));
        const labels = await row.locator(".mbtns button").allInnerTexts();
        check(`${tag}: row ${r.key} buttons at its end, the Scene's`, labels.join("|") === r.buttons.map((x) => x.label).join("|"), labels.join("|"));
        const cut = await row.evaluate((li) => [...li.querySelectorAll(".mline, .fact, button")].filter((e) => e.getBoundingClientRect().right > li.getBoundingClientRect().right + 0.5 || e.scrollWidth > e.clientWidth + 1).length);
        check(`${tag}: row ${r.key} nothing clipped`, cut === 0);
        // A born link's nest sits right under its row.
        const nest = await row.evaluate((li) => li.nextElementSibling?.classList.contains("mnest") ? li.nextElementSibling.querySelector(".born")?.dataset.link : null);
        check(`${tag}: row ${r.key} ${r.born ? "has its link's nest under it" : "has no nest"}`, r.born ? nest === String(r.born.link) : nest === null || nest === undefined, String(nest));
      }
      const pills = [...(m.mac_link ? [m.mac_link] : []), ...m.rows.filter((r) => r.born).map((r) => r.born)];
      check(`${tag}: ${pills.length} link pill(s)`, (await page.locator(".born").count()) === pills.length);
      if (m.mac_link) {
        const under = await page.locator(".mrows > li.you").evaluate((li) => li.nextElementSibling?.querySelector(".born")?.dataset.link);
        check(`${tag}: Add another Mac's link nests under You`, under === String(m.mac_link.link));
      }
      for (const pl of pills) {
        const pill = page.locator(`.born[data-link="${pl.link}"]`);
        check(`${tag}: pill ${pl.link} shows the address up to its code, then the code's mask`, (await pill.locator(".pill .pre").innerText()) === pl.prefix && (await pill.locator(".pill .code[aria-hidden=true]").count()) === 1);
        const btn = pill.locator(".pill-row button");
        const bad = await styled(btn, pl.copy.style, v);
        check(`${tag}: pill ${pl.link} [ ${pl.copy.label} ] ${pl.copy.style}`, (await btn.innerText()) === pl.copy.label && !bad, bad ?? "");
        const cut = await pill.evaluate((e) => [...e.querySelectorAll(".pill, .pre, button, p")].filter((x) => x.scrollWidth > x.clientWidth + 1 || x.getBoundingClientRect().right > e.closest(".pane").getBoundingClientRect().right).length);
        check(`${tag}: pill ${pl.link} nothing clipped`, cut === 0);
      }
      const who = page.locator('input[data-field="invite-name"]');
      check(`${tag}: the composer reads the Scene (placeholder, value)`, (await who.getAttribute("placeholder")) === m.name.placeholder && (await who.inputValue()) === m.name.value && (await who.isEnabled()) === m.name.enabled);
      check(`${tag}: the composer has the keyboard ${w.activates && !b.sheet ? "(activates)" : "only when it may"}`, (await who.evaluate((e) => e === document.activeElement)) === (w.activates && !b.sheet));
      check(`${tag}: Return in the composer is the Scene's Invite`, w.ret?.do === "invite" || !!b.sheet);
      check(`${tag}: no paste field`, (await page.locator("input.paste").count()) === 0);
    } else {
      const p = b.pane.plan;
      const glyphs = await page.locator("li.row .glyph").evaluateAll((els) => els.map((e) => getComputedStyle(e).color));
      // A row ahead's dot is the canvas's #7b8494 (--dim), as the wizard draws it.
      const want = { current: v.frost, done: v.green, ahead: v.dim };
      const bad = p.rows.map((r, i) => (glyphs[i] === hex(want[r.glyph]) ? null : `${i + 1}:${r.glyph}=${glyphs[i]}`)).filter(Boolean);
      check(`${tag}: glyph colours`, bad.length === 0 && glyphs.length === p.rows.length, bad.join(" "));
      const placed = p.current === null
        ? await page.locator(".plan > .below > .nest").count()
        : await page.locator(`ol.rows > li.row:nth-of-type(${p.current}) + li.under > .nest`).count();
      check(`${tag}: nest ${p.current === null ? "below the plan" : `under row ${p.current}`}`, placed === 1);
      check(`${tag}: the nest is the Scene's (${p.nest.kind})`, (await page.locator(`.nest[data-nest="${p.nest.kind}"]`).count()) === 1);
      const spinning = ["checking", "gate", "calm", "priming"].includes(p.nest.kind) || (p.nest.kind === "token" && p.nest.checking);
      check(`${tag}: spinner ${spinning ? "turns" : "absent"}`, (await page.locator(".spin:not(.rest)").count()) === (spinning ? 1 : 0));
      if (p.nest.kind === "token") check(`${tag}: token placeholder`, await page.locator(`input.paste[placeholder="${p.nest.placeholder}"]`).isVisible());
      if (p.nest.kind === "token") {
        const shown = await page.locator(".nest input.paste").inputValue();
        check(`${tag}: the token field ${p.nest.held ? "shows its dots" : "is empty"}`, p.nest.held ? DOTS.test(shown) : shown === "", JSON.stringify(shown));
      }
      check(`${tag}: paste fields — ${p.nest.kind === "token" ? "the token's" : "no token field"}, no invite door (B159)`, (await page.locator("input.paste").count()) === (p.nest.kind === "token" ? 1 : 0));
      // Beneath the rows: the mark switch (owner-class), then the exits.
      const sw = page.getByRole("switch");
      check(`${tag}: mark switch ${b.pane.mark ? (b.pane.mark.on ? "on" : "off") : "absent"}`, b.pane.mark ? (await sw.count()) === 1 && (await sw.isChecked()) === b.pane.mark.on && (await sw.isEnabled()) === b.pane.mark.enabled : (await sw.count()) === 0);
      if (b.pane.mark) check(`${tag}: the switch is labelled with the Scene's words`, (await page.getByRole("switch", { name: b.pane.mark.label, exact: true }).count()) === 1);
      check(`${tag}: exits at the foot`, (await page.locator(".space > .exits button").count()) === b.pane.exits.length);
      // A denied microphone's door hangs right under its row (the last one), before the
      // plan's closing line.
      check(
        `${tag}: microphone door ${b.pane.mic ? "under the last row" : "absent"}`,
        b.pane.mic
          ? await page.evaluate(() => {
              const m = document.querySelector("ol.rows > li.mic");
              const rows = document.querySelectorAll("ol.rows > li.row");
              let prev = m?.previousElementSibling;
              while (prev?.classList.contains("under")) prev = prev.previousElementSibling;
              return !!m && !!m.querySelector("button") && prev === rows[rows.length - 1];
            })
          : (await page.locator(".mic").count()) === 0,
      );
    }

    await page.screenshot({ path: join(out, `settings-${entry.variant}-${scheme}.png`), fullPage: true, animations: "disabled" });
    const bottom = await page.evaluate(() => Math.ceil(document.querySelector(".set > .behind > .pane").getBoundingClientRect().bottom));
    if (b.pane.kind === "members") check(`${tag}: the toolbar's tabs include Members`, b.tabs.some((t) => t.label === "Members"));
    heights.push(bottom);
    check(`${tag}: the pane fits the window (${bottom} ≤ ${SIZES.settings.height})`, bottom <= SIZES.settings.height);
    if (b.sheet) {
      check(`${tag}: the sheet's title`, (await page.locator(".sheet h2").innerText()) === b.sheet.title);
      check(`${tag}: blocked line ${b.sheet.blocked ? "shown" : "absent"}`, (await page.locator(".sheet .blocked").count()) === (b.sheet.blocked ? 1 : 0));
      await sheetChecks(tag, page, "settings", w, v);
    }
    await ctx.close();
  }
}

// ── Settings: clicks and keys ──

const set = (variant) => settings.find((e) => e.variant === variant).scene;
{
  const { ctx, page } = await open("settings", set("general-update-ready"));
  await page.getByRole("tab", { name: "Space" }).click();
  await page.getByRole("tab", { name: "General" }).click();
  let c = await calls(page);
  check("settings: a tab sends its action; the selected one nothing", acted(c, "settings", { do: "settings-tab", arg: "space" }) && c.length === 1, JSON.stringify(c));
  await clear(page);
  await page.getByRole("checkbox").click();
  c = await calls(page);
  check("settings: the login box sends its action", acted(c, "settings", { do: "set-login-item", arg: true }));
  check("settings: the box keeps the Scene's state until the Scene changes", !(await page.getByRole("checkbox").isChecked()));
  await clear(page);
  for (const name of ["Update", "What's new", "Copy diagnostics", "Report a problem…", "GitHub"]) await page.getByRole("button", { name, exact: true }).click();
  c = await calls(page);
  check(
    "settings: Update, What's new, the help row and the GitHub door send their actions",
    ["update", "whats-new", "copy-diagnostics", "report-problem", "report-issue"].every((d) => acted(c, "settings", { do: d })),
    JSON.stringify(c),
  );
  await clear(page);
  await page.locator(".update-line").click();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  check("settings General: no Return or Esc default (the Scene names none)", (await calls(page)).length === 0);
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("repair-token"));
  await page.locator(".plan h1").click();
  await page.keyboard.press("Enter");
  check("settings repair: Return is the Scene's ret (key return)", (await calls(page)).some((x) => x.cmd === "key" && x.args.key === "return" && x.args.surface === "settings"));
  await clear(page);
  const SECRET = "cfut_ExampleTokenValue0123456789abcdefABCDEF";
  const r = await page.evaluate((t) => {
    const input = document.querySelector("input.paste");
    input.focus();
    const dt = new DataTransfer();
    dt.setData("text/plain", t);
    const ev = new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true });
    input.dispatchEvent(ev);
    return { value: input.value, prevented: ev.defaultPrevented };
  }, SECRET);
  const c = await calls(page);
  check("settings repair: a paste goes to paste_secret(token) and nowhere else", c.length === 1 && c[0].cmd === "paste_secret" && c[0].args.field === "token" && c[0].args.text === SECRET, JSON.stringify(c.map((x) => x.cmd)));
  check("settings repair: the field is empty right after the paste", r.value === "" && r.prevented);
  check("settings repair: the pasted text is nowhere in the page", !(await page.content()).includes(SECRET));
  // The Scene says the paste is held: the field shows its dots; refused, it's empty again.
  const field = () => page.locator(".nest input.paste").inputValue();
  await page.evaluate((s) => window.__emit(s), set("repair-token-checking"));
  await page.waitForFunction(() => document.querySelector(".nest input.paste").value !== "", null, { timeout: 2000 }).catch(() => {});
  const dots = await field();
  check("settings repair: a held paste shows as dots, nothing of the paste", DOTS.test(dots) && dots.length !== SECRET.length && !(await page.content()).includes(SECRET), JSON.stringify(dots));
  await page.evaluate((s) => window.__emit(s), set("repair-other-account"));
  await page.waitForFunction(() => document.querySelector(".nest input.paste").value === "", null, { timeout: 2000 }).catch(() => {});
  check("settings repair: turned down, the field is empty again with its line", (await field()) === "" && norm(await page.locator(".nest").innerText()).includes(norm(set("repair-other-account").body.pane.plan.nest.error)));
  await clear(page);
  await page.getByRole("button", { name: "Create token in Cloudflare" }).click();
  check("settings repair: Create token sends its action", acted(await calls(page), "settings", { do: "create-token" }));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-screen-off"));
  await page.getByRole("button", { name: "Turn it back on" }).click();
  await page.getByRole("button", { name: "Disconnect this Mac…" }).click();
  const c = await calls(page);
  check("settings: Turn it back on and Disconnect this Mac… send their actions", acted(c, "settings", { do: "turn-back-on" }) && acted(c, "settings", { do: "disconnect" }), JSON.stringify(c));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-mic-off"));
  await page.getByRole("button", { name: "Turn on the microphone" }).click();
  check("settings: the microphone's quiet door sends its action", acted(await calls(page), "settings", { do: "open-mic-settings" }));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("disconnect-sheet"));
  await page.getByRole("button", { name: "Disconnect", exact: true }).click();
  await page.getByRole("button", { name: "Cancel" }).click();
  const c = await calls(page);
  check("settings sheet: Disconnect confirms, Cancel is back", acted(c, "settings", { do: "confirm-disconnect" }) && acted(c, "settings", { do: "back" }), JSON.stringify(c));
  await clear(page);
  await page.getByRole("tab", { name: "General" }).click({ force: true }).catch(() => {});
  check("settings sheet: the window behind takes no clicks", (await calls(page)).length === 0);
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("disconnect-sheet-blocked"));
  await page.getByRole("button", { name: "Disconnect", exact: true }).click({ force: true }).catch(() => {});
  check("settings sheet blocked: Disconnect is greyed and sends nothing", !(await calls(page)).some((x) => x.cmd === "act"));
  await ctx.close();
}

// ── Region 6: the Members tab, Settings · Space's new doors, Mine · Everyone ──

const pasteAt = (page, selector, text) =>
  page.evaluate(
    ([sel, t]) => {
      const el = sel ? document.querySelector(sel) : document.body;
      if (sel) el.focus();
      const dt = new DataTransfer();
      dt.setData("text/plain", t);
      const ev = new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true });
      el.dispatchEvent(ev);
      return { value: el.value ?? null, prevented: ev.defaultPrevented };
    },
    [selector, text],
  );
// The core's own “Copied” (N26's pattern), read off the born link the core shows copied.
const COPIED = set("members-born-copied").body.pane.rows.find((r) => r.born).born.copy.label;
{
  const w = set("members");
  const { ctx, page } = await open("settings", w);
  const who = page.locator('input[data-field="invite-name"]');
  await who.pressSequentially("Li");
  let c = await calls(page);
  check("members: typing a name goes to field(invite-name) on every input", c.filter((x) => x.cmd === "field" && x.args.field === "invite-name").map((x) => x.args.value).join(",") === "L,Li", JSON.stringify(c));
  await clear(page);
  await who.press("Enter");
  c = await calls(page);
  check("members: Return with Invite greyed does nothing", c.length === 0, JSON.stringify(c));
  // The Scene answers with the typed name (its two edits counted, B107) and Invite live.
  const typed = { ...w, body: { ...w.body, pane: { ...w.body.pane, name: { ...w.body.pane.name, value: "Li", edits: 2 }, invite: { ...w.body.pane.invite, enabled: true } } } };
  await page.evaluate((s) => window.__emit(s), typed);
  await page.waitForTimeout(30);
  check("members: the composer shows the Scene's value back", (await who.inputValue()) === "Li");
  await who.press("Enter");
  c = await calls(page);
  check("members: Return in the composer is Invite (never key(return))", acted(c, "settings", { do: "invite" }) && !c.some((x) => x.cmd === "key"), JSON.stringify(c));
  await clear(page);
  await page.getByRole("button", { name: "Invite", exact: true }).click();
  check("members: [ Invite ] sends its action", acted(await calls(page), "settings", { do: "invite" }));
  await clear(page);
  await page.evaluate((s) => window.__emit(s), w);
  await page.waitForTimeout(30);
  await page.getByRole("button", { name: "Add another Mac", exact: true }).click();
  await page.locator('li.mrow[data-key="i_sona"]').getByRole("button", { name: "New link", exact: true }).click();
  await page.locator('li.mrow[data-key="m_mher"]').getByRole("button", { name: "Remove…", exact: true }).click();
  await page.locator('li.mrow[data-key="i_ani"]').getByRole("button", { name: "Remove…", exact: true }).click();
  await page.locator('li.mrow[data-key="d_air"]').getByRole("button", { name: "Remove…", exact: true }).click();
  c = await calls(page);
  check(
    "members: Add another Mac, New link and each Remove… send the Scene's action",
    acted(c, "settings", { do: "add-mac" }) &&
      acted(c, "settings", { do: "new-link", arg: "i_sona" }) &&
      acted(c, "settings", { do: "remove", arg: { kind: "member", id: "m_mher" } }) &&
      acted(c, "settings", { do: "remove", arg: { kind: "invite", id: "i_ani" } }) &&
      acted(c, "settings", { do: "remove", arg: { kind: "mac", id: "d_air" } }),
    JSON.stringify(c),
  );
  await ctx.close();
}
for (const variant of ["members-born", "members-add-another-mac"]) {
  const w = set(variant);
  const m = w.body.pane;
  const pill = m.mac_link ?? m.rows.find((r) => r.born).born;
  const { ctx, page } = await open("settings", w);
  const btn = page.locator(`.born[data-link="${pill.link}"] .pill-row button`);
  const before = await btn.evaluate((e) => e.getBoundingClientRect().width);
  await btn.click();
  let c = await calls(page);
  check(`${variant}: Copy link copies this link only — by its handle (law 10, law 13)`, c.length === 1 && acted(c, "settings", { do: "copy-invite", arg: pill.link }), JSON.stringify(c));
  // The runner answers with the Scene where it reads “Copied” — the core's own entry for
  // the member's link; the own-Mac nest takes the same word.
  const copied = variant === "members-born" ? set("members-born-copied") : JSON.parse(JSON.stringify(w));
  if (variant !== "members-born") copied.body.pane.mac_link.copy.label = COPIED;
  await page.evaluate((s) => window.__emit(s), copied);
  await page.waitForTimeout(30);
  check(`${variant}: Copy link reads the Scene's “${COPIED}”`, (await btn.innerText()) === COPIED);
  const after = await btn.evaluate((e) => e.getBoundingClientRect().width);
  check(`${variant}: “${COPIED}” doesn't shift the pill`, Math.abs(before - after) < 0.5, `${before} vs ${after}`);
  check(`${variant}: Invite drops to outline while the link shows`, m.invite.style === "plain" && !(await page.getByRole("button", { name: "Invite", exact: true }).getAttribute("class")).includes("filled"));
  // The code never reaches the page: the pill holds the handle and the address up to /join/.
  const html = await page.content();
  check(`${variant}: the page holds no code past /join/`, !new RegExp(`${pill.prefix.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[A-Za-z0-9]`).test(html));
  await ctx.close();
}
for (const variant of ["remove-member", "remove-invite", "remove-mac"]) {
  const w = set(variant);
  const { ctx, page } = await open("settings", w);
  const [cancel, answer] = w.body.sheet.buttons;
  await page.getByRole("button", { name: cancel.label, exact: true }).click();
  await page.getByRole("button", { name: answer.label, exact: true }).click();
  let c = await calls(page);
  check(`${variant}: ${cancel.label} is back, ${answer.label} confirms`, acted(c, "settings", { do: "back" }) && acted(c, "settings", { do: "confirm-remove" }), JSON.stringify(c));
  await clear(page);
  await page.locator('input[data-field="invite-name"]').focus().catch(() => {});
  await page.keyboard.press("Enter");
  c = await calls(page);
  check(`${variant}: the composer behind the sheet takes no Return`, !c.some((x) => x.cmd === "act"), JSON.stringify(c));
  check(`${variant}: the composer doesn't hold the keyboard under the sheet`, !(await page.locator('input[data-field="invite-name"]').evaluate((e) => e === document.activeElement)));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-owner-mark"));
  await page.getByRole("switch").click();
  const c = await calls(page);
  check("space · owner: the mark switch sends its action", acted(c, "settings", { do: "set-mark", arg: false }), JSON.stringify(c));
  check("space · owner: the switch keeps the Scene's state until the Scene changes", await page.getByRole("switch").isChecked());
  await clear(page);
  const LINK = "v.myshroom.workers.dev/join/q8w3ztSECRET";
  // No door shows (B159): a paste anywhere in the tab is the invite's.
  await page.locator(".plan h1").click();
  const r = await pasteAt(page, null, LINK);
  const c2 = await calls(page);
  check("space · owner: a paste anywhere goes to paste_secret(invite) and nowhere else", c2.length === 1 && c2[0].cmd === "paste_secret" && c2[0].args.field === "invite" && c2[0].args.text === LINK, JSON.stringify(c2.map((x) => x.cmd)));
  check("space · owner: the paste is the window's, not the page's", r.prevented);
  check("space · owner: the pasted link is nowhere in the page", !(await page.content()).includes("SECRET"));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-member"));
  await page.locator(".plan h1").click();
  await pasteAt(page, null, "https://v.myshroom.workers.dev/join/abc");
  let c = await calls(page);
  check("space · member: a paste anywhere is the invite's", c.some((x) => x.cmd === "paste_secret" && x.args.field === "invite"), JSON.stringify(c.map((x) => x.cmd)));
  await clear(page);
  await page.getByRole("button", { name: "Leave this space…", exact: true }).click();
  check("space · member: Leave this space… sends its action", acted(await calls(page), "settings", { do: "disconnect" }));
  check("space · member: no mark switch, no Members tab", (await page.getByRole("switch").count()) === 0 && (await page.getByRole("tab", { name: "Members" }).count()) === 0);
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-owner-class"));
  await page.getByRole("button", { name: "Add a token here", exact: true }).click();
  check("space · owner-class: Add a token here sends its action", acted(await calls(page), "settings", { do: "add-token" }));
  await ctx.close();
}
{
  const { ctx, page } = await open("settings", set("space-owner-class-add-token"));
  await page.locator(".plan h1").click();
  await pasteAt(page, null, "cfut_ExampleTokenValue0123456789abcdefABCDEF");
  const c = await calls(page);
  check("space · owner-class token: a paste anywhere lands in the token field first", c.some((x) => x.cmd === "paste_secret" && x.args.field === "token") && !c.some((x) => x.cmd === "paste_secret" && x.args.field === "invite"), JSON.stringify(c.map((x) => x.cmd)));
  await ctx.close();
}
{
  const w = set("leave-sheet");
  const { ctx, page } = await open("settings", w);
  await page.getByRole("button", { name: "Save them to Movies first", exact: true }).click();
  await page.getByRole("button", { name: "Leave", exact: true }).click();
  const c = await calls(page);
  check("leave sheet: Save them to Movies first and Leave send their actions", acted(c, "settings", { do: "save-stranded-to-movies" }) && acted(c, "settings", { do: "confirm-disconnect" }), JSON.stringify(c));
  await ctx.close();
}
{
  const w = lib("library-everyone");
  const { ctx, page } = await open("library", w);
  await page.getByRole("radio", { name: "Everyone", exact: true }).click();
  check("library everyone: the selected scope sends nothing", (await calls(page)).length === 0);
  await page.getByRole("radio", { name: "Mine", exact: true }).click();
  check("library everyone: Mine sends its action", acted(await calls(page), "library", { do: "set-scope", arg: "mine" }));
  await clear(page);
  await page.locator('li.lrow[data-key="rmher"] .meta').click();
  check("library everyone: a click on another member's row still selects", acted(await calls(page), "library", { do: "select-row", arg: "rmher" }));
  await ctx.close();
}
{
  const w = lib("library-delete-theirs");
  const { ctx, page } = await open("library", w);
  await page.getByRole("radio", { name: "Mine", exact: true }).click({ force: true }).catch(() => {});
  check("library delete theirs: the scope behind the sheet takes no clicks", (await calls(page)).length === 0);
  await ctx.close();
}

// ── Region 6: the popover's member banners and the stranded stop ──

// The popover's pickers (DEVIATIONS #43, #45): a row per picker — its accessible name is the
// Scene's label and the pick's name ("Record: Built-in Display"), its menu the Scene's
// choices, greyed as it says.
const pickers = (c) => [
  ["source", c.record_label, c.sources],
  ["mic", c.mic_label, c.mics],
  ["camera", c.camera_label, c.cameras],
];
// The row names what it records: System default's own device, beneath it in the menu (N44).
const isDefault = (x) => x.action.do.endsWith("-default");
const shownOf = (choices) => {
  const c = choices.find((x) => x.selected) ?? choices.find((x) => !isDefault(x) && (x.action.arg == null || x.action.arg?.kind === "camera")) ?? choices[0];
  return c?.detail ?? c?.label ?? "";
};
async function menuOf(page, i, shot) {
  const row = page.locator(".picker").nth(i).locator(".prow");
  if ((await row.getAttribute("aria-disabled")) === "true") return null;
  await row.click();
  const items = await page.locator(".picker").nth(i).locator('[role="menuitemradio"]').evaluateAll((os) => os.map((o) => ({ label: o.textContent.trim(), disabled: o.disabled, checked: o.getAttribute("aria-checked") === "true" })));
  // The menu open, for the canvas's menu boards (MicMenu, CameraMenu).
  if (shot) await page.screenshot({ path: shot, fullPage: true, animations: "disabled" });
  await page.keyboard.press("Escape");
  return items;
}

for (const entry of popovers) {
  const p = entry.scene;
  for (const scheme of ["light", "dark"]) {
    const tag = `popover/${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open("popover", p, { colorScheme: scheme });
    const banner = page.locator(".banner");
    if (p.banner) {
      check(`${tag}: the banner says the Scene's line`, (await banner.locator("p").first().innerText()) === p.banner.line);
      check(`${tag}: the banner is ${p.banner.tone}`, (await banner.getAttribute("class")).split(/\s+/).includes(p.banner.tone));
      const labels = await banner.locator("button").allInnerTexts();
      check(`${tag}: the banner's buttons are the Scene's`, labels.join("|") === p.banner.buttons.map((x) => x.label).join("|"), labels.join("|"));
      for (const x of p.banner.buttons) {
        const cls = await banner.getByRole("button", { name: x.label, exact: true }).getAttribute("class");
        check(`${tag}: “${x.label}” styled ${x.style}`, cls.split(/\s+/).includes(x.style), cls);
        const font = await banner.getByRole("button", { name: x.label, exact: true }).evaluate((e) => getComputedStyle(e).fontFamily === getComputedStyle(document.body).fontFamily);
        check(`${tag}: “${x.label}” in the popover's own type`, font);
      }
    } else check(`${tag}: no banner`, (await banner.count()) === 0);
    if (p.body.kind === "config") {
      const c = p.body;
      const start = page.getByRole("button", { name: c.start.label, exact: true });
      check(`${tag}: Start recording ${c.start.enabled ? "live" : "greyed"}`, (await start.isEnabled()) === c.start.enabled);
      // Library · Settings · Quit, as the header's icons, named by the Scene.
      const tools = await page.locator(".head .tools button").evaluateAll((bs) => bs.map((b) => b.getAttribute("aria-label")));
      check(`${tag}: the header's icons are the Scene's footer`, tools.join("|") === c.footer.map((f) => f.label).join("|"), tools.join("|"));
      for (const [i, [what, label, choices]] of pickers(c).entries()) {
        const row = page.locator(".picker").nth(i).locator(".prow");
        const name = await row.getAttribute("aria-label");
        check(`${tag}: the ${what} row is named “${label}: ${shownOf(choices)}”`, name === `${label}: ${shownOf(choices)}`, name);
        // Greyed only when nothing real can be picked: nothing live but No mic or Camera off.
        const greyed = !choices.some((x) => x.enabled && !(x.action.do !== "set-source" && !isDefault(x) && x.action.arg == null));
        check(`${tag}: the ${what} row ${greyed ? "greyed" : "live"}`, ((await row.getAttribute("aria-disabled")) === "true") === greyed);
        const items = await menuOf(page, i, entry.variant === "config" ? join(out, `popover-config-menu-${what}-${scheme}.png`) : undefined);
        if (items) {
          check(`${tag}: the ${what} menu is the Scene's choices, greyed and picked as it says`, JSON.stringify(items.map((o) => [o.label, o.disabled, o.checked])) === JSON.stringify(choices.map((x) => [x.label + (x.detail ?? ""), !x.enabled, x.selected])), JSON.stringify(items));
        }
      }
      check(`${tag}: the menus close again`, (await page.locator(".menu").count()) === 0);
    }
    if (p.body.kind === "paused") {
      const b = p.body;
      check(`${tag}: Resume ${b.resume.enabled ? "live" : "greyed"}`, (await page.getByRole("button", { name: b.resume.label, exact: true }).isEnabled()) === b.resume.enabled);
      const rows = await page.locator(".card .rowbtn").evaluateAll((bs) => bs.map((x) => x.querySelector(".what > span").textContent));
      const order = [b.stop, ...(b.marker ? [b.marker] : []), b.restart, b.discard];
      check(`${tag}: the rows are Finish & Share, ${b.marker ? "Drop a marker, " : ""}Restart, Discard… (the canvas's order)`, rows.join("|") === order.map((x) => x.label).join("|"), rows.join("|"));
      const discard = await page.locator(".card .rowbtn").last().getAttribute("class");
      check(`${tag}: Discard… is red text (law 9)`, discard.split(/\s+/).includes("red-text"), discard);
      if (b.marker) {
        // N64: the board's flag in the accent while it can be dropped; once dropped at this
        // pause, greyed with a tick.
        const row = page.locator(".card .rowbtn").nth(1);
        check(`${tag}: “${b.marker.label}” ${b.marker.enabled ? "live" : "greyed"}`, (await row.isEnabled()) === b.marker.enabled);
        const flag = await row.locator("path.flag").count();
        check(`${tag}: ${b.marker.enabled ? "the flag" : "a tick, no flag"} beside it`, (flag === 1) === b.marker.enabled);
        if (b.marker.enabled) {
          const fill = await row.locator("path.flag").evaluate((e) => getComputedStyle(e).fill);
          check(`${tag}: the flag is filled in the accent`, fill === hex((await vars(page)).accent), fill);
        }
      }
    }
    if (p.body.kind === "question") {
      const labels = await page.locator(".answers button").allInnerTexts();
      check(`${tag}: the question's buttons are the Scene's`, labels.join("|") === p.body.buttons.map((x) => x.label).join("|"), labels.join("|"));
      const last = p.body.buttons.at(-1);
      const cls = await page.locator(".answers button").last().getAttribute("class");
      check(`${tag}: the answer is rightmost, styled ${last.style}`, cls.split(/\s+/).includes(last.style), cls);
      check(`${tag}: no Return default (law 9)`, p.ret == null);
    }
    if (p.body.kind === "stopped") {
      const line = p.body.title == null ? await page.locator(".told .line").innerText() : await page.locator(".title .context").innerText();
      check(`${tag}: the stop's title and line`, (p.body.title == null || (await page.locator("main.pop h1").innerText()) === p.body.title) && line === p.body.line);
      const labels = await page.locator(".stack button").allInnerTexts();
      check(`${tag}: the stop's buttons are the Scene's`, labels.join("|") === p.body.buttons.map((x) => x.label).join("|"), labels.join("|"));
    }
    const body = norm(await page.locator("main.pop").innerText());
    const b = p.body;
    // config's field labels are its rows' names now (checked above); the picks are visible.
    const bodyLines =
      b.kind === "config"
        ? [b.quality_label, b.room_line, b.mic_line, b.camera_line, b.hint, b.start.label, b.start.note, b.mic_hint?.label, b.camera_settings?.label, ...b.qualities.map((q) => q.label), ...pickers(b).map(([, , cs]) => shownOf(cs))]
        : b.kind === "paused"
          ? [b.title, b.context, b.sub, ...[b.resume, b.stop, b.marker, b.restart, b.discard].filter(Boolean).map((x) => x.label)]
          : b.kind === "question"
            ? [b.title, b.line, ...b.buttons.map((x) => x.label)]
            : [b.title, b.line, b.link, ...b.buttons.map((x) => x.label)];
    const lines = [p.banner?.line, ...(p.banner?.buttons ?? []).map((x) => x.label), ...bodyLines, p.status_line].filter(Boolean).map(norm);
    const missing = lines.filter((t) => !body.includes(t));
    check(`${tag}: every line is visible`, missing.length === 0, missing.join(" | "));
    const wide = await page.evaluate(() => [document.documentElement, ...document.querySelectorAll(".pop, .banner, .row, .card, .answers")].filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => e.className || e.tagName));
    check(`${tag}: nothing scrolls sideways at 320`, wide.length === 0, wide.join(" "));
    const cut = await page.evaluate(() => [...document.querySelectorAll(".banner p, .banner button, main.pop h1, .context, .told .line, .hint, .status, .answers button, .stack button")].filter((e) => e.scrollWidth > e.clientWidth + 1 || e.getBoundingClientRect().right > document.documentElement.clientWidth).length);
    check(`${tag}: nothing clipped`, cut === 0);
    check(`${tag}: no page errors`, errors.length === 0, errors.join(" | "));
    check(`${tag}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === "popover");
    const c = (await calls(page)).filter((x) => x.cmd !== "popover_fit");
    check(`${tag}: rendering calls nothing but its fit`, c.length === 0, JSON.stringify(c));
    // The popover follows the person (DEVIATIONS #39): its window losing the keyboard —
    // another app coming forward, another Space — closes nothing. What closes it is the
    // runner's to see (Esc, the mushroom, a click somewhere else).
    await page.evaluate(() => window.dispatchEvent(new Event("blur")));
    const lost = (await calls(page)).filter((x) => x.cmd !== "popover_fit");
    check(`${tag}: losing the keyboard closes nothing`, lost.length === 0, JSON.stringify(lost));
    const h = await page.evaluate(() => Math.ceil(document.querySelector("main.pop").getBoundingClientRect().height));
    check(`${tag}: fits the popover's tallest (${h} ≤ 720)`, h <= 720);
    // The menus' checks left a row focused: the picture is the popover as it opens.
    await page.evaluate(() => document.activeElement?.blur());
    await page.screenshot({ path: join(out, `popover-${entry.variant}-${scheme}.png`), fullPage: false, clip: { x: 0, y: 0, width: 320, height: Math.min(720, h + 4) }, animations: "disabled" });
    await ctx.close();
  }
}
// ── Region 3: the countdown — the Scene's digit and line on the scrim, light and dark ──
for (const entry of countdowns) {
  for (const scheme of ["light", "dark"]) {
    const tag = `countdown/${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open("countdown", entry.scene, { colorScheme: scheme });
    check(`${tag}: the digit is the Scene's`, (await page.locator("#digit").innerText()) === String(entry.scene.digit));
    check(`${tag}: the line is the Scene's`, (await page.locator("#line").innerText()) === entry.scene.line);
    check(`${tag}: no page errors`, errors.length === 0, errors.join(" | "));
    check(`${tag}: listens to its own slice`, (await page.evaluate(() => window.__listenTarget()))?.label === "countdown");
    await clear(page);
    await page.mouse.click(200, 200);
    await page.keyboard.press("Escape");
    const c = await calls(page);
    check(`${tag}: any click cancels, Esc too`, acted(c, "countdown", { do: "cancel-countdown" }) && c.some((x) => x.cmd === "key" && x.args.key === "esc"), JSON.stringify(c));
    await page.screenshot({ path: join(out, `countdown-${entry.variant}-${scheme}.png`), animations: "disabled" });
    await ctx.close();
  }
}

// Region 3's controls send the Scene's actions.
{
  const pop = (variant) => popovers.find((e) => e.variant === variant).scene;
  // Drop a marker (N64) sends its action; dropped, the greyed row sends nothing.
  const mk = await open("popover", pop("paused"));
  await clear(mk.page);
  await mk.page.getByRole("button", { name: pop("paused").body.marker.label, exact: true }).click();
  check("paused: Drop a marker sends its action", acted(await calls(mk.page), "popover", pop("paused").body.marker.action), JSON.stringify(await calls(mk.page)));
  await mk.ctx.close();
  const md = await open("popover", pop("paused-marker-dropped"));
  await clear(md.page);
  await md.page.getByRole("button", { name: pop("paused-marker-dropped").body.marker.label, exact: true }).click({ force: true });
  check("paused-marker-dropped: Marker dropped sends nothing", !(await calls(md.page)).some((x) => x.cmd === "act"), JSON.stringify(await calls(md.page)));
  await md.ctx.close();
  const cfg = await open("popover", pop("recording-off"));
  await clear(cfg.page);
  await cfg.page.getByRole("button", { name: "Turn it back on", exact: true }).click();
  await cfg.page.locator(".picker").first().locator(".prow").click();
  await cfg.page.getByRole("menuitemradio", { name: "Camera only", exact: true }).click();
  let c = await calls(cfg.page);
  check("recording-off: Turn it back on and picking Camera only send their actions", acted(c, "popover", { do: "turn-back-on" }) && acted(c, "popover", { do: "set-source", arg: { kind: "camera" } }), JSON.stringify(c));
  await cfg.ctx.close();
  // The pickers by keyboard: the row opens on ↓, ↓ moves, Return picks — and the menu's
  // Return and Esc are the menu's, never the popover's (no key sent to the core).
  const pk = await open("popover", pop("config-camera-picked"));
  await clear(pk.page);
  const camRow = pk.page.locator(".picker").nth(2).locator(".prow");
  await camRow.focus();
  await pk.page.keyboard.press("ArrowDown");
  check("config-camera-picked: ↓ on the camera row opens its menu", (await pk.page.locator(".picker").nth(2).locator(".menu").count()) === 1);
  await pk.page.keyboard.press("Escape");
  check("config-camera-picked: Esc in a menu closes the menu only", (await pk.page.locator(".menu").count()) === 0 && !(await calls(pk.page)).some((x) => x.cmd === "key"), JSON.stringify(await calls(pk.page)));
  await pk.page.keyboard.press("ArrowDown");
  await pk.page.keyboard.press("ArrowDown");
  await pk.page.keyboard.press("Enter");
  c = await calls(pk.page);
  check("config-camera-picked: ↓ then Return picks Camera off, and Return isn't the popover's", acted(c, "popover", { do: "set-camera", arg: null }) && !c.some((x) => x.cmd === "key"), JSON.stringify(c));
  await clear(pk.page);
  await pk.page.locator(".picker").nth(1).locator(".prow").click();
  await pk.page.getByRole("menuitemradio", { name: "No mic", exact: true }).click();
  await pk.page.locator(".picker").nth(2).locator(".prow").click();
  await pk.page.getByRole("menuitemradio", { name: "FaceTime HD Camera", exact: true }).click();
  c = await calls(pk.page);
  check("config-camera-picked: No mic and a camera send their picks", acted(c, "popover", { do: "set-mic", arg: null }) && acted(c, "popover", { do: "set-camera", arg: "cam" }), JSON.stringify(c));
  await clear(pk.page);
  await pk.page.locator(".picker").nth(0).locator(".prow").click();
  await pk.page.locator(".hint").click();
  check("config-camera-picked: a click outside an open menu closes it, and sends nothing", (await pk.page.locator(".menu").count()) === 0 && !(await calls(pk.page)).some((x) => x.cmd === "act" || x.cmd === "key"));
  await pk.ctx.close();
  // Camera only with one camera: the camera picker keeps that camera, live, its menu that one pick.
  const one = await open("popover", pop("config-camera-only-one"));
  const oneRow = one.page.locator(".picker").nth(2).locator(".prow");
  check("config-camera-only-one: the camera picker stays live on its one camera", (await oneRow.getAttribute("aria-disabled")) !== "true" && !(await oneRow.evaluate((e) => e.classList.contains("greyed"))), await oneRow.getAttribute("aria-label"));
  await oneRow.click({ timeout: 2000 }).catch(() => {});
  const oneItems = await one.page.locator(".picker").nth(2).locator('[role="menuitemradio"]').allInnerTexts();
  // System default (N44) first, meaning that camera; then the camera by name; no Camera off.
  check("config-camera-only-one: its menu is System default and the one camera, no Camera off", oneItems.map((t) => t.replace(/\s+/g, " ").trim()).join("|") === "System default FaceTime HD Camera|FaceTime HD Camera", oneItems.join("|"));
  await one.ctx.close();
  for (const variant of ["discard-confirm", "restart-confirm"]) {
    const dc = await open("popover", pop(variant));
    await clear(dc.page);
    await dc.page.locator("main.pop h1").click();
    await dc.page.keyboard.press("Enter");
    await dc.page.keyboard.press("Escape");
    c = await calls(dc.page);
    check(`${variant}: Return is sent as a key (the core does nothing with it), Esc means Back`, c.some((x) => x.cmd === "key" && x.args.key === "return") && c.some((x) => x.cmd === "key" && x.args.key === "esc") && !c.some((x) => x.cmd === "act"), JSON.stringify(c));
    await dc.ctx.close();
  }
}

{
  const pop = (variant) => popovers.find((e) => e.variant === variant).scene;
  const { ctx, page } = await open("popover", pop("member-revoked"));
  await clear(page);
  await page.getByRole("button", { name: "Save three to Movies", exact: true }).click();
  await page.getByRole("button", { name: "Leave this space…", exact: true }).click();
  let c = await calls(page);
  check("member-revoked: Save three to Movies and Leave this space… send their actions", acted(c, "popover", { do: "save-stranded-to-movies" }) && acted(c, "popover", { do: "disconnect" }), JSON.stringify(c));
  await clear(page);
  await page.getByRole("button", { name: "Start recording", exact: true }).click({ force: true }).catch(() => {});
  check("member-revoked: the greyed Start sends nothing", !(await calls(page)).some((x) => x.cmd === "act"));
  await ctx.close();
  const o = await open("popover", pop("needs-update-owner-class"));
  await o.page.getByRole("button", { name: "Add a token here", exact: true }).click();
  check("needs an update · owner-class: Add a token here sends its action", acted(await calls(o.page), "popover", { do: "add-token" }));
  await o.ctx.close();
  const s = await open("popover", pop("stopped-stranded"));
  await clear(s.page);
  await s.page.locator("main.pop h1").click();
  await s.page.keyboard.press("Enter");
  c = await calls(s.page);
  check("stopped-stranded: Return is the Scene's ret (Save to Movies)", c.some((x) => x.cmd === "key" && x.args.key === "return") && pop("stopped-stranded").ret?.do === "save-to-movies");
  await s.ctx.close();
}

// ── Send feedback (N60): the form and the sent face, light and dark ──

for (const entry of feedbacks) {
  const w = entry.scene;
  const b = w.body;
  // The disclosure's text is shown only when it's opened; the words and the reply-to are
  // the person's own, in their fields.
  // The sent face shows only its own lines.
  const sceneForText = b.sent ? { ...w, body: { sent: b.sent } } : { ...w, body: { ...b, preview: null, words: null, reply_to: null, title: null } };
  for (const scheme of ["light", "dark"]) {
    const tag = `feedback/${entry.variant} [${scheme}]`;
    const { ctx, page, errors } = await open("feedback", w, { colorScheme: scheme });
    const v = await vars(page);
    await common(tag, page, errors, "feedback", sceneForText, { allow: ["field"] });
    // The system font, as every window has it: a browser's default would be a serif in a
    // Mac's webview (Linux's WebKit falls back to a sans, which hid it).
    const fonts = await page.evaluate(() => ["main.fb", "main.fb button", "main.fb p"].map((q) => document.querySelector(q)).filter(Boolean).map((e) => getComputedStyle(e).fontFamily));
    check(`${tag}: its words in the system font`, fonts.length >= 2 && fonts.every((f) => f.includes("-apple-system")), fonts.join(" | "));
    const want = b.sent ? [b.sent.done] : [...b.buttons, ...(b.mail ? [b.mail] : [])];
    const shown = await page.locator("button:visible").allInnerTexts();
    check(`${tag}: the Scene's buttons, no others`, shown.map(norm).sort().join("|") === want.map((x) => norm(x.label)).sort().join("|"), shown.join("|"));
    for (const x of want) {
      const loc = page.getByRole("button", { name: x.label, exact: true });
      const bad = (await loc.count()) === 1 ? await styled(loc, x.style, v) : "missing";
      check(`${tag}: “${x.label}” ${x.enabled ? "enabled" : "disabled"}, ${x.style}`, !bad && (await loc.isEnabled()) === x.enabled, bad ?? "");
    }
    if (!b.sent) {
      const t = page.locator("textarea.words");
      check(`${tag}: the words read the Scene`, (await t.inputValue()) === b.words.value && (await t.getAttribute("placeholder")) === b.words.placeholder && (await t.isEnabled()) === b.words.enabled);
      if (b.words.enabled) check(`${tag}: the words have the keyboard`, await t.evaluate((e) => e === document.activeElement));
      const box = page.locator(".log input[type=checkbox]");
      check(`${tag}: the log box reads the Scene`, (await box.isChecked()) === b.log.on && (await box.isEnabled()) === b.log.enabled);
      const xs = await page.locator(".acts button").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().right));
      check(`${tag}: Send is rightmost`, xs.length === 2 && xs[1] > xs[0], JSON.stringify(xs));
      const fits = await page.evaluate(() => document.documentElement.scrollHeight <= window.innerHeight + 1);
      check(`${tag}: the form fits the window`, fits);
      if (b.preview) {
        await page.locator(".what summary").click();
        check(`${tag}: “See exactly what's sent” shows what goes`, norm(await page.locator(".what pre").innerText()) === norm(b.preview));
        const open = await page.evaluate(() => document.documentElement.scrollHeight <= window.innerHeight + 1);
        check(`${tag}: open, it still fits the window`, open);
        await page.screenshot({ path: join(out, `feedback-${entry.variant}-open-${scheme}.png`), animations: "disabled" });
        await page.locator(".what summary").click();
      }
    }
    await page.screenshot({ path: join(out, `feedback-${entry.variant}-${scheme}.png`), animations: "disabled" });
    await ctx.close();
  }
}

{
  const fb = (variant) => feedbacks.find((e) => e.variant === variant).scene;
  const f = await open("feedback", fb("feedback-filled"));
  await clear(f.page);
  await f.page.locator("textarea.words").press("Enter");
  let c = await calls(f.page);
  check("feedback: Return in the words is a new line, nothing sent", !c.some((x) => x.cmd === "act" || x.cmd === "key"), JSON.stringify(c));
  await clear(f.page);
  await f.page.keyboard.press("Meta+Enter");
  c = await calls(f.page);
  check("feedback: ⌘Return is Send", acted(c, "feedback", { do: "send-feedback" }), JSON.stringify(c));
  await clear(f.page);
  await f.page.locator(".log input").click();
  c = await calls(f.page);
  check("feedback: the log box sends its action", acted(c, "feedback", fb("feedback-filled").body.log.action), JSON.stringify(c));
  await clear(f.page);
  await f.page.keyboard.press("Escape");
  c = await calls(f.page);
  check("feedback: Esc is the Scene's esc", c.some((x) => x.cmd === "key" && x.args.surface === "feedback" && x.args.key === "esc"), JSON.stringify(c));
  await f.ctx.close();
  const e = await open("feedback", fb("feedback-empty"));
  await clear(e.page);
  await e.page.keyboard.press("Meta+Enter");
  check("feedback: ⌘Return with Send greyed sends nothing", !(await calls(e.page)).some((x) => x.cmd === "act"));
  await e.ctx.close();
  const s = await open("feedback", fb("feedback-sent"));
  await clear(s.page);
  await s.page.keyboard.press("Enter");
  c = await calls(s.page);
  check("feedback: on the sent face Return is the Scene's ret (Done)", c.some((x) => x.cmd === "key" && x.args.key === "return") && fb("feedback-sent").ret?.do === "close-feedback", JSON.stringify(c));
  await s.ctx.close();
  const m = await open("feedback", fb("feedback-failed"));
  await clear(m.page);
  await m.page.getByRole("button", { name: "email it to hi@shroom.am", exact: true }).click();
  check("feedback: a failed send's mail link sends its action", acted(await calls(m.page), "feedback", { do: "mail-feedback" }));
  await m.ctx.close();
}

// ── the editor (N63): the core's faces, light and dark; then driven ──
{
  const media = mkdtempSync(join(tmpdir(), "shroom-editor-"));
  try {
    // The core's faces, over a test video as long as their recording (and its peaks).
    const length = Math.ceil(Math.max(...editors.map((e) => e.scene.body.source_duration)));
    for (const d of ["core", "drive"]) mkdirSync(join(media, d));
    const core = makeMedia(join(media, "core"), length);
    const coreServer = await serveMedia(core, editors[0].scene.body.rec);
    await editorLooks({ browser, origin: server.origin, mediaOrigin: coreServer.origin, entries: editors, check });
    coreServer.close();
    // The drive's faces (editor-fixture.mjs), held to the core's: the same shape, the same
    // words, face for face — so what the drive proves of them holds of the core's.
    const fixture = editorScenes(75);
    // Where two slices first differ in shape — keys, and each value's kind (an empty list
    // matches any) — or null.
    const unlike = (a, b, at = "") => {
      if (Array.isArray(a) && Array.isArray(b)) return a.length && b.length ? unlike(a[0], b[0], `${at}[0]`) : null;
      if (a && b && typeof a === "object" && typeof b === "object") {
        const [ka, kb] = [Object.keys(a).sort().join(), Object.keys(b).sort().join()];
        if (ka !== kb) return `${at || "."}: ${ka} vs ${kb}`;
        for (const k of Object.keys(a)) {
          const d = unlike(a[k], b[k], `${at}.${k}`);
          if (d) return d;
        }
        return null;
      }
      return (a === null) === (b === null) && typeof a === typeof b ? null : `${at}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`;
    };
    const canonical = (v) => JSON.stringify(v, (_, x) => (x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => a.localeCompare(b))) : x));
    const words = (w) => canonical([w.id, w.ret, w.esc, w.body.title, w.body.hint, w.body.footer, w.body.footer_tone, w.body.media.line, w.body.media.retry, w.body.revert.label, w.body.revert.action, w.body.done, w.body.labels]);
    for (const f of fixture) {
      const c = editors.find((e) => e.variant === f.variant);
      if (!c) continue;
      const d = unlike(f.scene, c.scene);
      check(`editor fixture/${f.variant}: the core's shape`, d === null, d ?? "");
      check(`editor fixture/${f.variant}: the core's words, buttons and keys`, words(f.scene) === words(c.scene), `${words(f.scene)} vs ${words(c.scene)}`);
    }
    check("editor fixture: it covers the core's faces", editors.every((c) => c.variant === "editor-ready" || fixture.some((f) => f.variant === c.variant)), editors.map((e) => e.variant).join(" "));
    const drive = makeMedia(join(media, "drive"), 75);
    const driveServer = await serveMedia(drive, REC);
    await editorDrives({ browser, origin: server.origin, mediaOrigin: driveServer.origin, entries: fixture, check });
    driveServer.close();
  } finally {
    rmSync(media, { recursive: true, force: true });
  }
}

await browser.close();
server.close();
console.log(`windows: library ${SIZES.library.width}×${SIZES.library.height} (min ${SIZES.library.min.width}×${SIZES.library.min.height}) · settings ${SIZES.settings.width}×${SIZES.settings.height}, tallest pane ${Math.max(...heights)}px`);

const failed = results.filter((r) => !r.ok);
for (const f of failed) console.log(`FAIL ${f.name}${f.detail ? ` — ${f.detail}` : ""}`);
console.log(`\n${library.length} library + ${settings.length} settings + ${feedbacks.length} feedback + ${editors.length} editor + ${popovers.length} popover + ${countdowns.length} countdown scenes × 2 schemes · ${results.length - failed.length}/${results.length} checks passed · screenshots in ${out}`);
process.exit(failed.length ? 1 : 0);
