# shroom.am on a Cloudflare Worker — the landing, an agent door, and short video links

> **Done (2026-10-13).** Items 1 and 2 are built and live: shroom.am is the `shroom-site`
> Worker (`site/`, deployed by `.github/workflows/site.yml`), with `/agent` and `/install`.
> Item 3 was left out of v1, as recommended. What follows is the note as it was decided.

*Design note, 2026-10-13. Not decided, nothing deployed: everything that touches
Cloudflare, DNS or GitHub waits on Bagrat's go (the commands are in §5).*

Three asks, in the order they depend on each other:

1. Move the shroom.am landing off GitHub Pages and onto a Cloudflare Worker.
2. An agent door beside `curl … | sh`: a line you paste to your coding agent, which reads
   an address on shroom.am. The address serves markdown to agents and HTML to browsers.
3. Video links on shroom.am (`shroom.am/v/…`) that lead to each person's own
   `v.<name>.workers.dev/<id>`.

**In short:** 1 — yes, and do it soon (§1.1: shroom.am is down today, and it may be open to
takeover). 2 — yes: one address, `/agent`, generated from `agent-skill/SKILL.md`. 3 — not
for v1. It would break the ratified "Built to outlive us" promise and put Bagrat's account
in the path of every link, as a service he runs. Choose the custom-domain tier (N2)
instead. A safe minimal shape is in §3.6 if he wants it anyway.

### Bagrat's answers (2026-10-13)

- **shroom.am is verified on GitHub**, so §0.1's takeover worry is closed. GitHub Pages
  and the plugin marketplace that used to live on the domain are **decommissioned**, and
  the POC plugin's marketplace entry gets **delisted**.
- The zone is on his Cloudflare account. **Every push to `main` deploys everything:**
  the site and the release alike.
- **Item 3 stays out.** Revisit in its own session after v1 ships.
- **The agent page is a markdown mirror of the landing plus the install instruction.**
  The agent asks, then runs the same command the person would have pasted. That narrows
  §2.4. See "Simplified" there, which replaces the SKILL.md-rendering plan and drops
  `/agent/status.sh`.

### As built (2026-10-13, on `main`)

Bagrat's go: build it on `main`; every push to `main` deploys the site; releases stay on
tags; he turns GitHub Pages off himself; the POC plugin is delisted
(bagrat/claude-plugins#1).

- `site/worker.js`: `/` (the landing, or its markdown to an agent), `/install` (the
  script as plain text, never a redirect), `/agent` (the markdown, or a page showing it
  word for word to a browser or a link preview), `/agent.md`, `www` → 301 to the apex,
  and nothing else. Only these paths run the script (`run_worker_first`). The favicons
  are free static files.
- `site/agent.md`: the landing in markdown, plus the agent's part (§2.4, Simplified),
  headed "If you're an agent" since Bagrat's review of 2026-10-13 (see "Reworded" below).
  `site/build.mjs` writes `site/public/` (not committed).
  `site/wrangler.jsonc`: Worker `shroom-site`, Custom Domains `shroom.am` and
  `www.shroom.am`, no workers.dev, no logs.
- Tests: `site/test/worker.test.mjs` (the routes and the negotiation table) and
  `site/test/agent.test.mjs` (the agent page carries the landing's ratified lines, the
  same command, and the ask-first rule). Both run in CI and before each deploy.
  Verified under the real runtime with `wrangler dev` 4.145.0: every route answered as
  tested. That run also found a Worker module can only export handlers, so `APEX` is no
  longer exported.
- `.github/workflows/site.yml`: on a push to `main` touching the site, it tests, then
  deploys from the `site` environment (`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`),
  then checks shroom.am serves this commit. Without the secrets it warns and deploys
  nothing.
- **Copy awaiting ratification** (landing, below the install line): "Or ask your coding
  agent" · `Install shroom on my Mac: read https://shroom.am/agent and follow it.` [Copy]
  · "Read it first — the same address shows what your agent will read. It asks you
  before it runs anything." Also every line of `site/agent.md` under "If you're an
  agent", the line under its title, and the agent page's "This is what your coding agent
  reads when you ask it to install shroom — word for word."
- **Reworded (2026-10-13, Bagrat's review: the heading "For your coding agent" spoke to
  the person, and the reader is the agent).** The agent's part now speaks to the agent
  throughout; a line under the title says whose "you" the landing's lines are. Reviewed
  beyond the heading, it also changed what the agent does: it installs only when the
  person asked (`/` serves this page to an agent that was only asked what shroom is);
  it no longer checks the Mac itself (`uname -m` is wrong under Rosetta, a cloud or
  remote agent said "this one can't run it yet" about a machine that isn't theirs, and
  the check ran before the ask, so "It asks you before it runs anything" wasn't true);
  an agent that can't run the command hands it over to paste; a stop isn't worked
  around and a second run takes a new yes; done is the installer's last line; a pasted
  token is deleted and replaced. `site/test/agent.test.mjs` holds each. §2.4's block
  below is the design-time draft, not the served text.

**Before the first deploy can succeed:** add the `site` environment and its two secrets
on GitHub, and delete the zone's apex and `www` records (keep MX ×3 and SPF). Workers
Custom Domains won't take a hostname that already has records.

---

## 0. What's true today (probed 2026-10-13, read-only)

- **shroom.am answers with GitHub's "Site not found · GitHub Pages" 404**, through
  Cloudflare's proxy (`server: cloudflare`, `x-github-request-id`, `cf-cache-status:
  DYNAMIC`). That's the end of POC-LEARNINGS §11.1: the archive commit moved `docs/` into
  `archived-poc/`, and main was pushed. `/install` is the same 404. **The v1 install
  line `curl -fsSL https://shroom.am/install | sh` does not work today.** The landing,
  the agent skill, the join page's command and ARCHITECTURE §11 all print it.
  RELEASING.md works around it with the raw.githubusercontent URL.
- DNS (resolved over DoH): NS `athena`/`gerald.ns.cloudflare.com`. The apex and `www`
  are **proxied (orange)**: they resolve to Cloudflare anycast `104.21.77.5` /
  `172.67.202.197`, not GitHub's `185.199.108–111.153`. POC §7 said the Pages records had
  to be grey-cloud, so someone changed them after the POC. MX `route1–3.mx.cloudflare.net`
  plus SPF TXT are **Email Routing** (N8's `issues@shroom.am`), and they must survive
  any cut-over.
- `site/` is the v1 landing (`index.html`, two favicons, `test/`), and nothing deploys
  it. `tests/checkpoint/local-site.mjs` is the only server that has ever served it. That
  server's routes are the shape the real one needs: `/` → the landing, `/install` →
  `installer/install.sh` as `text/plain`, one file, never a redirect (ARCHITECTURE §11).
- The video Worker (`worker/src/worker.js`) serves `/<id>` (16-char base64url, B14),
  `?k=` key redemption, owner-ness from a cookie (`ownerOf(request)`), `/m/<id>/…` media,
  `/oembed`, and OG tags whose `og:url` is its own origin. Copy link on the page copies
  `location.origin + location.pathname`.

### 0.1 Act on this first, whatever is decided below

A Cloudflare-proxied hostname whose origin is GitHub Pages answers "Site not found" for
`Host: shroom.am`. That is the textbook **dangling-custom-domain** state. If any GitHub
user adds `shroom.am` as their Pages custom domain, GitHub may serve their site there,
unless Bagrat's account has **verified** the domain (GitHub → Settings → Pages →
Verified domains, which adds a TXT `_github-pages-challenge-bagrat.shroom.am`). I can't
see his GitHub settings or the zone's records from here. **Check it now:** either verify
the domain or delete the GitHub-pointing apex/`www` records. §1's cut-over removes them
anyway.

---

## 1. The landing on a Worker

### 1.1 Shape: Worker + static assets, not Pages

| | Worker + static assets | Pages |
|---|---|---|
| Static requests | free and unlimited ([a]) | free and unlimited |
| A little logic (`/install` content-type, `/agent` negotiation, `www`→apex) | the same Worker, `run_worker_first` on just those paths ([b]) | Pages Functions (billed like Workers) or `_headers`/`_redirects` |
| Cloudflare's direction | where new features land ([c]) | supported, but new projects are pointed at Workers ([c]) |
| POC scars | none | `pages project create` refuses OAuth (§6); random `*.pages.dev` suffix; the MTU trap hit the Pages asset-upload endpoint specifically |
| One stack with the product | the video Worker is already a Worker | a second product surface |

**Recommended: one Worker, `shroom-site`, with static assets, on Custom Domains
`shroom.am` and `www.shroom.am`, and `workers_dev: false`.** Every asset request (the
landing, the favicons) is free and doesn't count toward the 100k/day free-plan limit
([a]). The Worker script runs only for the paths that need logic, so the free limit never
comes near:

```jsonc
// site/wrangler.jsonc (sketch)
{
  "name": "shroom-site",
  "main": "worker.js",
  "compatibility_date": "2026-10-01",
  "assets": {
    "directory": "public",            // build output: never site/test/
    "binding": "ASSETS",
    "run_worker_first": ["/install", "/agent", "/agent.md", "/agent/*"]
  },
  "routes": [
    { "pattern": "shroom.am", "custom_domain": true },
    { "pattern": "www.shroom.am", "custom_domain": true }
  ],
  "workers_dev": false,
  "observability": { "enabled": false }   // no request logs: nothing to hold (see §3.5)
}
```

The script: `/install` → `installer/install.sh` as `text/plain; charset=utf-8`, 200, no
redirect (ARCHITECTURE §11: "serves one file and never redirects"). `/agent*` → §2.
`www.shroom.am/*` → 301 to the apex. Everything else → `env.ASSETS.fetch(request)`. A
small build step (`site/build.mjs`) copies `index.html`, the favicons, `install.sh` and
the generated `/agent` files into `public/`. Only that step's output is uploaded, so
`site/test/` never ships. `local-site.mjs` keeps working for walking the release flow
locally. The deployed Worker is the same idea with no `localhost` swaps, and it can be
tested under `workerd`, which the repo already runs.

**Spec tests first** (the build rule): `/install` is plain text, byte-identical to
`installer/install.sh`, and never a 3xx. `/` is `site/index.html`. A `www` request is a
301 to the apex. Nothing else exists (404). §2's negotiation table.

### 1.2 Deploying it

- **First deploy, by hand, from Bagrat's Mac:** `npx wrangler deploy -c
  site/wrangler.jsonc`. It needs a wrangler login whose scopes cover Workers scripts,
  routes and the zone. The POC's narrow `account:read user:read pages:write` set (§6)
  isn't enough. Plain `wrangler login` with the default scopes is.
- **After that, a CI job (recommended once it's proven):** on a push to `main` touching
  `site/**`, `installer/**` or `agent-skill/**`, run tests and then `wrangler deploy`,
  with an API token scoped to *Account · Workers Scripts: Edit* plus *Zone (shroom.am) ·
  Workers Routes: Edit*, held in a `site` GitHub environment, with actions pinned by SHA
  like the rest of `.github/`. This makes "push main" deploy the landing, which CLAUDE.md
  already treats as a user-facing act, so there's no new rule. The installer served at
  `/install` becomes main's installer, which is already the claim RELEASING.md makes.
- Rollback: `npx wrangler rollback` (Workers keeps versions). Or remove the custom
  domain, and shroom.am goes back to answering nothing.

### 1.3 What happens to GitHub Pages and the marketplace (POC §11)

- **Landing:** this *is* the landing-page answer §11.1 was waiting for. GitHub Pages
  stops being part of shroom: turn Pages off for `bagrat/shroom` and remove its custom
  domain. Keep the domain **verified** on GitHub so nobody else can claim it (§0.1). The
  POC's `shroom-site` Pages project (§11.3, prod overwritten with test pages) becomes
  moot. Delete it when convenient.
- **Marketplace:** not coupled to the site at all. The coupling is
  `bagrat/claude-plugins`' unpinned entry pointing at `bagrat/shroom`, whose
  `plugin.json` moved into `archived-poc/`. That decision (delist / pin to `v0.1.16` /
  accept) is still open and unchanged by this. The agent door (§2) is the v1 successor
  to "install the plugin", and it needs no marketplace. **Suggest: delist** when the
  agent door ships.

### 1.4 DNS cut-over (shroom.am stays on Cloudflare NS; nothing changes at domain.am)

1. Look at the zone's records and write them down: apex A/AAAA, `www`, MX ×3, SPF TXT,
   any `_github-pages-challenge` TXT.
2. Delete the apex A/AAAA and the `www` record. Workers Custom Domains create their own
   (read-only, proxied) records and refuse while a conflicting record exists. Nothing is
   lost in the gap, because the site is a 404 today.
3. `npx wrangler deploy -c site/wrangler.jsonc`. This creates both custom domains and
   their certificates (minutes, not the ~30–60 min GitHub took).
4. Leave **MX ×3 and the SPF TXT** alone (Email Routing).
5. Verify: `curl -sI https://shroom.am/`, `curl -fsSL https://shroom.am/install | head
   -3`, `curl -sI https://www.shroom.am/` (301), §2's negotiation curls.

---

## 2. The agent door

### 2.1 The idea, and the said-line

The landing's install block gets a second door, below the command:

> **Install in one line**
> `curl -fsSL https://shroom.am/install | sh`  [Copy]
> Read it first — the same address opens as readable source.
>
> **Or ask your coding agent**
> `Install shroom on my Mac: read https://shroom.am/agent and follow it.`  [Copy]
> Read it first — the same address shows the steps your agent will follow.

Two stacked blocks, not a tab toggle. That's no new script (the page's only script stays
Copy) and no hidden state, and both doors stay visible. That fits N6's "the hero carries
the whole message". The copy is **new said-lines**, so it's an N-row for Bagrat to
ratify, then fixture lines in `spec/r1.json` that `site/test/landing.test.mjs` will hold
verbatim.

### 2.2 Routes

| Route | Answers |
|---|---|
| `/agent` | negotiated (below): the markdown for agents, the HTML rendering of it for browsers |
| `/agent.md` | always the markdown (`text/markdown; charset=utf-8`), for agents that can't set headers or for a person who wants the raw text |
| `/agent/status.sh` | `agent-skill/scripts/status.sh`, `text/plain` (§2.4) |
| `/install` | unchanged: always the script, never negotiated. A curl pipe must never get markdown |

Every `/agent` response carries `Vary: Accept` and `Cache-Control: no-store`.
Cloudflare's edge cache doesn't key on `Vary: Accept`, so the negotiated route is never
edge-cached. It's a Worker invocation, and that's fine at this volume (§3.4's numbers;
the landing itself stays a free asset). Behind the route, the two faces are static assets
(`agent.html`, `agent.md`) fetched through `ASSETS`, so the logic is one function.
Cloudflare's own "Markdown for Agents" conversion ([d]) is Pro-plan-only and converts
HTML generically. A hand-written document is better anyway.

### 2.3 Negotiation rules (prototyped and run locally)

In order:

1. Path ends in `.md` → markdown.
2. **Known link-unfurlers** (`Slackbot`, `Twitterbot`, `facebookexternalhit`,
   `Discordbot`, `LinkedInBot`, `TelegramBot`, `WhatsApp`) → HTML, so a pasted
   `shroom.am/agent` link unfurls as a card, not raw text. This is the only User-Agent
   rule, and it runs the other way from usual: UA picks out crawlers, never agents.
   Agents' UAs change every release.
3. `Accept` names `text/markdown` with q > 0, at least as high as `text/html` →
   markdown.
4. `Accept` names `text/html` (every browser does) → HTML.
5. Anything else (`*/*`, empty, `text/plain`) → markdown. That's curl, wget,
   python-requests, and agents' plain fetchers: a tool, not a browser.

**Measured:** Claude Code's WebFetch sends `Accept: text/markdown, text/html, */*` with
`User-Agent: Claude-User (claude-code/2.1.296; …)` (echoed by httpbin.org/headers from
this session). Cloudflare names Claude Code and OpenCode as agents that send it ([e]). The
prototype below (minus rule 2) gave every case its intended face: Chrome / Safari /
Firefox → HTML. Claude Code, `*/*`, no Accept, and `text/html;q=0.5, text/markdown` →
markdown. `text/markdown;q=0, text/html` and `text/html, text/markdown;q=0.5` → HTML.
`/agent.md` from Chrome → markdown.

```js
function wants(pathname, accept) {
  if (pathname.endsWith(".md")) return "markdown";
  const q = (type) => {
    let best = -1; // -1: not named
    for (const part of (accept ?? "").split(",")) {
      const [mt, ...params] = part.trim().toLowerCase().split(";").map((s) => s.trim());
      if (mt !== type) continue;
      const qp = params.find((p) => p.startsWith("q="));
      best = Math.max(best, qp ? Number(qp.slice(2)) || 0 : 1);
    }
    return best;
  };
  const md = q("text/markdown"), html = q("text/html");
  if (md > 0 && md >= html) return "markdown";
  if (html > 0) return "html";
  return "markdown";
}
```

### 2.4 What the markdown says

**Simplified (Bagrat, 2026-10-13): this replaces the plan below.** `/agent.md` is the
landing in markdown (the hero's lines, the two price tiles, the trust strip, the system
requirements), plus one section addressed to the agent:

```markdown
## For your coding agent

Install shroom on this Mac for the person you're helping. Ask first; run only on a yes.

1. Check it's a Mac that can run shroom: macOS 14 or newer, Apple silicon
   (`sw_vers -productVersion`, `uname -m` → `arm64`). If not, tell them it can't run
   here yet, and stop.
2. Show them this command exactly, and ask whether to run it:
   `curl -fsSL https://shroom.am/install | sh`
   It downloads shroom, checks it's genuine, puts it in ~/Applications and opens it.
   They can read it first at https://shroom.am/install.
3. On a clear yes, run it once, and tell them what it printed.
4. Then hand over: shroom's window walks them through the rest. If it asks for a
   Cloudflare token, that goes into shroom's window, never into this chat.
```

Both faces come from one source. A build step writes `agent.md` and `agent.html` from
the landing's content, so they can't drift, and a test holds them to the landing's
ratified lines. **With this, no `status.sh` is served and the agent does nothing after
the install.** That's narrower than UX-STATEMACHINE's guided-agent overlay (§ "The
guided-agent overlay"), where the agent narrates setup through the status seam
(`agent-skill/`). Bagrat decides whether the overlay stays as a separate opt-in skill or
gets retired. Either way it's a spec change for DEVIATIONS.

*The superseded plan, kept for the record:*

**Single source:** a build step renders `/agent.md` from `agent-skill/SKILL.md`. It drops
the frontmatter, swaps "the skill folder's `scripts/status.sh`" for the shroom.am
address, and prepends a short head for an agent that has never heard of shroom. The
skill and the served document can't drift, and `agent-skill/test` extends to the
rendered file. SKILL.md already carries every rule this door needs: install only on a
clear yes, the token never in chat, read state only through the status reader, product
words only, leave at recording.

The head (draft, product voice):

```markdown
# Install shroom

shroom is a screen recorder for the Mac. Its videos live in the person's own
Cloudflare account. These are the steps for you, their coding agent.

**You never change their Mac without asking.** Show them each command exactly as you
will run it, ask, and run it only on a clear yes. Silence, "maybe" or a question is not
a yes.

Check first: shroom runs on macOS 14 or newer, Apple silicon. If `uname -m` isn't
`arm64` or `sw_vers -productVersion` is below 14, say "shroom runs on a Mac with Apple
silicon and macOS 14 or newer — this one can't run it yet." and stop.

The command: `curl -fsSL https://shroom.am/install | sh`
(with an invite link: `curl -fsSL https://shroom.am/install | sh -s -- --join <link>`)
They can read it first: https://shroom.am/install is the script itself.
```

The rest is SKILL.md's body. Its status reader becomes
`curl -fsSL https://shroom.am/agent/status.sh | sh -s -- --wait 60`: read-only, it
writes nothing, and the agent shows it once in the same ask ("…and I'll follow shroom's
progress with a small read-only check from the same address"). Better, and an open
question: **ship `status.sh` inside the app bundle** (`shroom.app/Contents/Resources/`).
Then the agent runs a local, signed, read-only file and fetches nothing after the
install.

**Install it as a skill? Not offered.** The skill's job ends at `all-set` ("Leave at
recording"), so a persistent copy in `~/.claude/skills/` would be a machine mutation that
buys nothing. A member joining later gets the command from their invite page. The
document ends with the parting line. If Bagrat wants a "keep this skill" door later, it's
one asked step (`mkdir -p ~/.claude/skills/shroom-setup && curl -fsSL
https://shroom.am/agent.md -o …`), and it's agent-specific (Claude Code's path), which is
why I'd leave it out.

**The HTML face** of `/agent` is the same text rendered at build time (no markdown
library at runtime), under a line like "These are the steps your coding agent follows."
That's the agent door's "Read it first", the same as `/install`'s.

**Safety posture.** "Read this URL and follow it" is the shape prompt-injection defences
are rightly wary of. The document leans into that. It asks the agent to show and ask,
not to act. Its one command is the same one the landing prints. It never asks for a
secret. Some agents will refuse `curl | sh` from fetched text or ask on their own, and
that's the posture we want, not a failure.

---

## 3. Video links on shroom.am

### 3.1 Options

| Option | How it works | Verdict |
|---|---|---|
| **A. Target in the path** | `shroom.am/v/<name>/<id>` → 302 `https://v.<name>.workers.dev/<id>` (`/s/<name>/<id>` for N42's `shroom` label) | Stateless, no storage, no writes. The only viable shape. Open-redirect-lite (§3.5). |
| **B. Registry (KV)** | users register `name → origin`; lookup per click | Needs a write API, identity, abuse handling and moderation: a service in every sense. Free KV reads are also capped at 100k/day ([a]). **Reject.** |
| **C. Signed tokens** | a signature proves the target is genuine | Who signs? A key in every app isn't a secret. Per-space keys need a key registry, which is B again. **Reject.** |
| **D. Full proxy** | shroom.am fetches and returns the page and media | Video bytes through Bagrat's account (excluded by the ask). It also breaks the page: owner-ness is a cookie on the space's origin, `?k=` redemption sets it, and the page's `fetch("/_shroom/…")` calls are same-origin. **Reject.** |
| **D′. Head-only proxy for crawlers** | 302 for people; for unfurler UAs, fetch the upstream page and return its `<head>` with 200 | One subrequest per unfurl, UA cloaking, and shroom.am serving attacker-chosen HTML to crawlers. Only worth it if a major platform refuses to follow the 302 (§3.2). **Don't build up front.** |
| **E. 302 vs 301** | | **302.** A 301 is cached by browsers indefinitely, so a mistake becomes permanent. Crawlers follow both. |

### 3.2 Do link previews survive a redirect?

With a 302, the crawler lands on the user's own Worker and reads the OG tags the video
Worker already renders server-side (`unfurl()` in `worker/src/worker.js`). No proxy is
needed if the crawler follows redirects:

| Platform | Follows the page redirect? | Whose tags / URL | Confidence |
|---|---|---|---|
| Facebook / Messenger / Instagram | yes. Checks 301/302, then `og:url`, then `rel=canonical`, and follows the chain ([f]) | the final URL's tags. Its `og:url` (workers.dev) becomes the canonical. Cross-domain `og:url` needs an allow-list since 2020 ([f]), which doesn't matter here because the redirect already lands on workers.dev | documented |
| Slack | yes for the page (link shorteners unfurl daily). **Not** for `og:image` redirects ([g]). Ours are direct absolute workers.dev URLs, so fine | final page's tags. Cached 30+ days ([g]) | observed + secondary |
| X / Twitter | yes. Every X link is already a `t.co` redirect | final page's tags | observed. X's troubleshooting doc is paywalled to fetchers (HTTP 402) |
| iMessage | yes. The sender's device fetches like Safari | final page's tags | observed |
| LinkedIn | yes. Post Inspector shows the redirect trail | final page | observed |
| Discord | **mixed.** A user report shows a 301-ed link giving an empty embed while the target embedded fine ([h]) | — | **test it** |

**`og:url` stays the workers.dev URL.** If the user's page claimed a shroom.am `og:url`,
Facebook would follow it back into the redirect, which is a loop. So the card, the
"open" target and the address bar all end on workers.dev. The shroom.am form only exists
in the message text. That's half the branding the ask hopes for.

**Test before believing** (ARCHITECTURE §10's protocol, extended): from a deployed
redirect, `curl -sIL -A '<UA>'` per bot UA, then a manual post per platform with
`?v=N` cache-busting, Discord first.

### 3.3 What the link buys, honestly

- A shroom-branded string in the message, until it's clicked.
- *Not* protection from `workers.dev` blocks (some corporate filters block it). A
  redirect still lands there, and only the full proxy (D) would dodge that.
- *Not* a stable URL across moves. That would need the registry (B).

### 3.4 Cost to Bagrat's account

Unit: **1 Worker request per click or crawler fetch** of a shroom.am link. CPU for a
validated 302 is well under 1 ms. Video bytes: zero (redirect only). Rough volume of
10–15 requests per shared video (8–12 views + 2–4 unfurl fetches; iMessage fetches on
devices, Slack once per share):

| Scenario | Requests | Free plan (100k/day, 10 ms CPU) | Paid ($5/mo: 10M req + 30M CPU-ms incl.; +$0.30/M) |
|---|---|---|---|
| Today-ish: 50 spaces × 30 videos/mo | ~18k/mo (~600/day) | fine | $5 floor |
| 2,000 spaces × 40 videos/mo | ~1.2M/mo (~40k/day avg, weekday peaks near 100k) | **at the cliff** | $5 |
| 20,000 spaces | ~12M/mo | over, daily | ~$5.60 |
| One video goes viral: 500k clicks in a day | 0.5M/day | **Error 1027 for everyone's links**, plus the Worker-run landing routes (`/install`, `/agent` → 429), until 00:00 UTC ([i], [b]) | +$0.15 |
| Abuse: 1,000 req/s for a day | 86M/day | dead after ~2 minutes of it | ~$23/day; sustained for a month, ~$700 |

Sources: Workers pricing and limits ([a], [i]).

On the free plan, **an attacker needs about 100k requests (minutes from one laptop) to
take every shroom.am video link and the install line offline for the rest of the UTC
day.** On the paid plan the same attack turns into money. Mitigations, best first:

1. **A separate Cloudflare account for the redirector.** Limits are per account, so
   exhausting the redirector's quota can never take down `/install`.
2. **Paid plan ($5/mo)** if it ships at all. On free, the cliff is a DoS lever.
3. **A WAF rate-limiting rule on the zone** (per-IP), which runs before the Worker and
   so before billing. Verify what the free plan includes before relying on it. The
   in-Worker rate-limit binding runs *after* the request is already billed, and it's
   per-location and approximate ([j]).
4. **Fail closed** on the route. Fail-open with no origin is just an error.

A **Redirect Rule** instead of a Worker would cost nothing per request. Free zones get
10 Single Redirects with wildcards ([k]). But wildcard `*` matches across `/` and regex
is Business-only ([k], [l]), so a rule can't stop `/v/x.evil.com/…` style host
injection into `https://v.${1}.workers.dev/…`. An open redirect on shroom.am is how a
domain lands on Safe-Browsing lists. **Worker with a strict regex, not a rule.**

### 3.5 Trust, privacy and the principle

- **shroom.am learns every shared video's capability.** The 16-char id *is* the access
  control for an unlisted video. Every shroom.am link carries `name + id`, plus the
  viewer's IP, UA and referrer, through Bagrat's account. With `observability` off and
  no logging, his dashboard keeps no paths. But Cloudflare's processing does see them,
  and a future "just turn logs on to debug" would store everyone's video keys. Today
  shroom.am learns nothing about anyone's videos.
- **Phishing reach.** Anyone can name a Worker `v` on their own account and mint
  `shroom.am/v/<them>/<16 chars>` → their page. The regex limits targets to
  `v|shroom.<label>.workers.dev`. It can't tell a shroom space from a lookalike: a
  `/_shroom/ping` check is trivially faked, and it costs a subrequest. The abuse reports
  land on shroom.am.
- **"Never a SaaS we operate": yes, this makes shroom.am a service Bagrat operates.** It
  sits in the availability path of every link that uses it, it has a bill and an abuse
  inbox, and its uptime is now his: domain renewal at domain.am, his account standing,
  his quota.
- **It breaks a ratified line.** The trust strip says *"Built to outlive us — Links play
  from your own account — if shroom vanished tomorrow, they'd keep working."* (N6;
  `site/index.html`). A shroom.am link stops working if shroom.am lapses. That sentence
  would have to be reworded or carved out ("…links on your own address"), and that's a
  stance change, not copy.
- **The honest opt-out is the existing upgrade tier (N2):** a custom domain on the
  person's own Worker (`videos.me.com/<id>`). It looks like them, caches at the edge,
  and depends on nobody. If shroom.am links shipped, they should be **opt-in per space,
  off by default**, and the page's Copy link would have to learn which form to copy.
  That's a spec change to U8 ("every link reads `v.<name>.workers.dev/<id>`") and to the
  page's one copy door. Keyed owner URLs (`?k=`) must never take the shroom.am form.

### 3.6 If Bagrat wants it anyway: the minimal safe shape

Opt-in. A separate Cloudflare account on the paid plan. A Worker on
`shroom.am/v/*` and `/s/*`, with `^/(v|s)/([a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)/([A-Za-z0-9_-]{16})$`
→ 302 `https://(v|shroom).<name>.workers.dev/<id>` (query dropped, so `?k=` can never
ride through), and everything else 404. `Cache-Control: no-store`,
`Referrer-Policy: no-referrer`, observability off, a WAF per-IP rate rule, and fail
closed. No proxy, no head-fetch (D′ only if Discord fails the test). And the trust-strip
line changes first.

---

## 4. Recommendations

1. **Landing → Worker + static assets** (`shroom-site`, custom domains on apex + `www`,
   Worker script only on `/install` and `/agent*`). Do §0.1's takeover check now.
   Cut over as in §1.4. First deploy by hand, CI later. Turn GitHub Pages off but keep
   the domain verified there. This unblocks the install line every v1 surface prints.
2. **Agent door: build it.** `/agent` negotiated by §2.3's rules (`.md` → unfurler UA
   → Accept q-values → `text/html` present → markdown), `/agent.md` always markdown,
   rendered from `agent-skill/SKILL.md` at build time, plus two stacked doors on the
   landing. No "install as a skill" step.
3. **shroom.am video links: don't build for v1.** They make shroom.am an operated
   service, contradict "Built to outlive us", hold every shared video's capability, and
   on the free plan hand anyone a one-laptop DoS on all links. The custom-domain tier
   (N2) is the honest answer to "links that look like shroom/me". If he still wants it,
   §3.6.

## 5. Open questions for Bagrat

1. **Is `shroom.am` verified on your GitHub account** (Settings → Pages → Verified
   domains)? If not, the 404 state is claimable. Verify it or delete the records today.
2. Is the shroom.am zone on your main Cloudflare account, the same one the POC used? The
   Worker must live on the zone's account for Custom Domains.
3. CI deploys on push to `main`, or by hand only? (CI means one more scoped token in a
   GitHub environment.)
4. The agent door's said-lines (§2.1): ratify as drafted, or reword? And `/agent` as the
   path (vs `/setup`, which collides with the in-app "set up" vocabulary)?
5. Should `status.sh` ship inside the app bundle, so the agent fetches nothing after the
   install?
6. Marketplace entry for the POC plugin: delist now that the agent door replaces it?
7. Item 3: drop it, or keep it as a post-v1 opt-in on its own paid account, knowing it
   rewrites the "Built to outlive us" line?

## 6. Steps that need Bagrat's go (none run)

GitHub (web):
1. Settings → Pages → *Verified domains* → add `shroom.am`, and put the TXT record it
   names in the zone (or confirm it's there).
2. `bagrat/shroom` → Settings → Pages → remove the custom domain, Source: *None*.

Cloudflare (shroom.am zone, dashboard → DNS):
3. Write down every record. Delete the apex A/AAAA and the `www` record. **Keep the
   three MX and the SPF TXT.**

On his Mac, from the repo after the site Worker is built and tested:
```bash
npx wrangler login
```
```bash
npx wrangler deploy -c site/wrangler.jsonc
```
```bash
curl -sI https://shroom.am/
curl -fsSL https://shroom.am/install | head -3
curl -sI https://www.shroom.am/
curl -s -H 'Accept: text/markdown' https://shroom.am/agent | head -5
curl -s -H 'Accept: text/html' https://shroom.am/agent | head -5
```
Later (optional): an API token scoped to *Workers Scripts: Edit* (account) plus *Workers
Routes: Edit* (zone shroom.am) as `CLOUDFLARE_API_TOKEN` in a `site` GitHub environment,
then a deploy job. Afterwards (optional): delete the POC Pages project `shroom-site`.

---

## Sources

- [a] Cloudflare Workers pricing: free 100k req/day and 10 ms CPU; paid $5/mo, 10M req +
  30M CPU-ms, +$0.30/M req; static asset requests "free and unlimited"; KV free 100k
  reads / 1k writes per day — https://developers.cloudflare.com/workers/platform/pricing/
- [b] Static assets billing ("requests to the Worker script … are billed"; free-tier
  overage on `run_worker_first` paths → 429) —
  https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/ ;
  routing — https://developers.cloudflare.com/workers/static-assets/routing/worker-script/
- [c] Workers vs Pages —
  https://developers.cloudflare.com/workers/static-assets/compatibility-matrix ,
  https://developers.cloudflare.com/workers/static-assets/migrate-from-pages/
- [d] Markdown for Agents (Pro+ only) —
  https://developers.cloudflare.com/fundamentals/reference/markdown-for-agents/
- [e] Cloudflare blog, Markdown for Agents (names Claude Code and OpenCode as sending
  `Accept: text/markdown`) — https://blog.cloudflare.com/markdown-for-agents/
- [f] Facebook, canonical URL / redirects / `og:url` —
  https://developers.facebook.com/docs/sharing/webmasters/getting-started/versioned-link
- [g] Slack unfurl notes (secondary) — https://nuxtseo.com/learn-seo/nuxt/mastering-meta/slack ;
  Slack unfurling docs — https://docs.slack.dev/messaging/unfurling-links-in-messages/
- [h] Discord community report on 301-redirected embeds —
  https://support.discord.com/hc/fr/community/posts/4407571303447-Enhance-embeds-of-link-messages-URL-redirection
- [i] Workers limits (Error 1027, fail open/closed, 50 subrequests on free) —
  https://developers.cloudflare.com/workers/platform/limits/
- [j] Workers rate-limiting binding (10/60 s periods, per-location, eventually
  consistent) — https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/
- [k] Redirect availability (Free: 10 Single Redirects, wildcard yes, regex no) —
  https://developers.cloudflare.com/rules/url-forwarding/
- [l] Rules language (`*` matches across `/`; `matches` is Business+) —
  https://developers.cloudflare.com/ruleset-engine/rules-language/operators/#wildcard-matching
- Measured here: shroom.am headers and body (GitHub Pages "Site not found" through
  Cloudflare), DoH records for NS/A/AAAA/MX/TXT, and Claude Code WebFetch's request
  headers via https://httpbin.org/headers — all 2026-10-13.
