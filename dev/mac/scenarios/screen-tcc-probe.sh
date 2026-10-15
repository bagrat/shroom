# The screen-permission probe (DECISIONS: the perm-screen double prompt). Can an app
# appear in System Settings' Screen & System Audio Recording list without macOS's own
# "would like to record this computer's screen" alert? A throwaway bundle,
# am.shroom.probe, is asked three ways from a clean slate, and each way's grants row,
# windows and screen are written to MAC_OUT. Nothing here is shroom's own code: it settles
# what the CoreGraphics and ScreenCaptureKit calls do on this macOS.
set -u
. dev/mac/lib.sh
P="$MAC_OUT/probe"
mkdir -p "$P"
APP="$RUNNER_TEMP/probe/ShroomProbe.app"
mkdir -p "$APP/Contents/MacOS"
cat >"$APP/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>am.shroom.probe</string>
<key>CFBundleName</key><string>ShroomProbe</string>
<key>CFBundleExecutable</key><string>probe</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSUIElement</key><true/>
</dict></plist>
EOF
cat >"$RUNNER_TEMP/probe/main.swift" <<'EOF'
import CoreGraphics
import Foundation
import ScreenCaptureKit

let how = CommandLine.arguments.dropFirst().first ?? "preflight"
let log = CommandLine.arguments.dropFirst(2).first ?? "/tmp/probe.log"
func say(_ s: String) {
    let line = "\(Date()) \(how): \(s)\n"
    if let h = FileHandle(forWritingAtPath: log) { h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); h.closeFile() }
    else { try? line.write(toFile: log, atomically: true, encoding: .utf8) }
}
say("preflight before = \(CGPreflightScreenCaptureAccess())")
switch how {
case "request":
    let t = Date()
    say("request = \(CGRequestScreenCaptureAccess()) after \(Date().timeIntervalSince(t)) s")
case "scshare":
    SCShareableContent.getExcludingDesktopWindows(false, onScreenWindowsOnly: true) { c, e in
        say("shareable: \(c?.displays.count ?? -1) displays, error \(String(describing: e))")
    }
default: break
}
RunLoop.main.run(until: Date().addingTimeInterval(25))
say("preflight after = \(CGPreflightScreenCaptureAccess())")
EOF
swiftc -O -o "$APP/Contents/MacOS/probe" "$RUNNER_TEMP/probe/main.swift" 2>"$P/swiftc.log" || { cat "$P/swiftc.log"; exit 1; }
codesign -f -s - --identifier am.shroom.probe "$APP" 2>>"$P/swiftc.log"
LOG="$P/probe.log"

rows() {
  echo "── $1" >>"$P/rows.txt"
  sudo sqlite3 "$SYS_DB" "select client, auth_value, auth_reason, last_modified from access where service='kTCCServiceScreenCapture'" >>"$P/rows.txt" 2>&1
}
look() {
  sleep "${2:-3}"
  screencapture -x "$P/$1.png" >/dev/null 2>&1
  { echo "── $1"; windows; } >>"$P/windows.txt"
}
pane() {
  open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
  look "$1" 6
  osascript -e 'tell application "System Settings" to quit' >/dev/null 2>&1
  sleep 2
}
# macOS 26 draws the screen alert from universalAccessAuthWarn, not UserNotificationCenter.
warn_up() { windows | grep -q '^universalAccessAuthWarn:'; }
warn_click() {
  osascript -e "tell application \"System Events\" to tell process \"universalAccessAuthWarn\" to click button \"$1\" of window 1" >>"$P/rows.txt" 2>&1
  sleep 2
}
gone() { pkill -f "$APP/Contents/MacOS/probe" 2>/dev/null; warn_up && warn_click Deny; pkill -x universalAccessAuthWarn 2>/dev/null; sleep 1; }
fresh() {
  gone
  tccutil reset ScreenCapture am.shroom.probe >>"$P/rows.txt" 2>&1
  sleep 1
  rows "$1: after the reset"
}

# 1. Preflight only — what grants() does on every launch.
fresh "preflight"
open -n "$APP" --args preflight "$LOG"
look "1-preflight-launched"
rows "preflight: launched"
pane "1-preflight-pane"
rows "preflight: pane opened"
if grep -q '^am.shroom.probe' <(sudo sqlite3 "$SYS_DB" "select client from access where service='kTCCServiceScreenCapture'"); then
  note info "preflight alone registers" "yes — a grants row exists"
else
  note info "preflight alone registers" "no grants row"
fi

# 3. CGRequestScreenCaptureAccess — today's call — and a second ask after it.
fresh "request"
open -n "$APP" --args request "$LOG"
look "3-request-launched" 4
rows "request: launched"
warn_up && note info "request raises the alert" "yes" || note info "request raises the alert" "no"
# The alert's own "Open System Settings" — where does it go?
warn_click "Open System Settings"
look "3-request-alert-button" 6
rows "request: after the alert's button"
osascript -e 'tell application "System Settings" to quit' >/dev/null 2>&1
gone
sleep 2
open -n "$APP" --args request "$LOG"
look "3-request-again" 4
warn_up && note info "a second request raises the alert" "yes" || note info "a second request raises the alert" "no"
rows "request: asked again"
gone
sleep 2

# 2. ScreenCaptureKit's shareable content.
fresh "scshare"
open -n "$APP" --args scshare "$LOG"
look "2-scshare-launched" 4
rows "scshare: launched"
warn_up && note info "scshare raises the alert" "yes" || note info "scshare raises the alert" "no"
pane "2-scshare-pane"
gone
rows "scshare: after the pane"

# 4. A request whose alert is denied, then asked again.
fresh "deny"
open -n "$APP" --args request "$LOG"
look "4-deny-launched" 4
warn_click Deny
sleep 2
rows "deny: after Deny"
gone
sleep 2
open -n "$APP" --args request "$LOG"
look "4-deny-again" 4
warn_up && note info "a request after Deny raises the alert" "yes" || note info "a request after Deny raises the alert" "no"
pane "4-deny-pane"
gone

cat "$LOG" "$P/rows.txt"
tccutil reset ScreenCapture am.shroom.probe >/dev/null 2>&1
exit 0
