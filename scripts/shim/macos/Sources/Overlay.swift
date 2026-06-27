// shroom — fullscreen overlay (the countdown + the Discard/Restart action panel).
//
// Why this exists: in fullscreen the menu-bar tray AUTOHIDES, so a tray-glyph
// countdown (or an NSAlert that steals focus) is the wrong surface for "recording
// is about to change." Instead we draw a transparent, always-on-top window that
// floats ABOVE other apps' fullscreen spaces WITHOUT switching Spaces — verified on
// device. One surface, two uses:
//   • countdown — a big 3-2-1 before capture starts (Start) or restarts (Restart);
//                 click anywhere or Esc cancels.
//   • choices   — a question + N custom-drawn buttons (a destructive one + safe
//                 ones), e.g. Discard → Keep / Resume / Restart / Discard. The
//                 accidental-click guard that also offers the adjacent actions.
//
// The enabling trick (don't lose these two lines): a borderless, clear NSWindow at
// CGShieldingWindowLevel with collectionBehavior [.canJoinAllSpaces,
// .fullScreenAuxiliary, .stationary]. We show it with orderFrontRegardless() and
// NEVER call NSApp.activate — activating an accessory app over someone else's
// fullscreen forces a Space switch; we want to ride the CURRENT space.
//
// Look: the whole screen dims (black 0.45). Behind the content we add a SOFT GLOW —
// the same black, more opaque in the middle, feathered to nothing at the edges via a
// radial gradient — so the text/buttons sit on a darker, readable area with NO
// visible box boundary (no frosted panel, no border). Buttons are CUSTOM-DRAWN and
// hit-tested in mouseDown (not NSButton): the view's mouseDown + acceptsFirstMouse
// is the path proven to take clicks over a fullscreen app without the app being
// active. This component shows, reports the choice via a callback, and hides — it
// NEVER terminates the app.

import Cocoa
import CoreImage

// Borderless windows can't become key by default; allow it so Esc reaches keyDown.
final class OverlayWindow: NSWindow {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }
}

// One labeled action on the choices panel. `destructive` tints it red.
struct OverlayAction {
    let title: String
    let destructive: Bool
    let handler: () -> Void
}

// The single full-screen overlay view: dim + soft glow + text + custom buttons.
final class OverlayView: NSView {
    enum Mode { case countdown, choices }

    var mode: Mode = .countdown
    var big = ""                      // countdown digit
    var title = ""
    var detail = ""
    var caption = ""                  // countdown hint under the number

    // Resolved by the Overlay before display (all in view/screen coordinates).
    var glowImage: NSImage?           // precomputed Gaussian-blurred dark blob
    var glowRect = NSRect.zero
    var titleY: CGFloat = 0
    var detailY: CGFloat = 0
    var buttons: [(rect: NSRect, action: OverlayAction)] = []

    var onBackground: (() -> Void)?   // countdown: a click anywhere = cancel
    var onEsc: (() -> Void)?

    override var isOpaque: Bool { false }
    override var acceptsFirstResponder: Bool { true }
    // Take the very first click even when our (accessory) app isn't active.
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func draw(_ dirty: NSRect) {
        NSColor(white: 0, alpha: 0.45).setFill() // dim the whole screen
        bounds.fill()
        glowImage?.draw(in: glowRect)             // soft, same-color, blurred edges

        switch mode {
        case .countdown:
            drawCentered(big, size: 190, weight: .regular, color: .white, y: bounds.midY - 95)
            if !caption.isEmpty {
                drawCentered(caption, size: 21, weight: .medium,
                             color: NSColor(white: 1, alpha: 0.9), y: bounds.midY - 215)
            }
        case .choices:
            if !title.isEmpty {
                drawCentered(title, size: 32, weight: .semibold, color: .white, y: titleY)
            }
            if !detail.isEmpty {
                drawCentered(detail, size: 18, weight: .regular,
                             color: NSColor(white: 1, alpha: 0.85), y: detailY)
            }
            for b in buttons {
                // Opaque fills so a button never depends on the screen behind it
                // (a translucent fill looks washed out over bright content).
                let fill = b.action.destructive
                    ? NSColor.systemRed.withAlphaComponent(0.95)
                    : NSColor(white: 0.26, alpha: 1.0)
                NSBezierPath(roundedRect: b.rect, xRadius: 12, yRadius: 12).fill(with: fill)
                drawCentered(b.action.title, size: 18, weight: .semibold, color: .white,
                             y: b.rect.midY - 11, in: b.rect)
            }
        }
    }

    private func drawCentered(_ s: String, size: CGFloat, weight: NSFont.Weight,
                              color: NSColor, y: CGFloat, in box: NSRect? = nil) {
        let para = NSMutableParagraphStyle(); para.alignment = .center
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: size, weight: weight),
            .foregroundColor: color, .paragraphStyle: para,
        ]
        let ns = s as NSString
        let sz = ns.size(withAttributes: attrs)
        let midX = (box ?? bounds).midX
        ns.draw(at: NSPoint(x: midX - sz.width / 2, y: y), withAttributes: attrs)
    }

    override func mouseDown(with event: NSEvent) {
        if mode == .countdown { onBackground?(); return }
        let p = convert(event.locationInWindow, from: nil)
        for b in buttons where b.rect.contains(p) { b.action.handler(); return }
        // a tap off the buttons does nothing — force a deliberate choice
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 { onEsc?() }   // Esc
    }
}

private extension NSBezierPath {
    func fill(with color: NSColor) { color.setFill(); fill() }
}

// Owns the overlay window. Each show builds a fresh window; finish() fades it out
// and runs exactly one outcome (guarded so a click can't race the timer).
final class Overlay {
    private var window: OverlayWindow?
    private var view: OverlayView?
    private var timer: Timer?
    private var done = true

    var isShowing: Bool { window != nil }

    private let buttonW: CGFloat = 178
    private let buttonH: CGFloat = 56
    private let buttonGap: CGFloat = 16

    // A dark rounded blob, Gaussian-blurred so its edges genuinely dissolve (no
    // visible boundary). Rendered once per overlay (not per frame). The shape is
    // inset by `blur*2` inside the canvas so the blur has room to fade fully to clear
    // before the image edge — otherwise the image edge itself would be a hard line.
    private func makeGlow(center: NSPoint, content: NSSize, corner: CGFloat,
                          alpha: CGFloat, blur: CGFloat) -> (NSImage, NSRect) {
        let pad = blur * 2
        let size = NSSize(width: content.width + pad * 2, height: content.height + pad * 2)
        let base = NSImage(size: size)
        base.lockFocus()
        NSColor(white: 0, alpha: alpha).setFill()
        NSBezierPath(roundedRect: NSRect(x: pad, y: pad, width: content.width, height: content.height),
                     xRadius: corner, yRadius: corner).fill()
        base.unlockFocus()
        let rect = NSRect(x: center.x - size.width / 2, y: center.y - size.height / 2,
                          width: size.width, height: size.height)
        guard let tiff = base.tiffRepresentation, let bm = NSBitmapImageRep(data: tiff),
              let cg = bm.cgImage, let f = CIFilter(name: "CIGaussianBlur") else { return (base, rect) }
        let ci = CIImage(cgImage: cg)
        f.setValue(ci, forKey: kCIInputImageKey)
        f.setValue(blur, forKey: kCIInputRadiusKey)
        guard let out = f.outputImage,
              let outCG = CIContext().createCGImage(out.cropped(to: ci.extent), from: ci.extent)
        else { return (base, rect) }
        return (NSImage(cgImage: outCG, size: size), rect)
    }

    private func makeView() -> (OverlayView, NSRect) {
        window?.orderOut(nil)   // never leak a prior overlay (flows shouldn't overlap)
        timer?.invalidate()
        let screen = NSScreen.main ?? NSScreen.screens[0]
        let win = OverlayWindow(contentRect: screen.frame, styleMask: .borderless,
                                backing: .buffered, defer: false)
        win.isOpaque = false
        win.backgroundColor = .clear
        win.hasShadow = false
        win.ignoresMouseEvents = false
        win.level = NSWindow.Level(rawValue: Int(CGShieldingWindowLevel()))
        win.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
        let v = OverlayView(frame: screen.frame)
        win.contentView = v
        win.orderFrontRegardless()      // show WITHOUT activating → no Space switch
        win.makeFirstResponder(v)       // best-effort, for Esc
        window = win; view = v; done = false
        return (v, screen.frame)
    }

    // A 3-2-1 the user can abort. onComplete fires at zero, onCancel on click/Esc.
    func countdown(seconds: Int, onComplete: @escaping () -> Void, onCancel: @escaping () -> Void) {
        var remaining = max(1, seconds)
        let (v, screen) = makeView()
        let cancel: () -> Void = { [weak self] in
            guard let self = self else { return }
            self.finish { onCancel() }
        }
        v.mode = .countdown
        v.big = String(remaining)
        v.caption = "click anywhere to cancel"
        // Tall enough to cover BOTH the number and the "click anywhere…" caption.
        (v.glowImage, v.glowRect) = makeGlow(center: NSPoint(x: screen.midX, y: screen.midY - 55),
                                             content: NSSize(width: 380, height: 420),
                                             corner: 150, alpha: 0.62, blur: 58)
        v.onBackground = cancel
        v.onEsc = cancel
        v.needsDisplay = true
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] t in
            guard let self = self else { return }
            remaining -= 1
            if remaining <= 0 { t.invalidate(); self.finish { onComplete() } }
            else { v.big = String(remaining); v.needsDisplay = true }
        }
    }

    // A question + N buttons. The first non-destructive action is the Esc default.
    func choices(title: String, detail: String, actions: [OverlayAction]) {
        let (v, screen) = makeView()
        let n = max(1, actions.count)
        let rowW = CGFloat(n) * buttonW + CGFloat(n - 1) * buttonGap
        let startX = screen.midX - rowW / 2
        let buttonRectY = screen.midY - 80                  // a clear gap below the text
        v.mode = .choices
        v.title = title
        v.detail = detail
        v.titleY = screen.midY + 70
        v.detailY = screen.midY + 30
        v.buttons = actions.enumerated().map { i, a in
            let rect = NSRect(x: startX + CGFloat(i) * (buttonW + buttonGap), y: buttonRectY,
                              width: buttonW, height: buttonH)
            let wrapped = OverlayAction(title: a.title, destructive: a.destructive) { [weak self] in
                guard let self = self else { return }
                self.finish { a.handler() }
            }
            return (rect, wrapped)
        }
        // Soft blurred glow around the whole content (title + gap + button row).
        (v.glowImage, v.glowRect) = makeGlow(center: NSPoint(x: screen.midX, y: screen.midY + 12),
                                             content: NSSize(width: rowW + 190, height: 296),
                                             corner: 105, alpha: 0.68, blur: 52)
        if let safe = actions.first(where: { !$0.destructive }) {
            v.onEsc = { [weak self] in
                guard let self = self else { return }
                self.finish { safe.handler() }
            }
        }
        v.needsDisplay = true
    }

    // External cancel (e.g. a tray click during the countdown) — no-op if hidden.
    func cancel() { view?.onBackground?() }

    // Fade out, tear the window down, then run the (single) outcome callback.
    private func finish(_ then: @escaping () -> Void) {
        guard !done else { return }
        done = true
        timer?.invalidate(); timer = nil
        guard let win = window else { then(); return }
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.15
            win.animator().alphaValue = 0
        }, completionHandler: { [weak self] in
            win.orderOut(nil)
            self?.window = nil; self?.view = nil
            then()
        })
    }
}
