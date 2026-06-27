# Shroom — agent-first, self-hosted Loom alternative

> Status: **semi-spec / carry-over.** This is the distilled output of a design
> thread (held in Bagrat's `tent` workspace, `inbox/shroom-agent-first-loom.md`,
> 2026-06-27). It is the *starting point* for the new repo, not a finished plan.
> Decisions marked **(locked)** were settled deliberately; everything else is
> open or deferred to implementation.

---

## 1. The reframe — why this exists

`~/shroom` is a parked Phoenix 1.8 Loom clone. The valuable part was the
client-side TS (`assets/js/`): MediaRecorder screen+cam capture, chunked
multipart upload straight to S3 during recording, client thumbnail extraction,
pause/resume/cancel, Safari/Firefox quirks solved. The Phoenix backend was the
drag (presigned URLs, part assembly, a `RemuxWorker` on Fly to inject duration,
Postgres, Fly deploy).

**Why it was parked:** Bagrat lost confidence he could *sell* it as SaaS — but
he still wants a Loom alternative, and thinks many do.

**The reframe that revives it:** make it **agent-first**. Ship *code, not a
service*. There's nothing to host, operate, or support, so the "can't sell SaaS"
problem dissolves. This is a **fresh, greenfield repo** that reuses old shroom's
*patterns* and the *idea*, not its code. (Notably, the prized multipart-upload IP
is **retired**, not ported — see §5.)

---

## 2. Distribution model **(locked)**

- **A Claude plugin, BYO-storage.** The user installs the plugin and supplies
  their own object storage (Cloudflare R2 by default) + a local git repo. No
  central server, no central anything. **Adoption replaces sales.**
- **Open-source story:** "the first agent-native Loom; your videos live in your
  own git and your own bucket."
- **Storage is S3-compatible, not R2-only (locked).** R2 is the default, but the
  storage layer speaks the S3 API so card-averse users can point at B2 / AWS /
  MinIO. Architect for it even if v1 ships R2-first. This is also the escape
  hatch that keeps the credential design honest (see §8).

---

## 3. Architecture — the zero-server spine

- **git repo = the database.** Tiny `<id>.md` per video (metadata + transcript).
  Stable `id` in frontmatter; link by id, never by path. (Mirrors tent's
  substrate philosophy.)
- **Object storage (R2 default) = the bytes.** Key cost lever: **egress is
  free** → the bill is a flat "~$15/TB stored" regardless of views (~$0.015/GB-mo;
  ~1 GB/hour of video → ~$1.80 for 100 hrs; personal use ≈ free). This is what
  makes "cheaper than Loom" *structurally* true, not a margin trick.
- **Public bucket + unguessable keys = permanent, unlisted playback URLs.**
  Retires the old "presigned URL expired" playback bug.
- **Static player page** on Cloudflare Pages. Playback needs no server.
- **No Cloudflare Worker in v1** — capture is local, so the credentialed upload
  runs on the trusted local machine.

Three components, two clean interfaces (**fifo in, events out**):

```
  ┌─────────────┐  control fifo   ┌──────────────────┐  events.ndjson  ┌────────┐
  │ control shim │ ───────────────▶│  recorder script │ ───────────────▶│  agent │
  │ (native)     │  pause/resume/  │  (deterministic) │  segment_uploaded│ (brain)│
  │ menubar+hotkey│  stop          │  owns ffmpeg+up  │  published/done  │        │
  └─────────────┘                 └──────────────────┘                 └────────┘
```

---

## 4. Capture (recording) **(locked architecture)**

- **Lean local ffmpeg, Mac-first.** The audience already has Claude Code +
  terminal, so requiring ffmpeg is fine. Local capture is more agent-native,
  yields fragmented MP4 / HLS directly (no remux — **kills `RemuxWorker`**), and
  keeps the credentialed upload on the trusted local machine. **v1 = screen +
  mic.** **Camera-bubble PiP deferred** (Loom's signature *and* the single
  hardest piece: real-time `filter_complex` overlay + per-OS device handling).

- **Recorder script = deterministic hands.** Owns the whole ffmpeg lifecycle
  (start / pause / resume / stop via segments), the upload, and finalize+deploy.
  **Reads** a control fifo, **writes** an `events.ndjson`. This is the
  determinism boundary: recording is pure mechanism with nothing for an LLM to
  decide in real time, and you actively *don't* want token latency /
  nondeterminism in the path of "did my stop button work." `stop` = write `q` to
  ffmpeg stdin (clean shutdown, valid moov atom).
  - "Agent-driven" ≠ the LLM babysitting ffmpeg. The agent *orchestrates the
    session* (launches the recorder; does title/chapters/"keep this?"/"publish?"
    before & after). This split is *why* the button path works even with no agent
    session live.

- **Control shim = thin, native, swappable per-OS.** A macOS menu-bar item (the
  "tray") + a **no-permission global hotkey** (Carbon `RegisterEventHotKey`,
  **not** an `NSEvent` tap — avoids the Accessibility TCC prompt; the hotkey is
  the instant stop that leaves *no visual trace*, since reaching for the menu bar
  is itself captured). It only writes `pause`/`resume`/`stop` into the fifo.
  - **Cross-platform note:** there is *no* pure-JS tray (a tray inherently needs
    native code); Electron is too heavy (150 MB for two buttons). Keep the
    **control contract platform-independent** (the fifo, pure Node) and make the
    button helper a ~40-line per-OS shim. v1 ships a Mac shim — **Swift-JIT**
    (`swift control.swift`, JIT-compiled, no binary to sign, no Gatekeeper
    quarantine) *or* a small prebuilt **Go `systray`** helper. Gatekeeper
    quarantine is not a blocker for plugin-delivered binaries: `com.apple.quarantine`
    is applied by the *downloading app* (browser), **not** by `npm install` /
    `git clone`.

- **Pause = segment boundary (locked), not SIGSTOP.** SIGSTOP on a live capture
  gives timestamp glitches / dead air / a-v drift. A segment-boundary pause is a
  clean cut — and it's *the same mechanism* as chunked upload (a pause is just a
  deliberate segment cut). So pause/resume falls out of the upload design for
  free.

---

## 5. Upload — HLS + file-by-file **(locked)**

**Retires multipart entirely.** Switch from chunked multipart-during-record to
**HLS segments + file-by-file PUT.**

- **Key insight:** file-by-file incremental upload and HLS-as-delivery-format are
  the *same* decision. To upload during recording, never reassemble server-side,
  and avoid multipart, the delivered artifact must stay multi-file = HLS
  (segments + `.m3u8`). Choosing file-by-file *is* choosing HLS.
- **It collapses four things into one:**
  1. **pause/resume** = segment boundary (§4),
  2. **incremental upload** = `PutObject` each segment as ffmpeg writes it (no
     part numbers / ETags / 5 MB-min / Complete / abort; a failed segment = retry
     one idempotent PUT),
  3. **no-remux finalize** = `.m3u8` carries duration → kills `RemuxWorker`,
  4. **delivery format** = segments stay segments, never reassembled.
- **Format:** fMP4 / CMAF segments (Safari plays HLS-fMP4 natively; DASH-ready).
- **Segment duration:** **6 s default, a config constant** (adjustable in code).
- **Playlist uploaded only at finalize, not live (locked).** Segments still
  stream up *incrementally during* recording (that's what makes `/stop`
  near-instant — bytes already up); only the `.m3u8` is deferred to the end, and
  **uploading the playlist *is* the "go live" act.** No viewer can have the link
  before publish → no reader ever sees a half-written playlist → the
  atomic-publish problem dissolves. Side benefit: nobody can watch
  mid-recording (privacy-safe default; no live-watch in v1).
- **Fail-safe uploader (locked).** Core principle: **the recording is sacred and
  never blocks on upload.** ffmpeg keeps writing segments to disk regardless of
  network; the uploader retries with backoff in the background; if it never
  catches up, finalize uploads the remainder. Deterministic keys + idempotent
  PUTs make retry/resume trivial — on restart, diff local segments against the
  bucket and upload the gap. Local disk = source of truth until a segment is
  confirmed up.
- **Cost is negligible:** segments live in the bucket (no object-count cap — the
  20k cap is a *Pages* limit, not storage), egress free, slightly more Class-A
  PUTs (~$4.50/M; ~100 PUTs for a 10-min video).
- **Key/prefix scheme: deferred to implementation.** (Unguessable `<id>/` prefix.)

---

## 6. Playback & publish **(locked)**

- **Per-video static HTML, generated from one template at finalize** — *not* a
  param-driven single page. Reason = **link unfurling**: crawlers don't run JS,
  so per-video `og:` meta tags are required for Slack/Twitter/iMessage preview
  cards (core to what makes a Loom link feel good). Bonus: instant load (values
  baked in) and privacy-friendly. One template in source → N generated outputs,
  re-derivable from metadata.
- **Player needs hls.js** (~100 KB, one file, no build) for non-Safari browsers;
  Safari plays HLS natively. Bundled into the hosted page.
- **Pages cost ≈ $0** regardless of page count (bandwidth uncapped/free, static
  files free). Per-video pages touch free-tier *limits*, not charges: ~500
  builds/mo (direct `wrangler pages deploy` likely doesn't count — confirm), 20k
  files/site free (~10k videos). Distant; mitigations = batch deploys / upgrade.
  **The only real dollar cost in the system is bucket storage** (~$15/TB-mo).
- **No publish *step*. `/stop` *is* the publish.** Post-setup: record → link.
  The agent opens the local preview, then in chat says "say `publish` or
  `discard`." "Publish?" is a **conversation turn, never an in-page button** —
  the preview is a *read-only viewer* (same call tent made with its explorer;
  writes flow through the agent). A `file://` page can't reach the terminal
  Claude process anyway.

- **The `events.ndjson` is also the durable recovery artifact** = the
  `pending-publish` intent file. The agent launches the recorder as a
  **harness-tracked background task** and is **re-invoked on completion** (no
  hour-long blocking tail; user stays free to chat), reads the `published` event
  → presents / `open`s the URL. If the terminal/session dies mid-recording, the
  detached script still finishes and the link still goes live; the `published`
  event sits unconsumed, and the next `/shroom` run drains it ("your last
  recording is live: <url>"). More robust than a long-lived page poking an
  ephemeral session (rejected as fragile).

---

## 7. The agent layer — the actual product

This is what Loom charges for, free here, and the answer to the skeptic's "this
is just ffmpeg + a bucket":

- Auto **title / TL;DR / chapters** (local whisper).
- **Transcripts → semantic search** over the library.
- **Cross-linking** videos into notes by id.
- **Editing-as-a-sentence** ("trim the dead air at 4:10" → ffmpeg, no timeline UI).
- **Derived artifacts** (GIF / captions / changelog).
- **Smart retention** ("these 12 unviewed clips — archive?").

**Determinism boundary:** *scripts* = capture (ffmpeg flags), upload (file-by-
file), commit (id / frontmatter / push). *Skills* = titles, chapters, links,
"keep this?", "publish?".

---

## 8. Onboarding & setup flow

### `/shroom:setup` sequence **(locked)**

1. **Silent local-env check** (ffmpeg, wrangler, whisper, git) — no questions.
   Runs first so we only prompt about what's *actually* missing.
2. **One consolidated `AskUserQuestion`** with two questions at once:
   *install the missing tools?* (single toggle over all of them, exact commands
   shown — "propose → confirm → run" as **one approval, not N**) + *where should
   the library live?* (default `~/shroom`, free-text override). Handle "yes dir /
   no install" gracefully. `git init` on the chosen dir folds into the same
   install approval. **General rule: any system mutation = propose the exact
   command, ask, then run. Never silently mutate the machine.**
3. **Cloudflare** (below).

### Cloudflare sub-sequence **(locked)**

1. **Explain before the browser opens** — ask for **narrow scopes** (R2 + Pages +
   account-read); say why. Trust beat for an HN audience.
2. **`wrangler login`** → OAuth, then `whoami` → grab account ID. *Skippable* if
   `whoami` already returns valid scopes (returning user).
3. **One consolidated gate checklist** — the human-only gates (email
   verification, R2 activation ToS + card) both need the dashboard, so surface
   them **together** with deep-links (we have the account ID): one browser trip,
   not two ping-pongs.
4. **Auto-poll to continue** — `wrangler r2 bucket list` / a capability probe; no
   "type done."
5. **Provision** — create bucket, set public access, create the Pages project,
   and **generate an R2 S3 API token** for the uploader (§8 creds). End-to-end,
   so the very next record is record → link.

### Auth = OAuth, not token paste **(locked)**

`wrangler login` (and the official Cloudflare MCP, `mcp.cloudflare.com/mcp`) both
avoid token copy-paste — the real "super smooth" unlock. For the *shipped*
product use **wrangler** (deterministic, purpose-built, testable); keep the MCP
as an optional debug helper.

### Email-verification detection: probe capability, not state **(locked)**

There's no clean "account verified" flag from `wrangler whoami`. So don't detect
the gate — **attempt the first real gated op (enable R2 / create bucket) and
branch on the result.** Returning verified account ⇒ it just succeeds, the
verification beat is skipped silently. Cold account ⇒ create fails with a
specific state error ⇒ surface the checklist, auto-poll, retry. Makes the whole
setup **idempotent / re-runnable.** Same "probe → skip or surface" rule applies
to login itself.
- **Build-time task:** catalogue the actual error shapes so "not verified" vs
  "R2 not enabled" vs "needs card" vs "insufficient scope" are distinguishable.

### Onboarding principle: value before friction **(locked)**

The very first `/record` always renders **locally and instantly** — a `file://`
player plays the local preview MP4, showing the real player + auto-title +
chapters + transcript. The scary cloud setup is offered *after* she's seen it
work, at peak motivation.
- **The local preview is a *different format* from the cloud (locked).** hls.js
  fetches segments via XHR and Chrome blocks fetch from `file://` (null-origin
  CORS), so HLS + `file://` is flaky. Fix: ffmpeg `tee`s two outputs in one
  encode — **HLS/fMP4 → uploaded**, and a **parallel progressive `preview.mp4`
  → plays via `<video src="file://…">` with zero JS, zero server**, finalizing
  the instant ffmpeg exits. (Alt: concat `-c copy` at stop — but `tee` is ready
  the instant you stop.)

### Custom domain: deferred **(locked)**

A custom domain means DNS = a third human gate. v1 leans on free managed
subdomains (`*.r2.dev` for bytes, `*.pages.dev` for the player). `r2.dev` is
rate-limited / discouraged for "production," but fine for personal/unlisted Loom
links — and it makes setup require **zero DNS**. Custom domain (and a GitHub
remote for backup/multi-device) become optional later steps.

---

## 9. Credentials & privacy **(locked)**

- **Two distinct credentials:**
  - **Provisioning** (create bucket/Pages, gates) → the **wrangler OAuth
    session** (Cloudflare-specific — fine, provisioning is R2-specific anyway).
  - **Per-segment upload** → the **S3-compatible API with R2 access keys (SigV4
    PUT)**, *not* `wrangler r2 object put` — so the same upload path works
    against B2/AWS/MinIO unchanged (routing uploads through wrangler would
    silently re-lock to R2).
- **Storage location: `~/.shroom/` (mode 600), never in the git repo.** The repo
  is the metadata DB and may sync to GitHub later; secrets stay out of the
  substrate.
- **Privacy posture (locked):** with local-git + direct Pages deploy there is
  *no public repo*. Source (metadata + transcripts) stays private on disk;
  privacy is handled at the **deploy boundary** — only the published player page
  + chosen fields ship to Pages. Default is safe: nothing exposed unless
  published. (Matters because the agent commits transcripts of screen
  recordings.)

---

## 10. v1 scope

**In:** screen + mic capture; local instant preview; HLS + file-by-file upload to
R2; per-video static pages on Pages; the agent layer (title/chapters/transcript/
search/edit-as-sentence); `/shroom:setup`; record → link.

**Deferred:** camera-bubble PiP; custom domain; GitHub remote; live mid-recording
watch; a Cloudflare Worker; a `/shroom billing` cost command (reads CF usage so
the user sees spend — keep in mind); the durable log-watching server idea
(superseded by the `events.ndjson` intent file).

---

## 11. Open / decide-at-implementation

- Key/prefix scheme for bucket objects.
- The Cloudflare error-shape catalogue (the empirical bit for §8 capability-probe).
- Exact `events.ndjson` schema.
- Repo layout / packaging as a Claude plugin (skills, scripts, templates dirs).
- `tee` vs concat-at-stop for the local preview (lean `tee`).

---

*Provenance: distilled from `tent/inbox/shroom-agent-first-loom.md` (2026-06-27),
which holds the full reasoning, rejected alternatives, and Bagrat's own words.*
