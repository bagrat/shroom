#!/usr/bin/env node
// A day or a week of how people find and install shroom, as JSON on stdout
// (the analytics proposal, minimal tier):
//
//   site       shroom.am's own counts, from Analytics Engine (`shroom_site`, written by
//              site/worker.js): per address, answer, kind of client and link tag (`?ref=`);
//              what came via each tag; the linking hosts; the countries
//   releases   each release's download counts on GitHub (running totals, so a week is the
//              difference from the span before)
//   traffic    the repository's views, clones, referrers and paths, which GitHub keeps for
//              14 days only
//   cloudflare Cloudflare Web Analytics' own count of the landing (its beacon, kept on by
//              Bagrat, 2026-10-14): visits, page views, linking hosts, countries, devices,
//              paths — read over Cloudflare's GraphQL API with the same token
//
// Run, in a cloud session (Node's fetch takes the proxy only when told to):
//   NODE_USE_ENV_PROXY=1 NODE_EXTRA_CA_CERTS=/root/.ccr/ca-bundle.crt node --no-warnings dev/stats/collect.mjs [--days 1|7] [--end YYYY-MM-DD]
// --days 1 is yesterday (the daily run), 7 the week before today (the default).
// The Cloudflare token needs *Account Analytics: Read* on shroom.am's account: the cloud
// environment's CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID (Bagrat's go, 2026-10-14), or
// SHROOM_STATS_TOKEN and SHROOM_STATS_ACCOUNT where set. Without them, or when the read is
// refused, the site's part is left out and says why. GitHub's parts need no token in a cloud session (its proxy signs api.github.com);
// elsewhere, SHROOM_STATS_GITHUB_TOKEN (traffic needs push access).

const REPO = "bagrat/shroom";
const DATASET = "shroom_site";
const DAY = 86_400_000;

/** The `days` whole UTC days that end at the start of `end`, as SQL-ready bounds and a name.
 * @param {Date} end */
export function week(end, days = 7) {
  const to = new Date(Date.UTC(end.getUTCFullYear(), end.getUTCMonth(), end.getUTCDate()));
  const from = new Date(to.getTime() - days * DAY);
  const day = (/** @type {Date} */ d) => d.toISOString().slice(0, 10);
  return { id: day(from), from: day(from), to: day(to) };
}

/** Each link tag (`?ref=`) and what came through it: landing visits, agents reading /agent,
 * installer runs, and every request. Untagged traffic isn't listed.
 * @param {{ route: string, answer: string, client: string, ref?: string, n: number }[]} rows */
export function cameVia(rows) {
  /** @type {Map<string, { k: string, visits: number, agents: number, installRuns: number, all: number }>} */
  const by = new Map();
  for (const r of rows) {
    if (!r.ref) continue;
    const t = by.get(r.ref) ?? { k: r.ref, visits: 0, agents: 0, installRuns: 0, all: 0 };
    const f = funnel([r]);
    t.visits += f.landingVisits; t.agents += f.agentPageAgents + f.landingAgents; t.installRuns += f.installRuns; t.all += r.n;
    by.set(r.ref, t);
  }
  return [...by.values()].sort((a, b) => b.all - a.all);
}

/** The funnel from the site's rows: what each count means, by name.
 * @param {{ route: string, answer: string, client: string, n: number }[]} rows */
export function funnel(rows) {
  const sum = (/** @type {(r: typeof rows[0]) => boolean} */ f) => rows.filter(f).reduce((t, r) => t + r.n, 0);
  const agent = (/** @type {string} */ c) => c.startsWith("agent:");
  return {
    landingVisits: sum((r) => r.route === "/" && r.client === "browser"),
    landingAgents: sum((r) => r.route === "/" && r.answer === "markdown" && agent(r.client)),
    agentPageAgents: sum((r) => (r.route === "/agent" || r.route === "/agent.md") && r.answer === "markdown" && agent(r.client)),
    agentPageBrowsers: sum((r) => (r.route === "/agent" || r.route === "/agent.md") && r.client === "browser"),
    installRuns: sum((r) => r.route === "/install" && (r.client === "curl" || r.client === "wget")),
    installReads: sum((r) => r.route === "/install" && r.client === "browser"),
    previews: sum((r) => r.client === "preview"),
    bots: sum((r) => r.client === "bot"),
    other: sum((r) => r.client === "other"),
  };
}

/** @param {string} sql @param {string} account @param {string} token */
async function ae(sql, account, token) {
  const res = await fetch(`https://api.cloudflare.com/client/v4/accounts/${account}/analytics_engine/sql`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}` },
    body: `${sql} FORMAT JSON`,
  });
  if (!res.ok) throw new Error(`Analytics Engine answered ${res.status}: ${(await res.text()).slice(0, 200)}`);
  return /** @type {{ data: Record<string, string | number>[] }} */ (await res.json()).data;
}

/** @param {{ from: string, to: string }} w */
async function site(w) {
  const token = process.env.SHROOM_STATS_TOKEN || process.env.CLOUDFLARE_API_TOKEN;
  const account = process.env.SHROOM_STATS_ACCOUNT || process.env.CLOUDFLARE_ACCOUNT_ID;
  if (!token || !account) return { missing: "No Cloudflare token and account id are set — the site's counts are left out." };
  const where = `timestamp >= toDateTime('${w.from} 00:00:00') AND timestamp < toDateTime('${w.to} 00:00:00')`;
  const rows = (await ae(`SELECT blob1 AS route, blob2 AS answer, blob3 AS client, blob6 AS ref, SUM(_sample_interval) AS n FROM ${DATASET} WHERE ${where} GROUP BY route, answer, client, ref`, account, token))
    .map((r) => ({ route: String(r.route), answer: String(r.answer), client: String(r.client), ref: String(r.ref ?? ""), n: Number(r.n) }));
  const top = async (/** @type {string} */ blob) =>
    (await ae(`SELECT ${blob} AS k, SUM(_sample_interval) AS n FROM ${DATASET} WHERE ${where} AND ${blob} != '' GROUP BY k ORDER BY n DESC LIMIT 15`, account, token))
      .map((r) => ({ k: String(r.k), n: Number(r.n) }));
  return { funnel: funnel(rows), cameVia: cameVia(rows), rows, referrers: await top("blob4"), countries: await top("blob5") };
}

/** Cloudflare Web Analytics' answer, shaped like the site's part.
 * @param {any} a one account from GraphQL's `viewer.accounts` */
export function webAnalytics(a) {
  const top = (/** @type {any[]} */ rows, /** @type {string} */ dim, by = "visits") =>
    (rows ?? []).filter((r) => r.dimensions[dim]).map((r) => ({ k: r.dimensions[dim], n: by === "count" ? r.count : r.sum.visits }));
  return {
    pageviews: a.t?.[0]?.count ?? 0,
    visits: a.t?.[0]?.sum?.visits ?? 0,
    referrers: top(a.r, "refererHost").filter((r) => r.k !== "shroom.am" && r.n > 0),
    countries: top(a.c, "countryName"),
    devices: top(a.d, "deviceType"),
    paths: top(a.p, "requestPath", "count"),
  };
}

/** @param {{ from: string, to: string }} w */
async function cloudflare(w) {
  const token = process.env.SHROOM_STATS_TOKEN || process.env.CLOUDFLARE_API_TOKEN;
  const account = process.env.SHROOM_STATS_ACCOUNT || process.env.CLOUDFLARE_ACCOUNT_ID;
  if (!token || !account) return { missing: "No Cloudflare token and account id are set — Cloudflare's count is left out." };
  const f = `{date_geq:"${w.from}",date_lt:"${w.to}",requestHost:"shroom.am"}`;
  const group = (/** @type {string} */ alias, /** @type {string} */ dim, /** @type {number} */ n, order = "sum_visits_DESC") =>
    `${alias}:rumPageloadEventsAdaptiveGroups(limit:${n},orderBy:[${order}],filter:${f}){count sum{visits} dimensions{${dim}}}`;
  const query = `{viewer{accounts(filter:{accountTag:"${account}"}){t:rumPageloadEventsAdaptiveGroups(limit:1,filter:${f}){count sum{visits}} ${group("r", "refererHost", 15)} ${group("c", "countryName", 15)} ${group("d", "deviceType", 5)} ${group("p", "requestPath", 10, "count_DESC")}}}}`;
  const res = await fetch("https://api.cloudflare.com/client/v4/graphql", {
    method: "POST",
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
    body: JSON.stringify({ query }),
  });
  const body = /** @type {any} */ (await res.json().catch(() => ({})));
  const account0 = body?.data?.viewer?.accounts?.[0];
  if (!res.ok || !account0) throw new Error(`Cloudflare's analytics answered ${res.status}: ${JSON.stringify(body?.errors ?? "").slice(0, 200)}`);
  return webAnalytics(account0);
}

/** @param {string} path */
async function github(path) {
  /** @type {Record<string, string>} */
  const headers = { accept: "application/vnd.github+json", "user-agent": "shroom-stats" };
  if (process.env.SHROOM_STATS_GITHUB_TOKEN) headers.authorization = `Bearer ${process.env.SHROOM_STATS_GITHUB_TOKEN}`;
  const res = await fetch(`https://api.github.com/repos/${REPO}${path}`, { headers });
  if (!res.ok) throw new Error(`GitHub answered ${res.status} for ${path}`);
  return res.json();
}

async function releases() {
  /** @type {{ tag_name: string, published_at: string, assets: { name: string, download_count: number }[] }[]} */
  const list = await github("/releases?per_page=100");
  return list.map((r) => ({
    tag: r.tag_name,
    published: r.published_at,
    downloads: Object.fromEntries(r.assets.map((a) => [a.name, a.download_count])),
  }));
}

async function traffic() {
  try {
    const [views, clones, referrers, paths] = await Promise.all(
      ["/traffic/views", "/traffic/clones", "/traffic/popular/referrers", "/traffic/popular/paths"].map(github),
    );
    return { views: views.views, clones: clones.clones, referrers, paths };
  } catch (e) {
    return { missing: `The repository's traffic wasn't readable: ${/** @type {Error} */ (e).message}` };
  }
}

async function main() {
  const arg = (/** @type {string} */ name) => { const i = process.argv.indexOf(name); return i > 0 ? process.argv[i + 1] : undefined; };
  const end = arg("--end"), days = Number(arg("--days") ?? 7);
  if (!Number.isInteger(days) || days < 1) throw new Error("--days takes a whole number of days");
  const w = { ...week(end ? new Date(`${end}T00:00:00Z`) : new Date(), days), days };
  // A refused Cloudflare read leaves the site's part out, said so; GitHub's parts still count.
  const counts = await site(w).catch((e) => ({ missing: `shroom.am's counts weren't readable: ${e.message}` }));
  const cf = await cloudflare(w).catch((e) => ({ missing: `Cloudflare's count wasn't readable: ${e.message}` }));
  const out = { ...w, collected: new Date().toISOString(), site: counts, cloudflare: cf, releases: await releases(), traffic: await traffic() };
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
