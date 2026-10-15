# shroom — v2 UX walkthrough: install → first share

> Status: **derived, not decided.** This doc walks both personas from "never heard of
> shroom" to "sent someone a link," moment by moment. Every *mechanical* claim under a
> moment is settled evidence — the 2026-07-20 probes ([`RESEARCH.md`](RESEARCH.md)
> "Hands-on probe results", R7.6), [`POC-LEARNINGS.md`](POC-LEARNINGS.md), and the
> ratified stack (the stack decision (2026-07-20)). Every open
> *product* call is flagged **U1–U10** in §6 for Bagrat — the walkthrough recommends
> where a recommendation is honest, it decides nothing.
>
> The said-lines are **drafts**: first-pass copy written under the product-voice rule, to
> be rewritten at will. What must survive rewriting is the *moment structure* — where a
> line is needed, what state it covers, and what it may not leak.
>
> Date: 2026-07-20.
>
> **Update, same day — superseded on decisions:** the walkthrough session ran live with
> Bagrat; **every U-flag in §6 is now decided** and the decided spec is
> [`UX-STATEMACHINE.md`](../UX-STATEMACHINE.md) — where the two disagree, the machine doc
> wins. Notable divergences from this doc's recommendations: U1 = setup-first gate (no
> local recording), U5 = permissions at end of onboarding, U7 = notify-then-apply, stop
> never auto-copies the clipboard — and the **member model** (admin/member credential
> split, invites, the C persona) was pulled into v1, dissolving U10.

---

## 0. Voice rules this doc obeys

The project rule (CLAUDE.md): warm, brief, product concepts only — write the said-line,
not the mechanism. Applied to this walkthrough concretely:

- **Cloudflare's own nouns are allowed exactly where the user is looking at them.** The
  setup flow sends the user to Cloudflare's dashboard; the page they see says "API
  Tokens", their link says `workers.dev`. Copy that names what's on the user's screen is
  honest; copy that names what's behind it is a leak.
- **Never in any said-line:** Worker, bucket, R2 (as a noun the user must parse — "your
  Cloudflare storage" is the product concept), HLS, segment, TCC, csreq, prime, SigV4,
  token-derivation, wizard-internal state names.
- **Numbers are welcome when they reassure** (POC §8: real cost figures framed cheap);
  they must be real ones.
- Said-lines appear as quoted lines: “like this.” System-owned dialogs (macOS, the
  Cloudflare dashboard) are quoted as *their* text where known; shroom only writes the
  framing around them.

Evidence keys used below: **(POC §n)** = POC-LEARNINGS section · **(Rn)** = RESEARCH
section · **(probe)** = "Hands-on probe results" / R7.6 / R7.7a · **(§4)** = the
stack decision (2026-07-20) · **(§5.n)** = the rethink's product requirements (not public).

---

## 1. The two personas

**The dev** (the rethink's personas A/B — the real v1 target). Comfortable pasting a
terminal command and reading what it does first; owns or will happily create a Cloudflare
account; wants zero ceremony and no surprises; may audit the installer, may choose
`--from-source`. Their whole flow is self-serve inside the app.

**The guided user** (persona C as the rethink narrowed it: *anybody who uses an
agent*). They don't open terminals for fun; their agent (Claude or another) does the
mechanical work and narrates the rest. Crucially, in v1 they still bring **their own
Cloudflare account** — including its card-on-file gate (POC §6) — because the
handed-credential team model is v2 (§5.7). The agent softens every step; it removes only
the terminal, not the account. Whether v1 *claims* this persona publicly is **U6**.

Same app, same setup engine, two front doors (R7.7: the BYO-agent skill is a thin
narrator over the identical call sequence — one implementation). §3 walks the dev; §4
overlays the guided deltas.

---

## 2. The spine at a glance

Narrated in the recommended order — welcome → cloud setup → first recording (permissions
happen there) → stop → recipient. Two flagged decisions could reorder it: U1 (may they
record before cloud setup?) and U5 (when permissions are asked).

| # | Moment | The user does | Measured ground |
|---|---|---|---|
| 3.1 | Install | paste one command (or agent runs it) | terminal installs carry no quarantine → zero Gatekeeper, zero dialogs (R3 §2) |
| 3.2 | First launch | nothing — the tray icon appears | script-launched app opens clean (R3); welcome copy = POC §8 findings |
| 3.3 | Cloud setup | ~4 clicks + one paste on Cloudflare, pick a link name | whole wizard hand-run live in ~10 min incl. signup-grade gates (R7.6) |
| 3.4 | Permissions | 1 inline click (mic) + 1 Settings switch (screen), once ever | E1/E2: grants survive every update; priming clears the extra consent (probe, POC §4) |
| 3.5 | First recording | confirm the pre-filled panel → 3-2-1 → talk | §5.1 UX; capture core ratified (§4); tray semantics POC §8 |
| 3.6 | Stop | click Stop in the menu | stop = published (§5.3); page exists at mint (R4); upload is background (POC §5) |
| 3.7 | The recipient | clicks the link | first frame well under 1 s on the measured path (R6.2); unfurl reality per platform (R5) |

---

## 3. Moment by moment (dev persona)

### 3.1 Install — one command, zero dialogs

The landing page (or README — see the P17 note in §6) offers three ways, in this order:

1. **The script** (primary): `curl -fsSL https://shroom.am/install | sh` — readable
   source, fetches the release, verifies the checksum, places `shroom.app`, launches it.
2. **The tap** (same thing, brew-shaped): `brew install bagrat/tap/shroom` (formula, not
   cask — the cask lane is closed for unsigned apps Sept 2026, R3 §3).
3. **The .dmg** (documented secondary): for people who insist, with the honest 5-step
   "Open Anyway" walkthrough written out (R3 §1). The docs say plainly the script is the
   nicer path, and why.

Because the bytes arrive by terminal, **macOS shows nothing at all** — no "unverified
developer," no Settings trip, no admin password (R3 §2: quarantine is set by browsers,
not by curl; this is how rustup/nix/ollama ship). The measured install experience is:
one command, a few seconds, the app is running.

Script output is user-facing copy and follows the voice rule — it says what it did, not
how:

> “Getting shroom… verified ✓
> Installed to Applications. Look for the mushroom in your menu bar.”

Engineering notes (not copy): install to `~/Applications` — no admin rights needed, and
the probe showed TCC tooling wants an LS-resolvable real path (probe, gotcha 3). Releases
ship developer-signed (5b), so nothing signing-shaped happens on the user's machine ever;
`--from-source` is the offered trust-maximalist flag and the only path that still uses
per-machine signing (5a) (§4 ledger).

### 3.2 First launch — the mushroom appears

No window grab, no modal. The tray icon appears, and opens into a small welcome panel —
value and price in two breaths (POC §8: welcome first, real figures framed cheap; never
name the user's ambient email):

> “**shroom** records your screen and hands you a link to share — no account with us,
> no subscription. Your videos live in *your own* Cloudflare storage: free for about six
> hours of video, and pennies after that — about 100 hours for $2.40 a month.
>
> Two steps before your first video: connect your Cloudflare, and let macOS allow
> recording. About five minutes, once.
>
> [ Set up my space ]        [ Later ]”

- The numbers are the POC's measured basis: ~27 MB/min ≈ 1.6 GB/h at normal quality;
  R2 ≈ $0.015/GB-mo; free tier 10 GB (POC §2/§6).
- “Later” is honest: the app stays in the tray, and Record politely points back to setup
  until it's done (POC §8: gate at use, don't nudge at start). Whether "Later" still
  allows a *local-only* first recording — the POC's value-before-friction move — is
  **U1**, the biggest open call in this doc.
- Whether shroom offers to start at login (a tray recorder is useless if it isn't
  running) is **U4**.

### 3.3 Cloud setup — your own Cloudflare, once

The wizard is one panel with the whole plan visible from the start, pre-marked with what's
already true (POC §8: show the plan once with real ✅/⬜, never reprint a blank plan;
idempotent + resumable by construction):

> “Your space — set up once, yours forever:
> ⬜ A Cloudflare account (free — they'll want a verified email, and a card to switch on
>   storage; the card is their rule, storage stays free at your size)
> ⬜ One access token for shroom (a pre-filled page — review, create, paste)
> ⬜ Your link name (part of every link you share)
> ⬜ Wiring it up (shroom does this part)”

Sub-moments, in order:

**a) The account.** A real question with two self-explanatory options (POC §8):

> “Do you already have a Cloudflare account?
> [ I have one ]   [ Not yet — take me there ]”

Both dashboard gates are pre-warned **together, before** the browser trip (POC §6: email
verification + the R2 card requirement surface as separate late failures if you let
them). Signup tip that costs nothing: accounts created with Google sign-in arrive
pre-verified (POC §6) — the wizard doesn't push it, the docs may mention it.

**b) The token.** The wizard opens the **pre-filled** Cloudflare page — name and both
permissions already set (probe P4: the deep link pre-fills for real; the form shows
"Workers Scripts Write, Workers R2 Storage Write"). The address bar shows a plain
`dash.cloudflare.com` URL the user can read — an OSS-trust bonus (R7.2).

> “I've opened your Cloudflare page with the form filled in: shroom needs exactly two
> permissions, nothing more. Review it, click **Create Token**, copy the result, and
> paste it here.
>
> [ paste field ]
>
> Your token stays on this Mac, in your keychain of files — never anywhere else.”

(Last line's exact wording open; the *fact* is fixed: `~/.shroom/`, mode 600, never on a
command line — POC §5, working agreement.)

**c) The link name.** New accounts have no workers.dev name yet — the wizard detects
this silently and asks (probe: GET → 10007 is a wizard-internal state, the user never
sees a "missing subdomain" concept; PUT creates it under the minimal token, no dashboard
trip):

> “Pick your link name — it's part of every link you share:
> `https://v.___________.workers.dev/…`
> This one's a keeper: choose a name you'll be glad to see in a year.”

- The keeper framing is mandated by the probe finding (R7.6: the subdomain is
  account-wide and permanent-feeling; the wizard must present it as such).
- The first label of the hostname is shroom's to choose and is user-visible in every
  link (`v.NAME.workers.dev` vs `shroom.NAME.workers.dev`) — cosmetic, permanent,
  flagged **U8**.

**d) The wiring.** Everything else is shroom's: ~6 REST calls (R7.1), no region question
(the probe confirmed auto-placement), then the ~40 s wait for the new name to come alive
(measured, R7.6 — a calm line, not a spinner that looks stuck):

> “Setting up your space… switching your name on takes about a minute, one time only.”

A silent self-check closes the loop (derived keys → a tiny write/read/delete round-trip,
R7.1 call 6). Done state:

> “Your space is ready. Every video you record becomes a link at
> `v.<name>.workers.dev` — record your first from the menu bar. 🍄”

**Error states, each with its said-line** (the wizard is a classified-error state
machine — attempt, classify, route to the fix, auto-poll, resume; POC §6, transfers per
R7.4):

| State | How it shows (ground) | Said-line |
|---|---|---|
| Paste looks garbled / truncated | verify fails (routing by prefix first — R7.6 gotcha) | “That doesn't look quite right — tokens sometimes lose characters in the copy. Copy it once more and paste again.” |
| Paste is a real token but can't do the job | a probe call fails with insufficient permission (classify by use, not by echo) | “This token can't manage your video space. Easiest fix: the pre-filled page — it creates one with exactly the right access, nothing more. [ Open it again ]” |
| Email not verified | 403 code 8000077 on first write (POC §6 — match the numeric code) | “Cloudflare wants your email verified before it opens storage. Check your inbox for their message — I'll notice the moment it's done and carry on.” |
| Storage not switched on (card gate) | R2 enable fails needs-payment (POC §6) | “One step left on Cloudflare's side: adding a card switches storage on. The free tier stays free — the card is their rule, not a charge. [ Open that page ]” |
| Link name taken | code 10031 (probe P2) | “That name's taken — try another. It'll be part of every link you share.” |
| Connection drops mid-setup | any network failure; state is resumable by design | “Connection dropped — everything so far is saved. I'll pick up right here when you're back online.” |
| Wizard reopened after success | all checks pass | the same plan, all ✅, plus: “Your space is ready — nothing to redo.” |

Wizard internals the copy never mentions (product-voice discipline, listed so nobody
"helpfully" surfaces them later): the 10007 no-subdomain state; verify-endpoint routing
by token prefix; the deploy response's empty-bindings echo (validate by probing, not by
echo); the propagation poll; key derivation. All from R7.6.

### 3.4 Permissions — the once-ever grant

Recommended timing: **at the first record arm**, not inside the cloud wizard (gate at
use — POC §8; and the priming step below must sit close to a real capture anyway). This
is **U5**. The moment plays in four beats:

**Preamble** — one panel, both asks named, the once-ness stated (it's true: E1/E2 proved
grants survive every update):

> “Before your first recording, macOS asks for two permissions — microphone and screen.
> Each is a one-time yes: updates never ask again.”

**Mic** — the system prompt, correctly named and answerable inline (bundle-launched app;
mic resolves fully before anything else so dialogs never stack — POC §4). The Info.plist
usage strings are said-lines too:

> mic: “shroom records your voice with your video.”
> camera: “shroom shows and records your camera only when you choose it.”

**Screen** — can't be granted from a prompt; it's a Settings switch, and it takes effect
on next launch (POC §4). The app opens the right pane and says so:

> “macOS handles screen recording in System Settings — it's open. Flip the switch next
> to shroom, and when macOS offers to quit and reopen it, say yes. I'll be right where
> you left me.”

The app returns to the arm flow on relaunch — the user never re-navigates.

**The quiet extra consent** — right after the grant goes live, shroom runs its ~1 s
invisible warm-up capture so macOS's *second* consent (the window-picker bypass, ~30-day
cadence) fires **now**, at setup, never mid-recording (POC §4 priming discipline; the
probe confirmed the dialog still exists on Tahoe and is non-fatal but real). While
priming, the arm UI carries one line of context, because the dialog is Apple-worded and
arrives unannounced:

> “If macOS double-checks about recording, approve it — that's the last one for a good
> while.”

FAQ/docs (not in-app) carry the honest recurring bit: macOS re-asks on its own rhythm
(~monthly) whether shroom may keep recording — one click; Apple's policy, not shroom's,
and the paid certificates shroom refuses wouldn't remove it either (R3 §5).

### 3.5 The first recording

Tray click → the config panel: pre-filled with last-used, never skipped — a quick
confirm, not a form (§5.1). Fields, with their fixed semantics:

- **Record:** This screen / Camera. Screen entries carry *human* display names ("LG
  UltraFine", "Built-in Display") — flagged in the rethink as easier-when-agent-first; now
  a hard app requirement (§5.2).
- **Show yourself:** the corner bubble toggle (screen mode only). In v1 per the ledger
  (P7's "iff cheap" condition met — the un-excluded preview-window trick, zero
  compositing). Draggable while recording, not resizable (§5.2).
- **Microphone:** by name; default prefers built-in and never auto-picks an iPhone
  (Continuity mic is poison — POC §2).
- **Quality:** Normal (1080p) / 2K / 4K (POC §2 presets).
- **[ Start recording ]** — “Starts after 3-2-1 — click anywhere to cancel.”

The countdown is the full-transparent-screen 3-2-1 overlay, cancelable by a click
(§5.1; the overlay-over-fullscreen recipe is POC §8's, its v2 home is a ratification
residual, not a UX question).

**While recording**, everything lives in the tray (§5.1, POC §8 semantics — proven and
carried verbatim):

- Icon shows the recording state. **Click = instant pause + menu opens** (menu only
  opens once pause is confirmed — never catch the menu in the last frame).
- Menu: **Resume · Drop a marker · Restart · Stop · Discard…** One obvious action per
  state; Stop and Discard are deliberate menu choices so a stray click can never publish
  or destroy.
- Markers are a v1 editing aid recorded into metadata; where they *surface* is deferred
  with the v2 editor (§5.1).
- **Discard…** opens the four-way panel: Keep · Resume · Restart · Discard (POC §8).
  Restart re-arms with the countdown running immediately.
- Network is never the recording's problem: capture writes locally regardless; upload is
  opportunistic background (“the recording is sacred” — POC §5).

### 3.6 Stop — the link moment

Stop is chosen from the menu. What the user experiences, in order (all under the §5.3
law: **stop = published, no publish prompt, instant link**):

1. The recording finalizes (~instant — probe: 102 ms).
2. **The real sharable page opens in the browser.** Not a preview, not a processing
   screen — the URL that will be sent. The page exists the moment the id exists, upload
   state notwithstanding (P9 mandate, R4 route).
3. The title moment. Recommended shape (this is **U2**): the page opens with the title
   field focused, placeholder “Untitled — Jul 20”, saving on blur/Enter — renaming is
   the one core edit and takes five seconds whenever the user pleases (§5.3, the rethink's
   P10: manual is fine).
4. The tray turns into the honest upload surface (R11: backlog is a UX surface, not an
   engineering problem):

> tray while draining: “Uploading — 40%. Your link already works.”
> tray when done: “Fully live ✓”

The sender-facing page carries the share affordances:

> [ Copy link ]  “Anyone with the link can watch.”

— the one-line honesty of the unlisted model (P1: unguessable + noindex; no passwords, no
expiry in v1). Download appears when the single-file version is ready (second-class by
design — R11). Where the “fully live” signal lives beyond the tray (notification?) is
**U9** — notifications are one more macOS permission ask.

**Offline stop** (the state P9 exists for): same flow, one different line —

> “You're offline — your link is ready and safe to share. The video follows the moment
> you're back online.”

The link, sent now, shows the recipient the on-its-way page (below) until the queue
drains. The tray shows “Waiting for connection” instead of a percentage.

### 3.7 The recipient

The other half of the product. What they get, per state (R4.1 page states):

- **Live** (the normal case): the minimal player page — instant start (measured: first
  frame well under 1 s on the probe path, R6.2), custom controls, no comments, no
  sign-in, no shroom account anywhere. Captions are deferred (P8) but the machinery
  under them is v1 (VTT-grade timestamps), so they arrive later for *old* videos too.
- **On its way** (link shared before upload finished):

  > “This video is on its way. The page refreshes itself — nothing to do.”

- **Removed** (the P2 tombstone, for links that outlive their video in old messages):

  > “This video was removed by its owner.”

**Unfurl honesty** (R5 — the requirement degrades gracefully and the copy must never
oversell): in Slack, Linear, Teams, X the link renders a rich card — title, poster,
duration — and plays on click-through (inline playback is provider-allowlisted there).
iMessage and Discord genuinely play inline, via the small teaser MP4 the design carries
for exactly this (R5/synthesis). Every page is noindex — a leaked link never becomes a
search result (P1, R4.1).

### 3.8 The quiet tail

- **Rename**: edit the title on the page; it's live worldwide on the next load (R4:
  metadata write, strongly consistent — no redeploy concept exists). Docs carry one
  honest note: chat apps cache their link cards for a while, so an already-posted card
  keeps the old title; new shares pick up the new one (R4.2).
- **Tray at rest**: Copy last link · Library · New recording — re-share is one of the
  two jobs the thin library exists for (§5.6).
- The MP4 download finishes uploading in the background after the link is live —
  second-class, never blocking (R11).

---

## 4. The guided walkthrough (agent persona deltas)

Same spine; what changes is who narrates. The agent's chat lines are said-lines too and
obey the same voice rule. Per moment:

| Moment | Delta | The agent's said-line (draft) |
|---|---|---|
| Install | The agent proposes the exact command and runs it on approval — never silently (working agreement, both shroom's and the agent platforms'). | “I'll install shroom — one command, it fetches the app and checks it's genuine. Okay to run it?” |
| Welcome / setup | The agent *is* the narrator; the setup engine is the same (R7.7: one implementation, two front doors). It walks the plan, opens pages, watches state. | “Your videos will live in your own Cloudflare account — free at your size. Cloudflare will ask two things of you directly: verify your email, and add a card to switch storage on (their rule — storage stays free). I'll handle the rest.” |
| The token paste | **The paste goes into the app, never into chat.** A secret in the chat transcript is a leak by design; the app owns secret intake (POC §8: secrets ride files, never argv/chat). The agent points, the human pastes. | “Cloudflare will show you a token — paste it straight into shroom's window, not here. I never need to see it.” |
| Dashboard gates | The agent cannot and must not click inside the user's Cloudflare session; it instructs and waits, and resumes off the app's state (the wizard auto-polls — the agent narrates the same resume). | “Once you've clicked Create Token on that page, tell me — or just paste it into shroom and I'll notice.” |
| Permissions | Only the human can click macOS's dialogs, and the key dialogs are invisible to screenshot automation (POC §4) — the agent must instruct-and-wait on the app's reported state, never claim to "see" the dialog. | “macOS will ask you directly — once for the microphone, once in System Settings for the screen. Take your time; I'll know when it's done.” |
| Recording onward | **The agent leaves the stage.** The hot path is the app's (the rethink: no LLM on the hot path — “you'd be annoyed to find one there”). The agent's parting line closes its own role. | “You're all set. From here it's just the mushroom in your menu bar — record, stop, share the link. You won't need me for that.” |

Design seam this table implies (engineering, one line): the app must expose a
deterministic setup/permission status the skill can read (the POC's `--prep`-style JSON
seam, reborn) — the agent narrates *state*, it never infers from screenshots.

---

## 5. Beyond the first share (thin, so the doc has edges)

- **Second Mac (v1! — P12):** install, then “join your existing space.” The honest UX
  bar: the same one paste, nothing new to invent — but the id-minting key (R4's
  HMAC-in-the-id) must travel to machine 2 somehow, which the paste alone doesn't carry.
  Join-flow shape is **U10**; the key-travel seam belongs to the scaffold session (R10
  territory).
- **Custom domain (the upgrade tier):** settings offer, honestly framed per R6.3 — the
  free name is personal-scale and fine; the domain buys “links that look like you” and
  edge caching, and requires the domain on the user's Cloudflare. Never pitched as a
  reliability rescue.
- **Delete:** from the library, with the tombstone truth in the confirm: “Delete removes
  the video everywhere. The link will stay, and will say the video is gone.” (P2.)
- **Updates:** measured reality — no quarantine, no Gatekeeper, no re-grant, ever
  (R3 + E1/E2). Whether updates apply silently or announce themselves is **U7**.
- **Library:** thin by design (§5.6) — re-share and clean up. Storage-cost visibility
  (“your videos cost ~$0.40/mo”) is P13, unasked in the rethink; parked, not designed.

---

## 6. Open product decisions (flagged, not made)

Each with a recommendation and the rejected shape, house style. None block scaffolding;
U1/U5/U6 shape first-run code paths and are worth deciding first.

- **U1 — May the user record before cloud setup?** The POC's value-before-friction
  principle (first recording renders locally, instantly; cloud pitched after you've seen
  it work — POC §8) vs the clean two-step welcome above. *Recommend:* allow it — “Later”
  yields a local-only recording with a local preview and a visible “set up your space to
  get a link” affordance; publish drains once setup completes. *Cost:* local-library
  states exist before credentials do. *Rejected shape:* hard setup-first gate — simpler,
  but throws away the POC's best conversion finding.
- **U2 — The title moment at stop.** *Recommend:* page opens with the focused title
  field + dated placeholder (manual-is-fine per the rethink's P10; zero blocking). *Rejected
  shapes:* a pre-page modal (blocks the link moment); head-transcript auto-title (P10
  demoted it to garnish — the door stays open, nothing in v1 depends on it).
- **U3 — Whose video is it, to the viewer?** The *rethink's* P5 (sender
  name/avatar/branding on the page — never asked; distinct from the OAuth probe that
  reused the id). Gates the metadata schema now even if UI comes later
  (the rethink, not public). *Recommend:* anonymous-by-default, optional display name in
  settings; schema carries the field from day one.
- **U4 — Start at login?** A tray recorder that isn't running records nothing.
  *Recommend:* one real question in the welcome panel, default yes, never silent.
- **U5 — Permission timing.** *Recommend:* at first record arm (§3.4) — gate at use,
  and priming sits next to a real capture. *Rejected shape:* inside the cloud wizard —
  front-loads the scariest dialogs onto the day the user has seen zero value.
- **U6 — Does v1 claim the guided persona publicly?** The agent path works (§4), but the
  card gate and own-account requirement are real cliffs for a non-techie. *Recommend:*
  ship the skill, market to devs; let the guided story grow into the v2 team model
  rather than promise it now.
- **U7 — Update posture.** Zero-friction updates are measured fact; the question is
  silent-auto vs notify-then-apply. *Recommend:* auto-download, apply on next
  quit/launch, a quiet “what's new” in the tray menu — OSS-trust prefers visible-but-
  effortless over invisible.
- **U8 — The first hostname label.** `v.NAME.workers.dev` (short, quiet) vs
  `shroom.NAME.workers.dev` (brand in every link). Permanent-feeling, purely cosmetic,
  cheap to decide now. *Recommend:* `v`.
- **U9 — Where “fully live” lands.** Tray-state only, or also a macOS notification
  (one more permission ask, but the only signal that reaches a user who walked away).
  *Recommend:* tray always; offer notifications the first time an upload outlives the
  open page.
- **U10 — Second-Mac join flow.** Same-paste-again (plus solving key-travel invisibly)
  vs an explicit export/handoff artifact. *Recommend:* same paste, key travels via the
  space itself; anything the user must carry between Macs is a defeat. (Mechanism: R10
  territory, scaffold session.)

**Bagrat action items surfaced by this doc** (not product decisions):

1. **The real workers.dev name pick** — the last pre-scaffold item (§4 ledger); the
   probe's throwaway was deleted, the account currently has none.
2. **P17 timing** (landing page in v1's definition-of-done): §3.1's install command
   needs *a* stable URL — shroom.am/install is the product answer; a GitHub raw URL is
   the honest interim. Ties into the unpushed-main decision (POC §11).
