// Typing in the fields the Scene owns — a title being renamed, the Library's search, the
// Members composer's “Who's this for?”, the link name — one key at a time, as a person
// types, in the built windows in WebKit, with the core stood in for: every field() call is
// answered the way the core answers it, a little later (gallery-kit.mjs's `echo`), or held
// and handed over when the check says.
//
//   cargo run -q -p shroom-core --example gallery [-- library | members]   (run for you)
//   npm run build --workspace app                   (run it first: this serves app/dist)
//   node app/test/typing-gallery.mjs
//
// For each field: typed key by key with the core answering between the keys, every letter
// stays and the caret stays behind the last one; typed faster than the core answers, the
// same; an answer to an older edit, or a Scene for something else, never writes over newer
// typing and never moves the caret — at the end of the text or in the middle of it; what
// the core refuses (a character outside the link name's set, a letter past the field's
// length) is still taken back out, the caret where it was; with nothing typed and
// unanswered the Scene's value is the field's; typing the core never answers gives way to
// the Scene's value. A core that stalls is not one that never answers: while it says
// nothing, typing stands however long; while its answers keep coming, however slowly, the
// same; and where the field did give way and the answers were only late, whatever is typed
// after that stands against every one of them. Then the title's own: the rename opens with
// the old title selected, once; Return after typing follows the last key. And the
// windows': a Scene for something else moves the keyboard nowhere; the search field's Esc
// empties it there and then.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { webkit } from "playwright";
import { dist, echo, gallery, serve, stub } from "./gallery-kit.mjs";

for (const f of ["library.html", "settings.html", "onboarding.html"]) {
  if (!existsSync(join(dist, f))) {
    console.error(`app/dist/${f} is missing — run \`npm run build --workspace app\` first.`);
    process.exit(2);
  }
}

const onboarding = gallery();
const members = gallery("members");
const library = gallery("library");
const scene = (list, variant, window) => {
  const e = list.find((x) => x.variant === variant && (!window || x.window === window));
  if (!e) throw new Error(`no gallery entry ${variant}`);
  return e.scene;
};

// The four fields: the window each lives in, the Scene it's typed in, and what the core
// holds it to. `was` is what the field shows when the person gets to it.
const FIELDS = [
  { name: "title", page: "library", scene: scene(library, "library-renaming"), sel: 'input[data-field="title"]', cap: 256 },
  { name: "library-search", page: "library", scene: scene(library, "library"), sel: 'input[data-field="library-search"]', cap: 256 },
  { name: "invite-name", page: "settings", scene: scene(members, "members", "settings"), sel: 'input[data-field="invite-name"]', cap: 80 },
  { name: "name", page: "onboarding", scene: scene(onboarding, "name"), sel: "input.name", cap: 63 },
];
const READY = { library: ".lib", settings: ".set", onboarding: "main.ob" };
const SIZE = { library: { width: 680, height: 560 }, settings: { width: 600, height: 620 }, onboarding: { width: 520, height: 600 } };

// What a person types: Bagrat's own word, and a longer one for typing ahead of the core
// (every character one the link name keeps too).
const WORD = "renamed";
const LONG = "renamed-again-and-again";
// The give-way (typed.ts): how long typing waits on a core that has spoken since the last
// key without counting it, before the Scene's value comes back.
const UNANSWERED = 2000;

/** Runs in the page: counts select() per field and keeps every write to a field's value
 * from script — [field, what it showed, what was written] — typing itself isn't one. */
const spy = () => {
  const which = (e) => e.dataset.field || (e.classList.contains("name") ? "name" : "");
  window.__spy = { select: {}, writes: [] };
  const select = HTMLInputElement.prototype.select;
  HTMLInputElement.prototype.select = function () {
    const k = which(this);
    window.__spy.select[k] = (window.__spy.select[k] ?? 0) + 1;
    return select.call(this);
  };
  const d = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
  Object.defineProperty(HTMLInputElement.prototype, "value", {
    configurable: true,
    get() {
      return d.get.call(this);
    },
    set(v) {
      window.__spy.writes.push([which(this), d.get.call(this), String(v)]);
      d.set.call(this, v);
    },
  });
};

const results = [];
const check = (name, ok, detail = "") => results.push({ name, ok: !!ok, detail });
const server = await serve();
const browser = await webkit.launch();

/** The window on `sc`, the core answering `delay` ms after each edit (null: held). */
async function open(page_, sc, delay) {
  const ctx = await browser.newContext({ viewport: SIZE[page_] });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript(stub, sc);
  await page.addInitScript(echo, { scene: sc, delay });
  await page.addInitScript(spy);
  await page.goto(`${server.origin}/${page_}.html`);
  await page.waitForSelector(READY[page_]);
  await page.waitForTimeout(80);
  return { ctx, page, errors };
}

const state = (input) =>
  input.evaluate((e) => ({ value: e.value, start: e.selectionStart, end: e.selectionEnd, focused: e === document.activeElement }));
const at = (s, value, caret) => s.value === value && s.start === caret && s.end === caret && s.focused;
const show = (s) => `“${s.value}” ${s.start}–${s.end}${s.focused ? "" : " (not focused)"}`;
const sent = async (page, name) => (await page.evaluate(() => window.__calls)).filter((c) => c.cmd === "field" && c.args.field === name).map((c) => c.args.value);
const settle = (page) => page.waitForTimeout(20);
const deliver = async (page, n) => {
  await page.evaluate((n) => window.__deliver(n ?? undefined), n ?? null);
  await settle(page);
};
const tick = async (page) => {
  await page.evaluate(() => window.__tick());
  await settle(page);
};
/** Forget the writes so far (the window drawing itself). */
const mark = (page) => page.evaluate(() => (window.__spy.writes = []));
/** The writes since `mark` that changed what `name` showed. */
const overwrites = (page, name) => page.evaluate((n) => window.__spy.writes.filter(([k, was, now]) => k === n && was !== now), name);
const prefixes = (w) => [...w].map((_, i) => w.slice(0, i + 1));
const left = async (page, n) => {
  for (let i = 0; i < n; i++) await page.keyboard.press("ArrowLeft");
};

/** The keyboard in the field, the way a person gets it there: the title and the two asks
 * have it when they appear; the search field takes a click. What it shows then. */
async function reach(f, page) {
  const input = page.locator(f.sel);
  if (!(await state(input)).focused) await input.click();
  const s = await state(input);
  await mark(page);
  return { input, was: s.value };
}

for (const f of FIELDS) {
  const tag = f.name;

  // As a person types: the core answers between the keys.
  {
    const { ctx, page, errors } = await open(f.page, f.scene, 8);
    const { input } = await reach(f, page);
    for (const ch of WORD) {
      await page.keyboard.type(ch);
      await page.waitForTimeout(40);
    }
    const s = await state(input);
    check(`${tag}: typed key by key, every letter stays`, s.value === WORD, show(s));
    check(`${tag}: typed key by key, the caret stays behind the last letter`, at(s, WORD, WORD.length), show(s));
    const went = await sent(page, f.name);
    check(`${tag}: every key goes to field() once, in order`, went.join("|") === prefixes(WORD).join("|"), went.join("|"));
    const over = await overwrites(page, f.name);
    check(`${tag}: typed key by key, no answer writes over the field`, over.length === 0, JSON.stringify(over));
    check(`${tag}: no errors in the page`, errors.length === 0, errors.join(" | "));
    await ctx.close();
  }

  // Ahead of the core: the first answers land while the later keys are still being typed.
  {
    const { ctx, page } = await open(f.page, f.scene, 150);
    const { input } = await reach(f, page);
    await page.keyboard.type(LONG, { delay: 20 });
    let s = await state(input);
    check(`${tag}: typed ahead of the core, every letter stays`, at(s, LONG, LONG.length), show(s));
    await page.waitForTimeout(450);
    s = await state(input);
    check(`${tag}: … and still once every answer is in`, at(s, LONG, LONG.length), show(s));
    const went = await sent(page, f.name);
    check(`${tag}: … and the core was sent each key's text once`, went.join("|") === prefixes(LONG).join("|"), went.slice(-3).join("|"));
    const over = await overwrites(page, f.name);
    check(`${tag}: typed ahead of the core, no answer writes over the field`, over.length === 0, JSON.stringify(over.slice(0, 4)));
    await ctx.close();
  }

  // Late answers, one at a time; then the same in the middle of the text.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input } = await reach(f, page);
    await page.keyboard.type("abc");
    let s = await state(input);
    check(`${tag}: three keys, none answered yet`, at(s, "abc", 3), show(s));
    await deliver(page, 1);
    s = await state(input);
    check(`${tag}: the answer to an older edit leaves newer typing alone`, at(s, "abc", 3), show(s));
    await tick(page);
    s = await state(input);
    check(`${tag}: a Scene for something else leaves newer typing alone`, at(s, "abc", 3), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the last answer agrees with it`, at(s, "abc", 3), show(s));
    await left(page, 2);
    await page.keyboard.type("x");
    s = await state(input);
    check(`${tag}: a key in the middle goes in at the caret`, at(s, "axbc", 2), show(s));
    await tick(page);
    s = await state(input);
    check(`${tag}: typed in the middle, a Scene for something else keeps the text and the caret`, at(s, "axbc", 2), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: typed in the middle, its answer keeps the caret`, at(s, "axbc", 2), show(s));
    await page.keyboard.type("yz");
    await deliver(page, 1);
    s = await state(input);
    check(`${tag}: typed in the middle, the answer to an older edit keeps the text and the caret`, at(s, "axyzbc", 4), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and so does the last one`, at(s, "axyzbc", 4), show(s));
    const over = await overwrites(page, f.name);
    check(`${tag}: late answers write nothing over the field`, over.length === 0, JSON.stringify(over.slice(0, 4)));
    await ctx.close();
  }

  // What the core refuses comes back out: here, a letter past the field's length.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input } = await reach(f, page);
    const full = "a".repeat(f.cap);
    await page.keyboard.insertText(full);
    await deliver(page);
    let s = await state(input);
    check(`${tag}: ${f.cap} characters fit`, at(s, full, f.cap), `${s.value.length} characters, caret ${s.start}–${s.end}`);
    await page.keyboard.type("b");
    s = await state(input);
    check(`${tag}: one more shows until the core answers`, s.value === `${full}b`, `${s.value.length} characters`);
    await deliver(page);
    s = await state(input);
    check(`${tag}: a letter past ${f.cap} is taken back out, the caret at the end`, at(s, full, f.cap), `${s.value.length} characters, caret ${s.start}–${s.end}`);
    await left(page, 3);
    await page.keyboard.type("b");
    await deliver(page);
    s = await state(input);
    const cutMid = `${"a".repeat(f.cap - 3)}baa`;
    check(`${tag}: typed in the middle of a full field, the caret stays behind the new letter`, at(s, cutMid, f.cap - 2), `${s.value.length} characters ending “${s.value.slice(-5)}”, caret ${s.start}–${s.end}`);
    await ctx.close();
  }

  // The Scene still owns the field: with nothing typed and unanswered, its value shows.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input } = await reach(f, page);
    await page.keyboard.type("ab");
    await deliver(page);
    await page.evaluate(([n, v]) => window.__say(n, v), [f.name, "elsewhere"]);
    await settle(page);
    let s = await state(input);
    check(`${tag}: a value the core changed by itself shows`, s.value === "elsewhere", show(s));
    await page.evaluate((n) => window.__forget(n), f.name);
    await settle(page);
    s = await state(input);
    check(`${tag}: its window's fields cleared, the field is empty`, s.value === "", show(s));
    await page.keyboard.type("q");
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the next key is typed and kept`, at(s, "q", 1), show(s));
    await ctx.close();
  }

  // An edit the core never took: nothing waits for its answer forever.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input, was } = await reach(f, page);
    await page.keyboard.type("zz");
    await page.evaluate(() => window.__lose());
    await tick(page);
    let s = await state(input);
    check(`${tag}: typing the core hasn't answered yet stands`, at(s, "zz", 2), show(s));
    await page.waitForTimeout(UNANSWERED + 300);
    s = await state(input);
    check(`${tag}: typing the core never answers gives way to the Scene's value`, s.value === was, show(s));
    await page.keyboard.type("q");
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the next key is typed and kept`, at(s, `${was}q`, was.length + 1), show(s));
    await ctx.close();
  }

  // … nor while the core is busy with other things: Scenes arriving all the while, none of
  // them counting the edit, are no reason to wait longer.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input, was } = await reach(f, page);
    await page.keyboard.type("zz");
    await page.evaluate(() => window.__lose());
    for (let i = 0; i < 6; i++) {
      await tick(page);
      await page.waitForTimeout(400);
    }
    const s = await state(input);
    check(`${tag}: an edit the core never took gives way though Scenes for other things keep arriving`, s.value === was, show(s));
    await ctx.close();
  }

  // A core that stalls — it says nothing for longer than the give-way, and the person types
  // on. There is nothing new to give way to: what's typed stands, and stands on while the
  // answers arrive late, one after another.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input } = await reach(f, page);
    await page.keyboard.type("abc");
    await page.waitForTimeout(UNANSWERED + 300);
    let s = await state(input);
    check(`${tag}: a core that says nothing takes nothing back — typing stands past the give-way`, at(s, "abc", 3), show(s));
    await page.keyboard.type("d");
    await deliver(page, 1);
    s = await state(input);
    check(`${tag}: … a key typed after the stall stands when the first late answer arrives`, at(s, "abcd", 4), show(s));
    await deliver(page, 2);
    s = await state(input);
    check(`${tag}: … and through the late answers after it`, at(s, "abcd", 4), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the last answer agrees with it`, at(s, "abcd", 4), show(s));
    const went = await sent(page, f.name);
    check(`${tag}: … and the core was sent what was typed, each key once`, went.join("|") === "a|ab|abc|abcd", went.join("|"));
    const over = await overwrites(page, f.name);
    check(`${tag}: a stalled core's late answers write nothing over the field`, over.length === 0, JSON.stringify(over.slice(0, 4)));
    await ctx.close();
  }

  // A core that is slow, not gone: its answers come further apart than the keys, each
  // within the give-way of the one before. Every answer that counts further starts the
  // wait again.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input } = await reach(f, page);
    await page.keyboard.type("abc");
    for (let i = 0; i < 2; i++) {
      await page.waitForTimeout(1300);
      await deliver(page, 1);
    }
    await page.waitForTimeout(1300);
    let s = await state(input);
    check(`${tag}: answers still coming, slower than the give-way all told — typing stands`, at(s, "abc", 3), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the last one agrees with it`, at(s, "abc", 3), show(s));
    const over = await overwrites(page, f.name);
    check(`${tag}: a slow core's answers write nothing over the field`, over.length === 0, JSON.stringify(over.slice(0, 4)));
    await ctx.close();
  }

  // A core that said something after the last key and then stalled: the field can't tell
  // that from an edit never taken, and gives way. Its answers were only late — and whatever
  // is typed after the give-way is newer than every one of them, so none writes over it.
  {
    const { ctx, page } = await open(f.page, f.scene, null);
    const { input, was } = await reach(f, page);
    await page.keyboard.type("abc");
    await tick(page);
    await page.waitForTimeout(UNANSWERED + 300);
    let s = await state(input);
    check(`${tag}: a core that spoke and then stalled — the field gave way to the Scene's value`, s.value === was, show(s));
    await mark(page);
    const now = `${was}d`;
    await page.keyboard.type("d");
    s = await state(input);
    check(`${tag}: … and the next key goes in`, at(s, now, now.length), show(s));
    await deliver(page, 1);
    s = await state(input);
    check(`${tag}: typed after a give-way, the late answer to an older edit leaves it alone`, at(s, now, now.length), show(s));
    await deliver(page, 2);
    s = await state(input);
    check(`${tag}: … and so do the late answers after it`, at(s, now, now.length), show(s));
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and its own answer agrees with it`, at(s, now, now.length), show(s));
    const went = await sent(page, f.name);
    check(`${tag}: … and the core was sent it once, after the three before`, went.join("|") === `a|ab|abc|${now}`, went.join("|"));
    const over = await overwrites(page, f.name);
    check(`${tag}: typed after a give-way, no late answer writes over the field`, over.length === 0, JSON.stringify(over.slice(0, 4)));
    await page.keyboard.type("e");
    await deliver(page);
    s = await state(input);
    check(`${tag}: … and the field is in step again: the next key is answered and kept`, at(s, `${now}e`, now.length + 1), show(s));
    await ctx.close();
  }
}

// The link name's own refusals: a character outside its set, uppercase, in the middle, and
// one that's older than newer typing.
{
  const f = FIELDS.find((x) => x.name === "name");
  const { ctx, page, errors } = await open(f.page, f.scene, null);
  const { input } = await reach(f, page);
  await page.keyboard.type("ab");
  await deliver(page);
  await page.keyboard.type("!");
  let s = await state(input);
  check("name: a character outside the set shows until the core answers", s.value === "ab!", show(s));
  await deliver(page);
  s = await state(input);
  check("name: a character the core refused is taken back out", at(s, "ab", 2), show(s));
  await page.keyboard.type("C");
  await deliver(page);
  s = await state(input);
  check("name: uppercase comes back lowercase, the caret behind it", at(s, "abc", 3), show(s));
  await left(page, 2);
  await page.keyboard.type("_");
  await deliver(page);
  s = await state(input);
  check("name: refused in the middle, the caret stays where it was", at(s, "abc", 1), show(s));
  await page.keyboard.type("x!y");
  s = await state(input);
  check("name: three more keys in the middle, none answered yet", at(s, "ax!ybc", 4), show(s));
  await deliver(page, 2);
  s = await state(input);
  check("name: a refusal older than newer typing waits for it", at(s, "ax!ybc", 4), show(s));
  await deliver(page);
  s = await state(input);
  check("name: … and is taken out with the last answer, the caret behind what was typed", at(s, "axybc", 3), show(s));
  // The plan moves on with a key still unanswered: the field goes, and nothing of it lingers.
  await page.keyboard.type("z");
  await page.evaluate((s) => window.__emit(s), scene(onboarding, "wiring"));
  await page.waitForTimeout(UNANSWERED + 300);
  check("name: the step moves on mid-typing, the field is gone (no errors in the page)", (await input.count()) === 0 && errors.length === 0, errors.join(" | "));
  await ctx.close();
}
{
  // After an edit the core never took, what the core refuses still comes back out — once
  // typing has rested for the give-way — and the caret stays behind what was kept.
  const f = FIELDS.find((x) => x.name === "name");
  const { ctx, page } = await open(f.page, f.scene, null);
  const { input } = await reach(f, page);
  await page.keyboard.type("ab");
  await deliver(page);
  await page.keyboard.type("c");
  await page.evaluate(() => window.__lose());
  await tick(page);
  await page.waitForTimeout(UNANSWERED + 300);
  let s = await state(input);
  check("name: a key the core never took gives way to what it holds", at(s, "ab", 2), show(s));
  await page.keyboard.type("C!");
  await deliver(page);
  await page.waitForTimeout(UNANSWERED + 300);
  s = await state(input);
  check("name: … and after it, what the core refuses is still taken back out", at(s, "abc", 3), show(s));
  await ctx.close();
}

// The title's own: the rename opens with the old title selected, ready to retype — once.
{
  const f = FIELDS.find((x) => x.name === "title");
  const { ctx, page, errors } = await open(f.page, f.scene, null);
  const input = page.locator(f.sel);
  const old = f.scene.body.rows.find((r) => r.rename).rename.value;
  const selects = () => page.evaluate(() => window.__spy.select.title ?? 0);
  let s = await state(input);
  check("title: the rename opens with the old title selected, the keyboard in it", s.focused && s.value === old && s.start === 0 && s.end === old.length, show(s));
  check("title: selected once as it opens", (await selects()) === 1, String(await selects()));
  for (let i = 0; i < 3; i++) await tick(page);
  s = await state(input);
  check("title: a Scene for something else leaves the selection as it is", s.focused && s.start === 0 && s.end === old.length, show(s));
  check("title: … and selects nothing again", (await selects()) === 1, String(await selects()));
  await page.keyboard.press("ArrowRight");
  await tick(page);
  s = await state(input);
  check("title: once the caret has moved, a Scene for something else leaves it there", at(s, old, old.length), show(s));
  await left(page, 4);
  await tick(page);
  s = await state(input);
  check("title: … in the middle too", at(s, old, old.length - 4), show(s));
  await page.keyboard.type("!");
  await deliver(page);
  const edited = `${old.slice(0, -4)}!${old.slice(-4)}`;
  s = await state(input);
  check("title: a key in the middle of the old title is kept, the caret behind it", at(s, edited, old.length - 3), show(s));
  await page.keyboard.press("Enter");
  const c = (await page.evaluate(() => window.__calls)).slice(-2);
  check(
    "title: Return follows the last key (field, then the Scene's ret)",
    c.length === 2 && c[0].cmd === "field" && c[0].args.value === edited && c[1].cmd === "key" && c[1].args.key === "return" && c[1].args.surface === "library",
    JSON.stringify(c),
  );
  check("title: selected once in all of it", (await selects()) === 1, String(await selects()));
  // The save landed: the Scene has no title being renamed, and the field gives way to it.
  await page.evaluate((s) => window.__emit(s), scene(library, "library"));
  await settle(page);
  check("title: once the Scene has no rename, the field is gone and the title is back", (await input.count()) === 0 && (await page.locator('li.lrow[data-key="r502"] .title').innerText()) === old);
  await page.waitForTimeout(UNANSWERED + 300);
  check("title: … and nothing of the field lingers (no errors in the page)", errors.length === 0, errors.join(" | "));
  await ctx.close();
}
{
  // Bagrat's own steps: rename, type the new title over the selected old one.
  const f = FIELDS.find((x) => x.name === "title");
  const { ctx, page } = await open(f.page, f.scene, 8);
  const input = page.locator(f.sel);
  for (const ch of WORD) {
    await page.keyboard.type(ch);
    await page.waitForTimeout(40);
  }
  const s = await state(input);
  check("title: typed over the selected old title, the new one is all there (“renamed”, never “d”)", at(s, WORD, WORD.length), show(s));
  check("title: … selected once, when the rename opened", (await page.evaluate(() => window.__spy.select.title ?? 0)) === 1);
  await ctx.close();
}

// A Scene for something else moves the keyboard nowhere: wherever the person put it stays.
for (const [name, page_, sc] of [
  ["library, a title being renamed", "library", scene(library, "library-renaming")],
  ["settings · members", "settings", scene(members, "members", "settings")],
  ["onboarding · name", "onboarding", scene(onboarding, "name-typed")],
]) {
  const { ctx, page } = await open(page_, sc, null);
  const held = await page.evaluate(() => {
    // The first button that takes the keyboard (a row's hover-only ones don't until shown).
    for (const b of document.querySelectorAll("button:not([disabled])")) {
      b.focus();
      if (document.activeElement !== b) continue;
      b.dataset.held = "1";
      return true;
    }
    return false;
  });
  for (let i = 0; i < 2; i++) await tick(page);
  const still = await page.evaluate(() => document.activeElement?.dataset?.held === "1");
  check(`${name}: a Scene for something else leaves the keyboard on the button that has it`, held && still, `held ${held}, still ${still}`);
  await ctx.close();
}
{
  // The name invites carry (B158): Change opens it as a field that takes the keyboard — once.
  const shown = scene(members, "members", "settings");
  const changing = scene(members, "members-from-changing", "settings");
  const { ctx, page } = await open("settings", shown, null);
  await page.getByRole("button", { name: shown.body.pane.from.change.label, exact: true }).click();
  check("members: Change sends its action", (await page.evaluate(() => window.__calls)).some((x) => x.cmd === "act" && x.args.action.do === "change-from"));
  await page.evaluate((s) => window.__emit(s), changing);
  await page.waitForSelector('input[data-field="from-name"]');
  check("members: the name being changed has the keyboard", await page.locator('input[data-field="from-name"]').evaluate((e) => e === document.activeElement));
  await page.keyboard.press("Enter");
  check("members: Return in it is Save", (await page.evaluate(() => window.__calls)).some((x) => x.cmd === "act" && x.args.action.do === "save-from"));
  await ctx.close();
}

// The search field's Esc empties it there and then, as a macOS search field does.
{
  const f = FIELDS.find((x) => x.name === "library-search");
  const { ctx, page } = await open(f.page, f.scene, null);
  const { input } = await reach(f, page);
  await page.keyboard.type("ab");
  await page.keyboard.press("Escape");
  let s = await state(input);
  check("library-search: Esc empties the field at once", at(s, "", 0), show(s));
  const c = await page.evaluate(() => window.__calls);
  check("library-search: Esc is the field's (field, not key)", (await sent(page, f.name)).join("|") === "a|ab|" && !c.some((x) => x.cmd === "key"), JSON.stringify(c));
  await deliver(page, 1);
  s = await state(input);
  check("library-search: an older answer doesn't bring the text back", at(s, "", 0), show(s));
  await deliver(page);
  s = await state(input);
  check("library-search: … and it stays empty", at(s, "", 0), show(s));
  await page.keyboard.type("c");
  await deliver(page);
  s = await state(input);
  check("library-search: … and the next key is typed and kept", at(s, "c", 1), show(s));
  await ctx.close();
}

await browser.close();
server.close();

const failed = results.filter((r) => !r.ok);
for (const f of failed) console.log(`FAIL ${f.name}${f.detail ? ` — ${f.detail}` : ""}`);
console.log(`\n${FIELDS.length} typed fields · ${results.length - failed.length}/${results.length} checks passed`);
process.exit(failed.length ? 1 : 0);
