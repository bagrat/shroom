# A probe, not a check: what a one-line Homebrew install-and-join would do on a fresh Mac.
# Installs the public tap's cask as a person would, then hands shroom a join link by
# `open -a`, and does the same with two throwaway test apps (no URL types; one custom
# scheme) to tell macOS's behaviour apart from shroom's. Reports what happened; the
# verdict is read, not asserted. The link is invented and reaches nothing real.
set -u
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
APP="$HOME/Applications/shroom.app"
LINK="https://probe-owner.example.workers.dev/join/AbCdEfGh12345678"
SLINK="shroom://join/probe-owner.example.workers.dev/AbCdEfGh12345678"
OUT=${MAC_OUT:-$(mktemp -d)}
LSREG=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
shot() { screencapture -x "$OUT/$1.png" 2>/dev/null; echo "  screenshot: $1.png"; }
quit() { osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1; sleep 2; pkill -f "shroom.app/Contents/MacOS/" 2>/dev/null; sleep 1; true; }
browsers() { pgrep -lx Safari 2>/dev/null; pgrep -lf "Google Chrome.app/Contents/MacOS" 2>/dev/null | head -1; true; }
args_of() { ps -axo args | grep "$1/Contents/MacOS/" | grep -v grep | sed 's/^/  argv: /'; }
logtail() { [ -f "$HOME/.shroom/logs/run.log" ] && grep -E "member:|invite|link" "$HOME/.shroom/logs/run.log" | sed 's/^/  log: /' | tail -n 8; ls -l "$HOME/.shroom/invite" 2>/dev/null | sed 's/^/  file: /'; true; }

# Which app macOS would hand a URL to with plain `open` (no -a).
cat >"$OUT/handler.swift" <<'EOF'
import AppKit
for s in CommandLine.arguments.dropFirst() {
  let u = URL(string: s)!
  print("  handler for \(u.scheme!): \(NSWorkspace.shared.urlForApplication(toOpen: u)?.path ?? "none")")
}
EOF
swiftc -O -o "$OUT/handler" "$OUT/handler.swift" 2>&1 | tail -n 3

echo "macOS $(sw_vers -productVersion), brew $(brew --version | head -n 1)"
echo "── before anything"
"$OUT/handler" "$LINK" "$SLINK" "shroomprobe://join/x"
browsers | sed 's/^/  browser running: /'

echo "══ probe 1: brew install --cask bagrat/shroom/shroom && open -a shroom \"<https join link>\""
rm -rf "$APP"
t0=$(date +%s)
brew install --cask bagrat/shroom/shroom 2>&1 | tail -n 4; rc=${PIPESTATUS[0]}
echo "  brew exit $rc ($(( $(date +%s) - t0 ))s); $(defaults read "$APP/Contents/Info" CFBundleShortVersionString 2>/dev/null) at $APP"
/usr/libexec/PlistBuddy -c 'Print :CFBundleURLTypes' "$APP/Contents/Info.plist" 2>&1 | tr -s ' \n' ' ' | sed 's/^/  URL types: /'; echo
"$OUT/handler" "$LINK" "$SLINK"
said=$(open -a shroom "$LINK" 2>&1); echo "  open -a exit $? ${said:+said: $said}"
sleep 10
pgrep -f "$APP/Contents/MacOS/" >/dev/null && echo "  shroom: running" || echo "  shroom: NOT running"
args_of "$APP"
browsers | sed 's/^/  browser running: /'
logtail
shot p1-https-cold
echo "── probe 1b: the same link to the running shroom (warm)"
open -a shroom "$LINK"; echo "  open -a exit $?"; sleep 5; logtail; shot p1b-https-warm
quit

echo "══ probe 2 (shroom): open -a shroom \"<shroom:// join link>\", cold"
rm -rf "$HOME/.shroom/invite"
open -a shroom "$SLINK"; echo "  open -a exit $?"
sleep 10
args_of "$APP"; logtail; shot p2-scheme-cold
quit

echo "══ probe 4: brew install --cask bagrat/shroom/shroom && open -a shroom (fresh, no link)"
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
[ -e "$APP" ] && echo "  (still there after zap)"
brew install --cask bagrat/shroom/shroom >/dev/null 2>&1 && open -a shroom; echo "  line exit $?"
sleep 10
pgrep -f "$APP/Contents/MacOS/" >/dev/null && echo "  shroom: running" || echo "  shroom: NOT running"
shot p4-plain
echo "── probe 3 (shroom): plain open \"<shroom:// link>\" right after that install"
"$OUT/handler" "$SLINK"
quit
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
brew install --cask bagrat/shroom/shroom >/dev/null 2>&1
"$OUT/handler" "$SLINK"
said=$(open "$SLINK" 2>&1); echo "  plain open exit $? ${said:+said: $said}"
sleep 10; args_of "$APP"; logtail; shot p3-plain-open
quit

echo "══ the test apps: what macOS itself does"
cat >"$OUT/probe.swift" <<'EOF'
import AppKit
let log = "/tmp/" + (Bundle.main.bundleIdentifier ?? "probe") + ".log"
func say(_ s: String) {
  let line = "\(Date().timeIntervalSince1970) \(s)\n"
  if let h = FileHandle(forWritingAtPath: log) { h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); h.closeFile() }
  else { FileManager.default.createFile(atPath: log, contents: line.data(using: .utf8)) }
}
class D: NSObject, NSApplicationDelegate {
  func applicationWillFinishLaunching(_ n: Notification) { say("willFinishLaunching argv=\(CommandLine.arguments)") }
  func applicationDidFinishLaunching(_ n: Notification) { say("didFinishLaunching") }
  func application(_ a: NSApplication, open urls: [URL]) { say("openURLs \(urls.map { $0.absoluteString })") }
}
let d = D(); NSApplication.shared.delegate = d; NSApplication.shared.run()
EOF
stage=$(mktemp -d)
for kind in plain scheme; do
  b="$stage/probe-$kind.app"; mkdir -p "$b/Contents/MacOS"
  swiftc -O -o "$b/Contents/MacOS/probe" "$OUT/probe.swift" 2>&1 | tail -n 3
  types=""
  [ "$kind" = scheme ] && types='<key>CFBundleURLTypes</key><array><dict><key>CFBundleURLName</key><string>am.shroom.probe</string><key>CFBundleURLSchemes</key><array><string>shroomprobe</string></array></dict></array>'
  cat >"$b/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>am.shroom.probe.$kind</string><key>CFBundleName</key><string>probe-$kind</string>
<key>CFBundleExecutable</key><string>probe</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/>$types</dict></plist>
EOF
  codesign -s - --force "$b" >/dev/null 2>&1
done
for kind in plain scheme; do
  rm -f "/tmp/am.shroom.probe.$kind.log"
  mv "$stage/probe-$kind.app" "$HOME/Applications/"   # moved in, as brew moves a cask's app
done

echo "── probe 1 (macOS): open -a probe-plain \"<https link>\", cold, no URL types at all"
said=$(open -a probe-plain "$LINK" 2>&1); echo "  open -a exit $? ${said:+said: $said}"
sleep 4; sed 's/^/  app: /' /tmp/am.shroom.probe.plain.log 2>/dev/null || echo "  app: no log"
browsers | sed 's/^/  browser running: /'
echo "── warm"
open -a probe-plain "$LINK"; sleep 2; tail -n 1 /tmp/am.shroom.probe.plain.log | sed 's/^/  app: /'
echo "── a website's way in: does plain open of the https link ever reach it?"
"$OUT/handler" "$LINK"
pkill -x probe

echo "── probe 3 (macOS): plain open \"shroomprobe://…\" right after the move, before any launch"
for i in 0 1 2 3 4 5; do "$OUT/handler" "shroomprobe://join/x"; sleep 1; done
said=$(open "shroomprobe://join/plain-first" 2>&1); echo "  plain open exit $? ${said:+said: $said}"
sleep 4; sed 's/^/  app: /' /tmp/am.shroom.probe.scheme.log 2>/dev/null || echo "  app: no log"
pkill -x probe; rm -f /tmp/am.shroom.probe.scheme.log
echo "── probe 2 (macOS): open -a probe-scheme \"shroomprobe://…\", cold"
said=$(open -a probe-scheme "shroomprobe://join/with-a" 2>&1); echo "  open -a exit $? ${said:+said: $said}"
sleep 4; sed 's/^/  app: /' /tmp/am.shroom.probe.scheme.log 2>/dev/null || echo "  app: no log"
"$OUT/handler" "shroomprobe://join/x"
pkill -x probe
echo "── lsregister -f, then the handler again"
$LSREG -f "$HOME/Applications/probe-scheme.app"; echo "  lsregister exit $?"
"$OUT/handler" "shroomprobe://join/x"

rm -rf "$HOME/Applications/probe-plain.app" "$HOME/Applications/probe-scheme.app"
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
echo "done — read, not asserted"
exit 0
