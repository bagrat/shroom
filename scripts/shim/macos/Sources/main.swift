// shroom — macOS control shim (the "tray").
//
// This is the per-OS native piece of recorder process model B (locked, verified):
//
//     shim  ──launches──▶  node record.mjs  ──spawns──▶  ffmpeg
//   (this file)            (deterministic core)          (capture)
//
// What it owns, and ONLY this:
//   1. The Screen-Recording TCC permission. An on-device, ad-hoc-signed binary is
//      its own TCC principal (stable cdhash → the grant persists for this build's
//      life). The grant it holds is inherited by its grandchild ffmpeg, so capture
//      works even though the shim never touches the screen itself (verified).
//   2. A menu-bar item (the "tray") — the human's hands on the recording.
//   3. Launching the Node recorder and writing newline commands to its control fifo.
//
// What it deliberately does NOT own: the ffmpeg recipe, upload, finalize, events —
// all of that stays in the portable Node core (the determinism boundary, SPEC §4).
// The shim writes the SAME fifo any shell would (`echo start > control.fifo`); it
// is a thin launcher + buttons, nothing more.
//
// THE CONSENT BOUNDARY (this is the point of the rework): the agent can LAUNCH the
// shim but a human must START the actual capture. So the recorder launches `armed`
// (no ffmpeg yet, see record.mjs) and the tray's "Start Recording" is what writes
// `start`. A person always knowingly begins a screen recording.
//
// (Swift notes for the reader: `@objc` marks methods the menu can call as targets;
// `NSStatusItem` is the menu-bar item; `.accessory` activation policy = menu-bar
// only, no Dock icon. The countdown, click-to-pause gesture, state icon, Discard,
// and Restart are all here; the no-permission global hotkey lands later.)

import Cocoa
import CoreGraphics
import AVFoundation
import Darwin

// MARK: - TCC responsibility (disclaim re-exec)

// THE PROBLEM this solves: TCC doesn't attribute a permission to the process that
// *asks* — it walks up to the "responsible process" (the ancestor that owns the
// session). Launched as a background child of Terminal/Claude, our Screen-Recording
// request lands on *that* ancestor: the user sees "Terminal" (not "shroom-shim") in
// the prompt and in System Settings, and ad-hoc signing buys us nothing because the
// grant isn't even keyed to our binary.
//
// THE FIX: re-exec ourselves once with responsibility DISCLAIMED. The private libc
// call `responsibility_spawnattrs_setdisclaim` tells posix_spawn that the new image
// is its OWN responsible process — so the running shim is its own TCC principal,
// named "shroom-shim", with a stable cdhash from the ad-hoc signature. We pass
// POSIX_SPAWN_SETEXEC so it *replaces* this image (keeps pid + the stdout pipe the
// parent reads), and guard with an env var so it happens exactly once.
private func reexecDisclaimingResponsibility() {
    let guardKey = "SHROOM_SHIM_DISCLAIMED"
    if getenv(guardKey) != nil { return }   // already the disclaimed image

    // Private symbol in libsystem; absent → run in-process (best effort).
    typealias DisclaimFn =
        @convention(c) (UnsafeMutablePointer<posix_spawnattr_t?>, Int32) -> Int32
    guard let sym = dlsym(UnsafeMutableRawPointer(bitPattern: -2),  // RTLD_DEFAULT
                          "responsibility_spawnattrs_setdisclaim") else { return }
    let setDisclaim = unsafeBitCast(sym, to: DisclaimFn.self)

    // Resolve our own executable path (argv[0] may be relative).
    var size: UInt32 = 0
    _NSGetExecutablePath(nil, &size)
    var pathBuf = [CChar](repeating: 0, count: Int(size))
    guard _NSGetExecutablePath(&pathBuf, &size) == 0 else { return }
    var resolved = [CChar](repeating: 0, count: Int(PATH_MAX))
    let exePath = realpath(pathBuf, &resolved) != nil
        ? String(cString: resolved) : String(cString: pathBuf)

    var attr: posix_spawnattr_t?
    posix_spawnattr_init(&attr)
    defer { posix_spawnattr_destroy(&attr) }
    let POSIX_SPAWN_SETEXEC: Int16 = 0x0040   // replace this image, like execve
    posix_spawnattr_setflags(&attr, POSIX_SPAWN_SETEXEC)
    _ = setDisclaim(&attr, 1)

    // argv: ours, NULL-terminated.
    var argv: [UnsafeMutablePointer<CChar>?] = CommandLine.arguments.map { strdup($0) }
    argv.append(nil)
    // envp: current environment + our one-shot guard, NULL-terminated.
    var envp: [UnsafeMutablePointer<CChar>?] = []
    var e = environ
    while let cur = e.pointee { envp.append(strdup(cur)); e = e.advanced(by: 1) }
    envp.append(strdup("\(guardKey)=1"))
    envp.append(nil)

    exePath.withCString { _ = posix_spawn(nil, $0, nil, &attr, argv, envp) }
    // SETEXEC only returns on FAILURE — fall through and run in-process.
}

// MARK: - Permissions primer (`shroom --permissions`)

// Run by /shroom:record as a throwaway FIRST launch before the real tray: request
// Screen Recording + Microphone from THIS binary — the disclaimed "shroom" principal
// — so both prompts read "shroom" (not Terminal) and the grants are held by the same
// identity that records. Then exit; the real background tray launches fresh and
// inherits the grants. Idempotent: if both are already granted this prints and exits
// without prompting, so every record after the first is silent.
//
// Why a throwaway launch matters for Screen Recording: that grant can't be given from
// the prompt — the user toggles it in System Settings and it only takes effect on the
// NEXT launch. The primer IS that first launch, so the real tray (the next process)
// sees it live — no "quit and relaunch the tray" dance.
//
// Prints one line of JSON: {"screen":"granted|prompted","mic":"granted|denied"}.
//
// Mic and Screen Recording behave differently and must NOT share the screen. Mic is an
// instant inline Allow/Don't-Allow, so we resolve it FIRST and fully — never stacked
// under another dialog. Screen Recording can't be granted from its prompt at all (the
// user toggles it in System Settings, effective only on the NEXT launch), so we just
// REGISTER "shroom" in the Privacy list and report "prompted" — then EXIT. We don't
// hold a native dialog here: stacking one over the system prompt was confusing. The
// record command owns the screen gate instead — it opens System Settings and asks the
// user (its own AskUserQuestion) to toggle "shroom" on before it launches the real tray
// (which, as a fresh process, then sees the grant).
private func runPermissionsPrimerAndExit() -> Never {
    // 1) Microphone — resolve completely before the screen request so the two prompts
    //    never stack (allowing mic must not race/dismiss anything else).
    var micGranted = AVCaptureDevice.authorizationStatus(for: .audio) == .authorized
    if !micGranted {
        let sem = DispatchSemaphore(value: 0)
        AVCaptureDevice.requestAccess(for: .audio) { ok in micGranted = ok; sem.signal() }
        sem.wait()
    }

    // 2) Screen Recording — preflight; if not granted, the request registers "shroom"
    //    in the Privacy list (returns false until the user toggles it + relaunches).
    let screen = CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess()
        ? "granted" : "prompted"

    print("{\"screen\":\"\(screen)\",\"mic\":\"\(micGranted ? "granted" : "denied")\"}")
    exit(0)
}

// MARK: - Bypass-picker priming capture (`shroom --prime-capture`)

// Run by /shroom:record at PRIMING time — after Screen Recording is granted, BEFORE the
// real tray. Sequoia gates direct screen capture behind a SECOND consent (the "bypass the
// system private window picker" alert) that is separate from the Screen-Recording grant;
// it re-fires the first time capture runs after a throttle elapses (and whenever our
// install path changes). Left to the real recording, that alert lands mid-capture and
// interrupts the take. So we provoke + clear it here with a ~1s throwaway capture: open
// the SAME AVFoundation screen path the recipe uses (AVCaptureScreenInput is the
// avfoundation indev underneath ffmpeg), pull frames to a discard-only output, tear down,
// exit. This runs under the SAME disclaimed "shroom" principal as the real recorder (the
// disclaim re-exec still happens — capture only works because the grant inherits), so the
// throwaway both captures successfully AND clears the consent: the real recording right
// after is silent, and every later record until the throttle next elapses (which then
// surfaces here, at priming, not mid-record). We never write a file.
private final class PrimeCaptureSink: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate {}

private func runPrimeCaptureAndExit() -> Never {
    guard let input = AVCaptureScreenInput(displayID: CGMainDisplayID()) else { exit(1) }
    let session = AVCaptureSession()
    guard session.canAddInput(input) else { exit(1) }
    session.addInput(input)

    // A discard-only data output so the graph actually pulls screen frames (which is
    // what trips the consent) without ever writing anything.
    let output = AVCaptureVideoDataOutput()
    output.alwaysDiscardsLateVideoFrames = true
    let sink = PrimeCaptureSink()
    output.setSampleBufferDelegate(sink, queue: DispatchQueue(label: "am.shroom.prime"))
    if session.canAddOutput(output) { session.addOutput(output) }

    session.startRunning()
    Thread.sleep(forTimeInterval: 1.2)   // long enough to actually pull screen frames
    session.stopRunning()
    exit(0)
}

// MARK: - App-icon rendering (`--render-icon <px> <path>`)

// Render the colored mushroom mark (docs/logo.svg, the same shape the tray draws) at
// `size`×`size` into a PNG, then exit. build.sh calls this for each iconset size and
// runs iconutil → shroom.icns, so the app's Privacy-pane icon is generated on-device
// from source — no committed image blob.
private func renderAppIconAndExit(size: Int, to path: String) -> Never {
    guard size > 0,
          let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0) else { exit(1) }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    drawShroomAppIcon(canvas: CGFloat(size))
    NSGraphicsContext.restoreGraphicsState()
    guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
    do { try png.write(to: URL(fileURLWithPath: path)); exit(0) } catch { exit(1) }
}

// The mushroom on a rounded slate tile, in the Nord palette of docs/logo.svg. Design
// space matches the SVG: x∈[24,96], y∈[26,104] with y pointing DOWN; we fit + flip it
// into a centered region of the square canvas.
private func drawShroomAppIcon(canvas c: CGFloat) {
    // Rounded-rect tile (a touch of inset so the corners aren't clipped).
    let inset = c * 0.04
    let tile = NSBezierPath(roundedRect: NSRect(x: inset, y: inset, width: c - 2 * inset,
                                                height: c - 2 * inset),
                            xRadius: c * 0.225, yRadius: c * 0.225)
    NSColor(srgbRed: 0x2E/255, green: 0x34/255, blue: 0x40/255, alpha: 1).setFill() // Nord polar night
    tile.fill()

    // Fit the 72×78 mark into ~62% of the canvas, centered, y flipped for AppKit.
    let scale = (c * 0.62) / 78.0
    let drawnW = 72 * scale, drawnH = 78 * scale
    let offX = (c - drawnW) / 2 - 24 * scale
    let offY = (c - drawnH) / 2 + 104 * scale
    func P(_ x: CGFloat, _ y: CGFloat) -> NSPoint { NSPoint(x: offX + scale * x, y: offY - scale * y) }

    let stem = NSBezierPath()
    stem.move(to: P(50, 68))
    stem.curve(to: P(46, 96),  controlPoint1: P(49, 82),  controlPoint2: P(46, 92))
    stem.curve(to: P(60, 104), controlPoint1: P(46, 101), controlPoint2: P(52, 104))
    stem.curve(to: P(74, 96),  controlPoint1: P(68, 104), controlPoint2: P(74, 101))
    stem.curve(to: P(70, 68),  controlPoint1: P(74, 92),  controlPoint2: P(71, 82))
    stem.close()
    NSColor(srgbRed: 0xE5/255, green: 0xE9/255, blue: 0xF0/255, alpha: 1).setFill() // snow storm
    stem.fill()

    let cap = NSBezierPath()
    cap.move(to: P(24, 64))
    cap.curve(to: P(60, 26), controlPoint1: P(24, 42), controlPoint2: P(40, 26))
    cap.curve(to: P(96, 64), controlPoint1: P(80, 26), controlPoint2: P(96, 42))
    cap.curve(to: P(24, 64), controlPoint1: P(86, 75), controlPoint2: P(34, 75))
    cap.close()
    NSColor(srgbRed: 0x88/255, green: 0xC0/255, blue: 0xD0/255, alpha: 1).setFill() // frost
    cap.fill()

    NSColor(srgbRed: 0xEC/255, green: 0xEF/255, blue: 0xF4/255, alpha: 1).setFill() // spots
    for (cx, cy, r) in [(33.0, 58.0, 3.4), (88, 58, 2.2), (50, 44, 6), (72, 48, 3.6), (64, 58, 2.6)] {
        let p = P(CGFloat(cx), CGFloat(cy)); let rr = CGFloat(r) * scale
        NSBezierPath(ovalIn: NSRect(x: p.x - rr, y: p.y - rr, width: 2 * rr, height: 2 * rr)).fill()
    }
}

// MARK: - Arguments

// shroom-shim --recorder <record.mjs> [--node node] [--fifo <path>] [--log <path>]
//             -- [args passed straight to `node record.mjs` ...]
//
// `--fifo` defaults to "<out>/control.fifo", derived from a `--out <dir>` in the
// passthrough (matching what record.mjs uses), so the agent only has to say --out.
struct Args {
    var recorder: String = ""
    var node: String = "node"
    var fifo: String = ""
    var log: String = ""
    var passthrough: [String] = []
}

func parseArgs() -> Args {
    var a = Args()
    let argv = Array(CommandLine.arguments.dropFirst())
    var i = 0
    while i < argv.count {
        let arg = argv[i]
        if arg == "--" { a.passthrough = Array(argv[(i + 1)...]); break }
        func next() -> String { i += 1; return i < argv.count ? argv[i] : "" }
        switch arg {
        case "--recorder": a.recorder = next()
        case "--node":     a.node = next()
        case "--fifo":     a.fifo = next()
        case "--log":      a.log = next()
        default: break
        }
        i += 1
    }
    // Derive fifo/log from --out in the passthrough when not given explicitly.
    if a.fifo.isEmpty || a.log.isEmpty, let oi = a.passthrough.firstIndex(of: "--out"),
       oi + 1 < a.passthrough.count {
        let out = a.passthrough[oi + 1]
        if a.fifo.isEmpty { a.fifo = (out as NSString).appendingPathComponent("control.fifo") }
        if a.log.isEmpty  { a.log  = (out as NSString).appendingPathComponent("shim-node.log") }
    }
    return a
}

// MARK: - App

enum RecState: Equatable { case armed, counting, recording, pausing, paused, restarting, stopped }

final class ShimController: NSObject, NSApplicationDelegate {
    let args: Args
    var statusItem: NSStatusItem!
    var node: Process?
    let overlay = Overlay()   // fullscreen countdown + Discard/Restart confirm
    var nodeBuf = ""   // accumulates recorder stdout for line-by-line event parsing
    // Set when a "Restart" is in flight: the current recorder is being discarded
    // (cancel) and, once it exits, we relaunch a fresh one instead of quitting.
    var pendingRestart = false
    // Whether the (current) recorder has emitted `armed` — i.e. it's reading the
    // fifo and a `start` won't be dropped. Used to time the auto-start after a
    // Restart, whose countdown can finish before the fresh recorder has re-armed.
    var recorderArmed = false
    var awaitingAutoStart = false
    var state: RecState = .armed { didSet { render() } }

    init(_ args: Args) { self.args = args }

    func applicationDidFinishLaunching(_ note: Notification) {
        ensureScreenRecordingAccess()
        ensureMicrophoneAccess()
        ensureFifo()
        setupStatusItem()
        launchRecorder()
    }

    // Never leave an orphaned recorder behind: SIGTERM it on the way out so it
    // finalizes (if recording) or aborts (if armed) and exits cleanly.
    func applicationWillTerminate(_ note: Notification) {
        node?.terminate()
    }

    // MARK: TCC

    // Register the shim as the Screen-Recording principal up front so the user can
    // grant explicitly, instead of being surprised when ffmpeg later trips the
    // prompt. The grant is inherited by the grandchild ffmpeg. A first-ever grant
    // can need one relaunch to take effect; we surface that rather than silently
    // failing (auto-relaunch is a later refinement).
    func ensureScreenRecordingAccess() {
        if CGPreflightScreenCaptureAccess() { return }
        let granted = CGRequestScreenCaptureAccess() // prompts on first call
        if !granted {
            let a = NSAlert()
            a.messageText = "Screen Recording permission needed"
            a.informativeText = """
            Grant “shroom” Screen Recording in System Settings → Privacy & \
            Security → Screen Recording, then quit and relaunch shroom. Capture \
            won’t work until it’s granted.
            """
            a.addButton(withTitle: "OK")
            a.runModal()
        }
    }

    // Hold the Microphone grant on the "shroom" principal so the grandchild ffmpeg
    // inherits it — mirrors ensureScreenRecordingAccess. Without this the mic prompt
    // fires from ffmpeg instead and TCC attributes it to the parent (Terminal). The
    // record flow primes this up front via `shroom --permissions`; this launch-time
    // call is the safety net (and a no-op that never prompts once granted).
    func ensureMicrophoneAccess() {
        guard AVCaptureDevice.authorizationStatus(for: .audio) != .authorized else { return }
        AVCaptureDevice.requestAccess(for: .audio) { _ in }   // non-blocking; primes the grant
    }

    // MARK: fifo

    // Make sure the control fifo exists before any tray click can write to it (the
    // recorder also mkfifo's it, but the user might click fast). EEXIST is fine.
    // mkdir the session dir first — the shim can win the race to it before the
    // recorder's own mkdir, and mkfifo into a missing dir is ENOENT.
    func ensureFifo() {
        guard !args.fifo.isEmpty else { return }
        ensureParentDir(args.fifo)
        if mkfifo(args.fifo, 0o600) != 0 && errno != EEXIST {
            NSLog("shroom-shim: mkfifo(%@) failed: %d", args.fifo, errno)
        }
    }

    func ensureParentDir(_ filePath: String) {
        let dir = (filePath as NSString).deletingLastPathComponent
        guard !dir.isEmpty else { return }
        try? FileManager.default.createDirectory(
            atPath: dir, withIntermediateDirectories: true)
    }

    // Write one newline command to the fifo. Non-blocking open so a missing reader
    // (recorder not up yet) can never freeze the UI; done off the main thread.
    func send(_ cmd: String) {
        let fifo = args.fifo
        guard !fifo.isEmpty else { return }
        DispatchQueue.global(qos: .userInitiated).async {
            let fd = open(fifo, O_WRONLY | O_NONBLOCK)
            if fd < 0 { NSLog("shroom-shim: open fifo for %@ failed: %d", cmd, errno); return }
            defer { close(fd) }
            let line = cmd + "\n"
            _ = line.withCString { write(fd, $0, strlen($0)) }
        }
    }

    // MARK: recorder process

    // Launch `node record.mjs <passthrough>` as a child so the shim is its TCC
    // ancestor (model B). stdout/stderr are tee'd to the session log for debugging.
    func launchRecorder() {
        guard !args.recorder.isEmpty else {
            NSLog("shroom-shim: no --recorder given"); return
        }
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        p.arguments = [args.node, args.recorder] + args.passthrough
        // Read the recorder's stdout so we can react to its events (open the menu
        // only once it confirms `paused`, so the menu never lands in the recording)
        // and tee it to the session log.
        var logHandle: FileHandle? = nil
        if !args.log.isEmpty {
            ensureParentDir(args.log)
            FileManager.default.createFile(atPath: args.log, contents: nil)
            logHandle = FileHandle(forWritingAtPath: args.log)
        }
        let outPipe = Pipe()
        p.standardOutput = outPipe
        p.standardError = outPipe
        outPipe.fileHandleForReading.readabilityHandler = { [weak self] h in
            let data = h.availableData
            guard !data.isEmpty else { return }
            logHandle?.write(data)
            self?.consumeNodeOutput(data)
        }
        p.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                guard let self = self else { return }
                // Restart: the recorder we just discarded has exited — relaunch a
                // fresh one (new id, pristine session dir + events), WITHOUT quitting
                // the shim. `cancel` deleted the session dir (fifo included), so
                // re-create the fifo first. State is driven by the Restart overlay
                // flow (.restarting → .recording on auto-start, or → .armed if the
                // countdown is canceled); the fresh recorder's `armed` event sets
                // recorderArmed and may fire the queued auto-start (onRecorderArmed).
                if self.pendingRestart {
                    self.pendingRestart = false
                    self.node = nil
                    self.nodeBuf = ""
                    self.recorderArmed = false
                    self.ensureFifo()
                    self.launchRecorder()
                    return
                }
                self.state = .stopped
                // The recording finished (stop, or the process ended); the shim's
                // job is done. Linger briefly so the menu reads "Finished", then quit.
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { NSApp.terminate(nil) }
            }
        }
        do { try p.run(); node = p } catch {
            NSLog("shroom-shim: failed to launch recorder: %@", error.localizedDescription)
        }
    }

    // MARK: tray
    //
    // The tray's PRIMARY (left) click is the one obvious action for the current
    // state — no menu standing in the way:
    //   armed     → start a cancelable 3-2-1 countdown (the fullscreen overlay,
    //               visible even when the tray autohides), then capture
    //   counting  → cancel, back to armed (so a stray click can't actually start a
    //               recording — you get 3 s to undo, which doubles as "get ready")
    //   recording → pause IMMEDIATELY, then open the menu — reaching for the tray
    //               halts capture (SPEC §4) — offering Resume / Stop / Restart
    //   paused    → open the menu (Resume / Stop / Restart)
    // Stop and Resume stay deliberate MENU choices, never a blind toggle: Stop is
    // the publish act, so a stray click must not end-and-publish. Right-click (or
    // control-click) opens the menu anywhere. The menu's escape is **Discard** —
    // stop without publishing, delete the session, and quit (covers the old Quit).
    // The paused menu also offers **Restart** — throw the take away and start fresh
    // without quitting (discard + relaunch the recorder; see onRestart). Both
    // **Discard** and **Restart** confirm first via the overlay (accidental-click
    // guard); Restart then shows the same countdown and auto-starts the fresh take.

    func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.target = self
            button.action = #selector(statusClicked)
            // Receive both buttons ourselves instead of auto-popping a menu, so
            // left-click can run a state action.
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }
        render()
    }

    @objc func statusClicked() {
        let ev = NSApp.currentEvent
        let isRight = ev?.type == .rightMouseUp
            || ev?.modifierFlags.contains(.control) == true
        switch state {
        case .armed:
            if isRight { showMenu() } else { beginCountdown() }
        case .counting:
            cancelCountdown()                 // any click aborts the countdown
        case .recording:
            // Pause, but DON'T open the menu yet — wait for the recorder to confirm
            // it has actually stopped ffmpeg (onRecorderPaused). Opening the menu
            // immediately would catch it in the recording's last frame.
            send("pause"); state = .pausing
        case .pausing:
            break                             // ignore clicks until the pause lands
        case .paused:
            showMenu()
        case .restarting:
            break                             // ignore clicks until the re-arm lands
        case .stopped:
            break
        }
    }

    // MARK: recorder output

    // Parse the recorder's NDJSON stdout line by line. We only need a couple of
    // events; substring matching is enough and avoids pulling in a JSON parser.
    func consumeNodeOutput(_ data: Data) {
        guard let s = String(data: data, encoding: .utf8) else { return }
        nodeBuf += s
        while let nl = nodeBuf.firstIndex(of: "\n") {
            let line = String(nodeBuf[..<nl])
            nodeBuf = String(nodeBuf[nodeBuf.index(after: nl)...])
            if line.contains("\"event\":\"paused\"") {
                DispatchQueue.main.async { [weak self] in self?.onRecorderPaused() }
            } else if line.contains("\"event\":\"armed\"") {
                DispatchQueue.main.async { [weak self] in self?.onRecorderArmed() }
            }
        }
    }

    // The recorder has actually paused (ffmpeg stopped, last segment closed) — NOW
    // it's safe to open the menu; it can't end up in the recording.
    func onRecorderPaused() {
        guard state == .pausing else { return }
        state = .paused
        showMenu()
    }

    // The (possibly freshly relaunched) recorder is armed and reading the fifo. If a
    // Restart's countdown already finished and queued an auto-start, fire it now.
    func onRecorderArmed() {
        recorderArmed = true
        if awaitingAutoStart {
            awaitingAutoStart = false
            recorderArmed = false
            send("start"); state = .recording
        }
    }

    // MARK: countdown

    // A 3-2-1 the user can abort (click the overlay or the tray → back to armed).
    // The countdown is the fullscreen overlay (visible even when the tray is
    // autohidden in fullscreen), not a tray glyph. Capture begins only when it
    // reaches zero — that's when `start` hits the fifo.
    func beginCountdown() {
        state = .counting
        overlay.countdown(seconds: 3, onComplete: { [weak self] in
            guard let self = self else { return }
            self.recorderArmed = false
            self.send("start"); self.state = .recording
        }, onCancel: { [weak self] in
            self?.state = .armed
        })
    }

    func cancelCountdown() { overlay.cancel() } // runs the countdown's onCancel → armed

    // MARK: tray rendering

    // Repaint the menu-bar glyph + tooltip for the current state. This is NOT a
    // menu — the menu is popped on demand (showMenu) so left-click stays an action.
    func render() {
        guard let button = statusItem?.button else { return }
        button.toolTip = label(for: state)
        // Armed = the shroom mushroom mark; every other state stays a text glyph
        // (a colored dot/bars reads faster for live/paused than a tiny silhouette).
        if case .armed = state {
            button.image = armedIcon
            button.attributedTitle = NSAttributedString(string: "")
            return
        }
        button.image = nil
        var attrs: [NSAttributedString.Key: Any] = [:]
        if case .recording = state { attrs[.foregroundColor] = NSColor.systemRed } // universal "live"
        button.attributedTitle = NSAttributedString(string: glyph(for: state), attributes: attrs)
    }

    // The armed-state menu-bar icon: the shroom mushroom (cap + stem) with the
    // cap's spots punched out as transparent holes — the same mark as the site
    // logo (variation #45). Built as a template image so macOS tints it for the
    // current menu bar (white on a dark bar, black on a light one).
    lazy var armedIcon: NSImage = makeMushroomIcon()

    func makeMushroomIcon() -> NSImage {
        // Design space matches docs/logo.svg: the mark occupies x∈[24,96],
        // y∈[26,104] with y pointing DOWN (SVG convention). Fit that 72×78 box
        // into a 17×16pt image and flip y for AppKit's bottom-left origin.
        let w: CGFloat = 17, h: CGFloat = 16
        let img = NSImage(size: NSSize(width: w, height: h), flipped: false) { _ in
            guard let ctx = NSGraphicsContext.current else { return false }
            let pad: CGFloat = 1
            let s = min((w - 2 * pad) / 72.0, (h - 2 * pad) / 78.0)
            let offX = (w - 72 * s) / 2 - 24 * s
            let offY = (h - 78 * s) / 2 + 104 * s
            func P(_ x: CGFloat, _ y: CGFloat) -> NSPoint { NSPoint(x: offX + s * x, y: offY - s * y) }

            let body = NSBezierPath()
            body.move(to: P(24, 64))
            body.curve(to: P(60, 26), controlPoint1: P(24, 42), controlPoint2: P(40, 26))
            body.curve(to: P(96, 64), controlPoint1: P(80, 26), controlPoint2: P(96, 42))
            body.curve(to: P(24, 64), controlPoint1: P(86, 75), controlPoint2: P(34, 75))
            body.close()
            body.move(to: P(50, 68))
            body.curve(to: P(46, 96), controlPoint1: P(49, 82), controlPoint2: P(46, 92))
            body.curve(to: P(60, 104), controlPoint1: P(46, 101), controlPoint2: P(52, 104))
            body.curve(to: P(74, 96), controlPoint1: P(68, 104), controlPoint2: P(74, 101))
            body.curve(to: P(70, 68), controlPoint1: P(74, 92), controlPoint2: P(71, 82))
            body.close()
            NSColor.black.setFill()
            body.fill()

            // Punch the cap spots out as transparent holes (variation #45).
            let holes: [(CGFloat, CGFloat, CGFloat)] = [
                (33, 58, 3.4), (88, 58, 2.2), (50, 44, 6), (72, 48, 3.6), (64, 58, 2.6),
            ]
            ctx.compositingOperation = .destinationOut
            for (cx, cy, r) in holes {
                let c = P(cx, cy)
                let rr = r * s
                NSBezierPath(ovalIn: NSRect(x: c.x - rr, y: c.y - rr, width: 2 * rr, height: 2 * rr)).fill()
            }
            return true
        }
        img.isTemplate = true
        return img
    }

    func glyph(for s: RecState) -> String {
        switch s {
        case .armed:           return "🍄"         // ready (fallback; render() uses armedIcon)
        case .counting:        return "•"          // counting down (number is on the overlay)
        case .recording:       return "●"          // live (red)
        case .pausing:         return "❚❚"
        case .paused:          return "❚❚"
        case .restarting:      return "↻"          // discarding + re-arming
        case .stopped:         return "✓"
        }
    }
    func label(for s: RecState) -> String {
        switch s {
        case .armed:           return "shroom — ready (click to start)"
        case .counting:        return "shroom — starting… (click to cancel)"
        case .recording:       return "shroom — recording (click to pause)"
        case .pausing:         return "shroom — pausing…"
        case .paused:          return "shroom — paused"
        case .restarting:      return "shroom — starting over…"
        case .stopped:         return "shroom — finished"
        }
    }

    // MARK: menu

    // Pop the state-appropriate menu at the status item. Temporarily assigning the
    // menu + performClick is the standard way to show an NSMenu from a status button
    // whose left-click is otherwise bound to an action; we clear it again right after
    // so the next left-click hits statusClicked.
    func showMenu() {
        let menu = NSMenu()
        let header = NSMenuItem(title: label(for: state), action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        menu.addItem(.separator())

        func item(_ title: String, _ sel: Selector) {
            let m = NSMenuItem(title: title, action: sel, keyEquivalent: "")
            m.target = self
            menu.addItem(m)
        }
        switch state {
        case .paused:
            item("Resume", #selector(onResume))
            item("Stop", #selector(onStop))
            item("Restart", #selector(onRestart))   // throw this take away, start fresh
        case .counting: item("Cancel", #selector(onCancel))
        default:        break   // armed / stopped: just Discard
        }
        menu.addItem(.separator())
        item("Discard", #selector(onDiscard))

        statusItem.menu = menu
        statusItem.button?.performClick(nil)
        statusItem.menu = nil   // restore: left-click → statusClicked
    }

    // MARK: menu actions
    @objc func onResume() { send("resume"); state = .recording }
    @objc func onStop()   { send("stop");   state = .stopped }
    @objc func onCancel() { cancelCountdown() }

    // Restart (paused only): confirm first (the accidental-click guard), then start
    // over without quitting. Confirm + countdown are the fullscreen overlay, so they
    // work even when the tray is autohidden in fullscreen.
    @objc func onRestart() {
        guard node != nil, state == .paused else { return }
        overlay.choices(
            title: "Start over?",
            detail: "Deletes the current recording and starts fresh.",
            actions: [
                OverlayAction(title: "Keep recording", destructive: false) { /* stay paused */ },
                OverlayAction(title: "Start over", destructive: true) { [weak self] in self?.performRestart() },
            ])
    }

    // Discard the current recorder (`cancel` stops ffmpeg WITHOUT publishing and
    // deletes the session) and relaunch a fresh one — terminationHandler does the
    // relaunch because pendingRestart is set. The countdown runs in PARALLEL with
    // the ~1s relaunch: when it reaches zero we start immediately if the fresh
    // recorder is already armed, else as soon as its `armed` event lands. Canceling
    // the countdown leaves us armed (fresh), ready for a manual start.
    func performRestart() {
        pendingRestart = true
        recorderArmed = false
        awaitingAutoStart = false
        state = .restarting
        send("cancel")
        // Backstop: if the recorder is wedged and never exits, SIGTERM it so
        // terminationHandler still fires and we relaunch.
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { [weak self] in
            guard let self = self, self.pendingRestart, let n = self.node, n.isRunning else { return }
            n.terminate()
        }
        overlay.countdown(seconds: 3, onComplete: { [weak self] in
            self?.autoStartAfterRestart()
        }, onCancel: { [weak self] in
            self?.state = .armed   // stay armed; don't auto-start
        })
    }

    // The Restart countdown reached zero: start the fresh recording if it's armed,
    // otherwise queue it for the imminent `armed` event (onRecorderArmed).
    func autoStartAfterRestart() {
        if recorderArmed {
            recorderArmed = false
            send("start"); state = .recording
        } else {
            awaitingAutoStart = true
        }
    }

    // Discard: throw this recording away and quit. From armed/stopped there's nothing
    // recorded, so skip the confirm and just tear down (covers the old "Quit").
    // Otherwise confirm first. `cancel` stops ffmpeg without publishing and deletes
    // the session; the recorder exits and terminationHandler quits us (delayed
    // terminate is a backstop if wedged).
    @objc func onDiscard() {
        switch state {
        case .armed, .stopped:
            performDiscard()   // nothing recorded — just tear down (covers old "Quit")
        default:
            // A fat-fingered Discard can pivot to a safe action instead of a binary
            // choice: Keep (stay paused), Resume, Restart, or actually Discard.
            overlay.choices(
                title: "Discard this recording?",
                detail: "It won’t be saved or shared.",
                actions: [
                    OverlayAction(title: "Keep", destructive: false) { /* stay paused */ },
                    OverlayAction(title: "Resume", destructive: false) { [weak self] in self?.onResume() },
                    OverlayAction(title: "Restart", destructive: false) { [weak self] in self?.performRestart() },
                    OverlayAction(title: "Discard", destructive: true) { [weak self] in self?.performDiscard() },
                ])
        }
    }

    func performDiscard() {
        send("cancel")
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { NSApp.terminate(nil) }
    }
}

// MARK: - main

// Build step (run by build.sh): render the app icon from our own mushroom mark and
// exit. No TCC / disclaim needed — it's offscreen drawing. Must run BEFORE the
// disclaim re-exec so the build isn't spawning disclaimed copies.
if let ri = CommandLine.arguments.firstIndex(of: "--render-icon"),
   ri + 2 < CommandLine.arguments.count,
   let px = Int(CommandLine.arguments[ri + 1]) {
    renderAppIconAndExit(size: px, to: CommandLine.arguments[ri + 2])
}

// Become our own TCC principal before AppKit (and any capture) starts: re-exec
// disclaimed so the Screen-Recording + Microphone grants are named "shroom", not
// "Terminal".
reexecDisclaimingResponsibility()

// Permissions-priming mode (run by /shroom:record before the real tray): request
// screen + mic as "shroom", then exit. Idempotent — silent when already granted.
if CommandLine.arguments.contains("--permissions") {
    runPermissionsPrimerAndExit()
}

// Bypass-picker priming mode (run by /shroom:record after Screen Recording is granted,
// before the real tray): a ~1s throwaway capture that provokes + clears the Sequoia
// "bypass private window picker" consent up front, so it never interrupts a recording.
if CommandLine.arguments.contains("--prime-capture") {
    runPrimeCaptureAndExit()
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // menu-bar only, no Dock icon
let controller = ShimController(parseArgs())
app.delegate = controller
app.run()
