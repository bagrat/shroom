# Testing shroom

Three layers, fastest first. Most iteration lives in the top two; the VM is for
the things only a genuinely clean machine can prove.

| Layer | Proves | Speed |
| --- | --- | --- |
| **Unit tests** | script logic (probe, install-plan, SigV4, render, lifecycle) | seconds |
| **Script-direct** | a real script end-to-end against your machine | seconds |
| **Clean-machine VM** | real installs + first-run TCC prompts on a pristine macOS | minutes |

## Fast loops (no VM)

Per-package unit tests:

```
node --test scripts/setup/test/        # (or recorder/, uploader/, page/, deploy/, transcribe/)
```

Simulate a machine that's missing every tool — exercises the full `/shroom:setup`
install-plan UX with **zero side effects** (the probe scans `env.PATH`):

```
PATH=/usr/bin:/bin node scripts/setup/setup.mjs probe --json
```

## Clean-machine testing with Tart (macOS VM)

PATH-narrowing fakes "missing" but never proves the real side effects. Only a clean
VM exercises:

- `brew install ffmpeg` / `wrangler` from nothing, and the Node ≥22 path
- the `swiftc not found → xcode-select --install` gate and the **shim compile**
- **first-run Screen-Recording + Microphone TCC prompts** from a fresh principal
  (every prior run rode an inherited terminal-tree grant, so this was never truly proven)

We use [Tart](https://tart.run) (Apple-Silicon macOS VMs, copy-on-write clones).
Requirements: Apple Silicon Mac, macOS 13+, ~30 GB free. Apple permits up to 2 macOS
VMs per host.

### 1. Install Tart (one-time, host)

Prefer the prebuilt **notarized binary** — `brew install` can fall back to a source
build that fails on an outdated Command Line Tools:

```
curl -fsSL -o /tmp/tart.tar.gz https://github.com/cirruslabs/tart/releases/latest/download/tart.tar.gz
tar -xzf /tmp/tart.tar.gz -C /tmp
mkdir -p ~/Applications && mv /tmp/tart.app ~/Applications/
ln -sf ~/Applications/tart.app/Contents/MacOS/tart /opt/homebrew/bin/tart
tart --version
```

(`softnet`, only needed for isolated VM networking, installs fine via
`brew install cirruslabs/cli/softnet`.)

### 2. Pull a clean base image (one-time, ~30 GB)

The `-base` image deliberately has **no Homebrew and no Xcode tools**, so setup's
installs are real:

```
tart clone ghcr.io/cirruslabs/macos-sequoia-base:latest shroom-clean
```

Login inside the VM is `admin` / `admin`.

### 3. Build the golden (one-time)

Boot it, prep it, then stop it — **a stopped VM is the snapshot**:

```
tart run shroom-clean        # opens a GUI window
```

Inside the VM:

1. Install Claude Code: `curl -fsSL https://claude.ai/install.sh | bash`, then run
   `claude` and sign in.
2. **Do not** install ffmpeg / wrangler / Xcode tools — leaving them absent is the
   whole point.

Then shut it down cleanly:

```
tart stop shroom-clean
```

`shroom-clean` is now your golden. **Never run it directly again** except to update
it (e.g. a newer Claude Code) — always clone it for tests.

### 4. Per-test loop (disposable, resets each run)

```
tart clone shroom-clean shroom-run               # instant copy-on-write clone
tart run shroom-run                              # GUI window — drive the test here
# ...
tart stop shroom-run && tart delete shroom-run   # roll back to pristine
```

You drive the GUI (TCC prompts need real clicks); manage the VM lifecycle from the
host CLI.

### Getting the plugin into the test VM

**Published path — the real user experience (clean clone):**

```
/plugin marketplace add bagrat/claude-plugins
/plugin install shroom
```

**Local working tree — to test uncommitted edits:** boot with the repo mounted and
add a local marketplace pointing at it:

```
tart run shroom-run --dir=plugin:/path/to/shroom-claude-plugin
# inside the VM the tree is at: /Volumes/My Shared Files/plugin
```

### What to validate in the VM

- **`/shroom:setup`** — brew-installs ffmpeg + wrangler from nothing; Node ≥22 path;
  `xcode-select --install` gate → `swiftc` shim compile; optionally the Cloudflare
  R2 + Pages provisioning (already validated live, so optional here).
- **`/shroom:record`** — first-run Screen-Recording + Mic TCC prompts from a fresh
  principal; tray Start → Stop → live link; Discard → "nothing to publish".
- Reset (`delete` + re-`clone`) between scenarios for a truly clean run each time.

See the recorder/tray validation checklist in the project notes for the full set of
tray-gesture cases (countdown cancel, multi-take join, Restart, Discard 4-way, etc.).
