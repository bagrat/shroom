# The setup window's size as macOS has it: the onboarding drive's email-gate path with the
# gate held 30 s, and meanwhile the window's frame (CGWindowBounds, title bar included) and
# its web area (the page's viewport) read from the accessibility tree, and the screen shot.
# Settles whether WindowSpec's height is the window's frame or the page's.
size() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
    const out = list.filter(w => /shroom/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 0)
      .map(w => "frame " + JSON.stringify(w.kCGWindowBounds));
    const se = Application("System Events");
    for (const p of se.processes.whose({ name: { _contains: "shroom" } })()) {
      for (const win of p.windows()) {
        out.push("ax window " + JSON.stringify({ size: win.size(), pos: win.position() }));
        for (const e of win.entireContents()) {
          const r = e.role();
          if (r === "AXWebArea" || r === "AXScrollArea" || r === "AXScrollBar") out.push("ax " + r + " " + JSON.stringify(e.size()));
        }
      }
    }
    out.join("\n")' 2>&1
}
( sleep 20; for i in 1 2 3; do size >>"$MAC_OUT/size.txt"; screencapture -x "$MAC_OUT/shot-$i.png"; sleep 4; done ) &
probe=$!
GATE_HOLD_MS=30000 node app/test/onboarding-drive.mjs email-gate
code=$?
wait "$probe" 2>/dev/null
cat "$MAC_OUT/size.txt"
exit $code
