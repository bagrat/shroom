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
// only, no Dock icon. The countdown, click-to-pause gesture, state icon, and global
// hotkey land in the next milestone — this is the skeleton that compiles, owns TCC,
// launches the recorder, and drives the fifo.)

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

enum RecState { case armed, recording, paused, stopped }

final class ShimController: NSObject, NSApplicationDelegate {
    let args: Args
    var statusItem: NSStatusItem!
    var node: Process?
    var state: RecState = .armed { didSet { rebuildMenu() } }

    init(_ args: Args) { self.args = args }

    func applicationDidFinishLaunching(_ note: Notification) {
        ensureScreenRecordingAccess()
        ensureFifo()
        setupStatusItem()
        launchRecorder()
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
        if !args.log.isEmpty {
            ensureParentDir(args.log)
            FileManager.default.createFile(atPath: args.log, contents: nil)
            if let fh = FileHandle(forWritingAtPath: args.log) {
                p.standardOutput = fh
                p.standardError = fh
            }
        }
        p.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                self?.state = .stopped
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

    func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        rebuildMenu()
    }

    // A state-aware menu: only the actions that make sense in the current state.
    // (The richer "click the tray to pause + open the menu" gesture is next.)
    func rebuildMenu() {
        guard let statusItem = statusItem else { return }
        statusItem.button?.title = glyph(for: state)
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
        case .armed:     item("Start Recording", #selector(onStart))
        case .recording: item("Pause", #selector(onPause)); item("Stop", #selector(onStop))
        case .paused:    item("Resume", #selector(onResume)); item("Stop", #selector(onStop))
        case .stopped:   break
        }
        menu.addItem(.separator())
        item("Quit", #selector(onQuit))
        statusItem.menu = menu
    }

    func glyph(for s: RecState) -> String {
        switch s {
        case .armed:     return "○"   // ready
        case .recording: return "●"   // live
        case .paused:    return "❚❚"
        case .stopped:   return "✓"
        }
    }
    func label(for s: RecState) -> String {
        switch s {
        case .armed:     return "shroom — ready to record"
        case .recording: return "shroom — recording"
        case .paused:    return "shroom — paused"
        case .stopped:   return "shroom — finished"
        }
    }

    // MARK: menu actions
    @objc func onStart()  { send("start");  state = .recording }
    @objc func onPause()  { send("pause");  state = .paused }
    @objc func onResume() { send("resume"); state = .recording }
    @objc func onStop()   { send("stop");   state = .stopped }
    @objc func onQuit() {
        // If a recording is live, stop it cleanly first so the recorder finalizes.
        if state == .recording || state == .paused { send("stop") }
        NSApp.terminate(nil)
    }
}

// MARK: - main

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // menu-bar only, no Dock icon
let controller = ShimController(parseArgs())
app.delegate = controller
app.run()
