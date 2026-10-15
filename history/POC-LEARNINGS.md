# shroom POC — consolidated learnings

> **What this is.** The distilled, load-bearing knowledge from building the agent-first shroom
> POC (v0.1.16, archived 2026-07-19 under `archived-poc/`, removed from the tree 2026-10-13 — read it
> in the private bagrat/shroom-archive at tag `v0.1.5`: `git show v0.1.5:archived-poc/<path>`). It exists so the
> v2 rearchitecture (the rethink) can **stand on everything the POC proved and
> never re-hit its walls**. Everything here was empirically validated on real hardware unless
> marked otherwise; "we" = Bagrat + Claude across ~3 weeks of build sessions (2026-06-27 →
> 2026-07-19). File pointers reference the archived tree, which remains the working reference
> implementation for every mechanism described.
>
> How to use it: designing any v2 subsystem (capture, permissions, upload, publish, setup,
> distribution), first read that section here, then read the archived source it points to.

---

## 1. What the POC was, and its final state

**Agent-first, self-hosted Loom alternative shipped as a Claude Code plugin.** Claude
orchestrated record → publish via two commands (`/shroom:setup`, `/shroom:record`) and two
skills (cleanup, search); deterministic Node scripts did every mechanism; a native macOS
menu-bar shim owned capture permissions and the recording controls. It **worked end-to-end
on a clean machine**: tray-driven record with pause/resume/restart/discard → HLS uploaded
live to the user's own R2 → stop = instant title suggestion (head transcript) → static
player page on Pages → permanent unlisted link, enriched in the background with chapters +
searchable transcript, auto-archival of local bytes, always-present MP4 download.

**Final architecture (process model B, locked + verified):**

```
/shroom:record (Claude command, prose orchestration)
   └─ preflight.mjs --prep     one-shot: setup gate, update check, devices, perms prime, id mint
   └─ shroom.app (Swift shim)  tray + overlay + TCC principal + fifo writer
        └─ run-node record.mjs deterministic recorder: fifo in, events.ndjson out
             ├─ ffmpeg          input 0: avfoundation screen (video only), encode+segment+tee
             ├─ shroom --mic-tap input 1: native AVAudioEngine mic → f32le PCM → fifo
             └─ uploader        SigV4 file-by-file PUTs to R2 during capture
   └─ transcribe.mjs → whisper (local), write-meta.mjs → <id>.md, publish.mjs → build+deploy+commit
```

**Version history (all shipped):** 0.1.8–0.1.12 TCC hardening (disclaim re-exec, .app bundle,
own-principal prompts) · 0.1.13 green set (version check, post-update runner, cleanup+search
skills, player polish, run-node) · 0.1.14 bypass-picker timing fix (`--prime-capture`) ·
0.1.15 head transcription + slugged MP4 download · 0.1.16 one-call `--prep`, armed
`--cancel`, quiet recovery, stale-grant self-reset.

**State at archive time (2026-07-19):**

- The archive commit `92a7169` is **local-only (unpushed)**. Pushing it has two user-facing
  effects to decide deliberately: (a) the plugin marketplace (`bagrat/claude-plugins`)
  references this repo **unpinned** — with `.claude-plugin/plugin.json` moved into
  `archived-poc/`, fresh installs/updates of the plugin break; (b) the shroom.am landing
  page is served by GitHub Pages **from `docs/` on main**, which moved — the site stops
  updating (last successful build likely keeps serving).
- `dev/audio-prototypes/` (micpipe.swift, ratectl.swift, syncprobe.mjs + compiled binary)
  was untracked and **no longer exists** — its content shipped as
  `archived-poc/scripts/shim/macos/Sources/MicTap.swift`, and its findings are in §3.
- The `~/.claude/skills/shroom` symlink still points at this repo root; with plugin.json
  moved, the dev-loaded plugin is defunct (remove or repoint if resurrecting the POC).
- Known-open at freeze: host-validation of the 0.1.16 update/re-grant flow; a product-voice
  narration audit of record.md/setup.md (leaks kept recurring — see §8); TEST-PLAN steps
  ⑧/⑨ (search, cleanup on a real recording) never re-run after the last reset.

---

## 2. The one-encode capture recipe (ffmpeg on macOS) — validated

The single hardest-to-rediscover artifact. Canonical implementation:
`archived-poc/scripts/recorder/lib/config.mjs` + `lib/ffmpeg.mjs` (argv builder),
`record.mjs` (lifecycle). Validated on ffmpeg 7.1.1/Darwin 25; still true on 8.x.

One ffmpeg per **take** (a run between pauses), two inputs, one encode, tee'd to two outputs:

```
ffmpeg -hide_banner \
  -f avfoundation -framerate 30 -capture_cursor 1 -i "<videoIndex>:none" \
  -thread_queue_size 4096 -f f32le -ar <rate> -ac 1 -i <audio fifo> \
  -vf <scale-to-preset-box> -c:v h264_videotoolbox -b:v <preset> -pix_fmt yuv420p \
  -r 30 -force_key_frames 'expr:gte(t,n_forced*6)' \
  -c:a aac -b:a 128k \
  -f tee -map 0:v -map 1:a \
  "[f=hls:hls_segment_type=fmp4:hls_time=6:hls_playlist_type=vod:start_number=<N>:hls_fmp4_init_filename=init.mp4:hls_segment_filename=seg_%05d.m4s]stream_<k>.m3u8|[f=mp4:movflags=+faststart]preview_<k>.mp4"
```

Load-bearing facts, each learned the hard way:

- **`-r 30` + forced keyframes is THE key fix.** avfoundation reports a bogus input timebase
  (`1000k tbr`); without a forced constant output rate the HLS muxer cannot time-slice and
  **wedges silently with a 0-byte init.mp4**. This single missing flag caused every early
  "tee is broken" hang. Forced IDR every `hls_time` seconds gives clean segment boundaries.
- **Stop = write `q\n` to ffmpeg stdin** → exit 0, trailers/moov written in ~200 ms. SIGINT/
  TERM also finalize cleanly (exit 255) as escalation. **Never pass `-nostdin`** (blocks `q`).
  Whoever spawns ffmpeg must hold its stdin.
- **Segment duration 6 s**, a config constant. fMP4/CMAF segments (Safari plays natively).
- **Quality is a downscale-to-box preset** (`lib/quality.mjs`: normal 1080p ≈ 3.5 Mbps, 2K,
  4K; 30 fps): avfoundation always captures native res, cannot be constrained at capture, so
  scale in ffmpeg; never upscale. normal ≈ **27 MB/min ≈ 1.6 GB/hour** — the basis of all
  cost copy.
- **Resolve devices by NAME, never index** — avfoundation indices shift when a Continuity
  Camera appears (`lib/devices.mjs`).
- **~1 s warmup**: every take (including the first) loses ~1 s at its head — inherent to
  avfoundation start; accepted (SIGSTOP-pause rejected as worse).
- **iPhone Continuity mic is poison**: drops audio and, sharing the single avfoundation
  session, hitches the *video* too. Default mic selection prefers built-in and never
  auto-picks Continuity.
- **Camera-as-source works** (`--device "FaceTime HD Camera"`) — camera-only mode, not PiP.
  Product call: keep cameras out of the *screen* picker; camera-only is its own mode.
- **fifo reading in Node**: never `fs.createReadStream` on a fifo — a blocking threadpool
  read wedges `process.exit()` forever (recorder finalized but never exited ~15 min). Open
  `O_RDWR | O_NONBLOCK` and drive with `net.Socket` (`lib/control.mjs`). Regression-tested.

### Pause/resume = segment boundary (not SIGSTOP)

A pause `q`-stops the current take; resume spawns a new ffmpeg with **`-start_number <next
global index>`** so segment filenames stay contiguous across takes (note: plain
`start_number`, NOT `hls_start_number`). **`init.mp4` is byte-identical across takes**
(validated) so one shared init serves the session; the only join artifact needed is
`#EXT-X-DISCONTINUITY`. At finalize (`lib/finalize.mjs`) the master `stream.m3u8` is
assembled from the per-take playlists (one `EXT-X-MAP`, discontinuities between takes, one
`ENDLIST`) and per-take previews are concatenated into one `preview.mp4`. SIGSTOP-pause was
rejected on evidence: timestamp glitches, dead air, a/v drift.

---

## 3. Audio: ffmpeg's avfoundation mic capture is broken — capture natively

**The most valuable single finding.** Symptom: "cutting/clicking" audio from the built-in
mic. Two full diagnostic rounds went wrong before the truth:

- *Wrong turn 1:* "~6–10 % duration deficit; fix with finalize-time atempo." The deficit was
  real but a **symptom**; atempo was never built.
- *Wrong turn 2:* "clicks are natural speech-onset transients; not fixable." False.
- Also ruled out empirically: swr `aresample=async` compensation (stuffs silence → the
  original chop; high-async variant shifts pitch — ear-rejected), sample-rate mismatch
  (forcing device nominal 48 k changes nothing; avfoundation has **no** audio sample-rate
  option — it passively adopts device nominal), virtual HAL drivers (removing BlackHole/
  Loom/Krisp/Parrot + coreaudiod restart did NOT help), macOS Voice Isolation (no Mic Mode
  active), playback chain, hardware.

**Root cause (proven by isolation, ear + raw-sample analysis):** ffmpeg's
`libavdevice/avfoundation.m` **audio** capture has a ~decade-old buffer-handling bug
(trac #4437/#4513, ~2015): per-callback alloc/destroy instead of a ring buffer. It produces
**genuine digital splices** — adjacent-sample jumps of 12k–17k (up to 54 % FS) landing on a
new DC level, ~1/sec, worse during speech — on the built-in mic, on **every ffmpeg version**
(7.1.1 and 4.4.8 both clip; AirPods happen to survive it at 24 k). Native **AVAudioEngine**
capture of the same mic is crystal clear. `adeclick` only dulls it. Unfixable by flags.

**The shipped fix — two-input native audio (prototype-validated, then host-validated):**
keep ffmpeg for screen + encode + segment + tee (that path was never buggy); swap ONLY the
audio input. A native tap (`MicTap.swift`, a `--mic-tap` mode of the shim binary) streams
mono f32le PCM from AVAudioEngine to stdout; the recorder buffers it and feeds ffmpeg input
1 via a fifo (`-f f32le -ar <probed rate> -ac 1`). Device chosen by name via CoreAudio
(case-insensitive exact-then-substring match); **no resampling anywhere** — the recorder
probes the tap's true rate (`--mic-tap --probe` → `rate=N`) and tells ffmpeg the matching
`-ar`, so no DSP sits between mic and encoder. Drop `aresample=async` entirely: native PCM
is wall-clock-locked, async would only stuff silence back in.

**Results:** drift is dead — v−a **+0.013 s over 120 s (0.011 %)**, sign flips run-to-run
(⇒ residual ~0.1–0.2 s is start/stop boundary variance, not clock drift). Host-validated:
clean intelligible capture, pause/resume across take joins, mic grant inherited.

**Two-input plumbing gotchas (will bite ANY reimplementation of this shape):**

- **Deadlock:** if the tap's stdout pipes straight to the fifo, ffmpeg's ~1 s avfoundation
  *video* warmup means it isn't draining audio yet; the tap fills the 64 KB pipe, blocks,
  delivers nothing, and ffmpeg then blocks forever on its audio input. Intermittent (a
  race). **Fix: the orchestrator sits in the middle** with a large in-memory buffer
  (~16 MB PassThrough), opening the fifo write-end only after ffmpeg spawns; the mic never
  blocks. Plus a watchdog (no `init.mp4` within ~12 s → SIGKILL + abort, so a wedge never
  orphans processes holding mic+screen).
- **Stop ordering:** `q` ffmpeg and await finalize **first**, then kill the tap. Reverse
  order EOFs audio early → video-only tail (a phantom "drift").
- The fifo fd (`O_RDWR|O_NONBLOCK`) must not be inherited by children (breaks their writes)
  — solved by the node-in-the-middle topology anyway.
- No tap / probe failure → degrade to silent recording with a logged skip; never crash.

**Head transcription (0.1.15, validated design):** the recorder itself, once ~60 s of
segments exist, builds a temp closed VOD playlist over the head segments (they ARE decodable
mid-record; the live `preview.mp4` is not — faststart moov lands at close), extracts mono
16 k wav, runs whisper in the background → `head-transcript.json` ready **before stop**.
Empirical gate that justified it: titles authored from only the first 60 s matched
full-transcript titles in every titleable case (speakers state the topic up front). Gotcha
fixed in `e57bc32`: stop must await an in-flight head job with a bounded grace (~4 s) or the
recorder exits and orphans whisper mid-write.

### The single-tool endgame (researched, decided, deferred)

Question: is there ONE tool covering clean audio + live HLS + mp4 that kills ffmpeg AND the
mic tap? **Answer: yes — ScreenCaptureKit + AVAssetWriter covers everything.** The assumed
blocker ("no native live-HLS API") is wrong: `outputFileTypeProfile = .mpeg4AppleHLS` +
`preferredOutputSegmentInterval` + `AVAssetWriterDelegate.didOutputSegmentData` delivers an
init segment + `.m4s` fragments live; you write only the `.m3u8` text (WWDC20 "Author
fragmented MPEG-4 content with AVAssetWriter"). SCK gives screen+cursor+mic on one clock
(clean audio, inherent sync), VideoToolbox encode, and — because SCK is the sanctioned API —
**very likely retires the whole TCC/bypass-picker saga (§4)**. Costs: the entire recorder
becomes a large Swift surface (the reason the POC kept the 2-input design), and dual
HLS+MP4 needs two AVAssetWriters fed shared buffers or a finalize remux. GStreamer was
rejected (its only macOS screen source rides deprecated `AVCaptureScreenInput`); libobs
rejected (heavy, GUI-oriented, opaque vs the trust ethos). **For a v2 native app, SCK +
AVAssetWriter is the default candidate**, not the 2-input design — the 2-input design is
what you fall back to if v2 stays ffmpeg-based.

---

## 4. The macOS permission saga (TCC) — the tax the POC paid

Everything here is Sequoia-era (Darwin 25) empirical fact. The POC's launch chain
(agent → shell → shim → node → ffmpeg) is what made this hard; **a normally-launched,
properly-signed .app in v2 avoids most of it** — but the facts below explain *which* parts
remain (cdhash pinning, screen-grant timing, the bypass consent) regardless.

**TCC fundamentals learned:**

- TCC attributes a permission request to the **responsible process** (the session ancestor),
  not the requester. A shim launched from a terminal tree prompts as "Terminal"/"Claude"
  and the grant keys to *that* app.
- Fix shipped: **disclaim re-exec** — re-exec self once with
  `responsibility_spawnattrs_setdisclaim(attr, 1)` + `POSIX_SPAWN_SETEXEC` (private libc
  symbol via dlsym; keeps pid + stdout pipe; env-var guard for once-only). The running shim
  becomes its own TCC principal → prompts read "shroom". Full commented implementation:
  `archived-poc/scripts/shim/macos/Sources/main.swift` (top).
- **The disclaim does TWO jobs**: (1) name the prompt, (2) establish the responsibility
  boundary through which the **grant inherits to child node → grandchild ffmpeg**. This was
  proven when the "cleaner" LaunchServices-launch alternative (drop disclaim, launch via
  `open`) was tried: bundle identity was restored and prompts were correctly named, **but
  ffmpeg was denied frames** — 4× re-prompts, 0 segments ("Configuration of video device
  failed" in ffmpeg log). **Do not re-attempt "just drop disclaim + open".**
- Ship a real **`.app` bundle** (id `am.shroom`) with Info.plist usage strings + icon, even
  for a CLI-launched helper — Privacy panes then show a clean name/icon. The shim requests
  the **mic itself** (AVFoundation) so that prompt is also correctly named.
- **Ad-hoc signing gives a stable cdhash with no Apple account** — but TCC then pins the
  Screen-Recording grant **to the exact cdhash**: every rebuild/update orphans the grant
  while the Privacy toggle still reads ON (phantom state; capture denied). Shipped answer:
  when not authorized, self-reset our own entry (`tccutil reset ScreenCapture am.shroom` —
  sudo-free, needs LS registration) so re-granting is one clean toggle. The only true
  escape is a stable signing identity (identifier-based designated requirement) — i.e. a
  paid cert, rejected for the POC; **v2 must decide this consciously** (the rethink's
  no-cert stance is in direct tension with update-friction here).
- **Mic vs screen asymmetry** (both by design): mic is user-DB, grantable inline from the
  prompt → resolves within one run. Screen Recording can't be granted from its prompt
  (System-Settings toggle) and takes effect **only on the next process launch** — so the
  flow must be: register → user toggles → relaunch fresh → capture. First capture attempted
  in the *same* process wedges (that's the `screen_grant_inactive` wedge signature:
  "Configuration of video device failed, falling back to default" + zero frames; classify
  it and tell the user to just record again — `lib/wedge.mjs`).
- Inspecting the truth: screen grants live in the **system** TCC db
  (`/Library/Application Support/com.apple.TCC/TCC.db`, world-readable) —
  `select client,auth_value,hex(csreq) from access where service='kTCCServiceScreenCapture'`;
  decode csreq to see the pinned cdhash vs `codesign -dvvv`. (Mic lives in the user db.)

**The Sequoia "bypass private window picker" consent (separate from the TCC grant):**

- A second, date-throttled (~30-day) consent that fires at first *capture*, governed by
  `~/Library/Group Containers/group.com.apple.replayd/ScreenCaptureApprovals.plist`.
  Normally keyed by bundle id — but the disclaim re-exec **severs the LaunchServices bundle
  association**, so shroom's entry keys by **volatile file URL** (the install path), meaning
  every version bump (new path) re-fires it. The dialog also *displays the GUI session's
  controlling app* ("Claude"/"Terminal"), not shroom, because the shim isn't a foreground
  LS app at capture time — display name and storage key are resolved independently, and
  ffmpeg's (correct) responsible process is displayed by neither.
- Shipped fix (0.1.14) — fix the **timing**, accept the name: a throwaway ~1 s
  `AVCaptureScreenInput` capture (`--prime-capture`, later folded into `--prep`) provokes +
  clears the consent at **priming**, so it never lands mid-recording. Validated via the
  "fresh-path trick" (copy the .app to a replayd-unseen path → guaranteed first-use alert;
  same cdhash so grants still apply).
- Testing gotchas: replayd caches the plist via cfprefsd — `defaults delete` + `killall`
  looks like it works but gets rewritten; reliable reset needs a reboot or the fresh-path
  trick. The dialog is rendered by UserNotificationCenter and is **invisible to
  screenshot-based automation** (compositor-excluded) — a human must eyeball it.

**Priming architecture that emerged (0.1.16):** one native `--prep` call = mic request
(resolved fully first — never stack dialogs), screen preflight/register (+ stale-grant
self-reset), inline capture-prime when already granted, and staged recording identity —
so `/shroom:record` does *all* programmatic work before the picker and the tray launches
into a clean, silent world. Print machine-readable JSON on stdout; the orchestrator branches.

---

## 5. Upload, publish, and the page — architecture that worked and transfers whole

### The HLS insight (SPEC §5, fully vindicated)

**File-by-file upload and HLS-as-delivery-format are the same decision**, and it collapsed
four problems into one: pause/resume (= segment boundary), incremental upload (= idempotent
per-segment PUT, no multipart/ETags/5MB minimums), no-remux finalize (playlist carries
duration → the old SaaS's RemuxWorker stays dead), and delivery format (segments never
reassembled). **Playlist uploaded ONLY at finalize** — publishing the `.m3u8` *is* the
atomic go-live act; no viewer can ever see a half-written playlist, and nobody can watch
mid-recording. Two-phase publish overall: bytes (uploader's playlist PUT) then page
(deploy) — only the page step's `published` event carries the shareable `playbackUrl`.

### The uploader (`archived-poc/scripts/uploader/`)

- **"The recording is sacred"**: ffmpeg writes to disk regardless of network; the uploader
  is opportunistic background with backoff and **never blocks capture**; finalize uploads
  the remainder; on restart, diff local files vs bucket (HEAD) and upload the gap.
  Deterministic keys `<id>/<file>` (unguessable id = the prefix) + idempotent PUTs make
  resume trivial. Local disk is source of truth until confirmed up.
- **Zero-dependency SigV4** (`lib/s3.mjs`: PUT/HEAD/GET/DELETE/ListObjectsV2), verified
  byte-for-byte against botocore golden vectors — this is why the first real R2 PUT worked
  unmodified. Path-style addressing; works against R2/B2/AWS/MinIO. (For a SigV4 oracle use
  a throwaway botocore venv; aws-cli's bundled botocore is namespaced/unusable.)
- Degrade gracefully when the runtime lacks `fetch`: skip with an event, never a
  per-segment retry storm (a real P0 once: shim launched the recorder with the user's
  default Node 16 → no fetch → infinite retries → publish never completed; fixed by always
  launching through the version-selecting `run-node` AND by the graceful skip).
- `preview.mp4` and per-take intermediates are **local-only, never uploaded**; HLS only in
  the bucket, plus the optional download MP4.

### The download MP4 and the cross-origin trap

Pages site and R2 bucket are **different origins**, so the HTML `download="name"` attribute
is ignored — a downloaded file would be named by URL path. Two options existed:
`Content-Disposition` (needs the title at upload time) or **slug-in-key** (store the object
at `<id>/<title-slug>.mp4` so browsers name it from the path). Shipped: slug-in-key —
signer untouched, no metadata coupling. `archive-local` (upload MP4 + prune local HLS, each
half independently gated: upload needs storage, prune needs remote-confirmed) runs as
post-publish housekeeping **after** the title is written, since the slug needs it.

### Static per-video pages (`archived-poc/scripts/page/`, `templates/player.html`)

- **Per-video static HTML from one template, og: tags baked** — because unfurl crawlers
  don't run JS. This is what makes a link feel Loom-grade in Slack/iMessage. Poster
  extracted from preview.mp4 (best-effort). `og:video:duration`, twitter card, favicon.
- Player: `<video>` + hls.js only where needed (Safari plays HLS natively); one shared
  `/hls.min.js` at the site root (counted once against the Pages 20k-file limit); hls.js is
  **vendored explicitly, pinned + SHA-256-verified** (`vendor/fetch-hls.mjs`), never
  fetched at deploy. Chapter timeline (proportional seekable chunks), keyboard controls,
  copy-link/copy-embed, `?embed=1` chrome-less mode for iframes.
- **Escaping matters**: template rendering escapes for attribute/markup/JSON-island
  contexts, tested against `</script>` injection — agent/user-authored titles flow into
  committed public HTML.
- The publish step (`publish.mjs`) folds build + deploy + git-commit into **one process**
  so the orchestrator approves one command, not three.
- **Instant link is a product law**: publish never waits on transcription. Title at stop
  (head transcript → suggested name, or typed), link live immediately, then the full
  transcript **enriches the same stable URL in the background** (re-render + re-deploy,
  idempotent). Renames/enrich re-use the identical publish path.
- **`build-page` reads title/tldr/chapters ONLY from `--meta`** — every re-publish must
  pass it or the page regresses to "Untitled" (bug class hit twice; fixed everywhere, but
  any v2 "re-render" path must keep metadata mandatory).

### Library / metadata

git repo as the DB: one `<id>.md` per recording (frontmatter + transcript body), written
only by the deterministic `write-meta.mjs` (stable escaping/key order; judgment supplies
only title/tldr/chapters). Commit is automatic + best-effort (`commit_skipped` /
`commit_noop` never fail a publish). Secrets never in the library;
`~/.shroom/credentials.json` (mode 600) holds S3 keys + public bases + accountId + library
path, written **merge-not-clobber** so re-runs and later top-ups preserve fields.
**v2 note (the rethink):** local git as source of truth dies with multi-user — a shared
mutable index invites lost updates; design per-recording objects or a Worker-mediated index.

---

## 6. Cloudflare / wrangler — the empirical map

All settled live; this is exactly the knowledge a v2 setup wizard (or BYO-backend deployer)
needs. Reference implementation: `archived-poc/scripts/setup/` (+ `lib/wrangler-errors.mjs`
error catalogue).

**Auth model (two credentials, forced by Cloudflare's own limits):**

- **OAuth (wrangler login) with narrow scopes** `account:read user:read pages:write` — no
  token paste; `offline_access` is auto-appended (passing it as `--scopes` errors). Access
  token TTL ≈ 16–24 h with silent refresh. The "missing expected OAuth scopes" warning is
  harmless (the ~24 default scopes we deliberately skip).
- **There is NO R2 OAuth scope** (full 27-scope list checked). `r2 bucket create` over
  OAuth → `Authentication error [10000]` even on a verified, R2-enabled account. So R2
  requires a **dashboard-created Account API token** (Admin R&W): its value drives
  `wrangler r2` via `CLOUDFLARE_API_TOKEN`; its derived Access Key ID/Secret are the S3
  keys. Programmatic token minting needs a dashboard bootstrap anyway — not worth wiring.
- **`wrangler pages project create` refuses OAuth non-interactively** (hard CLI guard
  demanding `CLOUDFLARE_API_TOKEN`); **`wrangler pages deploy` rides OAuth fine**, even
  non-interactively (clean-VM proven). Workaround: create the project via REST
  (`POST /accounts/{id}/pages/projects`) with the OAuth token as Bearer — it carries
  `pages:write`; only the CLI guard rejected it.
- **Freshest-token bug**: wrangler maintains multiple config paths; a stale token file at
  one path can shadow the refreshed one at another (whoami refreshes but doesn't rewrite
  the orphan). Read ALL candidate configs and pick the one with the **latest future
  `expiration_time`**.
- `wrangler whoami` **exits 0 when logged out** — parse the text ("not authenticated"),
  not the exit code.

**Account gates (probe capability, not state — there is no clean "verified" flag):**

- Email verification blocks R2/token pages *and* in-scope writes. Detect by attempting a
  real write: `POST pages/projects` on an unverified account → HTTP 403 code **8000077**
  ("email must been verified", sic — match the numeric code, not the typo'd string).
  Google-SSO signups are pre-verified; email signups hit both gates.
- Enabling R2 requires a **credit card** even for the free tier — a genuine adoption
  blocker; pre-warn it, and surface both dashboard gates together (one browser trip).
- The whole setup is a **classified-error state machine**: attempt the op, classify
  (`not_logged_in` / `email_unverified` / `r2_not_enabled` / `needs_payment` /
  `insufficient_scope` / `r2_token_required`), route to the fix, auto-poll and retry —
  which makes setup **idempotent and resumable for free**. `already_exists` counts as
  success everywhere.

**Facts of the platform:** `*.r2.dev` public dev URL = zero-DNS public origin (rate-limited,
"not for production", fine for personal unlisted links); Pages URLs carry a random project
suffix (`shroom-site-eym.pages.dev`) — **parse the deploy output, never construct**; Pages
free tier ≈ 500 builds/mo and 20k files/site (per-video pages approach the file cap around
~10k videos; the only real dollar cost in the whole system is R2 storage ~$0.015/GB-mo,
egress $0); public-bucket enable is security-weakening → explicit consent + `--force`.

**Environmental trap (Bagrat's LAN):** a path-MTU black hole hangs `wrangler pages deploy`
uploads specifically (bodies ≳64–96 KB to the Pages asset-upload endpoint stall; R2 uploads
fine; plain curl reproduces it — NOT a shroom/wrangler bug). Fix: clamp MTU 1400
(`sudo networksetup -setMTU "Wi-Fi" 1400`). Publish paths need timeouts + a local-fallback
result (shipped: 90 s timeout → `publish_local {deployFailed:true}` + retry offer) because
a wedged deploy must never hang the flow. Also why "serve hls.min.js from R2 instead of
re-shipping 415 KB per deploy" stayed on the wishlist.

---

## 7. Distribution & release (plugin era) — what transfers

- **3-repo model**: product repo (owns `plugin.json` version; hosts the landing page from
  `docs/` via GitHub Pages → **shroom.am**) · thin generic marketplace repo
  (`bagrat/claude-plugins`, one unpinned entry per plugin — users add it once) · old SaaS
  repo renamed away. Release = bump `plugin.json` + push; marketplace repo untouched.
- **The version-keyed cache gotcha**: installed plugins live at
  `~/.claude/plugins/cache/…/<VERSION>/` — while the version string is unchanged,
  marketplace update + reinstall silently reuse the stale cache. Every user-visible change
  MUST bump the version (dev loop pain; one more reason the POC dev-loaded via the skills
  symlink instead).
- **Claude Desktop cannot install third-party plugins** (browse-only curated directory;
  verified on a clean VM) — the POC was CLI-only, a real reach limiter. `/reload-plugins`
  required after install (easy to miss). No install/post-install hook exists → shipped a
  version-marker **post-update runner** (`scripts/version/post-update.mjs` +
  `migrations.json`: per-version whatsNew + proposed actions like rebuild-shim; advances a
  last-seen marker so each fires exactly once, fail-soft) and a fail-soft **update check**
  (fetch plugin.json from main, semver-compare, one-line suggest, never auto).
- **shroom.am DNS** (registered at domain.am, DNS panel there, on Cloudflare NS): GitHub
  Pages apex A/AAAA records must be **DNS-only (grey cloud)** — proxied records make
  GitHub's domain check see Cloudflare IPs and refuse to activate (404). Cert provisioning
  takes ~30–60 min of `state: null` — don't thrash.
- **On-device compile as trust stance**: ship readable Swift, compile at setup (needs
  Xcode CLT; ~4 s), render even the icon from code (no committed blobs), ad-hoc sign. This
  was the POC's answer to "no certs, no opaque binaries" — v2's unsigned-app distribution
  problem (raised in the rethink) is the same question at bigger scale.
- Operational: from the agent's Bash, `git push` works but `gh` cannot read the keyring
  token — Bagrat dumps `gh auth token > /tmp/ghtok`, use `GH_TOKEN="$(cat …)"`, delete after.

---

## 8. The agent-first experience — honest retro (why v2 exists)

What the rethink already concluded: runtime LLM need is ~zero; agent-first was
an architecture shortcut (offload complexity to the agent), not product value. The concrete
frictions behind that conclusion, so v2 doesn't romanticize either direction:

**What genuinely hurt:**

- **The LLM was the event loop.** Background tasks (shim, transcribe, archive) complete →
  re-invoke the model → prose in `record.md` tells it how to branch on `events.ndjson`.
  That's a 500-line natural-language state machine you can't unit-test, with model
  nondeterminism in every transition. It mostly worked — and every "mostly" cost a session.
- **Latency + round-trips on the hot path.** Each pre-record check was a tool call with
  consent UX until the one-shot `preflight.mjs --prep` collapsed 4–5 round-trips into one
  (~1 s). stop → link involves model turns by design. An app does these in-process.
- **Product-voice policing never ended.** Internal names (`Path A`, `lastProfile`,
  `ready`), meta-narration ("let me offer…"), and infra nouns kept leaking into user-facing
  chat; the fix (write the said-line into the instruction, not the mechanism) helped but
  required constant auditing. A GUI simply doesn't have this failure mode.
- **Consent/permission UX fought the harness**: compound shell (`if [ -d … ]`) trips scary
  "can't analyze" prompts → rules like "Read tool, not cat; one script call, not
  hand-assembled shell"; `allowed-tools` lists needed auditing; secrets had to ride files,
  never argv (they land in consent prompts/ps/history).
- **Hooks were unreliable as UX**: the SessionStart "not configured" nudge injected context
  the model often didn't surface + nagged every session → replaced by a deterministic gate
  at the point of need (record refuses politely until setup ran). General lesson: **gate at
  use, don't nudge at start; and don't build UX on "the model will probably mention it".**
- **Dev loop friction**: version-keyed cache staleness vs live-editing via a skills-dir
  symlink — which broke `argv[1] === import.meta.url` entry-point guards (symlinked argv vs
  resolved import URL → `main()` silently skipped, scripts "ran" printing nothing; fix:
  `realpathSync(argv[1])`). Node-version reality (default Node 16/20 without `fetch`)
  forced a `run-node` selection shim for every invocation.
- **Distribution ceiling**: CLI-only install (see §7) capped the audience the product
  claimed to serve.

**What worked and must carry into v2 (in some form):**

- **The determinism boundary** — mechanism in exact/repeatable scripts, judgment above —
  kept every mechanism testable offline (seam injection everywhere: fake run/lookup/
  wrangler/whisper/S3) and is why 10 suites ran green with no network/binaries. In v2 this
  becomes "core engine vs UI/optional-AI garnish", but the discipline is the same.
- **events.ndjson as append-only truth + durable recovery artifact**: crash-safe,
  drainable after session death, debuggable after the fact (`dev/collect-logs.mjs`), and
  the natural seam between recorder and any supervisor (model, app, or test).
- **The armed/consent boundary**: launcher ≠ capture-starter; a human clicks Start. Even
  app-first, "no capture without a deliberate human act" is the right default.
- **Fail-soft as a contract** for every non-core path (update check, transcription, head
  transcribe, commit, prime): degraded output + an event, exit 0, never block the
  recording or the link.
- **Idempotence + probe-capability-not-state** in setup: re-run always safe, resume free,
  no stored "am I verified" state to go stale.
- **Value before friction**: first record renders locally/instantly (`preview.mp4` via the
  tee, `file://`-playable with zero JS — deliberately a *different* format from HLS, which
  Chrome blocks on `file://` null-origin CORS); cloud setup is pitched after you've seen it
  work.
- **LLM judgment where it's cheap and real**: transcript → title/TL;DR/chapters authoring
  rubric (grounded in transcript, no invention, chapters only ≥ ~2 min with real topic
  shifts) worked well — in v2 this survives as optional/BYO-agent garnish, exactly as the rethink
  concluded.

**Tray/overlay UX findings (transfer to any v2 recorder verbatim):** one obvious action per
state (armed → click = start; recording → click = instant pause + menu; Stop/Discard stay
deliberate menu choices so a stray click can't publish or destroy); cancelable 3-2-1
countdown; menu opens only after the recorder confirms pause (never catch the menu in the
last frame); Discard is a 4-way panel (Keep · Resume · Restart · Discard); Restart re-arms
with the countdown running in parallel with the relaunch. Fullscreen needs an overlay (the
menu bar autohides): borderless clear window at `CGShieldingWindowLevel`,
`canJoinAllSpaces + fullScreenAuxiliary`, shown via `orderFrontRegardless` (never
`NSApp.activate`), buttons hit-tested in `mouseDown` — the proven recipe for taking clicks
over other apps' fullscreen Spaces while staying an inactive accessory app
(`Sources/Overlay.swift`).

**Setup UX findings (transfer to any wizard):** welcome first (what it is + real cost
figures framed cheap: ~27 MB/min, free tier ≈ 6 h, 100 h ≈ $2.40/mo) → silent read-only
preflight → the plan shown **once, pre-marked** with real ✅/⬜ (never a blank plan that
gets reprinted) → one step at a time, re-rendering progress; consents are real questions
with ≥2 self-explanatory options; batch installs into one approval; pre-warn both dashboard
gates together; never name the user's ambient email; secrets pasted → written to a file the
provisioner consumes and deletes.

---

## 9. Testing harness learnings

- **Three layers**: unit (seam-injected, offline, seconds) → script-direct against the real
  machine → clean-VM for what only a pristine Mac proves (real installs, first-run TCC).
  PATH-narrowing (`PATH=/usr/bin:/bin`) fakes "everything missing" for install-plan UX with
  zero side effects.
- **Tart VM harness** (Apple-Silicon macOS VMs): golden `shroom-clean` from
  `ghcr.io/cirruslabs/macos-sequoia-base` (deliberately no brew/CLT; admin/admin; Claude
  Code installed) — never run the golden, always CoW-clone per test, delete after. Install
  Tart from the notarized release tarball (brew may source-build and fail on old CLT).
  **The `--dir` shared mount corrupted/truncated files** → test via the published git path
  (bump + push per iteration), not the mount. Log exfil via a second writable mount + a
  redacting collector. TCC prompts need real human clicks in the VM GUI.
- **Whisper detection**: presence-only PATH lookup, never execute — `whisper --help`
  imports torch and can blow timeouts, flapping to "absent". Only run tools whose version
  you must parse.
- **Kill process trees, not processes**, in lifecycle tests: launch each recorder
  `detached:true` (group leader) and `process.kill(-pid, 'SIGKILL')` in a finally +
  exit/SIGINT/SIGTERM net — killing just the parent orphans fake children (a leaked trio
  once ran 27 min).
- Shell-env trap: the harness's non-interactive shell may resolve an ancient node; run
  suites with an explicit modern binary (or via `run-node`) or `fetch`-dependent tests lie.

---

## 10. Reusable-assets inventory (in `archived-poc/`)

Direct-reuse candidates for v2, roughly in order of value:

| Asset | Where | Why it transfers |
| --- | --- | --- |
| ffmpeg tee recipe + constants | `scripts/recorder/lib/{config,ffmpeg,quality}.mjs` | the validated capture core, if v2 stays ffmpeg-based |
| Native mic tap | `scripts/shim/macos/Sources/MicTap.swift` | clean-audio capture + CoreAudio by-name selection, reusable as-is |
| Disclaim re-exec + TCC prep | `scripts/shim/macos/Sources/main.swift` (top ~240 lines) | the whole permission dance, commented; needed by any non-LS-launched helper |
| Fullscreen overlay technique | `scripts/shim/macos/Sources/Overlay.swift` | countdown/confirms over fullscreen Spaces — hard-won window-level recipe |
| SigV4 client | `scripts/uploader/lib/s3.mjs` (+ `test/sigv4.test.mjs` golden vectors) | zero-dep S3 for any BYO-storage uploader |
| Uploader queue semantics | `scripts/uploader/lib/uploader.mjs` | sacred-recording, gap-diff resume, publish-last gate |
| Take assembly / finalize | `scripts/recorder/lib/finalize.mjs` | master-playlist + preview concat across pauses |
| Head transcription | `scripts/recorder/lib/head-transcribe.mjs` | instant-title mechanism over closed segments |
| Player page + renderer | `templates/player.html`, `scripts/page/lib/render.mjs` | unfurl-correct static player w/ escaping tests, chapters, embed mode |
| Cloudflare error catalogue + provisioning | `scripts/setup/lib/{wrangler-errors,cloudflare}.mjs` | the classified-gate state machine any wizard needs |
| Node detection / selection | `scripts/setup/lib/node-detect.mjs`, `scripts/runtime/run-node` | only if v2 keeps a Node runtime dependency |
| Post-update migrations pattern | `scripts/version/` | version-marker + per-version whatsNew/actions, fail-soft |
| Diagnostics collector | `dev/collect-logs.mjs` | secret-safe session bundling; adapt paths |
| VM runbook | `TESTING.md` | full Tart golden/clone workflow |

---

## 11. Loose ends & live obligations at archive time

Things that outlive the POC and need an owner or a decision:

1. **The unpushed archive commit** (`92a7169`): before pushing, decide the marketplace
   story (delist shroom / pin the entry to the 0.1.16 tag / accept breakage) and the landing-page story (shroom.am serves from `docs/`
   on main, which moved into `archived-poc/`).
2. **Cloudflare test accounts** (a test account): a support case is open;
   self-service deletion unblocks ~**2026-07-28** when the R2 subs lapse. Passwords +
   account ids + steps are in project memory (`cloudflare-test-account-teardown`).
3. **shroom-site production deployment** was overwritten with test pages during the MTU
   diagnosis; rollback target `bfa510d5` via the Pages `/rollback` REST endpoint — parked
   by Bagrat (2026-06-30). Moot if v2 replaces the site.
4. The skills-dir symlink `~/.claude/skills/shroom` → this repo is now a defunct plugin
   mount (plugin.json moved); remove or repoint when v2 tooling lands.
5. Host ffmpeg drifted to **8.1.2** (a keg-reinstall side effect of the ffmpeg@4 audio
   test) — irrelevant to the archived POC, but any resurrected recipe/version-range check
   should expect 8.x.
