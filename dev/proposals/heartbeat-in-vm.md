# The heartbeat moves into a VM on this Mac

*Proposal, 2026-10-14. Ratified by Bagrat the same day, with §3 rewritten at his word: rely
on a Mac as little as possible ("only if mac-only-testable code is touched, otherwise
try to test all possible in cloud").*

**In short.** From 2026-10-14 this Mac carries only the official release, used the way a
person uses it. Nothing dev gets installed on it, and no lane launches a dev build on it.
The heartbeat is the last lane that still assumes the opposite: its preflight wants
`~/Applications/shroom-dev.app` on the host, with the host's grants. This proposal runs it
in a throwaway Tart clone instead, using the harness the hosted e2e run already uses, so the
local run and the CI run are the same code.

This builds the heartbeat's part of lane 2 in [`test-lanes.md`](test-lanes.md) (2026-10-13,
noted, not built): a local VM on Bagrat's Mac for what a runner can't settle. It also
narrows that lane to a last resort (§3).

## 1. What exists

- **The hosted e2e run** (`.github/workflows/e2e-full.yml`, PR #22) does the whole path on a
  GitHub-hosted macOS 26 runner:
  1. it installs through the real one-line installer (`dev/e2e/install.sh`);
  2. it opens the run (`e2e-run open`);
  3. it grants the app (`dev/mac/mac.sh prepare`);
  4. it runs setup, then the heartbeat, recorded (`mac.sh run e2e-full`);
  5. it closes the run.

  On that runner the timing bars are reported but not judged ("a Mac decides"), because
  hosted runners are noisy neighbours.
- **The L5 lanes** (`tests/l5/*-vm.sh`) already clone the `shroom-clean-tahoe` golden image
  headless and without sound. They write the screen-recording grant into the clone's own
  privacy database (SIP is off in the golden image), ship the bundle in with `tar` through
  `tart exec`, and delete the clone afterwards.
- **`dev/mac/lib.sh`** refuses to run anywhere but a GitHub-hosted runner (line 10). That is
  the only thing keeping `mac.sh` out of a Tart clone. The clone has the two properties the
  grants rely on: SIP off, and an always-logged-in, unlocked screen.

## 2. Proposed

A new lane, **`tests/l5/heartbeat-vm.sh`**, which is the e2e-full job in a Tart clone on this
Mac:

1. **Build on the host**, as today: `npm run build:dev --workspace app`, plus
   `cargo build -p shroom-e2e-live --bin e2e-run`. Building here is fine. Only installing
   and launching moves out.
2. **Clone** `shroom-clean-tahoe-e2e` (see §4) and run it `--no-graphics --no-audio`. Wait
   for the desktop as `take-vm.sh` does.
3. **Ship the tree**: `git archive HEAD`, plus the built bundle, `target/debug/e2e-run` and
   `node_modules`. Everything goes through `tart exec -i … tar -x`, never a shared folder.
4. **Open the run on the host**:
   `target/debug/e2e-run open <run-token file>`, with the account token read from the
   repo's `.secrets/`. Only the **run** token (two hours, the product's two powers) goes
   into the clone, as a mode-600 file the app removes once read, as in CI. The account
   token never leaves the host.
5. **In the clone**: `dev/e2e/install.sh`, then `dev/mac/mac.sh prepare`, then
   `dev/mac/mac.sh run e2e-full`. This is unchanged except for `lib.sh`'s guard (below).
6. **Bring the evidence out**: `results.tsv`, the summary, the drives' reports, the screen
   recording and the heartbeat's JSON.
7. **Close the run on the host** (`e2e-run close`), pass or fail, then delete the clone.

**`dev/mac/lib.sh`** accepts a second kind of machine: a Tart clone the lane has marked.
The lane writes `/etc/shroom-l5-clone` (root-owned) after checking `csrutil status` reports
disabled. `RUNNER_TEMP` is set to a folder in the clone. The guard keeps refusing everywhere
else, this Mac included.

**`heartbeat.mjs`** needs no change: inside the clone, `~/Applications` is the clone's own.

**Built (2026-10-14), and where it differs from the text above:**
- `tests/l5/build-e2e-image.sh` derives the image, and `tests/l5/heartbeat-vm.sh` is the
  lane.
- The tree goes in by `git archive HEAD` plus the built bundle and `node_modules`, so the
  lane runs the committed tree.
- In the clone, `MAC_JUDGE_BARS=1` fails a missed bar. `MAC_WATCH=0` leaves out the
  screen video, because the clone's recorder has no grant of its own. `E2E_SHOW=0` leaves
  out the show pass, because the clone has no camera for the bubble. The hosted run's
  defaults are unchanged.
- The guard for the host-launching scripts is `tests/test-machine.mjs`, plus a one-line
  copy in the two shell launchers. It accepts a GitHub-hosted runner, or any Mac with SIP
  off: the runner, or a golden image's clone. Bagrat's Mac keeps SIP on. So the existing
  Tart lanes needed no marker, only the guard file shipped into their clones. The marker
  `/etc/shroom-l5-clone` is what `dev/mac/lib.sh` and `dev/e2e/install.sh` ask for on top
  of SIP off, because they write grants and install.

## 3. Where a change is tested: the cloud first, a Mac only when it must

A change is tested in the cheapest place that can actually test it. It moves down this
list only when the place above can't:

1. **A cloud session and CI (Linux).** This is the default for every change: the crates,
   the Worker under workerd, the webviews, the fixtures, the media readers. The
   shroom-cloud skill's §1 lists what runs there, and the live e2e tests on the test
   account need no Mac.
2. **The hosted Mac** (GitHub's macOS 26 runner). Use it when a change touches
   **Mac-only code**, meaning anything the shroom-cloud skill's §4 lists:
   - capture (ScreenCaptureKit, camera, microphone, permissions, tray, bubble, countdown);
   - recording and the encoder (`shroom-capture`, `shroom-record`, `shroom-encode`);
   - the app shell (`app/src-tauri`, its drives);
   - the updater and the installer (`shroom-update`, `installer/`,
     `tools/package-release.sh`, `tools/build-ffmpeg.sh`);
   - signing.

   `dev/mac/loop.sh` runs the dev app there with the screen recorded, and e2e-full runs
   the whole path, the heartbeat included. A cloud session asks for both by itself.
3. **This lane, a Tart clone on Bagrat's Mac.** Only when the hosted Mac can't settle the
   question. Such cases are few: a timing bar that only a quieter machine decides (the
   hosted runner reports bars but doesn't judge them), a 4K display, a long soak past a
   job's time limit, or the hosted runner being down. When it does run, its bars are judged
   against the host figures of 2026-10-14 (keyed page 950 ms, A/V worst 12.1 ms, 0.47
   cores, repeated 0.32 %). A systematic miss goes to the deviations in `docs/DECISIONS.md`, never quietly
   loosened.
   **As measured (2026-10-14, DEVIATIONS #49):** five clone runs missed two bars every
   time, skipped frames and Stop → the keyed page, because of the virtual display and the
   guest's network stack. Bridged networking and more cores didn't help. In the clone,
   those two are judged against its own alarms (2.5 %, 3 s), which catch a gross
   regression. A miss of the product's bar is reported. The other five are judged as they
   are.
4. **Bagrat's own hands** on the official release: the feel of it, sound by ear, a real
   update arriving. That's a person using the product, not a test lane.

So the heartbeat of record is **the hosted e2e-full run**, and it runs on any change that
touches Mac-only code. A change outside that list never needs a Mac at all. RELEASING's
"Cutting a release" step 2 becomes: the hosted e2e-full run is green on the release's
commit, and the local lane runs only if a bar on it needs a verdict.

## 4. Machine changes (ratified 2026-10-14)

- **A derived golden image, `shroom-clean-tahoe-e2e`**: `shroom-clean-tahoe` plus node 22,
  Homebrew's `ffmpeg` (with `ffprobe`; the tree's own encoder build disables ffprobe),
  `cliclick`, and Playwright's WebKit and Chromium. That's about 1–2 GB beside the 50 GB
  base. Baking these in saves ~300 MB of downloads and minutes on every run. M4
  pre-approved a Tahoe base image; this is a second image, so it's asked here.
- No change to this Mac itself: no grants, no install, no home folder. That's the point.

## 5. What else is touched

- **Host-launching drives**: `app/test/*-drive.mjs` default `SHROOM_APP` to the built bundle
  and launch it on the host. Each gets a guard that refuses outside a clone or a runner, the
  way `lib.sh` does, so nothing launches a dev build here by accident. Their VM lanes
  (`take-vm.sh`, `surfaces-vm.sh`, …) are unaffected.
- **Docs**:
  - RELEASING "Cutting a release" step 2: "Run what only this Mac can" becomes "the hosted
    e2e-full run is green" (§3).
  - TESTING-PLAN §5 and the L2 row: "real machine" becomes "the hosted Mac; a clone on
    Bagrat's Mac only when it can't settle it".
  - The shroom-cloud skill's §4: "What needs the Mac" points at §3's order, and tells a
    session to try the hosted Mac before ever asking for the local lane.
  - BUILD-STATUS "Heartbeat" gets a column for where each run ran.
- **What a VM can't do**: a microphone or a camera (the clone has neither). The hosted run
  keeps those: OBS's pattern camera and the bubble in e2e-full's show pass. The attended
  microphone check stays the checkpoint's.

## 6. Bagrat's calls — answered 2026-10-14

1. The lane as proposed: **yes**.
2. The derived golden image `shroom-clean-tahoe-e2e`: **yes**.
3. Bars judged in the VM as the run of record: **no**. Rely on a Mac as little as possible.
   The cloud comes first, then the hosted Mac for Mac-only code; this lane is a last
   resort (§3).
