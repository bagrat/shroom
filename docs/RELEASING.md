# Releasing shroom

A release is a GitHub release of `bagrat/shroom` carrying exactly three files:

- `manifest.txt` — the version, the macOS floor, the zip's name and its SHA-256;
- `update.txt` — the same file under the name the app's updater asks for, so the release's
  download counts tell the app's checks from the install line's runs (DECISIONS B136);
- `shroom-<version>-arm64.zip` — the signed app.

The install line, the app's own updater and the Homebrew cask read nothing else
(`releases/latest/download/manifest.txt` — `update.txt` for the app, which falls back to
`manifest.txt` on a release without one — then `releases/download/v<version>/<zip>`;
DECISIONS B29, B31). You make one by pushing a tag: `.github/workflows/release.yml` builds,
signs, packages and publishes it. Nothing is ever uploaded by hand.

Two things to know before anything else:

- **The signing certificate is the app's identity, forever.** macOS keeps each person's
  Screen Recording permission by the app's identifier *and the certificate that signed it*
  (RESEARCH §5b, E1/E2). Sign one release with a different certificate and every user is
  asked for Screen Recording again. The repo pins the certificate
  (`tools/release-identity.sha1`), and the workflow refuses to build with any other.
- **CI has run; the release workflow hasn't.** CI has run on every push of `main` since
  2026-10-12 — six finished runs by the 13th. Its release build and its signing rehearsal were
  green in every one. Its Tests job was red in every one, at the Rust workspace: the
  recorder's real-time tests don't hold their pace on GitHub's runner. No tag has been
  pushed and no rehearsal started, so `release.yml` itself has never run. "What has not
  been proven" at the end says what those runs settled and what is still open.

## One-time setup

1. Get the workflows onto GitHub: push `main`. (Pushing `main` is always your go —
   CLAUDE.md.) CI starts by itself on that push. *(Done: they have been on `main` since
   2026-10-12; the `build/v1` branch they were written on is merged and gone.)*

2. Make the release identity. Run this once, on your Mac, in Terminal:

   ```bash
   tools/make-release-identity.sh
   ```

   It makes a self-signed code-signing certificate named `shroom release` (valid twenty
   years), puts it in your login keychain, and writes five files to
   `~/.shroom/release-identity/`. It prints no secret, changes no trust setting, and
   refuses to run a second time under the same name.

3. Make the GitHub environment the secrets live in. On github.com: the repo → **Settings**
   → **Environments** → **New environment** → name it `release` → **Configure
   environment**. Or:

   ```bash
   gh api --method PUT repos/bagrat/shroom/environments/release
   ```

4. Add the two secrets to that environment. On the environment's page: **Environment
   secrets** → **Add environment secret**, once for each name below — or let `gh` read the
   values straight from the files, so they never show on screen:

   ```bash
   gh secret set RELEASE_CERT_P12_BASE64 --repo bagrat/shroom --env release < ~/.shroom/release-identity/shroom-release.p12.base64
   ```

   ```bash
   gh secret set RELEASE_CERT_PASSWORD --repo bagrat/shroom --env release < ~/.shroom/release-identity/shroom-release.password
   ```

   | Secret | Its value |
   |---|---|
   | `RELEASE_CERT_P12_BASE64` | the whole contents of `shroom-release.p12.base64` (one long line) |
   | `RELEASE_CERT_PASSWORD` | the whole contents of `shroom-release.password` |

   They are *environment* secrets, not repository secrets: only a job that names the
   `release` environment can read them, and no pull request's job does.

5. Pin the certificate in the repo, then commit and push that one file. (The installer
   carries the same pin as `RELEASE_LEAF` in `installer/install.sh`, and the Homebrew cask
   as `leaf` in `installer/homebrew/shroom.rb`; their tests hold the three equal, so a new
   certificate means changing all three — DECISIONS B125, B174.)

   ```bash
   cp ~/.shroom/release-identity/shroom-release.sha1 tools/release-identity.sha1
   ```

   ```bash
   git add tools/release-identity.sha1
   ```

   ```bash
   git commit -m "chore: pin the release certificate"
   ```

   ```bash
   git push origin main
   ```

   The SHA-1 is public — it is inside every release's signature. Without this file the
   release workflow stops at its first job.

6. Back the identity up, offline. Copy `shroom-release.p12` and `shroom-release.password`
   to two places that are neither this Mac nor GitHub (a password manager entry with the
   file attached, and an encrypted USB stick, say), and `shroom-release.cer` with them —
   it is public, and signing a release by hand on this Mac asks for it ("If signing fails
   on the runner"). GitHub never shows a secret again, so the secret is not a backup. Lose
   both the keychain and the backup, and the next release has to be signed with a new
   certificate: every user re-grants Screen Recording, and nothing can undo that.

7. Remove the working copy from this Mac once the secrets are set and the backup exists:

   ```bash
   rm -r ~/.shroom/release-identity
   ```

8. *(Optional — your approval before each release.)* On the `release` environment's page:
   **Required reviewers** → add yourself, and leave **Prevent self-review** unticked (you
   are the only reviewer). Each release's build then waits for your click in the run's page
   before it can read the secrets.

9. *(Optional — who may release.)* Still on the environment's page: **Deployment branches
   and tags** → **Selected branches and tags** → add the tag rule `v*`, and the branch
   `main` if you want to use the rehearsal in step 11. And under **Settings** → **Rules** →
   **Rulesets** → **New tag ruleset**: target `v*`, restrict creations, updates and
   deletions, with yourself on the bypass list — so nobody else can push a release tag, and
   a published version's tag can't be moved.

10. Look at CI on `main`: all four jobs green — **Tests**, **Release build (ad hoc)**,
    **Signing rehearsal**, **Advisories**. The rehearsal does everything a release does to
    sign, with a throwaway certificate. (As of 2026-10-14 all four are green on pull
    requests; **Tests** leaves the A/V timing bars to a Mac as warnings — "What has not
    been proven".)

11. Rehearse with the real identity, publishing nothing: **Actions** → **Release** → **Run
    workflow** → branch `main`. It builds and signs exactly as a tag would, shows the
    signature and the SHA-256s in the run's summary, keeps the two files as an artifact
    for a week, and skips the publish job.

## Cutting a release

A release ships the version the tree is at, and a version is released once. The commands
below are written for **0.1.4**: where the tree stands as this is written (2026-10-13),
and not out when it was written. (The first release was **0.1.5**, 2026-10-13 — "What has
not been proven". 0.1.4 was never released.) Read your version for it.

1. See which version the tree is at. This prints it, and stops if the seven files that
   hold it disagree:

   ```bash
   tools/bump-version.sh --check
   ```

   If that version isn't out yet — as 0.1.4 isn't — there is nothing to move: its move is
   already committed (`chore: shroom 0.1.4`). Go on to step 2.

   If it is out (it is listed under **Releases** on github.com), move the tree to the next
   one first — every file that holds it, together. It must go up; the script refuses a
   version the tree already has, or a lower one:

   ```bash
   tools/bump-version.sh 0.1.5
   ```

   and read 0.1.5 for 0.1.4 from here on.

2. Write the release's notes: `docs/release-notes/<version>.md`, a short list of what
   someone using shroom would notice, one line each, in shroom's voice — no decision
   numbers, files, pull requests or internal names (DECISIONS B152). Copy the newest file's
   shape. They are what "What's new" opens, and product copy, so they go in by a PR you
   read. Check them:

   ```bash
   node tools/release-notes.mjs check 0.1.4
   ```

   Merged to `main`, they are also shroom.am's page for that version
   (`shroom.am/whats-new/<version>`, what the app's "What's new" opens; B153). The release
   refuses to build without them. Everything else that changed is added for
   you, folded under "Under the hood": GitHub's list of what was merged since the last
   release.

   Notes written or corrected after a release went out are put on it by **Actions** →
   **Release notes** → **Run workflow** on `main` with the version: it rewrites that
   release's body from the file and touches nothing else.

3. See the heartbeat green on the release's commit — the hosted e2e-full run (Actions →
   **E2E full** → Run workflow on `main`): installed, set up, recorded, played, a member,
   delete, on the test account. Nothing runs on your Mac
   (`dev/proposals/heartbeat-in-vm.md` §3). A timing bar it reports but can't judge is
   settled, only if it matters, in a Tart clone (`tests/l5/heartbeat-vm.sh`).

4. If you moved the version in step 1, or wrote the notes in step 2, commit them:

   ```bash
   git add docs/release-notes && git commit -am "chore: shroom 0.1.5"
   ```

   Then push `main`:

   ```bash
   git push origin main
   ```

5. Wait for CI to go green on that commit. It is also what makes the release build fast:
   its build job saves the caches the release run starts from.

6. **Actions** → **Release** → **Run workflow** → branch `main`, tick **Publish**, **Run
   workflow** (DECISIONS B126). The run refuses unless CI is green on that exact commit
   and the version isn't tagged yet; then it builds, signs, and publishes `v<version>`,
   making the tag on the commit it built. No tag to type, nothing to push.

   *(The old road still works: a tag pushed by hand publishes too —
   `git tag v0.1.4`, then `git push origin v0.1.4`. If a tag ruleset (step 9) lets only you
   create `v*` tags, the workflow's own tag is refused at the publish step — add
   **GitHub Actions** to the ruleset's bypass list, or push the tag by hand.)*

   *(If step 9 limited the `release` environment's deployment branches and tags, `main`
   must be on that list for a run from `main` to reach the signing secrets.)*

7. Watch the run. If you turned on required reviewers, the **Build,
   sign, package** job waits for **Review deployments** → **Approve and deploy**.

8. When the run is green, check the release (next section).

9. Bring Homebrew's copy up to it: on github.com, `bagrat/homebrew-shroom` → **Actions** →
   **Bump** → **Run workflow**. It runs by itself every hour anyway; running it now puts
   the release in `brew install --cask bagrat/shroom/shroom` at once, and wakes the hourly
   run if GitHub paused it after 60 quiet days. Its run ends "shroom <version> (from
   bagrat/shroom@…)" in the tap's commits, or "Already the latest." (Homebrew below.)

If the run fails before the publish job, nothing was published: fix what it says, and
either re-run the failed job or (a hand-pushed tag) delete the tag, fix, and tag again. A version that *was*
published is never replaced — installed apps already know its fingerprint. Bump and
release the next one.

## Checking a published release

What installs it checks it too (DECISIONS B125): the installer runs `codesign --verify` on
every download and, from the public releases, takes only a `shroom.app` signed as
`am.shroom` by the certificate `tools/release-identity.sha1` pins; the app's updater swaps
a staged copy in only if it meets the installed app's own designated requirement. A
release signed by anything else is never installed — so the pin must stay the release
identity's (One-time setup, step 5).

The publish job already asks the public address for both files and compares them with what
it built. To see for yourself:

1. The manifest the install line reads:

   ```bash
   curl -fsSL https://github.com/bagrat/shroom/releases/latest/download/manifest.txt
   ```

2. The zip, and its fingerprint — it must be the one the manifest's `arm64=` line ends
   with, and the one in the run's summary:

   ```bash
   curl -fsSLO https://github.com/bagrat/shroom/releases/download/v0.1.4/shroom-0.1.4-arm64.zip
   ```

   ```bash
   shasum -a 256 shroom-0.1.4-arm64.zip
   ```

   Don't unzip it on this Mac: an unpacked release app answers for `shroom://` links, and
   your Mac carries only the installed official release (DECISIONS B66).

3. The install line itself, on a clean Mac — a throwaway clone of the Tahoe image:

   ```bash
   tart clone shroom-clean-tahoe shroom-release-check
   ```

   ```bash
   tart run shroom-release-check
   ```

   In the VM's Terminal, the real installer against the real release:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/bagrat/shroom/main/installer/install.sh | sh
   ```

   (Once the landing page serves it, `curl -fsSL https://shroom.am/install | sh` is the
   same script.) Then, still in the VM, who signed what was installed — the last line must
   end with the SHA-1 in `tools/release-identity.sha1`:

   ```bash
   codesign -d -r- ~/Applications/shroom.app
   ```

   And when you are done:

   ```bash
   tart delete shroom-release-check
   ```

### Homebrew

`brew install --cask bagrat/shroom/shroom` installs from the tap `bagrat/homebrew-shroom`
(DECISIONS B174). Its `Casks/shroom.rb` is never edited there: the tap's **Bump** workflow
renders it from this repo's `installer/homebrew/shroom.rb` with the latest release's
version and SHA-256 (`installer/homebrew/render-cask.sh`), every hour and on demand
(Cutting a release, step 9). A change to the cask itself is a PR here, and reaches the tap
within the hour of its merge. To see what the tap should hold now:

```bash
installer/homebrew/render-cask.sh
```

The cask does what the install line does: the same zip and fingerprint, placed in
`~/Applications`, taken only if signed by the pinned certificate (its `leaf`, held equal to
`tools/release-identity.sha1` by `installer/test/cask.test.mjs` — a new certificate changes
it too), and its quarantine mark lifted after that check, since a self-signed app marked as
downloaded is refused at first open. The hosted Mac's `brew-cask` scenario runs it for real
against the latest release:

```bash
bash dev/mac/loop.sh run brew-cask
```

## What the two workflows do

Everything that builds, tests or signs runs on GitHub's `macos-26` runner (Apple silicon,
the same macOS as this Mac): CI's three Mac jobs and the release's **Build, sign, package**.
CI's **Advisories** only reads the lockfiles, so it runs on `ubuntu-24.04`.
The release's two other jobs touch no Mac tool and run on `ubuntu-24.04`: the tag check
before the build, and the publish after it. Both workflows build through one script,
`tools/release-build.sh`, so the app CI checks is the app a tag ships. Every action is
pinned to a commit.

**CI** (`.github/workflows/ci.yml`) — every push to `main`, every pull request:

| Job | What it does |
|---|---|
| Tests | the spec's readable form current (`tools/render-machine-md.py --check`), the Rust workspace, the Worker (unit suite, typecheck, the contract under workerd), the installer, the landing page, the agent skill, the media readers, the frontend's build and typecheck |
| Release build (ad hoc) | the release build with no identity (signed ad hoc), every bundle check, packaged as a stand-in. From `main` it saves the caches a tag needs |
| Signing rehearsal | makes a throwaway identity with `tools/make-release-identity.sh`, imports and trusts it as a release would, signs a file and checks macOS would keep a permission by it |
| Advisories | `cargo audit` against RustSec and `npm audit --audit-level=high` (DECISIONS B125): a known vulnerability in a pinned dependency fails it; an unmaintained-crate notice is listed, not fatal |

**Release** (`.github/workflows/release.yml`) — a pushed tag `v*`:

| Job | What it does |
|---|---|
| The tag is the tree's version | the tag equals the version in all seven version files; the commit is on `main`; the certificate is pinned; the version's notes are written (a rehearsal only warns) |
| Build, sign, package | restores the caches, builds the encoder only if its cache missed, imports the identity into a throwaway keychain (deleted whatever happens), checks it is the pinned certificate, runs the release build, packages it |
| Publish the release | the only job that can write to the repo: creates the release with its files and its notes — `docs/release-notes/<version>.md` under "What's new", GitHub's generated list folded under "Under the hood" (`tools/release-notes.mjs`, B152) — then fetches the files back from the public address |

### How long

Measured on this Mac (8 cores, 2026-10-12), with the same scripts:

| | Time |
|---|---|
| The release build from nothing (`tools/release-build.sh`, an empty target dir) | 95 s, and 83 s on a second run |
| The same after touching one of the app's source files | 23 s |
| The same with the dependencies already compiled and every one of shroom's own crates rebuilt — what a restored cache leaves to do | 38 s |
| The encoder from source (`tools/build-ffmpeg.sh`, tarball already downloaded) | 48 s |
| The tests' ffmpeg and ffprobe from source (`tools/build-ffmpeg-oracle.sh`) | 98 s |
| The Rust workspace's tests from nothing (a debug build of everything, then the tests — 520 of them now) | 161 s |

GitHub's standard macOS runner has fewer cores and a cold disk. CI's run of 2026-10-13 on
`e47e5e6`, as it found its caches: the release build job 1 min 56 s (the build itself
68 s), the signing rehearsal 1 min 17 s, the Tests job 7 min 12 s — 5 min 36 s of it the
Rust workspace, to its failure. A release run has not been measured: as a guess, ten to
twenty minutes with nothing cached and around five once the caches are warm. Each run's
page shows every step's time.

## Caching

What is cached, and under which name:

| Cache | Saved by | Name starts with |
|---|---|---|
| Compiled Rust dependencies and cargo's registry, release profile | CI's build job, from `main` | `v0-rust-release-v1-macos-26-` |
| The same for the test (debug) profile | CI's tests job, from `main` | `v0-rust-tests-macos-26-` |
| The encoder (`build/ffmpeg/sidecar`) | CI's build job, from `main` | `ffmpeg-sidecar-macos-26-` |
| The tests' ffmpeg and ffprobe | CI's tests job, from `main` | `ffmpeg-oracle-macos-26-` |
| npm's download cache | CI's build job, from `main` | `npm-macos-26-node22-` |

How it behaves:

- **A tag's run reads `main`'s caches and nothing else.** GitHub scopes a cache to the
  branch or tag that saved it; a run can read its own ref's caches and the default
  branch's. A cache saved by one tag's run would be invisible to the next tag — so the
  release workflow saves nothing, and CI saves from `main` only. A pull request's run reads
  `main`'s caches and saves nothing either.
- **Only dependencies are kept.** shroom's own crates are compiled on every run (the 38 s
  row above); the hundreds of crates they depend on are not.
- **A cached encoder is still checked.** Its cache is named by the hash of
  `tools/build-ffmpeg.sh` and the macOS floor; on a hit the build is skipped and
  `tools/build-ffmpeg.sh --check` holds the binary to the script's own checks and to the
  digest recorded when it was built.
- **GitHub's limits:** 10 GB of caches per repository (the least recently used go first
  when it is over), and any cache nobody read for 7 days is removed. After a quiet week
  the next run is a cold one — CI on `main` rebuilds and saves everything again. To warm
  them before a release without pushing anything: **Actions** → **CI** → **Run workflow**
  on `main`.
- **A new Xcode starts a new Rust cache.** The Rust caches are also named by the Xcode
  the image builds with (one dependency's build keeps that Xcode's path), so the run after
  GitHub changes the image's default Xcode is a cold one.
- **So can any refresh of the runner image.** GitHub rebuilds `macos-26` every week or
  so. The Rust caches are named by the compiler too, and by every environment variable
  the runner sets whose name starts with `CARGO`, `RUST`, `CC`, `CXX`, `CFLAGS` or `CMAKE`
  — so a refresh that changes one of those starts a cold cache, with nothing changed in
  this repo. The image's own Rust is the usual way that happens, and the one way it
  can't here: the workflows install and build with the Rust `Cargo.toml` names
  (`rust-version`), whatever the image came with. Raising `rust-version` is a new
  compiler, and a cold cache, by design. If a run on `main` is unexpectedly slow, its
  Rust cache step says "No cache found" — and that run saves the new one.
- **The two workflows must stay in step.** The `env` block at the top of each file, the
  runner image, the Rust cache's `shared-key` and the encoder's cache key are the same
  text in both; change one and the tag's build silently starts cold. The app's test
  `the_tag_workflow_builds_what_ci_cached` fails when they drift.

To look at the caches, or clear them (the next CI run on `main` rebuilds them):

```bash
gh cache list --repo bagrat/shroom
```

```bash
gh cache delete --all --repo bagrat/shroom
```

On github.com the same list is under **Actions** → **Caches**. To abandon the Rust cache
without deleting anything, change `RELEASE_CACHE: release-v1` to `release-v2` in *both*
workflow files.

## What CI does not cover

Recording needs a Mac with permissions, and `ci.yml`'s runner has none. None of these run
on your Mac any more (`dev/proposals/heartbeat-in-vm.md` §3) — the order is the cloud
first, the hosted Mac for Mac-only code, a Tart clone only when the hosted Mac can't settle
it — and a green `ci.yml` says nothing about them:

- **The heartbeat** and the other end-to-end lanes in `tests/e2e/` — a real recording
  through the installed dev app: the hosted Mac's **E2E full** workflow, or
  `tests/l5/heartbeat-vm.sh`.
- **The app's window drives and galleries** (`app/test/`) — the hosted Mac
  (`dev/mac/loop.sh`), or their Tart lanes.
- **The clean-Mac lanes** (`tests/l5/`) — a Tart VM and the
  `shroom-dev` identity; and **two Macs** (`tests/l6/`).

Every script that launches a dev build refuses anywhere but a hosted runner or a Mac with
SIP off (`tests/test-machine.mjs`).
- **The live Cloudflare tests** — they skip themselves without `SHROOM_E2E=1`; `e2e.yml`
  runs them on the test account.
- **Permissions as a fresh principal**, the feel of the thing, sound by ear — the
  checkpoint's territory (TESTING-PLAN §8).

One more, decided by the runner rather than by us: the Rust media tests encode H.264
through VideoToolbox. CI asks the runner first whether it can; if it can't, those test
files are left out of that run, named in a yellow warning and in the job summary — never
silently. If you see that warning, the media tests ran only on your Mac.

## If signing fails on the runner

codesign signs only with a certificate the signing Mac trusts for code signing, so the
workflow tells the runner — a throwaway virtual Mac — to trust yours for the length of the
job. That step (`.github/scripts/signing-keychain.sh`) is the one piece that could only be
written, not tried: it needs administrator rights on a runner. CI's **Signing rehearsal**
job runs it on every push with a throwaway certificate, so you will see it fail there long
before a tag does.

If it does fail and a release can't wait, sign on this Mac instead — once, by hand:

1. Get the certificate alone, as a file: `shroom-release.cer` from the backup (setup
   step 6) — or, if it isn't to hand, the same file out of your login keychain:

   ```bash
   security find-certificate -c "shroom release" -p ~/Library/Keychains/login.keychain-db | openssl x509 -outform DER -out shroom-release.cer
   ```

   Either way, check it is the pinned certificate — this prints the same forty characters
   `tools/release-identity.sha1` holds:

   ```bash
   shasum shroom-release.cer
   ```

2. Trust it for code signing here (macOS asks for your password; this changes your trust
   settings, which is why no script does it for you):

   ```bash
   security add-trusted-cert -p codeSign -k ~/Library/Keychains/login.keychain-db shroom-release.cer
   ```

3. Build with it into a throwaway folder that Spotlight skips. (Until step 5 removes it,
   that folder holds an app that answers for `shroom://` links — don't leave it lying.)

   ```bash
   CARGO_TARGET_DIR=/tmp/shroom-release.noindex/target tools/release-build.sh /tmp/shroom-release.noindex/out --identity "shroom release" --leaf "$(cat tools/release-identity.sha1)"
   ```

4. Package it:

   ```bash
   tools/package-release.sh /tmp/shroom-release.noindex/out/shroom.app 0.1.4 /tmp/shroom-release.noindex/dist
   ```

5. Publish it (the tag must already be pushed), then remove the folder:

   ```bash
   gh release create v0.1.4 --repo bagrat/shroom /tmp/shroom-release.noindex/dist/shroom-0.1.4-arm64.zip /tmp/shroom-release.noindex/dist/manifest.txt --title "shroom 0.1.4" --notes-file <(node tools/release-notes.mjs body 0.1.4 <(gh api repos/bagrat/shroom/releases/generate-notes -f tag_name=v0.1.4 --jq .body)) --verify-tag
   ```

   ```bash
   rm -r /tmp/shroom-release.noindex
   ```

## Still yours to decide

- **The release identifier on Macs that had the POC** (DECISIONS B66): the release is
  `am.shroom`, the identifier the POC used, so on those Macs it meets the POC's leftovers.
  Nothing here changes that; decide before the first public release.
- **When to push `main`** — each push is your go (CLAUDE.md).
- **Approval per release and tag protection** — steps 8 and 9 above. Off until you turn
  them on.
- **Which Xcode builds the release.** The runner's default is Xcode 26.6; this Mac has
  Xcode 27. The app's macOS floor is the same either way (it is checked), but an app is
  also shaped by the SDK it was linked against, so a release may look or behave slightly
  unlike the dev builds you have been running. GitHub offers an `xcode-27` image as a
  preview; switching means changing `runs-on` and `RUNNER_IMAGE` in both workflows.
- **Whether a release must wait for green CI.** The routine says wait; the release
  workflow doesn't enforce it.
- **Keeping the caches warm on a schedule** (a weekly CI run on `main`) — not set up; a
  quiet week just means one slower run.

## What has not been proven

**Settled by CI's first six runs on `main`** (2026-10-12 and 13 — the same in each):

- `macos-26` has what CI's scripts assume — `rustup`, Node 22 for `setup-node`, Xcode's
  command-line tools, `python3`;
- the runner **trusts a self-signed certificate** by the road the script takes (`sudo
  security authorizationdb` and `add-trusted-cert`): the Signing rehearsal is green;
- the release build passes its checks when linked against the runner's SDK, the floor
  probe (`tests/l5/minos-probe.sh`) included: Release build (ad hoc) is green — also with
  the bundle born a menu-bar app (`LSUIElement`, DECISIONS B113);
- **VideoToolbox encodes on the runner**: CI ran the whole Rust workspace, not the lane
  that leaves the media tests out;
- the actions CI pins behave as read.

**Found, and open — the Tests job has never been green there.** The Worker, the
installer, the landing, the agent skill, the media readers and the
frontend pass. The Rust workspace does not: the recorder's real-time tests miss their
pace on the runner — `beeps_land_on_their_seconds` in every run,
`a_frozen_screen_still_makes_a_full_length_video`,
`a_fast_source_is_thinned_to_the_constant_rate`,
`steady_source_two_takes_match_the_wall_clock` and
`a_take_whose_picture_lags_its_sound_still_ends` in some (“take 1 lasted 1.23 s, wanted
~1”; more than half of a take's ticks late). They pass on this Mac. Whether the runner
is held to them, or they stay with this Mac as the heartbeat does, is yours to decide
(DECISIONS B116); nothing was loosened. A sixth test, the Library's delete against a busy
writer, failed there four times in six through a fault of its own stand-in, mended on
`build/v1` (B116 e) — green on the runner since (PR #11's CI, twice).

**Ruled (Bagrat, 2026-10-13; DECISIONS B116):** the three wall-clock tests hold a take to
the window the test held it open; CI runs the recorder's test files one at a time; and
the two A/V timing bars run on CI in a step of their own whose miss is a warning, held on
a Mac by `cargo test --workspace` (`.github/scripts/rust-tests.sh --ci`, `--timing-bars`).
On branch `claude/open-work-issues-3-8` until it reaches `main`; this paragraph says
whether the Tests job then goes green.

**Found, and mended (2026-10-14):** after that ruling, two tests still went red on some PRs
and passed on a re-run. `a_fast_source_is_thinned_to_the_constant_rate` failed because its
test source was slow, not the recorder: drawing the pattern pixel by pixel took 10 ms a frame
in a debug build, so the "60 fps" source sent 17 frames in 1.1 s. The pattern is now drawn a
row at a time, 0.24 ms a frame, with the same bytes. Every test that draws it gets the speed,
the A/V timing bars included. `a_long_file_keeps_the_record_fresh…` waited a fixed 150 ms
for the upload's progress write; it now waits for the write itself. Under load in a cloud
session, each went from 50 failures in 50 to none.

**Settled by the first release** (`v0.1.5`, tagged on `c901be0`, 2026-10-13 20:07 UTC —
run 38460262629):

- the release workflow itself: a tag's build with the real identity, the publish, and the
  fetch back from the public address — all three jobs green on the first try;
- `gh` on the runner does what the publish job asks of it: `shroom 0.1.5` is listed as
  Latest, and the manifest and the zip read back from the public address match (SHA-256
  `547962d1527dc101b120a50967032282b6ca5746abf456df93dc1a19bc9f9266`, also checked from
  this Mac);
- how long a release run takes: the release was up about four minutes after the tag was
  pushed, queue included;
- **by Bagrat's hand (2026-10-14): invitations work** — tested with this release, a clean
  Tahoe clone beside his Mac and the install line at `https://shroom.am/install`.
