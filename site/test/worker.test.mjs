// shroom.am's Worker (site/worker.js) over what site/build.mjs puts together: the landing,
// the install line, the page for coding agents, What's new, the questions, privacy and terms
// pages, and nothing else.
// Run: node --test site/test/worker.test.mjs
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { build, FOOTER, HEADER, noteVersions, whatsNewPage } from "../build.mjs";
import worker, { byteRange, clientKind, refTag, referrerHost, wantsMarkdown } from "../worker.js";

const ROOT = new URL("../..", import.meta.url).pathname;
const out = build(join(mkdtempSync(join(tmpdir(), "shroom-site-")), "public"));

// The assets binding, as Cloudflare's behaves with html_handling "none": a file at its
// own path, or a 404.
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

const BROWSERS = {
  chrome: "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
  safari: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
};
// Claude Code's WebFetch, measured 2026-10-13 (history/proposals/shroom-am-worker.md §2.3).
const CLAUDE_CODE = "text/markdown, text/html, */*";

/** @type {{ indexes: string[], blobs: string[] }[]} */
let counted = [];
// The Analytics Engine binding: keeps what's written, to be read back.
const STATS = { writeDataPoint: (/** @type {{ indexes: string[], blobs: string[] }} */ p) => void counted.push(p) };

const get = (/** @type {string} */ path, /** @type {Record<string, string>} */ headers = {}, method = "GET", host = "shroom.am") =>
  worker.fetch(new Request(`https://${host}${path}`, { method, headers }), { ASSETS, STATS });

const file = (/** @type {string} */ p) => readFileSync(join(ROOT, p), "utf8");

describe("the install line", () => {
  it("is the installer itself, as plain text — never a redirect", async () => {
    for (const accept of [BROWSERS.chrome, "*/*", CLAUDE_CODE, "text/markdown"]) {
      const res = await get("/install", { accept });
      assert.equal(res.status, 200);
      assert.equal(res.headers.get("content-type"), "text/plain; charset=utf-8");
      assert.equal(await res.text(), file("installer/install.sh"));
    }
  });

  it("answers HEAD with no body", async () => {
    const res = await get("/install", {}, "HEAD");
    assert.equal(res.status, 200);
    assert.equal(await res.text(), "");
  });
});

describe("the landing", () => {
  it("is site/index.html to a browser", async () => {
    const res = await get("/", { accept: BROWSERS.chrome });
    assert.equal(res.headers.get("content-type"), "text/html; charset=utf-8");
    assert.equal(res.headers.get("vary"), "Accept");
    assert.equal(await res.text(), file("site/index.html"));
  });

  it("is the markdown to a coding agent", async () => {
    const res = await get("/", { accept: CLAUDE_CODE });
    assert.equal(res.headers.get("content-type"), "text/markdown; charset=utf-8");
    assert.equal(await res.text(), file("site/agent.md"));
  });
});

describe("the agent page", () => {
  it("is the markdown to an agent, and to anything that isn't a browser", async () => {
    for (const headers of [{ accept: CLAUDE_CODE }, { accept: "*/*" }, {}, { accept: "text/markdown" }, { accept: "text/plain" }]) {
      const res = await get("/agent", headers);
      assert.equal(res.headers.get("content-type"), "text/markdown; charset=utf-8", JSON.stringify(headers));
      assert.equal(res.headers.get("vary"), "Accept");
      assert.equal(await res.text(), file("site/agent.md"));
    }
  });

  it("is a page to a browser, with the markdown on it word for word", async () => {
    const res = await get("/agent", { accept: BROWSERS.safari });
    assert.equal(res.headers.get("content-type"), "text/html; charset=utf-8");
    const html = await res.text();
    const shown = html.match(/<pre>([\s\S]*)<\/pre>/)?.[1]
      .replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&amp;/g, "&");
    assert.equal(shown, file("site/agent.md"));
  });

  it("is a page to a link preview, whatever it accepts", async () => {
    for (const ua of ["Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)", "Discordbot/2.0", "Twitterbot/1.0", "facebookexternalhit/1.1"]) {
      const res = await get("/agent", { "user-agent": ua, accept: "*/*" });
      assert.equal(res.headers.get("content-type"), "text/html; charset=utf-8", ua);
      assert.match(await res.text(), /<meta property="og:title"/);
    }
  });

  it("is always the markdown at /agent.md", async () => {
    const res = await get("/agent.md", { accept: BROWSERS.chrome });
    assert.equal(res.headers.get("content-type"), "text/markdown; charset=utf-8");
    assert.equal(await res.text(), file("site/agent.md"));
  });
});

describe("the guide to the command line", () => {
  it("is the markdown to an agent, the page to a browser, the markdown at /cli.md", async () => {
    const md = await get("/cli", { accept: CLAUDE_CODE });
    assert.equal(md.headers.get("content-type"), "text/markdown; charset=utf-8");
    assert.equal(await md.text(), file("site/cli.md"));
    const page = await get("/cli", { accept: BROWSERS.safari });
    assert.equal(page.headers.get("content-type"), "text/html; charset=utf-8");
    const html = await page.text();
    assert.match(html, /<meta property="og:url" content="https:\/\/shroom.am\/cli">/);
    assert.match(html, /when you ask it to do something with your shroom videos/);
    assert.equal(await (await get("/cli.md", { accept: BROWSERS.chrome })).text(), file("site/cli.md"));
  });
});

describe("what's new", () => {
  const notes = (/** @type {string} */ v) => file(`docs/release-notes/${v}.md`).split("\n").filter((l) => l.startsWith("- ")).map((l) => l.slice(2));
  const newest = noteVersions(join(ROOT, "docs/release-notes"))[0];

  it("shows a release's list, under its version, in the landing's look", async () => {
    const res = await get("/whats-new/0.1.12", { accept: BROWSERS.safari });
    assert.equal(res.status, 200);
    assert.equal(res.headers.get("content-type"), "text/html; charset=utf-8");
    const page = await res.text();
    assert.match(page, /<h1>What's new<\/h1>/);
    assert.match(page, /shroom 0\.1\.12/);
    for (const line of notes("0.1.12")) assert.ok(page.includes(`<li>${line.replace(/&/g, "&amp;")}</li>`), line);
    assert.ok(page.includes(HEADER), "the pages' header");
    assert.match(page, /href="\/favicon\.svg"/, "the icons from the root, wherever the page sits");
    assert.ok(!/Under the hood|pull\/\d+/.test(page), "only what's new, never the list of merged work");
  });

  it("/whats-new is the newest release's", async () => {
    const res = await get("/whats-new", { accept: BROWSERS.chrome });
    assert.equal(res.status, 200);
    assert.equal(await res.text(), await (await get(`/whats-new/${newest}`)).text());
  });

  it("a version with no notes goes to the newest — never a 404", async () => {
    for (const path of ["/whats-new/0.1.11", "/whats-new/0.0.1", "/whats-new/latest", "/whats-new/..%2F..%2Fwrangler.jsonc", "/whats-new/0.1.12.html"]) {
      const res = await get(path);
      assert.equal(res.status, 302, path);
      assert.equal(res.headers.get("location"), "/whats-new", path);
    }
  });

  it("escapes the notes' words", () => {
    const page = whatsNewPage(file("site/index.html"), "9.9.9", "A word first.\n- <b>bold</b> & more\n");
    assert.ok(page.includes("<li>&lt;b&gt;bold&lt;/b&gt; &amp; more</li>"));
    assert.ok(page.includes('<p class="lead">A word first.</p>'));
  });

  it("orders versions as numbers, newest first", () => {
    const dir = mkdtempSync(join(tmpdir(), "shroom-notes-"));
    for (const v of ["0.1.9", "0.1.12", "0.2.0", "0.1.10"]) writeFileSync(join(dir, `${v}.md`), "- x\n");
    writeFileSync(join(dir, "README.md"), "");
    assert.deepEqual(noteVersions(dir), ["0.2.0", "0.1.12", "0.1.10", "0.1.9"]);
  });
});

describe("the rules for markdown", () => {
  const url = new URL("https://shroom.am/agent");
  const md = (/** @type {string | null} */ accept) => wantsMarkdown(new Request(url, { headers: accept === null ? {} : { accept } }), url);
  it("follows the Accept header's weights", () => {
    assert.equal(md(BROWSERS.chrome), false);
    assert.equal(md(CLAUDE_CODE), true);
    assert.equal(md("text/html;q=0.5, text/markdown"), true);
    assert.equal(md("text/html, text/markdown;q=0.5"), false);
    assert.equal(md("text/markdown;q=0, text/html"), false);
    assert.equal(md("*/*"), true);
    assert.equal(md(null), true);
  });
});

describe("nothing else", () => {
  it("has no other address", async () => {
    for (const path of ["/_/install.sh", "/_/agent.md", "/_/agent.html", "/_/cli.md", "/index.html.bak", "/install.sh", "/v/x/y"]) {
      assert.equal((await get(path)).status, 404, path);
    }
  });

  it("takes no writes", async () => {
    assert.equal((await get("/install", {}, "POST")).status, 405);
  });

  it("sends www to shroom.am, keeping the path", async () => {
    const res = await get("/install?x=1", {}, "GET", "www.shroom.am");
    assert.equal(res.status, 301);
    assert.equal(res.headers.get("location"), "https://shroom.am/install?x=1");
  });
});

for (const [title, stem] of /** @type {const} */ ([["the hero", "hero-2"], ["the how-to video", "how-to"]])) describe(title, () => {
  const mp4 = readFileSync(join(ROOT, `site/${stem}.mp4`));
  const size = mp4.byteLength;
  // The assets binding as Cloudflare's answers a file: with its length, in pieces, and the
  // whole file whatever Range it's asked for.
  const STREAMED = {
    /** @param {Request} r */
    async fetch(r) {
      if (new URL(r.url).pathname !== `/_/${stem}.mp4`) return ASSETS.fetch(r);
      let at = 0;
      const body = new ReadableStream({
        pull(c) {
          if (at >= size) return c.close();
          c.enqueue(new Uint8Array(mp4.subarray(at, at + 65536)));
          at += 65536;
        },
      });
      return new Response(body, { headers: { "content-length": String(size) } });
    },
  };
  for (const [name, assets] of /** @type {const} */ ([["a file sent with its length", STREAMED], ["a file sent without one", ASSETS]])) {
    const ask = (/** @type {Record<string, string>} */ headers = {}, method = "GET") =>
      worker.fetch(new Request(`https://shroom.am/${stem}.mp4`, { method, headers }), { ASSETS: assets, STATS });

    it(`answers the whole file, and says it takes ranges (${name})`, async () => {
      const res = await ask();
      assert.equal(res.status, 200);
      assert.equal(res.headers.get("content-type"), "video/mp4");
      assert.equal(res.headers.get("accept-ranges"), "bytes");
      assert.equal(res.headers.get("content-length"), String(size));
      assert.ok(Buffer.from(await res.arrayBuffer()).equals(mp4));
    });

    it(`answers a range with exactly its bytes, 206 (${name})`, async () => {
      for (const [range, first, last] of /** @type {[string, number, number][]} */ ([["bytes=0-1", 0, 1], ["bytes=100000-299999", 100000, 299999], [`bytes=${size - 10}-`, size - 10, size - 1], ["bytes=-500", size - 500, size - 1], [`bytes=5-${size + 99}`, 5, size - 1]])) {
        const res = await ask({ range });
        assert.equal(res.status, 206, range);
        assert.equal(res.headers.get("content-range"), `bytes ${first}-${last}/${size}`, range);
        assert.equal(res.headers.get("content-length"), String(last - first + 1), range);
        assert.ok(Buffer.from(await res.arrayBuffer()).equals(mp4.subarray(first, last + 1)), range);
      }
    });

    it(`refuses a range past the end, 416 (${name})`, async () => {
      const res = await ask({ range: `bytes=${size}-` });
      assert.equal(res.status, 416);
      assert.equal(res.headers.get("content-range"), `bytes */${size}`);
    });

    it(`answers HEAD with the length and no body (${name})`, async () => {
      const res = await ask({}, "HEAD");
      assert.equal(res.status, 200);
      assert.equal(res.headers.get("content-length"), String(size));
      assert.equal(await res.text(), "");
    });
  }

  it("reads a Range header the way players send it", () => {
    assert.deepEqual(byteRange("bytes=0-", 10), [0, 9]);
    assert.equal(byteRange(null, 10), null);
    assert.equal(byteRange("bytes=0-1,4-5", 10), null, "several ranges: the whole file");
    assert.equal(byteRange("items=0-1", 10), null);
    assert.equal(byteRange("bytes=-0", 10), "unsatisfiable");
    assert.deepEqual(byteRange("bytes=-20", 10), [0, 9]);
  });

  it("isn't counted: the landing's visit is", async () => {
    counted = [];
    await (await get(`/${stem}.mp4`, { range: "bytes=0-1" })).arrayBuffer();
    assert.deepEqual(counted, []);
  });

  it("is an address the Worker answers first", () => {
    assert.match(file("site/wrangler.jsonc"), new RegExp(`"run_worker_first": \\[[^\\]]*"/${stem}\\.mp4"`));
  });
});

describe("what's counted", () => {
  const CHROME_UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
  const count = async (/** @type {string} */ path, /** @type {Record<string, string>} */ headers = {}, method = "GET", host = "shroom.am") => {
    counted = [];
    const res = await get(path, headers, method, host);
    await res.arrayBuffer();
    return counted;
  };

  it("is one row per answer: the address, the answer, the kind of client, the linking host, the country", async () => {
    const rows = await count("/", { accept: BROWSERS.chrome, "user-agent": CHROME_UA, referer: "https://news.ycombinator.com/item?id=123" });
    assert.deepEqual(rows, [{ indexes: ["/"], blobs: ["/", "html", "browser", "news.ycombinator.com", "", ""] }]);
  });

  it("tells running the installer from reading it", async () => {
    assert.deepEqual((await count("/install", { "user-agent": "curl/8.7.1", accept: "*/*" }))[0].blobs, ["/install", "text", "curl", "", "", ""]);
    assert.deepEqual((await count("/install", { "user-agent": CHROME_UA, accept: BROWSERS.chrome }))[0].blobs, ["/install", "text", "browser", "", "", ""]);
  });

  it("tells an agent from a browser at /agent", async () => {
    assert.deepEqual((await count("/agent", { "user-agent": "Claude-User (claude-code/2.1.300)", accept: CLAUDE_CODE }))[0].blobs, ["/agent", "markdown", "agent:claude", "", "", ""]);
    assert.deepEqual((await count("/agent", { "user-agent": CHROME_UA, accept: BROWSERS.safari }))[0].blobs, ["/agent", "html", "browser", "", "", ""]);
    assert.deepEqual((await count("/agent", { "user-agent": "Slackbot-LinkExpanding 1.0", accept: "*/*" }))[0].blobs, ["/agent", "html", "preview", "", "", ""]);
  });

  it("keeps nothing that names a person: no IP, no User-Agent, no full address, no cookie", async () => {
    const rows = await count("/", {
      accept: BROWSERS.chrome, "user-agent": CHROME_UA, referer: "https://example.com/private/path?who=me",
      "cf-connecting-ip": "203.0.113.7", "x-forwarded-for": "203.0.113.7", cookie: "id=abc",
    });
    const kept = JSON.stringify(rows);
    for (const secret of ["203.0.113.7", "Chrome/141", "private", "who=me", "id=abc"]) assert.ok(!kept.includes(secret), secret);
    assert.equal(rows[0].blobs.length, 6);
  });

  it("counts nothing but a GET of one of the four addresses", async () => {
    assert.deepEqual(await count("/install", {}, "HEAD"), []);
    assert.deepEqual(await count("/install", {}, "POST"), []);
    assert.deepEqual(await count("/nope"), []);
    assert.deepEqual(await count("/", {}, "GET", "www.shroom.am"), []);
  });

  it("keeps a link's ?ref= tag, and the install line works with one", async () => {
    const rows = await count("/install?ref=newsletter", { "user-agent": "curl/8.7.1" });
    assert.deepEqual(rows[0].blobs, ["/install", "text", "curl", "", "", "newsletter"]);
    const res = await get("/install?ref=newsletter");
    assert.equal(await res.text(), file("installer/install.sh"));
    assert.equal((await count("/agent?utm_source=Newsletter", { accept: CLAUDE_CODE }))[0].blobs[5], "newsletter");
    assert.equal((await count("/?ref=x&utm_source=y"))[0].blobs[5], "x", "ref wins over utm_source");
  });

  it("answers the same with no counter, or a broken one", async () => {
    const plain = await worker.fetch(new Request("https://shroom.am/install"), { ASSETS });
    assert.equal(await plain.text(), file("installer/install.sh"));
    const broken = { writeDataPoint() { throw new Error("down"); } };
    const res = await worker.fetch(new Request("https://shroom.am/install"), { ASSETS, STATS: broken });
    assert.equal(res.status, 200);
    assert.equal(await res.text(), file("installer/install.sh"));
  });

  it("names a client by a fixed label, and a linking page by its host alone", () => {
    const req = (/** @type {Record<string, string>} */ h) => new Request("https://shroom.am/", { headers: h });
    assert.equal(clientKind(req({ "user-agent": "Wget/1.21.4" })), "wget");
    assert.equal(clientKind(req({ "user-agent": "Mozilla/5.0 AppleWebKit/537.36; compatible; ChatGPT-User/1.0" })), "agent:openai");
    assert.equal(clientKind(req({ "user-agent": "Googlebot/2.1" })), "bot");
    assert.equal(clientKind(req({ "user-agent": "python-requests/2.32" })), "other");
    assert.equal(clientKind(req({})), "other");
    assert.equal(referrerHost(req({ referer: "https://shroom.am/agent" })), "");
    assert.equal(referrerHost(req({ referer: "not a url" })), "");
    assert.equal(referrerHost(req({ referer: "https://X.com/someone/status/1" })), "x.com");
    const tag = (/** @type {string} */ q) => refTag(new URL(`https://shroom.am/${q}`));
    assert.equal(tag("?ref=Team-Wiki"), "team-wiki");
    assert.equal(tag("?ref=" + "a".repeat(40)), "a".repeat(32), "cut to 32");
    assert.equal(tag("?ref=a%20b"), "", "a space: no tag");
    assert.equal(tag("?ref=<script>"), "");
    assert.equal(tag("?ref=you@example.com"), "", "an address: no tag");
    assert.equal(tag(""), "");
  });
});

describe("the questions, privacy and terms pages", () => {
  const landing = file("site/index.html");
  for (const name of ["faq", "privacy", "terms"]) {
    it(`/${name} is site/pages/${name}.html in the landing's look`, async () => {
      const res = await get(`/${name}`, { accept: "*/*" });
      assert.equal(res.status, 200);
      assert.equal(res.headers.get("content-type"), "text/html; charset=utf-8", "a page to everyone, agents too");
      const page = await res.text();
      const source = file(`site/pages/${name}.html`);
      for (const li of source.match(/<li[\s\S]*?<\/li>/g) ?? []) assert.ok(page.includes(li), li);
      assert.ok(page.includes(source.match(/<h1>[^<]*<\/h1>/)?.[0] ?? "no h1"));
      assert.ok(page.includes(HEADER), "the pages' header");
      assert.ok(page.includes(FOOTER), "the pages' footer");
      assert.match(page, /<meta property="og:url" content="https:\/\/shroom\.am\/[a-z]+">/);
    });
  }

  it("closes with the landing's own links and visits line, and no Trouble, questions (B192)", () => {
    const hrefs = (/** @type {string} */ h) => [...h.matchAll(/<a href="([^"]+)">/g)].map((m) => m[1]);
    const foot = landing.match(/<footer class="foot">[\s\S]*?<\/footer>/)?.[0] ?? "";
    assert.deepEqual(hrefs(FOOTER), hrefs(foot));
    assert.ok(FOOTER.includes("<p>We count visits to this page — no cookies, and nothing that tells us who you are.</p>"));
    assert.doesNotMatch(FOOTER, /Trouble, questions/);
  });

  it("each is linked from the landing's foot and the pages' footer", () => {
    const foot = landing.match(/<footer class="foot">[\s\S]*?<\/footer>/)?.[0] ?? "";
    for (const name of ["faq", "privacy", "terms"]) {
      assert.match(foot, new RegExp(`<a href="/${name}">`));
      assert.match(FOOTER, new RegExp(`<a href="/${name}">`));
    }
  });

  it("is counted like the others", async () => {
    counted = [];
    await get("/privacy", { accept: BROWSERS.chrome });
    assert.equal(counted.length, 1);
    assert.deepEqual(counted[0].indexes, ["/privacy"]);
  });

  it("puts every coming-soon line's pill on its own line's end", () => {
    const faq = file("site/pages/faq.html");
    const soon = faq.match(/<li class="soon">[\s\S]*?<\/li>/g) ?? [];
    assert.equal(soon.length, 4, "editing, captions and chapters, view counts, deleting from the page");
    for (const li of soon) assert.match(li, /^<li class="soon"><span>[^<]+<\/span><em class="pill">Coming soon<\/em><\/li>$/);
    const lists = [...faq.matchAll(/<ul class="points limits">([\s\S]*?)<\/ul>/g)].map((m) => m[1]);
    assert.ok(lists[0]?.trim().startsWith('<li class="soon">'), "what's coming sits at the top");
    assert.ok(lists.slice(1).every((l) => !l.includes('class="soon"')), "the rest, under Also not yet, carry no pill");
  });
});

describe("/next, where the hero waited until it became /", () => {
  it("sends anyone who kept the link to /, for good, uncounted", async () => {
    counted = [];
    for (const accept of [BROWSERS.chrome, "*/*", "text/markdown"]) {
      const res = await get("/next", { accept });
      assert.equal(res.status, 301);
      assert.equal(res.headers.get("location"), "/");
    }
    assert.deepEqual(counted, []);
  });

  it("leaves nothing of the page behind", () => {
    assert.ok(!existsSync(join(out, "_/next.html")), "the build has no next.html");
    assert.ok(!existsSync(join(ROOT, "site/next.html")));
  });
});
