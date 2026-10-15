# shroom — v2 pre-decision research

> Status: **research input to the stack/brainstorm session, nothing decided.** Produced
> 2026-07-19 by six parallel deep-research agents over the rethink's research questions
> (**R1–R6, R8–R11**), scoped by the rethink's decisions. Every section ends in a
> recommendation **with rejected alternatives**; claims carry label keys ((a)/(b)/(c)… —
> each section defines its own, they vary slightly).
>
> **Update 2026-07-20:** the positioning decision (part of the stack decision, 2026-07-20: build
> shroom as the minimal no-server extreme) unblocked **R7**, researched and added below. The
> stack session now concentrates on **R3 · R6 · R7**.
>
> Section order preserves the original research pairings (internal cross-references point
> within pairs): **R1·R9** capture & renditions → **R2** shell → **R3** distribution →
> **R4·R5·R6** delivery → **R7** provisioning → **R10** layout & auth → **R8·R11**
> transcription & physics.

---

## Cross-cutting synthesis (read this first)

### The shape that emerges

If all six recommendations are taken together, a coherent candidate stack falls out — a
candidate for the session, not a decision:

- **Tauri v2 tray shell** over a **sealed Swift SCK + AVAssetWriter helper** (macOS 15
  floor; ~2.5–4.5k lines of boring Swift), webcam **bubble in v1** via the
  captured-preview-window trick (P7's "iff cheap" condition is met).
- **Single 1080p rendition** with a **master playlist + rendition-scoped keys from day
  one** — the v2 ABR ladder pre-paid, never retrofitted.
- **One Worker**: `/v/<id>` player + oEmbed + media proxy over a **fully private bucket**.
  Zero-DNS default = workers.dev; custom domain = the honestly-better tier. The page
  exists the moment the id is minted (P9), tombstones included (P2).
- **Bucket = the only source of truth**: per-recording `meta/<id>.json` objects,
  list-on-read, no central index, local-git SoT deleted. Two-Mac v1 (P12) works with no
  coordination service; the v2 team model extends it without migration.
- **Distribution inside the $0 line**: script/tap install (no quarantine), per-machine
  self-signed identity re-signed on every update, Sparkle-EdDSA-or-Tauri-updater.
- **Transcription: Apple SpeechAnalyzer by default** (local → transcript stays local,
  resolving P3), whisper.cpp downloaded-on-demand as fallback and the only
  cross-platform engine, Workers AI as a real-but-later door.
- *(added 2026-07-20)* **Setup: a deterministic in-app wizard over raw REST — no wrangler,
  no node** (R7). One pre-filled-URL token paste; ~6 calls provision bucket + Worker +
  workers.dev; the token itself doubles as the S3 credential (key-derivation rule). The
  BYO-agent skill narrates the same sequence, it doesn't own a second implementation.

### Findings that changed the picture

1. **`SCContentSharingPicker` needs no Screen-Recording TCC grant at all** (R1) — the only
   capture posture where the no-cert stance has *zero recurring cost* (no per-update
   re-grant, no monthly nag). Price: a system picker click per recording.
2. **Terminal-mediated installs never set quarantine** (R3) — script/tap installs have
   zero Gatekeeper friction, legitimately. The Homebrew **cask** door closes for unsigned
   apps Sept 2026; the **formula/tap** door (yabai model) stays open.
3. **R2 storage is strongly consistent, lists included** (R10) — per-recording objects are
   lost-update-free by construction, and a central `index.json` would have been a privacy
   hole (world-readable id enumeration) on top of a consistency hazard.
4. **Inline-playback unfurls are allowlist-gated on Slack, Teams, Linear and X** (R5) —
   there the honest requirement is "rich card + click-through." Genuine inline wins:
   iMessage and Discord via og:video, which must point at an **MP4, never HLS** — drafting
   a small **teaser MP4** into the design. (The POC's "working" unfurls were image cards.)
5. **r2.dev measured fine but is rejected as the default** (R6): HTTP/1.1-only, no edge
   cache, officially "not for production" — and the POC bucket had **no CORS**, so
   published POC links were likely silently broken in Chrome/hls.js all along.
6. **The MP4 tee is the enemy of "link now"** (R11/R9): it doubles upload volume and hits
   R2's ~5 GiB single-PUT wall at **59 min of 4K** — multipart (~150 lines in the SigV4
   client) is mandatory under unbounded length, and the MP4 should be second-class.
7. **Loom ships a 3-rung ladder** (R9, probed live); dual realtime encode is cheap on base
   M1 — the real second-rung cost is **+~40% upload**, hurting stop→link, not the wallet.
8. **R4 bonus**: an HMAC-in-the-id trick makes never-404 *verifiable* offline with zero
   storage — unknown ids 404, valid-but-not-yet-uploaded ids render "on its way."
9. *(added 2026-07-20, R7)* **The pasted R2 token IS the S3 credential** (Access Key = token
   id, Secret = SHA-256 of the value) — the POC's "derived keys shown once in the dashboard"
   bootstrap is obsolete. And Cloudflare's June-2026 **self-managed OAuth clients** (scopes =
   the full token-permission catalog, PKCE public clients) dissolve the old no-R2-OAuth-scope
   wall — kept as a zero-paste door, not the baseline.

### Seams and contradictions the session must resolve

- **Key layout**: R9 wants rendition-scoped media keys (`rec/<id>/r1080/…`); R10's layout
  says `rec/<id>/hls/…`. Adopt rendition-scoping *inside* R10's scheme — a one-line
  change; R10's meta/tombstone/consistency design is unaffected.
- **Worker in the read path**: R6's private-bucket default means the Worker exists from
  v1 day one (R10 designed v1 *writes* to need no Worker — still true; the two compose).
  This further cements R4's "mandated" status.
- **Picker vs grant**: picker-per-recording (zero TCC, one click each time) vs one-time
  Screen-Recording grant (re-grant/nag taxes, R3's E1 trick may fix) is a genuine product
  UX call, not a research question.
- **The one existential open fact**: a public bug report (Apr 2026) claims Sequoia rejects SCK for
  ad-hoc-signed apps without a Team ID (R2) — contradicted by the POC's own working record
  and likely a phantom-grant misdiagnosis (R3), but it must be settled empirically first.
  **✅ SETTLED 2026-07-20 (b): retired.** SCK captures at full rate on Tahoe 26.5 under a
  self-signed no-Team-ID identity (E1) — see "Hands-on probe results".

### The probe queue (cheap, decisive)

1. **E1** (R3 §7 ≈ R2 probe #1, ~30 min): ad-hoc + self-signed-identity SCK capture across
   an update on Tahoe — settles the Team-ID alarm *and* the zero-dollar stable-identity trick
   in one experiment. The single highest-stakes fact in the whole research set.
   **✅ EXECUTED 2026-07-20, PASSED — and E2 passed on top** (grant survives update under a
   self-signed cert; validation is trust-free ⇒ 5b developer-side signing is live; the
   Team-ID alarm retired). Full results: "Hands-on probe results" section at the end.
2. **Picker filter-reuse** (R1): can the picker's selection be reused across recordings?
3. **Dual-writer endurance** (R1): one VTCompressionSession → two passthrough
   AVAssetWriters, hours-long run on Apple Silicon.
4. **R10's two ~20-min R2 probes**: `If-None-Match: *` conditional-PUT semantics;
   prefix-scoped temporary-credential mint via the POC's own SigV4 client.
   **Temp-creds half ✅ (2026-07-20)** — passed in full, incl. cross-token-class semantics
   (R7.7a addendum + R10.4 #2). Conditional-PUT half still open.
5. **SpeechAnalyzer on this machine** (R8, via Yap): locale list, word-timing granularity,
   2 h runtime.
6. **Synthetic 6 h playlist** in hls.js/Safari (R11); one curl to Workers AI Whisper (R8).
7. *(added 2026-07-20)* **R7's hand-run wizard** (probe P3) — **✅ EXECUTED same day,
   passed in full** (P1/P2/P4 settled with it): deep-link prefill works, minimal token
   covers every call, S3 derivation confirmed, composed streaming loop live on workers.dev
   in ~10 min. Results: R7.6. Still open from R7: **P5** (OAuth client probe, decided to
   run early).
8. *(added 2026-07-20, session decision)* **The scap capture probe — now gates the core/shell
   ratification**: the MIT `scap-screencapturekit` crate driving SCK on Bagrat's machine,
   frames into the POC's validated ffmpeg recipe. Decides no-Swift-Rust-core vs the R1 Swift
   helper; scaffolding waits on it. (Needs a Rust toolchain — propose-then-install, house
   rules.)
   **✅ EXECUTED same day, PASSED in full** — no toolchain install needed (brew rust 1.88.0
   already present = the crate's own pin); solid 30 fps 4K BGRA into the POC recipe, artifacts
   shape-identical to POC output; zero Swift. Core/shell ratification unblocked. Full
   results + gotchas: "Hands-on probe results" section at the end.

### Rethink decisions the research resolved

- **P7 → bubble is in v1** (the "iff SCK makes it cheap" condition is met).
- **P3 → transcript stays local by default** (the default engine is local; the Cloudflare
  door, if opened later, moves the words with it — stated per engine in R8).
- **P14's invoice → near-zero recurring cost** if E1 passes (picker path: zero even
  without E1); worst case one Settings toggle + relaunch per update. What $99/yr would
  still buy shrinks to "a nicer DMG story" (R3). *(2026-07-20: E1 passed — update-time
  TCC cost measured at zero; only Apple's own periodic re-confirm policy remains.)*

---


> Labels: **(a)** verified from official docs/WWDC · **(b)** empirically tested by me today ·
> **(c)** community-reported · **(d)** inference. Confidence stated on load-bearing conclusions.

## R1 — ScreenCaptureKit + AVAssetWriter as the capture core

**TL;DR verdict**

- **SCK + AVAssetWriter is real and retires the whole POC tax** — live fMP4/HLS segments via
  `AVAssetWriterDelegate`, screen+mic on one clock, no ffmpeg, no MicTap, no avfoundation audio bug.
  Confidence: high.
- **The permissions win is bigger than POC §3 hoped, but it forks**: the `SCContentSharingPicker`
  path needs **no Screen-Recording TCC grant at all** (no cdhash orphaning on update, no Sequoia
  monthly nag) at the cost of a system picker in the record flow; the classic `SCShareableContent`
  path keeps the pre-filled-config UX but keeps the TCC toggle + **cdhash re-grant on every unsigned
  update** + the monthly-ish nag. This choice is the sharpest product decision R1 surfaces.
- **Dual HLS+MP4 output is a proven-parts pattern, not fantasy**: one `VTCompressionSession`, two
  writers with passthrough (nil `outputSettings`) is documented AVFoundation behavior; the boring
  fallback (finalize-time `-c copy` remux from segments) is cheap. Confidence: high on the parts,
  medium on the exact combination — one 1-day probe closes it.
- **P7 answer: the webcam bubble is cheap — it goes in v1.** Two near-free paths exist (Apple
  Presenter Overlay on 14+, or a floating always-on-top preview window that display capture picks up
  for free — QuickRecorder ships both). Manual GPU compositing is the polish path, not required.
- **Bindings verdict: Rust bindings are unexpectedly mature (screencapturekit-rs v8, active July
  2026), Node bindings are wrappers around Swift anyway** — but none remove the need to own
  AVAssetWriter/HLS authoring logic, so a sealed Swift helper remains the honest default; Rust is a
  credible competitor only if the shell is Tauri and you accept a fast-churning dependency.
- **Honest Swift surface: ~2,500–4,500 lines for full v1**, not "a few hundred." (POC shim was
  already 1,321 lines *without* owning capture or encode.)

---

### 1. Dual output: HLS + faststart MP4 from one capture

**Fact base.** AVAssetWriter has first-class fMP4/HLS authoring: set
`outputFileTypeProfile = .mpeg4AppleHLS`, `preferredOutputSegmentInterval`, and receive init +
`.m4s` segments live via `AVAssetWriterDelegate` — (a) [WWDC20 10011](https://developer.apple.com/videos/play/wwdc2020/10011/),
(a) [Apple: Writing fragmented MPEG-4 files for HLS](https://developer.apple.com/documentation/AVFoundation/writing-fragmented-mpeg-4-files-for-http-live-streaming).
You author the `.m3u8` text yourself — exactly the POC's playlist-assembly code territory (POC §2's
per-take playlists + discontinuity join transfers conceptually intact).

**Is two-writers-from-shared-buffers proven?** The parts are all documented:

- `AVAssetWriterInput` with **nil `outputSettings` = passthrough**: "appended samples are passed
  through, doing no processing… useful if you are appending buffers that are already in a desirable
  compressed format" — (a) AVAssetWriterInput header/docs, discussed in
  [Apple forums 680903](https://developer.apple.com/forums/thread/680903). H.264 in MP4 is within the
  supported passthrough set.
- So the one-encode/two-mux design is: SCK raw frames → **one `VTCompressionSession`** → the same
  compressed `CMSampleBuffer`s appended to (1) the HLS-profile writer and (2) a plain
  `.mp4` writer with `shouldOptimizeForNetworkUse` (faststart). Muxing is nearly free; there is
  **no second encode**. (d) inference from documented parts, high confidence; I found no public
  end-to-end sample of this exact pair → **cheap probe below**.
- Simpler-but-costlier alternative: two writers each doing its own encode (2× VideoToolbox). Proven
  category — every OBS "stream + record simultaneously" setup does two concurrent VT encodes (see
  R9 §2); costs a second hardware-encode session, which base Apple Silicon shrugs at.

**Fallback: finalize-time remux.** Because segments are CMAF/fMP4, three options, cheapest first:

1. **Byte-concat** `init.mp4` + all `.m4s` → a valid *fragmented* MP4 file. Free, but fMP4 downloads
   have wonky duration/seek behavior in some players (moov carries no duration; no sidx/mfra).
   OK-ish for a nice-to-have download, janky. (d)
2. **`ffmpeg -i stream.m3u8 -c copy -movflags +faststart`** — clean progressive MP4, IO-bound
   (a 2 h/3.2 GB recording remuxes in tens of seconds on SSD; faststart pass rewrites the file
   once). Keeps an ffmpeg dependency for one *cold, non-capture* path only. (d), high confidence —
   this is bog-standard ffmpeg remuxing.
3. **`SCRecordingOutput`** (macOS 15+): SCK records straight to file, attachable *alongside*
   sample-buffer outputs on the same `SCStream` (`addRecordingOutput`) — (a)
   [WWDC24 10088](https://developer.apple.com/videos/play/wwdc2024/10088/). Caveats: it's a second
   encode; one credible report of **corrupted MP4 when `captureMicrophone = true`** —
   (c) [Apple forums 805892](https://developer.apple.com/forums/thread/805892); crash behavior
   (non-finalized moov?) and faststart status undocumented. Not the primary choice.

**Known writer-path landmines (all avoidable, all with receipts):**

- **Use `.mpeg4AppleHLS`, not `.mpeg4CMAFCompliant`**: the CMAF profile crashed with
  variable-frame-rate realtime input (which SCK produces) on Intel Macs —
  (c) [Nonstrict, CMAF crash](https://nonstrict.eu/blog/2023/avassetwriter-crash-when-using-CMAF/).
  Apple-HLS profile unaffected.
- **Segment-`Data` bridging leak in Swift delegates** ("600 MB+ for a 3-minute recording") — fixed
  in **macOS 13.3+**, moot at a 14/15 floor, but know it exists if the floor ever drops —
  (c) [Nonstrict, segment-data leak](https://nonstrict.eu/blog/2023/avassetwriter-leaks-segment-data).
- **Audio gaps on device switch** (AirPods connect mid-recording): CoreAudio-level, hits
  AVCaptureSession and AVAudioEngine alike; detect PTS discontinuities and insert silence or the
  audio track runs short and desyncs — (c) [Nonstrict, audio gaps](https://nonstrict.eu/blog/2024/handling-audio-capture-gaps-on-macos).
  The POC never had to solve this (native tap + wall-clock PCM); the SCK mic path must watch for it.

**Multi-hour (P6: unbounded) behavior.** The HLS-delegate design is structurally right for long
runs: segments are handed to the delegate and dropped; nothing accumulates in the writer (the
plain-MP4 leg accumulates only sample-table metadata; the passthrough writer's tables for a
multi-hour 30 fps track are tens of MB at worst). Crash-resilience matches §5.5: closed segments +
rebuildable playlist = valid-but-truncated video; the MP4 leg is regenerable from segments, so its
crash-fragility doesn't matter. (d), medium-high confidence; no public long-run RSS data found →
probe below. Note the POC's per-take model (new writer per take, playlist discontinuity join)
also bounds any per-writer growth naturally.

### 2. Minimum-OS matrix and the defensible floor

| Feature | Minimum macOS | Source |
|---|---|---|
| SCK core capture (`SCStream`, `sourceRect` crop, window filter) | 12.3 | (a) Apple docs |
| System-audio capture (`capturesAudio`, 1–2 ch max) | 13.0 | (a) [OBS PR 6600](https://github.com/obsproject/obs-studio/pull/6600) |
| `SCContentSharingPicker` (no-TCC path), `SCScreenshotManager` | 14.0 | (a) WWDC23 10136 |
| Presenter Overlay (system feature; privacy-alert config 14.2) | 14.0/14.2 | (a) WWDC23; (c) screencapturekit-rs |
| **Mic capture (`captureMicrophone`, `microphoneCaptureDeviceID`)**, `SCRecordingOutput`, HDR | **15.0** | (a) [WWDC24 10088](https://developer.apple.com/videos/play/wwdc2024/10088/) |
| SpeechAnalyzer/SpeechTranscriber (R8 adjacency) | 26 | (a) WWDC25 |

Install-base mid-2026: Tahoe (26) is on a clear majority of active Macs; Sequoia (15) holds roughly
a fifth and falling; Sonoma-and-older are the tail — (c)
[TelemetryDeck macOS survey](https://telemetrydeck.com/survey/apple/macOS/versions/) (69% on 26.5 in
June 2026, in an updates-skewed population),
(c) [businessstats modelled view](https://businesstats.com/worldwide-macos-version-market-share/);
StatCounter is unusable for modern macOS splits (reports everything as Catalina).

**Verdict: floor = macOS 15 (Sequoia).** It covers ~85–90 % of active Macs in mid-2026 and rising,
and it buys the whole feature set natively — SCK mic kills the AVAudioEngine MicTap, the fifo, the
16 MB buffer topology, and the deadlock/stop-ordering gotchas (POC §3) outright. Below-floor compat
via the POC MicTap is *possible* (SCK video 12.3+ + AVAudioEngine mic) but reintroduces exactly the
plumbing v2 exists to delete — reject it; people on 13/14 keep the POC-era answer or upgrade.
A 26-only floor is too aggressive in 2026 (~25–30 % cut); gate SpeechAnalyzer at runtime instead.

### 3. Permissions: what the §4 saga actually reduces to

What a **normally-launched, bundle-identified .app doing capture in-process** erases, regardless of
path: all *attribution* pathology — disclaim re-exec, wrong-name prompts, grant inheritance to
child node/ffmpeg, the severed LaunchServices association that made the bypass consent key by
volatile file URL (POC §4). The capture process *is* the app; prompts name it; nothing inherits.
(d) from TCC mechanics + POC's own findings, high confidence.

What remains depends on which SCK front door you use:

**Path A — `SCContentSharingPicker` (macOS 14+): no Screen-Recording TCC at all.**
"Your app is not required to obtain separate permission to capture the screen… because of the
explicit action, your app is permitted to record selected content for the duration of the session"
— (a) [WWDC23 "What's new in privacy" 10053](https://developer.apple.com/videos/play/wwdc2023/10053/);
independently confirmed: "the programmatic API requires the user to enable Screen Recording
authorization… Because the new picker UI is based on user interaction, this is not needed" —
(c) [Nonstrict on Sonoma SCK](https://nonstrict.eu/blog/2023/a-look-at-screencapturekit-on-macos-sonoma/).
Consequences for the no-cert stance (P14): **no TCC grant ⇒ nothing pinned to a cdhash ⇒ unsigned
updates never orphan a screen grant, and the Sequoia nag never fires** (it exists to police apps
that *bypass* the picker). Cost: a system picker appears in the record flow — the user clicks the
display in Apple's UI instead of shroom's pre-filled panel. Unknown: whether the returned
`SCContentFilter` can be reused across recordings/launches to amortize that click (→ probe).
Quinn (Apple DTS) points developers at exactly this migration —
(a/c) [forums 760483](https://developer.apple.com/forums/thread/760483), quoting Sequoia release
notes: deprecated capture APIs must move to "ScreenCaptureKit and `SCContentSharingPicker`."

**Path B — `SCShareableContent` + TCC grant: the POC-shaped tax, minus attribution.**
Survives: System-Settings toggle + relaunch timing; **cdhash pinning — every unsigned update
orphans the grant** (POC §4, unchanged by SCK; TCC keys grants to code identity); and the
Sequoia-era re-approval nag — monthly in 15.0, "fewer dialogs for apps you use regularly" from
15.1, still present policy in 2025/26 — (c) [9to5Mac (monthly)](https://9to5mac.com/2024/08/14/macos-sequoia-screen-recording-prompt-monthly/),
(c) [9to5Mac (15.1 relaxation)](https://9to5mac.com/2024/10/07/macos-sequoia-screen-recording-popups/),
(c) [TidBITS](https://tidbits.com/2024/09/23/how-to-avoid-sequoias-repetitive-screen-recording-permissions-prompts/),
(c) [mjtsai roundup](https://mjtsai.com/blog/2024/08/08/sequoia-screen-recording-prompts-and-the-persistent-content-capture-entitlement/).
The nag *is* the POC's `ScreenCaptureApprovals.plist` bypass consent, generalized (same "bypass the
system private window picker" language) — for a real bundled app it keys by bundle id, so the POC's
every-version-re-fire pathology goes away, but the ~monthly cadence stays. The escape entitlement
(`com.apple.developer.persistent-content-capture`) is Apple-approved VNC-apps-only — unavailable to
an unsigned OSS app. (a/c)

Mic (and camera, for the bubble) TCC remains in **both** paths — but these are user-DB,
inline-grantable prompts: after an unsigned update changes the cdhash, worst case is a one-click
re-prompt at next record, not a Settings safari. (d) from TCC mechanics + POC §4's mic/screen
asymmetry, high confidence.

**Friendly display names** (§5.1's "LG monitor, not Screen 0") are deterministic now:
`NSScreen.localizedName` (10.15+) / SCK's `SCDisplay` + NSScreen bridge — the agent-era convenience
survives de-agenting. (a), high confidence.

### 4. P7 first-class: the webcam bubble (screen + cam compositing)

Three concrete ways to get the bubble **baked into the screen stream** (§5.2's v1-if-cheap shape):

1. **Apple Presenter Overlay (macOS 14+), the system does everything.** Requirements: an active
   `SCStream` **plus** an active `AVCaptureSession` on the camera; the *user* then flips it on in
   the green-camera **Video menu-bar item** — there is **no API to enable or disable it**; frames
   arrive in the SCK stream **already composited** ("small" = draggable floating bubble with
   segmentation, "large" = presenter behind/around content); app is notified via
   `stream(_:outputEffectDidStart:)`; while active, the app's own camera feed stops delivering
   normal frames — (a) [WWDC23 10136](https://developer.apple.com/videos/play/wwdc2023/10136/) /
   (c) [WWDCNotes](https://wwdcnotes.com/documentation/wwdc23-10136-whats-new-in-screencapturekit/),
   (c) [Nonstrict](https://nonstrict.eu/blog/2023/a-look-at-screencapturekit-on-macos-sonoma/),
   (a) [Apple FaceTime guide](https://support.apple.com/guide/facetime/presenter-overlay-video-conferencing-fctm6333f4bd/mac).
   **It does work in third-party recorders**: QuickRecorder ships it as its camera-overlay feature on
   14+ — (c) [QuickRecorder](https://github.com/lihaoyun6/QuickRecorder). Cost: ~zero code beyond
   running the camera session. Risk: activation is user-gesture-only (a said-line like "click the
   green camera icon and choose Overlay" — workable but not shroom-controlled), look is Apple's,
   not brandable.
2. **Floating preview window, captured for free (any macOS ≥12.3).** Show an always-on-top,
   draggable `NSWindow` with the camera preview; **display capture records whatever is on screen,
   bubble included** — zero compositing code. This is QuickRecorder's own fallback on 12/13 (c),
   and it is Loom-authentic UX (presenter sees the bubble; recording shows exactly what they see;
   drags are recorded). ~150–300 lines (window + `AVCaptureVideoPreviewLayer` + drag). Matches
   §5.1's "draggable, not resizable" precisely because *you* build the window.
3. **Manual GPU compositing.** SCK frames + `AVCaptureVideoDataOutput` frames → CoreImage/Metal
   overlay → encoder. Latest-camera-frame sampling handles clock mismatch; scale-to-256px + one
   blend per frame is negligible GPU work next to the encode. This is what the polished natives do
   (native Swift + ScreenCaptureKit with a camera overlay, or shader composition in Rust) —
   (c). ~300–600 lines + format/timing care. CPU cost: single-digit % on Apple Silicon
   (d, medium-high — shipping recorders do it realtime).

**P7 verdict: bubble in v1 = yes.** Ship path 2 as the deterministic baseline (works at any floor,
fully controlled, trivially testable), let path 1 light up when the user discovers it, defer path 3
to the v2 editor era. Confidence: high.

### 5. v2 freebies validated (§5.2 deferrals stay cheap)

- **Window capture**: `SCContentFilter(desktopIndependentWindow:)` — free. (a)
- **Region capture**: `SCStreamConfiguration.sourceRect` crop (12.3+) — free mechanically; the
  region-picking UI is the real work. (a)
- **System audio**: `capturesAudio = true` (13+) — **the whole virtual-driver headache (BlackHole
  et al.) is dead**; limit: 1–2 channels — (a/c) [OBS PR 6600](https://github.com/obsproject/obs-studio/pull/6600).
  Same-stream, same-clock as video and mic.
- **Mic**: 15+, same stream, per-device selection — the audio-tap-for-transcription requirement
  (P8/P5.5) is satisfied by teeing the mic `CMSampleBuffer`s to the transcription engine; no second
  capture, and the POC's head-transcription trick (closed segments are decodable mid-record)
  transfers unchanged. (a/d, high confidence)
- Real-world SCK dings worth knowing (all OBS-reported, none disqualifying): 2-channel audio max
  (c), a Sonoma-era capture-performance regression (FB13691162) —
  (c) [obs#10636](https://github.com/obsproject/obs-studio/issues/10636); historic
  fullscreen-toggle lag tied to color profiles — (c) [obs#7651](https://github.com/obsproject/obs-studio/issues/7651);
  a fullscreen-PowerPoint display-capture miss in the early SCK port —
  (c) [obs#6893](https://github.com/obsproject/obs-studio/issues/6893). Expect this *class* of bug;
  budget a wedge-classifier like the POC's.

### 6. Bindings: Rust / Node vs a sealed Swift helper

- **Rust — [screencapturekit-rs](https://github.com/doom-fish/screencapturekit-rs)** (also
  mirrored/renamed svtlabs): v8.0.1 released **2026-07-18**, 46 releases, ~880 k downloads, covers
  SCStream, picker, mic (15+), `SCRecordingOutput`, presenter-overlay config, zero-copy IOSurface,
  async — (c), verified current. **What it does not give you: the AVAssetWriter/HLS authoring
  side.** For that, Rust needs [cidre](https://github.com/yury/cidre) (broad AV/VT bindings,
  community-regarded as stable) or objc2-generated crates — workable, but now the hard 20 % of the
  recorder (writer profiles, segment delegate, passthrough muxing, the Nonstrict landmines) is
  written against unofficial bindings with API churn (v8 in July, v7 earlier in 2026).
  **Existence proof that the Rust road reaches production:** open-source Tauri + Rust recorders
  ship today (SCK on macOS, DXGI on Windows) — (c).
- **Node/Electron**: no first-class SCK binding exists; the ecosystem is Swift-sidecar wrappers —
  [aperture-node](https://github.com/wulkano/aperture-node) (Kap's recorder: literally a Swift CLI
  driven from Node), [node-mac-recorder](https://github.com/aslanon/node-mac-recorder),
  [ElectronCaptureKit](https://github.com/divyanshunegi/ElectronCaptureKit) (both thin,
  low-adoption). Electron's built-in `desktopCapturer` funnels through Chromium/MediaRecorder →
  WebM, wrong pipeline for HLS-native output. **The Node answer collapses to "ship a Swift helper
  anyway."** (c/d, high confidence)
- **Swift helper**: with the rethink's "contained Swift core OK," the comparison is *bindings vs
  first-party APIs with Apple's own docs, samples ([Nonstrict example](https://github.com/nonstrict-hq/ScreenCaptureKit-Recording-example)),
  and WWDC support*. The Swift helper wins on risk: every landmine found in this research has its
  fix documented in Swift terms, and QuickRecorder/ScreenKite/Screen Studio prove the whole
  recorder is buildable as boring Swift.

**Honest Swift-surface estimate for full v1** (capture + mic + HLS writer + playlist author +
per-take pause/resume + MP4 leg + camera bubble window + picker/TCC handling + IPC protocol +
error/wedge classification): **~2,500–4,500 lines** plus tests. Calibration: the POC shim did
*none* of the capture/encode and was already 1,321 lines (b — measured in
`archived-poc/scripts/shim/macos/Sources/`); Nonstrict's minimal record-to-disk example is a few
hundred lines with no audio/pause/HLS. "A few hundred lines" is off by 5–10×; it is still a small,
sealed, seam-testable component that changes rarely. (d, high confidence)

### R1 recommendation

**Adopt SCK + AVAssetWriter in a sealed Swift helper process as the v2 capture core; floor
macOS 15; bubble in v1 via the floating-preview-window path (Presenter Overlay as a freebie on
top); one-encode/two-mux via VTCompressionSession passthrough with finalize-remux as fallback.**
Present both permission postures to the stack session as a *product* choice, with a leaning toward
the **picker path** (Path A): it is the only design under which the no-cert stance costs nothing
recurring — no TCC toggle, no cdhash orphaning at update, no monthly nag — and its price (one
system click per recording, possibly amortizable) is exactly measurable by a 1-hour probe.

**Rejected alternatives:**
- **Keep the ffmpeg 2-input design (POC §2/§3)** — carries the avfoundation warmup, the fifo/buffer
  topology, the deprecated-API direction (`AVCaptureScreenInput` is obsoleted as of macOS 15, and
  Apple's release notes explicitly push capture apps to SCK), and all of §4's tax. Only defensible
  if v2 had refused Swift entirely; the rethink removed that constraint.
- **Rust bindings as the core (screencapturekit-rs + cidre)** — real and current, but moves the
  hardest, least-documented 20 % (writer profiles, segment delegate, passthrough) onto unofficial,
  fast-churning bindings, for zero product gain over a Swift helper the shell can spawn anyway.
  Reconsider only if the shell decision lands on Tauri *and* the team wants one language end-to-end.
- **Node/Electron native bindings** — they are Swift sidecars with extra steps; nothing mature.
- **`SCRecordingOutput` as the MP4 leg** — 15+-only is fine, but it's a redundant second encode
  with a reported mic-corruption bug and undocumented crash semantics; regenerating the MP4 from
  segments is strictly safer.
- **GStreamer / libobs** — re-rejected per POC §3 (deprecated screen source; heavy/opaque).

**Cheap probes for the unknowns research can't close** (each ≲ a day, most ≲ 2 h):
1. ~200-line Swift spike: SCK → VTCompressionSession → HLS-profile writer + passthrough MP4 writer
   simultaneously; verify segments play + MP4 is faststart. (Closes dual-output definitively.)
2. Picker ergonomics: can a stored `SCContentFilter` from `SCContentSharingPicker` start a *second*
   stream later without re-picking (same run; across runs)? Decides how much UX the no-TCC path
   really costs.
3. 2-hour synthetic run of the spike; watch RSS + playlist behavior (closes P6 writer-growth).
4. On Tahoe: confirm Presenter Overlay activates for a plain recorder (QuickRecorder claim, own
   machine) and that the small-overlay bubble lands in the recorded stream.

---

## R9 — Renditions: numbers first

**TL;DR verdict**

- **(b) Loom today (probed on three public videos): H.264 Main + AAC-LC mono 44.1 kHz, 4 s TS
  segments, VOD, audio as a shared rendition group, ladder = 720p@1.5 Mbps + 1080p@3.2 Mbps
  + native(≥1440p)@5.5 Mbps, capped at source resolution** (a 1080p-source video ships just two
  rungs). "Loom-grade" therefore means: floor rung 1.5 Mbps, not sub-Mbps mobile rungs.
- **Dual realtime encode is a non-issue on base Apple Silicon**: one M1-class Media Engine encodes
  1080p H.264 at several hundred fps; two 30 fps sessions consume a small fraction of one engine.
  CPU is not the reason to defer renditions. Confidence: high (community benchmarks + OBS practice).
- **The real cost of a second rung is upload, not storage-dollars**: +~40 % bytes (720p@1.5 on a
  3.5 Mbps main) ≈ +11 MB/min. Storage at R2 prices is noise ($0.02→$0.03 per recorded hour-month);
  but the offline/slow-uplink queue (P6 unbounded, P9 page-first) drains ~40 % slower.
- **ABR genuinely rescues bad wifi only if a lower rung exists**: hls.js has documented
  emergency-down-switching and size-capping; with a single rendition there is no rescue, only
  stalling. A 2-rung ladder (1.5 + 3.5) is exactly Loom's smoothness envelope for 1080p sources.
- **Verdict: single-rendition v1, but pay the upgrade path now** — master playlist from day one,
  rendition-scoped object keys, fMP4 everywhere. Then the v2 flip is a second encoder chain +
  one playlist line, with zero player or URL changes.

### 1. What Loom actually ships (empirical, 2026-07-19)

Method (b): took public Loom IDs embedded in Loom/Atlassian's own support pages, fetched
`loom.com/share/<id>`, extracted the signed CloudFront playlist URL, fetched master + media
playlists + segments, ffprobe'd the segments.

Master playlist (video `41f6aad1…`, native 3332×2034 source):

```
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="audio",DEFAULT="YES",URI="mediaplaylist-audio.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=1178x720,AUDIO="audio"
#EXT-X-STREAM-INF:BANDWIDTH=3200000,RESOLUTION=1768x1080,AUDIO="audio"
#EXT-X-STREAM-INF:BANDWIDTH=5500000,RESOLUTION=3332x2034,AUDIO="audio"
```

- Second video (2088×1440 source): same 1.5/3.2/5.5 rungs at 720/1080/native.
- Third video (1728×1080 source): **only two rungs** (1.5@720p, 3.2@1080p) → ladder is capped at
  source; rung bitrates are fixed constants, resolutions scale to the source's aspect.
- Segments: MPEG-TS (`EXT-X-VERSION:3`), 4 s target duration, `PLAYLIST-TYPE:VOD`.
- Codecs (ffprobe, b): video `h264` **Main** profile 30 fps; audio `aac` LC, **44.1 kHz mono**,
  ~160 kbps, in a **separate audio media playlist shared by all variants** (audio bytes stored
  once, not per-rung).
- Measured segment bitrate on the 1500 rung: 796 KB/4 s ≈ 1.59 Mbps — the BANDWIDTH numbers are
  honest.
- Loom's separately-fetched download MP4 exists alongside HLS (`transcoded/<id>.mp4`) — same
  HLS+MP4 dual-artifact shape as shroom's §5.5. (b)

Notes for shroom: no CODECS attribute (sloppy but tolerated by players); shroom should emit proper
`CODECS`/`RESOLUTION` and stay fMP4 (POC segments already are) — fMP4 needs `EXT-X-VERSION:6+`,
which Safari-native and hls.js both handle.

### 2. VideoToolbox dual-encode on base Apple Silicon

- Base M1/M2/M3 have one Media Engine encode block; M1 Max-class parts add a second — (c)
  [HandBrake discussion #3932](https://github.com/HandBrake/HandBrake/discussions/3932).
- Throughput reports for hardware H.264 1080p on base M1: ~**300–1000 fps** depending on filters
  (~320–350 fps typical for unfiltered 1080p; up to ~1000 fps reported) — (c)
  [MacRumors HandBrake threads](https://forums.macrumors.com/threads/someone-please-try-this-h-265-encoding-benchmark-in-handbrake-on-their-m1-mac.2277014/),
  (c) [Apple forums 678210](https://developer.apple.com/forums/thread/678210). Two realtime
  sessions at 30 fps = **60 fps aggregate ≈ 6–20 % of one engine.**
- Multiple concurrent VT sessions are routine in practice: every OBS-on-Mac "stream + record"
  setup runs two; utilities report *extra* throughput from parallel sessions. (c/d)
- The second rung also needs a downscale (1080p→720p per frame): GPU scaler work, negligible next
  to encode; the POC already always scales (avfoundation captures native), so this is not new cost
  in kind. (d)

**Conclusion: a 1080p+720p dual realtime encode is comfortably within base-M1 budget; even
+MP4-leg-as-third-encode would fit. Encoding capacity should not drive the renditions decision.**
Confidence: high. (10-minute probe if wanted: run the POC ffmpeg recipe with
`split` + two `h264_videotoolbox` outputs and watch Activity Monitor.)

### 3. Storage / upload delta, with shroom's numbers

Baseline (POC §2): normal preset ≈ 1080p @ 3.5 Mbps + 128 kbps audio ≈ **27 MB/min ≈ 1.6 GB/h**.

| Ladder | Added bytes | Total vs single | Notes |
|---|---|---|---|
| + 720p @ 1.5 Mbps (Loom's floor rung) | +11.3 MB/min | **≈ +40 %** | audio shared via rendition group (Loom-style), else +45 % |
| + 540p @ 0.9 Mbps | +6.8 MB/min | ≈ +25 % | below Loom's floor; only worth it for genuinely bad links |
| + both | +18 MB/min | ≈ +65 % | overkill for a talking-head/product-demo tool |

- R2 storage at $0.015/GB-month — (a) [R2 pricing](https://developers.cloudflare.com/r2/pricing/):
  single-rendition hour ≈ $0.024/mo; +720p rung ≈ $0.034/mo. **The §2 cost thesis is untouched by
  renditions** — storage dollars are noise either way.
- The bill that isn't noise: **upload volume** on the P6/P9 path. A 10-minute recording goes from
  ~270 MB to ~380 MB; on a 10 Mbps uplink that's ~3.6 min → ~5.1 min until "content arrived."
  Every offline-queue and flaky-wifi scenario worsens by the same 40 %. This is the honest argument
  for deferring the second rung, and it's a product argument, not a technical one. (d, numbers
  arithmetic)

### 4. Does ABR actually rescue bad wifi with a 2-rung ladder?

- hls.js: documented ABR with EWMA bandwidth estimation, **"emergency switch down"** on stall risk,
  `capLevelToPlayerSize` (default false) to cap the chosen rung at the video element's size, and
  `startLevel` (default = first level listed in the master; `-1` = bandwidth-estimate start,
  default estimate 500 kbps) — (a-for-the-library)
  [hls.js API docs](https://github.com/video-dev/hls.js/blob/master/docs/API.md). Practical
  settings for shroom: list the 720p rung first (fast start), `capLevelToPlayerSize: true` (the
  player page rarely displays >1080p), auto-ABR does the rest.
- Safari native HLS (macOS/iOS `<video>`) runs its own ABR off the same master playlist — the
  2-rung ladder benefits both player paths with zero player code. (a/d)
- With a **single rendition, no player can rescue anything**: a 3.5 Mbps stream on a 2 Mbps link
  buffers, full stop. So §5.2's "smooth on bad wifi" is *only* purchasable via a lower rung —
  ABR is the mechanism, and 1.5 Mbps@720p is Loom's own answer for what that rung should be. (d,
  high confidence)

### 5. Verdict: what v1 ships and what it must pre-pay

**Recommendation: single-rendition v1 (1080p@3.5 + audio), with the adaptive upgrade designed in
and priced at near-zero:**

1. **Master playlist from day one**, even with one variant: `master.m3u8` →
   `r1080/stream.m3u8`, correct `BANDWIDTH`/`RESOLUTION`/`CODECS`. Players and unfurl targets
   always reference `master.m3u8`; adding `r720/` later changes no URL and no player code.
2. **Rendition-scoped object keys** (`<id>/r1080/seg_00001.m4s`, `<id>/r1080/init.mp4`): the v2
   uploader writes a sibling prefix, idempotent-PUT logic untouched.
3. Audio: keep it muxed into the single variant for v1 simplicity, but note the Loom pattern
   (separate audio group) is the storage-optimal form if/when rung 2 lands; fMP4 supports it
   cleanly. Decide at flip time; either is additive.
4. Old videos simply stay single-rung (their master lists one variant) — no re-transcode
   obligation; the tombstone/dynamic-page work (R4) is unaffected.
5. The v2 flip = one more VTCompressionSession + writer chain in the Swift helper (R1 §1 pattern
   scales to N rungs by construction) + 40 % more upload. Encode capacity (§2 above) and player
   behavior (§4) are already proven.

**Rejected alternatives:**
- **Two rungs in v1** — encode-wise free, but +40 % upload directly degrades the *core* promise
  (stop → link → content-soon) on exactly the bad networks it's meant to help, and doubles the
  uploader/finalize surface in the first release. The rung helps the *viewer* on bad wifi; the
  upload tax hits the *sender* on bad wifi — v1 should optimize the sender.
- **Single flat playlist with no master (POC layout as-is)** — saves one file now, forces a
  URL/layout migration and player churn at the flip; the master costs ~10 lines. This is the one
  place the POC layout should *not* be carried forward verbatim.
- **HEVC (or AV1) ladder** — better bits/quality but patchier universal playback (hls.js/older
  Chrome/Edge cases) vs H.264's everywhere-ness; Loom itself still ships H.264 Main. Revisit only
  if storage math ever starts to matter (it doesn't at these volumes).
- **Sub-720p rescue rung (540p/360p)** — below Loom's own floor; screen text becomes illegible,
  which defeats the product (demos/PRs are text). If a viewer's link can't do 1.5 Mbps, "let it
  buffer" is more honest than unreadable video.

**Cheap probes:** (1) the 10-min dual-`h264_videotoolbox` tee run (closes encode cost with a
number); (2) throttle a browser (DevTools 1.5–3 Mbps profiles) against a POC recording re-served
with a hand-built 2-rung master to watch hls.js's switch behavior end-to-end — an afternoon, and it
would also validate `capLevelToPlayerSize` defaults for the player page.

---

## R2 — Desktop shell bake-off under shroom's exact constraints

> Researched 2026-07-19. Scope per the rethink's R2 question and decisions: shells are judged
> as **tray-first UI hosts over a sealed Swift capture helper** (Swift-tolerance decision), in
> **unsigned/ad-hoc mode only** (P14: the $0 line held), with Windows/Linux as **door-open, not
> launch-gating** (P15), and idle + during-capture footprint mattering because recordings are
> unbounded and the tray app lives forever (P6). Claim labels: **(a)** verified from official
> docs/first-party source, **(b)** community-reported, **(c)** inference. R3 (distribution/Gatekeeper)
> overlaps on updater mechanics; sources are stated so contradictions are checkable.

### TL;DR verdict

- **Tauri v2 is the recommendation** — the only candidate that simultaneously: has first-party
  tray + no-dock support, a first-party auto-updater whose trust chain is its own minisign keypair
  (no Apple cert anywhere in the loop — the only *framework* updater that is fully alive in
  unsigned mode), a near-zero idle footprint for a tray-only app, an OSS-trust posture (audited,
  foundation-governed, system webview instead of a shipped Chromium), and a real Rust
  capture-crate ecosystem behind the Windows/Linux door. **Open-source recorders already ship
  on Tauri v2 today.**
- **Electron is the strong runner-up, killed mainly by P14**: its built-in updater is *documented
  as requiring Apple code signing on macOS* — in the unsigned world you hand-roll updates. Add
  ~200 MB installed / ~100 MB per full update and Chromium-security-churn maintenance, against
  principles 4 (no bloat) and a small maintenance budget. Ironically it has the best cross-platform
  capture story (Chromium's own stack; Loom itself is Electron + native capture layers).
- **Pure Swift/SwiftUI is the best macOS citizen and the smallest** (Sparkle is the most
  battle-tested unsigned-friendly updater), but it makes the *entire app* the Swift surface the
  rethink capped at "a small, sealed helper," and closes the cross-platform door completely.
- **Flutter desktop and Wails are rejected** — Flutter: no first-party tray/updater, macOS
  multi-window still unlanded, zero capture ecosystem, unknown language; Wails: v2 has no tray at
  all, v3 (which does) is still alpha as of July 2026.
- **One shared, shell-independent alarm to probe before anything is final**: a 2026 public bug
  report claims macOS Sequoia rejects ScreenCaptureKit for ad-hoc-signed apps without a Team ID. This
  partially contradicts the POC's own empirical record (ad-hoc .app + Screen Recording grant
  *worked*, on the legacy capture path) and is fully explainable by POC §4 mechanics
  (cdhash-rotation + grant-active-only-on-next-launch) — but if the strong reading is true, it
  breaks the $0 stance for *every* shell equally. **The single highest-value 30-minute probe in
  this whole research round.** (Details in §"The ad-hoc + SCK question".)
- The POC §8 overlay/countdown recipe stays implementable everywhere **because it can live in the
  Swift helper itself** (Overlay.swift reuse) — in-shell overlay capability is a tiebreaker, not
  a gate.

### Comparison matrix

| Constraint | Tauri v2 | Electron | Flutter desktop | Swift/SwiftUI | Wails |
|---|---|---|---|---|---|
| Tray-first, no dock (macOS) | ✅ first-party TrayIcon + accessory policy; NSPanel plugin ecosystem (a) | ✅ most mature Tray API + `app.dock.hide()`/LSUIElement; huge menubar-app ecosystem (a) | ⚠️ community `tray_manager` only (a: package exists; no first-party) | ✅ native `MenuBarExtra`/`NSStatusItem` (a) | ❌ v2: none; ✅ v3: built-in but alpha (a) |
| Idle RAM, tray-only | ~5–20 MB with no webview alive; ~40–90 MB with hidden window (c/b) | ~80–150 MB (main+GPU procs even windowless) (b/c) | ~40–80 MB engine always resident (b, stale 2021 data) | ~15–40 MB (c) | ~like Tauri (c) |
| Bundle size | ~3–12 MB (b: 8.6 MiB measured) | ~180–250 MB installed (b: 244 MiB measured) | ~25–45 MB (b, 2021) | a few MB (c) | ~10–20 MB (b) |
| Auto-update, **unsigned/ad-hoc mode** | ✅ first-party updater; minisign keypair only, no Apple cert in the chain (a) | ❌ built-in `autoUpdater` requires signed app on macOS (a); hand-rolled replace only | ⚠️ `auto_updater` package wraps Sparkle → inherits Sparkle's posture (a/b) | ✅ Sparkle 2: EdDSA-signed updates, explicitly functional for ad-hoc/unsigned apps (a/b) | ❌ none in v2; v3 TBD |
| TCC grant chain (bundle-launched + Swift helper child) | ✅ expected clean; ⚠️ one unresolved sidecar-Accessibility issue to probe (b) | ✅ best-evidenced (every Electron recorder ships this shape) (b) | ✅/⚠️ same mechanics, no ecosystem evidence (c) | ✅ trivially (a) | (c) untested |
| POC §8 overlay/countdown | ✅ via Swift helper; in-shell possible via `objc2` NSWindow surgery (c) | ✅ in-shell documented (`screen-saver` level + `visibleOnFullScreen`) (a/b) | ⚠️ macOS multi-window unlanded → helper-only (a) | ✅ verbatim Overlay.swift (a: POC) | ⚠️ helper-only (c) |
| Windows/Linux door (capture) | ✅ Rust crates: scap (SCK/WGC/PipeWire), windows-capture; Tauri recorders ship Windows (a/b) | ✅ Chromium stack: WGC, PipeWire-portal, SCK + CoreAudio-tap (a) | ❌ nothing; 3 native cores via platform channels (c) | ❌ closed | ❌ nothing real (c) |
| CI testability leverage | ✅ Rust cores `cargo test` on hosted runners (a: windows-capture does exactly this) | ✅ Chromium fake-device/headless modes (b) | ⚠️ UI only | macOS only | ⚠️ |
| Small-budget maintenance | Rust learning curve, then low churn; system webview = OS patches it (c) | Chromium release treadmill is yours to ship (~8-week majors) (c) | Flutter churn + Dart unknown (c) | lowest churn, but all-Swift (c) | alpha churn (a) |
| OSS-trust fit | ✅ audited (NLNet-funded + CrabNebula), Commons Conservancy governance, no shipped browser (a) | ⚠️ you distribute a ~100 MB Chromium blob (c) | ✅ ok | ✅ readable source, tiny (a) | ✅ ok |
| Builder envelope (§10) | UI in TS ✅; core glue in Rust (new, but "will adopt on merit") | all TS ✅ | Dart ❌ unknown | all Swift ⚠️ (envelope caps Swift at sealed helper) | Go ✅ but alpha |

### The unknowns, answered

#### 1. TCC attribution per shell — do the POC §4 pains vanish?

**Mostly yes, for every shell, for the same reason — and the reason is shell-independent.** The
POC's disclaim-re-exec and grant-inheritance agony existed because the shim was launched from a
terminal process tree (agent → shell → shim), so TCC attributed prompts/grants to
Terminal/Claude (POC-LEARNINGS §4, empirical ground truth). A **normally-launched, bundle-identified
.app** is its own responsible process; children it spawns (the Swift capture helper, ffmpeg)
inherit through the responsibility boundary. Evidence this works in production shape: the entire
Electron screen-recorder ecosystem (Loom, Slack, Zoom huddles) ships exactly
app-spawns-native-capture-children, and Electron exposes first-party macOS screen-permission
plumbing (`desktopCapturer`, `NSAudioCaptureUsageDescription` guidance, macOS system-picker
support) — (a) [https://www.electronjs.org/docs/latest/api/desktop-capturer]. Confidence: high.

Per-shell notes:
- **Tauri**: sidecars ("external binaries") are bundled into `Contents/MacOS/` and spawned as
  children — the right shape (a) [https://v2.tauri.app/develop/sidecar/]. One caution flag:
  [tauri#8329](https://github.com/tauri-apps/tauri/issues/8329) reports a Swift sidecar failing to
  hold **Accessibility** rights in built apps, with mixed community resolutions ("code signing
  solved it for me" vs "signing did not solve it") (b, unresolved). Accessibility attributes to
  the *requesting binary* differently than Screen Recording attributes to the *responsible
  process*, so this may not transfer to shroom's permissions at all — but it is exactly the class
  of wall the POC taught us to probe, not assume. shroom v1 needs Screen Recording + mic only,
  not Accessibility. A community plugin exists for permission UX
  ([tauri-plugin-macos-permissions](https://crates.io/crates/tauri-plugin-macos-permissions)) (a: exists).
- **Electron**: best evidence base; no known systemic sidecar-TCC issues; note Electron ≥v39 moved
  desktop-audio capture to Apple's CoreAudio Tap API by default (a: Electron docs via
  desktopCapturer page) — irrelevant to v1 (system audio is v2) but shows active macOS capture
  maintenance.
- **Flutter**: same mechanics should hold (bundle-launched app + child helper) but there is no
  meaningful ecosystem evidence of Flutter-hosted screen recorders (c).
- **Swift**: the app *is* the principal; XPC services embedded in the bundle roll up to the host
  app for TCC purposes (c, standard platform behavior).
- **What does NOT vanish for anyone** (POC §4, ground truth; R3's beat, restated for coherence):
  ad-hoc signing pins the Screen Recording grant to the **cdhash**, so **every update orphans the
  grant** regardless of shell; the Sequoia bypass-picker consent and the screen-grant
  only-active-next-launch timing are OS behaviors, also shell-independent. The POC's mitigations
  (self-`tccutil reset`, `--prime-capture`, prep-before-tray) transfer to any shell because they
  live in the Swift helper.

#### The ad-hoc + SCK question (shared flag with R1/R3 — probe before committing)

A public bug report (opened 2026-04-09) claims:
`CGPreflightScreenCaptureAccess()` **always returns false for ad-hoc signed or unsigned binaries
on Sequoia**, even for a `.app` with stable bundle id, after `tccutil reset` + fresh prompt +
user approval — concluding Sequoia requires a Team-ID'd Developer ID cert for ScreenCaptureKit
(b). A 2026 blog repeats the claim (b, possibly derivative)
[https://www.screenify.studio/blog/2026-04-23-macos-screen-recording-permissions].

**Treat this as unconfirmed and probably misdiagnosed, but existential if true.** Two reasons for
skepticism, both from this repo's own empirical record (POC-LEARNINGS §4): (1) the POC's ad-hoc
signed `.app` *did* hold a working Screen Recording grant on Darwin 25 — on the
AVCaptureScreenInput/avfoundation path, not SCK; (2) the report's observed sequence (approve prompt →
preflight still false *in the same process run*) is **exactly** the POC's documented
"screen grant takes effect only on the next process launch" behavior, and their dev-loop rebuilds
rotate the cdhash every time, which produces precisely "permission granted in System Settings but
always denied." If their diagnosis is right anyway (SCK specifically gates on Team ID), then the
$0-no-certs line collapses **for every shell equally** and R3's compile-on-device / self-signed
identity tricks become load-bearing.
**Probe (30 min, do first):** 20-line Swift SCK capture CLI → wrap in ad-hoc `.app` with stable
bundle id → launch via Finder/`open` → grant → **quit and relaunch** → capture. Repeat after one
rebuild (expect orphaned grant; re-toggle) — this cleanly separates "SCK needs Team ID" from
"cdhash rotated / same-process timing."

#### 2. Tray/menu-bar quality per shell; §8 overlay implementable?

- **Architectural note that defuses most of this**: the countdown overlay, and any
  over-fullscreen chrome, can be rendered by the **Swift helper** using the POC's proven
  Overlay.swift recipe (CGShieldingWindowLevel, canJoinAllSpaces + fullScreenAuxiliary,
  orderFrontRegardless, accessory app) — the shell then only needs tray + config panel + settings.
  This keeps §8 implementable under **every** shell (a: POC ground truth).
- **Tauri v2**: first-party system tray (a)
  [https://v2.tauri.app/learn/system-tray/]; tray-only/no-dock via activation policy / LSUIElement
  is standard practice with real published guides and examples (b)
  [https://dev.to/hiyoyok/building-a-menubar-app-with-tauri-v2-what-nobody-tells-you-9a2],
  [https://github.com/ahkohd/tauri-macos-menubar-app-example]; NSPanel-grade popover polish via
  [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) (a: exists, community-maintained). For
  in-shell overlays Tauri exposes the raw `NSWindow`, so the POC's window-level surgery is a
  small `objc2` unsafe block (c, high confidence — the nspanel plugin does exactly this class of
  thing).
- **Electron**: the most mature tray API of any shell, plus documented always-on-top levels
  (`'screen-saver'`) and `setVisibleOnAllWorkspaces(true, {visibleOnFullScreen: true})` — the
  known recipe for over-fullscreen overlays (a)
  [https://www.electronjs.org/docs/latest/api/browser-window], with community-reported focus
  quirks (b) [https://github.com/electron/electron/issues/36364].
- **Flutter**: tray only via community `tray_manager` (a: package)
  [https://pub.dev/packages/tray_manager]; **multi-window has landed on Windows but is still
  unlanded on macOS as of 2026** (a/b)
  [https://ubuntu.com/blog/multiple-window-flutter-desktop] — a tray app that needs a config
  panel *and* an overlay *and* a picker is fighting the framework's window model on exactly our
  launch platform.
- **Swift**: native `MenuBarExtra`; Overlay.swift verbatim (a).
- **Wails**: v2 has **no tray support at all** ("System tray will not be supported in v2", (a/b)
  [https://github.com/wailsapp/wails/discussions/4514]); v3 has first-party systray but is
  **still alpha** (v3.0.0-alpha2.114 on 2026-07-05) (a)
  [https://github.com/wailsapp/wails/releases], [https://v3.wails.io/features/menus/systray/].

#### 3. Bundle sizes, idle RAM, auto-update in unsigned mode

**Sizes/RAM** (single-run community benchmarks — directionally reliable, precision low):
- Tauri 8.6 MiB bundle vs Electron 244 MiB; ~172 MB vs ~409 MB RAM at six open windows, macOS,
  2025-04 (b) [https://www.gethopp.app/blog/tauri-vs-electron]. Aggregate 2025–26 write-ups put
  Tauri idle at ~30–50 MB with a window vs Electron ~150–300 MB (b).
- **Tray-first changes the picture**: a Tauri app can run with *no webview instantiated* —
  idle is then a bare Rust process + tray, plausibly ~5–20 MB (c — cheap probe below). Electron
  keeps main (Node) + GPU/utility processes alive even windowless: ~80–150 MB idle (b/c).
  Flutter's engine is always resident (2021 measurements: ~38–73 MB idle, bundle 23–41 MB vs
  Electron 184 MB — stale, pre-Impeller, use as floor) (b)
  [https://getstream.io/blog/flutter-desktop-vs-electron/].
- **During capture** the shell is UI-only (capture/encode in the Swift helper + VideoToolbox), so
  per-shell deltas persist but don't compound; P6's multi-hour recordings stress the *helper*,
  not the shell. (c)
- **Update bandwidth is a real per-shell asymmetry in unsigned mode** (no delta-update service):
  Tauri/Swift full-bundle updates are ~5–15 MB; Electron full updates re-ship Chromium
  (~100 MB+). (c from the size data above)

**Auto-update × unsigned/ad-hoc (the P14-critical column; sources named for R3 cross-check):**
- **Electron: dead.** "Your application must be signed for automatic updates on macOS. This is a
  requirement of `Squirrel.Mac`." (a)
  [https://www.electronjs.org/docs/latest/api/auto-updater]; electron-builder's updater docs
  carry the same macOS signing requirement (a)
  [https://www.electron.build/docs/features/auto-update/]. Unsigned Electron means a hand-rolled
  download-and-replace updater.
- **Tauri: alive.** The v2 updater's entire trust chain is a Tauri-generated minisign keypair
  (`tauri signer generate`); macOS updates ship as `app.tar.gz`, verified against *your* key and
  swapped in place — **no Apple certificate appears anywhere in the updater docs** (a)
  [https://v2.tauri.app/plugin/updater/]. Tauri's macOS signing docs separately recommend ad-hoc
  signing for cert-less distribution so Apple Silicon downloads aren't "damaged" (a)
  [https://v2.tauri.app/distribute/sign/macos/]. Residual risk: quarantine/App-Translocation on
  the *first* install can break self-replacement (R3's territory); self-downloaded updates carry
  no quarantine attribute (c).
- **Sparkle 2 (Swift shell; Flutter via `auto_updater`): alive.** Sparkle's security model
  supports apps *not* signed with Developer ID via mandatory EdDSA update signing (a)
  [https://sparkle-project.org/documentation/]; maintainers explicitly account for
  ad-hoc-signed apps (incl. Homebrew-distributed ones) in the 2.7.3 security-fix discussion, with
  extra safeguards rather than a ban (b)
  [https://github.com/sparkle-project/Sparkle/discussions/2764]. Flutter's
  [auto_updater](https://pub.dev/packages/auto_updater) wraps Sparkle/WinSparkle (a: package),
  community-maintained (leanflutter).
- **Wails**: no built-in updater in v2 (c/a: absent from docs).
- **Shell-independent invoice, all options**: every update rotates the ad-hoc cdhash → Screen
  Recording re-grant (POC §4). The updater frameworks differ in whether the *swap* works
  unsigned; none of them can fix the re-grant. Budget the POC's stale-grant self-reset + one-toggle
  UX as a permanent v2 feature.

#### 4. Cross-platform capture posture — "three native cores anyway"?

**Mostly yes — and the rethink already accepted that by sealing capture into a Swift helper on
macOS.** A Loom-grade core (6 s fMP4 segments live, tee'd faststart MP4, wall-clock-locked audio)
is per-OS work no shell abstracts away at that quality bar. So shells are judged as UI hosts —
**but** two of them still carry genuine door-open value:
- **Tauri/Rust**: scap — SCK on macOS, Windows.Graphics.Capture
  on Windows, PipeWire on Linux (a: repo README) — grown for a shipping recorder
  that runs natively on macOS + Windows (a); plus
  [windows-capture](https://github.com/NiiightmareXD/windows-capture) (WGC) and the
  crabgrab/xcap family. A future Windows core can be a Rust sibling of the Swift helper rather
  than a from-scratch project (c).
- **Electron**: the only shell whose *built-in* stack captures on all three OSes — Chromium's
  getDisplayMedia/desktopCapturer rides WGC on Windows, the PipeWire portal path on
  Wayland/Linux, and SCK + CoreAudio-tap on macOS (a)
  [https://www.electronjs.org/docs/latest/api/desktop-capturer], with known Wayland quirks (b)
  [https://github.com/electron/electron/issues/37524]. Encode control (exact 6 s IDR cadence,
  fMP4 HLS) would ride WebCodecs + a JS muxer, an unproven-in-this-repo path (c). Precedent:
  **Loom's own desktop app is Electron UI + per-platform native recording layers + bundled ffmpeg
  doing client-side HLS** (a, first-party 2019 — stale but structural)
  [https://www.atlassian.com/blog/loom/behind-the-scenes-building-loom-for-desktop].
- **Flutter / Swift / Wails**: no credible capture story beyond writing the native cores and
  bridging (c). For Swift the door is simply closed (a separate Windows app someday).

#### 5. Testability without hardware (§7.5) — what can CI actually exercise?

**A happy asymmetry: the platforms Bagrat can't test (Windows/Linux) are the ones hosted CI can
genuinely exercise; macOS — the platform he *can* test — is the awkward one in CI.**
- **Windows**: GitHub-hosted `windows-latest` runners have an interactive desktop with a virtual
  display; the windows-capture crate (WGC-based) runs plain `cargo test` on `windows-latest` in
  its own CI (a: workflow file)
  [https://github.com/NiiightmareXD/windows-capture/blob/main/.github/workflows/rust.yml] —
  whether its tests drive a full frame-producing capture session needs a 10-minute read of the
  test source (flagged below).
- **Linux**: headless PipeWire screencast is real: GNOME's mutter has a headless backend with
  virtual monitors and a `RecordVirtual` D-Bus screencast API (since GNOME 40) producing genuine
  PipeWire streams with no display hardware (b)
  [https://gist.github.com/columbarius/50239ea3c4c70df8f240aa50f88e801a],
  [https://notes.nokun.eu/post/2024-09-22-mirrorhall/]; the wlroots route (sway headless +
  [xdg-desktop-portal-wlr](https://github.com/emersion/xdg-desktop-portal-wlr)) is the
  lighter-weight equivalent. An ubuntu runner can run the *real* portal→PipeWire capture path
  end-to-end.
- **macOS**: hosted macos-13+ images have **SIP disabled** (a/b)
  [https://github.com/actions/runner-images/issues/8162], and the community recipe of
  `sudo sqlite3` inserts into the system TCC.db for `kTCCServiceScreenCapture` works on
  macOS 13/14 runners (b, with Sonoma schema caveats)
  [https://github.com/actions/runner-images/issues/9529] — so even macOS capture smoke tests are
  possible in CI, fragile but real. Ground truth stays the POC's local Tart VM harness
  (POC §9: TCC prompts need human clicks) plus Bagrat's own machine.
- **Reference point**: OBS's own GitHub CI is build/format-focused, not capture-integration
  ([https://github.com/obsproject/obs-studio/actions]) (b) — nobody big is doing hosted-runner
  capture integration at scale, which reinforces the POC's determinism-boundary lesson: keep the
  engine seam-injectable (fake frame/audio sources) so 95 % of logic tests need no capture stack
  at all, and use the above as thin per-platform smoke jobs to keep untestable-platform code
  honest.

### Per-shell notes

**Tauri v2** — Stable since late 2024; NLNet-funded external audits of v1 and v2, CrabNebula
partnership (2,870+ paid work-hours in 2024), Commons Conservancy governance (a)
[https://v2.tauri.app/security/ecosystem/],
[https://v2.tauri.app/blog/partnership-crabnebula/] — unusually good OSS-trust posture for a
framework. UI is web tech in the system WebView (WKWebView/WebView2) → TS for all product UI
(builder envelope ✅), Rust only for the thin host glue + sidecar spawning. Costs: Rust is new to
Bagrat (the envelope allows any technology that meets the tech demands — the Rust surface here
is small and mostly framework-shaped); WKWebView quirks vs Chromium exist but a tray recorder's
UI is small; the sidecar-Accessibility issue (#8329) is the one TCC cloud, probed cheaply.

**Electron** — Everything works and is documented; it is what Loom itself ships. Rejected on the
P14 invoice (built-in updater requires Apple signing on macOS → hand-rolled updater in the $0
world), the footprint (~244 MiB bundle, ~100 MB updates, always-resident multi-process idle), the
maintenance treadmill (you ship and re-ship Chromium security updates on a small budget), and
the OSS-trust optics of a 100 MB opaque runtime. If v2 ever *needed* Chromium's cross-platform
capture stack as the Windows/Linux strategy, this decision deserves a revisit — that's the one
future in which Electron wins.

**Pure Swift/SwiftUI** — Technically the cleanest macOS product: native tray, Overlay.swift
verbatim, Sparkle 2 (the most battle-tested cert-less updater), a few-MB bundle, lowest idle RAM.
Rejected because it inverts the rethink's Swift decision (a *small sealed helper* becomes *the
entire app*), closes the cross-platform door permanently (P15 wants it open), and concentrates all
maintenance in a language Bagrat has explicitly bounded. Worth naming: since the capture helper is
Swift regardless, "Swift shell" remains the escape hatch if the chosen shell ever collapses — the
helper's code survives that pivot unchanged.

**Flutter desktop** — Rejected: no first-party tray or updater (community packages of moderate
bus-factor), macOS multi-window still unlanded (a/b, Ubuntu/Canonical 2026)
[https://ubuntu.com/blog/multiple-window-flutter-desktop], zero capture ecosystem (three native
cores *and* platform-channel bridges), Dart is unknown to Bagrat, and nothing it's best at
(pixel-identical custom UI) is something a tray recorder needs.

**Wails** — The Go affinity made it worth checking, but: v2 has no system tray ("will not be
supported in v2") and no updater; v3 has first-party systray but is still alpha
(v3.0.0-alpha2.114, 2026-07-05) (a) [https://github.com/wailsapp/wails/releases]. Betting the
product's spine on an alpha shell contradicts the calm-side-project constraint. **Revisit only if
v3 ships stable before the v2 build starts.**

### Cheap empirical probes (in priority order)

1. **Ad-hoc + SCK on current macOS** (~30 min, gates everything, shared with R1/R3): Swift SCK CLI
   in an ad-hoc `.app`, Finder-launched, grant → relaunch → capture; rebuild once to watch the
   cdhash orphaning. Settles the public Team-ID claim against the POC's contrary evidence.
2. **Tauri sidecar TCC chain** (~1 evening): bundle the same Swift helper as a sidecar in an
   ad-hoc Tauri app; verify prompt naming, grant inheritance, and capture through the child.
   Directly retires the #8329 cloud for our permission set.
3. **Idle-footprint reality check** (~1 hr): scaffolded tray-only Tauri app (no window) vs
   tray-only Electron vs the POC shim, Activity Monitor over 30 min. Replaces the (b)/(c) RAM
   rows with numbers.
4. **Tauri updater in unsigned mode** (~2 hrs, feeds R3): minisign-signed update of an ad-hoc
   app: does the swap succeed, does the replaced app launch clean (no quarantine), what exactly
   breaks in TCC after.
5. **CI capture matrix spike** (~half a day): one workflow, three jobs — windows: run a
   windows-capture example for 60 frames; ubuntu: mutter-headless (or sway-headless + portal)
   PipeWire screencast to file; macos: TCC.db insert + `screencapture` smoke. Whatever passes
   becomes the permanent honesty-check for untestable-platform code.

### Recommendation

**Tauri v2 as the shell, hosting the sealed Swift capture helper as a sidecar; product UI in
TypeScript in the webview; overlay/countdown rendered by the helper (POC Overlay.swift), tray via
Tauri's first-party API; updates via Tauri's minisign updater; Windows door kept open via the Rust
capture-crate ecosystem (scap/windows-capture).**
Conditional on probes 1–2 passing (ad-hoc SCK viability is existential and shell-independent;
sidecar TCC chain is Tauri-specific). Confidence: medium-high on the shell ranking; the ranking is
robust to everything except probe 1 failing, which would reopen P14 rather than the shell choice.

**Rejected alternatives, with reasons:**
- **Electron** — updater dead in unsigned mode (the one framework updater *documented* as
  requiring Apple signing), 20–30× footprint, Chromium-shipping maintenance on a small
  budget; keep on file as the revisit-candidate if Chromium's capture stack ever becomes the
  cross-platform strategy.
- **Pure Swift/SwiftUI** — best macOS citizen, but expands the rethink's "small sealed Swift
  helper" into an all-Swift app and welds the cross-platform door shut; survives as the
  escape-hatch architecture since the helper is Swift either way.
- **Flutter desktop** — no first-party tray/updater, macOS multi-window unlanded, no capture
  ecosystem, unknown language; nothing it excels at is needed here.
- **Wails** — right language, wrong maturity: tray-first requires v3, which is alpha (July 2026);
  revisit only if v3 stabilizes before the build starts.

---

## R3 — Unsigned macOS distribution & update in 2026: the full option space, priced

> Scope per the rethink (P14): **the $0 line holds.** This is not
> "should shroom pay $99/yr" — it is "the best achievable UX inside the unsigned space,
> priced honestly," with the paid channels kept as reference columns so the cost of the
> principle stays visible. Ground truth from POC-LEARNINGS §4/§7 (cdhash pinning, the
> phantom-grant state, compile-on-device at shim scale) is treated as settled and built on,
> not re-researched. macOS in scope: Sequoia 15 → Tahoe 26 (Bagrat's own machine is
> Darwin 25.5 ≈ 26.5) → "Golden Gate" 27 (announced WWDC June 2026, ships ~Sept 2026).
>
> Claim labels: **(a)** verified from official docs · **(b)** community-reported ·
> **(c)** inference. Confidence stated on load-bearing conclusions.

### TL;DR verdict

- **Gatekeeper only ever fires on the quarantine xattr — and terminal-mediated installs
  never set it.** A browser-downloaded unsigned app costs a ~5-step "Open Anyway" dance
  (System Settings + admin password, right-click-Open is gone since Sequoia); a
  `curl`/script/build-from-source install has **zero Gatekeeper friction, legitimately**.
  For shroom's audience (§3: techies + everyone-uses-an-agent), the install channel that
  avoids the browser avoids the entire first-launch problem.
- **The Homebrew cask channel is dying for unsigned apps**: `--no-quarantine` was
  deprecated in Homebrew 5 (Q4 2025) and casks failing Gatekeeper (unsigned/unnotarized)
  are evicted from homebrew/cask on **Sept 1, 2026** — six weeks from now. Cross it off;
  a **formula-style tap** (the yabai model) remains fully viable.
- **Update frameworks:** Sparkle 2 can replace a non-Apple-signed bundle in place
  (EdDSA-only validation); Tauri's updater is Apple-signing-independent (minisign) but
  community reports are mixed on unsigned macOS; **Electron autoUpdater hard-requires a
  signed app (Squirrel.Mac) — disqualified.** In-app updates don't re-quarantine, so no
  Gatekeeper re-dance — but every update orphans a cdhash-pinned Screen-Recording grant.
  **The update problem is not Gatekeeper; it is TCC identity.**
- **There is a credible zero-dollar stable-identity path:** a **per-machine self-signed
  signing identity, re-applied locally after every update** (exactly how yabai has shipped
  Accessibility-granted unsigned builds for years). TCC's csreq then pins
  `identifier + certificate leaf` instead of a cdhash, and the grant survives updates.
  Unverified for Screen Recording specifically — one ~30-minute local experiment settles
  it (specified below). If it fails, the floor is the POC's own answer: cdhash pinning +
  `tccutil` self-reset = **one Settings toggle + relaunch per update**, forever.
- **Compile-on-device scales to a Swift-core app (CLT-only, minutes) but not to
  Tauri/Electron scale for persona C** (multi-GB toolchains, 5–20 min first build,
  Node+Rust babysitting). It survives as the *option* for the trust-maximalist, not as the
  default channel.
- **P14's invoice, concrete:** the unsigned strategy below costs ~1 extra minute at
  install (script instead of drag-drop), **0–1 Settings round-trips per update**
  (0 if the self-signed experiment passes, 1 if not), plus Apple's monthly
  screen-capture re-confirmation nag that even $99/yr does not remove. What $99/yr would
  buy: no first-launch dialog for browser downloads, guaranteed-stable TCC identity, and
  entry to the (now signed-only) Homebrew cask ecosystem. That's the whole invoice.

---

### The matrix: channel × friction × update × TCC

Steps counted for a **non-techie-with-agent** user unless noted. "TCC re-grant" = Screen
Recording toggle round-trip (System Settings → Privacy & Security → Screen & System Audio
Recording → toggle → relaunch app); mic re-prompt is one inline click and rides along.

| Channel | $ | First-install friction | First-launch friction | Update friction (per update) | TCC re-grant per update |
|---|---|---|---|---|---|
| **1. Direct download** (browser .dmg/.zip, ad-hoc-signed) | $0 | Drag to /Applications | **"Not Opened" dialog → Settings → Open Anyway → relaunch → confirm → admin password** (~5 steps) (a) | If re-downloaded via browser: full Open Anyway dance **again** (new quarantined bundle) (c, high) | Yes — 1 toggle + relaunch (cdhash changed) |
| **2. Direct download, truly unsigned arm64** | $0 | same | **"App is damaged" dead end — no Open Anyway offered**; only xattr rescue (b, med-high) | same | — (never launches) |
| **3. Homebrew cask** (unsigned app) | $0 | `brew install --cask` | Quarantine applied by design → same Open Anyway dance (a) | **Channel dies Sept 1 2026** (unsigned casks evicted; `--no-quarantine` deprecated) (a) | Yes — 1 toggle + relaunch |
| **4. Installer script / tap formula, prebuilt binary** (curl-fetched, locally signed) | $0 | Paste one command (or agent runs it) — ~1 min | **None. No quarantine ⇒ Gatekeeper never evaluates it** (a/c, high) | In-app updater; no quarantine, no dialogs | **None if per-machine identity re-sign works (experiment E1/E2); else 1 toggle + relaunch** |
| **5. Compile-on-device, Swift-core app** (tap formula `--HEAD`-style or setup script) | $0 | CLT install (one-time, ~1–3 GB, interactive GUI prompt) + minutes of build | None (no quarantine; product of local build) (a/c, high) | Rebuild (minutes, background-able) | Same as row 4 (local identity signs each build) |
| **6. Compile-on-device, Tauri/Electron scale** | $0 | Rust (~1.4 GB) + Node + CLT; **first build 5–20 min** (b/c) | None | Rebuild 1–10 min | Same as row 4 |
| **7. [ref] Developer ID + notarization** | **$99/yr** | Drag to /Applications | None (first-launch "downloaded from Internet" confirm only) | Sparkle/standard, silent | **None** (Team-ID-anchored csreq, stable) (a) |
| **8. [ref] Mac App Store** | **$99/yr** | App Store install | None | Silent, OS-managed | None — but sandbox + review; screen-recorder entitlement friction; contradicts "never a SaaS-ish gatekeeper" posture anyway |

**Reading the matrix:** the unsigned space bifurcates entirely on *how the bytes arrive*.
Browser-arrival (rows 1–3) pays the Gatekeeper toll on first launch **and on every
re-download**, and row 3 is being deleted by Homebrew itself. Terminal/script-arrival
(rows 4–6) pays zero Gatekeeper toll forever. The remaining tax — TCC re-grant per
update — is identical across all unsigned rows and is attacked by the identity trick, not
by the channel. Row 4 is the recommendation; row 5 is its trust-maximalist sibling.

---

### 1. Gatekeeper reality, Sequoia → Tahoe → Golden Gate

**The 2026 first-launch UX for an unsigned browser download, step by step** (Sequoia 15
and Tahoe 26; (a) [Apple: Safely open apps on your Mac](https://support.apple.com/en-us/102445),
(b) [hacks.guide walkthrough](https://wiki.hacks.guide/wiki/Open_unsigned_applications_on_macOS_Sequoia_and_newer),
[Macworld](https://www.macworld.com/article/2457844/what-to-do-when-you-cant-open-an-app-you-just-installed-in-macos-sequoia.html)):

1. Double-click → dialog: *"'App' Not Opened — Apple could not verify 'App' is free of
   malware…"*. Buttons: **Done** / **Move to Trash**. There is no Open button here.
2. Open **System Settings → Privacy & Security**, scroll to the Security section, find
   *"'App' was blocked to protect your Mac"* → click **Open Anyway**.
3. Relaunch the app (or it re-prompts) → a second dialog now offers **Open Anyway**.
4. **Authenticate as an administrator** (password/Touch ID).
5. App opens. Subsequent launches are clean *for that exact downloaded copy*.

- **Right-click → Open is gone.** Removed in Sequoia 15.0; the System Settings détour is
  the only sanctioned path. (a/b, high) — [mjtsai roundup](https://mjtsai.com/blog/2024/07/05/sequoia-removes-gatekeeper-contextual-menu-override/),
  [iDownloadBlog](https://www.idownloadblog.com/2024/08/07/apple-macos-sequoia-gatekeeper-change-install-unsigned-apps-mac/).
- **Ad-hoc signing is the floor, not a nicety.** On Apple Silicon, arm64 code with *no*
  signature at all doesn't get the Open Anyway path — it gets the **"App is damaged and
  can't be opened"** dead end (Move to Trash / Cancel; no bypass offered), rescuable only
  by `xattr -d com.apple.quarantine`. Ad-hoc-signed bundles get the recoverable
  "Not Opened" flow above. (b, med-high; consistent across
  [Tauri #8763](https://github.com/tauri-apps/tauri/issues/8763) ("ad-hoc … will get rid
  of the 'app is damaged' dialog"), [swissmacuser Tahoe report](https://swissmacuser.ch/fix-macos-tahoe-app-is-damaged-and-cant-be-opened-move-trash/),
  and the [misleading-damaged-warning case catalog](https://github.com/milisp/misleading-macos-damaged-warning/blob/main/CASES.md).)
  **Whatever shroom ships must always be at least ad-hoc signed** — the POC already does this.
- **Tahoe (26.x):** same flow, harsher mood. Community reports say Tahoe enforces
  quarantine more aggressively and that the xattr workaround occasionally needs repeating;
  claims that "26.2+ requires Apple Developer certificates" trace to SEO-grade sources and
  are **not** corroborated by Apple docs or Eclectic Light — the Open Anyway path
  demonstrably still exists on Tahoe. (b, low-med on the "26.2 stricter" specifics;
  [swissmacuser](https://swissmacuser.ch/fix-macos-tahoe-app-is-damaged-and-cant-be-opened-move-trash/),
  [oobabooga Tahoe issue](https://github.com/oobabooga/text-generation-webui/issues/7305).)
- **Multi-binary bundles multiply the pain.** Tahoe gates each quarantined Mach-O
  individually — the oobabooga project reports "hundreds of allow-clicks" for a
  Python-env-style bundle. (b) A conventional single-sealed .app pays the dance once; a
  bundle that spawns loose quarantined helper binaries (a bundled ffmpeg!) pays per
  binary. Design note for v2: every executable lives under the one bundle seal, or
  arrives without quarantine.
- **Golden Gate (macOS 27, ships ~Sept 2026):** nothing announced that removes Open
  Anyway; the observable trajectory (Sequoia killed right-click, Homebrew capitulated,
  pre-2025-signed kexts failing in beta) is monotonic tightening. Plan for the dance to
  get uglier, not prettier. (b/c; [MacRumors macOS 27 roundup](https://www.macrumors.com/roundup/macos-27/).)
- **App Translocation footnote:** a quarantined app launched from ~/Downloads runs from a
  randomized read-only mount until moved by Finder — which breaks self-update and
  path-keyed state. Only affects quarantined arrivals; another reason the no-quarantine
  channel wins. (a/b, high; [Eclectic Light quarantine explainer](https://eclecticlight.co/2021/12/11/explainer-quarantine/).)

### 2. Quarantine is the whole trigger — and it's opt-in

Gatekeeper assesses an app **only when it carries `com.apple.quarantine`**, and that xattr
is set by the *downloading application* (browsers set it; `curl`/`git` do not; installer
scripts and compilers produce unquarantined files). (a, high —
[Eclectic Light](https://eclecticlight.co/2021/12/11/explainer-quarantine/),
[Gatekeeper and notarization in Sequoia](https://eclecticlight.co/2024/08/10/gatekeeper-and-notarization-in-sequoia/).)

Consequences, all load-bearing for shroom:

- A **`curl -fsSL https://shroom.am/install | sh`-style install** (or a tap formula, or
  an agent-driven setup skill — the rethink's assumed delivery anyway) yields an app that
  **Gatekeeper never looks at**. No dialogs, no Settings trip, no admin prompt. This is
  how rustup/nix/ollama ship, it is not an exploit, and it matches the OSS-trust posture:
  the user (or their agent) consciously executes an installer whose source is readable.
- In-app updaters that download with plain URLSession/reqwest do **not** quarantine their
  payloads (quarantine for non-browser apps requires opting in via `LSFileQuarantineEnabled`),
  so **updates installed by the app itself never re-trigger Gatekeeper**. Sparkle
  additionally strips quarantine defensively (`releaseFromQuarantine`). (a/b, high;
  [Sparkle #230](https://github.com/sparkle-project/Sparkle/issues/230).)
- The inverse: every *browser-mediated* re-download of a new version is a **fresh
  quarantined bundle → the full Open Anyway dance again**. An unsigned app must never use
  "download the new DMG" as its update story. (c, high.)

### 3. Homebrew: the cask door is closing; the formula door is open

- **Casks quarantine by default, by policy** — that's the point of the cask installer;
  `--no-quarantine` was the escape hatch. (a; [Homebrew discussion #4302](https://github.com/orgs/Homebrew/discussions/4302).)
- **Homebrew 5 (Q4 2025) deprecated `--no-quarantine`**, and homebrew/cask will **drop all
  casks that fail Gatekeeper checks (unsigned/unnotarized) on September 1, 2026**. Their
  stated reasoning: Apple-Silicon-only future + "does not wish to easily provide
  circumvention to macOS security features." (a, high —
  [Homebrew/brew#20755](https://github.com/Homebrew/brew/issues/20755),
  [deprecation discussion #6537](https://github.com/orgs/Homebrew/discussions/6537),
  [Workbrew: Homebrew 5.0.0](https://workbrew.com/blog/homebrew-5-0-0),
  [HN thread](https://news.ycombinator.com/item?id=45907259).)
  An unsigned shroom **cannot ship as a homebrew/cask cask, ever**, and even a
  third-party-tap cask inherits the quarantine-with-no-opt-out behavior → Gatekeeper dance
  on every install *and every upgrade*. Cross the whole cask lane off.
- **Formulae are untouched.** A formula in a personal tap (`brew install bagrat/tap/shroom`)
  that either builds from source or installs a curl-fetched prebuilt produces
  unquarantined output. **This is exactly how yabai — an unsigned, TCC-hungry menu-bar
  tool with a large user base — has shipped for years**, including the codesign-with-your-
  own-certificate step. (b, high; [yabai wiki: Installing](https://github.com/koekeishiya/yabai/wiki/Installing-yabai-(latest-release)).)
  Precedent exists at real scale; shroom's shape (tray app, techie audience) is the same
  genus.

### 4. Update frameworks × unsigned apps

| Framework | Can it replace an ad-hoc/unsigned bundle in place? | Notes |
|---|---|---|
| **Sparkle 2** | **Yes** — update validation is *EdDSA-or-Apple-signing*; for a non-Developer-ID app, the EdDSA (ed25519) archive signature is the required and sufficient check. Apple-signing-only validation is the deprecated leg, not EdDSA-only. (a/c, med-high — [docs](https://sparkle-project.org/documentation/), [publishing](https://sparkle-project.org/documentation/publishing/), [key-rotation policy](https://sparkle-project.org/documentation/eddsa-migration/), [discussion #2394](https://github.com/sparkle-project/Sparkle/discussions/2394)) | EdDSA keys cost $0 and never touch Apple. Caveats: keep the same EdDSA key forever (rotation without Developer ID is not supported); ad-hoc host apps must not enable Hardened-Runtime library validation (the [OpenClaw packager](https://docs.openclaw.ai/platforms/mac/signing) disables HR for ad-hoc builds for exactly this reason (b)); Sparkle strips quarantine from what it installs. |
| **Tauri updater** | **Probably** — validation is minisign against your embedded pubkey, explicitly "unrelated to OS-level code signing" (a — [Tauri v2 updater](https://v2.tauri.app/plugin/updater/), [macOS signing docs](https://v2.tauri.app/distribute/sign/macos/)). | Community reports on unsigned macOS are thinner and mixed: ad-hoc quirks ([#13804](https://github.com/tauri-apps/tauri/issues/13804)), /Applications write-permission issues ([#8372](https://github.com/tauri-apps/tauri/issues/8372)), prod-build oddities ([#13878](https://github.com/tauri-apps/tauri/issues/13878)). (b, med) Treat as "works with elbow grease," verify empirically if Tauri wins R2. |
| **Electron autoUpdater** | **No.** "Your application must be signed for automatic updates on macOS. This is a requirement of Squirrel.Mac." (a, high — [Electron docs](https://www.electronjs.org/docs/latest/api/auto-updater)) Squirrel validates the incoming bundle against the running app's designated requirement — for ad-hoc that DR is a cdhash that never matches the next version. (c, high) | Disqualifies Electron's native update path for an unsigned shroom outright; an Electron choice in R2 would need a hand-rolled updater anyway. |
| **Custom updater** | **Yes, trivially** — download tar.gz (no quarantine), verify ed25519/minisign, swap bundle atomically, relaunch. ~200 lines against the POC's "mechanism deterministic" bar. (c, high) | The only option that can also run the **post-install local re-sign step** (below) as a first-class part of the update, which no off-the-shelf framework does. |

**What breaks after any successful in-place update of an ad-hoc app** (all frameworks
equally): the new bundle has a new cdhash → the TCC Screen-Recording grant is orphaned
into the POC's documented **phantom state** (toggle reads ON, capture denied) → user needs
the self-reset + one toggle + relaunch (POC §4, ground truth). Mic re-prompts inline (one
click). Gatekeeper does **not** re-fire (no quarantine). So: **the update problem for an
unsigned app is purely a TCC-identity problem** — which is the next section.

### 5. TCC identity: escaping the cdhash without Apple

**Settled ground truth (POC §4, Darwin 25.x):** ad-hoc signing gives a stable-per-build
cdhash; TCC pins the Screen-Recording grant to it; every rebuild orphans the grant into
the phantom-ON state; `tccutil reset ScreenCapture am.shroom` makes re-granting one clean
toggle. Mic/screen asymmetry: mic inline, screen needs Settings + takes effect next launch.
Also (b, med, multiple 2025–26 reports): **Tahoe requires a real .app bundle** for Screen
Recording to stick at all — bare executables no longer even appear in the pane
([Screenify guide](https://www.screenify.studio/blog/2026-04-23-macos-screen-recording-permissions)) —
which shroom's bundle-shipping approach already satisfies.

**How TCC decides "same app":** the grant row stores a `csreq` — the app's **designated
requirement** at grant time — and re-validates the running code against it. For ad-hoc
signatures the default DR is `cdhash H"…"` (changes every build). For certificate-signed
code the default DR is `identifier "am.shroom" and certificate leaf = H"<cert hash>"` —
**stable across builds as long as the same certificate signs them.** (a/b, high —
[HackTricks TCC](https://hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html),
POC §4's own csreq inspection recipe.)

Three zero-dollar identity candidates, best first:

#### 5a. Per-machine self-signed identity + re-sign on every update (the yabai model) — RECOMMENDED, pending E1

At install time, shroom's installer creates a **local self-signed code-signing
certificate** in the user's keychain (one-time, automatable via `security`; admin once),
and signs the freshly-installed app with it. **Every subsequent update — downloaded or
locally built — is re-signed with the same local cert before first launch.** The DR (and
thus TCC's csreq) is then identifier+leaf-hash and identical across versions → the
Screen-Recording grant survives updates. The cert never leaves the machine, so the
"would another machine trust it" question never arises.

- Precedent at scale: **yabai** instructs every user to create exactly such a cert and
  re-codesign after every `brew upgrade`; Accessibility grants survive. (b, high —
  [yabai wiki](https://github.com/koekeishiya/yabai/wiki/Installing-yabai-(latest-release)),
  [dev-workflow writeup](https://evoleinik.com/posts/macos-dev-signing-preserve-permissions/).)
  Note yabai's *duplicate-entry* annoyance ([#1968](https://github.com/koekeishiya/yabai/issues/1968))
  is a bare-binary-at-versioned-path artifact — a bundle-id'd .app at a stable
  /Applications path doesn't hit it. (c, med-high)
- Known counter-claim to retire empirically: a public bug report asserts Sequoia's
  SCK/`CGPreflightScreenCaptureAccess` "silently rejects ad-hoc signed apps with no Team
  ID". (b, low-med) This
  contradicts the POC's own working ad-hoc grants on Darwin 25 and reads exactly like the
  phantom-orphaned-grant misdiagnosis (dev loop → new cdhash every build → "permission
  ignored"). (c) But *if* SCK's preflight specifically demands an Apple-chained Team ID,
  it would sink 5a for the SCK-based v2 core — hence experiment **E1** below, run against
  both the CG and SCK paths (feeds R1 too).

#### 5b. Developer-side self-signed identity (Bagrat signs once, everyone verifies)

Same cert idea, but Bagrat's, applied at build time — no per-machine signing step. TCC's
csreq check (`identifier … and certificate leaf = H"…"`) is a hash comparison against the
cert *embedded in the signature*, which should not require the user's machine to trust the
cert. (c, med — no authoritative source found either way; one community claim that
"macOS does not recognize self-signed Team IDs for TCC" is unsourced.) Sweeter if it
works (zero install-time machinery); riskier (AMFI/TCC could demand an Apple anchor today
or in 27). **E2** settles it; 5a is the fallback and is strictly more robust.

#### 5c. Free-Apple-ID "Apple Development" certificate — the gray column

A $0 Apple ID enrolled in Xcode yields an `Apple Development` certificate with a real
Team ID; community guidance confirms TCC grants persist across builds signed with it
(b — [Screenify](https://www.screenify.studio/blog/2026-04-23-macos-screen-recording-permissions),
[OpenClaw signing docs](https://docs.openclaw.ai/platforms/mac/signing)). It would also
neutralize the Team-ID concern (real Team ID). But: it's development-purpose per Apple's
program terms, certs expire yearly, Apple can revoke, and distribution-signed-with-dev-cert
sits against the spirit if not the letter of the agreement — a dependency on Apple's
tolerance that the $0 *principle* (no Apple dependency) was meant to avoid. Keep as a
known door; don't build on it. Gatekeeper treatment is unchanged (still unnotarized).
([Developer ID requires the paid program](https://developer.apple.com/developer-id/) (a).)

#### The recurring nag nobody escapes

Independent of updates and of signing: since Sequoia, macOS re-confirms screen-capture
access with a **~monthly "…can access this computer's screen and audio. Continue to
allow?" prompt**, still present through Tahoe. Paid Developer ID does *not* remove it;
the `persistent-content-capture` entitlement that does requires Apple's case-by-case
blessing; apps using `SCContentSharingPicker` (user picks content each time) are exempt —
an R1 architecture input, not an R3 lever. (a/b, high —
[MacRumors](https://www.macrumors.com/2024/08/15/macos-sequoia-screen-recording-app-permissions/),
[mjtsai](https://mjtsai.com/blog/2024/08/08/sequoia-screen-recording-prompts-and-the-persistent-content-capture-entitlement/),
[TechRadar](https://www.techradar.com/computing/windows/fed-up-with-macos-sequoias-screen-recording-reapproval-requests-heres-how-to-make-them-go-away-for-good).)
Honest P14 note: **part of the recurring TCC tax is Apple policy, not the $0 stance** —
the invoice for "no certs" must not be padded with costs the certs wouldn't remove.

### 6. Compile-on-device at full-app scale

The POC's trust stance (§7: readable source, compile at setup, ~4 s shim) priced at v2 scale:

| Stack | Toolchain the user must hold | Cold build | Verdict |
|---|---|---|---|
| **Swift core (+ SwiftPM)** | Xcode **Command Line Tools only** (one interactive `xcode-select --install`, ~1–3 GB installed; no full Xcode for a SwiftPM-built bundle — POC-proven at shim scale) | ~1–5 min for an app-scale target (c — **E4** times it) | **Viable.** The only stack where compile-on-device stays a calm, CLT-only, minutes-scale act. |
| **Tauri (Rust + web UI)** | rustup+toolchain ~1.4 GB **plus** Node/npm **plus** CLT | **5–20 min first build** (300+ crates) (b/c — no clean public benchmark; [universal builds double it](https://dev.to/hiyoyok/building-a-universal-binary-with-tauri-v2-its-easier-than-you-think-1b53)) | Tolerable for the trust-maximalist techie; **not a default channel**. Updates re-pay minutes per version. |
| **Electron** | Building *Electron itself* from source: depot_tools, tens of GB, hours — nobody does this; "compile on device" degenerates to "download the prebuilt Electron blob," i.e. **the opaque-blob posture the stance exists to avoid** (c, high) | n/a | **Fails the trust test structurally**, independent of the updater disqualification in §4. |

Compile-on-device also does nothing for TCC by itself (every local build = new cdhash) —
it *needs* 5a anyway. So its true v2 role: **an offered `--from-source` path (tap formula
/ script flag) on top of the same channel and identity machinery, not the mainline.**
Prebuilt-binary trust is instead served cheaper by: source on GitHub + CI builds with
**artifact attestation/provenance + published checksums** the installer verifies. (c)

### 7. The experiments (cheap, local, decisive)

> **Update 2026-07-20: E1 and E2 executed live — both PASSED** (E2 in a stronger two-step
> form). §5's 5a mechanism is proven AND obsoleted in the same hour: 5b (developer-side
> cert) is live, so the per-machine cert machinery is only needed for from-source builds.
> E3's mechanism (replace + re-sign + relaunch, no re-grant) was proven inside E1's update
> leg; the Sparkle-vs-custom wrapper choice still waits on the shell decision. Full
> chronicle incl. wizard-grade gotchas: "Hands-on probe results" at the end of this doc.

Bagrat-style empirical probes; none mutate anything permanently, all on the Tahoe machine:

- **E1 — Self-signed identity × Screen Recording × updates (~30 min, settles 5a):**
  create a self-signed code-signing cert ("shroom-local") via Keychain Access/`security`;
  sign a dummy bundled capture app (`codesign -f -s shroom-local Dummy.app`); grant Screen
  Recording; capture one frame (CGDisplayStream *and* SCKit paths); **touch the binary,
  rebuild, re-sign with the same cert**; relaunch → does capture work with **no** re-toggle?
  Check `codesign -dr -` shows identifier+leaf DR and the TCC.db csreq matches POC §4's
  inspection recipe. Also directly retires/confirms the public Team-ID claim (E1-SCK leg).
- **E2 — Does the csreq validate when the cert isn't trusted (~10 min, settles 5b):**
  after E1, delete the cert+key from the keychain (grant row still present), relaunch the
  signed app → if capture still works, TCC's leaf-hash check is trust-free and 5b
  (developer-side cert) is live. Confirm on a second user account.
- **E3 — Sparkle/custom-updater swap under the identity (~30 min):** EdDSA-sign a v2 of
  the dummy, let Sparkle (or a 50-line swap script) replace it in place, re-sign with the
  local cert as a post-step → confirm no Gatekeeper dialog and no TCC re-grant.
- **E4 — CLT-only app-scale build timing (~20 min):** `swift build -c release` of an
  app-shaped SwiftPM target + bundle assembly on the M-series machine; record cold/warm
  times to replace the (c)-grade numbers in §6.

### 8. Recommended unsigned strategy (and rejected alternatives)

#### Recommendation

1. **Channel: terminal-mediated install, never browser-quarantined.** One installer
   script (hosted on shroom.am, runnable by hand or by the BYO-agent setup skill) **plus**
   a personal-tap Homebrew **formula** wrapping the same logic. It fetches the prebuilt,
   checksum/attestation-verified .app via curl → no quarantine → **zero Gatekeeper
   friction on any macOS 15–27**, no dialogs, no admin theater beyond what the script
   states. A plain .dmg stays available on the releases page for people who insist,
   documented with the honest 5-step Open Anyway walkthrough.
2. **Identity: per-machine self-signed cert, created once by the installer, re-applied to
   every update (5a)** — pending E1. If E2 also passes, simplify to developer-side
   signing (5b) and drop the install-time cert machinery.
3. **Updates: in-app, ed25519-verified, quarantine-free, with re-sign as a built-in
   post-step.** Custom ~200-line updater preferred (it's the only way the re-sign step is
   first-class); Sparkle 2 EdDSA-only is the off-the-shelf alternative if the shell is
   native and we accept wiring the re-sign around it. Result: **auto-update with zero
   Gatekeeper and zero TCC re-grant** — the "frictionless auto-update" ideal of the rethink,
   achieved inside the $0 line.
4. **Fallback if E1 fails** (self-signed identities rejected for Screen Recording): keep
   the exact same channel/updater, accept cdhash pinning, and ship the POC's proven
   softener — stale-grant self-reset + a said-line at first post-update capture
   ("macOS asks again after updates — one toggle"). Cost: **one Settings toggle + relaunch
   per update**, mic one click. That is the *entire* worst-case recurring invoice.
5. **Trust posture:** source on GitHub, CI builds with published checksums + artifact
   attestation, optional `--from-source` build path (Swift-core scale only), readable
   installer. Techie→non-techie handoff stays a no-go per the rethink — the agent-driven
   script *is* the C-persona story, and it happens to be the frictionless one.

#### Rejected alternatives

- **Homebrew cask** — being actively deleted for unsigned apps (Sept 1, 2026) and
  quarantines by policy even in third-party taps; would re-impose the Open Anyway dance
  per update. ([Homebrew/brew#20755](https://github.com/Homebrew/brew/issues/20755))
- **Browser-download .dmg as the primary channel** — pays the worst first-launch UX in
  the whole matrix on every macOS since Sequoia, re-pays it on every re-download, and
  invites the translocation trap; survives only as a documented secondary.
- **Electron's auto-updater path** — Squirrel.Mac hard-requires Apple signing; a $0
  Electron app has no native update story at all. ([Electron docs](https://www.electronjs.org/docs/latest/api/auto-updater))
- **Compile-on-device as the default channel** — POC-proven at 4-second-shim scale, but
  at app scale it means multi-GB toolchains and minutes-per-update for every user, buys
  no TCC stability by itself, and fails entirely at Electron scale; demoted to an
  optional trust-maximalist path (Swift-core only).
- **Free Apple Development certificate for distribution (5c)** — works technically per
  community reports, but leans on Apple's tolerance and annual cert churn; the $0 line is
  about independence from Apple, not just the fee. Kept as a documented emergency door.
- **Paying the $99/yr (reference)** — would buy: clean first launch for browser downloads,
  guaranteed TCC identity stability, cask eligibility, and nothing else (the monthly
  screen-capture nag stays). With E1 expected to pass and the script channel already at
  zero Gatekeeper friction, the purchasable delta shrinks to "a nicer DMG story" — the
  firmest empirical support the $0 principle has had yet. Decision per the rethink: **hold.**
- **Mac App Store (reference)** — paid account required (the rethink's tension resolved:
  it is a real contradiction), plus sandbox/review and screen-recorder entitlement
  friction; strictly dominated by Developer ID even inside the paid space for this app.

---


> Researched 2026-07-19. Claim labels: **(a)** verified from official docs [link] · **(b)** empirically
> tested now, on live POC infrastructure · **(c)** community-reported [link] · **(d)** inference.
> Empirical vantage caveat: all (b) network measurements were taken from one consumer connection,
> one ISP, one afternoon — fine for "does it work / order of magnitude," not a
> global latency survey.

---

## R4 — Kill per-video deploys: the Worker-served player route

**TL;DR verdict**

- **Mandated (P9) and buildable with no blockers found.** One Worker route `/v/<id>` rendering
  player + og tags from per-recording metadata in R2 (not KV) gives: link-exists-at-mint (offline
  included), rename-as-metadata-write, tombstones, and oEmbed — and **the rethink's rename-on-a-static-page question is solved by
  deletion**: "rename on a static page" stops being a problem because there is no static page.
- **Platform drift confirmed:** Workers-with-static-assets is Cloudflare's blessed path for new
  projects in 2026; Pages is not formally deprecated but is the legacy rail. Most POC §6 facts
  transfer; the Pages-specific ones (project-create guard, 500 builds/mo, random `pages.dev`
  suffix) die. **The no-R2-OAuth-scope wall still stands.**
- **Free-tier arithmetic passes with huge margin** at personal volume even if the Worker proxies
  every video byte; the ~$5 ceiling claim is verified (Workers Paid = $5/mo, 10M req + 30M CPU-ms
  included).
- **Metadata home: a per-recording `<id>/meta.json` object in R2**, not KV — R2 is strongly
  consistent globally, KV is eventually consistent with up-to-60 s propagation and cached
  negative lookups (poison for "content on its way" flipping to live).
- **Unfurl crawlers cannot tell a Worker-rendered page from static HTML** — no gotchas found at
  the protocol level; the real gotchas are operational (HEAD/Range handling, Bot Fight Mode on
  custom domains, platform-side unfurl caches vs rename).
- **Biggest incidental find:** the POC bucket has **no CORS configuration and r2.dev returns no
  `Access-Control-Allow-Origin`** (b) — hls.js playback from a different origin (Chrome et al.)
  was very likely broken in the POC. Worker-proxied segment delivery makes the whole CORS class
  vanish (same-origin) *and* lets the bucket stay fully private.

### R4.1 The route, concretely

```
GET /v/<id>            → player HTML (og/twitter tags server-rendered), X-Robots-Tag: noindex
GET /v/<id>?embed=1    → chromeless player (for iframes / oEmbed html)
GET /oembed?url=…      → oEmbed JSON (type=video, html=<iframe …?embed=1>)
GET /m/<id>/<file>     → (zero-DNS tier) media proxied from R2 binding: playlist, segments, poster, mp4
GET /v/<id>/download   → mp4 with Content-Disposition: attachment; filename="<current title>.mp4"
```

**Page states (P9 + P2):**

| State | Trigger | Page shows | og tags |
|---|---|---|---|
| **On its way** | id valid, no `meta.json` yet (or `status:"uploading"`) | "Your video is on its way" + auto-refresh/poll | title placeholder, poster placeholder |
| **Live** | `meta.json` present, playlist uploaded | player | full set (§R5 minimum set) |
| **Tombstone** | `meta.json` has `{status:"deleted", deletedAt}` | "This video was removed" | minimal (title = "Video removed", no og:video) |
| **Invalid id** | id fails validity check | 404 or same tombstone copy | — |

- **Delete = remove all media objects, overwrite `meta.json` with a small remnant**
  (`{status:"deleted", deletedAt, …optionally title}`). Cheap because pages render from metadata;
  the remnant is the P2 "small metadata remnant."
- **Rename = re-PUT `meta.json`.** R2 is strongly consistent — "readers will immediately see the
  latest object globally" (a: [R2 consistency](https://developers.cloudflare.com/r2/reference/consistency/)) —
  so a rename is live on the next page load, worldwide, with no deploy and no propagation window.
  This is the concrete death of that open question: it is resolved by deletion
  if R4 lands (it has now been mandated, so: resolved).
- **Never-404 without garbage-page pollution (option):** mint ids as `random ‖ short-HMAC(random)`
  under a key the Worker holds as a secret. The Worker can then verify offline-minted ids without
  any storage: valid-but-unknown → "on its way"; invalid → 404. Costs nothing, keeps the "link
  exists the moment the app mints it, even offline" contract airtight, and the key is naturally
  part of the shared config that P12's second machine receives anyway (seam with R10 — their
  credential-handoff design should carry this key; flagged, not designed here). (d)

**Metadata home — recommendation and rejected alternatives:**

- **Recommended: `<id>/meta.json` in the same R2 bucket, next to the segments.**
  Strongly consistent (a: above), multi-writer-safe by construction (one object per recording;
  whole-object PUT; the 1-write/sec/key R2 limit (a:
  [R2 limits](https://developers.cloudflare.com/r2/platform/limits/)) is irrelevant at rename
  frequency), no extra service to provision, and reads from the Worker are Class B ops
  (10M/month free, then $0.36/M — a: [R2 pricing](https://developers.cloudflare.com/r2/pricing/)).
  This is exactly the "per-recording objects" shape §5.6/P12 want; the library index question
  (list-on-read over `*/meta.json` vs a mediated index) is **R10's deliverable — stay out, but
  this layout is the compatible substrate.**
- **Rejected: KV.** Eventual consistency with "up to 60 seconds or more" propagation and — the
  killer — **cached negative lookups** (a:
  [how KV works](https://developers.cloudflare.com/kv/concepts/how-kv-works/)): the "on its way"
  page could keep answering "not yet" for a minute after the upload finished, at exactly the
  moment the sender is staring at it. Free-tier write cap (1,000 writes/day; a:
  [KV limits](https://developers.cloudflare.com/kv/platform/limits/)) is workable but adds a
  quota to think about for zero benefit. KV remains a fine *optional cache* later.
- **Rejected: D1.** A relational DB for a handful of JSON fields is a second stateful service,
  a migration story, and a backup story — for nothing v1 needs. Reconsider only if R10 lands on
  a Worker-mediated index that genuinely wants queries.
- **Trade-off owned:** an R2 read from a far-away edge adds ~100–300 ms to page TTFB (the bucket
  is regional, the Worker is global) (d). Acceptable for a page load; if it ever isn't, the fix
  is edge-caching the rendered HTML on a custom domain — an optimization, not a redesign.

**noindex mechanics (P1 — and load-bearing for R5):**

- Serve **`X-Robots-Tag: noindex`** on every `/v/*` response; optionally also
  `<meta name="robots" content="noindex">` in the HTML. Header and meta are equivalent for
  search engines (a:
  [Google block-indexing](https://developers.google.com/search/docs/crawling-indexing/block-indexing)).
- **Do NOT disallow `/v/` in robots.txt.** Two reasons, both hard: (1) Google only honors noindex
  if it can fetch the page — "the page or resource must not be blocked by a robots.txt file" (a:
  same doc); (2) several unfurl fetchers **do respect robots.txt at fetch time** — Discordbot
  (c: [PreviewOG](https://previewog.com/discord-link-preview/)) and Twitterbot (a:
  [X card troubleshooting](https://developer.x.com/en/docs/x-for-websites/cards/guides/troubleshooting-cards))
  — so a Disallow would kill their unfurls. Slack explicitly does *not* honor robots.txt (a:
  [Slack robots](https://api.slack.com/robots)).
- **Answer to the P1 interaction question: yes, unfurl crawlers still work with noindex set.**
  noindex is an *indexing* directive, not a fetch block; unfurl fetchers aren't search indexers
  and ignore it (a for Google semantics, d for the generalization — no counterexample found on
  any target platform). The combination "noindex header + permissive robots.txt" is exactly
  right for unlisted-by-obscurity.

### R4.2 Worker-rendered og tags vs static HTML — crawler gotchas

- **Protocol-level: none.** A Worker response is just an HTTP response; crawlers cannot tell.
  The POC's real lesson — og tags must be present in the raw HTML because crawlers don't run JS
  (POC §5) — transfers verbatim: the Worker must render tags server-side into the HTML string.
- **Handle HEAD and Range.** Slack's fetcher "fetches as little of the page as it can (using
  HTTP Range headers)" (a: [Slack robots](https://api.slack.com/robots) /
  (c: [knownagents](https://knownagents.com/agents/slackbot-linkexpanding))). Keep `<head>` small,
  og tags first, and don't 500 on HEAD/Range requests. (Static hosting got this for free; a
  hand-rolled Worker must not regress it.)
- **Latency is a non-issue.** Cold starts are ~5 ms and effectively hidden — Cloudflare pre-warms
  the Worker during the TLS ClientHello (a:
  [eliminating cold starts](https://blog.cloudflare.com/eliminating-cold-starts-with-cloudflare-workers/));
  crawler timeouts are in seconds.
- **Custom-domain tier gotcha: Cloudflare's own bot defenses can block unfurl crawlers.**
  Slackbot-blocked-by-Cloudflare is a documented failure class (c:
  [opengraphplus](https://opengraphplus.com/consumers/slack/issues/slackbot-blocked)). The wizard
  must not enable Bot Fight Mode / aggressive WAF on the video zone, or must ship a skip rule for
  known unfurl UAs. Zero-DNS tier (workers.dev) has no such user-configurable trap. (d on the
  wizard implication)
- **Renames vs platform unfurl caches:** platforms cache unfurls per URL (Slack ~30 min-ish,
  Discord minutes-to-hours (c: PreviewOG above)). An already-posted message keeps its old title;
  new shares pick up the rename after cache expiry. Not fixable from our side; document it as
  expected behavior, don't chase it.

### R4.3 Platform-drift check: Pages vs Workers in 2026

**Verdict: build v2 on Workers + static assets; treat Pages as legacy-but-alive.** Cloudflare's
migration guide does not declare Pages deprecated but recommends Workers for new projects —
"Workers has a distinctly broader set of features" (a:
[migrate from Pages](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/));
third-party 2026 consensus is the same: full feature parity for static assets since ~March 2026,
Workers as the greenfield default (c:
[mecanik.dev](https://mecanik.dev/en/posts/cloudflare-pages-vs-workers-which-to-use-in-2026/),
[cogley.jp](https://cogley.jp/articles/cloudflare-pages-to-workers-migration)). One genuine Pages
advantage lost: Pages supports custom domains on zones *outside* Cloudflare; **Workers custom
domains require the zone on your Cloudflare account** (a: migration guide matrix) — consistent
with R2's same requirement, so it doesn't change shroom's tiering.

**POC-LEARNINGS §6 fact-by-fact transfer table:**

| POC §6 fact | v2 (Workers) status |
|---|---|
| OAuth vs account-token split; narrow `--scopes` | **Transfers.** Default OAuth scope list now includes `workers:write`, `workers_scripts:write`, `workers_kv:write`, `d1:write`, etc. — narrow to what's needed (c: [scope list](https://github.com/cloudflare/workers-sdk/discussions/14161), [wrangler docs](https://developers.cloudflare.com/workers/wrangler/commands/general/)) |
| **No R2 OAuth scope** → dashboard account token required | **Still true as of 2026-07** — no `r2:*` in the default scope list (c: same sources). Cheap probe: `wrangler login --scopes-list`. The two-credential setup dance survives into v2 unchanged. |
| `pages project create` CLI-guard, REST workaround | **Dies.** No project-create step; `wrangler deploy` creates the Worker. |
| `pages deploy` rides OAuth non-interactively | **Expected analog** (`wrangler deploy` under `workers_scripts:write`) — (d), unverified; cheap probe: one hello-world deploy over OAuth on a clean account. |
| Direct-upload REST API (Pages) | **Replaced, still exists:** Workers Script Upload + `assets-upload-session` (manifest → JWT-batched file uploads → completion token → version create) (a: [direct uploads](https://developers.cloudflare.com/workers/static-assets/direct-upload/), [API](https://developers.cloudflare.com/api/resources/workers/subresources/assets/subresources/upload/)). More ceremony than Pages' but fully wizard-able without wrangler (feeds R7). |
| Freshest-token bug (stale config shadowing) | **Transfers** — it's a wrangler behavior, not a Pages one. |
| `whoami` exits 0 when logged out | **Transfers.** |
| Email-verification gate (HTTP 403 code 8000077), R2 credit-card gate | **Transfer** — account-level, endpoint-agnostic in principle; keep the probe-capability-not-state classifier, don't hard-code the Pages endpoint's code (d on exact code stability). |
| Classified-error state machine / idempotent setup | **Transfers as a pattern**; error catalogue needs re-validation against Workers endpoints. |
| `*.r2.dev` rate-limited, "not for production" | **Confirmed current** (see R6). |
| Random `pages.dev` suffix; "parse deploy output, never construct" | **Dies/changes:** workers.dev hostnames are deterministic — `<worker>.<account-subdomain>.workers.dev` after a one-time subdomain registration. Parsing output stays good hygiene. |
| Pages 20k files/site cap (≈10k videos) | **Replaced and mooted:** Workers static assets cap is 20k files *per version* free / 100k paid (a: [limits](https://developers.cloudflare.com/workers/platform/limits/)) — and the dynamic route means the asset count is O(app), not O(videos). The cap ceases to be a scaling axis at all. |
| 500 builds/month | **Dies** — no build service in the loop. |
| LAN MTU black hole on Pages asset uploads | **Transfers as environmental risk** to any large-body Cloudflare upload (Workers asset uploads included) — but exposure collapses from *every publish* to *app-version deploys only*, since publishes no longer deploy anything. R2 uploads were always unaffected. |
| Public-bucket enable = security-weakening consent | **Becomes optional:** with Worker-proxied media (R4.5) the bucket **stays fully private** — this consent disappears from setup on the zero-DNS tier. |

### R4.4 Free-tier arithmetic and the ~$5 ceiling

Current numbers, 2026-07 (a:
[Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/),
[limits](https://developers.cloudflare.com/workers/platform/limits/)):
Free = 100,000 req/day, 10 ms CPU/invocation; **static asset requests free and unlimited**;
Paid = $5/mo, 10M req + 30M CPU-ms included. KV free = 100k reads/day, 1k writes/day. R2 free =
10 GB storage, 1M Class A + 10M Class B ops/month.

| Scenario | Worker req/day (everything proxied) | Worker req/day (media direct from R2) | Verdict |
|---|---|---|---|
| Personal: 3 rec/wk × 5 min (50 segs), ~8 views each | ~200 | ~10 | 0.2% / noise of free tier |
| Heavy personal: 1 rec/day × 10 min, 15 views | ~1,600 | ~50 | 1.6% of free tier |
| v2 team of 10: 15 rec/day × 10 min, 15 views each | ~24,000 | ~1,600 | 24% of free tier proxied; noise if direct |
| R2 Class B (team case) | ~700k/mo | same | 7% of free 10M |
| CPU | ~1–3 ms per render/proxy invocation (d) | — | 10 ms cap fine; streaming a body costs no CPU while pumping |

**The rethink's ~$5 ceiling claim is verified**: even the team scenario never approaches the
paid tier's 10M req/month, so $5/mo is not just the ceiling — it's overkill; the honest statement
is "**free tier suffices; $5/mo removes the daily cap if you ever hit it.**"
One sharp edge: **the free cap fails closed** — Worker-routed requests over 100k/day get 429
until midnight UTC (a:
[static-assets billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/)).
That is an argument for keeping *segment bytes* off the Worker on the free tier if a video could
ever go semi-viral, or for the $5 upgrade being the documented remedy.

### R4.5 Segment delivery: straight-from-R2 vs Worker-proxied

Empirical + doc findings that decide this:

- **CORS discovery (b):** the live POC bucket answers cross-origin GETs **without any
  `Access-Control-Allow-Origin`** and 403s OPTIONS preflights; the archived setup scripts contain
  no CORS provisioning at all. hls.js uses XHR/fetch, which is CORS-gated — so **POC playback in
  Chrome/Firefox from the pages.dev origin was very likely broken** (Safari's native HLS uses
  media loads, which aren't CORS-gated, masking it) (d on the browser-impact inference; probe:
  open a POC link in Chrome and watch the console). Any direct-from-R2 design must add
  `PutBucketCors` to provisioning. Worker-proxied same-origin delivery deletes the entire class.
- **Egress is free either way** (a: [R2 pricing](https://developers.cloudflare.com/r2/pricing/)).
  Worker-proxied adds only request-count + negligible CPU; R2 reads via binding are Class B ops
  in both paths.
- **Proxying enables later (the v2 option value):** per-object authorization for team links
  (P1/R10), view/last-viewed tracking by counting playlist fetches (§5.4 v2 want), and
  **download naming without slug-in-key**: `Content-Disposition: attachment; filename=<title>.mp4`
  computed from live metadata — which un-couples the MP4's object key from the title and makes
  renames retroactively fix the download filename (the POC's slug-in-key hack dies).
  Implementation note: the Worker must forward `Range` headers to the R2 binding
  (`get(key, {range})`) and answer 206 for MP4 scrubbing (a:
  [R2 Workers API](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/)).
- **Cache API caveat:** functional cache operations are documented for custom domains and
  pages.dev — workers.dev is conspicuously absent and community-reported as a no-op (a/c:
  [Workers cache](https://developers.cloudflare.com/workers/runtime-apis/cache/)). So on the
  zero-DNS tier, proxied segments hit R2 every time (fine: Class B free tier is 10M/mo).

**Recommendation:**

- **Zero-DNS tier: proxy everything through the one Worker** (`/m/<id>/…` from the private
  bucket). One origin (no CORS at all), one hostname in every URL, bucket never made public
  (drops a scary setup consent), auth/view-tracking become additive later. The 100k/day
  fail-closed cap is the accepted cost — 20×+ headroom at personal volume, and "$5 removes it"
  is honest wording.
- **Custom-domain tier: media direct from the bucket's custom domain** (edge-cached, unlimited,
  h2/h3), Worker for HTML/meta/oEmbed only. Requires CORS config on the bucket for the player
  origin, and mind the **cached-404 trap**: on cache-enabled custom domains, 404s are cached by
  default, so an "on its way" page polling for `stream.m3u8` can keep seeing a cached 404 after
  publish (a: [R2 consistency](https://developers.cloudflare.com/r2/reference/consistency/)).
  Poll via the Worker binding (bypasses cache) or set a cache rule excluding `.m3u8`.
- **Rejected: r2.dev as the zero-DNS media path** — workable (see R6) but strictly worse than the
  same-effort Worker proxy: "not for production" wording, variable throttle, no HTTP/2, public
  bucket required, CORS config required, second hostname in URLs.
- **Rejected: Worker-proxied on the custom-domain tier too** — gives up free unlimited edge-cached
  delivery for no v1 gain; revisit only when team auth (R10) actually ships.

### R4.6 Cold start / global latency reality

- Cold start ~5 ms, hidden behind the TLS handshake pre-warm (a:
  [Cloudflare blog](https://blog.cloudflare.com/eliminating-cold-starts-with-cloudflare-workers/));
  independent 2026 benchmarks agree (<5 ms p95) (c:
  [tech-insider.org](https://tech-insider.org/cloudflare-workers-vs-lambda-2026/)). Not a factor.
- Page TTFB budget = RTT + render + metadata read. The metadata read dominates when the viewer is
  far from the bucket's region (~100–300 ms) (d). Measured baseline for static on the same
  network: pages.dev HTML TTFB 58 ms (b). Expected Worker page TTFB: ~150–500 ms globally —
  Loom-grade, and far inside crawler timeouts.

### R4.7 Overall recommendation & rejected alternatives

**Recommended:** one Worker, static assets for player JS/CSS (free, unlimited), dynamic `/v/<id>`
+ `/oembed` + (zero-DNS) `/m/…` routes, per-recording `meta.json` in R2, HMAC-verifiable ids,
`X-Robots-Tag: noindex` + permissive robots.txt, tombstone-from-remnant. Deployable by wizard or
BYO-agent through the documented direct-upload REST chain — no wrangler required in principle
(hand the details to R7).

- **Rejected: keep per-video static deploys (POC §5 architecture).** Cannot satisfy P9 (offline
  stop → no deploy → dead link), keeps the rename-redeploy machinery, keeps deploy-per-publish
  exposure to the MTU trap, walks toward the 20k-file cap, and burns a deploy cycle per rename.
  Every one of its jobs is done better by the route.
- **Rejected: static shell + client-side metadata fetch.** Breaks unfurls (crawlers don't run
  JS — POC §5's own lesson) and still needs a Worker for oEmbed; you'd run a Worker anyway and
  get worse pages.
- **Rejected: SSR framework on Workers (Next/Remix/Astro).** The page is one template with ~6
  variables; a framework adds cold-start weight, build tooling, and upgrade churn against
  [principle 4](../docs/PRINCIPLES.md) (no bloat) for zero functional gain. Hand-rendered template string + the escaping
  tests from the POC (`render.mjs` heritage) suffice.

**Cheap probes for the stack session:** (1) `wrangler login --scopes-list` — is there an R2 scope
yet; (2) hello-world Worker deployed over OAuth non-interactively on a fresh account — confirms
the Pages-deploy parity; (3) open a live POC link in Chrome — confirms/refutes the CORS playback
break; (4) one `assets-upload-session` REST walk-through with curl — de-risks the R7 wizard path.

---

## R5 — What unfurl-into-a-player actually requires, per platform

**TL;DR verdict**

- **The §5.4 requirement must be honestly downgraded for most business chat:** inline playback for
  arbitrary domains is **provider-allowlisted on Slack, Teams, X, and Linear** — shroom links
  there get, at best, a rich card (title/description/poster) + click-through. The doc should say
  so plainly.
- **Where inline genuinely works for arbitrary domains:** **iMessage** (og:video → direct MP4
  autoplays muted, loops — documented Apple behavior) and, community-proven at fxtwitter scale,
  **Discord** (direct-MP4 og:video / twitter:player:stream). **Notion** embeds the actual player
  page in an iframe (user picks "Embed"), which is inline playback by another door.
- **The POC's "og-tag unfurls that worked" were image cards, not players** — its template ships
  `og:type video.other` + `twitter:card summary_large_image` and **no `og:video` URL at all** (b:
  read from the live built page). v2 starts from card-grade, not player-grade.
- **og:video must point at an MP4, not HLS** — which drafts the MP4 tee into unfurl duty and
  raises a payload-size question (full MP4 vs small teaser) that is a real design decision.
- **An oEmbed endpoint is cheap under R4 and unlocks the iframely-powered ecosystem** (Notion's
  auto-detection among them) plus open-web CMSes — but none of the four allowlisted platforms
  above will start playing inline because of it.
- **noindex does not interfere with any of this** (see R4.1); the thing that would is a
  robots.txt Disallow (Discord/X/Google respect it; Slack ignores it) — keep robots.txt permissive.

### R5.1 Ground truth per platform

| Platform | Mechanism read | Inline playable for arbitrary domains? | What a shroom link gets | Sources |
|---|---|---|---|---|
| **Slack** | oEmbed, then Twitter Card / OG tags (fetches minimal bytes via Range) | **No** — inline video only for registered providers or an installed Slack app using `chat.unfurl` video blocks (apps must register ≤5 domains) | Rich card: title, description, poster image; click-through | (a) [Slack robots](https://api.slack.com/robots), [unfurl docs](https://docs.slack.dev/messaging/unfurling-links-in-messages/); (c) [Slack dev blog](https://medium.com/slack-developer-blog/everything-you-ever-wanted-to-know-about-unfurling-but-were-afraid-to-ask-or-how-to-make-your-e64b4bb9254), [clipy.online write-up](https://clipy.online/blogs/slack-screen-recorder/) |
| **Linear** | Named-provider auto-embeds only (YouTube, Loom, Descript, Figma) | **No** — arbitrary links stay links ("Keep as link" is the only alternative offered) | A plain link; no card, no player. The rethink's sentence "a link in a Linear comment unfurls into a real player" is **not achievable for an arbitrary domain today** | (a) [Linear editor docs](https://linear.app/docs/editor) |
| **Notion** | iframely (~1,900 QA'd domains) + generic "Create embed" = iframe of the URL | **Yes, via iframe** — paste → "Embed" renders the page itself; needs the page to allow framing + a good `?embed=1` view. Auto-detection for unknown domains may improve with an oEmbed endpoint (iframely honors discovery/og:video generically) | The real player, embedded, after one user click ("Embed") | (a) [Notion help](https://www.notion.com/help/embed-and-connect-other-apps); (c) [iframely providers](https://iframely.com/docs/providers) |
| **Teams** | OG tags (og:title + og:description + og:image required) → Adaptive Card; schema.org micro-capabilities for richer cards | **No** — the micro-capability template set is Article/Product/Event/LocalBusiness/Course/Restaurant; **there is no VideoObject template**; true custom unfurls need an installed Teams app | Card + click-through (admins can disable previews tenant-wide) | (a) [micro-capabilities](https://learn.microsoft.com/en-us/microsoftteams/platform/messaging-extensions/how-to/micro-capabilities-for-website-links), [link unfurling](https://learn.microsoft.com/en-us/microsoftteams/platform/messaging-extensions/how-to/link-unfurling); (c) [opengraphplus](https://opengraphplus.com/consumers/teams/issues/preview-not-showing) |
| **Discord** | OG tags (+ Twitter tags as fallback); respects robots.txt (`Discordbot`) | **Yes (community-proven, officially undocumented):** direct-MP4 `og:video`/`og:video:secure_url` (+`twitter:player:stream`) renders Discord's native inline player — this is exactly how FxTwitter/TwitFix work ("embeds the raw mp4 file"); iframe-style players remain provider-allowlisted | Inline playing video if the MP4 recipe is followed; else poster card | (c) [FxEmbed](https://github.com/FxEmbed/FxEmbed), [TwitFix](https://github.com/daisyUniverse/TwitFix), [embed recipe gist](https://gist.github.com/mohsreg/927bf8b2092515ee1a8ee88c3e4d2c14), [PreviewOG](https://previewog.com/discord-link-preview/) (claims allowlist-only — superseded by the FxTwitter existence proof; confidence medium-high, probe cheap) |
| **iMessage** | OG tags via LinkPresentation (fetched by the **sender's** device) | **Yes (documented):** `og:video` → direct MP4 (`video/mp4` only on iOS) autoplays muted, loops, minimal controls; `og:video` supersedes `og:image` | Inline playing video in the bubble | (a) [Apple TN3156](https://developer.apple.com/documentation/technotes/tn3156-create-rich-previews-for-messages); (c) [emerge agency recipe](https://www.emergeagency.com/insights/detail/rich-video-previews-in-ios-macos-messages/) |
| **X** | Twitter Card tags; Twitterbot respects robots.txt | **No** — Player Card requires per-domain approval, which is effectively not granted to small third parties in 2026 | `summary_large_image` at best: poster image with domain overlay (post-2023 X strips title text from link cards); click-through | (a) [player card approval](https://developer.twitter.com/en/docs/twitter-for-websites/cards/guides/player-card-approval), [troubleshooting](https://developer.x.com/en/docs/x-for-websites/cards/guides/troubleshooting-cards); (c) [devcommunity](https://devcommunity.x.com/t/are-player-cards-deprecated/165663), [card guide](https://www.thatdevpro.com/reference/html-twitter-cards/) |

**Honest requirement wording for the rethink's unfurl requirement:** *"the link unfurls into a rich preview
everywhere; it plays inline in iMessage and Discord, embeds as a real player in Notion, and is a
rich card + one click in Slack, Teams, Linear and X."* Anything stronger is not buildable from
the outside — the platforms gate inline players on allowlists (Slack/Teams/X) or named
integrations (Linear).

### R5.2 What an oEmbed endpoint unlocks that og tags don't

- **Unlocks:** the iframely/Embedly-consuming ecosystem — Notion's auto-embed detection for
  unknown domains, Confluence/Trello-class tools, and oEmbed-discovery CMSes (WordPress, Ghost).
  For all of them, a `type: "video"` response whose `html` is an iframe of `/v/<id>?embed=1`
  turns "paste a link" into "get the player." (a:
  [oEmbed spec](https://oembed.com/) discovery; c: [iframely oEmbed API](https://iframely.com/docs/oembed-api))
- **Does not unlock:** Slack, Teams, X, or Linear inline playback — their gates are allowlists,
  not missing metadata. Slack does list oEmbed first in its read order (a:
  [Slack robots](https://api.slack.com/robots)), but modern arbitrary-domain behavior is
  card-only (c/d; probe below).
- **Cost under R4: ~a dozen lines** (one Worker route + one `<link rel="alternate"
  type="application/json+oembed">` per page). **Recommendation: ship it in v1** — asymmetric
  cost/benefit. **Rejected alternative:** chasing iframely/Embedly *partnership listings* — the
  generic discovery path is enough and requires no relationship.
- Also required for the embed door: **no `X-Frame-Options`, permissive/absent
  `frame-ancestors`** on `/v/<id>?embed=1` (Notion-style embeds die otherwise), while the
  non-embed page may stay stricter. (d — standard mechanics)

### R5.3 Minimum metadata set (the union that serves every column above)

```html
<meta property="og:type"            content="video.other" />
<meta property="og:title"           content="…" />
<meta property="og:description"     content="…" />
<meta property="og:url"             content="https://…/v/<id>" />
<meta property="og:site_name"       content="…" />           <!-- P5-adjacent; configurable -->
<meta property="og:image"           content="https://…/poster.jpg" />  <!-- 1200×630, HTTPS, <5 MB -->
<meta property="og:video"           content="https://…/unfurl.mp4" />  <!-- direct MP4, NOT HLS -->
<meta property="og:video:secure_url" content="https://…/unfurl.mp4" />
<meta property="og:video:type"      content="video/mp4" />
<meta property="og:video:width"     content="1920" />
<meta property="og:video:height"    content="1080" />
<meta property="og:video:duration"  content="…" />
<meta name="twitter:card"           content="summary_large_image" />
<meta name="twitter:title"          content="…" />
<meta name="twitter:description"    content="…" />
<meta name="twitter:image"          content="https://…/poster.jpg" />
<meta name="twitter:player:stream"  content="https://…/unfurl.mp4" /> <!-- Discord belt+braces -->
<link rel="alternate" type="application/json+oembed" href="https://…/oembed?url=…" />
```
Plus response headers: `X-Robots-Tag: noindex`; **no** X-Frame-Options on the embed view; and the
poster/MP4 URLs must themselves be crawler-fetchable (no robots.txt Disallow on their host —
Twitterbot checks robots.txt for images too (a: X troubleshooting above)).

**The og:video payload question (design decision, flag for the stack session):** iMessage
*autoplays and loops* whatever `og:video` points at, on the sender's and receivers' devices; a
1.6 GB/hour full MP4 is hostile there ("the larger the video, the longer it will take to buffer"
— c: emerge agency). Options: (1) point og:video at the full MP4 anyway — simplest, fine for
short clips, ugly for hour-long ones; (2) **generate a small low-res teaser (~10–20 s) at
finalize and use it as `unfurl.mp4`** — best unfurl UX, one more ffmpeg/AVAssetWriter output;
(3) omit og:video above some duration threshold — degrades iMessage/Discord to cards for long
videos. Leaning (2)-with-(3)-fallback (d); cheap to prototype once the capture core exists.
Keep the key **stable and title-independent** (`<id>/unfurl.mp4`) so renames never break posted
unfurls.

### R5.4 Per-platform test protocol (for the v2 test plan)

| Platform | Validator / method | Cache-bust |
|---|---|---|
| Generic OG lint | [opengraph.xyz](https://www.opengraph.xyz/), [metatags.io](https://metatags.io/); curl with each bot UA (`Slackbot-LinkExpanding 1.0`, `Discordbot/2.0`, `Twitterbot`, `facebookexternalhit`) asserting 200 + tags in raw HTML — CI-able | n/a |
| Facebook debugger | [Sharing Debugger](https://developers.facebook.com/tools/debug/) — best generic og fetcher with forced re-scrape, useful even off-target | "Scrape again" |
| Slack | Private test channel; paste link; no official validator | append `?v=N` |
| Discord | Private server; paste link | append `?v=N` |
| iMessage | Send to self, iOS + macOS (sender's device renders — test both directions) | new URL |
| Teams | Compose-box preview in a dev tenant; [validator.schema.org](https://validator.schema.org/) if micro-capability JSON-LD is added | new URL |
| X | Post from a test account (Card Validator preview was removed in 2022 — c: [devcommunity](https://devcommunity.x.com/t/card-validator-preview-removal/175006)) | new URL |
| Notion | Paste in a scratch page → choose "Embed"; also run the URL through [iframely debugger](https://iframely.com/debug) (same engine Notion uses) | re-paste |
| Linear | Paste in a scratch issue comment; expect a plain link (assert non-regression, not magic) | re-paste |

### R5.5 Recommendation & rejected alternatives

**Recommended:** ship the R5.3 tag set + oEmbed route + teaser-MP4 decision; word the marketing
claim per R5.1's honest sentence; adopt R5.4 into the test plan.

- **Rejected: an official Slack/Teams app to get true inline players.** It would work
  (Slack `chat.unfurl` video blocks; Teams messaging-extension unfurls) but requires a per-workspace
  app install and an event-receiving endpoint — a support surface and adoption step wildly out of
  proportion for v1, though the user's own Worker *could* host the endpoint someday (v2+ candidate,
  BYO-backend-compatible, park it).
- **Rejected: pursuing X Player Card approval** — effectively unobtainable for arbitrary small
  domains in 2026 (c: above), and X is the least important target per the rethink.
- **Rejected: HLS URL in og:video** — no unfurler consumes HLS; MP4 or nothing.

---

## R6 — Is r2.dev good enough to stream video to real recipients?

**TL;DR verdict**

- **Empirically today: yes for personal scale, with headroom** (b): TTFB 120–250 ms, single-stream
  5–8 MB/s (≈40–67 Mbps, 12–20× the 3.5 Mbps stream rate), six parallel 2 MB segment fetches all
  complete ≤1.1 s, 60-request burst all-200, Range/206 works. A 6 s-segment HLS viewer needs
  ~0.06 req/s and ~0.44 MB/s — three simultaneous viewers use ~1% of the documented throttle
  threshold.
- **Officially: it remains "not intended for production," with a variable rate limit** —
  throttling (429s) at "hundreds of requests/second," and "bandwidth (throughput) may also be
  throttled" (a). The risk is not that it breaks at shroom's volumes; it's that **none of the
  good numbers above are promised.**
- **Structural (b) findings that are not in the docs:** r2.dev serves **HTTP/1.1 only** (ALPN
  refuses h2), does **no edge caching** (no Cache-Control/CF-Cache-Status; every GET hits the
  bucket), and — POC-specific — the bucket has **no CORS config**, which hls.js needs cross-origin.
- **The middle path exists and is better than r2.dev at the same DNS cost: a Worker on
  workers.dev proxying a private bucket** (R4.5). Same zero-DNS persona, no public bucket, no
  CORS, no "not for production" asterisk, h2/h3, auth-ready. **Recommended as the zero-DNS
  default; r2.dev rejected.**
- **Custom-domain tier requires the user's domain as a zone on the same Cloudflare account**
  (nameserver move; a) — the no-domain persona genuinely cannot use it, so P4's two-tier framing
  is correct; the upgrade buys edge caching, analytics, no rate limit, and h2/h3.
- **Loom-grade "instant streaming" is cleared** on the measured path (first frame well under 1 s
  nearby); a multi-continent vantage check is the one probe money can't skip forever.

### R6.1 Documented r2.dev behavior (a)

From [R2 limits](https://developers.cloudflare.com/r2/platform/limits/) and
[public buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/), 2026-07:
r2.dev is "not intended for production usage"; exceeding a **variable** rate limit ("hundreds of
requests/second") → temporary 429s; **bandwidth may also be throttled**; no custom caching, no
analytics; the production answer is a custom domain (up to 100 per bucket). Community threads
confirm the limit is variable and unpublished by design (c:
[community](https://community.cloudflare.com/t/what-is-r2-rate-limit/746975)).

### R6.2 Empirical probe (b) — live POC bucket, 2026-07-19

Method: read-only curl against the still-published POC recording
(the POC's public bucket, 35 s HLS VOD, 6×~1.7–2 MB fMP4 segments + 11.4 MB
MP4), from one consumer connection, ~50 Mbps-class downlink. Single vantage; the POC-era
LAN MTU black hole affected *uploads* to Pages endpoints only and does not taint these download
measurements (POC §6).

| Measurement | Result |
|---|---|
| Playlist TTFB (fresh conn) | 0.12–0.20 s |
| Segment TTFB / full fetch (~2 MB, serial) | 0.12–0.20 s / 0.26–0.38 s (5.3–6.3 MB/s) |
| 6 parallel segment GETs (≈ aggressive player or 3 viewers) | all 200; slowest total 1.10 s; aggregate ≈ 10 MB/s (likely local-downlink-bound) |
| 11.4 MB MP4 sustained | 8.35 MB/s (~67 Mbps) |
| Burst: 60 GETs, 6-way parallel | 60× 200, no 429, no slowdown |
| Repeat fetches | no cache headers, TTFB unchanged → every request hits the bucket (no edge cache) |
| Range request | 206, honored (MP4 scrubbing OK) |
| ALPN | server selects **http/1.1** despite h2 offered (pages.dev negotiated h2 with the same client) |
| CORS | no `Access-Control-Allow-Origin` with a matching Origin; OPTIONS → 403 (no bucket CORS config in the POC) |

**Arithmetic vs Loom-grade (§5.4):** startup = HTML + playlist + init + first segment ≈ 2 MB over
3–4 round trips → **first frame in well under 1 s** on the measured path; steady-state need is
0.44 MB/s vs 5–8 MB/s measured. A 20-viewer simultaneous team watch ≈ 3–4 req/s — two orders of
magnitude under the documented throttle onset. HTTP/1.1-only costs little for HLS (players fetch
mostly serially; browsers open parallel h1 connections) but is a real difference from
custom-domain delivery (h2/h3) (d on impact).

### R6.3 The two supported tiers (P4), characterized honestly

**Zero-DNS tier.** Two sub-options exist; only one should ship:

- **r2.dev direct — rejected as default.** It works today (above), but: unpromised variable
  limits + explicit "not for production" wording (the wizard would have to caveat its own
  default), public-bucket enable required (a security-weakening consent), bucket CORS config
  required for hls.js, no HTTP/2, a second hostname in every URL, and no path to auth or view
  tracking. Acceptable as a debug/fallback path only.
- **Worker-on-workers.dev proxying a private bucket — recommended default** (details in R4.5).
  Same "no domain, no DNS, ever" persona; hostname is the user-picked
  `<name>.<subdomain>.workers.dev`; the only quota that matters is 100k req/day fail-closed
  (huge margin at personal volume; "$5 removes it" is the honest remedy line). No Cache API on
  workers.dev (a/c: [cache docs](https://developers.cloudflare.com/workers/runtime-apis/cache/))
  means every proxied read is an R2 Class B op — 10M/month free covers it ~15× at team volume.
- **Honest wording for the wizard (zero-DNS):** "runs on Cloudflare's free developer hostname;
  personal-scale streaming with generous headroom; heavy simultaneous audiences may hit the free
  daily cap — $5/mo on your account removes it."

**Custom-domain tier.** Requires the domain to exist **as a zone on the user's same Cloudflare
account** (DNS managed by Cloudflare — i.e., nameserver move if the domain lives elsewhere) (a:
[public buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/); same constraint
for Workers custom domains, a: migration guide). Buys: real CDN caching for segments
(configurable cache rules), h2/h3, per-domain analytics, **no r2.dev-style rate limit**, and a
grown-up URL. Costs/traps: nameserver migration is the setup cliff; **cached 404s** can delay the
"on its way"→live flip (R4.5); user-enabled **Bot Fight Mode can silently kill unfurls** (R4.2);
delete latency: cached segments can outlive deletion until TTL/purge (a: R2 consistency doc) —
worth one sentence in the P2 tombstone story.

**Is there a Workers-route middle path for domain-less users?** Workers *routes* proper require a
zone (so: no), but the workers.dev sub-option above **is** the middle path in every way that
matters, and it's free. (a/d)

### R6.4 Recommendation & rejected alternatives

**Recommended:** zero-DNS default = Worker-proxied private bucket on workers.dev; upgrade tier =
custom domain on the user's zone with segments direct from the bucket domain (+ bucket CORS, +
cache-rule hygiene). State plainly in user-facing copy that the free hostname tier is
personal-scale; the custom domain is the "my links look like me and cache at the edge" upgrade,
not a reliability rescue.

- **Rejected: r2.dev as the shipped default** — reasons in R6.3; it fails the "honestly worded"
  bar (P4) since Cloudflare's own docs disclaim it, even though it measures fine today.
- **Rejected: requiring a custom domain (killing the zero-DNS persona)** — contradicts the
  rethink's P4 decision and the C-persona ambitions; the workers.dev path removes the need.
- **Rejected: third-party CDN in front of r2.dev** — new vendor, new config surface, against the
  Cloudflare-shaped-product acceptance (§2) and BYO simplicity.

**Cheap probes that would settle the residual unknowns:** (1) run the same curl battery from 2–3
distant vantages (a friend's machine / a free latency-test service) against the POC URLs —
settles global TTFB; (2) a 30-minute soak: 3 looping simulated players against r2.dev vs a
workers.dev proxy — settles whether "variable throttling" ever shows at real volumes; (3) the
Chrome-playback CORS check (R4.7) — one browser tab.

---


## R7 — Wizard without wrangler: full REST provisioning

> Researched **2026-07-20** — the positioning decision (minimal no-server extreme; part of
> the stack decision, 2026-07-20) unblocked the "waits for a direction" deferral. The direction
> also *shrank* the question: no Pages (R4 killed per-video deploys), no r2.dev public-access
> enable (R6 rejected r2.dev), no KV/D1 (R10's bucket-only layout). What remains to provision:
> **one private bucket, one Worker, one workers.dev hostname, one token.** POC-LEARNINGS §6 is
> treated as settled ground truth. **Same-day update: probe P3 (the wizard, hand-run) was
> executed live with Bagrat and passed in full — results in R7.6; the load-bearing claims
> below are now (b)-grade.**
>
> Claim labels: **(a)** verified from official docs [link] · **(b)** POC empirical ground truth
> (POC-LEARNINGS §6) · **(c)** community-reported · **(d)** inference.

### TL;DR verdict

- **Yes — the whole setup is ~6 REST calls from a compiled app. No wrangler, no node, no CLI,
  no bundler.** Verify token → resolve account → create private bucket → one multipart PUT
  deploys the Worker *with its R2 binding and HMAC secret in the same call* → ensure the
  account's workers.dev subdomain → flip the script's subdomain route on. Every endpoint is
  documented public REST (a).
- **The POC's "programmatic token minting needs a dashboard bootstrap anyway — not worth
  wiring" verdict inverts — because no minting is needed at all.** For any R2-scoped API
  token, **S3 Access Key ID = the token's `id`, Secret Access Key = SHA-256 of the token
  value**, for both user and account tokens (a:
  [R2 authentication](https://developers.cloudflare.com/r2/api/tokens/)). The pasted token
  *is* the S3 credential; the POC's "dashboard shows derived keys once" dance is dead.
- **One paste, pre-filled: officially supported.** Cloudflare documents dashboard deep links
  that pre-fill the token-creation form with an exact permission set (a:
  [token template URLs](https://developers.cloudflare.com/fundamentals/api/how-to/account-owned-token-template/)).
  The wizard shows one URL (auditable in the address bar — OSS-trust bonus), the user clicks
  Create, pastes the result. Minimal set for the zero-DNS tier: **Workers Scripts Edit +
  Workers R2 Storage Edit**, account-scoped, nothing else (a:
  [permissions reference](https://developers.cloudflare.com/fundamentals/api/reference/permissions/)).
- **The no-R2-OAuth-scope wall fell in June 2026.** Self-managed OAuth clients now expose the
  *API-token permission catalog* as scopes, with PKCE public clients — no client secret, a
  desktop app qualifies (a:
  [changelog](https://developers.cloudflare.com/changelog/post/2026-06-03-public-oauth-clients/),
  [create an OAuth client](https://developers.cloudflare.com/fundamentals/oauth/create-an-oauth-client/)).
  A zero-paste in-app flow is now *possible* — but it hangs a shroom-registered OAuth client
  (permanent once public, domain-verified) into every setup, its token TTL/refresh is
  undocumented, and an OAuth access token almost certainly doesn't satisfy the S3 derivation
  rule (d) — so the flow would still have to mint a durable token afterward. **Door, not
  baseline** (probe P5).
- **The POC's classified-error state machine transfers verbatim** (b): `10000` auth,
  `8000077` email-unverified, needs-payment on R2 enable; the two dashboard gates (email
  verify + R2 credit card) remain the only human steps besides the paste. New states to add:
  workers.dev-subdomain-taken; per-call insufficient-scope.

### R7.1 The call sequence, concretely

Zero-DNS tier (the default), in order, all against `api.cloudflare.com/client/v4` with
`Authorization: Bearer <pasted token>`:

| # | Call | Purpose | Notes |
|---|---|---|---|
| 0 | `GET /user/tokens/verify` or `GET /accounts/{id}/tokens/verify` | token sanity, status, expiry | account tokens are `cfat_`-prefixed (a: [account tokens](https://developers.cloudflare.com/fundamentals/api/get-started/account-owned-tokens/)) — classify the paste by prefix, verify accordingly (a: [user verify](https://developers.cloudflare.com/api/resources/user/subresources/tokens/methods/verify/), [account verify](https://developers.cloudflare.com/api/resources/accounts/subresources/tokens/methods/verify/)) |
| 1 | `GET /accounts` | resolve the account id | expected to work with any account-scoped token (d — probe P1) |
| 2 | `POST /accounts/{id}/r2/buckets` `{name}` | the private bucket | (a: [bucket create](https://developers.cloudflare.com/api/resources/r2/subresources/buckets/methods/create/)); `already_exists` counts as success (b) |
| 3 | `PUT /accounts/{id}/workers/scripts/{name}` | deploy the player/proxy/oEmbed Worker | multipart: metadata `{main_module, compatibility_date, bindings}` + one ESM module part (a: [multipart metadata](https://developers.cloudflare.com/workers/configuration/multipart-upload-metadata/)). Bindings carry everything: `{type:"r2_bucket", bucket_name}` + `{type:"secret_text"}` for R4's HMAC id-key — no separate secrets call, no storage credentials in the Worker, ever |
| 4 | `GET` / `PUT /accounts/{id}/workers/subdomain` `{subdomain}` | the account's `*.workers.dev` name | user-visible name choice; taken-name error shape undocumented (a: [subdomains](https://developers.cloudflare.com/api/resources/workers/subresources/subdomains/); probe P2) |
| 5 | `POST /accounts/{id}/workers/scripts/{name}/subdomain` `{enabled:true, previews_enabled:false}` | turn the route on | (a: [script subdomain](https://developers.cloudflare.com/api/resources/workers/subresources/scripts/subresources/subdomain/)) |
| 6 | S3 `HeadBucket` + tiny `PutObject`/`DeleteObject` via SigV4 | prove the locally-derived S3 keys | keys = token id + SHA-256(token value) (a); reuses the POC's zero-dependency SigV4 client (b) |

**Custom-domain tier adds two calls** (domain must be a zone on the same account — R6 (a)):
`POST /accounts/{id}/r2/buckets/{bucket}/domains/custom` `{domain, zoneId, enabled, minTLS}`
for direct segment delivery (a:
[R2 custom domain](https://developers.cloudflare.com/api/resources/r2/subresources/buckets/subresources/domains/subresources/custom/methods/create/))
and `PUT /accounts/{id}/workers/domains` to put the player Worker on the domain (a:
[Workers domains](https://developers.cloudflare.com/api/resources/workers/subresources/domains/);
exact body fields to confirm in P3). The tier's token needs zone-level permissions on top —
its template URL is a superset of the zero-DNS one (d).

**Design consequence for the stack session: the Worker must stay one dependency-free ESM
file.** Provisioning has no bundler, so the player/proxy/oEmbed Worker is authored as a single
module; player assets (hls.js, fonts, poster fallbacks) live **in the bucket** and ride the
existing `/m/` proxy route — which also fulfills the POC wishlist item "serve hls.min.js from
R2." Workers *static assets* would work too but cost an extra upload-session dance
(session-create + manifest + completion JWT) for nothing v1 needs (d) — rejected below.

### R7.2 The paste UX, honestly priced

- The wizard shows a **pre-filled token-template URL**; the dashboard opens with name and
  permissions filled; the user reviews, clicks through resource scoping, creates, copies,
  pastes. **~4 clicks + one paste, once per account, ever.** Template URLs exist for both
  user tokens (`dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=…&name=…`) and
  account tokens (`dash.cloudflare.com/?to=/:account/api-tokens&…`) (a: template URLs doc).
- **Prefer the account-owned token**: it's a service principal, not a person — it survives
  the person's role changing, and it's the natural shape for R10's v2 team model. Creating
  one requires the user to be Super Admin of the account (a) — true by construction for
  personas A/B on their own accounts. `cfat_` prefix makes it self-identifying (a).
- The template `key` slugs for the two permission groups aren't printed in the docs; they're
  discoverable once via `GET /user/tokens/permission_groups` (a:
  [create tokens via API](https://developers.cloudflare.com/fundamentals/api/how-to/create-via-api/))
  — probe P4 captures them, then they're constants in the wizard.
- The paste lands in `~/.shroom/credentials.json` mode 600, exactly as the POC did (b);
  nothing else ever holds it (the Worker gets bindings, not keys).

### R7.3 The OAuth door (new since June 2026, unproven)

What exists (a): any Cloudflare account can register an OAuth client; scopes mirror the token
permission catalog; Authorization Code + PKCE only (no device flow, no client-credentials);
PKCE public clients need no secret; clients are private-by-default, public requires DNS-TXT
domain verification and **public is permanent**. What it would buy: setup with *zero* paste —
in-app browser, consent screen, done. Why it is not the baseline: (1) it makes a
Bagrat-registered artifact (the client) load-bearing in every user's setup — forks would
re-register their own client id (config, not code, but a real asymmetry); (2) access-token
TTL/refresh semantics are undocumented — a background upload queue needs long-lived
credentials; (3) the S3 derivation rule is defined for API tokens, not OAuth access tokens
(d), so the flow would end by minting a durable account token anyway — meaning the OAuth
consent must carry *token-minting* permission, a **wider** grant than the two-permission
paste. Re-evaluate after P5 with real TTL/scope data, as a v2.x polish.

### R7.4 What transfers from the POC map, what dies

- **Transfers (b):** the classified-error state machine and its idempotent
  classify → fix → auto-poll → retry loop; `already_exists` = success; error codes `10000`
  (auth) and `8000077` (email-unverified, match the numeric code); surfacing both dashboard
  gates (email verify, R2 credit-card enable) in one browser trip; the zero-dependency SigV4
  client (now also deriving its keys locally); mode-600 credential storage.
- **Dies, happily:** wrangler entirely — and with it the OAuth-scope dance, the multi-path
  config shadowing / freshest-token bug, the `pages project create` CLI guard, and node as a
  setup dependency. Pages deploys die with it (R4), which also retires the **LAN-MTU black
  hole's only known trigger** (the Pages asset-upload endpoint; R2 uploads were always clean)
  (b) — the 90 s-timeout-and-fallback pattern survives as belt-and-suspenders, not as a
  known-necessary shield.

### R7.5 Probes

- **P1 ✅ (2026-07-20):** `GET /accounts` resolves the account under just the two
  permissions; verify endpoints calibrated (see R7.6 — the routing gotcha).
- **P2 ✅ (2026-07-20):** taken subdomain name → code **10031**; no-subdomain-yet account →
  code **10007** on GET; `PUT` succeeds under the minimal token (no dashboard trip needed).
- **P3 ✅ (2026-07-20):** executed live, passed in full — R7.6.
- **P4 ✅ (2026-07-20):** settled by observation — the account-token template deep link
  pre-fills with `workers_scripts` + `workers_r2` (rendered "Workers Scripts Write, Workers
  R2 Storage Write" in the dashboard form).
- **P5 ✅ (2026-07-20):** executed live, full results in **R7.7a** — scope catalog captured
  (371 scopes; `workers-r2.write` present — the old wall confirmed gone), access TTL
  measured at 16 h, refresh requires `offline_access` and rotates per use, and the S3
  derivation rule is **structurally impossible** for OAuth tokens (no token id exists).
  The paste-baseline decision (R7.7) stands, now (b)-grade.

### R7.6 P3 executed live (2026-07-20) — the wizard, hand-run (b)

Fresh account-owned token via the pre-filled deep link → the full call sequence → the
composed streaming loop → full teardown, on the main account, ~10 minutes end to end.
Facts captured, all (b):

- **The deep link pre-fills for real** — form opened with name + both permissions set from
  the guessed slugs. One paste later the wizard had everything it ever needs.
- **Verify must route by token prefix:** `GET /user/tokens/verify` answers **`1000` Invalid
  API Token** for a healthy `cfat_` account token; `GET /accounts/{id}/tokens/verify`
  returns `active`. A naive wizard would misdiagnose a good paste — classify by `cfat_`
  first, verify second. (Account id needed for account-verify comes from `GET /accounts`,
  which works with the token itself — no chicken-and-egg.)
- **Fresh-account subdomain reality:** this account had no workers.dev subdomain — GET →
  **10007** (a wizard state the docs never mention); `PUT {"subdomain": …}` created one
  under the minimal token. The name is an account-wide keeper (every zero-DNS link lives
  under it) — the wizard must present it as such. Probe used a throwaway and `DELETE`d it;
  account restored to pristine.
- **Bucket create:** instant, auto-placed **EEUR** with no `locationHint` — the wizard
  never asks a region question.
- **One-PUT deploy confirmed:** module + `r2_bucket` binding + `secret_text` binding in a
  single multipart PUT. Gotcha: the deploy response echoes `"bindings": []` even when
  bindings applied — don't validate against the echo; probe the Worker instead.
- **workers.dev propagation ≈ 40 s** on a fresh subdomain before first 200 — the wizard
  needs a calm said-line wait here, not a timeout failure. Serves **HTTP/2** (r2.dev is
  h1-only — R6).
- **S3 derivation rule CONFIRMED:** Access Key = token id, Secret = SHA-256(token value)
  signed PUT/HEAD/GET roundtrips against `<account>.r2.cloudflarestorage.com` via the POC
  SigV4 client, unchanged.
- **The composed v1 loop, working:** the S3-written object was immediately visible through
  the Worker's R2 binding (`list`) and served by the `/m/` proxy route on workers.dev —
  R6's delivery + R7's provisioning + R10's one-bucket-truth, running together.
- **Teardown clean:** object (S3 DELETE, 204), Worker, bucket, subdomain — all removed by
  the same token; the account ended exactly as it began.

### R7.7a P5 executed live (2026-07-20) — the OAuth door, probed (b)

Private client `shroom-p5-probe` on the main account; PKCE public-client flow from a
local zero-dep script; full teardown (tokens revoked, effect verified immediate). All (b):

- **Client creation UX** (Manage Account → OAuth clients): "None (PKCE)" is a first-class
  auth method — no forced secret; `http://localhost:<port>/callback` accepted as a
  redirect; refresh is a grant-type option; **no token-TTL configuration exists anywhere**;
  private-by-default is structural — there is no visibility toggle, clients *become*
  public-eligible only by filling the optional-required fields (client URL, logo, …) +
  domain verification.
- **Scope catalog** (`GET /client/v4/oauth/scopes`, needs any valid token): **371 scopes**,
  ids mapping 1:1 to token permission names. `workers-scripts.write` and
  `workers-r2.write` both present (plus finer `workers-r2-bucket-item.write`) — **the old
  no-R2-OAuth-scope wall is confirmed dissolved** at the catalog level.
- **Consent screen**: scopes grouped human-readably by category ("Developer Platform (2)");
  an **account picker** (multi-account users choose the target — wizard-relevant); a red
  unverified-app shield for private clients; revocation path stated on-screen (My Profile
  → Access Management → Connected Applications).
- **Tokens**: access token `cfoat…` (93 ch), **TTL `expires_in` 57600 s = 16 h** (the
  undocumented number, measured). **No refresh token by default** even with the refresh
  grant type configured — the **`offline_access` scope is the trigger**, accepted
  silently and **not surfaced on the consent screen** (durable access is invisible to the
  consenting user — a transparency ding worth remembering). Refresh token `cfort…`
  (93 ch); **rotates on every use** (single-use — a background queue must persist the new
  one atomically or a crash between refresh-and-persist forces re-consent). Refresh
  absolute lifetime: unmeasurable in-session, still undocumented (residual).
- **API behavior**: `cfoat` works for real calls (`/accounts` 200) but **both** verify
  endpoints answer `1000 Invalid API Token` — R7.6's routing gotcha completes into a
  three-way rule: user tokens → user-verify; `cfat_` → account-verify; `cfoat`/`cfort` →
  **no verify endpoint exists at all**; classify by prefix, probe by use.
- **S3 derivation: structurally impossible for OAuth tokens.** The rule needs a token id
  (Access Key = id); no verify endpoint yields one, and client-id-as-access-key gets 401
  from R2's S3 endpoint. R7.3's (d)-grade inference is now (b): **the OAuth door cannot
  carry the S3 credential story** — it would have to end by minting a durable API token,
  i.e. a consent *wider* than the two-permission paste.
- **Revocation**: `POST /oauth2/revoke` works for both token types; effect immediate
  (`/accounts` 200 → 403).
- **Verdict**: the door is mechanically real and well-made (16 h + rotating refresh is
  fine for an interactive app), but derivation-dead-end + offline_access consent opacity +
  the private/public asymmetry (public = permanent, domain-verified, shroom-owned
  registration) keep it a v2.x polish door. **R7.7's paste-baseline decision stands,
  upgraded to (b).**

**Mint-through-OAuth follow-up (same day; the last variant, closed platform-side).**
Bagrat's product judgment — single-consent UX may be worth the wider scope + public-client
registration — sent us to probe OAuth → `POST /accounts/{id}/tokens` (mint the durable
two-permission token, then derive as usual). The probe died at step zero, decisively:
**Cloudflare's OAuth scope catalog contains no token-management scope at all.** Exhaustive
sweeps of the captured 371-scope catalog for `token`/`api`/`account`/`user`/`member`
(id + name, case-insensitive) surface nothing but Zero-Trust "Access: Service Tokens" —
no equivalent of "API Tokens Edit" or "Account API Tokens Write" exists as an OAuth
scope, and clients can only request scopes drawn from this same catalog (the client
form's scope picker IS it). Reading: deliberate anti-escalation design — an OAuth grant
is bounded to resource scopes and can never be parlayed into durable, arbitrary-permission
credentials. Consequence: **the paste is not merely shroom's preferred mechanism; it is
the only mechanism the platform permits for a third-party app to end up holding durable
credentials.** The single-consent UX is unreachable regardless of what shroom is willing
to pay for it. Reopening condition: Cloudflare ships a token-management OAuth scope
(watch the catalog — it will grow; the check is one authenticated GET).
**Side-gate test (same day, Bagrat's challenge, (b)):** could the broad
`account-settings.write` scope cover token management anyway? No — with it granted and
echoed in the token's scope, both `GET …/tokens/permission_groups` and
`POST /accounts/{id}/tokens` answer **403 code 9109** ("Unauthorized to access requested
resource"). The catalog omission is enforced at the API layer, not a labeling gap; the
closure is total, catalog-side and API-side. (Consent-screen note: the three-permission
ask incl. "Account Settings Write" rendered without extra warnings — the UX would have
been fine; the platform is what says no.)

**Temp-creds addendum (same day, follow-up probe; settles R10.4 #2 in passing).** The
question: could `POST /accounts/{id}/r2/temp-access-credentials` give OAuth a Worker-free
S3 story? Measured (b):

- **Happy path (cfat_ parent)**: mint works with `{bucket, parentAccessKeyId,
  permission: "object-read-write", ttlSeconds, prefixes}`; **`ttlSeconds: 604800` (7 d)
  accepted** (effective lifetime unverified in-session); response = `accessKeyId` (echoes
  the parent key id) + 64-hex secret + a ~600-char `sessionToken`. **Prefix scoping is
  enforced**: PUT/GET/DELETE inside the prefix 200/200/204, PUT outside **403**. SigV4
  needs only `x-amz-security-token` as a fourth signed header — a three-line extension to
  the POC signer, verified working.
- **Cross-class semantics (the surprise)**: minting *authorization* and parent *identity*
  are independent. A `cfoat` bearer with `workers-r2.write` **can drive the mint** (HTTP
  200) — but only by naming an **existing API-token-backed R2 access key** as parent; an
  empty parent gets 401/10002, and a cfoat token can never self-parent (it has no id).
  The cfoat-minted temp creds are fully functional (same roundtrip + enforcement).
- **Consequences**: (1) **pure-OAuth setup stays credential-dead** — no path on any leg
  (derivation, direct use, temp-creds) avoids first creating an API token, so the
  wider-consent conclusion is now closed from every direction; (2) independently of
  OAuth, **temp-creds are a live blast-radius option for the paste world**: the recorder
  could hold only short-lived, bucket+prefix-scoped creds minted at record-start from the
  pasted parent — no durable S3 secret at rest, no Worker in the write path (which Bagrat
  ruled out same day: writes always go direct to R2). A stack-session design option, not
  a decision — the durable derived key remains the offline-friendliest baseline (temp
  creds expire; a long-offline queue must re-mint at drain time, which requires being
  online anyway — mild, but it's a moving part the derived key doesn't have).

### R7.7 Recommendation & rejected alternatives

**Recommended:** a deterministic in-app wizard speaking raw REST. One pre-filled-URL paste
(account-owned token; Workers Scripts Edit + Workers R2 Storage Edit), the six-call sequence,
the POC error state machine, S3 keys derived locally from the same token. The BYO-agent setup
skill becomes a thin narrator over the identical call sequence — one implementation, two
front doors.

- **Rejected: embedding or driving wrangler.** The POC paid this bill in full — config
  shadowing, freshest-token bug, CLI-only guards, a node runtime as a hard dependency — and
  every operation the wizard needs is now plain REST (b). Wrangler bought nothing but its
  OAuth client, which the token paste replaces.
- **Rejected: OAuth as the baseline flow.** One month old, undocumented token lifetime, a
  permanent shroom-registered client in the chain, and it still ends in token-minting with a
  wider consent than the paste it replaces (R7.3). Door stays open via P5.
- **Rejected: Global API Key / legacy email+key auth.** All-powerful by construction; fails
  least-privilege and the OSS-trust posture outright.
- **Rejected: dashboard-created S3 keys (the POC flow).** Obsoleted by the derivation rule —
  one more dashboard screen and one more copy-paste for zero benefit (a).
- **Rejected: paste-broad-then-mint-narrow token laundering.** Least-privilege theater: the
  two-permission token is already minimal for what the app keeps doing (Worker updates on
  app update, object writes on every recording), and minting would require adding API-Tokens
  Edit to the paste — a *wider* ask. Revisit only if R10's v2 team onboarding wants delegated
  minting.
- **Rejected: Workers static assets for player assets.** Adds the assets-upload-session
  dance to provisioning; the bucket + `/m/` proxy already serve the same bytes with zero
  extra setup surface (d).

---


## R10 — Minimal multi-user auth on BYO-backend + the v1 layout that survives it

> Researched 2026-07-19. Doc facts checked against developers.cloudflare.com as of this date
> (R2 pricing page last modified 2026-05-28). Claim labels: **(a)** verified from official
> docs, **(b)** community-reported, **(c)** inference. P12 (two Macs, one library, v1) made
> the layout half of this urgent; the auth half remains v2 design-ahead.

**TL;DR verdict**

- **v1 needs no Worker, no Durable Object, and no coordination service in the write path.**
  Per-recording objects under unguessable-id prefixes + list-on-read is lost-update-free *by
  construction* for the two-Mac case, because R2 is strongly consistent for reads, writes,
  deletes **and lists** (a). The only shared mutable object is each recording's own
  `meta/<id>.json`, which has a single natural writer.
- **The v2 auth spine already exists as a Cloudflare primitive: R2 temporary access
  credentials** — short-lived S3 credentials derived from a parent R2 token, **scopable to a
  bucket + key prefixes/objects + permission level + TTL**, mintable by a Worker either via
  one API call or **locally by signing a JWT** (no Cloudflare API round-trip) (a). Teammates
  never hold real keys; revoking the parent kills every derived credential instantly (a).
  This beats presigned-PUT-per-object (too chatty for HLS's hundreds of segments) and
  per-user permanent R2 tokens (bucket-scoped only → teammates could clobber each other).
- **Never put a fixed-key index object in the public bucket.** Beyond lost updates, it's an
  *enumeration leak*: public buckets serve any key read-only (listing is disabled) (a), so a
  guessable `index.json` would hand out every video id and break unlisted-by-obscurity.
  Per-recording metas inherit the id's unguessability. This kills the central-index option
  independently of concurrency.
- **Viewing stays auth-less. Nothing in the write-auth design touches the read path** —
  media is served read-only from the public bucket, the player Worker reads metadata via its
  R2 binding, and the P9 "page always exists" state machine (pending / live / tombstone)
  works with zero writes at link-copy time. P1 confirmed, not refuted.
- **Concrete deliverable below:** `meta/<id>.json` + `rec/<id>/…` layout, JSON metadata
  objects (not R2 custom metadata), client-minted 96-bit ids claimed with create-only
  conditional PUTs (`If-None-Match: *`, supported by R2's S3 API (a)/(b)), tombstone =
  `meta` rewritten to `state:"deleted"`. v2 adds credentials and an `owner` field; **no key
  moves, no migration.**
- Two cheap probes would de-risk the whole thing: (1) conditional PUT semantics via the
  POC's SigV4 client, (2) minting a prefix-scoped temp credential and verifying it can't
  write outside its prefix. ~20 minutes total against a scratch bucket.

---

### R10.1 Verified platform facts this design stands on

| Fact | Status | Source |
|---|---|---|
| R2 is strongly consistent globally for reads, writes, deletes, **and list** ("the list operation will list all objects at that point in time"); concurrent writes to one key are last-writer-wins | (a) | [Consistency model](https://developers.cloudflare.com/r2/reference/consistency/) |
| Token/permission changes are the exception: up to ~1 min to propagate | (a) | same page |
| Concurrent writes to the *same key* limited to 1/sec; object key ≤1,024 B; custom metadata ≤8,192 B/object; 5 GiB single PUT / 4.995 TiB multipart; up to 1,000,000 buckets/account; Cloudflare REST API (not S3 API) limited to 1,200 req/5 min | (a) | [R2 limits](https://developers.cloudflare.com/r2/platform/limits/) |
| Public buckets (r2.dev or custom domain) expose **read-only GET; listing is disabled** ("public buckets do not let you list the bucket contents") | (a) | [Public buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/) |
| Presigned URLs: GET/HEAD/PUT/DELETE, expiry 1 s–7 days, scoped to **one operation on one object**, bearer semantics, S3-endpoint only (never custom domains); Content-Type can be pinned into the signature | (a) | [Presigned URLs](https://developers.cloudflare.com/r2/api/s3/presigned-urls/) |
| Presigning works from a Worker with `aws4fetch` (`sign(..., {signQuery:true})`), officially documented | (a) | [aws4fetch example](https://developers.cloudflare.com/r2/examples/aws/aws4fetch/) |
| **Temporary access credentials**: derived from a parent R2 API token; bound to exactly one bucket; scopable by `permission` (`object-read-write` etc.), `prefixes[]`, `objects[]`, `ttlSeconds`; returned as accessKeyId + secretAccessKey + sessionToken (`X-Amz-Security-Token`); revoking the parent token kills all derived credentials immediately | (a) | [Temporary credentials](https://developers.cloudflare.com/r2/api/s3/temporary-credentials/), [API: POST /accounts/{account_id}/r2/temp-access-credentials](https://developers.cloudflare.com/api/resources/r2/subresources/temporary_credentials/methods/create/) |
| Temp credentials can also be minted **locally** — sign an HS256 JWT with the parent token's secret access key, use it as the session token; per-action scoping (`GetObject`, `PutObject`, …) available on this path | (a) | [Temporary credentials](https://developers.cloudflare.com/r2/api/s3/temporary-credentials/) |
| TTL: `ttlSeconds` default 900, max 604,800 (7 days) | (a)/(b) — the max appears in the API reference; the docs page urges "shortest value that fits" | [API reference](https://developers.cloudflare.com/api/resources/r2/subresources/temporary_credentials/methods/create/) |
| R2 API tokens: Object R&W / Object Read tokens **scope to a set of buckets — no key-prefix scoping on permanent tokens**; S3 creds derive as accessKeyId = token `id`, secret = SHA-256 of token `value`; account-owned tokens creatable via API but require **Super Administrator** | (a) | [R2 auth/tokens](https://developers.cloudflare.com/r2/api/tokens/), [Account-owned tokens](https://developers.cloudflare.com/fundamentals/api/get-started/account-owned-tokens/) |
| **Conditional writes**: PutObject / CreateMultipartUpload accept `If-Match` / `If-None-Match` (incl. `*`), failing with `412 Precondition Failed`; Workers binding exposes the same as `onlyIf` | (a) for the header list on PutObject ([S3 compat](https://developers.cloudflare.com/r2/api/s3/api/), [Workers API](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/)); (b) for the 412-on-PUT semantics ([docs PR #5303](https://github.com/cloudflare/cloudflare-docs/pull/5303), [community thread](https://community.cloudflare.com/t/conditional-put-not-supported-in-r2/397626)) — **probe named below** |
| Pricing (Standard): storage $0.015/GB-mo; Class A (PutObject, ListObjects, …) $4.50/M; Class B (GetObject, HeadObject) $0.36/M; free tier 10 GB + 1 M class A + 10 M class B per month; egress $0 | (a), page modified 2026-05-28 | [R2 pricing](https://developers.cloudflare.com/r2/pricing/) |
| Workers: free 100k req/day + 10 ms CPU/invocation; paid $5/mo with 10 M req + 30 M CPU-ms included. KV is eventually consistent (**up to 60 s+**, negative lookups cached too). Durable Objects exist on the free plan (SQLite backend, 100k req/day). Request bodies capped at 100 MB (free/Pro account plans) | (a) | [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/), [How KV works](https://developers.cloudflare.com/kv/concepts/how-kv-works/), [Workers limits](https://developers.cloudflare.com/workers/platform/limits/) |

---

### R10.2 THE PROPOSED v1 LAYOUT (the deliverable)

One bucket, public access enabled (r2.dev or custom domain — R6's territory). All keys
below; nothing else is ever created.

```
meta/<id>.json                  ← the per-recording record. Small, mutable, authoritative.
                                   ALSO the tombstone (state:"deleted") after delete.

rec/<id>/hls/index.m3u8         ← playlist. Uploading it is the atomic media go-live (POC §5 law).
rec/<id>/hls/seg-00001.m4s …    ← segments. Immutable once written.
rec/<id>/<title-slug>.mp4       ← download tee (slug-in-key so browsers name the file, POC §5).
rec/<id>/poster.jpg             ← unfurl/poster image (best-effort).
rec/<id>/tx/captions.vtt        ← reserved; only exists if transcription runs bucket-side (P3).
rec/<id>/tx/transcript.json     ← reserved; word-timestamped transcript (P8 requires VTT-grade).

.shroom/manifest.json           ← bucket manifest: {"schema":1, "createdAt":…}. Written once by
                                   setup; wizard idempotence marker. RULE: fixed-key ⇒ world-
                                   readable in a public bucket ⇒ MUST stay secrets-free forever.
```

`<id>`: client-minted, `crypto.randomBytes(12).toString('base64url')` — 16 chars, 96 bits,
exactly the POC's mint (`archived-poc/scripts/recorder/preflight.mjs:165`). At 10k
recordings, collision odds ≈ 10⁻²¹ (c). The id is the security model (P1), so it stays
maximum-entropy and structure-free.

**`meta/<id>.json` schema (v1, `schema:1`):**

```jsonc
{
  "schema": 1,
  "id": "aB3xK9…",
  "state": "uploading" | "live" | "deleted",     // the P9/P2 state machine, one field
  "createdAt": "2026-07-19T10:00:00Z",
  "publishedAt": null | "…",
  "deletedAt": null | "…",                        // set when state:"deleted" (tombstone)
  "title": "Untitled" | "…",                      // the one core edit (rename = rewrite this file)
  "durationSec": null | 123.4,
  "media": {                                      // null while uploading; paths, not URLs
    "hls": "rec/<id>/hls/index.m3u8",
    "mp4": "rec/<id>/demo-flow.mp4" | null,
    "poster": "rec/<id>/poster.jpg" | null,
    "width": 2560, "height": 1440
  },
  "captions": null | { "vtt": "rec/<id>/tx/captions.vtt" },   // additive later (P8)
  "owner": null,                                  // RESERVED: v2 member id. v1 always null.
  "sender": null | { "name": "…", "avatarUrl": "…" },  // RESERVED: P5 unanswered. Optional forever.
  "origin": { "device": "personal-mac", "app": "0.2.0" }  // optional, debugging only
}
```

Schema rules: additive-only evolution; readers ignore unknown fields; `schema` bumps only
for breaking changes (design goal: never). **View counters are deliberately NOT in this
file** — v2 view tracking (Worker-written) gets its own store (`views/<id>` object or
KV/DO, R4's call), so `meta/<id>.json` keeps exactly one writer class: the owner's client
(later: the Worker acting for the owner). That single-writer rule is what makes LWW safe.

**Write choreography (v1, either Mac):**

1. Mint id locally. If online: `PUT meta/<id>.json` with `If-None-Match: *` and
   `state:"uploading"` — a create-only claim; a 412 means re-mint (never overwrite).
   If offline: skip — the link still works (see read seam below).
2. Segments upload opportunistically (POC uploader unchanged: idempotent PUTs, diff-and-resume).
3. Playlist `PUT` at finalize = media go-live (unchanged POC law: no half-written playlists).
4. Rewrite `meta` → `state:"live"`, duration, media block. Rename later = rewrite `meta`
   (optionally with `If-Match: <etag>`; plain LWW is acceptable for one human).
5. Enrichments (mp4, poster, transcript) land afterwards, each followed by a `meta` rewrite.

**Delete → tombstone (P2):** rewrite `meta/<id>.json` to
`{schema, id, state:"deleted", deletedAt, title?}` **first**, then list-and-delete
`rec/<id>/*`. If interrupted, the page already says "removed" and a later GC sweep
(compare `rec/` prefixes against metas) reaps orphans — crash-safe without transactions.
The tombstone is a ~100-byte object that answers forever.

**Library list (both Macs, no Worker):** one `ListObjectsV2 prefix=meta/` page returns up
to 1,000 metas **with their ETags** (a) — so the app GETs only metas whose ETag changed
since its local cache. Full cold refresh at 1,000 videos ≈ 1 class A + 1,000 class B ≈
**$0.00036** ([pricing](https://developers.cloudflare.com/r2/pricing/)); warm refresh ≈ 1
class A + a few GETs. List latency at this scale is not a documented number — inference (c):
one page, low hundreds of ms; probe below.

**Why this is multi-writer-safe with zero coordination (the P12 answer, high confidence):**
concurrent recordings from two Macs write **disjoint key sets** (fresh 96-bit ids); the
create-only claim makes even a collision harmless; list-on-read is strongly consistent (a)
so both Macs see each other's uploads immediately; the only key two writers could ever
share is one recording's `meta`, where LWW between one human's two machines is benign.
**No git, no index object, no Worker, no DO. The local git SoT is simply deleted, replaced
by the bucket itself.**

**The R4 seam (explicit):** the player Worker route `/v/<id>` reads `meta/<id>.json` via
its R2 binding and renders by `state`:
- object **absent** → "content on its way" pending page (this is what makes the link valid
  the instant it's copied, even when the Mac was offline at stop — P9). A bogus id renders
  the same pending page, which leaks nothing (indistinguishable by design).
- `state:"uploading"` → same pending page, richer (title if present).
- `state:"live"` → player + og tags from `media.*`, media fetched by the viewer's browser
  straight from the public bucket (Worker never proxies video bytes).
- `state:"deleted"` → friendly tombstone (P2).
R2 remains the source of truth; if R4 wants KV/cache for read latency it must be a derived
cache only (KV's 60 s+ eventual consistency (a) would otherwise break rename/tombstone
freshness guarantees).

#### Rejected layouts (house style: with reasons)

| Rejected | Why |
|---|---|
| **Keep local git SoT, sync it** | Dead by P12: a synced repo *is* a shared mutable index — lost updates on push races, plus git-on-two-machines is a coordination service by another name. The POC itself predicted this (POC §5 "Library/metadata" v2 note). |
| **Single `index.json` in the bucket** | (1) LWW lost updates between writers (a); (2) 1 write/sec/key limit (a); (3) **fatal even single-user: a fixed, guessable key in a public bucket is world-readable — it would enumerate every unguessable id and destroy the P1 model.** |
| **R2 custom metadata (`x-amz-meta-*`) as the metadata store** | 8,192-byte cap (a); invisible to S3-API list (values not returned); mutation requires a CopyObject metadata-directive dance; public custom domains would serve it as headers. Fine later as a *mirror* (the Workers binding's list can include customMetadata as a read accelerator) — never as the authority. |
| **Co-locate meta at `rec/<id>/meta.json`** | Respectable (one prefix per recording simplifies temp-cred scoping to a single `prefixes` entry). Loses the one-list-with-ETags delta sync over `meta/`, and delete becomes "remove everything under the prefix *except* meta.json" — an awkward invariant. The chosen split keeps `rec/<id>/` fully deletable and `meta/` cheaply scannable. Cost: v2 credentials name two scopes (`prefixes:["rec/<id>/"]`, `objects:["meta/<id>.json"]`) — directly supported by the API (a). |
| **Per-user key prefixes (`rec/<user>/<id>/`)** | Breaks v1→v2 key stability (v1 has no user), leaks team structure into public URLs, and buys nothing: prefix-scoped *temporary* credentials already isolate writers per-recording without encoding identity into keys. |
| **KV as the metadata store** | Eventually consistent, 60 s+ windows incl. negative lookups (a); moves the source of truth out of the user's bucket (the bucket-is-your-exportable-library is part of the BYO thesis); forces a Worker into the v1 write path. |
| **DO-mediated authoritative index** | Strong consistency R2 already provides (a); adds a stateful component, a deploy, and a SoT migration for zero v1 benefit. Revisit only if a library grows past many thousands and list latency measurably hurts, or v2 needs cross-recording atomic invariants (none identified). DOs do exist on the free plan now (a), so cost isn't the objection — necessity is. |
| **Timestamp-structured ids (ULID etc.)** | Sortability is worthless (createdAt lives in meta; list is over `meta/`), and embedding time in a URL that *is* the credential gives attackers structure for free. Max-entropy random wins. |

---

### R10.3 The unknowns, answered bullet-by-bullet

**1. Worker-minted presigned PUTs — feasible? per-user tokens? blast radius? revocation?**

Feasible and officially documented: a Worker holding the parent R2 token's S3 keys signs
SigV4 query-string URLs with `aws4fetch` (a)
([example](https://developers.cloudflare.com/r2/examples/aws/aws4fetch/),
[presigned docs](https://developers.cloudflare.com/r2/api/s3/presigned-urls/)). Each URL
grants **one method on one object** for 1 s–7 days, Content-Type pinnable; it's a bearer
token; only the S3 endpoint honors it. Per-user lightweight tokens: shroom-defined opaque
secrets whose hashes the Worker stores (KV or Worker config); the Worker validates the
bearer, then signs. Blast radius of a leaked URL: that single object until expiry.
Revocation: delete the member token (mint stops); already-minted URLs die at expiry — keep
them minutes-short.

**Verdict: feasible but demoted to secondary.** HLS uploads are *hundreds of small objects
per recording*; presigning each makes the Worker a chatty availability dependency in the
middle of recording and complicates the offline queue (batch-presign ahead? re-request on
drain?). Confidence: high. Use presigns only where a single-object grant is naturally
shaped (e.g., a future browser-based avatar upload).

**2. The better primitive research surfaced: R2 temporary access credentials (this is the
v2 recommendation).**

`POST /accounts/{account_id}/r2/temp-access-credentials` with
`{bucket, parentAccessKeyId, permission:"object-read-write", ttlSeconds, prefixes:[…],
objects:[…]}` returns a full S3 credential triple (key id, secret, session token) that any
S3 client uses with `X-Amz-Security-Token` (a). Better still, a Worker can mint them
**without any Cloudflare API call** by HS256-signing a JWT with the parent token's secret —
zero added latency, no 1,200-req/5-min REST limit exposure, and per-action scoping
(`PutObject` only) on this path (a)
([docs](https://developers.cloudflare.com/r2/api/s3/temporary-credentials/)).

v2 flow (design-ahead, not build): teammate's app authenticates to the Worker with its
member token → Worker create-only-claims `meta/<id>.json` with `owner` set (the atomic
ownership registration; 412 = id taken) → Worker mints a temp credential scoped to
`prefixes:["rec/<id>/"] + objects:["meta/<id>.json"]`, `object-read-write`, TTL sized to
the upload window (renewable for offline drains) → the client's existing SigV4 uploader
(POC `lib/s3.mjs`) uploads directly to R2, adding only the session-token header ((c) from
the SigV4 spec — trivial, and probe 2 verifies it). The Worker touches **one request per
recording**, not per segment; media bytes never transit it.

"Nobody can clobber others' videos" holds at the storage layer, not by politeness: the
credential physically cannot write outside its recording's prefix (a).

**3. Per-user permanent R2 API tokens instead?**

- *Mintable programmatically from an account token?* Yes — account-owned tokens have a
  creation API — but it requires **Super Administrator** on the account (a)
  ([docs](https://developers.cloudflare.com/fundamentals/api/get-started/account-owned-tokens/)),
  and something must hold a token-minting token, whose blast radius (mint arbitrary
  tokens) dwarfs a bucket-scoped parent's.
- *Bucket- or prefix-scoped?* **Bucket-scoped only.** Object R&W tokens "can scope your
  token to a set of buckets"; no prefix scoping exists on permanent tokens (a)
  ([docs](https://developers.cloudflare.com/r2/api/tokens/)). So every teammate token could
  clobber every video — fails the core requirement inside one bucket. One-bucket-per-user
  would fix scoping (1 M buckets allowed (a)) but shatters the single shared library:
  the player Worker binds buckets statically, list spans buckets, delete/GC multiplies.
- *Offboarding?* Clean in isolation (delete the token; S3 creds die; ≤1 min propagation
  (a)) — but it requires Cloudflare-API access at offboarding time, i.e. the admin's
  high-privilege credential again.

**Verdict: rejected** (confidence: high). Everything it offers, temp credentials offer
with narrower scope and a smaller standing secret.

**4. Index without lost updates — and does v1 need the Worker in the write path?**

Answered by the layout (R10.2): per-recording objects + list-on-read, no index object
anywhere, ever — for consistency (LWW races), for the 1-write/sec/key limit, and for the
enumeration leak. Cost and consistency verified: strong list-after-write (a), ~$0.0004 per
cold 1,000-video refresh (a-derived). **v1 works with plain per-recording objects and no
Worker at all in the write path** (high confidence). The Worker v1 *does* have is the R4
read-path player route — mandated by P9 independently of anything here. v2 *adds* the
Worker to the write path solely as credential-minter and ownership-registrar; the index
story doesn't change (a Worker-side list cache is an optional read optimization, never
truth).

**5. Does any of this force auth into the view path (P1)?**

**No — confirmed.** Public bucket access is read-only GET with listing disabled (a), so
exposure stays exactly "whoever holds an unguessable id." The Worker reads meta via
binding; viewers fetch media directly; temp credentials and member tokens exist only on
the write side. Tombstone and pending states render without any write at view time. One
honest consequence to record (P3): anything bucket-resident under `rec/<id>/tx/` is
readable by link-holders — if transcription runs on the user's Cloudflare, the transcript
is public-to-link-holders, exactly as the rethink's P3 anticipated ("follows the
engine"). And two hygiene rules the layout already encodes: fixed-key objects
(`.shroom/manifest.json`) must stay secrets-free, and the v2 member registry (names, token
hashes) must live Worker-side (KV/DO/secrets), never in the public bucket.

**6. Revocation & blast-radius matrix (v1 and v2):**

| Credential | Held by | Scope | If leaked | Revocation |
|---|---|---|---|---|
| Parent R2 token (Object R&W, one bucket) | v1: each Mac (recommend **one token per machine**); v2: Worker secret only | whole bucket, read+write | overwrite/delete anything in the bucket (never account-wide) | delete/roll in dashboard or API; kills all derived temp creds **immediately** (a); ≤1 min propagation (a) |
| Temp credential | teammate app, per recording | `rec/<id>/` + `meta/<id>.json`, TTL-bounded | scribble inside one recording until TTL | expires by itself; parent roll kills it now |
| Presigned URL | whoever holds it | one object, one method, ≤7 d | that one object | expiry; parent roll |
| Member token (v2, shroom-defined) | teammate config | "may ask the Worker" | mint new recordings + touch own recordings until deleted | delete its hash from Worker storage (KV: ≤60 s+ lag (a); use a DO or Worker env if instant matters) |

v1 recommendation: mint **two** dashboard R2 tokens at setup (work Mac / personal Mac) so a
lost laptop is offboarded by deleting one token without re-keying the other. Cheap, and it
rehearses the v2 posture.

**7. What v2 adds (and what it doesn't touch):**

Adds: `owner` values in meta (field already reserved), the Worker write-path endpoints
(claim + mint + rename-for-owner), member registry in Worker storage, temp-cred signing.
Touches **zero** existing keys, key shapes, meta fields, or the read path. That is the
"survives v2 unchanged" claim, and it's structural, not aspirational.

---

### R10.4 Probes (Bagrat-style: settle by doing, ~20 min total, scratch bucket)

1. **Conditional PUT** (the one (b)-grade load-bearing fact): with the POC SigV4 client,
   `PUT` a scratch key twice with `If-None-Match: *` — expect 200 then 412. Also probe an
   `If-Match: <etag>` rewrite. 30 seconds; upgrades the claim to (a)-equivalent.
2. **Temp credentials end-to-end**: dashboard R2 token → `POST
   /accounts/{id}/r2/temp-access-credentials` scoped to `prefixes:["rec/test1/"]` → PUT
   inside the prefix (expect 200) and outside it (expect 403) using the POC client + the
   `x-amz-security-token` header in the signature. This single probe validates the entire
   v2 auth spine and the small signer change.
   **✅ EXECUTED 2026-07-20 (b), passed exactly as designed** — parent = the *derived* key
   (no dashboard token needed), prefix enforcement confirmed (200 in / 403 out), the
   4th-signed-header signer change works, 7-day `ttlSeconds` accepted. Full results in
   R7.7a's temp-creds addendum.
3. **Local JWT mint**: repeat probe 2 minting locally (HS256 over the parent secret) — the
   no-API-call path the Worker would actually use.
4. **List at scale**: script 5–10k tiny `meta/` objects into the scratch bucket; time
   `ListObjectsV2` pages from the laptop and from a Worker. (Baseline available today for
   free: list the POC's real bucket.)
5. **Same-key write pressure**: hammer one key at >1 write/sec from two clients; observe
   whether R2 429s, errors, or silently LWWs — documents the rename-race worst case.

---


> Researched 2026-07-19. Pricing/limits carry their as-of date; claims labeled
> **(a)** verified from official docs, **(b)** community-reported, **(c)** inference /
> back-of-envelope (arithmetic shown). Ground truth from POC-LEARNINGS §2/§3/§5 is taken
> as given, not re-researched. Rethink decisions honored: P10 (manual title fine; price
> both doors), P8 (VTT-grade timestamps are a HARD requirement), P3 (privacy follows the
> engine — stated per option), P6 (unbounded length — engines judged on 2 h+).

---

## R8 — Transcription engine matrix (feeds title, captions, search)

**TL;DR verdict**

- **The cheap door P10 asked about is Apple-local, not Cloudflare.** SpeechAnalyzer /
  SpeechTranscriber (macOS 26+) is zero-install, zero-blob, on-device, ~45× realtime,
  emits per-word timing — and Bagrat's own machine (Darwin 25 = macOS 26 Tahoe) can run it
  today. It is the natural v1 default engine.
- **Its two real gaps: OS floor and languages.** macOS 26+ only (~50–70 % of active Macs
  mid-2026, growing) and ~10 languages / ~30 locales — Armenian/Russian likely absent.
  whisper.cpp (downloaded-on-demand, SHA-256-pinned) is the honest fallback for both, and
  is also the only engine that ports to Windows/Linux later (P15 door).
- **Workers AI Whisper is genuinely cheap** — $0.0005/audio-minute ≈ **$0.03 per hour of
  video**, free tier ≈ 3.5 h/day — but the *engineering* isn't: audio must be chunked to
  ~1 MB (no official limit documented; ~2 MB requests fail), and per-chunk VTT must be
  offset-stitched. Real door, best opened when the R4 Worker/BYO-backend exists anyway.
- **P8 compliance inverts intuition:** word-level timestamps are *strongest* on Apple
  (per-run `audioTimeRange`) and Workers AI (`words[]` in the schema), *weakest* on
  whisper.cpp (approximate `-ml 1`/`--split-on-word`; `--dtw` still experimental).
  Segment-grade VTT is solid on all engines.
- **The title/chapter LLM has a home in every option** — BYO-agent skill (local engines),
  a Workers AI text model on the user's account (~$0.0002/title, free-tier-covered), or
  nowhere (P10's manual default). No option forces shroom to operate or bundle an LLM.
- **Recommendation:** v1 = SpeechAnalyzer default, fail-soft to "no transcript" below
  macOS 26; whisper.cpp as opt-in fallback (non-Apple languages / older Macs); Workers AI
  deferred until BYO-backend lands. Emit an engine-agnostic transcript artifact
  (segments + optional words + `.vtt`) so engines stay swappable.

### The matrix

| | **Apple SpeechAnalyzer / SpeechTranscriber** | **whisper.cpp** (download-on-demand) | **Workers AI Whisper** (user's own CF) | **Workers AI Deepgram nova-3** (reference) |
|---|---|---|---|---|
| **Floor / requirement** | macOS 26+ only (a) ([Argmax](https://www.argmaxinc.com/blog/apple-and-argmax)). Tahoe ≈ 50–70 % of active Macs mid-2026 (b) ([TelemetryDeck](https://telemetrydeck.com/survey/apple/macOS/versions/), [businessstats](https://businesstats.com/worldwide-macos-version-market-share/)) | Any Apple Silicon Mac (Metal); ports to Win/Linux (a) ([repo](https://github.com/ggml-org/whisper.cpp)) | Any machine + network + user's CF account (a) | Same (a) ([model page](https://developers.cloudflare.com/workers-ai/models/nova-3/)) |
| **Quality (WER)** | ≈ whisper-small class: beats Whisper Small on LibriSpeech (2.12 % vs 3.74 % clean) (b) ([get-inscribe](https://get-inscribe.com/blog/apple-speech-api-benchmark.html)); slightly *behind* small.en on hard earnings-call audio (14.0 vs 12.8 WER) (b) ([Argmax](https://www.argmaxinc.com/blog/apple-and-argmax)) | Model-dependent: small ≈ Apple; large-v3-turbo best local quality (b) | large-v3-turbo weights = best whisper quality — *minus chunk-boundary damage* (c); hallucination-on-silence reports exist (b) ([community](https://community.cloudflare.com/t/cloudflare-workers-ai-whisper-transcription-produces-corrupted-repetitive-output/835162)) | Nova-3: strong commercial ASR (b) |
| **Speed on 2 h audio** | ~45× RT: 34-min file in 45 s → 2 h ≈ **2.7 min** (b) ([MacStories](https://www.macstories.net/stories/hands-on-how-apples-new-speech-apis-outpace-whisper-for-lightning-fast-transcription/)) | large-v3-turbo ~10–20× RT w/ Metal on M-Pro class → 2 h ≈ **6–12 min**; small faster; base-M1 large ≈ 2–3× RT (b) ([promptquorum](https://www.promptquorum.com/local-llms/apple-silicon-whisper-metal-benchmark), [fazm](https://fazm.ai/blog/whisper-cpp-metal-apple-silicon)) | ~115 × 1 MB chunks; serial ≈ 6–10 min, parallel ≈ **~2 min** (c, arithmetic §"chunking") | Single HTTP call up to body limit; built for long audio (b) |
| **Word/VTT timestamps (P8)** | Per-word timing via `audioTimeRange` attribute on result runs (a) ([API](https://developer.apple.com/documentation/speech/speechtranscriber/resultattributeoption/audiotimerange)); Argmax lists word timestamps supported (b). VTT authored from runs — trivial (c) | Native `-ovtt` segment cues (a) ([cli README](https://github.com/ggml-org/whisper.cpp/tree/master/examples/cli)); word-level only approximate: `-ml 1`, `-sow`, `--dtw` experimental (a/b) — **weakest word-grade of the four** | `vtt` string + `segments[]` + `words[{start,end}]` in output schema (a for base [whisper](https://developers.cloudflare.com/workers-ai/models/whisper/); b for turbo [schema via Speechall](https://speechall.com/models/cloudflare.whisper-large-v3-turbo)) — but per-chunk, needs offset-stitching (c) | Word timestamps native (b) |
| **Footprint & OSS-trust** | **Zero.** Model is system-managed via `AssetInventory.assetInstallationRequest` — downloaded by macOS, outside the app, shared across apps (a/b) ([dev.to WWDC25-277](https://dev.to/arshtechpro/wwdc-2025-the-next-evolution-of-speech-to-text-using-speechanalyzer-6lo)). No blobs in repo or bundle | Binary: small C++ build (compile-on-device viable). Models: base 148 MB / small 488 MB / large-v3-turbo 1.62 GB, q5_0 574 MB (a) ([HF](https://huggingface.co/ggerganov/whisper.cpp/tree/main)) — download-on-demand w/ pinned SHA-256 (the POC's vendored-hls.js pattern); **bundling rejected** (§rejected) | Zero local footprint. Trust cost: audio leaves the machine (see privacy row) | Zero local; closed commercial model (trust-stance mismatch) |
| **Cost** (as of 2026-07-19) | $0 | $0 (electricity) | **$0.0005/audio-min** (41–47 neurons/min) → $0.03/h of video; free tier 10 k neurons/day ≈ **214 min audio/day** (a) ([pricing](https://developers.cloudflare.com/workers-ai/platform/pricing/)) | $0.0052/min HTTP → 10× whisper; free tier ≈ 21 min/day (a) |
| **Privacy (P3 consequence)** | Words never leave the machine ("no data leaves the device", (b) dev.to). Transcript local-only by default; bucket upload is a separate, optional act | Same — fully local | Audio bytes + transcript transit **Cloudflare's inference fleet** (user's account/billing, *not* user hardware). Bucket-resident transcript is the natural landing → **anyone holding the unguessable link can fetch the words** (P1 model). Must be said honestly in setup copy | Same, plus a third party's (Deepgram's) model |
| **Title/chapter LLM home** | BYO-agent skill, or nowhere (P10 manual). Could *optionally* call a Workers AI text model even with local transcription | Same | Natural: Workers AI text model (e.g. llama-70b-fast: $0.29/M in, $2.25/M out (a)) on the same account, in the same Worker — ~$0.0002/title, free-tier-covered (c) | Same |
| **Fail-soft shape** | API absent below macOS 26 → feature simply off; no partial states | Model not downloaded → skip, POC-style soft skip (proven pattern) | Network/quota/413 errors per chunk → partial transcripts possible; needs retry + give-up states | Same |

### Answers to the R8 unknowns, explicitly

**Apple SpeechAnalyzer/SpeechTranscriber — quality/speed vs whisper.** Speed: decisively
faster — 34-min, 7 GB video in 45 s vs MacWhisper Large V3 Turbo's 1:41 (2.2×) (b)
([MacStories](https://www.macstories.net/stories/hands-on-how-apples-new-speech-apis-outpace-whisper-for-lightning-fast-transcription/),
echoed by [MacRumors](https://www.macrumors.com/2025/06/18/apple-transcription-api-faster-than-whisper/)).
Quality: whisper-small class, *not* large class — it beats Whisper Small on clean read
speech (2.12 % vs 3.74 % WER) (b, [get-inscribe](https://get-inscribe.com/blog/apple-speech-api-benchmark.html))
but trails WhisperKit small.en on hard conversational audio (14.0 vs 12.8 WER on
earnings22) while running 2× faster than small.en (70× vs 35× RT) (b,
[Argmax](https://www.argmaxinc.com/blog/apple-and-argmax)). For screen-recording
narration (clear, single speaker, topic stated up front — POC §3's empirical gate) this
is title/captions/search-grade. **Confidence: high** on "good enough for shroom's jobs."

**Word-level timestamps (VTT-grade?).** Yes, with one nuance. Timing arrives as
`audioTimeRange` (a `CMTimeRange`) attached to *runs* of the result `AttributedString`
(`SpeechTranscriber.ResultAttributeOption.audioTimeRange`, (a)
[Apple docs](https://developer.apple.com/documentation/speech/speechtranscriber/resultattributeoption/audiotimerange));
Argmax's comparison table lists word timestamps as supported (b). WWDC25-277 demos
word-by-word karaoke highlighting driven by it (b,
[session](https://developer.apple.com/videos/play/wwdc2025/277/)). Run granularity =
word-scale in practice, but **probe it** (below) before writing it into the spec.
Punctuation and capitalization are native. VTT is authored by us from runs — cue
segmentation is our logic either way (c).

**Min-OS floor vs R1's matrix.** macOS 26+ (a/b). R1's capture floor is macOS 15 for SCK
`captureMicrophone` — and macOS 26 is 15's direct successor (Apple skipped 16–25), so the
transcription floor is exactly **one macOS release above the capture floor**. Adoption
mid-2026: Tahoe ~52 % (businessstats) to ~69 % on 26.5 among TelemetryDeck-measured Macs
(b — app-telemetry samples skew engaged users; treat as upper bound). A v1 shipping
late-2026/2027 can defensibly say: *transcription (and later captions) requires macOS 26;
below it the recording still publishes, untranscribed* — the POC's fail-soft rule
verbatim. Below-floor users who *want* transcripts take the whisper.cpp fallback.

**whisper.cpp footprint vs the no-opaque-blobs stance.** Binary: whisper.cpp is a small,
readable C++ codebase; build-from-source on device is realistic (minutes, no Python — the
POC's compile-on-device trust posture ports, unlike its Python-whisper dependency).
Models: ggml files are weights, not code — 148 MB (base) / 488 MB (small) / 1.62 GB
(large-v3-turbo) / 574 MB (turbo-q5_0) (a,
[HF listing](https://huggingface.co/ggerganov/whisper.cpp/tree/main)). **Bundling is
rejected** (would 10×–50× the app download and violate "minimal/no bloat");
download-on-demand with pinned SHA-256 and explicit user consent mirrors the POC's
vendored-hls.js discipline and keeps the trust story honest: the repo contains no blobs,
the app fetches disclosed, hash-locked artifacts. RAM while running: ~852 MB (small) to
~3.9 GB (large) (a, [repo README](https://github.com/ggml-org/whisper.cpp)) — fine as a
transient background job, notable on 8 GB machines.

**whisper.cpp Metal performance on typical machines.** Community benchmarks cluster:
large-v3-turbo ≈ 10–20× realtime on M2-Pro-class with Metal (60 s in ~2.8–6 s) (b,
[promptquorum](https://www.promptquorum.com/local-llms/apple-silicon-whisper-metal-benchmark),
[fazm](https://fazm.ai/blog/whisper-cpp-metal-apple-silicon)); Metal gives 30–60 % over
CPU, more on big models; base-M1 + full large-v3 is the bad case (~2–3× RT). Numbers vary
with quantization/flash-attn — hence the bench probe below. `small` or `turbo-q5_0` is
the sane default preset.

**Workers AI Whisper — pricing/free tier at realistic volume.** (a, as of 2026-07-19,
[pricing](https://developers.cloudflare.com/workers-ai/platform/pricing/)):
`@cf/openai/whisper` $0.00045/min (41.14 neurons/min), `whisper-large-v3-turbo`
$0.0005/min (46.63 neurons/min). Free allocation 10,000 neurons/day →
**≈ 214 min of audio per day free** (c: 10,000 ÷ 46.63). Realistic shroom volume (say
30 min recorded/day) uses ~1,400 neurons — **free forever, in practice**; a paid heavy
day of 5 h costs ~$0.09 (c). Verdict on P10's "if it will be cheap": **cheap: yes.**

**Workers AI — audio-length/size limits.** The docs state none — a documented
documentation gap (a, [cloudflare-docs issue #17916](https://github.com/cloudflare/cloudflare-docs/issues/17916)).
Empirically: ~0.7 MB succeeds, ~2 MB throws `InferenceUpstreamError`, ~4.2 MB fails (b,
[community](https://community.cloudflare.com/t/inferenceupstreamerror-for-large-audio-files-2-mb/624759));
Cloudflare's own tutorial chunks to **1 MB** (a,
[chunking tutorial](https://developers.cloudflare.com/workers-ai/guides/tutorials/build-a-workers-ai-whisper-with-chunking/)).
Platform request-body caps (100 MB free/pro) are not the binding constraint — the model
input is (a, [Workers limits](https://developers.cloudflare.com/workers/platform/limits/)).
**Design number: ≤ 1 MB audio per request.**

**Chunked HLS-audio upload pattern (made concrete).** shroom's segments are muxed
video+audio fMP4 — don't send those. The app extracts the AAC track (128 kbps mono ≈
0.96 MB/min, POC §2) per N segments and posts ~60 s chunks: (c)
- Chunk at **HLS segment boundaries** (6 s multiples) — chunk start-times are then known
  exactly from the playlist, so per-chunk VTT/`words[]` offsets are exact arithmetic, not
  guesswork. (Cloudflare's tutorial byte-splits mid-frame and just concatenates text,
  losing timestamps — do not copy it.)
- Boundary damage: a word straddling a 60 s cut gets mangled ~once per chunk; mitigate
  with 1–2 s overlap + dedupe, or accept for title/search grade.
- Transport: direct REST (`POST /accounts/{id}/ai/run/@cf/openai/whisper-large-v3-turbo`,
  base64 body) with the user's token — no Worker strictly required (a, model-page curl
  examples); or via the R4 Worker once BYO-backend exists (free-plan Worker CPU 10 ms is
  fine — inference time isn't Worker CPU; 50 subrequests/request caps in-Worker fan-out
  (a)).
- 2 h recording = ~115 chunks ≈ $0.06 / 5,600 neurons (inside the daily free 10 k) (c).

**Latency for the head-60 s title path.** One ~1 MB request; community-reported
inference for ~1 min audio is single-digit seconds; add RTT → **~3–8 s** end-to-end (b/c).
Local engines beat it (SpeechAnalyzer ~1–2 s, whisper.cpp small ~4–6 s incl. model load
(c)), but all three are inside "title lands while you're still looking at the page."

**Where the title/chapter LLM lives, per engine (P10).**
- *Local engines:* no in-app LLM (the rethink: you'd be annoyed to find one). Title
  authoring = BYO-agent skill over the local transcript (optional garnish), or nowhere —
  manual title, P10's chosen default.
- *Workers AI engine:* the same account runs text models — llama-3.3/3.1-70b-fast at
  $0.293/M input, $2.253/M output (a, pricing page): a title from a 60 s head transcript
  is ~400 tokens ≈ **$0.0002, free-tier-covered** (c). Natural shape: the R4 Worker
  finishes chunk-stitching → calls the text model → writes title/chapters into metadata.
- *Mixed mode* is legal: local transcription + Workers AI titling (transcript text, not
  audio, leaves the machine) — smaller privacy surface than CF transcription, same LLM
  home. Worth remembering, not recommending yet.

**VTT/caption emission quality per engine (P8 hard requirement).**
- *Apple:* word-level `audioTimeRange` + native punctuation → best substrate; we write
  the VTT cues ourselves (needed anyway for cue-length/line-break rules).
- *whisper.cpp:* `-ovtt` gives clean segment cues (a); word-level timing is its weak spot
  — `-ml 1`/`--split-on-word` are heuristic, `--dtw` token timestamps are explicitly
  experimental (a, cli README). Captions (cue-level): fine. Karaoke-grade word highlight:
  not reliable.
- *Workers AI:* `vtt` + `segments[]` + `words[{word,start,end}]` returned directly (a
  for `@cf/openai/whisper`; the turbo schema also carries per-segment `words` (b,
  [Speechall's schema mirror](https://speechall.com/models/cloudflare.whisper-large-v3-turbo)
  — confirm in probe). Chunking means we re-emit the VTT with offsets regardless.
- **All engines satisfy P8 at segment/cue grade; Apple and Workers AI satisfy word grade.**
  Store the richest available: `transcript.json` = `{engine, language, segments:[{start,
  end, text, words?:[{w,start,end}]}]}` + derived `captions.vtt` — old videos then never
  need re-transcription when captions ship (P8's exact condition).

### Recommendation (R8)

**v1: SpeechAnalyzer as the default engine on macOS 26+, fail-soft to "no transcript"
below; whisper.cpp (download-on-demand, `small` or `turbo-q5_0`) as the opt-in fallback;
Workers AI deferred until the R4 Worker/BYO-backend exists.** Emit the engine-agnostic
transcript artifact + VTT from day one. Confidence: high on the default; medium on
whether the fallback belongs in v1 at all (it exists to serve pre-26 Macs and non-Apple
languages — including, concretely, Armenian/Russian, which Apple's ~10-language set (b,
Argmax) almost certainly lacks; Whisper's 100 languages include them).

**Rejected alternatives, with reasons:**
- *Bundle whisper.cpp + model in the app* — 0.6–1.6 GB of opaque weights in every
  download violates "minimal/no bloat" and the no-blobs trust stance; download-on-demand
  costs one consent dialog and keeps both. Rejected on principle and size.
- *Python `whisper` CLI (the POC status quo)* — requires a user Python install (the exact
  C-persona burden R8 was asked to remove), slower than whisper.cpp on the same models,
  and adds nothing whisper.cpp lacks. Rejected; the POC's seam design (soft-skip,
  injectable engine) is what carries over, not the engine.
- *Workers AI as the v1 default* — cheap in dollars, expensive in mechanism (chunking,
  offset-stitching, retry states, hallucination-on-silence fail-softs) and it moves the
  words off the machine for every user by default (P3). It becomes attractive *after*
  BYO-backend exists and for machines that can't run local models. Deferred, not killed.
- *Deepgram nova-3 on Workers AI* — handles long audio without DIY chunking and has
  native word timestamps, but 10× whisper's price, a 21 min/day free tier, and a closed
  commercial model sit badly with the trust stance. Reference column only.
- *SFSpeechRecognizer (pre-26 Apple API) as the fallback* — 3.5–4× worse WER than
  SpeechAnalyzer (b, get-inscribe), 1-minute audio limits and flaky long-file behavior in
  the old API; whisper.cpp is strictly better as the below-floor path. Rejected.

**Cheap probes (an afternoon, Bagrat's machine is already on Tahoe):**
1. **SpeechAnalyzer reality check** — run [Yap](https://github.com/finnvoor/yap) (or a
   ~20-line Swift script; needs consent to install) over one POC recording and one 2 h
   file: dump `SpeechTranscriber.supportedLocales` (settles Armenian/Russian), inspect
   run granularity of `audioTimeRange` (settles word-grade claim), time the 2 h run
   (settles long-file behavior — the one thing no source measured).
2. **Workers AI turbo probe** — one curl of a 60 s AAC chunk against Bagrat's existing CF
   account: confirms the real size ceiling, whether turbo returns `words[]`, and
   end-to-end latency. Minutes, ~free.
3. **whisper.cpp bench** — `brew install whisper-cpp` (consent) + `whisper-bench` on his
   hardware: pins the RTF numbers the matrix currently takes from community posts.

---

## R11 — Long-recording physics (unbounded length, P6)

**TL;DR verdict**

- **The R2 single-PUT wall is real and closer than it looks: 4.995 GiB** (a). The teed
  MP4 crosses it at ≈ **3 h 17 m (normal 1080p), 1 h 57 m (2K), 59 min (4K)**. Under P6
  (unbounded) + configurable quality, **multipart in the zero-dep SigV4 client is
  mandatory, not an edge case** — a 1-hour 4K recording already needs it. HLS segments
  (6 s ≈ 2.7–9 MB) never come near any limit.
- **Playlist scale is a non-problem; player configuration is the real risk.** A 6 h
  recording is a ~130 KB playlist — trivial to parse in hls.js and native Safari. The
  documented failure mode is hls.js's `backBufferLength: Infinity` default eating memory
  over hours of *playback*; set it (~90 s) and move on.
- **Transcription survives 2 h+ on every R8 engine**: SpeechAnalyzer ≈ 2.7 min,
  whisper.cpp turbo ≈ 6–12 min (RAM constant — 30 s windows), Workers AI ≈ 2–10 min
  chunked at ~$0.06. No engine needs re-architecture for length.
- **Upload backlog: the page-always-exists model (P9) absorbs the median case easily and
  must be *designed* for the tail.** At normal preset (3.63 Mbps) any uplink ≥ ~5 Mbps
  keeps "content on its way" in the seconds range — US median fixed upload is ~56 Mbps
  (b). The tail is real though: hotel/café/LTE at 2–3 Mbps means a 30-min recording is
  watchable **15–40 min after stop**, and 4K on a 10 Mbps uplink adds ~30 min/recorded-hour.
- **The MP4 tee silently doubles every number** — it's a second full copy of the
  recording, uploadable only after stop (faststart rewrite at finalize). Keep it
  sequenced after HLS (POC already does), surface it as a separately-arriving "download
  ready" state, and consider making it optional.

### R2 single-PUT wall (the MP4 tee)

Limits (a, as of 2026-07-19,
[R2 limits](https://developers.cloudflare.com/r2/platform/limits/)): max object
5 TiB; **max single-part upload 4.995 GiB**; multipart: part min 5 MiB / max 5 GiB, max
10,000 parts, **all parts except the last must be the same size**, incomplete uploads
auto-abort after 7 days (a,
[multipart docs](https://developers.cloudflare.com/r2/objects/multipart-objects/)).

When the teed MP4 exceeds a single PUT (c — arithmetic from the POC's own presets,
`archived-poc/scripts/recorder/lib/quality.mjs`: video 3.5 / 6 / 12 Mbps + 0.128 audio;
4.995 GiB = 5,363 MB decimal):

| Preset | Rate | MP4 hits 4.995 GiB at |
|---|---|---|
| normal (1080p, 3.63 Mbps) | 27.2 MB/min ≈ 1.63 GB/h | **≈ 197 min (3 h 17 m)** |
| 2K (6.13 Mbps) | 46.0 MB/min ≈ 2.76 GB/h | **≈ 117 min (1 h 57 m)** |
| 4K (12.13 Mbps) | 91.0 MB/min ≈ 5.46 GB/h | **≈ 59 min** |

Consequences for the zero-dependency SigV4 client (POC §5): multipart is three more
signed calls (`CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload` with an
XML body) — a modest, well-bounded addition (~100–200 lines) to `lib/s3.mjs`, which
already signs arbitrary methods/queries. Design points: pick a fixed part size (e.g.
128 MiB → ceiling 1.25 TiB, ~13 h of 4K — comfortably "unbounded, best effort");
threshold at ~4 GiB (single PUT below, multipart above); rely on the 7-day auto-abort as
the crash-cleanup backstop, since a crashed multipart otherwise silently accrues storage.
R2's multipart follows S3 conventions (rclone etc. work against it (a)), so the botocore
golden-vector test approach transfers. **Confidence: high.**

Two adjacent physics facts worth having on the table (c):
- `movflags=+faststart` rewrites the whole MP4 at finalize (moov relocation) — at 4K/2 h
  that's copying ~11 GB at stop: seconds-to-tens-of-seconds of finalize latency and a
  transient 2× disk footprint. Local disk envelope for a 2 h 4K session ≈ HLS 10.9 GB +
  MP4 10.9 GB + transient copy ≈ **~33 GB peak**.
- Storage-cost honesty: R2's free tier is 10 GB — one 2 h 4K recording (HLS+MP4 ≈ 22 GB)
  exceeds it on its own; at normal preset the free tier holds ≈ 3 h with MP4s, ≈ 6 h
  HLS-only (matches POC cost copy). The library's "your storage costs ~$X" idea (P13)
  earns its keep at these scales.

### HLS VOD playlist at multi-hour scale

Arithmetic (c): 6 s segments → 600/h. A VOD playlist entry
(`#EXTINF:6.00000,` + `seg_%05d.m4s`) ≈ 30–35 bytes:

| Duration | Segments | Playlist size |
|---|---|---|
| 2 h | 1,200 | ~40 KB |
| 6 h | 3,600 | ~130 KB |
| 24 h | 14,400 | ~500 KB |

- **hls.js:** no reported problems parsing VOD playlists at thousands of segments — the
  documented long-playback failure mode is memory, not parsing: the default
  `backBufferLength: Infinity` never evicts played media, and multi-hour sessions grow
  until the tab dies (b, [Mux's production post-mortem](https://www.mux.com/blog/an-hls-js-cautionary-tale-qoe-and-video-player-memory),
  [hls.js #1235](https://github.com/video-dev/hls.js/issues/1235),
  [#5402](https://github.com/video-dev/hls.js/issues/5402)). Fix is configuration, not
  architecture: `backBufferLength: 90` (Mux suggests 30 for live), default
  `maxBufferLength: 30` is fine. The POC's player config should adopt this regardless of
  recording length.
- **Native Safari:** no documented playlist-size/segment-count cap found (searched; Apple
  HLS is the delivery format for multi-hour movies/events, and fMP4/CMAF 6 s VOD is its
  home turf). Treat as low-risk (c, absence-of-evidence — covered by the probe).
- **Probe:** script-generate a synthetic 6 h playlist (duplicate one segment 3,600×),
  serve locally, load in hls.js + Safari, watch memory. An hour of work; settles both
  players empirically.

### Whisper-class engines on 2 h audio

Answered fully in R8's speed row; the R11-relevant facts: every engine is
streaming/windowed, so **memory is constant w.r.t. duration** (whisper decodes 30 s
windows; whisper.cpp RAM = model-dependent 852 MB–3.9 GB (a); SpeechAnalyzer is a
streaming `AsyncSequence` API with a system-managed model (b)). Runtimes for 2 h:
SpeechAnalyzer ≈ 2.7 min (b-derived), whisper.cpp large-v3-turbo ≈ 6–12 min on M-Pro
Metal (b), small ≈ 2–5 min (b/c), Workers AI ≈ 2 min parallel / 6–10 min serial at ~115
chunks, $0.06 (c). Only Workers AI *requires* chunking; local engines take the file
whole. Head-transcription (POC §3) is orthogonal: it reads only the first ~60 s
regardless of total length — unaffected by P6. **Confidence: high.**

### Upload-queue backlog math (and the P9 product judgment)

Model (c): HLS uploads opportunistically *during* recording (POC §5); the MP4 exists
only after stop. Assume ~80 % effective uplink utilization (per-segment PUT overhead).
Recording rate r, uplink U, duration D → backlog drain after stop ≈ D·(r−0.8U)/0.8U when
positive.

**Time until the page is watchable (HLS drained), normal preset (3.63 Mbps):**

| Uplink (nominal) | Who this is | 10-min rec | 30-min rec | 2-h rec |
|---|---|---|---|---|
| 2 Mbps | bad hotel/café wifi, weak LTE | ~13 min | ~38 min | ~2.5 h |
| 3 Mbps | DSL, congested cable up | ~5 min | ~15 min | ~61 min |
| 5 Mbps | modest cable up | **seconds** (keeps up live) | seconds | seconds |
| 10+ Mbps | typical cable/fiber | seconds | seconds | seconds |
| ~56 Mbps | **US median fixed upload, 2026** (b, [Ookla via allconnect](https://www.allconnect.com/blog/us-internet-speeds-globally)) | seconds | seconds | seconds |

**4K preset (12.13 Mbps)** shifts the cliff: a 10 Mbps uplink no longer keeps up
(30-min rec → ~15 min wait; 1 h → ~31 min); it takes ~15+ Mbps effective to stay live (c).

**The MP4 doubles everything:** after HLS drains, the download asset uploads for
D·r/0.8U more — e.g. a 30-min normal recording on a 5 Mbps uplink: page watchable in
seconds, **"download available" ~27 min later**; a 2 h recording on 10 Mbps: ~54 min (c).
Fully-offline recordings pay both terms after reconnect: a 1 h normal recording ≈ 3.3 GB
total ≈ 55 min on 10 Mbps, 12 min on 56 Mbps (c).

**Is a long-visible waiting state a product problem? The numbers say: rarely, and
addressably.** On median links (≥ 10 Mbps up) the P9 "content on its way" state lives for
seconds at normal preset — invisible. It becomes minutes-to-hours in exactly three cases:
(1) sub-4 Mbps uplinks (hotels, cafés, LTE, non-fiber DSL — precisely the "ad-hoc help"
recording context), (2) 4K preset on ordinary links, (3) offline recording. Since P9
already mandates the always-existing page, the design bill is: an honest progress state
on the page ("processing — X %" or a coarse ETA), a sender-side tray/notification for
"fully live" (P9 asked for this), and copy that never promises "instant" — only "link
now." Two sharper implications the stack session should carry: **the MP4 should be
explicitly second-class** (deferred, maybe optional, never gating "watchable"), and the
**quality picker should whisper the uplink cost** (4K on hotel wifi is a 30-min-per-hour
promise — the POC's cost-copy pattern extends naturally). Not a reason to bound length
(P6 stands); it is a reason to make the waiting state first-class UI. R2-side friction is
nil: a 2 h recording ≈ 1,200 Class-A ops of the 1 M/month free tier (a).

### Recommendation (R11)

**Build multipart into the SigV4 client from day one; fix hls.js buffer config; treat
backlog as a UX surface (progress + "fully live" signal + second-class MP4), not an
engineering problem; leave recording length unbounded.** Confidence: high throughout —
this section is mostly arithmetic over verified limits.

**Rejected alternatives, with reasons:**
- *Cap recording length to dodge multipart* — contradicts P6, and the cap would have to
  sit at ~1 h (4K) to work; multipart is ~150 lines against a client already built for it.
- *Drop the MP4 tee* (all numbers halve) — kills the download nice-to-have (§5.4) and
  the single-file archive; making it *deferred/optional* buys nearly the same physics
  without the product loss.
- *Fragmented-MP4 progressive upload of the download file* (upload the MP4 during
  recording, no faststart) — avoids the post-stop bulk but ships a moov-at-end or
  fragmented file that browsers/download-consumers handle worse, and doubles *live*
  upload bandwidth exactly when the link is weakest. Not worth it.
- *Adaptive/low rung to fix slow-uplink waits* (upload a 540p rung first) — real, but
  it's R9's question (renditions) and pays an encode + storage cost; the honest v1 answer
  is the progress state.

**Probes:** the synthetic 6 h playlist (above); one multipart upload against Bagrat's
existing R2 bucket via the POC's `lib/s3.mjs` extended with the three calls (validates
the golden-vector approach on `CompleteMultipartUpload`'s XML signing); and — cheapest of
all — re-watch one of the POC's still-published recordings on a throttled connection
(Network Link Conditioner) to feel the "content on its way" state before designing it.

---

## Hands-on probe results (2026-07-20 session) — scap capture probe + E1 + E2, all (b)

> Executed live on the Tahoe machine (macOS 26.5.2, Darwin 25F84, external 4K display in
> clamshell). Probe code: a ~380-line Rust binary (session scratchpad, disposable — the
> recipe below is the durable artifact). Everything in this section is (b), same-day.

### The scap capture probe — PASSED (core/shell ratification unblocked)

**Setup facts:**

- **Zero toolchain install**: the machine already had brew `rust` 1.88.0 (a forgotten
  `brew install rust`) — and the crates' own `rust-toolchain.toml` pins **exactly 1.88.0**.
- The `scap-*` crates are **not on crates.io** — consumed as git dependencies on their
  upstream monorepo, pinned to main HEAD `fb3f2438` (2026-07-19). Their hakari
  `workspace-hack` dep (would drag the monorepo's whole unified dep set) was **stubbed out
  via `[patch]` → empty local crate — works cleanly**; this is the pattern a v2 Cargo.toml
  would use.
- `scap-targets` was **not needed**: display enumeration + filter built directly on cidre
  (`sc::ShareableContent::current()` → `sc::ContentFilter::with_display_excluding_windows`).
  Direct cidre dep pinned to the fork rev `bf84b67079a8`; cargo unifies it with scap's
  workspace-inherited copy.
- Cold build **1 m 59 s** including the full git fetch (upstream repo ~200 MB into `~/.cargo/git`)
  and all of cidre; incremental rebuilds **0.7 s**; release binary **733 KB**.

**The one real API gotcha (will bite any reimplementation):** SCK's default pixel format
on Tahoe is **not CPU-readable**. Frames deliver at full rate (296 callbacks/10 s) with
`image_buf` present, but `CVPixelBufferGetBaseAddress` returns NULL — silent zero-byte
extraction. Fix: `cfg.set_pixel_format(cv::PixelFormat::_32_BGRA)` (cidre's own docs do
this). After forcing BGRA: stride = exactly `w*4` on this display (no padding), base
address valid. Corollary: **`CGPreflightScreenCaptureAccess` true + a started stream ≠
frames** — the only honest liveness signal is a delivered, lockable frame (the POC's
wedge-classifier discipline transfers to SCK unchanged).

**Capture quality (BGRA 3840×2160 @ 30 fps requested):**

| run | frames | effective fps | first frame |
|---|---|---|---|
| terminal `frames 60` | 61 | 30.49 | 28 ms |
| bundle `frames 90` (granted) | 90 | 29.94 | 26 ms |
| bundle post-update `frames 90` | 91 | 30.27 | 46 ms |
| bundle `record 20 s` | 601 rcvd / 600 written | 30.0 | — |

**The composed pipeline (probe's namesake): SCK frames → 30 Hz frame clock (duplicate-on-idle)
→ rawvideo BGRA pipe → the POC recipe's encode half** (`-f rawvideo -pix_fmt bgra -i -` →
`-vf scale` 4K→1080p → `h264_videotoolbox -b:v 3500k` → forced 6 s keyframes → `-f tee`
HLS-fMP4 + faststart MP4):

- 30 s record: 900 frames written (26 duplicated ticks), ffmpeg exit 0, **finalize 102 ms**
  (POC's ~200 ms claim beaten), playlist duration **exactly 30.000000** — the frame clock
  makes duration honest by construction (no avfoundation bogus-timebase fix needed; the
  `-r 30 -force_key_frames` recipe lines carry over verbatim and behave).
- Artifacts shape-identical to POC output: `EXT-X-VERSION:7`, `EXT-X-MAP init.mp4` (804 B,
  never the 0-byte wedge), 6.000000 s `.m4s` segments ~2.3 MB (≈3.2 Mbps), VOD+ENDLIST;
  MP4 leg moov-before-mdat (faststart verified).
- Cost at steady state (4K→1080p): probe **12 % CPU / ~150 MB RSS** (row-copy dominated),
  ffmpeg **56 % CPU / ~265 MB** — ~0.7 core total; R9's "encode capacity is not the
  constraint" confirmed at the composed-pipeline level. (v2 note: SCK can GPU-scale at
  capture via cfg width/height — would delete most of both numbers; not exercised.)
- EOF-on-stdin is the stop signal for the rawvideo topology (the POC's `q` applies only to
  device inputs); ffmpeg finalizes both tee outputs cleanly on it.

**Verdict: the no-Swift Rust core is real on this machine.** The MIT 349-line crate +
cidre drives SCK end-to-end into the validated encode recipe with zero Swift and no
binding surprises beyond the pixel-format default. R1's "sealed Swift helper remains the
honest default" is now counter-proven at probe scale for the capture leg — the remaining
Swift-only territory is AVAssetWriter-native HLS authoring. **The shell session can ratify Tauri v2 + Rust core on evidence.**

Honest caveats: single-display clamshell config; video-only (mic = cpal/native per
POC findings, not re-probed); 30 s scale, not multi-hour (R11 probes stand); scap
crates are unversioned workspace members (pin-by-rev or vendor — a v2 decision); R1's
binding-churn caveat stands (cidre fork moves).

### E1 + E2 — PASSED (the $0 identity question is closed)

**Method chronicle, incl. everything that fought back (wizard-grade material):**

1. **Tahoe's redesigned Keychain Access has no visible Certificate Assistant** — the
   documented yabai UI recipe ("Create a Certificate…") is dead on this machine. The
   scripted path replaces it, and is what a wizard/installer would run anyway:
   `openssl req -x509` (CN=shroom-local, EKU=codeSigning critical, config-file form) →
   `openssl pkcs12 -export` — **must force legacy algs** (`-keypbe PBE-SHA1-3DES
   -certpbe PBE-SHA1-3DES -macalg sha1`) because Homebrew OpenSSL 3.6's modern defaults
   fail `security import` with "MAC verification failed" → `security import -T
   /usr/bin/codesign` → `security add-trusted-cert -p codeSign` (user domain, **no sudo**;
   at most GUI auth dialogs — nothing password-shaped on a command line).
2. **CLI-app TCC gotcha**: `CGRequestScreenCaptureAccess` from a process that exits
   immediately (~3 ms) leaves a **silent deny row** (`auth_value 0`) — no prompt shown, app
   never listed in the Settings pane, and **all later requests return false silently**
   (the prompt is once-per-state). Recovery = the POC's own move: `tccutil reset
   ScreenCapture am.shroom.probe` + re-request **from a process that stays alive** through
   the dance. (Moot for a real tray app, which lives; real for any helper/wizard step.)
3. **`tccutil reset` needs an LS-resolvable install path**: it failed with -10814 while
   the .app lived under `/private/tmp` (even after explicit `lsregister -f`); moving the
   bundle to `~/Applications` fixed resolution instantly. Probes and installers should
   treat "app at a real path" as a precondition for any TCC surgery.
4. Settings pane caches — quit/reopen to see fresh entries.

**E1 (grant survival across update under a self-signed identity):**

- Signed DR: `identifier "am.shroom.probe" and certificate leaf = H"a864…e622"`;
  `TeamIdentifier=not set`. After the user toggle, the TCC row's 72-byte csreq **decodes
  to that exact string** (POC §4 inspection recipe, system TCC.db is world-readable) —
  TCC pinned the grant to identifier+leaf, not a cdhash.
- First granted capture: full-rate frames (90 @ 29.94 fps). **This alone retires
  the public Team-ID claim's strong reading** — SCK on
  Tahoe 26.5 demands no Apple-chained Team ID.
- Update leg: content change → rebuild (cdhash `17264ba2…` → `a8c89449…`) → re-sign with
  the same cert → replace installed bundle → relaunch → **full-rate capture with zero user
  action**. No Settings visit, no toggle, no prompt, no phantom state. **E1 PASS.**

**E2 (is the leaf-hash check trust-free?), run in a stronger two-step form:**

- (a) `security remove-trusted-cert` (trust settings gone) → capture unaffected.
- (b) `security delete-identity` (cert **and** key gone; `find-identity` shows zero) →
  capture unaffected. **TCC validates against the certificate embedded in the signature —
  no keychain presence, no trust settings required. E2 PASS ⇒ 5b is live.**

**What this changes in R3's recommendation (§8):** identity flips from 5a-primary to
**5b-primary** — Bagrat signs releases once with a developer-side self-signed cert; user
machines create/import/trust nothing; grants survive every update. Per-machine 5a remains
only for the `--from-source` path (local builds need local signing). The update story
inside the $0 line is now: **download → re-sign not even needed (5b ships signed) →
replace → relaunch**, with zero recurring TCC cost. E3's mechanism (swap + relaunch, no
Gatekeeper because terminal-fetched = unquarantined, no re-grant) was demonstrated inside
E1's update leg; only the Sparkle-vs-custom wrapper choice remains, and it waits on the
shell decision.

**The replayd bypass-picker consent survives on Tahoe:** Bagrat observed the
"bypass the system window picker" prompt fire during the session's capture runs
(invisible to screenshot automation, as POC §4 documented — a human had to report it).
Impact: zero — every run still delivered full frame counts (898/900, 90, 91, 601), so on
Tahoe it is non-fatal to an in-flight stream and one approval clears it for the ~30-day
cadence. The POC's "prime it before the first real recording" discipline (POC §4) remains
the right v2 softener so it never lands mid-recording. No monthly re-confirm nag was seen
(single-session window — expected). Residuals: second-user-account
E2 confirmation; ad-hoc negative control (skipped — the POC's own cdhash-pinned
`am.shroom` row in the same TCC table is the standing evidence that cdhash pinning is
real); macOS 27 could change AMFI/TCC behavior — the standing R3 caveat, now with a
concrete regression test to re-run per major release (this probe, ~10 min).
