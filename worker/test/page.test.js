// The page's faces as the whole-spec audit found them wanting (DECISIONS B43): the
// ratified custom player with its first frame (Region 4), the recipient's line over the
// thin bar (N27, N40), the owner's empty stage and its in-place poll, tombstoned bytes,
// the page POSTs' body cap, and a Copy that only says "Copied" when it copied (N1).
// What a browser does with all this is driven for real by `npm run drive:page`.
import { describe, expect, it } from "vitest";
import { COPY, COPY_JS, PLAYER_JS, STALL_SECONDS, clock, forgetSecret, handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";
import { M1, MHER, MHER_MAC, NOW, ORIGIN, OWNER_AIR, REC, cookieFor, ledger, metaKey, metaOf, space, textOf } from "./space-fixture.js";

const get = (env, path, init) => handle(new Request(`${ORIGIN}${path}`, init), env, NOW);
const html = async (env, path, init) => (await get(env, path, init)).text();
const live = (over = {}) => metaOf(REC, { durationSec: 272.4, media: { hls: `rec/${REC}/master.m3u8`, mp4: null, poster: `rec/${REC}/poster.jpg`, width: 2880, height: 1800 }, ...over });
const uploading = (done, total, ageSec = 10) => metaOf(REC, { state: "uploading", media: null, progress: { done, total, updatedAt: new Date(NOW - ageSec * 1000).toISOString() } });
const ownerCookie = () => cookieFor(M1, OWNER_AIR);

describe("the player (Region 4: first frame, play, scrub bar, 0:00 / 4:32, fullscreen — custom controls)", () => {
  it("is the custom player on the recipient's page: no native controls, the poster as its first frame", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: live() } }), `/${REC}`);
    const video = body.match(/<video[^>]*>/)?.[0] ?? "";
    expect(video).toContain(`id="v"`);
    expect(video).not.toMatch(/\scontrols/);
    expect(video).toContain(`poster="/m/${REC}/poster.jpg"`);
    expect(body).toContain(`class="player" data-dur="272.4" style="--ar:2880/1800"`);
    expect(body).toContain(`<button type="button" class="pp" aria-label="${COPY.play}">`);
    expect(body).toMatch(new RegExp(`<input type="range" class="seek" min="0" max="272.4"[^>]*aria-label="${COPY.seek}"`));
    expect(body).toContain(`<span class="time">0:00 / 4:32</span>`);
    expect(body).toContain(`<button type="button" class="fs" aria-label="${COPY.fullscreen}">`);
    expect(body).toContain(`shroomPlayer(document.querySelector(".player"),"/m/${REC}/master.m3u8"`);
  });

  it("shows the whole frame: the picture keeps the recording's shape and the control bar sits under it, never over it", async () => {
    // Bagrat's 16:10 take on 0.1.10: a bar laid over the picture hid its bottom row. The
    // layout itself is measured in real browsers by worker/dev/page-drive.mjs ("the whole frame").
    const body = await html(space({ objects: { [metaKey(REC)]: live() } }), `/${REC}`);
    const css = body.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
    expect(css).toMatch(/\.player video\{[^}]*aspect-ratio:var\(--ar,16\/9\)[^}]*object-fit:contain/);
    expect(css).toMatch(/\.ctl\{position:relative;flex:none/);
    expect(css).not.toMatch(/\.ctl\{[^}]*position:absolute/);
  });

  it("has no poster before one exists, and reads 0:00 until the length is known", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: metaOf(REC) } }), `/${REC}`);
    expect(body.match(/<video[^>]*>/)?.[0]).not.toContain("poster=");
    expect(body).toContain(`<span class="time">0:00</span>`);
  });

  it("is the same player on the owner page and the embed face", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    const owner = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`class="title"`);
    expect(owner.match(/<video[^>]*>/)?.[0]).toContain(`poster="/m/${REC}/poster.jpg"`);
    expect(owner.match(/<video[^>]*>/)?.[0]).not.toMatch(/\scontrols/);
    expect(owner).toContain(`class="fs"`);
    const embed = await html(env, `/${REC}?embed=1`);
    expect(embed.match(/<video[^>]*>/)?.[0]).toContain(`poster="${ORIGIN}/m/${REC}/poster.jpg"`);
    expect(embed.match(/<video[^>]*>/)?.[0]).not.toMatch(/\scontrols/);
    expect(embed).toContain(`<span class="time">0:00 / 4:32</span>`);
  });

  // Bagrat, 2026-10-25: at the end the head stopped short of the bar's end, the fill running
  // past it — a 0.1 s step held the value under a length that isn't a multiple of it. The
  // head travels inside the bar, and the played part (and the chapters) on the head's travel.
  // Measured in real browsers by tests/e2e/pages-gallery.mjs (the seek-head faces).
  it("lets the head reach the very end, and lays the played part and the chapters on the head's travel", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    for (const body of [await html(env, `/${REC}`), await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } }), await html(env, `/${REC}?embed=1`)]) {
      expect(body).toMatch(/<input type="range" class="seek" min="0" max="272\.4" step="any"/);
      const css = body.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? "";
      expect(css).toContain(`--th:14px`);
      expect(css).toContain(`calc(var(--th)/2 + var(--p)*(100% - var(--th)))`);
      expect(css).toContain(`calc(var(--th)/2 + var(--b)*(100% - var(--th)))`);
    }
    // The played part follows what the bar holds, so the two never part at the end.
    expect(PLAYER_JS).toContain(`sk.style.setProperty("--p",String(d?Math.min(1,(+sk.value||0)/d):0))`);
  });

  it("prints time the same on the server and in the page's script", () => {
    const shroomClock = new Function(`${PLAYER_JS};return shroomClock`)();
    for (const [s, want] of [[0, "0:00"], [0.9, "0:00"], [7, "0:07"], [59.99, "0:59"], [60, "1:00"], [272.4, "4:32"], [3599, "59:59"], [3600, "1:00:00"], [3729, "1:02:09"], [-3, "0:00"]]) {
      expect(clock(s), String(s)).toBe(want);
      expect(shroomClock(s), String(s)).toBe(want);
    }
    // A length rounds: the stream's 7.98 s reads as the record's 0:08.
    for (const [s, want] of [[7.98, "0:08"], [272.4, "4:32"], [272.5, "4:33"], [3599.6, "1:00:00"]]) {
      expect(clock(s, true), String(s)).toBe(want);
      expect(shroomClock(s, true), String(s)).toBe(want);
    }
  });
});

describe("the recipient's on-its-way faces (N27, N40)", () => {
  it("puts the line where the player will be, over a thin bar filled to the upload's progress", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(1, 4) } }), `/${REC}`);
    expect(body).toContain(`<div class="wait"><p class="line">${COPY.onItsWay}</p><span class="bar go"><i style="width:25%"></i></span></div>`);
    expect(body).toContain(`<meta http-equiv="refresh" content="5">`);
  });

  it("rests the bar when the upload stalls", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(3, 4, STALL_SECONDS + 1) } }), `/${REC}`);
    expect(body).toContain(`<div class="wait"><p class="line">${COPY.stalled}</p><span class="bar"><i style="width:75%"></i></span></div>`);
  });

  it("shows an unknown id the same frame and an empty bar — indistinguishable from a link whose details haven't arrived", async () => {
    const a = await html(space(), `/${REC}`);
    const b = await html(space(), "/zzzzzzzzzzzzzzzz");
    expect(a).toContain(`<div class="wait"><p class="line">${COPY.onItsWay}</p><span class="bar go"><i style="width:0%"></i></span></div>`);
    expect(a).toBe(b);
  });

  it("gives the embed face the same line and bar", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(2, 4) } }), `/${REC}?embed=1`);
    expect(body).toContain(`<div class="line"><p>${COPY.onItsWay}</p><span class="bar go"><i style="width:50%"></i></span></div>`);
    const stalled = await html(space({ objects: { [metaKey(REC)]: uploading(2, 4, STALL_SECONDS + 1) } }), `/${REC}?embed=1`);
    expect(stalled).toContain(`<span class="bar"><i style="width:50%"></i></span>`);
  });
});

describe("the owner page before and after the go-live (N26, N40)", () => {
  it("shows the player's empty frame while uploading — the upload line speaks, not the recipient's sentence", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(1, 4) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(body).toContain(`<div class="wait" id="w"></div>`);
    expect(textOf(body)).not.toContain(COPY.onItsWay);
    // One line from go-live (N65 (7)): the words keep no percent; the bar carries it.
    expect(textOf(body)).toContain(COPY.finishing);
    expect(textOf(body)).not.toContain("25%");
    // A small spinner in front of the words, no bar (Bagrat, 2026-10-18).
    expect(body).toContain(`<div class="upl" id="ul"><span class="spin" id="b" aria-hidden="true"></span><span class="up" id="u">${COPY.finishing}</span></div>`);
    const stalled = await html(space({ objects: { [metaKey(REC)]: uploading(1, 4, STALL_SECONDS + 1) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    // Stalled, its words say why and the spinner rests.
    expect(stalled).toContain(`<div class="upl" id="ul"><span class="spin rest" id="b" aria-hidden="true"></span><span class="up" id="u">${COPY.waitingConnection}</span></div>`);
  });

  it("hands its poll the player to swap in, the lines for a removed video and unavailable, and Download's word", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(1, 4) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    const cfg = JSON.parse(body.match(/var c=(\{.*?\}),t=document/)?.[1].replace(/\\u003c/g, "<").replace(/\\u003e/g, ">").replace(/\\u0026/g, "&") ?? "null");
    expect(cfg).toMatchObject({ id: REC, live: false, mp4: false, notFound: COPY.notFound, unavailable: COPY.unavailable, dl: COPY.download });
    expect(cfg.finishing).toBe(COPY.finishing);
    expect(cfg.player).toContain(`<video id="v" playsinline preload="metadata"></video>`);
    // The poll's branches: Download wakes once the MP4 lands, the tombstone, unavailable.
    expect(body).toContain(`if(da){if(s.mp4)wake(da,"/d/"+c.id);else rest(da)}`);
    expect(body).toContain(`if(s.state==="removed")`);
    expect(body).toContain(`if(s.state==="unavailable")`);
  });

  it("fades its one line when it sees the upload finish, with no “Ready ✓” after it (N65 (7), B142 retired on the page)", async () => {
    const body = await html(space({ objects: { [metaKey(REC)]: uploading(1, 4) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    const cfg = JSON.parse(body.match(/var c=(\{.*?\}),t=document/)?.[1].replace(/\\u003c/g, "<").replace(/\\u003e/g, ">").replace(/\\u0026/g, "&") ?? "null");
    expect(cfg.ready).toBeUndefined();
    expect(COPY.ready).toBeUndefined();
    expect(body).not.toContain("Ready ✓");
    expect(body).toContain(`ul.classList.add("fade")`);
    // Opened after it drained: no line at all, as before.
    const done = await html(space({ objects: { [metaKey(REC)]: metaOf(REC) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(done).toContain(`<div class="upl" id="ul" hidden>`);
  });

  it("keeps polling a live page until the MP4 lands, and not once it has", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    const waiting = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(waiting).toContain(`"live":true,"drained":true,"pending":false,"mp4":false`);
    expect(waiting).toContain(`if(!c.live||!c.mp4||!c.drained)next(2000)`);
    const done = await html(space({ objects: { [metaKey(REC)]: live({ media: { hls: "x", mp4: `rec/${REC}/video.mp4`, poster: null, width: 1, height: 1 } }) } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(done).toContain(`"live":true,"drained":true,"pending":false,"mp4":true`);
    expect(done).toContain(`<a href="/d/${REC}">${COPY.download}</a>`);
  });
});

describe("tombstoned bytes (r5: “It's removed everywhere”)", () => {
  const leftovers = { [`rec/${REC}/master.m3u8`]: "#EXTM3U", [`rec/${REC}/video.mp4`]: "mp4", [`rec/${REC}/poster.jpg`]: "jpg", [`rec/${REC}/teaser.mp4`]: "t", [`rec/${REC}/v0/seg_00000.m4s`]: "s" };

  it("are never served, though an interrupted delete or a late segment left them", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "deleted", deletedAt: "2026-10-02T11:30:00Z" }), ...leftovers } });
    for (const file of Object.keys(leftovers).map((k) => k.slice(`rec/${REC}/`.length))) {
      for (const method of ["GET", "HEAD"]) {
        const res = await get(env, `/m/${REC}/${file}`, { method });
        expect(res.status, `${method} ${file}`).toBe(404);
        expect(await res.text()).toBe("");
      }
    }
    expect((await get(env, `/m/${REC}/v0/seg_00000.m4s`, { headers: { range: "bytes=0-0" } })).status).toBe(404);
  });

  it("are served while the record is live or not yet written", async () => {
    for (const objects of [{ [metaKey(REC)]: live(), ...leftovers }, leftovers]) {
      const res = await get(space({ objects }), `/m/${REC}/master.m3u8`);
      expect(res.status).toBe(200);
      expect(await res.text()).toBe("#EXTM3U");
    }
  });

  it("fail closed when the record can't be read", async () => {
    const env = space({ objects: { [metaKey(REC)]: live(), ...leftovers } });
    const real = env.BUCKET.get;
    env.BUCKET.get = (k, o) => (k === metaKey(REC) ? Promise.reject(new Error("storage")) : real(k, o));
    expect((await get(env, `/m/${REC}/master.m3u8`)).status).toBe(503);
  });
});

describe("page bodies (§3.8's cap, applied to the page POSTs)", () => {
  const post = (env, path, body, headers = {}) => get(env, path, { method: "POST", headers: { origin: ORIGIN, cookie: ownerCookie(), "content-type": "application/json", ...headers }, body, duplex: "half" });

  it("refuses a declared body past 16 KiB unread, for the title and the mark", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    const big = JSON.stringify({ title: "x".repeat(17 * 1024) });
    expect((await post(env, `/_shroom/title/${REC}`, big)).status).toBe(413);
    expect((await post(env, "/_shroom/mark", JSON.stringify({ on: false, pad: "x".repeat(17 * 1024) }))).status).toBe(413);
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("stops reading an undeclared body at the first chunk past the cap", async () => {
    const env = space({ objects: { [metaKey(REC)]: live() } });
    let pulled = 0;
    const chunk = new TextEncoder().encode("x".repeat(4096));
    const stream = new ReadableStream({ pull(c) { pulled++; if (pulled > 100) c.close(); else c.enqueue(pulled === 1 ? new TextEncoder().encode(`{"title":"`) : chunk); } });
    expect((await post(env, `/_shroom/title/${REC}`, stream)).status).toBe(413);
    expect(pulled).toBeLessThan(10);
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("still takes an ordinary rename, and answers a broken body 400", async () => {
    forgetSecret();
    const env = space({ objects: { [metaKey(REC)]: live() } });
    const ok = await post(env, `/_shroom/title/${REC}`, JSON.stringify({ title: "Where the 502s come from" }));
    expect(ok.status).toBe(200);
    expect((await post(env, `/_shroom/title/${REC}`, "{nope")).status).toBe(400);
  });
});

describe("Copy says “Copied” only when it copied (N1)", () => {
  /** shroomCopy run against a fake browser. */
  function browser({ clipboard, exec }) {
    const calls = { exec: 0, selected: null };
    const document = {
      body: { appendChild() {} },
      createElement: () => ({ value: "", style: {}, setAttribute() {}, select() {}, remove() {} }),
      execCommand: () => { calls.exec++; return exec(); },
      createRange: () => ({ selectNodeContents(n) { this.n = n; } }),
    };
    const window = { getSelection: () => ({ removeAllRanges() {}, addRange(r) { calls.selected = r.n; } }) };
    const navigator = clipboard === undefined ? {} : { clipboard: { writeText: clipboard } };
    const copy = new Function("navigator", "document", "window", `${COPY_JS};return shroomCopy`)(navigator, document, window);
    return { copy, calls };
  }

  it("answers true when the Clipboard API wrote", async () => {
    const { copy, calls } = browser({ clipboard: () => Promise.resolve(), exec: () => true });
    expect(await copy("https://v/x", null)).toBe(true);
    expect(calls.exec).toBe(0);
  });

  it("falls back to the copy command when the Clipboard API refuses", async () => {
    const { copy } = browser({ clipboard: () => Promise.reject(new Error("denied")), exec: () => true });
    expect(await copy("https://v/x", null)).toBe(true);
  });

  it("answers false when nothing copied, and selects the visible text for ⌘C", async () => {
    const el = { id: "cmd" };
    const denied = browser({ clipboard: () => Promise.reject(new Error("denied")), exec: () => false });
    expect(await denied.copy("curl …", el)).toBe(false);
    expect(denied.calls.selected).toBe(el);
    const none = browser({ clipboard: undefined, exec: () => { throw new Error("no"); } });
    expect(await none.copy("https://v/x", null)).toBe(false);
  });

  it("is the copy door on the owner page — with Copy thumbnail beside it (N46) — and the join page, and the label waits for its answer", async () => {
    const owner = await html(space({ objects: { [metaKey(REC)]: live() } }), `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`function copied(el,label){return function(ok){if(!ok)return;el.textContent=c.copied;`);
    expect(owner).toContain(`shroomCopy(location.origin+"/"+c.id,null).then(copied(b,c.copy))`);
    expect(owner).toContain(`th.copy().then(copied(tb,c.copyThumb))`);
    expect(owner).not.toContain("navigator.clipboard.writeText(u)");
    const { CODE } = await import("./space-fixture.js");
    const join = await html(space(), `/join/${CODE.ani}`);
    expect(join).toContain(`shroomCopy(t,cmd).then(function(ok){if(!ok)return;b.textContent=c.copied`);
    expect(join).toContain("function shroomCopy(");
  });
});

describe("a member's own page gets the same player", () => {
  it("renders the custom player under a member's cookie", async () => {
    forgetSecret();
    const env = space({ objects: { [metaKey(REC)]: live() } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    const body = await html(env, `/${REC}`, { headers: { cookie: cookieFor(MHER, MHER_MAC) } });
    expect(body).toContain(`class="title"`);
    expect(body.match(/<video[^>]*>/)?.[0]).not.toMatch(/\scontrols/);
    expect(body).toContain(`class="pp"`);
  });
});

// The whole-spec audit's upload-truth gaps (#17, #18, #29, #30; DECISIONS B49): the page
// reads exactly what the Mac writes — `progress` while bytes move, the upload line gone only
// once every file it counted is up, and the owner's own chrome before the record's claim lands.
describe("upload truth (B49: progress, the drained gate, the claim first)", () => {
  const draining = (done, total, ageSec = 10) => live({ progress: { done, total, updatedAt: new Date(NOW - ageSec * 1000).toISOString() } });

  it("a live record still sending its second-class files plays, but isn't “Fully live ✓” yet", async () => {
    const env = space({ objects: { [metaKey(REC)]: draining(11, 13) } });
    const owner = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`<video id="v"`);
    expect(owner).toContain(`<span class="up" id="u">${COPY.finishing}</span>`);
    expect(owner).not.toContain(`<span class="up" id="u">${COPY.fullyLive}</span>`);
    expect(owner).toContain(`"live":true,"drained":false`);
    const st = JSON.parse(await html(env, `/_shroom/status/${REC}`));
    expect(st).toMatchObject({ state: "live", drained: false, pct: 84, stalled: false });
  });

  // Was “is “Fully live ✓” the moment the count is reached”: the restyle's page says nothing
  // once the upload drains — the upload line simply goes (Bagrat's call, for DEVIATIONS).
  it("drops the upload line once the count is reached — no “Fully live ✓” on the page", async () => {
    const env = space({ objects: { [metaKey(REC)]: draining(13, 13) } });
    const owner = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`<div class="upl" id="ul" hidden><span class="up" id="u"></span></div>`);
    expect(owner).not.toContain(COPY.fullyLive);
    // Under an open page the poll fades it, with no “Ready ✓” after it (N65 (7)).
    expect(owner).not.toContain("c.ready");
    expect(JSON.parse(await html(env, `/_shroom/status/${REC}`))).toMatchObject({ state: "live", drained: true });
  });

  it("a live record whose last files stopped moving reads stalled to its owner, never “Fully live ✓”", async () => {
    const env = space({ objects: { [metaKey(REC)]: draining(11, 13, STALL_SECONDS + 5) } });
    const owner = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`<span class="up" id="u">${COPY.waitingConnection}</span>`);
  });

  it("an upload whose progress moves is never stalled; one whose updatedAt aged is", async () => {
    const moving = space({ objects: { [metaKey(REC)]: uploading(4, 12, STALL_SECONDS - 5) } });
    expect(JSON.parse(await html(moving, `/_shroom/status/${REC}`))).toEqual({ state: "uploading", pct: 33, stalled: false });
    const aged = space({ objects: { [metaKey(REC)]: uploading(4, 12, STALL_SECONDS + 5) } });
    expect(JSON.parse(await html(aged, `/_shroom/status/${REC}`))).toEqual({ state: "uploading", pct: 33, stalled: true });
  });

  it("the maker's page before its record exists has the owner's chrome — title field, Copy link, the upload line", async () => {
    const env = space();
    await ledger(env, REC, M1, OWNER_AIR.id);
    const owner = await html(env, `/${REC}`, { headers: { cookie: ownerCookie() } });
    expect(owner).toContain(`class="title"`);
    expect(owner).toContain(`placeholder="Untitled — `);
    expect(owner).toContain(`<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button>`);
    expect(owner).toContain(`<span class="up" id="u">${COPY.finishing}</span>`);
    expect(owner).toContain(`"pending":true`);
    // A title typed before the record exists is sent again once it does.
    expect(owner).toContain(`if(r.status===404)c.pend=v`);
    expect(owner).toContain(`if(c.pend&&s.state!=="waiting")send(c.pend)`);
    // A recipient — and an id with no ledger entry — still reads on its way.
    expect(await html(env, `/${REC}`)).toContain(COPY.onItsWay);
    expect(await html(space(), `/${REC}`, { headers: { cookie: ownerCookie() } })).toContain(COPY.onItsWay);
  });
});
