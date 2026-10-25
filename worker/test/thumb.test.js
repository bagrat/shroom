// Copy thumbnail (N46, DECISIONS B164): the owner page stores one picture per title beside
// the video — a still before the loop exists, then the loop — and copies it wrapped in the
// link. The cookie's one byte-writing power (WORKER-INTERFACE §3.5, §4.17): checked like a
// title save, a GIF or a JPEG by its first bytes, capped in size, width and count, named by
// its contents, recorded only for the title it was drawn for.
import { createHash } from "node:crypto";
import { beforeEach, describe, expect, it } from "vitest";
import { COPY, COPY_JS, THUMB, THUMB_JS, forgetSecret, handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";
import { MHER, MHER_MAC, OWNER_MAC, ROSTER, roster } from "./space-fixture.js";

const ID = "AbCdEfGhIjKlMnOp";
const HOST = "https://v.example.workers.dev";
const NOW = Date.parse("2026-10-15T12:00:00Z");
const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk";
const sha = (s) => createHash("sha256").update(s).digest("hex");
const SECRET = Buffer.alloc(32, 7).toString("base64url");
const metaOf = (over = {}) => JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-15T11:00:00Z", title: "Where the 502s come from", titled: true,
  durationSec: 272, media: { hls: `rec/${ID}/master.m3u8`, mp4: null, poster: `rec/${ID}/poster.jpg`, teaser: `rec/${ID}/teaser.mp4`, width: 1920, height: 1080 }, owner: "m1", ...over });
const grant = (over = {}) => JSON.stringify({ member: "m1", role: "owner", device: OWNER_MAC.id, exp: new Date(NOW + 600_000).toISOString(), ...over });

const req = (env, path, init = {}) => handle(new Request(`${HOST}${path}`, init), env, NOW);
const space = (extra = {}) => ({ BUCKET: fakeBucket({ ".shroom/cookie.key": SECRET, [ROSTER]: JSON.stringify(roster()), [`meta/${ID}.json`]: metaOf(), ...extra }) });
async function cookieFor(env, g = grant()) {
  const k = `${K.slice(0, 42)}Q`;
  env.BUCKET.store.set(`.shroom/keys/${sha(k)}`, new TextEncoder().encode(g));
  const set = (await req(env, `/${ID}?k=${k}`)).headers.get("set-cookie");
  return set ? set.split(";")[0] : null;
}
const metaNow = (env) => JSON.parse(new TextDecoder().decode(env.BUCKET.store.get(`meta/${ID}.json`)));

/** A GIF89a's header, its logical screen, and the trailer. */
function gif(w = 600, h = 338, size = 2000) {
  const b = new Uint8Array(size);
  b.set([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, w & 255, w >> 8, h & 255, h >> 8]);
  b[size - 1] = 0x3b;
  return b;
}
/** A JPEG's start, one frame header (SOF0) with its size, and its end. */
function jpeg(w = 1200, h = 675, size = 2000) {
  const b = new Uint8Array(size);
  b.set([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00, 0xff, 0xc0, 0x00, 0x11, 0x08, h >> 8, h & 255, w >> 8, w & 255]);
  b.set([0xff, 0xd9], size - 2);
  return b;
}
const put = (env, bytes, { cookie, type = "image/gif", title = "Where the 502s come from", origin = HOST, id = ID, from } = {}) =>
  req(env, `/_shroom/thumb/${id}`, { method: "POST", body: bytes,
    headers: { "content-type": type, origin, "x-shroom-title": encodeURIComponent(title ?? ""), ...(from === undefined ? {} : { "x-shroom-from": from }), ...(cookie ? { cookie } : {}) } });

beforeEach(() => forgetSecret());

describe("storing a thumbnail", () => {
  it("stores the loop beside the video, named by its contents, and points the record at it for its title", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const bytes = gif();
    const res = await put(env, bytes, { cookie });
    expect(res.status).toBe(200);
    const name = `thumb-${createHash("sha256").update(bytes).digest("hex").slice(0, 16)}.gif`;
    expect(await res.json()).toEqual({ url: `/m/${ID}/${name}` });
    expect(env.BUCKET.store.has(`rec/${ID}/${name}`)).toBe(true);
    expect(metaNow(env).thumb).toEqual({ title: "Where the 502s come from", from: "poster.jpg", gif: name });
    // The same picture again is the same file.
    expect((await put(env, bytes, { cookie })).status).toBe(200);
    expect([...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${ID}/thumb-`))).toHaveLength(1);
  });

  it("keeps the still and the loop for one title, and starts over when the title changes", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await put(env, jpeg(), { cookie, type: "image/jpeg" })).status).toBe(200);
    expect((await put(env, gif(), { cookie })).status).toBe(200);
    expect(Object.keys(metaNow(env).thumb).sort()).toEqual(["from", "gif", "jpg", "title"]);
    env.BUCKET.touch(`meta/${ID}.json`, JSON.stringify({ ...metaNow(env), title: "Why the 502s were the cache" }));
    expect((await put(env, jpeg(1200, 675, 2100), { cookie, type: "image/jpeg", title: "Why the 502s were the cache" })).status).toBe(200);
    const thumb = metaNow(env).thumb;
    expect(thumb.title).toBe("Why the 502s were the cache");
    expect(thumb.gif).toBeUndefined();
    // The earlier title's pictures stay: a paste already sent keeps its picture.
    expect([...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${ID}/thumb-`))).toHaveLength(3);
  });

  it("records an untitled video's picture against no title", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ title: "Untitled — Oct 15", titled: false }) });
    const cookie = await cookieFor(env);
    expect((await put(env, gif(), { cookie, title: "" })).status).toBe(200);
    expect(metaNow(env).thumb.title).toBeNull();
  });

  it("refuses a picture drawn for a title the video no longer has", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await put(env, gif(), { cookie, title: "An older title" })).status).toBe(409);
    expect((await put(env, gif(), { cookie, title: "" })).status).toBe(409);
    expect([...env.BUCKET.store.keys()].some((k) => k.startsWith(`rec/${ID}/thumb-`))).toBe(false);
  });

  it("is a title save's power: same-origin, a live cookie, and the right to rename this video", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    expect((await put(env, gif())).status).toBe(403);
    expect((await put(env, gif(), { cookie, origin: "https://evil.example" })).status).toBe(403);
    const member = await cookieFor(env, grant({ member: MHER, role: "member", device: MHER_MAC.id }));
    expect((await put(env, gif(), { cookie: member })).status).toBe(403);
    expect([...env.BUCKET.store.keys()].some((k) => k.startsWith(`rec/${ID}/thumb-`))).toBe(false);
  });

  it("takes only a GIF or a JPEG, by its first bytes, under its caps", async () => {
    const env = space();
    const cookie = await cookieFor(env);
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0]);
    expect((await put(env, png, { cookie, type: "image/png" })).status).toBe(415);
    expect((await put(env, png, { cookie, type: "image/gif" })).status).toBe(415);
    expect((await put(env, jpeg(), { cookie, type: "image/gif" })).status).toBe(415);
    expect((await put(env, gif(THUMB.gifWidth + 1), { cookie })).status).toBe(400);
    expect((await put(env, jpeg(THUMB.jpgWidth + 1), { cookie, type: "image/jpeg" })).status).toBe(400);
    expect((await put(env, gif(600, 338, THUMB.gifMax + 1), { cookie })).status).toBe(413);
    expect((await put(env, jpeg(1200, 675, THUMB.jpgMax + 1), { cookie, type: "image/jpeg" })).status).toBe(413);
    expect([...env.BUCKET.store.keys()].some((k) => k.startsWith(`rec/${ID}/thumb-`))).toBe(false);
  });

  it("stops at its count per video", async () => {
    const extra = {};
    for (let i = 0; i < THUMB.perVideo; i++) extra[`rec/${ID}/thumb-${String(i).padStart(16, "0")}.gif`] = "x";
    const env = space(extra);
    const cookie = await cookieFor(env);
    expect((await put(env, gif(), { cookie })).status).toBe(429);
  });

  it("refuses a video that isn't live, is deleted, sealed, or doesn't exist", async () => {
    for (const [over, code] of [[{ state: "uploading" }, 409], [{ state: "deleted" }, 409]]) {
      const env = space({ [`meta/${ID}.json`]: metaOf(over) });
      expect((await put(env, gif(), { cookie: await cookieFor(env) })).status).toBe(code);
    }
    const sealed = space({ [`.shroom/gone/${ID}`]: "{}" });
    expect((await put(sealed, gif(), { cookie: await cookieFor(sealed) })).status).toBe(409);
    const env = space();
    const cookie = await cookieFor(env);
    expect((await put(env, gif(), { cookie, id: "ZzZzZzZzZzZzZzZz" })).status).toBe(404);
  });
});

// N63: an edit, its copy and its bake change the first frame Copy thumbnail draws from. A
// picture says which it was drawn from (`x-shroom-from`; a page from before the header drew
// from `poster.jpg`), and the Worker stores only one drawn from what the video shows now.
describe("the first frame a picture was drawn from", () => {
  const EDIT = { version: 1, source_duration: 272, in: 8, out: 241, cuts: [{ start: 150, end: 162, kind: "retake", confidence: "high", applied: true }], rev: 3 };
  const COPY3 = { rev: 3, file: "edit-3.mp4", duration: 221, poster: "edit-3-poster.jpg", teaser: "edit-3-teaser.mp4" };
  const thumbs = (env) => [...env.BUCKET.store.keys()].filter((k) => k.startsWith(`rec/${ID}/thumb-`));

  it("is stamped on the record: the recording's own poster with no edit, said or not", async () => {
    for (const from of [undefined, "poster.jpg"]) {
      const env = space();
      expect((await put(env, gif(), { cookie: await cookieFor(env), from })).status).toBe(200);
      expect(metaNow(env).thumb.from).toBe("poster.jpg");
    }
  });

  it("is refused, and nothing stored, while an edit waits for its copy: there is nothing of the video to draw from", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ edits: EDIT }) });
    const cookie = await cookieFor(env);
    for (const from of [undefined, "poster.jpg", "edit-3-poster.jpg", ""]) expect((await put(env, gif(), { cookie, from })).status).toBe(409);
    expect(thumbs(env)).toEqual([]);
    expect(metaNow(env).thumb).toBeUndefined();
  });

  it("once the copy is up, is the copy's own first frame — a picture of the original is refused", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ edits: { ...EDIT, copy: COPY3 }, thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif", jpg: "thumb-fedcba9876543210.jpg" } }) });
    const cookie = await cookieFor(env);
    for (const from of [undefined, "poster.jpg", "edit-2-poster.jpg"]) expect((await put(env, jpeg(), { cookie, type: "image/jpeg", from })).status).toBe(409);
    expect(thumbs(env)).toEqual([]);
    // Drawn from the copy's: stored, and it starts over — the original's loop isn't kept beside it.
    const res = await put(env, jpeg(), { cookie, type: "image/jpeg", from: "edit-3-poster.jpg" });
    expect(res.status).toBe(200);
    const { url } = await res.json();
    expect(metaNow(env).thumb).toEqual({ title: "Where the 502s come from", from: "edit-3-poster.jpg", jpg: url.split("/").pop() });
    expect((await req(env, `/_shroom/status/${ID}`)).status).toBe(200);
    expect((await (await req(env, `/_shroom/status/${ID}`)).json()).thumb).toBe(url);
    // The loop drawn next from the same joins it.
    expect((await put(env, gif(), { cookie, from: "edit-3-poster.jpg" })).status).toBe(200);
    expect(Object.keys(metaNow(env).thumb).sort()).toEqual(["from", "gif", "jpg", "title"]);
  });

  it("after the bake, is the first frame the record names", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ durationSec: 221, bakedFrom: [{ at: "2026-10-15T11:30:00Z", rev: 3, keep: [[8, 150], [162, 241]] }],
      media: { hls: null, mp4: `rec/${ID}/edit-3.mp4`, poster: `rec/${ID}/edit-3-poster.jpg`, teaser: `rec/${ID}/edit-3-teaser.mp4`, width: 1920, height: 1080 } }) });
    const cookie = await cookieFor(env);
    expect((await put(env, gif(), { cookie })).status).toBe(409);
    expect((await put(env, gif(), { cookie, from: "edit-3-poster.jpg" })).status).toBe(200);
    expect(metaNow(env).thumb.from).toBe("edit-3-poster.jpg");
  });
});

/** The page's Copy thumbnail script in a stand-in browser: every picture loads, the canvas
 * draws nothing, and what it asks of the network and the clipboard is kept. */
function thumbPage(c) {
  const fetched = [], copied = [], images = [];
  const ctx = new Proxy({}, { get: (_, k) => (k === "measureText" ? () => ({ width: 10 }) : k === "createLinearGradient" ? () => ({ addColorStop() {} })
    : k === "getImageData" ? () => ({ data: new Uint8ClampedArray(64).fill(255) }) : () => {}), set: () => true });
  const element = (tag) => ({ tag, style: {}, kids: [], appendChild(x) { this.kids.push(x); }, setAttribute() {}, getContext: () => ctx,
    toBlob(done, type) { done(new Blob(["x"], { type })); }, get outerHTML() { return `<${tag} ${this.href ?? this.src ?? ""}>${this.kids.map((k) => k.outerHTML).join("")}`; } });
  function Image() { images.push(this); this.decode = () => Promise.resolve(); }
  function ClipboardItem(parts) { this.parts = parts; }
  const document = { createElement: element, addEventListener() {}, removeEventListener() {}, execCommand: () => false, body: { appendChild() {} } };
  const navigator = { clipboard: { writeText: (text) => { copied.push({ text }); return Promise.resolve(); }, write: (items) => { copied.push({ html: items[0].parts["text/html"] }); return Promise.resolve(); } } };
  const fetch = (url, init) => { fetched.push({ url, init }); return Promise.resolve({ ok: true, json: () => Promise.resolve({ url: `/m/${ID}/thumb-1111111111111111.jpg` }) }); };
  const shroomThumbs = new Function("document", "navigator", "location", "window", "ClipboardItem", "Image", "fetch", "setTimeout", "clearTimeout",
    `${COPY_JS};${THUMB_JS};return shroomThumbs`)(document, navigator, { origin: HOST }, { ClipboardItem }, ClipboardItem, Image, fetch, () => 0, () => {});
  const th = shroomThumbs({ id: ID, live: true, store: true, title: "Where the 502s come from", titled: true, thumb: null, gif: false, teaser: false, len: "3:41",
    from: "poster.jpg", psrc: `/m/${ID}/poster.jpg`, tsrc: `/m/${ID}/teaser.mp4`, rw: 16, rh: 9, caps: THUMB, ...c });
  return { th, fetched, copied, images };
}
const settle = async () => { for (let i = 0; i < 20; i++) await new Promise((ok) => setImmediate(ok)); };

describe("the page's own drawing (N46, N63)", () => {
  it("tells the Worker which first frame it drew from", async () => {
    const p = thumbPage({ from: "edit-3-poster.jpg", psrc: `/m/${ID}/edit-3-poster.jpg`, tsrc: `/m/${ID}/edit-3-teaser.mp4` });
    p.th.live();
    await settle();
    expect(p.images.map((i) => i.src)).toEqual([`/m/${ID}/edit-3-poster.jpg`]);
    expect(p.fetched).toHaveLength(1);
    expect(p.fetched[0].url).toBe(`/_shroom/thumb/${ID}`);
    expect(p.fetched[0].init.headers).toMatchObject({ "content-type": "image/jpeg", "x-shroom-from": "edit-3-poster.jpg" });
  });

  it("draws nothing with nothing to draw from, and copies the link alone — never a first frame that's cut", async () => {
    for (const store of [true, false]) {
      const p = thumbPage({ store, from: "", psrc: "", tsrc: "" });
      p.th.live();
      p.th.teaser();
      p.th.retitle("A new title");
      await settle();
      expect(p.images).toEqual([]);
      expect(p.fetched).toEqual([]);
      expect(await p.th.copy()).toBe(true);
      expect(p.copied).toEqual([{ text: `${HOST}/${ID}` }]);
    }
  });

  it("copies the video's picture wrapped in its link", async () => {
    const p = thumbPage({ store: false, thumb: `/m/${ID}/thumb-0123456789abcdef.gif`, gif: true });
    expect(await p.th.copy()).toBe(true);
    expect(await (await p.copied[0].html).text()).toContain(`${HOST}/m/${ID}/thumb-0123456789abcdef.gif`);
  });
});

describe("serving and copying it", () => {
  it("serves a stored loop as media, and stops when the video is sealed", async () => {
    const env = space({ [`rec/${ID}/thumb-0123456789abcdef.gif`]: gif() });
    const res = await req(env, `/m/${ID}/thumb-0123456789abcdef.gif`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("image/gif");
    expect(res.headers.get("x-content-type-options")).toBe("nosniff");
    env.BUCKET.store.set(`.shroom/gone/${ID}`, new TextEncoder().encode("{}"));
    expect((await req(env, `/m/${ID}/thumb-0123456789abcdef.gif`)).status).toBe(404);
  });

  it("puts Copy thumbnail beside Copy link on the owner page, there from the first paint, greyed until the video plays (Bagrat, 2026-10-25)", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" } }) });
    const cookie = await cookieFor(env);
    const live = await (await req(env, `/${ID}`, { headers: { cookie } })).text();
    expect(live).toContain(`<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button><button class="copy thumb" id="th" data-tip="${COPY.copyThumbnail}">${COPY.copyThumbnail}</button>`);
    const waiting = space({ [`meta/${ID}.json`]: metaOf({ state: "uploading", media: null }) });
    const page = await (await req(waiting, `/${ID}`, { headers: { cookie: await cookieFor(waiting) } })).text();
    expect(page).toContain(`<button class="copy thumb" id="th" data-tip="${COPY.preparingThumbnail}" aria-disabled="true">${COPY.copyThumbnail}</button>`);
    // A viewer's page has them too (N46, the viewer's ruling) — see below.
    expect(await (await req(env, `/${ID}`)).text()).toContain(COPY.copyThumbnail);
  });

  it("hands the page the picture made for the current title, and nothing for an older one", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" } }) });
    const cookie = await cookieFor(env);
    expect(await (await req(env, `/${ID}`, { headers: { cookie } })).text()).toContain(`"thumb":"/m/${ID}/thumb-0123456789abcdef.gif"`);
    env.BUCKET.touch(`meta/${ID}.json`, metaOf({ thumb: { title: "An older title", gif: "thumb-0123456789abcdef.gif" } }));
    expect(await (await req(env, `/${ID}`, { headers: { cookie } })).text()).toContain(`"thumb":null`);
  });

  it("tells the page when the teaser exists, and the Library the newest picture for the title", async () => {
    const env = space();
    expect(await (await req(env, `/_shroom/status/${ID}`)).json()).toMatchObject({ state: "live", teaser: true, thumb: null });
    env.BUCKET.touch(`meta/${ID}.json`, metaOf({ thumb: { title: "Where the 502s come from", jpg: "thumb-fedcba9876543210.jpg" } }));
    expect((await (await req(env, `/_shroom/status/${ID}`)).json()).thumb).toBe(`/m/${ID}/thumb-fedcba9876543210.jpg`);
  });
});

describe("the viewer's page (N46, settling Region 4's open question)", () => {
  it("shows Copy link and Copy thumbnail beside Download, each named and with a tooltip", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" }, media: { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, poster: `rec/${ID}/poster.jpg`, width: 1920, height: 1080 } }) });
    const page = await (await req(env, `/${ID}`)).text();
    expect(page).toContain(`<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button><button class="copy thumb" id="th" data-tip="${COPY.copyThumbnail}">${COPY.copyThumbnail}</button>`);
    expect(page).toContain(`<a href="/d/${ID}">${COPY.download}</a>`);
    expect(page).not.toContain(`class="title"`);
  });

  it("gives a viewer the newest picture for the title — a viewer stores nothing", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif", jpg: "thumb-fedcba9876543210.jpg" } }) });
    const page = await (await req(env, `/${ID}`)).text();
    expect(page).toContain(`"thumb":"/m/${ID}/thumb-0123456789abcdef.gif"`);
    expect(page).toContain(`"store":false`);
  });

  it("names the owner's buttons the same way", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" } }) });
    const page = await (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text();
    expect(page).toContain(`id="th" data-tip="${COPY.copyThumbnail}"`);
    expect(page).toContain(`"store":true`);
  });
});

describe("the picture's length (Bagrat on 0.1.15: the first pictures had no time)", () => {
  it("is in the status, so a page opened while the video uploaded draws it once the video is live", async () => {
    const env = space();
    expect(await (await req(env, `/_shroom/status/${ID}`)).json()).toMatchObject({ state: "live", dur: 272 });
    env.BUCKET.touch(`meta/${ID}.json`, metaOf({ durationSec: null }));
    expect((await (await req(env, `/_shroom/status/${ID}`)).json()).dur).toBe(0);
  });

  it("is set from the status before the page draws, at the go-live", async () => {
    const env = space({ [`meta/${ID}.json`]: metaOf({ state: "uploading", media: null, durationSec: null }) });
    const page = await (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text();
    expect(page).toContain(`if(s.dur)th.length(shroomClock(s.dur,true))`);
  });
});

describe("Copy thumbnail waits until its picture is ready (N46; Bagrat: “Yes disabled until ready, but tooltips need padding between the button, wording is ok”)", () => {
  const owner = async (over) => {
    const env = space({ [`meta/${ID}.json`]: metaOf(over) });
    return (await req(env, `/${ID}`, { headers: { cookie: await cookieFor(env) } })).text();
  };
  const viewer = async (over) => (await req(space({ [`meta/${ID}.json`]: metaOf(over) }), `/${ID}`)).text();
  const ready = `<button class="copy thumb" id="th" data-tip="${COPY.copyThumbnail}">${COPY.copyThumbnail}</button>`;

  it("is greyed on the owner page, saying it's preparing, until the loop for the title is stored", async () => {
    const waiting = `<button class="copy thumb" id="th" data-tip="${COPY.preparingThumbnail}" aria-disabled="true">${COPY.copyThumbnail}</button>`;
    expect(await owner({})).toContain(waiting);
    // The still alone isn't ready while a loop can still be made…
    expect(await owner({ thumb: { title: "Where the 502s come from", jpg: "thumb-fedcba9876543210.jpg" } })).toContain(waiting);
    // …nor a loop made for an older title.
    expect(await owner({ thumb: { title: "An older title", gif: "thumb-0123456789abcdef.gif" } })).toContain(waiting);
    expect(await owner({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" } })).toContain(ready);
  });

  it("wakes with the still when no loop can be made: the upload finished with no teaser", async () => {
    const drained = { progress: { done: 9, total: 9, updatedAt: "2026-10-15T11:59:00Z" } };
    const media = { hls: `rec/${ID}/master.m3u8`, mp4: null, poster: `rec/${ID}/poster.jpg`, teaser: null, width: 1920, height: 1080 };
    expect(await owner({ ...drained, media, thumb: { title: "Where the 502s come from", jpg: "thumb-fedcba9876543210.jpg" } })).toContain(ready);
  });

  it("doesn't copy while it waits, wakes when the loop is stored or can't be made, and waits again on a new title", async () => {
    const page = await owner({});
    expect(page).toContain(`tb.onclick=function(){if(tb.getAttribute("aria-disabled"))return;`);
    expect(page).toMatch(/th=shroomThumbs\(c\.thumbs,function\(ok\)\{/);
    expect(page).toContain(`if(s.drained)th.drained()`);
  });

  it("is hidden on the viewer's page until a picture made for the title exists", async () => {
    expect(await viewer({})).toMatch(/id="th" [^>]* hidden>/);
    expect(await viewer({ thumb: { title: "An older title", gif: "thumb-0123456789abcdef.gif" } })).toMatch(/id="th" [^>]* hidden>/);
    expect(await viewer({ thumb: { title: "Where the 502s come from", jpg: "thumb-fedcba9876543210.jpg" } })).toContain(ready);
  });

  it("draws every tooltip a clear gap below its button, Download's too", async () => {
    const media = { hls: `rec/${ID}/master.m3u8`, mp4: `rec/${ID}/video.mp4`, poster: `rec/${ID}/poster.jpg`, width: 1920, height: 1080 };
    const page = await viewer({ thumb: { title: "Where the 502s come from", gif: "thumb-0123456789abcdef.gif" }, media });
    expect(page).toContain(`<span class="dl" data-tip="${COPY.download}"><a href="/d/${ID}">${COPY.download}</a></span>`);
    expect(page).toMatch(/\[data-tip\]::after\{content:attr\(data-tip\);position:absolute;top:calc\(100% \+ 8px\)/);
    expect(page).not.toMatch(/id="(c|th)" title=/);
    expect(await owner({ media })).toContain(`<span id="dl" data-tip="${COPY.download}">`);
  });

  // A page opened while an edit waits for its copy has nothing to draw from (N63): the copy's
  // first frame. When the copy lands under the open page, the poll hands its first frame and
  // teaser over, and the picture is made and Copy thumbnail wakes, with no reload (the
  // orchestrator relaying Bagrat's rule of 2026-10-25: a button that never gets ready is a bug).
  it("wakes in place when an edit's copy lands under the open page, drawn from the copy", async () => {
    const page = await owner({ edits: { version: 1, source_duration: 150, in: 0, out: 150, cuts: [{ start: 10, end: 20, kind: "manual", applied: true }], rev: 3 } });
    expect(page).toContain(`"from":"","psrc":"","tsrc":""`);
    expect(page).toContain(`<button class="copy thumb" id="th" data-tip="${COPY.preparingThumbnail}" aria-disabled="true">`);
    // The page's drawing takes a new first frame (a copy's), and draws from it.
    expect(THUMB_JS).toContain(`source:function(f,p,t){if(!f||f===c.from)return;c.from=f;c.psrc=p;c.tsrc=t||"";c.teaser=!!t;c.thumb=null;c.gif=false;cb(false);if(c.live)make()}`);
    // The poll hands over the copy's, as the status names it.
    expect(page).toContain(`var cq=s.copy;if(cq&&cq.poster)th.source(cq.poster.slice(cq.poster.lastIndexOf("/")+1),cq.poster,cq.teaser||"");`);
  });
});
