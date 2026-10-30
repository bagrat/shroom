// POST shroom.am/teams (site/teams.js; spec N88, DECISIONS B277): each check in its order, the
// one message and what it carries, and the route.
// Run: node --test site/test/teams.test.mjs
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { EMAIL, LIMITERS, MAX_BYTES, MAX_EMAIL, asked, handleTeams, parseSignup } from "../teams.js";
import { build } from "../build.mjs";
import worker from "../worker.js";

describe("a signup's shape", () => {
  for (const email of ["dana@example.com", "dana.k+shroom@mail.example.com", "  dana@example.com  ", "x@xn--e1a.example.org"]) {
    it(`takes ${JSON.stringify(email)}`, () => assert.equal(parseSignup({ email, website: "" })?.email, email.trim()));
  }

  for (const [why, body] of /** @type {[string, unknown][]} */ ([
    ["not an object", "dana@example.com"],
    ["an array", ["dana@example.com"]],
    ["no address", { website: "" }],
    ["an unknown field", { email: "dana@example.com", name: "Dana" }],
    ["an address that isn't text", { email: 7 }],
    ["no domain's dot", { email: "dana@example" }],
    ["no @", { email: "dana.example.com" }],
    ["two addresses", { email: "dana@example.com, sam@example.com" }],
    ["two @", { email: "dana@example@example.com" }],
    ["a space inside", { email: "dana k@example.com" }],
    ["a line break (a header smuggled in)", { email: "dana@example.com\r\nBcc: x@example.com" }],
    ["angle brackets", { email: "<dana@example.com>" }],
    ["an empty label", { email: "dana@example..com" }],
    ["an address past 254 characters", { email: `${"a".repeat(64)}@${"b".repeat(190)}.com` }],
    ["a honeypot that isn't text", { email: "dana@example.com", website: 1 }],
  ])) {
    it(`refuses ${why}`, () => assert.equal(parseSignup(body), null));
  }

  it("holds addresses to 254 characters", () => {
    const at = `${"a".repeat(60)}@${"b".repeat(MAX_EMAIL - 60 - 1 - 4)}.com`;
    assert.equal(at.length, MAX_EMAIL);
    assert.ok(EMAIL.test(at) && parseSignup({ email: at }));
  });
});

describe("the time it was asked", () => {
  it("reads in Armenia time, as the board's email does", () => {
    assert.equal(asked(new Date("2026-10-30T10:52:00Z")), "30 Oct, 14:52 AMT");
    assert.equal(asked(new Date("2026-12-31T21:05:00Z")), "1 Jan, 01:05 AMT");
  });
});

/** The site's rate limiters as site/wrangler.jsonc declares them, by binding name. */
const RATELIMITS = Object.fromEntries(
  JSON.parse(readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8").replace(/^\s*\/\/.*$/gm, "")).ratelimits.map(
    (/** @type {{ name: string, namespace_id: string, simple: { limit: number, period: number } }} */ r) => [r.name, r],
  ),
);

/** A rate limiter that counts by key up to the limit the config gives its binding. @param {string} name */
function limiter(name) {
  /** @type {Map<string, number>} */
  const seen = new Map();
  return {
    limit: async (/** @type {{ key: string }} */ { key }) => {
      seen.set(key, (seen.get(key) ?? 0) + 1);
      return { success: /** @type {number} */ (seen.get(key)) <= RATELIMITS[name].simple.limit };
    },
  };
}

function harness(/** @type {Record<string, unknown>} */ more = {}) {
  /** @type {{ from: string, to: string, raw: string }[]} */
  const sent = [];
  const env = {
    REPORT_TO: "inbox@example.com",
    EMAIL: { send: async (/** @type {any} */ m) => void sent.push(m) },
    EMAIL_MESSAGE: class {
      constructor(/** @type {string} */ from, /** @type {string} */ to, /** @type {string} */ raw) {
        Object.assign(this, { from, to, raw });
      }
    },
    ...Object.fromEntries(Object.keys(RATELIMITS).map((name) => [name, limiter(name)])),
    ...more,
  };
  const post = (/** @type {unknown} */ body, ip = "198.51.100.7") =>
    handleTeams(
      new Request("https://shroom.am/teams", { method: "POST", headers: { "content-type": "application/json", "cf-connecting-ip": ip }, body: typeof body === "string" ? body : JSON.stringify(body) }),
      env,
    );
  return { env, sent, post };
}

/** The message's text, decoded. @param {string} raw */
const text = (raw) => {
  const b64 = raw.split("\r\n\r\n")[1].replace(/\r\n/g, "");
  return new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
};

const DANA = { email: "dana@example.com", website: "" };

describe("POST /teams", () => {
  it("sends one plain message to the inbox in REPORT_TO, from report-sender@shroom.am, Reply-To the address", async () => {
    const { post, sent } = harness();
    const res = await post(DANA);
    assert.equal(res.status, 200);
    assert.deepEqual(await res.json(), { ok: true });
    assert.equal(res.headers.get("cache-control"), "no-store");
    assert.equal(sent.length, 1);
    assert.equal(sent[0].from, "report-sender@shroom.am");
    assert.equal(sent[0].to, "inbox@example.com");
    const raw = sent[0].raw;
    assert.match(raw, /^From: shroom\.am <report-sender@shroom\.am>\r\n/);
    assert.match(raw, /\r\nTo: <inbox@example\.com>\r\n/);
    assert.match(raw, /\r\nReply-To: <dana@example\.com>\r\n/);
    assert.match(raw, /\r\nSubject: Company sign-in: dana@example\.com\r\n/);
    assert.match(raw, /\r\nContent-Type: text\/plain; charset=utf-8\r\n/);
    assert.ok(!raw.includes("multipart"), "nothing attached");
    assert.match(text(raw), /^dana@example\.com asked to hear when company sign-in is ready\.\n\nSent from shroom\.am, \d{1,2} [A-Z][a-z]{2}, \d\d:\d\d AMT\. Nothing else is kept\.$/);
    assert.ok(!raw.includes("198.51.100.7"), "the visitor's IP isn't in it");
  });

  it("trims the address it sends", async () => {
    const { post, sent } = harness();
    assert.equal((await post({ email: "  dana@example.com " })).status, 200);
    assert.match(sent[0].raw, /\r\nSubject: Company sign-in: dana@example\.com\r\n/);
  });

  it("answers a filled honeypot as if sent, and sends nothing", async () => {
    const { post, sent } = harness();
    const res = await post({ email: "dana@example.com", website: "http://spam.example.com" });
    assert.equal(res.status, 200);
    assert.deepEqual(await res.json(), { ok: true });
    assert.equal(sent.length, 0);
  });

  it("refuses a body past 4 KB before reading what it says", async () => {
    const { post, sent } = harness();
    const res = await post("x".repeat(MAX_BYTES + 1));
    assert.equal(res.status, 413);
    assert.deepEqual(await res.json(), { error: "size" });
    assert.equal(sent.length, 0);
  });

  it("refuses what isn't one address", async () => {
    const { post, sent } = harness();
    for (const body of ["{not json", { email: "dana@example" }, { email: "a@example.com", extra: 1 }]) {
      const res = await post(body);
      assert.equal(res.status, 400);
      assert.deepEqual(await res.json(), { error: "shape" });
    }
    assert.equal(sent.length, 0);
  });

  it("is off without its inbox, its binding, either rate limiter, or with TEAMS=off", async () => {
    for (const more of [{ REPORT_TO: undefined }, { EMAIL: undefined }, { TEAMS: "off" }, { [LIMITERS.rate]: undefined }, { [LIMITERS.ceiling]: undefined }]) {
      const { post, sent } = harness(more);
      const res = await post(DANA);
      assert.equal(res.status, 503, JSON.stringify(Object.keys(more)));
      assert.deepEqual(await res.json(), { error: "off" });
      assert.equal(sent.length, 0);
    }
  });

  it("answers 429 past three a minute from one address, and sends nothing more", async () => {
    const { post, sent } = harness();
    const limit = RATELIMITS[LIMITERS.rate].simple.limit;
    assert.equal(limit, 3);
    for (let i = 0; i < limit; i++) assert.equal((await post(DANA)).status, 200);
    const res = await post(DANA);
    assert.equal(res.status, 429);
    assert.deepEqual(await res.json(), { error: "rate" });
    assert.equal(sent.length, limit);
    assert.equal((await post(DANA, "198.51.100.8")).status, 200, "another address still goes");
  });

  it("holds everyone to one ceiling, and never spends the reports' budget", async () => {
    const { post, env } = harness();
    const ceiling = RATELIMITS[LIMITERS.ceiling].simple.limit;
    for (let i = 0; i < ceiling; i++) assert.equal((await post(DANA, `198.51.100.${i + 1}`)).status, 200);
    assert.equal((await post(DANA, "203.0.113.1")).status, 429, "the ceiling is spent");
    const { success } = await env.REPORT_CEILING.limit({ key: "all" });
    assert.ok(success, "the feedback page's ceiling is untouched");
  });

  it("says so when the binding refuses", async () => {
    const { post } = harness({ EMAIL: { send: async () => Promise.reject(new Error("email to x not allowed")) } });
    const res = await post(DANA);
    assert.equal(res.status, 502);
    assert.deepEqual(await res.json(), { error: "send" });
  });
});

describe("the route", () => {
  const out = build(join(mkdtempSync(join(tmpdir(), "shroom-site-")), "public"));
  const ASSETS = {
    /** @param {Request} r */
    async fetch(r) {
      try {
        return new Response(readFileSync(join(out, decodeURIComponent(new URL(r.url).pathname))));
      } catch {
        return new Response("", { status: 404 });
      }
    },
  };

  it("takes only POST at /teams", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/teams"), { ASSETS });
    assert.equal(res.status, 405);
    assert.equal(res.headers.get("allow"), "POST");
  });

  it("routes POST /teams to the handler (off here: no inbox)", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/teams", { method: "POST", body: JSON.stringify(DANA) }), { ASSETS });
    assert.equal(res.status, 503);
  });

  it("reaches the Worker first, with its own pair of rate limiters, each in its own namespace", () => {
    const config = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");
    assert.match(config, /"run_worker_first": \[[^\]]*"\/teams"/);
    for (const name of Object.values(LIMITERS)) assert.ok(RATELIMITS[name], name);
    assert.deepEqual([RATELIMITS.TEAMS_RATE.simple, RATELIMITS.TEAMS_CEILING.simple], [{ limit: 3, period: 60 }, { limit: 30, period: 60 }]);
    const ids = Object.values(RATELIMITS).map((r) => r.namespace_id);
    assert.equal(new Set(ids).size, ids.length);
    assert.match(config, /"observability": \{ "enabled": false \}/, "request logs stay off");
  });

  it("says on the privacy page what a signup sends, and that nothing keeps it", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/privacy", { headers: { accept: "text/html" } }), { ASSETS });
    const html = await res.text();
    assert.match(html, /<li><b>Company sign-in\.<\/b> When you leave your email address on our home page to hear when company sign-in is ready, it goes once by email to the person who makes shroom, only so we can tell you when it’s ready\. shroom\.am keeps none of it\.<\/li>/);
  });
});
