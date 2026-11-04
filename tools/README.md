# tools/

Scripts run by hand, by the release build, or by CI. Each file's own header says what it
does and why; this is the index.

| File | What it's for | Run by |
|---|---|---|
| `bump-version.sh` | Moves the version in all seven places together; `--check` says whether they agree | you, before a release (docs/RELEASING.md); the tag workflow (`--check`) |
| `release-build.sh` | The one release build: the release config, the bundled encoder, the floor probe, the notices check | `tests/l5/build-release.sh`, CI's release build, the tag workflow |
| `package-release.sh` | Zips a built `shroom.app` as a release the installer and updater read (`manifest.txt` + zip) | the release build; the update drive |
| `make-release-identity.sh` | Makes the release's self-signed signing identity, once | you, once (docs/RELEASING.md, one-time setup); CI's signing rehearsal |
| `release-identity.sha1` | The release certificate's SHA-1 — the pin the tag workflow and the installer check | read by the tag workflow and `installer/install.sh` |
| `build-ffmpeg.sh` | Builds the release's LGPL encoder sidecar from the pinned FFmpeg tarball | you, before a release build; CI |
| `build-ffmpeg-oracle.sh` | Builds the media tests' independent ffmpeg/ffprobe for a Mac without one | CI's Tests job |
| `ffmpeg-NOTICE.txt` | FFmpeg's notice, shipped in the app beside the encoder | the release bundle |
| `third-party-notices.py` | Writes `THIRD-PARTY-NOTICES.md` from what the app ships; `--check` fails when it's stale | you, after a dependency change; CI; the release build |
| `licence-texts/` | Standard licence texts the notices script falls back on | `third-party-notices.py` |
| `render-machine-md.py` | Writes `spec/MACHINE.md`, the readable form of `spec/machine.json` (the spec); `--check` fails when it's stale | you, after every change to the JSON; CI's Tests job |
| `render-machine-canvas.mjs` | Writes the shroom UI canvas's **The machine** overview board from `spec/machine.json` (`--regions` also writes six region flow boards, for an agent's own look, not the canvas); needs `elkjs` (a root devDependency) | you, after every merge that touches the JSON (docs/DESIGN.md) |
| `machine-said-tags.mjs` | Tags the canvas's screen boards' said-lines with their ids (`--tag`) and reports drift between the boards' words and the JSON | you, with the canvas regeneration |
| `ci-route.py` | Whether a pull request's change can reach CI's Mac lanes (`BASE HEAD` → `mac=true\|false`, and why); `--reads` lists the web files a Mac lane reads, `--self-test` its own cases; a site-only tool (`render-og-card.mjs`) counts as web, and so does a change to `spec/machine.json` confined to the landing's state and the ledger (B276) | CI's route job, on every pull request |
| `e2e-asked.py` | Whether a push to a `claude/…` branch asked for an e2e run (`FILE BEFORE HEAD` → `asked=true\|false`): only the branch's own commits past main count, never a merge that brought main's requests in; its cases are `tools/test/e2e-asked.test.mjs` | e2e-full.yml's and e2e-migration.yml's first job, on every push to the request files |
| `machine-diff.py` | What a change to `spec/machine.json` changes, in words: states, said-lines, actions, edges, rules and groups added, removed or changed, with status and citations (`[base] [head]`, default `origin/main` and the working tree); `--touched` lists only the parts touched, as the router reads them | you, for a PR that touches the JSON; CI's machine job, on the run's summary |
| `machine-picture.mjs` | A before/after picture (SVG and an `index.html`) of the regions a change to the JSON touches; needs `elkjs` | you, with `machine-diff.py`; CI's machine job, as the `machine-picture` artifact |
| `render-tray-icons.mjs` | Renders the five menu-bar faces from SVG into `app/src-tauri/icons/tray/` | you, after a face changes |
| `render-app-icon.mjs` | Renders the app icon (the mushroom on a dark rounded square) into `app/src-tauri/icons/` and `icon.icns` | you, after the icon changes |
