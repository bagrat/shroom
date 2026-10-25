# Releasing shroom

A release is a GitHub release of `bagrat/shroom` carrying exactly three files:

- `manifest.txt` — the version, the macOS floor, the zip's name and its SHA-256;
- `update.txt` — the same file under the name the app's updater asks for, so the release's
  download counts tell the app's checks from the install line's runs (DECISIONS B136);
- `shroom-<version>-arm64.zip` — the signed app.

The install line, the app's own updater and the Homebrew cask read nothing else
(`releases/latest/download/manifest.txt` — `update.txt` for the app, which falls back to
`manifest.txt` on a release without one — then `releases/download/v<version>/<zip>`;
DECISIONS B29, B31). You make one with **Actions → Release → Run workflow** on `main`,
Publish ticked (B126): `.github/workflows/release.yml` builds, signs, packages and
publishes it, and tags the commit it built. A session can't push tags, so the hand-pushed
tag is Bagrat's alone ("Cutting a release"). Nothing is ever uploaded by hand.

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

1. Get the workflows onto GitHub: merge them to `main` (by a PR, as everything reaches
   `main` — CLAUDE.md). CI starts by itself on that merge. *(Done: they have been on `main` since
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
   certificate means changing all three — DECISIONS B125, B175.)

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

Steps 8 and 9 are **required**: they are what keeps the release identity to the code you
meant to ship. A run of the workflow uses the copy of `release.yml` on the branch it runs
from, so the workflow's own checks can't guard the secrets; the environment's rules can,
because they live in GitHub's settings, not in a branch. Check both before every launch.

8. *(Required — your approval before each signed build.)* On the `release` environment's
   page: **Required reviewers** → add yourself, and leave **Prevent self-review** unticked
   (you are the only reviewer). Each signed build then waits for your click in the run's
   page before it can read the secrets.

9. *(Required — who may sign.)* Still on the environment's page: **Deployment branches
   and tags** → **Selected branches and tags** → add the branch `main`, the branch
   `early-release` (early builds: [Pre-releases](#pre-releases)) and the tag rule `v*` —
   nothing else. A rehearsal off any other branch runs outside the environment, signed ad
   hoc. And under **Settings** → **Rules** → **Rulesets** → **New tag ruleset**: target
   `v*`, restrict creations, updates and deletions, with yourself on the bypass list — so
   nobody else can push a release tag, and a published version's tag can't be moved.

10. Look at CI on `main`: all four jobs green — **Tests**, **Release build (ad hoc)**,
    **Signing rehearsal**, **Advisories**. The rehearsal does everything a release does to
    sign, with a throwaway certificate. (As of 2026-10-14 all four are green on pull
    requests; **Tests** leaves the A/V timing bars to a Mac as warnings — "What has not
    been proven".)

11. Rehearse with the real identity, publishing nothing: **Actions** → **Release** → **Run
    workflow** → branch `main`. It builds and signs exactly as a tag would (off `main`, a
    rehearsal builds the same but never signs with the identity), shows the
    signature and the SHA-256s in the run's summary, keeps the two files as an artifact
    for a week, and skips the publish job.

## Cutting a release

A release ships the version the tree is at, and a version is released once. The commands
below are written for **0.1.4**: where the tree stands as this is written (2026-10-13),
and not out when it was written. (The first release was **0.1.5**, 2026-10-13 — "What has
not been proven". 0.1.4 was never released.) Read your version for it.

**From a cloud session** (Bagrat, 2026-10-19, after 0.1.26):

- A session can't push tags: its push is refused with HTTP 403. So a release from a session
  is always **Actions → Release → Run workflow** on `main` with **Publish**, which builds
  `main`'s head.
- To keep a change out of a release, hold its merge until the release is out. Don't pin an
  older commit.
- The tag road (step 6's note) is Bagrat's, by hand.

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

   If an early build heading to a higher version is out (Pre-releases), release that
   version instead: with `0.1.33-early.1` out and the tree at an unreleased `0.1.32`, move
   it to `0.1.33`, so early Macs take the release too.

   Between releases the tree's version moves only when a build goes out (DECISIONS B248): a
   page change while its version is unreleased re-stamps that version's row in
   `worker/page-version.json` (the page-version test prints it), and once a build of it is
   published — a release or an early build — the row is frozen and the next page change
   needs the next version (`tools/page-version-out.sh`, in CI, holds that).

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

4. If you moved the version in step 1, or wrote the notes in step 2, commit them on a
   `claude/…` branch and merge its PR (nothing is pushed to `main` directly — CLAUDE.md):

   ```bash
   git add docs/release-notes && git commit -am "chore: shroom 0.1.5"
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

## Pre-releases

An early build (DECISIONS B220) is a release marked **pre-release** on GitHub: the same
signed zip, `manifest.txt` and `update.txt`, made by the same workflow, signed by the same
pinned certificate — so a Mac that takes it keeps its Screen Recording permission. GitHub's
"latest" never points at a pre-release, so the install line, Homebrew and every app on the
official channel never see it. Only a Mac on the **early channel** takes it.

**Versions.** An early build is `x.y.z-early.n` on the way to release `x.y.z`, n counting
from 1: `0.1.24` → `0.1.25-early.1` → `0.1.25-early.2` → `0.1.25` → `0.1.26-early.1`. Every reader of
a version keeps that order: the app's updater, the page compare, `tools/bump-version.sh`, the
installer and the release workflow. `tools/bump-version.sh 0.1.25-early.1` moves the tree there
from `0.1.24`, released or not, but never down from `0.1.25`: the tree holds its own version's
page row, and an early build's page must be newer (B248). So from an unreleased `0.1.25`, the
early build is `0.1.26-early.1`. The next is `-early.2`, and `0.1.26` itself is the release. The release workflow takes an early build's version only with **Early** ticked, and
an early build only with that version.

**Branches.** The channel's own code lives on `main`. Early builds are made on the
long-lived integration branch, `claude/early-integration`: `main` plus the feature pull
requests held for a release, each merged in. They are published from `early-release`,
which only ever points at a commit of that branch (DECISIONS B254): sessions push only
`claude/` branches, and the `release` environment allows no `claude/` branch. It takes `main` by merging it in, never by a
rebase. `main` stays free — fixes and features merge to it as always, and releases are cut
from it as always (Cutting a release, above). A feature reaches `main` through its own pull
request, never through this branch.

**A hotfix while an early build is out.** Release it from `main` as the version the early
build was heading to: with `0.1.25-early.2` out, the fix ships as `0.1.25`. An early Mac takes
it (it's newer), and the integration branch, after merging `main` in, moves to
`0.1.26-early.1` for the next early build. The workflow refuses an early build of a version
already released — it would sort below it, and no Mac would take it.

**The pointer.** A pre-release tagged `early`, titled "shroom early channel", is not a
release: its `manifest.txt` and `update.txt` name the newest build, early or official. The
first publish that finds none makes it; every publish after puts its own manifest there
unless the pointer already names a newer version. Nothing is uploaded by hand. To see what
it names:

```bash
curl -fsSL https://github.com/bagrat/shroom/releases/download/early/manifest.txt
```

**Cutting one**, on the integration branch:

1. Merge `main` in, and the feature pull requests' newest heads; resolve, and run the
   checks (CI's Tests job, as a contributor runs them).
2. Move it to the next early version (`tools/bump-version.sh 0.1.25-early.1`, or `-early.2`
   after it), commit and push. Its draft pull request (never merged) is what runs CI on its
   head.
3. See CI green on that head, and the hosted-Mac heartbeat (**Actions** → **E2E full** →
   Run workflow on the branch).
4. **Actions** → **Early release branch** → **Run workflow** on
   `claude/early-integration`. It moves `early-release` forward to that head, and refuses
   unless CI is green on it, so what's built next is the commit CI and the heartbeat tested.
   Then **Actions** → **Release** → **Run workflow** → `early-release`, tick **Publish** and
   **Early**. With the environment's required reviewer set (One-time setup, step 8), the
   build waits for your **Approve and deploy**. The run refuses unless CI is green on that commit, the version is an early
   build's and isn't tagged, and its release isn't out; it needs no release notes
   (shroom.am's What's new sends a version it has no notes for to the newest release's). It
   tags `v<version>` on the commit it built.
5. The run's publish job fails if `releases/latest` turned out to be the early build, and
   warns if the pointer doesn't name it.

The release workflow publishes an early build from `early-release` and no other branch,
and it is the one branch besides `main` the `release` environment allows (One-time setup,
step 9). Renaming it means changing all of them: `EARLY_BRANCH` and the publish job's
condition in `release.yml`, `early-release.yml`, and that list.

**Taking the early channel on a Mac.** Once, in Terminal:

```bash
curl -fsSL https://shroom.am/install | sh -s -- --early
```

It installs the newest build the pointer names (or, with no early build out, the latest release, and says so) — over the shroom that is there, the
way an update does — and leaves `~/.shroom/channel` holding `early`. From then on the app's
own updater reads the pointer and the latest release and takes the newer, so an official
release that is out is taken too. `run.log` says `boot: channel early`, and Copy
diagnostics says `shroom <version> (early channel)`. To leave the channel:

```bash
rm ~/.shroom/channel
```

The app stays on the build it has, and takes the next release above it (`0.1.25` after
`0.1.25-early.2`).

**A space's page.** An early app puts its own page on its space as any update does, so the
people watching that space's videos see the early build's page. A page only ever moves up:
an older official app on another Mac of the same space leaves it as it is, and the next
build — early or official — replaces it. Take the early channel only on a Mac whose space's
viewers can live with that.

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
   your Mac carries only the installed release, official or early (DECISIONS B66, B220).

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
(DECISIONS B175). Its `Casks/shroom.rb` is never edited there: the tap's **Bump** workflow
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

**CI** (`.github/workflows/ci.yml`) — every push to `main`, every pull request. It is
routed (the **Route** job, `tools/ci-route.py`): `main` and Run workflow run every job; a
pull request runs the Linux jobs always and the Mac's (the two Mac test lanes, the release
build, the signing rehearsal) only when it touches something outside `site/`, `worker/`,
`docs/`, `dev/`, `history/`, `.claude/`, `spec/MACHINE.md` and top-level prose, or a web
file a Mac lane reads (the router finds those in the Mac lanes' sources:
`python3 tools/ci-route.py --reads`). A skipped Mac job reports as skipped. An early build
from a branch needs those jobs green, so it is of a branch that touched the app — which
always runs them.

| Job | What it does |
|---|---|
| Tests | the gate: green when every test lane this run needs is — **Tests · web (Linux)** (the spec's readable form current, the third-party notices, the frontend's build, typecheck and unit suite, the Worker's unit suite, typecheck and contract under workerd, the release notes, the landing page, the agent skill, the media readers), **Tests · Rust (Linux)** (the crates with no platform in them, `rust-tests.sh --ci-linux`), **Tests · Settings fits its window (Linux)** (every General, Videos and Space pane no taller than the window's page, `app/test/settings-fit.mjs`, N79), **Tests · Rust and media (Mac)** (the rest of the workspace and the real recordings, `rust-tests.sh --ci`) and **Tests · installer (Mac)** (the installer and the cask, with real `ditto` and `codesign`) |
| Release build (ad hoc) | the release build with no identity (signed ad hoc), every bundle check, packaged as a stand-in; then signed after the build with a throwaway identity, as a release is (`--resign`). From `main` it saves the caches a tag needs |
| Signing rehearsal | makes a throwaway identity with `tools/make-release-identity.sh`, imports and trusts it as a release would, signs a file and checks macOS would keep a permission by it |
| Advisories | `cargo audit` against RustSec and `npm audit --audit-level=high` (DECISIONS B125): a known vulnerability in a pinned dependency fails it; an unmaintained-crate notice is listed, not fatal |

**Release** (`.github/workflows/release.yml`) — a pushed tag `v*`:

| Job | What it does |
|---|---|
| The tag is the tree's version | the tag equals the version in all seven version files; the commit is on `main`; the certificate is pinned; the version's notes are written (a rehearsal only warns) |
| Build, sign, package | restores the caches, builds the encoder only if its cache missed, runs the release build with no key on the runner (signed ad hoc), then imports the identity into a throwaway keychain (its key not exportable; deleted whatever happens), checks it is the pinned certificate, signs the finished bundle with it (`--resign`), packages it. A rehearsal off `main` skips the identity and keeps the ad hoc build |
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
  (`dev/mac/loop.sh`), or their Tart lanes. (The galleries' Settings height check alone runs
  in CI: `settings-fit.mjs`.)
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
