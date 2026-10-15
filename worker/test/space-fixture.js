// A Region 6 space for the Worker's unit tests (WORKER-INTERFACE §2.3's roster, §3's
// credentials): Bagrat (m1) with a token Mac and a token-less MacBook Air, Mher as a
// member, and one invite in every state. Every secret here is a fixture.
import { createHash, createHmac } from "node:crypto";
import { handle } from "../src/worker.js";
import { fakeBucket } from "./fake-bucket.js";

export const HOST = "v.myshroom.workers.dev";
export const ORIGIN = `https://${HOST}`;
/** 2026-10-02T12:00:00Z — the golden vector's `iat`. */
export const NOW = Date.parse("2026-10-02T12:00:00Z");
export const SEC = 1000;
export const HOUR = 3600 * SEC;
export const DAY = 24 * HOUR;
export const iso = (ms) => new Date(ms).toISOString();

export const ROSTER = ".shroom/registry/roster.json";
export const HEALTH = ".shroom/health.json";
export const ledgerKey = (id) => `.shroom/registry/recs/${id}`;
export const goneKey = (id) => `.shroom/gone/${id}`;
export const rateKey = (did) => `.shroom/registry/rate/${did}`;
export const grantKey = (kHash) => `.shroom/keys/${kHash}`;
export const metaKey = (id) => `meta/${id}.json`;

/** The golden vector's parent (shared with crates/shroom-cloud/tests/member_vector.rs). */
export const PARENT = {
  account_id: "0123456789abcdef0123456789abcdef",
  bucket: "shroom-space",
  key_id: "00112233445566778899aabbccddeeff",
  secret: "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
};
/** Another token's derived pair: a parent the Worker doesn't hold. */
export const OTHER_PARENT_SECRET = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

export const COOKIE_KEY_BYTES = Buffer.alloc(32, 7);
export const COOKIE_KEY = COOKIE_KEY_BYTES.toString("base64url");

export const sha = (s) => createHash("sha256").update(s).digest("hex");
export const b64url = (b) => Buffer.from(b).toString("base64url");

/** A registry id: the prefix and 16 base64url characters (§2.3). Padded with "_" so
 * "A1" and "A10" stay distinct. */
export const rid = (prefix, name) => `${prefix}_${name.padEnd(16, "_").slice(0, 16)}`;
/** A recording id (B14): 16 base64url characters. */
export const recId = (n) => `rec${String(n).padStart(13, "0")}`;
export const REC = "AbCdEfGhIjKlMnOp";

/** A device's key, as a Mac makes it before its first call (§3.2). */
export function deviceKey(name) {
  const id = rid("d", name);
  const secret = b64url(createHash("sha256").update(`secret:${id}`).digest()).slice(0, 43);
  return { id, secret, verifier: sha(secret), bearer: `Bearer ${id}.${secret}` };
}

export const M1 = "m1";
export const MHER = rid("m", "Mher");
export const GONE = rid("m", "Gone"); // a removed member
export const OWNER_MAC = deviceKey("OwnerMac"); // m1, holds the token (proofAt fresh)
export const OWNER_AIR = deviceKey("OwnerAir"); // m1, no token
export const MHER_MAC = deviceKey("MherMac");
export const GONE_MAC = deviceKey("GoneMac"); // GONE's device, removed with them
export const NEW_MAC = deviceKey("NewMac"); // a Mac that hasn't joined yet

/** Join codes (16 characters, 96 bits — §3.3) and their invites. */
export const CODE = {
  ani: "AniCodeAniCode00", own: "OwnCodeOwnCode00", old: "OldCodeOldCode00",
  cancelled: "CanCodeCanCode00", used: "UseCodeUseCode00", oldOwn: "OldOwnOldOwn0000",
  unknown: "NoSuchCodeNoSuch",
};
export const INV = {
  ani: rid("i", "Ani"), own: rid("i", "Own"), old: rid("i", "Old"),
  cancelled: rid("i", "Cancelled"), used: rid("i", "Used"), oldOwn: rid("i", "OldOwn"),
};

const invite = (over) => ({
  kind: "member", label: null, inviter: "Bagrat", by: OWNER_MAC.id,
  createdAt: iso(NOW - HOUR), expiresAt: iso(NOW - HOUR + 7 * DAY),
  state: "pending", used: null, endedAt: null, ...over,
});

/** The standard roster. `over` replaces top-level tables wholesale. */
export function roster(over = {}) {
  return {
    v: 1,
    createdAt: iso(NOW - 30 * DAY),
    members: {
      [M1]: { role: "owner", name: "Bagrat", state: "active", joinedAt: iso(NOW - 30 * DAY) },
      [MHER]: { role: "member", label: "Mher", inviter: "Bagrat", state: "active", joinedAt: iso(NOW - 20 * DAY), endedAt: null, invite: INV.used, reinvites: [] },
      [GONE]: { role: "member", label: "Gohar", inviter: "Bagrat", state: "removed", joinedAt: iso(NOW - 25 * DAY), endedAt: iso(NOW - 5 * DAY), invite: rid("i", "GoneInv"), reinvites: [] },
    },
    devices: {
      [OWNER_MAC.id]: { member: M1, name: "MacBook Pro", verifier: OWNER_MAC.verifier, state: "active", joinedAt: iso(NOW - 30 * DAY), endedAt: null, via: "enroll", tokenKey: PARENT.key_id, proofAt: iso(NOW - HOUR) },
      [OWNER_AIR.id]: { member: M1, name: "MacBook Air", verifier: OWNER_AIR.verifier, state: "active", joinedAt: iso(NOW - 10 * DAY), endedAt: null, via: rid("i", "AirInv"), tokenKey: null, proofAt: null },
      [MHER_MAC.id]: { member: MHER, name: null, verifier: MHER_MAC.verifier, state: "active", joinedAt: iso(NOW - 20 * DAY), endedAt: null, via: INV.used, tokenKey: null, proofAt: null },
      [GONE_MAC.id]: { member: GONE, name: null, verifier: GONE_MAC.verifier, state: "removed", joinedAt: iso(NOW - 25 * DAY), endedAt: iso(NOW - 5 * DAY), via: rid("i", "GoneInv"), tokenKey: null, proofAt: null },
    },
    invites: {
      [INV.ani]: { codeHash: sha(CODE.ani), ...invite({ label: "Ani" }) },
      [INV.own]: { codeHash: sha(CODE.own), ...invite({ kind: "own_mac", createdAt: iso(NOW - 10 * 60 * SEC), expiresAt: iso(NOW + 50 * 60 * SEC) }) },
      [INV.old]: { codeHash: sha(CODE.old), ...invite({ label: "Old", createdAt: iso(NOW - 9 * DAY), expiresAt: iso(NOW - 2 * DAY) }) },
      [INV.oldOwn]: { codeHash: sha(CODE.oldOwn), ...invite({ kind: "own_mac", createdAt: iso(NOW - 3 * HOUR), expiresAt: iso(NOW - 2 * HOUR) }) },
      [INV.cancelled]: { codeHash: sha(CODE.cancelled), ...invite({ label: "Cat", state: "cancelled", endedAt: iso(NOW - DAY) }) },
      [INV.used]: { codeHash: sha(CODE.used), ...invite({ label: "Mher", createdAt: iso(NOW - 21 * DAY), expiresAt: iso(NOW - 14 * DAY), state: "used", used: { at: iso(NOW - 20 * DAY), member: MHER, device: MHER_MAC.id, verifier: MHER_MAC.verifier } }) },
    },
    ...over,
  };
}

/** A space: the bucket (cookie key, roster, any extra objects) and the R2_PARENT binding. */
export function space({ roster: r = roster(), objects = {}, parent = PARENT } = {}) {
  const BUCKET = fakeBucket({ ".shroom/cookie.key": COOKIE_KEY, ...(r === null ? {} : { [ROSTER]: typeof r === "string" ? r : JSON.stringify(r) }), ...objects });
  /** @type {any} */
  const env = { BUCKET };
  if (parent) env.R2_PARENT = JSON.stringify(parent);
  return env;
}

/** A meta record, as a recording Mac claims it. */
export const metaOf = (id, over = {}) => JSON.stringify({
  schema: 1, id, state: "live", createdAt: "2026-10-02T11:00:00Z", title: "Untitled — Oct 2",
  media: { hls: `rec/${id}/master.m3u8`, mp4: null, width: 1920, height: 1080 }, owner: M1, ...over,
});

/** A ledger entry (§2.4), as the Worker or a token Mac writes it. */
export async function ledger(env, id, member, device = null) {
  await env.BUCKET.put(ledgerKey(id), JSON.stringify({ v: 1, member, device, at: iso(NOW - HOUR) }), { customMetadata: { m: member } });
}

/** One request to the Worker. `body` objects are sent as JSON. */
export function call(env, method, path, { body, auth, headers = {}, now = NOW, host = HOST } = {}) {
  const init = { method, headers: { ...headers } };
  if (body !== undefined) {
    init.body = typeof body === "string" ? body : JSON.stringify(body);
    init.headers["content-type"] ??= "application/json";
  }
  if (auth) init.headers.authorization = auth;
  return handle(new Request(`https://${host}${path}`, init), env, now);
}

/** The JSON body of a response (null when it isn't JSON). */
export async function bodyOf(res) {
  const text = await res.clone().text();
  try { return JSON.parse(text); } catch { return null; }
}

/** Parent proof (§3.2): HMAC under the parent's S3 secret over the domain-separated
 * request — method, host, path, timestamp and the body's hash. */
export function parentProof({ method = "POST", path = "/_shroom/api/enroll", body = "", ts = Math.floor(NOW / 1000), secret = PARENT.secret, host = HOST, prefix = "shroom-parent-v1\n" } = {}) {
  const raw = typeof body === "string" ? body : JSON.stringify(body);
  const msg = `${prefix}${method}\n${host}\n${path}\n${ts}\n${sha(raw)}`;
  return `ShroomParent ${ts}.${createHmac("sha256", Buffer.from(secret, "utf8")).update(msg).digest("base64url")}`;
}

/** An owner cookie, signed as the Worker signs it (B25: base64url JSON + HMAC under
 * .shroom/cookie.key). Claims are the amended `{m, d, t}` unless a test forges others. */
export function cookie(claims) {
  const payload = b64url(JSON.stringify(claims));
  const sig = createHmac("sha256", COOKIE_KEY_BYTES).update(payload).digest("base64url");
  return `shroom_owner=${payload}.${sig}`;
}
export const cookieFor = (member, device) => cookie({ m: member, d: device.id ?? device, t: Math.floor(NOW / 1000) - 60 });

/** The claims inside a set-cookie header (null when none was set). */
export function cookieClaims(setCookie) {
  if (!setCookie) return null;
  const value = setCookie.split(";")[0].split("=").slice(1).join("=");
  return JSON.parse(Buffer.from(value.split(".")[0], "base64url").toString("utf8"));
}

/** Runs `other` (another request, start to finish) inside the first put to `key` — after
 * this request read the object, before its conditional write lands. Deterministic
 * interleaving at the CAS. */
export function raceAtPut(bucket, key, other) {
  let fired = false;
  const prev = bucket.hooks.beforePut;
  bucket.hooks.beforePut = async (k, v, o) => {
    if (!fired && k === key) {
      fired = true;
      await other();
    }
    if (prev) await prev(k, v, o);
  };
}

/** The visible text of a page: block tags become spaces, inline tags vanish (so a line
 * with a <b> or <kbd> inside still reads whole), entities decoded, spaces collapsed. */
export function textOf(html) {
  return html
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<\/?(p|div|h[1-6]|li|ul|ol|br|hr|section|main|header|footer|nav|button|pre|label|input|textarea|title|head|body|html|meta|link|svg|figure|figcaption|aside|article|table|tr|td|th)\b[^>]*>/gi, " ")
    .replace(/<[^>]+>/g, "")
    .replace(/&nbsp;/g, " ")
    .replace(/&#(\d+);/g, (_, n) => String.fromCodePoint(Number(n)))
    .replace(/&#x([0-9a-f]+);/gi, (_, n) => String.fromCodePoint(parseInt(n, 16)))
    .replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")
    .trim();
}

/** The decoded text inside <title>. */
export function titleOf(html) {
  const m = html.match(/<title>([\s\S]*?)<\/title>/i);
  return m ? textOf(m[1]) : null;
}

export const UA = {
  mac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
  macChrome: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
  iphone: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
  ipad: "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
  android: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Mobile Safari/537.36",
  windows: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
  linux: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
  chromebook: "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36",
};
