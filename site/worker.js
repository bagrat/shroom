// shroom.am — the landing, the install line and the page for coding agents
// (dev/proposals/shroom-am-worker.md).
//
// The favicons and the link preview card (og.png) are static files Cloudflare serves without
// running this script. It runs only for the paths site/wrangler.jsonc names
// (`run_worker_first`), and answers from the files site/build.mjs puts in site/public:
//
//   /            the landing — or, to a coding agent, the same page in markdown
//   (any of them may carry ?ref=<tag>, which is counted and changes nothing else)
//   /install     installer/install.sh as plain text: one file, never a redirect, so a
//                browser shows the source and curl gets the script
//   /agent       what a coding agent reads (markdown); a browser gets it as a page
//   /agent.md    the markdown, whoever asks
//   /whats-new   the newest release's "What's new" (DECISIONS B152)
//   /whats-new/<version>
//                that release's — what the app's "What's new" opens; a version with no
//                notes (never released, or from before them) goes to /whats-new, never a 404
//   /faq, /privacy, /terms
//                questions and known limits, privacy, terms: site/pages, as pages
//   www.…        a permanent redirect to shroom.am
//
// Nothing else exists: the files behind these answers (/_/…) aren't addresses.
//
// Each answer to a GET of one of these addresses is counted: one row in the account's
// Analytics Engine (`STATS`), holding which address, which answer, a coarse kind of client,
// the host of the page that linked here, the country, and the link's own `?ref=` tag (or
// `utm_source`) — never an IP, a User-Agent, a full referring address, a cookie or anything
// that ties two requests together.

const APEX = "shroom.am";

// Link previews (Slack, X, Facebook and iMessage, Discord, LinkedIn, …) need the page, not
// its markdown. The one place a User-Agent decides anything: it finds crawlers, never agents.
const UNFURLERS = /slackbot|twitterbot|facebookexternalhit|facebookcatalog|discordbot|linkedinbot|telegrambot|whatsapp|skypeuripreview|redditbot|embedly|mastodon/i;

/** Markdown or the page, for one request. `.md` always means markdown; a link preview
 * always gets the page; then the Accept header: markdown when it asks for it at least as
 * much as for HTML, the page when it names HTML (every browser does), and markdown for
 * anything else — curl, wget and agents' plain fetchers send *\/* or nothing.
 * @param {Request} request @param {URL} url */
export function wantsMarkdown(request, url) {
  if (url.pathname.endsWith(".md")) return true;
  if (UNFURLERS.test(request.headers.get("user-agent") ?? "")) return false;
  const accept = request.headers.get("accept") ?? "";
  const q = (/** @type {string} */ type) => {
    let best = -1; // -1: not named at all
    for (const part of accept.split(",")) {
      const [media, ...params] = part.trim().toLowerCase().split(";").map((s) => s.trim());
      if (media !== type) continue;
      const qp = params.find((p) => p.startsWith("q="));
      best = Math.max(best, qp ? Number(qp.slice(2)) || 0 : 1);
    }
    return best;
  };
  const md = q("text/markdown"), html = q("text/html");
  if (md > 0 && md >= html) return true;
  if (html > 0) return false;
  return true;
}

const HTML = "text/html; charset=utf-8";
const MARKDOWN = "text/markdown; charset=utf-8";
const TEXT = "text/plain; charset=utf-8";

// The kinds of client a row may name; the User-Agent itself is never kept. Agents first:
// some of their fetchers also say "Mozilla".
const AGENTS = /** @type {[string, RegExp][]} */ ([
  ["agent:claude", /claude|anthropic/i],
  ["agent:openai", /chatgpt|openai|gptbot/i],
  ["agent:perplexity", /perplexity/i],
  ["agent:cursor", /cursor/i],
  ["agent:gemini", /gemini|google-extended/i],
]);

/** The coarse kind of client a request came from — one of a fixed set of labels.
 * @param {Request} request */
export function clientKind(request) {
  const ua = request.headers.get("user-agent") ?? "";
  if (UNFURLERS.test(ua)) return "preview";
  for (const [kind, re] of AGENTS) if (re.test(ua)) return kind;
  if (/^curl\//i.test(ua)) return "curl";
  if (/^wget\//i.test(ua)) return "wget";
  if (/bot|crawl|spider|slurp/i.test(ua)) return "bot";
  if (/^mozilla\//i.test(ua) && /text\/html/i.test(request.headers.get("accept") ?? "")) return "browser";
  return "other";
}

/** The host of the page that linked here, or "" — never the rest of the address.
 * @param {Request} request */
export function referrerHost(request) {
  try {
    const host = new URL(request.headers.get("referer") ?? "").hostname.toLowerCase();
    return host === APEX || host === `www.${APEX}` ? "" : host;
  } catch {
    return "";
  }
}

/** The tag a shared link carries (`?ref=newsletter`, or `utm_source`): letters, digits and dashes,
 * lowercased, at most 32 characters — anything else is no tag at all.
 * @param {URL} url */
export function refTag(url) {
  const tag = (url.searchParams.get("ref") || url.searchParams.get("utm_source") || "").toLowerCase().slice(0, 32);
  return /^[a-z0-9-]+$/.test(tag) ? tag : "";
}

/** The one row counted for an answer: [route, answer, client, referrer host, country, ref tag].
 * @param {Request} request @param {string} route @param {string} type */
export function row(request, route, type, url = new URL(request.url)) {
  const answer = type === HTML ? "html" : type === MARKDOWN ? "markdown" : "text";
  // @ts-ignore — `cf` is Cloudflare's addition to Request.
  const country = String(request.cf?.country ?? "");
  return [route, answer, clientKind(request), referrerHost(request), country, refTag(url)];
}

/** @param {number} status @param {Record<string, string>} [extra] */
function plain(status, extra = {}) {
  return new Response(status === 404 ? "Not found\n" : "", { status, headers: { "content-type": TEXT, ...extra } });
}

/** One of the files behind the addresses, under its own type. A negotiated answer says so
 * (`Vary: Accept`), so no cache hands the markdown to a browser.
 * @param {{ ASSETS: { fetch(r: Request): Promise<Response> }, STATS?: { writeDataPoint(p: { indexes: string[], blobs: string[] }): void } }} env
 * @param {Request} request @param {URL} url @param {string} file @param {string} type */
async function serve(env, request, url, file, type, negotiated = false) {
  const res = await env.ASSETS.fetch(new Request(new URL(file, url), { method: request.method }));
  if (!res.ok) return plain(503);
  const headers = new Headers({
    "content-type": type,
    "cache-control": "public, max-age=300",
    "x-content-type-options": "nosniff",
  });
  if (negotiated) headers.set("vary", "Accept");
  if (request.method === "GET") {
    try {
      const blobs = row(request, url.pathname, type, url);
      env.STATS?.writeDataPoint({ indexes: [url.pathname], blobs });
    } catch {
      // Counting never stands in the way of an answer.
    }
  }
  return new Response(request.method === "HEAD" ? null : res.body, { status: 200, headers });
}

/** `/whats-new/<version>`: that release's notes, or the newest when it has none.
 * @param {any} env @param {Request} request @param {URL} url */
function whatsNew(env, request, url) {
  const m = url.pathname.match(/^\/whats-new\/([^/]*)\/?$/);
  if (!m) return null;
  return (async () => {
    const version = m[1];
    if (/^\d+\.\d+\.\d+$/.test(version)) {
      const file = `/_/whats-new/${version}.html`;
      const probe = await env.ASSETS.fetch(new Request(new URL(file, url), { method: "HEAD" }));
      if (probe.ok) return serve(env, request, url, file, HTML);
    }
    return new Response(null, { status: 302, headers: { location: "/whats-new", "cache-control": "public, max-age=300" } });
  })();
}

export default {
  /** @param {Request} request @param {any} env */
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.hostname === `www.${APEX}`) {
      return new Response(null, { status: 301, headers: { location: `https://${APEX}${url.pathname}${url.search}` } });
    }
    if (request.method !== "GET" && request.method !== "HEAD") return plain(405, { allow: "GET, HEAD" });

    switch (url.pathname) {
      case "/":
        return wantsMarkdown(request, url)
          ? serve(env, request, url, "/_/agent.md", MARKDOWN, true)
          : serve(env, request, url, "/index.html", HTML, true);
      case "/install":
        return serve(env, request, url, "/_/install.sh", TEXT);
      case "/agent":
        return wantsMarkdown(request, url)
          ? serve(env, request, url, "/_/agent.md", MARKDOWN, true)
          : serve(env, request, url, "/_/agent.html", HTML, true);
      case "/agent.md":
        return serve(env, request, url, "/_/agent.md", MARKDOWN);
      case "/whats-new":
        return serve(env, request, url, "/_/whats-new/latest.html", HTML);
      case "/faq":
      case "/privacy":
      case "/terms":
        return serve(env, request, url, `/_${url.pathname}.html`, HTML);
      default:
        return whatsNew(env, request, url) ?? plain(404);
    }
  },
};
