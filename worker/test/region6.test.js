// Region 6's Worker, against docs/WORKER-INTERFACE.md (provisional) and spec/machine.json:
// every new route's request, response and error classes (§4), the registry's CAS (§2),
// the credentials (§3), the join page's faces with their lines verbatim (§4.3, r6), and
// §7.1's L1 list in full. Numbers in the test names are §7.1's.
//
// Written before the code (CLAUDE.md: spec tests first). Every test here fails until
// Region 6's Worker lands, because the routes it drives don't exist yet.
import { createHash, createHmac } from "node:crypto";
import { SYSREQ, sysreqLine } from "./sysreq.js";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { COPY, escapeHtml } from "../src/worker.js";
import {
  CODE, DAY, GONE, GONE_MAC, HEALTH, HOST, HOUR, INV, M1, MHER, MHER_MAC, NEW_MAC, NOW, ORIGIN,
  OTHER_PARENT_SECRET, OWNER_AIR, OWNER_MAC, PARENT, REC, ROSTER, SEC, UA, COOKIE_KEY, bodyOf, call,
  cookie, cookieClaims, cookieFor, deviceKey, grantKey, iso, ledger, ledgerKey, goneKey, metaKey, metaOf,
  parentProof, raceAtPut, rateKey, recId, rid, roster, sha, space, textOf, titleOf,
} from "./space-fixture.js";

// ───────── the spec's words (r6, r4), taken verbatim from the JSON ─────────

const machine = JSON.parse(readFileSync(new URL("../../spec/machine.json", import.meta.url), "utf8"));
const region = (n) => ({ states: Object.entries(machine.states).filter(([, s]) => s.region === n).map(([id, s]) => ({ id, ...s })) });
const r6 = region(6);
const r4 = region(4);
const saidOf = (spec, id) => {
  const s = spec.states.find((x) => x.id === id);
  if (!s) throw new Error(`no state ${id}`);
  return s.said;
};
/** The r6 line of `state` that starts with `prefix` — verbatim by construction. */
const pick = (state, prefix, spec = r6) => {
  const l = saidOf(spec, state).find((x) => x.text.startsWith(prefix));
  if (!l) throw new Error(`${state}: no line starting ${prefix}`);
  return l.text;
};
const allR4 = r4.states.flatMap((s) => s.said.map((l) => l.text));
const r4line = (prefix) => {
  const t = allR4.find((x) => x.startsWith(prefix));
  if (!t) throw new Error(`r4: no line starting ${prefix}`);
  return t;
};

const SPEC_CODE = "v.myshroom.workers.dev/join/7xk2pm";
const withCode = (text, code) => text.replace(SPEC_CODE, `${HOST}/join/${code}`);
// The system-requirements line is ratified as a shape with facts to fill (r6 note); the page
// says it with the facts that ship (test/sysreq.js, as the landing's test does).
const filled = (text) => (SYSREQ.test(text) ? sysreqLine : text);
// The copy table's lines are one case each (the records unreadable, …), checked where that case is.
const everyCase = (lines) => lines.filter((l) => !(l.cites ?? []).includes("copy table"));
const LANDING = (code) => everyCase(saidOf(r6, "join-landing")).map((l) => ({ ...l, text: filled(withCode(l.text, code)) }));

const W = {
  hero: pick("join-landing", "Bagrat invited you"),
  sevenDays: pick("join-landing", "This invite is for one person"),
  ownerNote: pick("join-landing-owner-view", "This is the page Ani will see"),
  elsewhere: pick("join-landing-elsewhere", "shroom runs on a Mac"),
  copyLink: pick("join-landing-elsewhere", "Copy link"),
  ownHero: pick("join-landing-own-mac", "Add this Mac"),
  ownSame: pick("join-landing-own-mac", "Same videos, same members"),
  ownHour: pick("join-landing-own-mac", "This link is for one Mac"),
  expired: pick("join-expired-page", "This invite has expired"),
  unknown: pick("join-expired-page", "This invite link doesn't work"),
  ownExpired: pick("join-expired-page", "This link has expired"),
  used: pick("join-used-page", "This invite was already used"),
  youreIn: pick("join-used-page", "You're in"),
  mark: pick("join-landing", "made with shroom"),
  // §4.3: new copy (open question 11) — the transient face, never "ask for a fresh one".
  transient: "This invite link doesn't work right now — try again a little later.",
  // r4 (N40, §4.18): the owner page's fourth upload face.
  waitingSpace: r4line("Waiting for your space — your link already works."),
  waitingBagrat: r4line("Waiting for Bagrat's space — your link already works."),
};
const GET_SHROOM_BLOCK = (code) => LANDING(code).filter((l) => ![W.hero, W.sevenDays].includes(l.text) && !l.text.startsWith("shroom records your screen"));
const INSTALL = (code) => `curl -fsSL https://shroom.am/install | sh -s -- --join ${HOST}/join/${code}`;
// The Homebrew join line (N50, N51): the link whole, https:// and all.
const BREW = (code) => `brew install --cask bagrat/shroom/shroom && open -a ~/Applications/shroom.app "https://${HOST}/join/${code}"`;

// ───────── helpers ─────────

const API = "/_shroom/api";
const api = (env, method, route, opts) => call(env, method, `${API}${route}`, opts);
async function expectError(res, status, error, extra = {}) {
  expect(res.status, `${status} ${error}`).toBe(status);
  expect(res.headers.get("content-type") ?? "").toMatch(/^application\/json/);
  expect(await bodyOf(res)).toMatchObject({ shroom: 1, error, ...extra });
}
async function expectOk(res, status = 200) {
  const b = await bodyOf(res);
  expect(res.status, JSON.stringify(b)).toBe(status);
  expect(b).toMatchObject({ shroom: 1 });
  return b;
}
const rosterPuts = (env) => env.BUCKET.ops.filter((o) => o.op === "put" && o.key === ROSTER);
const stored = (env) => env.BUCKET.json(ROSTER);
const joinBody = (code, dev, { name = null, previous = null } = {}) => ({ code, device: { id: dev.id, verifier: dev.verifier, name }, previous });
const enrollBody = (dev, name = "MacBook Pro", ownerName = "Bagrat") => ({ device: { id: dev.id, verifier: dev.verifier, name }, ownerName });
function enroll(env, body, { ts, secret, signBody, path = `${API}/enroll`, prefix } = {}) {
  const raw = JSON.stringify(body);
  return call(env, "POST", `${API}/enroll`, {
    body: raw,
    auth: parentProof({ path, body: signBody ?? raw, ts, secret, prefix }),
  });
}
const page = (env, path, { ua = UA.mac, cookie: c, method = "GET" } = {}) =>
  call(env, method, path, { headers: { "user-agent": ua, ...(c ? { cookie: c } : {}) } });
const mint = (env, dev, rec = REC, purpose = "upload", extra = {}) => api(env, "POST", "/mint", { auth: dev.bearer, body: { rec, purpose, ...extra } });
const jwtOf = (b) => Buffer.from(b.sessionToken, "base64").toString("utf8").replace(/^jwt\//, "");
const claimsOf = (jwt) => JSON.parse(Buffer.from(jwt.split(".")[1], "base64url").toString("utf8"));

/** The golden vector — the same inputs and bytes as crates/shroom-cloud/tests/member_vector.rs. */
const GOLDEN = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9."
  + "eyJhdWQiOiIwMTIzNDU2Nzg5YWJjZGVmMDEyMzQ1Njc4OWFiY2RlZi5yMi5jbG91ZGZsYXJlc3RvcmFnZS5jb20iLCJidWNrZXQiOiJzaHJvb20tc3BhY2UiLCJleHAiOjE3OTE1NDcyMDAsImlhdCI6MTc5MDk0MjQwMCwiaXNzIjoiMDAxMTIyMzM0NDU1NjY3Nzg4OTlhYWJiY2NkZGVlZmYiLCJwYXRocyI6eyJvYmplY3RQYXRocyI6WyJtZXRhL0FiQ2RFZkdoSWpLbE1uT3AuanNvbiJdLCJwcmVmaXhQYXRocyI6WyJyZWMvQWJDZEVmR2hJaktsTW5PcC8iXX0sInNjb3BlIjoib2JqZWN0LXJlYWQtd3JpdGUiLCJzdWIiOiIwMTIzNDU2Nzg5YWJjZGVmMDEyMzQ1Njc4OWFiY2RlZiJ9."
  + "SjlFTu38wdAOOr-s_PB-X6_Qy8vwi7sf2DhE4c2TNA4";

/** A request descriptor for the batteries below: every new route, with a working body. */
const ROUTES = [
  { name: "peek", method: "POST", route: "/peek", body: { code: CODE.ani } },
  { name: "join", method: "POST", route: "/join", body: joinBody(CODE.ani, NEW_MAC) },
  { name: "me", method: "GET", route: "/me", auth: MHER_MAC },
  { name: "mint", method: "POST", route: "/mint", auth: MHER_MAC, body: { rec: REC, purpose: "upload" } },
  { name: "key", method: "POST", route: "/key", auth: MHER_MAC, body: { kHash: sha("k".repeat(43)) } },
  { name: "library", method: "POST", route: "/library", auth: MHER_MAC, body: { scope: "mine", known: {}, knownThrough: null, after: null } },
  { name: "members", method: "GET", route: "/members", auth: OWNER_AIR },
  { name: "invites", method: "POST", route: "/invites", auth: OWNER_AIR, body: { id: rid("i", "Zed"), codeHash: sha("ZedCodeZedCode00"), kind: "member", label: "Zed", inviter: "Bagrat" } },
  { name: "cancel", method: "DELETE", route: `/invites/${INV.ani}`, auth: OWNER_AIR },
  { name: "remove member", method: "DELETE", route: `/members/${MHER}`, auth: OWNER_AIR },
  { name: "remove device", method: "DELETE", route: `/devices/${OWNER_AIR.id}`, auth: OWNER_MAC },
  { name: "leave", method: "DELETE", route: "/devices/self", auth: MHER_MAC },
  { name: "mark", method: "POST", route: "/mark", auth: OWNER_AIR, body: { on: false } },
  { name: "health", method: "POST", route: "/health", auth: MHER_MAC, body: { rec: REC, uploads: "refused" } },
];
const send = (env, r, extra = {}) => api(env, r.method, r.route, { auth: r.auth?.bearer, body: r.body, ...extra });

// ════════════════════════════════════════════════════════════════════════════════════
// §4.2 · #26 — ping
// ════════════════════════════════════════════════════════════════════════════════════

describe("ping, amended (§4.2, #26)", () => {
  it("answers iface 2 and the deploying app's version, uncached, with no storage read", async () => {
    const env = space();
    const res = await call(env, "GET", "/_shroom/ping");
    expect(res.status).toBe(200);
    expect(res.headers.get("cache-control")).toBe("no-store");
    const b = await bodyOf(res);
    expect(b).toEqual({ shroom: true, iface: 2, version: expect.stringMatching(/^\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?$/) });
    expect(env.BUCKET.ops).toEqual([]);
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4 · §3.8 — the shape rule, Origin, bodies
// ════════════════════════════════════════════════════════════════════════════════════

describe("the shape rule (§4, U2)", () => {
  it.each(ROUTES.map((r) => [r.name, r]))("%s answers JSON carrying \"shroom\":1, success or error", async (_, r) => {
    for (const auth of [r.auth, deviceKey("Stranger"), null]) {
      const env = space();
      const res = await api(env, r.method, r.route, { auth: auth?.bearer, body: r.body });
      if (res.status === 204) continue;
      expect(res.headers.get("content-type") ?? "", `${r.name} ${res.status}`).toMatch(/^application\/json/);
      const b = await bodyOf(res);
      expect(b?.shroom, `${r.name} ${res.status}`).toBe(1);
      if (res.status >= 400) expect(typeof b.error).toBe("string");
    }
  });

  it("a 404 from peek is join-expired only in its own shape: {shroom:1, error:\"unknown\"}", async () => {
    const res = await api(space(), "POST", "/peek", { body: { code: CODE.unknown } });
    expect(res.status).toBe(404);
    expect(await bodyOf(res)).toEqual({ shroom: 1, error: "unknown" });
  });
});

describe("cross-site and abuse limits (§3.8, #14)", () => {
  it.each(ROUTES.map((r) => [r.name, r]))("%s refuses any Origin header — same-origin included — and writes nothing", async (_, r) => {
    for (const origin of [ORIGIN, "https://evil.example", "null"]) {
      const env = space();
      await expectError(await send(env, r, { headers: { origin } }), 403, "origin");
      expect(env.BUCKET.written()).toEqual([]);
    }
  });

  it("takes JSON bodies only, at most 16 KiB (128 KiB for the Library), else 413", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/peek", { body: "{not json" }), 400, "bad-request");
    const pad = (n) => JSON.stringify({ ...joinBody(CODE.ani, NEW_MAC), pad: "x".repeat(n) });
    expect((await api(env, "POST", "/join", { body: pad(17 * 1024) })).status).toBe(413);
    // A full slice — 1,000 cached ids — fits the Library's 128 KiB; past that is 413.
    const lib = (etagLength) => ({ scope: "mine", known: Object.fromEntries(Array.from({ length: 1000 }, (_, i) => [recId(i), "e".repeat(etagLength)])), knownThrough: recId(999), after: null });
    expect(JSON.stringify(lib(60)).length).toBeGreaterThan(17 * 1024);
    expect(JSON.stringify(lib(60)).length).toBeLessThan(128 * 1024);
    expect((await api(env, "POST", "/library", { auth: MHER_MAC.bearer, body: lib(60) })).status).not.toBe(413);
    expect(JSON.stringify(lib(140)).length).toBeGreaterThan(128 * 1024);
    expect((await api(env, "POST", "/library", { auth: MHER_MAC.bearer, body: lib(140) })).status).toBe(413);
    expect(env.BUCKET.written()).toEqual([]);
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §2 · #4 — the registry: one roster, every change one CAS
// ════════════════════════════════════════════════════════════════════════════════════

const zed = { id: rid("i", "Zed"), codeHash: sha("ZedCodeZedCode00"), kind: "member", label: "Zed", inviter: "Bagrat" };

describe("mutate — the roster's compare-and-swap (§2.2, #4)", () => {
  it("writes the roster only on the ETag it read", async () => {
    const env = space();
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    const puts = rosterPuts(env);
    expect(puts).toHaveLength(1);
    const onlyIf = puts[0].options?.onlyIf;
    const etag = typeof onlyIf?.get === "function" ? onlyIf.get("if-match") : onlyIf?.etagMatches;
    expect(String(etag).replace(/^"|"$/g, "")).toBe("e0");
  });

  it("a lost CAS retries from a fresh read — the other writer's change survives", async () => {
    const env = space();
    raceAtPut(env.BUCKET, ROSTER, async () => {
      const other = stored(env);
      other.invites[rid("i", "Other")] = { ...other.invites[INV.ani], codeHash: sha("OtherCodeOther00"), label: "Other" };
      env.BUCKET.touch(ROSTER, JSON.stringify(other));
    });
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    expect(rosterPuts(env)).toHaveLength(2);
    const r = stored(env);
    expect(r.invites[zed.id]).toMatchObject({ label: "Zed", state: "pending" });
    expect(r.invites[rid("i", "Other")]).toMatchObject({ label: "Other" });
  });

  it("five lost CASes → 503 busy, after exactly five conditional writes", async () => {
    const env = space();
    const before = env.BUCKET.text(ROSTER);
    env.BUCKET.hooks.beforePut = async (key) => {
      if (key === ROSTER) env.BUCKET.touch(ROSTER, before); // another writer, every time
    };
    await expectError(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 503, "busy");
    expect(rosterPuts(env)).toHaveLength(5);
    expect(stored(env).invites[zed.id]).toBeUndefined();
  }, 15_000);

  it("a precondition that throws instead of answering null is a lost CAS, retried", async () => {
    const env = space();
    env.BUCKET.opts.preconditionThrows = true;
    raceAtPut(env.BUCKET, ROSTER, async () => env.BUCKET.touch(ROSTER, env.BUCKET.text(ROSTER)));
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    expect(stored(env).invites[zed.id]).toBeDefined();
  });

  it("R2 refusing a fast second write to the one key is a lost CAS, retried (C5)", async () => {
    const env = space();
    env.BUCKET.refuse(ROSTER, 2);
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    expect(rosterPuts(env)).toHaveLength(3);
    expect(stored(env).invites[zed.id]).toBeDefined();
  });

  it.each(ROUTES.map((r) => [r.name, r]))("%s: a missing roster → 503 space/unwired, and the Worker never creates it", async (_, r) => {
    const env = space({ roster: null });
    await expectError(await send(env, r), 503, "space", { reason: "unwired" });
    expect(env.BUCKET.store.has(ROSTER)).toBe(false);
    expect(env.BUCKET.written().filter((k) => k.startsWith(".shroom/registry/"))).toEqual([]);
  });

  it.each(ROUTES.map((r) => [r.name, r]))("%s: an unparseable roster → 503 space/broken, left exactly as found", async (_, r) => {
    const env = space({ roster: "{ hand-edited, oops" });
    await expectError(await send(env, r), 503, "space", { reason: "broken" });
    expect(env.BUCKET.text(ROSTER)).toBe("{ hand-edited, oops");
    expect(rosterPuts(env)).toEqual([]);
  });

  it("reads the roster fresh on every request — no isolate cache", async () => {
    const env = space();
    await expectOk(await api(env, "GET", "/me", { auth: MHER_MAC.bearer }));
    const r = stored(env);
    r.devices[MHER_MAC.id].state = "removed";
    env.BUCKET.touch(ROSTER, JSON.stringify(r));
    await expectError(await api(env, "GET", "/me", { auth: MHER_MAC.bearer }), 403, "removed", { why: "removed" });
  });
});

describe("pruning and caps (§2.3, #5)", () => {
  it("prunes inside any mutate, never a record that still backs something on screen (U7)", async () => {
    const r = roster();
    const at = (ms) => iso(NOW - ms);
    const base = r.invites[INV.ani];
    const deadDev = deviceKey("DeadDev");
    const oldDev = deviceKey("OldDev");
    Object.assign(r.devices, {
      [deadDev.id]: { ...r.devices[OWNER_AIR.id], verifier: deadDev.verifier, state: "removed", endedAt: at(181 * DAY) },
      [oldDev.id]: { ...r.devices[OWNER_AIR.id], verifier: oldDev.verifier, state: "left", endedAt: at(179 * DAY) },
    });
    r.members[rid("m", "Ancient")] = { role: "member", label: "Ancient", inviter: "Bagrat", state: "left", joinedAt: at(900 * DAY), endedAt: at(800 * DAY), invite: rid("i", "x"), reinvites: [] };
    Object.assign(r.invites, {
      [rid("i", "PendOld")]: { ...base, codeHash: sha("1"), createdAt: at(400 * DAY), expiresAt: at(393 * DAY) },
      [rid("i", "UsedLive")]: { ...base, codeHash: sha("2"), state: "used", used: { at: at(300 * DAY), member: M1, device: oldDev.id, verifier: oldDev.verifier } },
      [rid("i", "UsedDead")]: { ...base, codeHash: sha("3"), state: "used", used: { at: at(300 * DAY), member: M1, device: deadDev.id, verifier: deadDev.verifier } },
      [rid("i", "Canc31")]: { ...base, codeHash: sha("4"), state: "cancelled", endedAt: at(31 * DAY) },
      [rid("i", "Canc29")]: { ...base, codeHash: sha("5"), state: "cancelled", endedAt: at(29 * DAY) },
      [rid("i", "Own31")]: { ...base, kind: "own_mac", label: null, codeHash: sha("6"), createdAt: at(31 * DAY + HOUR), expiresAt: at(31 * DAY) },
      [rid("i", "Own29")]: { ...base, kind: "own_mac", label: null, codeHash: sha("7"), createdAt: at(29 * DAY + HOUR), expiresAt: at(29 * DAY) },
    });
    const env = space({ roster: r });
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    const after = stored(env);
    expect(Object.keys(after.invites)).toEqual(expect.arrayContaining([rid("i", "PendOld"), rid("i", "UsedLive"), rid("i", "Canc29"), rid("i", "Own29"), zed.id]));
    for (const gone of [rid("i", "UsedDead"), rid("i", "Canc31"), rid("i", "Own31")]) expect(after.invites[gone], gone).toBeUndefined();
    expect(after.devices[deadDev.id]).toBeUndefined();
    expect(after.devices[oldDev.id]).toBeDefined();
    expect(after.members[rid("m", "Ancient")]).toMatchObject({ label: "Ancient" }); // members never go
    expect(after.members[GONE]).toMatchObject({ label: "Gohar" });
  });

  const pendings = (n, { expired = false, kind = "member" } = {}) => Object.fromEntries(Array.from({ length: n }, (_, i) => [rid("i", `P${expired ? "x" : "p"}${kind[0]}${i}`), {
    ...roster().invites[INV.ani], kind, label: kind === "member" ? `P${i}` : null, codeHash: sha(`pend${expired}${kind}${i}`),
    expiresAt: iso(expired ? NOW - DAY : NOW + DAY),
  }]));

  it("caps pending, unexpired invites at 20 — expired ones don't count (U7)", async () => {
    const r = roster();
    Object.assign(r.invites, pendings(18), pendings(30, { expired: true })); // + Ani and the own-Mac link = 20 live
    const env = space({ roster: r });
    await expectError(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 409, "full");
    const fewer = roster();
    Object.assign(fewer.invites, pendings(17), pendings(30, { expired: true }));
    await expectOk(await api(space({ roster: fewer }), "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
  });

  it("caps all invites in the document at 500", async () => {
    // Rows nothing can evict (expired pending member invites — they are Members rows, U7).
    // B37: cancelled and ended rows no longer hold the cap; see the eviction test below.
    const r = roster();
    delete r.invites[INV.cancelled];
    delete r.invites[INV.oldOwn];
    const base = r.invites[INV.old];
    for (let i = Object.keys(r.invites).length; i < 500; i++) r.invites[rid("i", `C${i}`)] = { ...base, codeHash: sha(`c${i}`) };
    await expectError(await api(space({ roster: r }), "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 409, "full");
  });

  it("caps active members at 200 — a join past it is 409 full, and nothing is consumed", async () => {
    const r = roster();
    for (let i = 0; i < 199; i++) r.members[rid("m", `A${i}`)] = { ...r.members[MHER], label: `A${i}` };
    const env = space({ roster: r });
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) }), 409, "full");
    expect(stored(env).invites[INV.ani].state).toBe("pending");
  });

  it("caps m1's active Macs at 20", async () => {
    const r = roster();
    for (let i = 0; i < 18; i++) { const d = deviceKey(`Own${i}`); r.devices[d.id] = { ...r.devices[OWNER_AIR.id], verifier: d.verifier }; }
    await expectError(await api(space({ roster: r }), "POST", "/join", { body: joinBody(CODE.own, NEW_MAC, { name: "Mac mini" }) }), 409, "full");
  });

  it("normalizes what people typed: NFC, control characters stripped, trimmed", async () => {
    const env = space();
    await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: { ...zed, label: "  A\u0301ni\u0007 ", inviter: "Bag\u0000rat\u001b" } }), 201);
    expect(stored(env).invites[zed.id]).toMatchObject({ label: "\u00c1ni", inviter: "Bagrat" });
    await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.own, NEW_MAC, { name: " Mac\u0007 mini e\u0301\u001b" }) }));
    expect(stored(env).devices[NEW_MAC.id].name).toBe("Mac mini \u00e9");
  });

  it("refuses a label over 64 characters or an inviter over 40", async () => {
    for (const body of [{ ...zed, label: "x".repeat(65) }, { ...zed, inviter: "y".repeat(41) }, { ...zed, label: "   " }]) {
      const env = space();
      const res = await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body });
      expect([400, 409]).toContain(res.status); // §4.12 says 400 bad-request; §2.3's cap table says 409 full
      expect((await bodyOf(res)).error).toMatch(/^(bad-request|full)$/);
      expect(rosterPuts(env)).toEqual([]);
    }
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.4 · §4.5 · #6–#10 — the join machine
// ════════════════════════════════════════════════════════════════════════════════════

describe("peek (§4.4, #10)", () => {
  it("answers a pending member invite with the inviter and kind — never the label — and writes nothing", async () => {
    const env = space();
    const res = await api(env, "POST", "/peek", { body: { code: CODE.ani } });
    const b = await expectOk(res);
    expect(b).toEqual({ shroom: 1, state: "pending", kind: "member", inviter: "Bagrat", expiresAt: roster().invites[INV.ani].expiresAt, now: iso(NOW), iface: 2 });
    expect(JSON.stringify(b)).not.toContain("Ani\"");
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("names an own-Mac link's kind", async () => {
    expect(await bodyOf(await api(space(), "POST", "/peek", { body: { code: CODE.own } }))).toMatchObject({ state: "pending", kind: "own_mac" });
  });

  it("derives expired from the clock, and reports cancelled and used — without writing", async () => {
    const env = space();
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: CODE.old } }))).toMatchObject({ shroom: 1, state: "expired" });
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: CODE.oldOwn } }))).toMatchObject({ state: "expired", kind: "own_mac" });
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: CODE.cancelled } }))).toMatchObject({ state: "cancelled" });
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: CODE.used } }))).toMatchObject({ state: "used" });
    // The same pending invite reads expired one second past its time.
    const late = await api(env, "POST", "/peek", { body: { code: CODE.ani }, now: Date.parse(roster().invites[INV.ani].expiresAt) + SEC });
    expect(await bodyOf(late)).toMatchObject({ state: "expired" });
    expect(env.BUCKET.written()).toEqual([]);
    expect(stored(env).invites[INV.old].state).toBe("pending"); // expired is derived, never stored
  });

  it("answers an unknown code 404 unknown, and a missing code 400", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/peek", { body: { code: CODE.unknown } }), 404, "unknown");
    await expectError(await api(env, "POST", "/peek", { body: {} }), 400, "bad-request");
    await expectError(await api(env, "POST", "/peek", { body: { code: 7 } }), 400, "bad-request");
    expect(env.BUCKET.written()).toEqual([]);
  });
});

describe("join (§4.5)", () => {
  it("consumes a member invite in one write: a new member, its device, the invite used", async () => {
    const env = space();
    const res = await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { name: "Ani's MacBook" }) });
    const b = await expectOk(res);
    expect(b).toEqual({
      shroom: 1, kind: "member", role: "member", member: expect.stringMatching(/^m_[A-Za-z0-9_-]{16}$/), device: NEW_MAC.id, inviter: "Bagrat",
      space: { host: HOST, account_id: PARENT.account_id, bucket: PARENT.bucket, label: "v", subdomain: "myshroom", owner_name: "Bagrat", iface: 2 },
    });
    expect(JSON.stringify(b)).not.toMatch(/"Ani"|Ani'?s? MacBook/); // the member never receives its own label
    expect(rosterPuts(env)).toHaveLength(1);
    const r = stored(env);
    expect(r.members[b.member]).toMatchObject({ role: "member", label: "Ani", inviter: "Bagrat", state: "active", joinedAt: iso(NOW), invite: INV.ani });
    expect(r.devices[NEW_MAC.id]).toMatchObject({ member: b.member, verifier: NEW_MAC.verifier, state: "active", via: INV.ani, tokenKey: null });
    expect(r.devices[NEW_MAC.id].name ?? null).toBeNull(); // a sharing name is kept only for own-Mac
    expect(r.invites[INV.ani]).toMatchObject({ state: "used", used: { at: iso(NOW), member: b.member, device: NEW_MAC.id, verifier: NEW_MAC.verifier } });
    expect(JSON.stringify(r)).not.toContain(NEW_MAC.secret);
  });

  it("joins an own-Mac link as m1, owner-class, keeping the Mac's sharing name", async () => {
    const env = space();
    const b = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.own, NEW_MAC, { name: "Mac mini" }) }));
    expect(b).toMatchObject({ kind: "own_mac", role: "owner", member: M1, device: NEW_MAC.id, space: { owner_name: "Bagrat" } });
    expect(stored(env).devices[NEW_MAC.id]).toMatchObject({ member: M1, name: "Mac mini", state: "active", via: INV.own });
  });

  it("refuses unknown, cancelled, expired and used invites in their own classes, consuming and writing nothing", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.unknown, NEW_MAC) }), 404, "unknown");
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.cancelled, NEW_MAC) }), 410, "cancelled");
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.old, NEW_MAC) }), 410, "expired");
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.oldOwn, NEW_MAC) }), 410, "expired");
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.used, NEW_MAC) }), 409, "used");
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("#8 replays a lost reply: the same code, device and verifier get the identical 200, and no second member", async () => {
    const env = space();
    const first = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) }));
    const members = Object.keys(stored(env).members).length;
    const again = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) }));
    expect(again).toEqual(first);
    expect(Object.keys(stored(env).members)).toHaveLength(members);
    // The same device with another verifier is someone else.
    await expectError(await api(env, "POST", "/join", { body: { ...joinBody(CODE.ani, NEW_MAC), device: { id: NEW_MAC.id, verifier: sha("other"), name: null } } }), 409, "used");
  });

  it("answers a device id already in the roster 409 conflict, leaving the invite pending", async () => {
    const env = space();
    const clash = { ...NEW_MAC, id: OWNER_AIR.id };
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.ani, clash) }), 409, "conflict");
    expect(stored(env).invites[INV.ani].state).toBe("pending");
  });

  it("refuses a malformed device id or verifier 400", async () => {
    for (const device of [{ id: "d_short", verifier: NEW_MAC.verifier }, { id: OWNER_AIR.id.replace("d_", "m_"), verifier: NEW_MAC.verifier },
      { id: NEW_MAC.id, verifier: NEW_MAC.verifier.toUpperCase() }, { id: NEW_MAC.id, verifier: NEW_MAC.verifier.slice(1) }, { id: NEW_MAC.id }]) {
      const env = space();
      await expectError(await api(env, "POST", "/join", { body: { code: CODE.ani, device, previous: null } }), 400, "bad-request");
      expect(env.BUCKET.written()).toEqual([]);
    }
  });

  it("#6 single use: two joins with different devices, interleaved at the CAS → exactly one 200, the other 409 used", async () => {
    const env = space();
    const other = deviceKey("OtherMac");
    let second;
    raceAtPut(env.BUCKET, ROSTER, async () => { second = await api(env, "POST", "/join", { body: joinBody(CODE.ani, other) }); });
    const first = await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) });
    expect(second, "the second join ran inside the first one's roster write").toBeDefined();
    const statuses = [first.status, second.status].sort();
    expect(statuses).toEqual([200, 409]);
    const loser = first.status === 409 ? first : second;
    expect(await bodyOf(loser)).toMatchObject({ shroom: 1, error: "used" });
    const r = stored(env);
    expect([r.devices[NEW_MAC.id], r.devices[other.id]].filter(Boolean)).toHaveLength(1);
  });

  it("#7 cancel vs consume — the cancel lands first: the join retries and reads 410 cancelled", async () => {
    const env = space();
    let cancel;
    raceAtPut(env.BUCKET, ROSTER, async () => { cancel = await api(env, "DELETE", `/invites/${INV.ani}`, { auth: OWNER_AIR.bearer }); });
    const join = await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) });
    expect(cancel, "the cancel ran inside the join's roster write").toBeDefined();
    expect(cancel.status).toBe(204);
    await expectError(join, 410, "cancelled");
    expect(stored(env).devices[NEW_MAC.id]).toBeUndefined();
  });

  it("#7 cancel vs consume — the join lands first: the cancel retries and reads 409 used, naming the member", async () => {
    const env = space();
    let join;
    raceAtPut(env.BUCKET, ROSTER, async () => { join = await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) }); });
    const cancel = await api(env, "DELETE", `/invites/${INV.ani}`, { auth: OWNER_AIR.bearer });
    expect(join, "the join ran inside the cancel's roster write").toBeDefined();
    const joined = await expectOk(join);
    await expectError(cancel, 409, "used", { member: joined.member });
    expect(stored(env).invites[INV.ani].state).toBe("used");
  });
});

describe("join with previous — re-invited after removal (§4.5 step 2, #9, H2)", () => {
  const previous = { id: GONE_MAC.id, secret: GONE_MAC.secret };

  it("revives the removed member with its old label; the invite's label goes to reinvites only", async () => {
    const env = space();
    const b = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { previous }) }));
    expect(b).toMatchObject({ member: GONE, role: "member", kind: "member" });
    const r = stored(env);
    expect(r.members[GONE]).toMatchObject({ state: "active", endedAt: null, label: "Gohar", reinvites: [{ invite: INV.ani, label: "Ani", at: iso(NOW) }] });
    expect(r.devices[NEW_MAC.id]).toMatchObject({ member: GONE, state: "active" });
    expect(r.devices[GONE_MAC.id].state).toBe("removed"); // the old secret stays dead
    expect(r.invites[INV.ani]).toMatchObject({ state: "used", used: { member: GONE } });
  });

  it("a previous device still active → 409 already-here, the invite still pending, nothing written", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { previous: { id: MHER_MAC.id, secret: MHER_MAC.secret } }) }), 409, "already-here");
    expect(stored(env).invites[INV.ani].state).toBe("pending");
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("ignores a previous with a wrong secret: a fresh member, the removed one untouched", async () => {
    const env = space();
    const b = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { previous: { id: GONE_MAC.id, secret: MHER_MAC.secret } }) }));
    expect(b.member).not.toBe(GONE);
    expect(stored(env).members[GONE].state).toBe("removed");
  });

  it("never revives m1, never moves a device across members, and an own-Mac link revives nobody", async () => {
    const r = roster();
    r.devices[OWNER_AIR.id] = { ...r.devices[OWNER_AIR.id], state: "removed", endedAt: iso(NOW - DAY) };
    const env = space({ roster: r });
    const b = await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { previous: { id: OWNER_AIR.id, secret: OWNER_AIR.secret } }) }));
    expect(b.member).toMatch(/^m_/);
    expect(b.role).toBe("member");
    expect(stored(env).devices[OWNER_AIR.id].state).toBe("removed");

    const env2 = space();
    const own = await expectOk(await api(env2, "POST", "/join", { body: joinBody(CODE.own, NEW_MAC, { name: "Mac mini", previous }) }));
    expect(own).toMatchObject({ member: M1, role: "owner" });
    expect(stored(env2).members[GONE].state).toBe("removed");
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §3.2 · #13 · #15 — device keys, parent proof, enroll
// ════════════════════════════════════════════════════════════════════════════════════

describe("device key (§3.2, #13)", () => {
  it("an unknown device, a wrong secret or a malformed header → 401 unauthorized", async () => {
    const env = space();
    for (const auth of [deviceKey("Stranger").bearer, `Bearer ${MHER_MAC.id}.${OWNER_AIR.secret}`, `Bearer ${MHER_MAC.id}`, `Bearer ${MHER_MAC.secret}`, `Basic ${MHER_MAC.id}.${MHER_MAC.secret}`, ""]) {
      await expectError(await api(env, "GET", "/me", { auth: auth || undefined }), 401, "unauthorized");
    }
  });

  it("a removed or left device, or a removed or left member → 403 removed with why", async () => {
    const r = roster();
    const left = deviceKey("LeftMac");
    r.devices[left.id] = { ...r.devices[MHER_MAC.id], verifier: left.verifier, state: "left", endedAt: iso(NOW - DAY) };
    const env = space({ roster: r });
    await expectError(await api(env, "GET", "/me", { auth: GONE_MAC.bearer }), 403, "removed", { why: "removed" });
    await expectError(await api(env, "GET", "/me", { auth: left.bearer }), 403, "removed", { why: "left" });
    const r2 = roster();
    r2.members[MHER] = { ...r2.members[MHER], state: "left", endedAt: iso(NOW - DAY) };
    await expectError(await api(space({ roster: r2 }), "GET", "/me", { auth: MHER_MAC.bearer }), 403, "removed", { why: "left" });
    const r3 = roster();
    r3.members[MHER] = { ...r3.members[MHER], state: "removed", endedAt: iso(NOW - DAY) };
    await expectError(await api(space({ roster: r3 }), "POST", "/mint", { auth: MHER_MAC.bearer, body: { rec: REC, purpose: "upload" } }), 403, "removed", { why: "removed" });
  });

  it("takes the role from the roster, never the body", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/invites", { auth: MHER_MAC.bearer, body: { ...zed, role: "owner", member: M1 } }), 403, "forbidden");
    await expectError(await api(env, "POST", "/mark", { auth: MHER_MAC.bearer, body: { on: false, role: "owner" } }), 403, "forbidden");
    expect(env.BUCKET.written()).toEqual([]);
  });
});

describe("enroll and parent proof (§3.2, §4.6, #15)", () => {
  it("enrolls a new Mac under m1 with the proof, stamping proofAt and the parent's key id", async () => {
    const env = space();
    const b = await expectOk(await enroll(env, enrollBody(NEW_MAC, "Mac Studio")));
    expect(b).toEqual({ shroom: 1, member: M1, device: NEW_MAC.id, iface: 2, token: true });
    expect(stored(env).devices[NEW_MAC.id]).toMatchObject({ member: M1, name: "Mac Studio", verifier: NEW_MAC.verifier, state: "active", via: "enroll", tokenKey: PARENT.key_id, proofAt: iso(NOW) });
  });

  it("creates m1 in a freshly seeded roster, and refreshes its name when it changed", async () => {
    const seeded = { v: 1, createdAt: iso(NOW - HOUR), members: {}, devices: {}, invites: {} };
    const env = space({ roster: seeded });
    await expectOk(await enroll(env, enrollBody(OWNER_MAC)));
    expect(stored(env).members[M1]).toMatchObject({ role: "owner", name: "Bagrat", state: "active" });
    await expectOk(await enroll(env, enrollBody(OWNER_MAC, "MacBook Pro", "Bagrat A.")));
    expect(stored(env).members[M1].name).toBe("Bagrat A.");
  });

  it("accepts a clock within 300 s; past it → 401 clock, either way", async () => {
    for (const skew of [-299, 299]) await expectOk(await enroll(space(), enrollBody(NEW_MAC), { ts: Math.floor(NOW / 1000) + skew }));
    for (const skew of [-301, 301]) await expectError(await enroll(space(), enrollBody(NEW_MAC), { ts: Math.floor(NOW / 1000) + skew }), 401, "clock");
  });

  it("another token's key → 401 stale-parent; no binding → 503 space/no-parent", async () => {
    await expectError(await enroll(space(), enrollBody(NEW_MAC), { secret: OTHER_PARENT_SECRET }), 401, "stale-parent");
    await expectError(await enroll(space({ parent: null }), enrollBody(NEW_MAC)), 503, "space", { reason: "no-parent" });
  });

  it("rejects a proof without the domain prefix (a JWT-shaped message), a swapped body and another path", async () => {
    const env = space();
    await expectError(await enroll(env, enrollBody(NEW_MAC), { prefix: "" }), 401, "stale-parent");
    await expectError(await enroll(env, enrollBody(NEW_MAC), { signBody: JSON.stringify(enrollBody(OWNER_AIR)) }), 401, "stale-parent");
    await expectError(await enroll(env, enrollBody(NEW_MAC), { path: `${API}/me` }), 401, "stale-parent");
    // A JWT signing input signed with the parent's secret is no proof either.
    const jwtSig = createHmac("sha256", PARENT.secret).update(GOLDEN.split(".").slice(0, 2).join(".")).digest("base64url");
    const res = await call(env, "POST", `${API}/enroll`, { body: JSON.stringify(enrollBody(NEW_MAC)), auth: `ShroomParent ${Math.floor(NOW / 1000)}.${jwtSig}` });
    await expectError(res, 401, "stale-parent");
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("only ever writes under m1: a member's device or another verifier → 409 conflict", async () => {
    const env = space();
    await expectError(await enroll(env, enrollBody(MHER_MAC)), 409, "conflict");
    await expectError(await enroll(env, { ...enrollBody(OWNER_AIR), device: { id: OWNER_AIR.id, verifier: sha("another"), name: "MacBook Air" } }), 409, "conflict");
    await expectOk(await enroll(env, { ...enrollBody(NEW_MAC), member: MHER, role: "member" }));
    expect(stored(env).devices[NEW_MAC.id].member).toBe(M1);
    expect(stored(env).devices[MHER_MAC.id].member).toBe(MHER);
  });

  it("revives an m1 Mac another owner Mac removed (U8)", async () => {
    const r = roster();
    r.devices[OWNER_AIR.id] = { ...r.devices[OWNER_AIR.id], state: "removed", endedAt: iso(NOW - DAY) };
    const env = space({ roster: r });
    await expectOk(await enroll(env, enrollBody(OWNER_AIR, "MacBook Air")));
    expect(stored(env).devices[OWNER_AIR.id]).toMatchObject({ state: "active", endedAt: null, proofAt: iso(NOW), member: M1 });
  });

  it("writes nothing when proofAt is under 24 hours old and nothing else changed (C5)", async () => {
    const env = space(); // OWNER_MAC proved an hour ago
    const b = await expectOk(await enroll(env, enrollBody(OWNER_MAC, "MacBook Pro")));
    expect(b).toMatchObject({ member: M1, device: OWNER_MAC.id, token: true });
    expect(rosterPuts(env)).toEqual([]);
  });

  it("re-stamps when proofAt is older than 24 hours, or the Mac's name changed", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].proofAt = iso(NOW - 25 * HOUR);
    const env = space({ roster: r });
    await expectOk(await enroll(env, enrollBody(OWNER_MAC, "MacBook Pro")));
    expect(stored(env).devices[OWNER_MAC.id].proofAt).toBe(iso(NOW));
    const env2 = space();
    await expectOk(await enroll(env2, enrollBody(OWNER_MAC, "Bagrat's MacBook Pro")));
    expect(stored(env2).devices[OWNER_MAC.id]).toMatchObject({ name: "Bagrat's MacBook Pro", proofAt: iso(NOW) });
  });

  it("sets the space's uploads back to ok (§4.18)", async () => {
    const env = space({ objects: { [HEALTH]: JSON.stringify({ v: 1, uploads: "blocked", since: iso(NOW - HOUR), why: "no-parent", by: null }) } });
    await expectOk(await enroll(env, enrollBody(NEW_MAC)));
    expect(env.BUCKET.json(HEALTH)).toMatchObject({ uploads: "ok" });
  });

  // The same Mac set up again (DECISIONS B144): a fresh device key after its folder went —
  // Disconnect, an uninstall, by hand — is one Mac, never "MacBook Pro (2)".
  const STAMP = sha("this Mac, this account, this space");
  const stamped = (dev, name = "MacBook Pro", machine = STAMP) => ({ ...enrollBody(dev, name), device: { ...enrollBody(dev, name).device, machine } });

  it("keeps the machine stamp it's given, and adds it to a Mac that had none", async () => {
    const env = space();
    await expectOk(await enroll(env, stamped(OWNER_MAC)));
    expect(stored(env).devices[OWNER_MAC.id].machine).toBe(STAMP);
    await expectOk(await enroll(env, stamped(NEW_MAC, "Mac Studio", sha("another Mac"))));
    expect(stored(env).devices[NEW_MAC.id]).toMatchObject({ machine: sha("another Mac"), state: "active" });
  });

  it("a new key on a stamped Mac retires the old record — it left; nothing else changes", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].machine = STAMP;
    const env = space({ roster: r });
    await expectOk(await enroll(env, stamped(NEW_MAC)));
    const d = stored(env).devices;
    expect(d[OWNER_MAC.id]).toMatchObject({ state: "left", endedAt: iso(NOW) });
    expect(d[NEW_MAC.id]).toMatchObject({ state: "active", machine: STAMP, member: M1 });
    expect(d[OWNER_AIR.id].state).toBe("active");
    expect(d[MHER_MAC.id].state).toBe("active");
  });

  it("a stamp from another Mac, or another account on it, retires nothing", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].machine = STAMP;
    const env = space({ roster: r });
    await expectOk(await enroll(env, stamped(NEW_MAC, "MacBook Pro", sha("the other account"))));
    expect(stored(env).devices[OWNER_MAC.id].state).toBe("active");
  });

  it("a record from before stamps goes by its name — only a token Mac's, quiet for a day", async () => {
    // Bagrat's 0.1.10 reinstall: the old record has no stamp and hasn't proved the token since.
    const r = roster();
    r.devices[OWNER_MAC.id].proofAt = iso(NOW - 25 * HOUR);
    const env = space({ roster: r });
    await expectOk(await enroll(env, stamped(NEW_MAC)));
    expect(stored(env).devices[OWNER_MAC.id]).toMatchObject({ state: "left", endedAt: iso(NOW) });
    // Proved within the day: another Mac with the same name, kept.
    const env2 = space();
    await expectOk(await enroll(env2, stamped(NEW_MAC)));
    expect(stored(env2).devices[OWNER_MAC.id].state).toBe("active");
    // A Mac that came in by an own-Mac link is never taken by its name.
    const env3 = space();
    await expectOk(await enroll(env3, stamped(NEW_MAC, "MacBook Air")));
    expect(stored(env3).devices[OWNER_AIR.id].state).toBe("active");
  });

  it("a token Mac retired by mistake comes back at its next enroll", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].proofAt = iso(NOW - 25 * HOUR);
    const env = space({ roster: r });
    await expectOk(await enroll(env, stamped(NEW_MAC)));
    expect(stored(env).devices[OWNER_MAC.id].state).toBe("left");
    await expectOk(await enroll(env, stamped(OWNER_MAC, "MacBook Pro", sha("its own stamp"))));
    expect(stored(env).devices[OWNER_MAC.id]).toMatchObject({ state: "active", endedAt: null, machine: sha("its own stamp") });
  });

  it("refuses a machine stamp that isn't a sha-256", async () => {
    await expectError(await enroll(space(), stamped(NEW_MAC, "MacBook Pro", "not-a-hash")), 400, "bad-request");
  });

  it("answers an unwired roster 503 space/unwired and a broken one space/broken — never seeding it", async () => {
    const env = space({ roster: null });
    await expectError(await enroll(env, enrollBody(NEW_MAC)), 503, "space", { reason: "unwired" });
    expect(env.BUCKET.store.has(ROSTER)).toBe(false);
    await expectError(await enroll(space({ roster: "nope" }), enrollBody(NEW_MAC)), 503, "space", { reason: "broken" });
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.7 — me
// ════════════════════════════════════════════════════════════════════════════════════

describe("me (§4.7)", () => {
  it("tells a member who they are and whose space — no owner block", async () => {
    const b = await expectOk(await api(space(), "GET", "/me", { auth: MHER_MAC.bearer }));
    expect(b).toEqual({
      shroom: 1,
      you: { member: MHER, device: MHER_MAC.id, role: "member", name: null },
      space: { host: HOST, account_id: PARENT.account_id, bucket: PARENT.bucket, owner_name: "Bagrat", iface: 2 },
    });
  });

  it("gives owner-class Macs the mark, the token-holder count and their own flag", async () => {
    const env = space({ objects: { ".shroom/mark.json": JSON.stringify({ on: false }) } });
    const air = await expectOk(await api(env, "GET", "/me", { auth: OWNER_AIR.bearer }));
    expect(air).toMatchObject({ you: { member: M1, device: OWNER_AIR.id, role: "owner", name: "MacBook Air" }, space: { owner_name: "Bagrat" }, owner: { mark: false, tokenHolders: 1, token: false } });
    const mac = await expectOk(await api(env, "GET", "/me", { auth: OWNER_MAC.bearer }));
    expect(mac.owner).toEqual({ mark: false, tokenHolders: 1, token: true });
  });

  it("counts token holders by proofAt within 30 days on active m1 Macs alone — never tokenKey (H1)", async () => {
    const r = roster();
    const recent = deviceKey("Recent"), stale = deviceKey("Stale"), removed = deviceKey("RemovedTok");
    Object.assign(r.devices, {
      [recent.id]: { ...r.devices[OWNER_AIR.id], verifier: recent.verifier, proofAt: iso(NOW - 29 * DAY), tokenKey: "someone-else" },
      [stale.id]: { ...r.devices[OWNER_AIR.id], verifier: stale.verifier, proofAt: iso(NOW - 31 * DAY), tokenKey: PARENT.key_id },
      [removed.id]: { ...r.devices[OWNER_AIR.id], verifier: removed.verifier, proofAt: iso(NOW - DAY), state: "removed", endedAt: iso(NOW - HOUR) },
    });
    const b = await expectOk(await api(space({ roster: r }), "GET", "/me", { auth: OWNER_AIR.bearer }));
    expect(b.owner).toMatchObject({ tokenHolders: 2, mark: true });
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §3.4 · §4.8 · #16–#19 — mint
// ════════════════════════════════════════════════════════════════════════════════════

describe("mint (§3.4, §4.8)", () => {
  it("#16 the golden vector: the Worker's JWT is byte-equal to mint.rs's, claims in alphabetical order", async () => {
    // The fixture itself: GOLDEN is HMAC-SHA256 under the parent's secret over its own signing input.
    const [h, c, s] = GOLDEN.split(".");
    expect(createHmac("sha256", Buffer.from(PARENT.secret, "utf8")).update(`${h}.${c}`).digest("base64url")).toBe(s);
    expect(Buffer.from(c, "base64url").toString("utf8")).toBe(
      `{"aud":"${PARENT.account_id}.r2.cloudflarestorage.com","bucket":"shroom-space","exp":${NOW / 1000 + 604800},"iat":${NOW / 1000},"iss":"${PARENT.key_id}",`
      + `"paths":{"objectPaths":["meta/${REC}.json"],"prefixPaths":["rec/${REC}/"]},"scope":"object-read-write","sub":"${PARENT.account_id}"}`);

    const b = await expectOk(await mint(space(), MHER_MAC));
    const jwt = jwtOf(b);
    expect(jwt).toBe(GOLDEN);
    expect(Buffer.from(b.sessionToken, "base64").toString("utf8")).toBe(`jwt/${GOLDEN}`);
    expect(b.secretAccessKey).toBe(sha(GOLDEN));
    expect(b.accessKeyId).toBe(PARENT.key_id);
  });

  it("answers the credential's whole shape — the owner is the ledger's member (C4)", async () => {
    const b = await expectOk(await mint(space(), MHER_MAC));
    expect(b).toEqual({
      shroom: 1, accessKeyId: PARENT.key_id, secretAccessKey: expect.stringMatching(/^[0-9a-f]{64}$/), sessionToken: expect.any(String),
      expiresAt: iso(NOW + 604800 * SEC), bucket: PARENT.bucket, endpoint: `https://${PARENT.account_id}.r2.cloudflarestorage.com`, owner: MHER,
    });
  });

  it("caps an upload credential at seven days and a manage credential at fifteen minutes, whatever the body asks", async () => {
    const env = space();
    const up = await expectOk(await mint(env, MHER_MAC, REC, "upload", { ttl: 2592000, exp: NOW / 1000 + 2592000, expiresAt: iso(NOW + 30 * DAY) }));
    const c = claimsOf(jwtOf(up));
    expect(c.exp - c.iat).toBe(604800);
    env.BUCKET.touch(metaKey(REC), metaOf(REC, { owner: MHER }));
    const mg = await expectOk(await mint(env, MHER_MAC, REC, "manage", { ttl: 604800 }));
    const m = claimsOf(jwtOf(mg));
    expect(m.exp - m.iat).toBe(900);
    expect(mg.expiresAt).toBe(iso(NOW + 900 * SEC));
  });

  it("#18 every JWT names exactly the request's two paths — nothing under .shroom/, nothing else", async () => {
    const env = space();
    for (const rec of [REC, recId(1), "-_-_-_-_-_-_-_-_", "shroom0000000000"]) {
      const c = claimsOf(jwtOf(await expectOk(await mint(env, MHER_MAC, rec))));
      expect(c.paths).toEqual({ objectPaths: [`meta/${rec}.json`], prefixPaths: [`rec/${rec}/`] });
      expect(JSON.stringify(c)).not.toContain(".shroom");
      expect(Object.keys(c)).toEqual(["aud", "bucket", "exp", "iat", "iss", "paths", "scope", "sub"]);
      expect(c.scope).toBe("object-read-write");
    }
  });

  it("#18 refuses a rec that isn't B14's shape 400, before any key or claim is built", async () => {
    for (const rec of ["AbCdEfGhIjKlMnO", "AbCdEfGhIjKlMnOpQ", "AbCdEfGh/jKlMnOp", "AbCdEfGh.jKlMnOp", "AbCdEf%2FhIjKlMn", "AbCdEf..IjKlMnOp", "../../.shroom/x", "", null, 16]) {
      const env = space();
      await expectError(await mint(env, MHER_MAC, rec), 400, "bad-request");
      const touched = env.BUCKET.ops.map((o) => o.key);
      expect(touched.filter((k) => k !== ROSTER), String(rec)).toEqual([]);
    }
    await expectError(await mint(space(), MHER_MAC, REC, "admin"), 400, "bad-request");
  });

  it("#17 the first upload mint writes the ledger create-only, with the member in customMetadata", async () => {
    const env = space();
    await expectOk(await mint(env, MHER_MAC));
    expect(env.BUCKET.json(ledgerKey(REC))).toEqual({ v: 1, member: MHER, device: MHER_MAC.id, at: iso(NOW) });
    expect(env.BUCKET.customMetadata(ledgerKey(REC))).toEqual({ m: MHER });
    const put = env.BUCKET.ops.find((o) => o.op === "put" && o.key === ledgerKey(REC));
    const onlyIf = put.options?.onlyIf;
    const createOnly = typeof onlyIf?.get === "function" ? onlyIf.get("if-none-match") === "*" : onlyIf?.etagDoesNotMatch === "*";
    expect(createOnly).toBe(true);
  });

  it("#17 writes the ledger before it reads the parent: a no-parent mint still records the owner and blocks uploads", async () => {
    const env = space({ parent: null });
    await expectError(await mint(env, MHER_MAC), 503, "space", { reason: "no-parent" });
    expect(env.BUCKET.json(ledgerKey(REC))).toMatchObject({ member: MHER, device: MHER_MAC.id });
    expect(env.BUCKET.json(HEALTH)).toMatchObject({ v: 1, uploads: "blocked", why: "no-parent", since: iso(NOW) });
  });

  it("#17 another member's later mint → 403 not-yours, and the ledger never changes", async () => {
    const env = space();
    await expectOk(await mint(env, MHER_MAC));
    const before = env.BUCKET.text(ledgerKey(REC));
    const r = stored(env);
    const ani = deviceKey("AniMac");
    r.members[rid("m", "Ani")] = { ...r.members[MHER], label: "Ani" };
    r.devices[ani.id] = { ...r.devices[MHER_MAC.id], member: rid("m", "Ani"), verifier: ani.verifier };
    env.BUCKET.touch(ROSTER, JSON.stringify(r));
    await expectError(await mint(env, ani), 403, "not-yours");
    await expectError(await mint(env, ani, REC, "manage"), 403, "not-yours");
    expect(env.BUCKET.text(ledgerKey(REC))).toBe(before);
  });

  it("create-only ledger: two members' first mints racing on one id → one 200, one 403 not-yours", async () => {
    const r = roster();
    const ani = deviceKey("AniMac");
    r.members[rid("m", "Ani")] = { ...r.members[MHER], label: "Ani" };
    r.devices[ani.id] = { ...r.devices[MHER_MAC.id], member: rid("m", "Ani"), verifier: ani.verifier };
    const env = space({ roster: r });
    let second;
    raceAtPut(env.BUCKET, ledgerKey(REC), async () => { second = await mint(env, ani); });
    const first = await mint(env, MHER_MAC);
    expect(second, "the other member's mint ran inside the first ledger write").toBeDefined();
    expect([first.status, second.status].sort()).toEqual([200, 403]);
    const winner = first.status === 200 ? MHER : rid("m", "Ani");
    expect(env.BUCKET.json(ledgerKey(REC)).member).toBe(winner);
    expect((await bodyOf(first.status === 403 ? first : second)).error).toBe("not-yours");
  });

  it("#17 a re-mint of one's own id needs no budget; owner-class manages any id; a member manages only their own", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { owner: MHER }), [metaKey(recId(9))]: metaOf(recId(9)) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await ledger(env, recId(9), M1, OWNER_MAC.id);
    expect(await expectOk(await mint(env, MHER_MAC))).toMatchObject({ owner: MHER });
    expect(await expectOk(await mint(env, MHER_MAC, REC, "manage"))).toMatchObject({ owner: MHER });
    expect(await expectOk(await mint(env, OWNER_AIR, REC, "manage"))).toMatchObject({ owner: MHER });
    expect(await expectOk(await mint(env, OWNER_AIR, recId(9), "manage"))).toMatchObject({ owner: M1 });
    await expectError(await mint(env, MHER_MAC, recId(9), "manage"), 403, "not-yours");
    expect(env.BUCKET.store.has(rateKey(MHER_MAC.id)) ? env.BUCKET.json(rateKey(MHER_MAC.id)).n : 0).toBe(0);
  });

  it("#17 manage needs a meta (404 absent); a tombstoned id → 410 deleted", async () => {
    const env = space({ objects: { [metaKey(recId(2))]: JSON.stringify({ schema: 1, id: recId(2), state: "deleted", owner: MHER }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await ledger(env, recId(2), MHER, MHER_MAC.id);
    await expectError(await mint(env, MHER_MAC, REC, "manage"), 404, "absent");
    await expectError(await mint(env, MHER_MAC, recId(2)), 410, "deleted");
  });

  it("#17 a legacy id (a meta, no ledger) is m1's: owner-class mints it, a member gets 403 not-yours", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { owner: MHER }) } }); // meta.owner is never trusted
    await expectError(await mint(env, MHER_MAC), 403, "not-yours");
    await expectError(await mint(env, MHER_MAC, REC, "manage"), 403, "not-yours");
    expect(await expectOk(await mint(env, OWNER_AIR, REC, "manage"))).toMatchObject({ owner: M1 });
    expect(await expectOk(await mint(env, OWNER_AIR))).toMatchObject({ owner: M1 });
  });

  it("#19 the upload budget: the 21st first mint from one device in an hour → 429 too-many; re-mints and other devices are free", async () => {
    const env = space();
    for (let i = 0; i < 20; i++) await expectOk(await mint(env, MHER_MAC, recId(i)));
    const res = await mint(env, MHER_MAC, recId(20));
    await expectError(res, 429, "too-many");
    expect((await bodyOf(res)).retryAfter).toBeGreaterThan(0);
    expect(env.BUCKET.store.has(ledgerKey(recId(20)))).toBe(false); // over budget → no ledger entry
    await expectOk(await mint(env, MHER_MAC, recId(3))); // a re-mint never counts
    await expectOk(await mint(env, OWNER_AIR, recId(100)));
    expect(env.BUCKET.json(rateKey(MHER_MAC.id))).toMatchObject({ n: 20 });
    // The next clock hour opens a fresh budget.
    const nextHour = await api(env, "POST", "/mint", { auth: MHER_MAC.bearer, body: { rec: recId(22), purpose: "upload" }, now: NOW + HOUR });
    await expectOk(nextHour);
  });

  it("refuses a removed member's mint 403 removed — and a removed device's, whatever its credentials", async () => {
    const env = space();
    expect((await api(env, "DELETE", `/members/${MHER}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
    await expectError(await mint(env, MHER_MAC), 403, "removed", { why: "removed" });
    await expectError(await mint(env, GONE_MAC), 403, "removed");
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §3.3 · §3.5 · §4.9 · §4.17 · #20–#22 — key, redeem, the amended cookie
// ════════════════════════════════════════════════════════════════════════════════════

const K = "kKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKkKk"; // 43 characters, as 32 random bytes
const K2 = "zZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZzZz";

describe("key — the one-time page key for every Mac (§3.3, §4.9, #21)", () => {
  it("writes {member, device, exp} create-only for ten minutes and answers its expiry", async () => {
    const env = space();
    const b = await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K) } }));
    expect(b).toEqual({ shroom: 1, exp: iso(NOW + 600 * SEC) });
    expect(env.BUCKET.json(grantKey(sha(K)))).toEqual({ member: M1, device: OWNER_AIR.id, exp: iso(NOW + 600 * SEC) });
    const put = env.BUCKET.ops.find((o) => o.op === "put" && o.key === grantKey(sha(K)));
    const onlyIf = put.options?.onlyIf;
    expect(typeof onlyIf?.get === "function" ? onlyIf.get("if-none-match") : onlyIf?.etagDoesNotMatch).toBe("*");
    await expectOk(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(K2) } }));
    expect(env.BUCKET.json(grantKey(sha(K2)))).toMatchObject({ member: MHER, device: MHER_MAC.id });
  });

  it("a replay from the same device is 200 and leaves the first grant; another device's grant → 409 conflict", async () => {
    const env = space();
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K) } }));
    const first = env.BUCKET.text(grantKey(sha(K)));
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K) }, now: NOW + 5 * SEC }));
    expect(env.BUCKET.text(grantKey(sha(K)))).toBe(first);
    await expectError(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(K) } }), 409, "conflict");
    expect(env.BUCKET.text(grantKey(sha(K)))).toBe(first);
  });

  it("refuses a kHash that isn't exactly 64 lower-case hex characters 400 (H5), writing nothing", async () => {
    const h = sha(K);
    for (const kHash of [h.toUpperCase(), h.slice(1), `${h}0`, `../${h.slice(3)}`, `../registry/roster.json`, h.replace(/./, "g"), K, "", null]) {
      const env = space();
      await expectError(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash } }), 400, "bad-request");
      expect(env.BUCKET.written()).toEqual([]);
    }
  });

  it("refuses removed devices and strangers", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/key", { auth: GONE_MAC.bearer, body: { kHash: sha(K) } }), 403, "removed");
    await expectError(await api(env, "POST", "/key", { auth: deviceKey("Stranger").bearer, body: { kHash: sha(K) } }), 401, "unauthorized");
    expect(env.BUCKET.written()).toEqual([]);
  });
});

describe("redeem and the amended cookie {m, d, t} (§3.5, §4.17, #20, #22)", () => {
  const redeem = (env, k = K, now = NOW) => call(env, "GET", `/${REC}?k=${k}`, { now });

  it("redeems a grant into a cookie naming the member and the issuing device — no o", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K) } }));
    const res = await redeem(env);
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(`/${REC}`);
    const set = res.headers.get("set-cookie");
    expect(set).toMatch(/^shroom_owner=[^;]+; Max-Age=34560000; Path=\/; HttpOnly; Secure; SameSite=Lax$/);
    expect(cookieClaims(set)).toEqual({ m: M1, d: OWNER_AIR.id, t: Math.floor(NOW / 1000) });
    expect(env.BUCKET.store.has(grantKey(sha(K)))).toBe(false);
    expect((await redeem(env)).headers.get("set-cookie")).toBeNull();
  });

  it("refuses a grant whose device has since been removed — the same silent 302", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K) } }));
    expect((await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: OWNER_MAC.bearer })).status).toBe(204);
    const res = await redeem(env);
    expect(res.status).toBe(302);
    expect(res.headers.get("set-cookie")).toBeNull();
  });

  it("gives no cookie for a grant with no device; redeems the token Mac's fallback shape, ignoring its role", async () => {
    const exp = iso(NOW + 300 * SEC);
    const env = space({ objects: {
      [metaKey(REC)]: metaOf(REC),
      [grantKey(sha(K))]: JSON.stringify({ member: M1, role: "owner", exp }),
      [grantKey(sha(K2))]: JSON.stringify({ member: M1, role: "owner", device: OWNER_MAC.id, exp }),
    } });
    expect((await redeem(env, K)).headers.get("set-cookie")).toBeNull();
    expect(cookieClaims((await redeem(env, K2)).headers.get("set-cookie"))).toEqual({ m: M1, d: OWNER_MAC.id, t: Math.floor(NOW / 1000) });
  });

  it("#22 two redeems of one k interleaved between the read and the CAS → exactly one set-cookie", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await expectOk(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(K) } }));
    let second;
    raceAtPut(env.BUCKET, grantKey(sha(K)), async () => { second = await redeem(env); });
    const first = await redeem(env);
    expect(second).toBeDefined(); // the redeem spends the grant by a conditional write
    const cookies = [first, second].map((r) => r.headers.get("set-cookie")).filter(Boolean);
    expect(cookies).toHaveLength(1);
    for (const r of [first, second]) {
      expect(r.status).toBe(302);
      expect(r.headers.get("location")).toBe(`/${REC}`);
    }
  });

  it("an owner-class cookie gets owner-mode: the title field and the switch", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const body = await (await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text();
    expect(body).toContain(`class="title"`);
    expect(body).toContain(`id="mk"`);
  });

  it("a removed device's cookie renders a recipient and is refused on title and mark", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const c = cookieFor(M1, OWNER_AIR);
    expect(await (await page(env, `/${REC}`, { cookie: c })).text()).toContain(`class="title"`);
    expect((await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: OWNER_MAC.bearer })).status).toBe(204);
    const body = await (await page(env, `/${REC}`, { cookie: c })).text();
    expect(body).not.toContain(`class="title"`);
    expect(body).not.toContain(`id="mk"`);
    const post = (path, b) => call(env, "POST", path, { body: b, headers: { origin: ORIGIN, cookie: c } });
    expect((await post(`/_shroom/title/${REC}`, { title: "x" })).status).toBe(403);
    expect((await post("/_shroom/mark", { on: false })).status).toBe(403);
    expect(env.BUCKET.json(metaKey(REC)).title).toBe("Untitled — Oct 2");
  });

  it("a removed member's cookie loses its powers too — on every device", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { owner: MHER }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    const c = cookieFor(MHER, MHER_MAC);
    expect(await (await page(env, `/${REC}`, { cookie: c })).text()).toContain(`class="title"`);
    expect((await api(env, "DELETE", `/members/${MHER}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
    expect(await (await page(env, `/${REC}`, { cookie: c })).text()).not.toContain(`class="title"`);
    expect((await call(env, "POST", `/_shroom/title/${REC}`, { body: { title: "x" }, headers: { origin: ORIGIN, cookie: c } })).status).toBe(403);
  });

  it("a cookie without d — from before this change — is a recipient", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const old = cookie({ m: M1, o: true, t: Math.floor(NOW / 1000) - 60 });
    const body = await (await page(env, `/${REC}`, { cookie: old })).text();
    expect(body).not.toContain(`class="title"`);
    expect((await call(env, "POST", "/_shroom/mark", { body: { on: false }, headers: { origin: ORIGIN, cookie: old } })).status).toBe(403);
  });

  it("a cookie whose device belongs to another member is a recipient", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const body = await (await page(env, `/${REC}`, { cookie: cookieFor(M1, MHER_MAC) })).text();
    expect(body).not.toContain(`class="title"`);
  });

  it("a member's title applies only where the ledger names them — never where meta.owner says so", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { owner: M1 }), [metaKey(recId(1))]: metaOf(recId(1), { owner: MHER }), [metaKey(recId(2))]: metaOf(recId(2), { owner: MHER }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await ledger(env, recId(1), M1, OWNER_MAC.id);
    const c = cookieFor(MHER, MHER_MAC);
    const post = (id) => call(env, "POST", `/_shroom/title/${id}`, { body: { title: "Mine now" }, headers: { origin: ORIGIN, cookie: c } });
    expect(await (await page(env, `/${REC}`, { cookie: c })).text()).toContain(`class="title"`);
    expect((await post(REC)).status).toBe(200);
    for (const id of [recId(1), recId(2)]) { // the ledger names m1; a legacy id is m1's
      expect(await (await page(env, `/${id}`, { cookie: c })).text()).not.toContain(`class="title"`);
      expect((await post(id)).status).toBe(403);
    }
    // Owner-class edits all of them.
    const o = cookieFor(M1, OWNER_AIR);
    expect((await call(env, "POST", `/_shroom/title/${recId(2)}`, { body: { title: "Legacy" }, headers: { origin: ORIGIN, cookie: o } })).status).toBe(200);
  });

  it("the mark is owner-class only, by cookie and by device key (§4.16)", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const m = (c, on) => call(env, "POST", "/_shroom/mark", { body: { on }, headers: { origin: ORIGIN, cookie: c } });
    expect((await m(cookieFor(MHER, MHER_MAC), false)).status).toBe(403);
    expect((await m(cookieFor(M1, OWNER_AIR), false)).status).toBe(200);
    expect(env.BUCKET.json(".shroom/mark.json")).toMatchObject({ on: false });
    await expectError(await api(env, "POST", "/mark", { auth: MHER_MAC.bearer, body: { on: true } }), 403, "forbidden");
    expect(await expectOk(await api(env, "POST", "/mark", { auth: OWNER_AIR.bearer, body: { on: true } }))).toMatchObject({ on: true });
    expect(env.BUCKET.json(".shroom/mark.json")).toMatchObject({ on: true });
    await expectError(await api(env, "POST", "/mark", { auth: OWNER_AIR.bearer, body: { on: "yes" } }), 400, "bad-request");
  });

  it("with the roster broken, every cookie renders a recipient — pages still render (C8)", async () => {
    const env = space({ roster: "{broken", objects: { [metaKey(REC)]: metaOf(REC) } });
    const res = await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) });
    expect(res.status).toBe(200);
    const body = await res.text();
    expect(body).not.toContain(`class="title"`);
    expect(body).not.toContain(`id="mk"`);
    const missing = space({ roster: null, objects: { [metaKey(REC)]: metaOf(REC) } });
    expect(await (await page(missing, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text()).not.toContain(`class="title"`);
  });

  it("recipients pay nothing: a cookie-less page never reads the roster", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await page(env, `/${REC}`);
    expect(env.BUCKET.read()).not.toContain(ROSTER);
    await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) }); // a cookie is checked live
    expect(env.BUCKET.read().filter((k) => k === ROSTER)).toHaveLength(1);
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.11–§4.15 · #23 — Members, invites, removals
// ════════════════════════════════════════════════════════════════════════════════════

describe("members (§4.11, #23)", () => {
  it("lists the owner's Macs, active members, every pending invite and every label ever", async () => {
    const b = await expectOk(await api(space(), "GET", "/members", { auth: OWNER_AIR.bearer }));
    expect(b.now).toBe(iso(NOW));
    expect(b.owner).toEqual({ id: M1, name: "Bagrat", devices: expect.any(Array) });
    expect(b.owner.devices).toEqual(expect.arrayContaining([
      { id: OWNER_MAC.id, name: "MacBook Pro", joinedAt: roster().devices[OWNER_MAC.id].joinedAt, token: true, self: false },
      { id: OWNER_AIR.id, name: "MacBook Air", joinedAt: roster().devices[OWNER_AIR.id].joinedAt, token: false, self: true },
    ]));
    expect(b.owner.devices).toHaveLength(2);
    expect(b.members).toEqual([{ id: MHER, label: "Mher", joinedAt: roster().members[MHER].joinedAt }]); // removed Gohar's row is gone
    expect(b.labels).toEqual({ [MHER]: "Mher", [GONE]: "Gohar" });
    const byId = Object.fromEntries(b.invites.map((i) => [i.id, i]));
    expect(Object.keys(byId).sort()).toEqual([INV.ani, INV.own, INV.old, INV.oldOwn].sort()); // pending, expired or not; never used or cancelled
    expect(byId[INV.ani]).toEqual({ id: INV.ani, kind: "member", label: "Ani", createdAt: roster().invites[INV.ani].createdAt, expiresAt: roster().invites[INV.ani].expiresAt, expired: false });
    expect(byId[INV.old]).toMatchObject({ kind: "member", expired: true });
    expect(byId[INV.own]).toMatchObject({ kind: "own_mac", label: null, expired: false });
    const text = JSON.stringify(b);
    for (const code of Object.values(CODE)) { expect(text).not.toContain(code); expect(text).not.toContain(sha(code)); }
    expect(text).not.toContain(OWNER_AIR.verifier);
  });

  it("says token only while proofAt is under 30 days, whatever tokenKey says (H1)", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].proofAt = iso(NOW - 31 * DAY);
    r.devices[OWNER_AIR.id] = { ...r.devices[OWNER_AIR.id], proofAt: iso(NOW - 29 * DAY), tokenKey: "an-old-token" };
    const b = await expectOk(await api(space({ roster: r }), "GET", "/members", { auth: OWNER_AIR.bearer }));
    const tok = Object.fromEntries(b.owner.devices.map((d) => [d.id, d.token]));
    expect(tok).toEqual({ [OWNER_MAC.id]: false, [OWNER_AIR.id]: true });
  });

  it("is owner-class only", async () => {
    await expectError(await api(space(), "GET", "/members", { auth: MHER_MAC.bearer }), 403, "forbidden");
  });
});

describe("invites (§4.12, §4.13)", () => {
  it("creates a member invite for seven days and an own-Mac link for one hour — the server sets the clock", async () => {
    const env = space();
    const b = await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: { ...zed, expiresAt: iso(NOW + 365 * DAY) } }), 201);
    expect(b).toEqual({ shroom: 1, invite: { id: zed.id, kind: "member", label: "Zed", createdAt: iso(NOW), expiresAt: iso(NOW + 7 * DAY), expired: false } });
    expect(stored(env).invites[zed.id]).toMatchObject({ codeHash: zed.codeHash, kind: "member", label: "Zed", inviter: "Bagrat", by: OWNER_AIR.id, state: "pending", createdAt: iso(NOW), expiresAt: iso(NOW + 7 * DAY) });
    const own = { id: rid("i", "Own2"), codeHash: sha("Own2CodeOwn2Code"), kind: "own_mac", label: null, inviter: "Bagrat" };
    const o = await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: own }), 201);
    expect(o.invite).toMatchObject({ kind: "own_mac", label: null, expiresAt: iso(NOW + HOUR) });
    const peek = await bodyOf(await api(env, "POST", "/peek", { body: { code: "ZedCodeZedCode00" } }));
    expect(peek).toMatchObject({ state: "pending", kind: "member", inviter: "Bagrat" });
  });

  it("is idempotent on (id, codeHash); the same id with another hash and no renew → 409 conflict", async () => {
    const env = space();
    const first = await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    const again = await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed, now: NOW + 5 * SEC }), 200);
    expect(again).toEqual(first);
    await expectError(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: { ...zed, codeHash: sha("another") } }), 409, "conflict");
  });

  it("refuses a malformed id, codeHash or kind 400, and members 403", async () => {
    for (const body of [{ ...zed, id: "i_short" }, { ...zed, id: REC }, { ...zed, codeHash: "x" }, { ...zed, codeHash: zed.codeHash.toUpperCase() }, { ...zed, kind: "admin" }, { ...zed, inviter: "" }]) {
      await expectError(await api(space(), "POST", "/invites", { auth: OWNER_AIR.bearer, body }), 400, "bad-request");
    }
    await expectError(await api(space(), "POST", "/invites", { auth: MHER_MAC.bearer, body: zed }), 403, "forbidden");
  });

  it("[ New link ] renews the same row: a new code, a fresh seven days, the old code dead at once", async () => {
    const env = space();
    const fresh = "AniNewCodeAniNew";
    const b = await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: { id: INV.old, codeHash: sha(fresh), renew: true } });
    expect([200, 201]).toContain(b.status);
    expect((await bodyOf(b)).invite).toMatchObject({ id: INV.old, label: "Old", createdAt: iso(NOW), expiresAt: iso(NOW + 7 * DAY), expired: false });
    await expectError(await api(env, "POST", "/peek", { body: { code: CODE.old } }), 404, "unknown");
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: fresh } }))).toMatchObject({ state: "pending" });
  });

  it("renew never resurrects: used → 409 used, cancelled → 410 cancelled, unknown → 404 unknown", async () => {
    const env = space();
    const renew = (id) => api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: { id, codeHash: sha("NewNewNewNewNew0"), renew: true } });
    await expectError(await renew(INV.used), 409, "used");
    await expectError(await renew(INV.cancelled), 410, "cancelled");
    await expectError(await renew(rid("i", "Never")), 404, "unknown");
  });

  it("cancel: 204, the record kept as cancelled, and the next peek, join and page see it", async () => {
    const env = space();
    expect((await api(env, "DELETE", `/invites/${INV.ani}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
    expect(stored(env).invites[INV.ani]).toMatchObject({ state: "cancelled", endedAt: iso(NOW) });
    expect(await bodyOf(await api(env, "POST", "/peek", { body: { code: CODE.ani } }))).toMatchObject({ state: "cancelled" });
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC) }), 410, "cancelled");
    expect(textOf(await (await page(env, `/join/${CODE.ani}`)).text())).toContain(W.expired);
    expect((await api(env, "DELETE", `/invites/${INV.ani}`, { auth: OWNER_AIR.bearer })).status).toBe(204); // idempotent
  });

  it("cancels an expired pending invite like a live one; refuses a used one 409 with the member; unknown 404; members 403", async () => {
    const env = space();
    expect((await api(env, "DELETE", `/invites/${INV.old}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
    await expectError(await api(env, "DELETE", `/invites/${INV.used}`, { auth: OWNER_AIR.bearer }), 409, "used", { member: MHER });
    await expectError(await api(env, "DELETE", `/invites/${rid("i", "Never")}`, { auth: OWNER_AIR.bearer }), 404, "unknown");
    await expectError(await api(env, "DELETE", `/invites/${INV.ani}`, { auth: MHER_MAC.bearer }), 403, "forbidden");
    expect(stored(env).invites[INV.ani].state).toBe("pending");
  });
});

describe("removals (§4.14, §4.15)", () => {
  it("removing a member removes every one of their devices and keeps the label forever", async () => {
    const r = roster();
    const second = deviceKey("MherAir");
    r.devices[second.id] = { ...r.devices[MHER_MAC.id], verifier: second.verifier };
    const env = space({ roster: r });
    expect((await api(env, "DELETE", `/members/${MHER}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
    const after = stored(env);
    expect(after.members[MHER]).toMatchObject({ state: "removed", endedAt: iso(NOW), label: "Mher" });
    expect(after.devices[MHER_MAC.id].state).toBe("removed");
    expect(after.devices[second.id].state).toBe("removed");
    await expectError(await api(env, "GET", "/me", { auth: second.bearer }), 403, "removed");
    const list = await expectOk(await api(env, "GET", "/members", { auth: OWNER_AIR.bearer }));
    expect(list.members).toEqual([]);
    expect(list.labels[MHER]).toBe("Mher");
    expect((await api(env, "DELETE", `/members/${MHER}`, { auth: OWNER_AIR.bearer })).status).toBe(204); // idempotent
  });

  it("refuses m1 409 owner, an unknown member 404, a member caller 403", async () => {
    const env = space();
    await expectError(await api(env, "DELETE", `/members/${M1}`, { auth: OWNER_AIR.bearer }), 409, "owner");
    await expectError(await api(env, "DELETE", `/members/${rid("m", "Never")}`, { auth: OWNER_AIR.bearer }), 404, "unknown");
    await expectError(await api(env, "DELETE", `/members/${MHER}`, { auth: MHER_MAC.bearer }), 403, "forbidden");
  });

  it("removes another owner Mac; refuses self, a token holder, a member's Mac and an unknown one", async () => {
    const env = space();
    await expectError(await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: OWNER_AIR.bearer }), 409, "self");
    await expectError(await api(env, "DELETE", `/devices/${OWNER_MAC.id}`, { auth: OWNER_AIR.bearer }), 409, "token-holder");
    await expectError(await api(env, "DELETE", `/devices/${MHER_MAC.id}`, { auth: OWNER_AIR.bearer }), 409, "not-owner-mac");
    await expectError(await api(env, "DELETE", `/devices/${deviceKey("Never").id}`, { auth: OWNER_AIR.bearer }), 404, "unknown");
    await expectError(await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: MHER_MAC.bearer }), 403, "forbidden");
    expect((await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: OWNER_MAC.bearer })).status).toBe(204);
    expect(stored(env).devices[OWNER_AIR.id]).toMatchObject({ state: "removed", endedAt: iso(NOW) });
    expect((await api(env, "DELETE", `/devices/${OWNER_AIR.id}`, { auth: OWNER_MAC.bearer })).status).toBe(204); // idempotent
    expect(stored(env).members[M1].state).toBe("active");
  });

  it("a token holder's flag lapses after 30 days without proof, and Remove… works again", async () => {
    const r = roster();
    r.devices[OWNER_MAC.id].proofAt = iso(NOW - 31 * DAY);
    const env = space({ roster: r });
    expect((await api(env, "DELETE", `/devices/${OWNER_MAC.id}`, { auth: OWNER_AIR.bearer })).status).toBe(204);
  });

  it("self: a member's last Mac leaves, and the member with it — the label stays", async () => {
    const env = space();
    expect((await api(env, "DELETE", "/devices/self", { auth: MHER_MAC.bearer })).status).toBe(204);
    const r = stored(env);
    expect(r.devices[MHER_MAC.id]).toMatchObject({ state: "left", endedAt: iso(NOW) });
    expect(r.members[MHER]).toMatchObject({ state: "left", endedAt: iso(NOW), label: "Mher" });
  });

  it("self: a member with another active Mac stays; m1 never changes state", async () => {
    const r = roster();
    const second = deviceKey("MherAir");
    r.devices[second.id] = { ...r.devices[MHER_MAC.id], verifier: second.verifier };
    const env = space({ roster: r });
    expect((await api(env, "DELETE", "/devices/self", { auth: MHER_MAC.bearer })).status).toBe(204);
    expect(stored(env).members[MHER].state).toBe("active");
    expect((await api(env, "DELETE", "/devices/self", { auth: OWNER_MAC.bearer })).status).toBe(204);
    expect((await api(env, "DELETE", "/devices/self", { auth: OWNER_AIR.bearer })).status).toBe(204);
    expect(stored(env).members[M1].state).toBe("active");
    expect(stored(env).devices[OWNER_MAC.id].state).toBe("left");
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.10 · #24 — the Library
// ════════════════════════════════════════════════════════════════════════════════════

describe("library (§4.10, #24)", () => {
  const lib = (env, dev, body) => api(env, "POST", "/library", { auth: dev.bearer, body: { scope: "mine", known: {}, knownThrough: null, after: null, ...body } });
  /** Pages until next is null, each call carrying the next slice of the client's cache:
   * at most 1,000 cached ids after `after`, knownThrough naming the last of them (null
   * when the cache has no more) — §4.10's client contract. */
  async function walk(env, dev, body = {}, cache = {}) {
    const items = [];
    const owners = {};
    const cached = Object.keys(cache).sort();
    let after = null, calls = 0;
    do {
      const rest = cached.filter((id) => after === null || id > after);
      const slice = rest.slice(0, 1000);
      const known = Object.fromEntries(slice.map((id) => [id, cache[id]]));
      const knownThrough = rest.length > slice.length ? slice.at(-1) : null;
      const b = await expectOk(await lib(env, dev, { ...body, known, knownThrough, after }));
      items.push(...(b.items ?? []));
      Object.assign(owners, b.owners ?? {});
      after = b.next;
      if (++calls > 200) throw new Error("the Library never ended");
    } while (after);
    return { items, owners, calls };
  }
  async function shelf({ mher = 3, m1 = 2, legacy = 1, other = 1 } = {}) {
    const env = space();
    const ids = { mher: [], m1: [], legacy: [], other: [] };
    let n = 0;
    const add = async (kind, member) => {
      const id = recId(n++);
      ids[kind].push(id);
      await env.BUCKET.put(metaKey(id), metaOf(id, { owner: member ?? M1 }));
      if (member) await ledger(env, id, member);
    };
    for (let i = 0; i < mher; i++) await add("mher", MHER);
    for (let i = 0; i < m1; i++) await add("m1", M1);
    for (let i = 0; i < legacy; i++) await add("legacy", null);
    for (let i = 0; i < other; i++) await add("other", GONE);
    env.BUCKET.ops.length = 0;
    return { env, ids };
  }

  it("forces members to mine: their own ids by the ledger, never by meta.owner", async () => {
    const { env, ids } = await shelf();
    await env.BUCKET.put(metaKey(ids.m1[0]), metaOf(ids.m1[0], { owner: MHER })); // a forged meta.owner
    const { items } = await walk(env, MHER_MAC);
    expect(items.map((i) => i.id).sort()).toEqual(ids.mher);
    for (const i of items) expect(i.owner).toBe(MHER);
  });

  it("refuses a member everyone or owners — 403 forbidden", async () => {
    const { env } = await shelf();
    await expectError(await lib(env, MHER_MAC, { scope: "everyone" }), 403, "forbidden");
    await expectError(await lib(env, MHER_MAC, { scope: "owners" }), 403, "forbidden");
  });

  it("gives owner-class everyone, legacy ids as m1's; mine is m1's own plus the legacy ones", async () => {
    const { env, ids } = await shelf();
    const every = await walk(env, OWNER_AIR, { scope: "everyone" });
    const owner = Object.fromEntries(every.items.map((i) => [i.id, i.owner]));
    expect(Object.keys(owner).sort()).toEqual([...ids.mher, ...ids.m1, ...ids.legacy, ...ids.other].sort());
    for (const id of ids.legacy) expect(owner[id]).toBe(M1);
    for (const id of ids.other) expect(owner[id]).toBe(GONE);
    const mine = await walk(env, OWNER_AIR, { scope: "mine" });
    expect(mine.items.map((i) => i.id).sort()).toEqual([...ids.m1, ...ids.legacy].sort());
  });

  it("sends bodies only for ETags that differ from known", async () => {
    const { env, ids } = await shelf();
    const cold = await walk(env, MHER_MAC);
    for (const i of cold.items) { expect(i.meta).toMatchObject({ id: i.id, state: "live" }); expect(typeof i.etag).toBe("string"); }
    const known = Object.fromEntries(cold.items.map((i) => [i.id, i.etag]));
    env.BUCKET.ops.length = 0;
    const warm = await walk(env, MHER_MAC, {}, known);
    for (const i of warm.items) expect(i.meta).toBeUndefined();
    expect(env.BUCKET.read().filter((k) => k.startsWith("meta/"))).toEqual([]);
    env.BUCKET.touch(metaKey(ids.mher[1]), metaOf(ids.mher[1], { title: "Renamed" }));
    const delta = await walk(env, MHER_MAC, {}, known);
    expect(delta.items.filter((i) => i.meta).map((i) => [i.id, i.meta.title])).toEqual([[ids.mher[1], "Renamed"]]);
  });

  it("returns tombstones as deleted", async () => {
    const { env, ids } = await shelf();
    env.BUCKET.touch(metaKey(ids.mher[0]), JSON.stringify({ schema: 1, id: ids.mher[0], state: "deleted", owner: MHER }));
    const { items } = await walk(env, MHER_MAC);
    expect(items.find((i) => i.id === ids.mher[0]).meta).toMatchObject({ state: "deleted" });
  });

  it("GETs at most 40 bodies a call, then cuts the page after the 40th and points next at it", async () => {
    const { env, ids } = await shelf({ mher: 0, m1: 50, legacy: 0, other: 0 });
    const first = await expectOk(await lib(env, OWNER_AIR, { scope: "everyone" }));
    expect(first.items.filter((i) => i.meta)).toHaveLength(40);
    expect(first.next).toBe(ids.m1[39]);
    expect(first.items.every((i) => i.id <= ids.m1[39])).toBe(true);
    expect(env.BUCKET.read().filter((k) => k.startsWith("meta/")).length).toBeLessThanOrEqual(40);
    const all = await walk(env, OWNER_AIR, { scope: "everyone" });
    expect(new Set(all.items.filter((i) => i.meta).map((i) => i.id)).size).toBe(50);
  });

  it("follows a short truncated page from either list — fewer than the limit is never the end (P3)", async () => {
    const { env, ids } = await shelf({ mher: 10, m1: 0, legacy: 0, other: 0 });
    env.BUCKET.opts.pageSize = 3;
    const first = await expectOk(await lib(env, MHER_MAC));
    expect(first.next).not.toBeNull();
    expect(first.items.every((i) => i.id <= first.next)).toBe(true);
    const { items, calls } = await walk(env, MHER_MAC);
    expect(items.map((i) => i.id).sort()).toEqual(ids.mher);
    expect(calls).toBeGreaterThanOrEqual(4);
    expect(env.BUCKET.ops.filter((o) => o.op === "list").every((o) => (o.options?.limit ?? 1000) <= 1000)).toBe(true);
  });

  it("never judges past knownThrough: the page ends there and next names it (D)", async () => {
    const { env, ids } = await shelf({ mher: 10, m1: 0, legacy: 0, other: 0 });
    const cold = await walk(env, MHER_MAC);
    const etag = Object.fromEntries(cold.items.map((i) => [i.id, i.etag]));
    const known = Object.fromEntries(ids.mher.slice(0, 5).map((id) => [id, etag[id]]));
    env.BUCKET.ops.length = 0;
    const b = await expectOk(await lib(env, MHER_MAC, { known, knownThrough: ids.mher[4] }));
    expect(b.items.every((i) => i.id <= ids.mher[4])).toBe(true);
    expect(b.next).toBe(ids.mher[4]);
    expect(env.BUCKET.read().filter((k) => k.startsWith("meta/"))).toEqual([]);
  });

  it("the owners scope gives the whole id → member map across pages, uncapped, from the ledger alone (H3)", async () => {
    const env = space();
    const want = {};
    for (let i = 0; i < 1500; i++) {
      const id = recId(i);
      const member = i % 3 === 0 ? MHER : i % 3 === 1 ? M1 : GONE;
      want[id] = member;
      await ledger(env, id, member);
    }
    env.BUCKET.ops.length = 0;
    const { owners, calls, items } = await walk(env, OWNER_AIR, { scope: "owners" });
    expect(owners).toEqual(want);
    expect(items).toEqual([]);
    expect(calls).toBeGreaterThanOrEqual(2);
    expect(env.BUCKET.read().filter((k) => k.startsWith(".shroom/registry/recs/") || k.startsWith("meta/"))).toEqual([]);
    const lists = env.BUCKET.ops.filter((o) => o.op === "list");
    expect(lists.every((o) => o.key === ".shroom/registry/recs/" && o.options?.include?.includes("customMetadata"))).toBe(true);
  }, 20_000);

  it("the owners scope walks short pages too", async () => {
    const env = space();
    for (let i = 0; i < 30; i++) await ledger(env, recId(i), i % 2 ? MHER : M1);
    env.BUCKET.opts.pageSize = 7;
    const { owners } = await walk(env, OWNER_MAC, { scope: "owners" });
    expect(Object.keys(owners)).toHaveLength(30);
  });

  it("never carries a label, and refuses a removed device", async () => {
    const { env } = await shelf();
    const every = await walk(env, OWNER_AIR, { scope: "everyone" });
    expect(JSON.stringify(every)).not.toMatch(/"Mher"|"Gohar"/);
    await expectError(await lib(env, GONE_MAC, {}), 403, "removed");
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.18 · #25 — health, and the owner page's "Waiting for your space"
// ════════════════════════════════════════════════════════════════════════════════════

describe("deletion records (DECISIONS B125)", () => {
  const tomb = (id) => metaOf(id, { state: "deleted", owner: MHER });

  it("the maker seals its own deleted video; the record outranks a meta rewritten to live", async () => {
    const env = space({ objects: { [metaKey(REC)]: tomb(REC) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    expect((await api(env, "POST", "/gone", { auth: MHER_MAC.bearer, body: { rec: REC } })).status).toBe(204);
    expect(env.BUCKET.json(goneKey(REC))).toEqual({ v: 1, by: MHER_MAC.id, at: iso(NOW) });
    // Its upload credential still reaches the meta: written back to live, it stays removed.
    env.BUCKET.touch(metaKey(REC), metaOf(REC, { owner: MHER }));
    expect(await (await call(env, "GET", `/_shroom/status/${REC}`)).json()).toEqual({ state: "removed" });
    expect((await call(env, "GET", `/m/${REC}/master.m3u8`)).status).toBe(404);
    await expectError(await mint(env, MHER_MAC, REC, "upload"), 410, "deleted");
    await expectError(await mint(env, MHER_MAC, REC, "manage"), 410, "deleted");
    await expectOk(await mint(env, OWNER_AIR, REC, "manage"));
    // Sealing again is a no-op.
    expect((await api(env, "POST", "/gone", { auth: MHER_MAC.bearer, body: { rec: REC } })).status).toBe(204);
  });

  it("the owner seals a member's deleted video; another member can't; a live one can't be sealed", async () => {
    const env = space({ objects: { [metaKey(REC)]: tomb(REC), [metaKey(recId(3))]: metaOf(recId(3)) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await expectError(await api(env, "POST", "/gone", { auth: GONE_MAC.bearer, body: { rec: REC } }), 403, "removed");
    await expectError(await api(env, "POST", "/gone", { auth: OWNER_AIR.bearer, body: { rec: recId(3) } }), 409, "not-deleted");
    await expectError(await api(env, "POST", "/gone", { auth: MHER_MAC.bearer, body: { rec: "../x" } }), 400, "bad-request");
    expect(env.BUCKET.store.has(goneKey(recId(3)))).toBe(false);
    expect((await api(env, "POST", "/gone", { auth: OWNER_AIR.bearer, body: { rec: REC } })).status).toBe(204);
    expect(env.BUCKET.store.has(goneKey(REC))).toBe(true);
  });

  it("a sealed video's title can't be edited, whatever its meta says now", async () => {
    const env = space({ objects: { [metaKey(REC)]: tomb(REC), [goneKey(REC)]: JSON.stringify({ v: 1, by: MHER_MAC.id, at: iso(NOW) }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    // A stale upload credential wrote it back to live after the seal.
    env.BUCKET.touch(metaKey(REC), metaOf(REC, { owner: MHER, title: "Before" }));
    for (const c of [cookieFor(MHER, MHER_MAC), cookieFor(M1, OWNER_AIR)]) {
      const res = await call(env, "POST", `/_shroom/title/${REC}`, { body: { title: "Renamed" }, headers: { origin: ORIGIN, cookie: c } });
      expect(res.status).toBe(409);
    }
    expect(env.BUCKET.json(metaKey(REC)).title).toBe("Before");
  });

  it("the Library lists a sealed video as deleted, whatever its meta says now — every scope that lists", async () => {
    const live = recId(4);
    const env = space({ objects: { [metaKey(REC)]: tomb(REC), [metaKey(live)]: metaOf(live, { owner: MHER }), [goneKey(REC)]: JSON.stringify({ v: 1, by: MHER_MAC.id, at: iso(NOW) }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await ledger(env, live, MHER, MHER_MAC.id);
    env.BUCKET.touch(metaKey(REC), metaOf(REC, { owner: MHER, title: "Secret plans" }));
    for (const [dev, scope] of [[MHER_MAC, "mine"], [OWNER_AIR, "everyone"], [OWNER_MAC, "everyone"]]) {
      const b = await expectOk(await api(env, "POST", "/library", { auth: dev.bearer, body: { scope, known: {}, knownThrough: null, after: null } }));
      const byId = Object.fromEntries(b.items.map((i) => [i.id, i]));
      expect(byId[REC].meta, `${scope} via ${dev.id}`).toEqual({ schema: 1, id: REC, state: "deleted", createdAt: "2026-10-02T11:00:00Z" });
      expect(JSON.stringify(byId[REC])).not.toContain("Secret plans");
      expect(byId[live].meta.state).toBe("live");
    }
  });

  it("an id nobody has recorded can't be sealed — the maker's first mint on it still works", async () => {
    const env = space();
    const fresh = recId(9);
    await expectError(await api(env, "POST", "/gone", { auth: GONE_MAC.bearer, body: { rec: fresh } }), 403, "removed");
    for (const dev of [MHER_MAC, OWNER_AIR]) await expectError(await api(env, "POST", "/gone", { auth: dev.bearer, body: { rec: fresh } }), 404, "absent");
    expect(env.BUCKET.store.has(goneKey(fresh))).toBe(false);
    await expectOk(await mint(env, MHER_MAC, fresh, "upload"));
  });

  it("no credential a member can mint reaches a deletion record", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const b = await expectOk(await mint(env, MHER_MAC, recId(8), "upload"));
    expect(JSON.stringify(claimsOf(jwtOf(b)).paths)).not.toContain(".shroom/gone");
  });

  it("only a recording's maker, or the owner, can say its uploads are refused", async () => {
    const env = space();
    await ledger(env, REC, M1, OWNER_AIR.id);
    await expectError(await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: REC, uploads: "refused" } }), 403, "not-yours");
    expect(env.BUCKET.written()).not.toContain(HEALTH);
    expect((await api(env, "POST", "/health", { auth: OWNER_AIR.bearer, body: { rec: REC, uploads: "refused" } })).status).toBe(204);
  });
});

describe("page key budget (DECISIONS B125)", () => {
  it("a device asks for at most 60 page keys an hour; its upload budget is its own", async () => {
    const env = space();
    for (let i = 0; i < 60; i++) await expectOk(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(`key-${i}`) } }));
    await expectError(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha("key-60") } }), 429, "too-many");
    expect(env.BUCKET.store.has(grantKey(sha("key-60")))).toBe(false);
    expect(env.BUCKET.json(rateKey(MHER_MAC.id))).toMatchObject({ k: 60, n: 0 });
    // Another device's keys, and this device's first upload, still go through.
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha("owner-key") } }));
    await expectOk(await mint(env, MHER_MAC, recId(9), "upload"));
    expect(env.BUCKET.json(rateKey(MHER_MAC.id))).toMatchObject({ k: 60, n: 1 });
  });
});

describe("health (§4.18, #25)", () => {
  const stalledMeta = (id, over = {}) => metaOf(id, { state: "uploading", media: null, progress: { done: 1, total: 4, updatedAt: iso(NOW - 10 * 60 * SEC) }, ...over });
  const blocked = JSON.stringify({ v: 1, uploads: "blocked", since: iso(NOW - HOUR), why: "refused", by: MHER_MAC.id });

  it("a device's refused report blocks the space; its ok report clears it; identical reports write nothing", async () => {
    const env = space();
    await ledger(env, REC, MHER, MHER_MAC.id);
    expect((await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: REC, uploads: "refused" } })).status).toBe(204);
    expect(env.BUCKET.json(HEALTH)).toEqual({ v: 1, uploads: "blocked", since: iso(NOW), why: "refused", by: MHER_MAC.id });
    const writes = () => env.BUCKET.written().filter((k) => k === HEALTH).length;
    const before = writes();
    expect((await api(env, "POST", "/health", { auth: OWNER_AIR.bearer, body: { rec: recId(1), uploads: "refused" } })).status).toBe(204);
    expect(writes()).toBe(before);
    expect((await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: REC, uploads: "ok" } })).status).toBe(204);
    expect(env.BUCKET.json(HEALTH)).toMatchObject({ uploads: "ok" });
    expect((await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: REC, uploads: "ok" } })).status).toBe(204);
    expect(writes()).toBe(before + 1);
  });

  it("refuses a malformed rec or report 400, and removed devices 403", async () => {
    const env = space();
    await expectError(await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: "../x", uploads: "refused" } }), 400, "bad-request");
    await expectError(await api(env, "POST", "/health", { auth: MHER_MAC.bearer, body: { rec: REC, uploads: "blocked" } }), 400, "bad-request");
    await expectError(await api(env, "POST", "/health", { auth: GONE_MAC.bearer, body: { rec: REC, uploads: "refused" } }), 403, "removed");
    expect(env.BUCKET.written()).toEqual([]);
  });

  it("a stalled owner page reads Waiting for your space to an owner-class viewer", async () => {
    const env = space({ objects: { [metaKey(REC)]: stalledMeta(REC), [HEALTH]: blocked } });
    const body = await (await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text();
    expect(body).toContain(W.waitingSpace);
    expect(body).not.toContain(COPY.waitingConnection);
  });

  it("a member's own stalled video reads Waiting for Bagrat's space", async () => {
    const env = space({ objects: { [metaKey(REC)]: stalledMeta(REC), [HEALTH]: blocked } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    const body = await (await page(env, `/${REC}`, { cookie: cookieFor(MHER, MHER_MAC) })).text();
    expect(body).toContain(W.waitingBagrat);
    expect(body).not.toContain(W.waitingSpace);
  });

  it("with uploads ok or no record, a stall still reads Waiting for connection — blocked is what changes it", async () => {
    const env0 = space({ objects: { [metaKey(REC)]: stalledMeta(REC), [HEALTH]: blocked } });
    expect(await (await page(env0, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text()).toContain(W.waitingSpace);
    for (const objects of [{}, { [HEALTH]: JSON.stringify({ v: 1, uploads: "ok", since: iso(NOW - HOUR), why: null, by: null }) }]) {
      const env = space({ objects: { [metaKey(REC)]: stalledMeta(REC), ...objects } });
      const body = await (await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text();
      expect(body).toContain(COPY.waitingConnection);
      expect(body).not.toContain("Waiting for your space");
    }
  });

  // Corrected to the spec (DECISIONS B49): N40 / Region 4 beat 7 — a space that serves but
  // can't take uploads opens the page “as usual, its upload line saying so”, so the maker's
  // own meta-less page keeps the owner chrome (the title field) with the waiting words.
  it("the meta-less row: the ledger names the viewer and the space is blocked → the waiting words, the page as usual", async () => {
    const env = space({ objects: { [HEALTH]: blocked } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    const member = await (await page(env, `/${REC}`, { cookie: cookieFor(MHER, MHER_MAC) })).text();
    expect(member).toContain(W.waitingBagrat);
    expect(member).toContain(`class="title"`);
    // Corrected to the spec again (DECISIONS B53): the owner-class row with no ledger entry
    // is the token Mac's take under a token that died before it (its mint can't write the
    // entry) — N40's “opens as usual” holds for it too: the title field, Copy link, the
    // waiting words.
    const owner = await (await page(env, `/${recId(5)}`, { cookie: cookieFor(M1, OWNER_AIR) })).text();
    expect(owner).toContain(W.waitingSpace);
    expect(owner).toContain(`class="title"`);
    expect(owner).toContain(`<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button>`);
    expect(owner).toContain(`"pending":true`);
    expect(owner).not.toContain(COPY.onItsWay);
    // A recipient of the same id on the same blocked space: still on its way, no why.
    const recipient = await (await page(env, `/${recId(5)}`)).text();
    expect(recipient).toContain(COPY.onItsWay);
    expect(recipient).not.toMatch(/Waiting for (your|Bagrat's) space/);
    expect(recipient).not.toContain(`class="title"`);
    const clear = space();
    expect(await (await page(clear, `/${recId(5)}`, { cookie: cookieFor(M1, OWNER_AIR) })).text()).toContain(COPY.onItsWay);
  });

  it("a recipient's page never changes with health, and the status poll never carries it", async () => {
    const env = space({ objects: { [metaKey(REC)]: stalledMeta(REC), [HEALTH]: blocked } });
    const body = await (await page(env, `/${REC}`)).text();
    expect(body).toContain(COPY.stalled);
    expect(body).not.toMatch(/Waiting for (your|Bagrat's) space/);
    expect(await (await page(env, `/${REC}`, { cookie: cookieFor(M1, OWNER_AIR) })).text()).toContain(W.waitingSpace); // the owner's does
    const withHealth = await (await call(env, "GET", `/_shroom/status/${REC}`)).text();
    const without = await (await call(space({ objects: { [metaKey(REC)]: stalledMeta(REC) } }), "GET", `/_shroom/status/${REC}`)).text();
    expect(withHealth).toBe(without);
    expect(withHealth).not.toMatch(/block|health|space/i);
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §4.3 · #11 · #12 — the join page
// ════════════════════════════════════════════════════════════════════════════════════

describe("the join page (§4.3, r6, #11)", () => {
  const expectFace = async (res) => {
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toMatch(/^text\/html/);
    expect(res.headers.get("x-robots-tag")).toBe("noindex");
    expect(res.headers.get("cache-control")).toBe("no-store");
    expect(res.headers.get("referrer-policy")).toBe("no-referrer");
    expect(res.headers.get("content-security-policy") ?? "").toContain("frame-ancestors 'none'");
    const html = await res.text();
    expect(html).toContain(W.mark); // every face: "made with shroom" + the mark, linking home
    expect(html).toContain(`href="https://shroom.am"`);
    return html;
  };
  const hasLines = (html, lines) => {
    const t = textOf(html);
    for (const l of lines) {
      if (l.role === "tooltip") expect(html, l.text).toContain(l.text);
      else expect(t, l.text).toContain(l.text);
    }
  };
  const lacks = (html, texts) => { const t = textOf(html); for (const x of texts) expect(t, x).not.toContain(x); };

  it("join-landing: every line verbatim, the hero as the title, the install command with the link built in, the deep link", async () => {
    const env = space();
    const html = await expectFace(await page(env, `/join/${CODE.ani}`));
    hasLines(html, LANDING(CODE.ani));
    expect(titleOf(html)).toBe(W.hero);
    expect(textOf(html)).toContain(INSTALL(CODE.ani));
    expect(textOf(html)).toContain(BREW(CODE.ani));
    expect(html).toContain(`shroom://join/${HOST}/${CODE.ani}`);
    expect(html).toContain("maxTouchPoints"); // iPadOS's Macintosh UA flips to the elsewhere face
    lacks(html, [W.ownerNote.split(" Ani ")[0], W.used, W.expired]);
  });

  it("join-landing never shows the invite's label to anyone but an owner-class cookie", async () => {
    const r = roster();
    r.invites[INV.ani].label = "Zaruhi-Private";
    for (const c of [undefined, cookieFor(MHER, MHER_MAC), cookieFor(M1, GONE_MAC), cookie({ m: M1, o: true, t: 1 })]) {
      const html = await (await page(space({ roster: r }), `/join/${CODE.ani}`, { cookie: c })).text();
      expect(titleOf(html)).toBe(W.hero);
      expect(html).not.toContain("Zaruhi-Private");
    }
  });

  it("join-landing · owner view: the same page with one quiet note, for an owner-class cookie checked live", async () => {
    const env = space();
    const html = await expectFace(await page(env, `/join/${CODE.ani}`, { cookie: cookieFor(M1, OWNER_AIR) }));
    hasLines(html, [...LANDING(CODE.ani), { text: W.ownerNote }]);
    expect(titleOf(html)).toBe(W.hero);
    // A member's cookie, or a removed owner Mac's, sees the plain landing.
    for (const c of [cookieFor(MHER, MHER_MAC), cookieFor(M1, GONE_MAC)]) {
      lacks(await (await page(env, `/join/${CODE.ani}`, { cookie: c })).text(), [W.ownerNote]);
    }
    const r = roster();
    r.devices[OWNER_AIR.id] = { ...r.devices[OWNER_AIR.id], state: "removed", endedAt: iso(NOW - HOUR) };
    lacks(await (await page(space({ roster: r }), `/join/${CODE.ani}`, { cookie: cookieFor(M1, OWNER_AIR) })).text(), [W.ownerNote]);
    expect(env.BUCKET.written()).toEqual([]); // opening it here doesn't use it up
    expect(stored(env).invites[INV.ani].state).toBe("pending");
  });

  it.each(["iphone", "ipad", "android", "windows", "linux", "chromebook"])("join-landing-elsewhere on %s: the hero stays, one line and Copy link replace the install block", async (ua) => {
    const html = await expectFace(await page(space(), `/join/${CODE.ani}`, { ua: UA[ua] }));
    hasLines(html, everyCase(saidOf(r6, "join-landing-elsewhere")));
    expect(titleOf(html)).toBe(W.hero);
    lacks(html, ["curl -fsSL", "brew install", pick("join-landing", "Get shroom"), pick("join-landing", "Already have shroom?"), pick("join-landing", "Paste this into Terminal")]);
  });

  it("a Mac's browser — Safari or Chrome — gets the Mac face", async () => {
    for (const ua of [UA.mac, UA.macChrome]) {
      const html = await (await page(space(), `/join/${CODE.ani}`, { ua })).text();
      expect(textOf(html)).toContain(INSTALL(CODE.ani));
      lacks(html, [W.elsewhere]);
    }
  });

  it("join-landing · own-Mac variant: its own hero and lines, the same Get-shroom block, no seven-days line", async () => {
    const html = await expectFace(await page(space(), `/join/${CODE.own}`));
    hasLines(html, [...saidOf(r6, "join-landing-own-mac"), ...GET_SHROOM_BLOCK(CODE.own)]);
    expect(titleOf(html)).toBe(W.ownHero);
    expect(textOf(html)).toContain(INSTALL(CODE.own));
    lacks(html, [W.sevenDays, W.hero]);
  });

  it("the own-Mac variant's owner view carries the note's words without a name", async () => {
    const html = await expectFace(await page(space(), `/join/${CODE.own}`, { cookie: cookieFor(M1, OWNER_AIR) }));
    hasLines(html, everyCase(saidOf(r6, "join-landing-own-mac")));
    expect(textOf(html)).toContain("opening it here doesn't use it up.");
    expect(textOf(html)).not.toContain("This is the page null");
  });

  it("join-expired · page: expired and cancelled read the inviter's line alone — no install block", async () => {
    for (const code of [CODE.old, CODE.cancelled]) {
      const html = await expectFace(await page(space(), `/join/${code}`));
      expect(textOf(html)).toContain(W.expired);
      expect(titleOf(html)).toBe(W.expired);
      lacks(html, ["curl -fsSL", "Open shroom", W.sevenDays]);
    }
    // A pending invite one second past its time is expired by the clock.
    const late = await call(space(), "GET", `/join/${CODE.ani}`, { headers: { "user-agent": UA.mac }, now: Date.parse(roster().invites[INV.ani].expiresAt) + SEC });
    expect(textOf(await late.text())).toContain(W.expired);
  });

  it("join-expired · page: an own-Mac link past its hour has its own words", async () => {
    const html = await expectFace(await page(space(), `/join/${CODE.oldOwn}`));
    expect(textOf(html)).toContain(W.ownExpired);
    expect(titleOf(html)).toBe(W.ownExpired);
    lacks(html, [W.expired, "curl -fsSL"]);
  });

  it("join-expired · page, nameless: an unknown or malformed code", async () => {
    for (const code of [CODE.unknown, "7xk2pm", "AniCodeAniCode0", "AniCodeAniCode000", "%3Cscript%3E"]) {
      const html = await expectFace(await page(space(), `/join/${code}`));
      expect(textOf(html), code).toContain(W.unknown);
      expect(titleOf(html)).toBe(W.unknown);
      lacks(html, ["Bagrat", "curl -fsSL"]);
    }
  });

  it("join-used · page: the honest line — and You're in for the joiner's own cookie", async () => {
    const env = space();
    for (const c of [undefined, cookieFor(M1, OWNER_AIR), cookieFor(GONE, GONE_MAC)]) {
      const html = await expectFace(await page(env, `/join/${CODE.used}`, { cookie: c }));
      expect(textOf(html)).toContain(W.used);
      expect(titleOf(html)).toBe(W.used);
      lacks(html, [W.youreIn, "curl -fsSL"]);
    }
    const mine = await expectFace(await page(env, `/join/${CODE.used}`, { cookie: cookieFor(MHER, MHER_MAC) }));
    expect(textOf(mine)).toContain(W.youreIn);
    lacks(mine, [W.used]);
  });

  it("carries the mark on every face whatever mark.json says, and never reads mark.json (U6)", async () => {
    for (const [code, ua] of [[CODE.ani, UA.mac], [CODE.ani, UA.iphone], [CODE.own, UA.mac], [CODE.old, UA.mac], [CODE.used, UA.mac], [CODE.unknown, UA.mac]]) {
      const env = space({ objects: { ".shroom/mark.json": JSON.stringify({ on: false }) } });
      await expectFace(await page(env, `/join/${code}`, { ua }));
      expect(env.BUCKET.read()).not.toContain(".shroom/mark.json");
    }
  });

  it("opening never consumes: no face writes, and each does at most one roster read", async () => {
    for (const code of [CODE.ani, CODE.own, CODE.old, CODE.cancelled, CODE.used, CODE.unknown]) {
      const env = space();
      await expectFace(await page(env, `/join/${code}`, { cookie: cookieFor(M1, OWNER_AIR) }));
      expect(env.BUCKET.written(), code).toEqual([]);
      expect(env.BUCKET.read().filter((k) => k === ROSTER).length, code).toBeLessThanOrEqual(1);
    }
  });

  it("answers HEAD like GET, and refuses other methods 405", async () => {
    const head = await page(space(), `/join/${CODE.ani}`, { method: "HEAD" });
    expect(head.status).toBe(200);
    expect(head.headers.get("content-type")).toMatch(/^text\/html/);
    expect((await call(space(), "POST", `/join/${CODE.ani}`, { body: {} })).status).toBe(405);
  });

  it("a missing or unreadable roster, or a storage error, renders the transient line — never 'ask for a fresh one'", async () => {
    const throwing = space();
    const get = throwing.BUCKET.get;
    throwing.BUCKET.get = async (k, o) => { if (k === ROSTER) throw new Error("R2 internal error"); return get(k, o); };
    for (const env of [space({ roster: null }), space({ roster: "{broken" }), throwing]) {
      const html = await expectFace(await page(env, `/join/${CODE.ani}`));
      expect(textOf(html)).toContain(W.transient);
      expect(textOf(html)).not.toContain("fresh one");
      expect(env.BUCKET.written()).toEqual([]);
    }
  });
});

describe("escaping what people typed (§3.8, H10, #12)", () => {
  const INVITER = "<script>alert(1)</script>";
  const LABEL = `"><img src=x onerror=alert(1)>`;
  const evil = () => {
    const r = roster();
    for (const i of Object.values(r.invites)) { i.inviter = INVITER; if (i.label !== null) i.label = LABEL; }
    r.members[MHER].inviter = INVITER;
    return r;
  };
  const safe = (html, where) => {
    expect(html, where).not.toContain("<script>alert");
    expect(html, where).not.toContain("<img");
    expect(html, where).not.toContain(`"><img`);
    expect(html, where).not.toContain(`"&gt;&lt;img`); // the quote, too, is escaped
    const title = html.match(/<title>([\s\S]*?)<\/title>/i)?.[1] ?? "";
    expect(title, where).not.toMatch(/<|"/);
  };

  it.each([
    ["join-landing", CODE.ani, UA.mac, undefined, `${INVITER} invited you to record with shroom.`],
    ["owner view", CODE.ani, UA.mac, "owner", `This is the page ${LABEL} will see`],
    ["elsewhere", CODE.ani, UA.iphone, undefined, `${INVITER} invited you to record with shroom.`],
    ["expired", CODE.old, UA.mac, undefined, `This invite has expired — ask ${INVITER} for a fresh one.`],
    ["cancelled", CODE.cancelled, UA.mac, "owner", `This invite has expired — ask ${INVITER} for a fresh one.`],
    ["used", CODE.used, UA.mac, undefined, `If that wasn't you, ask ${INVITER} for a fresh one.`],
    ["own-Mac", CODE.own, UA.mac, "owner", W.ownHero],
  ])("%s renders the inviter and label as text, in the body and in <title>", async (_, code, ua, who, shown) => {
    const env = space({ roster: evil() });
    const html = await (await page(env, `/join/${code}`, { ua, cookie: who ? cookieFor(M1, OWNER_AIR) : undefined })).text();
    safe(html, `${code} ${who}`);
    if (shown) expect(textOf(html)).toContain(shown);
  });

  it("the hero in <title> is the escaped inviter line", async () => {
    const html = await (await page(space({ roster: evil() }), `/join/${CODE.ani}`)).text();
    expect(html).toContain(`<title>${escapeHtml(`${INVITER} invited you to record with shroom.`)}</title>`);
  });

  it("a member viewer's waiting line escapes the inviter too", async () => {
    const env = space({ roster: evil(), objects: { [metaKey(REC)]: metaOf(REC, { state: "uploading", media: null, progress: { done: 1, total: 4, updatedAt: iso(NOW - HOUR) } }), [HEALTH]: JSON.stringify({ v: 1, uploads: "blocked", since: iso(NOW), why: "refused", by: null }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    const html = await (await page(env, `/${REC}`, { cookie: cookieFor(MHER, MHER_MAC) })).text();
    safe(html, "owner page");
    expect(textOf(html)).toContain(`Waiting for ${INVITER}'s space — your link already works.`);
  });
});

// ════════════════════════════════════════════════════════════════════════════════════
// §3.6 · §3.7 · P0 #1, #3 — key confinement and secrets
// ════════════════════════════════════════════════════════════════════════════════════

describe("key confinement for every new route (§3.6, P0 #1)", () => {
  const FIXED = new Set([".shroom/mark.json", ".shroom/cookie.key", ROSTER, HEALTH]);
  const ID16 = "[A-Za-z0-9_-]{16}";
  /** Keys a request may touch: the fixed ones, its own grant, its own ids, its caller's counter. */
  const allowed = (k, { recs = [], caller = null }) =>
    FIXED.has(k)
    || /^\.shroom\/keys\/[0-9a-f]{64}$/.test(k)
    || recs.some((id) => k === ledgerKey(id) || k === metaKey(id) || k === goneKey(id))
    || (caller !== null && k === rateKey(caller.id));
  const hostile = [
    { route: "/peek", method: "POST", body: { code: "../../.shroom/cookie.key" } },
    { route: "/peek", method: "POST", body: { code: "%2E%2E%2Fcookie.key" } },
    { route: "/join", method: "POST", body: { code: CODE.ani, device: { id: "d_../../roster.json", verifier: NEW_MAC.verifier, name: "../x" }, previous: { id: "../keys/x", secret: "y" } } },
    { route: "/join", method: "POST", body: joinBody(CODE.ani, NEW_MAC, { previous: { id: "d_..%2F..%2Fcookie", secret: "x" } }) },
    { route: "/me", method: "GET", auth: { bearer: "Bearer d_../../roster.json.secret" } },
    { route: "/mint", method: "POST", auth: MHER_MAC, body: { rec: "../../.shroom/cookie.key", purpose: "upload" } },
    { route: "/mint", method: "POST", auth: MHER_MAC, body: { rec: REC, purpose: "upload", paths: [".shroom/"], key: ".shroom/cookie.key" }, recs: [REC], caller: MHER_MAC },
    { route: "/mint", method: "POST", auth: OWNER_AIR, body: { rec: REC, purpose: "manage" }, recs: [REC], caller: OWNER_AIR },
    { route: "/key", method: "POST", auth: MHER_MAC, body: { kHash: "../registry/roster.json" } },
    { route: "/key", method: "POST", auth: MHER_MAC, body: { kHash: sha(K) }, caller: MHER_MAC },
    { route: "/library", method: "POST", auth: MHER_MAC, body: { scope: "mine", known: { "../../.shroom/cookie.key": "e0" }, knownThrough: "../../.shroom/x", after: "../../.shroom/keys/" }, recs: [REC] },
    { route: "/library", method: "POST", auth: MHER_MAC, body: { scope: "mine", known: {}, knownThrough: null, after: null }, recs: [REC] },
    { route: "/library", method: "POST", auth: OWNER_AIR, body: { scope: "owners", known: {}, knownThrough: null, after: null } },
    { route: "/members", method: "GET", auth: OWNER_AIR },
    { route: "/invites", method: "POST", auth: OWNER_AIR, body: { id: "../../cookie.key", codeHash: "../x", kind: "member", label: "x", inviter: "y" } },
    { route: "/invites/..%2F..%2Fcookie.key", method: "DELETE", auth: OWNER_AIR },
    { route: "/members/..%2Fx", method: "DELETE", auth: OWNER_AIR },
    { route: "/devices/..%2F..%2Froster.json", method: "DELETE", auth: OWNER_AIR },
    { route: "/devices/self", method: "DELETE", auth: MHER_MAC },
    { route: "/mark", method: "POST", auth: OWNER_AIR, body: { on: false, key: ".shroom/cookie.key" } },
    { route: "/health", method: "POST", auth: MHER_MAC, body: { rec: "../../.shroom/cookie.key", uploads: "refused" } },
    { route: "/health", method: "POST", auth: MHER_MAC, body: { rec: REC, uploads: "refused" }, recs: [REC] },
    { route: "/gone", method: "POST", auth: MHER_MAC, body: { rec: "../../.shroom/cookie.key" } },
    { route: "/gone", method: "POST", auth: MHER_MAC, body: { rec: REC, key: ".shroom/cookie.key" }, recs: [REC] },
  ];

  it.each(hostile.map((h) => [`${h.method} ${h.route} ${JSON.stringify(h.body ?? "").slice(0, 60)}`, h]))("%s touches only its own keys", async (_, h) => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    env.BUCKET.ops.length = 0;
    const res = await api(env, h.method, h.route, { auth: h.auth?.bearer, body: h.body });
    expect(res.status, "the route answers").not.toBe(405);
    if (res.status !== 204) expect((await bodyOf(res))?.shroom, "a shroom-shaped answer").toBe(1);
    for (const o of env.BUCKET.ops) {
      if (o.op === "list") {
        expect(["meta/", ".shroom/registry/recs/"], `list ${o.key}`).toContain(o.key);
        const after = o.options?.startAfter;
        if (after !== undefined) expect(after, "startAfter").toMatch(new RegExp(`^(meta/${ID16}\\.json|\\.shroom/registry/recs/${ID16})$`));
      } else {
        expect(o.key, o.op).not.toContain("..");
        expect(allowed(o.key, h), `${o.op} ${o.key}`).toBe(true);
      }
    }
  });

  it("the join page reads only the roster and the cookie key", async () => {
    const env = space();
    for (const code of [CODE.ani, "..%2F..%2F.shroom%2Fcookie.key", "%2Fshroom%2Fcookie.key"]) {
      const res = await page(env, `/join/${code}`, { cookie: cookieFor(M1, OWNER_AIR) });
      expect(res.status).toBe(200);
      expect(res.headers.get("content-type")).toMatch(/^text\/html/);
    }
    expect(env.BUCKET.read()).toContain(ROSTER);
    for (const k of env.BUCKET.read()) expect([ROSTER, ".shroom/cookie.key"]).toContain(k);
  });

  // The cookie-bearing page routes, the redeem and the page POSTs (WORKER-INTERFACE §6:
  // the allowlist grows with every route): an owner's or a member's cookie, a valid or a
  // hostile ?k=, hostile POST bodies — each touches only its recording's own keys, its
  // ledger entry, a grant under .shroom/keys/<64 hex>, the roster, health, mark and the
  // cookie key.
  const pageAllowed = (k) => FIXED.has(k) || /^\.shroom\/keys\/[0-9a-f]{64}$/.test(k)
    || k === metaKey(REC) || k === ledgerKey(REC) || k === goneKey(REC) || k.startsWith(`rec/${REC}/`);
  const K_VALID = "pPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPpPp";
  const pageCalls = [
    { name: "GET /<id>, owner cookie", method: "GET", path: `/${REC}`, cookie: () => cookieFor(M1, OWNER_AIR) },
    { name: "GET /<id>, member cookie", method: "GET", path: `/${REC}`, cookie: () => cookieFor(MHER, MHER_MAC) },
    { name: "GET /<id>, a removed member's cookie", method: "GET", path: `/${REC}`, cookie: () => cookieFor(GONE, GONE_MAC) },
    { name: "GET /<id>?embed=1, owner cookie", method: "GET", path: `/${REC}?embed=1`, cookie: () => cookieFor(M1, OWNER_AIR) },
    { name: "GET /m/<id>/…, owner cookie", method: "GET", path: `/m/${REC}/master.m3u8`, cookie: () => cookieFor(M1, OWNER_AIR) },
    { name: "GET /<id>?k=<valid>", method: "GET", path: `/${REC}?k=${K_VALID}`, grant: true },
    { name: "GET /<id>?k=<valid>, member cookie held", method: "GET", path: `/${REC}?k=${K_VALID}`, grant: true, cookie: () => cookieFor(MHER, MHER_MAC) },
    { name: "GET /<id>?k=<unknown>", method: "GET", path: `/${REC}?k=${"u".repeat(43)}` },
    { name: "GET /<id>?k=<traversal>", method: "GET", path: `/${REC}?k=${encodeURIComponent("../../.shroom/cookie.key")}`, readsNothing: true },
    { name: "GET /<traversal>?k=<valid>", method: "GET", path: `/..%2F.shroom%2Fcookie.key?k=${K_VALID}`, grant: true },
    { name: "POST title, owner, hostile body", method: "POST", path: `/_shroom/title/${REC}`, cookie: () => cookieFor(M1, OWNER_AIR), body: { title: "x", id: "../../.shroom/cookie.key", key: ".shroom/registry/roster.json" } },
    { name: "POST title, member, hostile body", method: "POST", path: `/_shroom/title/${REC}`, cookie: () => cookieFor(MHER, MHER_MAC), body: { title: "../../.shroom/keys/x", rec: "../meta/x" } },
    { name: "POST title, traversal id", method: "POST", path: "/_shroom/title/..%2F..%2F.shroom%2Fx", cookie: () => cookieFor(M1, OWNER_AIR), body: { title: "x" } },
    { name: "POST mark, owner, hostile body", method: "POST", path: "/_shroom/mark", cookie: () => cookieFor(M1, OWNER_AIR), body: { on: false, key: ".shroom/cookie.key", at: "../x" } },
    { name: "POST mark, member", method: "POST", path: "/_shroom/mark", cookie: () => cookieFor(MHER, MHER_MAC), body: { on: false } },
  ];

  it.each(pageCalls.map((c) => [c.name, c]))("%s touches only its own keys", async (_, c) => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC), [`rec/${REC}/master.m3u8`]: "#EXTM3U" } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    if (c.grant) await env.BUCKET.put(`.shroom/keys/${sha(K_VALID)}`, JSON.stringify({ member: MHER, device: MHER_MAC.id, exp: new Date(NOW + 600_000).toISOString() }));
    env.BUCKET.ops.length = 0;
    const headers = { ...(c.cookie ? { cookie: c.cookie() } : {}), ...(c.method === "POST" ? { origin: `https://${HOST}` } : {}) };
    const res = await call(env, c.method, c.path, { headers, body: c.body });
    expect(res.status).toBeLessThan(500);
    if (c.readsNothing) expect(env.BUCKET.ops, "refused before any read").toEqual([]);
    else expect(env.BUCKET.ops.length, "the route reads something").toBeGreaterThan(0);
    for (const o of env.BUCKET.ops) {
      expect(o.op, o.key).not.toBe("list");
      expect(o.key, o.op).not.toContain("..");
      expect(pageAllowed(o.key), `${o.op} ${o.key}`).toBe(true);
    }
  });

  it("no minted credential ever names a .shroom/ path, whatever the caller", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    for (const [dev, purpose] of [[OWNER_AIR, "manage"], [OWNER_MAC, "manage"], [MHER_MAC, "upload"]]) {
      const b = await expectOk(await mint(env, dev, dev === MHER_MAC ? recId(7) : REC, purpose));
      expect(JSON.stringify(claimsOf(jwtOf(b)).paths)).not.toContain(".shroom");
    }
  });
});

describe("secrets never leave (§3.7, P0 #3)", () => {
  it("no response body or header carries R2_PARENT's secret, the cookie key or a device secret — on any route or error path", async () => {
    const secrets = [PARENT.secret, COOKIE_KEY, Buffer.from(COOKIE_KEY, "base64url").toString("hex"), MHER_MAC.secret, OWNER_AIR.secret, JSON.stringify(PARENT)];
    const seen = [];
    const keep = async (res) => {
      seen.push(await res.clone().text());
      for (const [k, v] of res.headers) seen.push(`${k}: ${v}`);
    };
    for (const env of [space(), space({ roster: null }), space({ roster: "{x" }), space({ parent: null })]) {
      for (const r of ROUTES) {
        await keep(await send(env, r));
        await keep(await send(env, r, { headers: { origin: "https://evil.example" } }));
        await keep(await api(env, r.method, r.route, { auth: `Bearer ${MHER_MAC.id}.${OWNER_AIR.secret}`, body: r.body }));
      }
      await keep(await enroll(env, enrollBody(NEW_MAC)));
      await keep(await enroll(env, enrollBody(NEW_MAC), { secret: OTHER_PARENT_SECRET }));
      await keep(await page(env, `/join/${CODE.ani}`, { cookie: cookieFor(M1, OWNER_AIR) }));
      await keep(await call(env, "GET", "/_shroom/ping"));
    }
    const all = seen.join("\n");
    expect(all, "the battery reached the routes").toContain('"shroom":1');
    expect(all, "and minted at least once").toContain("sessionToken");
    for (const s of secrets) expect(all.includes(s), s.slice(0, 12)).toBe(false);
  });
});

// ───────── review fixes (B37's amendments to the provisional design) ─────────

describe("review fixes", () => {
  it("a token outranks the m1 cap: 20 active m1 Macs, then enroll of a new Mac and of a removed one both go through (§3.7)", async () => {
    const r = roster();
    r.devices[OWNER_AIR.id] = { ...r.devices[OWNER_AIR.id], state: "removed", endedAt: iso(NOW - DAY) };
    for (let i = 0; i < 19; i++) {
      const d = deviceKey(`Intruder${i}`);
      r.devices[d.id] = { ...r.devices[OWNER_MAC.id], verifier: d.verifier, tokenKey: null, proofAt: null, via: rid("i", `X${i}`) };
    }
    const env = space({ roster: r });
    expect(Object.values(stored(env).devices).filter((d) => d.member === M1 && d.state === "active")).toHaveLength(20);
    // An own-Mac join is still capped.
    await expectError(await api(env, "POST", "/join", { body: joinBody(CODE.own, NEW_MAC, { name: "Mac mini" }) }), 409, "full");
    await expectOk(await enroll(env, enrollBody(NEW_MAC, "Mac Studio")));
    await expectOk(await enroll(env, enrollBody(OWNER_AIR, "MacBook Air")));
    const after = stored(env);
    expect(after.devices[NEW_MAC.id]).toMatchObject({ member: M1, state: "active" });
    expect(after.devices[OWNER_AIR.id]).toMatchObject({ member: M1, state: "active" });
  });

  it("a member's manage mint on a tombstone is 410 deleted; owner-class may still manage it", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "deleted", owner: MHER }) } });
    await ledger(env, REC, MHER, MHER_MAC.id);
    await expectError(await mint(env, MHER_MAC, REC, "upload"), 410, "deleted");
    await expectError(await mint(env, MHER_MAC, REC, "manage"), 410, "deleted");
    await expectOk(await mint(env, OWNER_AIR, REC, "manage"));
  });

  it("a member's ?k= never replaces a live owner-class cookie (login CSRF); the grant is spent either way", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    await expectOk(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(K) } }));
    const res = await call(env, "GET", `/${REC}?k=${K}`, { headers: { cookie: cookieFor(M1, OWNER_AIR) } });
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe(`/${REC}`);
    expect(res.headers.get("set-cookie")).toBeNull();
    expect(env.BUCKET.store.has(grantKey(sha(K)))).toBe(false);
    // The owner's own key still goes through over a member's cookie: no member's link
    // can lock an owner's browser out.
    await expectOk(await api(env, "POST", "/key", { auth: OWNER_AIR.bearer, body: { kHash: sha(K2) } }));
    const own = await call(env, "GET", `/${REC}?k=${K2}`, { headers: { cookie: cookieFor(MHER, MHER_MAC) } });
    expect(cookieClaims(own.headers.get("set-cookie"))).toMatchObject({ m: M1, d: OWNER_AIR.id });
    // A stale owner cookie (its Mac removed) doesn't hold: the member's grant lands.
    const K3 = "qQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQqQq";
    await expectOk(await api(env, "POST", "/key", { auth: MHER_MAC.bearer, body: { kHash: sha(K3) } }));
    const stale = await call(env, "GET", `/${REC}?k=${K3}`, { headers: { cookie: cookieFor(M1, deviceKey("Ghost")) } });
    expect(cookieClaims(stale.headers.get("set-cookie"))).toMatchObject({ m: MHER, d: MHER_MAC.id });
  });

  it("at the 500-invite cap, cancelled and ended rows give way, oldest first; pending ones never", async () => {
    const r = roster();
    const base = r.invites[INV.cancelled];
    for (let i = Object.keys(r.invites).length; i < 500; i++) {
      r.invites[rid("i", `C${i}`)] = { ...base, codeHash: sha(`c${i}`), endedAt: iso(NOW - DAY + i * SEC) };
    }
    const env = space({ roster: r });
    const b = await expectOk(await api(env, "POST", "/invites", { auth: OWNER_AIR.bearer, body: zed }), 201);
    expect(b.invite.id).toBe(zed.id);
    const after = stored(env);
    expect(Object.keys(after.invites)).toHaveLength(500);
    expect(after.invites[INV.ani].state).toBe("pending");
    expect(after.invites[INV.old].state).toBe("pending");
    expect(after.invites[INV.used].state).toBe("used"); // its Mac is active
  });

  it("a revived member's row reads the re-join date; the first join date stays in its history (§4.5)", async () => {
    const env = space();
    await expectOk(await api(env, "POST", "/join", { body: joinBody(CODE.ani, NEW_MAC, { previous: { id: GONE_MAC.id, secret: GONE_MAC.secret } }) }));
    const m = stored(env).members[GONE];
    expect(m).toMatchObject({ joinedAt: iso(NOW), reinvites: [{ invite: INV.ani, label: "Ani", at: iso(NOW), joinedBefore: iso(NOW - 25 * DAY) }] });
    const list = await expectOk(await api(env, "GET", "/members", { auth: OWNER_AIR.bearer }));
    expect(list.members).toContainEqual({ id: GONE, label: "Gohar", joinedAt: iso(NOW) });
  });

  it("a used own-Mac link: You're in for the Mac it let in, the own-Mac words for everyone else — never 'ask Bagrat'", async () => {
    const r = roster();
    const air = r.devices[OWNER_AIR.id];
    r.invites[INV.own] = { ...r.invites[INV.own], state: "used", used: { at: iso(NOW - 60 * SEC), member: M1, device: OWNER_AIR.id, verifier: air.verifier } };
    const env = space({ roster: r });
    for (const c of [undefined, cookieFor(M1, OWNER_MAC), cookieFor(MHER, MHER_MAC)]) {
      const t = textOf(await (await page(env, `/join/${CODE.own}`, { cookie: c })).text());
      expect(t).toContain(COPY.ownUsed);
      expect(t).not.toContain(W.used);
      expect(t).not.toContain(W.youreIn);
    }
    const mine = textOf(await (await page(env, `/join/${CODE.own}`, { cookie: cookieFor(M1, OWNER_AIR) })).text());
    expect(mine).toContain(W.youreIn);
  });

  it("Leave is idempotent: a retry after a lost reply is the same 204, and writes nothing", async () => {
    const env = space();
    expect((await api(env, "DELETE", "/devices/self", { auth: MHER_MAC.bearer })).status).toBe(204);
    const puts = rosterPuts(env).length;
    expect((await api(env, "DELETE", "/devices/self", { auth: MHER_MAC.bearer })).status).toBe(204);
    expect(rosterPuts(env)).toHaveLength(puts);
    // A removed Mac's Disconnect is done too; a stranger is still 401.
    expect((await api(env, "DELETE", "/devices/self", { auth: GONE_MAC.bearer })).status).toBe(204);
    await expectError(await api(env, "DELETE", "/devices/self", { auth: deviceKey("Stranger").bearer }), 401, "unauthorized");
    await expectError(await api(env, "DELETE", "/devices/self", { auth: `Bearer ${MHER_MAC.id}.${GONE_MAC.secret}` }), 401, "unauthorized");
  });

  it("the Library never makes up an end of the list: empty truncated pages past the follow budget → 503 busy", async () => {
    const env = space({ objects: { [metaKey(REC)]: metaOf(REC) } });
    const real = env.BUCKET.list.bind(env.BUCKET);
    env.BUCKET.list = async (o) => (o.prefix === "meta/" ? { objects: [], truncated: true, cursor: btoa("meta/"), delimitedPrefixes: [] } : real(o));
    for (const scope of ["everyone", "mine"]) {
      await expectError(await api(env, "POST", "/library", { auth: OWNER_AIR.bearer, body: { scope, known: {}, knownThrough: null, after: null } }), 503, "busy");
    }
  });

  it("ledger entries without metadata cost at most 40 parallel reads a call; the page ends at the last one resolved", async () => {
    const objects = {};
    for (let i = 0; i < 100; i++) {
      objects[ledgerKey(recId(i))] = JSON.stringify({ v: 1, member: MHER, device: MHER_MAC.id, at: iso(NOW - HOUR) });
      objects[metaKey(recId(i))] = metaOf(recId(i), { owner: MHER });
    }
    const env = space({ objects });
    const gets = () => env.BUCKET.ops.filter((o) => o.op === "get" && o.key.startsWith(".shroom/registry/recs/")).length;
    const owners = await expectOk(await api(env, "POST", "/library", { auth: OWNER_AIR.bearer, body: { scope: "owners", after: null } }));
    expect(gets()).toBe(40);
    expect(Object.keys(owners.owners)).toHaveLength(40);
    expect(owners.next).toBe(recId(39));
    expect(Object.values(owners.owners).every((m) => m === MHER)).toBe(true);
    const before = gets();
    const mine = await expectOk(await api(env, "POST", "/library", { auth: MHER_MAC.bearer, body: { scope: "mine", known: {}, knownThrough: null, after: null } }));
    expect(gets() - before).toBe(40);
    expect(mine.items).toHaveLength(40);
    expect(mine.items.every((i) => i.owner === MHER)).toBe(true);
    expect(mine.next).toBe(recId(39));
  });
});
