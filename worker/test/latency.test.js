// Stop → the owner's page inside TESTING-PLAN §4's 1 s (DECISIONS B59 g): the app opens
// `/<id>?k=` and the browser follows the redirect to the clean page, so every storage round
// trip on that path is time the person waits. Measured here in rounds: a bucket whose every
// call waits for the round it was issued in (calls issued together share one), so reads
// made side by side cost one round and reads made one after another cost one each.
//
// A round closes when the Worker can go no further without storage — not on a clock. It
// once closed 5 ms after its first call, and a loaded runner, slow to finish the cookie's
// HMAC between two reads issued side by side, split one round into two. Now it closes only
// once no WebCrypto work is in flight and a turn of the event loop has passed with nothing
// new issued: whatever joins it could have gone out together, whatever doesn't had to wait.
import { createHash } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { forgetSecret, handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";
import { OWNER_MAC, ROSTER, roster } from "./space-fixture.js";

const ID = "AbCdEfGhIjKlMnOp";
const HOST = "https://v.example.workers.dev";
const NOW = Date.parse("2026-10-02T12:00:00Z");
const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk";
const sha = (s) => createHash("sha256").update(s).digest("hex");
const SECRET = Buffer.alloc(32, 7).toString("base64url");
const META = JSON.stringify({ schema: 1, id: ID, state: "live", createdAt: "2026-10-02T11:00:00Z", title: "Untitled — Oct 2",
  media: { hls: `rec/${ID}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, owner: "m1" });
const GRANT = JSON.stringify({ member: "m1", device: OWNER_MAC.id, exp: new Date(NOW + 600_000).toISOString() });

/** WebCrypto calls still running: the Worker's only work that isn't storage and outlasts a
 * turn of the event loop. A round waits for them; `idle` hears when the last one ends. */
const crypt = { busy: 0, idle: () => {} };
const SUBTLE = ["importKey", "sign", "verify", "digest"];
const subtle = Object.fromEntries(SUBTLE.map((name) => [name, crypto.subtle[name]]));
beforeEach(() => {
  for (const name of SUBTLE) {
    crypto.subtle[name] = (...args) => {
      crypt.busy++;
      return subtle[name].apply(crypto.subtle, args).finally(() => {
        if (--crypt.busy === 0) crypt.idle();
      });
    };
  }
});
afterEach(() => {
  for (const name of SUBTLE) delete crypto.subtle[name];
  crypt.idle = () => {};
});

/** The bucket, paced in rounds: each call waits for the round it joined to close, and a
 * round closes once the Worker is waiting on storage alone. */
function paced(inner) {
  let waiting = [];
  let due = false;
  const seen = { rounds: 0 };
  // After every microtask the calls before it queued: anything issued alongside them has
  // been issued by now, unless WebCrypto still runs — then its end brings us back here.
  const close = () => {
    due = false;
    if (crypt.busy > 0 || !waiting.length) return;
    seen.rounds++;
    const go = waiting;
    waiting = [];
    for (const g of go) g();
  };
  const later = () => {
    if (!due) { due = true; setImmediate(close); }
  };
  crypt.idle = later;
  const wrap = (name) => async (...args) => {
    await new Promise((go) => {
      waiting.push(go);
      later();
    });
    return inner[name](...args);
  };
  const bucket = { ...inner };
  for (const name of ["get", "head", "put", "delete", "list"]) bucket[name] = wrap(name);
  return { bucket, seen };
}

/** The runtime's ctx: what `waitUntil` was handed, finished on demand. */
function ctx() {
  const later = [];
  return { waitUntil: (p) => later.push(p), settle: () => Promise.all(later), later };
}

const space = () => fakeBucket({ ".shroom/cookie.key": SECRET, [ROSTER]: JSON.stringify(roster()), [`meta/${ID}.json`]: META, [`.shroom/keys/${sha(K)}`]: GRANT });

beforeEach(() => forgetSecret());

describe("Stop → the owner's page (B59 g)", () => {
  it("redeems a key in two storage rounds — the reads together, then the spend — and reaps the grant after the reply", async () => {
    const inner = space();
    const { bucket, seen } = paced(inner);
    const c = ctx();
    const res = await handle(new Request(`${HOST}/${ID}?k=${K}`), { BUCKET: bucket }, NOW, c);
    expect(res.status).toBe(302);
    expect(res.headers.get("set-cookie")).toMatch(/^shroom_owner=/);
    expect(seen.rounds).toBe(2);
    // The delete was handed to the runtime, not waited on; it lands once that finishes.
    expect(c.later).toHaveLength(1);
    await c.settle();
    expect(inner.store.has(`.shroom/keys/${sha(K)}`)).toBe(false);
  });

  it("without a runtime ctx (the tests, the local stand-in) the delete is awaited before the reply", async () => {
    const inner = space();
    const res = await handle(new Request(`${HOST}/${ID}?k=${K}`), { BUCKET: inner }, NOW);
    expect(res.status).toBe(302);
    expect(inner.store.has(`.shroom/keys/${sha(K)}`)).toBe(false);
  });

  it("a spent, unknown or expired key still costs one round, and gives no cookie", async () => {
    const inner = space();
    inner.store.set(`.shroom/keys/${sha(K)}`, new TextEncoder().encode(JSON.stringify({ ...JSON.parse(GRANT), spent: "2026-10-02T11:59:00Z" })));
    const { bucket, seen } = paced(inner);
    const res = await handle(new Request(`${HOST}/${ID}?k=${K}`), { BUCKET: bucket }, NOW, ctx());
    expect(res.headers.get("set-cookie")).toBeNull();
    expect(seen.rounds).toBe(1);
  });

  it("reads who the owner is beside the record: the owner's page in two rounds (one once the secret is warm)", async () => {
    const inner = space();
    const cookie = (await handle(new Request(`${HOST}/${ID}?k=${K}`), { BUCKET: inner }, NOW)).headers.get("set-cookie").split(";")[0];
    forgetSecret();
    const cold = paced(inner);
    const body = await (await handle(new Request(`${HOST}/${ID}`, { headers: { cookie } }), { BUCKET: cold.bucket }, NOW, ctx())).text();
    expect(body).toContain('id="t"');
    // The record and the mark beside the cookie's secret, then the roster.
    expect(cold.seen.rounds).toBe(2);
    const warm = paced(inner);
    await handle(new Request(`${HOST}/${ID}`, { headers: { cookie } }), { BUCKET: warm.bucket }, NOW, ctx());
    expect(warm.seen.rounds).toBe(1);
  });
});
