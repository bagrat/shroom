# shroom macOS control shim (the "tray")

The per-OS native piece of recorder **process model B** (locked + verified —
[[mac-shim-permission-decision]], SPEC §4):

```
  shim  ──launches──▶  node record.mjs  ──spawns──▶  ffmpeg
 (this dir)            (deterministic core)          (capture)
```

It owns three things and nothing else:

1. **Screen-Recording permission (TCC).** An on-device, **ad-hoc-signed** binary is
   its *own* TCC principal — it appears under its own name in System Settings →
   Privacy & Security → Screen Recording, not under Claude/Terminal. The grant it
   holds is inherited by its grandchild ffmpeg, so capture works even though the
   shim never touches the screen itself. Ad-hoc signing gives a **stable cdhash**,
   so the grant **persists for the life of this build** (re-prompts only when an
   update changes the binary — no Apple Developer account needed).
2. **The menu-bar item (the tray)** — the human's hands on the recording.
3. **Launching the Node recorder + writing its control fifo.** The shim writes the
   exact same newline commands any shell would (`echo start > control.fifo`); it is
   a thin launcher + buttons. The ffmpeg recipe, upload, finalize, and events all
   stay in the portable Node core (the determinism boundary).

**The consent boundary it enforces:** the recorder launches **`armed`** (no ffmpeg
yet — see [`../../recorder/`](../../recorder/)); the tray's **Start Recording** is
what writes `start`. The agent can launch the shim but a **human** always knowingly
begins the screen capture.

## Build (on-device, never a precompiled blob)

```sh
./build.sh        # swiftc -O … + codesign --force --sign -  → build/shroom-shim
```

We ship **readable source** ([`Sources/main.swift`](Sources/main.swift)) and compile
it here at install. A precompiled binary that captures the screen and sits near
cloud creds is exactly the opaque thing an OSS audience shouldn't be asked to trust
— on-device compile means the bytes running are the bytes you can read. `build.sh`
requires the Xcode **Command Line Tools** (`swiftc`); if missing it prints the one
fix (`xcode-select --install`) and stops. Output (`build/`) is gitignored.

## Run

```sh
build/shroom-shim \
  --recorder /abs/path/to/scripts/recorder/record.mjs \
  [--node node] [--fifo <path>] [--log <path>] \
  -- --out <session-dir> --device "Capture screen 0" --audio default --quality normal
```

- Everything after `--` is passed **straight to `node record.mjs`** (so the agent's
  picker choices — device/audio/quality — flow through unchanged).
- `--fifo`/`--log` default to `<out>/control.fifo` and `<out>/shim-node.log`, derived
  from the `--out` in the passthrough (matching what the recorder uses), so usually
  you only pass `--out`.
- On first launch it requests Screen Recording (registers the shim as the TCC
  principal). A first-ever grant can need one quit+relaunch to take effect.

The tray menu is **state-aware**: `armed` → **Start Recording**; `recording` →
**Pause** / **Stop**; `paused` → **Resume** / **Stop**. When the recorder exits, the
shim shows "finished" briefly and quits. **Quit** while recording stops cleanly
first (so the recorder finalizes).

## Not here yet (next milestone)

The 3-2-1 countdown before capture, the **click-the-tray-to-pause-and-open-the-menu**
gesture, a state-changing tray icon, and the no-permission global **hotkey** (Carbon
`RegisterEventHotKey` — avoids the Accessibility TCC prompt) all land next. This
milestone is the skeleton that compiles, owns TCC, launches the recorder, and drives
the fifo.

## Layout

```
Sources/main.swift   the shim: TCC + tray + recorder launch + fifo writes
build.sh             on-device compile + ad-hoc sign
build/               output (gitignored)
```

Cross-platform: the portable core (ffmpeg + fifo + Node recorder) is shared; this
`macos/` shim is one swappable per-OS implementation. Windows/Linux shims layer in
later against the same fifo/events contract.
