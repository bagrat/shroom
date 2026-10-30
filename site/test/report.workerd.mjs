// POST /report under workerd (Miniflare), with a real `send_email` binding: the message is
// made by `cloudflare:email` and handed to the binding, which delivers only to its allowed
// destination — as Cloudflare's delivers only to the account's verified ones (DECISIONS B180).
// Needs the repo's node_modules. Run: node --test site/test/report.workerd.mjs
import { readFileSync } from "node:fs";
import { after, describe, it } from "node:test";
import assert from "node:assert/strict";
import { Miniflare } from "miniflare";

const SITE = new URL("..", import.meta.url).pathname;
const modules = ["worker.js", "report.js", "teams.js", "clean.js"].map((f) => ({ type: "esm", path: `${SITE}${f}`, contents: readFileSync(`${SITE}${f}`, "utf8") }));

/** The rate limiters site/wrangler.jsonc binds. */
const RATELIMITS = JSON.parse(readFileSync(`${SITE}wrangler.jsonc`, "utf8").replace(/^\s*\/\/.*$/gm, "")).ratelimits;

/** One Worker whose binding may send only to `allowed`, its secret naming `to`.
 * @param {string} to @param {string} allowed */
function site(to, allowed) {
  return new Miniflare({
    host: "127.0.0.1",
    port: 0,
    logRequests: false,
    cf: false,
    workers: [{
      config: {
        name: "shroom-site",
        compatibilityDate: "2026-10-01",
        manifest: {
          mainModule: "worker.js",
          modulesRoot: SITE,
          modules: Object.fromEntries(modules.map((m) => [m.path.slice(SITE.length), { type: "esm", contents: m.contents }])),
        },
        env: {
          REPORT_TO: { type: "text", value: to },
          EMAIL: { type: "send-email", allowedDestinationAddresses: [allowed] },
          ...Object.fromEntries(RATELIMITS.map((/** @type {any} */ r) => [r.name, { type: "rate-limit", namespace: r.namespace_id, simple: r.simple }])),
        },
      },
    }],
  });
}

const report = JSON.stringify({ kind: "problem", from: "app", words: "It stopped.", log: "  1 boot: launch login" });
/** @type {Miniflare[]} */
const running = [];
after(() => Promise.all(running.map((m) => m.dispose())));

describe("POST /report under workerd", () => {
  it("hands one message to the binding, which takes it for the verified inbox", async () => {
    const mf = site("inbox@example.com", "inbox@example.com");
    running.push(mf);
    const res = await mf.dispatchFetch("https://shroom.am/report", { method: "POST", body: report, headers: { "content-type": "application/json" } });
    assert.equal(res.status, 200, await res.clone().text());
    assert.deepEqual(await res.json(), { ok: true });
  });

  it("says it couldn't send when the binding refuses the address", async () => {
    const mf = site("someone-else@example.com", "inbox@example.com");
    running.push(mf);
    const res = await mf.dispatchFetch("https://shroom.am/report", { method: "POST", body: report, headers: { "content-type": "application/json" } });
    assert.equal(res.status, 502);
    assert.deepEqual(await res.json(), { error: "send" });
  });
});
