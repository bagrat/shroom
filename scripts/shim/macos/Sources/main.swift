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
import Darwin

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

enum RecState: Equatable { case armed, counting(Int), recording, pausing, paused, restarting, stopped }

final class ShimController: NSObject, NSApplicationDelegate {
    let args: Args
    var statusItem: NSStatusItem!
    var node: Process?
    var countdownTimer: Timer?
    var nodeBuf = ""   // accumulates recorder stdout for line-by-line event parsing
    // Set when a "Restart" is in flight: the current recorder is being discarded
    // (cancel) and, once it exits, we relaunch a fresh one instead of quitting.
    var pendingRestart = false
    var state: RecState = .armed { didSet { render() } }

    init(_ args: Args) { self.args = args }

    func applicationDidFinishLaunching(_ note: Notification) {
        ensureScreenRecordingAccess()
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
            Grant “shroom-shim” Screen Recording in System Settings → Privacy & \
            Security → Screen Recording, then quit and relaunch the shim. Capture \
            won’t work until it’s granted.
            """
            a.addButton(withTitle: "OK")
            a.runModal()
        }
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
                // fresh one (new id, pristine session dir + events) and go back to
                // `armed`, WITHOUT quitting the shim. `cancel` deleted the session
                // dir (fifo included), so re-create the fifo before relaunching.
                if self.pendingRestart {
                    self.pendingRestart = false
                    self.node = nil
                    self.nodeBuf = ""
                    self.ensureFifo()
                    self.state = .armed
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
    //   armed     → start a cancelable 3-2-1 countdown, then capture
    //   counting  → cancel, back to armed (so a stray click can't actually start a
    //               recording — you get 3 s to undo, which doubles as "get ready")
    //   recording → pause IMMEDIATELY, then open the menu — reaching for the tray
    //               halts capture (SPEC §4) — offering Resume / Stop
    //   paused    → open the menu (Resume / Stop)
    // Stop and Resume stay deliberate MENU choices, never a blind toggle: Stop is
    // the publish act, so a stray click must not end-and-publish. Right-click (or
    // control-click) opens the menu anywhere. The menu's escape is **Discard** —
    // stop without publishing, delete the session, and quit (covers the old Quit).
    // The paused menu also offers **Restart** — throw the take away and start fresh
    // without quitting the shim (discard + relaunch the recorder; see onRestart).

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

    // MARK: countdown

    // A 3-2-1 the user can abort (click during it → back to armed). Capture begins
    // only when it reaches zero — that's when `start` hits the fifo.
    func beginCountdown() {
        var remaining = 3
        state = .counting(remaining)
        countdownTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] t in
            guard let self = self else { return }
            remaining -= 1
            if remaining <= 0 {
                t.invalidate(); self.countdownTimer = nil
                self.send("start"); self.state = .recording
            } else {
                self.state = .counting(remaining)
            }
        }
    }

    func cancelCountdown() {
        countdownTimer?.invalidate(); countdownTimer = nil
        state = .armed
    }

    // MARK: tray rendering

    // Repaint the menu-bar glyph + tooltip for the current state. This is NOT a
    // menu — the menu is popped on demand (showMenu) so left-click stays an action.
    func render() {
        guard let button = statusItem?.button else { return }
        button.toolTip = label(for: state)
        var attrs: [NSAttributedString.Key: Any] = [:]
        if case .recording = state { attrs[.foregroundColor] = NSColor.systemRed } // universal "live"
        button.attributedTitle = NSAttributedString(string: glyph(for: state), attributes: attrs)
    }

    func glyph(for s: RecState) -> String {
        switch s {
        case .armed:           return "○"          // ready
        case .counting(let n): return String(n)    // 3 · 2 · 1
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
        case .counting(let n): return "shroom — starting in \(n)… (click to cancel)"
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

    // Start over without quitting: discard the current recorder (`cancel` stops
    // ffmpeg without publishing and deletes the session), then — once it exits —
    // terminationHandler relaunches a fresh recorder and re-arms (pendingRestart).
    // Same teardown as Discard; the only difference is we relaunch instead of quit.
    @objc func onRestart() {
        guard node != nil, state != .restarting, state != .stopped else { return }
        if case .counting = state { cancelCountdown() }
        pendingRestart = true
        state = .restarting
        send("cancel")
        // Backstop, mirroring Discard: if the recorder is wedged and never exits,
        // SIGTERM it so terminationHandler still fires and we re-arm.
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { [weak self] in
            guard let self = self, self.pendingRestart, let n = self.node, n.isRunning else { return }
            n.terminate()
        }
    }

    @objc func onDiscard() {
        // Throw this recording away: `cancel` stops ffmpeg WITHOUT publishing and
        // deletes the session, then the recorder exits and terminationHandler quits
        // us. This also covers the old "Quit" — discarding an armed/empty session
        // just tears down and closes. Delayed terminate is a backstop if wedged.
        if case .counting = state { cancelCountdown() }
        send("cancel")
        DispatchQueue.main.asyncAfter(deadline: .now() + 8) { NSApp.terminate(nil) }
    }
}

// MARK: - main

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // menu-bar only, no Dock icon
let controller = ShimController(parseArgs())
app.delegate = controller
app.run()
