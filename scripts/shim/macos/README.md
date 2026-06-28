# shroom macOS control shim (the "tray")

The per-OS native piece of recorder **process model B** (locked + verified —
[[mac-shim-permission-decision]], SPEC §4):

```
  shim  ──launches──▶  node record.mjs  ──spawns──▶  ffmpeg
 (this dir)            (deterministic core)          (capture)
```

It owns three things and nothing else:

1. **Screen-Recording + Microphone permission (TCC).** An on-device,
   **ad-hoc-signed** binary is its *own* TCC principal — it appears as **“shroom”** in
   System Settings → Privacy & Security, not under Claude/Terminal. Two mechanisms get
   it there: at startup it **re-execs itself with responsibility disclaimed**
   (`responsibility_spawnattrs_setdisclaim`) so it's its own responsible process (not
   the Terminal that launched it), and it **requests both permissions itself** (the
   grandchild ffmpeg would otherwise trip the mic prompt and get it pinned on
   Terminal). The grants it holds are inherited by ffmpeg, so capture works even
   though the shim never touches the screen or mic itself. Ad-hoc signing gives a
   **stable cdhash**, so the grants **persist for the life of this build** (re-prompts
   only when an update changes the binary — no Apple Developer account needed). It's a
   real `.app` bundle whose `Info.plist` carries the mic usage string, the clean
   "shroom" name, and our icon (so the Privacy panes show the mushroom, not a generic
   exec icon). `/shroom:record` front-loads both prompts via `shroom --permissions`
   before the real tray launches.
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
./build.sh        # swiftc -O → render icon → iconutil → codesign  → build/shroom.app
```

We ship **readable source** ([`Sources/main.swift`](Sources/main.swift)) and compile
it here at install — even the icon is rendered on-device from the same mushroom mark
the tray draws (no committed image blob). A precompiled binary that captures the
screen and sits near cloud creds is exactly the opaque thing an OSS audience shouldn't
be asked to trust — on-device compile means the bytes running are the bytes you can
read. `build.sh` requires the Xcode **Command Line Tools** (`swiftc`); if missing it
prints the one fix (`xcode-select --install`) and stops. Output (`build/`) is
gitignored. We package a real **`.app` bundle** (not a bare binary) so TCC shows our
icon + clean "shroom" name in the Privacy panes.

## Run

```sh
build/shroom.app/Contents/MacOS/shroom \
  --recorder /abs/path/to/scripts/recorder/record.mjs \
  [--node node] [--fifo <path>] [--log <path>] \
  -- --out <session-dir> --device "Capture screen 0" --audio default --quality normal
```

- Everything after `--` is passed **straight to `node record.mjs`** (so the agent's
  picker choices — device/audio/quality — flow through unchanged).
- `--fifo`/`--log` default to `<out>/control.fifo` and `<out>/shim-node.log`, derived
  from the `--out` in the passthrough (matching what the recorder uses), so usually
  you only pass `--out`.
- On first launch it requests Screen Recording + Microphone (registers "shroom" as
  the TCC principal). Screen Recording's grant takes effect on the next launch — which
  is why `/shroom:record` runs `--permissions` as a throwaway first, then launches the
  real tray fresh.

The tray's **primary (left) click** is the one obvious action per state — no menu
in the way:

| state | left-click | right / control-click |
| --- | --- | --- |
| `○` armed | start → **3-2-1 countdown** → record | menu (Discard) |
| `•` counting | **cancel** → armed | same |
| `●` recording (red) | **pause**, then open the menu | same |
| `❚❚` paused | open the menu (Resume / Stop / Restart) | same |
| `↻` restarting | (busy — discarding + re-arming) | — |

The 3-2-1 countdown is **cancelable** (click during it → back to armed), so a stray
click can't actually begin a recording. Pausing-on-click is **instant**, but the
menu opens only once the recorder confirms it has stopped ffmpeg (the shim watches
the recorder's `paused` event) — so the menu is **never caught in the recording's
last frame**. **Stop** and **Resume** stay deliberate menu choices (Stop is the
publish act; a stray click must not end-and-publish).

### The fullscreen overlay ([`Sources/Overlay.swift`](Sources/Overlay.swift))

In fullscreen the menu-bar tray **autohides**, so the countdown and the destructive
confirms are drawn as a **transparent, always-on-top overlay** that floats *above
other apps' fullscreen spaces without switching Spaces* (a borderless clear window
at `CGShieldingWindowLevel` with `canJoinAllSpaces + fullScreenAuxiliary`, shown via
`orderFrontRegardless` — never `NSApp.activate`). It renders the big countdown and,
for Discard/Restart, a question + custom-drawn buttons on a soft Gaussian-blurred
dark glow (no visible box). Buttons are hit-tested in `mouseDown` (not `NSButton`) —
the path proven to take clicks over a fullscreen app while the accessory app is
inactive.

- **Discard** opens a 4-way panel — **Keep · Resume · Restart · Discard** — so a
  fat-fingered Discard can pivot to a safe action instead of being a binary choice.
  Discard stops without publishing, **deletes the session**, and quits (covers the
  old "Quit"). From `armed`/`stopped` there's nothing recorded, so it skips the
  prompt and just tears down.
- **Restart** confirms (**Keep recording · Start over**), then throws the take away
  and starts fresh **without quitting**: it discards the recorder (the same `cancel`)
  and, once that process exits, relaunches a fresh recorder (new id, pristine
  session). The start-over **countdown runs in parallel with the ~1s relaunch**, so
  capture auto-starts the moment it reaches zero (the shim waits for the fresh
  recorder's `armed` event if the relaunch is slower). The recorder stays a simple
  single-session machine; the shim owns the relaunch.

## Not here yet

The no-permission global **hotkey** (Carbon `RegisterEventHotKey` — avoids the
Accessibility TCC prompt) is deferred. Wiring `/shroom:record` to launch the shim
(instead of starting capture itself) and having `/shroom:setup` compile it is the
remaining step (S4). The overlay currently sizes to `NSScreen.main` — multi-display
placement is a later refinement.

## Layout

```
Sources/main.swift   the shim: TCC (disclaim + perms) + tray + recorder launch + fifo
                     + --render-icon (the app icon, drawn from the same mushroom mark)
Sources/Overlay.swift  fullscreen countdown + Discard/Restart confirm overlay
Info.plist           the bundle's Contents/Info.plist (mic usage string, name, icon)
build.sh             on-device: compile + render icon → .icns + assemble .app + sign
build/               output: shroom.app (gitignored)
```

Cross-platform: the portable core (ffmpeg + fifo + Node recorder) is shared; this
`macos/` shim is one swappable per-OS implementation. Windows/Linux shims layer in
later against the same fifo/events contract.
