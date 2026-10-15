# Where each test runs: hosted Macs fanned out, a local VM for the rest, one manual suite

*Design note, 2026-10-13. Noted, not built. Bagrat asked for it to be noted, not
built, for now. Nothing below is decided until he says go.*

**In short.** Three places run tests unattended, and one suite needs a person:

1. **The hosted Macs (later).** There is no VM inside a runner (§0). For more at once, ask
   for more runners: one per scenario, and one per macOS version. Each runner is already a
   fresh, throwaway Mac.
2. **A local VM on Bagrat's Mac.** It runs what a runner can't: a clean Mac, the test
   account, two Macs at once, long runs. It should run *all* of that unattended. Today
   those tests run on his own Mac and need its screen unlocked.
3. **A separate, complete manual suite.** It holds only what no machine can do: first-run
   permission clicks, real hardware, real hands, feel and real accounts. Each row says why
   no lane can take it, so the suite shrinks as the lanes grow.

## 0. What was found: no VM inside a hosted Mac

Probed on a held runner (`dev/mac/loop.sh run --hold`, run 38404233123):

- The runner is itself a VM: `Apple M1 (Virtual)`, `VirtualMac2,1`, macOS 26.6.2, 3 cores,
  7 GB, 92 GB free.
- `kern.hv_support: 0`, `kern.hv_vmm_present: 1`. `VZVirtualMachine.isSupported` is false,
  and so is `VZGenericPlatformConfiguration.isNestedVirtualizationSupported`.
- Tart 2.40.1, from its release tarball: `tart create --linux` ends "Invalid virtual
  machine configuration. Virtualization is not available on this hardware."
- GitHub's runner reference says the same: nested virtualization isn't supported on arm64
  macOS runners (Apple's Virtualization framework).
- Even on a Mac we own, Apple allows two macOS VMs per host at once (the "two VM slots"
  in `dev/AGENT-RUNS.md`). Nesting could never beat more runners.

Also seen: the runner is **not a clean Mac**. Homebrew is installed, and so is
`/opt/homebrew/bin/ffmpeg`.

## 1. The hosted Macs, fanned out (later)

### 1a. One runner per scenario

- Today `mac.yml` runs the scenarios one after another on one runner. A second request on
  the same branch cancels the first (`concurrency: mac-${{ github.ref }}`,
  `cancel-in-progress`).
- A warm run spends about 1.5 minutes on setup, cache and build (the build alone is 33 s).
  `selftest` and `settings` together took 85 s. Splitting pays off once a batch runs longer
  than about 5 minutes. All 12 scenarios is such a batch.
- **Limits.** Standard runners are free on a public repo. The account can run 5 macOS jobs
  at once (Free, Pro and Team plans). CI's macOS jobs (`ci.yml`'s three, on pull requests
  and `main`) share that 5. Plan for 3–4.
- **What changes:**
  - `mac.yml`: a matrix over the requested scenarios, each runner building for itself
    (warm, that beats building once and passing the app around).
  - `loop.sh`: fetch N evidence artifacts and merge them into one `summary.md`.
  - The concurrency group: per request, not per branch.
  - `--hold` stays one Mac.

### 1b. One runner per macOS version

- `macos-15` (Sequoia) could run the lanes the Sequoia golden runs today. That works for
  every lane except those that need a clean Mac.
- **`macos-14`, the floor, while it lasts.** The floor (14.0) is measured by reading the
  binaries (`tests/l5/minos-probe.sh`, B83) and has never been run on a real macOS 14.
  GitHub retires `macos-14` on **2026-11-02**. Before then it fails on purpose 14:00 to
  00:00 UTC on Oct 16, 19, 23, 26, 29 and 30. This is a one-time window.
  - Build on `macos-26` and carry the `.app` to the `macos-14` runner. Whether that
    image's Xcode can build the app is unproven, and the release is built on 26 anyway.
    Whether `mac.sh prepare`'s grants hold for an app signed on another runner is to be
    proven.
  - After Nov 2, the floor needs a Sonoma golden locally. That is a machine change
    outside TESTING-PLAN §9, so it needs his yes (like item 27's Ventura VM).

### 1c. Intel runners exist

`macos-15-intel` and `macos-26-intel` (4 cores, 14 GB). B7 keeps v1 Apple-silicon-only
because "Bagrat can't test Intel". A hosted Intel runner changes that premise. It is still
his decision (BUILD-STATUS item 7).

### 1d. Candidates to move there

- **Real Safari playback.** Today that is §4 of the cloud skill. The runner has Safari;
  `safaridriver --enable` needs sudo, and the runner has it. Unproven.
- **`onboarding-drive.mjs invite`** ("not yet run anywhere: it needs this Mac unlocked",
  item 28). A runner's screen is never locked.

## 2. What needs Bagrat's Mac: the local requirements

What no hosted runner can give, and the tests that need it:

| Need | Why not a hosted runner | Tests |
| --- | --- | --- |
| **The test account's token** | The hosted Mac is public. Never a token there. | `tests/e2e-live`'s suite needs only the token, no screen: it runs headless, from `e2e.yml` on the test account. `tests/e2e/`'s `heartbeat`, `record-check`, `tone-check`, `soak`, `kill-relaunch`, `playback` and `pages-gallery` record through the dev app, so they need a screen, grants *and* the token (`ci.yml`'s "not run here" list). |
| **A clean Mac**: no Homebrew, no ffmpeg, no Xcode | The runner has all three (§0) | `tests/l5/installer-vm.sh`, `preview-vm.sh`, and the bundled encoder recording with no ffmpeg anywhere (B82) |
| **Two Macs on one space** | One runner is one Mac. Two runners can't share a local Worker. | `tests/l6/two-macs.mjs`, and "the VM as a real second Mac" (TESTING-PLAN §5 L6, still to come) |
| **Long runs, true timing** | 3 shared cores. `record` ends with 2 there (cloud skill §5). | `soak.mjs` (2 h), `av_sync.rs`'s three-minute drift run (ignored, by hand) |
| **Other macOS versions** | `macos-15` covers Sequoia. 14 only until Nov 2 (§1b). | the floor; any lane per version |
| **His real account** | Never on any test machine | the checkpoint, the preview build, `space-teardown`. These stay his alone. |

Already in a local VM today (Tart clones of `shroom-clean-tahoe`, 26.6.2, and
`shroom-clean`, 15.7.7, SIP off, always logged in and unlocked, cloned per run and deleted):
`tests/l5/dock-vm.sh`, `surfaces-vm.sh`, `take-vm.sh`, `installer-vm.sh` and `preview-vm.sh`,
plus `tests/checkpoint/clean-mac.sh` for hands. `dock`, `surfaces` and `take` also run
on the hosted Mac, and only the clean-Mac part is truly local.

## 3. The local VM: prepare an image, run the local tests in it

**Why.** Bagrat's Mac locks when he walks away. A locked screen gives the mushroom no
place, no window the keyboard, and a capture no frames (BUILD-STATUS, "Behind the locked
screen"). So the drives on his Mac need him at the desk. A clone is always unlocked and
has a privacy database of its own (`take-vm.sh` grants there, never on the host). Moving
the local tests into clones frees his Mac and turns "unseen here" into checks.

**Proposed shape:**

- **A dev golden, beside the clean ones.** The clean goldens stay clean: they are what
  the installer and the bundled encoder are proven on. The dev golden carries what the
  local suite needs:
  - node, the repo's `node_modules`, and Playwright's browsers;
  - the test pattern and the tools the drives call.
  
  The dev app keeps being built and signed on the host and copied in, as `take-vm.sh`
  does today, so the image needs no Rust and no identity.
- **One runner for every local lane.** It clones, grants (as `take-vm.sh`), runs the named
  tests, brings back evidence in the hosted Mac's format (`summary.md`, and per lane
  `video.mp4` and `output.log`), and deletes the clone.
  - It should take the same `dev/mac/scenarios/*.sh` files. One scenario then runs
    unchanged in three places: a hosted runner, a local clone, his Mac.
  - The L5 scripts become scenarios with a "clean" or "dev" image tag.
- **Two at a time** (Apple's limit). L6 takes both: owner and member as two clones.
- **The token in a clone.** The heartbeat and the recording lanes need the test account's
  token inside the VM. That means a copy in the clone's `~/.shroom-dev`, mode 600, deleted
  with the clone, never in a golden. This is a new use of a secret: **his call.** Without
  it, those lanes stay on his Mac, unlocked.
- **Machine change.** A new golden (~30 GB, like M4's) is outside TESTING-PLAN §9's
  M1–M8, so it needs his yes first. So does a Sonoma golden (§1b).
- **What a clone can't have** (so it goes to §4): a camera, a microphone (a VM nobody sits
  at has its sound off, B111), a notch, a second display, a lid and a battery, a real hand's
  events, and a first-run permission prompt answered by anyone but a person.

## 4. The manual suite: complete, and on its own

**Today.** Manual checks sit inside `docs/CHECKPOINT.md`'s 45-minute sitting, mixed with setup
(building the preview, the local site, clearing up). Others sit in BUILD-STATUS items
20–25 and 28, and in "unseen anywhere" lines. Galleries, copy and deviations to ratify
sit beside them, but those are reviews, not tests.

**Proposed.** One file, for example `MANUAL-SUITE.md`. Every row names one check, what to
do, how long it takes, what to send back, and **why no lane can take it**. A row leaves
when a lane can. It runs in one sitting, and `docs/CHECKPOINT.md` becomes setup plus this
suite. Ratifying stays a separate list.

The rows, as known on 2026-10-13:

- **Permissions, as a person.** The first-run permission clicks as a principal macOS has
  never seen (item 23, CHECKPOINT §10). The "bypass the system window picker" consent when
  it surfaces (TESTING-PLAN §8). The Automation prompt when the installer quits shroom from
  Terminal.app (item 22). A `shroom://join/…` link opened by a person (item 23).
- **Real hardware.**
  - The camera: the bubble, Camera as the source, ⌘H with a bubble on screen.
  - A physical microphone's delay, by ear: the clap (item 24).
  - The behind-the-notch anchor on a crowded menu bar (item 21).
  - A wide or second display.
  - A real lid-close on battery mid-take (item 21).
  - A real login launch's `lgit` mark (item 21).
- **Real hands** (a drive's events go on shroom's own queue). The popover's first click.
  The popover and the bubble following you to a full-screen app, through the Dock and
  Mission Control. The count over a full-screen app, and with the Library behind another
  app (item 25). The Library with no Dock icon, then ⌘Q mid-take with the Library in front
  (items 21, 28).
- **Feel.** Countdown timing, popover snappiness, the bubble, and sound by ear (item 25,
  CHECKPOINT §7). The Dock while a set-up shroom starts and when an update comes back
  (item 28).
- **Real accounts.** The wizard end to end on his account (item 20). On a fresh Cloudflare
  account, another day: the card gate's real code and a first workers.dev name claim.
- **Unseen anywhere so far.** After an update, the restored Settings window in front of
  the app the person is in. This is a candidate for a lane, not a hand check, once one can
  show it.

## 5. Order, when he says go

1. **Before Nov 2, if he wants the floor run:** the scenarios on `macos-14` (§1b).
2. **One inventory table.** Every Mac-only test and every hand check, with where it runs
   today and where it could run (hosted, local clone, manual). §2 and §4 are the first
   draft.
3. **The dev golden and the one local runner** (§3), after his yes on the image and the
   token.
4. **`MANUAL-SUITE.md`** (§4), cut from CHECKPOINT and BUILD-STATUS.
5. **The hosted fan-out** (§1a), then per-version (§1b) and Intel if B7 changes.

**His decisions in this:**

- the token in a throwaway clone (§3);
- a dev golden, and a Sonoma one after Nov 2 (machine changes);
- running the floor on `macos-14` before it goes;
- Intel (B7).

Sources: [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
[Actions limits](https://docs.github.com/en/actions/reference/limits),
[macOS 14 image retirement](https://github.blog/changelog/2026-10-01-github-actions-macos-14-runner-image-retirement).
