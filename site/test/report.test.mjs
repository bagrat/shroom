// POST shroom.am/report and the feedback page (site/report.js, site/clean.js; DECISIONS B180):
// the cleaning, each check in its order, the one message and what it carries, and the page.
// Run: node --test site/test/report.test.mjs
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { cleanLine, cleanLog, LOG_LINES, LINE_CHARS } from "../clean.js";
import { FEEDBACK_QUESTIONS, MAX_BYTES, handleReport, parseReport, subject } from "../report.js";
import { build } from "../build.mjs";
import worker from "../worker.js";

const CASES = JSON.parse(readFileSync(new URL("./clean-cases.json", import.meta.url), "utf8")).cases;

describe("cleaning a log line", () => {
  for (const [input, want] of CASES) {
    it(JSON.stringify(input.trim().slice(0, 60)), () => assert.equal(cleanLine(input), want));
  }

  it("cuts a window's own error short before cleaning it", () => {
    const out = cleanLine(`  10 webview popover: ${"x".repeat(600)}`);
    assert.ok(out.length < 200, out);
    assert.ok(out.endsWith("…"));
  });

  it("keeps no line longer than its bound", () => {
    assert.ok(cleanLine("y ".repeat(1000)).length <= LINE_CHARS + 1);
  });

  it("keeps the last lines only, and drops empty ones", () => {
    const log = Array.from({ length: LOG_LINES + 50 }, (_, i) => `${i} core step`).join("\n\n");
    const out = cleanLog(log).split("\n");
    assert.equal(out.length, LOG_LINES);
    assert.equal(out.at(-1), `${LOG_LINES + 49} core step`);
  });
});

const good = (/** @type {Record<string, unknown>} */ more = {}) => ({
  kind: "problem",
  from: "app",
  words: "The link never finished uploading.",
  versions: { shroom: "0.1.18", macos: "15.3.1", chip: "Apple M2", memory: "16 GB" },
  setup: "2026-10-12, 10:02–10:15 · owns its space · 2 Macs",
  log: "  1200 updater staged 0.1.18 (/Users/jane/Applications/shroom.app)\n  413020 uploader 7f3a…: request to https://v.example-owner.workers.dev/x timed out",
  ...more,
});

describe("a report's shape", () => {
  it("takes a report from the app and feedback from the page", () => {
    assert.ok(parseReport(good()));
    assert.ok(parseReport({ kind: "feedback", from: "page", answers: ["", "Yes", "", "", ""], turnstile: "t" }));
  });

  for (const [why, body] of /** @type {[string, unknown][]} */ ([
    ["not an object", "hello"],
    ["an unknown field", good({ ip: "1.2.3.4" })],
    ["an unknown kind", good({ kind: "praise" })],
    ["an unknown sender", good({ from: "cli" })],
    ["nothing said", good({ words: "  " })],
    ["more answers than questions", { kind: "feedback", from: "page", answers: ["a", "b", "c", "d", "e", "f"] }],
    ["an answer that isn't text", { kind: "feedback", from: "page", answers: [1] }],
    ["an unknown version field", good({ versions: { kernel: "x" } })],
    ["a reply-to that isn't an address", good({ reply_to: "me at home" })],
    ["words past their bound", good({ words: "x".repeat(8001) })],
  ])) {
    it(`refuses ${why}`, () => assert.equal(parseReport(body), null));
  }
});

/** What the inbox would get: the binding keeps each message. */
function harness(/** @type {Record<string, unknown>} */ more = {}) {
  /** @type {{ from: string, to: string, raw: string }[]} */
  const sent = [];
  const limits = { ip: 0, all: 0 };
  const env = {
    REPORT_TO: "inbox@example.com",
    TURNSTILE_SECRET: "secret",
    EMAIL: { send: async (/** @type {any} */ m) => void sent.push(m) },
    EMAIL_MESSAGE: class {
      constructor(/** @type {string} */ from, /** @type {string} */ to, /** @type {string} */ raw) {
        Object.assign(this, { from, to, raw });
      }
    },
    REPORT_RATE: { limit: async () => ({ success: ++limits.ip <= 3 }) },
    REPORT_CEILING: { limit: async () => ({ success: ++limits.all <= 30 }) },
    ...more,
  };
  /** @type {string[]} */
  const checked = [];
  const fetcher = /** @type {typeof fetch} */ (async (_url, init) => {
    const token = /** @type {FormData} */ (init?.body).get("response");
    checked.push(String(token));
    return new Response(JSON.stringify({ success: token === "person" }));
  });
  const post = (/** @type {unknown} */ body, ip = "198.51.100.7") =>
    handleReport(
      new Request("https://shroom.am/report", { method: "POST", headers: { "content-type": "application/json", "cf-connecting-ip": ip }, body: typeof body === "string" ? body : JSON.stringify(body) }),
      env,
      fetcher,
    );
  return { env, sent, post, checked };
}

/** The message's parts, decoded. @param {string} raw */
function decode(raw) {
  const parts = [...raw.matchAll(/Content-Transfer-Encoding: base64\r\n(?:[^\r\n]+\r\n)*\r\n([A-Za-z0-9+/=\r\n]+?)(?:\r\n--|\r\n$)/g)];
  return parts.map((p) => new TextDecoder().decode(Uint8Array.from(atob(p[1].replace(/\r\n/g, "")), (c) => c.charCodeAt(0))));
}

describe("POST /report", () => {
  it("sends one message to the inbox in REPORT_TO, from report-sender@shroom.am, with the cleaned log attached", async () => {
    const { post, sent } = harness();
    const res = await post(good({ reply_to: "you@example.com" }));
    assert.equal(res.status, 200);
    assert.deepEqual(await res.json(), { ok: true });
    assert.equal(sent.length, 1);
    assert.equal(sent[0].from, "report-sender@shroom.am");
    assert.equal(sent[0].to, "inbox@example.com");
    const raw = sent[0].raw;
    assert.match(raw, /^From: shroom reports <report-sender@shroom\.am>\r\n/);
    assert.match(raw, /\r\nTo: <inbox@example\.com>\r\n/);
    assert.match(raw, /\r\nReply-To: <you@example\.com>\r\n/);
    assert.match(raw, /\r\nSubject: \[shroom\] A problem · 0\.1\.18 · macOS 15\.3\.1|\r\nSubject: =\?UTF-8\?B\?/);
    assert.match(raw, /filename="report\.txt"/);
    const [text, log] = decode(raw);
    assert.match(text, /What happened:\nThe link never finished uploading\./);
    assert.match(text, /shroom 0\.1\.18 · macOS 15\.3\.1 · Apple M2 · 16 GB/);
    assert.match(text, /Sent from: the app \(Settings → Report a problem\)/);
    assert.equal(log, "  1200 updater staged 0.1.18 (~/Applications/shroom.app)\n  413020 uploader 7f3a…: request to <your space> timed out\n");
    assert.ok(!raw.includes("jane") && !raw.includes("example-owner"), "nothing the cleaning removes reaches the inbox");
  });

  it("tags the app's Send feedback as feedback, from its own door, with no log when it's left out (N60)", async () => {
    const { post, sent } = harness();
    const res = await post({ kind: "feedback", from: "app", words: "Copy the link the moment I stop.", versions: { shroom: "0.1.18" } });
    assert.equal(res.status, 200);
    const raw = sent[0].raw;
    assert.ok(raw.includes(`\r\nSubject: =?UTF-8?B?${Buffer.from("[shroom] Feedback · 0.1.18").toString("base64")}?=\r\n`), "the subject says feedback");
    assert.ok(!raw.includes('filename="report.txt"'), "nothing to attach");
    const [text] = decode(raw);
    assert.match(text, /Feedback:\nCopy the link the moment I stop\./);
    assert.match(text, /Sent from: the app \(Send feedback\)/);
    assert.match(text, /No log was sent\./);
  });

  it("labels the page's answers with their questions, and needs Turnstile's yes", async () => {
    const { post, sent, checked } = harness();
    const body = { kind: "feedback", from: "page", name: "Sam", answers: ["Yes, at the card step.", "", "", "", ""] };
    assert.equal((await post({ ...body, turnstile: "robot" })).status, 403);
    assert.equal(sent.length, 0);
    assert.equal((await post({ ...body, turnstile: "person" }, "198.51.100.8")).status, 200);
    assert.deepEqual(checked, ["robot", "person"]);
    const [text] = decode(sent[0].raw);
    assert.match(text, new RegExp(`1\\. ${FEEDBACK_QUESTIONS[0].replace(/[?.]/g, "\\$&")}\\nYes, at the card step\\.`));
    assert.ok(!text.includes("2. "), "an empty answer isn't sent");
    assert.match(text, /No log was sent\./);
    assert.equal(subject(parseReport({ ...body, turnstile: "x" }) ?? good()), "[shroom] Feedback from Sam");
  });

  it("refuses a body past 64 KB before reading what it says", async () => {
    const { post, sent } = harness();
    const res = await post("x".repeat(MAX_BYTES + 1));
    assert.equal(res.status, 413);
    assert.deepEqual(await res.json(), { error: "size" });
    assert.equal(sent.length, 0);
  });

  it("refuses what isn't a report", async () => {
    const { post, sent } = harness();
    assert.equal((await post("{not json")).status, 400);
    assert.equal((await post({ kind: "problem", from: "app" })).status, 400);
    assert.equal(sent.length, 0);
  });

  it("is off without its inbox, its binding, or with REPORTS=off; the page's also without Turnstile's secret", async () => {
    for (const more of [{ REPORT_TO: undefined }, { EMAIL: undefined }, { REPORTS: "off" }]) {
      const { post, sent } = harness(more);
      const res = await post(good());
      assert.equal(res.status, 503);
      assert.deepEqual(await res.json(), { error: "off" });
      assert.equal(sent.length, 0);
    }
    const { post } = harness({ TURNSTILE_SECRET: undefined });
    assert.equal((await post({ kind: "feedback", from: "page", answers: ["x"], turnstile: "person" })).status, 503);
    assert.equal((await post(good())).status, 200, "the app's report needs no Turnstile");
  });

  it("answers 429 past the rate, and sends nothing", async () => {
    const { post, sent } = harness();
    for (let i = 0; i < 3; i++) assert.equal((await post(good())).status, 200);
    const res = await post(good());
    assert.equal(res.status, 429);
    assert.deepEqual(await res.json(), { error: "rate" });
    assert.equal(sent.length, 3);
  });

  it("says so when the binding refuses", async () => {
    const { post } = harness({ EMAIL: { send: async () => Promise.reject(new Error("email to x not allowed")) } });
    const res = await post(good());
    assert.equal(res.status, 502);
    assert.deepEqual(await res.json(), { error: "send" });
  });
});

describe("the site's routes", () => {
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

  it("takes only POST at /report", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/report"), { ASSETS });
    assert.equal(res.status, 405);
    assert.equal(res.headers.get("allow"), "POST");
  });

  it("routes POST /report to the handler (off here: no inbox)", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/report", { method: "POST", body: JSON.stringify(good()) }), { ASSETS });
    assert.equal(res.status, 503);
  });

  it("serves /feedback with Bagrat's five questions, and clean.js beside it", async () => {
    const res = await worker.fetch(new Request("https://shroom.am/feedback", { headers: { accept: "text/html" } }), { ASSETS });
    assert.equal(res.status, 200);
    const html = await res.text();
    assert.equal(FEEDBACK_QUESTIONS.length, 5);
    for (const q of FEEDBACK_QUESTIONS) assert.ok(html.includes(q.replace(/'/g, "&#39;")) || html.includes(q), q);
    assert.ok(!html.includes("What did you record"), "the question on what was recorded is gone (Bagrat, 2026-10-16)");
    assert.ok(html.includes("5. Was anything on the site unclear"), "renumbered");
    assert.match(html, /import \{ cleanLog \} from "\/clean\.js"/);
    assert.equal(readFileSync(join(out, "clean.js"), "utf8"), readFileSync(new URL("../clean.js", import.meta.url), "utf8"));
  });

  it("names no inbox in the Worker's config", () => {
    const config = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");
    assert.match(config, /"send_email": \[\{ "name": "EMAIL" \}\]/);
    assert.ok(!/destination_address|allowed_destination_addresses/.test(config.replace(/\/\/.*$/gm, "")));
  });
});
