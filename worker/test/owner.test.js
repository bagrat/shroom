// Owner-ness (U2, N26, N40; DECISIONS B25): a one-time key becomes a long-lived cookie
// and a clean URL; the cookie carries metadata powers only — the title, and for
// owner-class browsers the space-global mark switch. Grants and cookies are B25 as amended
// by WORKER-INTERFACE §3.5: the grant names the issuing device, the cookie is {m, d, t},
// and every use is checked against the roster; a member's edit rights come from the
// ownership ledger, never from meta.owner (§2.4).
import { createHash } from "node:crypto";
import { beforeEach, describe, expect, it } from "vitest";
import { COPY, forgetSecret, handle, status, untitled } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";
import { MHER, MHER_MAC, OWNER_MAC, ROSTER, ledgerKey, roster } from "./space-fixture.js";

const ID = "AbCdEfGhIjKlMnOp";
const HOST = "https://v.example.workers.dev";
const NOW = Date.parse("2026-10-02T12:00:00Z");
const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk"; // 43 chars, as 32 random bytes base64url
const sha = (s) => createHash("sha256").update(s).digest("hex");
const SECRET = Buffer.alloc(32, 7).toString("base64url");
const metaOf = (over = {}) => JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-02T11:00:00Z", title: "Untitled — Oct 2",
  media: { hls: `rec/${ID}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, owner: "m1", ...over });
const grant = (over = {}) => JSON.stringify({ member: "m1", role: "owner", device: OWNER_MAC.id, exp: new Date(NOW + 600_000).toISOString(), ...over });
const memberGrant = () => grant({ member: MHER, role: "member", device: MHER_MAC.id });

const req = (env, path, init = {}) => handle(new Request(`${HOST}${path}`, init), env, NOW);
const space = (extra = {}) => ({ BUCKET: fakeBucket({ ".shroom/cookie.key": SECRET, [ROSTER]: JSON.stringify(roster()), [`meta/${ID}.json`]: metaOf(), ...extra }) });
async function cookieFor(env, g = grant()) {
  const k = `${K.slice(0, 42)}Q`;
  env.BUCKET.store.set(`.shroom/keys/${sha(k)}`, new TextEncoder().encode(g));
  const res = await req(env, `/${ID}?k=${k}`);
  const set = res.headers.get("set-cookie");
  return set ? set.split(";")[0] : null;
}
const post = (env, path, body, cookie, origin = HOST) =>
  req(env, path, { method: "POST", headers: { "content-type": "application/json", origin, ...(cookie ? { cookie } : {}) }, body: JSON.stringify(body) });

beforeEach(() => forgetSecret());

describe("the one-time key", () => {
  it("becomes an HttpOnly, long-lived cookie and a redirect to the clean URL — once", async () => {
    const env = space({ [`.shroom/keys/${sha(K)}`]: grant() });
    const res = await req(env, `/${ID}?k=${K}`);
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(`/${ID}`);
    const set = res.headers.get("set-cookie");
    expect(set).toMatch(/^shroom_owner=[^;]+; Max-Age=34560000; Path=\/; HttpOnly; Secure; SameSite=Lax$/);
    expect(env.BUCKET.store.has(`.shroom/keys/${sha(K)}`)).toBe(false);
    const again = await req(env, `/${ID}?k=${K}`);
    expect(again.status).toBe(302);
    expect(again.headers.get("set-cookie")).toBeNull();
  });

  it("lands an expired, unknown or malformed key on the clean page with no cookie", async () => {
    const env = space({ [`.shroom/keys/${sha(K)}`]: grant({ exp: new Date(NOW - 1).toISOString() }) });
    for (const k of [K, "nope", "", "x".repeat(200)]) {
      const res = await req(env, `/${ID}?k=${encodeURIComponent(k)}`);
      expect(res.status).toBe(302);
      expect(res.headers.get("location")).toBe(`/${ID}`);
      expect(res.headers.get("set-cookie")).toBeNull();
    }
  });

  it("issues nothing when the space has no cookie key yet", async () => {
    const env = { BUCKET: fakeBucket({ [`meta/${ID}.json`]: metaOf(), [`.shroom/keys/${sha(K)}`]: grant() }) };
    expect((await req(env, `/${ID}?k=${K}`)).headers.get("set-cookie")).toBeNull();
  });
});

describe("the owner page", () => {
  it("gives the title field, Copy link and the switch to an owner-class browser", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const body = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(body).toContain(`placeholder="Untitled — Oct 2"`);
    expect(body).toContain(` autofocus`);
    expect(body).toContain(`>${COPY.copyLink}</button>`);
    // Every file up, the upload line has gone — no “Fully live ✓” on the restyled page.
    expect(body).toContain(`<div class="upl" id="ul" hidden>`);
    expect(body).not.toContain(COPY.fullyLive);
    expect(body).toContain(`id="mk" checked>${COPY.mark}`);
  });

  it("shows the upload line while bytes arrive, then waiting-for-connection when they stop", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ state: "uploading", media: null, progress: { done: 2, total: 5, updatedAt: "2026-10-02T11:59:50Z" } }) });
    const cookie = await cookieFor(env);
    const body = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(body).toContain(COPY.finishing);
    expect(body).not.toContain(`http-equiv="refresh"`); // never reloads under the owner's typing
    env.BUCKET.touch(`meta/${ID}.json`, metaOf({ state: "uploading", media: null, progress: { done: 2, total: 5, updatedAt: "2026-10-02T11:00:00Z" } }));
    expect(await (await req(env, `/${ID}`, { headers: { cookie } })).text()).toContain(COPY.waitingConnection);
  });

  it("gives a member the title on their own videos only, and never the switch", async () => {
    const env = space();
    const cookie = await cookieFor(env, memberGrant());
    const others = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(others).not.toContain(`class="title"`);
    env.BUCKET.touch(ledgerKey(ID), JSON.stringify({ v: 1, member: MHER, device: MHER_MAC.id, at: "2026-10-02T11:00:00Z" }), { m: MHER });
    const own = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(own).toContain(`class="title"`);
    expect(own).not.toContain(`id="mk"`);
  });

  // N69: Edit in shroom, the owner page's last quiet icon — a plain link to shroom's editor.
  it("gives the owner page Edit in shroom once every file is up, and no one else (N69)", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const body = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    const link = `<a class="copy edit" id="ed" href="shroom://edit/v.example.workers.dev/${ID}" data-tip="${COPY.editInShroom}" aria-label="${COPY.editInShroom}">${COPY.editInShroom}</a>`;
    expect(body).toContain(link);
    expect(body.indexOf(link)).toBeGreaterThan(body.indexOf(`id="dl"`)); // after Download, the last
    // Bytes still arriving: there, greyed and going nowhere, and the poll wakes it at the
    // drain (Bagrat, 2026-10-25: every icon there from the start).
    env.BUCKET.touch(`meta/${ID}.json`, metaOf({ progress: { done: 2, total: 5, updatedAt: "2026-10-02T11:59:50Z" } }));
    const up = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(up).toContain(`<a class="copy edit" id="ed" data-href="shroom://edit/v.example.workers.dev/${ID}" data-tip="${COPY.editInShroom}" aria-label="${COPY.editInShroom}" aria-disabled="true">${COPY.editInShroom}</a>`);
    expect(up).not.toMatch(/id="ed"[^>]* href=/);
    expect(up).toContain(`if(ed&&s.drained)wake(ed,ed.getAttribute("data-href"))`);
    // A recipient's page and the embed never have it.
    expect(await (await req(env, `/${ID}`)).text()).not.toContain("shroom://edit/");
    expect(await (await req(env, `/${ID}?embed=1`)).text()).not.toContain("shroom://edit/");
    // A member: on their own video's page, not on someone else's.
    const m = await cookieFor(env, memberGrant());
    expect(await (await req(env, `/${ID}`, { headers: { cookie: m } })).text()).not.toContain("shroom://edit/");
    env.BUCKET.touch(ledgerKey(ID), JSON.stringify({ v: 1, member: MHER, device: MHER_MAC.id, at: "2026-10-02T11:00:00Z" }), { m: MHER });
    expect(await (await req(env, `/${ID}`, { headers: { cookie: m } })).text()).toContain("shroom://edit/");
  });

  // Bagrat, 2026-10-25: the icons came in one by one and the row flickered. Every quiet icon
  // beside the title is there from the first paint, in its place, greyed until it can work.
  describe("the title's quiet icons, all there from the first paint", () => {
    const head = (body) => body.match(/<div class="head">([\s\S]*?)<\/div>/)?.[1] ?? "";
    const order = (h) => ['id="c"', 'id="th"', 'id="dl"', 'id="ed"'].map((k) => h.indexOf(k));
    const UP = { progress: { done: 2, total: 5, updatedAt: "2026-10-02T11:59:50Z" } };

    it("draws Copy link, Copy thumbnail, Download and Edit in shroom, in that order, the ones not ready greyed", async () => {
      for (const [face, over] of [["uploading", { state: "uploading", media: null, ...UP }], ["live, its last files still on the way", UP]]) {
        const env = space({ [`meta/${ID}.json`]: metaOf(over) });
        const h = head(await (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text());
        const at = order(h);
        expect(at.every((i, n) => i > 0 && (n === 0 || i > at[n - 1])), `${face}: ${h}`).toBe(true);
        expect(h, face).toContain(`<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button>`);
        expect(h, face).toContain(`<button class="copy thumb" id="th" data-tip="${COPY.preparingThumbnail}" aria-disabled="true">${COPY.copyThumbnail}</button>`);
        expect(h, face).toContain(`<span id="dl" data-tip="${COPY.download}"><a aria-disabled="true">${COPY.download}</a></span>`);
        expect(h, face).toMatch(/<a class="copy edit" id="ed" data-href="shroom:\/\/edit\/[^"]*"[^>]* aria-disabled="true">/);
        expect(h, face).not.toMatch(/ hidden[ >]/);
        expect(h, face).not.toContain(`href="/d/`);
      }
    });

    it("draws them before the record exists too, on the maker's own page", async () => {
      const env = space();
      env.BUCKET.store.delete(`meta/${ID}.json`);
      const cookie = await cookieFor(env);
      env.BUCKET.touch(ledgerKey(ID), JSON.stringify({ v: 1, member: "m1", device: OWNER_MAC.id, at: "2026-10-02T11:59:00Z" }), { m: "m1" });
      const body = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
      expect(body).toContain(`class="title"`);
      const at = order(head(body));
      expect(at.every((i) => i > 0)).toBe(true);
      expect(head(body)).not.toMatch(/ hidden[ >]/);
    });

    it("draws the ready ones awake: a fully-up video with its file", async () => {
      const env = space({ [`meta/${ID}.json`]: metaOf({ media: { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, width: 1920, height: 1080 } }) });
      const h = head(await (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text());
      expect(h).toContain(`<span id="dl" data-tip="${COPY.download}"><a href="/d/${ID}">${COPY.download}</a></span>`);
      expect(h).toContain(`<a class="copy edit" id="ed" href="shroom://edit/v.example.workers.dev/${ID}" data-tip="${COPY.editInShroom}" aria-label="${COPY.editInShroom}">`);
      expect(h).not.toContain(`aria-disabled="true">${COPY.download}`);
    });

    it("wakes each where it stands, never adding or showing one, and greys Download again if its file goes", async () => {
      const env = space({ [`meta/${ID}.json`]: metaOf({ state: "uploading", media: null, ...UP }) });
      const body = await (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text();
      expect(body).toContain(`function wake(el,href){if(!el.getAttribute("aria-disabled"))return;if(href)el.href=href;el.removeAttribute("aria-disabled")}`);
      expect(body).toContain(`function rest(el){if(el.getAttribute("aria-disabled"))return;el.removeAttribute("href");el.setAttribute("aria-disabled","true")}`);
      expect(body).toContain(`if(da){if(s.mp4)wake(da,"/d/"+c.id);else rest(da)}`);
      expect(body).not.toContain(`tb.hidden=false`);
      expect(body).not.toContain(`ed.hidden=false`);
      expect(body).not.toContain(`dl.appendChild`);
      // Greyed is drawn the same for each: dimmed, no hover, nothing under the pointer.
      expect(body).toContain(`#dl a[aria-disabled=true]{cursor:default}#dl a[aria-disabled=true]::before{opacity:.4}`);
      // The column keeps the player's width while the frame waits for it: nothing moves at the
      // go-live; and keeps the upload line's room once the line has gone: nothing moves at the drain.
      expect(body).toContain(`body:has(main .player,main .wait){--bar:56px}`);
      expect(body).toContain(`body:has(main .upl:not([hidden]),main .upl[data-room]){--room:calc(100dvh - 282px)}`);
      expect(body).toMatch(/<div class="upl" id="ul" data-room>/);
      // A change to what plays (a press on the strip, cuts landing) greys Download until the
      // poll, started again if it had stopped, finds its file (review of B247).
      expect(body).toContain(`function edited(){if(c.gone)return;var da=dl.firstChild;if(da)rest(da);if(!c.polling){gap=2000;next(0)}}`);
      expect(body).toContain(`document.addEventListener("shroom:keep",edited)`);
      expect(body).toContain(`function next(ms){c.polling=true;setTimeout(poll,ms)}`);
    });

    it("leaves the viewer's page as it is: what a viewer can't use isn't drawn (B166)", async () => {
      const env = space();
      const h = head(await (await req(env, `/${ID}`)).text());
      expect(h).toContain(`<h1>`);
      expect(h).not.toContain(`aria-disabled`);
      expect(h).not.toContain(COPY.download);
    });
  });

  it("is the recipient page for a forged or absent cookie", async () => {
    const env = space();
    const real = await cookieFor(env);
    const [name, value] = real.split("=");
    const [payload] = value.split(".");
    const forged = `${name}=${Buffer.from(JSON.stringify({ m: "m1", o: true, t: 0 })).toString("base64url")}.${payload.slice(0, 20)}`;
    for (const cookie of [forged, `${name}=${payload}`, "", "other=1"]) {
      const body = await (await req(env, `/${ID}`, { headers: cookie ? { cookie } : {} })).text();
      expect(body).not.toContain(`class="title"`);
      expect(body).not.toContain(`id="mk"`);
    }
  });

  it("answers a removed video's page and embed as a plain not found, 404 (B141)", async () => {
    const env = space({ [`meta/${ID}.json`]: JSON.stringify({ schema: 1, id: ID, state: "deleted", title: "Secret plans" }) });
    for (const [path, headers] of [[`/${ID}`, {}], [`/${ID}`, { cookie: await cookieFor(env) }], [`/${ID}?embed=1`, {}]]) {
      const res = await req(env, path, { headers });
      expect(res.status).toBe(404);
      const body = await res.text();
      expect(body).toContain(COPY.notFound);
      expect(body).not.toContain("removed");
      expect(body).not.toContain("Secret plans");
    }
    // An id with no record still reads as on its way (P9), never a 404.
    const none = await req(space(), "/ZzZzZzZzZzZzZzZz");
    expect(none.status).toBe(200);
    expect(await none.text()).toContain(COPY.onItsWay);
  });

  it("keeps the removed face for everyone", async () => {
    const env = space({ [`meta/${ID}.json`]: JSON.stringify({ schema: 1, id: ID, state: "deleted" }) });
    const cookie = await cookieFor(env);
    const body = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(body).toContain(COPY.notFound);
    expect(body).not.toContain(`class="title"`);
  });
});

describe("owner writes", () => {
  it("renames under the cookie, and the recipient page shows it", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const res = await post(env, `/_shroom/title/${ID}`, { title: "  Q3   plan " }, cookie);
    expect(await res.json()).toEqual({ title: "Q3 plan" });
    const meta = JSON.parse(new TextDecoder().decode(env.BUCKET.store.get(`meta/${ID}.json`)));
    expect(meta).toMatchObject({ title: "Q3 plan", titled: true, owner: "m1", media: { hls: `rec/${ID}/master.m3u8` } });
    expect(await (await req(env, `/${ID}`)).text()).toContain("<h1>Q3 plan</h1>");
    const owner = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(owner).toContain(`value="Q3 plan" placeholder="Untitled — Oct 2"`);
    expect(owner).not.toContain(" autofocus");
  });

  it("never saves an empty title — the page restores the previous one", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await post(env, `/_shroom/title/${ID}`, { title: "   " }, cookie)).status).toBe(400);
  });

  it("retries from a fresh read when the recording Mac rewrote the record meanwhile — never clobbers it", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ state: "uploading", media: null }) });
    const cookie = await cookieFor(env);
    const realGet = env.BUCKET.get.bind(env.BUCKET);
    let raced = false;
    env.BUCKET.get = async (key, o) => {
      const r = await realGet(key, o);
      if (key === `meta/${ID}.json` && !raced) {
        raced = true; // the Mac goes live between the Worker's read and its write
        env.BUCKET.touch(key, metaOf({ state: "live" }));
      }
      return r;
    };
    expect((await post(env, `/_shroom/title/${ID}`, { title: "Demo" }, cookie)).status).toBe(200);
    const meta = JSON.parse(new TextDecoder().decode(env.BUCKET.store.get(`meta/${ID}.json`)));
    expect(meta).toMatchObject({ state: "live", title: "Demo" });
  });

  it("refuses a tombstone, a stranger, another site, and a member's rename of someone else's video", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await post(env, `/_shroom/title/${ID}`, { title: "x" }, null)).status).toBe(403);
    expect((await post(env, `/_shroom/title/${ID}`, { title: "x" }, cookie, "https://evil.example")).status).toBe(403);
    const member = await cookieFor(env, memberGrant());
    expect((await post(env, `/_shroom/title/${ID}`, { title: "x" }, member)).status).toBe(403);
    env.BUCKET.touch(`meta/${ID}.json`, JSON.stringify({ schema: 1, id: ID, state: "deleted" }));
    expect((await post(env, `/_shroom/title/${ID}`, { title: "x" }, cookie)).status).toBe(409);
  });

  it("switches the mark for the whole space — owner-class only", async () => {
    const env = space();
    const member = await cookieFor(env, memberGrant());
    expect((await post(env, "/_shroom/mark", { on: false }, member)).status).toBe(403);
    const cookie = await cookieFor(env);
    expect(await (await post(env, "/_shroom/mark", { on: false }, cookie)).json()).toEqual({ on: false });
    const recipient = await (await req(env, `/${ID}`)).text();
    expect(recipient).not.toContain(`title="${COPY.mark}"`);
    expect(await (await req(env, `/${ID}`, { headers: { cookie } })).text()).toContain(`<input type="checkbox" id="mk">${COPY.mark}`);
  });
});

// The mark, switched off (U3, N26; DEVIATIONS #41 — Bagrat, on his own page: “unchecking
// made with shroom does not hide it even after refresh”). The write always landed and
// every recipient's page lost the mark; the owner's own page kept the logo whatever the
// switch said, so from the only seat that can flip it the switch looked dead. And then, on
// 0.1.4 (2026-10-13): “logo goes, but the checkbox and text remain, it is kind of
// pointless, all should go and they can turn back on from settings” — switched off, the
// owner's corner is empty too; Settings turns it back on.

/** The page's foot as a browser lays it out: where the switch stands, whether the mark's
 * logo shows (a `hidden` logo takes no room), and whether the foot itself shows (a
 * `hidden` foot takes the switch, its words and the logo with it). `null` when the page has
 * no foot. */
function footOf(body) {
  const foot = body.match(/<div class="foot"[^>]*>[\s\S]*?<\/div>/)?.[0] ?? null;
  if (!foot) return null;
  const box = foot.match(/<input type="checkbox"[^>]*>/)?.[0] ?? null;
  const logo = foot.match(/<a class="mark"[^>]*>/)?.[0] ?? null;
  const open = foot.match(/<div class="foot"[^>]*>/)[0];
  return { switch: box ? (/\schecked/.test(box) ? "on" : "off") : null, logo: logo ? (/\shidden/.test(logo) ? "hidden" : "shown") : null, foot: /\shidden/.test(open) ? "hidden" : "shown" };
}

/** The owner page's mark script, run against a stand-in for the page it was rendered into:
 * the switch, the logo, a `fetch` the test answers by hand, and the rest of the page's
 * arrival played out in a browser's order (DOMContentLoaded, then load, then pageshow).
 * On a Back a browser puts the control's last state there back over what the page
 * rendered: `restored` is a state put back before the page's scripts ran, `back` one put
 * back after them — where headless Chromium does it, between DOMContentLoaded and load.
 * `loading` stops before the arrival; `arrive()` plays it later. */
function markScript(body, { restored, back, loading = false } = {}) {
  const script = [...body.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]).find((s) => s.includes("/_shroom/mark"));
  expect(script, "the mark's script is on the page").toBeTruthy();
  const rendered = footOf(body);
  const logoTag = body.match(/<div class="foot">[\s\S]*?(<a class="mark"[^>]*>)/)?.[1] ?? "";
  const logoId = logoTag.match(/\sid="([^"]+)"/)?.[1] ?? null;
  const listeners = [];
  const box = { defaultChecked: rendered.switch === "on", checked: restored ?? rendered.switch === "on", onchange: null, addEventListener: (type, fn) => { if (type === "change") listeners.push(fn); } };
  const logo = { hidden: rendered.logo === "hidden" };
  const foot = { hidden: rendered.foot === "hidden" };
  /** What the page asked to hear about its own arrival, by `addEventListener` or by `on…`. */
  const hears = () => { const heard = []; return { heard, addEventListener: (type, fn) => { heard.push({ type, fn }); } }; };
  const say = (who, type) => { const e = { type, target: who, persisted: false }; if (typeof who[`on${type}`] === "function") who[`on${type}`](e); for (const l of who.heard) if (l.type === type) l.fn(e); };
  const document = {
    ...hears(),
    getElementById: (id) => (id === "mk" ? box : logoId !== null && id === logoId ? logo : null),
    querySelector: (q) => (q.includes("mark") ? logo : q === ".foot" ? foot : q.includes("mk") ? box : null),
  };
  const window = { ...hears(), document };
  const calls = [];
  const fetch = (url, init) => new Promise((ok, no) => calls.push({ url, init, ok, no }));
  new Function("document", "fetch", "window", "addEventListener", script)(document, fetch, window, window.addEventListener);
  const arrive = () => {
    say(document, "DOMContentLoaded");
    if (back !== undefined) box.checked = back;
    say(window, "load");
    say(window, "pageshow");
  };
  if (!loading) arrive();
  /** The person clicks the switch. */
  const flip = (on) => { box.checked = on; const e = { target: box }; if (box.onchange) box.onchange(e); for (const l of listeners) l(e); };
  const settled = () => new Promise((ok) => setTimeout(ok, 0));
  return { state: () => ({ switch: box.checked ? "on" : "off", logo: logo.hidden ? "hidden" : "shown", foot: foot.hidden ? "hidden" : "shown" }), calls, flip, settled, arrive };
}

describe("the mark, switched off (U3, N26; DEVIATIONS #41)", () => {
  const ON = { switch: "on", logo: "shown", foot: "shown" };
  // Off, nothing shows in the owner's corner: the foot is hidden whole.
  const OFF = { switch: "off", logo: "hidden", foot: "hidden" };
  const ownerPage = async (env, cookie, path = `/${ID}`) => (await req(env, path, { headers: { cookie } })).text();

  it("leaves the owner's own page too — the switch, its words and the logo — and stays gone after a reload", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect(footOf(await ownerPage(env, cookie))).toEqual(ON);
    expect((await post(env, "/_shroom/mark", { on: false }, cookie)).status).toBe(200);
    const off = await ownerPage(env, cookie);
    expect(footOf(off)).toEqual(OFF);
    // A hidden foot is gone, not merely marked: the page's own style mustn't show it anyway
    // (`.foot` lays itself out as flex, which a bare `hidden` doesn't beat).
    expect(off).toMatch(/\.foot\[hidden\]\{display:none\}/);
    expect(off).toMatch(/\.mark\[hidden\]\{display:none\}/);
    expect(footOf(await (await req(env, `/${ID}`)).text())).toBeNull();
    expect((await post(env, "/_shroom/mark", { on: true }, cookie)).status).toBe(200);
    expect(footOf(await ownerPage(env, cookie))).toEqual(ON);
    expect(footOf(await (await req(env, `/${ID}`)).text())).toEqual({ switch: null, logo: "shown", foot: "shown" });
  });

  it("is never served stale: the page and the write's answer are both no-store", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const write = await post(env, "/_shroom/mark", { on: false }, cookie);
    expect(write.headers.get("cache-control")).toBe("no-store");
    const page = await req(env, `/${ID}`, { headers: { cookie } });
    expect(page.headers.get("cache-control")).toBe("no-store");
    expect(page.headers.get("set-cookie")).toBeNull();
  });

  it("goes at once on the flip, and comes back at once — before the write's answer, without a reload", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const on = markScript(await ownerPage(env, cookie));
    expect(on.state()).toEqual(ON);
    on.flip(false);
    expect(on.state()).toEqual(OFF);
    expect(on.calls).toHaveLength(1);
    expect(on.calls[0].url).toBe("/_shroom/mark");
    expect(on.calls[0].init).toMatchObject({ method: "POST", body: JSON.stringify({ on: false }) });
    on.calls[0].ok({ ok: true, status: 200 });
    await on.settled();
    expect(on.state()).toEqual(OFF);

    await post(env, "/_shroom/mark", { on: false }, cookie);
    const off = markScript(await ownerPage(env, cookie));
    expect(off.state()).toEqual(OFF);
    off.flip(true);
    expect(off.state()).toEqual(ON);
    expect(off.calls[0].init).toMatchObject({ method: "POST", body: JSON.stringify({ on: true }) });
  });

  it("a flip that didn't save goes back — the switch and the mark together", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const body = await ownerPage(env, cookie);
    const offline = markScript(body);
    offline.flip(false);
    offline.calls[0].no(new TypeError("Load failed"));
    await offline.settled();
    expect(offline.state()).toEqual(ON);
    // A browser whose cookie stopped counting meanwhile (its Mac removed): the write is refused.
    const refused = markScript(body);
    refused.flip(false);
    refused.calls[0].ok({ ok: false, status: 403 });
    await refused.settled();
    expect(refused.state()).toEqual(ON);
  });

  it("the page's own render decides what the switch says — not a state the browser put back on a Back", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect(markScript(await ownerPage(env, cookie), { restored: false }).state()).toEqual(ON);
    await post(env, "/_shroom/mark", { on: false }, cookie);
    expect(markScript(await ownerPage(env, cookie), { restored: true }).state()).toEqual(OFF);
  });

  it("nor one it puts back after the page has drawn — the switch and the mark say what the space says, and nothing is written", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const on = markScript(await ownerPage(env, cookie), { back: false });
    expect(on.state()).toEqual(ON);
    expect(on.calls).toHaveLength(0);
    await post(env, "/_shroom/mark", { on: false }, cookie);
    const off = markScript(await ownerPage(env, cookie), { back: true });
    expect(off.state()).toEqual(OFF);
    expect(off.calls).toHaveLength(0);
  });

  it("a flip made while the page is still arriving stays as flipped", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const early = markScript(await ownerPage(env, cookie), { loading: true });
    early.flip(false);
    expect(early.state()).toEqual(OFF);
    early.arrive();
    expect(early.state()).toEqual(OFF);
    expect(early.calls).toHaveLength(1);
    early.calls[0].ok({ ok: true, status: 200 });
    await early.settled();
    expect(early.state()).toEqual(OFF);
  });

  // Two flips in quick succession (found in review; there before #41 too): each flip sent
  // a write of its own with nothing holding their order, so the later one could land first
  // and the page end showing ON over a space that says OFF.

  /** The network, carrying one of the page's writes to the space and its answer back. */
  const carry = async (env, cookie, page, call) => {
    call.carried = true;
    call.ok(await post(env, call.url, JSON.parse(call.init.body), cookie));
    await page.settled();
  };
  const sent = (page) => page.calls.map((c) => JSON.parse(c.init.body).on);

  it("two quick flips are saved one at a time, the later after the earlier — the page ends on what the space holds", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const page = markScript(await ownerPage(env, cookie));
    page.flip(false);
    page.flip(true);
    expect(page.state()).toEqual(ON);
    // The network at its worst: of the writes on their way, the last one sent arrives first.
    let most = 0;
    for (let away; (away = page.calls.filter((c) => !c.carried)).length > 0;) {
      most = Math.max(most, away.length);
      await carry(env, cookie, page, away[away.length - 1]);
    }
    expect(page.state()).toEqual(ON);
    expect(footOf(await ownerPage(env, cookie)), "what the space holds, against what the page was left showing").toEqual(page.state());
    expect(most, "writes on their way at once").toBe(1);
    expect(sent(page)).toEqual([false, true]);
  });

  it("a flip taken back while its write is away is written back once that one lands", async () => {
    const env = space({ ".shroom/mark.json": JSON.stringify({ on: false }) });
    const cookie = await cookieFor(env);
    const page = markScript(await ownerPage(env, cookie));
    page.flip(true);
    page.flip(false);
    expect(page.state()).toEqual(OFF);
    expect(sent(page)).toEqual([true]);
    await carry(env, cookie, page, page.calls[0]);
    expect(sent(page)).toEqual([true, false]);
    await carry(env, cookie, page, page.calls[1]);
    expect(sent(page)).toEqual([true, false]);
    expect(page.state()).toEqual(OFF);
    expect(footOf(await ownerPage(env, cookie))).toEqual(OFF);
  });

  it("flipped away and back again before the first write lands, nothing more is written — the space already holds it", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const page = markScript(await ownerPage(env, cookie));
    page.flip(false);
    page.flip(true);
    page.flip(false);
    expect(page.state()).toEqual(OFF);
    for (let away; (away = page.calls.filter((c) => !c.carried)).length > 0;) await carry(env, cookie, page, away[away.length - 1]);
    expect(sent(page)).toEqual([false]);
    expect(page.state()).toEqual(OFF);
    expect(footOf(await ownerPage(env, cookie))).toEqual(OFF);
  });

  it("a first write that didn't save, its flip already taken back: the page stays on what the space last said, and writes nothing more", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const body = await ownerPage(env, cookie);
    for (const fail of [(c) => c.no(new TypeError("Load failed")), (c) => c.ok({ ok: false, status: 403 })]) {
      const page = markScript(body);
      page.flip(false);
      page.flip(true);
      expect(sent(page)).toEqual([false]);
      fail(page.calls[0]);
      await page.settled();
      expect(page.state()).toEqual(ON);
      expect(sent(page)).toEqual([false]);
    }
    expect(footOf(await ownerPage(env, cookie))).toEqual(ON);
  });

  it("a flip made after a write that didn't save is saved like any other", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const page = markScript(await ownerPage(env, cookie));
    page.flip(false);
    page.calls[0].no(new TypeError("Load failed"));
    await page.settled();
    expect(page.state()).toEqual(ON);
    page.flip(false);
    expect(sent(page)).toEqual([false, false]);
    await carry(env, cookie, page, page.calls[1]);
    expect(page.state()).toEqual(OFF);
    expect(footOf(await ownerPage(env, cookie))).toEqual(OFF);
  });

  // Which faces the setting governs: every face of a video page and the embed (“The small
  // mark on every video page”, SM:1198). The join page carries its mark whatever the
  // switch says (region6.test.js, U6).
  it("governs every face of a video page and the embed — and an owner-class browser keeps the switch on each", async () => {
    const stalled = { done: 1, total: 4, updatedAt: "2026-10-02T11:00:00Z" };
    const moving = { done: 1, total: 4, updatedAt: "2026-10-02T11:59:50Z" };
    const faces = {
      live: metaOf(),
      "on its way": metaOf({ state: "uploading", media: null, progress: moving }),
      stalled: metaOf({ state: "uploading", media: null, progress: stalled }),
      unavailable: metaOf({ media: null }),
      removed: JSON.stringify({ schema: 1, id: ID, state: "deleted" }),
      "an id the space never heard of": null,
    };
    for (const [face, record] of Object.entries(faces)) {
      for (const on of [true, false]) {
        const env = space({ ".shroom/mark.json": JSON.stringify({ on }) });
        if (record === null) env.BUCKET.store.delete(`meta/${ID}.json`);
        else env.BUCKET.touch(`meta/${ID}.json`, record);
        const cookie = await cookieFor(env);
        const tag = `${face}, the mark ${on ? "on" : "off"}`;
        expect(footOf(await (await req(env, `/${ID}`)).text()), `a recipient — ${tag}`).toEqual(on ? { switch: null, logo: "shown", foot: "shown" } : null);
        expect((await (await req(env, `/${ID}?embed=1`)).text()).includes(`class="mark"`), `the embed — ${tag}`).toBe(on);
        expect(footOf(await ownerPage(env, cookie)), `the owner — ${tag}`).toEqual(on ? ON : OFF);
      }
    }
  });

  it("a member's own page follows it like any recipient's, and never gets the switch", async () => {
    for (const on of [true, false]) {
      const env = space({ ".shroom/mark.json": JSON.stringify({ on }) });
      env.BUCKET.touch(ledgerKey(ID), JSON.stringify({ v: 1, member: MHER, device: MHER_MAC.id, at: "2026-10-02T11:00:00Z" }), { m: MHER });
      const own = await ownerPage(env, await cookieFor(env, memberGrant()));
      expect(own).toContain(`class="title"`);
      expect(footOf(own)).toEqual(on ? { switch: null, logo: "shown", foot: "shown" } : null);
      expect(own).not.toContain("/_shroom/mark");
    }
  });
});

describe("the page's status", () => {
  it("says only what the upload line needs — never a title", () => {
    expect(status(null, NOW)).toEqual({ state: "waiting" });
    expect(status({ state: "uploading", title: "secret", progress: { done: 3, total: 4, updatedAt: "2026-10-02T11:59:59Z" } }, NOW)).toEqual({ state: "uploading", pct: 75, stalled: false });
    // `drained` (corrected to r4's `fully-live` — “queue drained”, DECISIONS B49): a record
    // without a count is drained; one whose count isn't reached says how far, and stalls.
    expect(status({ state: "live", media: { hls: "x" } }, NOW)).toEqual({ state: "live", poster: false, mp4: false, teaser: false, thumb: null, dur: 0, drained: true });
    // The owner page adds Download when the MP4 lands after the go-live, and swaps the
    // player in with its first frame (gap: the in-place poll never added Download).
    expect(status({ state: "live", title: "secret", media: { hls: "x", poster: "p", mp4: "m" } }, NOW)).toEqual({ state: "live", poster: true, mp4: true, teaser: false, thumb: null, dur: 0, drained: true });
    // Copy thumbnail's loop is drawn from the teaser once it exists (N46, B164).
    expect(status({ state: "live", media: { hls: "x", poster: "p", teaser: "t" } }, NOW)).toMatchObject({ teaser: true });
    expect(status({ state: "live", media: null }, NOW)).toEqual({ state: "unavailable" });
    expect(status({ state: "deleted" }, NOW)).toEqual({ state: "removed" });
  });

  it("dates the placeholder from the recording's day", () => {
    expect(untitled("2026-08-21T10:00:00Z")).toBe("Untitled — Aug 21");
  });
});
