// The four cleanup scores over an invented Library (POC, dev/proposals/cleanup-suggestions.md):
// twelve made-up videos with made-up plays, scored by the Worker's own `cleanupFacts` and
// `cleanupScore`, and the month's saving each formula could state. Prints Markdown.
//
//   node dev/cleanup-sample.mjs   (from worker/)
import { FADING_COST, FORMULAS, cleanupFacts, cleanupScore, monthlySaving } from "../src/worker.js";

const DAY = 86_400_000;
const NOW = Date.parse("2026-10-18T12:00:00Z");
const SINCE = NOW - 180 * DAY; // counting began six months ago
const at = (daysAgo) => new Date(NOW - daysAgo * DAY).toISOString();
/** Per-day plays: [daysAgo, plays] pairs. */
const days = (...pairs) => Object.fromEntries(pairs.map(([a, n]) => [at(a).slice(0, 10), n]));
/** A tenths histogram with `n` ends around `share` watched. */
const watch = (n, share) => { const w = Array(10).fill(0); if (n) w[Math.min(9, Math.floor(share * 10))] = n; return w; };
const rec = (plays, viewers, lastAgo, share, perDay) => ({ v: 1, plays, viewers, ends: plays, watch: watch(plays, share), lastAt: lastAgo === null ? null : at(lastAgo), days: perDay });

// title, age (days), minutes, record (null: never played)
const LIBRARY = [
  ["Q3 roadmap walkthrough", 160, 24, rec(41, 23, 120, 0.85, days([150, 30], [140, 8], [120, 3]))],
  ["Bug repro: login loop on Safari", 210, 3, rec(2, 1, 205, 0.95, days([205, 2]))],
  ["Onboarding: setting up your laptop", 300, 18, rec(52, 31, 9, 0.7, days([9, 2], [40, 3], [70, 2], [100, 4]))],
  ["Draft — ignore", 95, 6, null],
  ["Design review: settings sheet", 75, 11, rec(3, 2, 70, 0.15, days([70, 1], [72, 2]))],
  ["Weekly update, July 4", 106, 7, rec(9, 9, 100, 0.9, days([105, 6], [100, 3]))],
  ["Customer call recap (Acme)", 45, 32, null],
  ["How the deploy pipeline works", 400, 41, rec(19, 12, 25, 0.6, days([25, 1], [90, 2], [160, 3]))],
  ["Test take", 20, 1, null],
  ["Release 1.4 demo", 250, 14, rec(120, 80, 160, 0.8, days([240, 90], [200, 20], [160, 10]))],
  ["Code walkthrough: billing service", 380, 58, rec(4, 3, 300, 0.4, {})],
  ["Hiring: interview loop explained", 60, 9, rec(5, 5, 31, 0.95, days([31, 2], [50, 3]))],
];

const videos = LIBRARY.map(([title, age, mins, r]) => ({ title, facts: cleanupFacts({ state: "live", createdAt: at(age), durationSec: mins * 60 }, r, NOW, SINCE) }));
const total = videos.reduce((a, v) => a + v.facts.bytes, 0);
const gb = (b) => `${(b / 1e9).toFixed(2)} GB`;
const usd = (x) => (x < 0.01 ? `${(x * 100).toFixed(2)}¢` : `$${x.toFixed(2)}`);
const ago = (d) => (d < 60 ? `${Math.floor(d)} days` : `${Math.round(d / 30)} months`);

/** A draft line per reason — the words are Bagrat's to settle; these show the shape. */
function line(r) {
  switch (r.code) {
    case "never-played": return `Nobody has watched it in ${ago(r.quietDays)}.`;
    case "stopped-early": return `Opened ${r.plays === 1 ? "once" : `${r.plays} times`}; most stopped in the first quarter — last ${ago(r.quietDays)} ago.`;
    case "not-lately": return `Last watched ${ago(r.quietDays)} ago.`;
    case "big-and-quiet": return `Takes ${gb(r.bytes)} and hasn't been watched in ${ago(r.quietDays)}.`;
    default: return "";
  }
}

/** fading-cost's line: the room it takes, then the plays a person can check. */
function costLine(r) {
  const plays = r.plays === 0 ? `nobody has watched it in ${ago(r.quietDays)}` : `watched ${r.plays === 1 ? "once" : `${r.plays} times`}, the last ${ago(r.quietDays)} ago`;
  const early = r.code === "stopped-early" ? ", most stopping in the first quarter" : "";
  return `Takes ${gb(r.bytes)}; ${plays}${early}.`;
}
function unused() {
  switch (0) {
    default: return "";
  }
}

const out = [`Library: ${videos.length} videos, ${gb(total)} in all (${usd(monthlySaving(total, total))} a month past the free 10 GB). Counting began 6 months ago.`, ""];
out.push("| Video | Age | Size | Plays | Median watched | Last played |", "|---|---|---|---|---|---|");
for (const { title, facts: x } of videos) {
  out.push(`| ${title} | ${Math.floor(x.ageDays)} d | ${gb(x.bytes)} | ${x.plays} | ${x.medianWatched === null ? "—" : `${Math.round(x.medianWatched * 100)}%`} | ${x.lastPlayedAt ? `${Math.floor((NOW - Date.parse(x.lastPlayedAt)) / DAY)} d ago` : "never"} |`);
}
for (const f of FORMULAS) {
  const picked = videos.map((v) => ({ ...v, verdict: cleanupScore(v.facts, f, NOW) })).filter((v) => v.verdict.candidate).sort((a, b) => b.verdict.score - a.verdict.score);
  const freed = picked.reduce((a, v) => a + v.facts.bytes, 0);
  out.push("", `### ${f}: ${picked.length} suggested, frees ${gb(freed)} — saves ${usd(monthlySaving(total, freed))} a month (${usd(freed / 1e9 * 0.015)} at list price)`, "");
  for (const v of picked) out.push(`- **${v.title}** (${gb(v.facts.bytes)}) — ${f === "fading-cost" ? costLine(v.verdict.reason) : line(v.verdict.reason)}`);
}

// Inside the free 10 GB: the same Library less its two biggest videos still in use.
const small = videos.filter((v) => !["How the deploy pipeline works", "Onboarding: setting up your laptop"].includes(v.title));
const smallTotal = small.reduce((a, v) => a + v.facts.bytes, 0);
const smallPicked = small.filter((v) => cleanupScore(v.facts, "fading-cost", NOW).candidate);
const smallFreed = smallPicked.reduce((a, v) => a + v.facts.bytes, 0);
out.push("", `### Inside the free 10 GB (the same Library less the deploy and onboarding videos: ${gb(smallTotal)})`, "",
  `fading-cost still suggests ${smallPicked.length} (${smallPicked.map((v) => v.title).join(", ")}), freeing ${gb(smallFreed)} — saves ${usd(monthlySaving(smallTotal, smallFreed))} a month: the space is inside Cloudflare's free 10 GB.`);
// How the size gate moves it: half a cent a month (≈ 0.33 GB, about 6 minutes) instead of one.
FADING_COST.usdAtLeast = 0.005;
const looser = videos.filter((v) => cleanupScore(v.facts, "fading-cost", NOW).candidate);
out.push("", `With the gate at half a cent a month (≈ 0.33 GB), fading-cost suggests ${looser.length}: ${looser.map((v) => v.title).join(", ")} (${gb(looser.reduce((a, v) => a + v.facts.bytes, 0))}).`);
console.log(out.join("\n"));
