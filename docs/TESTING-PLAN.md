# shroom v1 — testing plan for the uninterrupted build

> Status: **draft, 2026-10-01** — written at Bagrat's go to build v1 end to end without
> interruptions. Companion to the spec, [`machine.json`](../spec/machine.json) (*what* to build; read as [`MACHINE.md`](../spec/MACHINE.md))
> and [`POC-LEARNINGS.md`](../history/POC-LEARNINGS.md) §9 (the harness lessons this extends). §9
> below is the pre-flight: everything that has to be in place before the run starts.

## The premise

Tests written alongside the code share their author's blind spots — that is how the POC's
UX slipped while its suites stayed green. In an uninterrupted build the author is also the
only reviewer, so "going well" is measured against **three oracles nobody writes during
the build**:

1. **The spec** — the states, the edges and rules in `spec/machine.json`, the 16 standing laws,
   the ratified said-lines.
2. **The POC's measured facts** — the capture recipe, the audio bug, the TCC mechanics,
   the probe numbers.
3. **Reality** — real ScreenCaptureKit frames, a real Cloudflare account, a real browser
   playing the real page.

Plus one end-to-end **heartbeat** that exists from the first day and stays green.

---

## 1. Build order — walking skeleton first

**Day one: the thinnest slice through every layer.** Tray click → 5 s of real SCK capture
→ the POC's segmented encode → direct-to-R2 upload → the Worker page → playback in a
headless browser. No wizard (a hand-wired test space), no polish. That slice *is* the
heartbeat's first version; every later region thickens it.

Then the regions, riskiest integration first: the capture core's hard edges (pause/resume,
crash → recovered take) → the page and upload truth (Region 4) → real provisioning
(Region 2) → permissions + install/update (Regions 1, 3) → members (Region 6) → Library and
Settings (Region 5).

*Rejected: building in the spec's region order (1 → 6).* The capture → upload → page chain
would land last, and weeks would pass with nothing end to end to say the build is off.
*Rejected: unit tests plus a final manual QA pass.* That is the POC's failure mode.

## 2. Oracle 1 — the spec as executable tests

Written **before** each region's implementation, committed separately.

### 2.1 State is a pure function

Law 1 (state is derived, never a step counter) becomes the core's shape:
`derive(world) → State`, where *world* is the probed facts (provisioned? token kind? which
permissions? a take open? online? the space reachable? member revoked? disk headroom? …).
Then:

- **Table tests** — every row of the spec's state inventory gets at least one world that
  must derive to it.
- **Exhaustive enumeration** over the fact space (it's small and finite) catches three
  failure classes the table can't: a world that derives to *no* state, a world two faces
  both claim, and an inventory state *no* world reaches.
- Faces that are row facts or copy variants (N18's nuance) are asserted as render
  properties of their state, not as states.

### 2.2 Edges come from the machine

`spec/machine.json` holds the machine (DECISIONS B133): its edges, and its **rules** — a move
that holds for a whole group of states, written once (✕ closes the window from any row; any
network-bound step goes offline and back). The tests hold the code to it both ways: every
edge an arm names (`Arm.edge`) is an edge of the machine, and every move the enumerations
make is an edge or falls under a rule (`shroom_spec::Spec::listed`); and every edge the map
drew (kept in the JSON, `source: map`) is realized by one world change or one user event in
its region's test, or that test says where it is. If the JSON and the code disagree, the
code is wrong or the JSON changes in a PR, with a deviation row where the spec bends.
(Until B133 the edges came from `spec/UX-MAP.html` through `spec/map.json`; both are in
`history/spec/`.)

### 2.3 The 16 standing laws become invariants

Checked across every state and every transition, not per screen. Examples of the shape:

| Law | Invariant under test |
|---|---|
| 2 The recording is sacred | No code path removes take bytes except Discard; fault injection (§5) never loses a take |
| 3 Only you make a link | A link id is minted only by Stop or Get link *(as built, the recording's id is minted at capture start as the upload prefix and becomes a link only by Stop or Get link — DEVIATIONS #4 proposes that wording, awaiting ratification; the invariant checks the link)* |
| 4 The quiet-take law | While a take is open, no window, banner, or page opens; Update/Disconnect/Leave/Quit are refused |
| 5 The click is the door | Unasked popovers are non-activating — never take key focus |
| 6 Dock iff a window | Activation policy tracks open windows, never popovers *(DEVIATIONS #38's narrowing to the setup window was rejected 2026-10-13; Bagrat confirmed law 6 at the 0.1.10 checkpoint, B138)* |
| 7 One ask, one fact | At most one banner and one status line per surface |
| 8 One default | Exactly one Return default per surface, matching the spec's table |
| 9 Destruction | Destructive actions are never the Return default; Esc = back |
| 10 Nothing auto-copies | No clipboard write without a click on a Copy control |
| 11 No dialog to an empty chair | Every OS permission request follows a user click in the same flow |
| 13 Secrets ride files | Secrets never in argv, logs, or IPC payloads to the webview; `~/.shroom` files mode 600 |
| 15, 16 Voice and numbers | The copy lint (§2.4) |

### 2.4 Copy is data

Said-lines are extracted from the spec into one strings file the UI renders from; a test
asserts the UI matches the spec **verbatim** for every *copy-ratified* section (draft
sections are checked for presence, not wording). A lint runs over every user-visible
string: a banned-word list (bucket, Worker, R2, HLS, ffmpeg, token kinds, error codes, …),
law 16's number rule, and the voice nouns of law 15. TypeSafe may be used as a dev-only
second opinion on new strings — never a product dependency.

### 2.5 Tests-first, and the deviation log

If a spec test goes red, the implementation changes — not the test. When the spec turns
out to be wrong or unbuildable, the change is written to **the deviations in `DECISIONS.md`** (what, why,
the evidence, the reversal) for Bagrat to ratify, and only then is the test updated. States
designed in the autonomous sitting (N29–N42, *awaiting ratification*) are tagged
**provisional** in the fixtures, so ratification churn costs a fixture edit.

## 3. Oracle 2 — the POC's measured facts, as regression tests

- **Capture recipe shape** — the scap probe proved the v2 path produces artifacts
  shape-identical to the POC's; an ffprobe signature (codecs, GOP, segment duration,
  timescale, playlist shape) of a reference capture becomes a golden.
- **`_32_BGRA` forced** — a startup assertion; SCK's default isn't CPU-readable.
- **Audio** — mic captured natively, never through ffmpeg's avfoundation input (POC §3).
  The bug's signatures become tests: duration deficit vs. wall clock, sample-gap analysis
  on a 30-minute tone capture.
- **TCC** — the app is always launched through LaunchServices (`open`), never as a child
  of the agent's shell, so it never rides an inherited terminal grant (the POC's false
  positive). `codesign -dr -` must show the identifier + leaf DR. **E1 as a standing
  regression:** rebuild + re-sign + relaunch → full-rate capture with zero user action.
- **Permission requests come from a living process** (E1 gotcha) — never from a
  short-lived helper.
- **Process trees, not processes** — any child (an encoder sidecar) dies with its group in
  every lifecycle test (POC §9).
- **Environment trap** — if a Cloudflare upload stalls on large bodies, suspect the LAN
  path-MTU black hole before the code.

## 4. Oracle 3 — reality: the media-truth harness

A **test-pattern window**: a QR code that changes every frame (frame number + monotonic
timestamp), a full-frame flash once a second, and a beep on the same tick. Captured
through the real path, published, then the published video is decoded back from the
Worker's URL and every frame's QR is read. That yields numbers:

| Measure | Proposed bar |
|---|---|
| Dropped / duplicated frames at 30 fps | ≤ 0.5 % over a 10-minute take |
| A/V offset (flash vs. beep) | ≤ 40 ms, no drift across the take |
| Pause/resume join | no frame from the paused span; ≤ 1 frame lost at the seam |
| Finalize after Stop | ≤ 200 ms (probe: 102 ms) |
| Stop → page answers 200 | ≤ 1 s on a live network (*"Your link opens right away."*) |
| Capture cost | ≤ ~1 core at 4K → 1080p (probe: ~0.7) |
| Captions file | present, timestamps monotonic |

**Privacy guard for unattended runs:** every real-display capture the harness makes first
covers the captured display with the test pattern, full screen, so nothing else that's
open is ever recorded — and only test-pattern captures are ever uploaded, always under
`shroom-test-…` and torn down after.

**Quiet guard for unattended runs:** no run plays sound through this Mac's audio output.
A browser that plays a recording does it muted, or plays one with no sound track (the page
drive); the loopback checks play only into the virtual device, never the default output.
A tab that starts playing takes a person's headphones off their phone (2026-10-13: the page
drive's 440 Hz fixture did, twice). A VM is started with `--no-audio` — Tart otherwise hands
the guest's sound and its microphone through to this Mac — every one a script starts; the
checkpoint's clones, which Bagrat sits at, he starts by hand (CHECKPOINT step 10).

*As built (DECISIONS B59 d, a logged substitution):* the QR is a **counter strip** (24-bit
counter + CRC-8, `shroom-testpattern`) phase-locked to a beep on every wall-clock second;
the flash's A/V reading is each beep against where the picture shows that second. The
bars above are unchanged. The captions row is out of v1: v1 doesn't transcribe
(DECISIONS B168, ARCHITECTURE §16 #29).

*As built (DECISIONS B114):* what a take leaves out is measured from the pattern too. Its
**dark field** — what the pattern draws black, above its bar and clear of its strips — is
read from a frame (`shroom_testpattern::field_lit`; `tests/e2e/field.mjs`): the pattern
alone lights none of it, shroom's popover over it a few percent, the count's scrim all of
it. `node app/test/take-drive.mjs left-out` holds both above the pattern while a take
records — a capture that leaves nothing out must see them there — and every frame of the
take must have under 0.2 % of the field lit. `not-left-out` is the reader's own proof, as
`--guard-probe` is the pattern guard's: the count taken off the list, and the reader must
find it in the take. The pattern guard itself looks again for a few seconds on a slow Mac
before it refuses; nothing records until a captured frame shows the pattern.

The same pattern drives a **synthetic capture seam** for the fast layer, so the encode,
segment, pause and upload logic are tested without a screen. Playback is verified in
headless WebKit: the page loads, `currentTime` advances, the duration matches, no media
errors — plus ffprobe against the published playlist.

## 5. The layers

| Layer | Covers | Runs |
|---|---|---|
| **L0 unit** | `derive()`, laws, copy lint, request signing, segmenter, mint/member logic, the Worker's routes — fake clock, fs, network, capture | every change, seconds |
| **L1 contract** | the Worker under local workerd; Cloudflare's REST API faked from recorded real responses (scrubbed); an in-process S3 fake | every change |
| **L2 real machine** | real SCK + mic + camera, the dedicated test space, the media-truth harness, **the heartbeat** — on the hosted Mac (E2E full, `dev/mac/loop.sh`), or a Tart clone when that can't settle it; never Bagrat's Mac (§5) | every change to Mac-only code |
| **L3 faults & soak** | kill -9 at random points in capture, encode, upload, wiring → relaunch must derive the right state, nothing lost; full disk via a small mounted disk image; network loss via a fault proxy inside the uploader's transport seam (no system changes); a 2-hour take with pauses, CPU and memory tracked | per milestone |
| **L4 UI** | **the state gallery**: the frontend rendered in a browser with the core's IPC mocked, one screenshot per inventory state, light and dark; plus screenshots of the real app's windows | per region |
| **L5 clean VM** | Tart clone per run: the curl install, first-run permissions as a fresh principal, `shroom://` registration, update swap | per milestone — needs Bagrat's clicks |
| **L6 two Macs** | owner + member as two instances with isolated state dirs on one test machine (the hosted Mac or a clone — never Bagrat's Mac, §5); then the VM as a real second Mac (joins, revocation, tombstone-before-resume) | Region 6 |

*Where each layer stands at `6a623b0`* (DECISIONS B63–B70; BUILD-STATUS has the runs):
L1 runs the real Worker under workerd (a 10-contract subset) and an in-process Cloudflare
and S3 fake whose shapes are hand-written — **recording real responses (scrubbed) still
waits on Bagrat's checkpoint** (B63). L3's network faults live in the L1 fake's transport
(no proxy, no system change, B64 a); its kills are random in capture, upload and wiring
(B69), and the 15-minute and 2-hour soaks have run (B70). L5 runs on a throwaway Tahoe
clone without anyone's clicks (the permissions as a fresh principal still need them). L6
runs as two instances on one Mac (B65) — since 2026-10-14 a test machine, not Bagrat's (§5); the VM as a second Mac is still to come.

*Since (DECISIONS B114):* L5 can record. `tests/l5/take-vm.sh` gives the dev build leave to
record inside a throwaway clone — written into the clone's own privacy database, which the
golden images' switched-off System Integrity Protection allows; never on this Mac, and
never in a golden image — and runs the take drives there: a take from a cold start and
what it leaves out, Stop and a file that plays, the reader's own proof — and then the
overlays' own drive, since this is the one clone where Return in the popover can start a
count (B115 f): anywhere shroom may not record, Return is answered by “recording is off”.
The clone has no
camera and, its sound off, no microphone: the bubble, Camera as the source and the
microphone's signals stay with `take-drive.mjs signals` on the hosted Mac (OBS's pattern camera; since 2026-10-14 never Bagrat's Mac, §5).

*Since (DECISIONS B116):* L5 has an update. `tests/l5/update-vm.sh` runs the update drive
in a throwaway clone with the person in another app, every keyboard-side check required:
found, staged, offered, taken, and shroom back by itself with its Settings window in front
of that app and the keyboard left alone. The clone is given leave to record the same way
— where macOS has recording switched off, the mushroom's amber dot outranks the update's
Frost dot, and the drive would wait for a dot that never shows.

*Since (DECISIONS B117):* L5 walks setup to its end. `tests/l5/setup-vm.sh` gives the dev
build the microphone and leave to record the same way — the microphone's row beside the
screen's, each in the clone's own privacy database — and runs the onboarding drive there
with the person in another app and every keyboard-side check required: all-set's Invite
someone, where Settings takes the keyboard, the Dock icon staying with it (law 6, B138), and the name is typed
straight in; then every path of the drive in one command. A drive never asks macOS for
anything, so a clone without both stops at the permissions chapter; the asks themselves,
answered by a person as a fresh principal, stay with §8.

*The `shroom` command (spec N62; DECISIONS B215–B219):* **L0** — its unit tests, and
`tests/voice.rs` (every line it says is in `say.rs`); the core's `tests/cli.rs` (every
recording ask, in every moment of a take, answered once as the click it stands for; the
refusal's settings link: General at the switch, lit, never flipped, nothing mid-take), and
the enumeration sending that link from every face. **L1** — `record.rs` (the refusal's
three lines and `settings_link`, against a stand-in socket);
`crates/shroom-cli/tests/space.rs` on the real `worker.js` (`worker/dev/serve.mjs`: list,
find, rename, delete with its tombstone and seal, invites, removal, batches and their
re-runs); `cleanup.rs` on a stand-in for the page's cleanup suggestions; `flavor.rs` (a
bundle's command reads its own app's folder, through its link too); `site/test/cli.test.mjs`
(the guide held to `main.rs` and `out.rs`); the cask's renderer (`installer/test/cask.test.mjs`);
the windows gallery's General with the switch lit.
**L2** — on the hosted Mac, `cli-socket` (the bundled command, the socket's mode, set up
first, the switch's refusal, opening the app), `settings-link` (the refusal's link opened
with the app, cold and warm: Settings on screen at the switch, the switch still off) and the
`settings` drive's switch (kept in `prefs.json`, the ask let through and refused again, the
link's light on and off); and the full e2e's command leg (`tests/e2e/cli-leg.mjs`): on PATH
from the install line, refused until the switch is on (the link followed, nothing flipped),
then a take it starts, pauses,
resumes and stops, renamed and deleted, on the test account. A real `brew install` linking
the command waits for the first release that carries it (B217).

**The heartbeat:** record (test pattern) → Stop → the page is live → a recipient browser
plays it → title set via the owner cookie → delete → the tombstone face. Green from day
one; every region extends it.

**Where it runs (2026-10-14, [`dev/proposals/heartbeat-in-vm.md`](../dev/proposals/heartbeat-in-vm.md)
§3):** a change is tested in the cheapest place that can test it — a cloud session and CI
first; the hosted Mac only for Mac-only code (capture, record and encode, the app shell,
the updater and installer, signing), where the **E2E full** workflow is the heartbeat of
record; a Tart clone on Bagrat's Mac (`tests/l5/heartbeat-vm.sh`) only for what the hosted
Mac can't settle (a timing bar, a 4K display, a long soak); never Bagrat's Mac itself, which
carries only the official release. Every script that launches a dev build refuses
anywhere but a hosted runner or a Mac with SIP off (`tests/test-machine.mjs`).

**Real-app UI automation (probed 2026-10-02 — works):** Tauri's `tauri-driver` doesn't
support macOS, but WebdriverIO's embedded WebDriver plugin does — built into the app only
under a `webdriver` feature (DECISIONS B9). UI flows run against that build; anything that
captures still runs through LaunchServices under the dev app's own grants, since a binary
the harness spawns directly would ride the terminal's.

## 6. The dashboard — "is it going well?"

Regenerated at every milestone into **`BUILD-STATUS.md`**, and published with the state
gallery as one private page Bagrat can open from anywhere:

- **Heartbeat** — green/red, last N runs.
- **Spec coverage** — inventory states derived / rendered / copy-matched (out of 139);
  edges realized out of 125; laws with passing invariants out of 16. (Spec coverage, not
  line coverage.)
- **Media numbers** — the §4 table, trended.
- **Deviation log length** — growth means drift.
- **Scaffold flags** — the spec's list, each *open · probed · pinned*.

## 7. Tripwires — where "uninterrupted" stops

The build pauses and asks when:

- an implementation would change the visible behavior of a **ratified** state;
- a POC wall resurfaces (TCC behaving unlike E1/E2, the audio bug's signature);
- the heartbeat stays red across three consecutive commits;
- a step needs a machine change outside the pre-approved list (§9);
- anything would touch Bagrat's real space, push `main`, or spend money.

## 8. What only Bagrat can verify — the milestone checkpoint

Batched into one ~30-minute script per milestone instead of blocking on each:

- First-run permission clicks in the clean VM (granting security permissions is a human
  act), and the "bypass the system window picker" consent when it surfaces.
- Cloudflare signup and adding a card on a genuinely new account (the agent can't create
  accounts or enter card details).
- Feel: countdown timing, popover snappiness, the bubble, audio by ear.
- Ratifying gallery states and deviation entries.

Between checkpoints, what still needs hands — a session's on a held hosted Mac, or
Bagrat's — is kept in [`dev/mac/MANUAL.md`](../dev/mac/MANUAL.md), each check with when it
was last seen.

## 9. Pre-flight — everything before the run starts

### Decisions (Bagrat's)

| # | Decision | Outcome |
|---|---|---|
| D1 | N29–N42: build as provisional, or ratify first | **Provisional** (2026-10-01) — tagged in the fixtures; ratified from the rendered gallery at a checkpoint |
| D2 | Frontend for the app's webviews | **Svelte 5 + TypeScript** (2026-10-01) — the UI is a pure render of the derived state; markup-first, least ceremony, small bundles. *Rejected: SolidJS (JSX, no gain for a pure render), vanilla TS (dozens of states and faces by hand-rolled DOM), React (ceremony and weight for a tray app).* The Worker's page stays a hand-rendered template + hls.js (R4) |
| D3 | Where the test space lives | **A dedicated Cloudflare account** (test@shroom.am, ratified 2026-10-14 — [`dev/proposals/e2e-test-account.md`](../dev/proposals/e2e-test-account.md)), holding nothing but tests. **Guards, enforced in the harness, not by convention:** *pinned* — a run starts only if its token verifies against the e2e account id and that account's workers.dev name isn't `myshroom`; *the product's powers* — the account token's extra power (API Tokens Edit) mints one short-lived token per run with exactly the product's two, deleted on the way out, and the tests drive that one; *its own space* — each run gets a run id (`<unix>-<hex4>`) and sets up under it, the label and the bucket as a pair: `v-<run>` over `shroom-space-<run>` (a dev build's `--e2e` reads it from `~/.shroom-dev/e2e-space-id`; every other build, the release first, keeps `v` over `shroom-space`); its close deletes its own Worker, bucket (emptied first) and run token, nothing else, and a crashed run's leftovers are reaped by the age in their run id (≥ 45 min). Runs side by side — the two E2E workflows, a Mac's heartbeat — never touch each other's space, so there is no lock and no queue (DECISIONS B131). *The trade-off:* no E2E deploys the literal `v` any more — the same code, a different string; *the workers.dev name* (`shroom-e2e`) was claimed once, by Bagrat's go (2026-10-14), after the no-name state was recorded. A named account refuses any further claim (`10036`, whatever the name — probed that day), so the name can't be renamed or re-taken through the API. **Consequence:** `name` (once, done), wiring from nothing, `space-found` and `space-storage-gone` run live; `gate-email`, `gate-card`, `name-taken` and `name-changed` can't recur there and stay on the L1 fakes, held to the recorded answers (`crates/shroom-cloud/tests/fixtures/`). Main-account testing retired on 2026-10-14, the e2e suite green: its `*_live` tests were removed, the fake's contract with Cloudflare moved to `tests/e2e-live`, and the token deleted. **The pre-merge run** (DECISIONS B127, `e2e-full.yml`) is this account's main lane: a branch installed, set up, recording and played, with the video kept. *Superseded 2026-10-14:* Bagrat's main account, namespaced to `shroom-test-…` with the `shroom-wizard-probe` token (2026-10-02, when a new account couldn't be created) |
| D4 | The in-frame design calls (ffmpeg sidecar vs libav, updater wrapper, overlay home, scap pinning, min-OS floor, Intel) | *Default unless Bagrat objects:* decide each with a probe, log it in `DECISIONS.md`, review at the checkpoint |

### Bagrat's hands (one sitting, ~45 min)

| # | Item | Why |
|---|---|---|
| H1 | ~~Make the existing `shroom-wizard-probe` token (`~/.shroom/p3-token`) available to the harness at `~/.shroom-dev/test-account-token` — a symlink he creates himself, so the secret stays one file~~ | *Retired 2026-10-14 with main-account testing (D3): the tests that read it are gone and the token is deleted. The e2e token lives in the repo's gitignored `.secrets/e2e-account-token` (with the account id in `.secrets/e2e-account-id`), and the heartbeat runs on that account* |
| H2 | A **dev** signing identity (`shroom-dev`, E1 recipe) — separate from the future release key, whose custody is a release decision | TCC grants survive every rebuild (E1) |
| H3 | Grant Screen Recording, Microphone and Camera once to the dev app (`am.shroom.dev` in `~/Applications`) and clear the window-picker consent | L2 runs unattended from then on |
| H4 | Keep the Mac awake and on power for the run | A sleeping Mac stops the run |
| H5 | *(optional)* Remove `ShroomProbe.app` + its grant; delete the `shroom-wizard-probe` token — once the e2e suite is green and main-account testing retires (D3) | Probe leftovers. *2026-10-14: main-account testing retired and the token deleted. Left, optional: `ShroomProbe.app` and its grant, and H1's `~/.shroom-dev/test-account-token` link, which now points at nothing* |

### Machine changes to pre-approve

| # | Change | Reversal |
|---|---|---|
| M1 | Project-local npm dev dependencies (Tauri CLI, the frontend toolchain, vitest, Playwright, wrangler) + Playwright's WebKit download | `rm -rf node_modules`, Playwright cache |
| M2 | Cargo dependencies (Tauri, scap crates pinned by rev, cidre, a QR decoder) | `cargo clean`, registry cache |
| M3 | The `shroom-dev` keychain identity | `security delete-identity` |
| M4 | A Tahoe Tart base image (~30 GB) beside the Sequoia golden | `tart delete` |
| M5 | Small disk images mounted for disk-full tests | detached after each run |
| M6 | If the encode leg goes sidecar: a minimal ffmpeg built from source into the repo's build dir | delete the build dir |
| M7 | Work on a local branch `build/v1`; commits never pushed *(used; the branch was merged into `main` and deleted 2026-10-13 — code lives on `main` now)* | branch delete |
| M8 | CLAUDE.md's phase rules rewritten for the build phase | revert the commit |
