// L1 under workerd (TESTING-PLAN §5; DECISIONS B63; tests-sys #3): a contract subset of the
// Worker suite, run against the real `src/worker.js` inside workerd — Cloudflare's own
// runtime — through miniflare's local mode, with its R2 emulator as the bucket. The unit
// suite runs the same module under Node against a hand-written fake bucket; this proves
// what that can't: the module loads and answers in workerd (behind a one-line entry — see
// the manifest below), Web Crypto there signs what
// Node verifies, and R2's own conditional writes, ranges, HEAD and custom metadata behave
// as the Worker counts on.
//
// Local only: no account, no `wrangler login`, no deploy, no network. Run:
//   cd worker && npm run -s test:workerd
// (node --test; the file isn't a vitest file, so `npx vitest run` doesn't pick it up.)
process.env.WRANGLER_SEND_METRICS = "false";
import { createHash, createHmac } from "node:crypto";
import { readFileSync } from "node:fs";
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { Miniflare } from "miniflare";
import { COPY, IFACE, STALL_SECONDS, VERSION, storageCode } from "../../src/worker.js";

const HOST = "v.myshroom.workers.dev";
const ORIGIN = `https://${HOST}`;
const API = "/_shroom/api";
const ROSTER = ".shroom/registry/roster.json";
const ID = "AbCdEfGhIjKlMnOp";
const GONE_ID = "GoNeGoNeGoNeGoNe";
const PARENT = {
  account_id: "0123456789abcdef0123456789abcdef",
  bucket: "shroom-space",
  key_id: "00112233445566778899aabbccddeeff",
  secret: "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
};
const sha = (s) => createHash("sha256").update(s).digest("hex");
const iso = (ms) => new Date(ms).toISOString();
const rid = (prefix, name) => `${prefix}_${name.padEnd(16, "_").slice(0, 16)}`;
function deviceKey(name) {
  const id = rid("d", name);
  const secret = Buffer.from(createHash("sha256").update(`secret:${id}`).digest()).toString("base64url").slice(0, 43);
  return { id, secret, verifier: sha(secret), bearer: `Bearer ${id}.${secret}` };
}
const OWNER_MAC = deviceKey("OwnerMac");
const NEW_MAC = deviceKey("NewMac");
const ANI_MAC = deviceKey("AniMac");
const RIVAL_MAC = deviceKey("RivalMac");
const CODE = { ani: "AniCodeAniCode00", race: "RaceCodeRaceCod0" };

// The clock is workerd's own (the module's default export reads Date.now()), so the roster
// is dated from this run's now.
const T0 = Date.now();
const HOUR = 3600_000;
const invite = (code, label) => ({
  codeHash: sha(code), kind: "member", label, inviter: "Bagrat", by: OWNER_MAC.id,
  createdAt: iso(T0 - HOUR), expiresAt: iso(T0 + 7 * 24 * HOUR), state: "pending", used: null, endedAt: null,
});
const ROSTER_DOC = {
  v: 1,
  createdAt: iso(T0 - 30 * 24 * HOUR),
  members: { m1: { role: "owner", name: "Bagrat", state: "active", joinedAt: iso(T0 - 30 * 24 * HOUR) } },
  devices: {
    [OWNER_MAC.id]: { member: "m1", name: "MacBook Pro", verifier: OWNER_MAC.verifier, state: "active", joinedAt: iso(T0 - 30 * 24 * HOUR), endedAt: null, via: "enroll", tokenKey: PARENT.key_id, proofAt: iso(T0 - HOUR) },
  },
  invites: { [rid("i", "Ani")]: invite(CODE.ani, "Ani"), [rid("i", "Race")]: invite(CODE.race, "Race") },
};
const metaOf = (id, over = {}) => JSON.stringify({
  schema: 1, id, state: "live", createdAt: iso(T0 - HOUR), title: "Untitled — Oct 2",
  media: { hls: `rec/${id}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, owner: "m1", ...over,
});
// The date the app deploys the module with — read from the app, so the two can't drift.
const COMPATIBILITY_DATE = readFileSync(new URL("../../../app/src-tauri/src/runtime/cloud.rs", import.meta.url), "utf8").match(/const COMPATIBILITY_DATE: &str = "([0-9-]+)";/)[1];
const SEGMENT = new Uint8Array(1000).map((_, i) => i % 256);

let mf;
let bucket;
const fetchW = (path, init = {}) => mf.dispatchFetch(`${ORIGIN}${path}`, init);
const api = (method, route, { body, auth } = {}) => fetchW(`${API}${route}`, {
  method,
  headers: { ...(body === undefined ? {} : { "content-type": "application/json" }), ...(auth ? { authorization: auth } : {}) },
  ...(body === undefined ? {} : { body: typeof body === "string" ? body : JSON.stringify(body) }),
});
const jsonOf = async (res) => JSON.parse(await res.text());
const roster = async () => JSON.parse(await (await bucket.get(ROSTER)).text());
function parentProof(raw, ts = Math.floor(Date.now() / 1000)) {
  const msg = `shroom-parent-v1\nPOST\n${HOST}\n${API}/enroll\n${ts}\n${sha(raw)}`;
  return `ShroomParent ${ts}.${createHmac("sha256", Buffer.from(PARENT.secret, "utf8")).update(msg).digest("base64url")}`;
}

before(async () => {
  const src = new URL("../../src/", import.meta.url).pathname;
  mf = new Miniflare({
    // Loopback only, a port the OS picks: nothing listens beyond this Mac.
    host: "127.0.0.1",
    port: 0,
    logRequests: false,
    // No request.cf fetched from Cloudflare: the run makes no network call at all.
    cf: false,
    workers: [{
      config: {
        name: "shroom",
        // The date the app deploys with (app/src-tauri/src/runtime/cloud.rs COMPATIBILITY_DATE).
        compatibilityDate: COMPATIBILITY_DATE,
        // The module byte for byte as the app uploads it, behind a one-line entry: workerd
        // refuses a main module whose named exports aren't handlers (COPY, VERSION, …),
        // which Cloudflare's upload accepts (DECISIONS B63 — open for Bagrat).
        manifest: {
          mainModule: "entry.js",
          modulesRoot: src,
          modules: {
            "entry.js": { type: "esm", contents: 'import w from "./worker.js";\nexport default w;\n' },
            "worker.js": { type: "esm", contents: readFileSync(`${src}worker.js`, "utf8") },
          },
        },
        env: { BUCKET: { type: "r2", name: PARENT.bucket }, R2_PARENT: { type: "text", value: JSON.stringify(PARENT) } },
      },
    }],
  });
  bucket = await mf.getR2Bucket("BUCKET");
  await bucket.put(".shroom/cookie.key", Buffer.alloc(32, 7).toString("base64url"));
  await bucket.put(ROSTER, JSON.stringify(ROSTER_DOC));
  await bucket.put(".shroom/player/hls.min.js", readFileSync(new URL("../../vendor/hls.min.js", import.meta.url)));
  await bucket.put(`meta/${ID}.json`, metaOf(ID));
  await bucket.put(`rec/${ID}/master.m3u8`, "#EXTM3U\n");
  await bucket.put(`rec/${ID}/v0/seg_00000.m4s`, SEGMENT);
  await bucket.put(`meta/${GONE_ID}.json`, JSON.stringify({ schema: 1, id: GONE_ID, state: "deleted", title: "Secret plans" }));
  await bucket.put(`rec/${GONE_ID}/v0/seg_00000.m4s`, SEGMENT);
});
after(async () => { await mf?.dispose(); });

describe("the page under workerd", () => {
  it("answers the reachability probe as a shroom space, this VERSION, uncached", async () => {
    const res = await fetchW("/_shroom/ping");
    assert.equal(res.status, 200);
    assert.equal(res.headers.get("cache-control"), "no-store");
    assert.deepEqual(await jsonOf(res), { shroom: true, iface: IFACE, version: VERSION });
  });

  it("never 404s an unknown id; plays a live one; a tombstone is a plain not found", async () => {
    const unknown = await fetchW("/ZzZzZzZzZzZzZzZz");
    assert.equal(unknown.status, 200);
    assert.ok((await unknown.text()).includes(COPY.onItsWay));
    const live = await (await fetchW(`/${ID}`)).text();
    assert.ok(live.includes(`/m/${ID}/master.m3u8`), "the player's playlist");
    const goneRes = await fetchW(`/${GONE_ID}`);
    assert.equal(goneRes.status, 404, "a removed video is a plain not found (B141)");
    const gone = await goneRes.text();
    assert.ok(gone.includes(COPY.notFound) && !gone.includes("Secret plans"));
  });

  // DECISIONS B94: the unknown id carries its own stall clock, and R2's emulator throws its
  // binding errors in the shape the page reads a code from (Cloudflare's documented one).
  it("an unknown id carries the tab's stall clock; R2's binding errors carry their code", async () => {
    const unknown = await (await fetchW("/ZzZzZzZzZzZzZzZz")).text();
    assert.ok(unknown.includes("shroom-seen:") && unknown.includes(String(STALL_SECONDS * 1000)), "the clock rides the page");
    const live = await (await fetchW(`/${ID}`)).text();
    assert.ok(!live.includes("shroom-seen:"));
    await assert.rejects(bucket.get("meta/" + "x".repeat(1100)), (e) => storageCode(e) === 10020);
  });

  // DEVIATIONS #41 (Bagrat, on his own page: “unchecking made with shroom does not hide it
  // even after refresh”): the whole round in the runtime a space runs — a key becomes the
  // cookie under workerd's Web Crypto, the page's own write lands in R2, and the next load
  // reads it back for the owner, a recipient, the embed and the app's Settings.
  it("the owner switches the mark off on the page: it is gone on the next load for everyone, the owner too — and back the same way", async () => {
    const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk";
    await bucket.put(`.shroom/keys/${sha(K)}`, JSON.stringify({ member: "m1", device: OWNER_MAC.id, exp: iso(Date.now() + 600_000) }));
    const redeemed = await fetchW(`/${ID}?k=${K}`, { redirect: "manual" });
    assert.equal(redeemed.status, 302);
    const cookie = (redeemed.headers.get("set-cookie") ?? "").split(";")[0];
    assert.match(cookie, /^shroom_owner=[^;]+$/, "the key became the owner's cookie");
    await redeemed.arrayBuffer();

    /** Where the switch stands, whether the logo shows, and whether the foot itself shows,
     * on a page's foot (null: no foot). */
    const footOf = (body) => {
      const foot = body.match(/<div class="foot"[^>]*>[\s\S]*?<\/div>/)?.[0] ?? null;
      if (!foot) return null;
      const box = foot.match(/<input type="checkbox"[^>]*>/)?.[0] ?? null;
      const logo = foot.match(/<a class="mark"[^>]*>/)?.[0] ?? null;
      const open = foot.match(/<div class="foot"[^>]*>/)[0];
      return { switch: box ? (/\schecked/.test(box) ? "on" : "off") : null, logo: logo ? (/\shidden/.test(logo) ? "hidden" : "shown") : null, foot: /\shidden/.test(open) ? "hidden" : "shown" };
    };
    const owner = async () => {
      const res = await fetchW(`/${ID}`, { headers: { cookie } });
      assert.equal(res.headers.get("cache-control"), "no-store", "the page is never served stale");
      return footOf(await res.text());
    };
    const recipient = async () => footOf(await (await fetchW(`/${ID}`)).text());
    const embedMark = async () => (await (await fetchW(`/${ID}?embed=1`)).text()).includes(`class="mark"`);
    const me = async () => (await jsonOf(await api("GET", "/me", { auth: OWNER_MAC.bearer }))).owner.mark;

    assert.deepEqual(await owner(), { switch: "on", logo: "shown", foot: "shown" });
    assert.deepEqual(await recipient(), { switch: null, logo: "shown", foot: "shown" });

    // The page's own write: same origin, the cookie, {"on":false}.
    const off = await fetchW("/_shroom/mark", { method: "POST", headers: { "content-type": "application/json", origin: ORIGIN, cookie }, body: JSON.stringify({ on: false }) });
    assert.equal(off.status, 200);
    assert.equal(off.headers.get("cache-control"), "no-store");
    assert.deepEqual(await jsonOf(off), { on: false });
    assert.equal(JSON.parse(await (await bucket.get(".shroom/mark.json")).text()).on, false, "the setting is in the space");

    assert.deepEqual(await owner(), { switch: "off", logo: "hidden", foot: "hidden" }, "the owner's own page: nothing in the corner — the switch, its words and the logo all gone");
    assert.equal(await recipient(), null, "a recipient's page carries no mark");
    assert.equal(await embedMark(), false, "nor the embed");
    assert.equal(await me(), false, "Settings reads the same setting");

    // Back on from Settings (the app's door, §4.16): the page follows on its next load.
    const on = await api("POST", "/mark", { auth: OWNER_MAC.bearer, body: { on: true } });
    assert.equal(on.status, 200);
    await on.arrayBuffer();
    assert.deepEqual(await owner(), { switch: "on", logo: "shown", foot: "shown" });
    assert.deepEqual(await recipient(), { switch: null, logo: "shown", foot: "shown" });
    assert.equal(await embedMark(), true);
    assert.equal(await me(), true);
  });
});

describe("media over R2's own reads", () => {
  it("serves a file with its type and length, HEAD without a body", async () => {
    const res = await fetchW(`/m/${ID}/v0/seg_00000.m4s`);
    assert.equal(res.status, 200);
    assert.equal(res.headers.get("content-type"), "video/iso.segment");
    assert.equal((await res.arrayBuffer()).byteLength, 1000);
    const head = await fetchW(`/m/${ID}/v0/seg_00000.m4s`, { method: "HEAD" });
    assert.equal(head.status, 200);
    assert.equal(head.headers.get("content-length"), "1000");
    assert.equal((await head.arrayBuffer()).byteLength, 0);
  });

  it("answers Range — a span and a suffix — with 206 and R2's own offsets", async () => {
    const span = await fetchW(`/m/${ID}/v0/seg_00000.m4s`, { headers: { range: "bytes=10-19" } });
    assert.equal(span.status, 206);
    assert.equal(span.headers.get("content-range"), "bytes 10-19/1000");
    const b = new Uint8Array(await span.arrayBuffer());
    assert.equal(b.length, 10);
    assert.equal(b[0], 10);
    const tail = await fetchW(`/m/${ID}/v0/seg_00000.m4s`, { headers: { range: "bytes=-100" } });
    assert.equal(tail.status, 206);
    assert.equal(tail.headers.get("content-range"), "bytes 900-999/1000");
    assert.equal(new Uint8Array(await tail.arrayBuffer())[0], 900 % 256);
  });

  it("a tombstoned recording's bytes are gone for everyone", async () => {
    const res = await fetchW(`/m/${GONE_ID}/v0/seg_00000.m4s`);
    assert.equal(res.status, 404);
    await res.arrayBuffer();
  });

  it("serves the space's pinned hls.js", async () => {
    const res = await fetchW("/_shroom/hls.js");
    assert.equal(res.headers.get("content-type"), "text/javascript");
    assert.equal(sha(Buffer.from(await res.arrayBuffer())), sha(readFileSync(new URL("../../vendor/hls.min.js", import.meta.url))));
  });
});

// N63 (DECISIONS B195): the edit rides the record, and the module reads it from R2's own
// bytes — the ranges that play onto the page, the copy as it is once it's up, the original
// out of reach behind it and never a word of a voice cut's transcript.
describe("the edit (N63) over R2's own reads", () => {
  const EDIT_ID = "EdItEdItEdItEdIt";
  const SAID = "THE-REMOVED-WORDS";
  const edits = (over = {}) => ({ version: 1, source_duration: 60, in: 2, out: 50, cuts: [{ start: 10, end: 20, kind: "manual", confidence: "high", applied: true, removed: SAID }], rev: 3, ...over });
  const COPY_OF = { rev: 3, file: "edit-3.mp4", duration: 37.9, poster: "edit-3-poster.jpg" };
  const media = { hls: `rec/${EDIT_ID}/master.m3u8`, mp4: `rec/${EDIT_ID}/video.mp4`, poster: `rec/${EDIT_ID}/poster.jpg`, width: 1920, height: 1080 };
  const put = (over) => bucket.put(`meta/${EDIT_ID}.json`, metaOf(EDIT_ID, { durationSec: 60, title: "Edited", media, ...over }));

  before(async () => {
    for (const f of ["master.m3u8", "video.mp4", "poster.jpg", "edit-3.mp4", "edit-3-poster.jpg"]) await bucket.put(`rec/${EDIT_ID}/${f}`, f);
    await bucket.put(`rec/${EDIT_ID}/v0/seg_00000.m4s`, SEGMENT);
  });

  it("plays the original with the ranges that play until the copy is up, with no Download", async () => {
    await put({ edits: edits() });
    const page = await (await fetchW(`/${EDIT_ID}`)).text();
    assert.ok(page.includes(`{"keep":[[2,10],[20,50]],"len":60,"st":"/_shroom/status/${EDIT_ID}","copy":false}`), "the ranges that play");
    assert.ok(page.includes(`<span class="time">0:00 / 0:38</span>`), "the edited length");
    assert.ok(!page.includes(`/d/${EDIT_ID}`) && !page.includes(SAID));
    assert.equal((await fetchW(`/d/${EDIT_ID}`)).status, 404);
    const seg = await fetchW(`/m/${EDIT_ID}/v0/seg_00000.m4s`);
    assert.equal(seg.status, 200, "the original is still reachable");
    await seg.arrayBuffer();
  });

  it("once the copy is up: plays it, Download hands it over, and the original answers 404", async () => {
    await put({ edits: edits({ copy: COPY_OF }) });
    const page = await (await fetchW(`/${EDIT_ID}`)).text();
    assert.ok(page.includes(`"/m/${EDIT_ID}/edit-3.mp4",`) && page.includes(`{"keep":null,"len":60,"st":"/_shroom/status/${EDIT_ID}","copy":true}`));
    assert.ok(page.includes(`poster="/m/${EDIT_ID}/edit-3-poster.jpg"`) && !page.includes(SAID));
    const st = await jsonOf(await fetchW(`/_shroom/status/${EDIT_ID}`));
    assert.deepEqual(st.copy, { src: `/m/${EDIT_ID}/edit-3.mp4`, dur: 37.9, poster: `/m/${EDIT_ID}/edit-3-poster.jpg` });
    const d = await fetchW(`/d/${EDIT_ID}`);
    assert.equal(d.status, 200);
    assert.equal(await d.text(), "edit-3.mp4");
    const part = await fetchW(`/m/${EDIT_ID}/edit-3.mp4`, { headers: { range: "bytes=0-3" } });
    assert.equal(part.status, 206);
    assert.equal(await part.text(), "edit");
    for (const f of ["master.m3u8", "v0/seg_00000.m4s", "video.mp4"]) {
      const res = await fetchW(`/m/${EDIT_ID}/${f}`);
      assert.equal(res.status, 404, f);
      await res.arrayBuffer();
    }
    const poster = await fetchW(`/m/${EDIT_ID}/poster.jpg`);
    assert.equal(poster.status, 200, "the original's first frame stays");
    await poster.arrayBuffer();
  });
});

// B197: a baked recording — `media` names the edit's own files, no stream — read from R2.
describe("a baked recording (B197) over R2's own reads", () => {
  const BAKED_ID = "BaKeBaKeBaKeBaKe";

  before(async () => {
    await bucket.put(`meta/${BAKED_ID}.json`, metaOf(BAKED_ID, { durationSec: 37.9, title: "Baked",
      media: { hls: null, mp4: `rec/${BAKED_ID}/edit-3.mp4`, poster: `rec/${BAKED_ID}/edit-3-poster.jpg`, width: 1920, height: 1080 },
      bakedFrom: [{ at: "2026-10-18T09:00:00.000Z", keep: [[2.125, 10.375], [20, 50]] }] }));
    for (const f of ["edit-3.mp4", "edit-3-poster.jpg", "video.mp4", "master.m3u8"]) await bucket.put(`rec/${BAKED_ID}/${f}`, f);
  });

  it("plays its one file as a live video, hands it over, and serves nothing it doesn't name", async () => {
    const page = await (await fetchW(`/${BAKED_ID}`)).text();
    assert.ok(page.includes(`shroomPlayer(document.querySelector(".player"),"/m/${BAKED_ID}/edit-3.mp4",`), "its own file plays");
    assert.ok(page.includes(`poster="/m/${BAKED_ID}/edit-3-poster.jpg"`) && page.includes(`<span class="time">0:00 / 0:38</span>`));
    assert.ok(!page.includes("10.375") && !page.includes("bakedFrom"), "bakedFrom stays in the record");
    const st = await jsonOf(await fetchW(`/_shroom/status/${BAKED_ID}`));
    assert.equal(st.state, "live");
    assert.equal(st.mp4, true);
    const d = await fetchW(`/d/${BAKED_ID}`);
    assert.equal(d.status, 200);
    assert.equal(await d.text(), "edit-3.mp4");
    const part = await fetchW(`/m/${BAKED_ID}/edit-3.mp4`, { headers: { range: "bytes=0-3" } });
    assert.equal(part.status, 206);
    assert.equal(await part.text(), "edit");
    for (const f of ["video.mp4", "master.m3u8"]) {
      const res = await fetchW(`/m/${BAKED_ID}/${f}`);
      assert.equal(res.status, 404, `${f}: what a bake left behind isn't the video`);
      await res.arrayBuffer();
    }
  });
});

// DECISIONS B109: the art rides inside the module — what workerd decodes and sends is the
// landing's own files, and no object in the bucket stands behind it.
describe("the tab icon, carried in the module", () => {
  const site = (name) => readFileSync(new URL(`../../../site/${name}`, import.meta.url));
  const ICONS = [["/favicon.svg", "image/svg+xml", "favicon.svg"], ["/favicon.png", "image/png", "favicon.png"], ["/favicon.ico", "image/png", "favicon.png"]];

  it("answers its three addresses with the landing's art, and HEAD without a body", async () => {
    for (const [path, type, file] of ICONS) {
      const art = site(file);
      const res = await fetchW(path);
      assert.equal(res.status, 200, path);
      assert.equal(res.headers.get("content-type"), type, path);
      assert.match(res.headers.get("cache-control") ?? "", /\bpublic\b.*max-age=\d{7,}/, path);
      assert.ok(Buffer.from(await res.arrayBuffer()).equals(art), `${path} is site/${file}, byte for byte`);
      const head = await fetchW(path, { method: "HEAD" });
      assert.equal(head.status, 200, path);
      assert.equal(head.headers.get("content-type"), type, path);
      assert.equal(head.headers.get("content-length"), String(art.length), path);
      assert.equal((await head.arrayBuffer()).byteLength, 0, path);
    }
    const listed = await bucket.list({ prefix: "favicon" });
    assert.equal(listed.objects.length, 0, "nothing was placed in storage for it");
  });

  it("is named in the head of a video's page and of the join page", async () => {
    for (const path of [`/${ID}`, "/ZzZzZzZzZzZzZzZz", `/${GONE_ID}`, `/join/${CODE.ani}`, "/join/NoSuchCodeNoSuch"]) {
      const html = await (await fetchW(path)).text();
      const head = html.slice(0, html.indexOf("</head>"));
      for (const link of [`<link rel="icon" type="image/svg+xml" href="/favicon.svg">`, `<link rel="icon" type="image/png" sizes="180x180" href="/favicon.png">`, `<link rel="apple-touch-icon" href="/favicon.png">`]) {
        assert.ok(head.includes(link), `${path}: ${link}`);
      }
    }
  });
});

describe("Region 6 over R2's conditional writes and workerd's Web Crypto", () => {
  it("enrolls a token Mac on a parent proof signed in Node, verified in workerd", async () => {
    const raw = JSON.stringify({ device: { id: NEW_MAC.id, verifier: NEW_MAC.verifier, name: "Mac Studio" }, ownerName: "Bagrat" });
    const res = await api("POST", "/enroll", { body: raw, auth: parentProof(raw) });
    const b = await jsonOf(res);
    assert.equal(res.status, 200, JSON.stringify(b));
    assert.deepEqual(b, { shroom: 1, member: "m1", device: NEW_MAC.id, iface: 2, token: true });
    assert.equal((await roster()).devices[NEW_MAC.id].tokenKey, PARENT.key_id);
    // Another token's key is a stale parent, never an enroll.
    const bad = `ShroomParent ${Math.floor(Date.now() / 1000)}.${createHmac("sha256", "f".repeat(64)).update("x").digest("base64url")}`;
    const refused = await api("POST", "/enroll", { body: raw, auth: bad });
    assert.equal(refused.status, 401);
    await refused.arrayBuffer();
  });

  it("joins once: the invite consumed in one conditional write, a replay identical, another device refused", async () => {
    const body = { code: CODE.ani, device: { id: ANI_MAC.id, verifier: ANI_MAC.verifier, name: null }, previous: null };
    const first = await api("POST", "/join", { body });
    const b = await jsonOf(first);
    assert.equal(first.status, 200, JSON.stringify(b));
    assert.equal(b.role, "member");
    assert.equal(b.space.account_id, PARENT.account_id);
    const r = await roster();
    assert.equal(r.invites[rid("i", "Ani")].state, "used");
    assert.equal(r.devices[ANI_MAC.id].member, b.member);
    const again = await api("POST", "/join", { body });
    assert.deepEqual(await jsonOf(again), b, "the lost reply replays identically");
    const other = await api("POST", "/join", { body: { ...body, device: { id: RIVAL_MAC.id, verifier: RIVAL_MAC.verifier, name: null } } });
    assert.equal(other.status, 409);
    assert.equal((await jsonOf(other)).error, "used");
    // Its bearer now reads who it is.
    const me = await api("GET", "/me", { auth: ANI_MAC.bearer });
    assert.equal(me.status, 200);
    assert.deepEqual((await jsonOf(me)).you, { member: b.member, device: ANI_MAC.id, role: "member", name: null });
  });

  it("two joins racing for one code inside workerd: exactly one lands (R2's etag CAS)", async () => {
    const join = (dev) => api("POST", "/join", { body: { code: CODE.race, device: { id: dev.id, verifier: dev.verifier, name: null }, previous: null } });
    const racers = [deviceKey("RaceA"), deviceKey("RaceB"), deviceKey("RaceC"), deviceKey("RaceD")];
    const answers = await Promise.all(racers.map(join));
    const statuses = await Promise.all(answers.map(async (a) => [a.status, (await jsonOf(a)).error ?? "ok"]));
    const ok = statuses.filter(([s]) => s === 200);
    assert.equal(ok.length, 1, JSON.stringify(statuses));
    for (const [s, e] of statuses.filter(([s]) => s !== 200)) assert.ok((s === 409 && e === "used") || (s === 503 && e === "busy"), `${s} ${e}`);
    const r = await roster();
    const joined = racers.filter((d) => r.devices[d.id]);
    assert.equal(joined.length, 1, "one device in the roster");
    assert.equal(r.invites[rid("i", "Race")].used.device, joined[0].id);
  });

  it("mints a credential Node verifies against the parent, and writes the ledger entry with its metadata", async () => {
    const rec = "MiNtMiNtMiNtMiNt";
    const res = await api("POST", "/mint", { auth: ANI_MAC.bearer, body: { rec, purpose: "upload" } });
    const b = await jsonOf(res);
    assert.equal(res.status, 200, JSON.stringify(b));
    const jwt = Buffer.from(b.sessionToken, "base64").toString("utf8").replace(/^jwt\//, "");
    const [h, c, s] = jwt.split(".");
    assert.equal(createHmac("sha256", Buffer.from(PARENT.secret, "utf8")).update(`${h}.${c}`).digest("base64url"), s, "HS256 under the parent's secret");
    assert.equal(b.secretAccessKey, sha(jwt));
    const claims = JSON.parse(Buffer.from(c, "base64url").toString("utf8"));
    assert.deepEqual(claims.paths, { objectPaths: [`meta/${rec}.json`], prefixPaths: [`rec/${rec}/`] });
    const entry = await bucket.get(`.shroom/registry/recs/${rec}`);
    assert.ok(entry, "the ledger entry");
    assert.equal(entry.customMetadata.m, b.owner, "x-amz-meta-m names the owner");
    const listed = await bucket.list({ prefix: ".shroom/registry/recs/", include: ["customMetadata"] });
    assert.equal(listed.objects.find((o) => o.key.endsWith(rec))?.customMetadata?.m, b.owner, "and R2's list carries it");
  });
});
