# shroom — the principles

What every decision in the build is weighed against. They were set in the rethink
(July 2026), before the stack was chosen, and they haven't moved since.

**The job.** A least-resistance quick video message. "Quick" means least friction, not
short: a PR demo, a question easier shown than written, help for a teammate, a landing-page
video. shroom's own landing page shows a video recorded with shroom.

**North star:** match Loom's UX, reject Loom's pricing, never become a SaaS operator.

## The non-negotiables

1. **Least-resistance UX** (Loom-grade).
2. **BYO-storage, no SaaS we operate.** Everything a user records lives on their own
   Cloudflare account and is billed by Cloudflare, not by us.
3. **Open source.**
4. **Minimal, no bloat.**
5. **Desktop, cross-platform eventually**: macOS first, Windows and Linux later. Those
   can't be tested where shroom is built today, which is a hard constraint against
   untestable hand-rolled per-platform native code.
6. **A coherent stack over sentiment.** Don't fragment the stack; adopt a technology only
   where it genuinely fits.

**Telemetry is opt-in only**, and ideally reaches the issue tracker without the user
needing a GitHub account.

## What follows from them

- **No LLM on the hot path.** Record, upload, link, player and editor are just an app.
  AI is optional garnish at the edges, and when it appears it runs on the user's own
  account or agent, never on one shroom operates.
- **BYO-backend.** The few dynamic needs (rename, views, members, crash reports) are met by
  serverless code the user deploys on their own Cloudflare.
- **The app stays small** and cheap to maintain.
